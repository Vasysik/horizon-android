//! The desktop launcher's glass look on the Android launcher's custom drawing: the pixel sunset,
//! frosted plates and the widgets of `launcher/src/glass_ui.rs`, at the same sizes and with the
//! same tokens (`launcher_tokens.rs`), made a little taller where a finger has to hit them.
//!
//! Nothing here knows Android: it draws over `Ui`, so the preview example can rasterise it on a
//! computer.
use crate::launcher_scene::{Backdrop, Model, tint_for};
use crate::launcher_tokens::{self as tokens, Glass, fill, ink, radius};
use horizon_client::{
    hud::Rect,
    ui_theme::{Face, Ui, color::hex},
};
use std::{collections::HashMap, time::Instant};

/// The tappable places a frame drew, in dp.
pub type Targets<A> = Vec<([f32; 4], A)>;
/// A decoded picture: width, height and RGBA.
pub type Pixels = std::sync::Arc<(u32, u32, Vec<u8>)>;

/// The icon sheet: `assets/icons.png` (Hugeicons' free set, the ones `horizon-theme` carries,
/// rasterised white on black at 96 px) and its names, in cell order, eight to a row.
const ICON_SHEET: &[u8] = include_bytes!("assets/icons.png");
const ICON_NAMES: &str = include_str!("assets/icons.txt");
const ICON_CELL: usize = 96;
const ICON_COLUMNS: usize = 8;

/// White at `a` (the look's `ink` and `fill` steps) in the HUD's colour encoding.
pub fn white(a: f32) -> u32 {
    hex(0xffffff, (a.clamp(0., 1.) * 255.).round() as u8)
}
/// `0xRRGGBB` at `a`.
pub fn tone(c: u32, a: f32) -> u32 {
    hex(c, (a.clamp(0., 1.) * 255.).round() as u8)
}

struct Sheet {
    width: usize,
    alpha: Vec<u8>,
    names: HashMap<&'static str, usize>,
}
impl Sheet {
    fn load() -> Option<Sheet> {
        let mut reader = png::Decoder::new(std::io::Cursor::new(ICON_SHEET))
            .read_info()
            .ok()?;
        let mut alpha = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut alpha).ok()?;
        alpha.truncate(info.buffer_size());
        Some(Sheet {
            width: info.width as usize,
            alpha,
            names: ICON_NAMES
                .lines()
                .enumerate()
                .map(|(i, n)| (n, i))
                .collect(),
        })
    }
    /// The icon as `side` × `side` RGBA, white with the sheet's coverage, box filtered from the
    /// 96 px cell: the atlas samples nearest, so each size is its own picture.
    fn icon(&self, cell: usize, side: u32) -> (u32, u32, Vec<u8>) {
        let (cx, cy) = (
            cell % ICON_COLUMNS * ICON_CELL,
            cell / ICON_COLUMNS * ICON_CELL,
        );
        let n = side as usize;
        let mut out = Vec::with_capacity(n * n * 4);
        for y in 0..n {
            let (y0, y1) = (y * ICON_CELL / n, ((y + 1) * ICON_CELL / n).max(y * ICON_CELL / n + 1));
            for x in 0..n {
                let (x0, x1) = (x * ICON_CELL / n, ((x + 1) * ICON_CELL / n).max(x * ICON_CELL / n + 1));
                let mut sum = 0u32;
                for sy in y0..y1 {
                    for sx in x0..x1 {
                        sum += u32::from(self.alpha[(cy + sy) * self.width + cx + sx]);
                    }
                }
                let a = (sum / ((y1 - y0) * (x1 - x0)) as u32) as u8;
                out.extend_from_slice(&[255, 255, 255, a]);
            }
        }
        (side, side, out)
    }
}

