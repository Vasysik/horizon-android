//! Touch input runtime, multi-touch event dispatch, and input mapping.

use super::model::*;
use std::collections::{BTreeMap, BTreeSet};
use winit::event::TouchPhase;
use winit::keyboard::KeyCode;

/// Action emitted by the control layout runtime to be processed by the game engine.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutOutputEvent {
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    MouseButton(bool, bool), // (is_down, is_left_button)
    Scroll(f32),
    Launcher(String),
    SendText(String),
}

/// Runtime state for an individual button.
#[derive(Debug, Clone, Default)]
pub struct ButtonRuntimeState {
    pub is_pressed: bool,
    pub is_toggled: bool,
    pub active_pointers: BTreeSet<u64>,
}

/// Runtime state for an analog joystick.
#[derive(Debug, Clone, Default)]
pub struct JoystickRuntimeState {
    pub active_pointer: Option<u64>,
    pub normalized_offset: (f32, f32), // (-1.0..1.0, -1.0..1.0)
    pub is_locked: bool,
    pub active_directions: BTreeSet<JoystickDirection>,
    pub lock_active: bool,
}

/// Internal mutable tracking state for touch points, held keys, and button toggles.
#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    pub is_in_menu: bool,
    pub physical_mouse_active: bool,
    pub gamepad_active: bool,
    pub layer_visibility_overrides: BTreeMap<String, bool>,
    pub buttons_state: BTreeMap<String, ButtonRuntimeState>,
    pub joysticks_state: BTreeMap<String, JoystickRuntimeState>,
    pub pointer_positions: BTreeMap<u64, (f32, f32)>,
    pub held_keys: BTreeSet<KeyCode>,
}

/// Runtime manager driving on-screen controls, multi-touch tracking, and layout rendering.
pub struct LayoutRuntime {
    layout: ControlLayout,
    screen_width: f32,
    screen_height: f32,
    display_density: f32,
    state: RuntimeState,
}

impl LayoutRuntime {
    pub fn new(layout: ControlLayout) -> Self {
        Self {
            layout,
            screen_width: 1920.0,
            screen_height: 1080.0,
            display_density: 1.0,
            state: RuntimeState::default(),
        }
    }

    pub fn layout(&self) -> &ControlLayout {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut ControlLayout {
        &mut self.layout
    }

    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    pub fn set_layout(&mut self, layout: ControlLayout) -> Vec<LayoutOutputEvent> {
        let release_events = self.release_all_inputs();
        self.layout = layout;
        self.state.buttons_state.clear();
        self.state.joysticks_state.clear();
        self.state.layer_visibility_overrides.clear();
        release_events
    }

    pub fn set_screen_dimensions(&mut self, width: f32, height: f32, density: f32) {
        self.screen_width = width.max(10.0);
        self.screen_height = height.max(10.0);
        self.display_density = density.max(0.1);
    }

    pub fn set_in_menu(&mut self, in_menu: bool) -> Vec<LayoutOutputEvent> {
        self.state.is_in_menu = in_menu;
        self.state.evaluate_layer_visibility_changes(&self.layout)
    }

    pub fn set_physical_mouse_active(&mut self, active: bool) -> Vec<LayoutOutputEvent> {
        self.state.physical_mouse_active = active;
        self.state.evaluate_layer_visibility_changes(&self.layout)
    }

    pub fn set_gamepad_active(&mut self, active: bool) -> Vec<LayoutOutputEvent> {
        self.state.gamepad_active = active;
        self.state.evaluate_layer_visibility_changes(&self.layout)
    }

    pub fn screen_width(&self) -> f32 {
        self.screen_width
    }

    pub fn screen_height(&self) -> f32 {
        self.screen_height
    }

    pub fn display_density(&self) -> f32 {
        self.display_density
    }

    pub fn is_layer_visible(&self, layer: &ControlLayer) -> bool {
        self.state.is_layer_visible(layer)
    }

    /// Handles a touch input event, returning game input actions.
    pub fn handle_touch(
        &mut self,
        pointer_id: u64,
        phase: TouchPhase,
        pos: (f32, f32),
    ) -> Vec<LayoutOutputEvent> {
        let sw = self.screen_width;
        let sh = self.screen_height;
        let density = self.display_density;
        self.state.handle_touch(&self.layout, sw, sh, density, pointer_id, phase, pos)
    }

    /// Releases all currently held keys and active controls.
    pub fn release_all_inputs(&mut self) -> Vec<LayoutOutputEvent> {
        self.state.release_all_inputs()
    }
}

impl RuntimeState {
    /// Checks if a layer is currently visible.
    pub fn is_layer_visible(&self, layer: &ControlLayer) -> bool {
        if let Some(&forced) = self.layer_visibility_overrides.get(&layer.uuid) {
            return forced;
        }
        if layer.hide {
            return false;
        }
        if self.physical_mouse_active && layer.hide_when_mouse {
            return false;
        }
        if self.gamepad_active && layer.hide_when_gamepad {
            return false;
        }
        match layer.visibility_type {
            VisibilityType::Always => true,
            VisibilityType::InGame => !self.is_in_menu,
            VisibilityType::InMenu => self.is_in_menu,
        }
    }

