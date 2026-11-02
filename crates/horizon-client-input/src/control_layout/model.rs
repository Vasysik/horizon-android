//! Clean-room data model for customizable touch control layouts.
//!
//! Provides data structures and serialization compatible with the layout specification
//! documented in `docs/control-layout-spec.md`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_EDITOR_VERSION: u32 = 12;

static UUID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique 16-character identifier for controls and layers.
pub fn generate_uuid() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u64;
    let seq = UUID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}{:08x}", (now & 0xFFFFFFFF) as u32, (seq ^ (now >> 32)) as u32)
}

/// Root control layout schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLayout {
    #[serde(rename = "editorVersion")]
    pub editor_version: u32,
    pub info: LayoutInfo,
    #[serde(default)]
    pub layers: Vec<ControlLayer>,
    #[serde(default)]
    pub styles: Vec<ButtonStyle>,
    #[serde(default, rename = "joystickStyles")]
    pub joystick_styles: Vec<JoystickStyle>,
}

impl Default for ControlLayout {
    fn default() -> Self {
        Self {
            editor_version: CURRENT_EDITOR_VERSION,
            info: LayoutInfo::default(),
            layers: Vec::new(),
            styles: Vec::new(),
            joystick_styles: Vec::new(),
        }
    }
}

impl ControlLayout {
    /// Clones the layout, regenerating all layer, button, and style UUIDs to prevent collisions.
    pub fn duplicate(&self, new_name: &str) -> Self {
        let mut uuid_map = BTreeMap::new();

        let mut new_styles = Vec::with_capacity(self.styles.len());
        for style in &self.styles {
            let new_id = generate_uuid();
            uuid_map.insert(style.uuid.clone(), new_id.clone());
            let mut s = style.clone();
            s.uuid = new_id;
            new_styles.push(s);
        }

        let mut new_joysticks_styles = Vec::with_capacity(self.joystick_styles.len());
        for style in &self.joystick_styles {
            let new_id = generate_uuid();
            uuid_map.insert(style.uuid.clone(), new_id.clone());
            let mut s = style.clone();
            s.uuid = new_id;
            new_joysticks_styles.push(s);
        }

        let mut new_layers = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let new_layer_id = generate_uuid();
            uuid_map.insert(layer.uuid.clone(), new_layer_id.clone());
            let mut l = layer.clone();
            l.uuid = new_layer_id;
            for btn in &mut l.normal_buttons {
                btn.uuid = generate_uuid();
                if let Some(ref style_id) = btn.button_style {
                    if let Some(remapped) = uuid_map.get(style_id) {
                        btn.button_style = Some(remapped.clone());
                    }
                }
                for ev in &mut btn.click_events {
                    if ev.is_layer_event() {
                        if let Some(remapped) = uuid_map.get(&ev.key) {
                            ev.key = remapped.clone();
                        }
                    }
                }
            }
            for txt in &mut l.text_boxes {
                txt.uuid = generate_uuid();
                if let Some(ref style_id) = txt.button_style {
                    if let Some(remapped) = uuid_map.get(style_id) {
                        txt.button_style = Some(remapped.clone());
                    }
                }
            }
            for joy in &mut l.joystick_buttons {
                joy.uuid = generate_uuid();
                if let Some(ref style_id) = joy.joystick_style_id {
                    if let Some(remapped) = uuid_map.get(style_id) {
                        joy.joystick_style_id = Some(remapped.clone());
                    }
                }
            }
            new_layers.push(l);
        }

        let mut info = self.info.clone();
        info.name = TranslatableText::single(new_name);

        Self {
            editor_version: CURRENT_EDITOR_VERSION,
            info,
            layers: new_layers,
            styles: new_styles,
            joystick_styles: new_joysticks_styles,
        }
    }
}

/// Metadata header describing the layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutInfo {
    pub name: TranslatableText,
    pub author: TranslatableText,
    pub description: TranslatableText,
    #[serde(rename = "versionCode", default)]
    pub version_code: i32,
    #[serde(rename = "versionName", default = "default_version_name")]
    pub version_name: String,
}

fn default_version_name() -> String {
    "1.0".to_string()
}

impl Default for LayoutInfo {
    fn default() -> Self {
        Self {
            name: TranslatableText::single("Custom"),
            author: TranslatableText::single("Player"),
            description: TranslatableText::single(""),
            version_code: 1,
            version_name: "1.0".to_string(),
        }
    }
}

