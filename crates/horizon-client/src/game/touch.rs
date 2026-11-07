//! Touch input (horizon_client::touch): on the playing screen the touch controls, drawn over the
//! HUD and turned into the virtual pad, mouse motion and the hotbar, laid out as the player's
//! touch-layout.json says (edited in place: a long press on pause); on every other screen the
//! first finger is the pointer, with a close button over the screens a world opens. Android's
//! back key is Escape, and the chat raises the on-screen keyboard and moves above it. On the
//! screens with slots a finger works them as Bedrock's touch inventory does (slots.rs). The player's
//! own buttons are in the layout as the game's own.
mod slots;
mod text_mirror;

use super::input_trace::InputOrigin;
use super::{Game, Screen, event_loop::EventLoopCtl};
use horizon_client::debug_keys::KeyAction;
use horizon_client::gamepad::ServerStick;
use horizon_client::input_mode::InputMode;
use horizon_client::{
    Result,
    gamepad::Button as Pad,
    hud::HudBatch,
    touch::{
        self, Action, Button, Chip, EditorAction, Interact, Layout, LayoutConfig, LayoutEditor,
        Shape, Show, Target, TouchControls,
    },
    ui_theme::{Face, Ui, color},
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use winit::{
    event::{ElementState, KeyEvent, TouchPhase, WindowEvent},
    keyboard::{Key, KeyCode, NamedKey},
    window::WindowId,
};

pub(super) const HELP: &[&str] = &[
    "touch-layout [edit|done|reset]  the touch controls' layout (touch-layout.json): show it, edit it in place, save, or go back to the default",
];

/// How long a tap holds the use trigger: vanilla samples input once a tick.
const USE_PULSE: Duration = Duration::from_millis(60);
/// How long the phone buzzes when the player breaks a block, and when a held finger splits a
/// stack (milliseconds).
pub(super) const BREAK_BUZZ: u32 = 25;
const SPLIT_BUZZ: u32 = 15;
/// The layout's file, beside options.txt.
pub(super) const LAYOUT_FILE: &str = "touch-layout.json";

/// A button of the player's own on the touch controls, as it is drawn.
#[derive(Clone, Debug)]
pub(super) struct ModFace {
    /// `<custom buttons>:<button id>`, its name in the layout.
    pub(super) key: String,
    pub(super) label: String,
    /// The atlas sprite drawn on it instead of the label.
    pub(super) icon: Option<String>,
    /// A sprite that is the whole button, and its pressed one (a custom button's look).
    pub(super) skin: Option<(String, String)>,
    pub(super) shape: Shape,
}

/// How one of the game's touch buttons looks: a pack's sprite, else drawn.
pub(super) struct ControlFace {
    pub(super) label: String,
    /// A resource pack's whole button: its sprite and its pressed sprite.
    pub(super) skin: Option<(String, Option<String>)>,
    pub(super) shape: Shape,
}

/// A wide button: 16 GUI pixels tall, as wide as its label, as vanilla's buttons.
const WIDE_HEIGHT: f32 = 16.0;

/// The layout in its file, for the title's options (no game yet).
fn saved_layout() -> LayoutConfig {
    std::fs::read_to_string(LAYOUT_FILE)
        .map_or_else(|_| LayoutConfig::default(), |t| LayoutConfig::from_json(&t))
}

/// The player's own buttons in the layout's file.
pub(super) fn saved_custom_buttons() -> Vec<horizon_client::settings::CustomButton> {
    saved_layout().custom
}

/// The player's own buttons into the layout's file.
pub(super) fn save_custom_buttons(buttons: Vec<horizon_client::settings::CustomButton>) {
    let mut layout = saved_layout();
    layout.set_custom(buttons);
    if let Err(e) = std::fs::write(LAYOUT_FILE, layout.to_json()) {
        log::warn!("Horizon: the touch layout was not saved: {e}");
    }
}

#[derive(Default)]
pub(super) struct Touch {
    controls: TouchControls,
    /// The finger acting as the mouse off the playing screen.
    pointer: touch::TouchPointer,
    /// The finger on a slot and the stack it picked (slots.rs).
    slots: slots::SlotTouch,
    /// A touch was seen (`has_touch`). The controls show while the input mode is the touch
    /// screen: from the start on Android, from this first touch elsewhere (game/input_mode.rs).
    seen: bool,
    /// The controls were live last frame (the playing screen).
    live: bool,
    use_until: Option<Instant>,
    /// A tap on a mob hits it: the attack trigger is held until then.
    attack_until: Option<Instant>,
    /// Where a tap landed, aimed along while its use or hit is held ([`USE_PULSE`]).
    tapped: Option<(Instant, [f32; 2])>,
    /// The press held on the view uses (a mob, the air) rather than breaks.
    hold_uses: bool,
    /// The gyroscope is listened to (the layout aims with it and the controls play).
    gyro_on: bool,
    /// The on-screen keyboard was asked for.
    keyboard: bool,
    /// The keyboard is wanted over the text field shown now: a typing screen opened, or a tap
    /// landed on the field; a tap beside it, the back key or the field going away end it.
    wanted: bool,
    /// The text field the keyboard was last decided for (`text_context`).
    context: Option<&'static str>,
    /// Where the last finger landed: lifted about there, it was a tap (`keyboard_after_tap`).
    tap_from: Option<[f32; 2]>,
    /// The keyboard has shown since it was asked for: its height falling to 0 then means the
    /// player put it away (the back key), which the game is not told otherwise.
    ime_seen: bool,
    /// The keyboard's editor on Android (text_mirror.rs).
    mirror: text_mirror::Mirror,
    /// The player's layout, read from touch-layout.json on the first frame.
    pub(super) config: Option<LayoutConfig>,
    /// The layout editor, while it is open.
    editor: Option<LayoutEditor>,
    /// The chat bar's button last tapped and when (it lights up a moment).
    chat_key: Option<(usize, Instant)>,
    /// The finger on the chat bar: its moves and lift are the bar's.
    chat_finger: Option<u64>,
    /// Fingers on the extra buttons: (finger, group, button id).
    extra_keys: Vec<(u64, String, String)>,
    /// The player's own buttons held now, by their name in the layout.
    custom_down: Vec<String>,
    /// The custom button a picture was asked for: the next one the inbox gets is its.
    import_image: Option<u32>,
    /// The mods' buttons as last offered to the layout.
    faces: Vec<ModFace>,
    /// The Sneak button holds sneak now (`touch_sync_sneak`).
    sneaking: bool,
}

/// The chat bar's keys, left to right: history up and down, completion, the cursor; then Paste.
const CHAT_KEYS: [KeyCode; 5] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::Tab,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
];

/// A pixel arrow centred on (cx, cy): a head 7 wide and a stem 3 wide. `dir`: 0 up, 1 down,
/// 2 left, 3 right.
fn arrow(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    c: [f32; 2],
    dir: u8,
    ink: u32,
) {
    let [cx, cy] = c;
    for k in 0..4 {
        let (k, w) = (k as f32, 2.0 * k as f32 + 1.0);
        let [x, y, w, h] = match dir {
            0 => [cx - k, cy - 3.0 + k, w, 1.0],
            1 => [cx - k, cy + 3.0 - k, w, 1.0],
            2 => [cx - 3.0 + k, cy - k, 1.0, w],
            _ => [cx + 3.0 - k, cy - k, 1.0, w],
        };
        batch.fill(atlas, x, y, w, h, ink);
    }
    let [x, y] = match dir {
        0 => [cx - 1.0, cy + 1.0],
        1 => [cx - 1.0, cy - 3.0],
        2 => [cx + 1.0, cy - 1.0],
        _ => [cx - 3.0, cy - 1.0],
    };
    batch.fill(atlas, x, y, 3.0, 3.0, ink);
}

/// A pixel clipboard centred on (cx, cy), for Paste: a board 9 by 11, its clip and two lines.
fn clipboard(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, c: [f32; 2], ink: u32) {
    let [cx, cy] = c;
    let (left, top) = (cx - 4.0, cy - 4.0);
    batch.fill(atlas, left, top, 1.0, 10.0, ink);
    batch.fill(atlas, left + 8.0, top, 1.0, 10.0, ink);
    batch.fill(atlas, left, top + 9.0, 9.0, 1.0, ink);
    batch.fill(atlas, left, top, 2.0, 1.0, ink);
    batch.fill(atlas, left + 7.0, top, 2.0, 1.0, ink);
    batch.fill(atlas, cx - 2.0, top - 1.0, 5.0, 2.0, ink);
    batch.fill(atlas, cx - 2.0, cy, 5.0, 1.0, ink);
    batch.fill(atlas, cx - 2.0, cy + 2.0, 5.0, 1.0, ink);
}