/// The scene and the plates' pictures for one screen size and accent.
struct Baked {
    model: Model,
    tint: Option<u32>,
    /// Texels across and down.
    size: (u32, u32),
    scene: Vec<u8>,
    glass: Vec<u8>,
    float: Vec<u8>,
}
impl Baked {
    fn new(width: f32, height: f32, look: &Glass) -> Baked {
        let tint = tint_for(look.accent);
        let model = Model::new(width as f64, height as f64);
        let size = model.texels();
        let mut backdrop = Backdrop::new(&model, f64::from(look.blur), true);
        backdrop.set_tint(tint);
        let plate = [16, 8, 0].map(|s| ((look.plate >> s) & 255) as f32 / 255.);
        let a = look.plate_alpha;
        let px = model.px();
        let frosted = |float: bool| -> Vec<u8> {
            let mut out = Vec::with_capacity((size.0 * size.1 * 4) as usize);
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let c = backdrop.at(
                        float,
                        (f64::from(x) + 0.5) * px,
                        (f64::from(y) + 0.5) * px,
                    );
                    let mixed =
                        [0, 1, 2].map(|i| ((plate[i] * a + c[i] * (1. - a)) * 255.).round() as u8);
                    out.extend_from_slice(&[mixed[0], mixed[1], mixed[2], 255]);
                }
            }
            out
        };
        Baked {
            tint,
            size,
            scene: model.picture(tint),
            glass: frosted(false),
            float: frosted(true),
            model,
        }
    }
}

/// A plate's kind: `.hzl-glass` (panels, the sidebar, dialogs) or `.hzl-float` (menus, sheets:
/// twice the blur, darker).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Plate {
    Glass,
    Float,
}

/// What every page draws with: the accent's colours, the scene and the icons.
pub struct Art {
    pub look: Glass,
    /// Performance: no scene or glass, only the plates' colour.
    pub plain: bool,
    /// Hold the stars and fireflies still.
    pub still: bool,
    baked: Option<Baked>,
    key: (u32, u32, u32),
    /// Bumped when a picture changes, so the atlas's copy is asked for again.
    generation: u64,
    sheet: Option<Sheet>,
    start: Instant,
}
impl Default for Art {
    fn default() -> Self {
        Self {
            look: Glass::default_look(),
            plain: false,
            still: false,
            baked: None,
            key: (0, 0, 0),
            generation: 0,
            sheet: Sheet::load(),
            start: Instant::now(),
        }
    }
}

fn hash(tag: u64, a: u64, b: u64) -> u64 {
    tag << 56 ^ a << 24 ^ b
}

impl Art {
    /// Bakes the scene for this screen and `accent`, when either changed. Call with the whole
    /// screen's size before anything draws.
    pub fn prepare(&mut self, width: f32, height: f32, accent: u32) {
        let key = (width.round() as u32, height.round() as u32, accent);
        if self.key != key || (self.baked.is_none() && !self.plain) {
            self.key = key;
            self.look = Glass::new(accent, tokens::TINT, tokens::BLUR);
            self.generation += 1;
            self.baked = (!self.plain).then(|| Baked::new(width, height, &self.look));
        }
    }

    /// The screen behind everything.
    pub fn background(&self, ui: &mut Ui) {
        let (w, h) = (ui.batch.width, ui.batch.height);
        let atlas = ui.atlas;
        ui.batch.fill(atlas, 0.0, 0.0, w, h, hex(tokens::PAGE, 255));
        let Some(baked) = &self.baked else {
            return;
        };
        let (tw, th) = baked.size;
        if let Some(rect) = ui.image(hash(1, self.generation, 0), || {
            Some((tw, th, baked.scene.clone()))
        }) {
            ui.batch.rect_uv(0.0, 0.0, w, h, rect, 0xffff_ffff);
        }
        if self.still {
            return;
        }
        let time = self.start.elapsed().as_secs_f64();
        for (x, y, size, c, a) in baked.model.twinkle(time, baked.tint) {
            ui.rounded(x, y, size.max(1.5), size.max(1.5), 1.0, tone(c, a));
        }
        let px = baked.model.px() as f32;
        let (flies, c) = baked.model.fireflies(time, baked.tint);
        for (x, y, a) in flies {
            ui.rounded(x as f32 * px, y as f32 * px, px, px, 0.0, tone(c, a));
        }
    }

