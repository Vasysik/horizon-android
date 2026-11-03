//! Android's controllers (Starling's): the key codes and motion axes Android gives a pad's
//! events, as the [`PadState`] gilrs gives on desktop. horizon-android takes the events from
//! winit before it makes touches of them (android.rs) and feeds them here; the game reads
//! the active pad's state each poll (game/pad/android.rs).
use super::{Button, PadState, length};

/// `InputDevice.SOURCE_GAMEPAD` without its class bit: only a pad's events carry it.
const SOURCE_GAMEPAD: u32 = 0x0400;
/// `InputDevice.SOURCE_CLASS_JOYSTICK`: a motion event of sticks, a hat and triggers.
const SOURCE_CLASS_JOYSTICK: u32 = 0x10;
/// A stick at rest reports a little drift, which would otherwise add to the touch controls'
/// stick; gilrs' own deadzone does the same on desktop.
const STICK_REST: f32 = 0.1;
const TRIGGER_REST: f32 = 0.05;
/// How far a stick or a trigger goes before its pad becomes the active one (gilrs' path too).
const TAKE_OVER: f32 = 0.5;

/// Whether an event comes from a controller: a pad's keys, or a joystick's axes.
pub fn from_pad(source: u32) -> bool {
    source & (SOURCE_GAMEPAD | SOURCE_CLASS_JOYSTICK) != 0
}

/// Whether a motion event carries a controller's axes (sticks, hat, triggers).
pub fn joystick(source: u32) -> bool {
    source & SOURCE_CLASS_JOYSTICK != 0
}

/// The pad button of an Android key code (`KeyEvent.KEYCODE_*`): the face buttons by position,
/// as Android names them (BUTTON_A is the bottom one on every pad), the shoulders, the stick
/// clicks, Start, Select and the D-pad.
pub fn button(keycode: i32) -> Option<Button> {
    Some(match keycode {
        19 => Button::DPadUp,
        20 => Button::DPadDown,
        21 => Button::DPadLeft,
        22 => Button::DPadRight,
        96 => Button::South,
        97 => Button::East,
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
        _ => return None,
    })
}

/// The pad button a key event is, if it is one: a pad's own buttons from anything (no keyboard
/// has them), the D-pad's keys only from a controller, so a keyboard's arrows stay its own.
pub fn pad_key(source: u32, keycode: i32) -> Option<Button> {
    let button = button(keycode)?;
    let dpad = matches!(
        button,
        Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight
    );
    (!dpad || from_pad(source)).then_some(button)
}

/// A joystick motion event's axes (`MotionEvent.AXIS_*`) as Android reports them: y down,
/// triggers 0..1. A pad reports its triggers as LTRIGGER and RTRIGGER, as BRAKE and GAS, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Axes {
    pub x: f32,
    pub y: f32,
    /// The right stick: Android's pads report it as Z and RZ.
    pub z: f32,
    pub rz: f32,
    /// The hat: the D-pad of most pads.
    pub hat_x: f32,
    pub hat_y: f32,
    pub ltrigger: f32,
    pub rtrigger: f32,
    pub brake: f32,
    pub gas: f32,
}

/// One controller: its buttons from key events, its axes from its last motion event.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidPad {
    keys: PadState,
    axes: Axes,
}

impl AndroidPad {
    pub fn key(&mut self, button: Button, down: bool) {
        self.keys.set(button, down);
    }

    pub fn motion(&mut self, axes: Axes) {
        self.axes = axes;
    }

    /// Its state as the game reads a pad: sticks up positive, the hat as the D-pad, a trigger
    /// pulled as far as its farther axis, or all the way when it came as a key.
    pub fn state(&self) -> PadState {
        let a = &self.axes;
        let mut state = PadState {
            buttons: self.keys.buttons,
            left: rest([a.x, -a.y]),
            right: rest([a.z, -a.rz]),
            triggers: [
                trigger(a.ltrigger.max(a.brake), self.keys.held(Button::LeftTrigger)),
                trigger(a.rtrigger.max(a.gas), self.keys.held(Button::RightTrigger)),
            ],
        };
        // Triggers are their values; `Edges` makes buttons of them.
        state.set(Button::LeftTrigger, false);
        state.set(Button::RightTrigger, false);
        for (value, negative, positive) in [
            (a.hat_x, Button::DPadLeft, Button::DPadRight),
            (a.hat_y, Button::DPadUp, Button::DPadDown),
        ] {
            if value <= -0.5 {
                state.set(negative, true);
            } else if value >= 0.5 {
                state.set(positive, true);
            }
        }
        state
    }
}