// The controls' looks, in window pixels: a plate, a pressed plate and a label at a control's
// opacity.
fn plate(a: f32) -> u32 {
    color::with_alpha(color::WHITE, 0.16 * a)
}
fn pressed(a: f32) -> u32 {
    color::with_alpha(color::WHITE, 0.38 * a)
}
fn ink(a: f32) -> u32 {
    color::with_alpha(color::WHITE, 0.85 * a)
}
fn circle(ui: &mut Ui, c: [f32; 2], r: f32, fill: u32) {
    let d = ui.density;
    ui.rounded(
        (c[0] - r) / d,
        (c[1] - r) / d,
        2.0 * r / d,
        2.0 * r / d,
        r / d,
        fill,
    );
}
fn label(ui: &mut Ui, c: [f32; 2], r: f32, text: &str, tint: u32) {
    let d = ui.density;
    let size = (r / d * 0.5).clamp(9.0, 16.0);
    ui.text_centered(
        Face::Semibold,
        size,
        c[0] / d,
        c[1] / d - size * 0.62,
        tint,
        text,
    );
}
/// A plate and an outline of any shape (circles as above).
fn plate_of(ui: &mut Ui, shape: Shape, c: [f32; 2], r: f32, fill: u32) {
    let d = ui.density;
    let [hw, hh] = shape.half(r);
    let corner = match shape {
        Shape::Square => r * 0.3,
        Shape::Round | Shape::Wide(_) => r,
    };
    ui.rounded(
        (c[0] - hw) / d,
        (c[1] - hh) / d,
        2.0 * hw / d,
        2.0 * hh / d,
        corner / d,
        fill,
    );
}
fn ring_of(ui: &mut Ui, shape: Shape, c: [f32; 2], r: f32, line: u32) {
    let d = ui.density;
    let [hw, hh] = shape.half(r);
    let corner = match shape {
        Shape::Square => r * 0.3,
        Shape::Round | Shape::Wide(_) => r,
    };
    ui.border(
        (c[0] - hw) / d,
        (c[1] - hh) / d,
        2.0 * hw / d,
        2.0 * hh / d,
        corner / d,
        line,
    );
}
/// A sprite over a button, `share` of its height, at an opacity.
fn icon(ui: &mut Ui, sprite: &str, c: [f32; 2], r: f32, share: f32, a: f32) {
    let gui = ui.batch.scale;
    let side = (2.0 * r * share / gui).round();
    let (x, y) = (
        (c[0] / gui - side / 2.0).round(),
        (c[1] / gui - side / 2.0).round(),
    );
    let tint = horizon_client::hud::rgba(255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8);
    horizon_client::ui::sprite_sized(ui.batch, ui.atlas, sprite, x, y, side, side, tint);
}
/// A label's size for a button's radius: a wide button's is its height's, not its width's.
fn text_r(shape: Shape, r: f32) -> f32 {
    match shape {
        Shape::Wide(_) => r * 1.5,
        Shape::Round | Shape::Square => r,
    }
}

/// The layout editor over whatever is behind it (the world, the title, the setup's settings):
/// every control where it will be, the selected one's chips, the opacity slider, and Done, Reset
/// and the movement control. `face` names a button or a mod's button (and gives the latter its
/// icon); `shape` is a button's; `skin` is a sprite drawn whole in a control's place (Starling's
/// look, packaging/touch, or a pack's).
pub(super) fn draw_editor(
    ui: &mut Ui,
    layout: &Layout,
    config: &LayoutConfig,
    editor: &LayoutEditor,
    face: &dyn Fn(Target) -> (String, Option<String>),
    shape: &dyn Fn(Button) -> Shape,
    skin: &dyn Fn(Target) -> Option<String>,
) {
    let d = ui.density;
    let (w, h) = (layout.width, layout.height);
    // The movement control as it moves the player now.
    let dpad = layout.dpad;
    ui.fill(0.0, 0.0, w / d, h / d, color::with_alpha(color::BG, 0.45));
    for (target, place, c, r) in LayoutEditor::targets(config, w, h) {
        // As it will look (at least faintly, to be found again), outlined if a ghost.
        let a = (place.opacity * config.opacity).max(0.35);
        let shape = match target {
            Target::Extra(i) => config.extra[i].shape,
            Target::Button(b) => shape(b),
            Target::Stick if dpad => Shape::Square,
            Target::Stick => Shape::Round,
        };
        // Drawn as it plays (a ghost only outlined, as it is not drawn while playing).
        if place.show != Show::Ghost
            && let Some(sprite) = skin(target)
        {
            let a = if place.show == Show::Off { 0.4 } else { a };
            icon(ui, &sprite, c, r, 1.0, a);
            if editor.selected == Some(target) {
                ring_of(ui, shape, c, r, color::ACCENT);
            }
            continue;
        }
        match place.show {
            Show::On => plate_of(ui, shape, c, r, pressed(a)),
            Show::Ghost => ring_of(ui, shape, c, r, ink(0.5)),
            Show::Off => plate_of(ui, shape, c, r, plate(0.5)),
        }
        if editor.selected == Some(target) {
            ring_of(ui, shape, c, r, color::ACCENT);
        }
        let (name, sprite) = match target {
            Target::Stick if dpad => ("D-pad".to_owned(), None),
            Target::Stick => ("Move".to_owned(), None),
            _ => face(target),
        };
        let tint = if place.show == Show::Off {
            ink(0.4)
        } else {
            ink(1.0)
        };
        // A mod's button with an icon shows it, as while playing (its name is on the opacity
        // slider once picked).
        let drawn = match target {
            Target::Extra(_) if !matches!(shape, Shape::Wide(_)) => sprite,
            _ => None,
        };
        match drawn {
            Some(sprite) => {
                let a = if place.show == Show::Off { 0.4 } else { 1.0 };
                icon(ui, &sprite, c, r, 0.6, a);
            }
            None => label(ui, c, text_r(shape, r), &name, tint),
        }
    }
    let shown = editor.selected.and_then(|t| {
        LayoutEditor::targets(config, w, h)
            .into_iter()
            .find(|(x, ..)| *x == t)
            .map(|(_, p, ..)| p.show)
    });
    for (chip, c, r) in editor.chips(config, w, h) {
        circle(ui, c, r, color::with_alpha(color::BG, 0.85));
        let text = match chip {
            Chip::Smaller => "-",
            Chip::Bigger => "+",
            Chip::Visibility => match shown {
                Some(Show::Ghost) => "Ghost",
                Some(Show::Off) => "Off",
                _ => "On",
            },
            Chip::Sneak => config.sneak.label(),
            Chip::Anchor if config.fixed_stick => "Fixed",
            Chip::Anchor => "Float",
            Chip::Delete => "Del",
        };
        label(ui, c, r, text, ink(1.0));
    }
    // The opacity slider: the selected control's, or everyone's.
    let ([x0, y], [x1, _]) = LayoutEditor::opacity_track(w, h);
    let value = editor.opacity(config);
    let u = layout.unit;
    ui.rounded(
        x0 / d,
        (y - 0.08 * u) / d,
        (x1 - x0) / d,
        0.16 * u / d,
        0.08 * u / d,
        color::with_alpha(color::BG, 0.85),
    );
    ui.rounded(
        x0 / d,
        (y - 0.08 * u) / d,
        (x1 - x0) * value / d,
        0.16 * u / d,
        0.08 * u / d,
        color::ACCENT,
    );
    circle(ui, [x0 + (x1 - x0) * value, y], 0.28 * u, ink(1.0));
    let whose = match editor.selected {
        Some(Target::Stick) => "Move".to_owned(),
        Some(Target::Button(b)) => b.label().to_owned(),
        Some(target @ Target::Extra(_)) => face(target).0,
        None => "all".to_owned(),
    };
    ui.text_centered(
        Face::Semibold,
        12.0,
        (x0 + x1) / 2.0 / d,
        (y - 0.75 * u) / d,
        ink(1.0),
        &format!("Opacity ({whose}): {:.0}%", value * 100.0),
    );
    for (action, c, r) in LayoutEditor::bar(w, h) {
        let fill = match action {
            EditorAction::Done => color::with_alpha(color::ACCENT, 0.9),
            EditorAction::Reset
            | EditorAction::Add
            | EditorAction::Movement
            | EditorAction::Interact
            | EditorAction::Vibrate
            | EditorAction::Gyro
            | EditorAction::ThreeFingers => color::with_alpha(color::BG, 0.85),
        };
        circle(ui, c, r, fill);
        let text = match action {
            EditorAction::Done => "Done",
            EditorAction::Reset => "Reset",
            EditorAction::Add => "Add",
            // The movement control picked (a press switches it): auto is the D-pad where the
            // stick moves as the keys.
            EditorAction::Movement => match config.dpad {
                Some(true) => "D-pad",
                Some(false) => "Stick",
                None => "Auto",
            },
            // What a tap and a press act on: what is touched, or the crosshair's.
            EditorAction::Interact => config.interact.label(),
            EditorAction::Vibrate if config.vibrate => "Vib on",
            EditorAction::Vibrate => "Vib off",
            EditorAction::Gyro if config.gyro => "Gyro on",
            EditorAction::Gyro => "Gyro off",
            EditorAction::ThreeFingers if config.three_fingers => "3-tap on",
            EditorAction::ThreeFingers => "3-tap off",
        };
        label(ui, c, r, text, ink(1.0));
    }
}

