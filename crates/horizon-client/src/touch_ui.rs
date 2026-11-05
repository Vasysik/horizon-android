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
    ToggleAutoJump,
    SetTouchSensitivity(f32),
    StepTouchSensitivity(i32),
    SetVirtualMouseSpeed(f32),
    StepVirtualMouseSpeed(i32),
    SetVirtualMousePointerSize(f32),
    StepGyroSensitivity(i32),
    StepButtonScale(i32),
    TriggerKey(KeyCode),
    OpenKeyboard,
    /// Kill the game process now (no save, no shutdown sequence).
    ForceKill,
    /// Open the game's log folder in the system file manager.
    OpenLogs,
    /// The naming field of a preset dialog was tapped: show the system keyboard.
    EditPresetField(bool),
    /// Send a custom keyboard key, mouse button/scroll, or gameplay action token immediately.
    SendCustomInput(String),
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
            Output::Backspace => {
                target.pop();
            }
            Output::Enter | Output::Close => {}
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
            Output::Backspace => {
                target.pop();
            }
            Output::Enter | Output::Close => {}
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
        if !controls.settings.virtual_mouse_enabled {
            self.goy_in_game_mouse = false;
            return false;
        }
        if self.floating_menu {
            return true;
        }
        match controls.settings.control_style {
            crate::touch::ControlStyle::Mcpe => {
                let mcpe_visible = controls.mcpe_controls_visible(in_menu, hide_gui, touch_active);
                !mcpe_visible && (in_menu || !captured || !mcpe_visible)
            }
            crate::touch::ControlStyle::GoyControl => {
                in_menu || !captured || self.goy_in_game_mouse
            }
        }
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
        if self.expanded && (3.0..23.0).contains(&p[1]) {
            if (3.0..36.0).contains(&p[0]) {
                return Some(Action::More);
            }
            if (39.0..72.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F3));
            }
            if (75.0..108.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F5));
            }
            if (111.0..144.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::KeyT));
            }
            if (147.0..186.0).contains(&p[0]) {
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
        }
        if !self.expanded {
            return;
        }
        for (label, x, width) in [
            ("...", 3.0, 33.0),
            ("F3", 39.0, 33.0),
            ("F5", 75.0, 33.0),
            ("Chat", 111.0, 33.0),
            ("Keys", 147.0, 39.0),
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
    b.text(a, label, x + 4.0, y + 6.0, rgba(245, 245, 245, 255), true);
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
            let dh = 210.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
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
                "Select a keyboard key, mouse click/scroll, or game action to send immediately."
            } else if mcpe_exclusive {
                "Exclusive binding: Keyboard OR Mouse (selecting one clears the other)."
            } else {
                "Each GoyControl button can trigger up to 3 Keyboard or Mouse events simultaneously."
            };
            ui.text(
                Face::SemiBold,
                13.5,
                dx + 16.0,
                dy + 14.0,
                text_c,
                &title_str,
            );
            ui.text(Face::Regular, 10.2, dx + 16.0, dy + 34.0, muted_c, sub_str);
            let cw = (dw - 40.0) * 0.5;
            let r_kb = [dx + 16.0, dy + 62.0, cw, 82.0];
            let r_ms = [dx + 24.0 + cw, dy + 62.0, cw, 82.0];
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
                    [r[0] + 6.0, r[1] + 16.0, r[2] - 12.0, 24.0],
                    text_c,
                    title,
                );
                ui.centered_fit(
                    Face::Regular,
                    10.0,
                    [r[0] + 6.0, r[1] + 44.0, r[2] - 12.0, 22.0],
                    muted_c,
                    sub,
                );
            }
            if for_custom_send {
                let r_can = [dx + 16.0, dy + 156.0, dw - 32.0, 36.0];
                ui.rounded(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, bg_hi);
                ui.border(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, border_c);
                ui.centered_fit(Face::Medium, 11.0, r_can, text_c, "Cancel");
            } else {
                let r_clr = [dx + 16.0, dy + 156.0, cw, 36.0];
                let r_can = [dx + 24.0 + cw, dy + 156.0, cw, 36.0];
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
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Full PC Keyboard (Swipe Horizontally)".to_owned()
            } else if mcpe_exclusive {
                "MCPE Exclusive Keyboard Binding (Clears Mouse Binding)".to_owned()
            } else {
                format!(
                    "Event Slot #{} — Full PC Keyboard (Swipe Horizontally)",
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
            let b_left = [dx + dw - 196.0, dy + 8.0, 56.0, 26.0];
            let b_right = [dx + dw - 134.0, dy + 8.0, 56.0, 26.0];
            let b_back = [dx + dw - 72.0, dy + 8.0, 58.0, 26.0];
            for (r, lbl) in [(b_left, "◀ Left"), (b_right, "Right ▶"), (b_back, "Back")] {
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
    use crate::ui_theme::{Face, Ui, color};
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

    // 2. Custom GoyControl Static Buttons (respecting `visible_in_game` and `visible_in_menu`)
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
            p.shadow_size,
            shadow_c,
        );
        ui.centered_fit(
            Face::SemiBold,
            (rh * 0.30).clamp(10.0, 15.0),
            [rx + 3.0, ry + 2.0, (rw - 6.0).max(6.0), (rh - 4.0).max(6.0)],
            color::with_alpha(text_c, (alpha.max(0.75) * hud_scale).clamp(0.0, 1.0)),
            &label,
        );
    }

    ui.finish();
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

        let (btn_w, btn_h) = if settings.control_style == ControlStyle::Mcpe {
            (42.0, 42.0)
        } else {
            (48.0, 42.0)
        };
        let btn_x = (settings.floating_menu_pos[0] * w).clamp(6.0, (w - btn_w - 6.0).max(6.0));
        let btn_y = (settings.floating_menu_pos[1] * h).clamp(4.0, (h - btn_h - 4.0).max(4.0));
        let btn_rect = [btn_x, btn_y, btn_w, btn_h];

        if let Some((drag_id, offset, mut moved)) = self.floating_drag {
            if drag_id == id {
                match phase {
                    TouchPhase::Moved => {
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
                                self.hub_scroll_drag = None;
                            }
                        }
                        return Some(InGameMenuAction::Close);
                    }
                    _ => {}
                }
            }
        }

        if phase == TouchPhase::Started && !self.expanded && inside(btn_rect) {
            self.floating_drag = Some((id, [p[0] - btn_x, p[1] - btn_y], false));
            return Some(InGameMenuAction::Close);
        }

        if !self.floating_menu {
            return None;
        }

        let (dw, dh, dx, dy, side_w, cx, cw, body_top, body_h) = hub_metrics(w, h);
        let content_pane = [cx - 4.0, body_top, cw + 8.0, body_h];

        // Handle ongoing sensitivity slider drag on Tab 0
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

        // Handle ongoing virtual-mouse-speed slider drag on Tab 1
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

        // 0. If "Send Custom Input" Picker Modal is open:
        if self.custom_input_picker != crate::touch::PickerStep::None {
            use crate::touch::PickerStep;
            if let Some((drag_id, last_x, mut moved)) = self.custom_kb_drag {
                if drag_id == id {
                    match phase {
                        TouchPhase::Moved => {
                            let dx_k = p[0] - last_x;
                            if dx_k.abs() > 2.0 {
                                moved = true;
                            }
                            let mdw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                            self.custom_kb_scroll_x =
                                (self.custom_kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                            self.custom_kb_drag = Some((drag_id, p[0], moved));
                            return Some(InGameMenuAction::Close);
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
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
                    let mdh = 210.0;
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let mcw = (mdw - 40.0) * 0.5;
                    if inside([mdx + 16.0, mdy + 62.0, mcw, 82.0]) {
                        self.custom_input_picker = PickerStep::Keyboard { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 24.0 + mcw, mdy + 62.0, mcw, 82.0]) {
                        self.custom_input_picker = PickerStep::MouseAndActions { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 16.0, mdy + 156.0, mdw - 32.0, 36.0])
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
                    let kb_top = mdy + 40.0;
                    let kb_h = mdh - 56.0;
                    let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                    let key_unit = 38.0;
                    let gap = 4.0;
                    for (r_idx, row) in KB_ROWS.iter().enumerate() {
                        let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                        let mut kx = mdx + 14.0 - self.custom_kb_scroll_x;
                        for (_lbl, tok, width_mult) in *row {
                            let kw = key_unit * (*width_mult) + (*width_mult - 1.0).max(0.0) * gap;
                            if inside([kx, ry, kw, row_h])
                                && p[0] >= mdx + 10.0
                                && p[0] <= mdx + mdw - 10.0
                            {
                                self.custom_input_picker = PickerStep::None;
                                self.floating_menu = false;
                                return Some(InGameMenuAction::SendCustomInput((*tok).to_owned()));
                            }
                            kx += kw + gap;
                        }
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
                self.sensitivity_dragging = false;
                self.pointer_size_dragging = false;
                self.mouse_speed_dragging = false;
                self.custom_input_picker = crate::touch::PickerStep::None;
                self.custom_kb_drag = None;
                self.hub_scroll_drag = None;
                return Some(InGameMenuAction::Close);
            }

            // Left Sidebar Category Tabs (Controls, Aiming, Buttons, Advanced)
            let tab_start_y = dy + 58.0;
            for ti in 0..4u8 {
                let ty = tab_start_y + ti as f32 * 44.0;
                if inside([dx + 10.0, ty, side_w - 16.0, 38.0]) {
                    if self.hub_tab != ti {
                        self.hub_tab = ti;
                        self.hub_scroll = 0.0;
                        self.tab_anim.set(0.0);
                        self.last_anim_tick.set(None);
                    }
                    return Some(InGameMenuAction::Close);
                }
            }

            // Check if touch started on the Sensitivity slider track in Tab 0
            if self.hub_tab == 0 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let sens_card_y = body_y + 138.0;
                let track_r = [cx, sens_card_y + 22.0, cw, 28.0];
                if inside(track_r) {
                    self.sensitivity_dragging = true;
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
            }

            // Check if touch started on the Virtual Mouse Speed or Pointer Size slider tracks in Tab 1
            if self.hub_tab == 1 && inside(content_pane) {
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
                // Tab 0: Controls
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
            1 => {
                // Tab 1: Aiming, Camera & Virtual Mouse
                let row_h = 46.0;
                let step = 54.0;
                let r0 = [cx, body_y + 18.0, cw, row_h]; // Movement & Aim Mode
                let r1 = [cx, body_y + 18.0 + step, cw, row_h]; // Joystick Active Zone
                let r2 = [cx, body_y + 18.0 + step * 2.0, cw, row_h]; // Virtual Mouse Toggle
                let spd_card = [cx, body_y + 180.0, cw, 50.0]; // Virtual Mouse Speed slider
                let size_card = [cx, body_y + 238.0, cw, 50.0]; // Pointer Size slider
                let r4 = [cx, body_y + 296.0, cw, row_h]; // Gyroscope Aiming
                let r5 = [cx, body_y + 350.0, cw, row_h]; // Gyro Sensitivity
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
                if inside(r5) {
                    return if p[0] < cx + cw * 0.5 {
                        Some(InGameMenuAction::StepGyroSensitivity(-1))
                    } else {
                        Some(InGameMenuAction::StepGyroSensitivity(1))
                    };
                }
            }
            2 => {
                // Tab 2: Buttons & Layout Options
                let row_h = 46.0;
                let step = 54.0;
                let scale_y = body_y + 18.0;
                if inside([cx, scale_y, cw * 0.5, row_h]) {
                    return Some(InGameMenuAction::StepButtonScale(-1));
                }
                if inside([cx + cw * 0.5, scale_y, cw * 0.5, row_h]) {
                    return Some(InGameMenuAction::StepButtonScale(1));
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
                    return Some(InGameMenuAction::ToggleVibrate);
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

        // Draw moveable Floating Menu Button (MCPE 42x42dp stone-bevel H button OR GoyControl 48x42dp '=' pill)
        if !self.expanded && bubble_alpha > 0.001 {
            if settings.control_style == ControlStyle::Mcpe {
                let btn_w = 42.0;
                let btn_h = 42.0;
                let btn_x =
                    (settings.floating_menu_pos[0] * w).clamp(6.0, (w - btn_w - 6.0).max(6.0));
                let btn_y =
                    (settings.floating_menu_pos[1] * h).clamp(4.0, (h - btn_h - 4.0).max(4.0));
                let scale_rgba = |r: u8, g: u8, bl: u8, al: u8| -> u32 {
                    rgba(r, g, bl, (f32::from(al) * bubble_alpha).round() as u8)
                };
                // Drop shadow
                ui.rounded(
                    btn_x + 1.5,
                    btn_y + 2.5,
                    btn_w,
                    btn_h,
                    4.0,
                    scale_rgba(0, 0, 0, 125),
                );
                // Dark MCPE outer border
                ui.rounded(btn_x, btn_y, btn_w, btn_h, 4.0, scale_rgba(18, 20, 24, 225));
                // Translucent MCPE stone-slate body (or emerald-slate when active)
                let body_col = if self.floating_menu {
                    scale_rgba(88, 130, 72, 225)
                } else {
                    scale_rgba(58, 62, 70, 205)
                };
                ui.rounded(
                    btn_x + 2.0,
                    btn_y + 2.0,
                    btn_w - 4.0,
                    btn_h - 4.0,
                    2.5,
                    body_col,
                );
                // Classic MCPE 3D top/left light highlight bevel & bottom/right dark shadow bevel
                let hi_bevel = scale_rgba(165, 172, 184, 180);
                let lo_bevel = scale_rgba(28, 30, 36, 200);
                ui.fill(btn_x + 2.0, btn_y + 2.0, btn_w - 4.0, 2.0, hi_bevel);
                ui.fill(btn_x + 2.0, btn_y + 4.0, 2.0, btn_h - 6.0, hi_bevel);
                ui.fill(btn_x + 2.0, btn_y + btn_h - 4.0, btn_w - 4.0, 2.0, lo_bevel);
                ui.fill(btn_x + btn_w - 4.0, btn_y + 4.0, 2.0, btn_h - 6.0, lo_bevel);
                // Horizon 'H' icon crisp and centered inside the MCPE button
                ui.icon(
                    Icon::HorizonLogo,
                    btn_x + (btn_w - 22.0) * 0.5,
                    btn_y + (btn_h - 22.0) * 0.5,
                    22.0,
                    scale_rgba(244, 246, 250, 245),
                );
            } else {
                let btn_w = 48.0;
                let btn_h = 42.0;
                let btn_x =
                    (settings.floating_menu_pos[0] * w).clamp(6.0, (w - btn_w - 6.0).max(6.0));
                let btn_y =
                    (settings.floating_menu_pos[1] * h).clamp(4.0, (h - btn_h - 4.0).max(4.0));
                ui.rounded(
                    btn_x + 1.5,
                    btn_y + 3.0,
                    btn_w,
                    btn_h,
                    14.0,
                    rgba(0, 0, 0, (135.0 * bubble_alpha).round() as u8),
                );
                ui.rounded(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    14.0,
                    color::with_alpha(
                        if self.floating_menu { accent } else { card_bg },
                        bubble_alpha,
                    ),
                );
                ui.stroke_rounded(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    14.0,
                    1.6,
                    color::with_alpha(
                        if self.floating_menu { text_c } else { accent },
                        bubble_alpha,
                    ),
                );
                ui.icon(
                    Icon::MenuEquals,
                    btn_x + (btn_w - 22.0) * 0.5,
                    btn_y + (btn_h - 22.0) * 0.5,
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

        // Left Sidebar Navigation Tabs (Controls, Aiming, Buttons, Advanced)
        let tabs: [(Icon, &str); 4] = [
            (Icon::Gamepad, "Controls"),
            (Icon::Crosshair, "Aiming"),
            (Icon::MicrosoftGrid, "Buttons"),
            (Icon::Flask, "Advanced"),
        ];
        let tab_start_y = dy + 58.0;
        for (ti, (ic, lbl)) in tabs.into_iter().enumerate() {
            let ty = tab_start_y + ti as f32 * 44.0;
            let active = self.hub_tab == ti as u8;
            let tr = [dx + 10.0, ty, side_w - 18.0, 36.0];
            if active {
                ui.rounded(tr[0], tr[1], tr[2], tr[3], 10.0, accent);
                ui.rounded(tr[0] - 4.0, tr[1] + 8.0, 3.0, 20.0, 1.5, accent_hi);
            }
            let fg = if active { text_c } else { muted_c };
            ui.icon(ic, tr[0] + 10.0, tr[1] + 10.0, 16.0, fg);
            ui.text(
                if active { Face::SemiBold } else { Face::Medium },
                11.2,
                tr[0] + 32.0,
                tr[1] + 9.0,
                fg,
                lbl,
            );
        }

        // Scrollable Right Content Panel with scissor clip and tab transition slide
        let tab_slide = (1.0 - tab_ease) * 14.0;
        let content_height = match self.hub_tab {
            0 => 368.0,
            1 => 410.0,
            2 => 242.0,
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
            1 => {
                // Tab 1: Aiming, Camera & Virtual Mouse
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
                draw_toggle_row(
                    &mut ui,
                    body_y + 350.0,
                    "Gyroscope Sensitivity",
                    &gyro_spd_lbl,
                    None,
                    Some("−   +"),
                );
            }
            2 => {
                // Tab 2: Buttons & Layout Options
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "BUTTON SIZING, HANDEDNESS & BEHAVIOR",
                );
                let step = 54.0;
                let scale_title = format!(
                    "Global Button Scale ({:.0}%)",
                    settings.button_scale * 100.0
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0,
                    &scale_title,
                    "Tap left half (−) or right half (+) to resize HUD buttons",
                    None,
                    Some("−   +"),
                );
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
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0,
                    "Haptic Feedback",
                    "Vibrate on block break, damage and touch button presses",
                    Some(settings.vibrate),
                    None,
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

/// Renders the complete MCPE + GoyControl Editor with:
/// - Real MCPE sprites/bevel buttons + MCPE custom buttons OR GoyControl Static Buttons
/// - GoyControl Customizable Analog Joystick + JoystickMoveArea indicator
/// - Interactive 8-point Corner & Edge Resize Handles
/// - Moveable/Draggable Floating Menu Pod (No Top Toolbar!) with Undo, Redo, Props, Add & Save
/// - Main Control Manager Dialog, Component Property Inspector, and Mouse/Keyboard Horizontal Picker
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
            if matches!(c, Control::Offhand | Control::Shield) {
                let lbl = if c == Control::Offhand {
                    "Swap"
                } else {
                    "Shield"
                };
                ui.centered_fit(
                    Face::SemiBold,
                    (rh * 0.28).clamp(9.0, 13.0),
                    [rx, ry, rw, rh],
                    text_c,
                    lbl,
                );
            }
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
        } else {
            // Classic MCPE stone bevel plate for MCPE custom buttons
            let a_u8 = (alpha * 255.0).round() as u8;
            ui.rounded(rx, ry, rw, rh, 3.0, rgba(18, 20, 24, a_u8));
            ui.rounded(
                rx + 2.0,
                ry + 2.0,
                (rw - 4.0).max(2.0),
                (rh - 4.0).max(2.0),
                2.0,
                rgba(58, 62, 70, (alpha * 210.0).round() as u8),
            );
            ui.fill(
                rx + 2.0,
                ry + 2.0,
                (rw - 4.0).max(2.0),
                2.0,
                rgba(175, 182, 194, (alpha * 195.0).round() as u8),
            );
            ui.fill(
                rx + 2.0,
                ry + 4.0,
                2.0,
                (rh - 6.0).max(2.0),
                rgba(175, 182, 194, (alpha * 195.0).round() as u8),
            );
            ui.fill(
                rx + 2.0,
                ry + rh - 4.0,
                (rw - 4.0).max(2.0),
                2.0,
                rgba(28, 30, 36, (alpha * 215.0).round() as u8),
            );
            ui.fill(
                rx + rw - 4.0,
                ry + 4.0,
                2.0,
                (rh - 6.0).max(2.0),
                rgba(28, 30, 36, (alpha * 215.0).round() as u8),
            );
        }
        ui.centered_fit(
            Face::SemiBold,
            (rh * 0.30).clamp(9.5, 14.5),
            [rx + 3.0, ry + 2.0, (rw - 6.0).max(6.0), (rh - 4.0).max(6.0)],
            text_c,
            &label,
        );

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

    // 8. Component Property Inspector Dialog (when ed.props_open == true)
    if ed.props_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 185));
        let dw = (w - 24.0).clamp(310.0, 548.0);
        let dh = (h - 18.0).clamp(248.0, 368.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx + 2.0, dy + 5.0, dw, dh, 20.0, rgba(0, 0, 0, 155));
        ui.rounded(dx, dy, dw, dh, 20.0, bg_card);
        ui.stroke_rounded(dx, dy, dw, dh, 20.0, 1.4, border_c);

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
                    "Exclusive Binding (Keyboard OR Mouse — selecting one clears the other)",
                )
            } else {
                (
                    format!("GoyControl Button — {lbl}"),
                    "Customize visual style, geometry, stroke, key/mouse bindings & behavior",
                )
            }
        } else if let Some(c) = ed.selected {
            (
                format!("MCPE Component — {}", c.name()),
                "Customize style, opacity, shape, stroke & menu/game visibility",
            )
        } else {
            (
                "Component Properties".to_owned(),
                "Configure control appearance & behavior",
            )
        };
        ui.rounded(dx + 14.0, dy + 9.0, 28.0, 28.0, 9.0, accent_soft);
        ui.border(dx + 14.0, dy + 9.0, 28.0, 28.0, 9.0, accent);
        ui.icon(Icon::Gear, dx + 20.0, dy + 15.0, 16.0, accent);
        ui.text(
            Face::SemiBold,
            13.0,
            dx + 48.0,
            dy + 8.0,
            text_c,
            &header_txt,
        );
        ui.text(Face::Regular, 9.0, dx + 48.0, dy + 24.0, muted_c, sub_txt);

        let close_r = [dx + dw - 74.0, dy + 9.0, 60.0, 28.0];
        ui.rounded(close_r[0], close_r[1], close_r[2], close_r[3], 9.0, accent);
        ui.centered_fit(Face::SemiBold, 10.5, close_r, accent_fg, "Done");

        // Segmented Material You 3 Tab Header (for both Joystick and Button inspectors)
        let tab_y = dy + 42.0;
        let tab_w = (dw - 28.0 - 8.0) / 3.0;
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
            let tx = dx + 14.0 + ti as f32 * (tab_w + 4.0);
            let on = ed.props_tab == ti;
            ui.rounded(
                tx,
                tab_y,
                tab_w,
                26.0,
                8.0,
                if on { accent_soft } else { bg_hi },
            );
            ui.border(
                tx,
                tab_y,
                tab_w,
                26.0,
                8.0,
                if on { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                10.0,
                [tx + 3.0, tab_y, tab_w - 6.0, 26.0],
                if on { accent } else { text_c },
                t_lbl,
            );
        }

        let body_y = tab_y + 34.0;
        if is_joystick {
            let js = &tc.settings.joystick_style;
            let p_move = tc.placement(Control::Move);
            let cw = (dw - 28.0 - 12.0) / 4.0;
            let sw_w = 22.0;
            let st_x = dx + 86.0 + 8.0 * (sw_w + 4.0) + 12.0;
            let st_w = (dw - (st_x - dx) - 14.0).max(80.0);

            match ed.props_tab {
                0 => {
                    // Joystick Tab 0: Renderer Style, Active Zone, Stick Knob Sizing & Stick Colors
                    ui.text(
                        Face::SemiBold,
                        9.4,
                        dx + 14.0,
                        body_y - 2.0,
                        muted_c,
                        "JOYSTICK STYLE & ACTIVE ZONE",
                    );
                    let y1 = body_y + 12.0;
                    let hw = (dw - 28.0 - 8.0) * 0.5;
                    let st_lbl = if tc.settings.control_style == ControlStyle::GoyControl
                        || p_move.use_goy_style
                    {
                        "Style: Custom GoyControl Stick"
                    } else {
                        "Style: Default MCPE D-Pad/Stick"
                    };
                    ui.rounded(dx + 14.0, y1, hw, 30.0, 9.0, accent_soft);
                    ui.border(dx + 14.0, y1, hw, 30.0, 9.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0, y1, hw - 8.0, 30.0],
                        accent,
                        st_lbl,
                    );

                    let area_txt = format!("Move Area: {}", js.move_area.label());
                    ui.rounded(dx + 14.0 + hw + 8.0, y1, hw, 30.0, 9.0, bg_hi);
                    ui.border(dx + 14.0 + hw + 8.0, y1, hw, 30.0, 9.0, border_c);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0 + hw + 8.0, y1, hw - 8.0, 30.0],
                        text_c,
                        &area_txt,
                    );

                    ui.text(
                        Face::SemiBold,
                        9.4,
                        dx + 14.0,
                        y1 + 38.0,
                        muted_c,
                        "STICK KNOB DIMENSIONS & STROKE",
                    );
                    let y2 = y1 + 52.0;
                    for (i, txt) in [
                        format!("Stick Sz {:.0}%", js.stick_size * 100.0),
                        format!("Stick Alpha {:.0}%", js.stick_opacity * 100.0),
                        format!("Stick Strk {:.1}dp", js.stick_stroke),
                        format!("Base Sz {:.0}%", p_move.size * 100.0),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        ui.rounded(rx, y2, cw, 28.0, 8.0, bg_hi);
                        ui.border(rx, y2, cw, 28.0, 8.0, border_c);
                        ui.text(Face::SemiBold, 11.0, rx + 5.0, y2 + 5.0, accent, "-");
                        ui.text(Face::SemiBold, 11.0, rx + cw - 11.0, y2 + 5.0, accent, "+");
                        ui.centered_fit(
                            Face::Medium,
                            9.0,
                            [rx + 12.0, y2, cw - 24.0, 28.0],
                            text_c,
                            &txt,
                        );
                    }

                    let y3 = y2 + 38.0;
                    ui.text(
                        Face::Medium,
                        9.5,
                        dx + 14.0,
                        y3 + 4.0,
                        muted_c,
                        "Stick Color:",
                    );
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 86.0 + i as f32 * (sw_w + 4.0);
                        ui.rounded(sx, y3, sw_w, 22.0, 6.0, *col);
                        if js.stick_color == *col {
                            ui.border(sx - 1.5, y3 - 1.5, sw_w + 3.0, 25.0, 7.0, accent);
                        }
                    }
                    ui.rounded(st_x, y3, st_w, 22.0, 6.0, bg_hi);
                    ui.border(st_x, y3, st_w, 22.0, 6.0, js.stick_stroke_color);
                    ui.centered_fit(
                        Face::Medium,
                        9.0,
                        [st_x, y3, st_w, 22.0],
                        text_c,
                        "Cycle Stick Stroke Col",
                    );
                }
                1 => {
                    // Joystick Tab 1: Analog Base Area Opacity, Stroke, Roundness, Mode & Colors
                    ui.text(
                        Face::SemiBold,
                        9.4,
                        dx + 14.0,
                        body_y - 2.0,
                        muted_c,
                        "ANALOG BASE RING GEOMETRY & INPUT MODE",
                    );
                    let y4 = body_y + 14.0;
                    for (i, txt) in [
                        format!("Base Alpha {:.0}%", js.base_opacity * 100.0),
                        format!("Base Strk {:.1}dp", js.base_stroke),
                        format!("Base Round {:.0}", js.base_roundness),
                        if tc.settings.mode == crate::touch::Mode::Dpad {
                            "Mode: D-Pad".into()
                        } else {
                            "Mode: Analog".into()
                        },
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        ui.rounded(rx, y4, cw, 30.0, 8.0, bg_hi);
                        ui.border(rx, y4, cw, 30.0, 8.0, border_c);
                        if i < 3 {
                            ui.text(Face::SemiBold, 11.0, rx + 5.0, y4 + 6.0, accent, "-");
                            ui.text(Face::SemiBold, 11.0, rx + cw - 11.0, y4 + 6.0, accent, "+");
                            ui.centered_fit(
                                Face::Medium,
                                9.0,
                                [rx + 12.0, y4, cw - 24.0, 30.0],
                                text_c,
                                &txt,
                            );
                        } else {
                            ui.centered_fit(
                                Face::SemiBold,
                                9.2,
                                [rx + 4.0, y4, cw - 8.0, 30.0],
                                accent,
                                &txt,
                            );
                        }
                    }

                    ui.text(
                        Face::SemiBold,
                        9.4,
                        dx + 14.0,
                        y4 + 40.0,
                        muted_c,
                        "ANALOG BASE FILL & STROKE PALETTE",
                    );
                    let y5 = y4 + 56.0;
                    ui.text(
                        Face::Medium,
                        9.5,
                        dx + 14.0,
                        y5 + 4.0,
                        muted_c,
                        "Base Color:",
                    );
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 86.0 + i as f32 * (sw_w + 4.0);
                        ui.rounded(sx, y5, sw_w, 22.0, 6.0, *col);
                        if js.base_color == *col {
                            ui.border(sx - 1.5, y5 - 1.5, sw_w + 3.0, 25.0, 7.0, accent);
                        }
                    }
                    ui.rounded(st_x, y5, st_w, 22.0, 6.0, bg_hi);
                    ui.border(st_x, y5, st_w, 22.0, 6.0, js.base_stroke_color);
                    ui.centered_fit(
                        Face::Medium,
                        9.0,
                        [st_x, y5, st_w, 22.0],
                        text_c,
                        "Cycle Base Stroke Col",
                    );
                }
                _ => {
                    // Joystick Tab 2: Active Zone & Visibility Rules
                    ui.text(
                        Face::SemiBold,
                        9.4,
                        dx + 14.0,
                        body_y - 2.0,
                        muted_c,
                        "ACTIVE TOUCH ZONE & VISIBILITY RULES",
                    );
                    let area_y = body_y + 14.0;
                    let hw = (dw - 28.0 - 8.0) * 0.5;
                    let area_txt = format!("Active Zone: {}", js.move_area.label());
                    ui.rounded(dx + 14.0, area_y, hw, 32.0, 9.0, bg_hi);
                    ui.border(dx + 14.0, area_y, hw, 32.0, 9.0, border_c);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0, area_y, hw - 8.0, 32.0],
                        text_c,
                        &area_txt,
                    );
                    let sz_txt = format!("Joystick Footprint: {:.0}%", p_move.size * 100.0);
                    let sz_x = dx + 14.0 + hw + 8.0;
                    ui.rounded(sz_x, area_y, hw, 32.0, 9.0, bg_hi);
                    ui.border(sz_x, area_y, hw, 32.0, 9.0, border_c);
                    ui.text(Face::SemiBold, 11.5, sz_x + 6.0, area_y + 7.0, accent, "-");
                    ui.text(
                        Face::SemiBold,
                        11.5,
                        sz_x + hw - 12.0,
                        area_y + 7.0,
                        accent,
                        "+",
                    );
                    ui.centered_fit(
                        Face::Medium,
                        9.5,
                        [sz_x + 16.0, area_y, hw - 32.0, 32.0],
                        text_c,
                        &sz_txt,
                    );

                    let y6 = area_y + 44.0;
                    let third = (dw - 28.0 - 8.0) / 3.0;
                    ui.rounded(dx + 14.0, y6, third, 32.0, 9.0, bg_hi);
                    ui.border(
                        dx + 14.0,
                        y6,
                        third,
                        32.0,
                        9.0,
                        if p_move.visible_in_game {
                            accent
                        } else {
                            border_c
                        },
                    );
                    ui.m3_checkbox(
                        dx + 20.0,
                        y6 + 7.0,
                        18.0,
                        p_move.visible_in_game,
                        "Visible in Game",
                    );

                    let vm_x = dx + 14.0 + third + 4.0;
                    ui.rounded(vm_x, y6, third, 32.0, 9.0, bg_hi);
                    ui.border(
                        vm_x,
                        y6,
                        third,
                        32.0,
                        9.0,
                        if p_move.visible_in_menu {
                            accent
                        } else {
                            border_c
                        },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        y6 + 7.0,
                        18.0,
                        p_move.visible_in_menu,
                        "Visible in Menu",
                    );

                    let pv_x = dx + 14.0 + (third + 4.0) * 2.0;
                    ui.rounded(pv_x, y6, third, 32.0, 9.0, accent_soft);
                    ui.border(pv_x, y6, third, 32.0, 9.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.5,
                        [pv_x + 4.0, y6, third - 8.0, 32.0],
                        accent,
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

            match ed.props_tab {
                0 => {
                    // Tab 0: Style Manager (MCPE Default vs GoyControl Static), Hint Text, Fill Color, Opacity, onClick & onToggle
                    let hw = (dw - 28.0 - 8.0) * 0.5;
                    let is_goy =
                        tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style;
                    ui.rounded(
                        dx + 14.0,
                        body_y,
                        hw,
                        28.0,
                        8.0,
                        if !is_goy { accent_soft } else { bg_hi },
                    );
                    ui.border(
                        dx + 14.0,
                        body_y,
                        hw,
                        28.0,
                        8.0,
                        if !is_goy { accent } else { border_c },
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 16.0, body_y, hw - 4.0, 28.0],
                        if !is_goy { accent } else { text_c },
                        "Style: Default (MCPE)",
                    );

                    ui.rounded(
                        dx + 14.0 + hw + 8.0,
                        body_y,
                        hw,
                        28.0,
                        8.0,
                        if is_goy { accent_soft } else { bg_hi },
                    );
                    ui.border(
                        dx + 14.0 + hw + 8.0,
                        body_y,
                        hw,
                        28.0,
                        8.0,
                        if is_goy { accent } else { border_c },
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 16.0 + hw + 8.0, body_y, hw - 4.0, 28.0],
                        if is_goy { accent } else { text_c },
                        "Style: Static (GoyControl)",
                    );

                    // Hint Text Row
                    let hy = body_y + 36.0;
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
                    let h_box_w = (dw - 28.0) * 0.54;
                    ui.rounded(dx + 14.0, hy, h_box_w, 28.0, 8.0, bg_hi);
                    ui.border(dx + 14.0, hy, h_box_w, 28.0, 8.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0, hy, h_box_w - 8.0, 28.0],
                        text_c,
                        &format!("Hint Text: \"{cur_hint}\" (Tap to Cycle)"),
                    );

                    let op_x = dx + 14.0 + h_box_w + 8.0;
                    let op_w = dw - 28.0 - h_box_w - 8.0;
                    ui.rounded(op_x, hy, op_w, 28.0, 8.0, bg_hi);
                    ui.border(op_x, hy, op_w, 28.0, 8.0, border_c);
                    ui.text(Face::SemiBold, 12.0, op_x + 6.0, hy + 5.0, accent, "-");
                    ui.text(
                        Face::SemiBold,
                        12.0,
                        op_x + op_w - 13.0,
                        hy + 5.0,
                        accent,
                        "+",
                    );
                    ui.centered_fit(
                        Face::Medium,
                        9.8,
                        [op_x + 16.0, hy, op_w - 32.0, 28.0],
                        text_c,
                        &format!("Transparency {:.0}%", p.opacity * 100.0),
                    );

                    // Button Fill Color Swatches
                    let cy1 = hy + 36.0;
                    ui.text(
                        Face::Medium,
                        9.5,
                        dx + 14.0,
                        cy1 + 4.0,
                        muted_c,
                        "Button Color:",
                    );
                    let sw_w = 24.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 98.0 + i as f32 * (sw_w + 5.0);
                        ui.rounded(sx, cy1, sw_w, 22.0, 6.0, *col);
                        if p.button_color == *col {
                            ui.border(sx - 1.5, cy1 - 1.5, sw_w + 3.0, 25.0, 7.0, accent);
                        }
                    }

                    // onClick & onToggle visual state color selectors
                    let cy2 = cy1 + 32.0;
                    let half = (dw - 28.0 - 8.0) * 0.5;
                    ui.rounded(dx + 14.0, cy2, half, 28.0, 8.0, p.on_click_color);
                    ui.border(dx + 14.0, cy2, half, 28.0, 8.0, p.on_click_stroke_color);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0, cy2, half - 8.0, 28.0],
                        text_c,
                        "onClick Style (Tap to Cycle)",
                    );

                    ui.rounded(
                        dx + 14.0 + half + 8.0,
                        cy2,
                        half,
                        28.0,
                        8.0,
                        p.on_toggle_color,
                    );
                    ui.border(
                        dx + 14.0 + half + 8.0,
                        cy2,
                        half,
                        28.0,
                        8.0,
                        p.on_toggle_stroke_color,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [dx + 18.0 + half + 8.0, cy2, half - 8.0, 28.0],
                        text_c,
                        "onToggle Style (Tap to Cycle)",
                    );

                    // Row 4: Visible in Game [✓] | Visible in Menu [✓] | Preset Visibility for All
                    let cy3 = cy2 + 34.0;
                    let third = (dw - 28.0 - 8.0) / 3.0;
                    ui.rounded(dx + 14.0, cy3, third, 28.0, 8.0, bg_hi);
                    ui.border(
                        dx + 14.0,
                        cy3,
                        third,
                        28.0,
                        8.0,
                        if p.visible_in_game { accent } else { border_c },
                    );
                    ui.m3_checkbox(
                        dx + 20.0,
                        cy3 + 5.0,
                        18.0,
                        p.visible_in_game,
                        "Visible in Game",
                    );

                    let vm_x = dx + 14.0 + third + 4.0;
                    ui.rounded(vm_x, cy3, third, 28.0, 8.0, bg_hi);
                    ui.border(
                        vm_x,
                        cy3,
                        third,
                        28.0,
                        8.0,
                        if p.visible_in_menu { accent } else { border_c },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        cy3 + 5.0,
                        18.0,
                        p.visible_in_menu,
                        "Visible in Menu",
                    );

                    let pv_x = dx + 14.0 + (third + 4.0) * 2.0;
                    ui.rounded(pv_x, cy3, third, 28.0, 8.0, accent_soft);
                    ui.border(pv_x, cy3, third, 28.0, 8.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.5,
                        [pv_x + 4.0, cy3, third - 8.0, 28.0],
                        accent,
                        "Preset Vis for All",
                    );
                }
                1 => {
                    // Tab 1: Stroke Size, Stroke Margin, Stroke Roundness, Stroke Color, Button Roundness, Shape, Shadow, Width/Height
                    let cw = (dw - 28.0 - 12.0) / 4.0;
                    for (i, txt) in [
                        format!("Stroke {:.1}dp", p.stroke_size),
                        format!("Margin {:.1}dp", p.stroke_margin),
                        format!("Strk Rnd {:.0}", p.stroke_roundness),
                        format!("Btn Rnd {:.0}", p.roundness),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        ui.rounded(rx, body_y, cw, 28.0, 7.0, bg_hi);
                        ui.border(rx, body_y, cw, 28.0, 7.0, border_c);
                        ui.text(Face::SemiBold, 11.5, rx + 5.0, body_y + 5.0, accent, "-");
                        ui.text(
                            Face::SemiBold,
                            11.5,
                            rx + cw - 11.0,
                            body_y + 5.0,
                            accent,
                            "+",
                        );
                        ui.centered_fit(
                            Face::Medium,
                            9.0,
                            [rx + 12.0, body_y, cw - 24.0, 28.0],
                            text_c,
                            &txt,
                        );
                    }

                    // Row 2: Shape (All 6 shapes), Shadow Size, Width Ratio, Height Ratio
                    let r2_y = body_y + 36.0;
                    for (i, txt) in [
                        format!("Shape: {}", shape_name(p.shape)),
                        format!("Shadow {:.1}dp", p.shadow_size),
                        format!("Width {:.0}%", p.width_ratio * 100.0),
                        format!("Height {:.0}%", p.height_ratio * 100.0),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        ui.rounded(rx, r2_y, cw, 28.0, 7.0, bg_hi);
                        ui.border(rx, r2_y, cw, 28.0, 7.0, border_c);
                        if i == 0 {
                            ui.centered_fit(
                                Face::SemiBold,
                                9.5,
                                [rx + 4.0, r2_y, cw - 8.0, 28.0],
                                accent,
                                &txt,
                            );
                        } else {
                            ui.text(Face::SemiBold, 11.5, rx + 5.0, r2_y + 5.0, accent, "-");
                            ui.text(
                                Face::SemiBold,
                                11.5,
                                rx + cw - 11.0,
                                r2_y + 5.0,
                                accent,
                                "+",
                            );
                            ui.centered_fit(
                                Face::Medium,
                                9.0,
                                [rx + 12.0, r2_y, cw - 24.0, 28.0],
                                text_c,
                                &txt,
                            );
                        }
                    }

                    // Row 3: Stroke Color Swatches
                    let r3_y = r2_y + 36.0;
                    ui.text(
                        Face::Medium,
                        9.5,
                        dx + 14.0,
                        r3_y + 4.0,
                        muted_c,
                        "Stroke Color:",
                    );
                    let sw_w = 24.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 98.0 + i as f32 * (sw_w + 5.0);
                        ui.rounded(sx, r3_y, sw_w, 22.0, 6.0, *col);
                        if p.stroke_color == *col {
                            ui.border(sx - 1.5, r3_y - 1.5, sw_w + 3.0, 25.0, 7.0, accent);
                        }
                    }
                }
                _ => {
                    // Tab 2: In MCPE mode, show Exclusive Single Binding (Keyboard OR Mouse);
                    // in GoyControl mode, show up to 3 Event Slots.
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
                        let half_w = (dw - 28.0 - 8.0) * 0.5;
                        for (ci, (title, status, active)) in [
                            ("KEYBOARD BINDING (EXCLUSIVE)", kb_lbl.as_str(), is_kb),
                            ("MOUSE / ACTION BINDING (EXCLUSIVE)", ms_lbl.as_str(), is_ms),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let ex = dx + 14.0 + ci as f32 * (half_w + 8.0);
                            ui.rounded(
                                ex,
                                body_y,
                                half_w,
                                42.0,
                                9.0,
                                if active { accent_soft } else { bg_hi },
                            );
                            ui.border(
                                ex,
                                body_y,
                                half_w,
                                42.0,
                                9.0,
                                if active { accent } else { border_c },
                            );
                            ui.centered_fit(
                                Face::SemiBold,
                                9.0,
                                [ex + 4.0, body_y + 4.0, half_w - 8.0, 16.0],
                                if active { accent } else { muted_c },
                                title,
                            );
                            ui.centered_fit(
                                Face::Medium,
                                10.2,
                                [ex + 4.0, body_y + 20.0, half_w - 8.0, 18.0],
                                if active { text_c } else { muted_c },
                                status,
                            );
                        }
                    } else {
                        let ew = (dw - 28.0 - 8.0) / 3.0;
                        for slot in 0..3 {
                            let ex = dx + 14.0 + slot as f32 * (ew + 4.0);
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
                            ui.rounded(ex, body_y, ew, 42.0, 9.0, accent_soft);
                            ui.border(ex, body_y, ew, 42.0, 9.0, accent);
                            ui.centered_fit(
                                Face::SemiBold,
                                9.5,
                                [ex + 4.0, body_y + 4.0, ew - 8.0, 16.0],
                                accent,
                                &format!("EVENT SLOT #{}", slot + 1),
                            );
                            ui.centered_fit(
                                Face::Medium,
                                10.5,
                                [ex + 4.0, body_y + 20.0, ew - 8.0, 18.0],
                                text_c,
                                &tok_name,
                            );
                        }
                    }

                    let by = body_y + 52.0;
                    let bw = (dw - 28.0 - 12.0) / 4.0;
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
                        let rx = dx + 14.0 + i as f32 * (bw + 4.0);
                        ui.rounded(
                            rx,
                            by,
                            bw,
                            30.0,
                            8.0,
                            if danger {
                                color::with_alpha(danger_c, 0.22)
                            } else if on {
                                accent_soft
                            } else {
                                bg_hi
                            },
                        );
                        ui.border(
                            rx,
                            by,
                            bw,
                            30.0,
                            8.0,
                            if danger {
                                danger_c
                            } else if on {
                                accent
                            } else {
                                border_c
                            },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            9.8,
                            [rx + 3.0, by, bw - 6.0, 30.0],
                            if danger {
                                danger_c
                            } else if on {
                                accent
                            } else {
                                text_c
                            },
                            lbl,
                        );
                    }

                    // Row 3 in Tab 2: Visible in Game [✓] | Visible in Menu [✓] | Preset Vis for All
                    let vy = by + 38.0;
                    let third = (dw - 28.0 - 8.0) / 3.0;
                    ui.rounded(dx + 14.0, vy, third, 28.0, 8.0, bg_hi);
                    ui.border(
                        dx + 14.0,
                        vy,
                        third,
                        28.0,
                        8.0,
                        if p.visible_in_game { accent } else { border_c },
                    );
                    ui.m3_checkbox(
                        dx + 20.0,
                        vy + 5.0,
                        18.0,
                        p.visible_in_game,
                        "Visible in Game",
                    );

                    let vm_x = dx + 14.0 + third + 4.0;
                    ui.rounded(vm_x, vy, third, 28.0, 8.0, bg_hi);
                    ui.border(
                        vm_x,
                        vy,
                        third,
                        28.0,
                        8.0,
                        if p.visible_in_menu { accent } else { border_c },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        vy + 5.0,
                        18.0,
                        p.visible_in_menu,
                        "Visible in Menu",
                    );

                    let pv_x = dx + 14.0 + (third + 4.0) * 2.0;
                    ui.rounded(pv_x, vy, third, 28.0, 8.0, accent_soft);
                    ui.border(pv_x, vy, third, 28.0, 8.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.5,
                        [pv_x + 4.0, vy, third - 8.0, 28.0],
                        accent,
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
                            let dx = p[0] - last_x;
                            if dx.abs() > 2.0 {
                                moved = true;
                            }
                            let dw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (dw - 28.0)).max(0.0);
                            ed.kb_scroll_x = (ed.kb_scroll_x - dx).clamp(0.0, max_kb_scroll);
                            if ended {
                                ed.active_drag = None;
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
                    let dh = 210.0;
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    let cw = (dw - 40.0) * 0.5;
                    if inside([dx + 16.0, dy + 62.0, cw, 82.0]) {
                        ed.picker_step = PickerStep::Keyboard { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 24.0 + cw, dy + 62.0, cw, 82.0]) {
                        ed.picker_step = PickerStep::MouseAndActions { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 16.0, dy + 156.0, cw, 36.0]) {
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
                    if inside([dx + 24.0 + cw, dy + 156.0, cw, 36.0]) || !inside([dx, dy, dw, dh]) {
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
                    let kb_top = dy + 40.0;
                    let kb_h = dh - 56.0;
                    let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                    let key_unit = 38.0;
                    let gap = 4.0;
                    for (r_idx, row) in KB_ROWS.iter().enumerate() {
                        let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                        let mut kx = dx + 14.0 - ed.kb_scroll_x;
                        for (lbl, tok, width_mult) in *row {
                            let kw = key_unit * (*width_mult) + (*width_mult - 1.0).max(0.0) * gap;
                            if inside([kx, ry, kw, row_h])
                                && p[0] >= dx + 10.0
                                && p[0] <= dx + dw - 10.0
                            {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                if let Some(idx) = ed.selected_custom
                                    && let Some(btn) = tc.settings.custom_buttons.get_mut(idx)
                                {
                                    if is_mcpe {
                                        // Strict single-binding exclusivity in MCPE mode:
                                        // selecting a keyboard binding clears any mouse binding.
                                        btn.set_exclusive_binding((*tok).to_owned());
                                        btn.custom_label = (*lbl).to_owned();
                                    } else {
                                        btn.set_event_slot(event_slot, (*tok).to_owned());
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
                                return false;
                            }
                            kx += kw + gap;
                        }
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

        // 3. If Component Property Inspector Dialog is open:
        if ed.props_open {
            let dw = (w - 24.0).clamp(310.0, 548.0);
            let dh = (h - 18.0).clamp(248.0, 368.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            if inside([dx + dw - 74.0, dy + 9.0, 60.0, 28.0]) || !inside([dx, dy, dw, dh]) {
                ed.props_open = false;
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }

            // Segmented 3-Tab Header clicks (shared by Joystick and Button inspectors)
            let tab_y = dy + 42.0;
            let tab_w = (dw - 28.0 - 8.0) / 3.0;
            for ti in 0..3 {
                let tx = dx + 14.0 + ti as f32 * (tab_w + 4.0);
                if inside([tx, tab_y, tab_w, 26.0]) {
                    ed.props_tab = ti;
                    tc.clicked = true;
                    return false;
                }
            }

            let body_y = tab_y + 34.0;
            let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
            if is_joystick {
                let mode_idx = tc.settings.mode.index();
                let slot = tc.settings.slot(Control::Move);
                let cw = (dw - 28.0 - 12.0) / 4.0;
                let sw_w = 22.0;
                let st_x = dx + 86.0 + 8.0 * (sw_w + 4.0) + 12.0;
                let st_w = (dw - (st_x - dx) - 14.0).max(80.0);

                match ed.props_tab {
                    0 => {
                        let y1 = body_y + 12.0;
                        let hw = (dw - 28.0 - 8.0) * 0.5;
                        if inside([dx + 14.0, y1, hw, 30.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].use_goy_style =
                                !tc.settings.layouts[mode_idx][slot].use_goy_style;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([dx + 14.0 + hw + 8.0, y1, hw, 30.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let y2 = y1 + 52.0;
                        for i in 0..4 {
                            let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                            if inside([rx, y2, cw, 28.0]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let sign = if p[0] < rx + cw * 0.5 { -1.0 } else { 1.0 };
                                match i {
                                    0 => {
                                        tc.settings.joystick_style.stick_size =
                                            (tc.settings.joystick_style.stick_size + sign * 0.05)
                                                .clamp(0.20, 0.85)
                                    }
                                    1 => {
                                        tc.settings.joystick_style.stick_opacity =
                                            (tc.settings.joystick_style.stick_opacity + sign * 0.10)
                                                .clamp(0.15, 1.0)
                                    }
                                    2 => {
                                        tc.settings.joystick_style.stick_stroke =
                                            (tc.settings.joystick_style.stick_stroke + sign * 0.5)
                                                .clamp(0.0, 8.0)
                                    }
                                    _ => {
                                        tc.settings.layouts[mode_idx][slot].size =
                                            (tc.settings.layouts[mode_idx][slot].size
                                                + sign * 0.025)
                                                .clamp(0.12, 0.45)
                                    }
                                }
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let y3 = y2 + 38.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = dx + 86.0 + i as f32 * (sw_w + 4.0);
                            if inside([sx, y3, sw_w, 22.0]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.stick_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        if inside([st_x, y3, st_w, 22.0]) {
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
                        let y4 = body_y + 14.0;
                        for i in 0..4 {
                            let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                            if inside([rx, y4, cw, 30.0]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let sign = if p[0] < rx + cw * 0.5 { -1.0 } else { 1.0 };
                                match i {
                                    0 => {
                                        tc.settings.joystick_style.base_opacity =
                                            (tc.settings.joystick_style.base_opacity + sign * 0.10)
                                                .clamp(0.15, 1.0)
                                    }
                                    1 => {
                                        tc.settings.joystick_style.base_stroke =
                                            (tc.settings.joystick_style.base_stroke + sign * 0.5)
                                                .clamp(0.0, 8.0)
                                    }
                                    2 => {
                                        tc.settings.joystick_style.base_roundness =
                                            (tc.settings.joystick_style.base_roundness + sign * 5.0)
                                                .clamp(0.0, 50.0)
                                    }
                                    _ => {
                                        tc.settings.mode = match tc.settings.mode {
                                            Mode::Dpad => Mode::Crosshair,
                                            Mode::Crosshair => Mode::Tap,
                                            Mode::Tap => Mode::Dpad,
                                        };
                                    }
                                }
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let y5 = y4 + 56.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = dx + 86.0 + i as f32 * (sw_w + 4.0);
                            if inside([sx, y5, sw_w, 22.0]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.base_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        if inside([st_x, y5, st_w, 22.0]) {
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
                        let area_y = body_y + 14.0;
                        let hw = (dw - 28.0 - 8.0) * 0.5;
                        if inside([dx + 14.0, area_y, hw, 32.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let sz_x = dx + 14.0 + hw + 8.0;
                        if inside([sz_x, area_y, hw, 32.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let sign = if p[0] < sz_x + hw * 0.5 { -1.0 } else { 1.0 };
                            tc.settings.layouts[mode_idx][slot].size =
                                (tc.settings.layouts[mode_idx][slot].size + sign * 0.025)
                                    .clamp(0.12, 0.45);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let y6 = area_y + 44.0;
                        let third = (dw - 28.0 - 8.0) / 3.0;
                        if inside([dx + 14.0, y6, third, 32.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].visible_in_game ^= true;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([dx + 14.0 + third + 4.0, y6, third, 32.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].visible_in_menu ^= true;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([dx + 14.0 + (third + 4.0) * 2.0, y6, third, 32.0]) {
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

            let body_y = tab_y + 34.0;
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
                    let hw = (dw - 28.0 - 8.0) * 0.5;
                    if inside([dx + 14.0, body_y, hw, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.use_goy_style = false);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 14.0 + hw + 8.0, body_y, hw, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.use_goy_style = true);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let hy = body_y + 36.0;
                    let h_box_w = (dw - 28.0) * 0.54;
                    if inside([dx + 14.0, hy, h_box_w, 28.0]) {
                        if let Some(idx) = ed.selected_custom {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let hints = [
                                "PRI", "SEC", "JUMP", "SNEAK", "SPRINT", "INV", "SWAP", "DROP",
                                "CHAT", "DEBUG", "3RD", "MOUSE", "MACRO", "CLICK",
                            ];
                            if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                let pos = hints
                                    .iter()
                                    .position(|h| *h == btn.custom_label)
                                    .map(|i| (i + 1) % hints.len())
                                    .unwrap_or(0);
                                btn.custom_label = hints[pos].to_owned();
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                        }
                        return false;
                    }
                    let op_x = dx + 14.0 + h_box_w + 8.0;
                    let op_w = dw - 28.0 - h_box_w - 8.0;
                    if inside([op_x, hy, op_w, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        let delta = if p[0] < op_x + op_w * 0.5 {
                            -0.10
                        } else {
                            0.10
                        };
                        mutate_placement(tc, &ed, &|pl| {
                            pl.opacity = (pl.opacity + delta).clamp(0.15, 1.0)
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let cy1 = hy + 36.0;
                    let sw_w = 24.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 98.0 + i as f32 * (sw_w + 5.0);
                        if inside([sx, cy1, sw_w, 22.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.button_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let cy2 = cy1 + 32.0;
                    let half = (dw - 28.0 - 8.0) * 0.5;
                    if inside([dx + 14.0, cy2, half, 28.0]) {
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
                    if inside([dx + 14.0 + half + 8.0, cy2, half, 28.0]) {
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
                    let cy3 = cy2 + 34.0;
                    let third = (dw - 28.0 - 8.0) / 3.0;
                    if inside([dx + 14.0, cy3, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_game = !pl.visible_in_game);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 14.0 + third + 4.0, cy3, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_menu = !pl.visible_in_menu);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 14.0 + (third + 4.0) * 2.0, cy3, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        tc.settings.apply_visibility_preset_all();
                        let _ = tc.settings.save();
                        ed.status_msg = Some("Applied preset visibility to all controls.".into());
                        tc.clicked = true;
                        return false;
                    }
                }
                1 => {
                    let cw = (dw - 28.0 - 12.0) / 4.0;
                    for i in 0..4 {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        if inside([rx, body_y, cw, 28.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let sign = if p[0] < rx + cw * 0.5 { -1.0 } else { 1.0 };
                            mutate_placement(tc, &ed, &move |pl| match i {
                                0 => pl.stroke_size = (pl.stroke_size + sign * 0.5).clamp(0.0, 8.0),
                                1 => {
                                    pl.stroke_margin =
                                        (pl.stroke_margin + sign * 1.0).clamp(-4.0, 8.0)
                                }
                                2 => {
                                    pl.stroke_roundness =
                                        (pl.stroke_roundness + sign * 3.0).clamp(0.0, 50.0)
                                }
                                _ => pl.roundness = (pl.roundness + sign * 3.0).clamp(0.0, 50.0),
                            });
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let r2_y = body_y + 36.0;
                    for i in 0..4 {
                        let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                        if inside([rx, r2_y, cw, 28.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let sign = if p[0] < rx + cw * 0.5 { -1.0 } else { 1.0 };
                            mutate_placement(tc, &ed, &move |pl| match i {
                                0 => pl.shape = (pl.shape + 1) % 6,
                                1 => {
                                    pl.shadow_size = (pl.shadow_size + sign * 1.0).clamp(0.0, 12.0)
                                }
                                2 => {
                                    pl.width_ratio = (pl.width_ratio + sign * 0.10).clamp(0.45, 3.0)
                                }
                                _ => {
                                    pl.height_ratio =
                                        (pl.height_ratio + sign * 0.10).clamp(0.45, 3.0)
                                }
                            });
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let r3_y = r2_y + 36.0;
                    let sw_w = 24.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = dx + 98.0 + i as f32 * (sw_w + 5.0);
                        if inside([sx, r3_y, sw_w, 22.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.stroke_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                }
                _ => {
                    if tc.settings.control_style == ControlStyle::Mcpe
                        && ed.selected_custom.is_some()
                    {
                        let half_w = (dw - 28.0 - 8.0) * 0.5;
                        // Left card: Exclusive Keyboard Binding
                        if inside([dx + 14.0, body_y, half_w, 42.0]) {
                            ed.picker_step = PickerStep::Keyboard { event_slot: 0 };
                            tc.clicked = true;
                            return false;
                        }
                        // Right card: Exclusive Mouse / Action Binding
                        if inside([dx + 14.0 + half_w + 8.0, body_y, half_w, 42.0]) {
                            ed.picker_step = PickerStep::MouseAndActions { event_slot: 0 };
                            tc.clicked = true;
                            return false;
                        }
                    } else {
                        let ew = (dw - 28.0 - 8.0) / 3.0;
                        for slot in 0..3 {
                            let ex = dx + 14.0 + slot as f32 * (ew + 4.0);
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
                    let bw = (dw - 28.0 - 12.0) / 4.0;
                    for i in 0..4 {
                        let rx = dx + 14.0 + i as f32 * (bw + 4.0);
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
                    let third = (dw - 28.0 - 8.0) / 3.0;
                    if inside([dx + 14.0, vy, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_game = !pl.visible_in_game);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 14.0 + third + 4.0, vy, third, 28.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_menu = !pl.visible_in_menu);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 14.0 + (third + 4.0) * 2.0, vy, third, 28.0]) {
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
}
