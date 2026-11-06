//! Textured 3D cuboids with a depth buffer. No frontend or GPU API dependency;
//! the same camera and RGBA output work in native UI, tests and future bridges.
use super::Model;
use image::{Rgba, RgbaImage};

#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -25.0,
            pitch: -8.0,
            zoom: 1.0,
        }
    }
}
impl Camera {
    pub fn rotate(&mut self, x: f32, y: f32) {
        self.yaw = (self.yaw + x).rem_euclid(360.0);
        self.pitch = (self.pitch + y).clamp(-65.0, 65.0);
    }
    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() {
            self.zoom = (self.zoom * factor).clamp(0.5, 2.0);
        }
    }
}

/// Original neutral fitting model used only when no skin is selected.
pub fn mannequin() -> RgbaImage {
    RgbaImage::from_fn(64, 64, |x, y| {
        if y >= 32 {
            Rgba([0, 0, 0, 0])
        } else {
            let shade = if (x / 4 + y / 4) % 2 == 0 { 165 } else { 140 };
            Rgba([shade, shade, shade + 10, 255])
        }
    })
}

struct Raster {
    image: RgbaImage,
    depth: Vec<f32>,
    camera: Camera,
}
impl Raster {
    fn project(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.camera.yaw.to_radians().sin_cos();
        let (x, z) = (x * c + z * s, -x * s + z * c);
        let (s, c) = self.camera.pitch.to_radians().sin_cos();
        let (y, z) = ((y - 16.0) * c - z * s, (y - 16.0) * s + z * c);
        let scale = self.image.height() as f32 / 39.0 * self.camera.zoom;
        let perspective = 100.0 / (100.0 + z);
        [
            self.image.width() as f32 * 0.5 + x * scale * perspective,
            self.image.height() as f32 * 0.5 - y * scale * perspective,
            z,
        ]
    }
    fn triangle(
        &mut self,
        points: [[f32; 3]; 3],
        uv: [[f32; 2]; 3],
        texture: &RgbaImage,
        light: f32,
    ) {
        let edge = |a: [f32; 3], b: [f32; 3], x: f32, y: f32| {
            (x - a[0]) * (b[1] - a[1]) - (y - a[1]) * (b[0] - a[0])
        };
        let area = edge(points[0], points[1], points[2][0], points[2][1]);
        if area.abs() < 0.0001 {
            return;
        }
        let minx = points
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let maxx = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, self.image.width() as f32) as u32;
        let miny = points
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let maxy = points
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, self.image.height() as f32) as u32;
        for y in miny..maxy {
            for x in minx..maxx {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let weights = [
                    edge(points[1], points[2], px, py) / area,
                    edge(points[2], points[0], px, py) / area,
                    edge(points[0], points[1], px, py) / area,
                ];
                if weights.iter().any(|w| *w < -0.00001) {
                    continue;
                }
                let depth = (0..3).map(|i| points[i][2] * weights[i]).sum::<f32>();
                let index = (y * self.image.width() + x) as usize;
                if depth >= self.depth[index] {
                    continue;
                }
                let u = (0..3)
                    .map(|i| uv[i][0] * weights[i])
                    .sum::<f32>()
                    .floor()
                    .clamp(0.0, texture.width() as f32 - 1.0) as u32;
                let v = (0..3)
                    .map(|i| uv[i][1] * weights[i])
                    .sum::<f32>()
                    .floor()
                    .clamp(0.0, texture.height() as f32 - 1.0) as u32;
                let mut color = *texture.get_pixel(u, v);
                if color[3] < 128 {
                    continue;
                }
                for channel in 0..3 {
                    color[channel] = (color[channel] as f32 * light) as u8;
                }
                color[3] = 255;
                self.image.put_pixel(x, y, color);
                self.depth[index] = depth;
            }
        }
    }
    fn cuboid(
        &mut self,
        texture: &RgbaImage,
        center: [f32; 3],
        dimensions: [f32; 3],
        uv: [f32; 2],
        inflate: f32,
        reverse_depth: bool,
    ) {
        let [w, h, d] = dimensions;
        let [cx, cy, cz] = center;
        let (x0, x1) = (cx - w / 2.0 - inflate, cx + w / 2.0 + inflate);
        let (y0, y1) = (cy - h / 2.0 - inflate, cy + h / 2.0 + inflate);
        let (z0, z1) = (cz - d / 2.0 - inflate, cz + d / 2.0 + inflate);
        let [u, v] = uv;
        let mut faces = [
            (
                [[x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0]],
                [u + d, v + d, w, h],
                1.0,
            ),
            (
                [[x1, y1, z0], [x1, y1, z1], [x1, y0, z1], [x1, y0, z0]],
                [u + d + w, v + d, d, h],
                0.78,
            ),
            (
                [[x1, y1, z1], [x0, y1, z1], [x0, y0, z1], [x1, y0, z1]],
                [u + d + w + d, v + d, w, h],
                0.9,
            ),
            (
                [[x0, y1, z1], [x0, y1, z0], [x0, y0, z0], [x0, y0, z1]],
                [u, v + d, d, h],
                0.82,
            ),
            (
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                [u + d, v, w, d],
                1.0,
            ),
            (
                [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
                [u + d + w, v, w, d],
                0.65,
            ),
        ];
        if reverse_depth {
            for (points, _, _) in &mut faces {
                for point in points {
                    point[0] = 2.0 * cx - point[0];
                    point[2] = 2.0 * cz - point[2];
                }
            }
        }
        for (points, [u, v, w, h], light) in faces {
            let points = points.map(|p| self.project(p));
            let uv = [[u, v], [u + w, v], [u + w, v + h], [u, v + h]];
            for [a, b, c] in [[0, 1, 2], [0, 2, 3]] {
                self.triangle(
                    [points[a], points[b], points[c]],
                    [uv[a], uv[b], uv[c]],
                    texture,
                    light,
                );
            }
        }
    }
}