impl Game<'_> {
    /// The player plays with a touch screen: a touch was seen, and on Android no keyboard, mouse
    /// or controller took over since (game/input_mode.rs).
    pub(super) fn has_touch(&self) -> bool {
        self.touch.seen && self.touch_shown()
    }
    fn touch_playing(&self) -> bool {
        self.screen == Screen::Playing
            && self.title.is_none()
            && self.player.is_some()
            && !self.over_world_screen()
    }

    /// A screen drawn over the world without leaving the playing screen: the sign editor, a
    /// book, a command block, a dialog. Fingers are a pointer there, as on any other screen.
    fn over_world_screen(&self) -> bool {
        self.sign_edit.is_some()
            || self.book.is_some()
            || self.command_block_screen.is_some()
            || self.server_packs.prompting()
            || self.dialog_open()
    }

    /// A screen opened in a world that the back key or the close button leaves.
    fn touch_closable(&self) -> bool {
        self.title.is_none()
            && (!matches!(self.screen, Screen::Playing | Screen::Dead) || self.over_world_screen())
    }

    /// The touch controls turn the view as a grabbed pointer does (mouse.rs).
    pub(super) fn touch_turns(&self) -> bool {
        self.touch.live && self.touch.editor.is_none()
    }

    pub(super) fn touch_size(&self) -> [u32; 2] {
        self.window.as_ref().map_or([1, 1], |w| {
            let s = w.inner_size();
            [s.width.max(1), s.height.max(1)]
        })
    }

    fn touch_layout(&self) -> Layout {
        let [w, h] = self.touch_size();
        // The scale the HUD is drawn at, which the GUI Scale option only caps.
        let scale = HudBatch::new([w, h]).scale;
        let mut layout = match &self.touch.config {
            Some(config) => Layout::from_config(config, w as f32, h as f32, scale),
            None => Layout::new(w as f32, h as f32, scale),
        };
        layout.dpad = self.touch_dpad();
        layout
    }

    /// Whether the D-pad moves the player: as the layout picked, else when the stick moves as the
    /// keys on this server (their eight directions on screen).
    fn touch_dpad(&self) -> bool {
        let picked = self.touch.config.as_ref().and_then(|c| c.dpad);
        picked.unwrap_or(self.remote_stick_mode() == Some(ServerStick::Keys))
    }

    /// Taps and presses on the view act on what is under the finger now: tap to interact, the
    /// touch controls in play, in first person (the ray goes through the finger from the eyes).
    pub(super) fn touch_taps(&self) -> bool {
        self.touch.live
            && self.touch.editor.is_none()
            && self.touch_shown()
            && self.camera_type.first_person()
            && self
                .touch
                .config
                .as_ref()
                .is_none_or(|c| c.interact == Interact::Tap)
    }

    /// The window pixel tap to interact aims through: a finger that may still tap or holds, or
    /// a tap whose click is still held.
    fn touch_aim_point(&self) -> Option<[f32; 2]> {
        if !self.touch_taps() {
            return None;
        }
        self.touch.controls.aim().or_else(|| {
            self.touch
                .tapped
                .filter(|(until, _)| Instant::now() < *until)
                .map(|(_, p)| p)
        })
    }

    /// The unit ray from the eyes through the finger on the screen (the camera's own projection:
    /// its vertical field of view and the window's shape), for [`Game::aim`].
    pub(super) fn touch_aim_dir(&self) -> Option<[f64; 3]> {
        let p = self.touch_aim_point()?;
        let fov = self.locator_camera?.3;
        let [w, h] = self.touch_size().map(f64::from);
        let (sx, sy) = (
            2.0 * f64::from(p[0]) / w - 1.0,
            1.0 - 2.0 * f64::from(p[1]) / h,
        );
        let t = (f64::from(fov).to_radians() / 2.0).tan();
        let forward = glam::DVec3::from_array(super::view_vector(self.yaw, self.pitch));
        let yaw = f64::from(self.yaw).to_radians();
        // The screen's right (yaw 0 faces +z, its right is -x) and up.
        let right = glam::DVec3::new(-yaw.cos(), 0.0, -yaw.sin());
        let up = right.cross(forward);
        let dir = forward + right * (sx * t * w / h) + up * (sy * t);
        Some(dir.normalize().to_array())
    }

    /// The look along [`Game::touch_aim_dir`] (yaw near the player's own): what the server is
    /// told the player looks at and places with while a finger aims, so a block or mob acted on
    /// off the crosshair is in view to it (and its anticheat).
    pub(super) fn touch_aim_rotation(&self) -> Option<(f32, f32)> {
        let [x, y, z] = self.touch_aim_dir()?;
        let yaw = (-x).atan2(z).to_degrees() as f32;
        let pitch = (-y).clamp(-1.0, 1.0).asin().to_degrees() as f32;
        let turn = (yaw - self.yaw).rem_euclid(360.0);
        let turn = if turn > 180.0 { turn - 360.0 } else { turn };
        Some((self.yaw + turn, pitch))
    }

    /// A block, with no mob before it, under the finger (or the crosshair), that a press breaks:
    /// a content mod's crank too, which a tap turns (game/kinetics.rs).
    fn touch_on_block(&self) -> bool {
        self.pick_entity_hit().is_none() && self.pick().is_some()
    }

    /// Tap to interact has no crosshair, and outlines a block only while a finger aims at it.
    pub(super) fn touch_hides_crosshair(&self) -> bool {
        self.touch_taps()
    }
    pub(super) fn touch_hides_outline(&self) -> bool {
        self.touch_taps() && self.touch_aim_point().is_none()
    }

    /// A short buzz of the phone (Bedrock's "Vibrate when breaking blocks" and "when splitting
    /// items"): while the touch screen plays and the layout leaves vibration on.
    pub(super) fn touch_vibrate(&self, millis: u32) {
        let on = self.touch.config.as_ref().is_none_or(|c| c.vibrate);
        if !on || !self.has_touch() {
            return;
        }
        #[cfg(target_os = "android")]
        crate::android::vibrate(millis);
        #[cfg(not(target_os = "android"))]
        let _ = millis;
    }

    /// Once a frame: aiming by turning the phone, while the layout asks for it and the touch
    /// controls play (the sensor is listened to only then). Its turn goes on the view as the
    /// mouse's would (`Entity.turn`).
    fn touch_gyro(&mut self) {
        let wanted = self.touch.live
            && self.touch.editor.is_none()
            && self.has_touch()
            && self.player.is_some()
            && self.touch.config.as_ref().is_some_and(|c| c.gyro);
        #[cfg(target_os = "android")]
        {
            if wanted != self.touch.gyro_on {
                // A phone without a gyroscope says so; it is not asked again until the next change.
                let started = crate::android::gyroscope(wanted);
                self.touch.gyro_on = wanted;
                if wanted && !started {
                    log::info!("Horizon: no gyroscope to aim with");
                }
            }
            if !wanted {
                return;
            }
            let Some([x, y, rotation]) = crate::android::take_gyro() else {
                return;
            };
            let [yaw, pitch] = touch::gyro_turn([x, y], rotation as i32);
            if yaw == 0.0 && pitch == 0.0 {
                return;
            }
            let mut turn = horizon_client::mouse::Rotation {
                x_rot: self.pitch,
                y_rot: self.yaw,
                x_rot_o: self.pitch,
                y_rot_o: self.yaw,
            };
            turn.turn(f64::from(yaw / 0.15), f64::from(pitch / 0.15));
            self.yaw = turn.y_rot;
            self.pitch = turn.x_rot;
        }
        #[cfg(not(target_os = "android"))]
        {
            self.touch.gyro_on = wanted;
        }
    }

    fn touch_layout_path() -> Result<std::path::PathBuf> {
        Ok(std::env::current_dir()?.join(LAYOUT_FILE))
    }

    fn save_touch_layout(&self) {
        let Some(config) = &self.touch.config else {
            return;
        };
        let saved =
            Self::touch_layout_path().and_then(|path| Ok(std::fs::write(path, config.to_json())?));
        if let Err(e) = saved {
            log::warn!("Horizon: the touch layout was not saved: {e}");
        }
    }

    /// A touch event; true when it was taken (the caller passes the rest on).
    pub(super) fn touch_event(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        let WindowEvent::Touch(t) = event else {
            return false;
        };
        self.touch.seen = true;
        // The touch controls come back with a touch after a keyboard, a mouse or a controller.
        self.input_used(InputMode::Touch);
        self.last_input = Instant::now();
        // The offer of the Starling folder, over the title, takes the touches while it shows.
        #[cfg(target_os = "android")]
        if self.files_offer_touch(t) || self.update_offer_touch(t) || self.native_builds_touch(t) {
            return true;
        }
        let p = [t.location.x as f32, t.location.y as f32];
        if t.phase == TouchPhase::Started {
            self.touch.tap_from = Some(p);
        }
        // The chat bar: a tap is its key, and the finger stays the bar's until it lifts.
        if t.phase == TouchPhase::Started
            && let Some(i) = self.chat_bar_at(p)
        {
            self.touch.chat_key = Some((i, Instant::now()));
            self.touch.chat_finger = Some(t.id);
            match CHAT_KEYS.get(i) {
                Some(&key) => {
                    let _ = self.touch_key_tap(key);
                }
                None => self.chat_paste(),
            }
            return true;
        }
        if self.touch.chat_finger == Some(t.id) {
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.touch.chat_finger = None;
            }
            return true;
        }
        // The extra buttons: held while a finger is on them.
        if t.phase == TouchPhase::Started
            && let Some((id, bind)) = self.extra_button_at(p)
        {
            self.touch.extra_keys.push((t.id, id.clone(), bind.clone()));
            self.extra_button(&id, &bind, true);
            return true;
        }
        if let Some(i) = self.touch.extra_keys.iter().position(|(f, ..)| *f == t.id) {
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                let (_, id, bind) = self.touch.extra_keys.remove(i);
                self.extra_button(&id, &bind, false);
            }
            return true;
        }
        if self.touch.editor.is_some() && (self.touch_playing() || self.title.is_some()) {
            let [w, h] = self.touch_size();
            let config = self.touch.config.get_or_insert_with(LayoutConfig::default);
            let editor = self.touch.editor.as_mut().unwrap();
            let action = editor.event(config, w as f32, h as f32, t.id, t.phase, p);
            if matches!(action, Some(EditorAction::Done | EditorAction::Add)) {
                self.touch.editor = None;
                self.save_touch_layout();
            }
            if action == Some(EditorAction::Add) {
                self.open_options_at(
                    self.title.is_some(),
                    horizon_client::options_ui::Page::Controls,
                );
                self.open_custom_buttons();
                self.ui.options.add_custom_button(&mut self.settings);
            }
            return true;
        }
        let layout = self.touch_layout();
        if t.phase == TouchPhase::Started
            && !self.touch.pointer.down()
            && self.touch_closable()
            && layout.close_at(p)
        {
            let _ = self.touch_key_tap(KeyCode::Escape);
            return true;
        }
        if !self.touch_playing()
            && !self.touch.pointer.owns(t.id)
            && self.slot_touch_event(event_loop, id, t)
        {
            self.keyboard_after_tap(t, p);
            return true;
        }
        // The mouse finger stays the mouse until it lifts, even if the screen closed under it.
        if !self.touch_playing() || self.touch.pointer.owns(t.id) {
            let slop = touch::slop(layout.width, layout.height);
            // A finger on a scrollbar drags it, as the mouse would.
            if t.phase == TouchPhase::Started && self.on_scrollbar(p) {
                self.touch.pointer.prefer_drag();
            }
            // Lists follow the finger one to one at this GUI scale.
            let gui_scale = HudBatch::new(self.touch_size()).scale;
            self.touch
                .pointer
                .set_gui_scale(gui_scale, self.settings.mouse_wheel_sensitivity);
            for mouse in self.touch.pointer.event(t, slop, Instant::now()) {
                self.handle_window_event(event_loop, id, mouse, InputOrigin::User);
            }
            self.keyboard_after_tap(t, p);
            return true;
        }
        self.touch.controls.three_fingers =
            self.touch.config.as_ref().is_some_and(|c| c.three_fingers);
        let actions = self
            .touch
            .controls
            .event(&layout, t.id, t.phase, p, Instant::now());
        self.touch_apply(actions);
        true
    }

    /// Android's back key: Escape (pause, close a screen), or Done in the layout editor.
    pub(super) fn touch_back_key(&mut self, event: &KeyEvent) -> bool {
        if event.logical_key != Key::Named(NamedKey::BrowserBack) {
            return false;
        }
        if event.state == ElementState::Pressed && !event.repeat {
            if self.touch.editor.take().is_some() {
                self.save_touch_layout();
            } else {
                let _ = self.touch_key_tap(KeyCode::Escape);
            }
        }
        true
    }

    /// Once per loop iteration, before the pads are read.
    pub(super) fn touch_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        let now = Instant::now();
        if self.touch.config.is_none() {
            let text = Self::touch_layout_path()
                .ok()
                .and_then(|path| std::fs::read_to_string(path).ok());
            self.touch.config =
                Some(text.map_or_else(LayoutConfig::default, |t| LayoutConfig::from_json(&t)));
        }
        // The mods' buttons, where the player put them (or their mods do).
        if self.touch_shown() {
            let (faces, offered): (Vec<_>, Vec<_>) = self
                .custom_touch_buttons()
                .into_iter()
                .map(|(face, place)| {
                    let offer = (face.key.clone(), place, face.shape);
                    (face, offer)
                })
                .unzip();
            if let Some(config) = &mut self.touch.config
                && (!offered.is_empty() || config.extra.iter().any(|e| e.live))
            {
                config.offer(&offered);
            }
            self.touch.faces = faces;
        }
        if let Some(id) = self.window.as_ref().map(|w| w.id()) {
            for mouse in self.touch.pointer.frame_events(now) {
                self.handle_window_event(event_loop, id, mouse, InputOrigin::User);
            }
            self.slot_touch_frame(event_loop, id);
        }
        let live = self.touch_playing();
        if self.touch.live && !live {
            // A screen opened over the world: let go of everything the fingers held.
            let actions = self.touch.controls.reset();
            self.touch_apply(actions);
            self.custom_buttons_up();
            if self.touch.editor.take().is_some() {
                self.save_touch_layout();
            }
        }
        self.touch.live = live;
        // Fingers resting on the controls send no events: still input for the idle frame cap
        // (Inactivity FPS Limit: AFK), as a held key would be.
        if self.touch.controls.touching() || self.touch.pointer.down() {
            self.last_input = now;
        }
        let actions = self.touch.controls.frame(now);
        self.touch_apply(actions);
        if self.touch.use_until.is_some_and(|t| now >= t) {
            self.touch.use_until = None;
            self.pad.touch_trigger(0, 0.0);
        }
        if self.touch.attack_until.is_some_and(|t| now >= t) {
            self.touch.attack_until = None;
            self.pad.touch_trigger(1, 0.0);
        }
        if self.touch.tapped.is_some_and(|(t, _)| now >= t) {
            self.touch.tapped = None;
        }
        self.touch_gyro();
        // The on-screen keyboard (its keys arrive as key events; on Android an editor behind it
        // holds the field, text_mirror.rs). The screens made for typing
        // (chat, sign, book, command block) open it with them; the other text fields on a tap on
        // them. Android shows it only to a focused window: back from the background, it is asked
        // for again once the focus is back.
        let context = self.text_context();
        if context != self.touch.context {
            self.touch.context = context;
            self.touch.wanted =
                matches!(context, Some("chat" | "sign" | "book" | "command" | "mod"));
            // The finger that opened it lifts on the new screen: that is not a tap beside its field.
            if self.touch.wanted {
                self.touch.tap_from = None;
            }
        }
        #[cfg(target_os = "android")]
        if self.touch.keyboard {
            if crate::android::ime_height() > 0 {
                self.touch.ime_seen = true;
            } else if self.touch.ime_seen {
                // Put away with the back key: a tap on the field brings it back.
                self.touch.wanted = false;
                self.touch.ime_seen = false;
            }
        }
        let keyboard = self.touch_shown() && self.touch.wanted && self.focused;
        // On Android the keyboard's editor shows it (text_mirror.rs).
        #[cfg(target_os = "android")]
        if self.window.is_some() {
            self.android_keyboard(keyboard);
        }
        #[cfg(not(target_os = "android"))]
        if keyboard != self.touch.keyboard
            && let Some(window) = &self.window
        {
            window.set_ime_allowed(keyboard);
        }
        if keyboard != self.touch.keyboard && self.window.is_some() {
            self.touch.keyboard = keyboard;
            self.touch.ime_seen = false;
        }
    }

    /// A tap decides the keyboard wherever it landed, a slot too: on the text field it comes up,
    /// beside it it goes away. A finger that moved (a list scrolling) leaves it as it is.
    fn keyboard_after_tap(&mut self, t: &winit::event::Touch, p: [f32; 2]) {
        if t.phase != TouchPhase::Ended {
            return;
        }
        let Some(from) = self.touch.tap_from.take() else {
            return;
        };
        let layout = self.touch_layout();
        let slop = touch::slop(layout.width, layout.height);
        if (p[0] - from[0]).hypot(p[1] - from[1]) >= slop {
            return;
        }
        let Some(context) = self.text_context() else {
            return;
        };
        let batch = HudBatch::new(self.touch_size());
        let gui = (p[0] / batch.scale, p[1] / batch.scale);
        let cursor = std::mem::replace(&mut self.cursor, gui);
        let on = self.on_text_field();
        self.cursor = cursor;
        let wanted = match (context, on) {
            // Its line, above the keyboard, keeps it; a tap on the rest puts it away, and with it
            // away any tap brings it back.
            ("chat", _) => {
                let lift = self.keyboard_height() as f32 / batch.scale;
                !self.touch.wanted || gui.1 + lift >= batch.height - 16.0
            }
            (_, Some(on)) => on,
            // Its place isn't known: the tap that focused it opens it, then a tap goes either way.
            (_, None) => self.touch.context != Some(context) || !self.touch.wanted,
        };
        self.touch.wanted = wanted;
        // A tap that just focused the field (the options' boxes start without the focus): the frame
        // must not take it for a new screen's and put the keyboard away.
        if wanted {
            self.touch.context = Some(context);
        }
    }

    /// Whether a text field that holds the focus shows it: with fingers, while the keyboard is up.
    pub(super) fn text_caret_shown(&self) -> bool {
        !self.touch_shown() || self.touch.wanted
    }

    /// The text field shown now, by name: the chat, a sign, a book, a command block, the anvil's
    /// name, the creative and recipe searches, the title's forms, the options' searches, the
    /// world options' number boxes.
    fn text_context(&mut self) -> Option<&'static str> {
        if self.screen == Screen::Chat {
            Some("chat")
        } else if self.sign_edit.is_some() {
            Some("sign")
        } else if matches!(self.book, Some(super::books::BookScreen::Edit { .. })) {
            Some("book")
        } else if self.command_block_screen.is_some() {
            Some("command")
        } else if self.station_typing() {
            Some("anvil")
        } else if self.screen == Screen::Creative && self.creative_screen.tab().search {
            Some("creative")
        } else if self.recipe_search_focused() {
            Some("recipe")
        } else if self.title.as_mut().is_some_and(|t| t.text_focused()) {
            Some("title")
        } else if self.screen == Screen::Options && self.ui.options.text_focused() {
            Some("options")
        } else if self.screen == Screen::WorldOptions && self.world_options_typing() {
            // The open world's game rules and the LAN port: boxes of digits.
            Some("world")
        } else {
            None
        }
    }

    /// Whether the cursor (where a tap just landed) is on the text field shown now; None where
    /// the field's place is not known.
    fn on_text_field(&mut self) -> Option<bool> {
        let (size, cursor) = (self.gui_size(), self.cursor);
        let inside = |r: [f32; 4]| {
            cursor.0 >= r[0] && cursor.0 < r[0] + r[2] && cursor.1 >= r[1] && cursor.1 < r[1] + r[3]
        };
        match self.text_context()? {
            "title" => self.title.as_mut().map(|t| t.text_field_at(cursor, size)),
            "options" => Some(self.ui.options.text_field_at(cursor.0, cursor.1)),
            // The tap's click focused the box under it, or none.
            "world" => Some(self.world_options_typing()),
            context => self.text_field_rect(context).map(inside),
        }
    }

    /// The text field's box in GUI pixels, a little larger for a finger, on the screens that
    /// draw one at a known place (vanilla's layouts).
    fn text_field_rect(&self, context: &str) -> Option<[f32; 4]> {
        let batch = HudBatch::new(self.touch_size());
        let w = batch.width;
        match context {
            // SignEditScreen and HangingSignEditScreen's board, around (w / 2, 90..125).
            "sign" => Some([w / 2.0 - 50.0, 60.0, 100.0, 110.0]),
            // BookEditScreen's page text, inside the 192x192 book at (w / 2 - 96, 2).
            "book" => Some([w / 2.0 - 66.0, 26.0, 132.0, 140.0]),
            // CommandBlockEditScreen's command box.
            "command" => Some([w / 2.0 - 150.0, 50.0, 300.0, 20.0]),
            // AnvilScreen's name box (62, 24), 103 wide.
            "anvil" => {
                let (ox, oy) = self.inventory_origin(&batch);
                Some([ox + 60.0, oy + 21.0, 107.0, 16.0])
            }
            // CreativeModeInventoryScreen's search box (82, 6), 80 wide.
            "creative" => {
                let (ox, oy) = Self::creative_origin(&batch);
                Some([ox + 80.0, oy + 3.0, 86.0, 14.0])
            }
            _ => None,
        }
    }

    /// Whether a finger landing at window pixel `p` is on a scrollbar's track, which it should
    /// drag like a held mouse instead of scrolling the list.
    fn on_scrollbar(&self, p: [f32; 2]) -> bool {
        let batch = HudBatch::new(self.touch_size());
        let (x, y) = (p[0] / batch.scale, p[1] / batch.scale);
        // A custom button's canvas: a finger paints, it doesn't scroll.
        if self.screen == Screen::Options && self.ui.options.painting() {
            return true;
        }
        // The title screen's player turns under the finger whichever way it moves.
        if self.on_title_player((x, y)) {
            return true;
        }
        if self.screen == Screen::Creative {
            let (ox, oy) = Self::creative_origin(&batch);
            return self.creative_screen.inside_scrollbar(x - ox, y - oy);
        }
        false
    }

    fn touch_apply(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::Stick(stick) => self.pad.touch_stick(stick),
                // Held as the left stick's click (key.sprint) while the stick sprints.
                Action::Sprint(on) => self.pad.touch_hold(Pad::LeftStick, on),
                // Relative motion as a grabbed mouse makes it (MouseHandler.onMove).
                Action::Look(dx, dy) => self.mouse.add_motion(dx as f64, dy as f64),
                Action::Press(Button::Jump) => self.pad.touch_hold(Pad::South, true),
                Action::Release(Button::Jump) => self.pad.touch_hold(Pad::South, false),
                Action::Press(Button::Attack) => self.pad.touch_trigger(1, 1.0),
                Action::Release(Button::Attack) => self.pad.touch_trigger(1, 0.0),
                // Held on a block it breaks it; on a mob or the air it is the right button
                // held (interact, eat, draw a bow), as Bedrock's press.
                Action::HoldStart(_) => {
                    self.touch.hold_uses = !self.touch_on_block();
                    let trigger = if self.touch.hold_uses { 0 } else { 1 };
                    self.pad.touch_trigger(trigger, 1.0);
                }
                Action::HoldStop => {
                    let trigger = if self.touch.hold_uses { 0 } else { 1 };
                    self.touch.hold_uses = false;
                    self.pad.touch_trigger(trigger, 0.0);
                }
                Action::Press(Button::Use) => {
                    self.touch.use_until = None;
                    self.pad.touch_trigger(0, 1.0);
                }
                Action::Release(Button::Use) => self.pad.touch_trigger(0, 0.0),
                // Sneak holds the key itself, in its own mode (touch_sync_sneak); the pad's
                // buttons for the rest (docs/controllers.md).
                Action::Press(Button::Sneak) => {}
                Action::Press(Button::Inventory) => self.pad.touch_tap(Pad::North),
                Action::Press(Button::Chat) => self.pad.touch_tap(Pad::DPadRight),
                Action::Press(Button::Pause) => self.pad.touch_tap(Pad::Start),
                Action::Press(Button::Drop) => self.pad.touch_tap(Pad::East),
                Action::Press(Button::Offhand) => self.pad.touch_tap(Pad::West),
                Action::Press(Button::Camera) => self.pad.touch_tap(Pad::DPadUp),
                Action::Press(Button::Command) => {
                    // key.command: the chat opened with a slash.
                    let _ = self.touch_key_tap(KeyCode::Slash);
                }
                // F3 held as the key while the finger is on it (KeyboardHandler: its release
                // toggles the debug screen).
                Action::Press(Button::Debug) => {
                    self.physical_keys.insert(KeyCode::F3);
                    self.debug_key(KeyCode::F3, KeyAction::Press);
                }
                Action::Release(Button::Debug) => {
                    self.physical_keys.remove(&KeyCode::F3);
                    self.debug_key(KeyCode::F3, KeyAction::Release);
                }
                // A combination's key pressed and let go while F3 is held: KeyboardHandler runs it,
                // and F3's release then leaves the debug screen as it was.
                Action::DebugCombo(i) => {
                    let binding = touch::DEBUG_COMBOS[i].0;
                    if let Some(key) =
                        horizon_client::keybinds::key_code(self.settings.keys.get(binding))
                    {
                        self.physical_keys.insert(key);
                        self.debug_key(key, KeyAction::Press);
                        self.physical_keys.remove(&key);
                        self.debug_key(key, KeyAction::Release);
                    }
                }
                // F3 is used now: its release leaves the debug screen as it was.
                Action::FunctionKeys => self.debug_keys.used_debug_key_as_modifier = true,
                Action::FunctionKey(i) => {
                    const KEYS: [KeyCode; touch::FUNCTION_KEYS.len()] = [
                        KeyCode::F1,
                        KeyCode::F2,
                        KeyCode::F4,
                        KeyCode::F5,
                        KeyCode::F6,
                        KeyCode::F7,
                        KeyCode::F8,
                        KeyCode::F9,
                        KeyCode::F10,
                        KeyCode::F11,
                        KeyCode::F12,
                    ];
                    let _ = self.touch_key_tap(KEYS[i]);
                }
                Action::Press(Button::Pick) => self.pad.touch_tap(Pad::DPadLeft),
                // Held as the pad's View (key.playerlist) while the finger is on it.
                Action::Press(Button::Players) => self.pad.touch_hold(Pad::Select, true),
                Action::Release(Button::Players) => self.pad.touch_hold(Pad::Select, false),
                Action::Release(_) => {}
                Action::Tap(p) => {
                    let now = Instant::now();
                    // Tap to interact aims where it landed while the click is held.
                    if self.touch_taps() {
                        self.touch.tapped = Some((now + USE_PULSE * 2, p));
                    }
                    // A mob under the finger is hit, as Bedrock's tap; the rest is used.
                    if self.touch_taps() && self.pick_entity_hit().is_some() {
                        self.pad.touch_trigger(1, 1.0);
                        self.touch.attack_until = Some(now + USE_PULSE);
                    } else {
                        self.pad.touch_trigger(0, 1.0);
                        self.touch.use_until = Some(now + USE_PULSE);
                    }
                }
                Action::Slot(slot) => self.selected = slot,
                Action::ToggleStats => self.toggle_stats_overlay(),
                Action::EditLayout => self.open_touch_editor(),
            }
        }
        self.touch_sync_sneak();
    }

    /// Whether the touch Sneak button holds sneak now (`sync_toggle` adds it to the key's state).
    pub(super) fn touch_sneaking(&self) -> bool {
        self.touch.sneaking
    }

    /// Hold or let go of sneak as the Sneak button says, in its mode and with the player's flight:
    /// only on the playing screen, as vanilla lets go of the keys under a screen and gives a
    /// toggled one back when it closes.
    fn touch_sync_sneak(&mut self) {
        let flying = self.player.as_ref().is_some_and(|p| p.flying);
        let mode = self
            .touch
            .config
            .as_ref()
            .map(|c| c.sneak)
            .unwrap_or_default();
        self.touch.controls.set_sneak(mode, flying);
        // The death screen: KeyMapping.resetToggleKeys when the player respawns.
        if self.screen == Screen::Dead {
            self.touch.controls.clear_sneak();
        }
        let sneaking = self.touch_playing() && self.touch.controls.sneaking();
        // Also when something let go of the keys under it (the window losing focus).
        let lost = sneaking && !self.keys.contains(&KeyCode::ShiftLeft);
        if sneaking != self.touch.sneaking || lost {
            let pressed = sneaking && !self.touch.sneaking;
            self.touch.sneaking = sneaking;
            self.sync_toggle(0);
            // As a press of the key reaches the game (it gets a sleeper up).
            if pressed {
                let _ = self.key_pressed(KeyCode::ShiftLeft);
            }
        }
    }

    pub(super) fn open_touch_editor(&mut self) {
        let actions = self.touch.controls.reset();
        self.touch_apply(actions);
        self.touch.editor = Some(LayoutEditor::default());
    }

    /// The controls over the HUD while playing (or the layout editor), the close button over
    /// screens.
    pub(super) fn draw_touch(&self, batch: &mut HudBatch) {
        if !self.touch_shown() {
            return;
        }
        let (playing, closable) = (self.touch_playing(), self.touch_closable());
        // F1 hides the buttons with the HUD; they still work, and F3 held shows them again.
        if playing && self.ui.hide_gui && !self.touch.controls.held(Button::Debug) {
            return;
        }
        // The editor also over the title, before a world.
        if !playing && !closable && self.touch.editor.is_none() {
            return;
        }
        let layout = self.touch_layout();
        let mut ui = Ui::new(&self.gui.atlas, batch, (-1.0, -1.0));
        // A mod's button: its plate, its icon or its label.
        let face_of = |key: &str| self.touch.faces.iter().find(|f| f.key == key);
        if closable {
            let (c, r) = layout.close;
            circle(&mut ui, c, r, color::with_alpha(color::BG, 0.7));
            label(&mut ui, c, r, "X", ink(1.0));
        } else if let (Some(editor), Some(config)) = (&self.touch.editor, &self.touch.config) {
            let face = |target: Target| match target {
                Target::Button(b) => (self.control_face(b).label, None),
                Target::Extra(i) => {
                    let key = &config.extra[i].key;
                    let face = face_of(key);
                    (
                        face.map_or_else(|| key.clone(), |f| f.label.clone()),
                        face.and_then(|f| f.icon.clone()),
                    )
                }
                Target::Stick => (String::new(), None),
            };
            let shape = |b: Button| self.control_face(b).shape;
            let skin = |target: Target| match target {
                Target::Button(b) => self.control_face(b).skin.map(|(sprite, _)| sprite),
                Target::Stick if !layout.dpad => self.mod_sprite("starling:touch/stick"),
                Target::Extra(i) => {
                    let skin = face_of(&config.extra[i].key).and_then(|f| f.skin.as_ref());
                    skin.map(|(sprite, _)| sprite.clone())
                }
                Target::Stick => None,
            };
            draw_editor(&mut ui, &layout, config, editor, &face, &shape, &skin);
        } else {
            let (base, knob) = self
                .touch
                .controls
                .stick
                .unwrap_or((layout.stick_home, layout.stick_home));
            // A ghost stick shows only while a finger holds it.
            let a = if self.touch.controls.stick.is_some() {
                layout.stick_alpha.max(0.6)
            } else {
                layout.stick_alpha
            };
            // Starling's own look (packaging/touch), or a pack's, when the atlas has it.
            let skin = |name: &str| self.mod_sprite(&format!("starling:touch/{name}"));
            if layout.dpad {
                // Bedrock's D-pad: four arrows, the one held lit; the middle holds nothing yet.
                let a = layout.stick_alpha.max(0.35);
                let held = self.touch.controls.dpad;
                for (cell, c, half) in layout.dpad_buttons() {
                    let on = held == Some(cell);
                    let (text, name) = match cell {
                        [0, -1] => ("↑", "dpad_up"),
                        [-1, 0] => ("←", "dpad_left"),
                        [1, 0] => ("→", "dpad_right"),
                        _ => ("↓", "dpad_down"),
                    };
                    let lit = skin(&format!("{name}_pressed")).filter(|_| on);
                    if let Some(sprite) = lit.or_else(|| skin(name)) {
                        icon(&mut ui, &sprite, c, half, 1.0, a);
                        continue;
                    }
                    plate_of(
                        &mut ui,
                        Shape::Square,
                        c,
                        half,
                        if on { pressed(a) } else { plate(a) },
                    );
                    label(&mut ui, c, half, text, ink(a));
                }
            } else if a > 0.0 {
                // The knob half the frame's size, as Bedrock's (and Starling's own) sprites are.
                match (skin("stick"), skin("stick_knob")) {
                    (Some(ring), Some(top)) => {
                        icon(&mut ui, &ring, base, layout.stick_radius, 1.0, a);
                        icon(&mut ui, &top, knob, layout.stick_radius * 0.5, 1.0, a);
                    }
                    _ => {
                        circle(&mut ui, base, layout.stick_radius, plate(a));
                        circle(&mut ui, knob, layout.stick_radius * 0.5, pressed(a));
                    }
                }
            }
            // Where the finger aims (tap to interact), as Horizon Android's (Vasysik, MIT): a
            // white ring outlined dark, and while it breaks a block, a whole disc growing inside
            // it to the rim as the block cracks (its size, never its angle, shows the progress).
            if let Some(p) = self.touch_aim_point() {
                let rgba = horizon_client::hud::rgba;
                let gui = ui.batch.scale;
                let radius = 16.0 * gui;
                ring_of(
                    &mut ui,
                    Shape::Round,
                    p,
                    radius + 0.7 * gui,
                    rgba(0, 0, 0, 110),
                );
                ring_of(&mut ui, Shape::Round, p, radius, rgba(255, 255, 255, 210));
                let breaking = self.touch.controls.holding() && !self.touch.hold_uses;
                let progress = self.mining.as_ref().map_or(0.0, |m| m.progress);
                let inner = (radius - 1.5 * gui).max(0.0) * progress.clamp(0.0, 1.0);
                if breaking && inner > 0.0 {
                    circle(&mut ui, p, inner, rgba(255, 255, 255, 90));
                    ring_of(&mut ui, Shape::Round, p, inner, rgba(255, 255, 255, 220));
                }
            }
            let sneaking = self.player.as_ref().is_some_and(|p| p.shift_key_down);
            for (button, c, r, a) in &layout.buttons {
                if *a <= 0.0 {
                    continue;
                }
                let on =
                    self.touch.controls.held(*button) || (*button == Button::Sneak && sneaking);
                let face = self.control_face(*button);
                // A pack's button is drawn whole, else it is a plate with its label.
                if let Some((skin, lit)) = &face.skin {
                    let sprite = lit.as_ref().filter(|_| on).unwrap_or(skin);
                    icon(&mut ui, sprite, *c, *r, 1.0, *a);
                    continue;
                }
                let fill = if on { pressed(*a) } else { plate(*a) };
                plate_of(&mut ui, face.shape, *c, *r, fill);
                label(&mut ui, *c, *r, &face.label, ink(*a));
            }
            // F3 held: its combinations, for another finger or the F3 finger sliding onto one.
            if self.touch.controls.held(Button::Debug) {
                let other = self.touch.controls.function_keys;
                let chips = if other {
                    layout.function_chips()
                } else {
                    layout.debug_chips()
                };
                for (i, c, half) in chips {
                    let shape = Shape::Wide(half[0] / half[1]);
                    plate_of(
                        &mut ui,
                        shape,
                        c,
                        half[1],
                        color::with_alpha(color::BG, 0.85),
                    );
                    label(
                        &mut ui,
                        c,
                        half[1] * 1.6,
                        if other {
                            touch::FUNCTION_KEYS[i]
                        } else {
                            touch::DEBUG_COMBOS.get(i).map_or("F keys", |c| c.1)
                        },
                        ink(1.0),
                    );
                }
            }
            // The player's own buttons, lit while held.
            for (i, c, r, a, shape) in &layout.extra {
                let extra = self.touch.config.as_ref().and_then(|c| c.extra.get(*i));
                let (Some(extra), true) = (extra.map(|e| e.key.clone()), *a > 0.0) else {
                    continue;
                };
                let Some(face) = face_of(&extra) else {
                    continue;
                };
                let on = self.touch.custom_down.contains(&extra);
                if let Some((skin, lit)) = &face.skin {
                    // One image for both: dimmed while held.
                    let held = if on && skin == lit { *a * 0.6 } else { *a };
                    icon(&mut ui, if on { lit } else { skin }, *c, *r, 1.0, held);
                    continue;
                }
                let fill = if on { pressed(*a) } else { plate(*a) };
                plate_of(&mut ui, *shape, *c, *r, fill);
                match (&face.icon, shape) {
                    (Some(sprite), Shape::Round | Shape::Square) => {
                        icon(&mut ui, sprite, *c, *r, 0.6, *a)
                    }
                    _ => label(&mut ui, *c, text_r(*shape, *r), &face.label, ink(*a)),
                }
            }
        }
        ui.finish();
    }

    /// Let go of everything the fingers held (the app went to the background, or the player took
    /// up a keyboard, a mouse or a controller).
    pub(super) fn touch_release_all(&mut self) {
        let actions = self.touch.controls.reset();
        self.touch_apply(actions);
        // No window events here: the screen gets them back when the app returns.
        let _ = self.touch.pointer.cancel();
        // The gyroscope rests in the background; the next frame that aims listens again.
        #[cfg(target_os = "android")]
        if std::mem::take(&mut self.touch.gyro_on) {
            crate::android::gyroscope(false);
        }
        // The system put the keyboard away: ask for it again when the chat is back.
        self.touch.keyboard = false;
    }

    /// The extra button under window pixel `p` while playing (not in the layout editor): its
    /// group, button id.
    fn extra_button_at(&self, p: [f32; 2]) -> Option<(String, String)> {
        if !self.touch_playing() || self.touch.editor.is_some() {
            return None;
        }
        let config = self.touch.config.as_ref()?;
        let i = self.touch_layout().extra_at(p)?;
        let (id, button) = config.extra.get(i)?.key.split_once(':')?;
        Some((id.to_owned(), button.to_owned()))
    }
    /// Whether the chat bar shows: the chat open on a touch screen.
    fn chat_bar_shown(&self) -> bool {
        self.screen == Screen::Chat && self.touch_shown()
    }
    /// The chat bar's buttons in GUI pixels (x, y, w, h): at the right, just above the text
    /// field (lifted with it over the keyboard).
    fn chat_bar(batch: &HudBatch) -> [[f32; 4]; 6] {
        let (w, h, gap) = (24.0, 16.0, 2.0);
        let y = batch.height - 14.0 - h - 2.0;
        let left = batch.width - 2.0 - 6.0 * w - 5.0 * gap;
        std::array::from_fn(|i| [left + i as f32 * (w + gap), y, w, h])
    }
    /// The chat bar's button under window pixel `p`.
    fn chat_bar_at(&self, p: [f32; 2]) -> Option<usize> {
        if !self.chat_bar_shown() {
            return None;
        }
        let batch = HudBatch::new(self.touch_size());
        let lift = self.keyboard_height() as f32 / batch.scale;
        let (x, y) = (p[0] / batch.scale, p[1] / batch.scale + lift);
        Self::chat_bar(&batch)
            .iter()
            .position(|r| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3])
    }
    /// The chat bar: vanilla buttons for ↑ ↓ Tab ← → and Paste, the one just tapped lit.
    pub(super) fn draw_chat_bar(&self, batch: &mut HudBatch) {
        if !self.chat_bar_shown() {
            return;
        }
        let atlas = &self.gui.atlas;
        let lit = self
            .touch
            .chat_key
            .filter(|(_, at)| at.elapsed() < Duration::from_millis(150))
            .map(|(i, _)| i);
        let ink = horizon_client::hud::rgba(255, 255, 255, 255);
        for (i, r) in Self::chat_bar(batch).iter().enumerate() {
            let sprite = if lit == Some(i) {
                "gui/sprites/widget/button_highlighted"
            } else {
                "gui/sprites/widget/button"
            };
            horizon_client::ui::nine_slice(batch, atlas, sprite, r[0], r[1], r[2], r[3], u32::MAX);
            let c = [(r[0] + r[2] / 2.0).floor(), (r[1] + r[3] / 2.0).floor()];
            match i {
                2 => {
                    let tw = HudBatch::text_width(atlas, "Tab");
                    batch.text(
                        atlas,
                        "Tab",
                        (c[0] - tw / 2.0).floor() + 1.0,
                        c[1] - 4.0,
                        ink,
                        true,
                    );
                }
                0 => arrow(batch, atlas, c, 0, ink),
                1 => arrow(batch, atlas, c, 1, ink),
                3 => arrow(batch, atlas, c, 2, ink),
                4 => arrow(batch, atlas, c, 3, ink),
                _ => clipboard(batch, atlas, c, ink),
            }
        }
    }
    /// How far the on-screen keyboard covers the window, in pixels (Android).
    fn keyboard_height(&self) -> u32 {
        #[cfg(target_os = "android")]
        if self.touch.keyboard {
            return crate::android::ime_height();
        }
        0
    }

    /// With the chat open over the on-screen keyboard, the HUD moves up above it.
    pub(super) fn lift_over_keyboard(&self, batch: &mut HudBatch) {
        let height = self.keyboard_height();
        if self.screen != Screen::Chat || height == 0 {
            return;
        }
        let lift = height as f32 / batch.scale;
        for vertex in &mut batch.vertices {
            vertex.pos[1] -= lift;
        }
        for (_, blend) in &mut batch.blends {
            if let horizon_client::hud::Blend::Depth(rect) = blend {
                rect[1] = rect[1].saturating_sub(height);
            }
        }
    }

    /// `touch-layout`: the layout, or open, close (saving) or reset its editor.
    /// `touch down|move|up|tap X Y [FINGER]`: a finger at a GUI pixel, as the touch screen's.
    pub(super) fn touch_input_command(
        &mut self,
        words: &[&str],
        event_loop: &dyn EventLoopCtl,
    ) -> Result<Option<Value>> {
        if words.first() != Some(&"touch") {
            return Ok(None);
        }
        let number = |i: usize| -> Result<f64> {
            let word = words.get(i).ok_or("touch down|move|up|tap X Y [FINGER]")?;
            Ok(word
                .parse::<f64>()
                .map_err(|_| format!("{word} is not a number"))?)
        };
        let phases: &[TouchPhase] = match words.get(1).copied() {
            Some("down") => &[TouchPhase::Started],
            Some("move") => &[TouchPhase::Moved],
            Some("up") => &[TouchPhase::Ended],
            Some("tap") => &[TouchPhase::Started, TouchPhase::Ended],
            _ => return Err("touch down|move|up|tap X Y [FINGER]".into()),
        };
        let scale = HudBatch::new(self.touch_size()).scale;
        let location = winit::dpi::PhysicalPosition::new(
            number(2)? * f64::from(scale),
            number(3)? * f64::from(scale),
        );
        let finger = if words.len() > 4 {
            number(4)? as u64
        } else {
            0
        };
        let id = self
            .window
            .as_ref()
            .map(|w| w.id())
            .ok_or("no window yet")?;
        for &phase in phases {
            let touch = winit::event::Touch {
                device_id: winit::event::DeviceId::dummy(),
                phase,
                location,
                force: None,
                id: finger,
            };
            self.handle_window_event(event_loop, id, WindowEvent::Touch(touch), InputOrigin::User);
        }
        Ok(Some(json!({ "ok": true, "screen": self.screen_label() })))
    }

    pub(super) fn touch_command(&mut self, words: &[&str]) -> Result<Option<Value>> {
        if words.first() != Some(&"touch-layout") {
            return Ok(None);
        }
        match words.get(1).copied() {
            None => {}
            Some("edit") => self.open_touch_editor(),
            Some("done") => {
                if self.touch.editor.take().is_some() {
                    self.save_touch_layout();
                }
            }
            Some("reset") => {
                self.touch.config = Some(LayoutConfig::default());
                self.save_touch_layout();
            }
            Some(other) => {
                return Err(format!("touch-layout takes edit, done or reset, not {other}").into());
            }
        }
        let config = self.touch.config.clone().unwrap_or_default();
        let layout: Value = serde_json::from_str(&config.to_json()).unwrap_or(Value::Null);
        Ok(Some(json!({
            "ok": true,
            "editing": self.touch.editor.is_some(),
            "selected": self.touch.editor.as_ref().and_then(|e| e.selected).map(|t| match t {
                Target::Stick => "stick".to_owned(),
                Target::Button(b) => b.id().to_owned(),
                Target::Extra(i) => config.extra.get(i).map_or_else(String::new, |e| e.key.clone()),
            }),
            "layout": layout,
        })))
    }

    /// A finger landed on or lifted from one of the layout's extra buttons: one of the player's
    /// own.
    fn extra_button(&mut self, id: &str, bind: &str, finger: bool) {
        use horizon_client::settings::CustomAction;
        if id != touch::CUSTOM_BUTTONS {
            return;
        }
        let held = format!("{id}:{bind}");
        let button = self
            .touch
            .config
            .as_ref()
            .and_then(|c| c.custom.iter().find(|b| b.id.to_string() == bind))
            .cloned();
        // A toggle flips with each press, and its finger lifting does nothing.
        let toggle = button.as_ref().is_some_and(|b| b.toggle);
        if toggle && !finger {
            return;
        }
        let on = if toggle {
            !self.touch.custom_down.contains(&held)
        } else {
            finger
        };
        self.touch.custom_down.retain(|k| *k != held);
        if on {
            self.touch.custom_down.push(held);
        }
        match button.map(|b| b.action) {
            Some(CustomAction::Send(line)) if finger => self.handle_chat_input(&line),
            Some(action) => self.custom_key(&action, on),
            None => {}
        }
    }

    /// The custom buttons on let go, as a screen opening lets go of every key.
    fn custom_buttons_up(&mut self) {
        for held in std::mem::take(&mut self.touch.custom_down) {
            let action = self.touch.config.as_ref().and_then(|c| {
                let id = held
                    .strip_prefix(touch::CUSTOM_BUTTONS)?
                    .strip_prefix(':')?;
                let button = c.custom.iter().find(|b| b.id.to_string() == id)?;
                Some(button.action.clone())
            });
            if let Some(action) = action {
                self.custom_key(&action, false);
            }
        }
    }

    /// A custom button's key goes down or up.
    fn custom_key(&mut self, action: &horizon_client::settings::CustomAction, pressed: bool) {
        use horizon_client::settings::CustomAction;
        let key = match action {
            CustomAction::Send(_) => return,
            CustomAction::Key(name) => {
                horizon_client::keybinds::key_code(self.settings.keys.get(name))
            }
            CustomAction::Input(name) => horizon_client::keybinds::key_code(name),
        };
        let Some(key) = key else { return };
        // F3 acts on its release, through its own handler (the game's F3 button too).
        if key == KeyCode::F3 {
            let state = if pressed {
                self.physical_keys.insert(key);
                KeyAction::Press
            } else {
                self.physical_keys.remove(&key);
                KeyAction::Release
            };
            self.debug_key(key, state);
        } else if pressed {
            let _ = self.control_key_down(key);
        } else {
            self.keys.remove(&key);
            self.physical_keys.remove(&key);
        }
    }

    /// The drawing page's Import PNG: the system's picker, whose file comes through the inbox.
    pub(super) fn pick_button_image(&mut self, id: u32) {
        if horizon_client::platform::pick_file() {
            self.touch.import_image = Some(id);
        } else {
            self.imports.say_failed("No file picker opened".into());
        }
    }

    /// The canvas waiting for the picture it asked for: what the inbox gets goes on it, whatever
    /// its size, and never through the imports. True while it waits; off the canvas it no longer
    /// does.
    pub(super) fn button_image_wait(&mut self) -> bool {
        if self.touch.import_image.is_none() {
            return false;
        }
        if self.screen != Screen::Options || !self.ui.options.painting() {
            self.touch.import_image = None;
            return false;
        }
        let Some(picture) = horizon_client::import::take_picture() else {
            return true;
        };
        self.touch.import_image = None;
        match picture {
            Ok((w, h, pixels)) => {
                self.ui.options.import_button_image(w, h, &pixels);
                self.imports
                    .say("Picture imported", "On the canvas: Done keeps it".into());
            }
            Err(e) => self.imports.say_failed(e),
        }
        true
    }

    /// The Custom Buttons page opened: it shows what the layout has.
    pub(super) fn open_custom_buttons(&mut self) {
        let config = self.touch.config.as_ref();
        self.settings.custom_buttons = config.map(|c| c.custom.clone()).unwrap_or_default();
    }

    /// The page changed the player's buttons: the layout takes them, and forgets the places of
    /// those that went.
    pub(super) fn save_custom_buttons(&mut self) {
        let buttons = self.settings.custom_buttons.clone();
        let config = self.touch.config.get_or_insert_with(LayoutConfig::default);
        config.set_custom(buttons);
        self.save_touch_layout();
    }

    /// A key press and release from the touch side (the back key, the close button).
    fn touch_key_tap(&mut self, key: KeyCode) -> Result<()> {
        self.control_key_down(key)?;
        self.keys.remove(&key);
        self.physical_keys.remove(&key);
        Ok(())
    }
}

