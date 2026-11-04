//! Render the actual original/lowered shaders: packing must preserve RGB,
//! cutout/fade behaviour and every HDR emission tier, rather than forcing alpha.
use wgpu::util::DeviceExt;

fn render(device: &wgpu::Device, queue: &wgpu::Queue, source: &str, variant: u8) -> Vec<u8> {
    render_paths(device, queue, source, variant, false, 255, false)
}
fn render_paths(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &str,
    variant: u8,
    split: bool,
    alpha: u8,
    mixed: bool,
) -> Vec<u8> {
    render_specialized(device, queue, source, variant, split, alpha, mixed, 0)
}
fn render_specialized(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &str,
    variant: u8,
    split: bool,
    alpha: u8,
    mixed: bool,
    settled: u8,
) -> Vec<u8> {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let (_, globals) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Globals"))
        .unwrap();
    let naga::TypeInner::Struct { members, span } = &globals.inner else {
        panic!("Globals")
    };
    let mut bytes = vec![0u8; *span as usize];
    let mut write = |name: &str, values: &[f32]| {
        let offset = members
            .iter()
            .find(|m| m.name.as_deref() == Some(name))
            .unwrap()
            .offset as usize;
        bytes[offset..offset + values.len() * 4].copy_from_slice(bytemuck::cast_slice(values));
    };
    let z = if variant == 0 || variant >= 4 {
        0.0
    } else {
        32.0
    };
    let mut matrix = glam::Mat4::IDENTITY.to_cols_array();
    matrix[14] = -z;
    write("view_proj", &matrix);
    write("fog_color", &[0.3, 0.5, 0.7, 0.8]);
    write(
        "fog",
        if variant == 0 || variant >= 4 {
            &[1e6, 2e6, 1e6, 2e6]
        } else {
            &[20.0, 50.0, 25.0, 60.0]
        },
    );
    write("filtering", &[0.0, 16.0, 0.0, 0.0]);
    if variant == 3 {
        write(
            "colored_lights",
            &[0.0, 0.0, 32.0, 8.0, 1.0, 0.25, 0.5, 1.0],
        );
        // This field is u32, unlike the floating-point helpers above.
        let member = members
            .iter()
            .find(|m| m.name.as_deref() == Some("colored_count"))
            .unwrap();
        bytes[member.offset as usize..member.offset as usize + 4]
            .copy_from_slice(&1u32.to_le_bytes());
    }
    if variant >= 4 {
        let offset = |name: &str| {
            members
                .iter()
                .find(|m| m.name.as_deref() == Some(name))
                .unwrap()
                .offset as usize
        };
        let table = offset("emissive_layers");
        for entry in 0..32usize {
            let key = if entry == usize::from(variant - 4) {
                0u32
            } else {
                99u32
            };
            bytes[table + entry * 16..table + entry * 16 + 4].copy_from_slice(&key.to_le_bytes());
        }
        let count = offset("emissive_count");
        bytes[count..count + 4].copy_from_slice(&32u32.to_le_bytes());
    }
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let textured = variant == 36;
    let edge = if textured { 16 } else { 1 };
    let white = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: edge,
            height: edge,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let texels: Vec<u8> = if textured {
        (0..256u32)
            .flat_map(|i| {
                [
                    i.wrapping_mul(37) as u8,
                    i.wrapping_mul(91) as u8,
                    i.wrapping_mul(17) as u8,
                    if i % 4 == 0 { 0 } else { alpha },
                ]
            })
            .collect()
    } else {
        vec![255, 255, 255, alpha]
    };
    queue.write_texture(
        white.as_image_copy(),
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(edge * 4),
            rows_per_image: Some(edge),
        },
        white.size(),
    );
    let array = white.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let light = white.create_view(&Default::default());
    let sampler = device.create_sampler(&Default::default());
    let entries: Vec<_> = (0..13)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            count: None,
            ty: match binding {
                0 => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                2 | 4 => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                _ => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if binding == 3 || binding == 12 {
                        wgpu::TextureViewDimension::D2
                    } else {
                        wgpu::TextureViewDimension::D2Array
                    },
                    multisampled: false,
                },
            },
        })
        .collect();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });
    let entries: Vec<_> = (0..13)
        .map(|binding| wgpu::BindGroupEntry {
            binding,
            resource: match binding {
                0 => uniform.as_entire_binding(),
                2 | 4 => wgpu::BindingResource::Sampler(&sampler),
                3 | 12 => wgpu::BindingResource::TextureView(&light),
                _ => wgpu::BindingResource::TextureView(&array),
            },
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        ..Default::default()
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Terrain interface regression"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let attributes =
        wgpu::vertex_attr_array![0=>Float32x3,1=>Uint16x2,2=>Uint16,4=>Uint8x2,3=>Unorm8x4];
    let instance_attributes = wgpu::vertex_attr_array![5=>Float32x4];
    let constants: &[(&str, f64)] = if settled == 2 {
        &[("TERRAIN_FILTERING", 0.0), ("TERRAIN_FULLY_VISIBLE", 1.0)]
    } else {
        &[]
    };
    let make_pipeline = |vertex, fragment| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(vertex),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 16,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &instance_attributes,
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(fragment),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    let pipelines = if split {
        vec![
            make_pipeline("vs_terrain_opaque", "fs_opaque"),
            make_pipeline(
                "vs_terrain_cutout",
                if settled == 2 {
                    "fs_cutout_static"
                } else {
                    "fs_solid"
                },
            ),
        ]
    } else {
        vec![make_pipeline("vs_terrain", "fs_solid")]
    };
    let mut vertices = Vec::new();
    for emission in 0..16u8 {
        let x = -1.0 + f32::from(emission) / 8.0;
        for [px, py] in [
            [x, -1.0],
            [x + 0.125, -1.0],
            [x + 0.125, 1.0],
            [x, -1.0],
            [x + 0.125, 1.0],
            [x, 1.0],
        ] {
            vertices.extend_from_slice(bytemuck::cast_slice(&[px, py, 0.5f32]));
            vertices.extend_from_slice(bytemuck::cast_slice(&[
                if textured {
                    ((px - x) * 8.0 * 32768.0) as u16
                } else {
                    16384u16
                },
                if textured {
                    ((py + 1.0) * 0.5 * 32768.0) as u16
                } else {
                    16384
                },
                if mixed && emission % 2 == 0 {
                    0x8000
                } else {
                    0
                },
            ]));
            vertices.extend_from_slice(&[240, 240, 64, 128, 192, 255 - emission]);
        }
    }
    let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &vertices,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let step = if variant == 0 || settled != 0 {
        1.0 / 32768.0
    } else if variant == 1 {
        1.5 / 32768.0
    } else {
        -(56.0 + 1.0 / 32768.0)
    };
    let instance = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[0.0f32, 0.0, z, step]),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 32,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_bind_group(0, &group, &[]);
        pass.set_vertex_buffer(0, vertex.slice(..));
        pass.set_vertex_buffer(1, instance.slice(..));
        for pipeline in &pipelines {
            pass.set_pipeline(pipeline);
            pass.draw(0..96, 0..1);
        }
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    read.slice(..).get_mapped_range().unwrap().to_vec()
}

