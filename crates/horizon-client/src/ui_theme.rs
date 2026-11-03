//! The design system's widgets drawn in the game (`crates/horizon-theme`:
//! palette, type scale, radii, control sizes, motion and the bundled fonts;
//! `docs/design-system.md`). Every menu is vanilla's (`ui`); what is left
//! here draws Horizon's own overlays that vanilla has no screen for: the
//! on-screen keyboard a controller types with (`osk`), plus the
//! `horizon.txt` preferences.
//!
//! Build a [`Ui`] over the frame's `HudBatch` with [`Ui::new`], lay out in dp
//! (the launcher's logical pixels) and give every interactive widget a stable
//! `id`. Clicks run the same layout with [`Ui::dry`] and read [`Ui::hit`]: one
//! function draws and hit-tests, so they never disagree.
use crate::hud::{HudAtlas, HudBatch, Rect};
use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont as _};
use std::{cell::RefCell, collections::HashMap, path::Path};

/// A Horizon-only preference from `horizon.txt` beside `options.txt`
/// (`key:value` lines), so vanilla's options file stays exactly as vanilla
/// writes it.
pub fn preference(dir: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("horizon.txt")).ok()?;
    text.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key).then(|| v.trim().to_owned())
    })
}
/// Set (or with None remove) one preference, keeping the others.
pub(crate) fn set_preference(dir: &Path, key: &str, value: Option<&str>) {
    let text = std::fs::read_to_string(dir.join("horizon.txt")).unwrap_or_default();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| l.split_once(':').is_none_or(|(k, _)| k.trim() != key))
        .map(str::to_owned)
        .collect();
    if let Some(v) = value {
        lines.push(format!("{key}:{v}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    let _ = std::fs::write(dir.join("horizon.txt"), out);
}

/// The dark palette (`horizon_theme::DARK`) in the HUD's colour encoding, plus the
/// translucent plates the overlays draw over the game.
pub mod color {
    use horizon_theme::DARK as P;
    pub const fn hex(c: u32, a: u8) -> u32 {
        (a as u32) << 24 | (c & 0xff) << 16 | (c >> 8 & 0xff) << 8 | (c >> 16 & 0xff)
    }
    /// The page, under every veil.
    pub const BG: u32 = hex(P.page, 255);
    /// Hover plates, secondary buttons, fields, the segmented track: the
    /// dark palette's soft plate, as ink over the veil so the plates read the
    /// same over any backdrop (over the page colour they are `plate_soft`).
    pub const RAISED: u32 = hex(P.ink, 20);
    /// Pressed plates and a switch that is off (`plate`, the same way).
    pub const HOVER: u32 = hex(P.ink, 36);
    /// One-pixel rules between rows, slider tracks.
    pub const BORDER: u32 = hex(P.rule, 255);
    /// Text and selection (`ink`).
    pub const TEXT: u32 = hex(P.ink, 255);
    /// Values and secondary text (`ink_soft`).
    pub const MUTED: u32 = hex(P.ink_soft, 255);
    pub const ACCENT: u32 = hex(P.accent, 255);
    pub const ON_ACCENT: u32 = hex(P.on_accent, 255);
    pub fn with_alpha(c: u32, a: f32) -> u32 {
        let base = (c >> 24) as f32;
        (c & 0x00ff_ffff) | ((base * a.clamp(0.0, 1.0)).round() as u32) << 24
    }
}

/// The launcher's type scale and shapes (`horizon_theme`), in dp.
pub(crate) const SMALL: f32 = horizon_theme::CAPTION.px;
pub(crate) const BODY: f32 = horizon_theme::LABEL.px;
/// The boot screen's wordmark (`DISPLAY`) and console lines (`MICRO`).
pub(crate) const DISPLAY: f32 = horizon_theme::DISPLAY.px;
pub(crate) const MICRO: f32 = horizon_theme::MICRO.px;
/// Fields; plates (hover rows, selection); pills (buttons, segments).
pub(crate) const RADIUS: f32 = horizon_theme::RADIUS;
pub(crate) const RADIUS_PLATE: f32 = horizon_theme::RADIUS_PLATE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Regular,
    Medium,
    /// The wordmark and page titles (the design system's 500).
    SemiBold,
    /// Counts, timestamps and the boot screen's console lines.
    Mono,
}

/// Vector icons used by the Material You 3 Android launcher navigation rail,
/// top bar, and progress controls.
static ANDROID_DPI_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static UI_SCALE_PERCENT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

pub fn set_android_dpi(dpi: f32) {
    if dpi.is_finite() && dpi >= 1.0 {
        ANDROID_DPI_BITS.store(
            dpi.clamp(1.0, 5.0).to_bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

pub fn set_ui_scale_percent(percent: u32) {
    let clamped = if percent == 0 {
        100
    } else {
        percent.clamp(75, 165)
    };
    UI_SCALE_PERCENT.store(clamped, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    HorizonLogo,
    ChevronDown,
    Play,
    Mods,
    Packs,
    Shaders,
    Installed,
    Settings,
    Pause,
    Close,
    Search,
    Check,
    Monitor,
    Gear,
    Flask,
    Terminal,
    MicrosoftGrid,
    Gamepad,
    ArrowLeft,
    MenuEquals,
    Undo,
    Redo,
    Plus,
    Crosshair,
    Pencil,
    Copy,
}

struct Fonts {
    faces: [FontRef<'static>; 4],
}
impl Fonts {
    fn load() -> Self {
        let f = |b: &'static [u8]| FontRef::try_from_slice(b).expect("bundled font");
        Self {
            faces: [
                f(horizon_theme::fonts::GEIST_REGULAR),
                f(horizon_theme::fonts::GEIST_MEDIUM),
                f(horizon_theme::fonts::GEIST_SEMIBOLD),
                f(horizon_theme::fonts::GEIST_MONO),
            ],
        }
    }
    fn face(&self, f: Face) -> &FontRef<'static> {
        &self.faces[f as usize]
    }
    /// The ab_glyph scale for an em size in pixels.
    fn scale(&self, f: Face, px: f32) -> PxScale {
        let font = self.face(f);
        let upem = font.units_per_em().unwrap_or(1000.0);
        PxScale::from(px * font.height_unscaled() / upem)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Glyph(Face, u16, u16),
    /// A quarter disc (`0`) or a quarter ring of `(radius_px, thickness_px)`.
    Corner(u16, u16),
    /// A supersampled vector icon at this pixel size.
    Vector(Icon, u16),
}
#[derive(Clone, Copy)]
struct Cached {
    rect: Option<Rect>,
    /// Offset of the bitmap from the pen (glyphs) in pixels.
    left: f32,
    top: f32,
}
/// Glyphs and corner masks rasterized into the atlas region `horizon/ui-text`.
struct Cache {
    fonts: Fonts,
    entries: HashMap<Key, Cached>,
    images: HashMap<String, Rect>,
    shelf: (u32, u32, u32),
    pending: Vec<crate::font::Patch>,
    /// The atlas the entries were uploaded to (a new atlas starts over).
    atlas: usize,
}
thread_local! {
    static CACHE: RefCell<Option<Cache>> = const { RefCell::new(None) };
}
/// Reloaded atlases can occupy the same address with empty run-time slots.
pub fn reset_atlas() {
    CACHE.with(|cache| *cache.borrow_mut() = None);
}
fn with_cache<R>(atlas: &HudAtlas, f: impl FnOnce(&mut Cache) -> R) -> R {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let id = atlas as *const HudAtlas as usize;
        let cache = c.get_or_insert_with(|| Cache {
            fonts: Fonts::load(),
            entries: HashMap::new(),
            images: HashMap::new(),
            shelf: (0, 0, 0),
            pending: Vec::new(),
            atlas: id,
        });
        if cache.atlas != id {
            cache.atlas = id;
            cache.entries.clear();
            cache.images.clear();
            cache.shelf = (0, 0, 0);
            cache.pending.clear();
        }
        f(cache)
    })
}

fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-6 {
        ((px - ax) * dx + (py - ay) * dy) / len2
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    ((px - (ax + t * dx)).powi(2) + (py - (ay + t * dy)).powi(2)).sqrt()
}

fn icon_hit(icon: Icon, u: f32, v: f32) -> bool {
    match icon {
        Icon::HorizonLogo => {
            let left_bar = seg_dist(u, v, 0.22, 0.16, 0.22, 0.84) <= 0.095;
            let right_bar = seg_dist(u, v, 0.78, 0.16, 0.78, 0.84) <= 0.095;
            let bridge = seg_dist(u, v, 0.22, 0.56, 0.78, 0.42) <= 0.088;
            left_bar || right_bar || bridge
        }
        Icon::ChevronDown => {
            seg_dist(u, v, 0.24, 0.38, 0.50, 0.64) <= 0.075
                || seg_dist(u, v, 0.50, 0.64, 0.76, 0.38) <= 0.075
        }
        Icon::Play => {
            // Rounded right-pointing play triangle.
            let x0 = 0.26;
            let x1 = 0.80;
            if u < x0 - 0.04 || u > x1 + 0.04 {
                return false;
            }
            let t = ((u - x0) / (x1 - x0)).clamp(0.0, 1.0);
            let half_h = (1.0 - t) * 0.31 + 0.025;
            (v - 0.50).abs() <= half_h
        }
        Icon::Mods => {
            // 4-lobe puzzle / clover grid mark matching the screenshot's Mods icon.
            let centers = [(0.33, 0.33), (0.67, 0.33), (0.33, 0.67), (0.67, 0.67)];
            let in_lobe = centers.iter().any(|&(cx, cy)| {
                let dx = (u - cx).abs();
                let dy = (v - cy).abs();
                let r = 0.07;
                let qx = (dx - 0.075).max(0.0);
                let qy = (dy - 0.075).max(0.0);
                (qx * qx + qy * qy).sqrt() <= r
            });
            let center_bridge = (u - 0.50).abs() <= 0.09 && (v - 0.50).abs() <= 0.09;
            let hole = ((u - 0.50).powi(2) + (v - 0.50).powi(2)).sqrt() <= 0.065;
            (in_lobe || center_bridge) && !hole
        }
        Icon::Packs => {
            // Layered isometric diamond/package stack matching the screenshot's Packs icon.
            let top_diamond = ((u - 0.50).abs() * 0.62 + (v - 0.34).abs()).abs() <= 0.22
                && ((u - 0.50).abs() * 0.62 + (v - 0.34).abs()).abs() >= 0.11;
            let top_fill = (u - 0.50).abs() * 0.65 + (v - 0.33).abs() <= 0.19;
            let mid_layer = seg_dist(u, v, 0.18, 0.53, 0.50, 0.70) <= 0.055
                || seg_dist(u, v, 0.50, 0.70, 0.82, 0.53) <= 0.055;
            let bot_layer = seg_dist(u, v, 0.18, 0.69, 0.50, 0.86) <= 0.055
                || seg_dist(u, v, 0.50, 0.86, 0.82, 0.69) <= 0.055;
            top_diamond || top_fill || mid_layer || bot_layer
        }
        Icon::Shaders => {
            // Sunburst / 8-ray sparkle icon matching the screenshot's Shaders icon.
            let (dx, dy) = (u - 0.50, v - 0.50);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= 0.15 {
                return true;
            }
            let dirs: [(f32, f32); 8] = [
                (1.0, 0.0),
                (-1.0, 0.0),
                (0.0, 1.0),
                (0.0, -1.0),
                (0.7071, 0.7071),
                (-0.7071, 0.7071),
                (0.7071, -0.7071),
                (-0.7071, -0.7071),
            ];
            dirs.iter().any(|&(rx, ry)| {
                seg_dist(
                    u,
                    v,
                    0.50 + rx * 0.24,
                    0.50 + ry * 0.24,
                    0.50 + rx * 0.39,
                    0.50 + ry * 0.39,
                ) <= 0.048
            })
        }
        Icon::Installed => {
            // Folder outline/fill matching the screenshot's Installed icon.
            let in_tab = (0.14..=0.45).contains(&u) && (0.20..=0.34).contains(&v);
            let in_body = (0.14..=0.86).contains(&u) && (0.30..=0.80).contains(&v);
            let in_inner = (0.22..=0.78).contains(&u) && (0.40..=0.72).contains(&v);
            (in_tab || in_body) && !in_inner
        }
        Icon::Settings => {
            // Horizontal tuning sliders with circular knobs matching the screenshot's Settings icon.
            let track1 = seg_dist(u, v, 0.16, 0.33, 0.84, 0.33) <= 0.048;
            let track2 = seg_dist(u, v, 0.16, 0.67, 0.84, 0.67) <= 0.048;
            let knob1 = ((u - 0.36).powi(2) + (v - 0.33).powi(2)).sqrt() <= 0.125;
            let knob1_hole = ((u - 0.36).powi(2) + (v - 0.33).powi(2)).sqrt() <= 0.05;
            let knob2 = ((u - 0.64).powi(2) + (v - 0.67).powi(2)).sqrt() <= 0.125;
            let knob2_hole = ((u - 0.64).powi(2) + (v - 0.67).powi(2)).sqrt() <= 0.05;
            ((track1 || knob1) && !knob1_hole) || ((track2 || knob2) && !knob2_hole)
        }
        Icon::Pause => {
            // Two vertical pause bars ||
            seg_dist(u, v, 0.34, 0.24, 0.34, 0.76) <= 0.068
                || seg_dist(u, v, 0.66, 0.24, 0.66, 0.76) <= 0.068
        }
        Icon::Close => {
            // Crisp X cross
            seg_dist(u, v, 0.27, 0.27, 0.73, 0.73) <= 0.062
                || seg_dist(u, v, 0.73, 0.27, 0.27, 0.73) <= 0.062
        }
        Icon::Search => {
            let d = ((u - 0.43).powi(2) + (v - 0.43).powi(2)).sqrt();
            let ring = (0.19..=0.28).contains(&d);
            let handle = seg_dist(u, v, 0.59, 0.59, 0.82, 0.82) <= 0.055;
            ring || handle
        }
        Icon::Check => {
            seg_dist(u, v, 0.22, 0.54, 0.42, 0.74) <= 0.068
                || seg_dist(u, v, 0.42, 0.74, 0.80, 0.28) <= 0.068
        }
        Icon::Monitor => {
            // Desktop monitor outline + stand + base matching Settings -> Video Settings.
            let outer = (0.14..=0.86).contains(&u) && (0.18..=0.66).contains(&v);
            let inner = (0.21..=0.79).contains(&u) && (0.25..=0.59).contains(&v);
            let screen = outer && !inner;
            let neck = (0.46..=0.54).contains(&u) && (0.66..=0.80).contains(&v);
            let base = seg_dist(u, v, 0.33, 0.80, 0.67, 0.80) <= 0.04;
            screen || neck || base
        }
        Icon::Gear => {
            // 8-tooth cogwheel with hollow center matching Settings -> Launcher Settings.
            let (dx, dy) = (u - 0.50, v - 0.50);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= 0.14 || d >= 0.41 {
                return false;
            }
            if (0.14..=0.28).contains(&d) {
                return true;
            }
            let dirs: [(f32, f32); 8] = [
                (1.0, 0.0),
                (-1.0, 0.0),
                (0.0, 1.0),
                (0.0, -1.0),
                (0.7071, 0.7071),
                (-0.7071, 0.7071),
                (0.7071, -0.7071),
                (-0.7071, -0.7071),
            ];
            dirs.iter().any(|&(rx, ry)| {
                let proj = dx * rx + dy * ry;
                let perp = (dx * ry - dy * rx).abs();
                (0.24..=0.39).contains(&proj) && perp <= 0.075
            })
        }
        Icon::Flask => {
            // Erlenmeyer laboratory flask with liquid level matching Settings -> Experimental Stuff.
            let rim = seg_dist(u, v, 0.36, 0.17, 0.64, 0.17) <= 0.038;
            let neck_l = seg_dist(u, v, 0.42, 0.17, 0.42, 0.40) <= 0.036;
            let neck_r = seg_dist(u, v, 0.58, 0.17, 0.58, 0.40) <= 0.036;
            let slope_l = seg_dist(u, v, 0.42, 0.40, 0.18, 0.80) <= 0.038;
            let slope_r = seg_dist(u, v, 0.58, 0.40, 0.82, 0.80) <= 0.038;
            let bottom = seg_dist(u, v, 0.18, 0.82, 0.82, 0.82) <= 0.038;
            let liquid = seg_dist(u, v, 0.30, 0.63, 0.70, 0.63) <= 0.032;
            rim || neck_l || neck_r || slope_l || slope_r || bottom || liquid
        }
        Icon::Terminal => {
            // ">_" terminal prompt matching Settings -> Diagnostics.
            let chev_top = seg_dist(u, v, 0.18, 0.30, 0.44, 0.50) <= 0.045;
            let chev_bot = seg_dist(u, v, 0.44, 0.50, 0.18, 0.70) <= 0.045;
            let underscore = seg_dist(u, v, 0.52, 0.72, 0.82, 0.72) <= 0.045;
            chev_top || chev_bot || underscore
        }
        Icon::MicrosoftGrid => {
            // 2x2 four-pane window grid matching Settings -> Account / Buttons.
            let in_x = (0.18..=0.46).contains(&u) || (0.54..=0.82).contains(&u);
            let in_y = (0.18..=0.46).contains(&v) || (0.54..=0.82).contains(&v);
            in_x && in_y
        }
        Icon::Gamepad => {
            // Controller / D-pad + buttons icon for Settings -> Controls.
            let body = ((u - 0.50).abs() <= 0.32 && (v - 0.52).abs() <= 0.20)
                || ((u - 0.24).powi(2) + (v - 0.58).powi(2)).sqrt() <= 0.15
                || ((u - 0.76).powi(2) + (v - 0.58).powi(2)).sqrt() <= 0.15;
            let dpad_h = (0.24..=0.42).contains(&u) && (0.48..=0.55).contains(&v);
            let dpad_v = (0.295..=0.365).contains(&u) && (0.42..=0.61).contains(&v);
            let btn1 = ((u - 0.68).powi(2) + (v - 0.46).powi(2)).sqrt() <= 0.045;
            let btn2 = ((u - 0.76).powi(2) + (v - 0.55).powi(2)).sqrt() <= 0.045;
            body && !(dpad_h || dpad_v || btn1 || btn2)
        }
        Icon::ArrowLeft => {
            let top = seg_dist(u, v, 0.62, 0.24, 0.34, 0.50) <= 0.055;
            let bot = seg_dist(u, v, 0.34, 0.50, 0.62, 0.76) <= 0.055;
            top || bot
        }
        Icon::MenuEquals => {
            // Sleek floating control-hub icon: two horizontal tracks with circular slider knobs.
            let top_bar = seg_dist(u, v, 0.16, 0.35, 0.84, 0.35) <= 0.058;
            let bot_bar = seg_dist(u, v, 0.16, 0.65, 0.84, 0.65) <= 0.058;
            let top_knob = ((u - 0.64).powi(2) + (v - 0.35).powi(2)).sqrt() <= 0.135;
            let top_hole = ((u - 0.64).powi(2) + (v - 0.35).powi(2)).sqrt() <= 0.052;
            let bot_knob = ((u - 0.36).powi(2) + (v - 0.65).powi(2)).sqrt() <= 0.135;
            let bot_hole = ((u - 0.36).powi(2) + (v - 0.65).powi(2)).sqrt() <= 0.052;
            (top_bar || bot_bar || top_knob || bot_knob) && !top_hole && !bot_hole
        }
        Icon::Undo => {
            let arc = {
                let dx = u - 0.54;
                let dy = v - 0.56;
                let r = (dx * dx + dy * dy).sqrt();
                (0.20..=0.31).contains(&r) && !(dx < -0.05 && dy > 0.08)
            };
            let arrow1 = seg_dist(u, v, 0.22, 0.42, 0.38, 0.26) <= 0.058;
            let arrow2 = seg_dist(u, v, 0.22, 0.42, 0.40, 0.52) <= 0.058;
            arc || arrow1 || arrow2
        }
        Icon::Redo => {
            let mu = 1.0 - u;
            let arc = {
                let dx = mu - 0.54;
                let dy = v - 0.56;
                let r = (dx * dx + dy * dy).sqrt();
                (0.20..=0.31).contains(&r) && !(dx < -0.05 && dy > 0.08)
            };
            let arrow1 = seg_dist(mu, v, 0.22, 0.42, 0.38, 0.26) <= 0.058;
            let arrow2 = seg_dist(mu, v, 0.22, 0.42, 0.40, 0.52) <= 0.058;
            arc || arrow1 || arrow2
        }
        Icon::Plus => {
            let h_bar = seg_dist(u, v, 0.22, 0.50, 0.78, 0.50) <= 0.068;
            let v_bar = seg_dist(u, v, 0.50, 0.22, 0.50, 0.78) <= 0.068;
            h_bar || v_bar
        }
        Icon::Crosshair => {
            let (dx, dy) = (u - 0.50, v - 0.50);
            let d = (dx * dx + dy * dy).sqrt();
            let outer_ring = (0.23..=0.30).contains(&d);
            let center_dot = d <= 0.065;
            let h_tick = (v - 0.50).abs() <= 0.042 && (0.12..=0.37).contains(&u.min(1.0 - u));
            let v_tick = (u - 0.50).abs() <= 0.042 && (0.12..=0.37).contains(&v.min(1.0 - v));
            outer_ring || center_dot || h_tick || v_tick
        }
        Icon::Pencil => {
            let shaft = seg_dist(u, v, 0.30, 0.70, 0.75, 0.25) <= 0.082;
            let tip = seg_dist(u, v, 0.19, 0.81, 0.28, 0.72) <= 0.048;
            let gap = seg_dist(u, v, 0.62, 0.38, 0.65, 0.35) <= 0.095;
            (shaft && !gap) || tip
        }
        Icon::Copy => {
            let front_outer = (0.32..=0.82).contains(&u) && (0.32..=0.84).contains(&v);
            let front_inner = (0.39..=0.75).contains(&u) && (0.39..=0.77).contains(&v);
            let front = front_outer && !front_inner;
            let back_outer = (0.18..=0.68).contains(&u)
                && (0.16..=0.68).contains(&v)
                && !((0.28..=0.86).contains(&u) && (0.28..=0.88).contains(&v));
            let back_inner = (0.25..=0.61).contains(&u) && (0.23..=0.61).contains(&v);
            let back = back_outer && !back_inner;
            front || back
        }
    }
}

impl Cache {
    fn region(atlas: &HudAtlas) -> Option<Rect> {
        atlas.sprites.get("horizon/ui-text").copied()
    }
    /// Reserve `w x h` pixels; on overflow forget everything and start over.
    fn place(&mut self, atlas: &HudAtlas, w: u32, h: u32) -> Option<(u32, u32, Rect)> {
        let region = Self::region(atlas)?;
        let (mut x, mut y, mut row) = self.shelf;
        if x + w + 1 > region.w {
            x = 0;
            y += row + 1;
            row = 0;
        }
        if y + h > region.h || w > region.w {
            self.entries.clear();
            self.images.clear();
            self.shelf = (0, 0, 0);
            return None;
        }
        self.shelf = (x + w + 1, y, row.max(h));
        Some((x, y, region))
    }
    fn upload(&mut self, atlas: &HudAtlas, w: u32, h: u32, alpha: &[f32]) -> Option<Rect> {
        let rgba: Vec<u8> = alpha
            .iter()
            .flat_map(|a| [255, 255, 255, (a.clamp(0.0, 1.0) * 255.0).round() as u8])
            .collect();
        self.upload_rgba(atlas, w, h, rgba)
    }
    fn upload_rgba(&mut self, atlas: &HudAtlas, w: u32, h: u32, rgba: Vec<u8>) -> Option<Rect> {
        let (x, y, region) = self.place(atlas, w, h)?;
        let (ax, ay) = (
            (region.u0 * atlas.width as f32).round() as u32 + x,
            (region.v0 * atlas.height as f32).round() as u32 + y,
        );
        self.pending.push(crate::font::Patch {
            x: ax,
            y: ay,
            width: w,
            height: h,
            rgba,
        });
        let (aw, ah) = (atlas.width as f32, atlas.height as f32);
        Some(Rect {
            u0: ax as f32 / aw,
            v0: ay as f32 / ah,
            u1: (ax + w) as f32 / aw,
            v1: (ay + h) as f32 / ah,
            w,
            h,
        })
    }
    fn glyph(&mut self, atlas: &HudAtlas, face: Face, id: ab_glyph::GlyphId, px: f32) -> Cached {
        let size = (px * 4.0).round() as u16;
        let key = Key::Glyph(face, id.0, size);
        if let Some(c) = self.entries.get(&key) {
            return *c;
        }
        let scale = self.fonts.scale(face, f32::from(size) / 4.0);
        let glyph = id.with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
        let made = match self.fonts.face(face).outline_glyph(glyph) {
            Some(outline) => {
                let b = outline.px_bounds();
                let (w, h) = (b.width() as u32, b.height() as u32);
                let mut alpha = vec![0.0; (w * h) as usize];
                outline.draw(|x, y, c| {
                    if x < w && y < h {
                        alpha[(y * w + x) as usize] = c;
                    }
                });
                // Light text on dark backgrounds reads thin: a gentle boost.
                for a in &mut alpha {
                    *a = a.powf(0.85);
                }
                Cached {
                    rect: self.upload(atlas, w, h, &alpha),
                    left: b.min.x,
                    top: b.min.y,
                }
            }
            None => Cached {
                rect: None,
                left: 0.0,
                top: 0.0,
            },
        };
        if made.rect.is_some() || made.left == 0.0 {
            self.entries.insert(key, made);
        }
        made
    }
    fn corner(&mut self, atlas: &HudAtlas, r: u16, thickness: u16) -> Option<Rect> {
        // Cap rasterized corner resolution to 56px so large circles (e.g. joysticks)
        // never overflow the 1024x512 UI text atlas shelf.
        let capped_r = r.clamp(1, 56);
        let scaled_t = if thickness == 0 {
            0
        } else {
            ((u32::from(thickness) * u32::from(capped_r))
                .div_ceil(u32::from(r.max(1)))
                .max(1)) as u16
        };
        let key = Key::Corner(capped_r, scaled_t);
        if let Some(c) = self.entries.get(&key) {
            return c.rect;
        }
        // The top-left quadrant of a circle centred at (r, r), 4x4 supersampled.
        let n = u32::from(capped_r);
        let rf = f32::from(capped_r);
        let tf = f32::from(scaled_t);
        let mut alpha = vec![0.0; (n * n) as usize];
        for y in 0..n {
            for x in 0..n {
                let mut hits = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                        let d = ((px - rf).powi(2) + (py - rf).powi(2)).sqrt();
                        if d <= rf && (scaled_t == 0 || d >= (rf - tf).max(0.0)) {
                            hits += 1;
                        }
                    }
                }
                alpha[(y * n + x) as usize] = hits as f32 / 16.0;
            }
        }
        let rect = self.upload(atlas, n, n, &alpha);
        if rect.is_some() {
            self.entries.insert(
                key,
                Cached {
                    rect,
                    left: 0.0,
                    top: 0.0,
                },
            );
        }
        rect
    }
    fn vector_icon(&mut self, atlas: &HudAtlas, icon: Icon, px: u16) -> Option<Rect> {
        let n = u32::from(px.clamp(8, 128));
        let key = Key::Vector(icon, n as u16);
        if let Some(c) = self.entries.get(&key) {
            return c.rect;
        }
        let nf = n as f32;
        let mut alpha = vec![0.0; (n * n) as usize];
        for y in 0..n {
            for x in 0..n {
                let mut hits = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let u = (x as f32 + (sx as f32 + 0.5) / 4.0) / nf;
                        let v = (y as f32 + (sy as f32 + 0.5) / 4.0) / nf;
                        if icon_hit(icon, u, v) {
                            hits += 1;
                        }
                    }
                }
                alpha[(y * n + x) as usize] = hits as f32 / 16.0;
            }
        }
        let rect = self.upload(atlas, n, n, &alpha);
        if rect.is_some() {
            self.entries.insert(
                key,
                Cached {
                    rect,
                    left: 0.0,
                    top: 0.0,
                },
            );
        }
        rect
    }
    fn image(&mut self, atlas: &HudAtlas, key: &str, w: u32, h: u32, rgba: &[u8]) -> Option<Rect> {
        if let Some(&rect) = self.images.get(key) {
            return Some(rect);
        }
        if w == 0 || h == 0 || rgba.len() != (w * h * 4) as usize {
            return None;
        }
        // Apply a soft rounded-corner mask to project icons so they sit cleanly in cards.
        let mut masked = rgba.to_vec();
        let r = (w.min(h) as f32 * 0.22).max(2.0);
        let (wf, hf) = (w as f32, h as f32);
        for y in 0..h {
            for x in 0..w {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let dx = if px < r {
                    r - px
                } else if px > wf - r {
                    px - (wf - r)
                } else {
                    0.0
                };
                let dy = if py < r {
                    r - py
                } else if py > hf - r {
                    py - (hf - r)
                } else {
                    0.0
                };
                if dx > 0.0 && dy > 0.0 {
                    let d = (dx * dx + dy * dy).sqrt();
                    let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                    let idx = ((y * w + x) * 4 + 3) as usize;
                    masked[idx] = (masked[idx] as f32 * cov).round() as u8;
                }
            }
        }
        let rect = self.upload_rgba(atlas, w, h, masked)?;
        self.images.insert(key.to_owned(), rect);
        Some(rect)
    }
}

