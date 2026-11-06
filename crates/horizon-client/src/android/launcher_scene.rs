//! The pixel sunset behind the launcher, from the desktop launcher's `launcher/src/scene.rs`
//! (the website's `HorizonScene`): the same seeds, draws and colour sums, so a window of the same
//! size shows the same picture. Only GPUI's painting is gone: [`Model::picture`] bakes the still
//! parts into one image, and what moves (stars, fireflies) is drawn by `launcher_glass.rs`.
//! [`Backdrop`] is the same picture blurred, for the glass plates.
#![allow(dead_code)]
use std::f64::consts::PI;

const SKY: [u32; 15] = [
    0x0a0919, 0x0f0c28, 0x151136, 0x1d1444, 0x281850, 0x361b5b, 0x481e66, 0x5d236e, 0x752873,
    0x902f76, 0xab3a75, 0xc64a71, 0xde5f6a, 0xef7c63, 0xf89d62,
];
const SKY_STEPS: usize = SKY.len() - 4;
/// The sky gradient's stop positions, `(i / SKY_STEPS)^(1 / 2.3)`: `sky_rgb` runs once per
/// texel of the baked sky, so they are worked out once, not per texel.
static SKY_STOPS: std::sync::LazyLock<[f64; SKY_STEPS + 1]> = std::sync::LazyLock::new(|| {
    std::array::from_fn(|i| (i as f64 / SKY_STEPS as f64).powf(1. / 2.3))
});
const SUN: [u32; 7] = [
    0xfff5d2, 0xffe4a0, 0xffca78, 0xffac5f, 0xff8c58, 0xfb6c62, 0xee5277,
];
const HALO: u32 = 0xff8d6a;
const FOG: u32 = 0xe0697c;
const RIM: u32 = 0xffb27c;
const CLOUD_HIGH: u32 = 0x5f2c7c;
const CLOUD_LOW: u32 = 0xc95985;
const CLOUD_TOP: u32 = 0xf3a6bd;
const FIREFLY: u32 = 0xffd98c;
const STAR_COOL: u32 = 0xd2c8ff;
const STAR_WARM: u32 = 0xfff3e4;
const MARGIN: i32 = 20;
/// The mockup's sun: right of centre, a little bigger and higher (`wideSun`), on wide windows.
const WIDE_SUN: (f64, f64, f64) = (0.72, 1.1, 0.1);
fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn smoothstep(a: f64, b: f64, v: f64) -> f64 {
    let t = clamp((v - a) / (b - a), 0., 1.);
    t * t * (3. - 2. * t)
}

/// JavaScript's `Math.round`: halves go up.
fn round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// The website's `mix`: each channel `a + (b - a) * t`, cut to an integer as its bit operations do.
fn mix(a: u32, b: u32, t: f64) -> u32 {
    [16, 8, 0].iter().fold(0, |out, &s| {
        let (x, y) = (((a >> s) & 255) as f64, ((b >> s) & 255) as f64);
        out | ((x + (y - x) * t) as i64 as u32 & 255) << s
    })
}

/// The website's `rng`: mulberry32.
struct Rng(i32);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let s = self.0;
        let mut t = (s ^ ((s as u32) >> 15) as i32).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ ((t as u32) >> 7) as i32).wrapping_mul(61 | t)) ^ t;
        ((t ^ ((t as u32) >> 14) as i32) as u32) as f64 / 4294967296.
    }
}

/// The website's `noise1d`: four octaves of smoothed value noise over 256 random lattice points,
/// kept as 32-bit floats like its `Float32Array`.
struct Noise([f32; 256]);

impl Noise {
    fn new(seed: i32) -> Noise {
        let mut rng = Rng(seed);
        Noise(std::array::from_fn(|_| rng.next() as f32))
    }

    fn at(&self, x: f64) -> f64 {
        let (mut value, mut amp, mut freq, mut norm) = (0., 0.5, 1., 0.);
        for _ in 0..4 {
            let p = x * freq;
            let i = p.floor();
            let s = p - i;
            let i = i as i64;
            let a = self.0[(i & 255) as usize] as f64;
            let b = self.0[((i + 1) & 255) as usize] as f64;
            value += (a + (b - a) * s * s * (3. - 2. * s)) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.1;
        }
        value / norm
    }
}

const TREES: [&[&str]; 3] = [
    &[
        "..###..", ".#####.", "#######", "#######", ".#####.", "...#...", "...#...",
    ],
    &[
        "..#..", "..#..", ".###.", "..#..", ".###.", "#####", "..#..", "..#..",
    ],
    &[".###.", "#####", "#####", "..#..", "..#.."],
];