    /// Evaluates visibility and releases keys of any controls that are no longer visible.
    pub fn evaluate_layer_visibility_changes(&mut self, layout: &ControlLayout) -> Vec<LayoutOutputEvent> {
        let mut events = Vec::new();
        for layer in &layout.layers {
            if !self.is_layer_visible(layer) {
                for btn in &layer.normal_buttons {
                    events.extend(self.release_button(btn));
                }
                for joy in &layer.joystick_buttons {
                    events.extend(self.release_joystick(joy));
                }
            }
        }
        events
    }

    /// Handles a touch input event against the layout geometry.
    pub fn handle_touch(
        &mut self,
        layout: &ControlLayout,
        screen_width: f32,
        screen_height: f32,
        density: f32,
        pointer_id: u64,
        phase: TouchPhase,
        pos: (f32, f32),
    ) -> Vec<LayoutOutputEvent> {
        let mut events = Vec::new();

        match phase {
            TouchPhase::Started => {
                self.pointer_positions.insert(pointer_id, pos);
                self.physical_mouse_active = false;

                let mut captured = false;

                for layer in layout.layers.iter().rev() {
                    if !self.is_layer_visible(layer) {
                        continue;
                    }

                    // 1. Joysticks
                    for joy in &layer.joystick_buttons {
                        if !matches!(joy.visibility_type, VisibilityType::Always) {
                            if joy.visibility_type == VisibilityType::InGame && self.is_in_menu {
                                continue;
                            }
                            if joy.visibility_type == VisibilityType::InMenu && !self.is_in_menu {
                                continue;
                            }
                        }

                        let (jx, jy, jr) = calculate_joystick_bounds(joy, screen_width, screen_height, density);
                        let dx = pos.0 - jx;
                        let dy = pos.1 - jy;
                        let dist = (dx * dx + dy * dy).sqrt();

                        if dist <= jr * 1.5 {
                            let state = self.joysticks_state.entry(joy.uuid.clone()).or_default();
                            state.active_pointer = Some(pointer_id);
                            events.extend(self.update_joystick(joy, dx, dy, jr));
                            captured = true;
                            break;
                        }
                    }
                    if captured {
                        break;
                    }

                    // 2. Normal Buttons
                    for btn in &layer.normal_buttons {
                        if !matches!(btn.visibility_type, VisibilityType::Always) {
                            if btn.visibility_type == VisibilityType::InGame && self.is_in_menu {
                                continue;
                            }
                            if btn.visibility_type == VisibilityType::InMenu && !self.is_in_menu {
                                continue;
                            }
                        }

                        let (bx, by, bw, bh) = calculate_button_bounds(&btn.position, &btn.button_size, screen_width, screen_height, density);
                        if pos.0 >= bx && pos.0 <= bx + bw && pos.1 >= by && pos.1 <= by + bh {
                            events.extend(self.press_button(btn, layout, pointer_id));
                            if !btn.is_penetrable {
                                captured = true;
                            }
                            break;
                        }
                    }
                    if captured {
                        break;
                    }
                }
            }
            TouchPhase::Moved => {
                self.pointer_positions.insert(pointer_id, pos);

                // Update active joysticks
                for layer in &layout.layers {
                    if !self.is_layer_visible(layer) {
                        continue;
                    }
                    for joy in &layer.joystick_buttons {
                        let is_active = self.joysticks_state.get(&joy.uuid)
                            .and_then(|s| s.active_pointer) == Some(pointer_id);
                        if is_active {
                            let (jx, jy, jr) = calculate_joystick_bounds(joy, screen_width, screen_height, density);
                            let dx = pos.0 - jx;
                            let dy = pos.1 - jy;
                            events.extend(self.update_joystick(joy, dx, dy, jr));
                        }
                    }
                }

                // Check button bounds
                for layer in &layout.layers {
                    if !self.is_layer_visible(layer) {
                        continue;
                    }
                    for btn in &layer.normal_buttons {
                        let (bx, by, bw, bh) = calculate_button_bounds(&btn.position, &btn.button_size, screen_width, screen_height, density);
                        let inside = pos.0 >= bx && pos.0 <= bx + bw && pos.1 >= by && pos.1 <= by + bh;

                        let is_holding = self.buttons_state.get(&btn.uuid)
                            .map(|s| s.active_pointers.contains(&pointer_id))
                            .unwrap_or(false);

                        if is_holding && !inside && !btn.is_swipple {
                            let state = self.buttons_state.entry(btn.uuid.clone()).or_default();
                            state.active_pointers.remove(&pointer_id);
                            if state.active_pointers.is_empty() && !btn.is_toggleable {
                                state.is_pressed = false;
                                events.extend(self.dispatch_click_events(&btn.click_events, false));
                            }
                        } else if !is_holding && inside && btn.is_swipple {
                            events.extend(self.press_button(btn, layout, pointer_id));
                        }
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.pointer_positions.remove(&pointer_id);

                for layer in &layout.layers {
                    for btn in &layer.normal_buttons {
                        let should_release = self.buttons_state.get(&btn.uuid)
                            .map(|s| s.active_pointers.contains(&pointer_id))
                            .unwrap_or(false);

                        if should_release {
                            let state = self.buttons_state.entry(btn.uuid.clone()).or_default();
                            state.active_pointers.remove(&pointer_id);
                            if state.active_pointers.is_empty() && !btn.is_toggleable {
                                state.is_pressed = false;
                                events.extend(self.dispatch_click_events(&btn.click_events, false));
                            }
                        }
                    }

                    for joy in &layer.joystick_buttons {
                        let is_active = self.joysticks_state.get(&joy.uuid)
                            .and_then(|s| s.active_pointer) == Some(pointer_id);
                        if is_active {
                            let state = self.joysticks_state.entry(joy.uuid.clone()).or_default();
                            state.active_pointer = None;
                            if !state.is_locked {
                                events.extend(self.release_joystick(joy));
                            }
                        }
                    }
                }
            }
        }

        events
    }

    fn press_button(&mut self, btn: &NormalButton, layout: &ControlLayout, pointer_id: u64) -> Vec<LayoutOutputEvent> {
        let mut events = Vec::new();

        let is_press = {
            let state = self.buttons_state.entry(btn.uuid.clone()).or_default();
            state.active_pointers.insert(pointer_id);
            if btn.is_toggleable {
                state.is_toggled = !state.is_toggled;
                state.is_pressed = state.is_toggled;
                state.is_toggled
            } else {
                state.is_pressed = true;
                true
            }
        };

        events.extend(self.dispatch_click_events(&btn.click_events, is_press));

        // Process layer events
        let mut evaluate_visibility = false;
        for ev in &btn.click_events {
            match ev.event_type {
                ClickEventType::SwitchLayer => {
                    let current = self.layer_visibility_overrides.get(&ev.key).copied().unwrap_or(false);
                    self.layer_visibility_overrides.insert(ev.key.clone(), !current);
                    evaluate_visibility = true;
                }
                ClickEventType::ShowLayer => {
                    self.layer_visibility_overrides.insert(ev.key.clone(), true);
                    evaluate_visibility = true;
                }
                ClickEventType::HideLayer => {
                    self.layer_visibility_overrides.insert(ev.key.clone(), false);
                    evaluate_visibility = true;
                }
                _ => {}
            }
        }

        if evaluate_visibility {
            events.extend(self.evaluate_layer_visibility_changes(layout));
        }

        events
    }

    fn release_button(&mut self, btn: &NormalButton) -> Vec<LayoutOutputEvent> {
        let should_release = if let Some(state) = self.buttons_state.get_mut(&btn.uuid) {
            state.active_pointers.clear();
            if state.is_pressed || state.is_toggled {
                state.is_pressed = false;
                state.is_toggled = false;
                true
            } else {
                false
            }
        } else {
            false
        };

        if should_release {
            self.dispatch_click_events(&btn.click_events, false)
        } else {
            Vec::new()
        }
    }

    fn update_joystick(
        &mut self,
        joy: &JoystickButton,
        dx: f32,
        dy: f32,
        radius: f32,
    ) -> Vec<LayoutOutputEvent> {
        let mut events = Vec::new();
        let dist = (dx * dx + dy * dy).sqrt();
        let max_radius = radius.max(1.0);

        let (dirs_to_activate, dirs_to_deactivate, lock_event_press, lock_event_release) = {
            let state = self.joysticks_state.entry(joy.uuid.clone()).or_default();

            if joy.can_lock && dist > max_radius * (1.0 + joy.lock_threshold) {
                state.is_locked = true;
            }

            let clamped_dist = dist.min(max_radius);
            let norm_x = if dist > 0.0 { (dx / dist) * (clamped_dist / max_radius) } else { 0.0 };
            let norm_y = if dist > 0.0 { (dy / dist) * (clamped_dist / max_radius) } else { 0.0 };
            state.normalized_offset = (norm_x, norm_y);

            let mut desired_directions = BTreeSet::new();
            let active_radius_ratio = clamped_dist / max_radius;

            if active_radius_ratio > joy.dead_zone_ratio {
                let angle = dy.atan2(dx);
                let deg = angle.to_degrees();

                if (-112.5..=-67.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Forward);
                } else if (-67.5..=-22.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Forward);
                    desired_directions.insert(JoystickDirection::Right);
                } else if (-22.5..=22.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Right);
                } else if (22.5..=67.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Right);
                    desired_directions.insert(JoystickDirection::Back);
                } else if (67.5..=112.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Back);
                } else if (112.5..=157.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Back);
                    desired_directions.insert(JoystickDirection::Left);
                } else if (-157.5..=-112.5).contains(&deg) {
                    desired_directions.insert(JoystickDirection::Forward);
                    desired_directions.insert(JoystickDirection::Left);
                } else {
                    desired_directions.insert(JoystickDirection::Left);
                }
            }

            let mut to_activate = Vec::new();
            let mut to_deactivate = Vec::new();

            for dir in [
                JoystickDirection::Forward,
                JoystickDirection::Back,
                JoystickDirection::Left,
                JoystickDirection::Right,
            ] {
                let was_active = state.active_directions.contains(&dir);
                let is_active = desired_directions.contains(&dir);

                if !was_active && is_active {
                    to_activate.push(dir);
                } else if was_active && !is_active {
                    to_deactivate.push(dir);
                }
            }
            state.active_directions = desired_directions;

            let (lock_press, lock_release) = if state.is_locked && !state.lock_active {
                state.lock_active = true;
                (true, false)
            } else if !state.is_locked && state.lock_active {
                state.lock_active = false;
                (false, true)
            } else {
                (false, false)
            };

            (to_activate, to_deactivate, lock_press, lock_release)
        };

        for dir in dirs_to_activate {
            if let Some(dir_events) = joy.direction_events.get(&dir) {
                events.extend(self.dispatch_click_events(dir_events, true));
            }
        }
        for dir in dirs_to_deactivate {
            if let Some(dir_events) = joy.direction_events.get(&dir) {
                events.extend(self.dispatch_click_events(dir_events, false));
            }
        }

        if lock_event_press {
            events.extend(self.dispatch_click_events(&joy.lock_events, true));
        } else if lock_event_release {
            events.extend(self.dispatch_click_events(&joy.lock_events, false));
        }

        events
    }

    fn release_joystick(&mut self, joy: &JoystickButton) -> Vec<LayoutOutputEvent> {
        let (dirs_to_release, was_lock_active) = if let Some(state) = self.joysticks_state.get_mut(&joy.uuid) {
            state.is_locked = false;
            state.active_pointer = None;
            state.normalized_offset = (0.0, 0.0);
            let dirs: Vec<JoystickDirection> = std::mem::take(&mut state.active_directions).into_iter().collect();
            let lock = state.lock_active;
            state.lock_active = false;
            (dirs, lock)
        } else {
            (Vec::new(), false)
        };

        let mut events = Vec::new();
        for dir in dirs_to_release {
            if let Some(dir_events) = joy.direction_events.get(&dir) {
                events.extend(self.dispatch_click_events(dir_events, false));
            }
        }
        if was_lock_active {
            events.extend(self.dispatch_click_events(&joy.lock_events, false));
        }

        events
    }

    /// Dispatches key/action down or up events.
    pub fn dispatch_click_events(&mut self, events: &[ClickEvent], is_press: bool) -> Vec<LayoutOutputEvent> {
        let mut out = Vec::new();
        for ev in events {
            match ev.event_type {
                ClickEventType::Key => {
                    if let Some(keycode) = resolve_keycode(&ev.key) {
                        if is_press {
                            self.held_keys.insert(keycode);
                            out.push(LayoutOutputEvent::KeyDown(keycode));
                        } else {
                            self.held_keys.remove(&keycode);
                            out.push(LayoutOutputEvent::KeyUp(keycode));
                        }
                    } else if ev.key == "GLFW_MOUSE_BUTTON_LEFT" {
                        out.push(LayoutOutputEvent::MouseButton(is_press, true));
                    } else if ev.key == "GLFW_MOUSE_BUTTON_RIGHT" {
                        out.push(LayoutOutputEvent::MouseButton(is_press, false));
                    }
                }
                ClickEventType::LauncherEvent => {
                    if is_press {
                        out.push(LayoutOutputEvent::Launcher(ev.key.clone()));
                    }
                }
                ClickEventType::SendText => {
                    if is_press {
                        out.push(LayoutOutputEvent::SendText(ev.key.clone()));
                    }
                }
                ClickEventType::SwitchLayer | ClickEventType::ShowLayer | ClickEventType::HideLayer => {
                    // Handled synchronously in press_button
                }
            }
        }
        out
    }

    /// Releases all currently held keys and active controls.
    pub fn release_all_inputs(&mut self) -> Vec<LayoutOutputEvent> {
        let mut events = Vec::new();
        for &key in &self.held_keys {
            events.push(LayoutOutputEvent::KeyUp(key));
        }
        self.held_keys.clear();
        self.buttons_state.clear();
        self.joysticks_state.clear();
        self.pointer_positions.clear();
        events
    }
}

