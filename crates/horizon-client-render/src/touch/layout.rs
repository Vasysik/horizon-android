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
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub center: [f32; 2],
    pub size: f32,
    pub opacity: f32,
    /// Pojav-style Pass-through: dragging on this button also turns the camera.
    #[serde(default)]
    pub passthrough: bool,
    /// Pojav-style Toggle mode: tapping latches the button on/off until tapped again.
    #[serde(default)]
    pub toggleable: bool,
    /// Pojav-style Swipeable: sliding a finger from outside onto this button activates it.
    #[serde(default)]
    pub swipeable: bool,
    /// Button border shape: 0 = Rounded (default), 1 = Square, 2 = Circle/Pill.
    #[serde(default)]
    pub shape: u8,
}
impl Placement {
    pub(super) fn new(x: f32, y: f32, size: f32) -> Self {
        Self {
            center: [x, y],
            size,
            opacity: 0.65,
            passthrough: false,
            toggleable: false,
            swipeable: false,
            shape: 0,
        }
    }
    pub fn rect(self, aspect: f32) -> [f32; 4] {
        let sx = self.size / aspect.max(0.1);
        [
            self.center[0] - sx / 2.0,
            self.center[1] - self.size / 2.0,
            sx,
            self.size,
        ]
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
        self.shape = self.shape.min(2);
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
    #[serde(default)]
    pub snap_to_grid: bool,
    #[serde(deserialize_with = "deserialize_layouts")]
    pub layouts: [[Placement; 14]; 3],
}
fn default_profile_name() -> String {
    "default".into()
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
    pub fn save_profile(&self, game_home: &std::path::Path, name: &str) -> std::io::Result<std::path::PathBuf> {
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
            "Pocket Edition (Default)",
            "Classic Bedrock D-Pad & Jump/Sneak/Attack/Use HUD",
        ),
        (
            "pojav",
            "PojavLauncher Classic",
            "Left PRI/SEC + Right Jump/Sneak/Swap with Pass-Through look",
        ),
        (
            "pvp",
            "PvP 4-Finger Claw",
            "Top-corner Attack/Use index triggers + bottom thumb controls",
        ),
        (
            "building",
            "Builder Split Crosshair",
            "Analog Joystick + Center Crosshair + dedicated Place/Break/Swap",
        ),
        (
            "speedrun",
            "Speedrun Pro",
            "High-visibility HUD with F3, F5, Swap, Inventory & fast triggers",
        ),
    ];