struct Star {
    x: f64,
    y: f64,
    size: f64,
    b: f64,
    phase: f64,
    speed: f64,
    sparkle: bool,
    cool: bool,
}

struct Cloud {
    x0: f64,
    y: i32,
    w: i32,
    h: i32,
    /// 1 body, 2 the lit bottom row, 3 the top edge.
    mask: Vec<u8>,
    speed: f64,
    depth: f64,
    body: u32,
    rim_base: f64,
}

struct Ridge {
    top: Vec<i32>,
    depth: f64,
    color: u32,
    rim: u32,
    rim_strength: f64,
    /// Trees standing on it: their column and sprite.
    trees: Vec<(i32, usize)>,
}

struct Firefly {
    x: f64,
    y: f64,
    phase: f64,
    blink: f64,
}

/// One window size's scene, as the website's `buildScene` makes it. Lengths are texels (`px`
/// logical pixels square) except the stars', which are logical pixels.
pub struct Model {
    width: f64,
    px: f64,
    w: i32,
    h: i32,
    horizon: i32,
    fog_top: i32,
    fog_bottom: i32,
    sun: (i32, i32, i32),
    glow: Vec<f64>,
    stars: Vec<Star>,
    clouds: Vec<Cloud>,
    ridges: Vec<Ridge>,
    flies: Vec<Firefly>,
}

fn make_cloud(rand: &mut Rng, w: i32, h: i32) -> Vec<u8> {
    let mut mask = vec![0u8; (w * h).max(0) as usize];
    let mut lobe = |rand: &mut Rng, from: i32, to: i32| {
        let (mut left, mut right, mut y) = (from, to, h);
        while y > 0 && right - left > 3 {
            let mut k = y.min(1 + (rand.next() * 2.).floor() as i32);
            while k > 0 {
                y -= 1;
                for x in left..right {
                    mask[(y * w + x) as usize] = 1;
                }
                k -= 1;
            }
            let span = (right - left) as f64;
            left += 1f64.max(round(span * (0.06 + rand.next() * 0.2))) as i32;
            right -= 1f64.max(round(span * (0.06 + rand.next() * 0.2))) as i32;
        }
    };
    lobe(rand, 0, w);
    if w > 24 && rand.next() < 0.75 {
        let a = round(w as f64 * (0.1 + rand.next() * 0.4)) as i32;
        let to = w.min(a + round(w as f64 * (0.35 + rand.next() * 0.2)) as i32);
        lobe(rand, a, to);
    }
    for i in 0..(w * h) as usize {
        if mask[i] == 0 {
            continue;
        }
        if i >= ((h - 1) * w) as usize {
            mask[i] = 2;
        } else if i < w as usize || mask[i - w as usize] == 0 {
            mask[i] = 3;
        }
    }
    mask
}