/// One frame of design-system drawing, in dp: the
/// launcher's logical pixels, scaled so the screen reads like the launcher's
/// 1040x700 window at any size.
pub struct Ui<'a> {
    pub atlas: &'a HudAtlas,
    pub batch: &'a mut HudBatch,
    /// The screen in dp.
    pub w: f32,
    pub h: f32,
    /// The cursor in dp.
    pub cursor: (f32, f32),
    /// Physical pixels per dp.
    pub density: f32,
    scale: f32,
    clip: Option<[f32; 4]>,
}
impl<'a> Ui<'a> {
    /// `cursor` is in GUI pixels, like every screen's input.
    pub fn new(atlas: &'a HudAtlas, batch: &'a mut HudBatch, cursor: (f32, f32)) -> Self {
        let scale = batch.scale;
        let (pw, ph) = (batch.width * scale, batch.height * scale);
        let density = (pw / 1040.0).min(ph / 700.0).clamp(1.0, 4.0);
        Self {
            atlas,
            w: pw / density,
            h: ph / density,
            cursor: (cursor.0 * scale / density, cursor.1 * scale / density),
            density,
            scale,
            clip: None,
            batch,
        }
    }
    /// Native phone DPI-adaptive launcher density; game HUD scale is untouched.
    pub fn mobile(atlas: &'a HudAtlas, batch: &'a mut HudBatch, cursor: (f32, f32)) -> Self {
        let mut ui = Self::new(atlas, batch, cursor);
        let pw = ui.batch.width * ui.scale;
        let ph = ui.batch.height * ui.scale;
        let hw_dpi = f32::from_bits(ANDROID_DPI_BITS.load(std::sync::atomic::Ordering::Relaxed));
        let user_scale = (UI_SCALE_PERCENT.load(std::sync::atomic::Ordering::Relaxed) as f32
            / 100.0)
            .clamp(0.75, 1.65);
        let base_density = if ph > pw {
            // Portrait phone screen: target ~410dp width so cards and touch buttons are comfortable.
            let target_dpi = (pw / 410.0).min(ph / 640.0);
            if hw_dpi >= 1.0 {
                (target_dpi * 0.7 + hw_dpi * 0.3).clamp(1.0, 4.5)
            } else {
                target_dpi.clamp(1.0, 4.5)
            }
        } else {
            // Landscape phone/tablet screen: target ~425dp height (and ~840dp width) so the
            // left navigation rail, header, and 2-column cards match phone DPI instead of
            // shrinking to a 700dp desktop monitor layout.
            let fit_phone = (pw / 840.0).min(ph / 425.0);
            let dpi = if hw_dpi >= 1.0 {
                // Blend hardware DisplayMetrics density with viewport fit so small screens never clip.
                (hw_dpi * 0.88).min(ph / 385.0).max(fit_phone)
            } else {
                fit_phone
            };
            dpi.clamp(0.65, 4.5)
        };
        // Ensure user_scale still keeps at least 365dp height in landscape so all 6 rail items fit.
        let max_safe_density = if ph > pw {
            (pw / 340.0).max(1.0)
        } else {
            (ph / 365.0).min(pw / 660.0).max(0.65)
        };
        ui.density = (base_density * user_scale)
            .min(max_safe_density)
            .clamp(0.5, 4.5);
        ui.w = pw / ui.density;
        ui.h = ph / ui.density;
        ui.cursor = (
            cursor.0 * ui.scale / ui.density,
            cursor.1 * ui.scale / ui.density,
        );
        ui
    }

