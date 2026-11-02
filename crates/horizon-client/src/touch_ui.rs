//! Android menu gestures and auxiliary keys, independent of gameplay fingers.
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    osk::{Keyboard, Output},
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use winit::{event::TouchPhase, keyboard::KeyCode};

#[derive(Default)]
pub struct TouchUi {
    pub expanded: bool,
    pub floating_menu: bool,
    pub virtual_mouse: bool,
    mouse_buttons: BTreeMap<u64, bool>,
    relative: Option<RelativeTouch>,
    relative_click: Option<bool>,
    pub keyboard: Option<Keyboard>,
    gesture: Option<(u64, [f32; 2], [f32; 2], bool)>,
    pointer_drag: Option<u64>,
    held_keys: BTreeMap<u64, KeyCode>,
    pub f3_latched: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InGameMenuAction {
    Close,
    OpenControlEditor,
    ApplyPreset(&'static str),
    CycleMovementMode,
    ToggleSplitTouch,
    ToggleLeftHanded,
    ToggleGyro,
    ToggleVirtualMouse,
    StepTouchSensitivity(i32),
    StepButtonScale(i32),
    StepButtonOpacity(i32),
    TriggerKey(KeyCode),
    OpenKeyboard,
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

// Shared draw/hit rectangles: utility-style buttons at the lower right.
fn mouse_button_rects(w: f32, h: f32) -> [([f32; 4], bool); 2] {
    [
        ([w - 53.0, h - 51.0, 50.0, 20.0], false),
        ([w - 53.0, h - 27.0, 50.0, 20.0], true),
    ]
}

/// Shared outlined pointer for virtual mouse and controller menus.
pub fn draw_pointer(batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32)) {
    let (x, y) = (cursor.0, cursor.1);
    const ROWS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 4, 2, 1];
    for (i, &w) in ROWS.iter().enumerate() {
        batch.fill(
            atlas,
            x - 1.0,
            y + i as f32,
            f32::from(w) + 2.0,
            1.0,
            rgba(0, 0, 0, 255),
        );
    }
    batch.fill(atlas, x - 1.0, y + 10.0, 2.0, 1.0, rgba(0, 0, 0, 255));
    for (i, &w) in ROWS.iter().enumerate().skip(1).take(8) {
        batch.fill(
            atlas,
            x,
            y + i as f32,
            f32::from(w) - 1.0,
            1.0,
            rgba(255, 255, 255, 255),
        );
    }
}

impl TouchUi {
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
    }
    pub fn cancel_relative(&mut self) {
        self.relative = None;
        self.relative_click = None;
    }
    pub fn owns_relative(&self, id: u64) -> bool {
        self.relative.as_ref().is_some_and(|t| t.id == id)
    }
    pub fn owns_gesture(&self, id: u64) -> bool {
        self.gesture.as_ref().is_some_and(|(owner, _, _, _)| *owner == id)
            || self.pointer_drag == Some(id)
    }
    /// Relative motion: touching down does not teleport the inventory cursor.
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
        let step = (point[0] - touch.last[0]).hypot(point[1] - touch.last[1]);
        // Apply subtle desktop-like pointer acceleration on real GUI viewports (>250dp)
        // while keeping synthetic unit test viewports (e.g. 200x200) exact 1:1.
        let accel = if size[0] > 250.0 {
            (1.15 + (step * 0.085).min(1.05)).clamp(1.0, 2.2)
        } else {
            1.0
        };
        let position = [0, 1].map(|i| {
            (touch.cursor[i] + (point[i] - touch.last[i]) * sensitivity * accel)
                .clamp(0.0, (size[i] - 1.0).max(0.0))
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
        if menu && self.virtual_mouse {
            for ([x, y, width, height], right) in mouse_button_rects(w, h) {
                if (x..x + width).contains(&p[0]) && (y..y + height).contains(&p[1]) {
                    return Some(Action::MouseButton(right));
                }
            }
        }
        if menu && (3.0..23.0).contains(&p[1]) && (w - 108.0..w - 56.0).contains(&p[0]) {
            return Some(Action::VirtualMouse);
        }
        if (3.0..23.0).contains(&p[1]) {
            if menu && (w - 53.0..w - 3.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::Escape));
            }
            if (3.0..36.0).contains(&p[0]) {
                return Some(Action::More);
            }
            if self.expanded && (39.0..72.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F3));
            }
            if self.expanded && (75.0..108.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F5));
            }
            if self.expanded && (111.0..144.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::KeyT));
            }
            if self.expanded && (147.0..186.0).contains(&p[0]) {
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
        if !self.expanded {
            button(b, a, "=", 39.0, 3.0, 28.0);
        }
        for (label, x, width) in [
            ("...", 3.0, 33.0),
            ("F3", 39.0, 33.0),
            ("F5", 75.0, 33.0),
            ("Chat", 111.0, 33.0),
            ("Keys", 147.0, 39.0),
        ] {
            if label != "..." && !self.expanded {
                continue;
            }
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
        if menu {
            button(
                b,
                a,
                if self.virtual_mouse {
                    "Mouse ON"
                } else {
                    "Mouse"
                },
                b.width - 108.0,
                3.0,
                50.0,
            );
        }
        if menu && self.virtual_mouse {
            for ([x, y, width, _], right) in mouse_button_rects(b.width, b.height) {
                button(b, a, if right { "RMB" } else { "LMB" }, x, y, width);
            }
        }
        if menu {
            button(b, a, "Back", b.width - 53.0, 3.0, 50.0);
        }
        if self.expanded {
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


impl TouchUi {
    /// Checks if the user tapped the '=' floating menu button or inside the open floating menu.
    pub fn floating_menu_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        p_gui: [f32; 2],
        settings: &crate::touch::Settings,
    ) -> Option<InGameMenuAction> {
        if !self.expanded && !self.floating_menu && (3.0..23.0).contains(&p_gui[1]) && (39.0..67.0).contains(&p_gui[0]) {
            self.floating_menu = true;
            return Some(InGameMenuAction::Close);
        }
        if !self.floating_menu {
            return None;
        }
        let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
        let p = [
            p_gui[0] * ui.batch.scale / ui.density,
            p_gui[1] * ui.batch.scale / ui.density,
        ];
        if (3.0..23.0).contains(&p_gui[1]) && (39.0..67.0).contains(&p_gui[0]) {
            self.floating_menu = false;
            return Some(InGameMenuAction::Close);
        }
        let (w, h) = (ui.w, ui.h);
        let dw = (w - 32.0).clamp(280.0, 500.0);
        let dh = (h - 24.0).clamp(230.0, 356.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        let inside = |r: [f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };
        if !inside([dx, dy, dw, dh]) {
            self.floating_menu = false;
            return Some(InGameMenuAction::Close);
        }
        // Header close button & Open HUD Editor button
        if inside([dx + dw - 76.0, dy + 10.0, 64.0, 28.0]) {
            self.floating_menu = false;
            return Some(InGameMenuAction::Close);
        }
        if inside([dx + dw - 208.0, dy + 10.0, 126.0, 28.0]) {
            self.floating_menu = false;
            return Some(InGameMenuAction::OpenControlEditor);
        }
        // Row 1: Control Presets
        let py = dy + 62.0;
        let pw = (dw - 28.0 - 16.0) / 5.0;
        let presets = ["default", "pojav", "pvp", "building", "speedrun"];
        for (i, id) in presets.iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            if inside([rx, py, pw, 28.0]) {
                return Some(InGameMenuAction::ApplyPreset(*id));
            }
        }
        // Row 2: Movement & HUD toggles
        let ty = py + 48.0;
        let tw = (dw - 28.0 - 12.0) / 4.0;
        if inside([dx + 14.0, ty, tw, 30.0]) {
            return Some(InGameMenuAction::CycleMovementMode);
        }
        if inside([dx + 14.0 + (tw + 4.0), ty, tw, 30.0]) {
            return Some(InGameMenuAction::ToggleSplitTouch);
        }
        if inside([dx + 14.0 + (tw + 4.0) * 2.0, ty, tw, 30.0]) {
            return Some(InGameMenuAction::ToggleGyro);
        }
        if inside([dx + 14.0 + (tw + 4.0) * 3.0, ty, tw, 30.0]) {
            return Some(InGameMenuAction::ToggleVirtualMouse);
        }
        // Row 3: Stepper controls (Sensitivity, Button Size, Opacity, Left-Handed)
        let sy = ty + 48.0;
        let sw = (dw - 28.0 - 12.0) / 4.0;
        // Sensitivity - / +
        let s0 = dx + 14.0;
        if inside([s0, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepTouchSensitivity(-1));
        }
        if inside([s0 + sw * 0.5, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepTouchSensitivity(1));
        }
        // Button Scale - / +
        let s1 = dx + 14.0 + (sw + 4.0);
        if inside([s1, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepButtonScale(-1));
        }
        if inside([s1 + sw * 0.5, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepButtonScale(1));
        }
        // Opacity - / +
        let s2 = dx + 14.0 + (sw + 4.0) * 2.0;
        if inside([s2, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepButtonOpacity(-1));
        }
        if inside([s2 + sw * 0.5, sy, sw * 0.5, 30.0]) {
            return Some(InGameMenuAction::StepButtonOpacity(1));
        }
        // Left-Handed
        let s3 = dx + 14.0 + (sw + 4.0) * 3.0;
        if inside([s3, sy, sw, 30.0]) {
            return Some(InGameMenuAction::ToggleLeftHanded);
        }
        // Row 4: Quick Keys (F1, F3, F5, Chat, Inv, Keyboard)
        let ky = sy + 48.0;
        let kw = (dw - 28.0 - 20.0) / 6.0;
        let quick = [
            KeyCode::F1,
            KeyCode::F3,
            KeyCode::F5,
            KeyCode::KeyE,
            KeyCode::KeyT,
        ];
        for (i, key) in quick.iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (kw + 4.0);
            if inside([rx, ky, kw, 28.0]) {
                self.floating_menu = false;
                return Some(InGameMenuAction::TriggerKey(*key));
            }
        }
        let rx_kb = dx + 14.0 + 5.0 * (kw + 4.0);
        if inside([rx_kb, ky, kw, 28.0]) {
            self.floating_menu = false;
            return Some(InGameMenuAction::OpenKeyboard);
        }
        let _ = settings;
        Some(InGameMenuAction::Close)
    }

    /// Draws the Material You 3 Floating '=' Quick Launcher & HUD Options Dialog in-game.
    pub fn draw_floating_menu(
        &self,
        b: &mut HudBatch,
        a: &HudAtlas,
        settings: &crate::touch::Settings,
    ) {
        if !self.floating_menu {
            return;
        }
        use crate::ui_theme::{Face, Icon, Ui, palette};
        let mut ui = Ui::mobile(a, b, (0.0, 0.0));
        let p = palette();
        let (w, h) = (ui.w, ui.h);
        ui.fill(0.0, 0.0, w, h, rgba(4, 6, 10, 175));
        let dw = (w - 32.0).clamp(280.0, 500.0);
        let dh = (h - 24.0).clamp(230.0, 356.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, rgba(16, 20, 28, 250));
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.2, p.border_light);
        ui.rounded(dx + 12.0, dy + 10.0, 28.0, 28.0, 9.0, p.accent_soft);
        ui.icon(Icon::MenuEquals, dx + 17.0, dy + 15.0, 18.0, p.accent);
        ui.label("Horizon Floating Quick Menu", dx + 48.0, dy + 15.0, 14.0, p.text, Face::SemiBold);

        // Edit HUD Layout & Close buttons
        ui.rounded(dx + dw - 208.0, dy + 10.0, 126.0, 28.0, 9.0, p.accent);
        ui.centered_fit("Edit HUD Controls", [dx + dw - 204.0, dy + 10.0, 118.0, 28.0], 11.5, rgba(10, 14, 20, 255), Face::SemiBold);
        ui.rounded(dx + dw - 76.0, dy + 10.0, 64.0, 28.0, 9.0, p.card_hi);
        ui.stroke_rounded(dx + dw - 76.0, dy + 10.0, 64.0, 28.0, 9.0, 1.0, p.border);
        ui.centered_fit("Close", [dx + dw - 74.0, dy + 10.0, 60.0, 28.0], 11.5, p.text, Face::Medium);

        // Section 1: Control Presets
        ui.label("CONTROL PRESET (INSTANT LAYOUT)", dx + 14.0, dy + 46.0, 9.5, p.accent, Face::SemiBold);
        let py = dy + 62.0;
        let pw = (dw - 28.0 - 16.0) / 5.0;
        let presets = [
            ("default", "Pocket PE"),
            ("pojav", "Pojav"),
            ("pvp", "PvP Claw"),
            ("building", "Builder"),
            ("speedrun", "Speedrun"),
        ];
        for (i, (id, title)) in presets.iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            let active = settings.active_profile.eq_ignore_ascii_case(id)
                || (*id == "default" && settings.active_profile.eq_ignore_ascii_case("pocket"));
            ui.rounded(rx, py, pw, 28.0, 8.0, if active { p.accent_soft } else { p.card_hi });
            ui.stroke_rounded(rx, py, pw, 28.0, 8.0, 1.0, if active { p.accent } else { p.border });
            ui.centered_fit(title, [rx + 3.0, py, pw - 6.0, 28.0], 10.5, if active { p.accent } else { p.text }, Face::SemiBold);
        }

        // Section 2: Movement & HUD Mode
        ui.label("MOVEMENT & INPUT OPTIONS", dx + 14.0, py + 34.0, 9.5, p.accent, Face::SemiBold);
        let ty = py + 48.0;
        let tw = (dw - 28.0 - 12.0) / 4.0;
        let move_lbl = match settings.movement_mode() {
            crate::touch::MovementMode::Dpad => "Move: D-Pad",
            crate::touch::MovementMode::Thumbstick => "Move: Joystick",
            crate::touch::MovementMode::FloatingThumbstick => "Move: Floating",
        };
        let split_on = settings.split_controls;
        for (i, (lbl, active)) in [
            (move_lbl, true),
            (if split_on { "Split Touch: ON" } else { "Split Touch: OFF" }, split_on),
            (if settings.gyroscope { "Gyro Aim: ON" } else { "Gyro Aim: OFF" }, settings.gyroscope),
            (if self.virtual_mouse { "Mouse: ON" } else { "Mouse: OFF" }, self.virtual_mouse),
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (tw + 4.0);
            ui.rounded(rx, ty, tw, 30.0, 8.0, if active { p.accent_soft } else { p.card_hi });
            ui.stroke_rounded(rx, ty, tw, 30.0, 8.0, 1.0, if active { p.accent } else { p.border });
            ui.centered_fit(lbl, [rx + 4.0, ty, tw - 8.0, 30.0], 10.5, if active { p.accent } else { p.text }, Face::Medium);
        }

        // Section 3: Fine Tuning Steppers
        ui.label("SENSITIVITY, SCALE & OPACITY (- / +)", dx + 14.0, ty + 34.0, 9.5, p.accent, Face::SemiBold);
        let sy = ty + 48.0;
        let sw = (dw - 28.0 - 12.0) / 4.0;
        for (i, txt) in [
            format!("Sens {:.0}%", settings.sensitivity * 100.0),
            format!("Scale {:.0}%", settings.button_scale * 100.0),
            format!("Alpha {:.0}%", settings.current_opacity() * 100.0),
            if settings.left_handed { "Left-Hand: ON".to_owned() } else { "Left-Hand: OFF".to_owned() },
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (sw + 4.0);
            ui.rounded(rx, sy, sw, 30.0, 8.0, p.card_hi);
            ui.stroke_rounded(rx, sy, sw, 30.0, 8.0, 1.0, p.border);
            if i < 3 {
                ui.label("-", rx + 6.0, sy + 7.0, 12.0, p.accent, Face::SemiBold);
                ui.label("+", rx + sw - 13.0, sy + 7.0, 12.0, p.accent, Face::SemiBold);
                ui.centered_fit(&txt, [rx + 14.0, sy, sw - 28.0, 30.0], 10.0, p.text, Face::Medium);
            } else {
                ui.centered_fit(&txt, [rx + 4.0, sy, sw - 8.0, 30.0], 10.0, if settings.left_handed { p.accent } else { p.text }, Face::Medium);
            }
        }

        // Section 4: Quick In-Game Shortcuts
        ui.label("QUICK DESKTOP KEYS", dx + 14.0, sy + 34.0, 9.5, p.accent, Face::SemiBold);
        let ky = sy + 48.0;
        let kw = (dw - 28.0 - 20.0) / 6.0;
        for (i, lbl) in ["F1 HUD", "F3 Info", "F5 View", "Inv (E)", "Chat (T)", "Keyboard"].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (kw + 4.0);
            ui.rounded(rx, ky, kw, 28.0, 8.0, p.card_hi);
            ui.stroke_rounded(rx, ky, kw, 28.0, 8.0, 1.0, p.border);
            ui.centered_fit(lbl, [rx + 3.0, ky, kw - 6.0, 28.0], 10.0, p.text, Face::Medium);
        }
        ui.finish();
    }
}

