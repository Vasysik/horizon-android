//! Android menu gestures and auxiliary keys, independent of gameplay fingers.
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    osk::{Keyboard, Output},
};
use std::{
    cell::Cell,
    collections::BTreeMap,
    time::{Duration, Instant},
};
use winit::{event::TouchPhase, keyboard::KeyCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetDialogState {
    pub is_copy: bool,
    pub target_id: String,
    pub name_input: String,
    pub desc_input: String,
    pub editing_desc: bool,
}

pub struct TouchUi {
    pub expanded: bool,
    pub floating_menu: bool,
    pub floating_drag: Option<(u64, [f32; 2], bool)>,
    pub hub_tab: u8,
    pub hub_scroll: f32,
    pub hub_max_scroll: Cell<f32>,
    pub hub_scroll_drag: Option<(u64, f32, f32, bool)>,
    pub presets_modal_open: bool,
    pub preset_dialog: Option<PresetDialogState>,
    pub cached_presets: Vec<crate::touch::PresetMeta>,
    pub cached_presets_style: Option<crate::touch::ControlStyle>,
    pub sensitivity_dragging: bool,
    /// The pointer-size slider of the Aiming tab is being dragged.
    pub pointer_size_dragging: bool,
    /// The virtual-mouse-speed slider of the Aiming tab is being dragged.
    pub mouse_speed_dragging: bool,
    /// The real-time 3D resolution scale slider of the Recommended tab is being dragged.
    pub resolution_scale_dragging: bool,
    /// The gyroscope sensitivity slider of the Aiming tab is being dragged.
    pub gyro_sensitivity_dragging: bool,
    /// The global button scale slider of the Buttons tab is being dragged.
    pub button_scale_dragging: bool,
    /// Step state for the Floating Menu "Send Custom Input" picker modal.
    pub custom_input_picker: crate::touch::PickerStep,
    /// Horizontal scroll offset for the Floating Menu custom keyboard picker.
    pub custom_kb_scroll_x: f32,
    /// Active drag state `(touch_id, last_x, moved)` for the Floating Menu custom keyboard picker.
    pub custom_kb_drag: Option<(u64, f32, bool)>,
    pub goy_mode: bool,
    pub virtual_mouse: bool,
    /// Mirror of `!settings.virtual_mouse_enabled` for legacy callers; authoritative state lives in `touch::Settings::virtual_mouse_enabled`.
    pub pointer_off: bool,
    /// Whether the virtual pointer has been spawned at the playable viewport center at least once.
    pub cursor_initialized: bool,
    /// Last observed playable viewport `(width, height)` in GUI coordinates for proportional resize handling.
    pub last_viewport: Option<(f32, f32)>,
    /// Manual in-game virtual pointer override when toggled via GoyControl's `mouse.virtual.toggle` button.
    pub goy_in_game_mouse: bool,
    pub open_anim: Cell<f32>,
    pub tab_anim: Cell<f32>,
    pub last_anim_tick: Cell<Option<Instant>>,
    mouse_buttons: BTreeMap<u64, bool>,
    latched_modifiers: std::collections::HashSet<KeyCode>,
    relative: Option<RelativeTouch>,
    relative_click: Option<bool>,
    pub keyboard: Option<Keyboard>,
    gesture: Option<(u64, [f32; 2], [f32; 2], bool)>,
    pointer_drag: Option<u64>,
    held_keys: BTreeMap<u64, KeyCode>,
    pub f3_latched: bool,
}

impl Default for TouchUi {
    fn default() -> Self {
        Self {
            expanded: false,
            floating_menu: false,
            floating_drag: None,
            hub_tab: 0,
            hub_scroll: 0.0,
            hub_max_scroll: Cell::new(0.0),
            hub_scroll_drag: None,
            presets_modal_open: false,
            preset_dialog: None,
            cached_presets: Vec::new(),
            cached_presets_style: None,
            sensitivity_dragging: false,
            pointer_size_dragging: false,
            mouse_speed_dragging: false,
            resolution_scale_dragging: false,
            gyro_sensitivity_dragging: false,
            button_scale_dragging: false,
            custom_input_picker: crate::touch::PickerStep::None,
            custom_kb_scroll_x: 0.0,
            custom_kb_drag: None,
            goy_mode: false,
            virtual_mouse: false,
            pointer_off: false,
            cursor_initialized: false,
            last_viewport: None,
            goy_in_game_mouse: false,
            open_anim: Cell::new(0.0),
            tab_anim: Cell::new(1.0),
            last_anim_tick: Cell::new(None),
            mouse_buttons: BTreeMap::new(),
            latched_modifiers: std::collections::HashSet::new(),
            relative: None,
            relative_click: None,
            keyboard: None,
            gesture: None,
            pointer_drag: None,
            held_keys: BTreeMap::new(),
            f3_latched: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InGameMenuAction {
    Close,
    OpenControlEditor,
    SetControlStyle(crate::touch::ControlStyle),
    SelectPreset(String),
    CopyPreset {
        source_id: String,
        name: String,
        description: String,
    },
    UpdatePresetMeta {
        id: String,
        name: String,
        description: String,
    },
    ApplyPreset(&'static str),
    LoadSavedProfile(usize),
    PresetVisibilityAll,
    CycleMovementMode,
    CycleJoystickArea,
    ToggleSplitTouch,
    ToggleInvertY,
    ToggleLeftHanded,
    ToggleGyro,
    ToggleVirtualMouse,
    ToggleSprintMode,
    ToggleInventoryControls,
    ToggleVibrate,
    ToggleUiVibrate,
    ToggleAutoJump,
    SetTouchSensitivity(f32),
    StepTouchSensitivity(i32),
    SetVirtualMouseSpeed(f32),
    StepVirtualMouseSpeed(i32),
    SetVirtualMousePointerSize(f32),
    SetGyroSensitivity(f32),
    StepGyroSensitivity(i32),
    SetButtonScale(f32),
    StepButtonScale(i32),
    TriggerKey(KeyCode),
    OpenKeyboard,
    /// Kill the game process now (no save, no shutdown sequence).
    ForceKill,
    /// Open the game's log folder in the system file manager.
    OpenLogs,
    /// The naming field of a preset dialog was tapped: show the system keyboard.
    EditPresetField(bool),
    /// Adjust real-time 3D world resolution scale percentage (25%..=100%) without recreating UI surface.
    SetResolutionScale(u32),
    /// Toggle in-game animations (animated textures, view bobbing, chunk fade, UI transitions).
    ToggleGameAnimations,
    /// Send a custom keyboard key, mouse button/scroll, or gameplay action token immediately.
    SendCustomInput(String),
}

fn key_code_to_char(code: KeyCode) -> Option<char> {
    match code {
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        KeyCode::Digit0 | KeyCode::Numpad0 => Some('0'),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some('1'),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some('2'),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some('3'),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some('4'),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some('5'),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some('6'),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some('7'),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some('8'),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus | KeyCode::NumpadSubtract => Some('-'),
        KeyCode::Period | KeyCode::NumpadDecimal => Some('.'),
        KeyCode::Comma => Some(','),
        KeyCode::Slash | KeyCode::NumpadDivide => Some('/'),
        _ => None,
    }
}

/// Whether a dialog consumed an output that confirms or dismisses it. The
/// library only edits dialog state; the Android host hides its soft keyboard.
pub fn output_dismisses(out: &Output) -> bool {
    matches!(
        out,
        Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape)
    )
}

/// Applies an on-screen or hardware keyboard `Output` event to an active Control Editor
/// Save Preset dialog or Floating Menu Copy/Edit Preset dialog.
pub fn apply_editor_or_preset_output(
    ed: Option<&mut crate::touch::Editor>,
    preset_dlg: Option<&mut PresetDialogState>,
    out: &Output,
) -> bool {
    if let Some(ed) = ed
        && ed.save_dialog_open
    {
        let max_len = if ed.editing_save_desc { 48 } else { 28 };
        let target = if ed.editing_save_desc {
            &mut ed.save_desc_input
        } else {
            &mut ed.save_name_input
        };
        match out {
            Output::Text(s) => {
                for ch in s.chars() {
                    if !ch.is_control() && target.chars().count() < max_len {
                        target.push(ch);
                    }
                }
            }
            Output::Backspace | Output::Key(KeyCode::Backspace) => {
                target.pop();
            }
            Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape) => {}
            Output::Key(code) => {
                if let Some(ch) = key_code_to_char(*code)
                    && target.chars().count() < max_len
                {
                    target.push(ch);
                }
            }
        }
        return true;
    }
    if let Some(dlg) = preset_dlg {
        let max_len = if dlg.editing_desc { 48 } else { 28 };
        let target = if dlg.editing_desc {
            &mut dlg.desc_input
        } else {
            &mut dlg.name_input
        };
        match out {
            Output::Text(s) => {
                for ch in s.chars() {
                    if !ch.is_control() && target.chars().count() < max_len {
                        target.push(ch);
                    }
                }
            }
            Output::Backspace | Output::Key(KeyCode::Backspace) => {
                target.pop();
            }
            Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape) => {}
            Output::Key(code) => {
                if let Some(ch) = key_code_to_char(*code)
                    && target.chars().count() < max_len
                {
                    target.push(ch);
                }
            }
        }
        return true;
    }
    false
}
struct RelativeTouch {
    id: u64,
    origin: [f32; 2],
    last: [f32; 2],
    cursor: [f32; 2],
    since: Instant,
    dragged: bool,
    right_sent: bool,
}
pub enum Gesture {
    None,
    Tap,
    Scroll(f32),
}
pub enum Action {
    LatchF3,
    Pick,
    DebugCombo(KeyCode),
    Key(KeyCode),
    Keyboard,
    VirtualMouse,
    MouseButton(bool),
    More,
}

const KEYS: [(&str, KeyCode); 28] = [
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("Tab", KeyCode::Tab),
    ("Swap", KeyCode::KeyF),
    ("Drop", KeyCode::KeyQ),
    ("Adv", KeyCode::KeyL),
    ("Chat", KeyCode::KeyT),
    ("/", KeyCode::Slash),
    ("Enter", KeyCode::Enter),
    ("Esc", KeyCode::Escape),
    ("Shift", KeyCode::ShiftLeft),
    ("Ctrl", KeyCode::ControlLeft),
    ("Inv", KeyCode::KeyE),
    ("Pick", KeyCode::ContextMenu),
    ("F3+B", KeyCode::KeyB),
    ("F3+G", KeyCode::KeyG),
    ("F3+A", KeyCode::KeyA),
    ("F3+T", KeyCode::KeyT),
];

fn mouse_button_rects(w: f32, h: f32) -> [([f32; 4], bool); 2] {
    [
        ([w - 53.0, h - 52.0, 48.0, 22.0], false),
        ([w - 53.0, h - 28.0, 48.0, 22.0], true),
    ]
}

/// Faithful Windows 11-style standard arrow pointer (`aero_arrow`) with sub-pixel
/// vector geometry, crisp dark outline, pure white body, subtle contact shadow,
/// and exact tip hotspot at `(cursor.0, cursor.1)` clamped cleanly to the screen bounds.
pub fn draw_pointer(batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32), scale: f32) {
    let scale = if scale.is_finite() {
        scale.clamp(0.5, 2.0)
    } else {
        1.0
    };
    let sw = batch.width.max(1.0);
    let sh = batch.height.max(1.0);
    let h = (sh * 0.052).clamp(12.0, 15.5) * scale;
    let s = h / 16.0;
    let x = cursor.0.clamp(0.0, (sw - 3.0 * scale).max(0.0));
    let y = cursor.1.clamp(0.0, (sh - 3.0 * scale).max(0.0));
    let pt = |dx: f32, dy: f32| -> [f32; 2] {
        [(x + dx * s).clamp(0.0, sw), (y + dy * s).clamp(0.0, sh)]
    };

    let shadow: u32 = rgba(0, 0, 0, 64);
    let outline: u32 = rgba(18, 18, 22, 248);
    let fill: u32 = rgba(255, 255, 255, 255);

    // 1. Subtle contact drop shadow (+0.65, +0.90) for contrast over bright surfaces
    let sx = 0.65;
    let sy = 0.90;
    batch.fill_quad(
        atlas,
        [
            pt(sx, sy),
            pt(4.1 + sx, 11.4 + sy),
            pt(sx, 14.8 + sy),
            pt(sx, sy),
        ],
        shadow,
    );
    batch.fill_quad(
        atlas,
        [
            pt(sx, sy),
            pt(10.6 + sx, 10.6 + sy),
            pt(6.2 + sx, 10.6 + sy),
            pt(4.1 + sx, 11.4 + sy),
        ],
        shadow,
    );
    batch.fill_quad(
        atlas,
        [
            pt(3.8 + sx, 11.0 + sy),
            pt(6.3 + sx, 10.0 + sy),
            pt(8.6 + sx, 15.3 + sy),
            pt(6.0 + sx, 16.4 + sy),
        ],
        shadow,
    );

    // 2. Crisp Windows 11 outer dark silhouette (exact tip at (0.0, 0.0))
    batch.fill_quad(
        atlas,
        [pt(0.0, 0.0), pt(4.1, 11.4), pt(0.0, 14.8), pt(0.0, 0.0)],
        outline,
    );
    batch.fill_quad(
        atlas,
        [pt(0.0, 0.0), pt(10.6, 10.6), pt(6.2, 10.6), pt(4.1, 11.4)],
        outline,
    );
    batch.fill_quad(
        atlas,
        [pt(3.8, 11.0), pt(6.3, 10.0), pt(8.6, 15.3), pt(6.0, 16.4)],
        outline,
    );

    // 3. Pure white interior fill inset cleanly inside the silhouette
    batch.fill_quad(
        atlas,
        [
            pt(0.85, 1.90),
            pt(4.05, 10.35),
            pt(0.85, 12.85),
            pt(0.85, 1.90),
        ],
        fill,
    );
    batch.fill_quad(
        atlas,
        [
            pt(0.85, 1.90),
            pt(8.65, 9.70),
            pt(5.75, 9.70),
            pt(4.05, 10.35),
        ],
        fill,
    );
    batch.fill_quad(
        atlas,
        [
            pt(4.15, 9.95),
            pt(5.55, 9.35),
            pt(7.60, 14.65),
            pt(6.20, 15.25),
        ],
        fill,
    );
}

impl TouchUi {
    /// Ensures the virtual mouse cursor spawns at the exact center `(width * 0.5, height * 0.5)`
    /// of the playable viewport and scales proportionally across viewport/orientation changes
    /// without offset drift or sudden jumps.
    pub fn sync_viewport_cursor(
        &mut self,
        cursor: &mut (f32, f32),
        [width, height]: [f32; 2],
    ) -> bool {
        if !width.is_finite() || !height.is_finite() || width <= 1.0 || height <= 1.0 {
            return false;
        }
        let max_x = (width - 1.0).max(0.0);
        let max_y = (height - 1.0).max(0.0);
        if !self.cursor_initialized || !cursor.0.is_finite() || !cursor.1.is_finite() {
            *cursor = (width * 0.5, height * 0.5);
            self.cursor_initialized = true;
            self.last_viewport = Some((width, height));
            self.cancel_relative();
            return true;
        }
        if let Some((old_w, old_h)) = self.last_viewport {
            if (old_w - width).abs() > 0.5 || (old_h - height).abs() > 0.5 {
                let nx = (cursor.0 / old_w.max(1.0) * width).clamp(0.0, max_x);
                let ny = (cursor.1 / old_h.max(1.0) * height).clamp(0.0, max_y);
                *cursor = (nx, ny);
                self.last_viewport = Some((width, height));
                self.cancel_relative();
                return true;
            }
        } else {
            self.last_viewport = Some((width, height));
        }
        cursor.0 = cursor.0.clamp(0.0, max_x);
        cursor.1 = cursor.1.clamp(0.0, max_y);
        false
    }

    /// Computes whether the virtual mouse pointer should be visible for the current state:
    /// - If `controls.editor.is_some()`, always returns false (never show cursor over Control Editor).
    /// - If `settings.virtual_mouse_enabled` is false, always returns false.
    /// - When the Floating Menu is open, returns true.
    /// - In MCPE mode: hidden when MCPE gameplay controls are visible; automatically visible
    ///   when MCPE controls are hidden (menus, inventory, HUD hidden, or all controls hidden).
    /// - In GoyControl mode: visible in menus or when toggled on in-game via `mouse.virtual.toggle`.
    pub fn compute_virtual_mouse_visibility(
        &mut self,
        controls: &crate::touch::TouchControls,
        in_menu: bool,
        hide_gui: bool,
        touch_active: bool,
        captured: bool,
    ) -> bool {
        self.pointer_off = !controls.settings.virtual_mouse_enabled;
        if controls.editor.is_some() {
            return false;
        }
        if self.floating_menu
            || self.preset_dialog.is_some()
            || self.presets_modal_open
            || self.custom_input_picker != crate::touch::PickerStep::None
        {
            return false;
        }
        if !controls.settings.virtual_mouse_enabled {
            self.goy_in_game_mouse = false;
            return false;
        }
        match controls.settings.control_style {
            crate::touch::ControlStyle::Mcpe => {
                if in_menu {
                    return true;
                }
                let mcpe_visible = controls.mcpe_controls_visible(false, hide_gui, touch_active);
                !mcpe_visible || !captured || self.goy_in_game_mouse
            }
            crate::touch::ControlStyle::GoyControl => in_menu || self.goy_in_game_mouse,
        }
    }

    /// Captures a physical or soft-keyboard `KeyCode` when the `Send Custom Input` picker modal is open.
    pub fn capture_custom_input_key(&mut self, code: KeyCode) -> Option<String> {
        if self.custom_input_picker == crate::touch::PickerStep::None {
            return None;
        }
        if code == KeyCode::Escape {
            self.custom_input_picker = crate::touch::PickerStep::None;
            return None;
        }
        let token = crate::keybinds::key_name(code)?.to_owned();
        self.custom_input_picker = crate::touch::PickerStep::None;
        self.floating_menu = false;
        Some(token)
    }

    /// Captures a typed character from the Android soft keyboard when `Send Custom Input` is open.
    pub fn capture_custom_input_text(&mut self, text: &str) -> Option<String> {
        if self.custom_input_picker == crate::touch::PickerStep::None {
            return None;
        }
        let ch = text.chars().find(|c| !c.is_control())?;
        let tok = crate::touch::TouchControls::char_to_key_token(ch)?;
        self.custom_input_picker = crate::touch::PickerStep::None;
        self.floating_menu = false;
        Some(tok.to_owned())
    }

    pub fn refresh_presets_cache(&mut self, style: crate::touch::ControlStyle) {
        self.cached_presets =
            crate::touch::Settings::list_presets(std::path::Path::new("."), style);
        self.cached_presets_style = Some(style);
    }
    pub fn ensure_presets_cache(&mut self, style: crate::touch::ControlStyle) {
        if self.cached_presets_style != Some(style) || self.cached_presets.is_empty() {
            self.refresh_presets_cache(style);
        }
    }
    pub fn key_down(&mut self, id: u64, key: KeyCode) -> bool {
        let first = !self.key_held(key);
        self.held_keys.insert(id, key);
        first
    }
    pub fn key_up(&mut self, id: u64) -> Option<KeyCode> {
        let key = self.held_keys.remove(&id)?;
        (!self.key_held(key)).then_some(key)
    }
    pub fn owns_key(&self, id: u64) -> bool {
        self.held_keys.contains_key(&id)
    }
    pub fn toggle_modifier(&mut self, key: KeyCode) -> bool {
        if self.latched_modifiers.remove(&key) {
            false
        } else {
            self.latched_modifiers.insert(key);
            true
        }
    }

    pub fn modifier_latched(&self, key: KeyCode) -> bool {
        self.latched_modifiers.contains(&key)
    }

    pub fn clear_modifiers(&mut self) {
        self.latched_modifiers.clear();
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        (key == KeyCode::F3 && self.f3_latched) || self.held_keys.values().any(|k| *k == key)
    }
    pub fn release_keys(&mut self) -> Vec<KeyCode> {
        let mut keys: Vec<_> = std::mem::take(&mut self.held_keys).into_values().collect();
        if self.f3_latched {
            keys.push(KeyCode::F3);
        }
        self.f3_latched = false;
        keys.sort();
        keys.dedup();
        keys
    }
    pub fn press_mouse(&mut self, id: u64, right: bool) -> bool {
        if let Some(touch) = &mut self.relative {
            touch.dragged = true;
        }
        let first = !self.mouse_buttons.values().any(|v| *v == right);
        self.mouse_buttons.insert(id, right);
        first
    }
    pub fn release_mouse(&mut self, id: u64) -> Option<(bool, bool)> {
        let right = self.mouse_buttons.remove(&id)?;
        Some((right, !self.mouse_buttons.values().any(|v| *v == right)))
    }
    pub fn mouse_held(&self) -> bool {
        !self.mouse_buttons.is_empty()
    }
    /// The pointer holds one of the auxiliary mouse buttons (a `press_mouse`
    /// without its `release_mouse`), so its releases end it even when the
    /// screen or layout changed underneath.
    pub fn owns_mouse(&self, id: u64) -> bool {
        self.mouse_buttons.contains_key(&id)
    }
    pub fn release_all_mouse(&mut self) -> Vec<bool> {
        let mut buttons: Vec<_> = std::mem::take(&mut self.mouse_buttons)
            .into_values()
            .collect();
        buttons.sort();
        buttons.dedup();
        buttons
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.pointer_drag = None;
        self.relative = None;
        self.relative_click = None;
        self.sensitivity_dragging = false;
        self.pointer_size_dragging = false;
        self.mouse_speed_dragging = false;
        self.resolution_scale_dragging = false;
        self.custom_kb_drag = None;
        self.hub_scroll_drag = None;
    }
    pub fn cancel_relative(&mut self) {
        self.relative = None;
        self.relative_click = None;
    }
    pub fn owns_relative(&self, id: u64) -> bool {
        self.relative.as_ref().is_some_and(|t| t.id == id)
    }
    pub fn owns_gesture(&self, id: u64) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|(owner, _, _, _)| *owner == id)
            || self.pointer_drag == Some(id)
    }
    /// Relative motion: touching down does not teleport the cursor, and movement
    /// uses linear 1:1 sensitivity scaling without non-linear acceleration drift.
    pub fn relative_touch(
        &mut self,
        id: u64,
        point: [f32; 2],
        phase: TouchPhase,
        cursor: [f32; 2],
        size: [f32; 2],
        sensitivity: f32,
        clicks: bool,
    ) -> [f32; 2] {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return cursor;
        }
        let sens = if sensitivity.is_finite() {
            sensitivity.clamp(0.1, 4.0)
        } else {
            1.0
        };
        if phase == TouchPhase::Started && self.relative.is_none() {
            self.relative = Some(RelativeTouch {
                id,
                origin: point,
                last: point,
                cursor,
                since: Instant::now(),
                dragged: !clicks,
                right_sent: false,
            });
        }
        let Some(touch) = self.relative.as_mut().filter(|t| t.id == id) else {
            return cursor;
        };
        if (touch.cursor[0] - cursor[0]).abs() > 1.0 || (touch.cursor[1] - cursor[1]).abs() > 1.0 {
            touch.cursor = cursor;
        }
        touch.dragged |= (point[0] - touch.origin[0]).hypot(point[1] - touch.origin[1]) > 3.0;
        let position = [0, 1].map(|i| {
            let max_bound = (size[i] - 1.0).max(0.0);
            (touch.cursor[i] + (point[i] - touch.last[i]) * sens).clamp(0.0, max_bound)
        });
        touch.cursor = position;
        touch.last = point;
        if phase == TouchPhase::Ended {
            if clicks && !touch.dragged && !touch.right_sent {
                self.relative_click = Some(touch.since.elapsed() >= Duration::from_millis(500));
            }
            self.relative = None;
        }
        position
    }
    /// false = LMB, true = RMB. A stationary hold clicks once, even before release.
    pub fn relative_click(&mut self, clicks: bool) -> Option<bool> {
        if !clicks {
            self.relative_click = None;
            return None;
        }
        if let Some(right) = self.relative_click.take() {
            return Some(right);
        }
        let touch = self.relative.as_mut()?;
        if !touch.dragged
            && !touch.right_sent
            && touch.since.elapsed() >= Duration::from_millis(500)
        {
            touch.right_sent = true;
            return Some(true);
        }
        None
    }
    /// Sliders retain mouse dragging; list rows use scroll-or-tap gestures.
    pub fn direct_pointer(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> bool {
        if phase == TouchPhase::Started
            && crate::widgets::last_targets().iter().any(|t| {
                let [x, y, w, h] = t.rect;
                matches!(t.kind, crate::widgets::Kind::Slider { .. })
                    && (x..x + w).contains(&p[0])
                    && (y..y + h).contains(&p[1])
            })
        {
            self.pointer_drag = Some(id);
        }
        if self.pointer_drag == Some(id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pointer_drag = None;
            }
            return true;
        }
        false
    }
    /// A drag scrolls; a tap clicks only on release. Never activate a row while scrolling it.
    pub fn gesture(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> Gesture {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return Gesture::None;
        }
        if phase == TouchPhase::Started {
            if self.gesture.is_none() {
                self.gesture = Some((id, p, p, false));
            }
            return Gesture::None;
        }
        let Some((owner, start, previous, dragged)) = &mut self.gesture else {
            return Gesture::None;
        };
        if *owner != id {
            return Gesture::None;
        }
        *dragged |= (p[0] - start[0]).hypot(p[1] - start[1]) > 12.0;
        let dy = p[1] - previous[1];
        *previous = p;
        if phase == TouchPhase::Ended {
            let tap = !*dragged;
            self.cancel();
            return if tap { Gesture::Tap } else { Gesture::None };
        }
        if *dragged {
            Gesture::Scroll(dy / 30.0)
        } else {
            Gesture::None
        }
    }
    pub fn action(&self, w: f32, h: f32, p: [f32; 2], menu: bool) -> Option<Action> {
        if !self.goy_mode {
            if menu && self.virtual_mouse {
                for ([x, y, width, height], right) in mouse_button_rects(w, h) {
                    if (x..x + width).contains(&p[0]) && (y..y + height).contains(&p[1]) {
                        return Some(Action::MouseButton(right));
                    }
                }
            }
            if menu && (3.0..23.0).contains(&p[1]) && (w - 53.0..w - 3.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::Escape));
            }
        }
        if (self.expanded || !self.goy_mode) && (3.0..23.0).contains(&p[1]) {
            if (39.0..72.0).contains(&p[0]) {
                return Some(Action::More);
            }
            if self.expanded && (75.0..108.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F3));
            }
            if self.expanded && (111.0..144.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F5));
            }
            if self.expanded && (147.0..180.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::KeyT));
            }
            if self.expanded && (183.0..222.0).contains(&p[0]) {
                return Some(Action::Keyboard);
            }
        }
        if self.expanded && (3.0..203.0).contains(&p[0]) && (27.0..219.0).contains(&p[1]) {
            let i = ((p[1] - 27.0) / 24.0) as usize * 4 + ((p[0] - 3.0) / 50.0) as usize;
            if i == 28 {
                return Some(Action::LatchF3);
            }
            return KEYS.get(i).map(|(_, k)| match i {
                23 => Action::Pick,
                24.. => Action::DebugCombo(*k),
                _ => Action::Key(*k),
            });
        }
        None
    }
    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas, menu: bool) {
        if menu && !self.goy_mode && crate::touch::current_hud_opacity_scale() > 0.01 {
            button(b, a, "Back", b.width - 53.0, 3.0, 50.0);
            if self.virtual_mouse {
                let [lmb, rmb] = mouse_button_rects(b.width, b.height);
                button(b, a, "LMB", lmb.0[0], lmb.0[1], lmb.0[2]);
                button(b, a, "RMB", rmb.0[0], rmb.0[1], rmb.0[2]);
            }
        }
        if !self.expanded {
            if !self.goy_mode && crate::touch::current_hud_opacity_scale() > 0.01 {
                button(b, a, "...", 39.0, 3.0, 33.0);
            }
            return;
        }
        for (label, x, width) in [
            ("...", 39.0, 33.0),
            ("F3", 75.0, 33.0),
            ("F5", 111.0, 33.0),
            ("Chat", 147.0, 33.0),
            ("Keys", 183.0, 39.0),
        ] {
            button(
                b,
                a,
                if label == "F3" && self.key_held(KeyCode::F3) {
                    "F3*"
                } else {
                    label
                },
                x,
                3.0,
                width,
            );
        }
        button(
            b,
            a,
            if self.f3_latched { "F3 ON" } else { "HoldF3" },
            3.0,
            195.0,
            48.0,
        );
        for (i, (label, _)) in KEYS.iter().enumerate() {
            button(
                b,
                a,
                label,
                3.0 + (i % 4) as f32 * 50.0,
                27.0 + (i / 4) as f32 * 24.0,
                48.0,
            );
        }
    }
    pub fn keyboard_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        p: [f32; 2],
    ) -> Option<Output> {
        let ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
        self.keyboard.as_mut()?.touch(
            ui.w,
            ui.h,
            false,
            [
                p[0] * ui.batch.scale / ui.density,
                p[1] * ui.batch.scale / ui.density,
            ],
        )
    }
    pub fn draw_keyboard(&self, b: &mut HudBatch, a: &HudAtlas) {
        if let Some(k) = &self.keyboard {
            let mut ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
            k.draw(&mut ui, false, crate::gamepad::GlyphStyle::Xbox);
            ui.finish();
        }
    }
}
fn button(b: &mut HudBatch, a: &HudAtlas, label: &str, x: f32, y: f32, w: f32) {
    if !crate::touch::theme_button(b, a, "button", [x, y, w, 20.0], false, u32::MAX, true) {
        b.fill(a, x, y, w, 20.0, rgba(16, 18, 24, 180));
    }
    let text_x = if label == "..." {
        x + (w - HudBatch::text_width(a, label)) * 0.5
    } else {
        x + 4.0
    };
    b.text(a, label, text_x, y + 6.0, rgba(245, 245, 245, 255), true);
}