/// A stick inside its rest is centred; past it, as reported (the game's deadzones come after).
fn rest(v: [f32; 2]) -> [f32; 2] {
    let len = length(v);
    if !len.is_finite() || len < STICK_REST {
        [0.0, 0.0]
    } else {
        [v[0].clamp(-1.0, 1.0), v[1].clamp(-1.0, 1.0)]
    }
}

fn trigger(value: f32, key: bool) -> f32 {
    if key {
        1.0
    } else if value.is_finite() && value >= TRIGGER_REST {
        value.min(1.0)
    } else {
        0.0
    }
}

/// The controllers seen, by Android's device id, and the one last used: only its input counts,
/// as on desktop.
#[derive(Clone, Debug, Default)]
pub struct AndroidPads {
    pads: Vec<(i32, AndroidPad)>,
    active: Option<i32>,
}

impl AndroidPads {
    /// None seen yet (for a `static`).
    pub const fn new() -> Self {
        Self {
            pads: Vec::new(),
            active: None,
        }
    }

    fn pad(&mut self, device: i32) -> &mut AndroidPad {
        let i = match self.pads.iter().position(|(d, _)| *d == device) {
            Some(i) => i,
            None => {
                self.pads.push((device, AndroidPad::default()));
                self.pads.len() - 1
            }
        };
        &mut self.pads[i].1
    }

    /// A key of `device`'s: pressed, it makes that pad the active one.
    pub fn key(&mut self, device: i32, button: Button, down: bool) {
        self.pad(device).key(button, down);
        if down {
            self.active = Some(device);
        }
    }

    /// A motion event of `device`'s: a stick pushed far, a trigger pulled or the hat pressed makes
    /// that pad the active one.
    pub fn motion(&mut self, device: i32, axes: Axes) {
        let pad = self.pad(device);
        pad.motion(axes);
        let state = pad.state();
        if state.buttons != 0
            || length(state.left) > TAKE_OVER
            || length(state.right) > TAKE_OVER
            || state.triggers.iter().any(|t| *t > TAKE_OVER)
        {
            self.active = Some(device);
        }
    }

