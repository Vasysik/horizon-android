//! Multitouch input using the engine's existing controller actions.
use crate::gamepad::{Button, PadState};
use crate::hud::{HudAtlas, HudBatch, rgba};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
const HOLD_DELAY: Duration = Duration::from_millis(350);
mod custom;
mod editor;
pub use custom::CustomButton;
pub mod layout;
pub use layout::{Control, Mode, Settings};
/// Entity-placement items retain short-tap placement, like building blocks.
pub fn item_places_entity_on_tap(item: &str) -> bool {
    let Some(id) = item.strip_prefix("minecraft:") else {
        return false;
    };
    id.ends_with("_boat")
        || id.ends_with("_raft")
        || id.ends_with("_spawn_egg")
        || id == "minecart"
        || id.ends_with("_minecart")
        || matches!(
            id,
            "armor_stand" | "item_frame" | "glow_item_frame" | "painting" | "end_crystal"
        )
}

/// Surface-targeted actions should not wait for a long press.
pub fn item_uses_on_tap(item: &str) -> bool {
    item_places_entity_on_tap(item)
        || item
            .strip_prefix("minecraft:")
            .is_some_and(|id| id.ends_with("_bucket") && id != "milk_bucket")
        || matches!(
            item,
            "minecraft:bone_meal"
                | "minecraft:flint_and_steel"
                | "minecraft:fire_charge"
                | "minecraft:honeycomb"
                | "minecraft:glass_bottle"
                | "minecraft:bucket"
                | "minecraft:water_bucket"
                | "minecraft:lava_bucket"
                | "minecraft:powder_snow_bucket"
        )
}
pub fn projectile_crosshair(item: &str) -> bool {
    matches!(
        item,
        "minecraft:bow"
            | "minecraft:crossbow"
            | "minecraft:trident"
            | "minecraft:snowball"
            | "minecraft:egg"
            | "minecraft:ender_pearl"
            | "minecraft:experience_bottle"
            | "minecraft:splash_potion"
            | "minecraft:lingering_potion"
            | "minecraft:wind_charge"
            | "minecraft:fishing_rod"
    )
}
/// Equal physical drags turn equally on either axis, including portrait screens.
pub fn camera_delta(delta: [f32; 2], width: u32, height: u32, s: &Settings) -> [f32; 2] {
    let scale = width.max(height).max(1) as f32;
    [
        delta[0] * width as f32 / scale * 180.0 * s.sensitivity * s.sensitivity_horizontal,
        delta[1] * height as f32 / scale * 180.0 * s.sensitivity * s.sensitivity_vertical,
    ]
}

/// In tap modes the crosshair is absent, so use Java's native hotbar gauge.
pub fn attack_indicator(setting: u8, crosshair: bool) -> u8 {
    if setting == 1 && !crosshair {
        2
    } else {
        setting
    }
}

/// A finger ray through the perspective projection, without rotating the view.
pub fn screen_direction(
    forward: [f64; 3],
    up: [f64; 3],
    point: [f32; 2],
    aspect: f64,
    fov: f64,
) -> [f64; 3] {
    let right = [
        forward[1] * up[2] - forward[2] * up[1],
        forward[2] * up[0] - forward[0] * up[2],
        forward[0] * up[1] - forward[1] * up[0],
    ];
    let t = (fov.to_radians() / 2.0).tan();
    let x = (f64::from(point[0]) * 2.0 - 1.0) * aspect * t;
    let y = (1.0 - f64::from(point[1]) * 2.0) * t;
    let ray = std::array::from_fn::<_, 3, _>(|i| forward[i] + right[i] * x + up[i] * y);
    let length = ray.iter().map(|v| v * v).sum::<f64>().sqrt();
    ray.map(|v| v / length)
}

pub fn hotbar_slot(width: f32, height: f32, point: [f32; 2]) -> Option<usize> {
    let x = point[0] - (width / 2.0 - 90.0);
    (point[1] >= height - 22.0 && point[1] < height && (0.0..180.0).contains(&x))
        .then(|| (x / 20.0) as usize)
}
#[derive(Clone, Copy)]
enum Role {
    Move([f32; 2]),
    Look {
        start: [f32; 2],
        since: Instant,
        dragged: bool,
        held_use: bool,
        committed: bool,
    },
    Ignore,
    Shield,
    Button(Button),
}
#[derive(Default)]
pub struct TouchControls {
    pub settings: Settings,
    pub force_crosshair: bool,
    pub projectile_crosshair: bool,
    pub shield_available: bool,
    /// Physical keypad profiles keep utility controls without a movement overlay.
    pub auxiliary_only: bool,
    /// Inventory utility buttons emit real modifier/mouse events, never movement.
    pub inventory_mode: bool,
    pub editor: Option<editor::Editor>,
    fingers: BTreeMap<u64, (Role, [f32; 2])>,
    custom_fingers: BTreeMap<u64, String>,
    custom_events: Vec<(String, bool)>,
    pending: PadState,
    pub aim: Option<[f32; 2]>,
    tap_aim: Option<[f32; 2]>,
    pending_tap: Option<bool>,
    aspect: f32,
    loaded: bool,
    pub chat_requested: bool,
    pub settings_requested: bool,
    pub debug_requested: bool,
    pub perspective_requested: bool,
    pub clicked: bool,
    pub context_use: bool,
    pub vertical_movement: bool,
    pub swimming: bool,
    pub mounted: bool,
    pub sneaking: bool,
    /// Snapshotted by a world gesture; drift cannot turn eating into mining.
    pub hold_uses_item: bool,
    pub holding_use: bool,
    pub holding: bool,
    pub break_progress: f32,
    last_marker: Option<([f32; 2], Instant)>,
    last_sneak_tap: Option<Instant>,
    sprint_latched: bool,
}
impl TouchControls {
    pub fn crosshair(&self) -> bool {
        self.force_crosshair || self.settings.crosshair()
    }