/// Whether a physical left click takes part in the pad's utility buttons
/// instead of going to the screen handler. Presses start only over a
/// touched button (anything else belongs to the world: attack, use).
/// Releases end whatever the pad already owns even when the layout no
/// longer touches, so a held press survives menu entry, exit and the IME
/// (a release the press never produced is a stray and is ignored).
pub fn aux_mouse_phase(pressed: bool, owned: bool, touched: bool) -> Option<TouchPhase> {
    (if pressed { touched } else { owned }).then_some(if pressed {
        TouchPhase::Started
    } else {
        TouchPhase::Ended
    })
}

const COLOR_SWATCHES: [(u32, &str); 8] = [
    (0xFF1E1B2E, "Obsidian"),
    (0xFF9A7BFF, "Violet"),
    (0xFFD0BCFF, "Lavender"),
    (0xFF38BDF8, "Cyan"),
    (0xFF34D399, "Emerald"),
    (0xFFFBBF24, "Amber"),
    (0xFFF87171, "Crimson"),
    (0xFFF4EFFA, "Snow"),
];

const KB_ROWS: &[&[(&str, &str, f32)]] = &[
    &[
        ("ESC", "key.keyboard.escape", 1.1),
        ("F1", "key.keyboard.f1", 1.0),
        ("F2", "key.keyboard.f2", 1.0),
        ("F3", "key.keyboard.f3", 1.0),
        ("F4", "key.keyboard.f4", 1.0),
        ("F5", "key.keyboard.f5", 1.0),
        ("F6", "key.keyboard.f6", 1.0),
        ("F7", "key.keyboard.f7", 1.0),
        ("F8", "key.keyboard.f8", 1.0),
        ("F9", "key.keyboard.f9", 1.0),
        ("F10", "key.keyboard.f10", 1.0),
        ("F11", "key.keyboard.f11", 1.0),
        ("F12", "key.keyboard.f12", 1.0),
        ("PRT", "key.keyboard.print.screen", 1.05),
        ("SCR", "key.keyboard.scroll.lock", 1.05),
        ("PAU", "key.keyboard.pause", 1.05),
    ],
    &[
        ("`", "key.keyboard.grave.accent", 1.0),
        ("1", "key.keyboard.1", 1.0),
        ("2", "key.keyboard.2", 1.0),
        ("3", "key.keyboard.3", 1.0),
        ("4", "key.keyboard.4", 1.0),
        ("5", "key.keyboard.5", 1.0),
        ("6", "key.keyboard.6", 1.0),
        ("7", "key.keyboard.7", 1.0),
        ("8", "key.keyboard.8", 1.0),
        ("9", "key.keyboard.9", 1.0),
        ("0", "key.keyboard.0", 1.0),
        ("-", "key.keyboard.minus", 1.0),
        ("=", "key.keyboard.equal", 1.0),
        ("BKSP", "key.keyboard.backspace", 1.65),
        ("INS", "key.keyboard.insert", 1.05),
        ("HOME", "key.keyboard.home", 1.1),
        ("PGUP", "key.keyboard.page.up", 1.1),
        ("NLK", "key.keyboard.num.lock", 1.05),
        ("KP/", "key.keyboard.keypad.divide", 1.0),
        ("KP*", "key.keyboard.keypad.multiply", 1.0),
        ("KP-", "key.keyboard.keypad.subtract", 1.0),
    ],
    &[
        ("TAB", "key.keyboard.tab", 1.35),
        ("Q", "key.keyboard.q", 1.0),
        ("W", "key.keyboard.w", 1.0),
        ("E", "key.keyboard.e", 1.0),
        ("R", "key.keyboard.r", 1.0),
        ("T", "key.keyboard.t", 1.0),
        ("Y", "key.keyboard.y", 1.0),
        ("U", "key.keyboard.u", 1.0),
        ("I", "key.keyboard.i", 1.0),
        ("O", "key.keyboard.o", 1.0),
        ("P", "key.keyboard.p", 1.0),
        ("[", "key.keyboard.left.bracket", 1.0),
        ("]", "key.keyboard.right.bracket", 1.0),
        ("\\", "key.keyboard.backslash", 1.3),
        ("DEL", "key.keyboard.delete", 1.05),
        ("END", "key.keyboard.end", 1.1),
        ("PGDN", "key.keyboard.page.down", 1.1),
        ("KP7", "key.keyboard.keypad.7", 1.0),
        ("KP8", "key.keyboard.keypad.8", 1.0),
        ("KP9", "key.keyboard.keypad.9", 1.0),
        ("KP+", "key.keyboard.keypad.add", 1.0),
    ],
    &[
        ("CAPS", "key.keyboard.caps.lock", 1.55),
        ("A", "key.keyboard.a", 1.0),
        ("S", "key.keyboard.s", 1.0),
        ("D", "key.keyboard.d", 1.0),
        ("F", "key.keyboard.f", 1.0),
        ("G", "key.keyboard.g", 1.0),
        ("H", "key.keyboard.h", 1.0),
        ("J", "key.keyboard.j", 1.0),
        ("K", "key.keyboard.k", 1.0),
        ("L", "key.keyboard.l", 1.0),
        (";", "key.keyboard.semicolon", 1.0),
        ("'", "key.keyboard.apostrophe", 1.0),
        ("ENTER", "key.keyboard.enter", 1.75),
        ("KP4", "key.keyboard.keypad.4", 1.0),
        ("KP5", "key.keyboard.keypad.5", 1.0),
        ("KP6", "key.keyboard.keypad.6", 1.0),
        ("KP=", "key.keyboard.keypad.equal", 1.0),
    ],
    &[
        ("LSHIFT", "key.keyboard.left.shift", 1.85),
        ("Z", "key.keyboard.z", 1.0),
        ("X", "key.keyboard.x", 1.0),
        ("C", "key.keyboard.c", 1.0),
        ("V", "key.keyboard.v", 1.0),
        ("B", "key.keyboard.b", 1.0),
        ("N", "key.keyboard.n", 1.0),
        ("M", "key.keyboard.m", 1.0),
        (",", "key.keyboard.comma", 1.0),
        (".", "key.keyboard.period", 1.0),
        ("/", "key.keyboard.slash", 1.0),
        ("RSHIFT", "key.keyboard.right.shift", 1.85),
        ("UP", "key.keyboard.up", 1.15),
        ("KP1", "key.keyboard.keypad.1", 1.0),
        ("KP2", "key.keyboard.keypad.2", 1.0),
        ("KP3", "key.keyboard.keypad.3", 1.0),
        ("KPENT", "key.keyboard.keypad.enter", 1.15),
    ],
    &[
        ("LCTRL", "key.keyboard.left.control", 1.35),
        ("LWIN", "key.keyboard.left.win", 1.15),
        ("LALT", "key.keyboard.left.alt", 1.25),
        ("SPACE", "key.keyboard.space", 4.2),
        ("RALT", "key.keyboard.right.alt", 1.25),
        ("MENU", "key.keyboard.menu", 1.15),
        ("RCTRL", "key.keyboard.right.control", 1.35),
        ("LEFT", "key.keyboard.left", 1.1),
        ("DOWN", "key.keyboard.down", 1.1),
        ("RIGHT", "key.keyboard.right", 1.1),
        ("KP0", "key.keyboard.keypad.0", 1.6),
        ("KP.", "key.keyboard.keypad.period", 1.0),
    ],
];

const MOUSE_AND_ACTION_ITEMS: &[(&str, &str)] = &[
    ("Left Click (LMB)", "key.mouse.left"),
    ("Right Click (RMB)", "key.mouse.right"),
    ("Middle Click (MMB)", "key.mouse.middle"),
    ("Mouse 4 (Back)", "key.mouse.4"),
    ("Mouse 5 (Forward)", "key.mouse.5"),
    ("Scroll Wheel Up", "mouse.scroll.up"),
    ("Scroll Wheel Down", "mouse.scroll.down"),
    ("Toggle Virtual Mouse", "mouse.virtual.toggle"),
    ("Mouse Look Area", "mouse.look"),
    ("Action: Attack / Mine", "action:key.attack"),
    ("Action: Use / Place", "action:key.use"),
    ("Action: Jump", "action:key.jump"),
    ("Action: Sneak", "action:key.sneak"),
    ("Action: Sprint", "action:key.sprint"),
    ("Action: Swap Hands (F)", "action:key.swapOffhand"),
    ("Action: Drop Item (Q)", "action:key.drop"),
    ("Action: Inventory (E)", "action:key.inventory"),
    ("Action: Pick Block", "action:key.pickItem"),
    ("Action: Open Chat (T)", "action:key.chat"),
    ("Action: Command (/)", "action:key.command"),
    ("Action: Player List", "action:key.playerlist"),
    ("Action: Hide HUD (F1)", "action:key.toggleGui"),
    ("Action: Screenshot (F2)", "action:key.screenshot"),
    ("Action: Debug Info (F3)", "action:key.debug.overlay"),
    ("Action: Perspective (F5)", "action:key.togglePerspective"),
    ("Action: Advancements (L)", "action:key.advancements"),
];

fn draw_input_picker_modal(
    ui: &mut crate::ui_theme::Ui<'_>,
    w: f32,
    h: f32,
    step: crate::touch::PickerStep,
    kb_scroll_x: f32,
    for_custom_send: bool,
    mcpe_exclusive: bool,
) {
    use crate::touch::PickerStep;
    use crate::ui_theme::Face;
    let theme = crate::ui_theme::active_theme();
    let bg_card = theme.bg_solid;
    let bg_hi = theme.surface;
    let border_c = theme.border;
    let accent = theme.accent;
    let accent_soft = theme.surface_selected;
    let text_c = theme.text_primary;
    let muted_c = theme.text_muted;
    let danger_c = theme.danger;

    ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 195));
    match step {
        PickerStep::AskInputType { event_slot } => {
            let dw = (w - 36.0).clamp(270.0, 440.0);
            let dh = 236.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx + 2.0, dy + 5.0, dw, dh, 18.0, rgba(0, 0, 0, 150));
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Choose Input Type".to_owned()
            } else if mcpe_exclusive {
                "MCPE Custom Button — Choose Exclusive Binding".to_owned()
            } else {
                format!("Assign Event Slot #{} — Choose Input Type", event_slot + 1)
            };
            let sub_str = if for_custom_send {
                "Tap any physical key, use System Keyboard, or pick from the PC layout below."
            } else if mcpe_exclusive {
                "Exclusive binding: Keyboard OR Mouse (selecting one clears the other)."
            } else {
                "Each GoyControl button can trigger up to 3 Keyboard or Mouse events simultaneously."
            };
            ui.text(
                Face::SemiBold,
                13.5,
                dx + 16.0,
                dy + 12.0,
                text_c,
                &title_str,
            );
            ui.text(Face::Regular, 10.0, dx + 16.0, dy + 31.0, muted_c, sub_str);
            let cw = (dw - 40.0) * 0.5;
            let r_kb = [dx + 16.0, dy + 54.0, cw, 76.0];
            let r_ms = [dx + 24.0 + cw, dy + 54.0, cw, 76.0];
            for (r, title, sub) in [
                (
                    r_kb,
                    "Keyboard Key",
                    if mcpe_exclusive {
                        "Bind 1 key (clears mouse binding)"
                    } else {
                        "Full 104-Key horizontal PC keyboard"
                    },
                ),
                (
                    r_ms,
                    "Mouse & Actions",
                    if mcpe_exclusive {
                        "Bind 1 mouse/action (clears key)"
                    } else {
                        "LMB, RMB, MMB, Scroll, Look & Actions"
                    },
                ),
            ] {
                ui.rounded(r[0], r[1], r[2], r[3], 14.0, accent_soft);
                ui.border(r[0], r[1], r[2], r[3], 14.0, accent);
                ui.centered_fit(
                    Face::SemiBold,
                    13.0,
                    [r[0] + 6.0, r[1] + 14.0, r[2] - 12.0, 24.0],
                    text_c,
                    title,
                );
                ui.centered_fit(
                    Face::Regular,
                    10.0,
                    [r[0] + 6.0, r[1] + 40.0, r[2] - 12.0, 22.0],
                    muted_c,
                    sub,
                );
            }
            let r_sys = [dx + 16.0, dy + 138.0, dw - 32.0, 34.0];
            ui.rounded(r_sys[0], r_sys[1], r_sys[2], r_sys[3], 10.0, bg_hi);
            ui.border(r_sys[0], r_sys[1], r_sys[2], r_sys[3], 10.0, accent);
            ui.centered_fit(
                Face::SemiBold,
                10.8,
                r_sys,
                accent,
                "⌨ Type / Capture Key (System / Physical Keyboard)",
            );
            if for_custom_send {
                let r_can = [dx + 16.0, dy + 180.0, dw - 32.0, 36.0];
                ui.rounded(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, bg_hi);
                ui.border(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, border_c);
                ui.centered_fit(Face::Medium, 11.0, r_can, text_c, "Cancel");
            } else {
                let r_clr = [dx + 16.0, dy + 180.0, cw, 36.0];
                let r_can = [dx + 24.0 + cw, dy + 180.0, cw, 36.0];
                ui.rounded(r_clr[0], r_clr[1], r_clr[2], r_clr[3], 10.0, bg_hi);
                ui.border(r_clr[0], r_clr[1], r_clr[2], r_clr[3], 10.0, danger_c);
                ui.centered_fit(
                    Face::SemiBold,
                    11.0,
                    r_clr,
                    danger_c,
                    "Clear / Unassign Slot",
                );
                ui.rounded(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, bg_hi);
                ui.border(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, border_c);
                ui.centered_fit(Face::Medium, 11.0, r_can, text_c, "Cancel");
            }
        }
        PickerStep::Keyboard { event_slot } => {
            let dw = (w - 20.0).clamp(300.0, 620.0);
            let dh = (h - 20.0).clamp(235.0, 340.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx + 2.0, dy + 5.0, dw, dh, 18.0, rgba(0, 0, 0, 150));
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Full PC Keyboard (Tap or Swipe)".to_owned()
            } else if mcpe_exclusive {
                "MCPE Exclusive Keyboard Binding (Clears Mouse)".to_owned()
            } else {
                format!(
                    "Event Slot #{} — Full PC Keyboard (Tap or Swipe)",
                    event_slot + 1
                )
            };
            ui.text_clipped(
                Face::SemiBold,
                12.5,
                dx + 14.0,
                dy + 11.0,
                (dw - 290.0).max(80.0),
                text_c,
                &title_str,
            );
            let b_ime = [dx + dw - 266.0, dy + 8.0, 64.0, 26.0];
            let b_left = [dx + dw - 196.0, dy + 8.0, 56.0, 26.0];
            let b_right = [dx + dw - 134.0, dy + 8.0, 56.0, 26.0];
            let b_back = [dx + dw - 72.0, dy + 8.0, 58.0, 26.0];
            for (r, lbl) in [
                (b_ime, "⌨ Sys Key"),
                (b_left, "◀ Left"),
                (b_right, "Right ▶"),
                (b_back, "Back"),
            ] {
                ui.rounded(r[0], r[1], r[2], r[3], 8.0, bg_hi);
                ui.border(r[0], r[1], r[2], r[3], 8.0, border_c);
                ui.centered_fit(Face::SemiBold, 10.0, r, text_c, lbl);
            }

            let kb_top = dy + 40.0;
            let kb_h = dh - 56.0;
            let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
            let key_unit = 38.0;
            let gap = 4.0;
            ui.set_clip(Some([dx + 10.0, kb_top, dw - 20.0, kb_h]));
            for (r_idx, row) in KB_ROWS.iter().enumerate() {
                let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                let mut kx = dx + 14.0 - kb_scroll_x;
                for (lbl, _tok, width_mult) in *row {
                    let kw = key_unit * (*width_mult) + (*width_mult - 1.0).max(0.0) * gap;
                    if kx + kw >= dx + 8.0 && kx <= dx + dw - 8.0 {
                        ui.rounded(kx, ry, kw, row_h, 6.0, bg_hi);
                        ui.border(kx, ry, kw, row_h, 6.0, border_c);
                        ui.centered_fit(
                            Face::SemiBold,
                            10.0,
                            [kx + 2.0, ry, kw - 4.0, row_h],
                            text_c,
                            lbl,
                        );
                    }
                    kx += kw + gap;
                }
            }
            ui.set_clip(None);

            let max_kb_scroll = (880.0 - (dw - 28.0)).max(1.0);
            let track_w = dw - 28.0;
            let thumb_w = (track_w * (track_w / 880.0)).clamp(48.0, track_w);
            let thumb_x =
                dx + 14.0 + (track_w - thumb_w) * (kb_scroll_x / max_kb_scroll).clamp(0.0, 1.0);
            ui.rounded(dx + 14.0, dy + dh - 12.0, track_w, 5.0, 2.5, border_c);
            ui.rounded(thumb_x, dy + dh - 12.0, thumb_w, 5.0, 2.5, accent);
        }
        PickerStep::MouseAndActions { event_slot } => {
            let dw = (w - 24.0).clamp(290.0, 560.0);
            let dh = (h - 20.0).clamp(235.0, 355.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Mouse Buttons, Wheel & Actions".to_owned()
            } else if mcpe_exclusive {
                "MCPE Exclusive Mouse/Action Binding (Clears Keyboard Binding)".to_owned()
            } else {
                format!(
                    "Event Slot #{} — Mouse Buttons, Wheel, Look & Actions",
                    event_slot + 1
                )
            };
            ui.text(
                Face::SemiBold,
                13.0,
                dx + 14.0,
                dy + 11.0,
                text_c,
                &title_str,
            );
            let b_back = [dx + dw - 72.0, dy + 8.0, 58.0, 26.0];
            ui.rounded(b_back[0], b_back[1], b_back[2], b_back[3], 8.0, bg_hi);
            ui.border(b_back[0], b_back[1], b_back[2], b_back[3], 8.0, border_c);
            ui.centered_fit(Face::SemiBold, 10.0, b_back, text_c, "Back");

            let cols = 3usize;
            let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
            let gx = dx + 12.0;
            let gy = dy + 40.0;
            let gw = dw - 24.0;
            let gh = dh - 50.0;
            let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
            let ch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
            for (i, (lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                let cx = gx + col as f32 * (cw + 6.0);
                let cy = gy + row as f32 * (ch + 4.0);
                let is_mouse = tok.starts_with("key.mouse.") || tok.starts_with("mouse.");
                ui.rounded(
                    cx,
                    cy,
                    cw,
                    ch,
                    7.0,
                    if is_mouse { accent_soft } else { bg_hi },
                );
                ui.border(
                    cx,
                    cy,
                    cw,
                    ch,
                    7.0,
                    if is_mouse { accent } else { border_c },
                );
                ui.centered_fit(
                    Face::Medium,
                    9.5,
                    [cx + 3.0, cy, cw - 6.0, ch],
                    if is_mouse { accent } else { text_c },
                    lbl,
                );
            }
        }
        PickerStep::None => {}
    }
}

/// Renders the live GoyControl HUD (Analog Joystick + Static Buttons) in-game and in menus
/// using the exact same `Ui::mobile` TrueType & rounded-shape pipeline as `draw_hud_editor`.
pub fn draw_goy_hud(
    tc: &crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    in_menu: bool,
    virtual_mouse: bool,
) {
    use crate::touch::{Control, ControlStyle};
    use crate::ui_theme::{Ui, color};
    crate::touch::set_current_hud_opacity_scale(if tc.editor.is_some() {
        1.0
    } else {
        tc.hud_opacity_scale
    });
    if tc.editor.is_some() || tc.hud_opacity_scale <= 0.001 {
        return;
    }
    let is_goy_system = tc.settings.control_style == ControlStyle::GoyControl;
    let has_goy_move = is_goy_system || tc.placement(Control::Move).use_goy_style;
    let has_goy_custom = is_goy_system
        || (0..tc.settings.custom_buttons.len()).any(|i| {
            tc.custom_visible_for_state(i, in_menu) && tc.custom_placement(i).use_goy_style
        });
    if !has_goy_move && !has_goy_custom {
        return;
    }
    let hud_scale = tc.hud_opacity_scale.clamp(0.0, 1.0);
    let mut ui = Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let text_c = color::hex(0xF4EFFA, 255);
    let state = tc.state();

    // 1. GoyControl Analog Joystick (Control::Move)
    if has_goy_move && tc.visible_for_state(Control::Move, in_menu) {
        let p = tc.placement(Control::Move);
        let mut p_draw = p;
        if tc.settings.mode != crate::touch::Mode::Dpad
            && let Some(anchor) = tc.move_anchor()
        {
            p_draw.center = anchor;
        }
        let [nx, ny, nw, nh] = p_draw.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let alpha = (p.opacity.clamp(0.15, 1.0) * hud_scale).clamp(0.0, 1.0);
        let js = &tc.settings.joystick_style;
        let side = rw.min(rh);
        let jx = rx + (rw - side) * 0.5;
        let jy = ry + (rh - side) * 0.5;
        let base_fill = color::with_alpha(js.base_color, js.base_opacity * alpha);
        let base_stroke = color::with_alpha(js.base_stroke_color, js.base_opacity * alpha);
        let br = (side * (js.base_roundness / 100.0)).clamp(0.0, side * 0.5);
        ui.rounded(jx, jy, side, side, br, base_fill);
        if js.base_stroke > 0.1 {
            ui.stroke_rounded(jx, jy, side, side, br, js.base_stroke, base_stroke);
        }
        let kside = side * js.stick_size.clamp(0.20, 0.85);
        let kx = jx + (side - kside) * 0.5 + state.left[0] * (side * 0.26);
        let ky = jy + (side - kside) * 0.5 - state.left[1] * (side * 0.26);
        let kr = kside * 0.5;
        let stick_fill = color::with_alpha(js.stick_color, js.stick_opacity * alpha);
        let stick_stroke = color::with_alpha(js.stick_stroke_color, js.stick_opacity * alpha);
        ui.rounded(kx, ky, kside, kside, kr, stick_fill);
        if js.stick_stroke > 0.1 {
            ui.stroke_rounded(kx, ky, kside, kside, kr, js.stick_stroke, stick_stroke);
        }
    }

    // 2. Custom GoyControl Static Buttons (MCPE custom buttons are drawn by `TouchControls::draw_custom` via `text_button`)
    for (i, custom) in tc.settings.custom_buttons.iter().enumerate() {
        if !tc.custom_visible_for_state(i, in_menu) {
            continue;
        }
        let p = tc.custom_placement(i);
        if !is_goy_system && !p.use_goy_style {
            continue;
        }
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let alpha = (p.opacity.clamp(0.15, 1.0) * hud_scale).clamp(0.0, 1.0);
        if alpha <= 0.001 {
            continue;
        }
        let label = custom.label();

        let is_active_token = tc.held_custom_tokens().any(|tok| {
            tok == custom.key
                || (!custom.second_key.is_empty() && tok == custom.second_key)
                || (!custom.third_key.is_empty() && tok == custom.third_key)
        });
        let is_vmouse_on = custom.key == "mouse.virtual.toggle" && virtual_mouse;
        let held = is_active_token && !p.toggleable;
        let toggled = (is_active_token && p.toggleable) || is_vmouse_on;

        let raw_fill = if held {
            p.on_click_color
        } else if toggled {
            p.on_toggle_color
        } else {
            p.button_color
        };
        let raw_stroke = if held {
            p.on_click_stroke_color
        } else if toggled {
            p.on_toggle_stroke_color
        } else {
            p.stroke_color
        };
        let fill_c = color::with_alpha(raw_fill, alpha);
        let stroke_c = color::with_alpha(raw_stroke, alpha);
        let shadow_c = color::with_alpha(p.shadow_color, alpha);
        ui.goy_box(
            rx,
            ry,
            rw,
            rh,
            p.shape,
            p.roundness,
            fill_c,
            p.stroke_size,
            p.stroke_margin,
            p.stroke_roundness,
            stroke_c,
            0.0,
            shadow_c,
        );
        let fg_col = color::with_alpha(text_c, (alpha.max(0.75) * hud_scale).clamp(0.0, 1.0));
        draw_button_icon_and_label(&mut ui, [rx, ry, rw, rh], p.icon_id, &label, fg_col);
    }

    ui.finish();
}

fn draw_mcpe_custom_plate(ui: &mut crate::ui_theme::Ui, rect: [f32; 4], alpha: f32, pressed: bool) {
    let [rx, ry, rw, rh] = rect;
    let a_u8 = (alpha * 255.0).round() as u8;
    ui.rounded(rx, ry, rw, rh, 3.0, rgba(18, 20, 24, a_u8));
    let inner_col = if pressed {
        rgba(38, 42, 48, (alpha * 225.0).round() as u8)
    } else {
        rgba(58, 62, 70, (alpha * 210.0).round() as u8)
    };
    ui.rounded(
        rx + 2.0,
        ry + 2.0,
        (rw - 4.0).max(2.0),
        (rh - 4.0).max(2.0),
        2.0,
        inner_col,
    );
    let (top_left_col, bot_right_col) = if pressed {
        (
            rgba(28, 30, 36, (alpha * 215.0).round() as u8),
            rgba(145, 152, 164, (alpha * 185.0).round() as u8),
        )
    } else {
        (
            rgba(175, 182, 194, (alpha * 195.0).round() as u8),
            rgba(28, 30, 36, (alpha * 215.0).round() as u8),
        )
    };
    ui.fill(rx + 2.0, ry + 2.0, (rw - 4.0).max(2.0), 2.0, top_left_col);
    ui.fill(rx + 2.0, ry + 4.0, 2.0, (rh - 6.0).max(2.0), top_left_col);
    ui.fill(
        rx + 2.0,
        ry + rh - 4.0,
        (rw - 4.0).max(2.0),
        2.0,
        bot_right_col,
    );
    ui.fill(
        rx + rw - 4.0,
        ry + 4.0,
        2.0,
        (rh - 6.0).max(2.0),
        bot_right_col,
    );
}

fn hub_metrics(w: f32, h: f32) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32) {
    let dw = (w - 16.0).clamp(352.0, 610.0);
    let dh = (h - 12.0).clamp(264.0, 420.0);
    let dx = (w - dw) * 0.5;
    let dy = (h - dh) * 0.5;
    let side_w = (dw * 0.25).clamp(114.0, 148.0);
    let cx = dx + side_w + 16.0;
    let cw = dw - side_w - 32.0;
    let body_top = dy + 54.0;
    let body_h = (dh - 64.0).max(160.0);
    (dw, dh, dx, dy, side_w, cx, cw, body_top, body_h)
}

impl TouchUi {
    fn step_animations(&self) -> (f32, f32) {
        let open_target = if self.floating_menu { 1.0 } else { 0.0 };
        if crate::ui_theme::motion::game_animations_disabled()
            || crate::ui_theme::motion::reduced_motion()
        {
            self.open_anim.set(open_target);
            self.tab_anim.set(1.0);
            return (open_target, 1.0);
        }
        let now = Instant::now();
        let dt = self
            .last_anim_tick
            .get()
            .map(|prev| now.duration_since(prev).as_secs_f32().clamp(0.0, 0.08))
            .unwrap_or(0.016);
        self.last_anim_tick.set(Some(now));
        let open_target = if self.floating_menu { 1.0 } else { 0.0 };
        let cur_open = self.open_anim.get();
        let next_open = if (cur_open - open_target).abs() < 0.005 {
            open_target
        } else {
            (cur_open + (open_target - cur_open) * (dt * 14.0).clamp(0.15, 1.0)).clamp(0.0, 1.0)
        };
        self.open_anim.set(next_open);

        let cur_tab = self.tab_anim.get();
        let next_tab = if cur_tab >= 0.995 {
            1.0
        } else {
            (cur_tab + (1.0 - cur_tab) * (dt * 16.0).clamp(0.18, 1.0)).clamp(0.0, 1.0)
        };
        self.tab_anim.set(next_tab);
        (next_open, next_tab)
    }