    /// A frosted plate with `r` round corners: the scene's blurred picture cut out in rows, the
    /// rows' ends at the coverage the circle gives them, so the edge stays smooth.
    pub fn plate(&self, ui: &mut Ui, rect: [f32; 4], r: f32, kind: Plate) {
        let [x, y, w, h] = rect;
        let Some(baked) = &self.baked else {
            ui.rounded(x, y, w, h, r, hex(self.look.plate, 255));
            return;
        };
        let (tw, th) = baked.size;
        let float = kind == Plate::Float;
        let Some(image) = ui.image(hash(if float { 3 } else { 2 }, self.generation, 0), || {
            Some((
                tw,
                th,
                if float {
                    baked.float.clone()
                } else {
                    baked.glass.clone()
                },
            ))
        }) else {
            ui.rounded(x, y, w, h, r, hex(self.look.plate, 255));
            return;
        };
        let d = ui.density;
        let s = ui.batch.scale;
        let (sw, sh) = (ui.batch.width * s, ui.batch.height * s);
        let (x0, y0) = ((x * d).round(), (y * d).round());
        let (x1, y1) = (((x + w) * d).round(), ((y + h) * d).round());
        let rp = (r * d).round().min((x1 - x0) / 2.).min((y1 - y0) / 2.).max(0.);
        let mut part = |a: f32, b: f32, c: f32, e: f32, alpha: f32| {
            if c <= a || e <= b {
                return;
            }
            let u = |v: f32| image.u0 + (image.u1 - image.u0) * (v / sw);
            let v = |v: f32| image.v0 + (image.v1 - image.v0) * (v / sh);
            ui.batch.quad(
                [[a / s, b / s], [c / s, b / s], [c / s, e / s], [a / s, e / s]],
                [[u(a), v(b)], [u(c), v(b)], [u(c), v(e)], [u(a), v(e)]],
                white(alpha),
            );
        };
        part(x0, y0 + rp, x1, y1 - rp, 1.);
        for i in 0..rp as u32 {
            let dy = rp - i as f32 - 0.5;
            let inset = rp - (rp * rp - dy * dy).max(0.).sqrt();
            let whole = inset.ceil();
            let edge = 1. - (whole - inset);
            for (top, bottom) in [
                (y0 + i as f32, y0 + i as f32 + 1.),
                (y1 - i as f32 - 1., y1 - i as f32),
            ] {
                part(x0 + whole, top, x1 - whole, bottom, 1.);
                if whole >= 1. {
                    part(x0 + whole - 1., top, x0 + whole, bottom, edge);
                    part(x1 - whole, top, x1 - whole + 1., bottom, edge);
                }
            }
        }
    }

    /// The icon called `name` at `size` dp, in `color`.
    pub fn icon(&self, ui: &mut Ui, name: &str, x: f32, y: f32, size: f32, color: u32) {
        let Some(sheet) = &self.sheet else {
            return;
        };
        let Some(&cell) = sheet.names.get(name) else {
            return;
        };
        let side = (size * ui.density).round().max(1.) as u32;
        if let Some(rect) = ui.image(hash(4, cell as u64, u64::from(side)), || {
            Some(sheet.icon(cell, side))
        }) {
            let s = ui.batch.scale;
            let d = ui.density;
            ui.batch.rect_uv(
                (x * d).round() / s,
                (y * d).round() / s,
                side as f32 / s,
                side as f32 / s,
                rect,
                color,
            );
        }
    }

    /// Horizon's mark: two bars above the line and their reflection under it (`docs/media/brand`).
    pub fn mark(&self, ui: &mut Ui, x: f32, y: f32, size: f32) {
        let u = size / 16.;
        for (bx, by, bw, bh, c) in [
            (4., 3., 2., 4., white(0.93)),
            (10., 3., 2., 4., white(0.93)),
            (4., 8., 2., 4., white(0.33)),
            (10., 8., 2., 4., white(0.33)),
            (0., 7., 16., 1., hex(self.look.accent_soft, 255)),
        ] {
            ui.rounded(x + bx * u, y + by * u, bw * u, bh * u, 0.0, c);
        }
    }

