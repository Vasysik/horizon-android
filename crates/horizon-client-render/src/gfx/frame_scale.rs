//! Scale the complete Android frame with the existing post-process copy pass.
use super::world_post;

pub(super) struct FrameScale {
    post: world_post::Post,
    format: wgpu::TextureFormat,
    target: Option<(wgpu::Texture, wgpu::BindGroup)>,
}

impl FrameScale {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            post: world_post::Post::new(device, format),
            format,
            target: None,
        }
    }

    pub(super) fn frame(&mut self, device: &wgpu::Device, size: [u32; 2]) -> wgpu::Texture {
        if self
            .target
            .as_ref()
            .is_none_or(|(texture, _)| [texture.width(), texture.height()] != size)
        {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Android client frame"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let group = self
                .post
                .bind(device, &texture.create_view(&Default::default()));
            self.target = Some((texture, group));
        }
        self.target.as_ref().expect("frame target").0.clone()
    }

    pub(super) fn present(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::Texture) {
        if let Some((_, group)) = &self.target {
            let view = output.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.format),
                ..Default::default()
            });
            self.post.finish_in_rect(
                encoder,
                &view,
                group,
                Some(winit::platform::android::native_content_rect([
                    output.width(),
                    output.height(),
                ])),
            );
        }
    }
}