fn control_display_info(c: crate::touch::Control, mode: crate::touch::MovementMode) -> (&'static str, &'static str) {
    use crate::touch::{Control, MovementMode};
    match c {
        Control::Move => match mode {
            MovementMode::Dpad => ("D-PAD", "Move D-Pad"),
            MovementMode::Thumbstick => ("STICK", "Move Joystick"),
            MovementMode::FloatingThumbstick => ("FLOAT", "Floating Stick"),
        },
        Control::Jump => ("JUMP", "Jump"),
        Control::Sneak => ("SNEAK", "Sneak / Crouch"),
        Control::Sprint => ("RUN", "Sprint"),
        Control::Attack => ("ATK", "Attack / Break"),
        Control::Use => ("USE", "Use / Place"),
        Control::Inventory => ("INV", "Inventory (E)"),
        Control::Chat => ("CHAT", "Chat (T)"),
        Control::Pause => ("ESC", "Pause Menu"),
        Control::Settings => ("HUD", "HUD Settings"),
        Control::Debug => ("F3", "Debug (F3)"),
        Control::Perspective => ("F5", "Camera (F5)"),
        Control::Offhand => ("SWAP", "Offhand (F)"),
        Control::Shield => ("SHLD", "Shield / Block"),
    }
}

/// Renders the full Material You 3 + Pocket Edition Touch Control Editor using TrueType fonts
/// and procedural Pocket Edition control shapes (works both in the Launcher and In-Game).
pub fn draw_hud_editor(
    tc: &crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    profile_name: &str,
    notice: Option<&str>,
) {
    use crate::touch::{Control, MovementMode, Target};
    use crate::ui_theme::{Face, Icon, Ui, palette};
    let Some(ed) = &tc.editor else {
        return;
    };
    let mut ui = Ui::mobile(a, b, (0.0, 0.0));
    let p = palette();
    let (w, h) = (ui.w, ui.h);

    // 1. Blueprint background & 5% alignment grid
    ui.fill(0.0, 0.0, w, h, rgba(9, 12, 18, 232));
    let step = if tc.settings.editor_grid { 0.05 } else { 0.10 };
    let mut g = step;
    while g < 1.0 {
        let gx = w * g;
        let gy = h * g;
        ui.fill(gx, 0.0, 1.0, h, rgba(255, 255, 255, 13));
        ui.fill(0.0, gy, w, 1.0, rgba(255, 255, 255, 13));
        g += step;
    }
    // Center crosshairs
    ui.fill(w * 0.5, 0.0, 1.2, h, rgba(168, 199, 250, 42));
    ui.fill(0.0, h * 0.5, w, 1.2, rgba(168, 199, 250, 42));

    // Hotbar reference guide at bottom center
    let hbw = (w * 0.38).clamp(180.0, 320.0);
    let hbh = 26.0;
    let hbx = (w - hbw) * 0.5;
    let hby = h - hbh - 6.0;
    ui.rounded(hbx, hby, hbw, hbh, 6.0, rgba(20, 26, 36, 150));
    ui.stroke_rounded(hbx, hby, hbw, hbh, 6.0, 1.0, rgba(255, 255, 255, 45));
    let slot_w = hbw / 9.0;
    for i in 1..9 {
        ui.fill(hbx + i as f32 * slot_w, hby + 3.0, 1.0, hbh - 6.0, rgba(255, 255, 255, 28));
    }
    ui.centered_fit("Hotbar Reference (9 Slots)", [hbx, hby, hbw, hbh], 9.5, rgba(210, 220, 238, 165), Face::Medium);

    // 2. Draw all 14 Built-in Default Pocket Edition Controls
    for c in Control::ALL {
        let vis = tc.visible(c);
        if !vis && !ed.show_hidden {
            continue;
        }
        let [nx, ny, nw, nh] = tc.placement(c).rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let selected = ed.target == Target::BuiltIn(c);
        let layout = tc.settings.layout_placement(c);
        let alpha_scale = if vis { layout.opacity.clamp(0.25, 1.0) } else { 0.42 };
        let (short, full_name) = control_display_info(c, tc.settings.movement_mode());

        if c == Control::Move {
            match tc.settings.movement_mode() {
                MovementMode::Dpad => {
                    // Iconic Pocket Edition 5-button cross D-Pad
                    let tw = rw / 3.0;
                    let th = rh / 3.0;
                    let bg = rgba(26, 32, 44, (215.0 * alpha_scale) as u8);
                    let bd = if selected { p.accent } else { rgba(220, 230, 248, (150.0 * alpha_scale) as u8) };
                    for (gx, gy, sym) in [
                        (1, 0, "▲"),
                        (0, 1, "◀"),
                        (1, 1, "●"),
                        (2, 1, "▶"),
                        (1, 2, "▼"),
                    ] {
                        let bx = rx + gx as f32 * tw + 1.5;
                        let by = ry + gy as f32 * th + 1.5;
                        let bw = (tw - 3.0).max(8.0);
                        let bh = (th - 3.0).max(8.0);
                        ui.rounded(bx, by, bw, bh, 6.0, bg);
                        ui.stroke_rounded(bx, by, bw, bh, 6.0, 1.2, bd);
                        ui.centered_fit(sym, [bx, by, bw, bh], (bh * 0.42).clamp(10.0, 18.0), p.text, Face::SemiBold);
                    }
                }
                MovementMode::Thumbstick | MovementMode::FloatingThumbstick => {
                    let rad = (rw.min(rh) * 0.48).max(12.0);
                    ui. rounded(rx, ry, rw, rh, rad, rgba(24, 30, 42, (195.0 * alpha_scale) as u8));
                    ui.stroke_rounded(rx, ry, rw, rh, rad, 1.5, if selected { p.accent } else { rgba(210, 224, 246, 150) });
                    let kw = rw * 0.42;
                    let kh = rh * 0.42;
                    ui.rounded(rx + (rw - kw) * 0.5, ry + (rh - kh) * 0.5, kw, kh, kw * 0.48, rgba(168, 199, 250, (195.0 * alpha_scale) as u8));
                }
            }
            ui.centered_fit(full_name, [rx, ry + rh - 16.0, rw, 14.0], 10.0, p.accent, Face::SemiBold);
        } else {
            let is_top = matches!(c, Control::Inventory | Control::Chat | Control::Pause | Control::Settings | Control::Debug | Control::Perspective);
            let rad = if is_top { 8.0 } else { (rw.min(rh) * 0.34).clamp(8.0, 26.0) };
            let fill_c = if selected {
                rgba(36, 54, 84, 235)
            } else if !vis {
                rgba(24, 26, 34, 135)
            } else {
                rgba(26, 32, 44, (215.0 * alpha_scale) as u8)
            };
            let stroke_c = if selected {
                p.accent
            } else if !vis {
                rgba(180, 188, 204, 95)
            } else {
                rgba(218, 228, 245, (155.0 * alpha_scale) as u8)
            };
            ui.rounded(rx, ry, rw, rh, rad, fill_c);
            ui.stroke_rounded(rx, ry, rw, rh, rad, if selected { 2.0 } else { 1.2 }, stroke_c);
            let font_sz = (rh * 0.32).clamp(9.5, 14.0);
            ui.centered_fit(short, [rx + 2.0, ry + 2.0, (rw - 4.0).max(8.0), (rh - 4.0).max(8.0)], font_sz, if vis { p.text } else { p.muted }, Face::SemiBold);
            if !vis {
                ui.centered_fit("HIDDEN", [rx, ry + rh - 12.0, rw, 11.0], 8.0, p.warn, Face::SemiBold);
            }
        }

        if selected {
            ui.stroke_rounded(rx - 2.0, ry - 2.0, rw + 4.0, rh + 4.0, 10.0, 2.0, p.accent);
            for (hx, hy) in [(rx - 3.0, ry - 3.0), (rx + rw - 3.0, ry - 3.0), (rx - 3.0, ry + rh - 3.0), (rx + rw - 3.0, ry + rh - 3.0)] {
                ui.rounded(hx, hy, 6.0, 6.0, 2.0, p.accent);
            }
            let badge = format!("{full_name} ({:.0}%)", layout.scale * 100.0);
            let bw = (ui.width(&badge, 10.0, Face::SemiBold) + 12.0).min(w - 16.0);
            let bx = (rx + (rw - bw) * 0.5).clamp(8.0, (w - bw - 8.0).max(8.0));
            let by = if ry > 26.0 { ry - 20.0 } else { (ry + rh + 4.0).min(h - 20.0) };
            ui.rounded(bx, by, bw, 17.0, 5.0, rgba(14, 19, 28, 235));
            ui.stroke_rounded(bx, by, bw, 17.0, 5.0, 1.0, p.accent);
            ui.centered_fit(&badge, [bx + 3.0, by, bw - 6.0, 17.0], 9.5, p.accent, Face::SemiBold);
        }
    }

    // 3. Draw all Custom Key / Action Buttons
    for (i, custom) in tc.settings.custom_buttons.iter().enumerate() {
        let [nx, ny, nw, nh] = tc.custom_placement(i).rect(tc.aspect());
        let layout = custom.layouts[tc.settings.mode.index()];
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let selected = ed.target == Target::Custom(i);
        let rad = (rw.min(rh) * 0.28).clamp(8.0, 20.0);
        let fill_c = if selected {
            rgba(40, 62, 96, 240)
        } else {
            rgba(30, 38, 54, (220.0 * layout.opacity.clamp(0.25, 1.0)) as u8)
        };
        ui.rounded(rx, ry, rw, rh, rad, fill_c);
        ui.stroke_rounded(rx, ry, rw, rh, rad, if selected { 2.2 } else { 1.3 }, if selected { p.accent } else { p.border_light });
        ui.centered_fit(&custom.label(), [rx + 3.0, ry + 2.0, (rw - 6.0).max(8.0), (rh - 10.0).max(10.0)], (rh * 0.30).clamp(9.5, 13.5), p.text, Face::SemiBold);

        // Property mini-tags (TOG / PASS / SWP)
        let mut tags = String::new();
        if layout.toggleable { tags.push_str("TOG "); }
        if layout.passthrough { tags.push_str("PASS "); }
        if layout.swipeable { tags.push_str("SWP"); }
        let tags = tags.trim();
        if !tags.is_empty() {
            ui.centered_fit(tags, [rx + 2.0, ry + rh - 12.0, (rw - 4.0).max(8.0), 10.0], 7.5, p.accent, Face::SemiBold);
        }
        if selected {
            for (hx, hy) in [(rx - 3.0, ry - 3.0), (rx + rw - 3.0, ry - 3.0), (rx - 3.0, ry + rh - 3.0), (rx + rw - 3.0, ry + rh - 3.0)] {
                ui.rounded(hx, hy, 6.0, 6.0, 2.0, p.accent);
            }
        }
    }

    // 4. Floating '=' Menu Button + Compact Top Quick Pill Bar
    let eq_rect = [10.0, 8.0, 38.0, 34.0];
    ui.rounded(eq_rect[0], eq_rect[1], eq_rect[2], eq_rect[3], 10.0, if ed.menu_open { p.accent } else { rgba(18, 23, 33, 240) });
    ui.stroke_rounded(eq_rect[0], eq_rect[1], eq_rect[2], eq_rect[3], 10.0, 1.2, p.accent);
    ui.icon(Icon::MenuEquals, eq_rect[0] + 10.0, eq_rect[1] + 8.0, 18.0, if ed.menu_open { rgba(10, 14, 20, 255) } else { p.accent });

    // Top quick bar next to '=' button
    let bar_x = 54.0;
    let bar_w = (w - bar_x - 10.0).max(220.0);
    ui.rounded(bar_x, 8.0, bar_w, 34.0, 10.0, rgba(16, 20, 29, 232));
    ui.stroke_rounded(bar_x, 8.0, bar_w, 34.0, 10.0, 1.0, p.border);

    let sel_title = match ed.target {
        Target::BuiltIn(c) => control_display_info(c, tc.settings.movement_mode()).1.to_owned(),
        Target::Custom(i) => tc.settings.custom_buttons.get(i).map(|b| format!("Custom: {}", b.label())).unwrap_or_else(|| "Custom".to_owned()),
    };
    let status_txt = if let Some(msg) = notice.filter(|s| !s.is_empty()) {
        msg.to_owned()
    } else {
        format!("{profile_name}  •  Selected: {sel_title}")
    };
    ui.label_fit(&status_txt, bar_x + 10.0, 18.0, (bar_w - 298.0).max(60.0), 11.0, p.text, Face::SemiBold);

    // Quick buttons on right side of top bar: [Size -] [Size +] [+ Key] [Options (=)] [Done]
    let q_done = [bar_x + bar_w - 64.0, 12.0, 58.0, 26.0];
    let q_opts = [q_done[0] - 68.0, 12.0, 64.0, 26.0];
    let q_add  = [q_opts[0] - 62.0, 12.0, 58.0, 26.0];
    let q_plus = [q_add[0] - 34.0, 12.0, 30.0, 26.0];
    let q_minus = [q_plus[0] - 34.0, 12.0, 30.0, 26.0];

    for (r, lbl, hi) in [
        (q_minus, "A-", false),
        (q_plus, "A+", false),
        (q_add, "+ Key", false),
        (q_opts, "Options", ed.menu_open),
        (q_done, "Done", true),
    ] {
        ui.rounded(r[0], r[1], r[2], r[3], 7.0, if lbl == "Done" { p.accent } else if hi { p.accent_soft } else { p.card_hi });
        ui.stroke_rounded(r[0], r[1], r[2], r[3], 7.0, 1.0, if lbl == "Done" || hi { p.accent } else { p.border });
        let tc_col = if lbl == "Done" { rgba(10, 14, 20, 255) } else if hi { p.accent } else { p.text };
        ui.centered_fit(lbl, r, 10.5, tc_col, Face::SemiBold);
    }

    // 5. Key / Action Picker Modal (when adding or changing a Custom Button)
    if ed.picking_for.is_some() {
        ui.fill(0.0, 0.0, w, h, rgba(4, 6, 10, 195));
        let pw = (w - 28.0).clamp(280.0, 540.0);
        let ph = (h - 24.0).clamp(220.0, 360.0);
        let px = (w - pw) * 0.5;
        let py = (h - ph) * 0.5;
        ui.rounded(px, py, pw, ph, 16.0, rgba(16, 20, 28, 252));
        ui.stroke_rounded(px, py, pw, ph, 16.0, 1.2, p.border_light);
        ui.label("Select Action or Desktop Key", px + 14.0, py + 12.0, 13.5, p.text, Face::SemiBold);

        // Mode tabs: [Game Actions] [Keyboard Keys] [Cancel]
        let t_act = [px + pw - 248.0, py + 8.0, 88.0, 26.0];
        let t_key = [px + pw - 154.0, py + 8.0, 88.0, 26.0];
        let t_can = [px + pw - 60.0, py + 8.0, 48.0, 26.0];
        for (r, lbl, on) in [(t_act, "Actions", !ed.raw_keys), (t_key, "Keys", ed.raw_keys), (t_can, "Close", false)] {
            ui.rounded(r[0], r[1], r[2], r[3], 7.0, if on { p.accent_soft } else { p.card_hi });
            ui.stroke_rounded(r[0], r[1], r[2], r[3], 7.0, 1.0, if on { p.accent } else { p.border });
            ui.centered_fit(lbl, r, 10.0, if on { p.accent } else { p.text }, Face::SemiBold);
        }

        let choices = crate::touch::custom::choices(ed.raw_keys);
        let cols = if ed.raw_keys { 6usize } else { 3usize };
        let rows = choices.len().div_ceil(cols).max(1);
        let gx = px + 12.0;
        let gy = py + 42.0;
        let gw = pw - 24.0;
        let gh = ph - 52.0;
        let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
        let ch = ((gh - (rows as f32 - 1.0) * 5.0) / rows as f32).clamp(22.0, 34.0);
        for (i, (_, label)) in choices.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cx = gx + col as f32 * (cw + 6.0);
            let cy = gy + row as f32 * (ch + 5.0);
            if cy + ch <= py + ph - 6.0 {
                ui.rounded(cx, cy, cw, ch, 7.0, p.card_hi);
                ui.stroke_rounded(cx, cy, cw, ch, 7.0, 1.0, p.border);
                ui.centered_fit(label, [cx + 4.0, cy, cw - 8.0, ch], 10.0, p.text, Face::Medium);
            }
        }
        ui.finish();
        return;
    }

    // 6. Floating '=' Editor Options Dialog (when ed.menu_open == true)
    if ed.menu_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 6, 10, 175));
        let dw = (w - 28.0).clamp(290.0, 530.0);
        let dh = (h - 20.0).clamp(240.0, 366.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, rgba(16, 20, 28, 250));
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.2, p.border_light);
        ui.rounded(dx + 12.0, dy + 10.0, 26.0, 26.0, 8.0, p.accent_soft);
        ui.icon(Icon::MenuEquals, dx + 16.0, dy + 14.0, 18.0, p.accent);
        ui.label("Control Editor & Presets", dx + 44.0, dy + 15.0, 13.5, p.text, Face::SemiBold);

        // Top-right close menu button
        ui.rounded(dx + dw - 76.0, dy + 10.0, 64.0, 26.0, 8.0, p.card_hi);
        ui.stroke_rounded(dx + dw - 76.0, dy + 10.0, 64.0, 26.0, 8.0, 1.0, p.border);
        ui.centered_fit("Resume", [dx + dw - 74.0, dy + 10.0, 60.0, 26.0], 10.5, p.accent, Face::SemiBold);

        // Row 1: Presets
        ui.label("1. APPLY PRESET LAYOUT", dx + 14.0, dy + 42.0, 9.5, p.accent, Face::SemiBold);
        let py = dy + 56.0;
        let pw = (dw - 28.0 - 16.0) / 5.0;
        for (i, (id, title)) in [
            ("default", "Pocket PE"),
            ("pojav", "Pojav"),
            ("pvp", "PvP Claw"),
            ("building", "Builder"),
            ("speedrun", "Speedrun"),
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            let active = tc.settings.active_profile.eq_ignore_ascii_case(id)
                || (id == "default" && tc.settings.active_profile.eq_ignore_ascii_case("pocket"));
            ui.rounded(rx, py, pw, 26.0, 7.0, if active { p.accent_soft } else { p.card_hi });
            ui.stroke_rounded(rx, py, pw, 26.0, 7.0, 1.0, if active { p.accent } else { p.border });
            ui.centered_fit(title, [rx + 2.0, py, pw - 4.0, 26.0], 10.0, if active { p.accent } else { p.text }, Face::SemiBold);
        }

        // Row 2: Global HUD & Editor Settings
        ui.label("2. MOVEMENT STYLE & EDITOR GRID", dx + 14.0, py + 32.0, 9.5, p.accent, Face::SemiBold);
        let gy = py + 46.0;
        let gw = (dw - 28.0 - 12.0) / 4.0;
        let move_lbl = match tc.settings.movement_mode() {
            MovementMode::Dpad => "D-Pad (Pocket)",
            MovementMode::Thumbstick => "Fixed Joystick",
            MovementMode::FloatingThumbstick => "Floating Stick",
        };
        let split_on = tc.settings.split_controls;
        for (i, (lbl, on)) in [
            (move_lbl, true),
            (if split_on { "Split Touch: ON" } else { "Split Touch: OFF" }, split_on),
            (if tc.settings.editor_grid { "5% Grid: ON" } else { "5% Grid: OFF" }, tc.settings.editor_grid),
            (if ed.show_hidden { "Hidden: SHOWN" } else { "Hidden: HIDE" }, ed.show_hidden),
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (gw + 4.0);
            ui.rounded(rx, gy, gw, 26.0, 7.0, if on { p.accent_soft } else { p.card_hi });
            ui.stroke_rounded(rx, gy, gw, 26.0, 7.0, 1.0, if on { p.accent } else { p.border });
            ui.centered_fit(lbl, [rx + 3.0, gy, gw - 6.0, 26.0], 10.0, if on { p.accent } else { p.text }, Face::Medium);
        }

        // Row 3 & 4: Selected Control Inspector
        ui.label(&format!("3. SELECTED BUTTON: {}", sel_title.to_uppercase()), dx + 14.0, gy + 32.0, 9.5, p.accent, Face::SemiBold);
        let iy = gy + 46.0;
        let iw = (dw - 28.0 - 12.0) / 4.0;
        let (scale_pct, alpha_pct, is_vis) = match ed.target {
            Target::BuiltIn(c) => {
                let l = tc.settings.layout_placement(c);
                (l.scale * 100.0, l.opacity * 100.0, tc.visible(c))
            }
            Target::Custom(idx) => tc.settings.custom_buttons.get(idx).map(|b| {
                let l = b.layouts[tc.settings.mode.index()];
                (l.scale * 100.0, l.opacity * 100.0, true)
            }).unwrap_or((100.0, 100.0, true)),
        };
        for (i, txt) in [
            format!("Size {:.0}%", scale_pct),
            format!("Alpha {:.0}%", alpha_pct),
            if is_vis { "Visible: YES".to_owned() } else { "Visible: NO".to_owned() },
            if tc.settings.left_handed { "Left-Hand: ON".to_owned() } else { "Left-Hand: OFF".to_owned() },
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (iw + 4.0);
            ui.rounded(rx, iy, iw, 28.0, 7.0, p.card_hi);
            ui.stroke_rounded(rx, iy, iw, 28.0, 7.0, 1.0, p.border);
            if i < 2 {
                ui.label("-", rx + 6.0, iy + 6.0, 12.0, p.accent, Face::SemiBold);
                ui.label("+", rx + iw - 13.0, iy + 6.0, 12.0, p.accent, Face::SemiBold);
                ui.centered_fit(&txt, [rx + 14.0, iy, iw - 28.0, 28.0], 9.5, p.text, Face::Medium);
            } else {
                ui.centered_fit(&txt, [rx + 4.0, iy, iw - 8.0, 28.0], 9.5, p.text, Face::Medium);
            }
        }

        // Row 4: Custom Button properties (Pass-Through, Toggle, Swipe, Rebind, Delete)
        let cy = iy + 34.0;
        if let Target::Custom(idx) = ed.target
            && let Some(custom) = tc.settings.custom_buttons.get(idx)
        {
            let layout = custom.layouts[tc.settings.mode.index()];
            let cw = (dw - 28.0 - 16.0) / 5.0;
            for (i, (lbl, on, danger)) in [
                (if layout.passthrough { "Pass-Thru: ON" } else { "Pass-Thru: OFF" }, layout.passthrough, false),
                (if layout.toggleable { "Mode: Toggle" } else { "Mode: Hold" }, layout.toggleable, false),
                (if layout.swipeable { "Swipe: ON" } else { "Swipe: OFF" }, layout.swipeable, false),
                ("Change Key", false, false),
                ("Delete Btn", false, true),
            ].into_iter().enumerate() {
                let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                let bg = if danger { rgba(92, 28, 34, 220) } else if on { p.accent_soft } else { p.card_hi };
                let bd = if danger { p.danger } else if on { p.accent } else { p.border };
                ui.rounded(rx, cy, cw, 26.0, 7.0, bg);
                ui.stroke_rounded(rx, cy, cw, 26.0, 7.0, 1.0, bd);
                ui.centered_fit(lbl, [rx + 2.0, cy, cw - 4.0, 26.0], 9.5, if danger { p.danger } else if on { p.accent } else { p.text }, Face::Medium);
            }
        } else {
            ui.label("Tip: Tap '+ Add Custom Key' below to create extra Pojav-style buttons with Pass-Through, Toggle & Swipe.", dx + 14.0, cy + 7.0, 9.5, p.muted, Face::Regular);
        }

        // Row 5: Bottom Action Bar
        let by = dy + dh - 36.0;
        let bw = (dw - 28.0 - 16.0) / 5.0;
        for (i, (lbl, primary)) in [
            ("+ Add Custom Key", true),
            ("Reset Selected", false),
            ("Reset All Default", false),
            ("Import File", false),
            ("Export File", false),
        ].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (bw + 4.0);
            ui.rounded(rx, by, bw, 26.0, 7.0, if primary { p.accent } else { p.card_hi });
            ui.stroke_rounded(rx, by, bw, 26.0, 7.0, 1.0, if primary { p.accent } else { p.border });
            ui.centered_fit(lbl, [rx + 2.0, by, bw - 4.0, 26.0], 9.5, if primary { rgba(10, 14, 20, 255) } else { p.text }, Face::SemiBold);
        }
    }

    ui.finish();
}