    pub fn viewport(&mut self, width: u32, height: u32) {
        self.aspect = width.max(1) as f32 / height.max(1) as f32;
        if !self.loaded {
            self.settings = Settings::load();
            self.loaded = true;
        }
    }
    fn aspect(&self) -> f32 {
        if self.aspect > 0.0 {
            self.aspect
        } else {
            16.0 / 9.0
        }
    }
    fn auxiliary_control(c: Control) -> bool {
        matches!(
            c,
            Control::Debug
                | Control::Chat
                | Control::Pause
                | Control::Settings
                | Control::Perspective
        )
    }
    pub fn auxiliary_at(&self, point: [f32; 2]) -> bool {
        self.custom_at(point).is_some()
            || Control::ALL.into_iter().any(|c| {
                Self::auxiliary_control(c)
                    && self.visible(c)
                    && layout::contains(self.placement(c).rect(self.aspect()), point)
            })
    }
    pub fn control_at(&self, point: [f32; 2]) -> Option<Control> {
        Control::ALL.into_iter().rev().find(|c| {
            self.visible(*c) && layout::contains(self.placement(*c).rect(self.aspect()), point)
        })
    }
    pub fn control_at_point(&self, point: [f32; 2]) -> bool {
        self.custom_at(point).is_some() || self.control_at(point).is_some()
    }
    /// Inventory overlays retain custom key holds, but never apply world gestures.
    pub fn clear_world_gesture(&mut self) {
        self.fingers
            .retain(|_, (role, _)| matches!(role, Role::Ignore));
        self.sprint_latched = false;
        self.pending = PadState::default();
        self.aim = None;
        self.tap_aim = None;
        self.pending_tap = None;
        self.last_marker = None;
        self.holding = false;
        self.holding_use = false;
    }
    pub fn owns_pointer(&self, id: u64) -> bool {
        self.fingers.contains_key(&id) || self.custom_fingers.contains_key(&id)
    }
    pub fn ignore(&mut self, id: u64) {
        self.fingers.insert(id, (Role::Ignore, [0.0; 2]));
    }
    pub fn tap(&mut self, button: Button) {
        self.pending.set(button, true);
    }
    fn visible(&self, c: Control) -> bool {
        if c == Control::Shield && !self.shield_available && self.editor.is_none() {
            return false;
        }
        if self.auxiliary_only && !Self::auxiliary_control(c) {
            return false;
        }
        if self.force_crosshair
            && matches!(c, Control::Attack | Control::Use)
            && self.settings.button_visibility[c as usize] == 0
        {
            return true;
        }

        self.settings.visible(c)
            || (self.settings.button_visibility[c as usize] != 2
                && c == Control::Use
                && self.settings.contextual_interact
                && (self.context_use
                    || self
                        .fingers
                        .values()
                        .any(|(r, _)| matches!(r, Role::Button(Button::LeftTrigger)))))
    }
    fn placement(&self, c: Control) -> layout::Placement {
        let mut p = self.settings.placement(c);
        let factory_swap = c == Control::Offhand
            && self.settings.layouts[self.settings.mode.index()][c as usize].center == [0.80, 0.96]
            && p.size == 0.075;
        // Keep factory top buttons separate on tall surfaces. Custom positions
        // and every landscape layout remain exactly as the player saved them.
        if self.aspect() < 1.0 && p.center[1] == 0.055 && p.size == 0.075 {
            let top = match c {
                Control::Debug => Some((0.42, 0.18)),
                Control::Chat => Some((0.46, 0.34)),
                Control::Pause => Some((0.50, 0.50)),
                Control::Settings => Some((0.54, 0.66)),
                Control::Perspective => Some((0.58, 0.82)),
                _ => None,
            };
            if let Some((old, new)) = top {
                let old = if self.settings.left_handed {
                    1.0 - old
                } else {
                    old
                };
                if (p.center[0] - old).abs() < 0.0001 {
                    p.center[0] = if self.settings.left_handed {
                        1.0 - new
                    } else {
                        new
                    };
                    p.size = p.size.min(self.aspect() * 0.12);
                }
            }
        }
        let mut p = self.fit_placement(p);
        if factory_swap {
            // Keep the factory Swap beside Inventory in physical button units,
            // including portrait displays and the left-handed layout.
            let inventory = self.placement(Control::Inventory);
            let direction = if self.settings.left_handed { -1.0 } else { 1.0 };
            p.center[0] = inventory.center[0]
                + direction * ((inventory.size + p.size) * 0.5 + 0.015) / self.aspect();
            let half = p.size / self.aspect() * 0.5;
            p.center[0] = p.center[0].clamp(half, 1.0 - half);
        }
        p
    }
    fn button(c: Control) -> Option<Button> {
        Some(match c {
            Control::Sprint => Button::LeftStick,
            Control::Jump => Button::South,
            Control::Sneak => Button::RightStick,
            Control::Attack => Button::RightTrigger,
            Control::Use | Control::Shield => Button::LeftTrigger,
            Control::Pause => Button::Start,
            Control::Inventory => Button::North,
            _ => return None,
        })
    }
    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas) {
        let (w, h) = (b.width, b.height);
        if self.editor.is_some() {
            self.draw_editor_toolbar(b, a);
        }
        let state = self.state();
        for control in Control::ALL {
            if !self.visible(control) {
                continue;
            }
            let p = self.placement(control);
            let size = p.size * h;
            let (x, y) = (p.center[0] * w - size / 2.0, p.center[1] * h - size / 2.0);
            let opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            let color = rgba(255, 255, 255, (opacity * 255.0).round() as u8);
            let held = (self.inventory_mode
                && Self::inventory_token(control)
                    .is_some_and(|token| self.held_custom_tokens().any(|held| held == token)))
                || (control == Control::Sneak && self.sneaking)
                || Self::button(control).is_some_and(|bt| {
                    state.held(bt)
                        || (bt == Button::LeftTrigger && state.triggers[0] > 0.0)
                        || (bt == Button::RightTrigger && state.triggers[1] > 0.0)
                });
            if control == Control::Move {
                if self.settings.mode == Mode::Dpad {
                    let cell = size / 3.0;
                    for (dx, dy, name) in [
                        (0, -1, "up"),
                        (-1, 0, "left"),
                        (1, 0, "right"),
                        (0, 1, "down"),
                    ] {
                        let pressed = match (dx, dy) {
                            (0, -1) => state.left[1] > 0.2,
                            (0, 1) => state.left[1] < -0.2,
                            (-1, 0) => state.left[0] < -0.2,
                            _ => state.left[0] > 0.2,
                        };
                        fitted_sprite(
                            b,
                            a,
                            &format!("controls/{name}{}", if pressed { "_pressed" } else { "" }),
                            [
                                x + cell * (dx + 1) as f32,
                                y + cell * (dy + 1) as f32,
                                cell,
                                cell,
                            ],
                            color,
                        );
                    }
                    // Vanilla shows the diagonal keys while their adjoining arrow is held.
                    if state.left[1].abs() > 0.2 {
                        let (dy, prefix) = if state.left[1] > 0.0 {
                            (0, "up")
                        } else {
                            (2, "down")
                        };
                        for (dx, suffix) in [(0, "left"), (2, "right")] {
                            let pressed = if dx == 0 {
                                state.left[0] < -0.2
                            } else {
                                state.left[0] > 0.2
                            };
                            sprite(
                                b,
                                a,
                                &format!(
                                    "controls/{prefix}_{suffix}{}",
                                    if pressed { "_pressed" } else { "" }
                                ),
                                [x + cell * dx as f32, y + cell * dy as f32, cell, cell],
                                color,
                            );
                        }
                    }
                } else {
                    sprite(b, a, "ui/joystick_frame", [x, y, size, size], color);
                    sprite(
                        b,
                        a,
                        "ui/joystick_knob",
                        [
                            x + size * (0.33 + state.left[0] * 0.25),
                            y + size * (0.33 - state.left[1] * 0.25),
                            size * 0.34,
                            size * 0.34,
                        ],
                        color,
                    );
                }
            } else {
                let pressed = held
                    || self
                        .fingers
                        .values()
                        .any(|(_, point)| layout::contains(p.rect(self.aspect()), *point));
                let themed = match control {
                    Control::Chat => Some("chat"),
                    Control::Pause => Some("pause"),
                    Control::Settings => Some("settings"),
                    Control::Debug => Some("debug"),
                    Control::Perspective => Some("perspective"),
                    _ => None,
                };
                if themed.is_some_and(|name| {
                    theme_button(b, a, name, [x, y, size, size], pressed, color, false)
                }) {
                    continue;
                }
                let name = match control {
                    Control::Sprint => "ui/sprint",
                    Control::Attack => "ui/attack",
                    Control::Use => "ui/interact",
                    Control::Jump if self.swimming => "controls/waterascend",
                    Control::Sneak if self.swimming => "controls/waterdescend",
                    Control::Jump if self.vertical_movement => "controls/flyingascend",
                    Control::Sneak if self.vertical_movement => "controls/flyingdescend",
                    Control::Sneak if self.mounted => "controls/dismount",
                    Control::Jump if self.settings.mode == Mode::Dpad => "controls/jump_dpad",
                    Control::Sneak if self.settings.mode == Mode::Dpad => "controls/sneak_dpad",
                    Control::Jump => "ui/jump",
                    Control::Sneak => "ui/sneak",
                    Control::Pause => "ui/pause_icon",
                    Control::Chat | Control::Debug | Control::Perspective => "gui",
                    Control::Inventory => "ui/more-dots",
                    Control::Settings => "ui/gear",
                    Control::Offhand | Control::Shield => "ui/joystick_knob",
                    _ => continue,
                };
                let bounds = [x, y, size, size];
                match control {
                    Control::Chat => region(b, a, "gui", [200.0, 82.0, 18.0, 18.0], bounds, color),
                    Control::Debug => region(b, a, "gui", [236.0, 82.0, 18.0, 18.0], bounds, color),
                    Control::Perspective => {
                        region(b, a, "gui", [236.0, 46.0, 18.0, 18.0], bounds, color)
                    }
                    Control::Offhand | Control::Shield => {
                        let label = if control == Control::Offhand {
                            "Swap"
                        } else {
                            "Shield"
                        };
                        text_button(b, a, bounds, label, pressed, color, false);
                    }
                    Control::Inventory => {
                        sprite(b, a, "ui/joystick_knob", bounds, color);
                        fitted_sprite(
                            b,
                            a,
                            name,
                            [x + size * 0.2, y + size * 0.2, size * 0.6, size * 0.6],
                            color,
                        );
                    }
                    Control::Pause | Control::Settings => {
                        // The same square frame as Chat, sampled without its speech glyph.
                        menu_frame(b, a, bounds, color);
                        fitted_sprite(
                            b,
                            a,
                            name,
                            [x + size * 0.23, y + size * 0.23, size * 0.54, size * 0.54],
                            color,
                        );
                    }
                    _ => sprite(
                        b,
                        a,
                        &format!("{name}{}", if held { "_pressed" } else { "" }),
                        bounds,
                        color,
                    ),
                }
            }
            if let Some(e) = &self.editor
                && e.selected == Some(control)
            {
                let green = rgba(255, 255, 255, 255);
                for (p, q) in [
                    ([x - 1.0, y - 1.0], [x + size + 1.0, y - 1.0]),
                    ([x - 1.0, y + size + 1.0], [x + size + 1.0, y + size + 1.0]),
                    ([x - 1.0, y - 1.0], [x - 1.0, y + size + 1.0]),
                    ([x + size + 1.0, y - 1.0], [x + size + 1.0, y + size + 1.0]),
                ] {
                    line(b, a, p, q, green);
                }
            }
        }
        self.draw_custom(b, a);
        if self.editor.is_some() {
            self.draw_editor(b, a);
        } else if let Some(point) = self.pointer() {
            let center = [point[0] * w, point[1] * h];
            let radius = self.settings.circle_size / 2.0;
            ring(b, a, center, radius + 0.7, rgba(0, 0, 0, 110));
            ring(b, a, center, radius, rgba(255, 255, 255, 210));
            // A complete inner circle expands towards the stationary outer rim.
            // Progress changes its radius, never its angular coverage.
            let inner = mining_radius(radius, self.break_progress);
            if inner > 0.0
                && let Some(white) = a.sprites.get("horizon/white")
            {
                let uv = [[(white.u0 + white.u1) / 2.0, (white.v0 + white.v1) / 2.0]; 4];
                for segment in circle_points().windows(2) {
                    let points = [segment[0], segment[1]]
                        .map(|p| [center[0] + inner * p[0], center[1] + inner * p[1]]);
                    b.quad(
                        [center, points[0], points[1], center],
                        uv,
                        rgba(255, 255, 255, 90),
                    );
                }
                ring(b, a, center, inner, rgba(255, 255, 255, 220));
            }
        }
    }
    pub fn shield_held(&self) -> bool {
        self.settings.shield_button
            && self.shield_available
            && self
                .fingers
                .values()
                .any(|(role, _)| matches!(role, Role::Shield))
    }
    pub fn pointer(&self) -> Option<[f32; 2]> {
        if self.crosshair() {
            return None;
        }
        self.fingers
            .values()
            .find_map(|(r, point)| matches!(r, Role::Look { .. }).then_some(*point))
            .or_else(|| {
                self.last_marker
                    .filter(|(_, t)| t.elapsed() < Duration::from_millis(180))
                    .map(|(p, _)| p)
            })
    }
    /// Coordinates are fractions of the drawable surface. Returns a look delta.
    pub fn update(&mut self, id: u64, point: [f32; 2], started: bool, ended: bool) -> [f32; 2] {
        if self.custom_update(id, point, started, ended) {
            return [0.0; 2];
        }
        if ended {
            if let Some((
                Role::Look {
                    start,
                    since,
                    dragged,
                    held_use,
                    ..
                },
                _,
            )) = self.fingers.remove(&id)
                && (!self.crosshair() || self.settings.crosshair_gestures)
                && !dragged
                && since.elapsed() < HOLD_DELAY
            {
                self.tap_aim = Some(if self.settings.crosshair() || self.projectile_crosshair {
                    [0.5, 0.5]
                } else {
                    start
                });
                self.last_marker = Some((start, Instant::now()));
                self.pending_tap = Some(held_use);
            }
            return [0.0; 2];
        }
        if started {
            let control = self.control_at(point);
            let role = match control {
                Some(Control::Move)
                    if !self
                        .fingers
                        .values()
                        .any(|(r, _)| matches!(r, Role::Move(_))) =>
                {
                    Role::Move(self.placement(Control::Move).center)
                }
                Some(Control::Shield) => {
                    self.clicked = true;
                    self.pending.triggers[0] = 1.0;
                    Role::Shield
                }
                Some(Control::Offhand) => {
                    self.clicked = true;
                    self.custom_events
                        .push(("action:key.swapOffhand".into(), true));
                    self.custom_events
                        .push(("action:key.swapOffhand".into(), false));
                    Role::Ignore
                }
                Some(Control::Chat) => {
                    self.chat_requested = true;
                    Role::Ignore
                }
                Some(Control::Settings) => {
                    self.settings_requested = true;
                    Role::Ignore
                }
                Some(Control::Debug) => {
                    self.debug_requested = true;
                    Role::Ignore
                }
                Some(Control::Perspective) => {
                    self.perspective_requested = true;
                    Role::Ignore
                }
                Some(Control::Sneak)
                    if self.settings.mode == Mode::Dpad
                        && !self.vertical_movement
                        && !self.swimming
                        && !self.mounted =>
                {
                    // The classic D-pad toggles crouching on a double tap.
                    let now = Instant::now();
                    if self
                        .last_sneak_tap
                        .take()
                        .is_some_and(|t| now.duration_since(t) < Duration::from_millis(300))
                    {
                        Role::Button(Button::RightStick)
                    } else {
                        self.last_sneak_tap = Some(now);
                        Role::Ignore
                    }
                }
                Some(Control::Sprint) if self.settings.sprint_toggle => {
                    self.sprint_latched = !self.sprint_latched;
                    Role::Ignore
                }
                Some(c) => Self::button(c).map_or(Role::Ignore, Role::Button),
                None => Role::Look {
                    start: point,
                    since: Instant::now(),
                    dragged: false,
                    held_use: self.hold_uses_item,
                    committed: false,
                },
            };
            self.fingers.insert(id, (role, point));
            if let Role::Button(button) = role {
                match button {
                    Button::LeftTrigger => self.pending.triggers[0] = 1.0,
                    Button::RightTrigger => self.pending.triggers[1] = 1.0,
                    _ => self.pending.set(button, true),
                }
            }
            return [0.0; 2];
        }
        let aspect = self.aspect();
        if let Some((role, previous)) = self.fingers.get_mut(&id) {
            let delta = if let Role::Look {
                start,
                dragged,
                since,
                committed,
                ..
            } = role
            {
                if !*dragged && since.elapsed() >= HOLD_DELAY {
                    *committed = true;
                }
                *dragged |= ((point[0] - start[0]) * aspect).hypot(point[1] - start[1]) > 0.018;
                if *dragged {
                    [point[0] - previous[0], point[1] - previous[1]]
                } else {
                    [0.0; 2]
                }
            } else {
                [0.0; 2]
            };
            *previous = point;
            return delta;
        }
        [0.0; 2]
    }
    pub fn clear(&mut self) {
        self.clear_custom();
        self.fingers.clear();
        self.last_sneak_tap = None;
        self.sprint_latched = false;
        self.pending = PadState::default();
        self.aim = None;
        self.tap_aim = None;
        self.pending_tap = None;
        self.last_marker = None;
        self.holding_use = false;
        self.holding = false;
        self.chat_requested = false;
        self.settings_requested = false;
        self.debug_requested = false;
        self.perspective_requested = false;
    }
    /// A short world tap never holds a mouse button into the next tick.
    /// Its item-use intent is resolved against the current entity target.
    pub fn take_tap(&mut self) -> Option<bool> {
        self.pending_tap.take()
    }
    pub fn poll_state(&mut self) -> PadState {
        let mut state = self.state().merge(&std::mem::take(&mut self.pending));
        self.aim = self.tap_aim.take();
        self.holding_use = false;
        self.holding = false;
        if !self.crosshair() || self.settings.crosshair_gestures {
            for (role, point) in self.fingers.values_mut() {
                if let Role::Look {
                    start: _,
                    since,
                    dragged,
                    held_use,
                    committed,
                } = role
                {
                    if !*dragged && since.elapsed() >= HOLD_DELAY {
                        *committed = true;
                    }
                    if !*dragged || *committed {
                        self.aim =
                            Some(if self.settings.crosshair() || self.projectile_crosshair {
                                [0.5, 0.5]
                            } else {
                                *point
                            });
                        if *committed {
                            self.holding = true;
                            self.holding_use = *held_use;
                            state.triggers[if *held_use { 0 } else { 1 }] = 1.0;
                            break;
                        }
                    }
                }
            }
        }
        state
    }
    /// Keep a contextual hold as use even after feeding consumed its last item.
    pub fn latch_use(&mut self) {
        for (role, _) in self.fingers.values_mut() {
            if let Role::Look {
                held_use,
                committed: true,
                ..
            } = role
            {
                *held_use = true;
                break;
            }
        }
        self.holding_use = true;
    }
    pub fn state(&self) -> PadState {
        let mut state = PadState::default();
        if self.settings.sprint_toggle && self.settings.sprint_button && self.sprint_latched {
            state.set(Button::LeftStick, true);
        }
        for (role, point) in self.fingers.values() {
            match role {
                Role::Move(anchor) => {
                    let radius = self.placement(Control::Move).size / 2.0;
                    state.left = [
                        (point[0] - anchor[0]) * self.aspect() / radius,
                        (anchor[1] - point[1]) / radius,
                    ];
                    if self.settings.mode == Mode::Dpad {
                        state.left = state
                            .left
                            .map(|v| if v.abs() < 1.0 / 3.0 { 0.0 } else { v.signum() });
                    }
                    // Bedrock starts auto sprint beyond the joystick background,
                    // and stops it as soon as the finger returns inside it.
                    let sprint = state.left[1] > 1.0;
                    let length = state.left[0].hypot(state.left[1]).max(1.0);
                    state.left = state.left.map(|v| v / length);
                    if self.settings.mode != Mode::Dpad
                        && self.settings.sprint_with_joystick
                        && sprint
                    {
                        state.set(Button::LeftStick, true);
                    }
                }
                Role::Button(Button::LeftTrigger) | Role::Shield => state.triggers[0] = 1.0,
                Role::Button(Button::RightTrigger) => state.triggers[1] = 1.0,
                Role::Button(b) => state.set(*b, true),
                _ => {}
            }
        }
        state
    }
}

