// Execute the actual optimized and reference WGSL on a host GPU.
// Invoked by test_android_gpu.py; this checks numerical equivalence, not Android FPS.
#[test]
fn sunrise_sectors_match_reference_on_real_gpu() {
    use super::sunrise_fan;
    use wgpu::util::DeviceExt;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("Sunrise GPU correctness: {}", adapter.get_info().name);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/sunrise.wgsl")).into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mut rays = Vec::<[f32; 4]>::new();
    for yaw in 0..360 {
        for pitch in -89..90 {
            let (sy, cy) = (yaw as f32).to_radians().sin_cos();
            let (sp, cp) = (pitch as f32).to_radians().sin_cos();
            rays.push([sy * cp, sp, cy * cp, 0.]);
        }
    }
    rays.extend([
        [1., 0., 0., 0.],
        [-1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., -1., 0., 0.],
    ]);
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&rays),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (rays.len() * 8) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    for alpha in [0.01, 0.2, 0.65, 1.] {
        for hemisphere in [-1., 1.] {
            let mut globals = vec![[1., 1., 1., alpha]];
            globals.extend(sunrise_fan::normals(
                alpha,
                glam::Vec3::new(0., 0., hemisphere),
            ));
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&globals),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bindings = [&uniform, &input, &output];
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &bindings
                    .iter()
                    .enumerate()
                    .map(|(i, b)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: b.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups((rays.len() as u32).div_ceil(64), 1, 1);
            }
            encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
            queue.submit([encoder.finish()]);
            read.slice(..)
                .map_async(wgpu::MapMode::Read, |r| r.unwrap());
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            {
                let mapped = read.slice(..).get_mapped_range().unwrap();
                let pairs: &[[f32; 2]] = bytemuck::cast_slice(&mapped);
                for (i, pair) in pairs.iter().enumerate() {
                    assert!(
                        (pair[0] - pair[1]).abs() < 1e-4,
                        "alpha={alpha} hemisphere={hemisphere} ray={:?} values={pair:?}",
                        rays[i]
                    );
                }
            }
            read.unmap();
        }
    }
}