impl Model {
    /// The scene for a window `width` × `height` logical pixels, its land `clamp(30 % of the
    /// height, 180, 320)` high as the website's default.
    pub fn new(width: f64, height: f64) -> Model {
        let land_height = clamp(height * 0.3, 180., 320.);
        let (sun_x, sun_scale, sun_rise) = if width >= 1024. {
            WIDE_SUN
        } else {
            (0.5, 1., 0.)
        };
        let px = clamp(round(width / 340.), 3., 6.);
        let w = (width / px).ceil() as i32;
        let h = (height / px).ceil() as i32;
        let land = land_height / px;
        let horizon = round(h as f64 - land * 0.55) as i32;
        let r = round(land * 0.4 * sun_scale) as i32;
        let sun = (
            round(w as f64 * sun_x) as i32,
            round(horizon as f64 - r as f64 * (0.45 + sun_rise)) as i32,
            r,
        );
        let fog_top = round(horizon as f64 - land * 0.18) as i32;
        let glow = (0..w)
            .map(|x| {
                let d = (x - sun.0) as f64 / (w as f64 * 0.3);
                (-d * d).exp()
            })
            .collect::<Vec<_>>();
        let sky_at = |y: f64| {
            let f = clamp(y / horizon as f64, 0., 1.).powf(2.3) * SKY_STEPS as f64;
            let i = f.floor() as usize;
            mix(SKY[i], SKY[(i + 1).min(SKY.len() - 1)], f - i as f64)
        };

        let mut rand = Rng(7);
        let sky_height = horizon as f64 * px;
        let star_count = round(width * sky_height / 5200.) as usize;
        let mut stars = Vec::with_capacity(star_count);
        for _ in 0..star_count {
            let x = rand.next() * width;
            let y = 2. + rand.next().powf(1.8) * sky_height * 0.75;
            let size = if rand.next() < 0.15 {
                1.8
            } else {
                1. + rand.next() * 0.4
            };
            let b = if rand.next() < 0.1 {
                0.8 + rand.next() * 0.2
            } else {
                0.25 + rand.next() * 0.4
            };
            stars.push(Star {
                x,
                y,
                size,
                b,
                phase: rand.next() * PI * 2.,
                speed: 0.5 + rand.next() * 2.,
                sparkle: rand.next() < 0.06,
                cool: rand.next() < 0.35,
            });
        }

        let mut clouds = Vec::new();
        let mut add_cloud = |rand: &mut Rng, y: f64, cw: f64, ch: f64, speed: f64, depth: f64| {
            let t = clamp(
                (y - horizon as f64 * 0.45) / (horizon as f64 * 0.55),
                0.,
                1.,
            );
            let x0 = rand.next() * (w as f64 + cw);
            let (cw, ch) = (round(cw) as i32, round(ch) as i32);
            clouds.push(Cloud {
                x0,
                y: round(y) as i32,
                w: cw,
                h: ch,
                mask: make_cloud(rand, cw, ch),
                speed,
                depth,
                body: mix(sky_at(y), mix(CLOUD_HIGH, CLOUD_LOW, t * t), 0.55),
                rim_base: 0.15 + t * 0.35,
            });
        };
        for _ in 0..3 {
            let cw = (120. + rand.next() * 140.) / px;
            let y = horizon as f64 - land * (0.45 + rand.next() * 0.25);
            let ch = cw * (0.2 + rand.next() * 0.06);
            let speed = 0.5 + rand.next() * 0.8;
            add_cloud(&mut rand, y, cw, ch, speed, 0.4);
        }
        for _ in 0..5 {
            let y = horizon as f64 - 5. - rand.next() * land * 0.4;
            let cw = w as f64 * (0.08 + rand.next() * 0.12);
            let ch = 3. + rand.next() * 3.;
            let speed = 1. + rand.next() * 1.6;
            add_cloud(&mut rand, y, cw, ch, speed, 0.8);
        }

        let cols = w + MARGIN * 2;
        #[allow(clippy::too_many_arguments)]
        let ridge = |seed: i32,
                     base: f64,
                     amp: f64,
                     block: i32,
                     feature: f64,
                     valley: f64,
                     depth: f64,
                     color: u32,
                     rim: u32,
                     rim_strength: f64| {
            let n = Noise::new(seed);
            let mut top = vec![0i32; cols as usize];
            let mut i = 0;
            while i < cols {
                let x = (i - MARGIN) as f64 + block as f64 / 2.;
                let d = (x - sun.0 as f64).abs() / (w as f64 * 0.5);
                let env = valley + (1. - valley) * (d * d * 1.8).min(1.);
                let v = clamp((n.at(x * px / feature) - 0.2) / 0.6, 0., 1.);
                let y = round((base - amp * env * v) / block as f64) as i32 * block;
                for k in 0..block {
                    if i + k < cols {
                        top[(i + k) as usize] = y;
                    }
                }
                i += block;
            }
            Ridge {
                top,
                depth,
                color,
                rim,
                rim_strength,
                trees: Vec::new(),
            }
        };
        let hz = horizon as f64;
        let mut ridges = vec![
            ridge(
                11,
                hz,
                land * 0.3,
                2,
                170.,
                0.2,
                0.25,
                0x3f1f5c,
                0xc45a7f,
                1.,
            ),
            ridge(
                23,
                hz + land * 0.22,
                land * 0.36,
                3,
                130.,
                0.35,
                0.55,
                0x25123e,
                0x7a3068,
                0.8,
            ),
            ridge(
                37,
                hz + land * 0.5,
                land * 0.38,
                4,
                100.,
                0.5,
                1.,
                0x100a1f,
                0x2c1640,
                0.6,
            ),
        ];
        let mut col = MARGIN + (rand.next() * 12.).floor() as i32;
        while col < cols - 8 {
            let sprite = (rand.next() * TREES.len() as f64).floor() as usize;
            ridges[2].trees.push((col, sprite));
            col += 14 + (rand.next() * 30.).floor() as i32;
        }

        let flies = (0..round(w as f64 / 36.) as usize)
            .map(|_| Firefly {
                x: rand.next() * w as f64,
                y: hz + land * (0.3 + rand.next() * 0.6),
                phase: rand.next() * PI * 2.,
                blink: 0.6 + rand.next() * 0.9,
            })
            .collect();

        Model {
            width,
            px,
            w,
            h,
            horizon,
            fog_top,
            fog_bottom: round(hz + land * 0.26) as i32,
            sun,
            glow,
            stars,
            clouds,
            ridges,
            flies,
        }
    }