pub fn calculate_button_bounds(
    pos: &ButtonPosition,
    size: &ButtonSize,
    screen_width: f32,
    screen_height: f32,
    density: f32,
) -> (f32, f32, f32, f32) {
    let (x, y) = pos.to_screen_pixels(screen_width, screen_height);
    let (w, h) = size.to_screen_pixels(screen_width, screen_height, density);
    (x, y, w, h)
}

pub fn calculate_joystick_bounds(
    joy: &JoystickButton,
    screen_width: f32,
    screen_height: f32,
    density: f32,
) -> (f32, f32, f32) {
    let (x, y) = joy.position.to_screen_pixels(screen_width, screen_height);
    let diameter = match joy.size_type {
        SizeType::Dp => (joy.size_dp * density).max(20.0),
        SizeType::Percentage => (joy.size_percentage as f32 / 10000.0 * screen_height).max(20.0),
    };
    (x + diameter * 0.5, y + diameter * 0.5, diameter * 0.5)
}

/// Resolves a string key identifier (such as "GLFW_KEY_W" or "W") to a `winit::keyboard::KeyCode`.
pub fn resolve_keycode(key_str: &str) -> Option<KeyCode> {
    let clean = key_str.strip_prefix("GLFW_KEY_").unwrap_or(key_str);
    match clean {
        "A" => Some(KeyCode::KeyA),
        "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC),
        "D" => Some(KeyCode::KeyD),
        "E" => Some(KeyCode::KeyE),
        "F" => Some(KeyCode::KeyF),
        "G" => Some(KeyCode::KeyG),
        "H" => Some(KeyCode::KeyH),
        "I" => Some(KeyCode::KeyI),
        "J" => Some(KeyCode::KeyJ),
        "K" => Some(KeyCode::KeyK),
        "L" => Some(KeyCode::KeyL),
        "M" => Some(KeyCode::KeyM),
        "N" => Some(KeyCode::KeyN),
        "O" => Some(KeyCode::KeyO),
        "P" => Some(KeyCode::KeyP),
        "Q" => Some(KeyCode::KeyQ),
        "R" => Some(KeyCode::KeyR),
        "S" => Some(KeyCode::KeyS),
        "T" => Some(KeyCode::KeyT),
        "U" => Some(KeyCode::KeyU),
        "V" => Some(KeyCode::KeyV),
        "W" => Some(KeyCode::KeyW),
        "X" => Some(KeyCode::KeyX),
        "Y" => Some(KeyCode::KeyY),
        "Z" => Some(KeyCode::KeyZ),
        "SPACE" => Some(KeyCode::Space),
        "LEFT_SHIFT" | "SHIFT" => Some(KeyCode::ShiftLeft),
        "RIGHT_SHIFT" => Some(KeyCode::ShiftRight),
        "LEFT_CONTROL" | "CONTROL" => Some(KeyCode::ControlLeft),
        "RIGHT_CONTROL" => Some(KeyCode::ControlRight),
        "ESCAPE" | "ESC" => Some(KeyCode::Escape),
        "ENTER" => Some(KeyCode::Enter),
        "TAB" => Some(KeyCode::Tab),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2),
        "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4),
        "5" => Some(KeyCode::Digit5),
        "6" => Some(KeyCode::Digit6),
        "7" => Some(KeyCode::Digit7),
        "8" => Some(KeyCode::Digit8),
        "9" => Some(KeyCode::Digit9),
        "0" => Some(KeyCode::Digit0),
        _ => None,
    }
}