/// Translatable string with default text and optional locale matches.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TranslatableText {
    pub default: String,
    #[serde(default, rename = "matchQueue")]
    pub match_queue: Vec<LocaleMatch>,
}

impl TranslatableText {
    pub fn single(text: impl Into<String>) -> Self {
        Self {
            default: text.into(),
            match_queue: Vec::new(),
        }
    }

    /// Resolves the text according to the user's active language tag.
    pub fn resolve(&self, lang_tag: &str) -> &str {
        for m in &self.match_queue {
            if m.language_tag.eq_ignore_ascii_case(lang_tag) {
                return &m.value;
            }
        }
        // Fallback: match prefix (e.g. "en" matches "en-US")
        let prefix = lang_tag.split('-').next().unwrap_or(lang_tag);
        for m in &self.match_queue {
            if m.language_tag.starts_with(prefix) {
                return &m.value;
            }
        }
        &self.default
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocaleMatch {
    pub language_tag: String,
    pub value: String,
}

/// A container layer grouping buttons and joysticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLayer {
    pub name: String,
    pub uuid: String,
    #[serde(default)]
    pub hide: bool,
    #[serde(default = "default_true", rename = "hideWhenMouse")]
    pub hide_when_mouse: bool,
    #[serde(default = "default_true", rename = "hideWhenGamepad")]
    pub hide_when_gamepad: bool,
    #[serde(rename = "visibilityType", default)]
    pub visibility_type: VisibilityType,
    #[serde(default, rename = "normalButtons")]
    pub normal_buttons: Vec<NormalButton>,
    #[serde(default, rename = "textBoxes")]
    pub text_boxes: Vec<TextBox>,
    #[serde(default, rename = "joystickButtons")]
    pub joystick_buttons: Vec<JoystickButton>,
}

fn default_true() -> bool {
    true
}

impl ControlLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            uuid: generate_uuid(),
            hide: false,
            hide_when_mouse: true,
            hide_when_gamepad: true,
            visibility_type: VisibilityType::Always,
            normal_buttons: Vec::new(),
            text_boxes: Vec::new(),
            joystick_buttons: Vec::new(),
        }
    }
}

/// Condition determining when controls or layers are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityType {
    #[default]
    Always,
    InGame,
    InMenu,
}

/// Normalized position on screen with 0..10000 range (0.01% resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ButtonPosition {
    pub x: i32,
    pub y: i32,
}

impl ButtonPosition {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Converts normalized coordinates (0..10000) to actual screen pixels.
    pub fn to_screen_pixels(self, screen_w: f32, screen_h: f32) -> (f32, f32) {
        let px = (self.x.clamp(0, 10000) as f32 / 10000.0) * screen_w;
        let py = (self.y.clamp(0, 10000) as f32 / 10000.0) * screen_h;
        (px, py)
    }

    /// Converts screen pixel coordinates to normalized 0..10000 coordinates.
    pub fn from_screen_pixels(px: f32, py: f32, screen_w: f32, screen_h: f32) -> Self {
        let x = if screen_w > 0.0 {
            ((px / screen_w) * 10000.0).round() as i32
        } else {
            0
        };
        let y = if screen_h > 0.0 {
            ((py / screen_h) * 10000.0).round() as i32
        } else {
            0
        };
        Self {
            x: x.clamp(0, 10000),
            y: y.clamp(0, 10000),
        }
    }
}

/// Size unit type: relative percentage or density-independent pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeType {
    #[default]
    Percentage,
    Dp,
}

/// Reference axis for percentage scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeReference {
    ScreenWidth,
    #[default]
    ScreenHeight,
}

/// Sizing configuration for widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonSize {
    #[serde(rename = "type", default)]
    pub size_type: SizeType,
    #[serde(rename = "widthDp", default = "default_size_dp")]
    pub width_dp: f32,
    #[serde(rename = "heightDp", default = "default_size_dp")]
    pub height_dp: f32,
    #[serde(rename = "widthPercentage", default = "default_size_pct")]
    pub width_percentage: i32,
    #[serde(rename = "heightPercentage", default = "default_size_pct")]
    pub height_percentage: i32,
    #[serde(rename = "widthReference", default)]
    pub width_reference: SizeReference,
    #[serde(rename = "heightReference", default)]
    pub height_reference: SizeReference,
}

fn default_size_dp() -> f32 {
    50.0
}

