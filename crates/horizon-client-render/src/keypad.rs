//! Editable keypad profile, using the existing controller motion path.
use crate::gamepad::{Button, PadState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
/// Action IDs are stable persistence keys, not physical device key codes.
pub const ACTIONS: &[(&str, &str, &[&str])] = &[
    (
        "key.keypad.forward",
        "Forward",
        &["key.keyboard.up", "key.keyboard.keypad.8"],
    ),
    (
        "key.keypad.back",
        "Backward",
        &["key.keyboard.down", "key.keyboard.keypad.2"],
    ),
    (
        "key.keypad.left",
        "Left",
        &["key.keyboard.left", "key.keyboard.keypad.4"],
    ),
    (
        "key.keypad.right",
        "Right",
        &["key.keyboard.right", "key.keyboard.keypad.6"],
    ),
    ("key.keypad.look_up", "Look up", &["key.keyboard.2"]),
    ("key.keypad.look_down", "Look down", &["key.keyboard.8"]),
    ("key.keypad.look_left", "Look left", &["key.keyboard.4"]),
    ("key.keypad.look_right", "Look right", &["key.keyboard.6"]),
    ("key.keypad.attack", "Attack", &["key.keyboard.1"]),
    ("key.keypad.use", "Use", &["key.keyboard.3"]),
    (
        "key.keypad.jump",
        "Jump",
        &[
            "key.keyboard.5",
            "key.keyboard.keypad.5",
            "key.keyboard.enter",
            "key.keyboard.keypad.enter",
        ],
    ),
    (
        "key.keypad.sneak",
        "Sneak",
        &["key.keyboard.0", "key.keyboard.keypad.0"],
    ),
    ("key.keypad.inventory", "Inventory", &["key.keyboard.7"]),
    (
        "key.keypad.next_slot",
        "Next hotbar slot",
        &["key.keyboard.9"],
    ),
    (
        "key.keypad.forward_left",
        "Forward + left",
        &["key.keyboard.keypad.7"],
    ),
    (
        "key.keypad.forward_right",
        "Forward + right",
        &["key.keyboard.keypad.9"],
    ),
    (
        "key.keypad.back_left",
        "Backward + left",
        &["key.keyboard.keypad.1"],
    ),
    (
        "key.keypad.back_right",
        "Backward + right",
        &["key.keyboard.keypad.3"],
    ),
];
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub bindings: BTreeMap<String, Vec<String>>,
}
impl Settings {
    pub fn keys(&self, name: &str) -> Vec<String> {
        self.bindings.get(name).cloned().unwrap_or_else(|| {
            ACTIONS
                .iter()
                .find(|a| a.0 == name)
                .map_or_else(Vec::new, |a| a.2.iter().map(|s| (*s).into()).collect())
        })
    }
    /// Menu navigation reuses the editable profile, plus standard UI keys.
    pub fn menu_button(&self, key: KeyCode) -> Option<Button> {
        match key {
            KeyCode::ArrowUp => return Some(Button::DPadUp),
            KeyCode::ArrowDown => return Some(Button::DPadDown),
            KeyCode::ArrowLeft => return Some(Button::DPadLeft),
            KeyCode::ArrowRight => return Some(Button::DPadRight),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => return Some(Button::South),
            KeyCode::Escape => return Some(Button::East),
            _ => {}
        }
        let name = crate::keybinds::key_name(key)?;
        [
            ("key.keypad.forward", Button::DPadUp),
            ("key.keypad.look_up", Button::DPadUp),
            ("key.keypad.back", Button::DPadDown),
            ("key.keypad.look_down", Button::DPadDown),
            ("key.keypad.left", Button::DPadLeft),
            ("key.keypad.look_left", Button::DPadLeft),
            ("key.keypad.right", Button::DPadRight),
            ("key.keypad.look_right", Button::DPadRight),
            ("key.keypad.jump", Button::South),
            ("key.keypad.use", Button::South),
            ("key.keypad.inventory", Button::East),
            ("key.keypad.sneak", Button::East),
            ("key.keypad.next_slot", Button::RightBumper),
        ]
        .into_iter()
        .find_map(|(action, button)| {
            self.keys(action)
                .iter()
                .any(|k| k == name)
                .then_some(button)
        })
    }
    pub fn set(&mut self, name: &str, value: String) {
        self.bindings.insert(name.into(), vec![value]);
    }
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if !self.enabled && self.bindings.is_empty() && !path.exists() {
            return Ok(());
        }
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path)
    }
}
/// DPAD_CENTER and eight-way Android D-pad codes lack PC physical codes in winit.
pub fn resolve_key(physical: PhysicalKey, logical: &Key) -> Option<KeyCode> {
    use winit::keyboard::NativeKeyCode;
    match (physical, logical) {
        (PhysicalKey::Code(code), _) => Some(code),
        (PhysicalKey::Unidentified(NativeKeyCode::Android(code)), _) => match code {
            23 => Some(KeyCode::Enter),
            268 => Some(KeyCode::Numpad7),
            269 => Some(KeyCode::Numpad1),
            270 => Some(KeyCode::Numpad9),
            271 => Some(KeyCode::Numpad3),
            _ => None,
        },
        (_, Key::Named(NamedKey::Enter)) => Some(KeyCode::Enter),
        _ => None,
    }
}
#[derive(Default)]
pub struct Keypad {
    settings: Settings,
    held: BTreeSet<KeyCode>,
    bits: u32,
    pending: u32,
}
impl Keypad {
    pub fn configure(&mut self, settings: &Settings) {
        if self.settings != *settings {
            self.clear();
            self.settings = settings.clone();
        }
    }
    pub fn clear(&mut self) {
        self.held.clear();
        self.bits = 0;
        self.pending = 0;
    }
    pub fn key(&mut self, key: KeyCode, down: bool) -> bool {
        let Some(name) = crate::keybinds::key_name(key) else {
            return false;
        };
        if !ACTIONS
            .iter()
            .any(|a| self.settings.keys(a.0).iter().any(|k| k == name))
        {
            return false;
        }
        if down {
            self.held.insert(key);
        } else {
            self.held.remove(&key);
        }
        let mut bits = 0;
        for (i, a) in ACTIONS.iter().enumerate() {
            if self
                .settings
                .keys(a.0)
                .iter()
                .filter_map(|k| crate::keybinds::key_code(k))
                .any(|k| self.held.contains(&k))
            {
                bits |= 1 << i;
            }
        }
        self.pending |= bits & !self.bits;
        self.bits = bits;
        true
    }
    pub fn state(&self) -> PadState {
        Self::state_for(self.bits)
    }
    pub fn poll(&mut self) -> PadState {
        Self::state_for(self.bits | std::mem::take(&mut self.pending))
    }
    fn state_for(bits: u32) -> PadState {
        let held = |i: u32| bits & (1 << i) != 0;
        let axis = |p, n| f32::from(u8::from(p)) - f32::from(u8::from(n));
        let mut left = [
            axis(
                held(3) || held(15) || held(17),
                held(2) || held(14) || held(16),
            ),
            axis(
                held(0) || held(14) || held(15),
                held(1) || held(16) || held(17),
            ),
        ];
        let len = left[0].hypot(left[1]).max(1.0);
        left = left.map(|v| v / len);
        let mut state = PadState {
            left,
            right: [axis(held(7), held(6)), axis(held(4), held(5))],
            triggers: [f32::from(u8::from(held(9))), f32::from(u8::from(held(8)))],
            ..Default::default()
        };
        for (bit, button) in [
            (10, Button::South),
            (11, Button::RightStick),
            (12, Button::North),
            (13, Button::RightBumper),
        ] {
            state.set(button, held(bit));
        }
        state
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_navigation_uses_profile_rebindings_and_standard_ui_keys() {
        let mut s = Settings::default();
        assert_eq!(s.menu_button(KeyCode::Digit5), Some(Button::South));
        assert_eq!(s.menu_button(KeyCode::Digit2), Some(Button::DPadUp));
        s.set("key.keypad.forward", "key.keyboard.z".into());
        assert_eq!(s.menu_button(KeyCode::KeyZ), Some(Button::DPadUp));
        assert_eq!(s.menu_button(KeyCode::ArrowUp), Some(Button::DPadUp));
        assert_eq!(s.menu_button(KeyCode::Escape), Some(Button::East));
        assert_eq!(s.menu_button(KeyCode::F3), None);
    }
    #[test]
    fn rebinding_and_overlapping_aliases_keep_normalized_eight_way_motion() {
        let mut k = Keypad::default();
        let mut settings = Settings::default();
        settings.set("key.keypad.forward", "key.keyboard.z".into());
        k.configure(&settings);
        assert!(!k.key(KeyCode::ArrowUp, true));
        k.key(KeyCode::KeyZ, true);
        k.key(KeyCode::Numpad9, true);
        let state = k.poll();
        assert!((state.left[0].hypot(state.left[1]) - 1.0).abs() < 1e-6);
        k.key(KeyCode::Numpad9, false);
        assert_eq!(k.poll().left, [0.0, 1.0]);
        k.key(KeyCode::Digit5, true);
        k.key(KeyCode::Enter, true);
        k.poll();
        k.key(KeyCode::Digit5, false);
        assert!(k.poll().held(Button::South));
        k.key(KeyCode::Enter, false);
        assert!(!k.poll().held(Button::South));
        let saved = serde_json::to_vec(&settings).unwrap();
        assert_eq!(
            serde_json::from_slice::<Settings>(&saved).unwrap(),
            settings
        );
    }
    #[test]
    fn all_default_profile_keys_resolve_and_diagonals_are_bindable() {
        for action in ACTIONS {
            for key in action.2 {
                assert!(crate::keybinds::key_code(key).is_some(), "{key}");
            }
        }
        for (code, key) in [
            (268, KeyCode::Numpad7),
            (269, KeyCode::Numpad1),
            (270, KeyCode::Numpad9),
            (271, KeyCode::Numpad3),
        ] {
            assert_eq!(
                resolve_key(
                    PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Android(code)),
                    &Key::Unidentified(winit::keyboard::NativeKey::Android(code))
                ),
                Some(key)
            );
        }
    }
    #[test]
    fn android_dpad_centre_and_numpad_enter_jump() {
        let code = resolve_key(
            PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Android(23)),
            &Key::Named(NamedKey::Enter),
        )
        .unwrap();
        let mut keypad = Keypad::default();
        keypad.key(code, true);
        assert!(keypad.poll().held(Button::South));
        keypad.clear();
        keypad.key(KeyCode::NumpadEnter, true);
        assert!(keypad.poll().held(Button::South));
    }
    #[test]
    fn short_taps_survive_one_poll_and_clear_discards_them() {
        let mut keypad = Keypad::default();
        keypad.key(KeyCode::Digit1, true);
        keypad.key(KeyCode::Digit1, false);
        assert_eq!(keypad.poll().triggers, [0.0, 1.0]);
        assert_eq!(keypad.poll(), PadState::default());
        keypad.key(KeyCode::Digit5, true);
        keypad.clear();
        assert_eq!(keypad.poll(), PadState::default());
    }
    #[test]
    fn diagonal_release_and_camera_are_independent() {
        let mut keypad = Keypad::default();
        keypad.key(KeyCode::ArrowUp, true);
        keypad.key(KeyCode::ArrowRight, true);
        keypad.key(KeyCode::Digit6, true);
        let s = keypad.state();
        assert!((s.left[0].hypot(s.left[1]) - 1.0).abs() < 0.0001);
        assert_eq!(s.right, [1.0, 0.0]);
        keypad.key(KeyCode::Digit6, false);
        assert_eq!(keypad.state().right, [0.0, 0.0]);
        keypad.clear();
        assert_eq!(keypad.state(), PadState::default());
    }
    #[test]
    fn attack_and_use_are_analog_triggers_with_independent_releases() {
        let mut keypad = Keypad::default();
        let mut edges = crate::gamepad::Edges::default();
        keypad.key(KeyCode::Digit1, true);
        keypad.key(KeyCode::Digit3, true);
        assert_eq!(keypad.state().triggers, [1.0, 1.0]);
        let pressed = edges.update(&keypad.state(), 0.4);
        assert!(pressed.contains(&(Button::RightTrigger, true)));
        assert!(pressed.contains(&(Button::LeftTrigger, true)));
        keypad.key(KeyCode::Digit1, false);
        assert_eq!(
            edges.update(&keypad.state(), 0.4),
            [(Button::RightTrigger, false)]
        );
        keypad.clear();
        assert_eq!(
            edges.update(&keypad.state(), 0.4),
            [(Button::LeftTrigger, false)]
        );
    }
}
