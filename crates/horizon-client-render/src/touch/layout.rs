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
    /// Default MCPE style (Bedrock D-Pad/Joystick & icon sprites).
    #[default]
    Mcpe,
    /// GoyControl style (customizable static buttons, custom joystick, up to 3 events, no icons).
    GoyControl,
}
impl ControlStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mcpe => "MCPE Style Touch Controls",
            Self::GoyControl => "GoyControl (Custom Static)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JoystickMoveArea {
    /// Move in mid-bottom-left region from anywhere inside that zone.
    MidBottomLeft,
    /// Move anywhere on the right side (ignores buttons).
    RightSideAnywhere,
    /// Joystick stays in its fixed absolute placement.
    #[default]
    AbsoluteFixed,
    /// Floating joystick center follows the finger where touched.
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
            move_area: JoystickMoveArea::AbsoluteFixed,
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
    3.0
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
            width_ratio: 1.0,
            height_ratio: 1.0,
            use_goy_style: false,
            stroke_size: 1.6,
            stroke_margin: 0.0,
            stroke_roundness: 10.0,
            stroke_color: 0xFF8A7AE6,
            button_color: 0xFF1E1B2E,
            shadow_size: 3.0,
            shadow_color: 0x96000000,
            roundness: 10.0,
            on_click_color: 0xFF423675,
            on_click_stroke_color: 0xFFD0BCFF,
            on_toggle_color: 0xFF543B94,
            on_toggle_stroke_color: 0xFFEADDFF,
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
            3.0
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub offhand_button: bool,
    pub shield_button: bool,
    pub circle_size: f32,
    pub button_scale: f32,
    pub custom_buttons: Vec<super::CustomButton>,
    pub analog_movement: bool,
    pub sprint_button: bool,
    pub sprint_toggle: bool,
    pub inventory_controls: bool,
    pub virtual_mouse_sensitivity: f32,
    pub crosshair_gestures: bool,
    pub gyroscope: bool,
    pub gyro_sensitivity: f32,
    pub attack_indicator_left: bool,
    #[serde(deserialize_with = "deserialize_visibility")]
    pub button_visibility: [u8; 14],
    pub horizontal_margin: f32,
    pub vertical_margin: f32,
    #[serde(default = "default_profile_name")]
    pub active_profile: String,
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
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
fn default_floating_pos() -> [f32; 2] {
    [0.44, 0.05]
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
            offhand_button: false,
            shield_button: false,
            circle_size: 32.0,
            button_scale: 1.0,
            custom_buttons: Vec::new(),
            analog_movement: false,
            sprint_button: true,
            sprint_toggle: false,
            inventory_controls: false,
            virtual_mouse_sensitivity: 1.0,
            crosshair_gestures: false,
            gyroscope: false,
            gyro_sensitivity: 1.0,
            attack_indicator_left: true,
            button_visibility: [0; 14],
            horizontal_margin: 0.0,
            vertical_margin: 0.0,
            active_profile: default_profile_name(),
            default_profile: default_profile_name(),
            control_style: ControlStyle::Mcpe,
            joystick_style: JoystickStyle::default(),
            floating_menu_pos: default_floating_pos(),
            snap_to_grid: false,
            layouts,
        }
    }
}
impl Settings {
    pub fn load() -> Self {
        Self::load_from(std::path::Path::new("horizon-touch.json"))
    }
    pub fn load_from(path: &std::path::Path) -> Self {
        let mut s: Self = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.sanitize();
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
        self.virtual_mouse_sensitivity = if self.virtual_mouse_sensitivity.is_finite() {
            self.virtual_mouse_sensitivity.clamp(0.25, 3.0)
        } else {
            1.0
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
        self.circle_size = if self.circle_size.is_finite() {
            self.circle_size.clamp(16.0, 64.0)
        } else {
            32.0
        };
        self.custom_buttons.truncate(32);
        self.custom_buttons
            .retain(|b| super::CustomButton::valid_token(&b.key));
        for button in &mut self.custom_buttons {
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
        self.joystick_style.sanitize();
        for (i, v) in self.floating_menu_pos.iter_mut().enumerate() {
            let def = default_floating_pos()[i];
            *v = if v.is_finite() {
                v.clamp(0.04, 0.94)
            } else {
                def
            };
        }
        for p in self.layouts.iter_mut().flatten() {
            p.sanitize();
        }
    }
    pub fn save(&self) -> std::io::Result<()> {
        let _ = self.save_profile(std::path::Path::new("."), &self.active_profile);
        self.save_to(std::path::Path::new("horizon-touch.json"))
    }
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
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
    pub fn list_profiles(game_home: &std::path::Path) -> Vec<String> {
        let mut names = vec!["default".to_owned()];
        let dir = game_home.join("controlmap");
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stem) = file_name.strip_suffix(".json") {
                let clean = Self::sanitize_profile_name(stem);
                if !names.contains(&clean) {
                    names.push(clean);
                }
            }
        }
        names
    }
    pub fn save_profile(
        &self,
        game_home: &std::path::Path,
        name: &str,
    ) -> std::io::Result<std::path::PathBuf> {
        let clean = Self::sanitize_profile_name(name);
        let dir = game_home.join("controlmap");
        std::fs::create_dir_all(&dir)?;
        let mut copy = self.clone();
        copy.active_profile = clean.clone();
        let target = dir.join(format!("{clean}.json"));
        copy.save_to(&target)?;
        let active_file = game_home.join("horizon-touch.json");
        let _ = copy.save_to(&active_file);
        Ok(target)
    }
    pub const PRESETS: [(&'static str, &'static str, &'static str); 5] = [
        (
            "default",
            "MCPE Style (Default)",
            "Authentic Bedrock D-Pad/Joystick & classic HUD buttons",
        ),
        (
            "goycontrol",
            "GoyControl (Custom Static)",
            "Super-optimized static buttons, analog stick & multi-event macros",
        ),
        (
            "pvp",
            "GoyControl PvP Claw",
            "4-finger claw layout with top-corner Attack/Use & pass-through",
        ),
        (
            "building",
            "Builder Split Crosshair",
            "Analog Joystick + Center Crosshair + dedicated Break/Place/Swap",
        ),
        (
            "speedrun",
            "Speedrun Pro",
            "Competitive HUD with F3, F5, Swap, Inventory & fast triggers",
        ),
    ];

    /// Builds the super-optimized default GoyControl button layout (static buttons, no icons,
    /// custom stroke/shadow/roundness, and direct mouse/keyboard events).
    pub fn default_goy_buttons() -> Vec<super::CustomButton> {
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
                    shape: u8| {
            let mut p = Placement::new(x, y, sz);
            p.width_ratio = wr;
            p.height_ratio = hr;
            p.passthrough = pass;
            p.toggleable = tog;
            p.use_goy_style = true;
            p.shape = shape;
            p.opacity = 0.78;
            p.stroke_size = 1.8;
            p.stroke_margin = 0.0;
            p.roundness = 10.0;
            p.stroke_roundness = 10.0;
            p.shadow_size = 3.0;
            super::CustomButton {
                key: k1.into(),
                second_key: k2.into(),
                third_key: String::new(),
                custom_label: label.into(),
                always_visible: false,
                layouts: [p; 3],
            }
        };
        vec![
            // Left combat & interaction triggers above joystick (Pass-Through camera enabled)
            make(
                "PRI",
                "key.mouse.left",
                "",
                0.11,
                0.40,
                0.13,
                1.15,
                0.95,
                true,
                false,
                0,
            ),
            make(
                "SEC",
                "key.mouse.right",
                "",
                0.24,
                0.40,
                0.13,
                1.15,
                0.95,
                true,
                false,
                0,
            ),
            // Right movement & action cluster
            make(
                "JUMP",
                "key.keyboard.space",
                "",
                0.89,
                0.74,
                0.145,
                1.10,
                1.0,
                true,
                false,
                2,
            ),
            make(
                "SNEAK",
                "key.keyboard.left.shift",
                "",
                0.76,
                0.84,
                0.115,
                1.10,
                0.95,
                false,
                true,
                0,
            ),
            make(
                "SPRINT",
                "key.keyboard.left.control",
                "key.keyboard.w",
                0.89,
                0.54,
                0.115,
                1.10,
                0.95,
                false,
                true,
                0,
            ),
            make(
                "SWAP",
                "key.keyboard.f",
                "",
                0.76,
                0.64,
                0.105,
                1.05,
                0.92,
                false,
                false,
                0,
            ),
            make(
                "INV",
                "key.keyboard.e",
                "",
                0.64,
                0.88,
                0.095,
                1.15,
                0.90,
                false,
                false,
                0,
            ),
            make(
                "DROP",
                "key.keyboard.q",
                "",
                0.89,
                0.36,
                0.095,
                1.05,
                0.90,
                false,
                false,
                0,
            ),
            // Top utility strip
            make(
                "ESC",
                "key.keyboard.escape",
                "",
                0.06,
                0.07,
                0.075,
                1.25,
                0.88,
                false,
                false,
                0,
            ),
            make(
                "DEBUG",
                "key.keyboard.f3",
                "",
                0.20,
                0.07,
                0.075,
                1.30,
                0.88,
                false,
                false,
                0,
            ),
            make(
                "3RD",
                "key.keyboard.f5",
                "",
                0.30,
                0.07,
                0.075,
                1.25,
                0.88,
                false,
                false,
                0,
            ),
            make(
                "CHAT",
                "key.keyboard.t",
                "",
                0.68,
                0.07,
                0.075,
                1.30,
                0.88,
                false,
                false,
                0,
            ),
            make(
                "TAB",
                "key.keyboard.tab",
                "",
                0.78,
                0.07,
                0.075,
                1.25,
                0.88,
                false,
                false,
                0,
            ),
            make(
                "MOUSE",
                "mouse.virtual.toggle",
                "",
                0.90,
                0.07,
                0.075,
                1.40,
                0.88,
                false,
                false,
                0,
            ),
        ]
    }

    pub fn set_control_style(&mut self, style: ControlStyle) {
        self.control_style = style;
        if style == ControlStyle::GoyControl {
            self.mode = Mode::Crosshair;
            self.split_controls = true;
            if self.custom_buttons.is_empty() {
                self.custom_buttons = Self::default_goy_buttons();
            }
            // In GoyControl mode, only Move (the customizable GoyControl Joystick) is shown from built-ins by default;
            // all actions come from the customizable GoyControl static buttons.
            for c in Control::ALL {
                self.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
            }
        } else {
            self.button_visibility = [0; 14];
        }
        self.sanitize();
    }

    pub fn apply_preset(&mut self, name: &str) {
        let clean = Self::sanitize_profile_name(name);
        let base_default = Self::default();
        self.layouts = base_default.layouts;
        self.button_visibility = [0; 14];
        match clean.as_str() {
            "default" | "mcpe" | "pocket" => {
                self.control_style = ControlStyle::Mcpe;
                self.mode = Mode::Dpad;
                self.split_controls = false;
                self.offhand_button = true;
                self.sprint_button = true;
            }
            "goycontrol" | "goy" => {
                self.control_style = ControlStyle::GoyControl;
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.joystick_style = JoystickStyle::default();
                self.custom_buttons = Self::default_goy_buttons();
                for c in Control::ALL {
                    self.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
                }
            }
            "pvp" => {
                self.control_style = ControlStyle::GoyControl;
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.custom_buttons = Self::default_goy_buttons();
                // Move PRI and SEC to top index-finger claw positions
                if self.custom_buttons.len() >= 2 {
                    for m in 0..3 {
                        self.custom_buttons[0].layouts[m].center = [0.13, 0.22];
                        self.custom_buttons[0].layouts[m].size = 0.155;
                        self.custom_buttons[1].layouts[m].center = [0.87, 0.22];
                        self.custom_buttons[1].layouts[m].size = 0.155;
                    }
                }
                for c in Control::ALL {
                    self.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
                }
            }
            "building" | "split" => {
                self.control_style = ControlStyle::Mcpe;
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.offhand_button = true;
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
            }
            "speedrun" => {
                self.control_style = ControlStyle::GoyControl;
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.custom_buttons = Self::default_goy_buttons();
                for c in Control::ALL {
                    self.button_visibility[c as usize] = if c == Control::Move { 1 } else { 2 };
                }
            }
            _ => {}
        }
        self.active_profile = clean;
        self.sanitize();
    }

    pub fn set_as_default_always(&mut self, game_home: &std::path::Path) {
        self.default_profile = self.active_profile.clone();
        let _ = self.save_profile(game_home, &self.active_profile);
        let _ = self.save_to(&game_home.join("horizon-touch.json"));
    }

    pub fn load_profile(game_home: &std::path::Path, name: &str) -> Self {
        let clean = Self::sanitize_profile_name(name);
        let candidate = game_home.join("controlmap").join(format!("{clean}.json"));
        let mut loaded = if candidate.is_file() {
            Self::load_from(&candidate)
        } else {
            let mut s = Self::load_from(&game_home.join("horizon-touch.json"));
            s.apply_preset(&clean);
            let _ = s.save_profile(game_home, &clean);
            s
        };
        loaded.active_profile = clean;
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
        let _ = self.save_profile(game_home, &clean);
        let reports = root_home.join("performance");
        std::fs::create_dir_all(&reports)?;
        let exported = reports.join(format!("horizon-controls-{clean}.json"));
        self.save_to(&exported)?;
        Ok(exported)
    }
    pub fn crosshair(&self) -> bool {
        self.mode == Mode::Crosshair || self.split_controls
    }
    pub fn visible(&self, c: Control) -> bool {
        match self.button_visibility[c as usize] {
            1 => return true,
            2 => return false,
            _ => {}
        }
        match c {
            Control::Offhand => self.offhand_button,
            Control::Shield => self.shield_button,
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
}