fn default_size_pct() -> i32 {
    1000 // 10%
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self {
            size_type: SizeType::Percentage,
            width_dp: 50.0,
            height_dp: 50.0,
            width_percentage: 1200,
            height_percentage: 1000,
            width_reference: SizeReference::ScreenHeight,
            height_reference: SizeReference::ScreenHeight,
        }
    }
}

impl ButtonSize {
    /// Computes width and height in actual pixels.
    pub fn to_screen_pixels(&self, screen_w: f32, screen_h: f32, display_density: f32) -> (f32, f32) {
        match self.size_type {
            SizeType::Dp => {
                let w = (self.width_dp * display_density).max(5.0);
                let h = (self.height_dp * display_density).max(5.0);
                (w, h)
            }
            SizeType::Percentage => {
                let ref_w = match self.width_reference {
                    SizeReference::ScreenWidth => screen_w,
                    SizeReference::ScreenHeight => screen_h,
                };
                let ref_h = match self.height_reference {
                    SizeReference::ScreenWidth => screen_w,
                    SizeReference::ScreenHeight => screen_h,
                };
                let w = (self.width_percentage as f32 / 10000.0 * ref_w).max(10.0);
                let h = (self.height_percentage as f32 / 10000.0 * ref_h).max(10.0);
                (w, h)
            }
        }
    }
}

/// Corner radius percentages (0..100%).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ButtonShape {
    #[serde(rename = "topStart", default)]
    pub top_start: f32,
    #[serde(rename = "topEnd", default)]
    pub top_end: f32,
    #[serde(rename = "bottomEnd", default)]
    pub bottom_end: f32,
    #[serde(rename = "bottomStart", default)]
    pub bottom_start: f32,
}

/// Text alignment within the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Standard interactive button.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalButton {
    pub uuid: String,
    pub text: TranslatableText,
    pub position: ButtonPosition,
    #[serde(rename = "buttonSize")]
    pub button_size: ButtonSize,
    #[serde(rename = "buttonStyle", default)]
    pub button_style: Option<String>,
    #[serde(rename = "textAlignment", default)]
    pub text_alignment: TextAlignment,
    #[serde(rename = "textBold", default)]
    pub text_bold: bool,
    #[serde(rename = "textItalic", default)]
    pub text_italic: bool,
    #[serde(rename = "textUnderline", default)]
    pub text_underline: bool,
    #[serde(rename = "visibilityType", default)]
    pub visibility_type: VisibilityType,
    #[serde(rename = "clickEvents", default)]
    pub click_events: Vec<ClickEvent>,
    #[serde(rename = "isSwipple", default)]
    pub is_swipple: bool,
    #[serde(rename = "isPenetrable", default)]
    pub is_penetrable: bool,
    #[serde(rename = "isToggleable", default)]
    pub is_toggleable: bool,
}

impl NormalButton {
    pub fn new(text: impl Into<String>, x: i32, y: i32) -> Self {
        Self {
            uuid: generate_uuid(),
            text: TranslatableText::single(text),
            position: ButtonPosition::new(x, y),
            button_size: ButtonSize::default(),
            button_style: None,
            text_alignment: TextAlignment::Center,
            text_bold: false,
            text_italic: false,
            text_underline: false,
            visibility_type: VisibilityType::Always,
            click_events: Vec::new(),
            is_swipple: false,
            is_penetrable: false,
            is_toggleable: false,
        }
    }
}

/// Static text label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBox {
    pub uuid: String,
    pub text: TranslatableText,
    pub position: ButtonPosition,
    #[serde(rename = "buttonSize")]
    pub button_size: ButtonSize,
    #[serde(rename = "buttonStyle", default)]
    pub button_style: Option<String>,
    #[serde(rename = "textAlignment", default)]
    pub text_alignment: TextAlignment,
    #[serde(rename = "textBold", default)]
    pub text_bold: bool,
    #[serde(rename = "textItalic", default)]
    pub text_italic: bool,
    #[serde(rename = "textUnderline", default)]
    pub text_underline: bool,
    #[serde(rename = "visibilityType", default)]
    pub visibility_type: VisibilityType,
}

/// Trigger mode for joysticks: continuous drag or directional tap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JoystickTriggerMode {
    #[default]
    Drag,
    Tap,
}

/// 4-cardinal joystick movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JoystickDirection {
    Forward,
    Back,
    Left,
    Right,
}

