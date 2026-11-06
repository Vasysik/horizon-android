//! Extra desktop keys, a touch keyboard and menu scrolling on Android.
use super::*;
use horizon_client::{
    debug_keys::KeyAction,
    osk::Output,
    touch_ui::{Action, Gesture},
};
use winit::event::{Touch, TouchPhase};

impl Game<'_> {
    pub(crate) fn android_apply_display_scale(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer)
            && let Err(error) = crate::display::apply(window, renderer)
        {
            crate::display::set_scale(crate::display::RenderScale::default());
            log::warn!("Android display scale: {error}");
        }
    }

    pub(super) fn android_keypad_menu_key(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        event: &winit::event::KeyEvent,
    ) -> Result<bool> {
        if !self.pad_in_menu()
            || crate::platform::active()
            || self.ui.options.selected_key.is_some()
        {
            return Ok(false);
        }
        let Some(key) = horizon_client::keypad::resolve_key(event.physical_key, &event.logical_key)
        else {
            return Ok(false);
        };
        // Menu arrows work independently of the optional gameplay keypad profile.
        let arrow = matches!(
            key,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        );
        if !self.settings.keypad.enabled && !arrow && !self.pad.menu.keyboard_focus {
            return Ok(false);
        }
        if !self.settings.keypad.enabled
            && !matches!(
                key,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Enter
                    | KeyCode::NumpadEnter
                    | KeyCode::Space
                    | KeyCode::Escape
            )
        {
            return Ok(false);
        }
        let Some(button) = self.settings.keypad.menu_button(key) else {
            return Ok(false);
        };
        if !event.repeat {
            self.pad.menu.keyboard_focus = true;
            self.pad.in_use = true;
            self.pad_menu_button(events, button, event.state == ElementState::Pressed)?;
        }
        Ok(true)
    }
    /// A freely moving physical pointer can use the same utility buttons as touch.
    pub(super) fn android_aux_mouse(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        const POINTER: u64 = u64::MAX - 2;
        if !self.settings.keypad.enabled || (self.captured && crate::keyboard_mouse_input()) {
            return false;
        }
        let WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
            ..
        } = event
        else {
            return false;
        };
        if *device_id == DeviceId::dummy() {
            return false;
        }
        let Some(batch) = self.gui_scale() else {
            return false;
        };
        let point = [self.cursor.0, self.cursor.1];
        let normalized = [point[0] / batch.width, point[1] / batch.height];
        // What the pad has under the pointer, and what it holds: touches are
        // position-dependent and change with the layout, ownership survives
        // screen transitions (a held press must complete before the pad
        // frees it, see `aux_mouse_phase`).
        let touched = self
            .pad
            .touch_ui
            .action(batch.width, batch.height, point, self.pad_in_menu())
            .is_some()
            || (!self.pad_in_menu() && self.pad.touch.auxiliary_at(normalized));
        let owned = self.pad.touch_ui.owns_key(POINTER)
            || self.pad.touch_ui.owns_mouse(POINTER)
            || self.pad.touch.owns_pointer(POINTER);
        let Some(phase) = horizon_client::touch_ui::aux_mouse_phase(
            *state == ElementState::Pressed,
            owned,
            touched,
        ) else {
            return false;
        };
        let touch = Touch {
            device_id: *device_id,
            phase,
            location: winit::dpi::PhysicalPosition::new(
                f64::from(point[0] * batch.scale),
                f64::from(point[1] * batch.scale),
            ),
            force: None,
            id: POINTER,
        };
        self.handle_window_event(events, id, WindowEvent::Touch(touch));
        true
    }

    pub(super) fn android_custom_key_held(&self, key: KeyCode) -> bool {
        self.pad
            .touch
            .held_custom_tokens()
            .chain(self.pad.late_custom_releases.iter().map(String::as_str))
            .any(|token| {
                if let Some(mapping) = token.strip_prefix("action:") {
                    horizon_client::keybinds::key_code(
                        horizon_client::keybinds::KeyBinds::default_of(mapping),
                    ) == Some(key)
                } else {
                    horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                        physical == key
                            || self
                                .settings
                                .keys
                                .translate_all(physical)
                                .contains(&Ok(key))
                    })
                }
            })
    }
    pub(super) fn android_custom_mouse_held(&self, button: MouseButton) -> bool {
        self.pad
            .touch
            .held_custom_tokens()
            .chain(self.pad.late_custom_releases.iter().map(String::as_str))
            .any(|token| {
                if let Some(mapping) = token.strip_prefix("action:") {
                    horizon_client::keybinds::mouse_button(
                        horizon_client::keybinds::KeyBinds::default_of(mapping),
                    ) == Some(button)
                } else if horizon_client::keybinds::mouse_button(token) == Some(button) {
                    true
                } else {
                    horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                        self.settings
                            .keys
                            .translate_all(physical)
                            .contains(&Err(button))
                    })
                }
            })
    }
    pub(super) fn android_custom_events(&mut self) {
        for (name, down) in self.pad.touch.key_events() {
            self.android_custom_token_edge(&name, down);
        }
    }

    pub(super) fn android_custom_token_edge(&mut self, token: &str, down: bool) {
        let ticking = self.title.is_none()
            && self.screen == Screen::Playing
            && !self.pause
            && !self.pad_in_menu();
        if down {
            if let Some(i) = self
                .pad
                .late_custom_releases
                .iter()
                .position(|t| t == token)
            {
                let prev = self.pad.late_custom_releases.remove(i);
                self.android_dispatch_custom_token(&prev, false);
            }
            self.pad
                .custom_pressed_at
                .insert(token.to_owned(), self.ticks);
        } else if ticking && self.pad.custom_pressed_at.get(token) == Some(&self.ticks) {
            if !self.pad.late_custom_releases.iter().any(|t| t == token) {
                self.pad.late_custom_releases.push(token.to_owned());
            }
            return;
        }
        self.android_dispatch_custom_token(token, down);
    }

    pub(super) fn android_custom_token_tap(&mut self, token: &str) {
        self.android_custom_token_edge(token, true);
        if token != "mouse.virtual.toggle"
            && token != "mouse.scroll.up"
            && token != "mouse.scroll.down"
        {
            self.android_custom_token_edge(token, false);
        }
    }

    pub(super) fn android_dispatch_custom_token(&mut self, name: &str, down: bool) {
        self.last_input = Instant::now();
        self.pad.in_use = true;
        if down && !self.pad_in_menu() && !self.captured && self.focused {
            self.capture(true);
        }
        if self.screen == Screen::Options && self.options_waiting_for_key() {
            if !down {
                return;
            }
            if let Some(mapping) = name.strip_prefix("action:") {
                let default = horizon_client::keybinds::KeyBinds::default_of(mapping);
                if let Some(key) = horizon_client::keybinds::key_code(default) {
                    let _ = self.options_key(key, None);
                } else if let Some(button) = horizon_client::keybinds::mouse_button(default) {
                    let _ = self.options_bind_mouse(button);
                }
                return;
            }
            if let Some(button) = horizon_client::keybinds::mouse_button(name) {
                let _ = self.options_bind_mouse(button);
                return;
            }
            if let Some(key) = horizon_client::keybinds::key_code(name) {
                let _ = self.options_key(key, None);
                return;
            }
        }
        if let Some(mapping) = name.strip_prefix("action:") {
            if mapping == "key.sneak" || mapping == "key.sprint" {
                let slot = usize::from(mapping == "key.sprint");
                let key = [KeyCode::ShiftLeft, KeyCode::ControlLeft][slot];
                if down {
                    self.pad.custom_keys.insert(key);
                    self.physical_keys.insert(key);
                    self.keys.insert(key);
                } else {
                    self.pad.custom_keys.remove(&key);
                    self.physical_keys.remove(&key);
                    self.keys.remove(&key);
                }
                self.toggles.0[slot].set_down(down, false);
                self.sync_toggle(slot);
                if slot == 1 && !down {
                    if let Some(player) = &mut self.player {
                        player.sprinting = false;
                    }
                }
                return;
            }
            if self.pad_in_menu()
                && down
                && mapping == "key.inventory"
                && matches!(self.screen, Screen::Inventory | Screen::Creative)
            {
                let _ = self.pad_back();
                return;
            }
            let default = horizon_client::keybinds::KeyBinds::default_of(mapping);
            let result = if let Some(key) = horizon_client::keybinds::key_code(default) {
                self.mapped_key(key, down, false)
            } else if let Some(button) = horizon_client::keybinds::mouse_button(default) {
                self.android_custom_mouse(button, down)
            } else {
                Ok(())
            };
            if let Err(error) = result {
                log::warn!("Custom touch action: {error}");
            }
            if down && self.screen == Screen::Chat {
                self.android_show_keyboard();
            }
            return;
        }
        if let Some(button) = horizon_client::keybinds::mouse_button(name) {
            if let Err(error) = self.android_custom_mouse(button, down) {
                log::warn!("Custom mouse button: {error}");
            }
            return;
        }
        if name == "mouse.scroll.up" || name == "mouse.scroll.down" {
            if down {
                let delta = if name == "mouse.scroll.up" { 8.0 } else { 1.0 };
                let next = horizon_client::mouse::ScrollWheelHandler::next_selection(
                    delta,
                    self.selected as i32,
                    9,
                );
                self.selected = next as usize;
            }
            return;
        }
        if name == "mouse.look" {
            return;
        }
        if name == "mouse.virtual.toggle" {
            if down {
                if self.pad_in_menu() || !self.pad.touch.settings.virtual_mouse_enabled {
                    self.pad.touch.settings.virtual_mouse_enabled =
                        !self.pad.touch.settings.virtual_mouse_enabled;
                    self.pad.touch_ui.pointer_off = !self.pad.touch.settings.virtual_mouse_enabled;
                    if self.pad.touch.settings.virtual_mouse_enabled && !self.pad_in_menu() {
                        self.pad.touch_ui.goy_in_game_mouse = true;
                    } else {
                        self.pad.touch_ui.goy_in_game_mouse = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                } else {
                    self.pad.touch_ui.goy_in_game_mouse = !self.pad.touch_ui.goy_in_game_mouse;
                }
                self.android_sync_pointer();
                self.android_touch_haptic(14);
            }
            return;
        }
        let Some(key) = horizon_client::keybinds::key_code(name) else {
            return;
        };
        if down {
            self.pad.custom_keys.insert(key);
            self.physical_keys.insert(key);
        } else {
            self.pad.custom_keys.remove(&key);
            self.physical_keys.remove(&key);
        }
        if down
            && horizon_client::keybinds::key_name(key)
                == Some(self.settings.keys.get("key.fullscreen"))
        {
            self.toggle_fullscreen();
            return;
        }
        if down && self.screenshot_shortcut(key, false) {
            return;
        }
        let action = if down {
            KeyAction::Press
        } else {
            KeyAction::Release
        };
        if self.debug_key(key, action) {
            return;
        }
        self.mod_key(key, down, false);
        self.native_key_event(key, down, false);
        if self.screen == Screen::Options {
            if down {
                let _ = self.options_key(key, None);
            }
            return;
        }
        // Raw Shift and Control are the touch Sneak and Sprint buttons:
        // set their toggle state directly so both Hold and Toggle modes work reliably,
        // and also update `self.keys` so inventory Shift-click works.
        if matches!(
            key,
            KeyCode::ShiftLeft | KeyCode::ShiftRight | KeyCode::ControlLeft | KeyCode::ControlRight
        ) {
            if down {
                self.keys.insert(key);
            } else {
                self.keys.remove(&key);
            }
            let slot = usize::from(matches!(key, KeyCode::ControlLeft | KeyCode::ControlRight));
            self.toggles.0[slot].set_down(down, false);
            self.sync_toggle(slot);
            if slot == 1 && !down {
                if let Some(player) = &mut self.player {
                    player.sprinting = false;
                }
            }
            for mapped in self.settings.keys.translate_all(key) {
                match mapped {
                    Ok(code)
                        if !matches!(
                            code,
                            KeyCode::ShiftLeft
                                | KeyCode::ShiftRight
                                | KeyCode::ControlLeft
                                | KeyCode::ControlRight
                        ) =>
                    {
                        let _ = self.mapped_key(code, down, false);
                    }
                    Err(button) => {
                        let _ = self.android_custom_mouse(button, down);
                    }
                    _ => {}
                }
            }
            return;
        }
        if self.pad_in_menu() && down {
            if key == KeyCode::Escape {
                let _ = self.pad_back();
                return;
            }
            if key == KeyCode::KeyE && matches!(self.screen, Screen::Inventory | Screen::Creative) {
                let _ = self.pad_back();
                return;
            }
            if self.screen == Screen::Inventory
                && (self.merchant_key(key) || self.recipe_book_key(key))
            {
                return;
            }
            if self.screen == Screen::Chat && self.title.is_none() {
                self.keys.insert(key);
                self.chat_key_code(key, None);
                return;
            }
            if self.title.is_some() {
                let (w, h) = self.gui_size();
                let shift = self.has_shift_down();
                if let Some(t) = self.title.as_mut() {
                    match key {
                        KeyCode::Escape => {
                            t.escape();
                            self.save_title_options();
                        }
                        KeyCode::F5 => t.refresh_servers(),
                        KeyCode::Tab => t.tab_dir(shift, (w, h)),
                        KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::ArrowUp
                        | KeyCode::ArrowDown => t.arrow_key(key, (w, h)),
                        KeyCode::Backspace => self.title_edit(None, true),
                        _ => {}
                    }
                }
                return;
            }
        }
        if !down && self.screen == Screen::Chat && self.title.is_none() {
            self.keys.remove(&key);
        }
        if self.screen == Screen::Playing
            && down
            && horizon_client::keybinds::key_name(key)
                .is_some_and(|n| self.settings.keys.get("key.smoothCamera") == n)
        {
            self.smooth_camera = !self.smooth_camera;
        }
        let translated = self.settings.keys.translate_all(key);
        if translated.is_empty() {
            if down && matches!(self.screen, Screen::Inventory | Screen::Creative) {
                let _ = self.key_pressed(key);
            }
        } else {
            for mapped in translated {
                let result = match mapped {
                    Ok(code) => self.mapped_key(code, down, false),
                    Err(button) => self.android_custom_mouse(button, down),
                };
                if let Err(error) = result {
                    log::warn!("Custom touch key: {error}");
                }
            }
        }
        if down && matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat {
            self.android_show_keyboard();
        }
    }

    pub(super) fn android_touch_haptic(&self, millis: u32) {
        if self.pad.touch.settings.ui_vibrate {
            crate::platform::vibrate(horizon_client::gamepad::Pulse {
                strong: 0.30,
                weak: 0.55,
                millis,
            });
        }
    }

    pub(super) fn android_set_virtual_mouse(&mut self, enabled: bool) {
        if !enabled {
            self.pad.touch_ui.cancel_relative();
            for right in self.pad.touch_ui.release_all_mouse() {
                let button = if right {
                    MouseButton::Right
                } else {
                    MouseButton::Left
                };
                let _ = self.android_custom_mouse(button, false);
            }
            self.pad.touch_ui.virtual_mouse = false;
            return;
        }
        self.pad.touch_ui.virtual_mouse = true;
        if let Some(b) = self.gui_scale() {
            self.pad
                .touch_ui
                .sync_viewport_cursor(&mut self.cursor, [b.width, b.height]);
        }
    }

    /// State-driven Virtual Mouse visibility synchronization across MCPE and GoyControl.
    pub(super) fn android_sync_pointer(&mut self) {
        self.pad.touch_ui.pointer_off = !self.pad.touch.settings.virtual_mouse_enabled;
        let in_menu = self.pad_in_menu();
        if let Some(b) = self.gui_scale() {
            self.pad
                .touch_ui
                .sync_viewport_cursor(&mut self.cursor, [b.width, b.height]);
        }
        let want = self.pad.touch_ui.compute_virtual_mouse_visibility(
            &self.pad.touch,
            in_menu,
            self.ui.hide_gui,
            crate::touch_visible(),
            self.captured,
        );
        if self.pad.touch_ui.virtual_mouse != want {
            self.android_set_virtual_mouse(want);
        }
    }

    fn android_custom_mouse(&mut self, button: MouseButton, down: bool) -> Result<()> {
        if self.screen == Screen::Options && self.options_waiting_for_key() {
            if down {
                return self.options_bind_mouse(button);
            }
            return Ok(());
        }
        if self.pad_in_menu() && !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            let right = button == MouseButton::Right;
            if matches!(button, MouseButton::Left | MouseButton::Right) {
                let id = self.window.as_ref().map_or(WindowId::dummy(), |w| w.id());
                return self.android_menu_mouse(&super::event_loop::Noop, id, right, down);
            }
            return Ok(());
        }
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            if down && !self.captured && self.focused {
                self.capture(true);
            }
            self.extras_mouse(button, down);
            self.mod_mouse_key(button, down);
            if down
                && self.settings.keys.get("key.smoothCamera")
                    == horizon_client::keybinds::mouse_name(button)
            {
                self.smooth_camera = !self.smooth_camera;
            }
            let mapped = self.settings.keys.translate_mouse_all(button);
            if mapped.is_empty() {
                return self.play_mouse(button, down);
            }
            for item in mapped {
                match item {
                    Err(b) => self.play_mouse(b, down)?,
                    Ok(k) => self.mapped_key(k, down, false)?,
                }
            }
            return Ok(());
        }
        if let Some(key) = self
            .settings
            .keys
            .container_mouse_key(button, self.creative)
        {
            if down && self.carried.is_none() {
                self.inventory_hotkey(key);
            }
            return Ok(());
        }
        let right = button == MouseButton::Right;
        if !matches!(button, MouseButton::Left | MouseButton::Right) {
            if self.settings.keys.get("key.pickItem")
                == horizon_client::keybinds::mouse_name(button)
            {
                if down {
                    self.inventory_pick_press();
                } else {
                    self.inventory_pick_release();
                }
                if let Some(container) = self.container {
                    self.container_changed(container);
                }
            }
            return Ok(());
        }
        if down {
            if self.screen == Screen::Creative {
                self.click_creative(right);
            } else {
                self.inventory_press(right);
            }
            if let Some(container) = self.container {
                self.container_changed(container);
            }
        } else {
            self.inventory_mouse_release(right);
            if self.screen == Screen::Creative && !right {
                self.creative_screen.release();
            }
            self.station_release();
            self.recipe_book_release();
            self.loom_release();
        }
        Ok(())
    }

    fn android_menu_mouse(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
        right: bool,
        down: bool,
    ) -> Result<()> {
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        if matches!(self.screen, Screen::Inventory | Screen::Creative) {
            self.android_custom_mouse(button, down)
        } else {
            let id = if id == WindowId::dummy() {
                self.window.as_ref().map_or(WindowId::dummy(), |w| w.id())
            } else {
                id
            };
            self.pad_synthetic_event(
                events,
                id,
                WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    button,
                    state: if down {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                },
            );
            Ok(())
        }
    }
    pub(super) fn android_aux_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) -> Result<bool> {
        self.pad.touch_ui.goy_mode = self.pad.touch.settings.control_style
            == horizon_client::touch::ControlStyle::GoyControl;
        if t.phase == TouchPhase::Ended
            && let Some((right, last)) = self.pad.touch_ui.release_mouse(t.id)
        {
            if last {
                self.android_menu_mouse(events, id, right, false)?;
            }
            return Ok(true);
        }
        if t.phase == TouchPhase::Cancelled {
            self.pad.touch_ui.cancel();
            self.android_release_touch_keys(true);
            return Ok(true);
        }
        if t.phase == TouchPhase::Ended && self.pad.touch_ui.owns_key(t.id) {
            if let Some(key) = self.pad.touch_ui.key_up(t.id) {
                self.android_key_up(key);
            }
            return Ok(true);
        }
        let size = if let Some(r) = self.renderer.as_ref() {
            winit::dpi::PhysicalSize::new(r.config.width, r.config.height)
        } else if let Some(window) = self.window.as_ref() {
            window.inner_size()
        } else {
            return Ok(false);
        };
        let mut b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if !self.pad.touch_ui.owns_relative(t.id)
            && !self.pad.touch_ui.owns_gesture(t.id)
            && let Some(menu_act) = self.pad.touch_ui.floating_menu_touch(
                &mut b,
                &self.gui.atlas,
                t.id,
                t.phase,
                p,
                &mut self.pad.touch.settings,
            )
        {
            use horizon_client::touch_ui::InGameMenuAction;
            self.pad.touch.clear();
            self.pad.touch.ignore(t.id);
            if t.phase == TouchPhase::Started {
                self.audio.play_button_click();
                self.android_touch_haptic(12);
            }
            match menu_act {
                InGameMenuAction::Close => {}
                InGameMenuAction::OpenControlEditor => {
                    self.pad.touch.open_editor();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SetControlStyle(style) => {
                    self.android_release_touch_keys(true);
                    self.pad.touch_ui.goy_in_game_mouse = false;
                    self.pad.touch.clear();
                    self.pad
                        .touch
                        .settings
                        .switch_control_system(std::path::Path::new("."), style);
                    self.pad.touch_ui.goy_mode =
                        style == horizon_client::touch::ControlStyle::GoyControl;
                    self.pad.touch_ui.refresh_presets_cache(style);
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SelectPreset(preset_id) => {
                    let cur_style = self.pad.touch.settings.control_style;
                    self.pad.touch.settings.select_preset(
                        std::path::Path::new("."),
                        cur_style,
                        &preset_id,
                    );
                    self.pad.touch_ui.refresh_presets_cache(cur_style);
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CopyPreset {
                    source_id,
                    name,
                    description,
                } => {
                    let cur_style = self.pad.touch.settings.control_style;
                    if let Ok(meta) = horizon_client::touch::Settings::copy_preset(
                        std::path::Path::new("."),
                        cur_style,
                        &source_id,
                        &name,
                        &description,
                    ) {
                        self.pad.touch.settings.select_preset(
                            std::path::Path::new("."),
                            cur_style,
                            &meta.id,
                        );
                        self.pad.touch_ui.refresh_presets_cache(cur_style);
                        self.settings.touch = self.pad.touch.settings.clone();
                    }
                }
                InGameMenuAction::UpdatePresetMeta {
                    id: preset_id,
                    name,
                    description,
                } => {
                    let cur_style = self.pad.touch.settings.control_style;
                    let _ = self.pad.touch.settings.update_preset_meta(
                        std::path::Path::new("."),
                        cur_style,
                        &preset_id,
                        &name,
                        &description,
                    );
                    self.pad.touch_ui.refresh_presets_cache(cur_style);
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ApplyPreset(name) => {
                    self.pad.touch.settings.apply_preset(name);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::LoadSavedProfile(idx) => {
                    let saved =
                        horizon_client::touch::Settings::list_profiles(std::path::Path::new("."));
                    if let Some(name) = saved.get(idx) {
                        self.pad.touch.settings = horizon_client::touch::Settings::load_profile(
                            std::path::Path::new("."),
                            name,
                        );
                        self.settings.touch = self.pad.touch.settings.clone();
                    }
                }
                InGameMenuAction::PresetVisibilityAll => {
                    self.pad.touch.settings.apply_visibility_preset_all();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CycleMovementMode => {
                    use horizon_client::touch::Mode;
                    self.pad.touch.settings.mode = match self.pad.touch.settings.mode {
                        Mode::Dpad => Mode::Crosshair,
                        Mode::Crosshair => Mode::Tap,
                        Mode::Tap => Mode::Dpad,
                    };
                    if self.pad.touch.settings.mode != Mode::Crosshair {
                        self.pad.touch.settings.split_controls = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CycleJoystickArea => {
                    self.pad.touch.settings.joystick_style.move_area =
                        self.pad.touch.settings.joystick_style.move_area.next();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleSplitTouch => {
                    self.pad.touch.settings.toggle_split_touch();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleInvertY => {
                    let next = !self.pad.touch.settings.invert_y;
                    self.pad.touch.settings.invert_y = next;
                    self.settings.invert_y = next;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleLeftHanded => {
                    self.pad.touch.settings.left_handed = !self.pad.touch.settings.left_handed;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleSprintMode => {
                    let next = !self.pad.touch.settings.sprint_toggle;
                    self.pad.touch.settings.sprint_toggle = next;
                    self.pad.touch.settings.toggle_sprint = next;
                    self.settings.toggle_sprint = next;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleInventoryControls => {
                    self.pad.touch.settings.inventory_controls =
                        !self.pad.touch.settings.inventory_controls;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::ToggleVibrate | InGameMenuAction::ToggleUiVibrate => {
                    let ui = matches!(menu_act, InGameMenuAction::ToggleUiVibrate);
                    let value = if ui {
                        &mut self.pad.touch.settings.ui_vibrate
                    } else {
                        &mut self.pad.touch.settings.vibrate
                    };
                    *value = !*value;
                    let next = *value;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    if next && ui {
                        self.android_touch_haptic(22);
                    } else {
                        crate::platform::cancel_vibration();
                    }
                }
                InGameMenuAction::ToggleAutoJump => {
                    let next = !self.pad.touch.settings.auto_jump;
                    self.pad.touch.settings.auto_jump = next;
                    self.settings.auto_jump = next;
                    if let Some(player) = &mut self.player {
                        player.auto_jump_enabled = next;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleGyro => {
                    self.pad.touch.settings.gyroscope = !self.pad.touch.settings.gyroscope;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleVirtualMouse => {
                    let next = !self.pad.touch.settings.virtual_mouse_enabled;
                    self.pad.touch.settings.virtual_mouse_enabled = next;
                    self.pad.touch_ui.pointer_off = !next;
                    if !next {
                        self.pad.touch_ui.goy_in_game_mouse = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SetVirtualMouseSpeed(spd) => {
                    self.pad.touch.settings.virtual_mouse_sensitivity = spd.clamp(0.25, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepVirtualMouseSpeed(dir) => {
                    self.pad.touch.settings.virtual_mouse_sensitivity =
                        (self.pad.touch.settings.virtual_mouse_sensitivity + dir as f32 * 0.25)
                            .clamp(0.25, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetVirtualMousePointerSize(scale) => {
                    self.pad.touch.settings.virtual_mouse_pointer_scale = scale.clamp(0.5, 2.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetGyroSensitivity(val) => {
                    self.pad.touch.settings.gyro_sensitivity = val.clamp(0.10, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepGyroSensitivity(dir) => {
                    self.pad.touch.settings.gyro_sensitivity =
                        (self.pad.touch.settings.gyro_sensitivity + dir as f32 * 0.20)
                            .clamp(0.20, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetTouchSensitivity(sens) => {
                    self.pad.touch.settings.sensitivity = sens.clamp(0.10, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepTouchSensitivity(dir) => {
                    self.pad.touch.settings.sensitivity =
                        (self.pad.touch.settings.sensitivity + dir as f32 * 0.15).clamp(0.10, 3.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetButtonScale(val) => {
                    self.pad.touch.settings.button_scale = val.clamp(0.50, 2.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepButtonScale(dir) => {
                    self.pad.touch.settings.button_scale =
                        (self.pad.touch.settings.button_scale + dir as f32 * 0.10).clamp(0.5, 2.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::TriggerKey(key) => {
                    if !self.debug_key(key, KeyAction::Press) {
                        self.key_pressed(key)?;
                    }
                    self.debug_key(key, KeyAction::Release);
                    if matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat
                    {
                        self.android_show_keyboard();
                    }
                }
                InGameMenuAction::OpenKeyboard => {
                    self.android_show_keyboard();
                }
                InGameMenuAction::ForceKill => {
                    let _ = self.pad.touch.settings.save();
                    std::process::exit(0);
                }
                InGameMenuAction::OpenLogs => {
                    // Logs are private to the app; the SAF provider shares the Reports
                    // folder the diagnostics exporter writes into.
                    if !crate::platform::open_documents("reports") {
                        self.system_message(horizon_client::text::Text::literal(
                            "Could not open Reports. Logs are in the game folder's logs/ directory.",
                        ));
                    }
                }
                InGameMenuAction::EditPresetField(description) => {
                    if let Some(dlg) = self.pad.touch_ui.preset_dialog.as_mut() {
                        dlg.editing_desc = description;
                        crate::platform::show(if description {
                            &dlg.desc_input
                        } else {
                            &dlg.name_input
                        });
                    }
                }
                InGameMenuAction::SetResolutionScale(pct) => {
                    let clamped = pct.clamp(25, 100);
                    let changed = self.pad.touch.settings.resolution_scale_percent != clamped
                        || self.settings.horizon.resolution_scale != clamped;
                    self.pad.touch.settings.resolution_scale_percent = clamped;
                    self.settings.horizon.resolution_scale = clamped;
                    self.settings.touch = self.pad.touch.settings.clone();
                    if let Some(r) = self.renderer.as_mut() {
                        r.set_world_post(
                            clamped as f32 / 100.0,
                            self.settings.horizon.fxaa,
                            self.settings.horizon.taa,
                            self.settings.horizon.upscaling.ratio(),
                        );
                        r.invalidate_menu_blur();
                    }
                    if changed {
                        let _ = self.pad.touch.settings.save();
                        self.settings.save(&Self::options_path());
                    }
                }
                InGameMenuAction::ToggleGameAnimations => {
                    let next = !self.pad.touch.settings.disable_game_animations;
                    self.pad.touch.settings.disable_game_animations = next;
                    horizon_client::ui_theme::motion::set_game_animations_disabled(next);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    if let Some(r) = self.renderer.as_mut() {
                        r.animated_textures = self.settings.horizon.animated_textures && !next;
                        r.invalidate_menu_blur();
                    }
                }
                InGameMenuAction::SendCustomInput(token) => {
                    self.android_sync_pointer();
                    self.android_custom_token_tap(&token);
                }
            }
            return Ok(true);
        }
        if t.phase == TouchPhase::Moved {
            return Ok(false);
        }
        if t.phase == TouchPhase::Started
            && !self.pad.touch_ui.owns_relative(t.id)
            && !self.pad.touch_ui.owns_gesture(t.id)
            && let Some(action) = self
                .pad
                .touch_ui
                .action(b.width, b.height, p, self.pad_in_menu())
        {
            {
                self.pad.touch.clear();
                self.pad.touch.ignore(t.id);
                match action {
                    Action::LatchF3 => {
                        let was = self.pad.touch_ui.key_held(KeyCode::F3);
                        self.pad.touch_ui.f3_latched = !self.pad.touch_ui.f3_latched;
                        let now = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !was && now {
                            self.physical_keys.insert(KeyCode::F3);
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        if was && !now {
                            self.android_key_up(KeyCode::F3);
                        }
                    }
                    Action::Pick => {
                        self.play_mouse(MouseButton::Middle, true)?;
                        self.play_mouse(MouseButton::Middle, false)?;
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::DebugCombo(key) => {
                        let held = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        self.debug_key(key, KeyAction::Press);
                        self.debug_key(key, KeyAction::Release);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Release);
                        }
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::MouseButton(right) => {
                        if self.pad.touch_ui.press_mouse(t.id, right) {
                            self.android_menu_mouse(events, id, right, true)?;
                        }
                    }
                    Action::VirtualMouse => {
                        if matches!(self.screen, Screen::Inventory | Screen::Creative) {
                            // Cancel a prior drag without clicking/dropping at
                            // the toggle button, which is outside the inventory.
                            self.inv_clicks = Default::default();
                            self.inv_drag = None;
                            self.creative_screen.release();
                            self.loom_release();
                        }
                        for right in self.pad.touch_ui.release_all_mouse() {
                            self.android_menu_mouse(events, id, right, false)?;
                        }
                        self.pad.touch_ui.cancel();
                        let next = !self.pad.touch.settings.virtual_mouse_enabled;
                        self.pad.touch.settings.virtual_mouse_enabled = next;
                        self.pad.touch_ui.pointer_off = !next;
                        let _ = self.pad.touch.settings.save();
                        self.settings.touch = self.pad.touch.settings.clone();
                        self.android_sync_pointer();
                    }
                    Action::More => self.pad.touch_ui.expanded = !self.pad.touch_ui.expanded,
                    Action::Keyboard => {
                        if crate::platform::active() {
                            crate::platform::hide();
                        } else {
                            self.android_show_keyboard();
                        }
                    }
                    Action::Key(key) => {
                        if key != KeyCode::F3 && !self.pad.touch_ui.key_held(KeyCode::F3) {
                            self.pad.touch_ui.expanded = false;
                        }
                        if key == KeyCode::Escape {
                            crate::platform::hide();
                            self.pad_back()?;
                        } else if matches!(
                            key,
                            KeyCode::Tab | KeyCode::ShiftLeft | KeyCode::ControlLeft
                        ) {
                            let down = if key == KeyCode::Tab {
                                if self.physical_keys.remove(&key) {
                                    false
                                } else {
                                    self.physical_keys.insert(key);
                                    true
                                }
                            } else {
                                self.pad.touch_ui.toggle_modifier(key)
                            };
                            if !self.pad_in_menu() {
                                self.mapped_key(key, down, false)?;
                            }
                        } else {
                            if !self.pad.touch_ui.key_down(t.id, key) {
                                return Ok(true);
                            }
                            self.physical_keys.insert(key);
                            if !self.debug_key(key, KeyAction::Press) {
                                self.key_pressed(key)?;
                            }
                            if matches!(key, KeyCode::KeyT | KeyCode::Slash)
                                && self.screen == Screen::Chat
                            {
                                self.android_show_keyboard();
                            }
                        }
                    }
                }
            }
            return Ok(true);
        }
        let _ = (events, id);
        Ok(false)
    }
    pub(super) fn android_show_keyboard(&mut self) {
        // The in-world keyboard button needs a real editor to receive text.
        if !self.pad_in_menu() {
            self.open_chat("");
        }
        let initial = if self.dialog_open() {
            self.dialog_edit_value()
        } else if self.screen == Screen::BlockEditor {
            let Some(value) = self.block_editor_edit_value() else {
                return;
            };
            value
        } else if self.screen == Screen::CommandBlock {
            self.command_block_edit_value()
        } else if self.sign_edit.is_some() {
            self.sign_input(Some(KeyCode::End), None);
            self.sign_current().unwrap_or_default()
        } else if self.screen == Screen::Chat && self.title.is_none() {
            self.chat_key_code(KeyCode::End, None);
            self.ui
                .chat_screen
                .as_ref()
                .map(|s| s.value().to_owned())
                .unwrap_or_default()
        } else if let Some(t) = self.title.as_mut() {
            t.field_key(KeyCode::End, false, false);
            t.focused_field()
                .map(|(_, s, _)| s.to_owned())
                .unwrap_or_default()
        } else if self.screen == Screen::Creative {
            self.creative_screen.search.move_cursor_to_end(false);
            self.creative_screen.search.value.clone()
        } else if self.recipe_search_focused() {
            self.recipe_search_value()
        } else if self.screen == Screen::Options {
            self.ui.options.android_edit_value(&self.gui.atlas)
        } else if self.screen == Screen::WorldOptions {
            self.world_options_edit_value()
        } else if self.mod_browser_typing() {
            self.mod_browser_edit_value()
        } else if self.station_typing() {
            self.station
                .as_mut()
                .map(|s| {
                    s.rename.move_cursor_to_end(false);
                    s.rename.value.clone()
                })
                .unwrap_or_default()
        } else if self.book.is_some() {
            self.book_edit_value().unwrap_or_default()
        } else {
            String::new()
        };
        if matches!(
            self.book,
            Some(super::books::BookScreen::Edit { signing: false, .. })
        ) {
            crate::platform::show_multiline(&initial);
        } else {
            crate::platform::show(&initial);
        }
    }
    /// Text from the Android keyboard while a preset naming field has focus.
    /// Both naming dialogs (the editor's save-as and the hub's copy) type here
    /// instead of showing a keyboard inside the dialog.
    fn android_preset_typing(&mut self, out: &Output) -> bool {
        let handled = horizon_client::touch_ui::apply_editor_or_preset_output(
            self.pad.touch.editor.as_mut(),
            self.pad.touch_ui.preset_dialog.as_mut(),
            out,
        );
        if handled && horizon_client::touch_ui::output_dismisses(out) {
            crate::platform::hide();
        }
        handled
    }

    pub(super) fn android_poll_keyboard(&mut self) {
        self.android_custom_events();
        self.android_sync_pointer();
        if let Some(ed) = self.pad.touch.editor.as_mut()
            && std::mem::take(&mut ed.keyboard_request)
        {
            let text = if ed.editing_save_desc {
                ed.save_desc_input.clone()
            } else {
                ed.save_name_input.clone()
            };
            crate::platform::show(&text);
        }
        if let Some(notice) = crate::documents::poll() {
            if notice.changed {
                self.refresh_pack_lists();
                self.settings.shader_packs =
                    horizon_client::shader_pack_list::scan(&Self::shader_packs_dir());
            }
            self.system_message(horizon_client::text::Text::literal(notice.text));
        }
        for result in crate::gallery::poll() {
            self.system_message(horizon_client::text::Text::literal(
                result.unwrap_or_else(|e| format!("File export failed: {e}")),
            ));
        }
        for out in crate::platform::poll() {
            if self.pad.touch.editor.is_some() {
                let captured = match &out {
                    Output::Text(t) => self.pad.touch.capture_custom_text(t),
                    Output::Key(code) => self.pad.touch.capture_custom_key_code(*code),
                    Output::Backspace => self.pad.touch.capture_custom_key_code(KeyCode::Backspace),
                    Output::Enter => self.pad.touch.capture_custom_key_code(KeyCode::Enter),
                    Output::Close => false,
                };
                if captured {
                    self.settings.touch = self.pad.touch.settings.clone();
                    crate::platform::hide();
                    continue;
                }
            }
            if self.pad.touch_ui.custom_input_picker != horizon_client::touch::PickerStep::None {
                let captured = match &out {
                    Output::Text(t) => self.pad.touch_ui.capture_custom_input_text(t),
                    Output::Key(code) => self.pad.touch_ui.capture_custom_input_key(*code),
                    Output::Backspace => self
                        .pad
                        .touch_ui
                        .capture_custom_input_key(KeyCode::Backspace),
                    Output::Enter => self.pad.touch_ui.capture_custom_input_key(KeyCode::Enter),
                    Output::Close => None,
                };
                if let Some(token) = captured {
                    crate::platform::hide();
                    self.android_sync_pointer();
                    self.android_custom_token_tap(&token);
                    continue;
                }
            }
            if self.android_preset_typing(&out) {
                continue;
            }
            let next_sign_line = self.sign_edit.is_some() && matches!(out, Output::Enter);
            let done =
                matches!(out, Output::Close) || (matches!(out, Output::Enter) && !next_sign_line);
            self.pad_osk_output(out);
            if done {
                crate::platform::hide();
            } else if next_sign_line {
                self.android_show_keyboard();
            }
        }
        // Title screens also use Playing: their focused text fields must keep
        // the IME until the user confirms or closes the editor.
        if !self.pad_in_menu()
            && self.pad.touch.editor.is_none()
            && self.pad.touch_ui.custom_input_picker == horizon_client::touch::PickerStep::None
            && crate::platform::active()
        {
            crate::platform::hide();
        }
    }
    pub(super) fn android_performance_context(&self) -> serde_json::Value {
        let renderer = self.renderer.as_ref().map(|r| {
            let [arena, vertices] = r.section_vertex_bytes();
            let memory = r.device.generate_allocator_report();
            serde_json::json!({"gpu":r.info.name,"backend":format!("{:?}",r.info.backend),
                "surface":[r.config.width,r.config.height],"world_size":r.world_render_size(),
                "present":format!("{:?}",r.config.present_mode),
                "wait_ms":r.last_wait.as_secs_f32()*1000.0,
                "gpu_profiling_requested":crate::device_runtime::preferences().gpu_profile,
                "gpu_encoder_timestamps_supported":r.device.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
                "terrain_pass_draws": r.terrain_pass_draws,
                "gpu_stage_distributions":r.gpu_stage_distributions().into_iter().map(|(stage,p50,p95,p99,n)|serde_json::json!({"stage":stage,"p50_ms":p50,"p95_ms":p95,"p99_ms":p99,"samples":n})).collect::<Vec<_>>(),
                "gpu_stages":r.gpu_stage_times().into_iter().map(|(stage, median_ms, max_ms, samples)|
                    serde_json::json!({"stage":stage,"median_ms":median_ms,"max_ms":max_ms,"samples":samples})).collect::<Vec<_>>(),
                "draw_calls":r.section_draw_calls,"drawn_sections":r.drawn_sections,
                "resident_sections":r.resident_sections(),"uploaded_bytes_total":r.uploaded_bytes,
                "arena_bytes":arena,"section_vertex_bytes":vertices,
                "gpu_allocated_bytes":memory.as_ref().map(|m|m.total_allocated_bytes),
                "gpu_reserved_bytes":memory.as_ref().map(|m|m.total_reserved_bytes)})
        });
        serde_json::json!({"renderer":renderer,"android_native":crate::device_runtime::diagnostics(),"android_power":crate::platform::performance_power(),
            "menu_blur_radius":self.menu_blur(),"shader_pack_active":!self.settings.horizon.shader_pack.is_empty(),
            "render_distance":self.settings.render_distance,
            "simulation_distance":self.settings.simulation_distance,"fps_limit":self.framerate_limit(),
            "focused":self.focused,"paused":self.pause,"in_world":self.player.is_some(),
            "playing_screen":self.screen == Screen::Playing,
            "idle_seconds":self.last_input.elapsed().as_secs(),
            "camera_angles":[self.yaw, self.pitch],
            "player_velocity":self.player.as_ref().map(|p|[p.velocity.x,p.velocity.y,p.velocity.z]),
            "underwater":self.ambience.was_underwater(),"mobs":self.mobs.len(),
            "particles":self.particles.len(),"chunks":self.world.chunks.len(),
            "dirty_light":self.world.dirty_light.len(),"pending_uploads":self.pending_uploads.len(),
            "pending_mesh_bytes":self.pending_uploads.iter().map(|(_,m)|(m.solid.len()+m.translucent.len()) as u64*std::mem::size_of::<horizon_client::mesher::TerrainVertex>() as u64).sum::<u64>(),
            "remote_backlog":self.remote_backlog(),
            "streaming":self.streaming.stages().into_iter().collect::<std::collections::BTreeMap<_,_>>()})
    }
    pub(super) fn android_key_up(&mut self, key: KeyCode) {
        if self.pad.custom_keys.contains(&key) {
            return;
        }
        self.physical_keys.remove(&key);
        self.keys.remove(&key);
        self.debug_key(key, KeyAction::Release);
    }
    pub(super) fn android_release_touch_keys(&mut self, cancelled: bool) {
        self.pad.touch_ui.clear_modifiers();
        if cancelled && matches!(self.screen, Screen::Inventory | Screen::Creative) {
            self.inv_clicks = Default::default();
            self.inv_drag = None;
            self.creative_screen.release();
            self.loom_release();
        }
        self.pad.touch_ui.cancel_relative();
        for right in self.pad.touch_ui.release_all_mouse() {
            let button = if right {
                MouseButton::Right
            } else {
                MouseButton::Left
            };
            if let Err(error) = self.android_custom_mouse(button, false) {
                log::warn!("Release virtual mouse: {error}");
            }
        }

        if cancelled {
            self.pad.hardware_keys.clear();
            crate::platform::cancel_vibration();
            self.pad.touch.clear();
            self.pad.touch_aim = None;
        }
        self.android_custom_events();
        for key in self.pad.touch_ui.release_keys() {
            if cancelled && key == KeyCode::F3 {
                self.physical_keys.remove(&key);
                self.debug_keys.modifier_down = false;
                self.debug_keys.used_debug_key_as_modifier = false;
                if self.screen == Screen::GameModeSwitcher {
                    self.close_gamemode_switcher(false);
                }
            } else {
                self.android_key_up(key);
            }
        }
    }
    pub(super) fn android_menu_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) {
        self.android_sync_pointer();
        if self.pad.touch_ui.virtual_mouse {
            self.android_virtual_touch(t, events, id);
            return;
        }
        let Some(b) = self.gui_scale() else {
            return;
        };
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if t.phase == TouchPhase::Started && self.screen == Screen::Options {
            self.ui.options.touch_start();
        }
        let direct = self.pad.touch_ui.direct_pointer(t.id, p, t.phase);
        self.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: t.location,
            },
        );
        if direct {
            if t.phase != TouchPhase::Moved {
                self.handle_window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: t.device_id,
                        button: MouseButton::Left,
                        state: if t.phase == TouchPhase::Started {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                    },
                );
            }
            return;
        }
        match self
            .pad
            .touch_ui
            .gesture(t.id, [t.location.x as f32, t.location.y as f32], t.phase)
        {
            Gesture::Tap => {
                let field = crate::platform::field_at(p);
                for state in [ElementState::Pressed, ElementState::Released] {
                    self.handle_window_event(
                        events,
                        id,
                        WindowEvent::MouseInput {
                            device_id: t.device_id,
                            button: MouseButton::Left,
                            state,
                        },
                    );
                }
                if field || self.sign_edit.is_some() {
                    self.android_show_keyboard();
                }
            }
            Gesture::Scroll(y) if self.screen == Screen::Options => {
                self.ui
                    .options
                    .touch_scroll(&self.settings, (b.width, b.height), (p[0], p[1]), y);
            }
            Gesture::Scroll(y) => self.handle_window_event(
                events,
                id,
                WindowEvent::MouseWheel {
                    device_id: t.device_id,
                    delta: MouseScrollDelta::LineDelta(0.0, y),
                    phase: TouchPhase::Moved,
                },
            ),
            Gesture::None => {}
        }
    }
    pub(super) fn android_virtual_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) {
        let Some(b) = self.gui_scale() else {
            return;
        };
        let position = self.pad.touch_ui.relative_touch(
            t.id,
            [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale],
            t.phase,
            [self.cursor.0, self.cursor.1],
            [b.width, b.height],
            self.pad.touch.settings.virtual_mouse_sensitivity,
            !self.pad.touch_ui.mouse_held(),
        );
        self.cursor = (position[0], position[1]);
        self.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: winit::dpi::PhysicalPosition::new(
                    f64::from(position[0] * b.scale),
                    f64::from(position[1] * b.scale),
                ),
            },
        );
        if t.phase == TouchPhase::Ended {
            self.android_virtual_mouse_clicks(events);
        }
    }
    pub(super) fn android_virtual_mouse_clicks(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
    ) {
        if !self.focused || !self.pad.touch_ui.virtual_mouse {
            self.pad.touch_ui.cancel_relative();
            for right in self.pad.touch_ui.release_all_mouse() {
                if let Some(id) = self.window.as_ref().map(|w| w.id())
                    && let Err(error) = self.android_menu_mouse(events, id, right, false)
                {
                    log::warn!("Release virtual mouse: {error}");
                }
            }
            return;
        }
        if !self.pad_in_menu() {
            return;
        }
        let Some(right) = self.pad.touch_ui.relative_click(true) else {
            return;
        };
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        if matches!(self.screen, Screen::Inventory | Screen::Creative) {
            if let Err(e) = self
                .android_custom_mouse(button, true)
                .and_then(|()| self.android_custom_mouse(button, false))
            {
                log::warn!("Virtual mouse click: {e}");
            }
        } else if let Some(id) = self.window.as_ref().map(|w| w.id()) {
            for state in [ElementState::Pressed, ElementState::Released] {
                self.handle_window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state,
                        button,
                    },
                );
            }
        }
    }
}