/// Handles touch input for the Material You 3 + Pocket Edition HUD Control Editor.
/// `point` is normalized `[0.0..=1.0, 0.0..=1.0]` across the viewport.
/// Returns `true` when the user taps `Done` to exit the editor.
pub fn touch_hud_editor(
    tc: &mut crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    id: u64,
    point: [f32; 2],
    started: bool,
    ended: bool,
) -> bool {
    use crate::touch::{Control, MovementMode, Target};
    let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let p = [point[0] * w, point[1] * h];
    let inside = |r: [f32; 4]| {
        p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
    };

    let Some(mut ed) = tc.editor.take() else {
        return false;
    };

    let closed = (|| {
    // 1. Handle ongoing drag updates
    if !started {
        if let Some((drag_id, offset, _)) = ed.active_drag {
            if drag_id == id {
                let sel = ed.target;
                let snap = tc.settings.editor_grid;
                match sel {
                    Target::BuiltIn(c) => {
                        let mut center = [point[0] - offset[0], point[1] - offset[1]];
                        if snap {
                            center = center.map(|v| (v / 0.025).round() * 0.025);
                        }
                        if tc.settings.left_handed {
                            center[0] = 1.0 - center[0];
                        }
                        let aspect = tc.aspect();
                        let place = tc.settings.layout_placement(c);
                        let size = (place.size * place.scale * tc.settings.button_scale)
                            .min(aspect)
                            .min(1.0);
                        let slot = tc.settings.layout_placement_mut(c);
                        slot.center = [
                            center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                            center[1].clamp(size / 2.0, 1.0 - size / 2.0),
                        ];
                    }
                    Target::Custom(idx) => {
                        let mut center = [point[0] - offset[0], point[1] - offset[1]];
                        if snap {
                            center = center.map(|v| (v / 0.025).round() * 0.025);
                        }
                        if tc.settings.left_handed {
                            center[0] = 1.0 - center[0];
                        }
                        let aspect = tc.aspect();
                        let mode = tc.settings.mode.index();
                        if let Some(button) = tc.settings.custom_buttons.get_mut(idx) {
                            let place = button.layouts[mode];
                            let size = (place.size * place.scale * tc.settings.button_scale)
                                .min(aspect)
                                .min(1.0);
                            let slot = &mut button.layouts[mode];
                            slot.center = [
                                center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                                center[1].clamp(size / 2.0, 1.0 - size / 2.0),
                            ];
                        }
                    }
                }
                if ended {
                    ed.active_drag = None;
                    let _ = tc.settings.save();
                }
            }
        }
        return false;
    }

    // Only process button taps on TouchPhase::Started
    // 2. If Key / Action Picker is open:
    if let Some(target) = ed.picking_for {
        let pw = (w - 28.0).clamp(280.0, 540.0);
        let ph = (h - 24.0).clamp(220.0, 360.0);
        let px = (w - pw) * 0.5;
        let py = (h - ph) * 0.5;
        let t_act = [px + pw - 248.0, py + 8.0, 88.0, 26.0];
        let t_key = [px + pw - 154.0, py + 8.0, 88.0, 26.0];
        let t_can = [px + pw - 60.0, py + 8.0, 48.0, 26.0];
        if inside(t_act) {
            ed.raw_keys = false;
            tc.clicked = true;
            return false;
        }
        if inside(t_key) {
            ed.raw_keys = true;
            tc.clicked = true;
            return false;
        }
        if inside(t_can) || !inside([px, py, pw, ph]) {
            ed.picking_for = None;
            tc.clicked = true;
            return false;
        }
        let choices = crate::touch::custom::choices(ed.raw_keys);
        let cols = if ed.raw_keys { 6usize } else { 3usize };
        let rows = choices.len().div_ceil(cols).max(1);
        let gx = px + 12.0;
        let gy = py + 42.0;
        let gw = pw - 24.0;
        let gh = ph - 52.0;
        let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
        let ch = ((gh - (rows as f32 - 1.0) * 5.0) / rows as f32).clamp(22.0, 34.0);
        for (i, (token, label)) in choices.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cx = gx + col as f32 * (cw + 6.0);
            let cy = gy + row as f32 * (ch + 5.0);
            if inside([cx, cy, cw, ch]) {
                let short_lbl: String = label.chars().take(10).collect();
                if let Some(idx) = target {
                    if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                        btn.key = (*token).to_owned();
                        btn.custom_label = short_lbl;
                    }
                    ed.target = Target::Custom(idx);
                } else if tc.settings.custom_buttons.len() < 24 {
                    let n = tc.settings.custom_buttons.len();
                    let mut button = crate::touch::CustomButton::new((*token).to_owned(), n);
                    button.custom_label = short_lbl;
                    tc.settings.custom_buttons.push(button);
                    ed.target = Target::Custom(tc.settings.custom_buttons.len() - 1);
                }
                ed.picking_for = None;
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }
        }
        return false;
    }

    // 3. Check Floating '=' Button and Top Quick Pill Bar
    let eq_rect = [10.0, 8.0, 38.0, 34.0];
    if inside(eq_rect) {
        ed.menu_open = !ed.menu_open;
        tc.clicked = true;
        return false;
    }
    let bar_x = 54.0;
    let bar_w = (w - bar_x - 10.0).max(220.0);
    let q_done = [bar_x + bar_w - 64.0, 12.0, 58.0, 26.0];
    let q_opts = [q_done[0] - 68.0, 12.0, 64.0, 26.0];
    let q_add  = [q_opts[0] - 62.0, 12.0, 58.0, 26.0];
    let q_plus = [q_add[0] - 34.0, 12.0, 30.0, 26.0];
    let q_minus = [q_plus[0] - 34.0, 12.0, 30.0, 26.0];

    if inside(q_done) {
        let _ = tc.settings.save();
        tc.editor = None;
        tc.clicked = true;
        return true;
    }
    if inside(q_opts) {
        ed.menu_open = !ed.menu_open;
        tc.clicked = true;
        return false;
    }
    if inside(q_add) {
        ed.picking_for = Some(None);
        ed.raw_keys = false;
        tc.clicked = true;
        return false;
    }
    if inside(q_minus) || inside(q_plus) {
        let delta = if inside(q_plus) { 0.10 } else { -0.10 };
        let sel = ed.target;
        match sel {
            Target::BuiltIn(c) => {
                let slot = tc.settings.layout_placement_mut(c);
                slot.scale = (slot.scale + delta).clamp(0.5, 2.2);
            }
            Target::Custom(idx) => {
                let mode = tc.settings.mode.index();
                if let Some(b) = tc.settings.custom_buttons.get_mut(idx) {
                    let slot = &mut b.layouts[mode];
                    slot.scale = (slot.scale + delta).clamp(0.5, 2.2);
                }
            }
        }
        let _ = tc.settings.save();
        tc.clicked = true;
        return false;
    }

    // 4. If Floating '=' Editor Options Dialog is open, handle its controls
    if ed.menu_open {
        let dw = (w - 28.0).clamp(290.0, 530.0);
        let dh = (h - 20.0).clamp(240.0, 366.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        if inside([dx + dw - 76.0, dy + 10.0, 64.0, 26.0]) || !inside([dx, dy, dw, dh]) {
            ed.menu_open = false;
            tc.clicked = true;
            return false;
        }
        // Row 1: Presets
        let py = dy + 56.0;
        let pw = (dw - 28.0 - 16.0) / 5.0;
        for (i, id) in ["default", "pojav", "pvp", "building", "speedrun"].into_iter().enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            if inside([rx, py, pw, 26.0]) {
                tc.settings.apply_preset(id);
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }
        }
        // Row 2: Movement & Grid
        let gy = py + 46.0;
        let gw = (dw - 28.0 - 12.0) / 4.0;
        if inside([dx + 14.0, gy, gw, 26.0]) {
            tc.settings.cycle_movement_mode();
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }
        if inside([dx + 14.0 + (gw + 4.0), gy, gw, 26.0]) {
            tc.settings.split_controls = !tc.settings.split_controls;
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }
        if inside([dx + 14.0 + (gw + 4.0) * 2.0, gy, gw, 26.0]) {
            tc.settings.editor_grid = !tc.settings.editor_grid;
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }
        if inside([dx + 14.0 + (gw + 4.0) * 3.0, gy, gw, 26.0]) {
            ed.show_hidden = !ed.show_hidden;
            tc.clicked = true;
            return false;
        }

        // Row 3: Selected Control Inspector (Size -/+, Alpha -/+, Visible, Left-Handed)
        let iy = gy + 46.0;
        let iw = (dw - 28.0 - 12.0) / 4.0;
        let sel = ed.target;
        // Size -/+
        let s0 = dx + 14.0;
        if inside([s0, iy, iw, 28.0]) {
            let delta = if p[0] < s0 + iw * 0.5 { -0.10 } else { 0.10 };
            match sel {
                Target::BuiltIn(c) => {
                    let l = tc.settings.layout_placement_mut(c);
                    l.scale = (l.scale + delta).clamp(0.5, 2.2);
                }
                Target::Custom(idx) => {
                    let mode = tc.settings.mode.index();
                    if let Some(b) = tc.settings.custom_buttons.get_mut(idx) {
                        let l = &mut b.layouts[mode];
                        l.scale = (l.scale + delta).clamp(0.5, 2.2);
                    }
                }
            }
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }
        // Alpha -/+
        let s1 = dx + 14.0 + (iw + 4.0);
        if inside([s1, iy, iw, 28.0]) {
            let delta = if p[0] < s1 + iw * 0.5 { -0.15 } else { 0.15 };
            match sel {
                Target::BuiltIn(c) => {
                    let l = tc.settings.layout_placement_mut(c);
                    l.opacity = (l.opacity + delta).clamp(0.2, 1.0);
                }
                Target::Custom(idx) => {
                    let mode = tc.settings.mode.index();
                    if let Some(b) = tc.settings.custom_buttons.get_mut(idx) {
                        let l = &mut b.layouts[mode];
                        l.opacity = (l.opacity + delta).clamp(0.2, 1.0);
                    }
                }
            }
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }
        // Visible toggle
        let s2 = dx + 14.0 + (iw + 4.0) * 2.0;
        if inside([s2, iy, iw, 28.0]) {
            if let Target::BuiltIn(c) = sel {
                if matches!(c, Control::Attack | Control::Use) && !tc.settings.split_controls {
                    tc.settings.split_controls = true;
                    tc.settings.set_visible(c, true);
                } else {
                    let visible = tc.visible(c);
                    tc.settings.set_visible(c, !visible);
                }
                let _ = tc.settings.save();
                tc.clicked = true;
            }
            return false;
        }
        // Left-Handed toggle
        let s3 = dx + 14.0 + (iw + 4.0) * 3.0;
        if inside([s3, iy, iw, 28.0]) {
            tc.settings.left_handed = !tc.settings.left_handed;
            let _ = tc.settings.save();
            tc.clicked = true;
            return false;
        }

        // Row 4: Custom button properties
        let cy = iy + 34.0;
        if let Target::Custom(idx) = sel {
            let cw = (dw - 28.0 - 16.0) / 5.0;
            let mode = tc.settings.mode.index();
            for i in 0..5 {
                let rx = dx + 14.0 + i as f32 * (cw + 4.0);
                if inside([rx, cy, cw, 26.0]) {
                    match i {
                        0 => if let Some(b) = tc.settings.custom_buttons.get_mut(idx) { let l = &mut b.layouts[mode]; l.passthrough = !l.passthrough; },
                        1 => if let Some(b) = tc.settings.custom_buttons.get_mut(idx) { let l = &mut b.layouts[mode]; l.toggleable = !l.toggleable; },
                        2 => if let Some(b) = tc.settings.custom_buttons.get_mut(idx) { let l = &mut b.layouts[mode]; l.swipeable = !l.swipeable; },
                        3 => {
                            ed.picking_for = Some(Some(idx));
                            ed.raw_keys = false;
                        }
                        4 => {
                            if idx < tc.settings.custom_buttons.len() {
                                tc.settings.custom_buttons.remove(idx);
                            }
                            ed.target = Target::BuiltIn(Control::Move);
                        }
                        _ => {}
                    }
                    let _ = tc.settings.save();
                    tc.clicked = true;
                    return false;
                }
            }
        }

        // Row 5: Bottom Action Bar
        let by = dy + dh - 36.0;
        let bw = (dw - 28.0 - 16.0) / 5.0;
        for i in 0..5 {
            let rx = dx + 14.0 + i as f32 * (bw + 4.0);
            if inside([rx, by, bw, 26.0]) {
                match i {
                    0 => {
                        ed.picking_for = Some(None);
                        ed.raw_keys = false;
                    }
                    1 => {
                        if let Target::BuiltIn(c) = sel {
                            let mode = tc.settings.mode.index();
                            let slot = tc.settings.slot(c);
                            *tc.settings.layout_placement_mut(c) =
                                crate::touch::Settings::default().layouts[mode][slot];
                            let _ = tc.settings.save();
                        }
                    }
                    2 => {
                        tc.settings.apply_preset("default");
                        let _ = tc.settings.save();
                    }
                    3 => {
                        // Profile import goes through the launcher's SAF bridge.
                        let _ = tc.settings.save();
                    }
                    4 => {
                        let _ = tc.settings.save();
                    }
                    _ => {}
                }
                tc.clicked = true;
                return false;
            }
        }
        return false;
    }

    // 5. Hit-test Custom Buttons & Built-in Default Controls on Canvas for selection & dragging
    let show_hidden = ed.show_hidden;
    for i in (0..tc.settings.custom_buttons.len()).rev() {
        let place = tc.custom_placement(i);
        let [nx, ny, nw, nh] = place.rect(tc.aspect());
        if (nx..=nx + nw).contains(&point[0]) && (ny..=ny + nh).contains(&point[1]) {
            ed.target = Target::Custom(i);
            ed.active_drag = Some((
                id,
                [point[0] - place.center[0], point[1] - place.center[1]],
                false,
            ));
            tc.clicked = true;
            return false;
        }
    }
    for &c in Control::ALL.iter().rev() {
        if !tc.visible(c) && !show_hidden {
            continue;
        }
        let place = tc.placement(c);
        let [nx, ny, nw, nh] = place.rect(tc.aspect());
        if (nx..=nx + nw).contains(&point[0]) && (ny..=ny + nh).contains(&point[1]) {
            ed.target = Target::BuiltIn(c);
            ed.active_drag = Some((
                id,
                [point[0] - place.center[0], point[1] - place.center[1]],
                false,
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
    fn mouse_toggle_is_available_without_a_setting_and_clicks_stay_on_right() {
        let mut ui = TouchUi::default();
        assert!(matches!(
            ui.action(427.0, 267.0, [350.0, 10.0], true),
            Some(Action::VirtualMouse)
        ));
        assert!(ui.action(427.0, 267.0, [350.0, 10.0], false).is_none());
        ui.virtual_mouse = true;
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 226.0], true),
            Some(Action::MouseButton(false))
        ));
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 250.0], true),
            Some(Action::MouseButton(true))
        ));
        assert!(ui.action(427.0, 267.0, [213.0, 250.0], true).is_none());
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
