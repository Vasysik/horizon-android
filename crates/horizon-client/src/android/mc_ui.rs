//! The launcher's Minecraft-flavoured skin: flat solid colours, sharp corners,
//! bevelled buttons and hard text shadows, drawn through the same `Ui` batcher
//! as the rest of the client. Every widget is a handful of plain quads (a
//! button is four, a panel four, a list row five) with no rounded-corner
//! rasterisation, textures, gradients or blur, so a full page stays cheap to
//! redraw on low-end GPUs. Light text carries vanilla's drop shadow; small
//! body text does not, which keeps scrolling lists at one quad per glyph.

use horizon_client::ui_theme::{Face, Ui, color::hex};

/// The skin's palette. Stones and greens are the vanilla widget colours; the
/// darks are deepslate neutrals so the launcher reads as Minecraft's own GUI
/// on a phone OLED.
pub mod color {
    use super::hex;
    /// The page behind everything: dimmed-panorama near black.
    pub const PAGE: u32 = hex(0x18181d, 255);
    /// Dialog panels above the page.
    pub const PANEL: u32 = hex(0x1f1f26, 255);
    pub const PANEL_HI: u32 = hex(0x4c4c56, 255);
    pub const PANEL_LO: u32 = hex(0x111115, 255);
    pub const PANEL_EDGE: u32 = hex(0x0b0b0e, 255);
    /// List rows ("slots").
    pub const SLOT: u32 = hex(0x141419, 255);
    pub const SLOT_BORDER: u32 = hex(0x2a2a31, 255);
    /// One-pixel rules between settings rows.
    pub const DIVIDER: u32 = hex(0x26262c, 255);
    /// Text.
    pub const TEXT: u32 = hex(0xf2f2f5, 255);
    pub const MUTED: u32 = hex(0xa6a6b0, 255);
    pub const FAINT: u32 = hex(0x6f6f7a, 255);
    /// Vanilla's hard drop shadow under light text.
    pub const SHADOW: u32 = hex(0x000000, 150);
    /// The dim veil behind dialogs.
    pub const DIM: u32 = hex(0x000000, 190);
    /// Stone buttons (vanilla widgets): the face and its bevel.
    pub const STONE_FACE: u32 = hex(0xc6c6c6, 255);
    pub const STONE_PRESSED: u32 = hex(0xb0b0b0, 255);
    pub const STONE_HI: u32 = hex(0xf2f2f2, 255);
    pub const STONE_LO: u32 = hex(0x555555, 255);
    pub const STONE_EDGE: u32 = hex(0x131313, 255);
    pub const STONE_TEXT: u32 = hex(0x3f3f3f, 255);
    /// Dark plates: inactive tabs, quiet buttons.
    pub const DARK_FACE: u32 = hex(0x3a3a42, 255);
    pub const DARK_PRESSED: u32 = hex(0x303038, 255);
    pub const DARK_HI: u32 = hex(0x5c5c66, 255);
    pub const DARK_LO: u32 = hex(0x1e1e23, 255);
    pub const DARK_EDGE: u32 = hex(0x0c0c0f, 255);
    /// Primary actions: grass green.
    pub const GREEN_FACE: u32 = hex(0x4c9a2a, 255);
    pub const GREEN_PRESSED: u32 = hex(0x41861f, 255);
    pub const GREEN_HI: u32 = hex(0x7ccb50, 255);
    pub const GREEN_LO: u32 = hex(0x2b5e16, 255);
    pub const GREEN_EDGE: u32 = hex(0x14240c, 255);
    /// Green text on dark plates ("ON", "Installed", list numbers).
    pub const GREEN_TEXT: u32 = hex(0x77d94a, 255);
    /// Destructive confirmations.
    pub const RED_FACE: u32 = hex(0xa63c32, 255);
    pub const RED_PRESSED: u32 = hex(0x92342b, 255);
    pub const RED_HI: u32 = hex(0xd16a58, 255);
    pub const RED_LO: u32 = hex(0x5f211a, 255);
    pub const RED_EDGE: u32 = hex(0x1c0c0a, 255);
    pub const RED_TEXT: u32 = hex(0xe28878, 255);
    /// Status-line errors.
    pub const ERROR: u32 = hex(0xe88177, 255);
    /// Edit fields: vanilla's black box in a gray frame (white when focused).
    pub const FIELD_BG: u32 = hex(0x0a0a0c, 255);
    pub const FIELD_BORDER: u32 = hex(0x6a6a74, 255);
    /// Progress bars and scrollbars.
    pub const TRACK: u32 = hex(0x0e0e11, 255);
    pub const TRACK_BORDER: u32 = hex(0x2e2e35, 255);
    pub const SCROLL_THUMB: u32 = hex(0x7c7c86, 255);
    /// The grass-block mark beside the launcher title.
    pub const GRASS_TOP: u32 = hex(0x5fb936, 255);
    pub const DIRT: u32 = hex(0x79553a, 255);
    pub const DIRT_DARK: u32 = hex(0x5d4029, 255);
    /// The highlight sliver inside a checked box.
    pub const CHECK_HI: u32 = hex(0xffffff, 70);
}

