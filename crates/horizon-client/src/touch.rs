//! Touch controls for phones (horizon-android): a stick that walks (floating under the finger,
//! or fixed with its knob going to the finger), dragging anywhere else that turns the view (a tap there uses the held item, a still press breaks), buttons for
//! every other action (jump, sneak, attack, use, inventory, chat, commands, pause, camera, drop,
//! offhand, the debug screen), and the hotbar's slots to tap. Each finger is claimed by one
//! control when it lands and keeps it until it lifts.
//!
//! As Bedrock's control modes, the taps and presses act on what is under the finger ("tap to
//! interact", [`Interact::Tap`], the default: no crosshair, no Attack and Use buttons) or on what
//! the crosshair is on ("aim crosshair", [`Interact::Crosshair`]).
//!
//! Where the controls are is a [`LayoutConfig`] the player edits in place ([`LayoutEditor`]):
//! a long press on pause opens it. It is kept as JSON beside options.txt (touch-layout.json).
//! Mods add buttons of their own ([`Extra`]), placed and edited as the game's.
//!
//! This is the state machine only: positions are window pixels, and the game turns the
//! [`Action`]s into its virtual pad, mouse motion and hotbar (game/touch.rs). Off the playing
//! screen the first finger is the pointer ([`TouchPointer`]).
use crate::settings::{CustomAction, CustomButton};
use serde_json::{Map, Value, json};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{
        DeviceId, ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent,
    },
};

/// A press that stays put this long breaks the block in view (Bedrock's touch default).
pub const HOLD: Duration = Duration::from_millis(300);
/// Holding pause this long opens the layout editor instead.
pub const EDIT_HOLD: Duration = Duration::from_millis(800);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Jump,
    Sneak,
    Inventory,
    Chat,
    Pause,
    /// Held: key.attack (hit what is in view, break blocks).
    Attack,
    /// Held: key.use (eat, draw a bow, raise a shield, place).
    Use,
    Drop,
    Offhand,
    /// key.togglePerspective.
    Camera,
    /// key.command: the chat with a slash.
    Command,
    /// key.debug.overlay (F3): held as the key, so a tap shows or hides the debug screen. While
    /// it is held, its combinations ([`DEBUG_COMBOS`]) show beside it.
    Debug,
    /// key.pickItem: the block (or mob) in view into the hand (Bedrock's Pick Block button).
    Pick,
    /// key.playerlist, held: the list of the server's players.
    Players,
}

/// F3's combinations offered while the F3 button is held: the key binding pressed with it and
/// what the chip says.
pub const DEBUG_COMBOS: [(&str, &str); 7] = [
    ("key.debug.showChunkBorders", "Chunks"),
    ("key.debug.showHitboxes", "Hitboxes"),
    ("key.debug.showAdvancedTooltips", "Tooltips"),
    ("key.debug.copyLocation", "Copy pos"),
    ("key.debug.reloadResourcePacks", "Packs"),
    ("key.debug.spectate", "Spectate"),
    ("key.debug.switchGameMode", "Modes"),
];

/// The other F keys, shown when a finger reaches the chip after F3's combinations.
pub const FUNCTION_KEYS: [&str; 11] = [
    "F1 HUD",
    "F2 Print",
    "F4",
    "F5 Camera",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
];

impl Button {
    pub const ALL: [Button; 14] = [
        Button::Jump,
        Button::Sneak,
        Button::Inventory,
        Button::Chat,
        Button::Pause,
        Button::Attack,
        Button::Use,
        Button::Drop,
        Button::Offhand,
        Button::Camera,
        Button::Command,
        Button::Debug,
        Button::Pick,
        Button::Players,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Button::Jump => "Jump",
            Button::Sneak => "Sneak",
            Button::Inventory => "Bag",
            Button::Chat => "Chat",
            Button::Pause => "II",
            Button::Attack => "Hit",
            Button::Use => "Use",
            Button::Drop => "Drop",
            Button::Offhand => "Swap",
            Button::Camera => "Cam",
            Button::Command => "/",
            Button::Debug => "F3",
            Button::Pick => "Pick",
            Button::Players => "List",
        }
    }
    /// Its name in touch-layout.json.
    pub fn id(self) -> &'static str {
        match self {
            Button::Jump => "jump",
            Button::Sneak => "sneak",
            Button::Inventory => "inventory",
            Button::Chat => "chat",
            Button::Pause => "pause",
            Button::Attack => "attack",
            Button::Use => "use",
            Button::Drop => "drop",
            Button::Offhand => "offhand",
            Button::Camera => "camera",
            Button::Command => "command",
            Button::Debug => "debug",
            Button::Pick => "pick",
            Button::Players => "players",
        }
    }
    /// Only for aiming the crosshair: tap to interact has no Attack and Use buttons (Bedrock's
    /// "Joystick & tap to interact" shows none).
    fn aims(self) -> bool {
        matches!(self, Button::Attack | Button::Use)
    }
}

/// What a tap and a still press on the view act on: Bedrock's "tap to interact" (what is under
/// the finger: a tap hits a mob or uses a block, a press breaks the block or interacts with the
/// mob) or "aim crosshair" (what the crosshair is on: a tap uses, a press breaks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Interact {
    #[default]
    Tap,
    Crosshair,
}

impl Interact {
    /// Its name in touch-layout.json.
    pub fn id(self) -> &'static str {
        match self {
            Interact::Tap => "tap",
            Interact::Crosshair => "crosshair",
        }
    }
    fn from_id(id: &str) -> Option<Self> {
        [Interact::Tap, Interact::Crosshair]
            .into_iter()
            .find(|i| i.id() == id)
    }
    /// What the editor's button says: what is touched, or what is aimed at (a press interacts in
    /// both, so not "tap").
    pub fn label(self) -> &'static str {
        match self {
            Interact::Tap => "Touch",
            Interact::Crosshair => "Aim",
        }
    }
    fn next(self) -> Self {
        match self {
            Interact::Tap => Interact::Crosshair,
            Interact::Crosshair => Interact::Tap,
        }
    }
}

/// Whether a control shows and works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    On,
    /// Not drawn, still works where it is (for players who know where it is).
    Ghost,
    /// Neither drawn nor working.
    Off,
}

impl Show {
    pub fn id(self) -> &'static str {
        match self {
            Show::On => "on",
            Show::Ghost => "ghost",
            Show::Off => "off",
        }
    }
    fn from_id(id: &str) -> Option<Self> {
        [Show::On, Show::Ghost, Show::Off]
            .into_iter()
            .find(|s| s.id() == id)
    }
    /// The editor's next state; the stick is never off.
    fn next(self, stick: bool) -> Self {
        match self {
            Show::On => Show::Ghost,
            Show::Ghost if stick => Show::On,
            Show::Ghost => Show::Off,
            Show::Off => Show::On,
        }
    }
}

/// How the Sneak button sneaks: while a finger is on it, or from one tap to the next (as it always
/// did). Flying, it only sneaks (goes down) while held either way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SneakMode {
    Hold,
    #[default]
    Toggle,
}

impl SneakMode {
    /// Its name in touch-layout.json.
    pub fn id(self) -> &'static str {
        match self {
            SneakMode::Hold => "hold",
            SneakMode::Toggle => "toggle",
        }
    }
    fn from_id(id: &str) -> Option<Self> {
        [SneakMode::Hold, SneakMode::Toggle]
            .into_iter()
            .find(|m| m.id() == id)
    }
    /// What the editor's chip says.
    pub fn label(self) -> &'static str {
        match self {
            SneakMode::Hold => "Hold",
            SneakMode::Toggle => "Toggle",
        }
    }
    /// The other one: the chip switches between them.
    fn next(self) -> Self {
        match self {
            SneakMode::Hold => SneakMode::Toggle,
            SneakMode::Toggle => SneakMode::Hold,
        }
    }
}

/// Where one control is: its centre as a share of the window's width and height (so a layout
/// fits any screen), its radius in units (a ninth of the shorter side), whether it shows, and
/// how opaque it is drawn (times the layout's own opacity).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub show: Show,
    pub opacity: f32,
}

const fn place(x: f32, y: f32, size: f32) -> Place {
    Place {
        x,
        y,
        size,
        show: Show::On,
        opacity: 1.0,
    }
}

/// A button off until the player turns it on in the editor (as Bedrock's optional ones).
const fn place_off(x: f32, y: f32, size: f32) -> Place {
    Place {
        show: Show::Off,
        ..place(x, y, size)
    }
}

/// The lowest opacity the editor sets: an invisible control is a ghost.
pub const MIN_OPACITY: f32 = 0.1;
/// The smallest a control gets, and a mod's button (a keybind's is as tall as a vanilla button,
/// less than that on a large screen).
const MIN_SIZE: f32 = 0.3;
const MIN_EXTRA_SIZE: f32 = 0.2;

/// How a button is shaped, which is also where a finger presses it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// A circle, as the game's own buttons.
    Round,
    /// A rounded square, as Bedrock's.
    Square,
    /// This many times as wide as it is tall: a button with a label.
    Wide(f32),
}

impl Shape {
    /// Half its width and height, for its radius (half its height).
    pub fn half(self, r: f32) -> [f32; 2] {
        match self {
            Shape::Wide(aspect) => [r * aspect.max(1.0), r],
            Shape::Round | Shape::Square => [r, r],
        }
    }
    /// Whether `p` presses a button of this shape centred on `c`, a little past its edge too.
    pub fn hits(self, c: [f32; 2], r: f32, p: [f32; 2]) -> bool {
        match self {
            Shape::Round => dist(p, c) <= r * 1.15,
            _ => {
                let [hw, hh] = self.half(r);
                (p[0] - c[0]).abs() <= hw * 1.15 && (p[1] - c[1]).abs() <= hh * 1.15
            }
        }
    }
}

/// The name of the player's own buttons among the mods' in the layout: `<this>:<id>`.
pub const CUSTOM_BUTTONS: &str = "starling.custom";

/// A button a mod put on the touch controls (one of its keybinds, or its own).
#[derive(Clone, Debug, PartialEq)]
pub struct Extra {
    /// `<mod id>:<button id>`: its name in touch-layout.json.
    pub key: String,
    pub place: Place,
    pub shape: Shape,
    /// Its mod runs and offers it now; otherwise it only keeps its place for when it does.
    pub live: bool,
}

/// The player's layout: the stick's resting place and every button's (the mods' too), and the
/// opacity of them all.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutConfig {
    pub stick: Place,
    /// Bedrock's D-pad in the stick's place (four arrows, the corners between them diagonals,
    /// up tapped twice to sprint) instead of the stick: always, never, or when none was picked
    /// (`None`) on a server where the stick moves as the keys. Its middle holds nothing yet.
    pub dpad: Option<bool>,
    pub buttons: Vec<(Button, Place)>,
    pub extra: Vec<Extra>,
    pub opacity: f32,
    /// How the Sneak button sneaks.
    pub sneak: SneakMode,
    /// What taps and presses on the view act on.
    pub interact: Interact,
    /// The stick stays where it rests ([`Layout::fixed_stick`]).
    pub fixed_stick: bool,
    /// The phone buzzes a moment when a block breaks and when a stack is split (Bedrock's
    /// "Vibrate when breaking blocks" and "when splitting items", both on by default).
    pub vibrate: bool,
    /// Turning the phone turns the view too ([`gyro_turn`]), as PojavLauncher's gyroscope.
    pub gyro: bool,
    /// Three fingers landing together show or hide the Stats overlay. Off by default: many
    /// play with three fingers.
    pub three_fingers: bool,
    /// Buttons of the player's own; their places are among `extra`.
    pub custom: Vec<CustomButton>,
}

/// The view's turn, in degrees right and down, for what the gyroscope turned (radians about
/// the device's own x and y, those of its natural portrait) on a display at `rotation`
/// (Surface.ROTATION_0..3: 1 with the phone's top to the left). The phone is a window on the
/// world: turned left about the screen's up, the view turns left; its top tilted toward the
/// player (its back to the sky), the view looks up.
pub fn gyro_turn(turned: [f32; 2], rotation: i32) -> [f32; 2] {
    let [x, y] = turned;
    // Rotation about the screen's right and up axes.
    let (right, up) = match rotation.rem_euclid(4) {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    };
    [-up.to_degrees(), -right.to_degrees()]
}