    pub fn fit_text(&self, face: Face, size: f32, text: &str, width: f32) -> String {
        if self.measure(face, size, text) <= width {
            return text.to_owned();
        }
        let px = (size * self.density * 4.0).round() / 4.0;
        with_cache(self.atlas, |cache| {
            let font = cache.fonts.face(face);
            let scaled = font.as_scaled(cache.fonts.scale(face, px));
            crate::ui_text::fit(text, width * self.density, |previous, ch| {
                let id = font.glyph_id(ch);
                previous.map_or(0.0, |p| scaled.kern(font.glyph_id(p), id)) + scaled.h_advance(id)
            })
        })
    }
    /// Move rasterized glyphs into the batch's atlas uploads (real frames only).
    pub fn finish(self) {
        let pending = with_cache(self.atlas, |c| std::mem::take(&mut c.pending));
        self.batch.patches.extend(pending);
    }

    /// Set or clear a scissor clip rectangle `[x, y, w, h]` in dp coordinates.
    pub fn set_clip(&mut self, rect: Option<[f32; 4]>) {
        self.clip = rect.map(|[x, y, w, h]| {
            [
                self.px_x(x),
                self.px_y(y),
                self.px_x(x + w.max(0.0)),
                self.px_y(y + h.max(0.0)),
            ]
        });
    }