/// Which bevelled plate a widget draws.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// Vanilla's stone button: secondary actions.
    Stone,
    /// A dark plate: inactive tabs, quiet buttons.
    Dark,
    /// Grass green: the primary action, the selected tab, "ON".
    Green,
    /// Red: destructive confirmations.
    Red,
    /// A dialog panel.
    Panel,
}

/// `(edge, highlight, shadow, face, pressed face, label colour, label shadow)`.
const fn parts(kind: Kind) -> (u32, u32, u32, u32, u32, u32, bool) {
    match kind {
        Kind::Stone => (
            color::STONE_EDGE,
            color::STONE_HI,
            color::STONE_LO,
            color::STONE_FACE,
            color::STONE_PRESSED,
            color::STONE_TEXT,
            false,
        ),
        Kind::Dark => (
            color::DARK_EDGE,
            color::DARK_HI,
            color::DARK_LO,
            color::DARK_FACE,
            color::DARK_PRESSED,
            color::TEXT,
            true,
        ),
        Kind::Green => (
            color::GREEN_EDGE,
            color::GREEN_HI,
            color::GREEN_LO,
            color::GREEN_FACE,
            color::GREEN_PRESSED,
            color::TEXT,
            true,
        ),
        Kind::Red => (
            color::RED_EDGE,
            color::RED_HI,
            color::RED_LO,
            color::RED_FACE,
            color::RED_PRESSED,
            color::TEXT,
            true,
        ),
        Kind::Panel => (
            color::PANEL_EDGE,
            color::PANEL_HI,
            color::PANEL_LO,
            color::PANEL,
            color::PANEL,
            color::TEXT,
            true,
        ),
    }
}

/// One solid rectangle; radius 0 keeps `Ui::rounded` on its single-quad path.
pub fn fill(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, color: u32) {
    if w > 0.0 && h > 0.0 {
        ui.rounded(x, y, w, h, 0.0, color);
    }
}

/// A bevelled plate: a 1dp dark edge, a 1dp highlight on the top/left, a 1dp
/// shadow on the bottom/right and the face — four quads. `pressed` flips the
/// bevel and darkens the face, exactly like vanilla's button under a finger.
pub fn bevel(ui: &mut Ui, rect: [f32; 4], kind: Kind, pressed: bool) {
    let [x, y, w, h] = rect;
    let (edge, hi, lo, face, pressed_face, _, _) = parts(kind);
    if w < 6.0 || h < 6.0 {
        fill(ui, x, y, w, h, if pressed { pressed_face } else { face });
        return;
    }
    let face = if pressed { pressed_face } else { face };
    let (c1, c2) = if pressed { (lo, hi) } else { (hi, lo) };
    fill(ui, x, y, w, h, edge);
    fill(ui, x + 1.0, y + 1.0, w - 2.0, h - 2.0, c1);
    fill(ui, x + 2.0, y + 2.0, w - 3.0, h - 3.0, c2);
    fill(ui, x + 2.0, y + 2.0, w - 4.0, h - 4.0, face);
}

/// Text with vanilla's hard drop shadow: one extra quad per glyph, reusing the
/// same cached atlas entry. For headings and light button labels only.
pub fn shadow_text(
    ui: &mut Ui,
    face: Face,
    size: f32,
    x: f32,
    y: f32,
    color: u32,
    text: &str,
) -> f32 {
    let off = (size / 8.0).round().max(1.0);
    ui.text(face, size, x + off, y + off, color::SHADOW, text);
    ui.text(face, size, x, y, color, text)
}

/// A bevelled button with its label centred, elided when too long, in the
/// pixel face at `font` dp. `pressed` shifts the label down a pixel like
/// vanilla's.
pub fn button(ui: &mut Ui, rect: [f32; 4], label: &str, kind: Kind, pressed: bool, font: f32) {
    let [x, y, w, h] = rect;
    bevel(ui, rect, kind, pressed);
    let (_, _, _, _, _, text, shadow) = parts(kind);
    let padding = 8.0;
    let label = ui.fit_text(Face::Pixel, font, label, (w - 2.0 * padding).max(8.0));
    let width = ui.measure(Face::Pixel, font, &label);
    let tx = x + ((w - width) / 2.0).max(padding);
    let ty = y + ((h - font * 1.45) / 2.0).max(0.0) + if pressed { 1.0 } else { 0.0 };
    if shadow {
        shadow_text(ui, Face::Pixel, font, tx, ty, text, &label);
    } else {
        ui.text(Face::Pixel, font, tx, ty, text, &label);
    }
}

/// A dialog panel: the panel bevel with no label.
pub fn panel(ui: &mut Ui, rect: [f32; 4]) {
    bevel(ui, rect, Kind::Panel, false);
}