    /// The static sky (`paintStaticSky`) at a point in logical pixels, as straight RGB in 0..=1:
    /// the gradient to the horizon, the sun's glow on it and the haze over the land.
    fn sky_rgb(&self, x: f64, y: f64) -> [f64; 3] {
        let horizon = self.horizon as f64 * self.px;
        let t = clamp(y / horizon, 0., 1.);
        let stops = &*SKY_STOPS;
        let stop = |i: usize| stops[i];
        let mut i = 0;
        while i + 1 < SKY_STEPS && t > stop(i + 1) {
            i += 1;
        }
        let f = clamp((t - stop(i)) / (stop(i + 1) - stop(i)), 0., 1.);
        let mut c = lerp(rgb(SKY[i]), rgb(SKY[i + 1]), f);
        // The glow: a radial gradient squeezed to an ellipse `width * 0.6` wide.
        let ry = horizon * 0.32;
        let u = (x - self.sun.0 as f64 * self.px) * ry / (self.width * 0.6);
        let d = (u * u + (y - horizon).powi(2)).sqrt() / ry;
        if d < 1. {
            let stops = [
                (0., SKY[14], 0.9),
                (0.3, SKY[13], 0.55),
                (0.65, SKY[12], 0.18),
                (1., SKY[12], 0.),
            ];
            c = over(c, gradient(&stops, d));
        }
        let fog_top = self.fog_top as f64 * self.px;
        if y >= fog_top {
            let t = clamp((y - fog_top) / (horizon - fog_top), 0., 1.);
            c = over(c, (rgb(FOG), 0.3 * t));
        }
        c
    }

    /// The sun's halo at a point, its brightest at `a` (the website breathes it between 0.45 and
    /// 0.5).
    fn halo(&self, x: f64, y: f64, a: f64) -> ([f64; 3], f64) {
        let (sx, sy) = (self.sun.0 as f64 * self.px, self.sun.1 as f64 * self.px);
        let r = self.sun.2 as f64 * 2.4 * self.px;
        let d = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt() / r;
        gradient(
            &[(0., HALO, a), (0.45, HALO, a * 0.3), (1., HALO, 0.)],
            d.min(1.),
        )
    }

    /// The pixel layer's moving parts at `time` (clouds, then the sun), as runs of one colour:
    /// `(x, y, length, colour)` in texels, fog already laid over the ones in its band.
    fn runs(&self, time: f64, view: (f64, f64)) -> Vec<(i32, i32, i32, u32)> {
        let mut runs = Vec::new();
        let push = |runs: &mut Vec<(i32, i32, i32, u32)>, x: i32, y: i32, c: u32| {
            let c = self.fogged(x, y, c);
            match runs.last_mut() {
                Some((rx, ry, len, rc)) if *ry == y && *rx + *len == x && *rc == c => *len += 1,
                _ => runs.push((x, y, 1, c)),
            }
        };
        for c in &self.clouds {
            let span = (self.w + c.w + 16) as f64;
            let shifted = (c.x0 + c.speed * time - view.0 * c.depth * 3.) % span;
            let left = round((shifted + span) % span) as i32 - c.w - 8;
            for cy in 0..c.h {
                let y = c.y + cy;
                if y < 0 || y >= self.h {
                    continue;
                }
                for cx in 0..c.w {
                    let m = c.mask[(cy * c.w + cx) as usize];
                    let x = left + cx;
                    if m == 0 || x < 0 || x >= self.w {
                        continue;
                    }
                    let color = match m {
                        2 => mix(c.body, RIM, c.rim_base + 0.5 * self.glow[x as usize]),
                        3 => mix(c.body, CLOUD_TOP, 0.14),
                        _ => c.body,
                    };
                    push(&mut runs, x, y, color);
                }
            }
        }
        // The sun, in slices that drift down (`drawSun`).
        let (sx, sy, r) = (
            self.sun.0 as f64 + round(-view.0 * 1.5),
            self.sun.1 as f64,
            self.sun.2 as f64,
        );
        let stripe = (r / 5.).round().max(4.);
        let phase = time * 2.2;
        let first = (sy - r).ceil().max(0.) as i32;
        let last = ((sy + r).floor() as i32).min(self.h - 1);
        for y in first..=last {
            let dy = y as f64 + 0.5 - sy;
            let half = (r * r - dy * dy).sqrt();
            if half.is_nan() || half <= 0. {
                continue;
            }
            let v = (dy + r) / (2. * r);
            if v > 0.38 {
                let gap = 1. + (v - 0.38) / (1. - 0.38) * stripe * 0.6;
                if (y as f64 - sy - phase).rem_euclid(stripe) < gap {
                    continue;
                }
            }
            let f = v * (SUN.len() - 1) as f64;
            let i = f.floor() as usize;
            let color = mix(SUN[i], SUN[(i + 1).min(SUN.len() - 1)], f - i as f64);
            let from = round(sx - half).max(0.) as i32;
            let to = (round(sx + half) as i32).min(self.w);
            for x in from..to {
                push(&mut runs, x, y, color);
            }
        }
        runs
    }