/// Analog movement joystick widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoystickButton {
    pub uuid: String,
    pub position: ButtonPosition,
    #[serde(rename = "sizeType", default)]
    pub size_type: SizeType,
    #[serde(rename = "sizeDp", default = "default_joystick_size_dp")]
    pub size_dp: f32,
    #[serde(rename = "sizePercentage", default = "default_joystick_size_pct")]
    pub size_percentage: i32,
    #[serde(rename = "visibilityType", default)]
    pub visibility_type: VisibilityType,
    #[serde(rename = "joystickStyleId", default)]
    pub joystick_style_id: Option<String>,
    #[serde(rename = "deadZoneRatio", default = "default_dead_zone")]
    pub dead_zone_ratio: f32,
    #[serde(rename = "lockThreshold", default = "default_lock_threshold")]
    pub lock_threshold: f32,
    #[serde(rename = "canLock", default = "default_true")]
    pub can_lock: bool,
    #[serde(rename = "triggerMode", default)]
    pub trigger_mode: JoystickTriggerMode,
    #[serde(rename = "directionEvents", default = "default_direction_events")]
    pub direction_events: BTreeMap<JoystickDirection, Vec<ClickEvent>>,
    #[serde(rename = "lockEvents", default = "default_lock_events")]
    pub lock_events: Vec<ClickEvent>,
}

fn default_joystick_size_dp() -> f32 {
    200.0
}
fn default_joystick_size_pct() -> i32 {
    2500 // 25% of height
}
fn default_dead_zone() -> f32 {
    0.5
}
fn default_lock_threshold() -> f32 {
    0.3
}

fn default_direction_events() -> BTreeMap<JoystickDirection, Vec<ClickEvent>> {
    let mut map = BTreeMap::new();
    map.insert(JoystickDirection::Forward, vec![ClickEvent::key("GLFW_KEY_W")]);
    map.insert(JoystickDirection::Back, vec![ClickEvent::key("GLFW_KEY_S")]);
    map.insert(JoystickDirection::Left, vec![ClickEvent::key("GLFW_KEY_A")]);
    map.insert(JoystickDirection::Right, vec![ClickEvent::key("GLFW_KEY_D")]);
    map
}

fn default_lock_events() -> Vec<ClickEvent> {
    vec![ClickEvent::key("GLFW_KEY_LEFT_CONTROL")]
}

impl Default for JoystickButton {
    fn default() -> Self {
        Self {
            uuid: generate_uuid(),
            position: ButtonPosition::new(1000, 8000),
            size_type: SizeType::Percentage,
            size_dp: 200.0,
            size_percentage: 2500,
            visibility_type: VisibilityType::Always,
            joystick_style_id: None,
            dead_zone_ratio: 0.5,
            lock_threshold: 0.3,
            can_lock: true,
            trigger_mode: JoystickTriggerMode::Drag,
            direction_events: default_direction_events(),
            lock_events: default_lock_events(),
        }
    }
}

/// Visual theme style for buttons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonStyle {
    pub name: String,
    pub uuid: String,
    #[serde(rename = "animateSwap", default)]
    pub animate_swap: bool,
    #[serde(rename = "commonStyle", default = "default_true")]
    pub common_style: bool,
    #[serde(rename = "lightStyle")]
    pub light_style: ButtonStyleConfig,
    #[serde(rename = "darkStyle")]
    pub dark_style: ButtonStyleConfig,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            uuid: generate_uuid(),
            animate_swap: false,
            common_style: true,
            light_style: ButtonStyleConfig::default(),
            dark_style: ButtonStyleConfig::default(),
        }
    }
}

/// Detailed appearance configuration (colors, font, borders).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonStyleConfig {
    #[serde(default = "default_alpha")]
    pub alpha: f32,
    #[serde(rename = "pressedAlpha", default = "default_alpha")]
    pub pressed_alpha: f32,
    #[serde(rename = "backgroundColor", default = "default_bg_color")]
    pub background_color: u32,
    #[serde(rename = "pressedBackgroundColor", default = "default_pressed_bg_color")]
    pub pressed_background_color: u32,
    #[serde(rename = "contentColor", default = "default_fg_color")]
    pub content_color: u32,
    #[serde(rename = "pressedContentColor", default = "default_fg_color")]
    pub pressed_content_color: u32,
    #[serde(rename = "fontSize", default)]
    pub font_size: Option<i32>,
    #[serde(rename = "borderWidth", default)]
    pub border_width: i32,
    #[serde(rename = "borderColor", default = "default_fg_color")]
    pub border_color: u32,
    #[serde(rename = "borderRadius", default)]
    pub border_radius: ButtonShape,
}