/// The dim veil behind a dialog.
pub fn dim(ui: &mut Ui) {
    fill(ui, 0.0, 0.0, ui.w, ui.h, color::DIM);
}

/// A vanilla edit box: black inset, gray frame, left-aligned Geist text. The
/// caret is drawn by the page so it can blink with the launcher's clock.
pub fn field(ui: &mut Ui, rect: [f32; 4], label: &str, font: f32, active: bool) {
    let [x, y, w, h] = rect;
    fill(
        ui,
        x,
        y,
        w,
        h,
        if active {
            color::TEXT
        } else {
            color::FIELD_BORDER
        },
    );
    fill(ui, x + 1.0, y + 1.0, w - 2.0, h - 2.0, color::FIELD_BG);
    let text = ui.fit_text(Face::Regular, font, label, (w - 20.0).max(8.0));
    ui.text(
        Face::Regular,
        font,
        x + 10.0,
        y + ((h - font * 1.45) / 2.0).max(0.0),
        color::TEXT,
        &text,
    );
}

/// A list row's slot: one fill plus a 1px frame; selected rows take the white
/// frame vanilla puts around the chosen slot.
pub fn slot(ui: &mut Ui, rect: [f32; 4], selected: bool) {
    let [x, y, w, h] = rect;
    fill(ui, x, y, w, h, color::SLOT);
    ui.border(
        x,
        y,
        w,
        h,
        0.0,
        if selected {
            color::TEXT
        } else {
            color::SLOT_BORDER
        },
    );
}

/// A square checkbox; when checked it fills with `tint` (green, or red for a
/// destructive confirmation). No glyph textures needed.
pub fn check(ui: &mut Ui, rect: [f32; 4], on: bool, tint: u32) {
    let [x, y, w, h] = rect;
    fill(ui, x, y, w, h, color::FIELD_BORDER);
    fill(ui, x + 1.0, y + 1.0, w - 2.0, h - 2.0, color::FIELD_BG);
    if on {
        fill(ui, x + 4.0, y + 4.0, w - 8.0, h - 8.0, tint);
        fill(ui, x + 4.0, y + 4.0, w - 8.0, 2.0, color::CHECK_HI);
    }
}

/// A vanilla-style progress bar: framed track, green fill with a highlight
/// sliver on top.
pub fn progress(ui: &mut Ui, rect: [f32; 4], fraction: f32) {
    let [x, y, w, h] = rect;
    fill(ui, x, y, w, h, color::TRACK_BORDER);
    fill(ui, x + 1.0, y + 1.0, w - 2.0, h - 2.0, color::TRACK);
    let width = (w - 4.0) * fraction.clamp(0.0, 1.0);
    if width >= 1.0 {
        fill(ui, x + 2.0, y + 2.0, width, h - 4.0, color::GREEN_FACE);
        fill(ui, x + 2.0, y + 2.0, width, 2.0f32.min(h - 4.0), color::GREEN_HI);
    }
}

/// A flat scrollbar: dark track, stone thumb. Two quads.
pub fn scrollbar(ui: &mut Ui, x: f32, top: f32, height: f32, thumb_y: f32, thumb_h: f32) {
    fill(ui, x, top, 6.0, height, color::TRACK);
    fill(ui, x, thumb_y, 6.0, thumb_h, color::SCROLL_THUMB);
}

/// A one-pixel rule between settings rows.
pub fn divider(ui: &mut Ui, x: f32, y: f32, w: f32) {
    fill(ui, x, y, w, 1.0, color::DIVIDER);
}

/// Vanilla's keyboard focus: a white frame standing off the widget, with a
/// dark line outside it so it reads on stone buttons too.
pub fn focus_ring(ui: &mut Ui, rect: [f32; 4]) {
    let [x, y, w, h] = rect;
    ui.border(x - 4.0, y - 4.0, w + 8.0, h + 8.0, 0.0, color::PANEL_EDGE);
    ui.border(x - 2.0, y - 2.0, w + 4.0, h + 4.0, 0.0, color::TEXT);
    ui.border(x - 3.0, y - 3.0, w + 6.0, h + 6.0, 0.0, color::TEXT);
}

/// The little grass-block mark beside the launcher title: four quads, no
/// textures.
pub fn grass_mark(ui: &mut Ui, x: f32, y: f32, s: f32) {
    fill(ui, x, y, s, s, color::DIRT);
    fill(ui, x, y, s, (s * 0.35).max(2.0), color::GRASS_TOP);
    fill(
        ui,
        x + s * 0.16,
        y + s * 0.52,
        (s * 0.16).max(2.0),
        (s * 0.16).max(2.0),
        color::DIRT_DARK,
    );
    fill(
        ui,
        x + s * 0.62,
        y + s * 0.7,
        (s * 0.16).max(2.0),
        (s * 0.16).max(2.0),
        color::DIRT_DARK,
    );
}
