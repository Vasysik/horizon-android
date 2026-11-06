//! Per-profile touch layouts. Coordinates are normalized; sizes are fractions
//! of surface height, so a button stays square on wide phones and tablets.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    Crosshair,
    Tap,
    Dpad,
}
impl Mode {
    pub const ALL: [Self; 3] = [Self::Crosshair, Self::Tap, Self::Dpad];
    pub fn label(self) -> &'static str {
        match self {
            Self::Crosshair => "Joystick & aim crosshair",
            Self::Tap => "Joystick & tap to interact",
            Self::Dpad => "D-Pad & tap to interact",
        }
    }
    pub const SCHEMES: [(Self, bool, &str); 4] = [
        (Self::Crosshair, false, "Joystick & aim crosshair"),
        (Self::Tap, false, "Joystick & tap to interact"),
        (Self::Dpad, false, "D-Pad & tap to interact"),
        (Self::Dpad, true, "D-Pad & aim crosshair"),
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Crosshair => 0,
            Self::Tap => 1,
            Self::Dpad => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Move,
    Sprint,
    Jump,
    Sneak,
    Attack,
    Use,
    Chat,
    Pause,
    Settings,
    Inventory,
    Debug,
    Perspective,
    Offhand,
    Shield,
}
impl Control {
    pub const ALL: [Self; 14] = [
        Self::Move,
        Self::Sprint,
        Self::Jump,
        Self::Sneak,
        Self::Attack,
        Self::Use,
        Self::Chat,
        Self::Pause,
        Self::Settings,
        Self::Inventory,
        Self::Debug,
        Self::Perspective,
        Self::Offhand,
        Self::Shield,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Move => "Movement",
            Self::Sprint => "Sprint",
            Self::Jump => "Jump / fly up",
            Self::Sneak => "Sneak / fly down",
            Self::Attack => "Attack / mine",
            Self::Use => "Use / place",
            Self::Chat => "Chat",
            Self::Pause => "Pause",
            Self::Settings => "Touch settings",
            Self::Inventory => "Inventory",
            Self::Debug => "Debug overlay (F3)",
            Self::Perspective => "Camera perspective (F5)",
            Self::Offhand => "Swap hands",
            Self::Shield => "Raise shield",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlStyle {
    /// Classic Bedrock look: every built-in control in the MCPE style.
    #[default]
    Mcpe,
    /// GoyControl style (customizable static buttons, custom joystick, up to 3 events, no icons).
    GoyControl,
}
impl ControlStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mcpe => "MCPE (Default)",
            Self::GoyControl => "GoyControl",
        }
    }
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Mcpe => "Vanilla touch controls",
            Self::GoyControl => "Custom layout with more options",
        }
    }
    pub fn section_title(self) -> &'static str {
        match self {
            Self::Mcpe => "MCPE Controls",
            Self::GoyControl => "GoyControl Controls",
        }
    }
    pub fn storage_subdir(self) -> &'static str {
        match self {
            Self::Mcpe => "controls/mcpe",
            Self::GoyControl => "controls/goycontrol",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system: ControlStyle,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JoystickMoveArea {
    /// Move in mid-bottom-left region from anywhere inside that zone.
    MidBottomLeft,
    /// Move anywhere on the right side (ignores buttons).
    RightSideAnywhere,
    /// Joystick stays in its fixed absolute placement.
    AbsoluteFixed,
    /// Floating joystick center follows the finger where touched.
    #[default]
    FollowFinger,
}
impl JoystickMoveArea {
    pub const ALL: [Self; 4] = [
        Self::MidBottomLeft,
        Self::RightSideAnywhere,
        Self::AbsoluteFixed,
        Self::FollowFinger,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::MidBottomLeft => "Mid-Bottom Left (Anywhere)",
            Self::RightSideAnywhere => "Right Side (Ignores Buttons)",
            Self::AbsoluteFixed => "Joystick in Absolute Place",
            Self::FollowFinger => "Follow Finger (Floating)",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::MidBottomLeft => Self::RightSideAnywhere,
            Self::RightSideAnywhere => Self::AbsoluteFixed,
            Self::AbsoluteFixed => Self::FollowFinger,
            Self::FollowFinger => Self::MidBottomLeft,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JoystickStyle {
    pub stick_stroke: f32,
    pub stick_stroke_color: u32,
    pub stick_color: u32,
    pub stick_opacity: f32,
    pub stick_size: f32,
    pub base_stroke: f32,
    pub base_stroke_color: u32,
    pub base_color: u32,
    pub base_opacity: f32,
    pub base_roundness: f32,
    pub move_area: JoystickMoveArea,
}
impl Default for JoystickStyle {
    fn default() -> Self {
        Self {
            stick_stroke: 2.0,
            stick_stroke_color: 0xFF9A7BFF,
            stick_color: 0xFFD8CEFF,
            stick_opacity: 0.90,
            stick_size: 0.42,
            base_stroke: 2.0,
            base_stroke_color: 0xFF6B5B95,
            base_color: 0xFF1A1726,
            base_opacity: 0.72,
            base_roundness: 50.0,
            move_area: JoystickMoveArea::FollowFinger,
        }
    }
}
impl JoystickStyle {
    pub fn sanitize(&mut self) {
        self.stick_stroke = if self.stick_stroke.is_finite() {
            self.stick_stroke.clamp(0.0, 8.0)
        } else {
            2.0
        };
        self.stick_opacity = if self.stick_opacity.is_finite() {
            self.stick_opacity.clamp(0.1, 1.0)
        } else {
            0.90
        };
        self.stick_size = if self.stick_size.is_finite() {
            self.stick_size.clamp(0.20, 0.85)
        } else {
            0.42
        };
        self.base_stroke = if self.base_stroke.is_finite() {
            self.base_stroke.clamp(0.0, 8.0)
        } else {
            2.0
        };
        self.base_opacity = if self.base_opacity.is_finite() {
            self.base_opacity.clamp(0.1, 1.0)
        } else {
            0.72
        };
        self.base_roundness = if self.base_roundness.is_finite() {
            self.base_roundness.clamp(0.0, 50.0)
        } else {
            50.0
        };
    }
}

fn default_true() -> bool {
    true
}
fn default_one() -> f32 {
    1.0
}
fn default_stroke_size() -> f32 {
    1.6
}
fn default_roundness() -> f32 {
    10.0
}
fn default_stroke_color() -> u32 {
    0xFF8A7AE6
}
fn default_button_color() -> u32 {
    0xFF1E1B2E
}
fn default_shadow_size() -> f32 {
    0.0
}
fn default_shadow_color() -> u32 {
    0x96000000
}
fn default_click_color() -> u32 {
    0xFF423675
}
fn default_click_stroke() -> u32 {
    0xFFD0BCFF
}
fn default_toggle_color() -> u32 {
    0xFF543B94
}
fn default_toggle_stroke() -> u32 {
    0xFFEADDFF
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub center: [f32; 2],
    pub size: f32,
    pub opacity: f32,
    /// GoyControl Pass-through: dragging on this button also turns the camera.
    #[serde(default)]
    pub passthrough: bool,
    /// GoyControl Toggle mode: tapping latches the button on/off until tapped again.
    #[serde(default)]
    pub toggleable: bool,
    /// GoyControl Swipeable: sliding a finger from outside onto this button activates it.
    #[serde(default)]
    pub swipeable: bool,
    /// Button shape: 0 = Rounded, 1 = Square, 2 = Circle/Pill, 3 = CutOctagon, 4 = Diamond, 5 = Hexagon.
    #[serde(default)]
    pub shape: u8,
    /// Optional vector icon badge (0 = None/Text only, 1..=12 = built-in vector icon).
    #[serde(default)]
    pub icon_id: u8,
    /// Independent horizontal expansion ratio (drag right/left handle).
    #[serde(default = "default_one")]
    pub width_ratio: f32,
    /// Independent vertical expansion ratio (drag bottom/top handle).
    #[serde(default = "default_one")]
    pub height_ratio: f32,
    /// Component Style Manager: false = Default (MCPE look), true = Static (GoyControl look).
    #[serde(default)]
    pub use_goy_style: bool,
    #[serde(default = "default_stroke_size")]
    pub stroke_size: f32,
    #[serde(default)]
    pub stroke_margin: f32,
    #[serde(default = "default_roundness")]
    pub stroke_roundness: f32,
    #[serde(default = "default_stroke_color")]
    pub stroke_color: u32,
    #[serde(default = "default_button_color")]
    pub button_color: u32,
    #[serde(default = "default_shadow_size")]
    pub shadow_size: f32,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: u32,
    #[serde(default = "default_roundness")]
    pub roundness: f32,
    #[serde(default = "default_click_color")]
    pub on_click_color: u32,
    #[serde(default = "default_click_stroke")]
    pub on_click_stroke_color: u32,
    #[serde(default = "default_toggle_color")]
    pub on_toggle_color: u32,
    #[serde(default = "default_toggle_stroke")]
    pub on_toggle_stroke_color: u32,
    /// Whether this control/button is visible while playing in the 3D world.
    #[serde(default = "default_true")]
    pub visible_in_game: bool,
    /// Whether this control/button is visible & interactive while a menu/inventory/title screen is open.
    #[serde(default)]
    pub visible_in_menu: bool,
}
impl Placement {
    pub fn new(x: f32, y: f32, size: f32) -> Self {
        Self {
            center: [x, y],
            size,
            opacity: 0.65,
            passthrough: false,
            toggleable: false,
            swipeable: false,
            shape: 0,
            icon_id: 0,
            width_ratio: 1.0,
            height_ratio: 1.0,
            use_goy_style: false,
            stroke_size: 1.6,
            stroke_margin: 0.0,
            stroke_roundness: 10.0,
            stroke_color: 0xFF8A7AE6,
            button_color: 0xFF1E1B2E,
            shadow_size: 0.0,
            shadow_color: 0x96000000,
            roundness: 10.0,
            on_click_color: 0xFF423675,
            on_click_stroke_color: 0xFFD0BCFF,
            on_toggle_color: 0xFF543B94,
            on_toggle_stroke_color: 0xFFEADDFF,
            visible_in_game: true,
            visible_in_menu: false,
        }
    }
    pub fn rect(self, aspect: f32) -> [f32; 4] {
        let wr = if self.width_ratio.is_finite() && self.width_ratio > 0.05 {
            self.width_ratio
        } else {
            1.0
        };
        let hr = if self.height_ratio.is_finite() && self.height_ratio > 0.05 {
            self.height_ratio
        } else {
            1.0
        };
        let sx = (self.size * wr) / aspect.max(0.1);
        let sy = self.size * hr;
        [self.center[0] - sx / 2.0, self.center[1] - sy / 2.0, sx, sy]
    }
    pub fn sanitize(&mut self) {
        if !self.size.is_finite() {
            self.size = 0.15;
        }
        if !self.opacity.is_finite() {
            self.opacity = 0.65;
        }
        self.size = self.size.clamp(0.045, 0.45);
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.shape = self.shape.min(5);
        self.icon_id = self.icon_id.min(12);
        self.width_ratio = if self.width_ratio.is_finite() && self.width_ratio > 0.05 {
            self.width_ratio.clamp(0.4, 3.2)
        } else {
            1.0
        };
        self.height_ratio = if self.height_ratio.is_finite() && self.height_ratio > 0.05 {
            self.height_ratio.clamp(0.4, 3.2)
        } else {
            1.0
        };
        self.stroke_size = if self.stroke_size.is_finite() {
            self.stroke_size.clamp(0.0, 8.0)
        } else {
            1.6
        };
        self.stroke_margin = if self.stroke_margin.is_finite() {
            self.stroke_margin.clamp(-4.0, 8.0)
        } else {
            0.0
        };
        self.stroke_roundness = if self.stroke_roundness.is_finite() {
            self.stroke_roundness.clamp(0.0, 50.0)
        } else {
            10.0
        };
        self.shadow_size = if self.shadow_size.is_finite() {
            self.shadow_size.clamp(0.0, 12.0)
        } else {
            0.0
        };
        self.roundness = if self.roundness.is_finite() {
            self.roundness.clamp(0.0, 50.0)
        } else {
            10.0
        };
        for c in &mut self.center {
            *c = if c.is_finite() {
                c.clamp(0.025, 0.975)
            } else {
                0.5
            };
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub split_controls: bool,
    pub contextual_interact: bool,
    pub left_handed: bool,
    pub sprint_with_joystick: bool,
    pub swap_jump_sneak: bool,
    pub sensitivity: f32,
    pub sensitivity_horizontal: f32,
    pub sensitivity_vertical: f32,
    #[serde(default)]
    pub invert_y: bool,
    pub offhand_button: bool,
    pub shield_button: bool,
    pub circle_size: f32,
    pub button_scale: f32,
    pub custom_buttons: Vec<super::CustomButton>,
    pub analog_movement: bool,
    pub sprint_button: bool,
    pub sprint_toggle: bool,
    /// The sprint control latches: a tap keeps sprinting until tapped again.
    #[serde(default)]
    pub toggle_sprint: bool,
    pub inventory_controls: bool,
    #[serde(default)]
    pub virtual_mouse_enabled: bool,
    #[serde(default)]
    pub disable_game_animations: bool,
    #[serde(default = "default_res_scale")]
    pub resolution_scale_percent: u32,
    pub virtual_mouse_sensitivity: f32,
    /// Rendered size of the virtual mouse pointer, where 1.0 is the default height.
    #[serde(default = "default_pointer_scale")]
    pub virtual_mouse_pointer_scale: f32,
    pub crosshair_gestures: bool,
    pub attack_button_look: bool,
    pub gyroscope: bool,
    pub gyro_sensitivity: f32,
    pub attack_indicator_left: bool,
    pub dynamic_joystick: bool,
    /// Fraction of the width, from the movement edge inward, where a dynamic
    /// joystick may start.
    pub dynamic_area_x: f32,
    /// Fraction of the height the dynamic area spans, centred on the movement
    /// control (1.0 is the full surface).
    pub dynamic_area_y: f32,
    #[serde(default)]
    pub auto_jump: bool,
    #[serde(default = "default_feedback_enabled")]
    pub vibrate: bool,
    #[serde(default)]
    pub ui_vibrate: bool,
    #[serde(default = "default_feedback_enabled")]
    pub show_control_hub: bool,
    #[serde(deserialize_with = "deserialize_visibility")]
    pub button_visibility: [u8; 14],
    pub horizontal_margin: f32,
    pub vertical_margin: f32,
    #[serde(default = "default_profile_name")]
    pub active_profile: String,
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default = "default_profile_name")]
    pub preset_id: String,
    #[serde(default = "default_preset_display_name")]
    pub preset_name: String,
    #[serde(default = "default_mcpe_preset_desc")]
    pub preset_description: String,
    #[serde(default = "default_profile_name")]
    pub active_mcpe_preset: String,
    #[serde(default = "default_profile_name")]
    pub active_goy_preset: String,
    #[serde(default)]
    pub control_style: ControlStyle,
    #[serde(default)]
    pub joystick_style: JoystickStyle,
    #[serde(default = "default_floating_pos")]
    pub floating_menu_pos: [f32; 2],
    #[serde(default)]
    pub snap_to_grid: bool,
    #[serde(deserialize_with = "deserialize_layouts")]
    pub layouts: [[Placement; 14]; 3],
}
fn default_profile_name() -> String {
    "default".into()
}
fn default_preset_display_name() -> String {
    "Default".into()
}
fn default_mcpe_preset_desc() -> String {
    "Vanilla touch controls".into()
}
fn default_goy_preset_desc() -> String {
    "Custom layout with more options".into()
}
fn default_feedback_enabled() -> bool {
    true
}
fn default_floating_pos() -> [f32; 2] {
    [0.44, 0.05]
}
fn default_pointer_scale() -> f32 {
    1.0
}
fn default_res_scale() -> u32 {
    100
}
// Append new buttons without resetting a player's existing ten-button layout.
fn deserialize_layouts<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[[Placement; 14]; 3], D::Error> {
    let rows = Vec::<Vec<Placement>>::deserialize(deserializer)?;
    if rows.len() != 3 || rows.iter().any(|r| !matches!(r.len(), 10 | 12 | 14)) {
        return Err(serde::de::Error::custom(
            "expected three touch layouts of 10, 12 or 14 controls",
        ));
    }
    let mut layouts = Settings::default().layouts;
    for (output, input) in layouts.iter_mut().zip(rows) {
        output[..input.len()].copy_from_slice(&input);
    }
    Ok(layouts)
}
fn deserialize_visibility<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u8; 14], D::Error> {
    let old = Vec::<u8>::deserialize(d)?;
    if !matches!(old.len(), 12 | 14) {
        return Err(serde::de::Error::custom(
            "expected 12 or 14 visibility values",
        ));
    }
    let mut values = [0; 14];
    values[..old.len()].copy_from_slice(&old);
    Ok(values)
}
impl Default for Settings {
    fn default() -> Self {
        let base = [
            Placement::new(0.13, 0.76, 0.32),
            Placement::new(0.72, 0.78, 0.105),
            Placement::new(0.92, 0.78, 0.16),
            Placement::new(0.81, 0.86, 0.115),
            Placement::new(0.80, 0.64, 0.16),
            Placement::new(0.92, 0.52, 0.16),
            Placement::new(0.46, 0.055, 0.075),
            Placement::new(0.50, 0.055, 0.075),
            Placement::new(0.54, 0.055, 0.075),
            Placement::new(0.73, 0.95, 0.075),
            Placement::new(0.42, 0.055, 0.075),
            Placement::new(0.58, 0.055, 0.075),
            Placement::new(0.80, 0.96, 0.075),
            Placement::new(0.92, 0.36, 0.12),
        ];
        let mut layouts = [base; 3];
        layouts[2][Control::Sneak as usize] = Placement::new(0.13, 0.76, 0.10);
        layouts[2][Control::Jump as usize] = Placement::new(0.90, 0.78, 0.14);
        for row in &mut layouts {
            for c in Control::ALL {
                row[c as usize].visible_in_game = true;
                row[c as usize].visible_in_menu = false;
            }
            row[Control::Attack as usize].passthrough = true;
        }
        Self {
            mode: Mode::Crosshair,
            split_controls: false,
            contextual_interact: false,
            left_handed: false,
            sprint_with_joystick: false,
            swap_jump_sneak: false,
            sensitivity: 1.0,
            sensitivity_horizontal: 1.0,
            sensitivity_vertical: 1.0,
            invert_y: false,
            offhand_button: false,
            shield_button: true,
            circle_size: 32.0,
            button_scale: 1.0,
            custom_buttons: Vec::new(),
            analog_movement: false,
            sprint_button: true,
            sprint_toggle: false,
            toggle_sprint: false,
            inventory_controls: false,
            virtual_mouse_enabled: false,
            disable_game_animations: false,
            resolution_scale_percent: default_res_scale(),
            virtual_mouse_sensitivity: 1.0,
            virtual_mouse_pointer_scale: default_pointer_scale(),
            crosshair_gestures: false,
            attack_button_look: true,
            gyroscope: false,
            gyro_sensitivity: 1.0,
            attack_indicator_left: true,
            dynamic_joystick: true,
            dynamic_area_x: 0.25,
            dynamic_area_y: 1.0,
            auto_jump: false,
            vibrate: true,
            ui_vibrate: false,
            show_control_hub: true,
            button_visibility: [0; 14],
            horizontal_margin: 0.0,
            vertical_margin: 0.0,
            active_profile: default_profile_name(),
            default_profile: default_profile_name(),
            preset_id: default_profile_name(),
            preset_name: default_preset_display_name(),
            preset_description: default_mcpe_preset_desc(),
            active_mcpe_preset: default_profile_name(),
            active_goy_preset: default_profile_name(),
            control_style: ControlStyle::Mcpe,
            joystick_style: JoystickStyle::default(),
            floating_menu_pos: default_floating_pos(),
            snap_to_grid: false,
            layouts,
        }
    }
}
impl Settings {
    /// Protected factory Default layout for MCPE Controls.
    pub fn default_mcpe() -> Self {
        Self::default()
    }

    /// Protected factory Default layout for GoyControl.
    pub fn default_goycontrol() -> Self {
        let mut s = Self {
            control_style: ControlStyle::GoyControl,
            mode: Mode::Crosshair,
            split_controls: true,
            preset_id: default_profile_name(),
            active_profile: default_profile_name(),
            preset_name: default_preset_display_name(),
            preset_description: default_goy_preset_desc(),
            joystick_style: JoystickStyle::default(),
            custom_buttons: Self::default_goy_buttons(),
            ..Self::default()
        };
        for c in Control::ALL {
            s.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
        }
        s.sanitize();
        s
    }

    /// Returns the protected Default configuration for the specified control system.
    pub fn default_for_style(style: ControlStyle) -> Self {
        match style {
            ControlStyle::Mcpe => Self::default_mcpe(),
            ControlStyle::GoyControl => Self::default_goycontrol(),
        }
    }

    /// Storage directory for a control system (`controls/mcpe` or `controls/goycontrol`).
    pub fn system_dir(game_home: &std::path::Path, style: ControlStyle) -> std::path::PathBuf {
        game_home.join(style.storage_subdir())
    }

    /// Returns true if this configuration represents the protected Default preset of its system.
    pub fn is_default_preset(&self) -> bool {
        Self::sanitize_profile_name(&self.preset_id) == "default"
    }

    /// Returns the active preset ID for the given control system.
    pub fn active_preset_for(&self, style: ControlStyle) -> &str {
        let id = match style {
            ControlStyle::Mcpe => self.active_mcpe_preset.as_str(),
            ControlStyle::GoyControl => self.active_goy_preset.as_str(),
        };
        if id.trim().is_empty() { "default" } else { id }
    }

    pub fn load() -> Self {
        Self::load_from(std::path::Path::new("horizon-touch.json"))
    }
    pub fn load_from(path: &std::path::Path) -> Self {
        let mut s: Self = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.sanitize();
        if s.control_style == ControlStyle::GoyControl && s.custom_buttons.is_empty() {
            s.custom_buttons = Self::default_goy_buttons();
            for c in Control::ALL {
                s.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
            }
        }
        // Migrate legacy action:key.sneak / action:key.sprint tokens without overwriting user custom keybinds.
        for button in &mut s.custom_buttons {
            if button.key == "action:key.sneak" {
                button.key = "key.keyboard.left.shift".into();
                if button.second_key == "action:key.sneak"
                    || button.second_key == "key.keyboard.left.shift"
                {
                    button.second_key.clear();
                }
            } else if button.key == "action:key.sprint" {
                button.key = "key.keyboard.left.control".into();
                if button.second_key == "action:key.sprint"
                    || button.second_key == "key.keyboard.left.control"
                {
                    button.second_key.clear();
                }
            }
        }
        // Restore the original top-row spacing without moving custom layouts.
        for row in &mut s.layouts {
            for (control, old, new) in [
                (Control::Chat, 0.45, 0.46),
                (Control::Settings, 0.55, 0.54),
                (Control::Debug, 0.40, 0.42),
                (Control::Perspective, 0.60, 0.58),
            ] {
                let p = &mut row[control as usize];
                if p.center == [old, 0.055] && p.size == 0.075 {
                    p.center[0] = new;
                }
            }
        }
        // Migrate factory Swap placements only; keep the player's own layout.
        for row in &mut s.layouts {
            let swap = &mut row[Control::Offhand as usize];
            if matches!(swap.center, [0.68, 0.80] | [0.66, 0.38])
                && (swap.size - 0.09).abs() < 0.0001
            {
                swap.center = [0.80, 0.96];
                swap.size = 0.075;
            }
        }
        // Move the old factory sprint position only; keep customized placements.
        for mode in [Mode::Crosshair, Mode::Tap] {
            let sprint = &mut s.layouts[mode.index()][Control::Sprint as usize];
            if sprint.center == [0.13, 0.50] && (sprint.size - 0.105).abs() < 0.0001 {
                sprint.center = [0.72, 0.78];
            }
        }
        s
    }
    pub fn sanitize(&mut self) {
        if self.mode == Mode::Tap && self.split_controls {
            self.mode = Mode::Crosshair;
            self.split_controls = false;
            self.crosshair_gestures = true;
        }
        if self.control_style == ControlStyle::Mcpe {
            if self.is_default_preset() {
                for row in &mut self.layouts {
                    row[Control::Attack as usize].passthrough = true;
                }
            }
            for row in &mut self.layouts {
                if row.iter().all(|p| p.visible_in_menu) {
                    for p in row.iter_mut() {
                        p.visible_in_menu = false;
                    }
                }
            }
        }
        if self.sprint_toggle != self.toggle_sprint {
            let latched = self.sprint_toggle || self.toggle_sprint;
            self.sprint_toggle = latched;
            self.toggle_sprint = latched;
        }
        self.virtual_mouse_sensitivity = if self.virtual_mouse_sensitivity.is_finite() {
            self.virtual_mouse_sensitivity.clamp(0.25, 3.0)
        } else {
            1.0
        };
        self.resolution_scale_percent = if self.resolution_scale_percent == 0 {
            100
        } else {
            self.resolution_scale_percent.clamp(25, 100)
        };
        self.virtual_mouse_pointer_scale = if self.virtual_mouse_pointer_scale.is_finite() {
            self.virtual_mouse_pointer_scale.clamp(0.5, 2.0)
        } else {
            default_pointer_scale()
        };
        self.gyro_sensitivity = if self.gyro_sensitivity.is_finite() {
            self.gyro_sensitivity.clamp(0.1, 3.0)
        } else {
            1.0
        };
        for value in [&mut self.horizontal_margin, &mut self.vertical_margin] {
            *value = if value.is_finite() {
                value.clamp(0.0, 0.12)
            } else {
                0.0
            };
        }
        if !self.sensitivity.is_finite() {
            self.sensitivity = 1.0;
        }
        self.sensitivity = self.sensitivity.clamp(0.1, 3.0);
        for value in [
            &mut self.sensitivity_horizontal,
            &mut self.sensitivity_vertical,
        ] {
            *value = if value.is_finite() {
                value.clamp(0.1, 3.0)
            } else {
                1.0
            };
        }
        self.button_scale = if self.button_scale.is_finite() {
            self.button_scale.clamp(0.5, 2.0)
        } else {
            1.0
        };
        self.dynamic_area_x = if self.dynamic_area_x.is_finite() {
            self.dynamic_area_x.clamp(0.0, 1.0)
        } else {
            0.25
        };
        self.dynamic_area_y = if self.dynamic_area_y.is_finite() {
            self.dynamic_area_y.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.circle_size = if self.circle_size.is_finite() {
            self.circle_size.clamp(16.0, 64.0)
        } else {
            32.0
        };
        self.custom_buttons.truncate(32);
        for button in &mut self.custom_buttons {
            if button.key.is_empty() {
                button.key = "key.keyboard.unknown".into();
            }
        }
        self.custom_buttons
            .retain(|b| super::CustomButton::valid_token(&b.key) || !b.active_tokens().is_empty());
        let is_mcpe = self.control_style == ControlStyle::Mcpe;
        for button in &mut self.custom_buttons {
            if is_mcpe {
                // Enforce strict single-binding exclusivity in MCPE mode (Keyboard OR Mouse)
                button.second_key.clear();
                button.third_key.clear();
            }
            for p in &mut button.layouts {
                p.sanitize();
            }
        }
        if self.active_profile.trim().is_empty() {
            self.active_profile = default_profile_name();
        }
        if self.default_profile.trim().is_empty() {
            self.default_profile = default_profile_name();
        }
        if self.preset_id.trim().is_empty() {
            self.preset_id = Self::sanitize_profile_name(&self.active_profile);
        } else {
            self.preset_id = Self::sanitize_profile_name(&self.preset_id);
        }
        if self.active_mcpe_preset.trim().is_empty() {
            self.active_mcpe_preset = default_profile_name();
        } else {
            self.active_mcpe_preset = Self::sanitize_profile_name(&self.active_mcpe_preset);
        }
        if self.active_goy_preset.trim().is_empty() {
            self.active_goy_preset = default_profile_name();
        } else {
            self.active_goy_preset = Self::sanitize_profile_name(&self.active_goy_preset);
        }
        if self.preset_id == "default" {
            self.preset_name = "Default".into();
            if self.preset_description.trim().is_empty()
                || (self.control_style == ControlStyle::GoyControl
                    && self.preset_description == default_mcpe_preset_desc())
                || (self.control_style == ControlStyle::Mcpe
                    && self.preset_description == default_goy_preset_desc())
            {
                self.preset_description = match self.control_style {
                    ControlStyle::Mcpe => default_mcpe_preset_desc(),
                    ControlStyle::GoyControl => default_goy_preset_desc(),
                };
            }
        } else if self.preset_name.trim().is_empty()
            || (self.preset_name == "Default" && self.preset_id != "default")
        {
            self.preset_name = self.preset_id.clone();
        }
        self.joystick_style.sanitize();
        if self.floating_menu_pos == [0.44, 0.05] {
            self.floating_menu_pos = default_floating_pos();
        }
        for (i, v) in self.floating_menu_pos.iter_mut().enumerate() {
            let def = default_floating_pos()[i];
            *v = if v.is_finite() {
                v.clamp(0.0, 0.94)
            } else {
                def
            };
        }
        for p in self.layouts.iter_mut().flatten() {
            p.sanitize();
        }
    }
    pub fn save(&self) -> std::io::Result<()> {
        let home = std::path::Path::new(".");
        if !self.is_default_preset() {
            let _ = self.save_profile(home, &self.preset_id);
        } else {
            Self::ensure_system_storage(home, self.control_style);
        }
        self.save_to(std::path::Path::new("horizon-touch.json"))
    }
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path)
    }
    pub fn sanitize_profile_name(name: &str) -> String {
        let cleaned: String = name
            .trim()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let trimmed = cleaned.trim_matches('-');
        if trimmed.is_empty() {
            "default".into()
        } else {
            trimmed.chars().take(40).collect()
        }
    }

    /// Ensures the independent storage directory for `style` (`controls/mcpe` or
    /// `controls/goycontrol`) exists, guarantees its protected `default.json` is intact,
    /// and migrates any legacy user profiles from `controlmap/` that match `style`.
    pub fn ensure_system_storage(game_home: &std::path::Path, style: ControlStyle) {
        let sys_dir = Self::system_dir(game_home, style);
        let _ = std::fs::create_dir_all(&sys_dir);
        let default_path = sys_dir.join("default.json");
        let needs_default = !default_path.is_file()
            || std::fs::read(&default_path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Self>(&b).ok())
                .is_none_or(|s| s.control_style != style || s.preset_id != "default");
        if needs_default {
            let def = Self::default_for_style(style);
            let _ = def.save_to(&default_path);
        }
        // Migrate legacy user profiles from `controlmap/` into the matching system directory once.
        let legacy_dir = game_home.join("controlmap");
        if legacy_dir.is_dir() {
            for entry in std::fs::read_dir(&legacy_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    continue;
                }
                let fname = entry.file_name().to_string_lossy().into_owned();
                let Some(stem) = fname.strip_suffix(".json") else {
                    continue;
                };
                let clean = Self::sanitize_profile_name(stem);
                if clean == "default" {
                    continue;
                }
                let target = sys_dir.join(format!("{clean}.json"));
                if target.exists() {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(entry.path())
                    && let Ok(mut parsed) = serde_json::from_slice::<Self>(&bytes)
                {
                    if parsed.control_style == style {
                        parsed.preset_id = clean.clone();
                        parsed.active_profile = clean.clone();
                        if parsed.preset_name.trim().is_empty() || parsed.preset_name == "Default" {
                            parsed.preset_name = stem.to_owned();
                        }
                        if parsed.preset_description.trim().is_empty() {
                            parsed.preset_description = format!("Saved {} preset", style.label());
                        }
                        parsed.sanitize();
                        let _ = parsed.save_to(&target);
                    }
                }
            }
        }
    }

    /// Lists all presets belonging to `style` (`MCPE` or `GoyControl` independently).
    /// Always includes the protected `Default` preset first, followed by user-created presets.
    pub fn list_presets(game_home: &std::path::Path, style: ControlStyle) -> Vec<PresetMeta> {
        Self::ensure_system_storage(game_home, style);
        let default_desc = match style {
            ControlStyle::Mcpe => default_mcpe_preset_desc(),
            ControlStyle::GoyControl => default_goy_preset_desc(),
        };
        let mut out = vec![PresetMeta {
            id: "default".into(),
            name: "Default".into(),
            description: default_desc,
            system: style,
            is_default: true,
        }];
        let dir = Self::system_dir(game_home, style);
        let mut user_presets = Vec::new();
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = file_name.strip_suffix(".json") else {
                continue;
            };
            let clean = Self::sanitize_profile_name(stem);
            if clean == "default" {
                continue;
            }
            if let Ok(bytes) = std::fs::read(entry.path())
                && let Ok(parsed) = serde_json::from_slice::<Self>(&bytes)
            {
                let name =
                    if parsed.preset_name.trim().is_empty() || parsed.preset_name == "Default" {
                        clean.clone()
                    } else {
                        parsed.preset_name.trim().to_owned()
                    };
                let description = if parsed.preset_description.trim().is_empty() {
                    format!("Custom {} layout", style.label())
                } else {
                    parsed.preset_description.trim().to_owned()
                };
                user_presets.push(PresetMeta {
                    id: clean,
                    name,
                    description,
                    system: style,
                    is_default: false,
                });
            }
        }
        user_presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        user_presets.dedup_by(|a, b| a.id == b.id);
        out.extend(user_presets);
        out
    }

    /// Loads a preset for a specific control system (`MCPE` or `GoyControl`).
    /// Loading `"default"` always returns the protected factory Default for that system.
    pub fn load_preset(game_home: &std::path::Path, style: ControlStyle, id: &str) -> Self {
        Self::ensure_system_storage(game_home, style);
        let clean = Self::sanitize_profile_name(id);
        if clean == "default" {
            let def = Self::default_for_style(style);
            let default_path = Self::system_dir(game_home, style).join("default.json");
            let _ = def.save_to(&default_path);
            return def;
        }
        let candidate = Self::system_dir(game_home, style).join(format!("{clean}.json"));
        if candidate.is_file() {
            let mut loaded = Self::load_from(&candidate);
            loaded.control_style = style;
            loaded.preset_id = clean.clone();
            loaded.active_profile = clean;
            if loaded.preset_name.trim().is_empty() || loaded.preset_name == "Default" {
                loaded.preset_name = loaded.preset_id.clone();
            }
            loaded.sanitize();
            loaded
        } else {
            Self::default_for_style(style)
        }
    }

    /// Allocates a unique user preset ID inside `controls/<style>/` that is never `"default"`.
    pub fn allocate_unique_preset_id(
        game_home: &std::path::Path,
        style: ControlStyle,
        desired_name: &str,
    ) -> String {
        Self::ensure_system_storage(game_home, style);
        let base_raw = Self::sanitize_profile_name(desired_name);
        let base = if base_raw == "default" {
            "custom-preset".to_owned()
        } else {
            base_raw
        };
        let dir = Self::system_dir(game_home, style);
        if !dir.join(format!("{base}.json")).exists() {
            return base;
        }
        for index in 2..=999 {
            let candidate = format!("{base}-{index}");
            if !dir.join(format!("{candidate}.json")).exists() {
                return candidate;
            }
        }
        format!("{base}-copy")
    }

    /// Duplicates any preset in `style` (including `"default"`) into a brand-new independent preset
    /// with the specified `new_name` and `new_description`. Never modifies the original preset.
    pub fn copy_preset(
        game_home: &std::path::Path,
        style: ControlStyle,
        source_id: &str,
        new_name: &str,
        new_description: &str,
    ) -> std::io::Result<PresetMeta> {
        let trimmed_name = new_name.trim();
        let display_name =
            if trimmed_name.is_empty() || trimmed_name.eq_ignore_ascii_case("default") {
                "Custom Copy".to_owned()
            } else {
                trimmed_name.chars().take(48).collect()
            };
        let trimmed_desc = new_description.trim();
        let display_desc = if trimmed_desc.is_empty() {
            format!("Copied from {source_id}")
        } else {
            trimmed_desc.chars().take(120).collect()
        };
        let mut duplicated = Self::load_preset(game_home, style, source_id);
        let new_id = Self::allocate_unique_preset_id(game_home, style, &display_name);
        duplicated.control_style = style;
        duplicated.preset_id = new_id.clone();
        duplicated.active_profile = new_id.clone();
        duplicated.preset_name = display_name.clone();
        duplicated.preset_description = display_desc.clone();
        match style {
            ControlStyle::Mcpe => duplicated.active_mcpe_preset = new_id.clone(),
            ControlStyle::GoyControl => duplicated.active_goy_preset = new_id.clone(),
        }
        duplicated.sanitize();
        let target = Self::system_dir(game_home, style).join(format!("{new_id}.json"));
        duplicated.save_to(&target)?;
        Ok(PresetMeta {
            id: new_id,
            name: display_name,
            description: display_desc,
            system: style,
            is_default: false,
        })
    }

    /// Updates the name and description of a user-created preset.
    /// Rejects modifying the protected `"default"` preset.
    pub fn update_preset_meta(
        &mut self,
        game_home: &std::path::Path,
        style: ControlStyle,
        id: &str,
        new_name: &str,
        new_description: &str,
    ) -> std::io::Result<PresetMeta> {
        let clean = Self::sanitize_profile_name(id);
        if clean == "default" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The Default preset is protected and cannot be modified",
            ));
        }
        let target = Self::system_dir(game_home, style).join(format!("{clean}.json"));
        if !target.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Preset not found",
            ));
        }
        let mut preset = Self::load_from(&target);
        let name_str: String = new_name.trim().chars().take(48).collect();
        let desc_str: String = new_description.trim().chars().take(120).collect();
        preset.control_style = style;
        preset.preset_id = clean.clone();
        preset.active_profile = clean.clone();
        if !name_str.is_empty() && !name_str.eq_ignore_ascii_case("default") {
            preset.preset_name = name_str;
        }
        preset.preset_description = desc_str;
        preset.sanitize();
        preset.save_to(&target)?;
        if self.control_style == style && self.preset_id == clean {
            self.preset_name = preset.preset_name.clone();
            self.preset_description = preset.preset_description.clone();
            let _ = self.save_to(&game_home.join("horizon-touch.json"));
        }
        Ok(PresetMeta {
            id: clean,
            name: preset.preset_name,
            description: preset.preset_description,
            system: style,
            is_default: false,
        })
    }

    /// Selects and activates a preset within `style`, preserving both systems' remembered selections.
    pub fn select_preset(&mut self, game_home: &std::path::Path, style: ControlStyle, id: &str) {
        let prev_mcpe = self.active_mcpe_preset.clone();
        let prev_goy = self.active_goy_preset.clone();
        let prev_floating = self.floating_menu_pos;
        let prev_invert_y = self.invert_y;
        let prev_vm_enabled = self.virtual_mouse_enabled;
        let prev_vm_sens = self.virtual_mouse_sensitivity;
        let prev_pointer_scale = self.virtual_mouse_pointer_scale;
        let prev_vibrate = self.vibrate;
        let prev_ui_vibrate = self.ui_vibrate;
        let prev_hub = self.show_control_hub;
        let prev_auto_jump = self.auto_jump;
        let mut loaded = Self::load_preset(game_home, style, id);
        let clean = loaded.preset_id.clone();
        loaded.active_mcpe_preset = if style == ControlStyle::Mcpe {
            clean.clone()
        } else {
            prev_mcpe
        };
        loaded.active_goy_preset = if style == ControlStyle::GoyControl {
            clean.clone()
        } else {
            prev_goy
        };
        loaded.floating_menu_pos = prev_floating;
        loaded.invert_y = prev_invert_y;
        loaded.virtual_mouse_enabled = prev_vm_enabled;
        loaded.virtual_mouse_sensitivity = prev_vm_sens;
        loaded.virtual_mouse_pointer_scale = prev_pointer_scale;
        loaded.vibrate = prev_vibrate;
        loaded.ui_vibrate = prev_ui_vibrate;
        loaded.show_control_hub = prev_hub;
        loaded.auto_jump = prev_auto_jump;
        *self = loaded;
        let _ = self.save_to(&game_home.join("horizon-touch.json"));
    }

    /// Switches between MCPE and GoyControl.
    /// Immediately activates the selected system's saved configuration and remembered preset
    /// without transferring, merging, resetting, or overwriting the other system's layouts.
    pub fn switch_control_system(&mut self, game_home: &std::path::Path, target: ControlStyle) {
        Self::ensure_system_storage(game_home, ControlStyle::Mcpe);
        Self::ensure_system_storage(game_home, ControlStyle::GoyControl);
        // Remember current system's active preset ID
        let current_id = Self::sanitize_profile_name(&self.preset_id);
        match self.control_style {
            ControlStyle::Mcpe => self.active_mcpe_preset = current_id,
            ControlStyle::GoyControl => self.active_goy_preset = current_id,
        }
        // Persist any unsaved state of a user-created preset in its own system folder before switching
        if !self.is_default_preset() {
            let current_path = Self::system_dir(game_home, self.control_style)
                .join(format!("{}.json", self.preset_id));
            let _ = self.save_to(&current_path);
        }
        let target_preset_id = match target {
            ControlStyle::Mcpe => self.active_mcpe_preset.clone(),
            ControlStyle::GoyControl => self.active_goy_preset.clone(),
        };
        let prev_mcpe = self.active_mcpe_preset.clone();
        let prev_goy = self.active_goy_preset.clone();
        let prev_floating = self.floating_menu_pos;
        let prev_invert_y = self.invert_y;
        let prev_vm_enabled = self.virtual_mouse_enabled;
        let prev_vm_sens = self.virtual_mouse_sensitivity;
        let prev_pointer_scale = self.virtual_mouse_pointer_scale;
        let prev_vibrate = self.vibrate;
        let prev_ui_vibrate = self.ui_vibrate;
        let prev_hub = self.show_control_hub;
        let prev_auto_jump = self.auto_jump;
        let mut loaded = Self::load_preset(game_home, target, &target_preset_id);
        loaded.control_style = target;
        loaded.active_mcpe_preset = prev_mcpe;
        loaded.active_goy_preset = prev_goy;
        loaded.floating_menu_pos = prev_floating;
        loaded.invert_y = prev_invert_y;
        loaded.virtual_mouse_enabled = prev_vm_enabled;
        loaded.virtual_mouse_sensitivity = prev_vm_sens;
        loaded.virtual_mouse_pointer_scale = prev_pointer_scale;
        loaded.vibrate = prev_vibrate;
        loaded.ui_vibrate = prev_ui_vibrate;
        loaded.show_control_hub = prev_hub;
        loaded.auto_jump = prev_auto_jump;
        *self = loaded;
        let _ = self.save_to(&game_home.join("horizon-touch.json"));
    }

    /// Control Editor Save Option A: Replace the currently selected user-created preset.
    /// Protected: Returns PermissionDenied if the currently selected preset is `"default"`.
    pub fn save_replace_preset(&mut self, game_home: &std::path::Path) -> std::io::Result<()> {
        if self.is_default_preset() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The Default preset cannot be replaced. Create a new preset instead.",
            ));
        }
        Self::ensure_system_storage(game_home, self.control_style);
        let clean = Self::sanitize_profile_name(&self.preset_id);
        self.preset_id = clean.clone();
        self.active_profile = clean.clone();
        match self.control_style {
            ControlStyle::Mcpe => self.active_mcpe_preset = clean.clone(),
            ControlStyle::GoyControl => self.active_goy_preset = clean.clone(),
        }
        self.sanitize();
        let target = Self::system_dir(game_home, self.control_style).join(format!("{clean}.json"));
        self.save_to(&target)?;
        self.save_to(&game_home.join("horizon-touch.json"))
    }

    /// Control Editor Save Option B: Save the current editor layout as a new independent preset
    /// in the active control system's storage directory and make it active.
    pub fn save_new_preset(
        &mut self,
        game_home: &std::path::Path,
        name: &str,
        description: &str,
    ) -> std::io::Result<PresetMeta> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Preset name cannot be empty",
            ));
        }
        if trimmed_name.eq_ignore_ascii_case("default") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot name a preset Default",
            ));
        }
        let style = self.control_style;
        let new_id = Self::allocate_unique_preset_id(game_home, style, trimmed_name);
        let display_name: String = trimmed_name.chars().take(48).collect();
        let display_desc: String = if description.trim().is_empty() {
            format!("Custom {} preset", style.label())
        } else {
            description.trim().chars().take(120).collect()
        };
        self.preset_id = new_id.clone();
        self.active_profile = new_id.clone();
        self.preset_name = display_name.clone();
        self.preset_description = display_desc.clone();
        match style {
            ControlStyle::Mcpe => self.active_mcpe_preset = new_id.clone(),
            ControlStyle::GoyControl => self.active_goy_preset = new_id.clone(),
        }
        self.sanitize();
        let target = Self::system_dir(game_home, style).join(format!("{new_id}.json"));
        self.save_to(&target)?;
        self.save_to(&game_home.join("horizon-touch.json"))?;
        Ok(PresetMeta {
            id: new_id,
            name: display_name,
            description: display_desc,
            system: style,
            is_default: false,
        })
    }

    pub fn list_profiles(game_home: &std::path::Path) -> Vec<String> {
        let active = Self::load_from(&game_home.join("horizon-touch.json"));
        Self::list_presets(game_home, active.control_style)
            .into_iter()
            .map(|p| p.id)
            .collect()
    }
    pub fn save_profile(
        &self,
        game_home: &std::path::Path,
        name: &str,
    ) -> std::io::Result<std::path::PathBuf> {
        let style = self.control_style;
        Self::ensure_system_storage(game_home, style);
        let clean = Self::sanitize_profile_name(name);
        let dir = Self::system_dir(game_home, style);
        std::fs::create_dir_all(&dir)?;
        if clean == "default" {
            let def = Self::default_for_style(style);
            let target = dir.join("default.json");
            let _ = def.save_to(&target);
            let _ = self.save_to(&game_home.join("horizon-touch.json"));
            return Ok(target);
        }
        let mut copy = self.clone();
        copy.active_profile = clean.clone();
        copy.preset_id = clean.clone();
        if copy.preset_name.trim().is_empty() || copy.preset_name == "Default" {
            copy.preset_name = name.trim().to_owned();
        }
        match style {
            ControlStyle::Mcpe => copy.active_mcpe_preset = clean.clone(),
            ControlStyle::GoyControl => copy.active_goy_preset = clean.clone(),
        }
        let target = dir.join(format!("{clean}.json"));
        copy.save_to(&target)?;
        let active_file = game_home.join("horizon-touch.json");
        let _ = copy.save_to(&active_file);
        Ok(target)
    }
    pub const PRESETS: [(&'static str, &'static str, &'static str); 1] = [(
        "default",
        "Default",
        "Protected default layout for the active control system",
    )];

    /// Builds the super-optimized default GoyControl button layout (static buttons, no icons,
    /// custom stroke/shadow/roundness, comfortable touch sizes, dedicated Mouse button, and
    /// smart `visible_in_game` / `visible_in_menu` defaults).
    pub fn default_goy_buttons() -> Vec<super::CustomButton> {
        #[allow(clippy::too_many_arguments)]
        let make = |label: &str,
                    k1: &str,
                    k2: &str,
                    x: f32,
                    y: f32,
                    sz: f32,
                    wr: f32,
                    hr: f32,
                    pass: bool,
                    tog: bool,
                    shape: u8,
                    in_game: bool,
                    in_menu: bool| {
            let mut p = Placement::new(x, y, sz);
            p.width_ratio = wr;
            p.height_ratio = hr;
            p.passthrough = pass;
            p.toggleable = tog;
            p.use_goy_style = true;
            p.shape = shape;
            p.opacity = 0.82;
            p.stroke_size = 1.8;
            p.stroke_margin = 0.0;
            p.roundness = 12.0;
            p.stroke_roundness = 12.0;
            p.shadow_size = 0.0;
            p.visible_in_game = in_game;
            p.visible_in_menu = in_menu;
            super::CustomButton {
                key: k1.into(),
                second_key: k2.into(),
                third_key: String::new(),
                custom_label: label.into(),
                always_visible: in_menu,
                layouts: [p; 3],
            }
        };
        vec![
            // Left combat & menu interaction triggers above joystick (visible in game AND menu)
            make(
                "PRI",
                "key.mouse.left",
                "",
                0.11,
                0.39,
                0.155,
                1.18,
                0.96,
                true,
                false,
                0,
                true,
                true,
            ),
            make(
                "SEC",
                "key.mouse.right",
                "",
                0.26,
                0.39,
                0.155,
                1.18,
                0.96,
                true,
                false,
                0,
                true,
                true,
            ),
            // Right movement & action cluster (visible in game only by default)
            make(
                "JUMP",
                "key.keyboard.space",
                "",
                0.89,
                0.73,
                0.165,
                1.12,
                1.0,
                true,
                false,
                2,
                true,
                false,
            ),
            make(
                "SNEAK",
                "key.keyboard.left.shift",
                "",
                0.74,
                0.84,
                0.138,
                1.12,
                0.96,
                false,
                true,
                0,
                true,
                false,
            ),
            make(
                "SPRINT",
                "key.keyboard.left.control",
                "",
                0.89,
                0.52,
                0.138,
                1.12,
                0.96,
                false,
                true,
                0,
                true,
                false,
            ),
            make(
                "SWAP",
                "key.keyboard.f",
                "",
                0.74,
                0.63,
                0.122,
                1.08,
                0.94,
                false,
                false,
                0,
                true,
                false,
            ),
            make(
                "INV",
                "key.keyboard.e",
                "",
                0.60,
                0.85,
                0.122,
                1.18,
                0.94,
                false,
                false,
                0,
                true,
                true,
            ),
            make(
                "DROP",
                "key.keyboard.q",
                "",
                0.89,
                0.34,
                0.115,
                1.08,
                0.92,
                false,
                false,
                0,
                true,
                false,
            ),
            // Top utility bar (with prominent MOUSE toggle & ESC visible in both game & menus)
            make(
                "ESC",
                "key.keyboard.escape",
                "",
                0.07,
                0.08,
                0.095,
                1.30,
                0.90,
                false,
                false,
                0,
                true,
                true,
            ),
            make(
                "DEBUG",
                "key.keyboard.f3",
                "",
                0.19,
                0.08,
                0.095,
                1.34,
                0.90,
                false,
                false,
                0,
                true,
                false,
            ),
            make(
                "3RD",
                "key.keyboard.f5",
                "",
                0.31,
                0.08,
                0.095,
                1.28,
                0.90,
                false,
                false,
                0,
                true,
                false,
            ),
            make(
                "CHAT",
                "key.keyboard.t",
                "",
                0.63,
                0.08,
                0.095,
                1.30,
                0.90,
                false,
                false,
                0,
                true,
                false,
            ),
            make(
                "TAB",
                "key.keyboard.tab",
                "",
                0.75,
                0.08,
                0.095,
                1.28,
                0.90,
                false,
                false,
                0,
                true,
                false,
            ),
            make(
                "MOUSE",
                "mouse.virtual.toggle",
                "",
                0.89,
                0.08,
                0.098,
                1.46,
                0.92,
                false,
                false,
                0,
                true,
                true,
            ),
        ]
    }

    /// Applies the smart preset visibility (`Visible in Game` vs `Visible in Menu`) across all
    /// built-in controls and custom GoyControl buttons.
    pub fn apply_visibility_preset_all(&mut self) {
        let is_mcpe = self.control_style == ControlStyle::Mcpe;
        for row in &mut self.layouts {
            for c in Control::ALL {
                let in_menu = if is_mcpe {
                    false
                } else {
                    !matches!(
                        c,
                        Control::Move
                            | Control::Jump
                            | Control::Sneak
                            | Control::Sprint
                            | Control::Offhand
                            | Control::Shield
                    )
                };
                row[c as usize].visible_in_game = true;
                row[c as usize].visible_in_menu = in_menu;
            }
        }
        for btn in &mut self.custom_buttons {
            let label = btn.custom_label.trim().to_ascii_uppercase();
            let is_movement_only = matches!(
                label.as_str(),
                "JUMP" | "SNEAK" | "SPRINT" | "SWAP" | "DROP" | "DEBUG" | "3RD" | "CHAT" | "TAB"
            ) || matches!(
                btn.key.as_str(),
                "key.keyboard.space"
                    | "action:key.jump"
                    | "key.keyboard.left.shift"
                    | "action:key.sneak"
                    | "key.keyboard.left.control"
                    | "action:key.sprint"
                    | "key.keyboard.q"
                    | "action:key.drop"
                    | "key.keyboard.f"
                    | "action:key.swapOffhand"
            );
            for p in &mut btn.layouts {
                p.visible_in_game = true;
                p.visible_in_menu = !is_movement_only;
            }
            btn.always_visible = !is_movement_only;
        }
    }

    pub fn set_control_style(&mut self, style: ControlStyle) {
        self.control_style = style;
        match style {
            ControlStyle::Mcpe => {
                self.button_visibility = [0; 14];
                self.custom_buttons.retain(|b| !b.layouts[0].use_goy_style);
            }
            ControlStyle::GoyControl => {
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                if self.custom_buttons.is_empty() {
                    self.custom_buttons = Self::default_goy_buttons();
                }
                for c in Control::ALL {
                    self.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
                }
            }
        }
        self.sanitize();
    }

    pub fn apply_preset(&mut self, name: &str) {
        let clean = Self::sanitize_profile_name(name);
        if clean == "default" {
            let style = self.control_style;
            let prev_mcpe = self.active_mcpe_preset.clone();
            let prev_goy = self.active_goy_preset.clone();
            let prev_floating = self.floating_menu_pos;
            let prev_invert_y = self.invert_y;
            let prev_vm_enabled = self.virtual_mouse_enabled;
            let prev_vm_sens = self.virtual_mouse_sensitivity;
            let prev_pointer_scale = self.virtual_mouse_pointer_scale;
            let prev_vibrate = self.vibrate;
            let prev_ui_vibrate = self.ui_vibrate;
            let prev_hub = self.show_control_hub;
            let prev_auto_jump = self.auto_jump;
            *self = Self::default_for_style(style);
            self.active_mcpe_preset = prev_mcpe;
            self.active_goy_preset = prev_goy;
            self.floating_menu_pos = prev_floating;
            self.invert_y = prev_invert_y;
            self.virtual_mouse_enabled = prev_vm_enabled;
            self.virtual_mouse_sensitivity = prev_vm_sens;
            self.virtual_mouse_pointer_scale = prev_pointer_scale;
            self.vibrate = prev_vibrate;
            self.ui_vibrate = prev_ui_vibrate;
            self.show_control_hub = prev_hub;
            self.auto_jump = prev_auto_jump;
            match style {
                ControlStyle::Mcpe => self.active_mcpe_preset = "default".into(),
                ControlStyle::GoyControl => self.active_goy_preset = "default".into(),
            }
            return;
        }
        self.active_profile = clean.clone();
        self.preset_id = clean;
        self.sanitize();
    }

    pub fn set_as_default_always(&mut self, game_home: &std::path::Path) {
        self.default_profile = self.preset_id.clone();
        match self.control_style {
            ControlStyle::Mcpe => self.active_mcpe_preset = self.preset_id.clone(),
            ControlStyle::GoyControl => self.active_goy_preset = self.preset_id.clone(),
        }
        if !self.is_default_preset() {
            let _ = self.save_profile(game_home, &self.preset_id);
        }
        let _ = self.save_to(&game_home.join("horizon-touch.json"));
    }

    pub fn load_profile(game_home: &std::path::Path, name: &str) -> Self {
        let active = Self::load_from(&game_home.join("horizon-touch.json"));
        let mut loaded = Self::load_preset(game_home, active.control_style, name);
        loaded.active_mcpe_preset = active.active_mcpe_preset;
        loaded.active_goy_preset = active.active_goy_preset;
        loaded.floating_menu_pos = active.floating_menu_pos;
        loaded.invert_y = active.invert_y;
        loaded.virtual_mouse_enabled = active.virtual_mouse_enabled;
        loaded.virtual_mouse_sensitivity = active.virtual_mouse_sensitivity;
        loaded.virtual_mouse_pointer_scale = active.virtual_mouse_pointer_scale;
        loaded.vibrate = active.vibrate;
        loaded.ui_vibrate = active.ui_vibrate;
        loaded.show_control_hub = active.show_control_hub;
        loaded.auto_jump = active.auto_jump;
        match loaded.control_style {
            ControlStyle::Mcpe => loaded.active_mcpe_preset = loaded.preset_id.clone(),
            ControlStyle::GoyControl => loaded.active_goy_preset = loaded.preset_id.clone(),
        }
        let _ = loaded.save_to(&game_home.join("horizon-touch.json"));
        loaded
    }
    pub fn export_profile(
        &self,
        root_home: &std::path::Path,
        game_home: &std::path::Path,
        name: &str,
    ) -> std::io::Result<std::path::PathBuf> {
        let clean = Self::sanitize_profile_name(name);
        if clean != "default" {
            let _ = self.save_profile(game_home, &clean);
        }
        let reports = root_home.join("performance");
        std::fs::create_dir_all(&reports)?;
        let exported = reports.join(format!("horizon-controls-{clean}.json"));
        self.save_to(&exported)?;
        Ok(exported)
    }
    pub fn crosshair(&self) -> bool {
        self.mode == Mode::Crosshair || self.split_controls
    }
    /// Toggles split-touch (crosshair aiming vs tap-anywhere interaction) across all movement modes.
    pub fn toggle_split_touch(&mut self) {
        if self.crosshair() {
            self.split_controls = false;
            if self.mode == Mode::Crosshair {
                self.mode = Mode::Tap;
            }
        } else if self.mode == Mode::Tap {
            self.mode = Mode::Crosshair;
            self.split_controls = false;
        } else {
            self.split_controls = true;
        }
    }
    pub fn visible(&self, c: Control) -> bool {
        if self.control_style == ControlStyle::GoyControl && c != Control::Move {
            return false;
        }
        match self.button_visibility[c as usize] {
            1 => return true,
            2 => return false,
            _ => {}
        }
        match c {
            Control::Offhand => self.offhand_button,
            Control::Shield => self.shield_button && !self.visible(Control::Use),
            Control::Attack | Control::Use => self.crosshair(),
            Control::Sprint => {
                self.sprint_button && self.mode != Mode::Dpad && !self.sprint_with_joystick
            }
            _ => true,
        }
    }
    pub fn slot(&self, c: Control) -> usize {
        match c {
            Control::Jump if self.mode == Mode::Dpad && self.swap_jump_sneak => {
                Control::Sneak as usize
            }
            Control::Sneak if self.mode == Mode::Dpad && self.swap_jump_sneak => {
                Control::Jump as usize
            }
            _ => c as usize,
        }
    }
    pub fn placement(&self, c: Control) -> Placement {
        let mut p = self.layouts[self.mode.index()][self.slot(c)];
        if self.left_handed {
            p.center[0] = 1.0 - p.center[0];
        }
        p
    }
}
pub fn contains([x, y, w, h]: [f32; 4], p: [f32; 2]) -> bool {
    (x..=x + w).contains(&p[0]) && (y..=y + h).contains(&p[1])
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_feedback_and_hub_preferences_migrate_and_roundtrip() {
        let mut legacy = serde_json::to_value(Settings::default()).unwrap();
        let fields = legacy.as_object_mut().unwrap();
        for name in ["ui_vibrate", "show_control_hub", "classic_control_hub"] {
            fields.remove(name);
        }
        fields.insert("floating_menu_pos".into(), serde_json::json!([0.44, 0.05]));
        fields.insert("vibrate".into(), serde_json::json!(false));
        let mut settings: Settings = serde_json::from_value(legacy).unwrap();
        settings.sanitize();
        assert!(!settings.ui_vibrate);
        assert!(!settings.vibrate);
        assert!(settings.show_control_hub);
        assert_eq!(settings.floating_menu_pos, [0.44, 0.05]);
        settings.ui_vibrate = true;
        settings.show_control_hub = false;
        let restored: Settings =
            serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
        assert!(restored.ui_vibrate);
        assert!(!restored.vibrate);
        assert!(!restored.show_control_hub);
    }

    #[test]
    fn gyro_preferences_roundtrip_and_reject_bad_values() {
        let mut s = super::Settings::default();
        assert!(!s.gyroscope);
        s.gyroscope = true;
        s.gyro_sensitivity = 2.0;
        let restored: super::Settings =
            serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert!(restored.gyroscope);
        assert_eq!(restored.gyro_sensitivity, 2.0);
        s.gyro_sensitivity = f32::NAN;
        s.sanitize();
        assert_eq!(s.gyro_sensitivity, 1.0);
    }
    #[test]
    fn blank_buttons_and_game_actions_survive_settings_validation() {
        let mut settings = super::Settings::default();
        settings
            .custom_buttons
            .push(super::super::CustomButton::new(
                "key.keyboard.unknown".into(),
                0,
            ));
        settings
            .custom_buttons
            .push(super::super::CustomButton::new("action:key.jump".into(), 1));
        settings.sanitize();
        assert_eq!(settings.custom_buttons.len(), 2);
        let encoded = serde_json::to_string(&settings.custom_buttons[1]).unwrap();
        assert_eq!(
            serde_json::from_str::<super::super::CustomButton>(&encoded)
                .unwrap()
                .key,
            "action:key.jump"
        );
        assert!(!super::super::CustomButton::valid_token(
            "action:no-such-action"
        ));
        assert!(
            super::super::custom::choices(false)
                .iter()
                .any(|(token, _)| token == "action:key.attack")
        );
        assert!(
            super::super::custom::choices(true)
                .iter()
                .any(|(token, _)| token == "key.keyboard.z")
        );
    }

    use super::*;

    #[test]
    fn added_buttons_override_scheme_and_removed_buttons_stay_hidden() {
        let mut settings = Settings {
            mode: Mode::Tap,
            ..Settings::default()
        };
        assert!(!settings.visible(Control::Attack));
        settings.button_visibility[Control::Attack as usize] = 1;
        assert!(settings.visible(Control::Attack));
        settings.button_visibility[Control::Chat as usize] = 2;
        assert!(!settings.visible(Control::Chat));
        let restored: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert!(restored.visible(Control::Attack));
        assert!(!restored.visible(Control::Chat));
    }

    #[test]
    fn old_layout_preserves_custom_buttons_when_shortcuts_are_added() {
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json["circle_size"] = 48.into();
        for row in json["layouts"].as_array_mut().unwrap() {
            row.as_array_mut().unwrap().truncate(10);
            row[6]["center"] = serde_json::json!([0.25, 0.2]);
            row[6]["opacity"] = serde_json::json!(0.35);
        }
        let settings: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(settings.circle_size, 48.0);
        for row in settings.layouts {
            assert_eq!(row[6].center, [0.25, 0.2]);
            assert_eq!(row[6].opacity, 0.35);
            assert_eq!(row[Control::Debug as usize].center, [0.42, 0.055]);
            assert_eq!(row[Control::Perspective as usize].center, [0.58, 0.055]);
        }
        let roundtrip: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(roundtrip.layouts[0][6].center, [0.25, 0.2]);
    }
    #[test]
    fn swap_factory_migration_keeps_custom_positions() {
        let file =
            std::env::temp_dir().join(format!("horizon-swap-layout-{}.json", std::process::id()));
        let mut settings = Settings::default();
        settings.layouts[0][Control::Offhand as usize] = super::Placement::new(0.68, 0.80, 0.09);
        settings.layouts[1][Control::Offhand as usize].center = [0.55, 0.43];
        settings.layouts[2][Control::Offhand as usize] = super::Placement::new(0.66, 0.38, 0.09);
        settings.save_to(&file).unwrap();
        let read = Settings::load_from(&file);
        assert_eq!(
            read.layouts[0][Control::Offhand as usize].center,
            [0.80, 0.96]
        );
        assert_eq!(
            read.layouts[1][Control::Offhand as usize].center,
            [0.55, 0.43]
        );
        assert_eq!(
            read.layouts[2][Control::Offhand as usize].center,
            [0.80, 0.96]
        );
        std::fs::remove_file(file).unwrap();
    }
    #[test]
    fn goy_control_style_and_three_events_roundtrip_and_resize() {
        let mut settings = Settings::default();
        settings.set_control_style(ControlStyle::GoyControl);
        assert_eq!(settings.control_style, ControlStyle::GoyControl);
        assert!(!settings.custom_buttons.is_empty());
        let mut btn = super::super::CustomButton::new("key.mouse.left".into(), 0);
        btn.set_event_slot(1, "key.keyboard.left.control".into());
        btn.set_event_slot(2, "key.keyboard.w".into());
        btn.custom_label = "MACRO".into();
        btn.layouts[0].use_goy_style = true;
        btn.layouts[0].width_ratio = 1.5;
        btn.layouts[0].height_ratio = 0.8;
        btn.layouts[0].stroke_size = 2.5;
        btn.layouts[0].stroke_margin = 1.5;
        btn.layouts[0].stroke_roundness = 12.0;
        btn.layouts[0].shadow_size = 4.0;
        btn.layouts[0].shape = 3;
        assert_eq!(btn.active_tokens().len(), 3);
        settings.custom_buttons.push(btn);
        settings.joystick_style.move_area = JoystickMoveArea::MidBottomLeft;
        let json = serde_json::to_vec(&settings).unwrap();
        let restored: Settings = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.control_style, ControlStyle::GoyControl);
        assert_eq!(
            restored.joystick_style.move_area,
            JoystickMoveArea::MidBottomLeft
        );
        let last = restored.custom_buttons.last().unwrap();
        assert_eq!(last.active_tokens().len(), 3);
        assert_eq!(last.layouts[0].width_ratio, 1.5);
        assert_eq!(last.layouts[0].shape, 3);
    }

    #[test]
    fn visible_in_game_and_menu_presets_and_named_profiles_roundtrip() {
        let mut settings = Settings::default();
        settings.set_control_style(ControlStyle::GoyControl);
        // Move joystick is visible in game only
        assert!(settings.layouts[0][Control::Move as usize].visible_in_game);
        assert!(!settings.layouts[0][Control::Move as usize].visible_in_menu);
        // PRI, SEC, MOUSE, INV, ESC are visible in both game and menu; JUMP, SNEAK, SPRINT are game-only
        let find_btn = |lbl: &str| {
            settings
                .custom_buttons
                .iter()
                .find(|b| b.custom_label == lbl)
                .unwrap()
        };
        for menu_btn in ["PRI", "SEC", "MOUSE", "INV", "ESC"] {
            let b = find_btn(menu_btn);
            assert!(b.layouts[0].visible_in_game, "{menu_btn} in game");
            assert!(b.layouts[0].visible_in_menu, "{menu_btn} in menu");
        }
        for game_btn in ["JUMP", "SNEAK", "SPRINT", "SWAP", "DROP"] {
            let b = find_btn(game_btn);
            assert!(b.layouts[0].visible_in_game, "{game_btn} in game");
            assert!(!b.layouts[0].visible_in_menu, "{game_btn} not in menu");
        }
        // Named profile save & load
        let tmp = std::env::temp_dir().join(format!("horizon-named-prof-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        settings.save_profile(&tmp, "My Custom PvP").unwrap();
        let listed = Settings::list_profiles(&tmp);
        assert!(listed.contains(&"my-custom-pvp".to_owned()));
        let loaded = Settings::load_profile(&tmp, "my-custom-pvp");
        assert_eq!(loaded.active_profile, "my-custom-pvp");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn independent_mcpe_and_goycontrol_storage_and_protected_defaults() {
        let tmp = std::env::temp_dir().join(format!("horizon-sep-systems-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // Fresh default is MCPE Default
        let mut settings = Settings::default();
        assert_eq!(settings.control_style, ControlStyle::Mcpe);
        assert!(settings.is_default_preset());

        // Default preset cannot be replaced
        assert!(settings.save_replace_preset(&tmp).is_err());

        // Copy Default MCPE preset into a new MCPE preset
        let mcpe_copy = Settings::copy_preset(
            &tmp,
            ControlStyle::Mcpe,
            "default",
            "My MCPE Layout",
            "Customized Bedrock layout",
        )
        .unwrap();
        assert_eq!(mcpe_copy.id, "my-mcpe-layout");
        assert_eq!(mcpe_copy.system, ControlStyle::Mcpe);
        assert!(!mcpe_copy.is_default);
        assert!(
            tmp.join("controls/mcpe/my-mcpe-layout.json").is_file(),
            "MCPE preset stored in controls/mcpe/"
        );
        assert!(
            !tmp.join("controls/goycontrol/my-mcpe-layout.json")
                .is_file(),
            "MCPE preset never leaks into controls/goycontrol/"
        );

        // Select MCPE custom preset and modify Jump position, then Replace existing preset
        settings.select_preset(&tmp, ControlStyle::Mcpe, &mcpe_copy.id);
        assert!(!settings.is_default_preset());
        settings.layouts[0][Control::Jump as usize].center = [0.85, 0.70];
        settings.save_replace_preset(&tmp).unwrap();

        // Switch to GoyControl: activates GoyControl Default, remembers MCPE preset
        settings.switch_control_system(&tmp, ControlStyle::GoyControl);
        assert_eq!(settings.control_style, ControlStyle::GoyControl);
        assert_eq!(settings.preset_id, "default");
        assert_eq!(settings.active_mcpe_preset, "my-mcpe-layout");

        // Save a new GoyControl preset via Option B (Create new preset as...)
        settings.custom_buttons[0].layouts[0].center = [0.20, 0.25];
        let goy_new = settings
            .save_new_preset(&tmp, "Claw Pro", "4-finger GoyControl claw")
            .unwrap();
        assert_eq!(goy_new.id, "claw-pro");
        assert!(tmp.join("controls/goycontrol/claw-pro.json").is_file());
        assert!(!tmp.join("controls/mcpe/claw-pro.json").is_file());

        // Switch back to MCPE: immediately restores "my-mcpe-layout" and its Jump position,
        // while remembering GoyControl's "claw-pro"
        settings.switch_control_system(&tmp, ControlStyle::Mcpe);
        assert_eq!(settings.control_style, ControlStyle::Mcpe);
        assert_eq!(settings.preset_id, "my-mcpe-layout");
        assert_eq!(settings.active_goy_preset, "claw-pro");
        assert_eq!(
            settings.layouts[0][Control::Jump as usize].center,
            [0.85, 0.70]
        );

        // Verify MCPE Default was never modified
        let mcpe_def = Settings::load_preset(&tmp, ControlStyle::Mcpe, "default");
        assert_eq!(
            mcpe_def.layouts[0][Control::Jump as usize].center,
            [0.92, 0.78]
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn virtual_mouse_and_floating_menu_settings_persist_across_system_switches() {
        let tmp = std::env::temp_dir().join(format!("horizon-vm-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let mut settings = Settings::default();
        settings.virtual_mouse_enabled = false;
        settings.virtual_mouse_sensitivity = 1.75;
        settings.virtual_mouse_pointer_scale = 1.6;
        settings.vibrate = false;
        settings.auto_jump = true;
        settings.toggle_sprint = true;
        settings.sanitize();
        assert!(settings.sprint_toggle);

        settings.switch_control_system(&tmp, ControlStyle::GoyControl);
        assert!(!settings.virtual_mouse_enabled);
        assert_eq!(settings.virtual_mouse_sensitivity, 1.75);
        assert_eq!(settings.virtual_mouse_pointer_scale, 1.6);
        assert!(!settings.vibrate);
        assert!(settings.auto_jump);

        settings.switch_control_system(&tmp, ControlStyle::Mcpe);
        assert!(!settings.virtual_mouse_enabled);
        assert_eq!(settings.virtual_mouse_sensitivity, 1.75);
        assert_eq!(settings.virtual_mouse_pointer_scale, 1.6);
        assert!(!settings.vibrate);
        assert!(settings.auto_jump);

        // A layout file written before the pointer-size setting keeps the default size.
        let mut old_json = serde_json::to_value(Settings::default()).unwrap();
        old_json
            .as_object_mut()
            .unwrap()
            .remove("virtual_mouse_pointer_scale");
        let old: Settings = serde_json::from_value(old_json).unwrap();
        assert_eq!(old.virtual_mouse_pointer_scale, 1.0);

        // Sizes outside the slider's range clamp to its ends.
        settings.virtual_mouse_pointer_scale = 9.0;
        settings.sanitize();
        assert_eq!(settings.virtual_mouse_pointer_scale, 2.0);
        settings.virtual_mouse_pointer_scale = 0.1;
        settings.sanitize();
        assert_eq!(settings.virtual_mouse_pointer_scale, 0.5);

        // Split touch toggles cleanly between Crosshair and Tap modes
        assert!(settings.crosshair());
        settings.toggle_split_touch();
        assert!(!settings.crosshair());
        assert_eq!(settings.mode, Mode::Tap);
        settings.toggle_split_touch();
        assert!(settings.crosshair());
        assert_eq!(settings.mode, Mode::Crosshair);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
