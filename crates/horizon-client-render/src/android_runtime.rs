//! Android renderer and platform policy.
#[path = "graphics_startup.rs"]
pub mod graphics_startup;
use crate::android_gamepad::{Devices, InputMode};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "android")]
static CACHE_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
pub fn configure_cache_dir(path: std::path::PathBuf) {
    let _ = CACHE_DIR.set(path);
}
#[cfg(target_os = "android")]
pub(crate) fn cache_dir() -> Option<&'static std::path::Path> {
    CACHE_DIR.get().map(std::path::PathBuf::as_path)
}
static INPUT_MODE: std::sync::Mutex<(InputMode, InputMode)> =
    std::sync::Mutex::new((InputMode::Auto, InputMode::Touch));
static GAMEPADS: std::sync::LazyLock<std::sync::Mutex<Devices>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Devices::default()));
pub fn input_mode() -> InputMode {
    INPUT_MODE.lock().unwrap().0
}
pub fn active_input() -> InputMode {
    INPUT_MODE.lock().unwrap().1
}
pub fn configure_input(mode: InputMode) {
    let mut input = INPUT_MODE.lock().unwrap();
    if input.0 != mode {
        *input = (
            mode,
            if mode == InputMode::Auto {
                InputMode::Touch
            } else {
                mode
            },
        );
        clear_gamepads();
        clear_keypad();
    }
}
pub fn note_input(kind: InputMode) -> bool {
    let mut input = INPUT_MODE.lock().unwrap();
    if input.0 != InputMode::Auto && input.0 != kind {
        return false;
    }
    if input.1 == InputMode::Controller && kind != InputMode::Controller {
        clear_gamepads();
    }
    input.1 = kind;
    true
}
pub fn gamepad_event(id: i32, event: &winit::event::DeviceEvent) -> bool {
    use winit::event::{DeviceEvent, ElementState};
    let activity = {
        let mut devices = GAMEPADS.lock().unwrap();
        match *event {
            DeviceEvent::Motion { axis, value } => devices.axis(id, axis, value),
            DeviceEvent::Button { button, state } => {
                devices.key(id, button, state == ElementState::Pressed)
            }
            DeviceEvent::Removed => {
                devices.clear();
                false
            }
            _ => return false,
        }
    };
    if activity {
        note_input(InputMode::Controller);
    }
    true
}
pub fn retain_gamepads(ids: &[i32]) {
    GAMEPADS.lock().unwrap().retain(ids);
}
pub fn clear_gamepads() {
    GAMEPADS.lock().unwrap().clear();
}
pub fn active_gamepad_id() -> Option<i32> {
    (active_input() == InputMode::Controller)
        .then(|| GAMEPADS.lock().unwrap().active_id())
        .flatten()
}
pub fn gamepad_state() -> crate::gamepad::PadState {
    if active_input() == InputMode::Controller {
        GAMEPADS.lock().unwrap().poll()
    } else {
        crate::gamepad::PadState::default()
    }
}
static ATLAS_LIMIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(16384);
/// The launcher opens the adapter before the resource worker builds the game atlas.
pub fn configure_atlas_limit(limit: u32) {
    ATLAS_LIMIT.store(limit, Ordering::Relaxed);
}
pub fn atlas_limit() -> u32 {
    ATLAS_LIMIT.load(Ordering::Relaxed)
}
static GPU_BATCHING: AtomicBool = AtomicBool::new(false);
static KEYPAD_ENABLED: AtomicBool = AtomicBool::new(false);
static KEYPAD: std::sync::LazyLock<std::sync::Mutex<crate::keypad::Keypad>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(crate::keypad::Keypad::default()));
pub fn configure_keypad_settings(settings: &crate::keypad::Settings) {
    KEYPAD.lock().unwrap().configure(settings);
    KEYPAD_ENABLED.store(settings.enabled, Ordering::Relaxed);
}
pub fn configure_keypad(enabled: bool) {
    KEYPAD_ENABLED.store(enabled, Ordering::Relaxed);
    clear_keypad();
}
pub fn keypad_enabled() -> bool {
    KEYPAD_ENABLED.load(Ordering::Relaxed)
}
pub fn keypad_key(key: winit::keyboard::KeyCode, down: bool) -> bool {
    keypad_enabled() && KEYPAD.lock().unwrap().key(key, down)
}
pub fn clear_keypad() {
    KEYPAD.lock().unwrap().clear();
}
pub fn keypad_state() -> crate::gamepad::PadState {
    KEYPAD.lock().unwrap().poll()
}
static OPEN_GL: AtomicBool = AtomicBool::new(cfg!(target_pointer_width = "32"));
pub fn configure_graphics(open_gl: bool) {
    OPEN_GL.store(open_gl, Ordering::Relaxed);
}
pub fn graphics_api() -> crate::graphics::GraphicsApi {
    if OPEN_GL.load(Ordering::Relaxed) {
        crate::graphics::GraphicsApi::OpenGl
    } else {
        crate::graphics::GraphicsApi::Vulkan
    }
}
pub fn configure_gpu(batching: bool) {
    GPU_BATCHING.store(batching, Ordering::Relaxed);
}
pub fn compatibility() -> bool {
    cfg!(target_os = "android") && !GPU_BATCHING.load(Ordering::Relaxed)
}

pub use crate::clipboard::{ClipboardHooks, clipboard_get, clipboard_set, install_clipboard};

static RETURN_TO_LAUNCHER: AtomicBool = AtomicBool::new(false);
pub fn return_to_launcher() {
    RETURN_TO_LAUNCHER.store(true, Ordering::Release);
}
pub fn take_launcher_return() -> bool {
    RETURN_TO_LAUNCHER.swap(false, Ordering::AcqRel)
}

#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn explicit_modes_reject_other_devices_and_auto_follows_activity() {
        configure_input(InputMode::Touch);
        assert!(!note_input(InputMode::Controller));
        assert_eq!(active_input(), InputMode::Touch);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Button {
                button: 96,
                state: winit::event::ElementState::Pressed,
            },
        );
        assert_eq!(gamepad_state(), crate::gamepad::PadState::default());
        configure_input(InputMode::Controller);
        assert!(!note_input(InputMode::Touch));
        configure_input(InputMode::Auto);
        assert!(note_input(InputMode::KeyboardMouse));
        assert_eq!(active_input(), InputMode::KeyboardMouse);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Motion {
                axis: 0,
                value: 0.05,
            },
        );
        assert_eq!(active_input(), InputMode::KeyboardMouse);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Motion {
                axis: 0,
                value: 1.0,
            },
        );
        assert_eq!(active_input(), InputMode::Controller);
        assert!(note_input(InputMode::Touch));
        assert_eq!(gamepad_state(), crate::gamepad::PadState::default());
        clear_gamepads();
        configure_input(InputMode::Auto);
    }
}
