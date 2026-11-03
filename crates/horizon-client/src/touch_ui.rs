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
struct RelativeTouch {
    id: u64,
    origin: [f32; 2],
    last: [f32; 2],
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
    let (x, y) = (cursor.0.floor(), cursor.1.floor());
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
                since: Instant::now(),
                dragged: !clicks,
                right_sent: false,
            });
        }
        let Some(touch) = self.relative.as_mut().filter(|t| t.id == id) else {
            return cursor;
        };
        touch.dragged |= (point[0] - touch.origin[0]).hypot(point[1] - touch.origin[1]) > 3.0;
        let position = [0, 1].map(|i| {
            (cursor[i] + (point[i] - touch.last[i]) * sensitivity)
                .clamp(0.0, (size[i] - 1.0).max(0.0))
        });
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