    /// A round tile with its name's first letter, for what has no picture yet.
    pub fn tile(&self, ui: &mut Ui, x: f32, y: f32, size: f32, hue: u32, label: &str) {
        ui.rounded(x, y, size, size, radius::ROW, hex(hue, 255));
        let letter: String = label.chars().next().unwrap_or('?').to_uppercase().collect();
        let font = size * 0.46;
        let w = ui.measure(Face::SemiBold, font, &letter);
        ui.text(
            Face::SemiBold,
            font,
            x + (size - w) / 2.,
            y + (size - font * 1.45) / 2.,
            white(ink::FULL),
            &letter,
        );
    }

    /// A button's look: the mockup's `solid`, `button` and quiet ones.
    pub fn button<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        label: &str,
        action: A,
        kind: Kind,
    ) {
        self.icon_label(ui, targets, rect, None, label, action, kind);
    }

    /// A button with an icon before its label.
    #[allow(clippy::too_many_arguments)]
    pub fn icon_label<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        icon: Option<&str>,
        label: &str,
        action: A,
        kind: Kind,
    ) {
        let [x, y, w, h] = rect;
        let (back, ink_color, face) = match kind {
            Kind::Primary => (
                hex(self.look.accent, 255),
                hex(self.look.on_accent, 255),
                Face::SemiBold,
            ),
            Kind::Secondary => (white(fill::BUTTON), white(ink::FULL), Face::Medium),
            Kind::Ghost => (0, white(ink::BODY), Face::Medium),
            Kind::Danger => (
                tone(tokens::DANGER_HOVER, 0.12),
                hex(tokens::DANGER, 255),
                Face::Medium,
            ),
            Kind::Off => (white(0.04), white(ink::OFF), Face::Medium),
        };
        ui.rounded(x, y, w, h, radius::CONTROL, back);
        let size = 14.;
        let icon_w = if icon.is_some() { 24. } else { 0. };
        let room = (w - 16. - icon_w).max(8.);
        let label = ui.fit_text(face, size, label, room);
        let width = ui.measure(face, size, &label) + icon_w;
        let left = x + (w - width) / 2.;
        if let Some(name) = icon {
            self.icon(ui, name, left, y + (h - 18.) / 2., 18., ink_color);
        }
        ui.text(
            face,
            size,
            left + icon_w,
            y + (h - 20.) / 2.,
            ink_color,
            &label,
        );
        if !matches!(kind, Kind::Off) {
            targets.push((rect, action));
        }
    }

    /// A square button with one icon: `ICON_BUTTON`.
    pub fn icon_button<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        name: &str,
        action: A,
        active: bool,
    ) {
        let [x, y, w, h] = rect;
        if active {
            ui.rounded(x, y, w, h, radius::CONTROL, white(fill::ACTIVE));
        }
        let size = (w.min(h) * 0.5).clamp(16., 24.);
        self.icon(
            ui,
            name,
            x + (w - size) / 2.,
            y + (h - size) / 2.,
            size,
            white(if active { ink::FULL } else { ink::MUTED }),
        );
        targets.push((rect, action));
    }

    /// A tab (`Tabs`, size `md`): 36 high round at 10, white at 12 % when open; an icon before the
    /// label and a count after it. Returns its width.
    #[allow(clippy::too_many_arguments)]
    pub fn tab<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        x: f32,
        y: f32,
        h: f32,
        icon: Option<&str>,
        label: &str,
        count: Option<usize>,
        open: bool,
        action: A,
    ) -> f32 {
        let size = 14.;
        let face = if open { Face::Medium } else { Face::Regular };
        let count = count.filter(|n| *n > 0).map(|n| n.to_string());
        let width = 24.
            + ui.measure(face, size, label)
            + if icon.is_some() { 22. } else { 0. }
            + count
                .as_ref()
                .map_or(0., |c| ui.measure(Face::Regular, 12., c) + 8.);
        if open {
            ui.rounded(x, y, width, h, radius::CONTROL, white(fill::ACTIVE));
        }
        let text = white(if open { ink::FULL } else { ink::BODY });
        let mut pen = x + 12.;
        if let Some(name) = icon {
            self.icon(ui, name, pen, y + (h - 16.) / 2., 16., text);
            pen += 22.;
        }
        pen += ui.text(face, size, pen, y + (h - 20.) / 2., text, label);
        if let Some(c) = count {
            ui.text(
                Face::Regular,
                12.,
                pen + 8.,
                y + (h - 17.) / 2.,
                white(ink::DIM),
                &c,
            );
        }
        targets.push(([x, y, width, h], action));
        width
    }

    /// A switch: the knob's track in the accent when on (`.hzl-switch`).
    pub fn toggle<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        on: bool,
        action: A,
    ) {
        let [x, y, w, h] = rect;
        let (tw, th) = (42., 24.);
        let (tx, ty) = (x + w - tw, y + (h - th) / 2.);
        ui.rounded(
            tx,
            ty,
            tw,
            th,
            th / 2.,
            if on {
                hex(self.look.accent, 255)
            } else {
                white(fill::BUTTON_HOVER)
            },
        );
        ui.rounded(
            tx + if on { tw - 21. } else { 3. },
            ty + 3.,
            18.,
            18.,
            9.,
            white(ink::FULL),
        );
        targets.push((rect, action));
    }

    /// A segmented control: `(label, action, chosen)`, sharing `rect`'s width.
    pub fn segments<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        options: &[(&str, A, bool)],
    ) {
        let [x, y, w, h] = rect;
        ui.rounded(x, y, w, h, radius::LARGE, white(0.04));
        let each = (w - 8. - 4. * (options.len() as f32 - 1.)) / options.len() as f32;
        for (i, (label, action, chosen)) in options.iter().enumerate() {
            let ox = x + 4. + i as f32 * (each + 4.);
            if *chosen {
                ui.rounded(
                    ox,
                    y + 4.,
                    each,
                    h - 8.,
                    radius::CONTROL,
                    white(fill::BUTTON_HOVER),
                );
            }
            let face = if *chosen { Face::Medium } else { Face::Regular };
            let label = ui.fit_text(face, 14., label, each - 8.);
            let tw = ui.measure(face, 14., &label);
            ui.text(
                face,
                14.,
                ox + (each - tw) / 2.,
                y + (h - 20.) / 2.,
                white(if *chosen { ink::FULL } else { ink::MUTED }),
                &label,
            );
            targets.push(([ox, y + 4., each, h - 8.], *action));
        }
    }

    /// A text field: white at 7 %, 11 % with the caret in it, the placeholder at 40 %.
    #[allow(clippy::too_many_arguments)]
    pub fn field<A: Copy>(
        &self,
        ui: &mut Ui,
        targets: &mut Targets<A>,
        rect: [f32; 4],
        icon: Option<&str>,
        value: &str,
        placeholder: &str,
        focused: bool,
        action: A,
    ) {
        let [x, y, w, h] = rect;
        ui.rounded(
            x,
            y,
            w,
            h,
            radius::ROW,
            white(if focused { fill::FIELD_FOCUS } else { fill::FIELD }),
        );
        let mut left = x + 12.;
        if let Some(name) = icon {
            self.icon(ui, name, left, y + (h - 18.) / 2., 18., white(ink::DIM));
            left += 26.;
        }
        let room = (x + w - left - 12.).max(8.);
        let empty = value.is_empty();
        let shown = ui.fit_text(Face::Regular, 14., if empty { placeholder } else { value }, room);
        let width = ui.text(
            Face::Regular,
            14.,
            left,
            y + (h - 20.) / 2.,
            white(if empty { ink::PLACEHOLDER } else { ink::FULL }),
            &shown,
        );
        if focused
            && std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .is_ok_and(|t| t.as_millis() % 1000 < 500)
        {
            ui.rounded(
                left + if empty { 0. } else { width } + 1.,
                y + (h - 18.) / 2.,
                1.5,
                18.,
                0.,
                hex(self.look.accent_soft, 255),
            );
        }
        targets.push((rect, action));
    }

    /// A progress bar in `accent-soft`.
    pub fn bar(&self, ui: &mut Ui, rect: [f32; 4], value: f32) {
        let [x, y, w, h] = rect;
        ui.rounded(x, y, w, h, h / 2., white(fill::SELECTED));
        let done = (w * value.clamp(0., 1.)).max(0.);
        if done > 0. {
            ui.rounded(x, y, done.max(h), h, h / 2., hex(self.look.accent_soft, 255));
        }
    }
}