/// Original optional resource-pack artwork. Missing states keep the default UI.
#[allow(clippy::too_many_arguments)]
pub fn theme_button(
    b: &mut HudBatch,
    a: &HudAtlas,
    name: &str,
    bounds: [f32; 4],
    pressed: bool,
    color: u32,
    frame: bool,
) -> bool {
    let (base, active) = match name {
        "chat" => (
            "gui/horizon/touch/chat_normal",
            "gui/horizon/touch/chat_pressed",
        ),
        "pause" => (
            "gui/horizon/touch/pause_normal",
            "gui/horizon/touch/pause_pressed",
        ),
        "settings" => (
            "gui/horizon/touch/settings_normal",
            "gui/horizon/touch/settings_pressed",
        ),
        "debug" => (
            "gui/horizon/touch/debug_normal",
            "gui/horizon/touch/debug_pressed",
        ),
        "perspective" => (
            "gui/horizon/touch/perspective_normal",
            "gui/horizon/touch/perspective_pressed",
        ),
        "button" => (
            "gui/horizon/touch/button_normal",
            "gui/horizon/touch/button_pressed",
        ),
        _ => return false,
    };
    let key = if pressed && a.sprites.contains_key(active) {
        active
    } else {
        base
    };
    let Some(rect) = a.sprites.get(key).copied() else {
        return false;
    };
    let [x, y, w, h] = bounds;
    if frame {
        crate::ui::nine_slice(b, a, key, x, y, w, h, color);
    } else {
        control_quad(b, a, x, y, w, h, rect, color);
    }
    true
}