impl Default for LayoutConfig {
    /// Laid out on a 20:9 phone: the stick at the bottom left, jump, attack, use and sneak at the
    /// bottom right, the menus along the top, drop and offhand beside the hotbar.
    fn default() -> Self {
        LayoutConfig {
            stick: place(0.12, 0.733, 1.3),
            dpad: None,
            buttons: vec![
                (Button::Jump, place(0.905, 0.7556, 0.8)),
                (Button::Sneak, place(0.805, 0.8556, 0.6)),
                (Button::Attack, place(0.815, 0.611, 0.6)),
                (Button::Use, place(0.92, 0.533, 0.6)),
                (Button::Inventory, place(0.95, 0.111, 0.55)),
                (Button::Chat, place(0.88, 0.111, 0.55)),
                (Button::Command, place(0.81, 0.111, 0.5)),
                (Button::Pause, place(0.5, 0.0833, 0.45)),
                (Button::Camera, place(0.43, 0.0833, 0.45)),
                (Button::Debug, place(0.57, 0.0833, 0.45)),
                (Button::Drop, place(0.711, 0.951, 0.4)),
                (Button::Offhand, place(0.289, 0.951, 0.4)),
                (Button::Pick, place_off(0.83, 0.4, 0.5)),
                (Button::Players, place_off(0.36, 0.0833, 0.45)),
            ],
            extra: Vec::new(),
            opacity: 1.0,
            sneak: SneakMode::Toggle,
            interact: Interact::Tap,
            fixed_stick: false,
            vibrate: true,
            gyro: false,
            three_fingers: false,
            custom: Vec::new(),
        }
    }
}

impl LayoutConfig {
    /// touch-layout.json; controls it does not name keep their default place (a layout from
    /// before a button existed), and names it does not know are skipped.
    pub fn from_json(text: &str) -> Self {
        let mut config = LayoutConfig::default();
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return config;
        };
        let read = |v: &Value, into: &mut Place, min_size: f32| {
            let num = |k: &str, d: f32| v[k].as_f64().map_or(d, |n| n as f32);
            // The first layouts said `visible` before ghosts existed.
            let legacy = v["visible"]
                .as_bool()
                .map(|on| if on { Show::On } else { Show::Off });
            *into = Place {
                x: num("x", into.x).clamp(0.0, 1.0),
                y: num("y", into.y).clamp(0.0, 1.0),
                size: num("size", into.size).clamp(min_size, 2.5),
                show: v["show"]
                    .as_str()
                    .and_then(Show::from_id)
                    .or(legacy)
                    .unwrap_or(into.show),
                opacity: num("opacity", into.opacity).clamp(MIN_OPACITY, 1.0),
            };
        };
        if let Some(opacity) = value["opacity"].as_f64() {
            config.opacity = (opacity as f32).clamp(MIN_OPACITY, 1.0);
        }
        // "stick" was what an unpicked layout saved before "auto".
        config.dpad = match value["movement"].as_str() {
            Some("dpad") => Some(true),
            Some("analog") => Some(false),
            _ => None,
        };
        // Layouts from before the choice toggle, as Sneak did then.
        config.sneak = value["sneak"]
            .as_str()
            .and_then(SneakMode::from_id)
            .unwrap_or_default();
        // Layouts from before the choice get Bedrock's default too.
        config.interact = value["interact"]
            .as_str()
            .and_then(Interact::from_id)
            .unwrap_or_default();
        config.fixed_stick = value["anchor"].as_str() == Some("fixed");
        config.vibrate = value["vibrate"].as_bool().unwrap_or(true);
        config.gyro = value["gyro"].as_bool().unwrap_or(false);
        config.three_fingers = value["three_fingers"].as_bool().unwrap_or(false);
        if value["stick"].is_object() {
            read(&value["stick"], &mut config.stick, MIN_SIZE);
            if config.stick.show == Show::Off {
                config.stick.show = Show::On;
            }
        }
        for (button, place) in &mut config.buttons {
            if value["buttons"][button.id()].is_object() {
                read(&value["buttons"][button.id()], place, MIN_SIZE);
            }
        }
        for v in value["custom"].as_array().into_iter().flatten().take(64) {
            let text = |key: &str| v[key].as_str().map(str::to_owned);
            let Some(action) = text("key")
                .map(CustomAction::Key)
                .or_else(|| text("input").map(CustomAction::Input))
                .or_else(|| text("send").map(CustomAction::Send))
            else {
                continue;
            };
            config.custom.push(CustomButton {
                id: v["id"].as_u64().unwrap_or(0) as u32,
                action,
                icon: text("icon"),
                image: v["image"].as_bool().unwrap_or(false),
                pressed: v["pressed"].as_bool().unwrap_or(false),
                toggle: v["toggle"].as_bool().unwrap_or(false),
            });
        }
        // The mods' buttons, those of mods not running now too (they keep their places).
        if let Some(mods) = value["mods"].as_object() {
            for (key, v) in mods.iter().filter(|(_, v)| v.is_object()).take(256) {
                let mut place = place(0.5, 0.5, 0.5);
                read(v, &mut place, MIN_EXTRA_SIZE);
                config.extra.push(Extra {
                    key: key.clone(),
                    place,
                    shape: Shape::Round,
                    live: false,
                });
            }
        }
        config
    }

    /// The player's own buttons where no world offers them (the editor over the title), `size`
    /// big, in a column until they are placed.
    pub fn offer_custom(&mut self, size: f32) {
        let offered: Vec<_> = self
            .custom
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let place = Place {
                    x: 0.5,
                    y: 0.25 + 0.1 * (i % 5) as f32,
                    size,
                    show: Show::On,
                    opacity: 1.0,
                };
                (format!("{CUSTOM_BUTTONS}:{}", b.id), place, Shape::Round)
            })
            .collect();
        self.offer(&offered);
    }

    /// The id of the custom button `target` is, if it is one.
    fn custom_id(&self, target: Target) -> Option<u32> {
        let Target::Extra(i) = target else {
            return None;
        };
        let key = &self.extra.get(i)?.key;
        key.strip_prefix(CUSTOM_BUTTONS)?
            .strip_prefix(':')?
            .parse()
            .ok()
    }

    /// The player's own buttons change: those that went take their places with them.
    pub fn set_custom(&mut self, buttons: Vec<CustomButton>) {
        self.extra.retain(|e| {
            let id = e
                .key
                .strip_prefix(CUSTOM_BUTTONS)
                .and_then(|k| k.strip_prefix(':'));
            id.is_none_or(|id| buttons.iter().any(|b| b.id.to_string() == id))
        });
        self.custom = buttons;
    }

    pub fn to_json(&self) -> String {
        let entry = |p: &Place| json!({ "x": p.x, "y": p.y, "size": p.size, "show": p.show.id(), "opacity": p.opacity });
        let buttons: Map<String, Value> = self
            .buttons
            .iter()
            .map(|(b, p)| (b.id().to_owned(), entry(p)))
            .collect();
        let mods: Map<String, Value> = self
            .extra
            .iter()
            .map(|e| (e.key.clone(), entry(&e.place)))
            .collect();
        let custom: Vec<Value> = self
            .custom
            .iter()
            .map(|b| {
                let mut entry = match &b.action {
                    CustomAction::Key(key) => json!({ "id": b.id, "key": key }),
                    CustomAction::Input(key) => json!({ "id": b.id, "input": key }),
                    CustomAction::Send(line) => json!({ "id": b.id, "send": line }),
                };
                if let Some(icon) = &b.icon {
                    entry["icon"] = json!(icon);
                }
                if b.image {
                    entry["image"] = json!(true);
                }
                if b.pressed {
                    entry["pressed"] = json!(true);
                }
                if b.toggle {
                    entry["toggle"] = json!(true);
                }
                entry
            })
            .collect();
        let value = json!({
            "version": 2,
            "opacity": self.opacity,
            "movement": match self.dpad {
                Some(true) => "dpad",
                Some(false) => "analog",
                None => "auto",
            },
            "sneak": self.sneak.id(),
            "interact": self.interact.id(),
            "anchor": if self.fixed_stick { "fixed" } else { "floating" },
            "vibrate": self.vibrate,
            "gyro": self.gyro,
            "three_fingers": self.three_fingers,
            "stick": entry(&self.stick),
            "buttons": buttons,
            "mods": mods,
            "custom": custom,
        });
        serde_json::to_string_pretty(&value).unwrap_or_default()
    }

    /// The mods' buttons this frame (`<mod id>:<button id>`, the place its mod gives it, its
    /// shape): each keeps the place the player gave it, if any; the rest are no longer live.
    pub fn offer(&mut self, offered: &[(String, Place, Shape)]) {
        for e in &mut self.extra {
            e.live = false;
        }
        for (key, place, shape) in offered {
            match self.extra.iter_mut().find(|e| e.key == *key) {
                Some(e) => {
                    e.live = true;
                    e.shape = *shape;
                }
                None => self.extra.push(Extra {
                    key: key.clone(),
                    place: *place,
                    shape: *shape,
                    live: true,
                }),
            }
        }
    }

    /// Whether `p` presses `target`, drawn centred on `c` with radius `r`.
    fn hits(&self, target: Target, c: [f32; 2], r: f32, p: [f32; 2]) -> bool {
        let shape = match target {
            Target::Extra(i) => self.extra.get(i).map_or(Shape::Round, |e| e.shape),
            Target::Stick | Target::Button(_) => Shape::Round,
        };
        shape.hits(c, r, p)
    }

    /// Whether `button` is part of the controls in this interaction mode.
    pub fn offers(&self, button: Button) -> bool {
        self.interact == Interact::Crosshair || !button.aims()
    }

    /// How opaque a control is drawn while playing: 0 for a ghost.
    pub fn alpha(&self, place: &Place) -> f32 {
        match place.show {
            Show::On => place.opacity * self.opacity,
            Show::Ghost | Show::Off => 0.0,
        }
    }

    fn place_mut(&mut self, target: Target) -> Option<&mut Place> {
        match target {
            Target::Stick => Some(&mut self.stick),
            Target::Button(button) => self
                .buttons
                .iter_mut()
                .find(|(b, _)| *b == button)
                .map(|(_, p)| p),
            Target::Extra(i) => self.extra.get_mut(i).map(|e| &mut e.place),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// The walking stick, -1..1, x right and y up.
    Stick([f32; 2]),
    /// Sprint on or off: the stick pushed forward past its rim starts it, and it lasts while
    /// the stick still points forward.
    Sprint(bool),
    /// Turn the view by these mouse counts.
    Look(f32, f32),
    Press(Button),
    Release(Button),
    /// Pause held past [`EDIT_HOLD`]: open the layout editor.
    EditLayout,
    /// A quick tap on the view, there: use the held item once (tapping a mob to interact hits it).
    Tap([f32; 2]),
    /// A press on the view held still past [`HOLD`], there: break (or use) until [`Action::HoldStop`].
    HoldStart([f32; 2]),
    HoldStop,
    /// Select hotbar slot 0..8.
    Slot(usize),
    /// Three fingers landing together, where that is on: show or hide the Stats overlay.
    ToggleStats,
    /// A chip beside the held F3 button: F3 with that combination ([`DEBUG_COMBOS`]).
    DebugCombo(usize),
    /// A finger reached the chip of the other F keys: F3 was used, and shows them now.
    FunctionKeys,
    /// One of the other F keys ([`FUNCTION_KEYS`]).
    FunctionKey(usize),
}

/// Fingers landing within this of each other count as one three-finger tap.
const CHORD: Duration = Duration::from_millis(350);

/// Where the controls are on a window, in its pixels.
#[derive(Clone, Debug)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    /// The size unit: a ninth of the shorter side.
    pub unit: f32,
    /// The stick takes fingers landing left of this and below `stick_top` (a floating stick).
    pub stick_right: f32,
    pub stick_top: f32,
    /// The stick stays where it rests and its knob goes to the finger (Bedrock's "Lock
    /// Joystick"), instead of starting under a finger landing anywhere on the left.
    pub fixed_stick: bool,
    /// Where the stick rests before a finger moves it, its reach, and how opaque it is drawn.
    pub stick_home: [f32; 2],
    pub stick_radius: f32,
    pub stick_alpha: f32,
    /// The D-pad instead of the stick: centred on `stick_home`, `stick_radius` from its middle
    /// to an edge.
    pub dpad: bool,
    /// Taps and presses on the view act on what is under the finger ([`Interact::Tap`]).
    pub tap: bool,
    /// The buttons that work: centre, radius, and how opaque they are drawn (0 for a ghost).
    pub buttons: Vec<(Button, [f32; 2], f32, f32)>,
    /// The mods' buttons that work, the same way: their index in [`LayoutConfig::extra`], and
    /// their shape.
    pub extra: Vec<(usize, [f32; 2], f32, f32, Shape)>,
    /// The hotbar: its left edge, top, slot width (vanilla's 182x22 at the GUI scale).
    pub hotbar: [f32; 3],
    /// The close button over screens opened in a world (they have none for touch).
    pub close: ([f32; 2], f32),
}

