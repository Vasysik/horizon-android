//! Android pointer snapshots translated without turning mouse clicks into touch.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub action: u32,
    pub x: f64,
    pub y: f64,
    pub rel_x: f64,
    pub rel_y: f64,
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
    x: Option<f64>,
    y: Option<f64>,
    touchpad_at: Option<(f64, f64)>,
}

impl MouseState {
    /// Captured deltas are per sample, rather than a total for the packet.
    pub fn update_with_history(
        &mut self,
        mut sample: Sample,
        history: impl IntoIterator<Item = (f64, f64)>,
    ) -> Vec<Event> {
        if sample.relative && !sample.touchpad && sample.action == 2 {
            let current = (sample.x, sample.y);
            sample.x = 0.0;
            sample.y = 0.0;
            for (x, y) in history {
                if x.is_finite() && y.is_finite() {
                    sample.x += x;
                    sample.y += y;
                }
            }
            sample.x += current.0;
            sample.y += current.1;
        }
        self.update(sample)
    }

    pub fn update(&mut self, sample: Sample) -> Vec<Event> {
        let mut events = Vec::new();
        let ending = sample.action == 3;
        if !ending && (!sample.x.is_finite() || !sample.y.is_finite()) {
            return events;
        }
        if ending {
            self.release(&mut events);
            if self.inside {
                events.push(Event::Leave);
            }
            self.inside = false;
            self.x = None;
            self.y = None;
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
            self.x = None;
            self.y = None;
            if matches!(sample.action, 2 | 7) {
                let (dx, dy) = if sample.x != 0.0 || sample.y != 0.0 {
                    (sample.x, sample.y)
                } else {
                    (sample.rel_x, sample.rel_y)
                };
                if dx != 0.0 || dy != 0.0 {
                    events.push(Event::Relative(dx, dy));
                }
            }
        } else {
            if sample.rel_x != 0.0 || sample.rel_y != 0.0 {
                events.push(Event::Relative(sample.rel_x, sample.rel_y));
            } else if matches!(sample.action, 2 | 7) {
                if let (Some(px), Some(py)) = (self.x, self.y) {
                    let (dx, dy) = (sample.x - px, sample.y - py);
                    if dx != 0.0 || dy != 0.0 {
                        events.push(Event::Relative(dx, dy));
                    }
                }
            }
            self.x = Some(sample.x);
            self.y = Some(sample.y);
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
            rel_x: 0.0,
            rel_y: 0.0,
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
    fn hover_exit_does_not_synthesize_leave_or_release_buttons() {
        let mut state = MouseState::default();
        state.update(sample(7, 0));
        state.update(sample(0, 1));
        let hover_exit = state.update(sample(10, 1));
        assert!(!hover_exit.contains(&Event::Leave));
        assert!(!hover_exit.contains(&Event::Button(1, false)));
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
                action: 7,
                x: 0.0,
                y: 0.0,
                rel_x: 12.0,
                rel_y: -4.0,
                ..relative
            }),
            [Event::Relative(12.0, -4.0)]
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

    #[test]
    fn captured_batch_preserves_every_delta_and_only_one_button_edge() {
        let mut state = MouseState::default();
        let packet = Sample {
            relative: true,
            x: 2.0,
            y: -3.0,
            ..sample(2, 1)
        };
        assert_eq!(
            state.update_with_history(packet, [(4.0, 1.0), (-1.0, 2.0)]),
            [
                Event::Enter,
                Event::Relative(5.0, 0.0),
                Event::Button(1, true)
            ]
        );
        assert_eq!(
            state.update_with_history(
                Sample {
                    x: 0.0,
                    y: 0.0,
                    ..packet
                },
                [(3.0, -2.0)]
            ),
            [Event::Relative(3.0, -2.0)]
        );
    }

    #[test]
    fn batching_does_not_change_total_motion_while_a_button_is_held() {
        let mut state = MouseState::default();
        let samples = [(2.0, 1.0), (3.0, -4.0), (-1.0, 2.0), (4.0, 3.0)];
        let mut total = (0.0, 0.0);
        for (x, y) in samples {
            for event in state.update(Sample {
                relative: true,
                x,
                y,
                ..sample(2, 1)
            }) {
                if let Event::Relative(x, y) = event {
                    total.0 += x;
                    total.1 += y;
                }
            }
        }
        assert_eq!(total, (8.0, 2.0));
        let mut batched = MouseState::default();
        let result = batched.update_with_history(
            Sample {
                relative: true,
                x: 4.0,
                y: 3.0,
                ..sample(2, 1)
            },
            samples[..3].iter().copied(),
        );
        assert!(result.contains(&Event::Relative(8.0, 2.0)));
    }

    #[test]
    fn history_never_moves_absolute_or_button_only_packets() {
        let mut state = MouseState::default();
        assert_eq!(
            state.update_with_history(sample(7, 0), [(500.0, -100.0)]),
            [Event::Enter, Event::Position(100.0, 200.0)]
        );
        assert_eq!(
            state.update_with_history(
                Sample {
                    relative: true,
                    ..sample(11, 2)
                },
                [(500.0, -100.0)]
            ),
            [Event::Button(2, true)]
        );
    }

    #[test]
    fn invalid_history_does_not_discard_valid_relative_motion() {
        let mut state = MouseState::default();
        assert_eq!(
            state.update_with_history(
                Sample {
                    relative: true,
                    x: 1.0,
                    y: 2.0,
                    ..sample(2, 0)
                },
                [(f64::NAN, 4.0), (2.0, -1.0), (f64::INFINITY, 0.0)]
            ),
            [Event::Enter, Event::Relative(3.0, 1.0)]
        );
    }
}