/// A custom button's size until it is placed, on a window this big.
pub(super) fn custom_button_size(w: f32, h: f32) -> f32 {
    let scale = HudBatch::new([w as u32, h as u32]).scale;
    WIDE_HEIGHT / 2.0 * scale / touch::unit(w, h)
}

impl Game<'_> {
    /// The atlas key of a GUI sprite: `<namespace>:my_icon` (a content mod's or a pack's,
    /// `assets/<namespace>/textures/gui/sprites/my_icon.png`) or a plain path.
    pub(super) fn mod_sprite(&self, sprite: &str) -> Option<String> {
        let atlas = &self.gui.atlas;
        let (ns, path) = sprite.split_once(':').unwrap_or(("minecraft", sprite));
        [
            format!("gui/sprites/{path}"),
            format!("{ns}:gui/sprites/{path}"),
            sprite.to_owned(),
        ]
        .into_iter()
        .filter(|k| ns == "minecraft" || !k.starts_with("gui/"))
        .find(|k| atlas.sprites.contains_key(k))
    }

    /// The player's own buttons on the touch controls: wide, what they do on them, in a column
    /// until they are placed.
    pub(super) fn custom_touch_buttons(&self) -> Vec<(ModFace, touch::Place)> {
        let Some(config) = &self.touch.config else {
            return Vec::new();
        };
        let [w, h] = self.touch_size();
        let size = custom_button_size(w as f32, h as f32);
        config
            .custom
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let label: String = b.label().chars().take(14).collect();
                let width = HudBatch::text_width(&self.gui.atlas, &label) + 10.0;
                let sprite = |name: &str| self.mod_sprite(&format!("starling:touch/{name}"));
                let own = |slot: u32| sprite(&format!("own_{slot}"));
                let icon = |end: &str| {
                    let icon = b.icon.as_deref()?;
                    sprite(&format!("custom_{icon}{end}"))
                };
                let up = if b.image { own(b.id) } else { icon("") };
                // Its pressed image; else its own image, dim while pressed, or the icon's sprite.
                let down = if b.pressed {
                    own(horizon_client::own_buttons::pressed_slot(b.id))
                } else if b.image {
                    own(b.id)
                } else {
                    icon("_pressed")
                };
                let skin = match (up, down) {
                    (Some(up), Some(down)) => Some((up, down)),
                    (None, Some(down)) => sprite("custom_blank").map(|up| (up, down)),
                    _ => None,
                };
                let face = ModFace {
                    key: format!("{}:{}", touch::CUSTOM_BUTTONS, b.id),
                    label,
                    icon: None,
                    shape: if skin.is_some() {
                        Shape::Square
                    } else {
                        Shape::Wide(width / WIDE_HEIGHT)
                    },
                    skin,
                };
                let place = touch::Place {
                    x: 0.5,
                    y: 0.25 + 0.1 * (i % 5) as f32,
                    size: if face.skin.is_some() { 0.5 } else { size },
                    show: Show::On,
                    opacity: 1.0,
                };
                (face, place)
            })
            .collect()
    }

    /// How one of the game's touch buttons looks now.
    pub(super) fn control_face(&self, button: Button) -> ControlFace {
        let name = button.id();
        let skin = self.mod_sprite(&format!("starling:touch/{name}")).map(|s| {
            let pressed = self.mod_sprite(&format!("starling:touch/{name}_pressed"));
            (s, pressed)
        });
        ControlFace {
            label: button.label().to_owned(),
            skin,
            shape: Shape::Round,
        }
    }
}