    /// `c` at texel (`x`, `y`) with the fog the first ridge lays over everything drawn so far.
    fn fogged(&self, x: i32, y: i32, c: u32) -> u32 {
        if y < self.fog_top.max(0) || y >= self.fog_bottom.min(self.h) {
            return c;
        }
        let t = if y < self.horizon {
            (y - self.fog_top) as f64 / (self.horizon - self.fog_top) as f64
        } else {
            1. - (y - self.horizon) as f64 / (self.fog_bottom - self.horizon) as f64
        };
        let g = self.glow[(x.clamp(0, self.w - 1)) as usize];
        mix(c, FOG, t * t * (0.16 + 0.3 * g))
    }

    /// Ridge `n` as texels (straight RGBA, 0 where empty), all its columns (`w + 2 × MARGIN`)
    /// from its highest texel to the bottom of the window plus `extra` rows, and the first row's
    /// texel `y`.
    fn ridge_texels(&self, n: usize, extra: i32) -> (Vec<[u8; 4]>, i32, i32, i32) {
        let ridge = &self.ridges[n];
        let cols = ridge.top.len() as i32;
        let tallest = ridge
            .trees
            .iter()
            .map(|&(_, s)| TREES[s].len() as i32)
            .max()
            .unwrap_or(0);
        let top_row = ridge.top.iter().copied().min().unwrap_or(0) - tallest;
        let rows = self.h + extra - top_row;
        let mut out = vec![[0u8; 4]; (cols * rows).max(0) as usize];
        let glow = |i: i32| self.glow[(i - MARGIN).clamp(0, self.w - 1) as usize];
        let mut put = |x: i32, y: i32, c: u32| {
            if x >= 0 && x < cols && y >= top_row && y < top_row + rows {
                let [r, g, b] = [16, 8, 0].map(|s| ((c >> s) & 255) as u8);
                out[((y - top_row) * cols + x) as usize] = [r, g, b, 255];
            }
        };
        for i in 0..cols {
            let top = ridge.top[i as usize];
            let left = ridge.top[(i - 1).max(0) as usize];
            let right = ridge.top[(i + 1).min(cols - 1) as usize];
            let rim = mix(ridge.color, ridge.rim, glow(i) * ridge.rim_strength);
            for y in top.max(top_row)..top_row + rows {
                let c = if y == top || y < left || y < right {
                    rim
                } else {
                    ridge.color
                };
                let c = if n == 0 {
                    self.fogged(i - MARGIN, y, c)
                } else {
                    c
                };
                put(i, y, c);
            }
        }
        for &(col, sprite) in &ridge.trees {
            let ground = ridge.top[col as usize];
            let rows_ = TREES[sprite];
            let trunk = rows_[rows_.len() - 1].find('#').unwrap_or(0) as i32;
            for (ty, row) in rows_.iter().enumerate() {
                let y = ground - rows_.len() as i32 + ty as i32;
                for (tx, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        put(col - trunk + tx as i32, y, ridge.color);
                    }
                }
            }
        }
        (out, cols, rows, top_row)
    }
}

fn rgb(c: u32) -> [f64; 3] {
    [16, 8, 0].map(|s| ((c >> s) & 255) as f64 / 255.)
}

fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

fn over(under: [f64; 3], (c, a): ([f64; 3], f64)) -> [f64; 3] {
    [0, 1, 2].map(|i| c[i] * a + under[i] * (1. - a))
}

/// A canvas gradient's colour and opacity at `d` between its stops (interpolated unpremultiplied).
fn gradient(stops: &[(f64, u32, f64)], d: f64) -> ([f64; 3], f64) {
    let mut i = 0;
    while i + 2 < stops.len() && d > stops[i + 1].0 {
        i += 1;
    }
    let (p0, c0, a0) = stops[i];
    let (p1, c1, a1) = stops[i + 1];
    let t = clamp((d - p0) / (p1 - p0), 0., 1.);
    (lerp(rgb(c0), rgb(c1), t), a0 + (a1 - a0) * t)
}

fn byte(v: f64) -> u8 {
    (clamp(v, 0., 1.) * 255.).round() as u8
}