impl Layout {
    /// The default layout. `gui_scale` is the scale the HUD is drawn at (`HudBatch::scale`), not
    /// the option.
    pub fn new(width: f32, height: f32, gui_scale: f32) -> Self {
        Self::from_config(&LayoutConfig::default(), width, height, gui_scale)
    }
    /// `config` on a window of this size; buttons that are off take no fingers.
    pub fn from_config(config: &LayoutConfig, width: f32, height: f32, gui_scale: f32) -> Self {
        let u = unit(width, height);
        let (w, h) = (width, height);
        let s = gui_scale.max(1.0);
        // Gui.renderHotbar: guiWidth / 2 - 91, guiHeight - 22, in whole GUI pixels.
        let (gui_w, gui_h) = ((w / s).ceil(), (h / s).ceil());
        let hotbar_left = ((gui_w as i32) / 2 - 91) as f32 * s;
        Layout {
            width,
            height,
            unit: u,
            stick_right: w * 0.4,
            stick_top: h * 0.3,
            fixed_stick: config.fixed_stick,
            stick_home: [config.stick.x * w, config.stick.y * h],
            stick_radius: config.stick.size * u,
            stick_alpha: config.alpha(&config.stick),
            dpad: config.dpad == Some(true),
            tap: config.interact == Interact::Tap,
            buttons: config
                .buttons
                .iter()
                .filter(|(b, p)| p.show != Show::Off && config.offers(*b))
                .map(|(b, p)| (*b, [p.x * w, p.y * h], p.size * u, config.alpha(p)))
                .collect(),
            extra: config
                .extra
                .iter()
                .enumerate()
                .filter(|(_, e)| e.live && e.place.show != Show::Off)
                .map(|(i, e)| {
                    let p = &e.place;
                    (i, [p.x * w, p.y * h], p.size * u, config.alpha(p), e.shape)
                })
                .collect(),
            hotbar: [hotbar_left, (gui_h - 22.0) * s, 20.0 * s],
            close: ([w - 0.8 * u, 0.8 * u], 0.5 * u),
        }
    }
    /// The D-pad's cell under `p`: its column and row, -1..1 from the middle (`[0, -1]` the up
    /// arrow, the corners diagonals); None off the pad.
    pub fn dpad_cell(&self, p: [f32; 2]) -> Option<[i8; 2]> {
        if !self.dpad {
            return None;
        }
        let half = self.stick_radius;
        let (dx, dy) = (p[0] - self.stick_home[0], p[1] - self.stick_home[1]);
        if dx.abs() > half * 1.1 || dy.abs() > half * 1.1 {
            return None;
        }
        let cell = |d: f32| {
            if d < -half / 3.0 {
                -1
            } else if d > half / 3.0 {
                1
            } else {
                0
            }
        };
        Some([cell(dx), cell(dy)])
    }
    /// The D-pad's arrows as drawn: each one's cell, centre and half size.
    pub fn dpad_buttons(&self) -> [([i8; 2], [f32; 2], f32); 4] {
        let third = self.stick_radius * 2.0 / 3.0;
        let [x, y] = self.stick_home;
        let at = |c: [i8; 2]| {
            let centre = [x + f32::from(c[0]) * third, y + f32::from(c[1]) * third];
            (c, centre, third * 0.46)
        };
        [at([0, -1]), at([-1, 0]), at([1, 0]), at([0, 1])]
    }
    /// Where the stick is for a finger landing at `p`, if the finger takes it: a fixed stick on
    /// its frame (a square as wide as its reach, a little past the edge too), a floating one
    /// under the finger on the left of the screen.
    fn stick_at(&self, p: [f32; 2]) -> Option<[f32; 2]> {
        if self.dpad {
            return None;
        }
        if self.fixed_stick {
            let reach = self.stick_radius * 1.25;
            let [x, y] = self.stick_home;
            return ((p[0] - x).abs() <= reach && (p[1] - y).abs() <= reach).then_some([x, y]);
        }
        (p[0] < self.stick_right && p[1] > self.stick_top).then_some(p)
    }
    /// Where the knob goes for a finger at `p` on the stick at `origin`, the stick's tilt (-1..1,
    /// x right, y up) and how far the finger is from the middle.
    fn stick_tilt(&self, origin: [f32; 2], p: [f32; 2]) -> ([f32; 2], [f32; 2], f32) {
        let [x, y] = origin;
        let (dx, dy) = (p[0] - x, p[1] - y);
        let r = self.stick_radius;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let k = len.min(r) / len;
        ([x + dx * k, y + dy * k], [dx * k / r, -dy * k / r], len)
    }
    /// The chips of F3's combinations ([`DEBUG_COMBOS`]) and, last, the one of the other F keys,
    /// shown while the F3 button is held. Each: its index, centre, and half width and height.
    pub fn debug_chips(&self) -> Vec<(usize, [f32; 2], [f32; 2])> {
        let u = self.unit;
        self.chips(
            DEBUG_COMBOS.len() + 1,
            [0.95 * u, 0.34 * u],
            [2.05 * u, 0.85 * u],
        )
    }
    /// The chips of the other F keys ([`FUNCTION_KEYS`]), in place of F3's combinations.
    pub fn function_chips(&self) -> Vec<(usize, [f32; 2], [f32; 2])> {
        let u = self.unit;
        self.chips(
            FUNCTION_KEYS.len(),
            [0.7 * u, 0.34 * u],
            [1.5 * u, 0.85 * u],
        )
    }
    /// `count` pills in two rows under the F3 button (over it near the bottom), kept on the
    /// screen.
    fn chips(
        &self,
        count: usize,
        half: [f32; 2],
        step: [f32; 2],
    ) -> Vec<(usize, [f32; 2], [f32; 2])> {
        let Some(&(_, c, r, _)) = self.buttons.iter().find(|(b, ..)| *b == Button::Debug) else {
            return Vec::new();
        };
        let u = self.unit;
        let columns = count.div_ceil(2);
        let width = step[0] * columns as f32;
        let left = (c[0] - width / 2.0).clamp(0.0, (self.width - width).max(0.0)) + step[0] / 2.0;
        let below = c[1] + r + 0.25 * u + half[1];
        let downward = below + step[1] + half[1] <= self.height;
        (0..count)
            .map(|i| {
                let (col, row) = ((i % columns) as f32, (i / columns) as f32);
                let y = if downward {
                    below + row * step[1]
                } else {
                    c[1] - r - 0.25 * u - half[1] - row * step[1]
                };
                (i, [left + col * step[0], y], half)
            })
            .collect()
    }
    fn debug_chip_at(&self, p: [f32; 2]) -> Option<usize> {
        chip_at(&self.debug_chips(), p)
    }
    fn function_chip_at(&self, p: [f32; 2]) -> Option<usize> {
        chip_at(&self.function_chips(), p)
    }
    /// Whether `p` presses the close button.
    pub fn close_at(&self, p: [f32; 2]) -> bool {
        dist(p, self.close.0) <= self.close.1 * 1.2
    }
    /// The mod button `p` presses, by its index in [`LayoutConfig::extra`] (drawn last, so the
    /// topmost).
    pub fn extra_at(&self, p: [f32; 2]) -> Option<usize> {
        self.extra
            .iter()
            .rev()
            .find(|(_, c, r, _, shape)| shape.hits(*c, *r, p))
            .map(|(i, ..)| *i)
    }
    fn button_at(&self, p: [f32; 2]) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(_, c, r, _)| dist(p, *c) <= r * 1.15)
            .map(|(b, ..)| *b)
    }
    fn slot_at(&self, p: [f32; 2]) -> Option<usize> {
        let [left, top, slot] = self.hotbar;
        let i = ((p[0] - left - slot * 0.05) / slot).floor();
        (p[1] >= top && (0.0..9.0).contains(&i)).then_some(i as usize)
    }
    /// Mouse counts per pixel dragged: the window's height turns the view about 180° at the
    /// default sensitivity (0.15° a count).
    fn look_scale(&self) -> f32 {
        1200.0 / self.height
    }
}

/// The size unit: a ninth of the window's shorter side.
pub fn unit(width: f32, height: f32) -> f32 {
    width.min(height) / 9.0
}