/// A button's look.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The accent, for the one main action.
    Primary,
    Secondary,
    Ghost,
    /// Removing something.
    Danger,
    /// Not available now: drawn, not tappable.
    Off,
}

thread_local! {
    static LOOK: std::cell::Cell<Glass> = std::cell::Cell::new(Glass::default_look());
}
/// The accent the older dialogs' buttons take, from the frame's [`Art`].
pub fn remember_look(look: Glass) {
    LOOK.set(look);
}

/// A button for code without an [`Art`] at hand: a label, in the accent's fill when `accent`.
pub fn plain_button<A: Copy>(
    ui: &mut Ui,
    targets: &mut Targets<A>,
    rect: [f32; 4],
    label: &str,
    action: A,
    accent: bool,
) {
    let [x, y, w, h] = rect;
    let look = LOOK.get();
    ui.rounded(
        x,
        y,
        w,
        h,
        radius::CONTROL,
        if accent {
            hex(look.accent, 255)
        } else {
            white(fill::BUTTON)
        },
    );
    let face = if accent { Face::SemiBold } else { Face::Medium };
    let size = if w < 80. { 13. } else { 14. };
    let label = ui.fit_text(face, size, label, (w - 16.).max(8.));
    let width = ui.measure(face, size, &label);
    ui.text(
        face,
        size,
        x + (w - width) / 2.,
        y + (h - size * 1.45) / 2.,
        if accent {
            hex(look.on_accent, 255)
        } else {
            white(ink::FULL)
        },
        &label,
    );
    targets.push((rect, action));
}

