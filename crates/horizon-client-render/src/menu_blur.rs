//! Vanilla's menu background blur (`GameRenderer.processBlurEffect`, the
//! `minecraft:blur` post chain): three rounds of a horizontal then a vertical
//! `post/box_blur` pass over the frame drawn so far, `menuBackgroundBlurriness`
//! framebuffer pixels wide, before the screen is drawn over it. Works in the
//! frame's own (gamma) values, as vanilla's main target does, and needs a
//! swapchain that can be copied from.

/// `blur.json`: the passes' directions, ping-ponging between the copied
/// frame and a swap texture; the last one writes the frame.
const PASSES: [[f32; 2]; 6] = [
    [1.0, 0.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [0.0, 1.0],
];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    size_dir: [f32; 4],
    radius: [f32; 4],
}

pub struct MenuBlur {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    uniforms: Vec<wgpu::Buffer>,
    /// The copied frame (`minecraft:main`) and `swap`.
    targets: Vec<(wgpu::Texture, wgpu::TextureView)>,
    groups: Vec<wgpu::BindGroup>,
    size: [u32; 2],
    format: wgpu::TextureFormat,
    pub cached_valid: bool,
    pub cached_size: (u32, u32),
    pub cached_radius: u8,
}
impl MenuBlur {
    /// `format` is the swapchain's; the blur works in its non-sRGB view.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let format = format.remove_srgb_suffix();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Menu blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("menu_blur.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Menu blur bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Menu blur"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Menu blur pass"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        // The post chain's "bilinear": true inputs, clamped like render targets.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Menu blur"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniforms = PASSES
            .iter()
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Menu blur pass"),
                    size: std::mem::size_of::<Uniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Self {
            layout,
            pipeline,
            sampler,
            uniforms,
            targets: Vec::new(),
            groups: Vec::new(),
            size: [0, 0],
            format,
            cached_valid: false,
            cached_size: (0, 0),
            cached_radius: 0,
        }
    }
    pub fn invalidate(&mut self) {
        self.cached_valid = false;
    }
    pub fn is_cached(&self, size: [u32; 2], radius: f32) -> bool {
        let copy_size = (size[0].max(1), size[1].max(1));
        let r = radius.round().clamp(0.0, 255.0) as u8;
        self.cached_valid
            && self.cached_size == copy_size
            && self.cached_radius == r
            && !self.targets.is_empty()
    }
    pub fn blit_cached(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> bool {
        if !self.cached_valid || self.targets.is_empty() || self.groups.is_empty() {
            return false;
        }
        let data = Uniform {
            size_dir: [
                self.cached_size.0.max(1) as f32,
                self.cached_size.1.max(1) as f32,
                0.0,
                0.0,
            ],
            radius: [0.0, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.uniforms[0], 0, bytemuck::bytes_of(&data));
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Menu blur cached blit"),
            color_attachments: &attachments,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[0], &[]);
        pass.draw(0..3, 0..1);
        true
    }
    fn ensure(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        let size = [size[0].max(1), size[1].max(1)];
        if self.size == size && !self.targets.is_empty() {
            return;
        }
        self.size = size;
        self.cached_valid = false;
        let downscaled = if cfg!(target_os = "android") {
            [(size[0] / 3).max(1), (size[1] / 3).max(1)]
        } else {
            size
        };
        self.targets = [size, downscaled]
            .into_iter()
            .map(|target_size| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Menu blur target"),
                    size: wgpu::Extent3d {
                        width: target_size[0],
                        height: target_size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                (texture, view)
            })
            .collect();
        self.groups = (0..PASSES.len())
            .map(|i| {
                let source = &self.targets[i % 2].1;
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Menu blur pass"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.uniforms[i].as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
    }
    /// Blur `frame` (the swapchain texture) in place through `target`, its
    /// non-sRGB view, by `radius` pixels (`menuBackgroundBlurriness`).
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &wgpu::Texture,
        target: &wgpu::TextureView,
        size: [u32; 2],
        radius: f32,
    ) {
        let copy_size = [
            size[0].min(frame.width()).max(1),
            size[1].min(frame.height()).max(1),
        ];
        self.ensure(device, copy_size);
        encoder.copy_texture_to_texture(
            frame.as_image_copy(),
            self.targets[0].0.as_image_copy(),
            wgpu::Extent3d {
                width: copy_size[0],
                height: copy_size[1],
                depth_or_array_layers: 1,
            },
        );
        if radius < 0.5 {
            self.cached_valid = true;
            self.cached_size = (copy_size[0], copy_size[1]);
            self.cached_radius = 0;
            return;
        }
        let active_passes = if cfg!(target_os = "android") {
            &PASSES[..2]
        } else {
            &PASSES[..]
        };
        let down_size = if cfg!(target_os = "android") {
            [(copy_size[0] / 3).max(1), (copy_size[1] / 3).max(1)]
        } else {
            copy_size
        };
        for (i, dir) in active_passes.iter().enumerate() {
            let src_size = if cfg!(target_os = "android") && i > 0 {
                down_size
            } else {
                copy_size
            };
            let pass_radius = if cfg!(target_os = "android") && i > 0 {
                (radius * 0.45).max(1.0)
            } else {
                radius
            };
            let data = Uniform {
                size_dir: [src_size[0] as f32, src_size[1] as f32, dir[0], dir[1]],
                radius: [pass_radius, 0.0, 0.0, 0.0],
            };
            queue.write_buffer(&self.uniforms[i], 0, bytemuck::bytes_of(&data));
            let output = if i + 1 == active_passes.len() {
                target
            } else {
                &self.targets[(i + 1) % 2].1
            };
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Menu blur pass"),
                color_attachments: &attachments,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.groups[i], &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_texture(
            frame.as_image_copy(),
            self.targets[0].0.as_image_copy(),
            wgpu::Extent3d {
                width: copy_size[0],
                height: copy_size[1],
                depth_or_array_layers: 1,
            },
        );
        self.cached_valid = true;
        self.cached_size = (copy_size[0], copy_size[1]);
        self.cached_radius = radius.round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn box_blur_shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("menu_blur.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