/// Bounded pixel output for interactive previews. Both skin layers and the cape
/// are actual geometry; rotating shows their backs, sides and top faces.
pub fn render(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    model: Model,
    camera: Camera,
    size: [u32; 2],
) -> RgbaImage {
    let [width, height] = size.map(|v| v.clamp(16, 1024));
    let mut raster = Raster {
        image: RgbaImage::new(width, height),
        depth: vec![f32::INFINITY; (width * height) as usize],
        camera,
    };
    let normalized = horizon_core::player_texture::process_legacy_skin(
        skin.width(),
        skin.height(),
        skin.as_raw(),
    )
    .and_then(|bytes| RgbaImage::from_raw(64, 64, bytes));
    let skin = normalized.as_ref().unwrap_or(skin);
    let cape_image = cape
        .and_then(|image| {
            horizon_core::player_texture::process_cape(
                image.width(),
                image.height(),
                image.as_raw(),
            )
        })
        .and_then(|bytes| RgbaImage::from_raw(64, 64, bytes));
    let cape = cape_image.as_ref();
    let arm = if model == Model::Slim { 3.0 } else { 4.0 };
    let modern = skin.height() == 64;
    for (center, dimensions, uv, outer) in [
        (
            [0.0, 28.0, 0.0],
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            Some([32.0, 0.0]),
        ),
        (
            [0.0, 18.0, 0.0],
            [8.0, 12.0, 4.0],
            [16.0, 16.0],
            modern.then_some([16.0, 32.0]),
        ),
        (
            [-4.0 - arm / 2.0, 18.0, 0.0],
            [arm, 12.0, 4.0],
            [40.0, 16.0],
            modern.then_some([40.0, 32.0]),
        ),
        (
            [4.0 + arm / 2.0, 18.0, 0.0],
            [arm, 12.0, 4.0],
            if modern { [32.0, 48.0] } else { [40.0, 16.0] },
            modern.then_some([48.0, 48.0]),
        ),
        (
            [-2.0, 6.0, 0.0],
            [4.0, 12.0, 4.0],
            [0.0, 16.0],
            modern.then_some([0.0, 32.0]),
        ),
        (
            [2.0, 6.0, 0.0],
            [4.0, 12.0, 4.0],
            if modern { [16.0, 48.0] } else { [0.0, 16.0] },
            modern.then_some([0.0, 48.0]),
        ),
    ] {
        raster.cuboid(skin, center, dimensions, uv, 0.0, false);
        if let Some(uv) = outer {
            raster.cuboid(
                skin,
                center,
                dimensions,
                uv,
                if dimensions[1] == 8.0 { 0.5 } else { 0.25 },
                false,
            );
        }
    }
    if let Some(cape) = cape {
        raster.cuboid(
            cape,
            [0.0, 16.0, 3.0],
            [10.0, 16.0, 1.0],
            [0.0, 0.0],
            0.0,
            true,
        );
    }
    raster.image
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_model_and_cape_change_the_actual_geometry() {
        let skin = mannequin();
        let front = render(&skin, None, Model::Classic, Camera::default(), [160, 200]);
        let mut camera = Camera::default();
        camera.rotate(160.0, 20.0);
        camera.zoom(1.1);
        let back = render(&skin, None, Model::Slim, camera, [160, 200]);
        assert_ne!(front, back);
        assert!(front.pixels().filter(|p| p[3] > 0).count() > 1000);
        let cape = super::super::library::cape_design(0);
        assert_ne!(
            back,
            render(&skin, Some(&cape), Model::Slim, camera, [160, 200])
        );
    }
}