    pub fn clip_rect(&self, rect: [f32; 4]) -> Option<[f32; 4]> {
        let [x, y, w, h] = rect;
        if let Some([cx0, cy0, cx1, cy1]) = self.clip {
            let d = self.density.max(0.001);
            let x0 = x.max(cx0 / d);
            let y0 = y.max(cy0 / d);
            let x1 = (x + w).min(cx1 / d);
            let y1 = (y + h).min(cy1 / d);
            (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
        } else {
            (w > 0.0 && h > 0.0).then_some(rect)
        }
    }

    // ---- geometry, snapped to physical pixels ----
    fn px_x(&self, v: f32) -> f32 {
        (v * self.density).round()
    }
    fn px_y(&self, v: f32) -> f32 {
        (v * self.density).round()
    }
    fn raw_fill(&mut self, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32, color: u32) {
        if let Some([cx0, cy0, cx1, cy1]) = self.clip {
            x0 = x0.max(cx0);
            y0 = y0.max(cy0);
            x1 = x1.min(cx1);
            y1 = y1.min(cy1);
        }
        if x1 > x0 && y1 > y0 {
            let s = self.scale;
            self.batch.fill(
                self.atlas,
                x0 / s,
                y0 / s,
                (x1 - x0) / s,
                (y1 - y0) / s,
                color,
            );
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn emit_clipped_uv(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: u32,
    ) {
        let w = x1 - x0;
        let h = y1 - y0;
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let (mut nx0, mut ny0, mut nx1, mut ny1) = (x0, y0, x1, y1);
        let (mut nu0, mut nv0, mut nu1, mut nv1) = (u0, v0, u1, v1);
        if let Some([cx0, cy0, cx1, cy1]) = self.clip {
            nx0 = x0.max(cx0);
            ny0 = y0.max(cy0);
            nx1 = x1.min(cx1);
            ny1 = y1.min(cy1);
            if nx1 <= nx0 || ny1 <= ny0 {
                return;
            }
            let du = u1 - u0;
            let dv = v1 - v0;
            nu0 = u0 + du * ((nx0 - x0) / w);
            nu1 = u0 + du * ((nx1 - x0) / w);
            nv0 = v0 + dv * ((ny0 - y0) / h);
            nv1 = v0 + dv * ((ny1 - y0) / h);
        }
        let s = self.scale;
        let (gx0, gy0, gx1, gy1) = (nx0 / s, ny0 / s, nx1 / s, ny1 / s);
        self.batch.quad(
            [[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]],
            [[nu0, nv0], [nu1, nv0], [nu1, nv1], [nu0, nv1]],
            color,
        );
    }
    fn corner_quad(&mut self, rect: Rect, x: f32, y: f32, r: f32, flip: (bool, bool), color: u32) {
        let (u0, u1) = if flip.0 {
            (rect.u1, rect.u0)
        } else {
            (rect.u0, rect.u1)
        };
        let (v0, v1) = if flip.1 {
            (rect.v1, rect.v0)
        } else {
            (rect.v0, rect.v1)
        };
        self.emit_clipped_uv(x, y, x + r, y + r, u0, v0, u1, v1, color);
    }
    fn corners(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, thickness: u16, color: u32) {
        if r < 1.0 {
            return;
        }
        let atlas = self.atlas;
        if let Some(rect) = with_cache(atlas, |c| c.corner(atlas, r as u16, thickness)) {
            self.corner_quad(rect, x0, y0, r, (false, false), color);
            self.corner_quad(rect, x1 - r, y0, r, (true, false), color);
            self.corner_quad(rect, x1 - r, y1 - r, r, (true, true), color);
            self.corner_quad(rect, x0, y1 - r, r, (false, true), color);
        }
    }
    /// A filled rectangle with rounded corners.
    pub fn rounded(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32) {
        if color >> 24 == 0 {
            return;
        }
        let (x0, y0, x1, y1) = (
            self.px_x(x),
            self.px_y(y),
            self.px_x(x + w),
            self.px_y(y + h),
        );
        let r = (radius * self.density)
            .round()
            .min(((x1 - x0) / 2.0).floor())
            .min(((y1 - y0) / 2.0).floor())
            .max(0.0);
        if r < 1.0 {
            self.raw_fill(x0, y0, x1, y1, color);
            return;
        }
        self.raw_fill(x0 + r, y0, x1 - r, y1, color);
        self.raw_fill(x0, y0 + r, x0 + r, y1 - r, color);
        self.raw_fill(x1 - r, y0 + r, x1, y1 - r, color);
        self.corners(x0, y0, x1, y1, r, 0, color);
    }
    /// A 1px rounded border.
    pub fn border(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32) {
        if color >> 24 == 0 {
            return;
        }
        let (x0, y0, x1, y1) = (
            self.px_x(x),
            self.px_y(y),
            self.px_x(x + w),
            self.px_y(y + h),
        );
        let r = (radius * self.density)
            .round()
            .min(((x1 - x0) / 2.0).floor())
            .min(((y1 - y0) / 2.0).floor())
            .max(0.0);
        self.raw_fill(x0 + r, y0, x1 - r, y0 + 1.0, color);
        self.raw_fill(x0 + r, y1 - 1.0, x1 - r, y1, color);
        self.raw_fill(x0, y0 + r, x0 + 1.0, y1 - r, color);
        self.raw_fill(x1 - 1.0, y0 + r, x1, y1 - r, color);
        self.corners(x0, y0, x1, y1, r, 1, color);
    }

    /// Draw a supersampled vector icon in `size x size` dp.
    pub fn icon(&mut self, icon: Icon, x: f32, y: f32, size: f32, color: u32) {
        if color >> 24 == 0 || size <= 0.0 {
            return;
        }
        let (x0, y0) = (self.px_x(x), self.px_y(y));
        let px = (size * self.density).round().clamp(8.0, 128.0) as u16;
        let atlas = self.atlas;
        if let Some(rect) = with_cache(atlas, |c| c.vector_icon(atlas, icon, px)) {
            let pxf = f32::from(px);
            self.emit_clipped_uv(
                x0,
                y0,
                x0 + pxf,
                y0 + pxf,
                rect.u0,
                rect.v0,
                rect.u1,
                rect.v1,
                color,
            );
        }
    }

    /// Draw the stylized two-tone purple Horizon `H` mark at `(x, y)` in `size x size` dp.
    pub fn horizon_logo(&mut self, x: f32, y: f32, size: f32) {
        // Subtle glow/shadow plate behind the mark, then the crisp supersampled H mark.
        let primary = color::hex(0x9A7BFF, 255);
        let accent = color::hex(0x7C5CE6, 255);
        self.icon(Icon::HorizonLogo, x, y + 0.5, size, accent);
        self.icon(Icon::HorizonLogo, x, y, size, primary);
    }

    /// Draw a 3D isometric Minecraft player head with top crown, front face (eyes, hair, skin),
    /// shaded side face, and 3D hat overlay layer.
    /// Draw a real Minecraft head: the classic 8x8 Steve face with its top and
    /// right side as a small 3D cube. Offline (cracked) accounts are Steve,
    /// which is what the launcher shows until a signed-in skin is available.
    pub fn avatar_head_3d(&mut self, x: f32, y: f32, size: f32, player_name: &str) {
        let _ = player_name;
        self.rounded(x, y, size, size, size * 0.28, color::hex(0x231E38, 255));
        self.border(x, y, size, size, size * 0.28, color::hex(0x4E4275, 255));
        self.rounded(
            x + size * 0.18,
            y + size * 0.78,
            size * 0.64,
            size * 0.12,
            size * 0.06,
            color::hex(0x08060E, 175),
        );

        const HAIR: u32 = 0x3B2A1B;
        const HAIR_HI: u32 = 0x4C3624;
        const SKIN: u32 = 0xC68E5E;
        const SKIN_SIDE: u32 = 0x9E6B42;
        const EYE: u32 = 0xF4F4F4;
        const IRIS: u32 = 0x4E6E9E;
        const NOSE: u32 = 0xA87548;
        const MOUTH: u32 = 0x6B4A32;
        const BEARD: u32 = 0x2E2117;

        // The classic Steve face, one entry per skin pixel.
        const FACE: [[u32; 8]; 8] = [
            [HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR],
            [HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR],
            [HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR, HAIR],
            [HAIR, SKIN, SKIN, SKIN, SKIN, SKIN, SKIN, HAIR],
            [SKIN, EYE, IRIS, SKIN, SKIN, IRIS, EYE, SKIN],
            [SKIN, SKIN, SKIN, NOSE, NOSE, SKIN, SKIN, SKIN],
            [SKIN, MOUTH, MOUTH, MOUTH, MOUTH, MOUTH, MOUTH, SKIN],
            [SKIN, BEARD, BEARD, BEARD, BEARD, BEARD, BEARD, SKIN],
        ];

        let pw = size * 0.088;
        let face = pw * 8.0;
        let cx = x + (size - face - pw * 3.2) * 0.5 + pw * 1.6;
        let cy = y + (size - face) * 0.53;

        // Top: hair, skewed up and to the right.
        for row in 0..8 {
            for col in 0..8 {
                let px = cx + col as f32 * pw + row as f32 * pw * 0.42;
                let py = cy - row as f32 * pw * 0.48 - pw * 0.48;
                let tone = if (row + col) % 3 == 0 { HAIR_HI } else { HAIR };
                self.fill(px, py, pw + 0.5, pw * 0.5 + 0.5, color::hex(tone, 255));
            }
        }
        // Right side: hair across the top, shaded skin below.
        for row in 0..8 {
            for col in 0..8 {
                let px = cx + face + row as f32 * pw * 0.42;
                let py = cy + col as f32 * pw - row as f32 * pw * 0.48;
                let tone = if col < 2 { HAIR } else { SKIN_SIDE };
                self.fill(px, py, pw + 0.5, pw + 0.5, color::hex(tone, 255));
            }
        }
        // Front: the real texture.
        for (row, pixels) in FACE.iter().enumerate() {
            for (col, tone) in pixels.iter().enumerate() {
                self.fill(
                    cx + col as f32 * pw,
                    cy + row as f32 * pw,
                    pw + 0.5,
                    pw + 0.5,
                    color::hex(*tone, 255),
                );
            }
        }
        // The hair's highlight runs along the top edge of the face.
        self.fill(cx, cy, face, pw * 0.45, color::hex(HAIR_HI, 210));
    }

    /// Draw an authentic Material You 3 Switch (`52x28dp` pill track with sliding thumb and checkmark).
    pub fn m3_switch(&mut self, x: f32, y: f32, on: bool) {
        let track_w = 52.0;
        let track_h = 28.0;
        if on {
            self.rounded(x, y, track_w, track_h, 14.0, color::hex(0x9A7BFF, 255));
            let thumb_d = 22.0;
            let tx = x + track_w - thumb_d - 3.0;
            let ty = y + (track_h - thumb_d) * 0.5;
            self.rounded(tx, ty, thumb_d, thumb_d, 11.0, color::hex(0x17112E, 255));
            self.icon(
                Icon::Check,
                tx + 4.0,
                ty + 4.0,
                14.0,
                color::hex(0x9A7BFF, 255),
            );
        } else {
            self.rounded(x, y, track_w, track_h, 14.0, color::hex(0x262336, 255));
            self.stroke_rounded(x, y, track_w, track_h, 14.0, 1.6, color::hex(0x7A7394, 255));
            let thumb_d = 16.0;
            let tx = x + 6.0;
            let ty = y + (track_h - thumb_d) * 0.5;
            self.rounded(tx, ty, thumb_d, thumb_d, 8.0, color::hex(0x9C96B0, 255));
        }
    }

    /// Draw an authentic Material You 3 Checkbox with label (`[✓] Label`) and return total width.
    pub fn m3_checkbox(&mut self, x: f32, y: f32, size: f32, checked: bool, label: &str) -> f32 {
        let r = (size * 0.28).clamp(4.0, 7.0);
        if checked {
            self.rounded(x, y, size, size, r, color::hex(0x9A7BFF, 255));
            let icon_s = (size - 4.0).max(10.0);
            self.icon(
                Icon::Check,
                x + (size - icon_s) * 0.5,
                y + (size - icon_s) * 0.5,
                icon_s,
                color::hex(0x17112E, 255),
            );
        } else {
            self.rounded(x, y, size, size, r, color::hex(0x1C192B, 255));
            self.stroke_rounded(x, y, size, size, r, 1.6, color::hex(0x7A7394, 255));
        }
        if !label.is_empty() {
            let tw = self.text(
                Face::Medium,
                12.5,
                x + size + 8.0,
                y + (size - 18.0) * 0.5,
                if checked {
                    color::hex(0xF4F0FC, 255)
                } else {
                    color::hex(0xB8B1CC, 255)
                },
                label,
            );
            size + 8.0 + tw
        } else {
            size
        }
    }

    /// Draw the circular player head avatar badge inside the top-right account pill.
    pub fn avatar_head(&mut self, x: f32, y: f32, size: f32) {
        let r = size / 2.0;
        self.rounded(x, y, size, size, r, color::hex(0x6E56C4, 255));
        // Hair/top cap and face details inside the circular badge.
        let inset = size * 0.18;
        let face_w = size - inset * 2.0;
        self.rounded(
            x + inset,
            y + inset,
            face_w,
            face_w,
            face_w * 0.28,
            color::hex(0xB89EFF, 255),
        );
        self.rounded(
            x + inset,
            y + inset,
            face_w,
            face_w * 0.36,
            face_w * 0.2,
            color::hex(0x41307A, 255),
        );
        let eye_w = (size * 0.11).max(2.0);
        let eye_h = (size * 0.13).max(2.0);
        let eye_y = y + size * 0.48;
        self.rounded(
            x + size * 0.31,
            eye_y,
            eye_w,
            eye_h,
            1.0,
            color::hex(0x231942, 255),
        );
        self.rounded(
            x + size * 0.58,
            eye_y,
            eye_w,
            eye_h,
            1.0,
            color::hex(0x231942, 255),
        );
    }

    /// Draw a cached RGBA project icon (`w x h`) into `(x, y, dw, dh)` dp.
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &mut self,
        key: &str,
        w: u32,
        h: u32,
        rgba: &[u8],
        x: f32,
        y: f32,
        dw: f32,
        dh: f32,
    ) -> bool {
        let atlas = self.atlas;
        let Some(rect) = with_cache(atlas, |c| c.image(atlas, key, w, h, rgba)) else {
            return false;
        };
        let (x0, y0, x1, y1) = (
            self.px_x(x),
            self.px_y(y),
            self.px_x(x + dw),
            self.px_y(y + dh),
        );
        self.emit_clipped_uv(x0, y0, x1, y1, rect.u0, rect.v0, rect.u1, rect.v1, u32::MAX);
        true
    }

    // ---- text ----
    /// The advance of `text` in dp.
    pub fn measure(&self, face: Face, size: f32, text: &str) -> f32 {
        let px = (size * self.density * 4.0).round() / 4.0;
        with_cache(self.atlas, |c| {
            let font = c.fonts.face(face);
            let scaled = font.as_scaled(c.fonts.scale(face, px));
            let mut w = 0.0;
            let mut prev = None;
            for ch in text.chars() {
                let id = font.glyph_id(ch);
                if let Some(p) = prev {
                    w += scaled.kern(p, id);
                }
                w += scaled.h_advance(id);
                prev = Some(id);
            }
            w / self.density
        })
    }
    /// Draw a line of text in a `line`-tall box whose top is `y` (the
    /// launcher's line heights: about 1.45 × size for the interface);
    /// returns its width in dp.
    pub(crate) fn text_line(
        &mut self,
        face: Face,
        size: f32,
        line: f32,
        x: f32,
        y: f32,
        color: u32,
        text: &str,
    ) -> f32 {
        let px = (size * self.density * 4.0).round() / 4.0;
        let real = color >> 24 != 0;
        let (x0, y0) = (self.px_x(x), self.px_y(y));
        let box_px = line * self.density;
        let atlas = self.atlas;
        let density = self.density;
        let mut quads = Vec::new();
        let width = with_cache(atlas, |c| {
            let scale = c.fonts.scale(face, px);
            let (ascent, descent) = {
                let f = c.fonts.face(face).as_scaled(scale);
                (f.ascent(), f.descent())
            };
            let baseline = y0 + ((box_px - (ascent - descent)) / 2.0 + ascent).round();
            let mut pen = x0;
            let mut prev = None;
            for ch in text.chars() {
                let (id, advance, kern) = {
                    let font = c.fonts.face(face);
                    let id = font.glyph_id(ch);
                    let f = font.as_scaled(scale);
                    (id, f.h_advance(id), prev.map_or(0.0, |p| f.kern(p, id)))
                };
                pen += kern;
                if real && !ch.is_whitespace() {
                    let g = c.glyph(atlas, face, id, px);
                    if let Some(rect) = g.rect {
                        quads.push(((pen + g.left).round(), baseline + g.top, rect));
                    }
                }
                pen += advance;
                prev = Some(id);
            }
            (pen - x0) / density
        });
        for (gx, gy, rect) in quads {
            self.emit_clipped_uv(
                gx,
                gy,
                gx + rect.w as f32,
                gy + rect.h as f32,
                rect.u0,
                rect.v0,
                rect.u1,
                rect.v1,
                color,
            );
        }
        width
    }
    /// Text in a `1.45 × size` line box whose top is `y`.
    pub fn text(&mut self, face: Face, size: f32, x: f32, y: f32, color: u32, text: &str) -> f32 {
        self.text_line(face, size, (size * 1.45).round(), x, y, color, text)
    }
    pub(crate) fn text_centered(
        &mut self,
        face: Face,
        size: f32,
        cx: f32,
        y: f32,
        color: u32,
        text: &str,
    ) {
        let w = self.measure(face, size, text);
        self.text(face, size, cx - w / 2.0, y, color, text);
    }

    /// Filled axis-aligned rectangle in dp.
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32) {
        if color >> 24 == 0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        self.raw_fill(
            self.px_x(x),
            self.px_y(y),
            self.px_x(x + w),
            self.px_y(y + h),
            color,
        );
    }

    /// Multi-pixel rounded border in dp (`stroke_w` dp thick) rendered in a single pass.
    pub fn stroke_rounded(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        stroke_w: f32,
        color: u32,
    ) {
        if color >> 24 == 0 || stroke_w <= 0.0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let (x0, y0, x1, y1) = (
            self.px_x(x),
            self.px_y(y),
            self.px_x(x + w),
            self.px_y(y + h),
        );
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let t = (stroke_w * self.density).round().clamp(1.0, 24.0);
        let r = (radius * self.density)
            .round()
            .min(((x1 - x0) / 2.0).floor())
            .min(((y1 - y0) / 2.0).floor())
            .max(0.0);
        if r < 1.0 {
            self.raw_fill(x0, y0, x1, (y0 + t).min(y1), color);
            self.raw_fill(x0, (y1 - t).max(y0), x1, y1, color);
            self.raw_fill(
                x0,
                (y0 + t).min(y1),
                (x0 + t).min(x1),
                (y1 - t).max(y0),
                color,
            );
            self.raw_fill(
                (x1 - t).max(x0),
                (y0 + t).min(y1),
                x1,
                (y1 - t).max(y0),
                color,
            );
            return;
        }
        self.raw_fill(x0 + r, y0, x1 - r, (y0 + t).min(y1), color);
        self.raw_fill(x0 + r, (y1 - t).max(y0), x1 - r, y1, color);
        self.raw_fill(x0, y0 + r, (x0 + t).min(x1), y1 - r, color);
        self.raw_fill((x1 - t).max(x0), y0 + r, x1, y1 - r, color);
        if t < r {
            let atlas = self.atlas;
            if let Some(rect) = with_cache(atlas, |c| c.corner(atlas, r as u16, t as u16)) {
                self.corner_quad(rect, x0, y0, r, (false, false), color);
                self.corner_quad(rect, x1 - r, y0, r, (true, false), color);
                self.corner_quad(rect, x1 - r, y1 - r, r, (true, true), color);
                self.corner_quad(rect, x0, y1 - r, r, (false, true), color);
            }
        } else {
            self.corners(x0, y0, x1, y1, r, 0, color);
        }
    }

    /// Centered fitted text inside `[x, y, w, h]` dp.
    pub fn centered_fit(&mut self, face: Face, size: f32, rect: [f32; 4], color: u32, text: &str) {
        let [x, y, w, h] = rect;
        if w <= 2.0 || h <= 2.0 || text.is_empty() {
            return;
        }
        let fitted = self.fit_text(face, size, text, (w - 4.0).max(4.0));
        let tw = self.measure(face, size, &fitted);
        let line_h = (size * 1.45).round();
        self.text(
            face,
            size,
            x + (w - tw) * 0.5,
            y + (h - line_h) * 0.5,
            color,
            &fitted,
        );
    }

    /// Draws a GoyControl shape (0=RoundedRect, 1=Square, 2=Circle/Pill, 3=CutOctagon, 4=Diamond, 5=Hexagon)
    /// with customizable shadow, fill, stroke thickness, stroke margin, and stroke roundness.
    #[allow(clippy::too_many_arguments)]
    pub fn goy_box(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        shape: u8,
        roundness: f32,
        fill_color: u32,
        stroke_size: f32,
        stroke_margin: f32,
        stroke_roundness: f32,
        stroke_color: u32,
        shadow_size: f32,
        shadow_color: u32,
    ) {
        let eff_radius = match shape {
            1 => 0.0,
            2 => w.min(h) * 0.5,
            3 => (w.min(h) * 0.22).max(4.0),
            4 => (w.min(h) * 0.36).max(6.0),
            5 => (w.min(h) * 0.28).max(5.0),
            _ => roundness.clamp(0.0, w.min(h) * 0.5),
        };
        let eff_stroke_r = match shape {
            1 => 0.0,
            2 => (w + stroke_margin * 2.0).min(h + stroke_margin * 2.0) * 0.5,
            _ => stroke_roundness.clamp(
                0.0,
                (w + stroke_margin * 2.0).min(h + stroke_margin * 2.0) * 0.5,
            ),
        };
        // 1. Drop shadow plate
        if shadow_size > 0.1 && (shadow_color >> 24) > 0 {
            let so = shadow_size.clamp(0.5, 14.0);
            self.rounded(x + so * 0.45, y + so * 0.75, w, h, eff_radius, shadow_color);
        }
        // 2. Main Fill Shape
        match shape {
            3 => {
                // Cut-Corner Octagon / Beveled Tactical Plate
                let cut = (w.min(h) * 0.20).clamp(3.0, 14.0);
                self.fill(x + cut, y, (w - cut * 2.0).max(1.0), h, fill_color);
                self.fill(x, y + cut, w, (h - cut * 2.0).max(1.0), fill_color);
                let half_c = cut * 0.5;
                self.fill(
                    x + half_c,
                    y + half_c,
                    (w - cut).max(1.0),
                    (h - cut).max(1.0),
                    fill_color,
                );
            }
            4 => {
                // Stepped Diamond / Rhombus Plate
                let steps = 7;
                for i in 0..steps {
                    let frac = (i as f32 + 0.5) / steps as f32;
                    let dist = (frac - 0.5).abs() * 2.0;
                    let sw = (w * (1.0 - dist * 0.72)).max(4.0);
                    let sy = y + (i as f32 / steps as f32) * h;
                    let sh = (h / steps as f32) + 0.8;
                    self.rounded(x + (w - sw) * 0.5, sy, sw, sh, 2.0, fill_color);
                }
            }
            5 => {
                // Hexagon Tactical Badge
                let steps = 5;
                for i in 0..steps {
                    let dist = ((i as f32 - 2.0).abs() / 2.0).powf(1.2);
                    let sw = (w * (1.0 - dist * 0.34)).max(4.0);
                    let sy = y + (i as f32 / steps as f32) * h;
                    let sh = (h / steps as f32) + 0.8;
                    self.rounded(x + (w - sw) * 0.5, sy, sw, sh, 3.0, fill_color);
                }
            }
            _ => {
                self.rounded(x, y, w, h, eff_radius, fill_color);
            }
        }
        // 3. Stroke Border (with stroke_margin offset and stroke_roundness)
        if stroke_size > 0.1 && (stroke_color >> 24) > 0 {
            let sx = x - stroke_margin;
            let sy = y - stroke_margin;
            let sw = (w + stroke_margin * 2.0).max(2.0);
            let sh = (h + stroke_margin * 2.0).max(2.0);
            self.stroke_rounded(sx, sy, sw, sh, eff_stroke_r, stroke_size, stroke_color);
        }
    }

    // ---- the launcher's components ----
}

// ---- the boot screen ----

/// The boot screen's palette on the pure black: a white wordmark and dimmed
/// console lines (the theme's ink is for light pages).
const BOOT_TEXT: u32 = color::hex(0xFFFFFF, 255);
const BOOT_LOG: u32 = color::hex(0x9BA1A6, 255);

/// The boot screen while the menu is not ready yet: the wordmark centred on
/// pure black and the console's last lines at the bottom right, as the
/// terminal prints them.
pub fn draw_boot(atlas: &HudAtlas, batch: &mut HudBatch, logs: &[String]) {
    // Pure black under everything: the frame's panorama (the title artwork
    // once the bundle is in) must not leak through.
    batch.fill(
        atlas,
        0.0,
        0.0,
        batch.width,
        batch.height,
        color::hex(0x000000, 255),
    );
    wordmark(atlas, batch);
    draw_boot_log(atlas, batch, logs);
}

/// The wordmark with its tagline, centred.
fn wordmark(atlas: &HudAtlas, batch: &mut HudBatch) {
    let mut ui = Ui::new(atlas, batch, (0.0, 0.0));
    let size = (ui.w / 16.0).clamp(DISPLAY, 96.0);
    let small = (size * 0.25).clamp(MICRO, 18.0);
    let block = size * 1.45 + small * 0.6 + small * 1.5;
    let top = (ui.h - block) / 2.0;
    ui.text_centered(Face::SemiBold, size, ui.w / 2.0, top, BOOT_TEXT, "Horizon");
    ui.text_centered(
        Face::Mono,
        small,
        ui.w / 2.0,
        top + size * 1.45 + small * 0.6,
        BOOT_LOG,
        "Rust based minecraft client",
    );
    ui.finish();
}

/// The console's last lines at the bottom left, over any loading screen: long
/// lines wrap word-wise, the block grows upwards.
pub fn draw_boot_log(atlas: &HudAtlas, batch: &mut HudBatch, logs: &[String]) {
    if logs.is_empty() {
        return;
    }
    let mut ui = Ui::new(atlas, batch, (0.0, 0.0));
    let (size, line, pad) = (MICRO, MICRO * 1.5, 16.0);
    let max_w = ui.w / 2.0;
    let max_rows = (((ui.h - pad) / line) as usize).clamp(1, 24);
    let mut y = ui.h - pad - line;
    let mut drawn = 0;
    'lines: for text in logs.iter().rev() {
        let chunks = wrapping_lines(&ui, Face::Mono, size, text, max_w);
        for chunk in chunks.iter().rev() {
            if drawn == max_rows {
                break 'lines;
            }
            ui.text(Face::Mono, size, pad, y, BOOT_LOG, chunk);
            y -= line;
            drawn += 1;
        }
    }
    ui.finish();
}