/// The scene blurred as `.hzl-glass` and `.hzl-float` see it through their backdrop filters
/// (blur, then `saturate(140%)`, and `brightness(0.62)` for the float): one colour per texel at
/// the scene's first frame, without the stars (a 24 px blur leaves nothing of them).
///
/// The accent's tint is not in the blurred texels: `color_blend` keeps the backdrop's luminance
/// and blur is linear, so tinting after the blur gives (up to gamut clipping) what tinting
/// before it did. A new accent is then a cheap [`set_tint`](Self::set_tint), not a new blur.
pub struct Backdrop {
    pub w: usize,
    pub h: usize,
    /// Logical pixels per texel.
    pub px: f64,
    /// The blurred scene before tint, saturation and brightness: one colour per texel.
    glass: Vec<[f32; 3]>,
    float: Vec<[f32; 3]>,
    /// The accent's tint, laid over a blurred colour as it is read.
    tint: Option<[f64; 3]>,
}

impl Backdrop {
    pub fn new(m: &Model, blur: f64, fast: bool) -> Backdrop {
        let (w, h) = (m.w as usize, m.h as usize);
        let mut base = vec![[0f64; 3]; w * h];
        for (i, c) in base.iter_mut().enumerate() {
            let (x, y) = (((i % w) as f64 + 0.5) * m.px, ((i / w) as f64 + 0.5) * m.px);
            *c = over(m.sky_rgb(x, y), m.halo(x, y, 0.45));
        }
        let mut put = |x: i32, y: i32, c: u32| {
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                base[y as usize * w + x as usize] = rgb(c);
            }
        };
        for (x, y, len, c) in m.runs(0., (0., 0.)) {
            for dx in 0..len {
                put(x + dx, y, c);
            }
        }
        for n in 0..m.ridges.len() {
            let (texels, cols, rows, top) = m.ridge_texels(n, 0);
            for row in 0..rows {
                for col in 0..cols {
                    let [r, g, b, a] = texels[(row * cols + col) as usize];
                    if a > 0 {
                        put(
                            col - MARGIN,
                            top + row,
                            (r as u32) << 16 | (g as u32) << 8 | b as u32,
                        );
                    }
                }
            }
        }
        let blurred = |sigma: f64| -> Vec<[f32; 3]> {
            let texels = if fast {
                box_blur_rgb(&base, w, h, sigma / m.px)
            } else {
                gaussian_blur_rgb(&base, w, h, sigma / m.px)
            };
            texels.into_iter().map(|c| c.map(|v| v as f32)).collect()
        };
        Backdrop {
            w,
            h,
            px: m.px,
            glass: blurred(blur),
            float: blurred(if blur <= 0. { 0. } else { blur + 24. }),
            tint: None,
        }
    }

    /// Lays `tint` (none for the violet) over the backdrop from now on; true if it changed.
    pub fn set_tint(&mut self, tint: Option<u32>) -> bool {
        let tint = tint.map(rgb);
        let changed = self.tint != tint;
        self.tint = tint;
        changed
    }

    /// The blurred colour at a point in logical pixels (bilinear between texel centres), tinted,
    /// saturated and dimmed as the filters do.
    pub fn at(&self, float: bool, x: f64, y: f64) -> [f32; 3] {
        let grid = if float { &self.float } else { &self.glass };
        let fx = clamp(x / self.px - 0.5, 0., (self.w - 1) as f64);
        let fy = clamp(y / self.px - 0.5, 0., (self.h - 1) as f64);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let px = |x: usize, y: usize| grid[y * self.w + x];
        let l = |a: [f32; 3], b: [f32; 3], t: f32| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
        let blurred = l(
            l(px(x0, y0), px(x1, y0), tx),
            l(px(x0, y1), px(x1, y1), tx),
            ty,
        );
        let mut c = blurred.map(f64::from);
        if let Some(t) = self.tint {
            c = color_blend(c, t);
        }
        let brightness = if float { 0.62 } else { 1. };
        saturate(c, 1.4).map(|v| (clamp(v, 0., 1.) * brightness) as f32)
    }
}