fn default_alpha() -> f32 {
    1.0
}
fn default_bg_color() -> u32 {
    0x80000000 // 50% translucent black
}
fn default_pressed_bg_color() -> u32 {
    0xB3888888 // 70% translucent gray
}
fn default_fg_color() -> u32 {
    0xFFFFFFFF // Opaque white
}

impl Default for ButtonStyleConfig {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            pressed_alpha: 1.0,
            background_color: default_bg_color(),
            pressed_background_color: default_pressed_bg_color(),
            content_color: default_fg_color(),
            pressed_content_color: default_fg_color(),
            font_size: Some(14),
            border_width: 0,
            border_color: default_fg_color(),
            border_radius: ButtonShape::default(),
        }
    }
}

/// Visual theme style for joysticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoystickStyle {
    pub name: String,
    pub uuid: String,
    #[serde(rename = "commonStyle", default = "default_true")]
    pub common_style: bool,
    #[serde(rename = "lightStyle")]
    pub light_style: JoystickStyleConfig,
    #[serde(rename = "darkStyle")]
    pub dark_style: JoystickStyleConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoystickStyleConfig {
    #[serde(default = "default_alpha")]
    pub alpha: f32,
    #[serde(rename = "backgroundColor", default = "default_bg_color")]
    pub background_color: u32,
    #[serde(rename = "joystickColor", default = "default_pressed_bg_color")]
    pub joystick_color: u32,
    #[serde(rename = "joystickCanLockColor", default = "default_can_lock_color")]
    pub joystick_can_lock_color: u32,
    #[serde(rename = "joystickLockedColor", default = "default_locked_color")]
    pub joystick_locked_color: u32,
    #[serde(rename = "lockMarkColor", default = "default_fg_color")]
    pub lock_mark_color: u32,
    #[serde(rename = "borderWidthRatio", default)]
    pub border_width_ratio: i32,
    #[serde(rename = "borderColor", default = "default_fg_color")]
    pub border_color: u32,
    #[serde(rename = "backgroundShape", default)]
    pub background_shape: i32,
    #[serde(rename = "joystickShape", default)]
    pub joystick_shape: i32,
    #[serde(rename = "joystickSize", default = "default_knob_size")]
    pub joystick_size: f32,
}

fn default_can_lock_color() -> u32 {
    0xFFFFA500 // Orange
}
fn default_locked_color() -> u32 {
    0xFFFF4444 // Red
}
fn default_knob_size() -> f32 {
    0.4 // 40% of outer diameter
}

impl Default for JoystickStyleConfig {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            background_color: default_bg_color(),
            joystick_color: default_pressed_bg_color(),
            joystick_can_lock_color: default_can_lock_color(),
            joystick_locked_color: default_locked_color(),
            lock_mark_color: default_fg_color(),
            border_width_ratio: 0,
            border_color: default_fg_color(),
            background_shape: 50, // 50 = circle
            joystick_shape: 50,
            joystick_size: default_knob_size(),
        }
    }
}

/// Action event triggered by control clicks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickEvent {
    #[serde(rename = "type")]
    pub event_type: ClickEventType,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickEventType {
    Key,
    LauncherEvent,
    SwitchLayer,
    ShowLayer,
    HideLayer,
    SendText,
}

impl ClickEvent {
    pub fn new(event_type: ClickEventType, key: impl Into<String>) -> Self {
        Self {
            event_type,
            key: key.into(),
        }
    }

    pub fn key(key_name: impl Into<String>) -> Self {
        Self::new(ClickEventType::Key, key_name)
    }

    pub fn launcher(event_name: impl Into<String>) -> Self {
        Self::new(ClickEventType::LauncherEvent, event_name)
    }

    pub fn switch_layer(layer_uuid: impl Into<String>) -> Self {
        Self::new(ClickEventType::SwitchLayer, layer_uuid)
    }

    pub fn is_layer_event(&self) -> bool {
        matches!(
            self.event_type,
            ClickEventType::SwitchLayer | ClickEventType::ShowLayer | ClickEventType::HideLayer
        )
    }
}

// ponytail: basic deserialization handles versions up to 12. Add migration pipeline when schema changes.
