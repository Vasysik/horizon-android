//! Android pointer snapshots translated without turning mouse clicks into touch.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub action: u32,
    pub x: f64,
    pub y: f64,
    pub wheel: (f32, f32),
    pub buttons: u32,
    pub relative: bool,
    pub touchpad: bool,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Enter,
    Leave,
    Position(f64, f64),
    Relative(f64, f64),
    Button(u32, bool),
    Wheel(f32, f32),
}

#[derive(Default)]
pub struct MouseState {
    buttons: u32,
    inside: bool,
    touchpad_at: Option<(f64, f64)>,
}

impl MouseState {
    pub fn update(&mut self, sample: Sample) -> Vec<Event> {
        let mut events = Vec::new();
        let ending = matches!(sample.action, 3 | 10);
        if !ending && (!sample.x.is_finite() || !sample.y.is_finite()) {
            return events;
        }
        if ending {
            self.release(&mut events);
            if self.inside {
                events.push(Event::Leave);
            }
            self.inside = false;
            self.touchpad_at = None;
            return events;
        }
        if !self.inside {
            events.push(Event::Enter);
            self.inside = true;
        }
        if sample.touchpad {
            if sample.action == 2 {
                if let Some((x, y)) = self.touchpad_at {
                    events.push(Event::Relative(sample.x - x, sample.y - y));
                }
            }
            self.touchpad_at = if sample.action == 1 {
                None
            } else {
                Some((sample.x, sample.y))
            };
        } else if sample.relative {
            if sample.action == 2 && (sample.x != 0.0 || sample.y != 0.0) {
                events.push(Event::Relative(sample.x, sample.y));
            }
        } else {
            events.push(Event::Position(sample.x, sample.y));
        }
        let buttons = if sample.action == 1 {
            0
        } else {
            sample.buttons & 31
        };
        for bit in [1, 2, 4, 8, 16] {
            if (buttons ^ self.buttons) & bit != 0 {
                events.push(Event::Button(bit, buttons & bit != 0));
            }
        }
        self.buttons = buttons;
        if sample.action == 8
            && sample.wheel.0.is_finite()
            && sample.wheel.1.is_finite()
            && sample.wheel != (0.0, 0.0)
        {
            events.push(Event::Wheel(sample.wheel.0, sample.wheel.1));
        }
        events
    }

    fn release(&mut self, events: &mut Vec<Event>) {
        for bit in [1, 2, 4, 8, 16] {
            if self.buttons & bit != 0 {
                events.push(Event::Button(bit, false));
            }
        }
        self.buttons = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(action: u32, buttons: u32) -> Sample {
        Sample {
            action,
            buttons,
            x: 100.0,
            y: 200.0,
            wheel: (0.0, 0.0),
            relative: false,
            touchpad: false,
        }
    }
    #[test]
    fn hover_never_presses_and_repeated_snapshots_do_not_double_click() {
        let mut state = MouseState::default();
        assert_eq!(
            state.update(sample(7, 0)),
            [Event::Enter, Event::Position(100.0, 200.0)]
        );
        assert!(state.update(sample(0, 1)).contains(&Event::Button(1, true)));
        assert!(!state
            .update(sample(2, 1))
            .iter()
            .any(|e| matches!(e, Event::Button(..))));
        assert!(state
            .update(sample(1, 0))
            .contains(&Event::Button(1, false)));
    }
    #[test]
    fn simultaneous_buttons_release_independently_and_cancel_clears_all() {
        let mut state = MouseState::default();
        state.update(sample(11, 3));
        assert!(state
            .update(sample(12, 2))
            .contains(&Event::Button(1, false)));
        assert_eq!(
            state.update(sample(3, 0)),
            [Event::Button(2, false), Event::Leave]
        );
        assert!(state.update(sample(0, 1)).contains(&Event::Button(1, true)));
    }
    #[test]
    fn captured_motion_is_relative_and_buttons_do_not_repeat_motion() {
        let mut state = MouseState::default();
        let relative = Sample {
            relative: true,
            x: 9000.0,
            y: -8000.0,
            ..sample(2, 0)
        };
        assert_eq!(
            state.update(relative),
            [Event::Enter, Event::Relative(9000.0, -8000.0)]
        );
        assert_eq!(
            state.update(Sample {
                action: 11,
                buttons: 4,
                ..relative
            }),
            [Event::Button(4, true)]
        );
    }
    #[test]
    fn wheel_preserves_axes_and_does_not_become_camera_motion() {
        let mut state = MouseState::default();
        let events = state.update(Sample {
            relative: true,
            wheel: (-1.0, 2.0),
            ..sample(8, 0)
        });
        assert_eq!(events, [Event::Enter, Event::Wheel(-1.0, 2.0)]);
    }
    #[test]
    fn touchpad_lifts_reset_the_anchor() {
        let mut state = MouseState::default();
        state.update(Sample {
            touchpad: true,
            ..sample(0, 0)
        });
        assert!(state
            .update(Sample {
                touchpad: true,
                x: 103.0,
                ..sample(2, 0)
            })
            .contains(&Event::Relative(3.0, 0.0)));
        state.update(Sample {
            touchpad: true,
            ..sample(1, 0)
        });
        assert!(!state
            .update(Sample {
                touchpad: true,
                x: 500.0,
                ..sample(2, 0)
            })
            .iter()
            .any(|e| matches!(e, Event::Relative(..))));
    }
}