/// How far a finger moves before it is no longer a tap, for a window of this size.
pub fn slop(width: f32, height: f32) -> f32 {
    unit(width, height) * 0.15
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[derive(Clone, Copy, Debug)]
enum Role {
    Stick {
        origin: [f32; 2],
        sprinting: bool,
    },
    Look {
        start: [f32; 2],
        last: [f32; 2],
        down: Instant,
        dragged: bool,
        /// Held still past [`HOLD`]: breaking (or using) until it lifts.
        holding: bool,
    },
    Button(Button, Instant),
    /// A finger on the D-pad: the cell it is on (None off the pad) and whether it sprints.
    DPad {
        cell: Option<[i8; 2]>,
        sprinting: bool,
    },
    /// A finger on the hotbar, or one that landed on nothing that takes it.
    Spent,
}

/// Up tapped twice this quickly sprints (Bedrock's D-pad).
const DOUBLE_TAP: Duration = Duration::from_millis(300);

/// The D-pad finger going from one cell to another: an arrow (a corner, two) moves as the stick
/// would at full tilt; the middle stands still.
fn dpad_step(from: Option<[i8; 2]>, to: Option<[i8; 2]>) -> Vec<Action> {
    let mut out = Vec::new();
    if from == to {
        return out;
    }
    let vector = |c: Option<[i8; 2]>| match c {
        Some([x, y]) if [x, y] != [0, 0] => {
            let v = [f32::from(x), -f32::from(y)];
            let len = v[0].hypot(v[1]);
            [v[0] / len, v[1] / len]
        }
        _ => [0.0, 0.0],
    };
    if vector(from) != vector(to) {
        out.push(Action::Stick(vector(to)));
    }
    out
}

/// The chip of `chips` under `p`.
fn chip_at(chips: &[(usize, [f32; 2], [f32; 2])], p: [f32; 2]) -> Option<usize> {
    chips
        .iter()
        .find(|(_, c, h)| (p[0] - c[0]).abs() <= h[0] && (p[1] - c[1]).abs() <= h[1] * 1.2)
        .map(|(i, ..)| *i)
}

#[derive(Default)]
pub struct TouchControls {
    fingers: Vec<(u64, Role)>,
    /// The held F3 button shows the other F keys, not its combinations.
    pub function_keys: bool,
    /// The three-finger tap is on ([`LayoutConfig::three_fingers`]).
    pub three_fingers: bool,
    /// When the fingers down now landed, the latest last (for the three-finger tap).
    landed: Vec<Instant>,
    /// Where the stick is drawn and how far it is pushed.
    pub stick: Option<([f32; 2], [f32; 2])>,
    /// The D-pad cell held now, and when up was last let go (twice quickly sprints).
    pub dpad: Option<[i8; 2]>,
    dpad_up_at: Option<Instant>,
    /// Sneak tapped on in Toggle mode. It outlasts flight and screens (vanilla gives a toggled key
    /// back when a screen closes) until tapped off, Hold is picked or the player dies.
    sneak_on: bool,
    /// The Sneak button's mode and whether the player flies ([`TouchControls::set_sneak`]).
    sneak_mode: SneakMode,
    flying: bool,
}

impl TouchControls {
    /// Each frame: the Sneak button's mode and whether the player flies. Flying, Sneak goes down
    /// only while held, whatever the mode; a sneak toggled on before take-off waits for the
    /// landing, as vanilla's toggled key outlives flight. Picking Hold drops it.
    pub fn set_sneak(&mut self, mode: SneakMode, flying: bool) {
        self.sneak_mode = mode;
        self.flying = flying;
        if mode == SneakMode::Hold {
            self.sneak_on = false;
        }
    }

    /// Whether the Sneak button sneaks now.
    pub fn sneaking(&self) -> bool {
        match self.sneak_mode {
            SneakMode::Toggle if !self.flying => self.sneak_on,
            _ => self.held(Button::Sneak),
        }
    }

    /// Forget a toggled sneak: the player died (`KeyMapping.resetToggleKeys` on respawn).
    pub fn clear_sneak(&mut self) {
        self.sneak_on = false;
    }

    /// A press of Sneak: in Toggle mode and not flying, it turns sneaking on or off.
    fn tap_sneak(&mut self) {
        if self.sneak_mode == SneakMode::Toggle && !self.flying {
            self.sneak_on = !self.sneak_on;
        }
    }

    pub fn event(
        &mut self,
        layout: &Layout,
        id: u64,
        phase: TouchPhase,
        p: [f32; 2],
        now: Instant,
    ) -> Vec<Action> {
        let mut out = Vec::new();
        match phase {
            TouchPhase::Started => {
                let held = self.held(Button::Debug);
                let other = layout
                    .function_chip_at(p)
                    .filter(|_| held && self.function_keys);
                let combo = layout
                    .debug_chip_at(p)
                    .filter(|_| held && !self.function_keys);
                let role = if let Some(i) = other {
                    out.push(Action::FunctionKey(i));
                    Role::Spent
                } else if combo == Some(DEBUG_COMBOS.len()) {
                    self.function_keys = true;
                    out.push(Action::FunctionKeys);
                    Role::Spent
                } else if let Some(i) = combo {
                    // Another finger on a combination while F3 is held.
                    out.push(Action::DebugCombo(i));
                    Role::Spent
                } else if let Some(button) = layout.button_at(p) {
                    // Pause acts when it lifts: a long press is the layout editor.
                    if button != Button::Pause {
                        out.push(Action::Press(button));
                    }
                    if button == Button::Sneak {
                        self.tap_sneak();
                    }
                    Role::Button(button, now)
                } else if let Some(slot) = layout.slot_at(p) {
                    out.push(Action::Slot(slot));
                    Role::Spent
                } else if let Some(cell) = layout.dpad_cell(p).filter(|_| {
                    !self
                        .fingers
                        .iter()
                        .any(|(_, r)| matches!(r, Role::DPad { .. }))
                }) {
                    out.extend(dpad_step(None, Some(cell)));
                    self.dpad = Some(cell);
                    let sprinting = cell[1] == -1
                        && self
                            .dpad_up_at
                            .is_some_and(|at| now.duration_since(at) < DOUBLE_TAP);
                    if sprinting {
                        out.push(Action::Sprint(true));
                    }
                    Role::DPad {
                        cell: Some(cell),
                        sprinting,
                    }
                } else if let Some(origin) = layout.stick_at(p).filter(|_| {
                    !self
                        .fingers
                        .iter()
                        .any(|(_, r)| matches!(r, Role::Stick { .. }))
                }) {
                    // A fixed stick's knob goes straight to the finger.
                    let (knob, stick, _) = layout.stick_tilt(origin, p);
                    self.stick = Some((origin, knob));
                    if stick != [0.0, 0.0] {
                        out.push(Action::Stick(stick));
                    }
                    Role::Stick {
                        origin,
                        sprinting: false,
                    }
                } else {
                    Role::Look {
                        start: p,
                        last: p,
                        down: now,
                        dragged: false,
                        holding: false,
                    }
                };
                self.fingers.retain(|(f, _)| *f != id);
                self.fingers.push((id, role));
                self.landed.retain(|t| now.duration_since(*t) < CHORD);
                self.landed.push(now);
                if self.three_fingers && self.landed.len() >= 3 && self.fingers.len() >= 3 {
                    // The three fingers stop whatever they began.
                    self.landed.clear();
                    // A Sneak pressed by this finger is taken back with its toggle.
                    if out.contains(&Action::Press(Button::Sneak)) {
                        self.tap_sneak();
                    }
                    let start = self.fingers.len() - 3;
                    for i in start..self.fingers.len() {
                        let role = std::mem::replace(&mut self.fingers[i].1, Role::Spent);
                        out.retain(|a| !matches!(a, Action::Press(_) | Action::Slot(_)));
                        out.extend(self.lift(role, false, now));
                    }
                    out.retain(|a| !matches!(a, Action::Tap(_)));
                    out.push(Action::ToggleStats);
                }
            }
            TouchPhase::Moved => {
                let Some((_, role)) = self.fingers.iter_mut().find(|(f, _)| *f == id) else {
                    return out;
                };
                match role {
                    Role::Stick { origin, sprinting } => {
                        let (knob, stick, len) = layout.stick_tilt(*origin, p);
                        let r = layout.stick_radius;
                        self.stick = Some((*origin, knob));
                        out.push(Action::Stick(stick));
                        let forward = stick[1] > stick[0].abs();
                        let sprint = if *sprinting {
                            forward && stick[1] > 0.5
                        } else {
                            forward && len > r * 1.15
                        };
                        if sprint != *sprinting {
                            *sprinting = sprint;
                            out.push(Action::Sprint(sprint));
                        }
                    }
                    Role::Look {
                        start,
                        last,
                        dragged,
                        ..
                    } => {
                        if !*dragged && dist(p, *start) > layout.unit * 0.15 {
                            *dragged = true;
                        }
                        if *dragged {
                            let s = layout.look_scale();
                            out.push(Action::Look((p[0] - last[0]) * s, (p[1] - last[1]) * s));
                        }
                        *last = p;
                    }
                    Role::DPad { cell, sprinting } => {
                        let to = layout.dpad_cell(p);
                        out.extend(dpad_step(*cell, to));
                        *cell = to;
                        self.dpad = to;
                        // Sprint lasts while the finger stays on the forward row.
                        if *sprinting && to.is_none_or(|c| c[1] != -1) {
                            *sprinting = false;
                            out.push(Action::Sprint(false));
                        }
                    }
                    // The F3 finger reached the chip of the other F keys.
                    Role::Button(Button::Debug, _) => {
                        if !self.function_keys
                            && layout.debug_chip_at(p) == Some(DEBUG_COMBOS.len())
                        {
                            self.function_keys = true;
                            out.push(Action::FunctionKeys);
                        }
                    }
                    Role::Button(..) | Role::Spent => {}
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some(i) = self.fingers.iter().position(|(f, _)| *f == id) else {
                    return out;
                };
                let (_, role) = self.fingers.remove(i);
                // The F3 finger slid onto a chip: it is pressed before F3 lets go.
                if matches!(role, Role::Button(Button::Debug, _)) {
                    if phase == TouchPhase::Ended {
                        if self.function_keys {
                            out.extend(layout.function_chip_at(p).map(Action::FunctionKey));
                        } else {
                            let combo = layout.debug_chip_at(p);
                            out.extend(
                                combo
                                    .filter(|c| *c < DEBUG_COMBOS.len())
                                    .map(Action::DebugCombo),
                            );
                        }
                    }
                    self.function_keys = false;
                }
                out.extend(self.lift(role, phase == TouchPhase::Ended, now));
            }
        }
        out
    }

    fn lift(&mut self, role: Role, ended: bool, now: Instant) -> Vec<Action> {
        match role {
            Role::Stick { sprinting, .. } => {
                self.stick = None;
                let mut out = vec![Action::Stick([0.0, 0.0])];
                if sprinting {
                    out.push(Action::Sprint(false));
                }
                out
            }
            Role::Look {
                last,
                down,
                dragged,
                holding,
                ..
            } => {
                if holding {
                    vec![Action::HoldStop]
                } else if ended && !dragged && now - down < HOLD {
                    vec![Action::Tap(last)]
                } else {
                    Vec::new()
                }
            }
            Role::DPad { cell, sprinting } => {
                let mut out = dpad_step(cell, None);
                if sprinting {
                    out.push(Action::Sprint(false));
                }
                if cell.is_some_and(|c| c == [0, -1]) {
                    self.dpad_up_at = Some(now);
                }
                self.dpad = None;
                out
            }
            Role::Button(Button::Pause, _) if ended => vec![Action::Press(Button::Pause)],
            Role::Button(Button::Pause, _) => Vec::new(),
            Role::Button(button, _) => vec![Action::Release(button)],
            Role::Spent => Vec::new(),
        }
    }

    /// Each frame: a view press held still past [`HOLD`] starts breaking, pause held past
    /// [`EDIT_HOLD`] opens the layout editor.
    pub fn frame(&mut self, now: Instant) -> Vec<Action> {
        let mut out = Vec::new();
        for (_, role) in &mut self.fingers {
            if let Role::Button(Button::Pause, down) = role
                && now - *down >= EDIT_HOLD
            {
                *role = Role::Spent;
                out.push(Action::EditLayout);
                continue;
            }
            if let Role::Look {
                last,
                down,
                dragged: false,
                holding,
                ..
            } = role
                && !*holding
                && now - *down >= HOLD
            {
                *holding = true;
                out.push(Action::HoldStart(*last));
            }
        }
        out
    }

    /// Where the finger on the view points while it may still tap, or while it holds (not while
    /// it only turns the view): what tap to interact acts on is found along it.
    pub fn aim(&self) -> Option<[f32; 2]> {
        self.fingers.iter().rev().find_map(|(_, role)| match role {
            Role::Look {
                last,
                dragged,
                holding,
                ..
            } if !*dragged || *holding => Some(*last),
            _ => None,
        })
    }

    /// Whether a finger on the view holds (breaks or uses) now.
    pub fn holding(&self) -> bool {
        self.fingers
            .iter()
            .any(|(_, r)| matches!(r, Role::Look { holding: true, .. }))
    }

    /// Lift every finger (a menu opened, the app went to the background).
    pub fn reset(&mut self) -> Vec<Action> {
        self.function_keys = false;
        let now = Instant::now();
        std::mem::take(&mut self.fingers)
            .into_iter()
            .flat_map(|(_, role)| self.lift(role, false, now))
            .collect()
    }

    /// Whether any finger is on a control: a player holding the stick still is not idle.
    pub fn touching(&self) -> bool {
        !self.fingers.is_empty()
    }

    /// Whether a control holds `button` down.
    pub fn held(&self, button: Button) -> bool {
        self.fingers
            .iter()
            .any(|(_, r)| matches!(r, Role::Button(b, _) if *b == button))
    }
}

/// A control the editor moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Stick,
    Button(Button),
    /// A mod's button, by its index in [`LayoutConfig::extra`].
    Extra(usize),
}

/// The buttons beside the control being edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    Smaller,
    Bigger,
    /// On, ghost, off ([`Show`]).
    Visibility,
    /// The Sneak button's only: Hold or Toggle ([`SneakMode`]).
    Sneak,
    /// The stick's only: floating or fixed ([`LayoutConfig::fixed_stick`]).
    Anchor,
    /// A custom button's only: it goes, with its image.
    Delete,
}

/// The editor's own buttons, at the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAction {
    Done,
    Reset,
    /// A custom button of the player's own, on its page in the options.
    Add,
    /// The stick or the D-pad.
    Movement,
    /// Tap to interact or aim the crosshair ([`Interact`]).
    Interact,
    /// The phone buzzing or not ([`LayoutConfig::vibrate`]).
    Vibrate,
    /// Aiming by turning the phone or not ([`LayoutConfig::gyro`]).
    Gyro,
    /// The three-finger tap or not ([`LayoutConfig::three_fingers`]).
    ThreeFingers,
}

/// The layout editor: drag a control to move it; the one last touched gets chips to make it
/// smaller or bigger and to turn it on, ghost or off (and Sneak's to hold or toggle); the opacity
/// slider sets the selected control's opacity, or every control's with none selected; Done keeps
/// the layout, Reset brings back the default.
#[derive(Default)]
pub struct LayoutEditor {
    pub selected: Option<Target>,
    /// The finger dragging the selected control, and the control's centre minus the finger.
    drag: Option<(u64, [f32; 2])>,
    /// The finger on the opacity slider.
    slider: Option<u64>,
}

const SIZE_STEP: f32 = 0.1;

impl LayoutEditor {
    /// Done, a new button, the movement control, the interaction mode, vibration, the gyroscope,
    /// the three-finger tap and Reset, near the top centre, under the row of buttons the default
    /// layout puts there.
    pub fn bar(width: f32, height: f32) -> [(EditorAction, [f32; 2], f32); 8] {
        let u = unit(width, height);
        let at = |x: f32| [width / 2.0 + x * u, 2.6 * u];
        [
            (EditorAction::Done, at(-4.69), 0.6 * u),
            (EditorAction::Add, at(-3.35), 0.6 * u),
            (EditorAction::Movement, at(-2.01), 0.6 * u),
            (EditorAction::Interact, at(-0.67), 0.6 * u),
            (EditorAction::Vibrate, at(0.67), 0.6 * u),
            (EditorAction::Gyro, at(2.01), 0.6 * u),
            (EditorAction::ThreeFingers, at(3.35), 0.6 * u),
            (EditorAction::Reset, at(4.69), 0.6 * u),
        ]
    }

    /// The opacity slider's track: its left and right ends.
    pub fn opacity_track(width: f32, height: f32) -> ([f32; 2], [f32; 2]) {
        let u = unit(width, height);
        let y = 3.9 * u;
        ([width / 2.0 - 3.2 * u, y], [width / 2.0 + 3.2 * u, y])
    }

    /// What the slider shows: the selected control's opacity, or the layout's.
    pub fn opacity(&self, config: &LayoutConfig) -> f32 {
        match self.selected {
            Some(Target::Stick) => config.stick.opacity,
            Some(Target::Button(b)) => config
                .buttons
                .iter()
                .find(|(x, _)| *x == b)
                .map_or(config.opacity, |(_, p)| p.opacity),
            Some(Target::Extra(i)) => config
                .extra
                .get(i)
                .map_or(config.opacity, |e| e.place.opacity),
            None => config.opacity,
        }
    }

    fn set_opacity(&self, config: &mut LayoutConfig, width: f32, height: f32, x: f32) {
        let ([x0, _], [x1, _]) = Self::opacity_track(width, height);
        let value = (((x - x0) / (x1 - x0)).clamp(0.0, 1.0) * 20.0).round() / 20.0;
        let value = value.max(MIN_OPACITY);
        match self.selected.and_then(|t| config.place_mut(t)) {
            Some(place) => place.opacity = value,
            None => config.opacity = value,
        }
    }

