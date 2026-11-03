//! What the player plays with (horizon_client::input_mode): the touch controls, the chat's bar and
//! the on-screen keyboard show while it is the touch screen. On Android the mouse's events come
//! here from the input hook (android.rs) and go on as the window events a computer's mouse
//! makes; a physical keyboard's keys and a controller (game/pad.rs) also count as the input used.
use super::Game;
#[cfg(target_os = "android")]
use super::{event_loop::EventLoopCtl, input_trace::InputOrigin};
use horizon_client::input_mode::InputMode;
#[cfg(target_os = "android")]
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
};

impl Game<'_> {
    /// The player used `input`. Off the touch screen the fingers let go of what they held and the
    /// on-screen keyboard goes (touch_frame); any input but a controller takes the menus back
    /// from the pad's pointer.
    pub(super) fn input_used(&mut self, input: InputMode) {
        let mode = self.input_mode.after(input, cfg!(target_os = "android"));
        if mode == self.input_mode {
            return;
        }
        if self.input_mode == InputMode::Touch {
            self.touch_release_all();
        }
        if mode != InputMode::Pad {
            self.pad.in_use = false;
        }
        self.input_mode = mode;
    }

    /// The touch controls, the chat's bar and the on-screen keyboard show.
    pub(super) fn touch_shown(&self) -> bool {
        self.input_mode == InputMode::Touch
    }

    /// Once per loop iteration, before the touch frame: the pointer capture Android took back,
    /// a physical keyboard's keys, and the mouse's events since the last iteration, as window
    /// events (its captured motion turns the view as a computer's raw motion does).
    #[cfg(target_os = "android")]
    pub(super) fn android_input_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        if crate::android::pointer_capture_lost() && self.captured {
            self.capture(false);
        }
        if crate::android::keyboard_used() {
            self.input_used(InputMode::KeyboardMouse);
        }
        let events = crate::android::take_mouse_events();
        if events.is_empty() {
            return;
        }
        self.input_used(InputMode::KeyboardMouse);
        let Some(id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        let device_id = DeviceId::dummy();
        for event in events {
            let event = match event {
                crate::android::MouseEvent::Moved(x, y) => WindowEvent::CursorMoved {
                    device_id,
                    position: PhysicalPosition::new(x, y),
                },
                crate::android::MouseEvent::Button(button, down) => WindowEvent::MouseInput {
                    device_id,
                    state: if down {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                    button,
                },
                crate::android::MouseEvent::Wheel(x, y) => WindowEvent::MouseWheel {
                    device_id,
                    delta: MouseScrollDelta::LineDelta(x, y),
                    phase: TouchPhase::Moved,
                },
                crate::android::MouseEvent::Motion(x, y) => {
                    self.grabbed_mouse_motion(x, y);
                    continue;
                }
            };
            self.handle_window_event(event_loop, id, event, InputOrigin::User);
        }
    }
}