    /// The active pad's device id and state; `None` before any pad was used.
    pub fn active(&self) -> Option<(i32, PadState)> {
        let device = self.active?;
        let pad = self.pads.iter().find(|(d, _)| *d == device)?;
        Some((device, pad.1.state()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYBOARD: u32 = 0x0301;
    const GAMEPAD_KEYS: u32 = 0x0501;
    const JOYSTICK: u32 = 0x0100_0010;

    #[test]
    fn key_codes_are_buttons_by_position() {
        let cases = [
            (96, Button::South),
            (97, Button::East),
            (99, Button::West),
            (100, Button::North),
            (102, Button::LeftBumper),
            (103, Button::RightBumper),
            (104, Button::LeftTrigger),
            (105, Button::RightTrigger),
            (106, Button::LeftStick),
            (107, Button::RightStick),
            (108, Button::Start),
            (109, Button::Select),
            (19, Button::DPadUp),
            (20, Button::DPadDown),
            (21, Button::DPadLeft),
            (22, Button::DPadRight),
        ];
        for (code, button) in cases {
            assert_eq!(super::button(code), Some(button), "{code}");
        }
        // BUTTON_C, BUTTON_Z, BUTTON_MODE (the system's), a letter.
        for code in [98, 101, 110, 29] {
            assert_eq!(super::button(code), None, "{code}");
        }
    }

    #[test]
    fn a_keyboards_arrows_stay_the_keyboards() {
        assert_eq!(pad_key(KEYBOARD, 19), None);
        assert_eq!(pad_key(GAMEPAD_KEYS, 19), Some(Button::DPadUp));
        assert_eq!(pad_key(JOYSTICK, 22), Some(Button::DPadRight));
        // A pad's own buttons, whatever the source says.
        assert_eq!(pad_key(KEYBOARD, 96), Some(Button::South));
        assert_eq!(pad_key(GAMEPAD_KEYS, 29), None);
        assert!(from_pad(GAMEPAD_KEYS) && from_pad(JOYSTICK) && !from_pad(KEYBOARD));
        // A touch screen, a mouse and a captured mouse are not joysticks.
        for source in [0x1002, 0x2002, 0x0002_0004] {
            assert!(!joystick(source), "{source:#x}");
        }
        assert!(joystick(JOYSTICK));
    }

    #[test]
    fn sticks_point_up_and_rest_centred() {
        let mut pad = AndroidPad::default();
        pad.motion(Axes {
            x: 0.5,
            y: -1.0,
            z: -0.25,
            rz: 0.75,
            ..Axes::default()
        });
        let s = pad.state();
        assert_eq!(s.left, [0.5, 1.0]);
        assert_eq!(s.right, [-0.25, -0.75]);
        // Drift at rest is nothing; a worn stick past 1 is 1.
        pad.motion(Axes {
            x: 0.05,
            y: 0.06,
            z: 1.2,
            ..Axes::default()
        });
        let s = pad.state();
        assert_eq!(s.left, [0.0, 0.0]);
        assert_eq!(s.right, [1.0, 0.0]);
        assert_eq!((s.buttons, s.triggers), (0, [0.0, 0.0]));
    }

    #[test]
    fn the_hat_is_the_dpad() {
        let mut pad = AndroidPad::default();
        pad.motion(Axes {
            hat_x: -1.0,
            hat_y: 1.0,
            ..Axes::default()
        });
        let s = pad.state();
        assert!(s.held(Button::DPadLeft) && s.held(Button::DPadDown));
        assert!(!s.held(Button::DPadRight) && !s.held(Button::DPadUp));
        // The D-pad's keys and the hat at once (pads that send both).
        pad.key(Button::DPadUp, true);
        assert!(pad.state().held(Button::DPadUp));
        pad.motion(Axes::default());
        pad.key(Button::DPadUp, false);
        assert_eq!(pad.state().buttons, 0);
    }

    #[test]
    fn triggers_read_either_axis_or_the_key() {
        let mut pad = AndroidPad::default();
        pad.motion(Axes {
            ltrigger: 0.3,
            brake: 0.6,
            gas: 0.8,
            ..Axes::default()
        });
        assert_eq!(pad.state().triggers, [0.6, 0.8]);
        pad.motion(Axes {
            rtrigger: 0.02,
            ..Axes::default()
        });
        assert_eq!(pad.state().triggers, [0.0, 0.0]);
        // A trigger that comes as a key pulls all the way, and is a value, not a button.
        pad.key(Button::LeftTrigger, true);
        let s = pad.state();
        assert_eq!(s.triggers, [1.0, 0.0]);
        assert!(!s.held(Button::LeftTrigger));
        assert_eq!(s.buttons, 0);
    }

    #[test]
    fn the_last_pad_used_is_the_one_read() {
        let mut pads = AndroidPads::default();
        assert_eq!(pads.active(), None);
        // Drift and a light push do not take over.
        pads.motion(
            7,
            Axes {
                x: 0.3,
                ..Axes::default()
            },
        );
        assert_eq!(pads.active(), None);
        pads.key(7, Button::South, true);
        let (device, state) = pads.active().unwrap();
        assert_eq!(device, 7);
        assert!(state.held(Button::South));
        assert_eq!(state.left, [0.3, 0.0]);
        // Another pad's stick pushed far takes over; the first one's buttons no longer count.
        pads.motion(
            9,
            Axes {
                y: -0.9,
                ..Axes::default()
            },
        );
        let (device, state) = pads.active().unwrap();
        assert_eq!(device, 9);
        assert!(!state.held(Button::South));
        assert_eq!(state.left, [0.0, 0.9]);
        // A release does not take over.
        pads.key(7, Button::South, false);
        assert_eq!(pads.active().unwrap().0, 9);
    }
}