/// A status dot.
pub fn dot(ui: &mut Ui, x: f32, y: f32, tone_color: u32) {
    ui.rounded(x, y, 7., 7., 3.5, hex(tone_color, 255));
}

/// Text cut to `w` and drawn; returns the text's width.
#[allow(clippy::too_many_arguments)]
pub fn fitted(
    ui: &mut Ui,
    face: Face,
    size: f32,
    x: f32,
    y: f32,
    w: f32,
    color: u32,
    text: &str,
) -> f32 {
    let text = ui.fit_text(face, size, text, w.max(8.));
    ui.text(face, size, x, y, color, &text)
}

/// `text` broken at words to fit `w`.
pub fn wrapped(ui: &Ui, face: Face, size: f32, text: &str, w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && ui.measure(face, size, &candidate) > w {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// Draws `text` over `w`, line by line; returns the height it took.
#[allow(clippy::too_many_arguments)]
pub fn paragraph(
    ui: &mut Ui,
    face: Face,
    size: f32,
    x: f32,
    y: f32,
    w: f32,
    color: u32,
    text: &str,
) -> f32 {
    let line = (size * 1.5).round();
    let lines = wrapped(ui, face, size, text, w);
    for (i, l) in lines.iter().enumerate() {
        ui.text(face, size, x, y + i as f32 * line, color, l);
    }
    lines.len() as f32 * line
}

/// Cuts what was drawn since `start` (an index into the batch's vertices) to `rect` in dp: the
/// HUD has no scissor, and a list has to scroll under the plates' edges rather than pop.
pub fn clip(ui: &mut Ui, start: usize, rect: [f32; 4]) {
    let k = ui.density / ui.batch.scale;
    let (left, top) = (rect[0] * k, rect[1] * k);
    let (right, bottom) = ((rect[0] + rect[2]) * k, (rect[1] + rect[3]) * k);
    for quad in ui.batch.vertices[start..].chunks_exact_mut(6) {
        let mut min = quad[0];
        let mut max = quad[0];
        let (mut min_y, mut max_y) = (quad[0], quad[0]);
        for v in quad.iter() {
            if v.pos[0] < min.pos[0] {
                min = *v;
            }
            if v.pos[0] > max.pos[0] {
                max = *v;
            }
            if v.pos[1] < min_y.pos[1] {
                min_y = *v;
            }
            if v.pos[1] > max_y.pos[1] {
                max_y = *v;
            }
        }
        if max.pos[0] <= left
            || min.pos[0] >= right
            || max_y.pos[1] <= top
            || min_y.pos[1] >= bottom
        {
            for v in quad.iter_mut() {
                v.pos = [0., 0.];
            }
            continue;
        }
        let along = |a: f32, b: f32, ua: f32, ub: f32, p: f32| {
            if b > a {
                ua + (ub - ua) * ((p - a) / (b - a))
            } else {
                ua
            }
        };
        for v in quad.iter_mut() {
            let x = v.pos[0].clamp(left, right);
            let y = v.pos[1].clamp(top, bottom);
            if x != v.pos[0] {
                v.uv[0] = along(min.pos[0], max.pos[0], min.uv[0], max.uv[0], x);
            }
            if y != v.pos[1] {
                v.uv[1] = along(min_y.pos[1], max_y.pos[1], min_y.uv[1], max_y.uv[1], y);
            }
            v.pos = [x, y];
        }
    }
}

/// The picture `key`, from a PNG's bytes already decoded by [`decode_png`], scaled to fit.
pub fn picture(ui: &mut Ui, key: u64, rgba: &Pixels, rect: [f32; 4]) {
    let [x, y, w, h] = rect;
    if let Some(image) = ui.image(hash(5, key, 0), || {
        Some((rgba.0, rgba.1, rgba.2.clone()))
    }) {
        draw_rect(ui, image, x, y, w, h);
    }
}

fn draw_rect(ui: &mut Ui, image: Rect, x: f32, y: f32, w: f32, h: f32) {
    let k = ui.density / ui.batch.scale;
    ui.batch
        .rect_uv(x * k, y * k, w * k, h * k, image, 0xffff_ffff);
}

/// A PNG as RGBA, box filtered down so its longer side is at most `max`. None for other formats.
pub fn decode_png(bytes: &[u8], max: u32) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut raw = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut raw).ok()?;
    let channels = info.color_type.samples();
    let (w, h) = (info.width as usize, info.height as usize);
    let scale = (w.max(h) as f32 / max.max(1) as f32).max(1.);
    let (ow, oh) = (
        ((w as f32 / scale).round() as usize).max(1),
        ((h as f32 / scale).round() as usize).max(1),
    );
    let mut out = Vec::with_capacity(ow * oh * 4);
    for y in 0..oh {
        let (y0, y1) = (y * h / oh, ((y + 1) * h / oh).max(y * h / oh + 1));
        for x in 0..ow {
            let (x0, x1) = (x * w / ow, ((x + 1) * w / ow).max(x * w / ow + 1));
            let mut sum = [0u32; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let p = &raw[(sy * w + sx) * channels..];
                    let (c, a) = match channels {
                        1 => ([p[0], p[0], p[0]], 255),
                        2 => ([p[0], p[0], p[0]], p[1]),
                        3 => ([p[0], p[1], p[2]], 255),
                        _ => ([p[0], p[1], p[2]], p[3]),
                    };
                    sum[0] += u32::from(c[0]);
                    sum[1] += u32::from(c[1]);
                    sum[2] += u32::from(c[2]);
                    sum[3] += u32::from(a);
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            out.extend(sum.map(|v| (v / n) as u8));
        }
    }
    Some((ow as u32, oh as u32, out))
}