    /// Every control of the interaction mode, ghosts and those off too (the running mods'
    /// buttons last), with its centre and radius in pixels.
    pub fn targets(
        config: &LayoutConfig,
        width: f32,
        height: f32,
    ) -> Vec<(Target, Place, [f32; 2], f32)> {
        let u = unit(width, height);
        let at = |p: &Place| [p.x * width, p.y * height];
        std::iter::once((
            Target::Stick,
            config.stick,
            at(&config.stick),
            config.stick.size * u,
        ))
        .chain(
            config
                .buttons
                .iter()
                .filter(|(b, _)| config.offers(*b))
                .map(|(b, p)| (Target::Button(*b), *p, at(p), p.size * u)),
        )
        .chain(
            config
                .extra
                .iter()
                .enumerate()
                .filter(|(_, e)| e.live)
                .map(|(i, e)| (Target::Extra(i), e.place, at(&e.place), e.place.size * u)),
        )
        .collect()
    }

    /// The chips of the selected control: under it, or over it near the bottom edge.
    pub fn chips(
        &self,
        config: &LayoutConfig,
        width: f32,
        height: f32,
    ) -> Vec<(Chip, [f32; 2], f32)> {
        let Some(selected) = self.selected else {
            return Vec::new();
        };
        let u = unit(width, height);
        let Some((_, _, c, r)) = Self::targets(config, width, height)
            .into_iter()
            .find(|(t, ..)| *t == selected)
        else {
            return Vec::new();
        };
        let below = c[1] + r + 0.7 * u;
        let y = if below + 0.45 * u > height {
            c[1] - r - 0.7 * u
        } else {
            below
        };
        let mut chips = vec![
            (Chip::Smaller, [c[0] - 1.1 * u, y], 0.42 * u),
            (Chip::Bigger, [c[0], y], 0.42 * u),
        ];
        if selected != Target::Stick {
            chips.push((Chip::Visibility, [c[0] + 1.1 * u, y], 0.42 * u));
        } else if config.dpad != Some(true) {
            chips.push((Chip::Anchor, [c[0] + 1.1 * u, y], 0.42 * u));
        }
        if selected == Target::Button(Button::Sneak) {
            chips.push((Chip::Sneak, [c[0] + 2.2 * u, y], 0.42 * u));
        }
        if config.custom_id(selected).is_some() {
            chips.push((Chip::Delete, [c[0] + 2.2 * u, y], 0.42 * u));
        }
        chips
    }