/// CSS's `color` blend of `source` over `backdrop` (0 to 1, sRGB): the source's hue and chroma at
/// the backdrop's luminance (`SetLum(Cs, Lum(Cb))` in Compositing and Blending Level 1).
fn color_blend(backdrop: [f64; 3], source: [f64; 3]) -> [f64; 3] {
    let lum = |c: [f64; 3]| 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    let d = lum(backdrop) - lum(source);
    let c = source.map(|v| v + d);
    let l = lum(c);
    let (n, x) = (c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
    let c = if n < 0. {
        c.map(|v| l + (v - l) * l / (l - n))
    } else {
        c
    };
    if x > 1. {
        c.map(|v| l + (v - l) * (1. - l) / (x - l))
    } else {
        c
    }
}

/// `c` under the tint, if any.
fn tinted_u32(c: u32, tint: Option<u32>) -> u32 {
    let Some(t) = tint else {
        return c;
    };
    let [r, g, b] = color_blend(rgb(c), rgb(t)).map(|v| byte(v) as u32);
    r << 16 | g << 8 | b
}

/// A Gaussian blur of standard deviation `sigma` texels, its edges held (CSS blurs a backdrop
/// with its edge pixels repeated).
fn gaussian_blur_rgb(src: &[[f64; 3]], w: usize, h: usize, sigma: f64) -> Vec<[f64; 3]> {
    if sigma <= 0. {
        return src.to_vec();
    }
    let radius = (sigma * 3.).ceil() as i64;
    let kernel: Vec<f64> = (-radius..=radius)
        .map(|i| (-(i * i) as f64 / (2. * sigma * sigma)).exp())
        .collect();
    let sum: f64 = kernel.iter().sum();
    let pass = |src: &[[f64; 3]], horizontal: bool| -> Vec<[f64; 3]> {
        let mut out = vec![[0.; 3]; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.; 3];
                for (k, wgt) in kernel.iter().enumerate() {
                    let d = k as i64 - radius;
                    let (sx, sy) = if horizontal {
                        ((x as i64 + d).clamp(0, w as i64 - 1) as usize, y)
                    } else {
                        (x, (y as i64 + d).clamp(0, h as i64 - 1) as usize)
                    };
                    let c = src[sy * w + sx];
                    for i in 0..3 {
                        acc[i] += c[i] * wgt;
                    }
                }
                out[y * w + x] = acc.map(|v| v / sum);
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

/// A Gaussian blur of standard deviation `sigma` texels, as three box blurs (their widths add
/// up to the Gaussian's variance; the shape is within a few percent of it), its edges held as
/// CSS holds a backdrop's. A pass costs the same few operations a texel whatever `sigma` is:
/// the Gaussian kernel it replaces cost `6 * sigma` taps a texel and pass.
fn box_blur_rgb(src: &[[f64; 3]], w: usize, h: usize, sigma: f64) -> Vec<[f64; 3]> {
    if sigma <= 0. {
        return src.to_vec();
    }
    let mut a = src.to_vec();
    let mut b = vec![[0.; 3]; w * h];
    for r in box_radii(sigma) {
        box_pass(&a, &mut b, w, h, r, true);
        box_pass(&b, &mut a, w, h, r, false);
    }
    a
}

/// The radii of three boxes (a box of radius `r` is `2r + 1` wide) whose variances add up to
/// `sigma`'s: Kutskir's "boxes for gauss".
fn box_radii(sigma: f64) -> [usize; 3] {
    let n = 3.;
    let ideal = (12. * sigma * sigma / n + 1.).sqrt();
    let mut lower = ideal.floor() as i64;
    if lower % 2 == 0 {
        lower -= 1;
    }
    let lower = lower.max(1);
    let upper = lower + 2;
    let l = lower as f64;
    // How many of the three take the lower width.
    let m = ((12. * sigma * sigma - n * l * l - 4. * n * l - 3. * n) / (-4. * l - 4.)).round();
    std::array::from_fn(|i| {
        let width = if (i as f64) < m { lower } else { upper };
        ((width - 1) / 2) as usize
    })
}

/// One box blur of radius `r` along the rows (`horizontal`) or the columns, edges held.
fn box_pass(
    src: &[[f64; 3]],
    out: &mut [[f64; 3]],
    w: usize,
    h: usize,
    r: usize,
    horizontal: bool,
) {
    let (lines, len, stride, step) = if horizontal {
        (h, w, w, 1)
    } else {
        (w, h, 1, w)
    };
    let r = r as i64;
    let scale = 1. / (2 * r + 1) as f64;
    for line in 0..lines {
        let base = line * stride;
        let at = |i: i64| src[base + i.clamp(0, len as i64 - 1) as usize * step];
        let mut sum = [0.; 3];
        for k in -r..=r {
            let c = at(k);
            for ch in 0..3 {
                sum[ch] += c[ch];
            }
        }
        for i in 0..len as i64 {
            out[base + i as usize * step] = sum.map(|v| v * scale);
            let (add, sub) = (at(i + 1 + r), at(i - r));
            for ch in 0..3 {
                sum[ch] += add[ch] - sub[ch];
            }
        }
    }
}

/// CSS `saturate(s)` (Filter Effects' matrix).
fn saturate([r, g, b]: [f64; 3], s: f64) -> [f64; 3] {
    [
        (0.213 + 0.787 * s) * r + (0.715 - 0.715 * s) * g + (0.072 - 0.072 * s) * b,
        (0.213 - 0.213 * s) * r + (0.715 + 0.285 * s) * g + (0.072 - 0.072 * s) * b,
        (0.213 - 0.213 * s) * r + (0.715 - 0.715 * s) * g + (0.072 + 0.928 * s) * b,
    ]
}

/// One still frame in RGBA, a texel per [`Model::px`] logical pixels, and what moves over it.
impl Model {
    /// Logical pixels per texel.
    pub fn px(&self) -> f64 {
        self.px
    }
    /// The picture's size in texels.
    pub fn texels(&self) -> (u32, u32) {
        (self.w as u32, self.h as u32)
    }

    /// The sky, the sun's halo, the clouds and sun at their first frame and the three ridges,
    /// tinted with `tint` (none for the violet).
    pub fn picture(&self, tint: Option<u32>) -> Vec<u8> {
        let (w, h) = (self.w as usize, self.h as usize);
        let tinted = tint.map(rgb);
        let mut base = vec![[0f64; 3]; w * h];
        for (i, c) in base.iter_mut().enumerate() {
            let (x, y) = (
                ((i % w) as f64 + 0.5) * self.px,
                ((i / w) as f64 + 0.5) * self.px,
            );
            *c = over(self.sky_rgb(x, y), self.halo(x, y, 0.5));
        }
        let mut put = |x: i32, y: i32, c: u32| {
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                base[y as usize * w + x as usize] = rgb(c);
            }
        };
        for (x, y, len, c) in self.runs(0., (0., 0.)) {
            for dx in 0..len {
                put(x + dx, y, c);
            }
        }
        for n in 0..self.ridges.len() {
            let (texels, cols, rows, top) = self.ridge_texels(n, 0);
            for row in 0..rows {
                for col in 0..cols {
                    let [r, g, b, a] = texels[(row * cols + col) as usize];
                    if a > 0 {
                        put(
                            col - MARGIN,
                            top + row,
                            (r as u32) << 16 | (g as u32) << 8 | b as u32,
                        );
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(w * h * 4);
        for c in base {
            let c = tinted.map_or(c, |t| color_blend(c, t));
            out.extend_from_slice(&[byte(c[0]), byte(c[1]), byte(c[2]), 255]);
        }
        out
    }

    /// The stars at `time`, as `(x, y, size, colour, alpha)` in logical pixels.
    pub fn twinkle(&self, time: f64, tint: Option<u32>) -> Vec<(f32, f32, f32, u32, f32)> {
        let sky_height = self.horizon as f64 * self.px;
        let (cool, warm) = (tinted_u32(STAR_COOL, tint), tinted_u32(STAR_WARM, tint));
        self.stars
            .iter()
            .filter_map(|s| {
                let twinkle = 0.5 + 0.5 * (time * s.speed + s.phase).sin();
                let alpha =
                    s.b * (1. - smoothstep(0.4, 0.82, s.y / sky_height)) * (0.35 + 0.65 * twinkle);
                (alpha >= 0.04).then_some((
                    (s.x - s.size / 2.) as f32,
                    (s.y - s.size / 2.) as f32,
                    s.size as f32,
                    if s.cool { cool } else { warm },
                    alpha as f32,
                ))
            })
            .collect()
    }

    /// The fireflies at `time`, as `(x, y, alpha)` in texels, and their colour.
    pub fn fireflies(&self, time: f64, tint: Option<u32>) -> (Vec<(i32, i32, f32)>, u32) {
        (
            self.flies
                .iter()
                .filter_map(|f| {
                    let a = (time * f.blink + f.phase).sin().max(0.).powi(3);
                    let x = round(f.x + (time * 0.35 + f.phase).sin() * 5.) as i32;
                    let y = round(f.y + (time * 0.5 + f.phase * 1.7).sin() * 2.5) as i32;
                    (a >= 0.05 && x >= 1 && x < self.w - 1 && y >= 1 && y < self.h - 1)
                        .then_some((x, y, a as f32))
                })
                .collect(),
            tinted_u32(FIREFLY, tint),
        )
    }
}

/// The player's accent as the tint the scene takes: `color-mix(in oklab, accent 58%, #767676)`.
/// None for the violet the picture was drawn in.
pub fn tint_for(accent: u32) -> Option<u32> {
    (accent != crate::launcher_tokens::ACCENT)
        .then(|| crate::launcher_tokens::mix_oklab(accent, 0x767676, 0.58))
}