#[test]
fn settled_terrain_preserves_fog_lighting_alpha_and_emissive_masks() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let source = super::terrain_shader(false);
    for alpha in [0, 64, 127, 128, 255] {
        for variant in 0..36 {
            let expected =
                render_specialized(&device, &queue, &source, variant, false, alpha, true, 1);
            let actual =
                render_specialized(&device, &queue, &source, variant, true, alpha, true, 2);
            assert_eq!(expected, actual, "scene {variant}, alpha {alpha}");
        }
    }
}

#[test]
fn specialized_sampling_preserves_textured_edges_and_cutouts() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let source = super::terrain_shader(false);
    let expected = render_specialized(&device, &queue, &source, 36, false, 255, true, 1);
    let actual = render_specialized(&device, &queue, &source, 36, true, 255, true, 2);
    assert!(
        expected.iter().filter(|&&v| v != 0).count() > 32,
        "Empty test image"
    );
    assert_eq!(expected, actual, "Textured sampling/cutout pixels changed");
}

#[test]
fn packed_interface_preserves_rgb_fog_fade_and_all_hdr_tiers_on_gpu() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    for variant in 0..36 {
        let original = render(&device, &queue, &super::terrain_shader(false), variant);
        let packed = render(&device, &queue, &super::terrain_shader(true), variant);
        assert_eq!(original, packed, "RGB/alpha differs for scene {variant}");
        if variant >= 4 {
            assert_eq!(
                &packed[4..7],
                &[255, 255, 255],
                "emissive slot {} was not selected",
                variant - 4
            );
            // 0.1 sits between UNORM8 codes 25 and 26; GPU rounding may choose either.
            // The original/lowered full image equality above remains exact.
            assert!((25..=26).contains(&packed[7]));
        }
        if variant < 2 {
            for emission in 0..16usize {
                let p = 4 * (2 * emission + 1);
                let alpha = match emission {
                    0 => 255,
                    1 | 5 => 134,
                    2 | 6 => 89,
                    3 | 7 => 45,
                    _ => 0,
                };
                assert_eq!(packed[p + 3], alpha, "HDR tier {emission} lost");
                if variant == 0 {
                    assert_eq!(&packed[p..p + 3], &[64, 128, 192]);
                }
            }
        }
    }
}

#[test]
fn split_opaque_cutout_preserves_pixels_for_alpha_fog_fade_and_emission() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    for alpha in [0, 64, 127, 128, 255] {
        for variant in 0..36 {
            let source = super::terrain_shader(false);
            let original = render_paths(&device, &queue, &source, variant, false, alpha, true);
            let split = render_paths(&device, &queue, &source, variant, true, alpha, true);
            assert_eq!(original, split, "scene {variant}, texture alpha {alpha}");
        }
    }
}

#[test]
fn static_cutout_preserves_alpha_threshold_fog_and_hdr_without_fading() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let source = super::terrain_shader(false);
    let specialized = source.replace("return solid_color(in);", "return fs_cutout_static(in);");
    // Calling a fragment entry point as a helper is forbidden by WGSL; rename
    // only its declaration in this independently rendered comparison module.
    let specialized = specialized.replace(
        "@fragment fn fs_cutout_static(in: TerrainOut) -> @location(0) vec4<f32>",
        "fn fs_cutout_static(in: TerrainOut) -> vec4<f32>",
    );
    for alpha in [0, 64, 127, 128, 255] {
        for variant in [0, 1] {
            let original = render_paths(&device, &queue, &source, variant, false, alpha, true);
            let actual = render_paths(&device, &queue, &specialized, variant, false, alpha, true);
            assert_eq!(original, actual, "scene {variant}, alpha {alpha}");
        }
    }
}