    /// A touch in the editor; Some when Done or Reset was pressed.
    pub fn event(
        &mut self,
        config: &mut LayoutConfig,
        width: f32,
        height: f32,
        id: u64,
        phase: TouchPhase,
        p: [f32; 2],
    ) -> Option<EditorAction> {
        let hit = |c: [f32; 2], r: f32| dist(p, c) <= r * 1.15;
        match phase {
            TouchPhase::Started => {
                if let Some((action, ..)) = Self::bar(width, height)
                    .into_iter()
                    .find(|(_, c, r)| hit(*c, *r))
                {
                    if action == EditorAction::Reset {
                        // The places go back; the player's own buttons are not a place.
                        let custom = std::mem::take(&mut config.custom);
                        *config = LayoutConfig::default();
                        config.custom = custom;
                    }
                    if action == EditorAction::Movement {
                        config.dpad = match config.dpad {
                            None => Some(true),
                            Some(true) => Some(false),
                            Some(false) => None,
                        };
                    }
                    if action == EditorAction::Vibrate {
                        config.vibrate = !config.vibrate;
                    }
                    if action == EditorAction::Gyro {
                        config.gyro = !config.gyro;
                    }
                    if action == EditorAction::ThreeFingers {
                        config.three_fingers = !config.three_fingers;
                    }
                    if action == EditorAction::Interact {
                        config.interact = config.interact.next();
                        // Attack and Use go with the crosshair.
                        if matches!(self.selected, Some(Target::Button(b)) if !config.offers(b)) {
                            self.selected = None;
                        }
                    }
                    return Some(action);
                }
                // The chips first: they can sit over the slider's row.
                if let Some((chip, ..)) = self
                    .chips(config, width, height)
                    .into_iter()
                    .find(|(_, c, r)| hit(*c, *r))
                {
                    if chip == Chip::Delete {
                        if let Some(own) = self.selected.and_then(|t| config.custom_id(t)) {
                            crate::own_buttons::remove_all(own);
                            let left = config.custom.iter().filter(|b| b.id != own);
                            config.set_custom(left.cloned().collect());
                        }
                        self.selected = None;
                    } else if chip == Chip::Sneak {
                        config.sneak = config.sneak.next();
                    } else if chip == Chip::Anchor {
                        config.fixed_stick = !config.fixed_stick;
                    } else if let Some(place) = self.selected.and_then(|t| config.place_mut(t)) {
                        let stick = self.selected == Some(Target::Stick);
                        let extra = matches!(self.selected, Some(Target::Extra(_)));
                        let (min, max) = match (stick, extra) {
                            (true, _) => (0.8, 2.5),
                            (false, true) => (MIN_EXTRA_SIZE, 2.0),
                            (false, false) => (MIN_SIZE, 2.0),
                        };
                        match chip {
                            Chip::Smaller => place.size = (place.size - SIZE_STEP).max(min),
                            Chip::Bigger => place.size = (place.size + SIZE_STEP).min(max),
                            Chip::Visibility => place.show = place.show.next(stick),
                            Chip::Sneak | Chip::Anchor | Chip::Delete => {}
                        }
                    }
                    return None;
                }
                let u = unit(width, height);
                let ([x0, y0], [x1, _]) = Self::opacity_track(width, height);
                if (p[1] - y0).abs() <= 0.45 * u && p[0] >= x0 - 0.3 * u && p[0] <= x1 + 0.3 * u {
                    self.slider = Some(id);
                    self.set_opacity(config, width, height, p[0]);
                    return None;
                }
                // The topmost control under the finger (buttons are drawn over the stick).
                let touched = Self::targets(config, width, height)
                    .into_iter()
                    .rev()
                    .find(|(t, _, c, r)| config.hits(*t, *c, *r, p));
                match touched {
                    Some((target, _, c, _)) => {
                        self.selected = Some(target);
                        self.drag = Some((id, [c[0] - p[0], c[1] - p[1]]));
                    }
                    None => self.selected = None,
                }
            }
            TouchPhase::Moved => {
                if self.slider == Some(id) {
                    self.set_opacity(config, width, height, p[0]);
                    return None;
                }
                if let Some((finger, offset)) = self.drag
                    && finger == id
                    && let Some(place) = self.selected.and_then(|t| config.place_mut(t))
                {
                    place.x = ((p[0] + offset[0]) / width).clamp(0.0, 1.0);
                    place.y = ((p[1] + offset[1]) / height).clamp(0.0, 1.0);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.drag.is_some_and(|(finger, _)| finger == id) {
                    self.drag = None;
                }
                if self.slider == Some(id) {
                    self.slider = None;
                }
            }
        }
        None
    }
}

/// What the pointer finger does, before it becomes window events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pointer {
    Move([f32; 2]),
    Press,
    Release,
    /// Wheel pixels: positive scrolls up (the content follows the finger down).
    Scroll(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Gesture {
    /// Down, not yet a tap, a scroll or a drag.
    #[default]
    Undecided,
    Scroll,
    /// The button is held: a drag (sliders, carried items).
    Drag,
}

/// Off the playing screen, the first finger is the pointer. It moves the cursor when it lands and
/// decides later: lifted where it landed it clicks, moved up or down it scrolls, moved sideways
/// (or held still past [`HOLD`] first) it drags with the button held. Other fingers are ignored.
#[derive(Default)]
pub struct TouchPointer {
    finger: Option<u64>,
    /// The touch device, for the window events made from it.
    device: Option<DeviceId>,
    start: [f32; 2],
    last: [f32; 2],
    down: Option<Instant>,
    gesture: Gesture,
    /// Wheel pixels per window pixel dragged (`set_gui_scale`); unset, 0.5.
    scroll_scale: Option<f32>,
    /// The finger landed where a drag is meant (a scrollbar): moving it drags, never scrolls.
    drag: bool,
}

/// The game turns this many wheel pixels into one wheel step...
const WHEEL_PIXELS_PER_STEP: f32 = 30.0;
/// ...and a wheel step moves a list this many GUI pixels (vanilla's lists, the options pages, the
/// pack lists and the world list alike).
const LIST_PIXELS_PER_STEP: f32 = 18.0;

impl TouchPointer {
    /// The GUI's scale (window pixels per GUI pixel) and the wheel sensitivity the game applies:
    /// a list then follows the finger one to one.
    pub fn set_gui_scale(&mut self, gui_scale: f32, wheel_sensitivity: f32) {
        self.scroll_scale = Some(
            WHEEL_PIXELS_PER_STEP
                / (LIST_PIXELS_PER_STEP * gui_scale.max(0.1) * wheel_sensitivity.max(0.01)),
        );
    }

    /// The finger that just landed drags with the button held whichever way it moves (it is on a
    /// scrollbar, which follows it); lifting it ends that.
    pub fn prefer_drag(&mut self) {
        self.drag = true;
    }

    /// Whether `finger` is the pointer.
    pub fn owns(&self, finger: u64) -> bool {
        self.finger == Some(finger)
    }

    /// Whether a finger is the pointer now.
    pub fn down(&self) -> bool {
        self.finger.is_some()
    }

    /// `slop`: how far the finger moves before a tap is a scroll or a drag, in pixels.
    pub fn step(
        &mut self,
        id: u64,
        phase: TouchPhase,
        p: [f32; 2],
        slop: f32,
        now: Instant,
    ) -> Vec<Pointer> {
        match phase {
            TouchPhase::Started if self.finger.is_none() => {
                self.finger = Some(id);
                self.start = p;
                self.last = p;
                self.down = Some(now);
                self.gesture = Gesture::Undecided;
                vec![Pointer::Move(p)]
            }
            TouchPhase::Moved if self.owns(id) => {
                let mut out = Vec::new();
                if self.gesture == Gesture::Undecided && dist(p, self.start) > slop {
                    let (dx, dy) = (p[0] - self.start[0], p[1] - self.start[1]);
                    if dy.abs() > dx.abs() && !self.drag {
                        self.gesture = Gesture::Scroll;
                        // Scrolling picks up from where the finger landed.
                        self.last = self.start;
                    } else {
                        self.gesture = Gesture::Drag;
                        out.push(Pointer::Press);
                    }
                }
                match self.gesture {
                    Gesture::Scroll => {
                        let scale = self.scroll_scale.unwrap_or(0.5);
                        out.push(Pointer::Scroll((p[1] - self.last[1]) * scale))
                    }
                    Gesture::Drag => out.push(Pointer::Move(p)),
                    Gesture::Undecided => {}
                }
                self.last = p;
                out
            }
            TouchPhase::Ended | TouchPhase::Cancelled if self.owns(id) => {
                self.finger = None;
                self.drag = false;
                match self.gesture {
                    Gesture::Undecided if phase == TouchPhase::Ended => {
                        vec![Pointer::Press, Pointer::Release]
                    }
                    Gesture::Drag => vec![Pointer::Move(p), Pointer::Release],
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Each frame: a pointer held still past [`HOLD`] presses (a long press, then a drag).
    pub fn frame(&mut self, now: Instant) -> Vec<Pointer> {
        if self.finger.is_some()
            && self.gesture == Gesture::Undecided
            && self.down.is_some_and(|d| now - d >= HOLD)
        {
            self.gesture = Gesture::Drag;
            return vec![Pointer::Press];
        }
        Vec::new()
    }

    /// A touch as the window events a mouse would send.
    pub fn event(&mut self, touch: &Touch, slop: f32, now: Instant) -> Vec<WindowEvent> {
        if touch.phase == TouchPhase::Started && self.finger.is_none() {
            self.device = Some(touch.device_id);
        }
        let p = [touch.location.x as f32, touch.location.y as f32];
        let device = touch.device_id;
        let steps = self.step(touch.id, touch.phase, p, slop, now);
        steps.into_iter().map(|s| window_event(s, device)).collect()
    }

    /// `frame` as window events.
    pub fn frame_events(&mut self, now: Instant) -> Vec<WindowEvent> {
        let Some(device) = self.device else {
            return Vec::new();
        };
        self.frame(now)
            .into_iter()
            .map(|s| window_event(s, device))
            .collect()
    }

    /// Let go without a click (the app went to the background).
    pub fn cancel(&mut self) -> Vec<WindowEvent> {
        let held = self.finger.is_some() && self.gesture == Gesture::Drag;
        self.finger = None;
        self.drag = false;
        match (held, self.device) {
            (true, Some(device)) => vec![window_event(Pointer::Release, device)],
            _ => Vec::new(),
        }
    }
}

fn window_event(step: Pointer, device_id: DeviceId) -> WindowEvent {
    match step {
        Pointer::Move(p) => WindowEvent::CursorMoved {
            device_id,
            position: PhysicalPosition::new(f64::from(p[0]), f64::from(p[1])),
        },
        Pointer::Press | Pointer::Release => WindowEvent::MouseInput {
            device_id,
            state: if step == Pointer::Press {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            button: MouseButton::Left,
        },
        Pointer::Scroll(y) => WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, f64::from(y))),
            phase: TouchPhase::Moved,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new(3200.0, 1440.0, 4.0)
    }

    #[test]
    fn the_stick_walks_and_centres_when_lifted() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls::default();
        assert!(
            t.event(&l, 1, TouchPhase::Started, [400.0, 1100.0], now)
                .is_empty()
        );
        let push = t.event(
            &l,
            1,
            TouchPhase::Moved,
            [400.0, 1100.0 - 10.0 * l.stick_radius],
            now,
        );
        // Pushed far past the rim: full forward, and running.
        assert!(
            matches!(push[..], [Action::Stick([x, y]), Action::Sprint(true)]
            if x == 0.0 && (y - 1.0).abs() < 1e-5)
        );
        assert_eq!(
            t.event(&l, 1, TouchPhase::Ended, [400.0, 900.0], now),
            vec![Action::Stick([0.0, 0.0]), Action::Sprint(false)]
        );
        assert!(t.stick.is_none());
    }

    #[test]
    fn pushing_past_the_rim_sprints_until_the_stick_turns_back() {
        let (l, now) = (layout(), Instant::now());
        let r = l.stick_radius;
        let mut t = TouchControls::default();
        t.event(&l, 1, TouchPhase::Started, [400.0, 1100.0], now);
        let walk = t.event(&l, 1, TouchPhase::Moved, [400.0, 1100.0 - r], now);
        assert!(!walk.contains(&Action::Sprint(true)));
        let run = t.event(&l, 1, TouchPhase::Moved, [400.0, 1100.0 - r * 1.3], now);
        assert!(run.contains(&Action::Sprint(true)));
        let still = t.event(&l, 1, TouchPhase::Moved, [400.0, 1100.0 - r * 0.8], now);
        assert!(!still.contains(&Action::Sprint(false)));
        let back = t.event(&l, 1, TouchPhase::Moved, [400.0, 1100.0 + r * 0.5], now);
        assert!(back.contains(&Action::Sprint(false)));
    }

    #[test]
    fn a_fixed_stick_stays_and_its_knob_goes_to_the_finger() {
        let config = LayoutConfig {
            fixed_stick: true,
            ..LayoutConfig::default()
        };
        assert_eq!(LayoutConfig::from_json(&config.to_json()), config);
        let (l, now) = (
            Layout::from_config(&config, 3200.0, 1440.0, 4.0),
            Instant::now(),
        );
        let [x, y] = l.stick_home;
        let r = l.stick_radius;
        let mut t = TouchControls::default();
        // Landing on the right half of the stick walks right at once.
        let land = t.event(&l, 1, TouchPhase::Started, [x + r * 0.5, y], now);
        assert!(
            matches!(land[..], [Action::Stick([sx, sy])] if (sx - 0.5).abs() < 1e-5 && sy == 0.0)
        );
        let push = t.event(&l, 1, TouchPhase::Moved, [x, y - 10.0 * r], now);
        assert!(push.contains(&Action::Sprint(true)));
        assert_eq!(t.stick, Some((l.stick_home, [x, y - r])));
        t.event(&l, 1, TouchPhase::Ended, [x, y - r], now);
        // Off its frame, even on the left, a finger turns the view.
        t.event(&l, 2, TouchPhase::Started, [x + r * 2.0, y - r * 2.0], now);
        assert!(t.stick.is_none() && t.aim().is_some());
    }

    #[test]
    fn three_fingers_together_toggle_the_stats_and_take_nothing_else() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls {
            three_fingers: true,
            ..TouchControls::default()
        };
        t.event(&l, 1, TouchPhase::Started, [2000.0, 600.0], now);
        t.event(&l, 2, TouchPhase::Started, [2200.0, 600.0], now + CHORD / 4);
        let third = t.event(&l, 3, TouchPhase::Started, [2400.0, 600.0], now + CHORD / 2);
        assert_eq!(third, vec![Action::ToggleStats]);
        // Lifting them neither uses nor breaks.
        assert!(t.frame(now + HOLD * 2).is_empty());
        for id in 1..=3 {
            assert!(
                t.event(&l, id, TouchPhase::Ended, [0.0, 0.0], now + HOLD * 2)
                    .is_empty()
            );
        }
        // Fingers far apart in time are just fingers.
        t.event(&l, 4, TouchPhase::Started, [2000.0, 600.0], now);
        t.event(&l, 5, TouchPhase::Started, [2200.0, 600.0], now + CHORD * 2);
        let late = t.event(&l, 6, TouchPhase::Started, [2400.0, 600.0], now + CHORD * 4);
        assert!(late.is_empty());
    }

    #[test]
    fn the_editor_deletes_a_custom_button_and_no_other() {
        let (w, h) = (3200.0, 1440.0);
        let mut config = LayoutConfig {
            custom: vec![CustomButton {
                id: 1,
                action: CustomAction::Send("/home".into()),
                icon: None,
                image: false,
                pressed: false,
                toggle: false,
            }],
            ..LayoutConfig::default()
        };
        config.offer_custom(0.5);
        let mut editor = LayoutEditor {
            selected: Some(Target::Button(Button::Jump)),
            ..LayoutEditor::default()
        };
        let delete = |editor: &LayoutEditor, config: &LayoutConfig| {
            editor
                .chips(config, w, h)
                .into_iter()
                .find(|(chip, ..)| *chip == Chip::Delete)
        };
        assert!(delete(&editor, &config).is_none());
        editor.selected = Some(Target::Extra(0));
        let (_, at, _) = delete(&editor, &config).unwrap();
        editor.event(&mut config, w, h, 1, TouchPhase::Started, at);
        assert!(config.custom.is_empty());
        assert!(config.extra.is_empty());
        assert_eq!(editor.selected, None);
    }

    #[test]
    fn custom_buttons_are_saved_and_outlast_a_reset() {
        let (w, h) = (3200.0, 1440.0);
        let mut config = LayoutConfig {
            custom: vec![
                CustomButton {
                    id: 1,
                    action: CustomAction::Key("key.sprint".into()),
                    icon: Some("star".into()),
                    image: false,
                    pressed: true,
                    toggle: true,
                },
                CustomButton {
                    id: 2,
                    action: CustomAction::Send("/home".into()),
                    icon: None,
                    image: true,
                    pressed: false,
                    toggle: false,
                },
                CustomButton {
                    id: 3,
                    action: CustomAction::Input("key.keyboard.f1".into()),
                    icon: None,
                    image: false,
                    pressed: false,
                    toggle: false,
                },
            ],
            ..LayoutConfig::default()
        };
        assert_eq!(LayoutConfig::from_json(&config.to_json()), config);
        let custom = config.custom.clone();
        let reset = LayoutEditor::bar(w, h)
            .into_iter()
            .find(|(a, ..)| *a == EditorAction::Reset)
            .unwrap()
            .1;
        let mut editor = LayoutEditor::default();
        editor.event(&mut config, w, h, 1, TouchPhase::Started, reset);
        assert_eq!(config.custom, custom);
        // One that goes takes its place along; a mod's stays.
        let extra = |key: &str| Extra {
            key: key.to_owned(),
            place: place(0.5, 0.5, 0.5),
            shape: Shape::Round,
            live: false,
        };
        config.extra = vec![
            extra(&format!("{CUSTOM_BUTTONS}:1")),
            extra(&format!("{CUSTOM_BUTTONS}:2")),
            extra("some_mod:key"),
        ];
        config.set_custom(vec![custom[1].clone()]);
        let kept: Vec<&str> = config.extra.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(kept, [&format!("{CUSTOM_BUTTONS}:2"), "some_mod:key"]);
    }

    #[test]
    fn three_fingers_are_just_fingers_unless_turned_on() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls::default();
        t.event(&l, 1, TouchPhase::Started, [2000.0, 600.0], now);
        t.event(&l, 2, TouchPhase::Started, [2200.0, 600.0], now);
        let third = t.event(&l, 3, TouchPhase::Started, [2400.0, 600.0], now);
        assert!(!third.contains(&Action::ToggleStats));
        // What they began goes on.
        assert!(t.fingers.iter().all(|(_, r)| !matches!(r, Role::Spent)));
        // Off for a layout from before the choice, and kept once picked.
        assert!(!LayoutConfig::default().three_fingers);
        let on = LayoutConfig {
            three_fingers: true,
            ..LayoutConfig::default()
        };
        assert_eq!(LayoutConfig::from_json(&on.to_json()), on);
    }

    #[test]
    fn the_pointer_clicks_on_a_tap_scrolls_up_and_down_and_drags_sideways() {
        let now = Instant::now();
        let mut p = TouchPointer::default();
        // A tap: the cursor moves, then a click where it landed.
        assert_eq!(
            p.step(1, TouchPhase::Started, [100.0, 100.0], 20.0, now),
            vec![Pointer::Move([100.0, 100.0])]
        );
        assert!(
            p.step(1, TouchPhase::Moved, [105.0, 103.0], 20.0, now)
                .is_empty()
        );
        assert_eq!(
            p.step(1, TouchPhase::Ended, [105.0, 103.0], 20.0, now),
            vec![Pointer::Press, Pointer::Release]
        );
        // Up and down: the wheel, no button.
        p.step(2, TouchPhase::Started, [100.0, 500.0], 20.0, now);
        let scroll = p.step(2, TouchPhase::Moved, [102.0, 400.0], 20.0, now);
        assert_eq!(scroll, vec![Pointer::Scroll(-50.0)]);
        assert!(
            p.step(2, TouchPhase::Ended, [102.0, 400.0], 20.0, now)
                .is_empty()
        );
        // Sideways: pressed, dragged, released (a slider).
        p.step(3, TouchPhase::Started, [100.0, 100.0], 20.0, now);
        assert_eq!(
            p.step(3, TouchPhase::Moved, [160.0, 105.0], 20.0, now),
            vec![Pointer::Press, Pointer::Move([160.0, 105.0])]
        );
        assert_eq!(
            p.step(3, TouchPhase::Ended, [170.0, 105.0], 20.0, now),
            vec![Pointer::Move([170.0, 105.0]), Pointer::Release]
        );
        // Held still: pressed by the frame, then free to drag up or down (an item).
        p.step(4, TouchPhase::Started, [100.0, 100.0], 20.0, now);
        assert_eq!(p.frame(now + HOLD), vec![Pointer::Press]);
        assert_eq!(
            p.step(4, TouchPhase::Moved, [100.0, 200.0], 20.0, now + HOLD),
            vec![Pointer::Move([100.0, 200.0])]
        );
        // A second finger is ignored while the first is down.
        assert!(
            p.step(5, TouchPhase::Started, [300.0, 300.0], 20.0, now)
                .is_empty()
        );
    }

    #[test]
    fn pause_acts_on_release_and_a_long_press_opens_the_editor() {
        let (l, now) = (layout(), Instant::now());
        let pause = l
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Pause)
            .unwrap()
            .1;
        let mut t = TouchControls::default();
        assert!(t.event(&l, 1, TouchPhase::Started, pause, now).is_empty());
        assert_eq!(
            t.event(&l, 1, TouchPhase::Ended, pause, now + HOLD),
            vec![Action::Press(Button::Pause)]
        );
        t.event(&l, 2, TouchPhase::Started, pause, now);
        assert_eq!(t.frame(now + EDIT_HOLD), vec![Action::EditLayout]);
        assert!(
            t.event(&l, 2, TouchPhase::Ended, pause, now + EDIT_HOLD)
                .is_empty()
        );
    }

    #[test]
    fn layouts_round_trip_and_fill_in_controls_they_lack() {
        let mut config = LayoutConfig {
            interact: Interact::Crosshair,
            ..LayoutConfig::default()
        };
        config.buttons[1].1 = Place {
            x: 0.5,
            y: 0.5,
            size: 1.1,
            show: Show::Off,
            opacity: 0.6,
        };
        config.buttons[2].1.show = Show::Ghost;
        config.opacity = 0.75;
        let back = LayoutConfig::from_json(&config.to_json());
        assert_eq!(back, config);
        // A layout from before ghosts and attack existed, with a name from the future.
        let old = r#"{"stick":{"x":0.2,"y":0.7,"size":1.5,"visible":false},"buttons":{"jump":{"x":0.9,"y":0.8,"size":0.9},"sneak":{"visible":false},"laser":{"x":0}}}"#;
        let read = LayoutConfig::from_json(old);
        assert_eq!(
            read.stick,
            Place {
                x: 0.2,
                y: 0.7,
                size: 1.5,
                show: Show::On,
                opacity: 1.0
            }
        );
        assert_eq!(
            read.buttons[0].1,
            Place {
                x: 0.9,
                y: 0.8,
                size: 0.9,
                show: Show::On,
                opacity: 1.0
            }
        );
        assert_eq!(read.buttons[1].1.show, Show::Off);
        assert_eq!(read.buttons[2], LayoutConfig::default().buttons[2]);
        // From before the debug button and the interaction mode: both as Bedrock's default.
        let debug = |c: &LayoutConfig| c.buttons.iter().find(|(b, _)| *b == Button::Debug).cloned();
        assert_eq!(debug(&read), debug(&LayoutConfig::default()));
        assert_eq!(read.interact, Interact::Tap);
        assert_eq!(LayoutConfig::from_json("not json"), LayoutConfig::default());
        // Off takes no fingers; a ghost does, and is not drawn; opacities multiply.
        let layout = Layout::from_config(&config, 3200.0, 1440.0, 6.0);
        assert!(!layout.buttons.iter().any(|(b, ..)| *b == Button::Sneak));
        let attack = layout
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Attack)
            .unwrap();
        assert_eq!(attack.3, 0.0);
        let jump = layout
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Jump)
            .unwrap();
        assert_eq!(jump.3, 0.75);
    }

    #[test]
    fn the_editor_drags_resizes_cycles_shows_sets_opacity_and_resets() {
        let (w, h) = (3200.0, 1440.0);
        let mut config = LayoutConfig::default();
        let mut editor = LayoutEditor::default();
        let jump = LayoutEditor::targets(&config, w, h)
            .into_iter()
            .find(|(t, ..)| *t == Target::Button(Button::Jump))
            .unwrap()
            .2;
        assert_eq!(
            editor.event(&mut config, w, h, 1, TouchPhase::Started, jump),
            None
        );
        assert_eq!(editor.selected, Some(Target::Button(Button::Jump)));
        editor.event(&mut config, w, h, 1, TouchPhase::Moved, [1600.0, 720.0]);
        editor.event(&mut config, w, h, 1, TouchPhase::Ended, [1600.0, 720.0]);
        assert_eq!((config.buttons[0].1.x, config.buttons[0].1.y), (0.5, 0.5));
        let chips = editor.chips(&config, w, h);
        let bigger = chips.iter().find(|(c, ..)| *c == Chip::Bigger).unwrap().1;
        let show = chips
            .iter()
            .find(|(c, ..)| *c == Chip::Visibility)
            .unwrap()
            .1;
        editor.event(&mut config, w, h, 2, TouchPhase::Started, bigger);
        assert!((config.buttons[0].1.size - 0.9).abs() < 1e-6);
        let mut seen = Vec::new();
        for id in 3..6 {
            editor.event(&mut config, w, h, id, TouchPhase::Started, show);
            seen.push(config.buttons[0].1.show);
        }
        assert_eq!(seen, [Show::Ghost, Show::Off, Show::On]);
        // The slider: the selected control's opacity, then with nothing selected, everyone's.
        let ([x0, y], [x1, _]) = LayoutEditor::opacity_track(w, h);
        editor.event(
            &mut config,
            w,
            h,
            6,
            TouchPhase::Started,
            [x0 + (x1 - x0) * 0.5, y],
        );
        assert_eq!(config.buttons[0].1.opacity, 0.5);
        editor.event(&mut config, w, h, 6, TouchPhase::Moved, [x0 - 100.0, y]);
        assert_eq!(config.buttons[0].1.opacity, MIN_OPACITY);
        assert_eq!(editor.opacity(&config), MIN_OPACITY);
        editor.event(&mut config, w, h, 6, TouchPhase::Ended, [x0, y]);
        editor.event(&mut config, w, h, 7, TouchPhase::Started, [10.0, 700.0]);
        assert_eq!(editor.selected, None);
        editor.event(
            &mut config,
            w,
            h,
            8,
            TouchPhase::Started,
            [x0 + (x1 - x0) * 0.8, y],
        );
        assert_eq!(config.opacity, 0.8);
        let bar = |action: EditorAction| {
            LayoutEditor::bar(w, h)
                .into_iter()
                .find(|(a, ..)| *a == action)
                .unwrap()
                .1
        };
        // Aiming the crosshair brings Attack and Use into the layout, touching takes them out.
        let has_use = |config: &LayoutConfig| {
            LayoutEditor::targets(config, w, h)
                .iter()
                .any(|(t, ..)| *t == Target::Button(Button::Use))
        };
        assert!(!has_use(&config));
        let interact = bar(EditorAction::Interact);
        editor.event(&mut config, w, h, 10, TouchPhase::Started, interact);
        assert_eq!(config.interact, Interact::Crosshair);
        assert!(has_use(&config));
        editor.event(&mut config, w, h, 11, TouchPhase::Started, interact);
        assert_eq!(config.interact, Interact::Tap);
        let reset = bar(EditorAction::Reset);
        assert_eq!(
            editor.event(&mut config, w, h, 9, TouchPhase::Started, reset),
            Some(EditorAction::Reset)
        );
        assert_eq!(config, LayoutConfig::default());
    }

    #[test]
    fn the_sneak_mode_is_saved_and_older_layouts_toggle() {
        let config = LayoutConfig {
            sneak: SneakMode::Hold,
            ..LayoutConfig::default()
        };
        let back = LayoutConfig::from_json(&config.to_json());
        assert_eq!(back, config);
        // A layout saved before the choice, and a name from the future: Toggle, as Sneak was.
        let old = r#"{"movement":"dpad","buttons":{"sneak":{"x":0.7}}}"#;
        assert_eq!(LayoutConfig::from_json(old).sneak, SneakMode::Toggle);
        let odd = r#"{"sneak":"double-tap"}"#;
        assert_eq!(LayoutConfig::from_json(odd).sneak, SneakMode::Toggle);
    }

    #[test]
    fn sneak_toggles_or_holds_and_only_holds_in_flight() {
        let (l, now) = (layout(), Instant::now());
        let sneak = l
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Sneak)
            .unwrap()
            .1;
        let mut t = TouchControls::default();
        let tap = |t: &mut TouchControls, id: u64| {
            t.event(&l, id, TouchPhase::Started, sneak, now);
            let down = t.sneaking();
            t.event(&l, id, TouchPhase::Ended, sneak, now);
            (down, t.sneaking())
        };
        // Toggle (the default): a tap on, the next off.
        assert_eq!(tap(&mut t, 1), (true, true));
        assert_eq!(tap(&mut t, 2), (false, false));
        // Hold: only while the finger is on it.
        t.set_sneak(SneakMode::Hold, false);
        assert_eq!(tap(&mut t, 3), (true, false));
        // Flying, either mode goes down only while held.
        for mode in [SneakMode::Hold, SneakMode::Toggle] {
            t.set_sneak(mode, true);
            assert_eq!(tap(&mut t, 4), (true, false));
        }
        // A sneak toggled on before take-off does not sink the player, and is back on landing;
        // taps in flight leave it be.
        t.set_sneak(SneakMode::Toggle, false);
        assert_eq!(tap(&mut t, 5), (true, true));
        t.set_sneak(SneakMode::Toggle, true);
        assert!(!t.sneaking());
        assert_eq!(tap(&mut t, 6), (true, false));
        t.set_sneak(SneakMode::Toggle, false);
        assert!(t.sneaking());
        // Screens lift the fingers, not the toggle; Hold and death drop it.
        t.reset();
        assert!(t.sneaking());
        t.set_sneak(SneakMode::Hold, false);
        t.set_sneak(SneakMode::Toggle, false);
        assert!(!t.sneaking());
        tap(&mut t, 7);
        t.clear_sneak();
        assert!(!t.sneaking());
        // A Sneak press that is one of a three-finger tap is taken back.
        t.three_fingers = true;
        t.event(&l, 8, TouchPhase::Started, [2000.0, 600.0], now);
        t.event(&l, 9, TouchPhase::Started, [2200.0, 600.0], now);
        let chord = t.event(&l, 10, TouchPhase::Started, sneak, now);
        assert_eq!(
            chord,
            vec![Action::Release(Button::Sneak), Action::ToggleStats]
        );
        assert!(!t.sneaking());
    }

    #[test]
    fn the_editor_switches_sneak_between_hold_and_toggle() {
        let (w, h) = (3200.0, 1440.0);
        let mut config = LayoutConfig::default();
        let mut editor = LayoutEditor::default();
        let at = |config: &LayoutConfig, target: Target| {
            LayoutEditor::targets(config, w, h)
                .into_iter()
                .find(|(t, ..)| *t == target)
                .unwrap()
                .2
        };
        // Only Sneak has the chip.
        let jump = at(&config, Target::Button(Button::Jump));
        editor.event(&mut config, w, h, 1, TouchPhase::Started, jump);
        editor.event(&mut config, w, h, 1, TouchPhase::Ended, jump);
        let chips = editor.chips(&config, w, h);
        assert!(!chips.iter().any(|(c, ..)| *c == Chip::Sneak));
        let sneak = at(&config, Target::Button(Button::Sneak));
        editor.event(&mut config, w, h, 2, TouchPhase::Started, sneak);
        editor.event(&mut config, w, h, 2, TouchPhase::Ended, sneak);
        let chips = editor.chips(&config, w, h);
        let chip = chips.iter().find(|(c, ..)| *c == Chip::Sneak).unwrap().1;
        assert!(chip[0] + 0.42 * unit(w, h) < w);
        editor.event(&mut config, w, h, 3, TouchPhase::Started, chip);
        assert_eq!(config.sneak, SneakMode::Hold);
        assert_eq!(editor.selected, Some(Target::Button(Button::Sneak)));
        editor.event(&mut config, w, h, 4, TouchPhase::Started, chip);
        assert_eq!(config.sneak, SneakMode::Toggle);
        // Nothing else of the button changed.
        assert_eq!(config, LayoutConfig::default());
    }

    #[test]
    fn the_dpad_walks_stands_still_in_the_middle_and_sprints_on_up_tapped_twice() {
        let (w, h) = (3200.0, 1440.0);
        let config = LayoutConfig {
            dpad: Some(true),
            ..LayoutConfig::default()
        };
        assert_eq!(LayoutConfig::from_json(&config.to_json()).dpad, Some(true));
        // A layout saved before the choice had "auto": the old unpicked "stick" reads as it.
        assert_eq!(
            LayoutConfig::from_json(r#"{"movement": "stick"}"#).dpad,
            None
        );
        let layout = Layout::from_config(&config, w, h, 4.0);
        let [x, y] = layout.stick_home;
        let third = layout.stick_radius * 2.0 / 3.0;
        let up = [x, y - third];
        let middle = [x, y];
        let up_left = [x - third, y - third];
        let mut t = TouchControls::default();
        let now = Instant::now();
        assert_eq!(
            t.event(&layout, 1, TouchPhase::Started, up, now),
            [Action::Stick([0.0, 1.0])]
        );
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(
            t.event(&layout, 1, TouchPhase::Moved, up_left, now),
            [Action::Stick([-s, s])]
        );
        assert_eq!(
            t.event(&layout, 1, TouchPhase::Moved, middle, now),
            [Action::Stick([0.0, 0.0])]
        );
        assert!(!t.held(Button::Sneak));
        assert_eq!(t.event(&layout, 1, TouchPhase::Ended, middle, now), []);
        // Up, let go, up again quickly: sprint while the finger stays on the forward row.
        t.event(&layout, 2, TouchPhase::Started, up, now);
        t.event(&layout, 2, TouchPhase::Ended, up, now);
        let again = now + Duration::from_millis(150);
        assert_eq!(
            t.event(&layout, 3, TouchPhase::Started, up, again),
            [Action::Stick([0.0, 1.0]), Action::Sprint(true)]
        );
        assert_eq!(
            t.event(&layout, 3, TouchPhase::Moved, middle, again),
            [Action::Stick([0.0, 0.0]), Action::Sprint(false)]
        );
        // Off the pad the left side turns the view, as anywhere without a control.
        let far = [
            layout.stick_home[0] + layout.stick_radius * 2.0,
            layout.stick_home[1],
        ];
        assert!(layout.dpad_cell(far).is_none());
    }

    #[test]
    fn mod_buttons_keep_the_place_the_player_gave_them() {
        let (w, h) = (3200.0, 1440.0);
        let mut config = LayoutConfig::default();
        let wide = Shape::Wide(3.0);
        let offered = [
            ("zoom:zoom".to_owned(), place(0.6, 0.5, 0.5), Shape::Square),
            ("sprint:toggle".to_owned(), place(0.1, 0.2, 0.24), wide),
        ];
        config.offer(&offered);
        assert_eq!(config.extra.len(), 2);
        // Pressed where they are, by their shape: the wide one far from its middle too.
        let layout = Layout::from_config(&config, w, h, 6.0);
        assert_eq!(layout.extra_at([0.6 * w, 0.5 * h]), Some(0));
        let r = 0.24 * unit(w, h);
        assert_eq!(layout.extra_at([0.1 * w + 2.5 * r, 0.2 * h]), Some(1));
        assert_eq!(
            layout.extra_at([0.1 * w + 2.5 * r, 0.2 * h + 1.5 * r]),
            None
        );
        // Moved in the editor, then saved and read back: the place is the player's.
        let mut editor = LayoutEditor::default();
        editor.event(
            &mut config,
            w,
            h,
            1,
            TouchPhase::Started,
            [0.6 * w, 0.5 * h],
        );
        assert_eq!(editor.selected, Some(Target::Extra(0)));
        editor.event(&mut config, w, h, 1, TouchPhase::Moved, [0.7 * w, 0.4 * h]);
        editor.event(&mut config, w, h, 1, TouchPhase::Ended, [0.7 * w, 0.4 * h]);
        let mut back = LayoutConfig::from_json(&config.to_json());
        assert!(back.extra.iter().all(|e| !e.live));
        back.offer(&offered[..1]);
        let find = |key: &str| back.extra.iter().find(|e| e.key == key).unwrap();
        let zoom = find("zoom:zoom");
        assert!(zoom.live && zoom.shape == Shape::Square);
        assert!((zoom.place.x - 0.7).abs() < 1e-5 && (zoom.place.y - 0.4).abs() < 1e-5);
        // A mod that stopped keeps its place (a small size too) but takes no fingers and is not
        // edited.
        let sprint = find("sprint:toggle");
        assert!(!sprint.live && (sprint.place.size - 0.24).abs() < 1e-5);
        let layout = Layout::from_config(&back, w, h, 6.0);
        assert_eq!(layout.extra.len(), 1);
        // The stick, the buttons but Attack and Use (tap to interact), the running mod's.
        assert_eq!(
            LayoutEditor::targets(&back, w, h).len(),
            1 + Button::ALL.len() - 2 + 1
        );
        // Reset brings back the mods' own places.
        let mut reset = LayoutConfig::default();
        reset.offer(&offered);
        assert_eq!(reset.extra[0].place, offered[0].1);
    }

    #[test]
    fn the_hotbar_sits_where_vanilla_draws_it() {
        // 3200x1440 at GUI scale 6: 534x240 GUI pixels, the hotbar from x 176 to 358, y 218.
        let l = Layout::new(3200.0, 1440.0, 6.0);
        assert_eq!(l.hotbar, [176.0 * 6.0, 218.0 * 6.0, 120.0]);
        assert_eq!(l.slot_at([176.0 * 6.0 + 186.0, 1400.0]), Some(1));
        assert_eq!(l.slot_at([176.0 * 6.0 + 186.0, 1200.0]), None);
    }

    #[test]
    fn the_view_turns_on_drag_uses_on_tap_and_breaks_on_hold() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls::default();
        t.event(&l, 2, TouchPhase::Started, [2000.0, 600.0], now);
        let look = t.event(&l, 2, TouchPhase::Moved, [2100.0, 600.0], now);
        assert!(matches!(look[..], [Action::Look(dx, dy)] if dx > 0.0 && dy == 0.0));
        assert!(
            t.event(&l, 2, TouchPhase::Ended, [2100.0, 600.0], now)
                .is_empty()
        );

        t.event(&l, 3, TouchPhase::Started, [2000.0, 600.0], now);
        assert_eq!(
            t.event(&l, 3, TouchPhase::Ended, [2000.0, 600.0], now + HOLD / 3),
            vec![Action::Tap([2000.0, 600.0])]
        );

        t.event(&l, 4, TouchPhase::Started, [2000.0, 600.0], now);
        assert_eq!(
            t.frame(now + HOLD),
            vec![Action::HoldStart([2000.0, 600.0])]
        );
        assert!(t.frame(now + HOLD * 2).is_empty());
        assert_eq!(
            t.event(&l, 4, TouchPhase::Ended, [2000.0, 600.0], now + HOLD * 3),
            vec![Action::HoldStop]
        );
    }

    #[test]
    fn the_finger_aims_until_it_turns_the_view_and_while_it_holds() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls::default();
        assert_eq!(t.aim(), None);
        // Landing (and moving less than a tap's slop) aims where the finger is.
        t.event(&l, 1, TouchPhase::Started, [2000.0, 600.0], now);
        assert_eq!(t.aim(), Some([2000.0, 600.0]));
        t.event(&l, 1, TouchPhase::Moved, [2005.0, 602.0], now);
        assert_eq!(t.aim(), Some([2005.0, 602.0]));
        // Dragged, it only turns the view.
        t.event(&l, 1, TouchPhase::Moved, [2300.0, 602.0], now);
        assert_eq!(t.aim(), None);
        t.event(&l, 1, TouchPhase::Ended, [2300.0, 602.0], now);
        // Held, it breaks what is under it, and keeps aiming as it turns the view.
        t.event(&l, 2, TouchPhase::Started, [2200.0, 500.0], now);
        t.frame(now + HOLD);
        assert!(t.holding());
        let turn = t.event(&l, 2, TouchPhase::Moved, [2500.0, 500.0], now + HOLD);
        assert!(matches!(turn[..], [Action::Look(..)]));
        assert_eq!(t.aim(), Some([2500.0, 500.0]));
        t.event(&l, 2, TouchPhase::Ended, [2500.0, 500.0], now + HOLD);
        assert_eq!(t.aim(), None);
        assert!(!t.holding());
    }

    #[test]
    fn tap_to_interact_has_no_attack_and_use_and_the_debug_button_is_there() {
        let (w, h) = (3200.0, 1440.0);
        let has = |config: &LayoutConfig, button: Button| {
            Layout::from_config(config, w, h, 6.0)
                .buttons
                .iter()
                .any(|(b, ..)| *b == button)
        };
        let tap = LayoutConfig::default();
        assert!(Layout::from_config(&tap, w, h, 6.0).tap);
        assert!(!has(&tap, Button::Attack) && !has(&tap, Button::Use));
        assert!(has(&tap, Button::Debug) && has(&tap, Button::Jump));
        let aim = LayoutConfig {
            interact: Interact::Crosshair,
            ..LayoutConfig::default()
        };
        assert!(!Layout::from_config(&aim, w, h, 6.0).tap);
        assert!(has(&aim, Button::Attack) && has(&aim, Button::Use));
        assert_eq!(LayoutConfig::from_json(&aim.to_json()), aim);
        // Pick Block and the player list wait in the editor, off; vibration is on.
        assert!(!has(&tap, Button::Pick) && !has(&tap, Button::Players));
        let shown = |c: &LayoutConfig, b: Button| {
            LayoutEditor::targets(c, w, h)
                .iter()
                .any(|(t, p, ..)| *t == Target::Button(b) && p.show == Show::Off)
        };
        assert!(shown(&tap, Button::Pick) && shown(&tap, Button::Players));
        assert!(tap.vibrate);
        let quiet = LayoutConfig {
            vibrate: false,
            ..LayoutConfig::default()
        };
        assert_eq!(LayoutConfig::from_json(&quiet.to_json()), quiet);
    }

    #[test]
    fn the_gyroscope_turns_the_view_as_the_phone_turns_on_any_side() {
        let close =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;
        let quarter = std::f32::consts::FRAC_PI_2;
        // Landscape, top to the left (ROTATION_90): the screen's up is the device's x. Turned
        // left a quarter about it: the view turns 90 degrees left.
        assert!(close(gyro_turn([quarter, 0.0], 1), [-90.0, 0.0]));
        // The other landscape (ROTATION_270): the screen's up is the device's -x.
        assert!(close(gyro_turn([-quarter, 0.0], 3), [-90.0, 0.0]));
        // Top tilted toward the player about the screen's right (the device's -y at 90):
        // looking up.
        assert!(close(gyro_turn([0.0, -quarter], 1), [0.0, -90.0]));
        assert!(close(gyro_turn([0.0, quarter], 3), [0.0, -90.0]));
        // Portrait as it is.
        assert!(close(gyro_turn([0.0, quarter], 0), [-90.0, 0.0]));
        let gyro = LayoutConfig {
            gyro: true,
            ..LayoutConfig::default()
        };
        assert!(!LayoutConfig::default().gyro);
        assert_eq!(LayoutConfig::from_json(&gyro.to_json()), gyro);
    }

    #[test]
    fn f3_held_offers_its_combinations_to_a_finger_or_a_slide() {
        let (l, now) = (layout(), Instant::now());
        let f3 = l
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Debug)
            .unwrap()
            .1;
        let chips = l.debug_chips();
        assert_eq!(chips.len(), DEBUG_COMBOS.len() + 1);
        // Under the button (it is at the top), on the screen, none over another.
        assert!(
            chips
                .iter()
                .all(|(_, c, h)| c[1] > f3[1] && c[0] - h[0] >= 0.0 && c[0] + h[0] <= l.width)
        );
        let mut t = TouchControls::default();
        // A chip is nothing while F3 is not held (the finger turns the view there).
        let hitboxes = chips[1].1;
        assert!(
            !t.event(&l, 1, TouchPhase::Started, hitboxes, now)
                .contains(&Action::DebugCombo(1))
        );
        t.event(&l, 1, TouchPhase::Ended, hitboxes, now + HOLD);
        // Held: a second finger on a chip presses that combination.
        assert_eq!(
            t.event(&l, 2, TouchPhase::Started, f3, now),
            vec![Action::Press(Button::Debug)]
        );
        assert_eq!(
            t.event(&l, 3, TouchPhase::Started, hitboxes, now),
            vec![Action::DebugCombo(1)]
        );
        assert!(t.event(&l, 3, TouchPhase::Ended, hitboxes, now).is_empty());
        // The F3 finger slid onto a chip: the combination, then F3 lets go.
        let chunks = chips[0].1;
        t.event(&l, 2, TouchPhase::Moved, chunks, now);
        assert_eq!(
            t.event(&l, 2, TouchPhase::Ended, chunks, now),
            vec![Action::DebugCombo(0), Action::Release(Button::Debug)]
        );
    }

    #[test]
    fn the_f3_finger_slides_on_to_the_other_f_keys() {
        let (l, now) = (layout(), Instant::now());
        let f3 = l
            .buttons
            .iter()
            .find(|(b, ..)| *b == Button::Debug)
            .unwrap()
            .1;
        let keys = l.function_chips();
        assert_eq!(keys.len(), FUNCTION_KEYS.len());
        assert!(
            keys.iter()
                .all(|(_, c, h)| c[0] - h[0] >= 0.0 && c[0] + h[0] <= l.width)
        );
        let more = l.debug_chips()[DEBUG_COMBOS.len()].1;
        let camera = keys[3].1;
        let mut t = TouchControls::default();
        t.event(&l, 1, TouchPhase::Started, f3, now);
        assert_eq!(
            t.event(&l, 1, TouchPhase::Moved, more, now),
            vec![Action::FunctionKeys]
        );
        assert!(t.event(&l, 1, TouchPhase::Moved, camera, now).is_empty());
        assert_eq!(
            t.event(&l, 1, TouchPhase::Ended, camera, now),
            vec![Action::FunctionKey(3), Action::Release(Button::Debug)]
        );
        // Back to the combinations the next time F3 is held.
        assert!(!t.function_keys);
        // Let go on the chip itself: nothing but F3's release.
        t.event(&l, 2, TouchPhase::Started, f3, now);
        t.event(&l, 2, TouchPhase::Moved, more, now);
        t.event(&l, 2, TouchPhase::Moved, [0.0, 0.0], now);
        assert_eq!(
            t.event(&l, 2, TouchPhase::Ended, [0.0, 0.0], now),
            vec![Action::Release(Button::Debug)]
        );
    }

    #[test]
    fn buttons_and_slots_take_their_fingers() {
        let (l, now) = (layout(), Instant::now());
        let mut t = TouchControls::default();
        let (_, jump, ..) = l.buttons[0];
        assert_eq!(
            t.event(&l, 5, TouchPhase::Started, jump, now),
            vec![Action::Press(Button::Jump)]
        );
        assert!(t.held(Button::Jump));
        // A second finger on the left half while jump is held still walks.
        assert!(
            t.event(&l, 6, TouchPhase::Started, [300.0, 1200.0], now)
                .is_empty()
        );
        assert!(t.stick.is_some());
        assert_eq!(
            t.reset(),
            vec![Action::Release(Button::Jump), Action::Stick([0.0, 0.0])]
        );
        let [left, top, slot] = l.hotbar;
        assert_eq!(
            t.event(
                &l,
                7,
                TouchPhase::Started,
                [left + slot * 4.5, top + 10.0],
                now
            ),
            vec![Action::Slot(4)]
        );
    }
}
