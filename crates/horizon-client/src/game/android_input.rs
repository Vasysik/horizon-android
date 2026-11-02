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
        events: &ActiveEventLoop,
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
        events: &ActiveEventLoop,
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
        let utility = self
            .pad
            .touch_ui
            .action(batch.width, batch.height, point, self.pad_in_menu())
            .is_some()
            || self.pad.touch_ui.owns_key(POINTER)
            || self.pad.touch.owns_pointer(POINTER)
            || (!self.pad_in_menu() && self.pad.touch.auxiliary_at(normalized));
        if !utility {
            return false;
        }
        let touch = Touch {
            device_id: *device_id,
            phase: if *state == ElementState::Pressed {
                TouchPhase::Started
            } else {
                TouchPhase::Ended
            },
            location: winit::dpi::PhysicalPosition::new(
                f64::from(point[0] * batch.scale),
                f64::from(point[1] * batch.scale),
            ),
            force: None,
            id: POINTER,
        };
        self.window_event(events, id, WindowEvent::Touch(touch));
        true
    }

    pub(super) fn android_custom_key_held(&self, key: KeyCode) -> bool {
        self.pad.touch.held_custom_tokens().any(|token| {
            if let Some(mapping) = token.strip_prefix("action:") {
                horizon_client::keybinds::key_code(horizon_client::keybinds::KeyBinds::default_of(
                    mapping,
                )) == Some(key)
            } else {
                horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                    self.settings
                        .keys
                        .translate_all(physical)
                        .contains(&Ok(key))
                })
            }
        })
    }
    pub(super) fn android_custom_mouse_held(&self, button: MouseButton) -> bool {
        self.pad.touch.held_custom_tokens().any(|token| {
            token.strip_prefix("action:").is_some_and(|mapping| {
                horizon_client::keybinds::mouse_button(
                    horizon_client::keybinds::KeyBinds::default_of(mapping),
                ) == Some(button)
            })
        })
    }
    pub(super) fn android_custom_events(&mut self) {
        for (name, down) in self.pad.touch.key_events() {
            if let Some(mapping) = name.strip_prefix("action:") {
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
                continue;
            }
            let Some(key) = horizon_client::keybinds::key_code(&name) else {
                continue;
            };
            if down {
                self.pad.custom_keys.insert(key);
            } else {
                self.pad.custom_keys.remove(&key);
            }
            if self.pad.hardware_keys.contains(&key) {
                continue;
            }
            if (!self.pad_in_menu() || !down)
                && horizon_client::android_runtime::keypad_key(key, down)
            {
                continue;
            }
            if down {
                self.physical_keys.insert(key);
            } else {
                self.physical_keys.remove(&key);
            }
            self.mod_key(key, down, false);
            self.native_key_event(key, down, false);
            let action = if down {
                KeyAction::Press
            } else {
                KeyAction::Release
            };
            if self.debug_key(key, action) {
                continue;
            }
            for mapped in self.settings.keys.translate_all(key) {
                let result = match mapped {
                    Ok(code) => self.mapped_key(code, down, false),
                    Err(button) => self.android_custom_mouse(button, down),
                };
                if let Err(error) = result {
                    log::warn!("Custom touch key: {error}");
                }
            }
            if down && matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat
            {
                self.android_show_keyboard();
            }
        }
    }

    pub(super) fn dispatch_layout_output_events(
        &mut self,
        events: impl IntoIterator<Item = horizon_client_input::control_layout::LayoutOutputEvent>,
    ) {
        use horizon_client_input::control_layout::LayoutOutputEvent;
        for event in events {
            match event {
                LayoutOutputEvent::KeyDown(key) => {
                    self.pad.custom_keys.insert(key);
                    self.physical_keys.insert(key);
                    self.mod_key(key, true, false);
                    self.native_key_event(key, true, false);
                    if !self.debug_key(key, KeyAction::Press) {
                        for mapped in self.settings.keys.translate_all(key) {
                            let result = match mapped {
                                Ok(code) => self.mapped_key(code, true, false),
                                Err(button) => self.android_custom_mouse(button, true),
                            };
                            if let Err(error) = result {
                                log::warn!("Custom control key down: {error}");
                            }
                        }
                    }
                    if matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat {
                        self.android_show_keyboard();
                    }
                }
                LayoutOutputEvent::KeyUp(key) => {
                    self.pad.custom_keys.remove(&key);
                    self.physical_keys.remove(&key);
                    self.keys.remove(&key);
                    self.mod_key(key, false, false);
                    self.native_key_event(key, false, false);
                    if !self.debug_key(key, KeyAction::Release) {
                        for mapped in self.settings.keys.translate_all(key) {
                            let result = match mapped {
                                Ok(code) => self.mapped_key(code, false, false),
                                Err(button) => self.android_custom_mouse(button, false),
                            };
                            if let Err(error) = result {
                                log::warn!("Custom control key up: {error}");
                            }
                        }
                    }
                }
                LayoutOutputEvent::MouseButton(down, is_left) => {
                    let button = if is_left {
                        MouseButton::Left
                    } else {
                        MouseButton::Right
                    };
                    if let Err(error) = self.android_custom_mouse(button, down) {
                        log::warn!("Custom control mouse button: {error}");
                    }
                }
                LayoutOutputEvent::Scroll(delta) => {
                    if self.screen == Screen::Playing && self.player.is_some() {
                        let step = if delta > 0.0 { -1 } else { 1 };
                        self.selected = (self.selected as i32 + step).rem_euclid(9) as usize;
                    }
                }
                LayoutOutputEvent::Launcher(name) => {
                    if name == "KEYBOARD" {
                        self.android_show_keyboard();
                    } else if name == "CHAT" {
                        self.open_chat("");
                    }
                }
                LayoutOutputEvent::SendText(text) => {
                    if self.screen == Screen::Chat {
                        if let Some(cs) = self.ui.chat_screen.as_mut() {
                            cs.insert(&text);
                        }
                    }
                }
            }
        }
    }

    fn android_custom_mouse(&mut self, button: MouseButton, down: bool) -> Result<()> {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            return self.play_mouse(button, down);
        }
        let right = button == MouseButton::Right;
        if !matches!(button, MouseButton::Left | MouseButton::Right) {
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
            self.loom_release();
        }
        Ok(())
    }

    fn android_menu_mouse(
        &mut self,
        events: &ActiveEventLoop,
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
            self.window_event(
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
        events: &ActiveEventLoop,
        id: WindowId,
    ) -> Result<bool> {
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
        let size = self.window.as_ref().unwrap().inner_size();
        let b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if let Some(action) = self
            .pad
            .touch_ui
            .action(b.width, b.height, p, self.pad_in_menu())
        {
            if t.phase == TouchPhase::Started {
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
                        self.pad.touch_ui.virtual_mouse ^= true;
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
                            let down = !self.physical_keys.remove(&key);
                            if down {
                                self.physical_keys.insert(key);
                            }
                            self.mapped_key(key, down, false)?;
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
                    Action::OpenControlDialog => {
                        let data_dir = crate::backend::data_dir();
                        let active = self.settings.touch.active_control_layout.as_str();
                        let storage = horizon_client_input::control_layout::LayoutStorage::new(
                            data_dir.join("control_layouts"),
                            active,
                        );
                        let _ = storage.ensure_default_layout();
                        let layouts = storage.list_layouts("en");
                        self.pad.touch_ui.replacement_dialog =
                            Some(horizon_client::touch_ui::ReplacementControlDialog::new(layouts));
                    }
                    Action::SelectControlLayout(slug) => {
                        let data_dir = crate::backend::data_dir();
                        let storage = horizon_client_input::control_layout::LayoutStorage::new(
                            data_dir.join("control_layouts"),
                            &slug,
                        );
                        if let Ok(layout) = storage.load_layout(&slug) {
                            if let Some(runtime) = self.pad.touch_ui.control_runtime.as_mut() {
                                let releases = runtime.set_layout(layout);
                                self.dispatch_layout_output_events(releases);
                            }
                            self.settings.touch.active_control_layout = slug.clone();
                            let _ = self.save_options();
                        }
                        if let Some(dialog) = self.pad.touch_ui.replacement_dialog.as_mut() {
                            for (i, l) in dialog.layouts.iter_mut().enumerate() {
                                l.is_active = l.file_stem == slug;
                                if l.is_active {
                                    dialog.selected_index = i;
                                }
                            }
                        }
                    }
                    Action::CloseControlDialog => {
                        self.pad.touch_ui.replacement_dialog = None;
                    }
                    Action::OpenControlEditor => {
                        let data_dir = crate::backend::data_dir();
                        let active = self.settings.touch.active_control_layout.as_str();
                        if let Some(runtime) = self.pad.touch_ui.control_runtime.as_mut() {
                            let releases = runtime.release_all_inputs();
                            self.dispatch_layout_output_events(releases);
                        }
                        self.pad.touch_ui.open_editor(&data_dir, active);
                    }
                    Action::CloseControlEditor => {
                        self.pad.touch_ui.close_editor();
                    }
                    Action::SaveControlEditor => {
                        if let Some(editor) = self.pad.touch_ui.editor.as_mut() {
                            let data_dir = crate::backend::data_dir();
                            let storage = horizon_client_input::control_layout::LayoutStorage::new(
                                data_dir.join("control_layouts"),
                                &editor.original_slug,
                            );
                            if let Err(e) = editor.save(&storage) {
                                log::error!("Failed to save control layout: {e}");
                            } else {
                                log::info!("Control layout saved successfully");
                                if let Some(runtime) = self.pad.touch_ui.control_runtime.as_mut() {
                                    let releases = runtime.set_layout(editor.layout.clone());
                                    self.dispatch_layout_output_events(releases);
                                }
                            }
                        }
                    }
                    Action::EditorAddButton => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.add_normal_button("NEW");
                        }
                    }
                    Action::EditorAddJoystick => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.add_joystick();
                        }
                    }
                    Action::EditorDuplicate => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.duplicate_selected();
                        }
                    }
                    Action::EditorDelete => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.delete_selected();
                        }
                    }
                    Action::EditorToggleLeftDrawer => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.left_drawer_open = !e.left_drawer_open;
                        }
                    }
                    Action::EditorToggleRightDrawer => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.right_drawer_open = !e.right_drawer_open;
                        }
                    }
                    Action::EditorTogglePreview => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.preview_mode = !e.preview_mode;
                        }
                    }
                    Action::EditorToggleFocusMode => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.layer_focus_mode = !e.layer_focus_mode;
                        }
                    }
                    Action::EditorAddLayer => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.add_layer("New Layer");
                        }
                    }
                    Action::EditorSelectLayer(idx) => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.active_layer_index = idx;
                        }
                    }
                    Action::EditorToggleLayerHide(idx) => {
                        if let Some(e) = self.pad.touch_ui.editor.as_mut() {
                            e.toggle_layer_visibility(idx);
                        }
                    }
                }
            }
            return Ok(true);
        }

        if let Some(editor) = self.pad.touch_ui.editor.as_mut() {
            let px = p[0];
            let py = p[1];
            match t.phase {
                TouchPhase::Started => {
                    if !editor.start_drag(t.id, (px, py)) {
                        editor.select_at((px, py));
                    }
                }
                TouchPhase::Moved => {
                    editor.update_drag(t.id, (px, py));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    editor.end_drag(t.id);
                }
            }
            return Ok(true);
        }

        if self.pad.touch_ui.replacement_dialog.is_none() {
            if let Some(runtime) = self.pad.touch_ui.control_runtime.as_mut() {
                runtime.set_screen_dimensions(b.width, b.height, b.scale);
                let menu_releases = runtime.set_in_menu(self.pad_in_menu());
                if !menu_releases.is_empty() {
                    self.dispatch_layout_output_events(menu_releases);
                }
                let was_owned = runtime.owns_pointer(t.id);
                let out_events = runtime.handle_touch(t.id, t.phase, (p[0], p[1]));
                let is_owned = runtime.owns_pointer(t.id);
                let has_events = !out_events.is_empty();

                if has_events {
                    self.dispatch_layout_output_events(out_events);
                }

                if was_owned || is_owned {
                    if !runtime.is_pointer_penetrable(t.id) {
                        self.pad.touch.ignore(t.id);
                        return Ok(true);
                    }
                }
            }
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
    pub(super) fn android_poll_keyboard(&mut self) {
        self.android_custom_events();
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
        if !self.pad_in_menu() && crate::platform::active() {
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
            if self.pad.touch.editor.is_some() {
                self.pad.touch.cancel_editor();
                self.pause = false;
            }
        }
        self.android_custom_events();
        if let Some(runtime) = self.pad.touch_ui.control_runtime.as_mut() {
            let releases = runtime.release_all_inputs();
            self.dispatch_layout_output_events(releases);
        }
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
    pub(super) fn android_menu_touch(&mut self, t: &Touch, events: &ActiveEventLoop, id: WindowId) {
        if self.pad.touch_ui.virtual_mouse {
            self.android_virtual_touch(t, events, id);
            return;
        }
        let Some(b) = self.gui_scale() else {
            return;
        };
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        let direct = self.pad.touch_ui.direct_pointer(t.id, p, t.phase);
        self.window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: t.location,
            },
        );
        if direct {
            if t.phase != TouchPhase::Moved {
                self.window_event(
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
                    self.window_event(
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
            Gesture::Scroll(y) => self.window_event(
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
        events: &ActiveEventLoop,
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
            self.settings.touch.virtual_mouse_sensitivity,
            !self.pad.touch_ui.mouse_held(),
        );
        self.window_event(
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
    }
    pub(super) fn android_virtual_mouse_clicks(&mut self, events: &ActiveEventLoop) {
        if !self.focused || !self.pad_in_menu() || !self.pad.touch_ui.virtual_mouse {
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
                self.window_event(
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