/// Shared text controls: keep the raised frame at rest, visibly depress it while held.
fn text_button(
    b: &mut HudBatch,
    a: &HudAtlas,
    bounds: [f32; 4],
    label: &str,
    pressed: bool,
    color: u32,
    themed: bool,
) {
    let custom_art = themed && theme_button(b, a, "button", bounds, pressed, color, true);
    if !custom_art {
        let [r, g, blue, alpha] = color.to_le_bytes();
        let tint = if pressed {
            rgba(r / 2, g / 2, blue / 2, alpha)
        } else {
            color
        };
        menu_frame(b, a, bounds, tint);
    }
    let [x, y, w, h] = bounds;
    let offset = if pressed { (h / 18.0).min(2.0) } else { 0.0 };
    let width = HudBatch::text_width(a, label);
    let start = b.vertices.len();
    let label_color = if custom_art {
        rgba(25, 25, 25, color.to_le_bytes()[3])
    } else {
        color
    };
    b.text(
        a,
        label,
        x + (w - width) / 2.0,
        y + (h - 8.0) / 2.0 + offset,
        label_color,
        false,
    );
    let scale = ((w - 6.0).max(1.0) / width.max(1.0)).min(((h - 6.0).max(1.0) / 8.0).min(1.0));
    b.scale_since(start, x + w / 2.0, y + h / 2.0 + offset, scale, scale);
}

/// Reuse Chat's complete 18x18 border, including its bottommost pixel row.
fn menu_frame(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], color: u32) {
    let border = (w / 18.0 * 2.0).min(h / 2.0);
    let xs = [200.0, 202.0, 216.0, 218.0];
    let ys = [82.0, 84.0, 96.0, 100.0];
    let dx = [x, x + border, x + w - border, x + w];
    let dy = [y, y + border, y + h - border * 2.0, y + h];
    for row in 0..3 {
        for col in 0..3 {
            let source = if row == 1 && col == 1 {
                [202.0, 84.0, 1.0, 1.0]
            } else {
                [
                    xs[col],
                    ys[row],
                    xs[col + 1] - xs[col],
                    ys[row + 1] - ys[row],
                ]
            };
            region(
                b,
                a,
                "gui",
                source,
                [
                    dx[col],
                    dy[row],
                    dx[col + 1] - dx[col],
                    dy[row + 1] - dy[row],
                ],
                color,
            );
        }
    }
}

fn nine(
    b: &mut HudBatch,
    a: &HudAtlas,
    name: &str,
    border: f32,
    [x, y, w, h]: [f32; 4],
    color: u32,
) {
    if let Some(r) = a.sprites.get(&format!("gui/bedrock/{name}")).copied() {
        let xs = [0.0, border, r.w as f32 - border, r.w as f32];
        let ys = [0.0, border, r.h as f32 - border, r.h as f32];
        let dx = [x, x + border, x + w - border, x + w];
        let dy = [y, y + border, y + h - border, y + h];
        for row in 0..3 {
            for col in 0..3 {
                let mut cell = r;
                cell.u0 = r.u0 + (r.u1 - r.u0) * xs[col] / r.w as f32;
                cell.u1 = r.u0 + (r.u1 - r.u0) * xs[col + 1] / r.w as f32;
                cell.v0 = r.v0 + (r.v1 - r.v0) * ys[row] / r.h as f32;
                cell.v1 = r.v0 + (r.v1 - r.v0) * ys[row + 1] / r.h as f32;
                control_quad(
                    b,
                    a,
                    dx[col],
                    dy[row],
                    dx[col + 1] - dx[col],
                    dy[row + 1] - dy[row],
                    cell,
                    color,
                );
            }
        }
    }
}