    pub fn apply_preset(&mut self, name: &str) {
        let clean = Self::sanitize_profile_name(name);
        let base_default = Self::default();
        self.layouts = base_default.layouts;
        self.button_visibility = [0; 14];
        match clean.as_str() {
            "default" | "pocket" => {
                // Pocket Edition style: D-Pad movement on bottom-left, Jump/Sneak/Sprint/Attack/Use on right
                self.mode = Mode::Dpad;
                self.split_controls = false;
                self.offhand_button = true;
                self.sprint_button = true;
                let idx = Mode::Dpad.index();
                self.layouts[idx][Control::Move as usize] = Placement::new(0.14, 0.73, 0.33);
                self.layouts[idx][Control::Jump as usize] = Placement::new(0.89, 0.74, 0.15);
                self.layouts[idx][Control::Sneak as usize] = Placement::new(0.89, 0.54, 0.13);
                self.layouts[idx][Control::Sprint as usize] = Placement::new(0.75, 0.82, 0.12);
                self.layouts[idx][Control::Attack as usize] = Placement::new(0.75, 0.62, 0.145);
                self.layouts[idx][Control::Use as usize] = Placement::new(0.89, 0.36, 0.145);
                self.layouts[idx][Control::Inventory as usize] = Placement::new(0.68, 0.94, 0.08);
                self.layouts[idx][Control::Offhand as usize] = Placement::new(0.78, 0.94, 0.08);
                // Show Attack, Use, Sprint, Offhand in Pocket Edition preset so all buttons are ready
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Sprint as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
            }
            "pojav" => {
                // PojavLauncher style: PRI/SEC on left above movement, Jump/Sneak/Swap on right with Pass-Through
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.offhand_button = true;
                self.shield_button = true;
                self.sprint_toggle = true;
                for m in 0..3 {
                    self.layouts[m][Control::Move as usize] = Placement::new(0.15, 0.75, 0.32);
                    let mut atk = Placement::new(0.11, 0.38, 0.14);
                    atk.passthrough = true;
                    self.layouts[m][Control::Attack as usize] = atk;
                    let mut use_btn = Placement::new(0.24, 0.38, 0.14);
                    use_btn.passthrough = true;
                    self.layouts[m][Control::Use as usize] = use_btn;
                    let mut jmp = Placement::new(0.89, 0.76, 0.155);
                    jmp.passthrough = true;
                    self.layouts[m][Control::Jump as usize] = jmp;
                    let mut snk = Placement::new(0.76, 0.84, 0.125);
                    snk.toggleable = true;
                    self.layouts[m][Control::Sneak as usize] = snk;
                    let mut spr = Placement::new(0.89, 0.56, 0.12);
                    spr.toggleable = true;
                    self.layouts[m][Control::Sprint as usize] = spr;
                    self.layouts[m][Control::Offhand as usize] = Placement::new(0.76, 0.64, 0.11);
                    self.layouts[m][Control::Inventory as usize] = Placement::new(0.64, 0.93, 0.085);
                }
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
                self.button_visibility[Control::Debug as usize] = 1;
                self.button_visibility[Control::Perspective as usize] = 1;
            }
            "pvp" => {
                // 4-Finger Claw PvP layout: Top-left Attack, Top-right Use (Pass-Through), bottom thumb movement/jump
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.offhand_button = true;
                self.shield_button = true;
                self.sprint_toggle = true;
                for m in 0..3 {
                    self.layouts[m][Control::Move as usize] = Placement::new(0.15, 0.75, 0.33);
                    let mut atk = Placement::new(0.13, 0.22, 0.165);
                    atk.passthrough = true;
                    self.layouts[m][Control::Attack as usize] = atk;
                    let mut use_btn = Placement::new(0.87, 0.22, 0.165);
                    use_btn.passthrough = true;
                    self.layouts[m][Control::Use as usize] = use_btn;
                    let mut jmp = Placement::new(0.89, 0.76, 0.16);
                    jmp.passthrough = true;
                    self.layouts[m][Control::Jump as usize] = jmp;
                    let mut spr = Placement::new(0.75, 0.82, 0.12);
                    spr.toggleable = true;
                    self.layouts[m][Control::Sprint as usize] = spr;
                    self.layouts[m][Control::Sneak as usize] = Placement::new(0.75, 0.62, 0.125);
                    self.layouts[m][Control::Offhand as usize] = Placement::new(0.89, 0.54, 0.115);
                    self.layouts[m][Control::Shield as usize] = Placement::new(0.63, 0.74, 0.115);
                }
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
                self.button_visibility[Control::Shield as usize] = 1;
            }
            "building" | "split" => {
                // Builder Split Crosshair: Joystick + Center Crosshair + side-by-side Break & Place
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.offhand_button = true;
                for m in 0..3 {
                    self.layouts[m][Control::Move as usize] = Placement::new(0.14, 0.75, 0.32);
                    let mut atk = Placement::new(0.77, 0.58, 0.15);
                    atk.passthrough = true;
                    self.layouts[m][Control::Attack as usize] = atk;
                    let mut use_btn = Placement::new(0.90, 0.58, 0.15);
                    use_btn.passthrough = true;
                    self.layouts[m][Control::Use as usize] = use_btn;
                    self.layouts[m][Control::Jump as usize] = Placement::new(0.90, 0.79, 0.15);
                    let mut snk = Placement::new(0.77, 0.79, 0.13);
                    snk.toggleable = true;
                    self.layouts[m][Control::Sneak as usize] = snk;
                    self.layouts[m][Control::Sprint as usize] = Placement::new(0.65, 0.79, 0.115);
                    self.layouts[m][Control::Offhand as usize] = Placement::new(0.65, 0.58, 0.11);
                }
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
            }
            "speedrun" => {
                self.mode = Mode::Crosshair;
                self.split_controls = true;
                self.offhand_button = true;
                self.sprint_toggle = true;
                for m in 0..3 {
                    self.layouts[m][Control::Debug as usize] = Placement::new(0.38, 0.065, 0.085);
                    self.layouts[m][Control::Perspective as usize] = Placement::new(0.62, 0.065, 0.085);
                    self.layouts[m][Control::Offhand as usize] = Placement::new(0.77, 0.48, 0.12);
                    self.layouts[m][Control::Inventory as usize] = Placement::new(0.89, 0.38, 0.11);
                }
                self.button_visibility[Control::Attack as usize] = 1;
                self.button_visibility[Control::Use as usize] = 1;
                self.button_visibility[Control::Offhand as usize] = 1;
                self.button_visibility[Control::Debug as usize] = 1;
                self.button_visibility[Control::Perspective as usize] = 1;
            }
            _ => {}
        }
        self.active_profile = clean;
        self.sanitize();
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
}