/// The lines `text` wraps into at `max_w`, word-wise; a word wider than
/// `max_w` on its own breaks at the edge.
fn wrapping_lines(ui: &Ui, face: Face, size: f32, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let mut word = word;
        loop {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if ui.measure(face, size, &candidate) <= max_w {
                line = candidate;
                break;
            }
            if line.is_empty() {
                // The word alone does not fit: hard-break it at the edge.
                let mut end = 0;
                for (i, ch) in word.char_indices() {
                    if i > 0 && ui.measure(face, size, &word[..i]) > max_w {
                        break;
                    }
                    end = i + ch.len_utf8();
                }
                lines.push(word[..end].to_owned());
                word = &word[end..];
            } else {
                lines.push(std::mem::take(&mut line));
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_reload_regenerates_text_and_corners_at_the_same_address() {
        let atlas = HudAtlas::build(Path::new("/nonexistent/horizon-test-assets")).unwrap();
        reset_atlas();
        let draw = |cache: &mut Cache| {
            let glyph = cache.fonts.face(Face::Regular).glyph_id('A');
            assert!(
                cache
                    .glyph(&atlas, Face::Regular, glyph, 16.0)
                    .rect
                    .is_some()
            );
            assert!(cache.corner(&atlas, 8, false).is_some());
            std::mem::take(&mut cache.pending)
        };
        let before = with_cache(&atlas, draw);
        assert_eq!(before.len(), 2);
        assert!(with_cache(&atlas, draw).is_empty());
        reset_atlas();
        let after = with_cache(&atlas, draw);
        assert_eq!(after.len(), 2);
        for (a, b) in before.iter().zip(&after) {
            assert_eq!((a.x, a.y, a.width, a.height), (b.x, b.y, b.width, b.height));
            assert_eq!(a.rgba, b.rgba);
        }
        reset_atlas();
    }

    #[test]
    fn the_boot_atlas_holds_the_run_time_glyph_region_without_a_pack() {
        let atlas = HudAtlas::boot();
        assert_eq!((atlas.width, atlas.height), (1024, 1024));
        assert_eq!(
            atlas.sprites["horizon/ui-text"].texels((atlas.width, atlas.height)),
            (1024, 512)
        );
        assert_eq!(
            atlas.sprites["horizon/white"].texels((atlas.width, atlas.height)),
            (4, 4)
        );
    }

    #[test]
    fn the_boot_screen_centres_the_wordmark_and_left_aligns_the_console() {
        reset_atlas();
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([1280, 720]);
        draw_boot(&atlas, &mut batch, &[]);
        // The first quad is the pure black backdrop over the frame's panorama.
        let (w, h) = (batch.width, batch.height);
        let bg = &batch.vertices[..6];
        assert!(bg.iter().all(|v| v.color == 0xFF00_0000));
        assert!(bg.iter().all(|v| v.pos[0] == 0.0 || v.pos[0] == w));
        assert!(bg.iter().all(|v| v.pos[1] == 0.0 || v.pos[1] == h));
        assert!(
            !batch.patches.is_empty(),
            "the glyphs are queued for the atlas"
        );

        let mut batch = HudBatch::new([1280, 720]);
        wordmark(&atlas, &mut batch);
        let (min, max) = batch
            .vertices
            .iter()
            .map(|v| v.pos[0])
            .fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
        assert!(
            min < batch.width / 2.0 && max > batch.width / 2.0,
            "the wordmark straddles the centre"
        );
        assert!(((min + max) / 2.0 - batch.width / 2.0).abs() < 1.0);

        let long = "[12:34:56] [Render thread/INFO]: Horizon: resources loaded in 1.0s; 1240 block states, 1500 items, 62 biomes; 8 worker threads".to_owned();
        let mut batch = HudBatch::new([1280, 720]);
        draw_boot_log(&atlas, &mut batch, &[long]);
        let (min, max) = batch
            .vertices
            .iter()
            .map(|v| v.pos[0])
            .fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
        assert!(min >= 4.0, "the line stays inside the left inset");
        assert!(
            max - min <= batch.width / 2.0 + 1.0,
            "a too wide line is cut to half the screen"
        );

        let mut batch = HudBatch::new([1280, 720]);
        let ui = Ui::new(&atlas, &mut batch, (0.0, 0.0));
        let lines = wrapping_lines(
            &ui,
            Face::Mono,
            MICRO,
            "the quick brown fox jumps over the lazy dog",
            90.0,
        );
        assert!(lines.len() > 1, "a long line wraps");
        assert!(
            lines
                .iter()
                .all(|l| ui.measure(Face::Mono, MICRO, l) <= 90.0),
            "every wrapped line fits"
        );
        assert_eq!(
            lines
                .iter()
                .map(|l| l.split_whitespace().count())
                .sum::<usize>(),
            9,
            "the wrap keeps every word"
        );

        let mut batch = HudBatch::new([1280, 720]);
        draw_boot_log(&atlas, &mut batch, &[]);
        assert!(batch.vertices.is_empty() && batch.patches.is_empty());
        reset_atlas();
    }
}