fn sprite(b: &mut HudBatch, a: &HudAtlas, name: &str, [x, y, w, h]: [f32; 4], color: u32) {
    fitted_sprite(b, a, name, [x, y, w, h], color);
}
fn mining_radius(radius: f32, progress: f32) -> f32 {
    if progress.is_finite() {
        (radius - 1.5).max(0.0) * progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
fn circle_points() -> &'static [[f32; 2]; 65] {
    static POINTS: std::sync::OnceLock<[[f32; 2]; 65]> = std::sync::OnceLock::new();
    POINTS.get_or_init(|| {
        std::array::from_fn(|i| {
            let angle = i as f32 / 64.0 * std::f32::consts::TAU;
            [angle.cos(), angle.sin()]
        })
    })
}
// One non-overlapping quad per segment, rather than many blended pixel squares.
fn ring(b: &mut HudBatch, a: &HudAtlas, center: [f32; 2], radius: f32, color: u32) {
    let Some(white) = a.sprites.get("horizon/white") else {
        return;
    };
    let uv = [[(white.u0 + white.u1) / 2.0, (white.v0 + white.v1) / 2.0]; 4];
    for segment in circle_points().windows(2) {
        let point = |p: [f32; 2], r: f32| [center[0] + p[0] * r, center[1] + p[1] * r];
        b.quad(
            [
                point(segment[0], radius + 0.7),
                point(segment[1], radius + 0.7),
                point(segment[1], (radius - 0.7).max(0.0)),
                point(segment[0], (radius - 0.7).max(0.0)),
            ],
            uv,
            color,
        );
    }
}
/// Glyphs such as the inventory ellipsis are deliberately wide and short.
/// Fit their original pixels inside the hit target without stretching them.
fn fitted_sprite(b: &mut HudBatch, a: &HudAtlas, name: &str, [x, y, w, h]: [f32; 4], color: u32) {
    let key = format!("gui/bedrock/{name}");
    if let Some(rect) = a.sprites.get(&key).copied() {
        let scale = (w / rect.w as f32).min(h / rect.h as f32);
        let (width, height) = (rect.w as f32 * scale, rect.h as f32 * scale);
        control_quad(
            b,
            a,
            x + (w - width) / 2.0,
            y + (h - height) / 2.0,
            width,
            height,
            rect,
            color,
        );
    }
}
fn region(
    b: &mut HudBatch,
    a: &HudAtlas,
    name: &str,
    [sx, sy, sw, sh]: [f32; 4],
    [x, y, w, h]: [f32; 4],
    color: u32,
) {
    if let Some(mut r) = a.sprites.get(&format!("gui/bedrock/{name}")).copied() {
        let (u, v) = (r.u0, r.v0);
        let (du, dv) = ((r.u1 - u) / r.w as f32, (r.v1 - v) / r.h as f32);
        r.u0 = u + sx * du;
        r.u1 = u + (sx + sw) * du;
        r.v0 = v + sy * dv;
        r.v1 = v + (sy + sh) * dv;
        control_quad(b, a, x, y, w, h, r, color);
    }
}

/// Atlas-wide clamp cannot protect an individual button from adjacent texels.
/// Keep UVs at texel centres and snap both screen edges to physical pixels.
#[allow(clippy::too_many_arguments)]
fn control_quad(
    b: &mut HudBatch,
    a: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mut r: crate::hud::Rect,
    color: u32,
) {
    let half = [0.5 / a.width.max(1) as f32, 0.5 / a.height.max(1) as f32];
    r.u0 += half[0];
    r.u1 -= half[0];
    r.v0 += half[1];
    r.v1 -= half[1];
    let snap = |v: f32| (v * b.scale).round() / b.scale;
    let (left, top, right, bottom) = (snap(x), snap(y), snap(x + w), snap(y + h));
    b.rect_uv(left, top, right - left, bottom - top, r, color);
}

fn line(
    b: &mut crate::hud::HudBatch,
    a: &crate::hud::HudAtlas,
    p: [f32; 2],
    q: [f32; 2],
    color: u32,
) {
    let steps = (q[0] - p[0]).abs().max((q[1] - p[1]).abs()).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        b.fill(
            a,
            p[0] + (q[0] - p[0]) * t - 0.7,
            p[1] + (q[1] - p[1]) * t - 0.7,
            1.4,
            1.4,
            color,
        );
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn factory_swap_stays_beside_inventory_without_covering_action_buttons() {
        for (w, h) in [(2560, 1600), (2400, 1080), (1080, 2400), (240, 320)] {
            for left_handed in [false, true] {
                for mode in Mode::ALL {
                    let mut touch = TouchControls::default();
                    touch.viewport(w, h);
                    touch.settings.left_handed = left_handed;
                    touch.settings.mode = mode;
                    let swap = touch.placement(Control::Offhand).rect(touch.aspect());
                    for control in [
                        Control::Inventory,
                        Control::Sprint,
                        Control::Sneak,
                        Control::Jump,
                    ] {
                        let other = touch.placement(control).rect(touch.aspect());
                        let overlaps = swap[0] < other[0] + other[2]
                            && other[0] < swap[0] + swap[2]
                            && swap[1] < other[1] + other[3]
                            && other[1] < swap[1] + swap[3];
                        assert!(!overlaps, "{w}x{h}, {left_handed}, {mode:?}: {control:?}");
                    }
                    touch.settings.layouts[mode.index()][Control::Offhand as usize].center =
                        [0.55, 0.43];
                    let custom = touch.placement(Control::Offhand).center;
                    assert!((custom[0] - if left_handed { 0.45 } else { 0.55 }).abs() < 0.0001);
                    assert_eq!(custom[1], 0.43);
                }
            }
        }
    }
    #[test]
    fn camera_calibration_uses_physical_distance_on_both_axes() {
        let mut settings = Settings::default();
        for (w, h) in [(2400, 1080), (1080, 2400)] {
            let turn = camera_delta([100.0 / w as f32, 100.0 / h as f32], w, h, &settings);
            assert!((turn[0] - turn[1]).abs() < 0.0001);
            settings.sensitivity_vertical = 0.5;
            let tuned = camera_delta([100.0 / w as f32, 100.0 / h as f32], w, h, &settings);
            assert!((tuned[0] - tuned[1] * 2.0).abs() < 0.0001);
            settings.sensitivity_vertical = 1.0;
        }
    }

    #[test]
    fn surface_tools_tap_but_projectiles_keep_hold_use_and_center_aim() {
        for id in [
            "bone_meal",
            "flint_and_steel",
            "fire_charge",
            "water_bucket",
            "honeycomb",
        ] {
            assert!(item_uses_on_tap(&format!("minecraft:{id}")));
        }
        for id in [
            "bow",
            "crossbow",
            "snowball",
            "egg",
            "wind_charge",
            "trident",
        ] {
            let id = format!("minecraft:{id}");
            assert!(!item_uses_on_tap(&id));
            assert!(projectile_crosshair(&id));
        }
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        assert!(!projectile_crosshair("minecraft:firework_rocket"));
        t.projectile_crosshair = true;
        t.hold_uses_item = true;
        t.update(1, [0.6, 0.4], true, false);
        t.update(1, [0.6, 0.4], false, true);
        t.poll_state();
        assert_eq!(t.aim, Some([0.5, 0.5]));
        assert_eq!(t.take_tap(), Some(true));
    }

    #[test]
    fn new_buttons_preserve_old_layouts_and_release_shield() {
        let settings = Settings::default();
        let mut old = serde_json::to_value(&settings).unwrap();
        for row in old["layouts"].as_array_mut().unwrap() {
            row.as_array_mut().unwrap().truncate(12);
        }
        old["button_visibility"]
            .as_array_mut()
            .unwrap()
            .truncate(12);
        old["layouts"][0][0]["center"] = serde_json::json!([0.2, 0.7]);
        let restored: Settings = serde_json::from_value(old).unwrap();
        assert_eq!(restored.layouts[0][0].center, [0.2, 0.7]);
        assert!(!restored.offhand_button);
        let mut t = TouchControls::default();
        assert!(!t.visible(Control::Offhand));
        t.settings.offhand_button = true;
        let p = t.placement(Control::Offhand).center;
        t.update(1, p, true, false);
        assert_eq!(
            t.key_events(),
            [
                ("action:key.swapOffhand".into(), true),
                ("action:key.swapOffhand".into(), false)
            ]
        );
        t.clear();
        t.settings.shield_button = true;
        t.shield_available = true;
        let p = t.placement(Control::Shield).center;
        t.update(2, p, true, false);
        assert!(t.shield_held());
        t.update(2, [0.5, 0.4], false, false);
        assert!(t.shield_held());
        assert!(t.poll_state().triggers[0] > 0.0);
        t.update(2, [0.5, 0.4], false, true);
        assert!(!t.shield_held());
        assert_eq!(t.poll_state().triggers[0], 0.0);
    }

    #[test]
    fn entity_placement_taps_do_not_change_consumption_and_throwing_holds() {
        for id in [
            "oak_boat",
            "oak_chest_boat",
            "bamboo_raft",
            "bamboo_chest_raft",
            "villager_spawn_egg",
            "minecart",
            "chest_minecart",
            "armor_stand",
            "item_frame",
            "painting",
            "end_crystal",
        ] {
            assert!(
                item_places_entity_on_tap(&format!("minecraft:{id}")),
                "{id}"
            );
        }
        for id in [
            "egg",
            "snowball",
            "wind_charge",
            "firework_rocket",
            "fishing_rod",
            "apple",
            "wheat_seeds",
            "diamond_helmet",
            "bow",
        ] {
            assert!(
                !item_places_entity_on_tap(&format!("minecraft:{id}")),
                "{id}"
            );
        }
        assert!(!item_places_entity_on_tap("example:spell_boat"));
    }

    #[test]
    fn world_taps_never_press_attack_or_use_and_remember_the_original_item() {
        for (mode, split) in [
            (Mode::Tap, false),
            (Mode::Dpad, false),
            (Mode::Crosshair, false),
            (Mode::Dpad, true),
        ] {
            for held_use in [false, true] {
                let mut t = TouchControls::default();
                t.settings.mode = mode;
                t.settings.split_controls = split;
                t.settings.crosshair_gestures = true;
                t.hold_uses_item = held_use;
                t.update(1, [0.6, 0.4], true, false);
                t.hold_uses_item = !held_use;
                t.update(1, [0.6, 0.4], false, true);
                assert_eq!(t.poll_state().triggers, [0.0; 2]);
                assert_eq!(t.take_tap(), Some(held_use));
                assert_eq!(t.take_tap(), None);
                t.update(2, [0.6, 0.4], true, false);
                t.update(2, [0.6, 0.4], false, true);
                t.clear();
                assert_eq!(t.take_tap(), None);
            }
        }
    }
    #[test]
    fn sprint_toggle_survives_finger_release_and_clears_on_focus_loss() {
        let mut t = TouchControls {
            loaded: true,
            aspect: 16.0 / 9.0,
            ..Default::default()
        };
        t.settings.sprint_toggle = true;
        let point = t.placement(Control::Sprint).center;
        t.update(1, point, true, false);
        t.update(1, point, false, true);
        assert!(t.poll_state().held(Button::LeftStick));
        t.update(2, point, true, false);
        t.update(2, point, false, true);
        assert!(!t.poll_state().held(Button::LeftStick));
        t.update(3, point, true, false);
        t.clear();
        assert!(!t.poll_state().held(Button::LeftStick));
        t.settings.sprint_toggle = false;
        t.update(4, point, true, false);
        assert!(t.poll_state().held(Button::LeftStick));
        t.update(4, point, false, true);
        assert!(!t.poll_state().held(Button::LeftStick));
    }
    #[test]
    fn global_button_scale_changes_hit_geometry_without_rewriting_saved_layouts() {
        let mut touch = TouchControls::default();
        touch.viewport(1920, 1080);
        let before = touch.placement(Control::Jump);
        let stored = touch.settings.placement(Control::Jump).size;
        touch.settings.button_scale = 1.5;
        let after = touch.placement(Control::Jump);
        assert!((after.size - before.size * 1.5).abs() < 0.00001);
        assert_eq!(touch.settings.placement(Control::Jump).size, stored);
        touch.settings.button_scale = f32::NAN;
        touch.settings.sanitize();
        assert_eq!(touch.settings.button_scale, 1.0);
    }

    #[test]
    fn oversized_controls_can_be_dragged_on_narrow_portrait_surfaces() {
        let mut touch = TouchControls {
            aspect: 0.4,
            loaded: true,
            ..Default::default()
        };
        touch.settings.layouts[0][Control::Move as usize].size = 0.45;
        touch.open_editor();
        let center = touch.placement(Control::Move).center;
        touch.edit_touch(77, center, true, false);
        assert_eq!(touch.editor.as_ref().unwrap().selected, Some(Control::Move));
        touch.edit_touch(77, [0.95, 0.95], false, false);
        let [x, y, w, h] = touch.placement(Control::Move).rect(touch.aspect());
        assert!(x >= 0.0 && y >= 0.0 && x + w <= 1.0001 && y + h <= 1.0001);
        touch.cancel_editor();
    }
    #[test]
    fn portrait_factory_shortcuts_do_not_overlap_and_margins_do_not_move_controls() {
        let mut touch = TouchControls::default();
        touch.viewport(480, 854);
        let controls = [
            Control::Debug,
            Control::Chat,
            Control::Pause,
            Control::Settings,
            Control::Perspective,
        ];
        let rects: Vec<_> = controls
            .iter()
            .map(|c| touch.placement(*c).rect(touch.aspect()))
            .collect();
        for pair in rects.windows(2) {
            assert!(pair[0][0] + pair[0][2] < pair[1][0]);
        }
        touch.settings.horizontal_margin = 0.12;
        touch.settings.vertical_margin = 0.12;
        assert_eq!(
            rects,
            controls
                .iter()
                .map(|c| touch.placement(*c).rect(touch.aspect()))
                .collect::<Vec<_>>()
        );
        touch.settings.layouts[0][Control::Chat as usize].center = [0.25, 0.2];
        assert_eq!(touch.placement(Control::Chat).center, [0.25, 0.2]);
    }

    use super::*;
    #[test]
    fn editor_add_row_creates_a_button_without_closing_the_editor() {
        let mut touch = TouchControls::default();
        touch.viewport(1920, 1080);
        touch.open_editor();
        assert!(!touch.edit_touch(1, [0.15, 0.165], true, true));
        assert!(!touch.edit_touch(2, [0.5, 0.875], true, true));
        assert!(touch.editor.is_some());
        assert_eq!(touch.settings.custom_buttons.len(), 1);
        assert_eq!(touch.settings.custom_buttons[0].key, "key.keyboard.unknown");
        touch.cancel_editor();
        assert!(touch.settings.custom_buttons.is_empty());
    }

    #[test]
    fn keypad_utility_overlay_keeps_buttons_without_movement_targets() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.auxiliary_only = true;
        assert!(!t.visible(Control::Move));
        assert!(!t.visible(Control::Jump));
        assert!(t.visible(Control::Debug));
        let at = t.placement(Control::Debug).center;
        assert!(t.auxiliary_at(at));
        t.update(77, at, true, false);
        assert!(t.debug_requested);
        t.update(77, at, false, true);
        assert!(!t.owns_pointer(77));
    }
    #[test]
    fn attack_gauge_survives_tap_modes_and_respects_off() {
        assert_eq!(attack_indicator(0, false), 0);
        assert_eq!(attack_indicator(1, false), 2);
        assert_eq!(attack_indicator(1, true), 1);
        assert_eq!(attack_indicator(2, false), 2);
    }
    #[test]
    fn shortcuts_consume_their_finger_and_preserve_movement() {
        let mut touch = TouchControls::default();
        touch.update(1, [0.13, 0.76], true, false);
        touch.update(1, [0.13, 0.68], false, false);
        let moving = touch.poll_state().left;
        assert_ne!(moving, [0.0; 2]);
        for (id, control) in [(2, Control::Debug), (3, Control::Perspective)] {
            let point = touch.settings.placement(control).center;
            touch.update(id, point, true, false);
            assert!(matches!(touch.fingers[&id].0, Role::Ignore));
            assert_eq!(touch.poll_state().left, moving);
            assert_eq!(touch.poll_state().triggers, [0.0; 2]);
            touch.update(id, point, false, true);
        }
        assert!(touch.debug_requested && touch.perspective_requested);
        touch.clear();
        assert!(!touch.debug_requested && !touch.perspective_requested);
    }
    #[test]
    fn mining_indicator_grows_outwards_with_bounded_radius() {
        assert_eq!(mining_radius(16.0, 0.0), 0.0);
        assert_eq!(mining_radius(16.0, 0.5), 7.25);
        assert_eq!(mining_radius(16.0, 1.0), 14.5);
        assert_eq!(mining_radius(16.0, 2.0), 14.5);
        assert_eq!(mining_radius(16.0, -1.0), 0.0);
        assert_eq!(mining_radius(16.0, f32::NAN), 0.0);
    }
    #[test]
    fn contextual_button_is_opt_in_targeted_and_owned_until_release() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.context_use = true;
        assert!(!t.visible(Control::Use));
        t.settings.contextual_interact = true;
        assert!(t.visible(Control::Use));
        t.update(1, [0.92, 0.52], true, false);
        t.context_use = false;
        assert!(t.visible(Control::Use));
        assert_eq!(t.poll_state().triggers[0], 1.0);
        t.update(1, [0.92, 0.52], false, true);
        assert!(!t.visible(Control::Use));
        t.settings.mode = Mode::Crosshair;
        assert!(t.visible(Control::Use));
    }
    #[test]
    fn looking_with_food_does_not_start_eating_and_use_button_never_disappears() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.hold_uses_item = true;
        t.update(1, [0.6, 0.4], true, false);
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        t.update(1, [0.7, 0.4], false, false);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - Duration::from_secs(1);
        }
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        assert!(!t.visible(Control::Use));
        assert_eq!(t.pointer(), Some([0.7, 0.4]));
    }
    #[test]
    fn all_four_schemes_are_selectable_and_large_icons_remain_inside_the_surface() {
        let mut t = TouchControls::default();
        for (i, (mode, split, _)) in Mode::SCHEMES.into_iter().enumerate() {
            t.settings.mode = mode;
            t.settings.split_controls = split;
            assert_eq!(t.settings.mode, mode);
            assert_eq!(t.settings.split_controls, split);
            assert_eq!(t.settings.crosshair(), i == 0 || i == 3);
        }
        t.cancel_editor();
        t.settings.layouts[0][Control::Jump as usize].center = [0.99, 0.99];
        t.settings.layouts[0][Control::Jump as usize].size = 0.4;
        let r = t.placement(Control::Jump).rect(t.aspect());
        assert!(r[0] >= 0.0 && r[1] >= 0.0 && r[0] + r[2] <= 1.00001 && r[1] + r[3] <= 1.00001);
    }
    #[test]
    fn optional_crosshair_gestures_target_center_for_both_movement_schemes() {
        for (mode, split) in [(Mode::Crosshair, false), (Mode::Dpad, true)] {
            let mut t = TouchControls::default();
            t.settings.mode = mode;
            t.settings.split_controls = split;
            t.settings.crosshair_gestures = true;
            t.update(1, [0.6, 0.4], true, false);
            t.update(1, [0.6, 0.4], false, true);
            assert_eq!(t.poll_state().triggers, [0.0; 2]);
            assert_eq!(t.take_tap(), Some(false));
            assert_eq!(t.aim, Some([0.5, 0.5]));
            t.inventory_mode = true;
            t.settings.button_visibility[Control::Attack as usize] = 2;
            assert!(!t.visible(Control::Attack));
        }
    }
    #[test]
    fn quick_tap_survives_one_poll_and_shared_buttons_release_independently() {
        let mut t = TouchControls::default();
        t.update(1, [0.92, 0.78], true, false);
        t.update(1, [0.92, 0.78], false, true);
        assert!(t.poll_state().held(Button::South));
        assert!(!t.poll_state().held(Button::South));
        t.update(2, [0.80, 0.64], true, false);
        t.update(3, [0.80, 0.64], true, false);
        t.update(2, [0.80, 0.64], false, true);
        assert_eq!(t.state().triggers[1], 1.0);
        t.update(3, [0.80, 0.64], false, true);
        assert_eq!(t.state().triggers[1], 0.0);
    }
    #[test]
    fn joystick_camera_and_jump_have_independent_owners() {
        let mut t = TouchControls::default();
        t.update(1, [0.13, 0.76], true, false);
        t.update(1, [0.20, 0.64], false, false);
        t.update(2, [0.6, 0.4], true, false);
        let d = t.update(2, [0.65, 0.45], false, false);
        assert!((d[0] - 0.05).abs() < 0.0001);
        t.update(3, [0.92, 0.78], true, false);
        assert!(t.state().held(Button::South));
        assert!(t.state().left[0] > 0.6);
        t.update(4, [0.09, 0.76], true, false);
        t.update(4, [0.02, 0.70], false, false);
        assert!(t.state().left[0] > 0.6);
        t.clear();
        assert_eq!(t.poll_state(), PadState::default());
    }
    #[test]
    fn auto_sprint_starts_outside_and_stops_inside_the_joystick() {
        let mut t = TouchControls::default();
        t.settings.sprint_with_joystick = true;
        t.update(1, [0.13, 0.76], true, false);
        t.update(1, [0.13, 0.62], false, false);
        assert!(!t.state().held(Button::LeftStick));
        t.update(1, [0.13, 0.58], false, false);
        assert!(t.state().held(Button::LeftStick));
        t.update(1, [0.13, 0.62], false, false);
        assert!(!t.state().held(Button::LeftStick));
    }
    #[test]
    fn tap_aim_and_camera_drag_are_distinct() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.update(1, [0.6, 0.4], true, false);
        t.update(1, [0.6, 0.4], false, true);
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        assert_eq!(t.take_tap(), Some(false));
        assert_eq!(t.aim, Some([0.6, 0.4]));
        assert_eq!(t.poll_state().triggers[0], 0.0);
        assert_eq!(t.aim, None);
        t.update(2, [0.6, 0.4], true, false);
        t.update(2, [0.7, 0.4], false, false);
        t.update(2, [0.7, 0.4], false, true);
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
    }

    #[test]
    fn long_hold_keeps_mining_while_the_camera_turns() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.update(1, [0.6, 0.4], true, false);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - Duration::from_millis(400);
        }
        assert_eq!(t.poll_state().triggers[1], 1.0);
        assert_eq!(t.aim, Some([0.6, 0.4]));
        assert_ne!(t.update(1, [0.7, 0.4], false, false), [0.0; 2]);
        assert_eq!(t.poll_state().triggers[1], 1.0);
        assert_eq!(t.aim, Some([0.7, 0.4]));
        t.update(1, [0.7, 0.4], false, true);
        assert_eq!(t.poll_state().triggers[1], 0.0);
    }
    #[test]
    fn held_food_survives_drift_and_releases_with_its_owner() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.hold_uses_item = true;
        t.update(1, [0.6, 0.4], true, false);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - Duration::from_millis(400);
        }
        assert_eq!(t.poll_state().triggers, [1.0, 0.0]);
        assert!(t.holding_use);
        assert_ne!(t.update(1, [0.66, 0.46], false, false), [0.0; 2]);
        t.hold_uses_item = false; // consuming the last item cannot become mining
        assert_eq!(t.poll_state().triggers, [1.0, 0.0]);
        assert_eq!(t.aim, Some([0.66, 0.46]));
        t.update(2, [0.4, 0.3], true, false);
        assert_ne!(t.update(2, [0.45, 0.3], false, false), [0.0; 2]);
        assert_eq!(t.poll_state().triggers, [1.0, 0.0]);
        t.update(1, [0.66, 0.46], false, true);
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        assert!(!t.holding_use);
    }
    #[test]
    fn contextual_hold_can_latch_use_without_turning_short_taps_into_holds() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.update(1, [0.6, 0.4], true, false);
        t.poll_state();
        assert!(!t.holding);
        if let Some((Role::Look { since, .. }, _)) = t.fingers.get_mut(&1) {
            *since = Instant::now() - Duration::from_millis(400);
        }
        assert_eq!(t.poll_state().triggers, [0.0, 1.0]);
        assert!(t.holding);
        t.latch_use();
        assert_eq!(t.poll_state().triggers, [1.0, 0.0]);
        assert_ne!(t.update(1, [0.66, 0.4], false, false), [0.0; 2]);
        t.update(1, [0.66, 0.4], false, true);
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        assert!(!t.holding);
        t.update(2, [0.6, 0.4], true, false);
        t.update(2, [0.6, 0.4], false, true);
        assert_eq!(t.poll_state().triggers, [0.0, 0.0]);
        assert_eq!(t.take_tap(), Some(false));
        assert!(!t.holding);
    }
    #[test]
    fn touch_marker_follows_camera_drag_without_starting_an_action() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        assert_eq!(t.pointer(), None);
        t.update(1, [0.6, 0.4], true, false);
        assert_eq!(t.pointer(), Some([0.6, 0.4]));
        assert_eq!(t.poll_state().triggers, [0.0; 2]);
        assert_eq!(t.aim, Some([0.6, 0.4]));
        t.update(1, [0.7, 0.4], false, false);
        t.poll_state();
        assert_eq!(t.pointer(), Some([0.7, 0.4]));
        assert_eq!(t.aim, None);
        t.clear();
        t.settings.mode = Mode::Crosshair;
        t.update(1, [0.6, 0.4], true, false);
        assert_eq!(t.pointer(), None);
    }
    #[test]
    fn cancel_layout_preserves_already_applied_touch_options() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Tap;
        t.settings.left_handed = true;
        let original = t.settings.placement(Control::Move).center;
        t.open_editor();
        t.edit_touch(1, original, true, false);
        t.edit_touch(1, [0.7, 0.7], false, true);
        assert_ne!(t.settings.placement(Control::Move).center, original);
        t.cancel_editor();
        assert_eq!(t.settings.placement(Control::Move).center, original);
        assert_eq!(t.settings.mode, Mode::Tap);
        assert!(t.settings.left_handed);
    }
    #[test]
    fn dpad_can_slide_between_directions_and_mirror() {
        let mut t = TouchControls::default();
        t.settings.mode = Mode::Dpad;
        t.update(1, [0.13, 0.66], true, false);
        assert_eq!(t.state().left, [0.0, 1.0]);
        t.update(1, [0.20, 0.76], false, false);
        assert_eq!(t.state().left, [1.0, 0.0]);
        t.clear();
        t.update(10, [0.13, 0.76], true, false);
        assert!(!t.poll_state().held(Button::RightStick));
        t.update(10, [0.13, 0.76], false, true);
        t.update(12, [0.13, 0.76], true, false);
        assert!(t.poll_state().held(Button::RightStick));
        t.clear();
        t.settings.swap_jump_sneak = true;
        t.update(11, [0.13, 0.76], true, false);
        assert!(t.state().held(Button::South));
        t.clear();
        t.settings.left_handed = true;
        t.update(2, [0.87, 0.66], true, false);
        assert_eq!(t.state().left, [0.0, 1.0]);
    }
    #[test]
    fn editing_owns_input_and_cancel_restores_layouts() {
        let mut t = TouchControls::default();
        t.open_editor();
        t.edit_touch(1, [0.13, 0.76], true, false);
        t.edit_touch(1, [0.18, 0.70], false, true);
        assert!((t.settings.placement(Control::Move).center[0] - 0.18).abs() < 0.001);
        assert_eq!(t.poll_state(), PadState::default());
        t.cancel_editor();
        assert_eq!(t.settings.placement(Control::Move).center, [0.13, 0.76]);
        t.settings.mode = Mode::Tap;
        t.open_editor();
        assert_eq!(t.settings.mode, Mode::Tap);
        t.edit_touch(3, [0.13, 0.76], true, true);
        t.edit_touch(4, [0.6, 0.82], true, true);
        t.edit_touch(5, [0.65, 0.73], true, true);
        assert!(t.settings.placement(Control::Move).size > 0.4);
        t.cancel_editor();
        assert_eq!(t.settings.mode, Mode::Tap);
    }
    #[test]
    fn editor_deselects_and_can_apply_opacity_to_the_current_mode() {
        let mut t = TouchControls::default();
        t.open_editor();
        t.edit_touch(1, [0.13, 0.76], true, true);
        t.edit_touch(2, [0.4, 0.82], true, true); // Opacity
        t.edit_touch(3, [0.5, 0.64], true, true); // Apply to all
        t.edit_touch(4, [0.5, 0.73], true, true);
        for p in t.settings.layouts[0] {
            assert!((p.opacity - 0.5).abs() < 0.001);
        }
        assert_eq!(t.settings.layouts[1][0].opacity, 0.65);
        t.edit_touch(5, [0.7, 0.4], true, true);
        assert!(t.editor.as_ref().unwrap().selected.is_none());
        t.cancel_editor();
        assert_eq!(t.settings.layouts[0][0].opacity, 0.65);
    }
    #[test]
    fn layouts_roundtrip_and_reject_invalid_sizes() {
        let mut s = Settings::default();
        s.mode = Mode::Dpad;
        s.layouts[2][2].center = [0.4, 0.5];
        let bytes = serde_json::to_vec(&s).unwrap();
        let mut r: Settings = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r.mode, Mode::Dpad);
        assert_eq!(r.layouts[2][2].center, [0.4, 0.5]);
        r.layouts[2][2].size = f32::NAN;
        r.sensitivity = f32::INFINITY;
        r.sanitize();
        assert!(r.layouts[2][2].size.is_finite());
        assert_eq!(r.sensitivity, 1.0);
        let rect = r.placement(Control::Jump).rect(20.0 / 9.0);
        assert!((rect[2] * (20.0 / 9.0) - rect[3]).abs() < 0.0001);
    }
    #[test]
    fn projection_targets_screen_point_without_turning_camera() {
        let forward = [0.0, 0.0, 1.0];
        let up = [0.0, 1.0, 0.0];
        assert_eq!(
            screen_direction(forward, up, [0.5, 0.5], 2.0, 90.0),
            forward
        );
        let ray = screen_direction(forward, up, [0.75, 0.25], 2.0, 90.0);
        assert!(ray[0] < 0.0 && ray[1] > 0.0 && ray[2] > 0.0);
        assert!((ray.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() < 1e-10);
    }
    #[test]
    fn hotbar_all_nine_slots() {
        for i in 0..9 {
            assert_eq!(
                hotbar_slot(427.0, 240.0, [133.5 + i as f32 * 20.0, 230.0]),
                Some(i)
            );
        }
        assert_eq!(hotbar_slot(427.0, 240.0, [320.0, 230.0]), None);
    }
    #[test]
    fn old_profiles_keep_custom_layouts_and_new_preferences_have_safe_defaults() {
        let mut settings: Settings =
            serde_json::from_str(r#"{"mode":"tap","sensitivity":1.5}"#).unwrap();
        settings.sanitize();
        assert_eq!(settings.mode, Mode::Tap);
        assert_eq!(settings.sensitivity, 1.5);
        assert_eq!(settings.circle_size, 32.0);
        assert!(!settings.analog_movement);
        assert!(settings.sprint_button);
        settings.circle_size = f32::NAN;
        settings.sanitize();
        assert_eq!(settings.circle_size, 32.0);
        settings.sprint_button = false;
        let center = settings.placement(Control::Sprint).center;
        let mut touch = TouchControls {
            settings,
            ..Default::default()
        };
        touch.update(1, center, true, false);
        assert!(!touch.poll_state().held(Button::LeftStick));
        touch.update(1, center, false, true);
        touch.settings.sprint_button = true;
        touch.update(2, center, true, false);
        assert!(touch.poll_state().held(Button::LeftStick));
        touch.update(2, center, false, true);
        touch.poll_state(); // consume the queued tap
        assert!(!touch.poll_state().held(Button::LeftStick));
    }
}
