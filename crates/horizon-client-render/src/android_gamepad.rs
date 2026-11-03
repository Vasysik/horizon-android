//! Android's standard controller axes and key codes, independent of USB/Bluetooth.
//! Sources are checked by the native event backend before reaching this decoder.
use crate::gamepad::{Button, PadState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputMode {
    #[default]
    Auto,
    Touch,
    KeyboardMouse,
    Controller,
}
impl InputMode {
    pub const ALL: [Self; 4] = [
        Self::Auto,
        Self::Touch,
        Self::KeyboardMouse,
        Self::Controller,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Touch => "Touch & Android keyboard",
            Self::KeyboardMouse => "Keyboard & mouse",
            Self::Controller => "Controller",
        }
    }
}

#[derive(Default)]
struct Device {
    keys: PadState,
    axes: [f32; 24],
    pending: PadState,
}
impl Device {
    fn state(&self) -> PadState {
        let a = &self.axes;
        let mut s = self.keys;
        s.left = [a[0], -a[1]];
        s.right = [a[11], -a[14]];
        s.triggers = [a[17].max(a[23]), a[18].max(a[22])];
        for (button, held) in [
            (Button::DPadLeft, a[15] < -0.5),
            (Button::DPadRight, a[15] > 0.5),
            (Button::DPadUp, a[16] < -0.5),
            (Button::DPadDown, a[16] > 0.5),
        ] {
            s.set(button, held || s.held(button));
        }
        s
    }
}

pub fn button(code: u32) -> Option<Button> {
    Some(match code {
        96 => Button::South,
        97 | 4 => Button::East,
        99 => Button::West,
        100 => Button::North,
        102 => Button::LeftBumper,
        103 => Button::RightBumper,
        104 => Button::LeftTrigger,
        105 => Button::RightTrigger,
        106 => Button::LeftStick,
        107 => Button::RightStick,
        108 => Button::Start,
        109 => Button::Select,
        19 => Button::DPadUp,
        20 => Button::DPadDown,
        21 => Button::DPadLeft,
        22 => Button::DPadRight,
        _ => return None,
    })
}

#[derive(Default)]
pub struct Devices {
    devices: BTreeMap<i32, Device>,
    active: Option<i32>,
}
impl Devices {
    /// Returns true only for a meaningful changed value; neutral/drifting events
    /// must not steal Auto mode back from a finger or a mouse.
    pub fn axis(&mut self, id: i32, axis: u32, value: f64) -> bool {
        if !matches!(axis, 0 | 1 | 11 | 14 | 15 | 16 | 17 | 18 | 22 | 23) {
            return false;
        }
        let value = if value.is_finite() {
            value.clamp(-1.0, 1.0) as f32
        } else {
            0.0
        };
        let d = self.devices.entry(id).or_default();
        let old = std::mem::replace(&mut d.axes[axis as usize], value);
        if value >= 0.5 && old < 0.5 {
            let trigger = match axis {
                17 | 23 => Some(0),
                18 | 22 => Some(1),
                _ => None,
            };
            if let Some(i) = trigger {
                d.pending.triggers[i] = value;
            }
        }
        let activity = value.abs() > 0.25 && (value - old).abs() > 0.01;
        if activity {
            self.active = Some(id);
        }
        activity
    }
    pub fn key(&mut self, id: i32, code: u32, down: bool) -> bool {
        let Some(b) = button(code) else { return false };
        let d = self.devices.entry(id).or_default();
        let activity = down && !d.keys.held(b);
        d.keys.set(b, down);
        if activity {
            d.pending.set(b, true);
        }
        if activity {
            self.active = Some(id);
        }
        activity
    }
    pub fn retain(&mut self, ids: &[i32]) {
        self.devices.retain(|id, _| ids.contains(id));
        if self.active.is_some_and(|id| !ids.contains(&id)) {
            self.active = None;
        }
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn active_id(&self) -> Option<i32> {
        self.active
    }
    pub fn poll(&mut self) -> PadState {
        self.active
            .and_then(|id| self.devices.get_mut(&id))
            .map(|d| d.state().merge(&std::mem::take(&mut d.pending)))
            .unwrap_or_default()
    }
    #[cfg(test)]
    pub fn state(&self) -> PadState {
        self.active
            .and_then(|id| self.devices.get(&id))
            .map(Device::state)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_y_is_inverted_and_hats_do_not_erase_key_dpad() {
        let mut d = Devices::default();
        d.axis(7, 1, -1.0);
        d.axis(7, 14, 0.7);
        d.key(7, 19, true);
        d.axis(7, 16, 0.0);
        assert_eq!(d.state().left, [0.0, 1.0]);
        assert_eq!(d.state().right, [0.0, -0.7]);
        assert!(d.state().held(Button::DPadUp));
        d.key(7, 19, false);
        assert!(!d.state().held(Button::DPadUp));
    }
    #[test]
    fn a_short_press_survives_until_one_poll_and_disconnect_clears_it() {
        let mut d = Devices::default();
        d.key(4, 96, true);
        assert_eq!(d.active_id(), Some(4));
        d.key(4, 96, false);
        assert!(d.poll().held(Button::South));
        assert!(!d.poll().held(Button::South));
        d.key(4, 96, true);
        d.retain(&[]);
        assert_eq!(d.active_id(), None);
        assert_eq!(d.poll(), PadState::default());
    }
    #[test]
    fn triggers_accept_analog_and_digital_reports_without_double_edges() {
        let mut d = Devices::default();
        d.key(3, 104, true);
        d.axis(3, 17, 0.0);
        assert!(d.state().held(Button::LeftTrigger));
        d.key(3, 104, false);
        d.axis(3, 23, 0.8);
        assert_eq!(d.state().triggers[0], 0.8);
        assert!(!d.key(3, 24, true)); // System volume is never a game action.
    }
    #[test]
    fn drift_neutral_and_repeats_do_not_select_another_controller() {
        let mut d = Devices::default();
        assert!(!d.axis(1, 0, 0.12));
        assert_eq!(d.state(), PadState::default());
        assert!(d.key(2, 96, true));
        assert!(!d.key(2, 96, true));
        d.axis(1, 0, 0.0);
        assert!(d.state().held(Button::South));
        d.retain(&[1]);
        assert_eq!(d.state(), PadState::default());
        d.axis(1, 0, f64::NAN);
        assert_eq!(d.state(), PadState::default());
    }
}