    /// Handles touch events for the draggable Floating Menu Button and the redesigned Horizon Control Hub.
    pub fn floating_menu_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        id: u64,
        phase: TouchPhase,
        p_gui: [f32; 2],
        settings: &mut crate::touch::Settings,
    ) -> Option<InGameMenuAction> {
        use crate::touch::ControlStyle;
        self.goy_mode = settings.control_style == ControlStyle::GoyControl;
        if !settings.show_control_hub {
            self.floating_menu = false;
            self.floating_drag = None;
            return None;
        }
        self.pointer_off = !settings.virtual_mouse_enabled;
        let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
        let (w, h) = (ui.w, ui.h);
        let p = [
            p_gui[0] * ui.batch.scale / ui.density,
            p_gui[1] * ui.batch.scale / ui.density,
        ];
        let inside = |r: [f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };

        let gui_unit = ui.batch.scale / ui.density;
        let (btn_x, btn_y, btn_w, btn_h) = if self.goy_mode {
            let (bw, bh) = (48.0, 42.0);
            (
                (settings.floating_menu_pos[0] * w).clamp(6.0, (w - bw - 6.0).max(6.0)),
                (settings.floating_menu_pos[1] * h).clamp(4.0, (h - bh - 4.0).max(4.0)),
                bw,
                bh,
            )
        } else {
            (
                3.0 * gui_unit,
                3.0 * gui_unit,
                33.0 * gui_unit,
                20.0 * gui_unit,
            )
        };
        let btn_rect = [btn_x, btn_y, btn_w, btn_h];

        if let Some((drag_id, offset, mut moved)) = self.floating_drag {
            if drag_id == id {
                match phase {
                    TouchPhase::Moved => {
                        if !self.goy_mode {
                            return Some(InGameMenuAction::Close);
                        }
                        let nx = (p[0] - offset[0]).clamp(6.0, (w - btn_w - 6.0).max(6.0));
                        let ny = (p[1] - offset[1]).clamp(4.0, (h - btn_h - 4.0).max(4.0));
                        if (nx - btn_x).hypot(ny - btn_y) > 5.0 {
                            moved = true;
                        }
                        if moved {
                            settings.floating_menu_pos = [
                                (nx / w.max(1.0)).clamp(0.03, 0.93),
                                (ny / h.max(1.0)).clamp(0.02, 0.90),
                            ];
                        }
                        self.floating_drag = Some((drag_id, offset, moved));
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.floating_drag = None;
                        if moved {
                            let _ = settings.save();
                        } else if phase == TouchPhase::Ended {
                            self.floating_menu = !self.floating_menu;
                            if self.floating_menu {
                                self.open_anim.set(0.0);
                                self.tab_anim.set(0.0);
                                self.last_anim_tick.set(None);
                                self.hub_scroll = 0.0;
                                self.refresh_presets_cache(settings.control_style);
                            } else {
                                self.presets_modal_open = false;
                                self.preset_dialog = None;
                                self.custom_input_picker = crate::touch::PickerStep::None;
                                self.custom_kb_drag = None;
                                self.sensitivity_dragging = false;
                                self.pointer_size_dragging = false;
                                self.mouse_speed_dragging = false;
                                self.gyro_sensitivity_dragging = false;
                                self.button_scale_dragging = false;
                                self.hub_scroll_drag = None;
                            }
                        }
                        return Some(InGameMenuAction::Close);
                    }
                    _ => {}
                }
            }
        }

        if phase == TouchPhase::Started && inside(btn_rect) {
            self.floating_drag = Some((id, [p[0] - btn_x, p[1] - btn_y], false));
            return Some(InGameMenuAction::Close);
        }

        if !self.floating_menu {
            return None;
        }

        let (dw, dh, dx, dy, side_w, cx, cw, body_top, body_h) = hub_metrics(w, h);
        let content_pane = [cx - 4.0, body_top, cw + 8.0, body_h];

        // Handle ongoing real-time 3D resolution scale slider drag on Tab 0 (Recommended)
        if self.resolution_scale_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (((25.0 + t * 75.0) / 5.0).round() as u32 * 5).clamp(25, 100);
                    return Some(InGameMenuAction::SetResolutionScale(pct));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.resolution_scale_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing sensitivity slider drag on Tab 0 (Recommended) or Tab 1 (Controls)
        if self.sensitivity_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.sensitivity_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing virtual-mouse-speed slider drag on Tab 0 or Tab 2
        if self.mouse_speed_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.mouse_speed_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing pointer-size slider drag on Tab 1
        if self.pointer_size_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.pointer_size_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing gyroscope-sensitivity slider drag on Tab 2
        if self.gyro_sensitivity_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.gyro_sensitivity_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing global-button-scale slider drag on Tab 3
        if self.button_scale_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.button_scale_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // 0. If "Send Custom Input" Picker Modal is open:
        if self.custom_input_picker != crate::touch::PickerStep::None {
            use crate::touch::PickerStep;
            if let Some((drag_id, last_x, mut moved)) = self.custom_kb_drag {
                if drag_id == id {
                    match phase {
                        TouchPhase::Moved => {
                            let dx_k = p[0] - last_x;
                            if dx_k.abs() > 3.5 {
                                moved = true;
                            }
                            let mdw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                            self.custom_kb_scroll_x =
                                (self.custom_kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                            self.custom_kb_drag = Some((drag_id, p[0], moved));
                            return Some(InGameMenuAction::Close);
                        }
                        TouchPhase::Ended => {
                            self.custom_kb_drag = None;
                            if !moved
                                && matches!(self.custom_input_picker, PickerStep::Keyboard { .. })
                            {
                                let mdw = (w - 20.0).clamp(300.0, 620.0);
                                let mdh = (h - 20.0).clamp(235.0, 340.0);
                                let mdx = (w - mdw) * 0.5;
                                let mdy = (h - mdh) * 0.5;
                                let kb_top = mdy + 40.0;
                                let kb_h = mdh - 56.0;
                                let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                                let key_unit = 38.0;
                                let gap = 4.0;
                                for (r_idx, row) in KB_ROWS.iter().enumerate() {
                                    let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                                    let mut kx = mdx + 14.0 - self.custom_kb_scroll_x;
                                    for (_lbl, tok, width_mult) in *row {
                                        let kw = key_unit * (*width_mult)
                                            + (*width_mult - 1.0).max(0.0) * gap;
                                        if inside([kx, ry, kw, row_h])
                                            && p[0] >= mdx + 10.0
                                            && p[0] <= mdx + mdw - 10.0
                                        {
                                            self.custom_input_picker = PickerStep::None;
                                            self.floating_menu = false;
                                            return Some(InGameMenuAction::SendCustomInput(
                                                (*tok).to_owned(),
                                            ));
                                        }
                                        kx += kw + gap;
                                    }
                                }
                            }
                            return Some(InGameMenuAction::Close);
                        }
                        TouchPhase::Cancelled => {
                            self.custom_kb_drag = None;
                            return Some(InGameMenuAction::Close);
                        }
                        _ => {}
                    }
                }
            }
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            match self.custom_input_picker {
                PickerStep::AskInputType { event_slot } => {
                    let mdw = (w - 36.0).clamp(270.0, 440.0);
                    let mdh = 236.0;
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let mcw = (mdw - 40.0) * 0.5;
                    if inside([mdx + 16.0, mdy + 54.0, mcw, 76.0]) {
                        self.custom_input_picker = PickerStep::Keyboard { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 24.0 + mcw, mdy + 54.0, mcw, 76.0]) {
                        self.custom_input_picker = PickerStep::MouseAndActions { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 16.0, mdy + 138.0, mdw - 32.0, 34.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + 16.0, mdy + 180.0, mdw - 32.0, 36.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::None;
                        return Some(InGameMenuAction::Close);
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::Keyboard { event_slot } => {
                    let mdw = (w - 20.0).clamp(300.0, 620.0);
                    let mdh = (h - 20.0).clamp(235.0, 340.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                    if inside([mdx + mdw - 266.0, mdy + 8.0, 64.0, 26.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + mdw - 196.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x - 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 134.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x + 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::AskInputType { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    self.custom_kb_drag = Some((id, p[0], false));
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::MouseAndActions { event_slot } => {
                    let mdw = (w - 24.0).clamp(290.0, 560.0);
                    let mdh = (h - 20.0).clamp(235.0, 355.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::AskInputType { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    let cols = 3usize;
                    let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
                    let gx = mdx + 12.0;
                    let gy = mdy + 40.0;
                    let gw = mdw - 24.0;
                    let gh = mdh - 50.0;
                    let mcw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
                    let mch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
                    for (i, (_lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                        let col = i % cols;
                        let row = i / cols;
                        let icx = gx + col as f32 * (mcw + 6.0);
                        let icy = gy + row as f32 * (mch + 4.0);
                        if inside([icx, icy, mcw, mch]) {
                            self.custom_input_picker = PickerStep::None;
                            self.floating_menu = false;
                            return Some(InGameMenuAction::SendCustomInput((*tok).to_owned()));
                        }
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::None => {}
            }
        }

        // Handle content pane vertical scroll drag
        if let Some((scroll_id, start_y, start_scroll, mut scrolled)) = self.hub_scroll_drag {
            if scroll_id == id {
                match phase {
                    TouchPhase::Moved => {
                        let dy_touch = p[1] - start_y;
                        if dy_touch.abs() > 6.0 {
                            scrolled = true;
                        }
                        if scrolled {
                            let max_s = self.hub_max_scroll.get().max(0.0);
                            self.hub_scroll = (start_scroll - dy_touch).clamp(0.0, max_s);
                        }
                        self.hub_scroll_drag = Some((scroll_id, start_y, start_scroll, scrolled));
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Cancelled => {
                        self.hub_scroll_drag = None;
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Ended => {
                        self.hub_scroll_drag = None;
                        if scrolled {
                            return Some(InGameMenuAction::Close);
                        }
                        // Fall through on tap release if not scrolled so the tapped row activates
                    }
                    _ => {}
                }
            }
        }

        // 1. If Copy / Edit Preset Dialog is open:
        if let Some(mut dlg) = self.preset_dialog.take() {
            if phase != TouchPhase::Started {
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::Close);
            }
            let mdw = (w - 28.0).clamp(310.0, 480.0);
            let mdh = (h - 16.0).clamp(242.0, 330.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            if !inside([mdx, mdy, mdw, mdh]) || inside([mdx + mdw - 76.0, mdy + 10.0, 64.0, 26.0]) {
                return Some(InGameMenuAction::Close);
            }
            let name_r = [mdx + 14.0, mdy + 42.0, (mdw - 36.0) * 0.45, 48.0];
            let desc_r = [
                mdx + 22.0 + (mdw - 36.0) * 0.45,
                mdy + 42.0,
                (mdw - 36.0) * 0.55,
                48.0,
            ];
            if inside(name_r) {
                dlg.editing_desc = false;
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::EditPresetField(false));
            }
            if inside(desc_r) {
                dlg.editing_desc = true;
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::EditPresetField(true));
            }
            let save_r = [mdx + 14.0, mdy + mdh - 38.0, mdw - 28.0, 30.0];
            if inside(save_r) {
                let name = if dlg.name_input.trim().is_empty() {
                    "Custom Preset".to_owned()
                } else {
                    dlg.name_input.trim().to_owned()
                };
                let description = if dlg.desc_input.trim().is_empty() {
                    format!("Custom {} layout", settings.control_style.label())
                } else {
                    dlg.desc_input.trim().to_owned()
                };
                if dlg.is_copy {
                    return Some(InGameMenuAction::CopyPreset {
                        source_id: dlg.target_id,
                        name,
                        description,
                    });
                } else {
                    return Some(InGameMenuAction::UpdatePresetMeta {
                        id: dlg.target_id,
                        name,
                        description,
                    });
                }
            }
            self.preset_dialog = Some(dlg);
            return Some(InGameMenuAction::Close);
        }

        // 2. If Preset Management Modal is open:
        if self.presets_modal_open {
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            self.ensure_presets_cache(settings.control_style);
            let mdw = (w - 24.0).clamp(320.0, 520.0);
            let mdh = (h - 18.0).clamp(248.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            if !inside([mdx, mdy, mdw, mdh]) || inside([mdx + mdw - 78.0, mdy + 10.0, 64.0, 28.0]) {
                self.presets_modal_open = false;
                return Some(InGameMenuAction::Close);
            }
            let list_y = mdy + 48.0;
            let row_h = 46.0;
            let max_rows = ((mdh - 60.0) / (row_h + 6.0)).floor().max(1.0) as usize;
            let presets = self.cached_presets.clone();
            for (idx, preset) in presets.iter().take(max_rows).enumerate() {
                let ry = list_y + idx as f32 * (row_h + 6.0);
                let row_r = [mdx + 14.0, ry, mdw - 28.0, row_h];
                let copy_r = [row_r[0] + row_r[2] - 68.0, ry + 8.0, 60.0, 30.0];
                let edit_r = [row_r[0] + row_r[2] - 104.0, ry + 8.0, 30.0, 30.0];
                if inside(copy_r) {
                    self.preset_dialog = Some(PresetDialogState {
                        is_copy: true,
                        target_id: preset.id.clone(),
                        name_input: format!("{} Copy", preset.name),
                        desc_input: preset.description.clone(),
                        editing_desc: false,
                    });
                    return Some(InGameMenuAction::Close);
                }
                if !preset.is_default && inside(edit_r) {
                    self.preset_dialog = Some(PresetDialogState {
                        is_copy: false,
                        target_id: preset.id.clone(),
                        name_input: preset.name.clone(),
                        desc_input: preset.description.clone(),
                        editing_desc: false,
                    });
                    return Some(InGameMenuAction::Close);
                }
                if inside(row_r) {
                    return Some(InGameMenuAction::SelectPreset(preset.id.clone()));
                }
            }
            return Some(InGameMenuAction::Close);
        }

        // 3. Main Horizon Control Hub Dialog
        if phase == TouchPhase::Started {
            if !inside([dx, dy, dw, dh]) || inside([dx + dw - 76.0, dy + 10.0, 64.0, 30.0]) {
                self.floating_menu = false;
                self.resolution_scale_dragging = false;
                self.sensitivity_dragging = false;
                self.pointer_size_dragging = false;
                self.mouse_speed_dragging = false;
                self.gyro_sensitivity_dragging = false;
                self.button_scale_dragging = false;
                self.custom_input_picker = crate::touch::PickerStep::None;
                self.custom_kb_drag = None;
                self.hub_scroll_drag = None;
                return Some(InGameMenuAction::Close);
            }

            // Left Sidebar Category Tabs (Recommended, Controls, Aiming, Buttons, Advanced)
            let tab_start_y = dy + 54.0;
            for ti in 0..5u8 {
                let ty = tab_start_y + ti as f32 * 39.0;
                if inside([dx + 10.0, ty, side_w - 16.0, 35.0]) {
                    if self.hub_tab != ti {
                        self.hub_tab = ti;
                        self.hub_scroll = 0.0;
                        self.tab_anim.set(0.0);
                        self.last_anim_tick.set(None);
                    }
                    return Some(InGameMenuAction::Close);
                }
            }

            // Check if touch started on sliders or quick chips in Tab 0 (Recommended)
            if self.hub_tab == 0 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let res_card_y = body_y + 18.0;
                let chip_y = res_card_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    if inside([rx, chip_y, chip_w, 26.0]) {
                        return Some(InGameMenuAction::SetResolutionScale(pct));
                    }
                }
                if inside([cx, res_card_y, cw, 47.0]) {
                    self.resolution_scale_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (((25.0 + t * 75.0) / 5.0).round() as u32 * 5).clamp(25, 100);
                    return Some(InGameMenuAction::SetResolutionScale(pct));
                }
                let sens_card_y = body_y + 118.0;
                if inside([cx, sens_card_y, cw, 50.0]) {
                    self.sensitivity_dragging = true;
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                let spd_card_y = body_y + 230.0;
                if inside([cx, spd_card_y, cw, 50.0]) {
                    self.mouse_speed_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
            }

            // Check if touch started on the Sensitivity slider card in Tab 1 (Controls)
            if self.hub_tab == 1 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let sens_card_y = body_y + 148.0;
                let track_r = [cx, sens_card_y, cw, 50.0];
                if inside(track_r) {
                    self.sensitivity_dragging = true;
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
            }

            // Check if touch started on the Virtual Mouse Speed or Pointer Size slider tracks in Tab 2 (Aiming)
            if self.hub_tab == 2 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let spd_card_y = body_y + 180.0;
                if inside([cx, spd_card_y, cw, 50.0]) {
                    self.mouse_speed_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                let size_card_y = body_y + 238.0;
                if inside([cx, size_card_y, cw, 50.0]) {
                    self.pointer_size_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                let gyro_card_y = body_y + 350.0;
                if inside([cx, gyro_card_y, cw, 50.0]) {
                    self.gyro_sensitivity_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
            }

            // Check if touch started on the Global Button Scale slider track in Tab 3 (Buttons)
            if self.hub_tab == 3 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let scale_card_y = body_y + 18.0;
                if inside([cx, scale_card_y, cw, 50.0]) {
                    self.button_scale_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
            }

            // Start scroll-or-tap tracking inside the right content pane
            if inside(content_pane) && self.hub_max_scroll.get() > 2.0 {
                self.hub_scroll_drag = Some((id, p[1], self.hub_scroll, false));
                return Some(InGameMenuAction::Close);
            }
        }

        // Rows activate on release only: pressing and releasing both used to run the row,
        // so switches toggled twice per tap and snapped back as if they had to be held.
        let activate = phase == TouchPhase::Ended && self.hub_scroll_drag.is_none();
        if !activate || !inside(content_pane) {
            return Some(InGameMenuAction::Close);
        }

        let body_y = body_top - self.hub_scroll;
        match self.hub_tab {
            0 => {
                // Tab 0: Recommended (Real-Time Resolution, Fast Sensitivity, Virtual Mouse, Quick Controls & Animations)
                let res_y = body_y + 18.0;
                let chip_y = res_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    if inside([rx, chip_y, chip_w, 24.0]) {
                        return Some(InGameMenuAction::SetResolutionScale(pct));
                    }
                }
                if inside([cx, res_y, cw, 46.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (25.0 + t * 75.0).round() as u32;
                    return Some(InGameMenuAction::SetResolutionScale(pct.clamp(25, 100)));
                }
                let sens_y = body_y + 118.0;
                if inside([cx, sens_y, cw, 50.0]) {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                let vmouse_y = body_y + 176.0;
                if inside([cx, vmouse_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }
                let spd_y = body_y + 230.0;
                if inside([cx, spd_y, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                let act_y = body_y + 288.0;
                let half_w = (cw - 10.0) * 0.5;
                if inside([cx, act_y, half_w, 44.0]) {
                    let next_style = if settings.control_style == ControlStyle::Mcpe {
                        ControlStyle::GoyControl
                    } else {
                        ControlStyle::Mcpe
                    };
                    return Some(InGameMenuAction::SetControlStyle(next_style));
                }
                if inside([cx + half_w + 10.0, act_y, half_w, 44.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenControlEditor);
                }
                let anim_y = body_y + 340.0;
                if inside([cx, anim_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleGameAnimations);
                }
            }
            1 => {
                // Tab 1: Controls
                let mode_y = body_y + 18.0;
                let half_w = (cw - 10.0) * 0.5;
                if inside([cx, mode_y, half_w, 46.0]) {
                    return Some(InGameMenuAction::SetControlStyle(ControlStyle::Mcpe));
                }
                if inside([cx + half_w + 10.0, mode_y, half_w, 46.0]) {
                    return Some(InGameMenuAction::SetControlStyle(ControlStyle::GoyControl));
                }

                // Customize Controls | Control Presets
                let act_y = body_y + 84.0;
                if inside([cx, act_y, half_w, 44.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenControlEditor);
                }
                if inside([cx + half_w + 10.0, act_y, half_w, 44.0]) {
                    self.refresh_presets_cache(settings.control_style);
                    self.presets_modal_open = true;
                    return Some(InGameMenuAction::Close);
                }

                // Sensitivity Slider card (y = body_y + 148.0, h = 50.0)
                let sens_y = body_y + 148.0;
                if inside([cx, sens_y, cw, 50.0]) {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }

                // Authoritative Virtual Mouse Toggle (y = body_y + 206.0, h = 46.0)
                let vmouse_y = body_y + 206.0;
                if inside([cx, vmouse_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }

                // Invert Y-Axis (y = body_y + 260.0, h = 46.0)
                let inv_y = body_y + 260.0;
                if inside([cx, inv_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleInvertY);
                }

                // Split Touch (Aim & Look) (y = body_y + 314.0, h = 46.0)
                let spl_y = body_y + 314.0;
                if inside([cx, spl_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleSplitTouch);
                }
            }
            2 => {
                // Tab 2: Aiming, Camera & Virtual Mouse
                let row_h = 46.0;
                let step = 54.0;
                let r0 = [cx, body_y + 18.0, cw, row_h]; // Movement & Aim Mode
                let r1 = [cx, body_y + 18.0 + step, cw, row_h]; // Joystick Active Zone
                let r2 = [cx, body_y + 18.0 + step * 2.0, cw, row_h]; // Virtual Mouse Toggle
                let spd_card = [cx, body_y + 180.0, cw, 50.0]; // Virtual Mouse Speed slider
                let size_card = [cx, body_y + 238.0, cw, 50.0]; // Pointer Size slider
                let r4 = [cx, body_y + 296.0, cw, row_h]; // Gyroscope Aiming
                let _r5 = [cx, body_y + 350.0, cw, row_h]; // Gyro Sensitivity
                if inside(r0) {
                    return Some(InGameMenuAction::CycleMovementMode);
                }
                if inside(r1) {
                    return Some(InGameMenuAction::CycleJoystickArea);
                }
                if inside(r2) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }
                if inside(spd_card) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                if inside(size_card) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                if inside(r4) {
                    return Some(InGameMenuAction::ToggleGyro);
                }
                if inside([cx, body_y + 350.0, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
            }
            3 => {
                // Tab 3: Buttons & Layout Options
                let row_h = 46.0;
                let step = 54.0;
                let scale_y = body_y + 18.0;
                if inside([cx, scale_y, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
                if inside([cx, body_y + 18.0 + step, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleLeftHanded);
                }
                if inside([cx, body_y + 18.0 + step * 2.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleSprintMode);
                }
                if inside([cx, body_y + 18.0 + step * 3.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleInventoryControls);
                }
            }
            _ => {
                // Tab 3: Advanced & Quick Actions
                let row_h = 46.0;
                let step = 54.0;
                if inside([cx, body_y + 18.0, cw, row_h]) {
                    return Some(if p[0] < cx + cw * 0.5 {
                        InGameMenuAction::ToggleUiVibrate
                    } else {
                        InGameMenuAction::ToggleVibrate
                    });
                }
                if inside([cx, body_y + 18.0 + step, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleAutoJump);
                }
                if inside([cx, body_y + 18.0 + step * 2.0, cw, row_h]) {
                    return Some(InGameMenuAction::PresetVisibilityAll);
                }
                let ky = body_y + 198.0;
                let kw = (cw - 25.0) / 6.0;
                let quick = [
                    KeyCode::F1,
                    KeyCode::F3,
                    KeyCode::F5,
                    KeyCode::KeyE,
                    KeyCode::KeyT,
                ];
                for (i, key) in quick.iter().enumerate() {
                    let rx = cx + i as f32 * (kw + 5.0);
                    if inside([rx, ky, kw, 36.0]) {
                        self.floating_menu = false;
                        return Some(InGameMenuAction::TriggerKey(*key));
                    }
                }
                if inside([cx + 5.0 * (kw + 5.0), ky, kw, 36.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenKeyboard);
                }
                let send_y = body_y + 244.0;
                if inside([cx, send_y, cw, 40.0]) {
                    self.custom_input_picker =
                        crate::touch::PickerStep::AskInputType { event_slot: 0 };
                    return Some(InGameMenuAction::Close);
                }
                let sys_y = body_y + 292.0;
                let half = (cw - 10.0) * 0.5;
                if inside([cx, sys_y, half, 38.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::ForceKill);
                }
                if inside([cx + half + 10.0, sys_y, half, 38.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenLogs);
                }
            }
        }

        Some(InGameMenuAction::Close)
    }

    /// Draws the Floating Menu Button (MCPE stone-slate `H` button vs GoyControl `=` bubble) and
    /// the redesigned Horizon Control Hub with smooth motion, generous spacing, scrollable content pane,
    /// and authoritative Virtual Mouse & gameplay settings.
    pub fn draw_floating_menu(
        &self,
        b: &mut HudBatch,
        a: &HudAtlas,
        settings: &crate::touch::Settings,
    ) {
        use crate::touch::ControlStyle;
        use crate::ui_theme::{Face, Icon, Ui, color};
        if !settings.show_control_hub {
            return;
        }
        let hud_scale = crate::touch::current_hud_opacity_scale();
        if hud_scale <= 0.001 && !self.floating_menu {
            return;
        }
        let mut ui = Ui::mobile(a, b, (0.0, 0.0));
        let (w, h) = (ui.w, ui.h);
        let (open_t, tab_t) = self.step_animations();
        let open_ease = 1.0 - (1.0 - open_t).powi(3);
        let tab_ease = 1.0 - (1.0 - tab_t).powi(3);

        let theme = crate::ui_theme::active_theme();
        let surface_bg = theme.bg_solid;
        let sidebar_bg = theme.sidebar_bg;
        let card_bg = theme.surface;
        let card_sel = theme.surface_selected;
        let border_c = theme.border;
        let accent = theme.accent;
        let accent_hi = theme.accent;
        let accent_fg = theme.accent_text;
        let text_c = theme.text_primary;
        let muted_c = theme.text_muted;
        let danger_c = theme.danger;

        let bubble_alpha = if self.floating_menu {
            1.0
        } else {
            hud_scale.clamp(0.0, 1.0)
        };

        if bubble_alpha > 0.001 {
            let gui_unit = ui.batch.scale / ui.density;
            let (bx, by, bw, bh) = if settings.control_style == ControlStyle::Mcpe {
                (
                    3.0 * gui_unit,
                    3.0 * gui_unit,
                    33.0 * gui_unit,
                    20.0 * gui_unit,
                )
            } else {
                let (bw, bh) = (48.0, 42.0);
                (
                    (settings.floating_menu_pos[0] * w).clamp(6.0, (w - bw - 6.0).max(6.0)),
                    (settings.floating_menu_pos[1] * h).clamp(4.0, (h - bh - 4.0).max(4.0)),
                    bw,
                    bh,
                )
            };
            if settings.control_style == ControlStyle::Mcpe {
                ui.fill(
                    bx,
                    by,
                    bw,
                    bh,
                    color::with_alpha(rgba(0, 0, 0, 120), bubble_alpha),
                );
                ui.icon(
                    Icon::HorizonLogo,
                    bx + bw * 0.25,
                    by + bh * 0.1,
                    bh * 0.8,
                    color::with_alpha(text_c, bubble_alpha),
                );
            } else {
                ui.rounded(
                    bx + 1.5,
                    by + 3.0,
                    bw,
                    bh,
                    14.0,
                    color::with_alpha(rgba(0, 0, 0, 135), bubble_alpha),
                );
                ui.rounded(
                    bx,
                    by,
                    bw,
                    bh,
                    14.0,
                    color::with_alpha(
                        if self.floating_menu { accent } else { card_bg },
                        bubble_alpha,
                    ),
                );
                ui.stroke_rounded(
                    bx,
                    by,
                    bw,
                    bh,
                    14.0,
                    1.6,
                    color::with_alpha(accent, bubble_alpha),
                );
                ui.icon(
                    Icon::MenuEquals,
                    bx + (bw - 22.0) * 0.5,
                    by + (bh - 22.0) * 0.5,
                    22.0,
                    color::with_alpha(
                        if self.floating_menu {
                            accent_fg
                        } else {
                            accent_hi
                        },
                        bubble_alpha,
                    ),
                );
            }
        }

        if !self.floating_menu {
            ui.finish();
            return;
        }

        // Backdrop scrim & Modal Shell with expressive entrance slide
        let scrim_a = (185.0 * open_ease.clamp(0.35, 1.0)).round() as u8;
        ui.fill(0.0, 0.0, w, h, rgba(6, 5, 14, scrim_a));
        let (dw, dh, dx, dy_base, side_w, cx, cw, body_top_base, body_h) = hub_metrics(w, h);
        let slide_y = (1.0 - open_ease) * 18.0;
        let dy = dy_base + slide_y;
        let body_top = body_top_base + slide_y;

        ui.rounded(dx + 2.0, dy + 6.0, dw, dh, 20.0, rgba(0, 0, 0, 170));
        ui.rounded(dx, dy, dw, dh, 20.0, surface_bg);

        // Left Sidebar Background & Vertical Divider
        ui.rounded(dx + 2.0, dy + 48.0, side_w, dh - 50.0, 16.0, sidebar_bg);
        ui.fill(dx + side_w, dy + 48.0, 1.0, dh - 48.0, border_c);
        ui.fill(dx, dy + 47.0, dw, 1.0, border_c);
        ui.stroke_rounded(dx, dy, dw, dh, 20.0, 1.4, border_c);

        // Top Header Bar with Active Control System Mode Badge
        ui.rounded(dx + 14.0, dy + 9.0, 30.0, 30.0, 9.0, card_sel);
        ui.border(dx + 14.0, dy + 9.0, 30.0, 30.0, 9.0, accent);
        ui.icon(
            if settings.control_style == ControlStyle::Mcpe {
                Icon::HorizonLogo
            } else {
                Icon::Settings
            },
            dx + 20.0,
            dy + 15.0,
            18.0,
            accent_hi,
        );
        ui.text(
            Face::SemiBold,
            13.5,
            dx + 52.0,
            dy + 8.0,
            text_c,
            "Horizon Control Hub",
        );
        ui.text(
            Face::Regular,
            9.6,
            dx + 52.0,
            dy + 25.0,
            muted_c,
            "Configure touch controls, virtual mouse & feedback",
        );

        // Active Control System Pill Badge in Header
        let mode_badge_txt = if settings.control_style == ControlStyle::Mcpe {
            "MCPE Controls"
        } else {
            "GoyControl HUD"
        };
        let badge_w = 104.0;
        let badge_r = [dx + dw - 188.0, dy + 11.0, badge_w, 26.0];
        if badge_r[0] > dx + 220.0 {
            ui.rounded(
                badge_r[0], badge_r[1], badge_r[2], badge_r[3], 8.0, card_sel,
            );
            ui.border(badge_r[0], badge_r[1], badge_r[2], badge_r[3], 8.0, accent);
            ui.centered_fit(Face::SemiBold, 9.5, badge_r, accent_hi, mode_badge_txt);
        }

        // "Done" Button in top-right
        let done_r = [dx + dw - 76.0, dy + 10.0, 62.0, 28.0];
        ui.rounded(done_r[0], done_r[1], done_r[2], done_r[3], 9.0, card_bg);
        ui.border(done_r[0], done_r[1], done_r[2], done_r[3], 9.0, border_c);
        ui.centered_fit(Face::SemiBold, 11.0, done_r, text_c, "Done");

        // Left Sidebar Navigation Tabs (Recommended, Controls, Aiming, Buttons, Advanced)
        let tabs: [(Icon, &str); 5] = [
            (Icon::HorizonLogo, "Recommended"),
            (Icon::Gamepad, "Controls"),
            (Icon::Crosshair, "Aiming"),
            (Icon::MicrosoftGrid, "Buttons"),
            (Icon::Flask, "Advanced"),
        ];
        let tab_start_y = dy + 54.0;
        for (ti, (ic, lbl)) in tabs.into_iter().enumerate() {
            let ty = tab_start_y + ti as f32 * 39.0;
            let active = self.hub_tab == ti as u8;
            let tr = [dx + 10.0, ty, side_w - 18.0, 34.0];
            if active {
                ui.rounded(tr[0], tr[1], tr[2], tr[3], 10.0, accent);
                ui.rounded(tr[0] - 4.0, tr[1] + 7.0, 3.0, 20.0, 1.5, accent_hi);
            }
            let fg = if active { text_c } else { muted_c };
            ui.icon(ic, tr[0] + 9.0, tr[1] + 9.0, 15.0, fg);
            ui.text(
                if active { Face::SemiBold } else { Face::Medium },
                10.6,
                tr[0] + 29.0,
                tr[1] + 8.0,
                fg,
                lbl,
            );
        }

        // Scrollable Right Content Panel with scissor clip and tab transition slide
        let tab_slide = (1.0 - tab_ease) * 14.0;
        let content_height = match self.hub_tab {
            0 => 394.0,
            1 => 368.0,
            2 => 410.0,
            3 => 242.0,
            _ => 340.0,
        };
        let max_scroll = (content_height - body_h).max(0.0);
        self.hub_max_scroll.set(max_scroll);
        let cur_scroll = self.hub_scroll.clamp(0.0, max_scroll);
        let body_y = body_top - cur_scroll + tab_slide;

        ui.set_clip(Some([cx - 2.0, body_top, cw + 4.0, body_h]));

        let draw_toggle_row = |ui: &mut Ui<'_>,
                               ry: f32,
                               title: &str,
                               sub: &str,
                               on: Option<bool>,
                               right_badge: Option<&str>| {
            let row_h = 46.0;
            ui.rounded(cx, ry, cw, row_h, 11.0, card_bg);
            ui.border(cx, ry, cw, row_h, 11.0, border_c);
            ui.text(Face::SemiBold, 11.0, cx + 14.0, ry + 6.0, text_c, title);
            ui.text(Face::Regular, 9.2, cx + 14.0, ry + 24.0, muted_c, sub);
            if let Some(state) = on {
                ui.m3_switch(cx + cw - 62.0, ry + 9.0, state);
            } else if let Some(badge) = right_badge {
                let bw = 64.0;
                let bx = cx + cw - bw - 10.0;
                let by = ry + 10.0;
                ui.rounded(bx, by, bw, 26.0, 8.0, card_sel);
                ui.border(bx, by, bw, 26.0, 8.0, accent);
                ui.centered_fit(Face::SemiBold, 10.5, [bx, by, bw, 26.0], accent_hi, badge);
            } else {
                ui.text(
                    Face::SemiBold,
                    14.0,
                    cx + cw - 20.0,
                    ry + 12.0,
                    accent_hi,
                    "›",
                );
            }
        };

        match self.hub_tab {
            0 => {
                // Tab 0: Recommended (Real-Time Resolution, Touch Sensitivity, Virtual Mouse, Quick Controls & Animations)
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "REAL-TIME 3D RESOLUTION & PERFORMANCE",
                );
                let res_y = body_y + 18.0;
                ui.rounded(cx, res_y, cw, 78.0, 11.0, card_bg);
                ui.border(cx, res_y, cw, 78.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    res_y + 6.0,
                    text_c,
                    "Real-Time 3D Resolution Scale (Zero-Lag)",
                );
                let res_pct = settings.resolution_scale_percent.clamp(25, 100);
                let res_txt = format!("{res_pct}%");
                let res_pill_w = 52.0;
                ui.rounded(
                    cx + cw - res_pill_w - 12.0,
                    res_y + 6.0,
                    res_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - res_pill_w - 12.0,
                    res_y + 6.0,
                    res_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - res_pill_w - 12.0, res_y + 6.0, res_pill_w, 18.0],
                    accent_hi,
                    &res_txt,
                );
                let r_sl_x = cx + 14.0;
                let r_sl_w = (cw - 28.0).max(40.0);
                let r_sl_y = res_y + 32.0;
                let r_frac = ((res_pct as f32 - 25.0) / 75.0).clamp(0.0, 1.0);
                ui.rounded(r_sl_x, r_sl_y, r_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(r_sl_x, r_sl_y, (r_sl_w * r_frac).max(6.0), 6.0, 3.0, accent);
                let r_thumb_x = r_sl_x + r_sl_w * r_frac;
                ui.rounded(r_thumb_x - 7.0, r_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(r_thumb_x - 7.0, r_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let chip_y = res_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    let on = (res_pct as i32 - pct as i32).abs() <= 2;
                    ui.rounded(
                        rx,
                        chip_y,
                        chip_w,
                        22.0,
                        7.0,
                        if on { accent } else { sidebar_bg },
                    );
                    if !on {
                        ui.border(rx, chip_y, chip_w, 22.0, 7.0, border_c);
                    }
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [rx + 2.0, chip_y, chip_w - 4.0, 22.0],
                        if on { accent_fg } else { text_c },
                        &format!("{pct}%"),
                    );
                }

                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 102.0,
                    muted_c,
                    "FAST AIM, VIRTUAL MOUSE & CONTROLS",
                );
                let sens_y = body_y + 118.0;
                ui.rounded(cx, sens_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, sens_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    sens_y + 6.0,
                    text_c,
                    "Touch Look Sensitivity",
                );
                let pct_txt = format!("{:.0}%", settings.sensitivity * 100.0);
                let pill_w = 48.0;
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, sens_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &pct_txt,
                );
                let sl_x = cx + 14.0;
                let sl_w = (cw - 28.0).max(40.0);
                let sl_y = sens_y + 33.0;
                let frac = ((settings.sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(sl_x, sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, sl_y, (sl_w * frac).max(6.0), 6.0, 3.0, accent);
                let thumb_x = sl_x + sl_w * frac;
                ui.rounded(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                draw_toggle_row(
                    &mut ui,
                    body_y + 176.0,
                    "Virtual Mouse Pointer",
                    "Off by default; enable for desktop pointer in menus & inventory",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );

                let spd_y = body_y + 230.0;
                ui.rounded(cx, spd_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, spd_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    spd_y + 6.0,
                    text_c,
                    "Virtual Mouse Speed",
                );
                let spd_pct = format!("{:.0}%", settings.virtual_mouse_sensitivity * 100.0);
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    spd_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    spd_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, spd_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &spd_pct,
                );
                let spd_sl_y = spd_y + 33.0;
                let spd_frac = ((settings.virtual_mouse_sensitivity - 0.25) / 2.75).clamp(0.0, 1.0);
                ui.rounded(sl_x, spd_sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, spd_sl_y, (sl_w * spd_frac).max(6.0), 6.0, 3.0, accent);
                let spd_thumb_x = sl_x + sl_w * spd_frac;
                ui.rounded(
                    spd_thumb_x - 7.0,
                    spd_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(spd_thumb_x - 7.0, spd_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let act_y = body_y + 288.0;
                let half_w = (cw - 10.0) * 0.5;
                let sys_sub = format!("Active: {}", settings.control_style.label());
                for (i, (ic, title, sub)) in [
                    (Icon::Gamepad, "Switch Control System", sys_sub.as_str()),
                    (
                        Icon::MicrosoftGrid,
                        "Customize Controls",
                        "Open layout & property studio",
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    ui.rounded(rx, act_y, half_w, 44.0, 11.0, card_bg);
                    ui.border(rx, act_y, half_w, 44.0, 11.0, border_c);
                    ui.rounded(rx + 10.0, act_y + 9.0, 26.0, 26.0, 8.0, card_sel);
                    ui.icon(ic, rx + 15.0, act_y + 14.0, 16.0, accent_hi);
                    ui.text(Face::SemiBold, 10.4, rx + 42.0, act_y + 6.0, text_c, title);
                    ui.text(Face::Regular, 8.6, rx + 42.0, act_y + 23.0, muted_c, sub);
                }

                draw_toggle_row(
                    &mut ui,
                    body_y + 340.0,
                    "In-Game Animations",
                    "Animated block textures, view bobbing, FOV transitions & menu motion",
                    Some(!settings.disable_game_animations),
                    None,
                );
            }
            1 => {
                // 1. Control Mode Section
                ui.text(Face::SemiBold, 10.2, cx, body_y, muted_c, "CONTROL MODE");
                let mode_y = body_y + 18.0;
                let half_w = (cw - 10.0) * 0.5;
                let is_mcpe = settings.control_style == ControlStyle::Mcpe;
                for (i, (title, sub, sel)) in [
                    ("MCPE (Default)", "Vanilla touch controls", is_mcpe),
                    ("GoyControl", "Custom layout & keybinds", !is_mcpe),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    ui.rounded(
                        rx,
                        mode_y,
                        half_w,
                        46.0,
                        11.0,
                        if sel { card_sel } else { card_bg },
                    );
                    ui.stroke_rounded(
                        rx,
                        mode_y,
                        half_w,
                        46.0,
                        11.0,
                        if sel { 1.6 } else { 1.0 },
                        if sel { accent } else { border_c },
                    );
                    let rad_x = rx + 11.0;
                    let rad_y = mode_y + 16.0;
                    ui.stroke_rounded(
                        rad_x,
                        rad_y,
                        14.0,
                        14.0,
                        7.0,
                        1.5,
                        if sel { accent_hi } else { muted_c },
                    );
                    if sel {
                        ui.rounded(rad_x + 3.5, rad_y + 3.5, 7.0, 7.0, 3.5, accent_hi);
                    }
                    ui.text(Face::SemiBold, 11.0, rx + 31.0, mode_y + 7.0, text_c, title);
                    ui.text(Face::Regular, 9.0, rx + 31.0, mode_y + 25.0, muted_c, sub);
                }

                // 2. Dynamic System Controls Section
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 68.0,
                    muted_c,
                    settings.control_style.section_title(),
                );
                let act_y = body_y + 84.0;
                let active_preset_lbl = format!("Preset: {}", settings.preset_name);
                for (i, (ic, title, sub)) in [
                    (
                        Icon::MicrosoftGrid,
                        "Customize Controls",
                        "Open layout editor",
                    ),
                    (
                        Icon::Settings,
                        "Control Presets",
                        active_preset_lbl.as_str(),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    ui.rounded(rx, act_y, half_w, 44.0, 11.0, card_bg);
                    ui.border(rx, act_y, half_w, 44.0, 11.0, border_c);
                    ui.rounded(rx + 10.0, act_y + 9.0, 26.0, 26.0, 8.0, card_sel);
                    ui.icon(ic, rx + 15.0, act_y + 14.0, 16.0, accent_hi);
                    ui.text(Face::SemiBold, 10.6, rx + 42.0, act_y + 6.0, text_c, title);
                    ui.text(Face::Regular, 8.8, rx + 42.0, act_y + 23.0, muted_c, sub);
                    ui.text(
                        Face::SemiBold,
                        13.0,
                        rx + half_w - 14.0,
                        act_y + 12.0,
                        muted_c,
                        "›",
                    );
                }

                // 3. Sensitivity Slider Card
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 132.0,
                    muted_c,
                    "POINTER, CAMERA & AIM",
                );
                let sens_y = body_y + 148.0;
                ui.rounded(cx, sens_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, sens_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    sens_y + 6.0,
                    text_c,
                    "Touch Look Sensitivity",
                );
                let pct_txt = format!("{:.0}%", settings.sensitivity * 100.0);
                let pill_w = 48.0;
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, sens_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &pct_txt,
                );
                let sl_x = cx + 14.0;
                let sl_w = (cw - 28.0).max(40.0);
                let sl_y = sens_y + 33.0;
                let frac = ((settings.sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(sl_x, sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, sl_y, (sl_w * frac).max(6.0), 6.0, 3.0, accent);
                let thumb_x = sl_x + sl_w * frac;
                ui.rounded(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                // 4. Authoritative Virtual Mouse Toggle, Invert Y-Axis & Split Touch
                draw_toggle_row(
                    &mut ui,
                    body_y + 206.0,
                    "Virtual Mouse Pointer",
                    "Windows 11 pointer; auto-shows when MCPE controls are hidden",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 260.0,
                    "Invert Y-Axis",
                    "Reverse vertical camera look direction",
                    Some(settings.invert_y),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 314.0,
                    "Split Touch (Aim & Look)",
                    "Use center crosshair instead of tap-anywhere interaction",
                    Some(settings.crosshair()),
                    None,
                );
            }
            2 => {
                // Tab 2: Aiming, Camera & Virtual Mouse
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "AIMING, VIRTUAL MOUSE & GYROSCOPE",
                );
                let move_lbl = match settings.mode {
                    crate::touch::Mode::Dpad => "D-Pad + Crosshair/Touch",
                    crate::touch::Mode::Crosshair => "Joystick + Crosshair",
                    crate::touch::Mode::Tap => "Joystick + Tap to Interact",
                };
                let area_lbl = settings.joystick_style.move_area.label();
                let gyro_spd_lbl = format!(
                    "Tilt Multiplier ({:.0}%) — Tap left (−) or right (+)",
                    settings.gyro_sensitivity * 100.0
                );
                let step = 54.0;
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0,
                    "Movement & Aim Mode",
                    move_lbl,
                    None,
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Joystick Active Zone",
                    area_lbl,
                    None,
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Virtual Mouse Pointer",
                    "Authoritative toggle for MCPE and GoyControl menus",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );

                // Virtual Mouse Speed slider card (smooth SetVirtualMouseSpeed slider replacing InGameMenuAction::StepVirtualMouseSpeed +/- buttons)
                let spd_y = body_y + 180.0;
                ui.rounded(cx, spd_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, spd_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    spd_y + 6.0,
                    text_c,
                    "Virtual Mouse Speed",
                );
                let spd_pct = format!("{:.0}%", settings.virtual_mouse_sensitivity * 100.0);
                let spd_pill_w = 48.0;
                ui.rounded(
                    cx + cw - spd_pill_w - 12.0,
                    spd_y + 6.0,
                    spd_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - spd_pill_w - 12.0,
                    spd_y + 6.0,
                    spd_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - spd_pill_w - 12.0, spd_y + 6.0, spd_pill_w, 18.0],
                    accent_hi,
                    &spd_pct,
                );
                let spd_sl_x = cx + 14.0;
                let spd_sl_w = (cw - 28.0).max(40.0);
                let spd_sl_y = spd_y + 33.0;
                let spd_frac = ((settings.virtual_mouse_sensitivity - 0.25) / 2.75).clamp(0.0, 1.0);
                ui.rounded(spd_sl_x, spd_sl_y, spd_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    spd_sl_x,
                    spd_sl_y,
                    (spd_sl_w * spd_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let spd_thumb_x = spd_sl_x + spd_sl_w * spd_frac;
                ui.rounded(
                    spd_thumb_x - 7.0,
                    spd_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(spd_thumb_x - 7.0, spd_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                // Pointer Size slider card
                let size_y = body_y + 238.0;
                ui.rounded(cx, size_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, size_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    size_y + 6.0,
                    text_c,
                    "Pointer Size",
                );
                let size_pct = format!("{:.0}%", settings.virtual_mouse_pointer_scale * 100.0);
                let size_pill_w = 48.0;
                ui.rounded(
                    cx + cw - size_pill_w - 12.0,
                    size_y + 6.0,
                    size_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - size_pill_w - 12.0,
                    size_y + 6.0,
                    size_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [
                        cx + cw - size_pill_w - 12.0,
                        size_y + 6.0,
                        size_pill_w,
                        18.0,
                    ],
                    accent_hi,
                    &size_pct,
                );
                let size_sl_x = cx + 14.0;
                let size_sl_w = (cw - 28.0).max(40.0);
                let size_sl_y = size_y + 33.0;
                let size_frac =
                    ((settings.virtual_mouse_pointer_scale - 0.5) / 1.5).clamp(0.0, 1.0);
                ui.rounded(size_sl_x, size_sl_y, size_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    size_sl_x,
                    size_sl_y,
                    (size_sl_w * size_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let size_thumb_x = size_sl_x + size_sl_w * size_frac;
                ui.rounded(
                    size_thumb_x - 7.0,
                    size_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(size_thumb_x - 7.0, size_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                draw_toggle_row(
                    &mut ui,
                    body_y + 296.0,
                    "Gyroscope Aiming",
                    "Tilt device for fine camera and bow aiming",
                    Some(settings.gyroscope),
                    None,
                );
                let _ = gyro_spd_lbl;
                let gyro_y = body_y + 350.0;
                ui.rounded(cx, gyro_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, gyro_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    gyro_y + 6.0,
                    text_c,
                    "Gyroscope Sensitivity",
                );
                let gyro_pct = format!("{:.0}%", settings.gyro_sensitivity * 100.0);
                let gyro_pill_w = 48.0;
                ui.rounded(
                    cx + cw - gyro_pill_w - 12.0,
                    gyro_y + 6.0,
                    gyro_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - gyro_pill_w - 12.0,
                    gyro_y + 6.0,
                    gyro_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [
                        cx + cw - gyro_pill_w - 12.0,
                        gyro_y + 6.0,
                        gyro_pill_w,
                        18.0,
                    ],
                    accent_hi,
                    &gyro_pct,
                );
                let gyro_sl_x = cx + 14.0;
                let gyro_sl_w = (cw - 28.0).max(40.0);
                let gyro_sl_y = gyro_y + 33.0;
                let gyro_frac = ((settings.gyro_sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(gyro_sl_x, gyro_sl_y, gyro_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    gyro_sl_x,
                    gyro_sl_y,
                    (gyro_sl_w * gyro_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let gyro_thumb_x = gyro_sl_x + gyro_sl_w * gyro_frac;
                ui.rounded(
                    gyro_thumb_x - 7.0,
                    gyro_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(gyro_thumb_x - 7.0, gyro_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);
            }
            3 => {
                // Tab 3: Buttons & Layout Options
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "BUTTON SIZING, HANDEDNESS & BEHAVIOR",
                );
                let step = 54.0;
                let scale_y = body_y + 18.0;
                ui.rounded(cx, scale_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, scale_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    scale_y + 6.0,
                    text_c,
                    "Global Button Scale",
                );
                let sc_pct = format!("{:.0}%", settings.button_scale * 100.0);
                let sc_pill_w = 48.0;
                ui.rounded(
                    cx + cw - sc_pill_w - 12.0,
                    scale_y + 6.0,
                    sc_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - sc_pill_w - 12.0,
                    scale_y + 6.0,
                    sc_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - sc_pill_w - 12.0, scale_y + 6.0, sc_pill_w, 18.0],
                    accent_hi,
                    &sc_pct,
                );
                let sc_sl_x = cx + 14.0;
                let sc_sl_w = (cw - 28.0).max(40.0);
                let sc_sl_y = scale_y + 33.0;
                let sc_frac = ((settings.button_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                ui.rounded(sc_sl_x, sc_sl_y, sc_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    sc_sl_x,
                    sc_sl_y,
                    (sc_sl_w * sc_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let sc_thumb_x = sc_sl_x + sc_sl_w * sc_frac;
                ui.rounded(sc_thumb_x - 7.0, sc_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(sc_thumb_x - 7.0, sc_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Left-Handed Layout",
                    "Mirror movement and action buttons horizontally",
                    Some(settings.left_handed),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Toggle Sprint",
                    "Latch sprint on tap instead of holding",
                    Some(settings.sprint_toggle || settings.toggle_sprint),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 3.0,
                    "Inventory Walk Controls",
                    "Keep movement controls active while inventory is open",
                    Some(settings.inventory_controls),
                    None,
                );
            }
            _ => {
                // Tab 3: Advanced & Quick Actions
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "FEEDBACK, GAMEPLAY & SHORTCUTS",
                );
                let step = 54.0;
                let feedback_width = (cw - 8.0) * 0.5;
                ui.rounded(cx, body_y + 18.0, feedback_width, 46.0, 8.0, card_bg);
                ui.rounded(
                    cx + feedback_width + 8.0,
                    body_y + 18.0,
                    feedback_width,
                    46.0,
                    8.0,
                    card_bg,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    11.0,
                    [cx, body_y + 18.0, feedback_width, 46.0],
                    text_c,
                    if settings.ui_vibrate {
                        "UI vibration: ON"
                    } else {
                        "UI vibration: OFF"
                    },
                );
                ui.centered_fit(
                    Face::SemiBold,
                    11.0,
                    [
                        cx + feedback_width + 8.0,
                        body_y + 18.0,
                        feedback_width,
                        46.0,
                    ],
                    text_c,
                    if settings.vibrate {
                        "Gameplay vibration: ON"
                    } else {
                        "Gameplay vibration: OFF"
                    },
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Auto-Jump",
                    "Automatically jump when walking into 1-block ledges",
                    Some(settings.auto_jump),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Apply Standard Menu Visibility",
                    "Reset in-game vs in-menu visibility flags for active system",
                    None,
                    None,
                );
                ui.text(
                    Face::SemiBold,
                    10.0,
                    cx,
                    body_y + 182.0,
                    muted_c,
                    "QUICK DESKTOP KEYS & CUSTOM INPUT",
                );
                let ky = body_y + 198.0;
                let kw = (cw - 25.0) / 6.0;
                for (i, lbl) in ["F1", "F3", "F5", "Inv", "Chat", "Keys"]
                    .into_iter()
                    .enumerate()
                {
                    let rx = cx + i as f32 * (kw + 5.0);
                    ui.rounded(rx, ky, kw, 36.0, 9.0, card_bg);
                    ui.border(rx, ky, kw, 36.0, 9.0, border_c);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.5,
                        [rx + 2.0, ky, kw - 4.0, 36.0],
                        text_c,
                        lbl,
                    );
                }
                let send_y = body_y + 244.0;
                ui.rounded(cx, send_y, cw, 40.0, 10.0, card_sel);
                ui.border(cx, send_y, cw, 40.0, 10.0, accent);
                ui.icon(Icon::Gamepad, cx + 12.0, send_y + 11.0, 18.0, accent_hi);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 38.0,
                    send_y + 5.0,
                    text_c,
                    "Send Custom Input (Keyboard / Mouse / Action)",
                );
                ui.text(
                    Face::Regular,
                    8.8,
                    cx + 38.0,
                    send_y + 21.0,
                    muted_c,
                    "Open 104-key PC keyboard, mouse buttons, scroll wheel & actions",
                );
                ui.text(
                    Face::SemiBold,
                    13.5,
                    cx + cw - 18.0,
                    send_y + 11.0,
                    accent_hi,
                    "›",
                );

                let sys_y = body_y + 292.0;
                let half = (cw - 10.0) * 0.5;
                ui.rounded(
                    cx,
                    sys_y,
                    half,
                    38.0,
                    10.0,
                    color::with_alpha(danger_c, 0.22),
                );
                ui.border(cx, sys_y, half, 38.0, 10.0, danger_c);
                ui.centered_fit(
                    Face::SemiBold,
                    10.8,
                    [cx, sys_y, half, 38.0],
                    danger_c,
                    "Force Kill",
                );
                ui.rounded(cx + half + 10.0, sys_y, half, 38.0, 10.0, card_bg);
                ui.border(cx + half + 10.0, sys_y, half, 38.0, 10.0, border_c);
                ui.centered_fit(
                    Face::SemiBold,
                    10.8,
                    [cx + half + 10.0, sys_y, half, 38.0],
                    text_c,
                    "Open Logs",
                );
            }
        }

        ui.set_clip(None);

        // Draw subtle scrollbar indicator if content overflows
        if max_scroll > 2.0 {
            let track_x = dx + dw - 8.0;
            let track_y = body_top + 4.0;
            let track_h = (body_h - 8.0).max(24.0);
            let thumb_h = (track_h * (body_h / content_height)).clamp(20.0, track_h);
            let thumb_y = track_y + (cur_scroll / max_scroll) * (track_h - thumb_h);
            ui.rounded(track_x, track_y, 3.0, track_h, 1.5, rgba(255, 255, 255, 24));
            ui.rounded(track_x, thumb_y, 3.0, thumb_h, 1.5, accent_hi);
        }

        // Preset Management Sub-Modal (when self.presets_modal_open == true)
        if self.presets_modal_open {
            ui.fill(0.0, 0.0, w, h, rgba(5, 4, 12, 195));
            let mdw = (w - 24.0).clamp(320.0, 520.0);
            let mdh = (h - 18.0).clamp(248.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 18.0, surface_bg);
            ui.stroke_rounded(mdx, mdy, mdw, mdh, 18.0, 1.5, accent);
            ui.text(
                Face::SemiBold,
                13.0,
                mdx + 16.0,
                mdy + 10.0,
                text_c,
                &format!("{} — Control Presets", settings.control_style.label()),
            );
            ui.text(
                Face::Regular,
                9.2,
                mdx + 16.0,
                mdy + 27.0,
                muted_c,
                &format!(
                    "Independent presets stored in controls/{}/",
                    settings.control_style.storage_subdir()
                ),
            );
            let back_r = [mdx + mdw - 78.0, mdy + 10.0, 64.0, 28.0];
            ui.rounded(back_r[0], back_r[1], back_r[2], back_r[3], 9.0, card_bg);
            ui.border(back_r[0], back_r[1], back_r[2], back_r[3], 9.0, border_c);
            ui.centered_fit(Face::SemiBold, 10.5, back_r, text_c, "Back");

            let list_y = mdy + 48.0;
            let row_h = 46.0;
            let max_rows = ((mdh - 60.0) / (row_h + 6.0)).floor().max(1.0) as usize;
            let fallback_presets;
            let presets_slice = if self.cached_presets_style == Some(settings.control_style)
                && !self.cached_presets.is_empty()
            {
                &self.cached_presets
            } else {
                fallback_presets = crate::touch::Settings::list_presets(
                    std::path::Path::new("."),
                    settings.control_style,
                );
                &fallback_presets
            };
            for (idx, preset) in presets_slice.iter().take(max_rows).enumerate() {
                let ry = list_y + idx as f32 * (row_h + 6.0);
                let row_r = [mdx + 14.0, ry, mdw - 28.0, row_h];
                let sel = settings.preset_id.eq_ignore_ascii_case(&preset.id);
                ui.rounded(
                    row_r[0],
                    row_r[1],
                    row_r[2],
                    row_r[3],
                    11.0,
                    if sel { card_sel } else { card_bg },
                );
                ui.stroke_rounded(
                    row_r[0],
                    row_r[1],
                    row_r[2],
                    row_r[3],
                    11.0,
                    if sel { 1.6 } else { 1.0 },
                    if sel { accent } else { border_c },
                );
                let rx = row_r[0] + 12.0;
                let rcy = ry + 16.0;
                ui.stroke_rounded(
                    rx,
                    rcy,
                    14.0,
                    14.0,
                    7.0,
                    1.5,
                    if sel { accent_hi } else { muted_c },
                );
                if sel {
                    ui.rounded(rx + 3.5, rcy + 3.5, 7.0, 7.0, 3.5, accent_hi);
                }
                let title = if preset.is_default {
                    format!("{} (Protected Default)", preset.name)
                } else {
                    preset.name.clone()
                };
                ui.text(
                    Face::SemiBold,
                    11.0,
                    row_r[0] + 34.0,
                    ry + 6.0,
                    text_c,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    9.0,
                    row_r[0] + 34.0,
                    ry + 24.0,
                    muted_c,
                    &preset.description,
                );

                if !preset.is_default {
                    let edit_r = [row_r[0] + row_r[2] - 104.0, ry + 8.0, 30.0, 30.0];
                    ui.rounded(edit_r[0], edit_r[1], edit_r[2], edit_r[3], 8.0, surface_bg);
                    ui.border(edit_r[0], edit_r[1], edit_r[2], edit_r[3], 8.0, border_c);
                    ui.icon(
                        Icon::Pencil,
                        edit_r[0] + 7.0,
                        edit_r[1] + 7.0,
                        16.0,
                        accent_hi,
                    );
                }

                let copy_r = [row_r[0] + row_r[2] - 68.0, ry + 8.0, 60.0, 30.0];
                ui.rounded(copy_r[0], copy_r[1], copy_r[2], copy_r[3], 8.0, card_sel);
                ui.border(copy_r[0], copy_r[1], copy_r[2], copy_r[3], 8.0, accent);
                ui.icon(
                    Icon::Copy,
                    copy_r[0] + 7.0,
                    copy_r[1] + 8.0,
                    14.0,
                    accent_hi,
                );
                ui.text(
                    Face::SemiBold,
                    9.8,
                    copy_r[0] + 24.0,
                    copy_r[1] + 7.0,
                    text_c,
                    "Copy",
                );
            }
        }

        // Copy / Edit Preset Name & Description Dialog
        if let Some(dlg) = &self.preset_dialog {
            ui.fill(0.0, 0.0, w, h, rgba(4, 3, 10, 210));
            let mdw = (w - 28.0).clamp(310.0, 480.0);
            let mdh = (h - 16.0).clamp(242.0, 330.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 18.0, surface_bg);
            ui.stroke_rounded(mdx, mdy, mdw, mdh, 18.0, 1.6, accent);
            ui.text(
                Face::SemiBold,
                13.0,
                mdx + 14.0,
                mdy + 10.0,
                text_c,
                if dlg.is_copy {
                    "Copy Preset Layout — Enter Name & Description"
                } else {
                    "Edit Preset Name & Description"
                },
            );
            let cancel_r = [mdx + mdw - 76.0, mdy + 10.0, 64.0, 26.0];
            ui.rounded(
                cancel_r[0],
                cancel_r[1],
                cancel_r[2],
                cancel_r[3],
                8.0,
                card_bg,
            );
            ui.border(
                cancel_r[0],
                cancel_r[1],
                cancel_r[2],
                cancel_r[3],
                8.0,
                border_c,
            );
            ui.centered_fit(Face::SemiBold, 10.0, cancel_r, text_c, "Cancel");

            let name_r = [mdx + 14.0, mdy + 42.0, (mdw - 36.0) * 0.45, 48.0];
            let desc_r = [
                mdx + 22.0 + (mdw - 36.0) * 0.45,
                mdy + 42.0,
                (mdw - 36.0) * 0.55,
                48.0,
            ];
            ui.text_field(
                name_r[0],
                name_r[1],
                name_r[2],
                name_r[3],
                "PRESET NAME",
                dlg.name_input.as_str(),
                "Tap to type",
                !dlg.editing_desc,
            );
            ui.text_field(
                desc_r[0],
                desc_r[1],
                desc_r[2],
                desc_r[3],
                "DESCRIPTION",
                dlg.desc_input.as_str(),
                "Tap to type",
                dlg.editing_desc,
            );

            ui.hint_line(
                mdx + 14.0,
                mdy + 104.0,
                mdw - 28.0,
                "Tap a field to type; press Enter to close keyboard, then tap Save below.",
                muted_c,
            );

            let save_r = [mdx + 14.0, mdy + mdh - 38.0, mdw - 28.0, 30.0];
            ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 9.0, accent);
            ui.centered_fit(
                Face::SemiBold,
                11.0,
                save_r,
                text_c,
                if dlg.is_copy {
                    "Create Independent Preset Copy"
                } else {
                    "Save Preset Details"
                },
            );
        }

        // Send Custom Input Picker Sub-Modal
        if self.custom_input_picker != crate::touch::PickerStep::None {
            draw_input_picker_modal(
                &mut ui,
                w,
                h,
                self.custom_input_picker,
                self.custom_kb_scroll_x,
                true,
                false,
            );
        }

        ui.finish();
    }
}

fn push_undo_snapshot(ed: &mut crate::touch::Editor, current: &crate::touch::Settings) {
    if ed.undo_stack.len() >= 40 {
        ed.undo_stack.remove(0);
    }
    ed.undo_stack.push(current.clone());
    ed.redo_stack.clear();
}

fn clamp_editor_custom_selection(ed: &mut crate::touch::Editor, settings: &crate::touch::Settings) {
    if let Some(idx) = ed.selected_custom
        && idx >= settings.custom_buttons.len()
    {
        ed.selected_custom = None;
        if ed.selected.is_none() {
            ed.selected = Some(crate::touch::Control::Move);
        }
    }
}

fn shape_name(shape: u8) -> &'static str {
    match shape {
        1 => "Square",
        2 => "Circle/Pill",
        3 => "Cut Octagon",
        4 => "Diamond",
        5 => "Hexagon",
        _ => "Rounded",
    }
}

pub fn control_icon_from_id(icon_id: u8) -> Option<(crate::ui_theme::Icon, &'static str)> {
    use crate::ui_theme::Icon;
    match icon_id {
        1 => Some((Icon::Crosshair, "Crosshair")),
        2 => Some((Icon::Gamepad, "Gamepad")),
        3 => Some((Icon::Play, "Attack")),
        4 => Some((Icon::Pause, "Pause")),
        5 => Some((Icon::Settings, "Settings")),
        6 => Some((Icon::Search, "Zoom")),
        7 => Some((Icon::Check, "Confirm")),
        8 => Some((Icon::Monitor, "Camera")),
        9 => Some((Icon::Flask, "Use/Potion")),
        10 => Some((Icon::Terminal, "Debug/Cmd")),
        11 => Some((Icon::MicrosoftGrid, "Inventory")),
        12 => Some((Icon::Share, "Swap/Drop")),
        _ => None,
    }
}

pub fn control_icon_label(icon_id: u8) -> &'static str {
    control_icon_from_id(icon_id).map_or("Text Only", |(_, name)| name)
}

fn draw_button_icon_and_label(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    icon_id: u8,
    label: &str,
    fg_col: u32,
) {
    let [rx, ry, rw, rh] = rect;
    if let Some((ic, _)) = control_icon_from_id(icon_id) {
        if !label.is_empty() && rw > rh * 1.35 {
            let ic_sz = (rh * 0.44).clamp(11.0, 18.0);
            ui.icon(ic, rx + 6.0, ry + (rh - ic_sz) * 0.5, ic_sz, fg_col);
            ui.centered_fit(
                crate::ui_theme::Face::SemiBold,
                (rh * 0.28).clamp(9.0, 14.0),
                [
                    rx + ic_sz + 8.0,
                    ry + 2.0,
                    (rw - ic_sz - 11.0).max(6.0),
                    (rh - 4.0).max(6.0),
                ],
                fg_col,
                label,
            );
        } else if !label.is_empty() && rh >= 34.0 {
            let ic_sz = (rh * 0.40).clamp(11.0, 18.0);
            ui.icon(
                ic,
                rx + (rw - ic_sz) * 0.5,
                ry + (rh * 0.52 - ic_sz).max(3.0),
                ic_sz,
                fg_col,
            );
            ui.centered_fit(
                crate::ui_theme::Face::SemiBold,
                (rh * 0.22).clamp(8.0, 11.5),
                [rx + 2.0, ry + rh * 0.52, (rw - 4.0).max(6.0), rh * 0.44],
                fg_col,
                label,
            );
        } else {
            let ic_sz = (rh.min(rw) * 0.52).clamp(12.0, 22.0);
            ui.icon(
                ic,
                rx + (rw - ic_sz) * 0.5,
                ry + (rh - ic_sz) * 0.5,
                ic_sz,
                fg_col,
            );
        }
    } else {
        ui.centered_fit(
            crate::ui_theme::Face::SemiBold,
            (rh * 0.30).clamp(9.5, 14.5),
            [rx + 3.0, ry + 2.0, (rw - 6.0).max(6.0), (rh - 4.0).max(6.0)],
            fg_col,
            label,
        );
    }
}

fn draw_preview_brackets(ui: &mut crate::ui_theme::Ui, rect: [f32; 4], col: u32) {
    let [x, y, w, h] = rect;
    let arm = 8.0;
    let t = 1.6;
    // Top-left ⌜
    ui.fill(x + 6.0, y + 6.0, arm, t, col);
    ui.fill(x + 6.0, y + 6.0, t, arm, col);
    // Top-right ⌝
    ui.fill(x + w - 6.0 - arm, y + 6.0, arm, t, col);
    ui.fill(x + w - 6.0 - t, y + 6.0, t, arm, col);
    // Bottom-left ⌞
    ui.fill(x + 6.0, y + h - 6.0 - t, arm, t, col);
    ui.fill(x + 6.0, y + h - 6.0 - arm, t, arm, col);
    // Bottom-right ⌟
    ui.fill(x + w - 6.0 - arm, y + h - 6.0 - t, arm, t, col);
    ui.fill(x + w - 6.0 - t, y + h - 6.0 - arm, t, arm, col);
}

/// Studio Slider Card matching `90511.png`:
/// Renders a rounded dark slate card with top-left icon + title, right value pill badge,
/// and a bottom horizontal cyan slider track (`#1DB4F6`) with a white circular thumb knob.
#[allow(clippy::too_many_arguments)]
fn draw_m3_stepper_cell(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    label: &str,
    fraction: f32,
    accent: u32,
    accent_soft: u32,
    bg_hi: u32,
    _border_c: u32,
    text_c: u32,
) {
    use crate::ui_theme::Face;
    let [rx, ry, cw, ch] = rect;
    let rad = (ch * 0.22).clamp(10.0, 14.0);
    // Clean Material 3 Expressive tonal container (no harsh border stroke)
    ui.rounded(rx, ry, cw, ch, rad, bg_hi);

    let (title_part, val_part) = if let Some((t_str, v_str)) = label.split_once('|') {
        (t_str.trim(), v_str.trim())
    } else if let Some((t_str, v_str)) = label.rsplit_once(' ') {
        (t_str.trim(), v_str.trim())
    } else {
        (label, "")
    };

    let t = fraction.clamp(0.0, 1.0);
    if ch >= 52.0 {
        ui.text_clipped(
            Face::Medium,
            9.8,
            rx + 12.0,
            ry + 8.0,
            (cw - 24.0).max(24.0),
            text_c,
            title_part,
        );

        let pill_w = (ui.measure(Face::SemiBold, 9.4, val_part) + 14.0).clamp(38.0, cw - 20.0);
        let pill_x = rx + cw - pill_w - 10.0;
        let pill_y = ry + 23.0;
        ui.rounded(pill_x, pill_y, pill_w, 18.0, 9.0, accent_soft);
        ui.centered_fit(
            Face::SemiBold,
            9.2,
            [pill_x + 3.0, pill_y, pill_w - 6.0, 18.0],
            accent,
            val_part,
        );

        // Material 3 Expressive 8dp Slider Track + Vertical Pill Handle
        let track_x = rx + 12.0;
        let track_w = (cw - 24.0).max(20.0);
        let track_y = ry + ch - 16.0;
        ui.rounded(track_x, track_y, track_w, 8.0, 4.0, accent_soft);
        let fill_w = (track_w * t).max(8.0);
        ui.rounded(track_x, track_y, fill_w, 8.0, 4.0, accent);
        let thumb_x = (track_x + track_w * t - 3.0).clamp(track_x, track_x + track_w - 6.0);
        ui.rounded(thumb_x, track_y - 3.0, 6.0, 14.0, 3.0, text_c);
    } else {
        let track_x = rx + 10.0;
        let track_w = (cw - 20.0).max(16.0);
        let track_y = ry + ch - 10.0;
        ui.rounded(track_x, track_y, track_w, 6.0, 3.0, accent_soft);
        let fill_w = (track_w * t).max(6.0);
        ui.rounded(track_x, track_y, fill_w, 6.0, 3.0, accent);
        let thumb_x = (track_x + track_w * t - 2.5).clamp(track_x, track_x + track_w - 5.0);
        ui.rounded(thumb_x, track_y - 2.0, 5.0, 10.0, 2.5, text_c);
        ui.text_clipped(
            Face::Medium,
            9.0,
            rx + 10.0,
            ry + 4.0,
            (cw * 0.58).max(20.0),
            text_c,
            title_part,
        );
        ui.text_clipped(
            Face::SemiBold,
            9.0,
            rx + cw - (ui.measure(Face::SemiBold, 9.0, val_part) + 10.0).min(cw * 0.45),
            ry + 4.0,
            (cw * 0.42).max(16.0),
            accent,
            val_part,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_m3_selector_cell(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    title: &str,
    value: &str,
    accent: u32,
    accent_soft: u32,
    bg_hi: u32,
    text_c: u32,
    muted_c: u32,
) {
    use crate::ui_theme::Face;
    let [rx, ry, cw, ch] = rect;
    let rad = (ch * 0.22).clamp(10.0, 14.0);
    ui.rounded(rx, ry, cw, ch, rad, bg_hi);
    ui.text_clipped(
        Face::Medium,
        9.6,
        rx + 12.0,
        ry + 8.0,
        (cw - 24.0).max(24.0),
        muted_c,
        title,
    );
    let pill_r = [rx + 10.0, ry + ch - 34.0, (cw - 20.0).max(24.0), 24.0];
    ui.rounded(
        pill_r[0],
        pill_r[1],
        pill_r[2],
        pill_r[3],
        12.0,
        accent_soft,
    );
    ui.centered_fit(
        Face::SemiBold,
        9.6,
        [pill_r[0] + 6.0, pill_r[1], pill_r[2] - 12.0, pill_r[3]],
        accent,
        &format!("‹  {value}  ›"),
    );
    let _ = text_c;
}

/// Circular Swatch Chip matching `90511.png` with a crisp white outer selection ring.
#[allow(clippy::too_many_arguments)]
fn draw_m3_swatch_chip(
    ui: &mut crate::ui_theme::Ui,
    sx: f32,
    sy: f32,
    sw_w: f32,
    sw_h: f32,
    col: u32,
    selected: bool,
    accent: u32,
    _text_c: u32,
) {
    let d = sw_w.min(sw_h);
    let cx = sx + (sw_w - d) * 0.5;
    let cy = sy + (sw_h - d) * 0.5;
    let rad = d * 0.5;
    if selected {
        ui.rounded(
            cx - 3.0,
            cy - 3.0,
            d + 6.0,
            d + 6.0,
            rad + 3.0,
            crate::ui_theme::color::with_alpha(accent, 0.32),
        );
    }
    ui.rounded(cx, cy, d, d, rad, col);
    ui.border(cx, cy, d, d, rad, rgba(255, 255, 255, 64));
    if selected {
        ui.stroke_rounded(
            cx - 2.0,
            cy - 2.0,
            d + 4.0,
            d + 4.0,
            rad + 2.0,
            2.0,
            rgba(255, 255, 255, 255),
        );
    }
}

pub fn draw_hud_editor(
    tc: &crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    profile_name: &str,
    notice: Option<&str>,
) {
    use crate::touch::{Control, ControlStyle, CustomButton, JoystickMoveArea, PickerStep};
    use crate::ui_theme::{Face, Icon, Ui, color};
    let Some(ed) = &tc.editor else {
        return;
    };
    let mut ui = Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let dp_to_gui = ui.density / ui.batch.scale;
    let gui_to_dp = ui.batch.scale / ui.density;

    let theme = crate::ui_theme::active_theme();
    let bg_card = theme.bg_solid;
    let bg_hi = theme.surface;
    let border_c = theme.border;
    let accent = theme.accent;
    let accent_soft = theme.surface_selected;
    let accent_fg = theme.accent_text;
    let text_c = theme.text_primary;
    let muted_c = theme.text_muted;
    let danger_c = theme.danger;

    // 1. Dark Blueprint Canvas & Alignment Grid
    ui.fill(0.0, 0.0, w, h, rgba(9, 10, 16, 228));
    let step = if tc.settings.snap_to_grid {
        0.025
    } else {
        0.05
    };
    let mut g = step;
    while g < 1.0 {
        ui.fill(w * g, 0.0, 1.0, h, rgba(255, 255, 255, 11));
        ui.fill(0.0, h * g, w, 1.0, rgba(255, 255, 255, 11));
        g += step;
    }
    ui.fill(w * 0.5, 0.0, 1.0, h, rgba(154, 123, 255, 38));
    ui.fill(0.0, h * 0.5, w, 1.0, rgba(154, 123, 255, 38));

    // JoystickMoveArea visual zone indicator when Move is selected
    if ed.selected == Some(Control::Move) && ed.selected_custom.is_none() {
        let zone = match tc.settings.joystick_style.move_area {
            JoystickMoveArea::MidBottomLeft => Some([0.0, h * 0.35, w * 0.45, h * 0.65]),
            JoystickMoveArea::RightSideAnywhere => Some([w * 0.50, 0.0, w * 0.50, h]),
            JoystickMoveArea::FollowFinger => Some([0.0, h * 0.25, w * 0.48, h * 0.75]),
            JoystickMoveArea::AbsoluteFixed => None,
        };
        if let Some([zx, zy, zw, zh]) = zone {
            let zx = if tc.settings.left_handed {
                w - zx - zw
            } else {
                zx
            };
            ui.rounded(
                zx + 2.0,
                zy + 2.0,
                zw - 4.0,
                zh - 4.0,
                12.0,
                rgba(154, 123, 255, 18),
            );
            ui.border(
                zx + 2.0,
                zy + 2.0,
                zw - 4.0,
                zh - 4.0,
                12.0,
                rgba(154, 123, 255, 75),
            );
            ui.text(
                Face::Medium,
                10.0,
                zx + 10.0,
                zy + 8.0,
                accent,
                &format!(
                    "Joystick Active Zone: {}",
                    tc.settings.joystick_style.move_area.label()
                ),
            );
        }
    }

    let _ = (gui_to_dp, muted_c, profile_name);

    // 3. Draw Built-in Controls (MCPE Style or GoyControl Joystick)
    let mut selected_rect_and_title: Option<([f32; 4], String)> = None;
    for c in Control::ALL {
        if tc.settings.control_style == ControlStyle::GoyControl && c != Control::Move {
            continue;
        }
        let vis = tc.visible(c);
        if !vis && !ed.show_hidden {
            continue;
        }
        let p = tc.placement(c);
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let is_sel = ed.selected == Some(c) && ed.selected_custom.is_none();
        let alpha = if vis {
            p.opacity.clamp(0.25, 1.0)
        } else {
            0.35
        };

        if (tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style)
            && c == Control::Move
        {
            // Draw GoyControl Customizable Joystick (Base Area + Inner Stick) identically to in-game
            let js = &tc.settings.joystick_style;
            let side = rw.min(rh);
            let jx = rx + (rw - side) * 0.5;
            let jy = ry + (rh - side) * 0.5;
            let base_fill = color::with_alpha(
                js.base_color,
                js.base_opacity * if vis { 1.0 } else { 0.45 },
            );
            let base_stroke = color::with_alpha(js.base_stroke_color, js.base_opacity);
            let br = (side * (js.base_roundness / 100.0)).clamp(0.0, side * 0.5);
            ui.rounded(jx, jy, side, side, br, base_fill);
            if js.base_stroke > 0.1 {
                ui.stroke_rounded(jx, jy, side, side, br, js.base_stroke, base_stroke);
            }
            let kside = side * js.stick_size.clamp(0.20, 0.85);
            let kx = jx + (side - kside) * 0.5;
            let ky = jy + (side - kside) * 0.5;
            let kr = kside * 0.5;
            let stick_fill = color::with_alpha(js.stick_color, js.stick_opacity);
            let stick_stroke = color::with_alpha(js.stick_stroke_color, js.stick_opacity);
            ui.rounded(kx, ky, kside, kside, kr, stick_fill);
            if js.stick_stroke > 0.1 {
                ui.stroke_rounded(kx, ky, kside, kside, kr, js.stick_stroke, stick_stroke);
            }
        } else if tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style {
            let fill_c = color::with_alpha(p.button_color, alpha);
            let stroke_c = color::with_alpha(p.stroke_color, alpha);
            let shadow_c = color::with_alpha(p.shadow_color, alpha);
            ui.goy_box(
                rx,
                ry,
                rw,
                rh,
                p.shape,
                p.roundness,
                fill_c,
                p.stroke_size,
                p.stroke_margin,
                p.stroke_roundness,
                stroke_c,
                p.shadow_size,
                shadow_c,
            );
            ui.centered_fit(
                Face::SemiBold,
                (rh * 0.28).clamp(9.0, 14.0),
                [rx + 3.0, ry + 2.0, (rw - 6.0).max(6.0), (rh - 4.0).max(6.0)],
                text_c,
                c.name(),
            );
        } else {
            // Authentic MCPE Sprite / Bevel rendering matching in-game MCPE controls!
            let gui_bounds = [
                rx * dp_to_gui,
                ry * dp_to_gui,
                rw * dp_to_gui,
                rh * dp_to_gui,
            ];
            let mcpe_col = rgba(255, 255, 255, (alpha * 255.0) as u8);
            crate::touch::draw_single_mcpe_control(
                tc, ui.batch, ui.atlas, c, gui_bounds, mcpe_col, is_sel,
            );
        }

        if !vis {
            ui.centered_fit(
                Face::SemiBold,
                8.0,
                [rx, ry + rh - 11.0, rw, 10.0],
                color::hex(0xFBBF24, 220),
                "HIDDEN",
            );
        }
        if is_sel {
            selected_rect_and_title = Some(([rx, ry, rw, rh], c.name().to_owned()));
        }
    }

    // 4. Draw Custom Buttons in BOTH MCPE and GoyControl modes!
    for (i, custom) in tc.settings.custom_buttons.iter().enumerate() {
        let p = tc.custom_placement(i);
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let is_sel = ed.selected_custom == Some(i);
        let alpha = p.opacity.clamp(0.25, 1.0);
        let label = custom.label();

        if tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style {
            let fill_c = color::with_alpha(p.button_color, alpha);
            let stroke_c = color::with_alpha(p.stroke_color, alpha);
            let shadow_c = color::with_alpha(p.shadow_color, alpha);
            ui.goy_box(
                rx,
                ry,
                rw,
                rh,
                p.shape,
                p.roundness,
                fill_c,
                p.stroke_size,
                p.stroke_margin,
                p.stroke_roundness,
                stroke_c,
                p.shadow_size,
                shadow_c,
            );
            let fg_col =
                crate::ui_theme::color::with_alpha(text_c, alpha.max(0.75).clamp(0.0, 1.0));
            draw_button_icon_and_label(&mut ui, [rx, ry, rw, rh], p.icon_id, &label, fg_col);
        } else {
            let dp_to_gui = ui.batch.width / w.max(1.0);
            let gui_bounds = [
                rx * dp_to_gui,
                ry * dp_to_gui,
                rw * dp_to_gui,
                rh * dp_to_gui,
            ];
            let mcpe_col = rgba(255, 255, 255, (alpha * 255.0).round() as u8);
            crate::touch::draw_mcpe_custom_button(
                ui.batch, ui.atlas, gui_bounds, &label, false, mcpe_col,
            );
        }

        if is_sel {
            selected_rect_and_title = Some(([rx, ry, rw, rh], format!("Button: {label}")));
        }
    }

    // 5. Draw 8 Interactive Corner & Edge Resize Handles around Selected Component
    if let Some(([rx, ry, rw, rh], sel_name)) = selected_rect_and_title {
        ui.stroke_rounded(rx - 2.0, ry - 2.0, rw + 4.0, rh + 4.0, 6.0, 1.5, accent);
        let handles = [
            (rx - 4.5, ry - 4.5, 9.0, 9.0),
            (rx + rw - 4.5, ry - 4.5, 9.0, 9.0),
            (rx - 4.5, ry + rh - 4.5, 9.0, 9.0),
            (rx + rw - 4.5, ry + rh - 4.5, 9.0, 9.0),
            (rx + rw - 3.5, ry + rh * 0.5 - 7.0, 7.0, 14.0),
            (rx - 3.5, ry + rh * 0.5 - 7.0, 7.0, 14.0),
            (rx + rw * 0.5 - 7.0, ry + rh - 3.5, 14.0, 7.0),
            (rx + rw * 0.5 - 7.0, ry - 3.5, 14.0, 7.0),
        ];
        for (hx, hy, hw, hh) in handles {
            ui.rounded(hx, hy, hw, hh, 3.0, accent);
            ui.border(hx, hy, hw, hh, 3.0, accent_fg);
        }
        let tag = format!("{sel_name} (Drag corners/edges to resize)");
        let tw = (ui.measure(Face::SemiBold, 9.5, &tag) + 12.0).min(w - 16.0);
        let tx = (rx + (rw - tw) * 0.5).clamp(8.0, (w - tw - 8.0).max(8.0));
        let ty = if ry > 24.0 {
            ry - 20.0
        } else {
            (ry + rh + 5.0).min(h - 20.0)
        };
        ui.rounded(tx, ty, tw, 16.0, 5.0, bg_card);
        ui.border(tx, ty, tw, 16.0, 5.0, accent);
        ui.centered_fit(
            Face::SemiBold,
            9.0,
            [tx + 3.0, ty, tw - 6.0, 16.0],
            accent,
            &tag,
        );
    }

    // 6. Larger Material You 3 Moveable Floating Menu Pod (`356x48dp`)
    let pod_w = if ed.pod_expanded { 356.0 } else { 52.0 };
    let pod_h = 48.0;
    let pod_x = (tc.settings.floating_menu_pos[0] * w).clamp(8.0, (w - pod_w - 8.0).max(8.0));
    let pod_y = (tc.settings.floating_menu_pos[1] * h).clamp(6.0, (h - pod_h - 6.0).max(6.0));

    ui.rounded(
        pod_x + 2.0,
        pod_y + 4.0,
        pod_w,
        pod_h,
        24.0,
        rgba(0, 0, 0, 145),
    );
    ui.rounded(pod_x, pod_y, pod_w, pod_h, 24.0, bg_card);
    ui.stroke_rounded(pod_x, pod_y, pod_w, pod_h, 24.0, 1.8, accent);

    // Main draggable M3 hub button on the left of the pod (40x40dp)
    ui.rounded(
        pod_x + 4.0,
        pod_y + 4.0,
        40.0,
        40.0,
        20.0,
        if ed.menu_open { accent } else { accent_soft },
    );
    ui.icon(
        if tc.settings.control_style == ControlStyle::Mcpe {
            Icon::HorizonLogo
        } else {
            Icon::MenuEquals
        },
        pod_x + 13.0,
        pod_y + 13.0,
        22.0,
        if ed.menu_open { accent_fg } else { accent },
    );

    if ed.pod_expanded {
        let items: [(Option<Icon>, &str, f32, bool, bool); 5] = [
            (Some(Icon::Undo), "", 42.0, !ed.undo_stack.is_empty(), false),
            (Some(Icon::Redo), "", 42.0, !ed.redo_stack.is_empty(), false),
            (Some(Icon::Gear), "Props", 72.0, true, ed.props_open),
            (Some(Icon::Plus), "Add", 64.0, true, false),
            (Some(Icon::Check), "Save", 68.0, true, true),
        ];
        let mut ix = pod_x + 50.0;
        for (ic, lbl, iw, enabled, hi) in items {
            let r = [ix, pod_y + 5.0, iw, 38.0];
            let bg = if lbl == "Save" {
                accent
            } else if hi {
                accent_soft
            } else {
                bg_hi
            };
            ui.rounded(r[0], r[1], r[2], r[3], 19.0, bg);
            ui.border(
                r[0],
                r[1],
                r[2],
                r[3],
                19.0,
                if hi || lbl == "Save" {
                    accent
                } else {
                    border_c
                },
            );
            let fg = if lbl == "Save" {
                accent_fg
            } else if !enabled {
                color::hex(0x5E5875, 200)
            } else if hi {
                accent
            } else {
                text_c
            };
            if let Some(icon) = ic {
                if lbl.is_empty() {
                    ui.icon(icon, r[0] + (iw - 18.0) * 0.5, r[1] + 10.0, 18.0, fg);
                } else {
                    ui.icon(icon, r[0] + 10.0, r[1] + 11.0, 16.0, fg);
                    ui.text(Face::SemiBold, 11.5, r[0] + 29.0, r[1] + 10.0, fg, lbl);
                }
            }
            ix += iw + 4.0;
        }
    }

    if let Some(msg) = ed
        .status_msg
        .as_deref()
        .or(notice)
        .filter(|s| !s.is_empty())
    {
        let mw = (ui.measure(Face::Medium, 10.5, msg) + 20.0).min(w - 20.0);
        let mx = (w - mw) * 0.5;
        let my = (pod_y + pod_h + 6.0).min(h - 48.0);
        ui.rounded(mx, my, mw, 22.0, 11.0, bg_card);
        ui.border(mx, my, mw, 22.0, 11.0, border_c);
        ui.centered_fit(
            Face::Medium,
            10.0,
            [mx + 6.0, my, mw - 12.0, 22.0],
            text_c,
            msg,
        );
    }

    // 6b. Unsaved Changes Back Confirmation Modal (when ed.back_confirm_open == true)
    if ed.back_confirm_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 205));
        let dw = (w - 36.0).clamp(290.0, 430.0);
        let dh = 178.0;
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.6, accent);
        ui.text(
            Face::SemiBold,
            14.0,
            dx + 18.0,
            dy + 16.0,
            text_c,
            "Unsaved Layout Changes",
        );
        ui.text(
            Face::Regular,
            10.5,
            dx + 18.0,
            dy + 42.0,
            muted_c,
            "You have modified your control layout in this session.",
        );
        ui.text(
            Face::Regular,
            10.0,
            dx + 18.0,
            dy + 60.0,
            muted_c,
            "Save your changes before leaving, or go back without saving?",
        );
        let bw = (dw - 44.0) * 0.5;
        let by = dy + dh - 50.0;
        let save_r = [dx + 18.0, by, bw, 36.0];
        let back_r = [dx + 26.0 + bw, by, bw, 36.0];
        ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 10.0, accent);
        ui.centered_fit(Face::SemiBold, 11.5, save_r, accent_fg, "Save");
        ui.rounded(back_r[0], back_r[1], back_r[2], back_r[3], 10.0, bg_hi);
        ui.border(back_r[0], back_r[1], back_r[2], back_r[3], 10.0, danger_c);
        ui.centered_fit(Face::SemiBold, 11.5, back_r, danger_c, "Back");
        ui.finish();
        return;
    }

    // 7. Step 1 / Step 2 Mouse vs Keyboard Horizontal Picker Modal
    if ed.picker_step != PickerStep::None {
        let mcpe_excl = tc.settings.control_style == ControlStyle::Mcpe;
        draw_input_picker_modal(
            &mut ui,
            w,
            h,
            ed.picker_step,
            ed.kb_scroll_x,
            false,
            mcpe_excl,
        );
        ui.finish();
        return;
    }

    // 8. Component Property Inspector Dialog — Studio Layout matching `90511.png` (when ed.props_open == true)
    if ed.props_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 7, 15, 208));
        let dw = (w - 16.0).clamp(350.0, 640.0);
        let dh = (h - 12.0).clamp(256.0, 384.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;

        // Material Design 3 Expressive Tonal Surface Palette synchronized with active theme
        let st_outer = theme.bg_solid;
        let st_inner = theme.surface;
        let st_card = theme.surface_bright;
        let st_border = color::with_alpha(theme.border, 0.28);
        let st_cyan = theme.accent;
        let st_cyan_soft = theme.surface_selected;
        let st_cyan_fg = theme.accent_text;
        let st_text = theme.text_primary;
        let st_muted = theme.text_muted;

        ui.rounded(dx + 2.0, dy + 6.0, dw, dh, 24.0, rgba(0, 0, 0, 165));
        ui.rounded(dx, dy, dw, dh, 24.0, st_outer);

        let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
        let is_mcpe_mode = tc.settings.control_style == ControlStyle::Mcpe;
        let (header_txt, sub_txt) = if is_joystick {
            (
                "Joystick & Analog Base Inspector".to_owned(),
                "Customize stick knob, analog base ring, active zone & visibility",
            )
        } else if let Some(idx) = ed.selected_custom {
            let lbl = tc
                .settings
                .custom_buttons
                .get(idx)
                .map(|b| b.label())
                .unwrap_or_else(|| "Button".into());
            if is_mcpe_mode {
                (
                    format!("MCPE Custom Button — {lbl}"),
                    "Exclusive Binding (Keyboard OR Mouse) • Icon, style & geometry",
                )
            } else {
                (
                    format!("GoyControl Button — {lbl}"),
                    "Customize visual style, icon badge, geometry, stroke & events",
                )
            }
        } else if let Some(c) = ed.selected {
            (
                format!("MCPE Component — {}", c.name()),
                "Customize style, icon, opacity, shape, stroke & visibility",
            )
        } else {
            (
                "Component Properties".to_owned(),
                "Configure control appearance & behavior",
            )
        };
        let cur_pl = if let Some(idx) = ed.selected_custom {
            tc.custom_placement(idx)
        } else if let Some(c) = ed.selected {
            tc.placement(c)
        } else {
            crate::touch::Placement::new(0.5, 0.5, 0.12)
        };
        let telem = format!(
            "X:{:.0}% Y:{:.0}%  •  Sz:{:.0}%",
            cur_pl.center[0] * 100.0,
            cur_pl.center[1] * 100.0,
            cur_pl.size * 100.0,
        );

        // Top Header Bar: [X] Close Button + Circular Double-Ring Preview Badge + Title/Sub + Telemetry Pill + [⛶] Center Snap + Cyan [✓ Done]
        let x_btn = [dx + 12.0, dy + 9.0, 26.0, 26.0];
        ui.rounded(x_btn[0], x_btn[1], x_btn[2], x_btn[3], 8.0, st_card);
        ui.border(x_btn[0], x_btn[1], x_btn[2], x_btn[3], 8.0, st_border);
        ui.icon(Icon::Close, x_btn[0] + 6.0, x_btn[1] + 6.0, 14.0, st_muted);

        // Circular double-ring live preview icon in header
        ui.rounded(dx + 44.0, dy + 8.0, 28.0, 28.0, 14.0, rgba(24, 28, 48, 255));
        ui.stroke_rounded(
            dx + 44.0,
            dy + 8.0,
            28.0,
            28.0,
            14.0,
            1.8,
            if is_joystick {
                tc.settings.joystick_style.base_stroke_color
            } else {
                cur_pl.stroke_color
            },
        );
        ui.rounded(
            dx + 51.0,
            dy + 15.0,
            14.0,
            14.0,
            7.0,
            if is_joystick {
                tc.settings.joystick_style.stick_color
            } else {
                cur_pl.button_color
            },
        );
        ui.stroke_rounded(dx + 51.0, dy + 15.0, 14.0, 14.0, 7.0, 1.4, st_cyan);

        ui.text_clipped(
            Face::SemiBold,
            12.8,
            dx + 78.0,
            dy + 7.0,
            (dw - 332.0).max(90.0),
            st_text,
            &header_txt,
        );
        ui.text_clipped(
            Face::Regular,
            8.8,
            dx + 78.0,
            dy + 23.0,
            (dw - 332.0).max(90.0),
            st_muted,
            sub_txt,
        );

        // Telemetry Pill + Center-Snap [⛶] Button + Cyan [✓ Done] Button
        let pill_w = 132.0;
        let pill_r = [dx + dw - 244.0, dy + 9.0, pill_w, 26.0];
        ui.rounded(pill_r[0], pill_r[1], pill_r[2], pill_r[3], 13.0, st_inner);
        ui.border(pill_r[0], pill_r[1], pill_r[2], pill_r[3], 13.0, st_border);
        ui.centered_fit(Face::Mono, 8.8, pill_r, st_text, &telem);

        let snap_r = [dx + dw - 106.0, dy + 9.0, 26.0, 26.0];
        ui.rounded(snap_r[0], snap_r[1], snap_r[2], snap_r[3], 8.0, st_card);
        ui.border(snap_r[0], snap_r[1], snap_r[2], snap_r[3], 8.0, st_border);
        ui.icon(
            Icon::Crosshair,
            snap_r[0] + 5.0,
            snap_r[1] + 5.0,
            16.0,
            st_cyan,
        );

        let close_r = [dx + dw - 74.0, dy + 9.0, 62.0, 26.0];
        ui.rounded(
            close_r[0], close_r[1], close_r[2], close_r[3], 13.0, st_cyan,
        );
        ui.centered_fit(Face::SemiBold, 10.2, close_r, st_cyan_fg, "✓ Done");

        // Full-Width M3 Workspace Area (single clean top segmented tab bar, no duplicate left rail)
        let wx = dx + 14.0;
        let ww = dw - 28.0;

        // Top 3-Segment Pill Bar synchronized with the Left Navigation Rail
        let tab_y = dy + 46.0;
        let tab_w = (ww - 8.0) / 3.0;
        ui.rounded(wx, tab_y - 2.0, ww, 28.0, 14.0, st_inner);
        ui.border(wx, tab_y - 2.0, ww, 28.0, 14.0, st_border);
        let tab_labels = if is_joystick {
            ["1. Stick & Style", "2. Analog Base", "3. Zone & Visibility"]
        } else {
            [
                "1. Style & Colors",
                "2. Shape & Stroke",
                "3. Events & Behavior",
            ]
        };
        for (ti, t_lbl) in tab_labels.into_iter().enumerate() {
            let tx = wx + 2.0 + ti as f32 * (tab_w + 2.0);
            let on = ed.props_tab == ti;
            if on {
                ui.rounded(tx, tab_y, tab_w, 24.0, 12.0, st_cyan);
            }
            ui.centered_fit(
                Face::SemiBold,
                9.8,
                [tx + 4.0, tab_y, tab_w - 8.0, 24.0],
                if on { st_cyan_fg } else { st_muted },
                t_lbl,
            );
        }

        // Main Inner Studio Container Card (`[wx, dy + 80.0, ww, dh - 92.0]`)
        let box_y = dy + 80.0;
        let box_h = dh - 92.0;
        ui.rounded(wx, box_y, ww, box_h, 16.0, st_inner);
        ui.border(wx, box_y, ww, box_h, 16.0, st_border);

        let body_y = box_y + 10.0;
        if is_joystick {
            let js = &tc.settings.joystick_style;
            let p_move = tc.placement(Control::Move);
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            match ed.props_tab {
                0 => {
                    // Section Header + Right Style Selector Pill
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "STICK KNOB DIMENSIONS & STROKE",
                    );
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    let st_pill_r = [wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0];
                    ui.rounded(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let st_lbl = if tc.settings.control_style == ControlStyle::GoyControl
                        || p_move.use_goy_style
                    {
                        "● Style: Custom GoyControl Stick ⌄"
                    } else {
                        "● Style: Default MCPE Stick ⌄"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, st_pill_r, st_cyan, st_lbl);

                    // 3 Studio Slider Cards + Right Live Component Preview Box with Corner Brackets
                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Stick Size|{:.0}%", js.stick_size * 100.0),
                            (js.stick_size - 0.20) / 0.65,
                        ),
                        (
                            format!("Stick Alpha|{:.0}%", js.stick_opacity * 100.0),
                            (js.stick_opacity - 0.15) / 0.85,
                        ),
                        (
                            format!("Stick Stroke|{:.1}dp", js.stick_stroke),
                            js.stick_stroke / 8.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    // Live Joystick Preview Box with Cyan Corner Framing Brackets
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let b_side = 44.0;
                    let bx = prev_r[0] + (prev_r[2] - b_side) * 0.5;
                    let by = prev_r[1] + (prev_r[3] - b_side) * 0.5;
                    let b_rad = (b_side * (js.base_roundness / 100.0)).clamp(4.0, b_side * 0.5);
                    ui.rounded(bx, by, b_side, b_side, b_rad, js.base_color);
                    ui.stroke_rounded(
                        bx,
                        by,
                        b_side,
                        b_side,
                        b_rad,
                        js.base_stroke.clamp(1.0, 3.0),
                        js.base_stroke_color,
                    );
                    let k_side = (b_side * js.stick_size.clamp(0.25, 0.80)).clamp(14.0, 34.0);
                    let kx = bx + (b_side - k_side) * 0.5;
                    let ky = by + (b_side - k_side) * 0.5;
                    ui.rounded(kx, ky, k_side, k_side, k_side * 0.5, js.stick_color);
                    ui.stroke_rounded(
                        kx,
                        ky,
                        k_side,
                        k_side,
                        k_side * 0.5,
                        js.stick_stroke.clamp(1.0, 3.0),
                        js.stick_stroke_color,
                    );

                    // Bottom Swatch Card ("STICK COLOR" + Circular Swatches + Right Cycle Stroke Button)
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "STICK COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            js.stick_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(
                        act_r[0],
                        act_r[1],
                        act_r[2],
                        act_r[3],
                        10.0,
                        1.5,
                        js.stick_stroke_color,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        act_r,
                        st_text,
                        "● Cycle Stick Stroke Col",
                    );
                }
                1 => {
                    // Joystick Tab 1: Analog Base Ring Geometry, Mode & Base Palette
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "ANALOG BASE RING GEOMETRY & INPUT MODE",
                    );
                    let mode_pill_w = 186.0f32.min(ww * 0.42);
                    let mode_pill_r = [
                        wx + ww - mode_pill_w - 12.0,
                        body_y - 2.0,
                        mode_pill_w,
                        24.0,
                    ];
                    ui.rounded(
                        mode_pill_r[0],
                        mode_pill_r[1],
                        mode_pill_r[2],
                        mode_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        mode_pill_r[0],
                        mode_pill_r[1],
                        mode_pill_r[2],
                        mode_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let mode_txt = if tc.settings.mode == crate::touch::Mode::Dpad {
                        "● Mode: D-Pad ⌄"
                    } else {
                        "● Mode: Analog Stick ⌄"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, mode_pill_r, st_cyan, mode_txt);

                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Base Alpha|{:.0}%", js.base_opacity * 100.0),
                            (js.base_opacity - 0.15) / 0.85,
                        ),
                        (
                            format!("Base Stroke|{:.1}dp", js.base_stroke),
                            js.base_stroke / 8.0,
                        ),
                        (
                            format!("Base Round|{:.0}", js.base_roundness),
                            js.base_roundness / 50.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let b_side = 46.0;
                    let bx = prev_r[0] + (prev_r[2] - b_side) * 0.5;
                    let by = prev_r[1] + (prev_r[3] - b_side) * 0.5;
                    let b_rad = (b_side * (js.base_roundness / 100.0)).clamp(2.0, b_side * 0.5);
                    ui.rounded(bx, by, b_side, b_side, b_rad, js.base_color);
                    ui.stroke_rounded(
                        bx,
                        by,
                        b_side,
                        b_side,
                        b_rad,
                        js.base_stroke.clamp(1.0, 3.5),
                        js.base_stroke_color,
                    );

                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "BASE COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            js.base_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(
                        act_r[0],
                        act_r[1],
                        act_r[2],
                        act_r[3],
                        10.0,
                        1.5,
                        js.base_stroke_color,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        act_r,
                        st_text,
                        "● Cycle Base Stroke Col",
                    );
                }
                _ => {
                    // Joystick Tab 2: Active Zone, Footprint Size & Visibility Rules
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "ACTIVE TOUCH ZONE & VISIBILITY RULES",
                    );
                    let area_y = body_y + 26.0;
                    let hw = (ww - 24.0 - 8.0) * 0.5;
                    let area_txt = format!("Active Zone: {}", js.move_area.label());
                    ui.rounded(wx + 12.0, area_y, hw, 38.0, 11.0, st_card);
                    ui.border(wx + 12.0, area_y, hw, 38.0, 11.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.6,
                        [wx + 16.0, area_y, hw - 8.0, 38.0],
                        st_text,
                        &area_txt,
                    );
                    let sz_txt = format!("Joystick Footprint|{:.0}%", p_move.size * 100.0);
                    let sz_x = wx + 12.0 + hw + 8.0;
                    draw_m3_stepper_cell(
                        &mut ui,
                        [sz_x, area_y, hw, 38.0],
                        &sz_txt,
                        (p_move.size - 0.12) / 0.33,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_border,
                        st_text,
                    );

                    let y6 = area_y + 48.0;
                    let third = (ww - 24.0 - 8.0) / 3.0;
                    ui.rounded(
                        wx + 12.0,
                        y6,
                        third,
                        34.0,
                        11.0,
                        if p_move.visible_in_game {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        wx + 12.0,
                        y6,
                        third,
                        34.0,
                        11.0,
                        if p_move.visible_in_game {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        wx + 18.0,
                        y6 + 8.0,
                        18.0,
                        p_move.visible_in_game,
                        "Visible in Game",
                    );

                    let vm_x = wx + 12.0 + third + 4.0;
                    ui.rounded(
                        vm_x,
                        y6,
                        third,
                        34.0,
                        11.0,
                        if p_move.visible_in_menu {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        vm_x,
                        y6,
                        third,
                        34.0,
                        11.0,
                        if p_move.visible_in_menu {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        y6 + 8.0,
                        18.0,
                        p_move.visible_in_menu,
                        "Visible in Menu",
                    );

                    let pv_x = wx + 12.0 + (third + 4.0) * 2.0;
                    ui.rounded(pv_x, y6, third, 34.0, 11.0, st_cyan_soft);
                    ui.border(pv_x, y6, third, 34.0, 11.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.5,
                        [pv_x + 4.0, y6, third - 8.0, 34.0],
                        st_cyan,
                        "Preset Vis for All",
                    );
                }
            }
        } else {
            let p = if let Some(idx) = ed.selected_custom {
                tc.custom_placement(idx)
            } else if let Some(c) = ed.selected {
                tc.placement(c)
            } else {
                crate::touch::Placement::new(0.5, 0.5, 0.12)
            };
            let cur_hint = if let Some(idx) = ed.selected_custom {
                tc.settings
                    .custom_buttons
                    .get(idx)
                    .map(|b| b.label())
                    .unwrap_or_default()
            } else if let Some(c) = ed.selected {
                c.name().to_owned()
            } else {
                String::new()
            };
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            match ed.props_tab {
                0 => {
                    // Button Tab 0: Studio Style Header + 3 Cards (Transparency, Hint Text, Icon Badge) + Right Live Preview + Bottom Color Swatch Bar
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "BUTTON STYLE, ICON BADGE & OPACITY",
                    );
                    let is_goy =
                        tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style;
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    let st_pill_r = [wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0];
                    ui.rounded(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let st_lbl = if is_goy {
                        "● Style: Static (GoyControl) ⌄"
                    } else {
                        "● Style: Default (MCPE) ⌄"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, st_pill_r, st_cyan, st_lbl);

                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    let icon_lbl = control_icon_label(p.icon_id);
                    draw_m3_stepper_cell(
                        &mut ui,
                        [wx + 12.0, cards_y, cw3, card_h],
                        &format!("Opacity|{:.0}%", p.opacity * 100.0),
                        (p.opacity - 0.15) / 0.85,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_border,
                        st_text,
                    );
                    draw_m3_selector_cell(
                        &mut ui,
                        [wx + 12.0 + (cw3 + 8.0), cards_y, cw3, card_h],
                        "Icon Badge",
                        icon_lbl,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_text,
                        st_muted,
                    );
                    draw_m3_selector_cell(
                        &mut ui,
                        [wx + 12.0 + (cw3 + 8.0) * 2.0, cards_y, cw3, card_h],
                        "Hint Label",
                        &cur_hint,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_text,
                        st_muted,
                    );

                    // Live Button Preview Box with Cyan Corner Framing Brackets (`⌜ ⌝ ⌞ ⌟`)
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let bw_p = (38.0 * p.width_ratio.clamp(0.6, 1.45)).clamp(26.0, 56.0);
                    let bh_p = (38.0 * p.height_ratio.clamp(0.6, 1.35)).clamp(24.0, 48.0);
                    let bx_p = prev_r[0] + (prev_r[2] - bw_p) * 0.5;
                    let by_p = prev_r[1] + (prev_r[3] - bh_p) * 0.5;
                    ui.goy_box(
                        bx_p,
                        by_p,
                        bw_p,
                        bh_p,
                        p.shape,
                        p.roundness,
                        color::with_alpha(p.button_color, p.opacity.clamp(0.35, 1.0)),
                        p.stroke_size.clamp(1.0, 3.5),
                        0.0,
                        p.stroke_roundness,
                        p.stroke_color,
                        2.0,
                        p.shadow_color,
                    );
                    draw_button_icon_and_label(
                        &mut ui,
                        [bx_p, by_p, bw_p, bh_p],
                        p.icon_id,
                        &cur_hint,
                        st_text,
                    );

                    // Bottom Swatch Card ("BUTTON COLOR" + Circular Swatches + onClick/onToggle Cycle Pills)
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "BUTTON COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            p.button_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 158.0f32.min(ww * 0.36);
                    let act_x = wx + ww - act_w - 20.0;
                    let half_h = (sw_card_h - 14.0) * 0.5;
                    let clk_r = [act_x, sw_y + 5.0, act_w, half_h];
                    let tog_r = [act_x, sw_y + 9.0 + half_h, act_w, half_h];
                    ui.rounded(
                        clk_r[0],
                        clk_r[1],
                        clk_r[2],
                        clk_r[3],
                        7.0,
                        p.on_click_color,
                    );
                    ui.stroke_rounded(
                        clk_r[0],
                        clk_r[1],
                        clk_r[2],
                        clk_r[3],
                        7.0,
                        1.3,
                        p.on_click_stroke_color,
                    );
                    ui.centered_fit(Face::SemiBold, 8.2, clk_r, st_text, "onClick Col (Cycle)");
                    ui.rounded(
                        tog_r[0],
                        tog_r[1],
                        tog_r[2],
                        tog_r[3],
                        7.0,
                        p.on_toggle_color,
                    );
                    ui.stroke_rounded(
                        tog_r[0],
                        tog_r[1],
                        tog_r[2],
                        tog_r[3],
                        7.0,
                        1.3,
                        p.on_toggle_stroke_color,
                    );
                    ui.centered_fit(Face::SemiBold, 8.2, tog_r, st_text, "onToggle Col (Cycle)");
                }
                1 => {
                    // Button Tab 1: Shape, Stroke, Roundness & Aspect Ratios + Live Preview + Stroke Color Swatches
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "SHAPE GEOMETRY, STROKE & ASPECT RATIO",
                    );
                    let sh_pill_w = 192.0f32.min(ww * 0.44);
                    let sh_pill_r = [wx + ww - sh_pill_w - 12.0, body_y - 2.0, sh_pill_w, 24.0];
                    ui.rounded(
                        sh_pill_r[0],
                        sh_pill_r[1],
                        sh_pill_r[2],
                        sh_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        sh_pill_r[0],
                        sh_pill_r[1],
                        sh_pill_r[2],
                        sh_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        sh_pill_r,
                        st_cyan,
                        &format!("● Shape: {} ⌄", shape_name(p.shape)),
                    );

                    let cards_y = body_y + 28.0;
                    let card_h = 35.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Stroke|{:.1}dp", p.stroke_size),
                            p.stroke_size / 8.0,
                        ),
                        (format!("Roundness|{:.0}", p.roundness), p.roundness / 50.0),
                        (
                            format!("Shadow|{:.1}dp", p.shadow_size),
                            p.shadow_size / 12.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }
                    let row2_y = cards_y + 39.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Scale|{:.0}%", p.size * 100.0),
                            (p.size - 0.05) / 0.39,
                        ),
                        (
                            format!("Width|{:.0}%", p.width_ratio * 100.0),
                            (p.width_ratio - 0.50) / 1.50,
                        ),
                        (
                            format!("Height|{:.0}%", p.height_ratio * 100.0),
                            (p.height_ratio - 0.50) / 1.50,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, row2_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    // Live Shape & Stroke Preview Box
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, 74.0];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let bw_p = (38.0 * p.width_ratio.clamp(0.6, 1.45)).clamp(26.0, 56.0);
                    let bh_p = (38.0 * p.height_ratio.clamp(0.6, 1.35)).clamp(24.0, 48.0);
                    let bx_p = prev_r[0] + (prev_r[2] - bw_p) * 0.5;
                    let by_p = prev_r[1] + (prev_r[3] - bh_p) * 0.5;
                    ui.goy_box(
                        bx_p,
                        by_p,
                        bw_p,
                        bh_p,
                        p.shape,
                        p.roundness,
                        color::with_alpha(p.button_color, p.opacity.clamp(0.35, 1.0)),
                        p.stroke_size.clamp(1.0, 3.5),
                        0.0,
                        p.stroke_roundness,
                        p.stroke_color,
                        2.0,
                        p.shadow_color,
                    );
                    draw_button_icon_and_label(
                        &mut ui,
                        [bx_p, by_p, bw_p, bh_p],
                        p.icon_id,
                        &cur_hint,
                        st_text,
                    );

                    // Bottom Stroke Color Swatch Strip
                    let sw_y = cards_y + 74.0 + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "STROKE COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            p.stroke_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, 1.5, st_cyan);
                    ui.centered_fit(Face::SemiBold, 8.8, act_r, st_text, "Reset Shape & Scale");
                }
                _ => {
                    // Button Tab 2: Events, Behavior Toggles & Visibility Checkboxes
                    if is_mcpe_mode && ed.selected_custom.is_some() {
                        let cur_tok = ed
                            .selected_custom
                            .and_then(|idx| tc.settings.custom_buttons.get(idx))
                            .map(|b| b.key.as_str())
                            .unwrap_or("");
                        let is_kb = CustomButton::is_keyboard_binding(cur_tok);
                        let is_ms = CustomButton::is_mouse_binding(cur_tok);
                        let kb_lbl = if is_kb {
                            format!(
                                "Active: {}",
                                crate::touch::custom::friendly_token_name(cur_tok)
                            )
                        } else {
                            "Inactive (Cleared)".to_owned()
                        };
                        let ms_lbl = if is_ms {
                            format!(
                                "Active: {}",
                                crate::touch::custom::friendly_token_name(cur_tok)
                            )
                        } else {
                            "Inactive (Cleared)".to_owned()
                        };
                        let half_w = (ww - 24.0 - 8.0) * 0.5;
                        for (ci, (title, status, active)) in [
                            ("KEYBOARD BINDING (EXCLUSIVE)", kb_lbl.as_str(), is_kb),
                            ("MOUSE / ACTION BINDING (EXCLUSIVE)", ms_lbl.as_str(), is_ms),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let ex = wx + 12.0 + ci as f32 * (half_w + 8.0);
                            ui.rounded(
                                ex,
                                body_y,
                                half_w,
                                42.0,
                                12.0,
                                if active { st_cyan_soft } else { st_card },
                            );
                            ui.border(
                                ex,
                                body_y,
                                half_w,
                                42.0,
                                12.0,
                                if active { st_cyan } else { st_border },
                            );
                            ui.centered_fit(
                                Face::SemiBold,
                                8.8,
                                [ex + 4.0, body_y + 4.0, half_w - 8.0, 16.0],
                                if active { st_cyan } else { st_muted },
                                title,
                            );
                            ui.centered_fit(
                                Face::SemiBold,
                                10.2,
                                [ex + 4.0, body_y + 20.0, half_w - 8.0, 18.0],
                                if active { st_text } else { st_muted },
                                status,
                            );
                        }
                    } else {
                        let ew = (ww - 24.0 - 8.0) / 3.0;
                        for slot in 0..3 {
                            let ex = wx + 12.0 + slot as f32 * (ew + 4.0);
                            let tok_name = if let Some(idx) = ed.selected_custom {
                                tc.settings
                                    .custom_buttons
                                    .get(idx)
                                    .map(|b| {
                                        crate::touch::custom::friendly_token_name(
                                            b.event_slot(slot),
                                        )
                                    })
                                    .unwrap_or_else(|| "Unassigned".into())
                            } else if let Some(c) = ed.selected {
                                if slot == 0 {
                                    c.name().to_owned()
                                } else {
                                    "Built-in".to_owned()
                                }
                            } else {
                                "Unassigned".into()
                            };
                            let has_binding = tok_name != "Unassigned" && tok_name != "None";
                            ui.rounded(
                                ex,
                                body_y,
                                ew,
                                42.0,
                                12.0,
                                if has_binding { st_cyan_soft } else { st_card },
                            );
                            ui.border(
                                ex,
                                body_y,
                                ew,
                                42.0,
                                12.0,
                                if has_binding { st_cyan } else { st_border },
                            );
                            ui.centered_fit(
                                Face::SemiBold,
                                8.8,
                                [ex + 4.0, body_y + 4.0, ew - 8.0, 16.0],
                                if has_binding { st_cyan } else { st_muted },
                                &format!("EVENT SLOT #{}", slot + 1),
                            );
                            ui.centered_fit(
                                Face::SemiBold,
                                10.2,
                                [ex + 4.0, body_y + 20.0, ew - 8.0, 18.0],
                                st_text,
                                &tok_name,
                            );
                        }
                    }

                    let by = body_y + 52.0;
                    let bw = (ww - 24.0 - 12.0) / 4.0;
                    let is_vis = if let Some(c) = ed.selected {
                        tc.visible(c)
                    } else {
                        true
                    };
                    for (i, (lbl, on, danger)) in [
                        (
                            if p.passthrough {
                                "Pass-Thru: ON"
                            } else {
                                "Pass-Thru: OFF"
                            },
                            p.passthrough,
                            false,
                        ),
                        (
                            if p.toggleable {
                                "Mode: Toggle"
                            } else {
                                "Mode: Hold"
                            },
                            p.toggleable,
                            false,
                        ),
                        (
                            if p.swipeable {
                                "Swipe: ON"
                            } else {
                                "Swipe: OFF"
                            },
                            p.swipeable,
                            false,
                        ),
                        (
                            if ed.selected_custom.is_some() {
                                "Delete Button"
                            } else if is_vis {
                                "Visible: YES"
                            } else {
                                "Visible: NO"
                            },
                            is_vis,
                            ed.selected_custom.is_some(),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (bw + 4.0);
                        ui.rounded(
                            rx,
                            by,
                            bw,
                            30.0,
                            10.0,
                            if danger {
                                color::with_alpha(danger_c, 0.24)
                            } else if on {
                                st_cyan_soft
                            } else {
                                st_card
                            },
                        );
                        ui.border(
                            rx,
                            by,
                            bw,
                            30.0,
                            10.0,
                            if danger {
                                danger_c
                            } else if on {
                                st_cyan
                            } else {
                                st_border
                            },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            9.4,
                            [rx + 3.0, by, bw - 6.0, 30.0],
                            if danger {
                                danger_c
                            } else if on {
                                st_cyan
                            } else {
                                st_text
                            },
                            lbl,
                        );
                    }

                    let vy = by + 38.0;
                    let third = (ww - 24.0 - 8.0) / 3.0;
                    ui.rounded(
                        wx + 12.0,
                        vy,
                        third,
                        28.0,
                        10.0,
                        if p.visible_in_game {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        wx + 12.0,
                        vy,
                        third,
                        28.0,
                        10.0,
                        if p.visible_in_game {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        wx + 18.0,
                        vy + 5.0,
                        18.0,
                        p.visible_in_game,
                        "Visible in Game",
                    );

                    let vm_x = wx + 12.0 + third + 4.0;
                    ui.rounded(
                        vm_x,
                        vy,
                        third,
                        28.0,
                        10.0,
                        if p.visible_in_menu {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        vm_x,
                        vy,
                        third,
                        28.0,
                        10.0,
                        if p.visible_in_menu {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        vy + 5.0,
                        18.0,
                        p.visible_in_menu,
                        "Visible in Menu",
                    );

                    let pv_x = wx + 12.0 + (third + 4.0) * 2.0;
                    ui.rounded(pv_x, vy, third, 28.0, 10.0, st_cyan_soft);
                    ui.border(pv_x, vy, third, 28.0, 10.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [pv_x + 4.0, vy, third - 8.0, 28.0],
                        st_cyan,
                        "Preset Vis for All",
                    );
                }
            }
        }
        ui.finish();
        return;
    }

    // 8b. Independent Control Editor Save Workflow Dialog (Option A: Replace / Option B: Create New)
    if ed.save_dialog_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 195));
        let dw = (w - 24.0).clamp(310.0, 520.0);
        let dh = (h - 14.0).clamp(250.0, 360.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.border(dx, dy, dw, dh, 18.0, accent);

        ui.text(
            Face::SemiBold,
            13.0,
            dx + 14.0,
            dy + 10.0,
            text_c,
            &format!(
                "Save {} Layout — {}",
                tc.settings.control_style.label(),
                tc.settings.preset_name
            ),
        );

        let discard_r = [dx + dw - 166.0, dy + 8.0, 84.0, 24.0];
        ui.rounded(
            discard_r[0],
            discard_r[1],
            discard_r[2],
            discard_r[3],
            7.0,
            bg_hi,
        );
        ui.border(
            discard_r[0],
            discard_r[1],
            discard_r[2],
            discard_r[3],
            7.0,
            danger_c,
        );
        ui.centered_fit(Face::SemiBold, 9.5, discard_r, danger_c, "Discard & Exit");

        let cancel_r = [dx + dw - 76.0, dy + 8.0, 62.0, 24.0];
        ui.rounded(
            cancel_r[0],
            cancel_r[1],
            cancel_r[2],
            cancel_r[3],
            7.0,
            bg_hi,
        );
        ui.border(
            cancel_r[0],
            cancel_r[1],
            cancel_r[2],
            cancel_r[3],
            7.0,
            border_c,
        );
        ui.centered_fit(Face::SemiBold, 9.5, cancel_r, text_c, "Cancel");

        // Option A: Replace existing preset (disabled/protected when Default is active)
        let is_def = tc.settings.is_default_preset();
        let opt_a_r = [dx + 14.0, dy + 38.0, dw - 28.0, 44.0];
        ui.rounded(
            opt_a_r[0],
            opt_a_r[1],
            opt_a_r[2],
            opt_a_r[3],
            10.0,
            if is_def {
                rgba(24, 21, 36, 180)
            } else {
                accent_soft
            },
        );
        ui.border(
            opt_a_r[0],
            opt_a_r[1],
            opt_a_r[2],
            opt_a_r[3],
            10.0,
            if is_def { border_c } else { accent },
        );
        ui.text(
            Face::SemiBold,
            10.5,
            opt_a_r[0] + 10.0,
            opt_a_r[1] + 6.0,
            if is_def { muted_c } else { text_c },
            if is_def {
                "Option A: Replace existing preset (Protected)"
            } else {
                "Option A: Replace existing preset"
            },
        );
        let opt_a_sub = if is_def {
            "Default layout cannot be overwritten. Please create a new preset below.".to_owned()
        } else {
            format!(
                "Update '{}' in controls/{}/",
                tc.settings.preset_name,
                tc.settings.control_style.storage_subdir()
            )
        };
        ui.text(
            Face::Regular,
            9.0,
            opt_a_r[0] + 10.0,
            opt_a_r[1] + 23.0,
            muted_c,
            &opt_a_sub,
        );
        let rep_btn_r = [dx + dw - 118.0, dy + 46.0, 96.0, 28.0];
        ui.rounded(
            rep_btn_r[0],
            rep_btn_r[1],
            rep_btn_r[2],
            rep_btn_r[3],
            8.0,
            if is_def { bg_hi } else { accent },
        );
        ui.centered_fit(
            Face::SemiBold,
            9.5,
            rep_btn_r,
            if is_def { muted_c } else { accent_fg },
            if is_def {
                "Protected"
            } else {
                "Replace & Save"
            },
        );

        // Option B: Create a new preset as...
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            dy + 86.0,
            accent,
            "OPTION B: CREATE A NEW PRESET AS...",
        );
        let iy = dy + 100.0;
        let nw = (dw - 28.0 - 8.0) * 0.44;
        let desc_w = (dw - 28.0 - 8.0) - nw;
        let name_box = [dx + 14.0, iy, nw, 46.0];
        let desc_box = [dx + 14.0 + nw + 8.0, iy, desc_w, 46.0];
        let name_focus = !ed.editing_save_desc;
        ui.text_field(
            name_box[0],
            name_box[1],
            name_box[2],
            name_box[3],
            "NEW PRESET NAME",
            ed.save_name_input.as_str(),
            "Tap to type",
            name_focus,
        );
        ui.text_field(
            desc_box[0],
            desc_box[1],
            desc_box[2],
            desc_box[3],
            "DESCRIPTION",
            ed.save_desc_input.as_str(),
            "Tap to type",
            !name_focus,
        );
        ui.hint_line(
            dx + 14.0,
            iy + 56.0,
            dw - 28.0,
            "Tap Name or Desc to type; press Enter, then tap Create New Preset.",
            muted_c,
        );

        let create_btn_r = [dx + 14.0, dy + dh - 34.0, dw - 28.0, 26.0];
        ui.rounded(
            create_btn_r[0],
            create_btn_r[1],
            create_btn_r[2],
            create_btn_r[3],
            8.0,
            accent,
        );
        ui.centered_fit(
            Face::SemiBold,
            10.5,
            create_btn_r,
            accent_fg,
            "Create New Preset & Save",
        );

        ui.finish();
        return;
    }

    // 9. Main Floating Control Manager Dialog (when ed.menu_open == true)
    if ed.menu_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 175));
        let dw = (w - 24.0).clamp(300.0, 540.0);
        let dh = (h - 18.0).clamp(245.0, 350.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.border(dx, dy, dw, dh, 18.0, border_c);
        ui.rounded(dx + 12.0, dy + 10.0, 26.0, 26.0, 8.0, accent_soft);
        ui.icon(
            if tc.settings.control_style == ControlStyle::Mcpe {
                Icon::HorizonLogo
            } else {
                Icon::MenuEquals
            },
            dx + 16.0,
            dy + 14.0,
            18.0,
            accent,
        );
        ui.text(
            Face::SemiBold,
            13.0,
            dx + 44.0,
            dy + 13.0,
            text_c,
            &format!(
                "{} Editor — Preset: {}",
                tc.settings.control_style.label(),
                tc.settings.preset_name
            ),
        );

        let resume_r = [dx + dw - 76.0, dy + 10.0, 62.0, 26.0];
        ui.rounded(
            resume_r[0],
            resume_r[1],
            resume_r[2],
            resume_r[3],
            8.0,
            bg_hi,
        );
        ui.border(
            resume_r[0],
            resume_r[1],
            resume_r[2],
            resume_r[3],
            8.0,
            border_c,
        );
        ui.centered_fit(Face::SemiBold, 10.5, resume_r, accent, "Resume");

        // Row 1: Save Workflow & Reset to Default for Active System
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            dy + 42.0,
            accent,
            &format!(
                "1. {} PRESET ACTIONS (STORAGE: controls/{}/)",
                tc.settings.control_style.section_title().to_uppercase(),
                tc.settings.control_style.storage_subdir()
            ),
        );
        let ey = dy + 56.0;
        let ew = (dw - 28.0 - 8.0) * 0.5;
        for (i, (title, primary)) in [
            ("Save Preset (Replace / Save As...)", true),
            ("Reset Canvas to Default Layout", false),
        ]
        .into_iter()
        .enumerate()
        {
            let rx = dx + 14.0 + i as f32 * (ew + 8.0);
            ui.rounded(rx, ey, ew, 30.0, 8.0, if primary { accent } else { bg_hi });
            ui.border(
                rx,
                ey,
                ew,
                30.0,
                8.0,
                if primary { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                10.0,
                [rx + 4.0, ey, ew - 8.0, 30.0],
                if primary { accent_fg } else { text_c },
                title,
            );
        }

        // Row 2: Presets belonging to this control system ONLY
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            ey + 36.0,
            accent,
            &format!(
                "2. LOAD {} PRESET",
                tc.settings.control_style.label().to_uppercase()
            ),
        );
        let py = ey + 50.0;
        let presets = crate::touch::Settings::list_presets(
            std::path::Path::new("."),
            tc.settings.control_style,
        );
        let max_show = presets.len().clamp(1, 5);
        let pw = (dw - 28.0 - 4.0 * (max_show as f32 - 1.0)) / max_show as f32;
        for (i, meta) in presets.iter().take(max_show).enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            let active = tc.settings.preset_id.eq_ignore_ascii_case(&meta.id);
            ui.rounded(
                rx,
                py,
                pw,
                28.0,
                7.0,
                if active { accent_soft } else { bg_hi },
            );
            ui.border(
                rx,
                py,
                pw,
                28.0,
                7.0,
                if active { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                9.5,
                [rx + 3.0, py, pw - 6.0, 28.0],
                if active { accent } else { text_c },
                &meta.name,
            );
        }

        // Row 3: Global Editor & Movement Options
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            py + 36.0,
            accent,
            "3. MOVEMENT, JOYSTICK AREA & GRID SNAP",
        );
        let gy = py + 50.0;
        let gw = (dw - 28.0 - 12.0) / 4.0;
        let move_lbl = match tc.settings.mode {
            crate::touch::Mode::Dpad => "Move: D-Pad",
            crate::touch::Mode::Crosshair => "Move: Joystick",
            crate::touch::Mode::Tap => "Move: Tap Stick",
        };
        let area_lbl = match tc.settings.joystick_style.move_area {
            JoystickMoveArea::MidBottomLeft => "Area: Mid-Left",
            JoystickMoveArea::RightSideAnywhere => "Area: Right Side",
            JoystickMoveArea::AbsoluteFixed => "Area: Fixed",
            JoystickMoveArea::FollowFinger => "Area: Follow",
        };
        for (i, (lbl, on)) in [
            (move_lbl, true),
            (area_lbl, true),
            (
                if tc.settings.snap_to_grid {
                    "Grid Snap: ON"
                } else {
                    "Grid Snap: OFF"
                },
                tc.settings.snap_to_grid,
            ),
            (
                if ed.show_hidden {
                    "Hidden: SHOWN"
                } else {
                    "Hidden: HIDE"
                },
                ed.show_hidden,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let rx = dx + 14.0 + i as f32 * (gw + 4.0);
            ui.rounded(rx, gy, gw, 28.0, 7.0, if on { accent_soft } else { bg_hi });
            ui.border(rx, gy, gw, 28.0, 7.0, if on { accent } else { border_c });
            ui.centered_fit(
                Face::Medium,
                9.5,
                [rx + 3.0, gy, gw - 6.0, 28.0],
                if on { accent } else { text_c },
                lbl,
            );
        }
    }

    ui.finish();
}

/// Handles touch input for the MCPE + GoyControl Editor (`draw_hud_editor`).
/// Returns `true` when the user completes saving or discarding to exit the editor.
fn apply_props_slider(
    tc: &mut crate::touch::TouchControls,
    ed: &crate::touch::Editor,
    tab: usize,
    index: usize,
    t: f32,
) {
    use crate::touch::Control;
    let t = t.clamp(0.0, 1.0);
    let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
    let mode_idx = tc.settings.mode.index();
    if is_joystick {
        let slot = tc.settings.slot(Control::Move);
        match (tab, index) {
            (0, 0) => tc.settings.joystick_style.stick_size = (0.20 + t * 0.65).clamp(0.20, 0.85),
            (0, 1) => tc.settings.joystick_style.stick_opacity = (0.15 + t * 0.85).clamp(0.15, 1.0),
            (0, _) => {
                tc.settings.joystick_style.stick_stroke = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0)
            }
            (1, 0) => tc.settings.joystick_style.base_opacity = (0.15 + t * 0.85).clamp(0.15, 1.0),
            (1, 1) => {
                tc.settings.joystick_style.base_stroke = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0)
            }
            (1, _) => {
                tc.settings.joystick_style.base_roundness = (t * 50.0).round().clamp(0.0, 50.0)
            }
            (_, _) => {
                tc.settings.layouts[mode_idx][slot].size = (0.12 + t * 0.33).clamp(0.12, 0.45)
            }
        }
        return;
    }
    let apply_pl = |pl: &mut crate::touch::Placement| match (tab, index) {
        (0, 0) => pl.opacity = (0.15 + t * 0.85).clamp(0.15, 1.0),
        (1, 0) => pl.stroke_size = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0),
        (1, 1) => {
            pl.roundness = (t * 50.0).round().clamp(0.0, 50.0);
            pl.stroke_roundness = pl.roundness;
        }
        (1, 2) => pl.shadow_size = (t * 12.0).round().clamp(0.0, 12.0),
        (1, 3) => pl.size = (0.05 + t * 0.39).clamp(0.05, 0.44),
        (1, 4) => pl.width_ratio = (0.50 + t * 1.50).clamp(0.50, 2.00),
        (1, 5) => pl.height_ratio = (0.50 + t * 1.50).clamp(0.50, 2.00),
        _ => {}
    };
    if let Some(idx) = ed.selected_custom {
        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
            for m in 0..3 {
                apply_pl(&mut btn.layouts[m]);
            }
        }
    } else if let Some(c) = ed.selected {
        let slot = tc.settings.slot(c);
        apply_pl(&mut tc.settings.layouts[mode_idx][slot]);
    }
}

pub fn touch_hud_editor(
    tc: &mut crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    id: u64,
    point: [f32; 2],
    started: bool,
    ended: bool,
) -> bool {
    use crate::touch::{Control, ControlStyle, CustomButton, EditorDragKind, Mode, PickerStep};
    let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let p = [point[0] * w, point[1] * h];
    let inside =
        |r: [f32; 4]| p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3];

    let Some(mut ed) = tc.editor.take() else {
        return false;
    };
    clamp_editor_custom_selection(&mut ed, &tc.settings);

    let closed = (|| -> bool {
        // 1. Handle ongoing drag / resize / floating-pod move / horizontal keyboard scroll
        if !started {
            if let Some((drag_id, kind)) = ed.active_drag {
                if drag_id == id {
                    match kind {
                        EditorDragKind::FloatingPod { offset, mut moved } => {
                            let pod_w = if ed.pod_expanded { 356.0 } else { 52.0 };
                            let cur_x = (tc.settings.floating_menu_pos[0] * w)
                                .clamp(8.0, (w - pod_w - 8.0).max(8.0));
                            let cur_y = (tc.settings.floating_menu_pos[1] * h)
                                .clamp(6.0, (h - 54.0).max(6.0));
                            let nx = (p[0] - offset[0]).clamp(8.0, (w - pod_w - 8.0).max(8.0));
                            let ny = (p[1] - offset[1]).clamp(6.0, (h - 54.0).max(6.0));
                            if (nx - cur_x).hypot(ny - cur_y) > 4.0 {
                                moved = true;
                            }
                            if moved {
                                tc.settings.floating_menu_pos = [
                                    (nx / w.max(1.0)).clamp(0.04, 0.92),
                                    (ny / h.max(1.0)).clamp(0.02, 0.90),
                                ];
                            }
                            if ended {
                                ed.active_drag = None;
                                if !moved {
                                    ed.menu_open = !ed.menu_open;
                                    ed.props_open = false;
                                }
                                let _ = tc.settings.save();
                            } else {
                                ed.active_drag =
                                    Some((drag_id, EditorDragKind::FloatingPod { offset, moved }));
                            }
                        }
                        EditorDragKind::KeyboardScroll { last_x, mut moved } => {
                            let dx_k = p[0] - last_x;
                            if dx_k.abs() > 3.5 {
                                moved = true;
                            }
                            let dw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (dw - 28.0)).max(0.0);
                            ed.kb_scroll_x = (ed.kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                            if ended {
                                ed.active_drag = None;
                                if !moved
                                    && let PickerStep::Keyboard { event_slot } = ed.picker_step
                                {
                                    let is_mcpe = tc.settings.control_style == ControlStyle::Mcpe;
                                    let dh = (h - 20.0).clamp(235.0, 340.0);
                                    let dx = (w - dw) * 0.5;
                                    let dy = (h - dh) * 0.5;
                                    let kb_top = dy + 40.0;
                                    let kb_h = dh - 56.0;
                                    let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                                    let key_unit = 38.0;
                                    let gap = 4.0;
                                    for (r_idx, row) in KB_ROWS.iter().enumerate() {
                                        let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                                        let mut kx = dx + 14.0 - ed.kb_scroll_x;
                                        for (lbl, tok, width_mult) in *row {
                                            let kw = key_unit * (*width_mult)
                                                + (*width_mult - 1.0).max(0.0) * gap;
                                            if inside([kx, ry, kw, row_h])
                                                && p[0] >= dx + 10.0
                                                && p[0] <= dx + dw - 10.0
                                            {
                                                push_undo_snapshot(&mut ed, &tc.settings);
                                                if let Some(idx) = ed.selected_custom
                                                    && let Some(btn) =
                                                        tc.settings.custom_buttons.get_mut(idx)
                                                {
                                                    if is_mcpe {
                                                        btn.set_exclusive_binding(
                                                            (*tok).to_owned(),
                                                        );
                                                        btn.custom_label = (*lbl).to_owned();
                                                    } else {
                                                        btn.set_event_slot(
                                                            event_slot,
                                                            (*tok).to_owned(),
                                                        );
                                                        if event_slot == 0
                                                            && (btn.custom_label.is_empty()
                                                                || btn.custom_label == "BTN")
                                                        {
                                                            btn.custom_label = (*lbl).to_owned();
                                                        }
                                                    }
                                                    let _ = tc.settings.save();
                                                }
                                                ed.picker_step = PickerStep::None;
                                                ed.status_msg = Some(if is_mcpe {
                                                    format!(
                                                        "Bound key '{lbl}' (exclusive — mouse binding cleared)."
                                                    )
                                                } else {
                                                    format!(
                                                        "Assigned key '{lbl}' to Event Slot #{}.",
                                                        event_slot + 1
                                                    )
                                                });
                                                tc.clicked = true;
                                                break;
                                            }
                                            kx += kw + gap;
                                        }
                                    }
                                }
                            } else {
                                ed.active_drag = Some((
                                    drag_id,
                                    EditorDragKind::KeyboardScroll {
                                        last_x: p[0],
                                        moved,
                                    },
                                ));
                            }
                        }
                        EditorDragKind::Move { offset } => {
                            let aspect = tc.aspect();
                            let mut center = [point[0] - offset[0], point[1] - offset[1]];
                            if tc.settings.left_handed {
                                center[0] = 1.0 - center[0];
                            }
                            if tc.settings.snap_to_grid {
                                center = center.map(|v| (v * 40.0).round() / 40.0);
                            }
                            let mode_idx = tc.settings.mode.index();
                            if let Some(idx) = ed.selected_custom {
                                if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                    let pl = &mut btn.layouts[mode_idx];
                                    let [_, _, rw, rh] = pl.rect(aspect);
                                    pl.center = [
                                        center[0].clamp(rw * 0.5, (1.0 - rw * 0.5).max(rw * 0.5)),
                                        center[1].clamp(rh * 0.5, (1.0 - rh * 0.5).max(rh * 0.5)),
                                    ];
                                }
                            } else if let Some(c) = ed.selected {
                                let slot = tc.settings.slot(c);
                                let pl = &mut tc.settings.layouts[mode_idx][slot];
                                let [_, _, rw, rh] = pl.rect(aspect);
                                pl.center = [
                                    center[0].clamp(rw * 0.5, (1.0 - rw * 0.5).max(rw * 0.5)),
                                    center[1].clamp(rh * 0.5, (1.0 - rh * 0.5).max(rh * 0.5)),
                                ];
                            }
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                        EditorDragKind::Resize {
                            mask,
                            start_pos,
                            start_size,
                            start_wr,
                            start_hr,
                        } => {
                            let dx = point[0] - start_pos[0];
                            let dy = point[1] - start_pos[1];
                            let mode_idx = tc.settings.mode.index();
                            let apply_resize = |pl: &mut crate::touch::Placement| {
                                if mask == 1 || mask == 2 {
                                    // Horizontal right/left edge handle: adjust width_ratio
                                    let dir = if mask == 1 { 1.0 } else { -1.0 };
                                    pl.width_ratio = (start_wr + dir * dx * 8.5).clamp(0.45, 3.0);
                                } else if mask == 4 || mask == 8 {
                                    // Vertical bottom/top edge handle: adjust height_ratio
                                    let dir = if mask == 4 { 1.0 } else { -1.0 };
                                    pl.height_ratio = (start_hr + dir * dy * 8.5).clamp(0.45, 3.0);
                                } else {
                                    // Corner handle: scale overall size + aspect ratio
                                    let sx = if (mask & 1) != 0 { dx } else { -dx };
                                    let sy = if (mask & 4) != 0 { dy } else { -dy };
                                    let diag = (sx + sy) * 0.85;
                                    pl.size = (start_size + diag).clamp(0.05, 0.44);
                                }
                            };
                            if let Some(idx) = ed.selected_custom {
                                if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                    apply_resize(&mut btn.layouts[mode_idx]);
                                }
                            } else if let Some(c) = ed.selected {
                                let slot = tc.settings.slot(c);
                                apply_resize(&mut tc.settings.layouts[mode_idx][slot]);
                            }
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                        EditorDragKind::PropsSlider { tab, index, rect } => {
                            let pad = if rect[3] >= 52.0 { 12.0 } else { 10.0 };
                            let track_x = rect[0] + pad;
                            let track_w = (rect[2] - pad * 2.0).max(16.0);
                            let t = ((p[0] - track_x) / track_w).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, tab, index, t);
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                    }
                }
            }
            return false;
        }

        // 1a. If Unsaved Changes Back Confirmation Modal is open:
        if ed.back_confirm_open {
            let dw = (w - 36.0).clamp(290.0, 430.0);
            let dh = 178.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            let bw = (dw - 44.0) * 0.5;
            let by = dy + dh - 50.0;
            let save_r = [dx + 18.0, by, bw, 36.0];
            let back_r = [dx + 26.0 + bw, by, bw, 36.0];
            if inside(save_r) {
                if tc.settings.is_default_preset() {
                    ed.back_confirm_open = false;
                    ed.save_dialog_open = true;
                    ed.editing_save_desc = false;
                    if ed.save_name_input.trim().is_empty() {
                        ed.save_name_input = format!("{} Custom", tc.settings.preset_name);
                    }
                    if ed.save_desc_input.trim().is_empty() {
                        ed.save_desc_input = tc.settings.preset_description.clone();
                    }
                    tc.clicked = true;
                    return false;
                }
                let _ = tc.settings.save_replace_preset(std::path::Path::new("."));
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            if inside(back_r) {
                tc.settings = ed.original.clone();
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            if !inside([dx, dy, dw, dh]) {
                ed.back_confirm_open = false;
                tc.clicked = true;
            }
            return false;
        }

        // 1b. If Save Dialog is open (Option A: Replace existing / Option B: Create new preset as...)
        if ed.save_dialog_open {
            let dw = (w - 24.0).clamp(310.0, 520.0);
            let dh = (h - 14.0).clamp(250.0, 360.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;

            // Cancel button -> back to editor
            if inside([dx + dw - 76.0, dy + 8.0, 62.0, 24.0]) || !inside([dx, dy, dw, dh]) {
                ed.save_dialog_open = false;
                tc.clicked = true;
                return false;
            }
            // Discard & Exit -> reload saved preset from disk and exit editor
            if inside([dx + dw - 166.0, dy + 8.0, 84.0, 24.0]) {
                let cur_style = tc.settings.control_style;
                let cur_id = tc.settings.preset_id.clone();
                tc.settings
                    .select_preset(std::path::Path::new("."), cur_style, &cur_id);
                tc.clicked = true;
                return true;
            }

            // Option A: Replace existing preset
            let is_def = tc.settings.is_default_preset();
            if inside([dx + 14.0, dy + 38.0, dw - 28.0, 44.0]) {
                if is_def {
                    ed.status_msg = Some(
                        "Default preset is protected and cannot be replaced. Create a new preset below."
                            .into(),
                    );
                    tc.clicked = true;
                    return false;
                }
                let _ = tc.settings.save_replace_preset(std::path::Path::new("."));
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }

            // Option B: Focus Name vs Description input
            let iy = dy + 100.0;
            let nw = (dw - 28.0 - 8.0) * 0.44;
            let desc_w = (dw - 28.0 - 8.0) - nw;
            if inside([dx + 14.0, iy, nw, 46.0]) {
                ed.editing_save_desc = false;
                ed.keyboard_request = true;
                tc.clicked = true;
                return false;
            }
            if inside([dx + 14.0 + nw + 8.0, iy, desc_w, 46.0]) {
                ed.editing_save_desc = true;
                ed.keyboard_request = true;
                tc.clicked = true;
                return false;
            }

            // Option B: Create New Preset & Save
            if inside([dx + 14.0, dy + dh - 34.0, dw - 28.0, 26.0]) {
                let name = if ed.save_name_input.trim().is_empty() {
                    format!("{} Custom", tc.settings.preset_name)
                } else {
                    ed.save_name_input.trim().to_owned()
                };
                let desc = if ed.save_desc_input.trim().is_empty() {
                    format!("Custom {} layout", tc.settings.control_style.label())
                } else {
                    ed.save_desc_input.trim().to_owned()
                };
                let _ = tc
                    .settings
                    .save_new_preset(std::path::Path::new("."), &name, &desc);
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            return false;
        }

        // 2. If Mouse vs Keyboard Picker Modal is open:
        if ed.picker_step != PickerStep::None {
            let is_mcpe = tc.settings.control_style == ControlStyle::Mcpe;
            match ed.picker_step {
                PickerStep::AskInputType { event_slot } => {
                    let dw = (w - 36.0).clamp(270.0, 440.0);
                    let dh = 236.0;
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    let cw = (dw - 40.0) * 0.5;
                    if inside([dx + 16.0, dy + 54.0, cw, 76.0]) {
                        ed.picker_step = PickerStep::Keyboard { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 24.0 + cw, dy + 54.0, cw, 76.0]) {
                        ed.picker_step = PickerStep::MouseAndActions { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 16.0, dy + 138.0, dw - 32.0, 34.0]) {
                        ed.keyboard_request = true;
                        ed.status_msg =
                            Some("Press any key on your keyboard to bind it to this slot.".into());
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 16.0, dy + 180.0, cw, 36.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        if let Some(idx) = ed.selected_custom
                            && let Some(btn) = tc.settings.custom_buttons.get_mut(idx)
                        {
                            if is_mcpe {
                                btn.set_exclusive_binding("key.keyboard.unknown".into());
                            } else {
                                btn.set_event_slot(event_slot, String::new());
                            }
                            let _ = tc.settings.save();
                        }
                        ed.picker_step = PickerStep::None;
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 24.0 + cw, dy + 180.0, cw, 36.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::None;
                        tc.clicked = true;
                        return false;
                    }
                    return false;
                }
                PickerStep::Keyboard { event_slot } => {
                    let dw = (w - 20.0).clamp(300.0, 620.0);
                    let dh = (h - 20.0).clamp(235.0, 340.0);
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    let max_kb_scroll = (880.0 - (dw - 28.0)).max(0.0);
                    if inside([dx + dw - 266.0, dy + 8.0, 64.0, 26.0]) {
                        ed.keyboard_request = true;
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 196.0, dy + 8.0, 56.0, 26.0]) {
                        ed.kb_scroll_x = (ed.kb_scroll_x - 140.0).clamp(0.0, max_kb_scroll);
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 134.0, dy + 8.0, 56.0, 26.0]) {
                        ed.kb_scroll_x = (ed.kb_scroll_x + 140.0).clamp(0.0, max_kb_scroll);
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 72.0, dy + 8.0, 58.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::AskInputType { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    ed.active_drag = Some((
                        id,
                        EditorDragKind::KeyboardScroll {
                            last_x: p[0],
                            moved: false,
                        },
                    ));
                    return false;
                }
                PickerStep::MouseAndActions { event_slot } => {
                    let dw = (w - 24.0).clamp(290.0, 560.0);
                    let dh = (h - 20.0).clamp(235.0, 355.0);
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    if inside([dx + dw - 72.0, dy + 8.0, 58.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::AskInputType { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    let cols = 3usize;
                    let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
                    let gx = dx + 12.0;
                    let gy = dy + 40.0;
                    let gw = dw - 24.0;
                    let gh = dh - 50.0;
                    let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
                    let ch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
                    for (i, (lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                        let col = i % cols;
                        let row = i / cols;
                        let cx = gx + col as f32 * (cw + 6.0);
                        let cy = gy + row as f32 * (ch + 4.0);
                        if inside([cx, cy, cw, ch]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            if let Some(idx) = ed.selected_custom
                                && let Some(btn) = tc.settings.custom_buttons.get_mut(idx)
                            {
                                if is_mcpe {
                                    // Strict single-binding exclusivity in MCPE mode:
                                    // selecting a mouse binding clears any keyboard binding.
                                    btn.set_exclusive_binding((*tok).to_owned());
                                    btn.custom_label =
                                        crate::touch::custom::friendly_token_name(tok);
                                } else {
                                    btn.set_event_slot(event_slot, (*tok).to_owned());
                                }
                                if *tok == "mouse.look" {
                                    for l in &mut btn.layouts {
                                        l.passthrough = true;
                                    }
                                }
                                let _ = tc.settings.save();
                            }
                            ed.picker_step = PickerStep::None;
                            ed.status_msg = Some(if is_mcpe {
                                format!("Bound '{lbl}' (exclusive — keyboard binding cleared).")
                            } else {
                                format!("Assigned '{lbl}' to Event Slot #{}.", event_slot + 1)
                            });
                            tc.clicked = true;
                            return false;
                        }
                    }
                    return false;
                }
                PickerStep::None => {}
            }
        }

        // 3. If Component Property Inspector Dialog is open (Studio Layout matching `90511.png`):
        if ed.props_open {
            let dw = (w - 16.0).clamp(350.0, 640.0);
            let dh = (h - 12.0).clamp(256.0, 384.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;

            // Close via top-left [X] button, top-right [✓ Done] button, or outside tap
            if inside([dx + 12.0, dy + 9.0, 26.0, 26.0])
                || inside([dx + dw - 74.0, dy + 9.0, 62.0, 26.0])
                || !inside([dx, dy, dw, dh])
            {
                ed.props_open = false;
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }
            // Center-Snap via telemetry pill OR [⛶] button
            if inside([dx + dw - 244.0, dy + 9.0, 164.0, 26.0]) {
                push_undo_snapshot(&mut ed, &tc.settings);
                let mode_idx = tc.settings.mode.index();
                if let Some(idx) = ed.selected_custom {
                    if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                        btn.layouts[mode_idx].center[0] = 0.5;
                    }
                } else if let Some(c) = ed.selected {
                    let slot = tc.settings.slot(c);
                    tc.settings.layouts[mode_idx][slot].center[0] = 0.5;
                }
                ed.status_msg = Some("Centered component horizontally (X = 50%).".into());
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }

            // Full-Width Workspace & Top 3-Segment Pill Bar clicks
            let wx = dx + 14.0;
            let ww = dw - 28.0;
            let tab_y = dy + 46.0;
            let tab_w = (ww - 8.0) / 3.0;
            for ti in 0..3 {
                let tx = wx + 2.0 + ti as f32 * (tab_w + 2.0);
                if inside([tx, tab_y - 2.0, tab_w, 28.0]) {
                    ed.props_tab = ti;
                    tc.clicked = true;
                    return false;
                }
            }

            let box_y = dy + 80.0;
            let box_h = dh - 92.0;
            let body_y = box_y + 10.0;
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
            if is_joystick {
                let mode_idx = tc.settings.mode.index();
                let slot = tc.settings.slot(Control::Move);

                match ed.props_tab {
                    0 => {
                        let st_pill_w = 216.0f32.min(ww * 0.46);
                        if inside([wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].use_goy_style =
                                !tc.settings.layouts[mode_idx][slot].use_goy_style;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let cards_y = body_y + 28.0;
                        let card_h = 74.0;
                        for i in 0..3 {
                            let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                            let rect = [rx, cards_y, cw3, card_h];
                            if inside(rect) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let t =
                                    ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0)).clamp(0.0, 1.0);
                                apply_props_slider(tc, &ed, 0, i, t);
                                ed.active_drag = Some((
                                    id,
                                    EditorDragKind::PropsSlider {
                                        tab: 0,
                                        index: i,
                                        rect,
                                    },
                                ));
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                        if inside(prev_r) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let sw_y = cards_y + card_h + 8.0;
                        let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                        let sw_d = 20.0;
                        let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                            if inside([sx, sw_row_y, sw_d, sw_d]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.stick_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let act_w = 152.0f32.min(ww * 0.34);
                        if inside([
                            wx + ww - act_w - 22.0,
                            sw_y + (sw_card_h - 28.0) * 0.5,
                            act_w,
                            28.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let cur = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| {
                                    *c == tc.settings.joystick_style.stick_stroke_color
                                })
                                .unwrap_or(0);
                            tc.settings.joystick_style.stick_stroke_color =
                                COLOR_SWATCHES[(cur + 1) % COLOR_SWATCHES.len()].0;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    1 => {
                        let mode_pill_w = 186.0f32.min(ww * 0.42);
                        if inside([
                            wx + ww - mode_pill_w - 12.0,
                            body_y - 2.0,
                            mode_pill_w,
                            24.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.mode = match tc.settings.mode {
                                Mode::Dpad => Mode::Crosshair,
                                Mode::Crosshair => Mode::Tap,
                                Mode::Tap => Mode::Dpad,
                            };
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let cards_y = body_y + 28.0;
                        let card_h = 74.0;
                        for i in 0..3 {
                            let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                            let rect = [rx, cards_y, cw3, card_h];
                            if inside(rect) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let t =
                                    ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0)).clamp(0.0, 1.0);
                                apply_props_slider(tc, &ed, 1, i, t);
                                ed.active_drag = Some((
                                    id,
                                    EditorDragKind::PropsSlider {
                                        tab: 1,
                                        index: i,
                                        rect,
                                    },
                                ));
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let sw_y = cards_y + card_h + 8.0;
                        let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                        let sw_d = 20.0;
                        let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                            if inside([sx, sw_row_y, sw_d, sw_d]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.base_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let act_w = 152.0f32.min(ww * 0.34);
                        if inside([
                            wx + ww - act_w - 22.0,
                            sw_y + (sw_card_h - 28.0) * 0.5,
                            act_w,
                            28.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let cur = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| {
                                    *c == tc.settings.joystick_style.base_stroke_color
                                })
                                .unwrap_or(0);
                            tc.settings.joystick_style.base_stroke_color =
                                COLOR_SWATCHES[(cur + 1) % COLOR_SWATCHES.len()].0;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    _ => {
                        let area_y = body_y + 26.0;
                        let hw = (ww - 24.0 - 8.0) * 0.5;
                        if inside([wx + 12.0, area_y, hw, 38.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let sz_x = wx + 12.0 + hw + 8.0;
                        let sz_rect = [sz_x, area_y, hw, 38.0];
                        if inside(sz_rect) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t =
                                ((p[0] - (sz_x + 10.0)) / (hw - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 2, 0, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 2,
                                    index: 0,
                                    rect: sz_rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let y6 = area_y + 48.0;
                        let third = (ww - 24.0 - 8.0) / 3.0;
                        if inside([wx + 12.0, y6, third, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].visible_in_game ^= true;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + third + 4.0, y6, third, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].visible_in_menu ^= true;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + (third + 4.0) * 2.0, y6, third, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.apply_visibility_preset_all();
                            let _ = tc.settings.save();
                            ed.status_msg =
                                Some("Applied preset visibility to all controls.".into());
                            tc.clicked = true;
                            return false;
                        }
                    }
                }
                return false;
            }

            let mode_idx = tc.settings.mode.index();
            let mutate_placement =
                |tc: &mut crate::touch::TouchControls,
                 ed: &crate::touch::Editor,
                 f: &dyn Fn(&mut crate::touch::Placement)| {
                    if let Some(idx) = ed.selected_custom {
                        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                            for m in 0..3 {
                                f(&mut btn.layouts[m]);
                            }
                        }
                    } else if let Some(c) = ed.selected {
                        let slot = tc.settings.slot(c);
                        f(&mut tc.settings.layouts[mode_idx][slot]);
                    }
                };

            match ed.props_tab {
                0 => {
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    if inside([wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.use_goy_style = !pl.use_goy_style);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, cards_y, cw3, card_h];
                        if inside(rect) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let sign = if p[0] < rx + cw3 * 0.5 { -1.0 } else { 1.0 };
                            match i {
                                0 => {
                                    let t = ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0))
                                        .clamp(0.0, 1.0);
                                    apply_props_slider(tc, &ed, 0, 0, t);
                                    ed.active_drag = Some((
                                        id,
                                        EditorDragKind::PropsSlider {
                                            tab: 0,
                                            index: 0,
                                            rect,
                                        },
                                    ));
                                }
                                1 => {
                                    mutate_placement(tc, &ed, &move |pl| {
                                        pl.icon_id = if sign < 0.0 {
                                            if pl.icon_id == 0 { 12 } else { pl.icon_id - 1 }
                                        } else {
                                            (pl.icon_id + 1) % 13
                                        };
                                    });
                                }
                                _ => {
                                    if let Some(idx) = ed.selected_custom {
                                        let hints = [
                                            "PRI", "SEC", "JUMP", "SNEAK", "SPRINT", "INV", "SWAP",
                                            "DROP", "CHAT", "DEBUG", "3RD", "MOUSE", "MACRO",
                                            "CLICK",
                                        ];
                                        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                            let pos = hints
                                                .iter()
                                                .position(|h| *h == btn.custom_label)
                                                .map(|idx_h| (idx_h + 1) % hints.len())
                                                .unwrap_or(0);
                                            btn.custom_label = hints[pos].to_owned();
                                        }
                                    }
                                }
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    // Tapping Live Preview Box also cycles Icon Badge
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    if inside(prev_r) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.icon_id = (pl.icon_id + 1) % 13;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        if inside([sx, sw_row_y, sw_d, sw_d]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.button_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let act_w = 158.0f32.min(ww * 0.36);
                    let act_x = wx + ww - act_w - 20.0;
                    let half_h = (sw_card_h - 14.0) * 0.5;
                    if inside([act_x, sw_y + 5.0, act_w, half_h]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            let idx = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| *c == pl.on_click_color)
                                .unwrap_or(0);
                            pl.on_click_color = COLOR_SWATCHES[(idx + 1) % COLOR_SWATCHES.len()].0;
                            pl.on_click_stroke_color =
                                COLOR_SWATCHES[(idx + 2) % COLOR_SWATCHES.len()].0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([act_x, sw_y + 9.0 + half_h, act_w, half_h]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            let idx = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| *c == pl.on_toggle_color)
                                .unwrap_or(0);
                            pl.on_toggle_color = COLOR_SWATCHES[(idx + 1) % COLOR_SWATCHES.len()].0;
                            pl.on_toggle_stroke_color =
                                COLOR_SWATCHES[(idx + 3) % COLOR_SWATCHES.len()].0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                }
                1 => {
                    let sh_pill_w = 192.0f32.min(ww * 0.44);
                    if inside([wx + ww - sh_pill_w - 12.0, body_y - 2.0, sh_pill_w, 24.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.shape = (pl.shape + 1) % 6);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let cards_y = body_y + 28.0;
                    let card_h = 35.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, cards_y, cw3, card_h];
                        if inside([rx, cards_y - 2.0, cw3, card_h + 4.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t = ((p[0] - (rx + 10.0)) / (cw3 - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 1, i, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 1,
                                    index: i,
                                    rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let row2_y = cards_y + 39.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, row2_y, cw3, card_h];
                        if inside([rx, row2_y - 2.0, cw3, card_h + 4.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t = ((p[0] - (rx + 10.0)) / (cw3 - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 1, 3 + i, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 1,
                                    index: 3 + i,
                                    rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, 74.0];
                    if inside(prev_r) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.shape = (pl.shape + 1) % 6);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let sw_y = cards_y + 74.0 + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        if inside([sx, sw_row_y, sw_d, sw_d]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.stroke_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    if inside([
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.shape = 0;
                            pl.roundness = 12.0;
                            pl.stroke_roundness = 12.0;
                            pl.width_ratio = 1.0;
                            pl.height_ratio = 1.0;
                            pl.stroke_size = 1.5;
                            pl.shadow_size = 4.0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                }
                _ => {
                    if tc.settings.control_style == ControlStyle::Mcpe
                        && ed.selected_custom.is_some()
                    {
                        let half_w = (ww - 24.0 - 8.0) * 0.5;
                        if inside([wx + 12.0, body_y, half_w, 42.0]) {
                            ed.picker_step = PickerStep::Keyboard { event_slot: 0 };
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + half_w + 8.0, body_y, half_w, 42.0]) {
                            ed.picker_step = PickerStep::MouseAndActions { event_slot: 0 };
                            tc.clicked = true;
                            return false;
                        }
                    } else {
                        let ew = (ww - 24.0 - 8.0) / 3.0;
                        for slot in 0..3 {
                            let ex = wx + 12.0 + slot as f32 * (ew + 4.0);
                            if inside([ex, body_y, ew, 42.0]) {
                                if ed.selected_custom.is_none() {
                                    push_undo_snapshot(&mut ed, &tc.settings);
                                    let idx = tc.settings.custom_buttons.len();
                                    let is_goy =
                                        tc.settings.control_style == ControlStyle::GoyControl;
                                    let mut nb = CustomButton::new("key.mouse.left".into(), idx);
                                    nb.custom_label = format!("BTN {}", idx + 1);
                                    for m in 0..3 {
                                        nb.layouts[m].use_goy_style = is_goy;
                                    }
                                    tc.settings.custom_buttons.push(nb);
                                    ed.selected_custom = Some(idx);
                                    ed.selected = None;
                                }
                                if ed.selected_custom.is_some() {
                                    ed.picker_step = PickerStep::AskInputType {
                                        event_slot: if tc.settings.control_style
                                            == ControlStyle::Mcpe
                                        {
                                            0
                                        } else {
                                            slot
                                        },
                                    };
                                }
                                tc.clicked = true;
                                return false;
                            }
                        }
                    }
                    let by = body_y + 52.0;
                    let bw = (ww - 24.0 - 12.0) / 4.0;
                    for i in 0..4 {
                        let rx = wx + 12.0 + i as f32 * (bw + 4.0);
                        if inside([rx, by, bw, 30.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            match i {
                                0 => mutate_placement(tc, &ed, &|pl| {
                                    pl.passthrough = !pl.passthrough
                                }),
                                1 => {
                                    mutate_placement(tc, &ed, &|pl| pl.toggleable = !pl.toggleable)
                                }
                                2 => mutate_placement(tc, &ed, &|pl| pl.swipeable = !pl.swipeable),
                                _ => {
                                    if let Some(idx) = ed.selected_custom {
                                        if idx < tc.settings.custom_buttons.len() {
                                            tc.settings.custom_buttons.remove(idx);
                                        }
                                        ed.selected_custom = None;
                                        ed.selected = Some(Control::Move);
                                        ed.props_open = false;
                                    } else if let Some(c) = ed.selected {
                                        let show = !tc.visible(c);
                                        tc.settings.button_visibility[c as usize] =
                                            if show { 1 } else { 2 };
                                    }
                                }
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let vy = by + 38.0;
                    let third = (ww - 24.0 - 8.0) / 3.0;
                    if inside([wx + 12.0, vy, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_game = !pl.visible_in_game);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([wx + 12.0 + third + 4.0, vy, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_menu = !pl.visible_in_menu);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([wx + 12.0 + (third + 4.0) * 2.0, vy, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        tc.settings.apply_visibility_preset_all();
                        let _ = tc.settings.save();
                        ed.status_msg = Some("Applied preset visibility to all controls.".into());
                        tc.clicked = true;
                        return false;
                    }
                }
            }
            return false;
        }

        // 4. If Main Control Manager Dialog is open:
        if ed.menu_open {
            let dw = (w - 24.0).clamp(300.0, 540.0);
            let dh = (h - 18.0).clamp(245.0, 350.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            if inside([dx + dw - 76.0, dy + 10.0, 62.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                ed.menu_open = false;
                tc.clicked = true;
                return false;
            }
            let ey = dy + 56.0;
            let ew = (dw - 28.0 - 8.0) * 0.5;
            // Button 1: Open Save Preset Dialog
            if inside([dx + 14.0, ey, ew, 30.0]) {
                ed.menu_open = false;
                ed.save_dialog_open = true;
                ed.editing_save_desc = false;
                ed.save_name_input = if tc.settings.is_default_preset() {
                    format!("{} Custom", tc.settings.preset_name)
                } else {
                    tc.settings.preset_name.clone()
                };
                ed.save_desc_input = tc.settings.preset_description.clone();
                tc.clicked = true;
                return false;
            }
            // Button 2: Reset Canvas to Default Layout of active system (without overwriting Default on disk)
            if inside([dx + 14.0 + ew + 8.0, ey, ew, 30.0]) {
                push_undo_snapshot(&mut ed, &tc.settings);
                let cur_style = tc.settings.control_style;
                let cur_id = tc.settings.preset_id.clone();
                let cur_name = tc.settings.preset_name.clone();
                let cur_desc = tc.settings.preset_description.clone();
                let active_mcpe = tc.settings.active_mcpe_preset.clone();
                let active_goy = tc.settings.active_goy_preset.clone();
                let fresh = crate::touch::Settings::default_for_style(cur_style);
                tc.settings.layouts = fresh.layouts;
                tc.settings.custom_buttons = fresh.custom_buttons;
                tc.settings.button_visibility = fresh.button_visibility;
                tc.settings.joystick_style = fresh.joystick_style;
                tc.settings.preset_id = cur_id;
                tc.settings.preset_name = cur_name;
                tc.settings.preset_description = cur_desc;
                tc.settings.active_mcpe_preset = active_mcpe;
                tc.settings.active_goy_preset = active_goy;
                clamp_editor_custom_selection(&mut ed, &tc.settings);
                ed.status_msg = Some(format!(
                    "Reset editor canvas to {} Default layout.",
                    cur_style.label()
                ));
                tc.clicked = true;
                return false;
            }

            let py = ey + 50.0;
            let presets = crate::touch::Settings::list_presets(
                std::path::Path::new("."),
                tc.settings.control_style,
            );
            let max_show = presets.len().clamp(1, 5);
            let pw = (dw - 28.0 - 4.0 * (max_show as f32 - 1.0)) / max_show as f32;
            for (i, meta) in presets.iter().take(max_show).enumerate() {
                let rx = dx + 14.0 + i as f32 * (pw + 4.0);
                if inside([rx, py, pw, 28.0]) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    tc.settings.select_preset(
                        std::path::Path::new("."),
                        tc.settings.control_style,
                        &meta.id,
                    );
                    clamp_editor_custom_selection(&mut ed, &tc.settings);
                    ed.save_name_input = tc.settings.preset_name.clone();
                    ed.save_desc_input = tc.settings.preset_description.clone();
                    ed.status_msg = Some(format!("Loaded preset '{}'.", meta.name));
                    tc.clicked = true;
                    return false;
                }
            }

            let gy = py + 50.0;
            let gw = (dw - 28.0 - 12.0) / 4.0;
            for i in 0..4 {
                let rx = dx + 14.0 + i as f32 * (gw + 4.0);
                if inside([rx, gy, gw, 28.0]) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    match i {
                        0 => {
                            tc.settings.mode = match tc.settings.mode {
                                Mode::Dpad => Mode::Crosshair,
                                Mode::Crosshair => Mode::Tap,
                                Mode::Tap => Mode::Dpad,
                            };
                        }
                        1 => {
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next()
                        }
                        2 => tc.settings.snap_to_grid = !tc.settings.snap_to_grid,
                        _ => ed.show_hidden = !ed.show_hidden,
                    }
                    let _ = tc.settings.save();
                    tc.clicked = true;
                    return false;
                }
            }
            return false;
        }

        // 5. Check Larger Material You 3 Moveable Floating Menu Pod (356x48dp)
        let pod_w = if ed.pod_expanded { 356.0 } else { 52.0 };
        let pod_h = 48.0;
        let pod_x = (tc.settings.floating_menu_pos[0] * w).clamp(8.0, (w - pod_w - 8.0).max(8.0));
        let pod_y = (tc.settings.floating_menu_pos[1] * h).clamp(6.0, (h - pod_h - 6.0).max(6.0));
        if inside([pod_x, pod_y, 48.0, pod_h]) {
            ed.active_drag = Some((
                id,
                EditorDragKind::FloatingPod {
                    offset: [p[0] - pod_x, p[1] - pod_y],
                    moved: false,
                },
            ));
            tc.clicked = true;
            return false;
        }
        if ed.pod_expanded && inside([pod_x, pod_y, pod_w, pod_h]) {
            let widths = [42.0, 42.0, 72.0, 64.0, 68.0];
            let mut ix = pod_x + 50.0;
            for (btn_i, iw) in widths.into_iter().enumerate() {
                if inside([ix, pod_y + 5.0, iw, 38.0]) {
                    match btn_i {
                        0 => {
                            // Undo
                            if let Some(prev) = ed.undo_stack.pop() {
                                ed.redo_stack.push(tc.settings.clone());
                                tc.settings = prev;
                                clamp_editor_custom_selection(&mut ed, &tc.settings);
                                let _ = tc.settings.save();
                                ed.status_msg = Some("Undid last change.".into());
                            }
                        }
                        1 => {
                            // Redo
                            if let Some(next) = ed.redo_stack.pop() {
                                ed.undo_stack.push(tc.settings.clone());
                                tc.settings = next;
                                clamp_editor_custom_selection(&mut ed, &tc.settings);
                                let _ = tc.settings.save();
                                ed.status_msg = Some("Redid change.".into());
                            }
                        }
                        2 => {
                            // Open Component Property Inspector
                            ed.props_open = !ed.props_open;
                            ed.menu_open = false;
                        }
                        3 => {
                            // + Add Button (supported in BOTH MCPE and GoyControl modes!)
                            if tc.settings.custom_buttons.len() < 32 {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let idx = tc.settings.custom_buttons.len();
                                let is_goy = tc.settings.control_style == ControlStyle::GoyControl;
                                let mut nb = CustomButton::new("key.mouse.left".into(), idx);
                                nb.custom_label = format!("BTN {}", idx + 1);
                                for m in 0..3 {
                                    nb.layouts[m].use_goy_style = is_goy;
                                }
                                tc.settings.custom_buttons.push(nb);
                                ed.selected_custom = Some(idx);
                                ed.selected = None;
                                ed.props_open = true;
                                ed.props_tab = 2;
                                if !is_goy {
                                    ed.picker_step = PickerStep::AskInputType { event_slot: 0 };
                                }
                                let _ = tc.settings.save();
                            }
                        }
                        _ => {
                            // Open Save Dialog (Option A: Replace / Option B: Create New Preset)
                            ed.save_dialog_open = true;
                            ed.menu_open = false;
                            ed.props_open = false;
                            ed.editing_save_desc = false;
                            ed.save_name_input = if tc.settings.is_default_preset() {
                                format!("{} Custom", tc.settings.preset_name)
                            } else {
                                tc.settings.preset_name.clone()
                            };
                            ed.save_desc_input = tc.settings.preset_description.clone();
                        }
                    }
                    tc.clicked = true;
                    return false;
                }
                ix += iw + 4.0;
            }
        }

        // 6. Check 8 Corner & Edge Resize Handles on currently selected component FIRST
        let aspect = tc.aspect();
        let cur_sel_placement = if let Some(idx) = ed.selected_custom {
            tc.settings
                .custom_buttons
                .get(idx)
                .map(|_| tc.custom_placement(idx))
        } else {
            ed.selected.map(|c| tc.placement(c))
        };
        if let Some(sel_p) = cur_sel_placement {
            let [nx, ny, nw, nh] = sel_p.rect(aspect);
            let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
            let handle_tests: [([f32; 4], u8); 8] = [
                ([rx - 10.0, ry - 10.0, 18.0, 18.0], 2 | 8),
                ([rx + rw - 8.0, ry - 10.0, 18.0, 18.0], 1 | 8),
                ([rx - 10.0, ry + rh - 8.0, 18.0, 18.0], 2 | 4),
                ([rx + rw - 8.0, ry + rh - 8.0, 18.0, 18.0], 1 | 4),
                ([rx + rw - 8.0, ry + rh * 0.5 - 12.0, 16.0, 24.0], 1),
                ([rx - 8.0, ry + rh * 0.5 - 12.0, 16.0, 24.0], 2),
                ([rx + rw * 0.5 - 12.0, ry + rh - 8.0, 24.0, 16.0], 4),
                ([rx + rw * 0.5 - 12.0, ry - 8.0, 24.0, 16.0], 8),
            ];
            for (hr, mask) in handle_tests {
                if inside(hr) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    ed.active_drag = Some((
                        id,
                        EditorDragKind::Resize {
                            mask,
                            start_pos: point,
                            start_size: sel_p.size,
                            start_wr: sel_p.width_ratio,
                            start_hr: sel_p.height_ratio,
                        },
                    ));
                    tc.clicked = true;
                    return false;
                }
            }
        }

        // 7. Hit-test Custom Buttons (in both MCPE and GoyControl modes) & Built-in Controls for Selection + Move Drag
        for i in (0..tc.settings.custom_buttons.len()).rev() {
            let pl = tc.custom_placement(i);
            let r = pl.rect(aspect);
            if crate::touch::layout::contains(r, point) {
                push_undo_snapshot(&mut ed, &tc.settings);
                ed.selected_custom = Some(i);
                ed.selected = None;
                ed.active_drag = Some((
                    id,
                    EditorDragKind::Move {
                        offset: [point[0] - pl.center[0], point[1] - pl.center[1]],
                    },
                ));
                tc.clicked = true;
                return false;
            }
        }

        for &c in Control::ALL.iter().rev() {
            if tc.settings.control_style == ControlStyle::GoyControl && c != Control::Move {
                continue;
            }
            let vis = tc.visible(c);
            if !vis && !ed.show_hidden {
                continue;
            }
            let pl = tc.placement(c);
            let r = pl.rect(aspect);
            if crate::touch::layout::contains(r, point) {
                push_undo_snapshot(&mut ed, &tc.settings);
                ed.selected = Some(c);
                ed.selected_custom = None;
                ed.active_drag = Some((
                    id,
                    EditorDragKind::Move {
                        offset: [point[0] - pl.center[0], point[1] - pl.center[1]],
                    },
                ));
                tc.clicked = true;
                return false;
            }
        }

        false
    })();

    if !closed {
        tc.editor = Some(ed);
    }
    closed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcpe_extra_keys_leave_a_separate_slot_for_the_control_hub() {
        let mut ui = TouchUi::default();
        assert!(matches!(ui.action(400.0, 240.0, [50.0, 10.0], false), Some(Action::More)));
        assert!(ui.action(400.0, 240.0, [10.0, 10.0], false).is_none());
        ui.expanded = true;
        assert!(matches!(ui.action(400.0, 240.0, [80.0, 10.0], false), Some(Action::Key(KeyCode::F3))));
        assert!(matches!(ui.action(400.0, 240.0, [160.0, 30.0], false), Some(Action::Key(KeyCode::F4))));
        ui.expanded = false;
        ui.goy_mode = true;
        assert!(ui.action(400.0, 240.0, [10.0, 10.0], false).is_none());
    }

    #[test]
    fn explicit_mouse_drag_does_not_generate_an_extra_tap_or_hold_click() {
        let mut ui = TouchUi::default();
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Started,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        ui.press_mouse(2, false);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_millis(700);
        assert_eq!(ui.relative_click(false), None);
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Ended,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn authoritative_virtual_mouse_visibility_and_viewport_centering() {
        let mut ui = TouchUi::default();
        let mut controls = crate::touch::TouchControls::default();
        let mut cursor = (0.0, 0.0);

        // Initial spawn centers cursor in playable viewport
        assert!(ui.sync_viewport_cursor(&mut cursor, [427.0, 240.0]));
        assert_eq!(cursor, (213.5, 120.0));

        // Moving cursor to (0.0, 0.0) after initialization does NOT teleport it back to center
        cursor = (0.0, 0.0);
        assert!(!ui.sync_viewport_cursor(&mut cursor, [427.0, 240.0]));
        assert_eq!(cursor, (0.0, 0.0));

        // Proportional resize preserves relative position without drift
        cursor = (213.5, 120.0);
        assert!(ui.sync_viewport_cursor(&mut cursor, [854.0, 480.0]));
        assert_eq!(cursor, (427.0, 240.0));

        // Default is OFF unless previously enabled by the user
        assert!(!controls.settings.virtual_mouse_enabled);
        assert!(!ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        controls.settings.virtual_mouse_enabled = true;

        // MCPE mode: hidden when MCPE gameplay controls are visible, visible when hidden (menus / F1)
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, false, true, true));
        assert!(ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        assert!(ui.compute_virtual_mouse_visibility(&controls, false, true, true, true));

        // Control Editor open: game cursor is ALWAYS hidden over the Control Editor
        controls.open_editor();
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, false, true, false));
        controls.cancel_editor();

        // Authoritative toggle OFF: remains hidden regardless of auto-visibility conditions
        controls.settings.virtual_mouse_enabled = false;
        assert!(!ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, true, true, false));
    }
    #[test]
    fn menu_mouse_buttons_share_ownership_and_release_outside_their_bounds() {
        let mut ui = TouchUi {
            virtual_mouse: true,
            ..Default::default()
        };
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 226.0], true),
            Some(Action::MouseButton(false))
        ));
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 250.0], true),
            Some(Action::MouseButton(true))
        ));
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], false).is_none());
        assert!(ui.press_mouse(1, false));
        assert!(!ui.press_mouse(2, false));
        assert!(ui.press_mouse(3, true));
        assert_eq!(ui.release_mouse(1), Some((false, false)));
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.release_all_mouse(), vec![true]);
        assert_eq!(ui.release_mouse(3), None);
        ui.virtual_mouse = false;
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], true).is_none());
    }
    #[test]
    fn virtual_mouse_moves_relatively_and_dragging_never_clicks() {
        let mut ui = TouchUi::default();
        let cursor = [100.0, 80.0];
        let size = [400.0, 240.0];
        assert_eq!(
            ui.relative_touch(
                1,
                [300.0, 200.0],
                TouchPhase::Started,
                cursor,
                size,
                2.0,
                true
            ),
            cursor
        );
        let p = ui.relative_touch(
            1,
            [310.0, 195.0],
            TouchPhase::Moved,
            cursor,
            size,
            2.0,
            true,
        );
        assert_eq!(p, [120.0, 70.0]);
        ui.relative_touch(1, [310.0, 195.0], TouchPhase::Ended, p, size, 2.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn virtual_mouse_tap_hold_cancel_and_click_preference() {
        let mut ui = TouchUi::default();
        let p = [100.0, 80.0];
        let size = [400.0, 240.0];
        ui.relative_touch(1, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative_touch(1, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), Some(false));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_secs(1);
        assert_eq!(ui.relative_click(true), Some(true));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(3, p, TouchPhase::Started, p, size, 1.0, false);
        ui.relative_touch(3, p, TouchPhase::Ended, p, size, 1.0, false);
        assert_eq!(ui.relative_click(false), None);
        ui.relative_touch(4, p, TouchPhase::Started, p, size, 1.0, true);
        ui.cancel_relative();
        ui.relative_touch(4, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn simultaneous_f3_f4_and_f6_release_independently() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(10, KeyCode::F3));
        assert!(ui.key_down(11, KeyCode::F4));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(11), Some(KeyCode::F4));
        assert!(ui.key_down(12, KeyCode::F6));
        assert_eq!(ui.key_up(12), Some(KeyCode::F6));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(10), Some(KeyCode::F3));
    }
    #[test]
    fn shared_key_and_latch_do_not_release_early() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(1, KeyCode::F3));
        assert!(!ui.key_down(2, KeyCode::F3));
        assert_eq!(ui.key_up(1), None);
        ui.f3_latched = true;
        assert_eq!(ui.key_up(2), None);
        assert_eq!(ui.release_keys(), vec![KeyCode::F3]);
        assert!(!ui.key_held(KeyCode::F3));
    }
    #[test]
    fn scrolling_never_clicks() {
        let mut u = TouchUi::default();
        assert!(matches!(
            u.gesture(1, [20.0, 200.0], TouchPhase::Started),
            Gesture::None
        ));
        assert!(
            matches!(u.gesture(1,[20.0,140.0],TouchPhase::Moved),Gesture::Scroll(v) if v == -2.0)
        );
        assert!(matches!(
            u.gesture(1, [20.0, 140.0], TouchPhase::Ended),
            Gesture::None
        ));
    }
    #[test]
    fn tap_release_cancel_and_second_finger() {
        let mut u = TouchUi::default();
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        assert!(matches!(
            u.gesture(2, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
        assert!(matches!(
            u.gesture(1, [21.0, 21.0], TouchPhase::Ended),
            Gesture::Tap
        ));
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        u.gesture(1, [20.0, 20.0], TouchPhase::Cancelled);
        assert!(matches!(
            u.gesture(1, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
    }

    /// The press/release decision table of a physical left click the pad may
    /// take: `(pressed, owned, touched)`.
    #[test]
    fn aux_mouse_presses_belong_to_the_pad_only_over_its_buttons() {
        // Gameplay in a world: attack and use are not the pad's.
        assert_eq!(aux_mouse_phase(true, false, false), None);
        // The free pointer uses the gameplay auxiliary buttons (keypad
        // profile) and menu actions alike: the press starts a touch.
        assert_eq!(
            aux_mouse_phase(true, false, true),
            Some(TouchPhase::Started)
        );
    }
    #[test]
    fn a_menu_press_survives_closing_the_menu_before_its_release() {
        // Press an action, then the screen closes and redraws elsewhere: the
        // release must still complete the press, through the owned key.
        let mut ui = TouchUi::default();
        let owned = |ui: &TouchUi| ui.owns_key(7);
        assert_eq!(aux_mouse_phase(true, owned(&ui), false), None);
        assert_eq!(
            aux_mouse_phase(true, owned(&ui), true),
            Some(TouchPhase::Started)
        );
        ui.key_down(7, KeyCode::KeyT);
        assert_eq!(
            aux_mouse_phase(false, owned(&ui), false),
            Some(TouchPhase::Ended)
        );
        assert_eq!(ui.key_up(7), Some(KeyCode::KeyT));
        // The pad released everything: later strays do nothing (a release
        // game screens can drop on menu entry also stays inert).
        assert_eq!(aux_mouse_phase(false, owned(&ui), false), None);
        assert_eq!(aux_mouse_phase(true, owned(&ui), false), None);
    }
    #[test]
    fn a_held_auxiliary_mouse_button_releases_after_a_screen_change() {
        // Where LMB/RMB utility buttons own the pointer, a release stays
        // theirs across ownership checks, then frees it.
        let mut ui = TouchUi::default();
        assert_eq!(aux_mouse_phase(false, ui.owns_mouse(7), false), None);
        ui.press_mouse(7, true);
        assert_eq!(
            aux_mouse_phase(false, ui.owns_mouse(7), false),
            Some(TouchPhase::Ended)
        );
        assert_eq!(ui.release_mouse(7), Some((true, true)));
        assert_eq!(aux_mouse_phase(false, ui.owns_mouse(7), false), None);
    }
    #[test]
    fn a_release_without_ownership_stays_with_the_game() {
        assert_eq!(aux_mouse_phase(false, false, true), None);
    }
}
