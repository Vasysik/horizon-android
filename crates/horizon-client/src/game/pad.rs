//! Controllers through gilrs (Horizon's own; docs/controllers.md): the
//! active pad's state each loop iteration, its button edges dispatched as
//! the vanilla key mappings they are bound to, the left stick as an analog
//! move vector and the right stick turning the view. `horizon-ctl pad`
//! injects a virtual pad on top of the real one.
use super::event_loop::EventLoopCtl;
use super::{Game, Screen};
use horizon_client::{
    Result,
    gamepad::{self, Button, Edges, GlyphStyle, HealthWatch, PadState, Rumble},
    keybinds::{KeyBinds, key_code, mouse_button},
    keymapping::SNEAK,
};
use serde_json::{Value, json};
use std::time::Instant;
use winit::keyboard::KeyCode;

pub(super) const HELP: &[&str] = &[
    "pad BUTTON down|up|tap         a virtual controller button (south east west north lb rb lt rt select start ls rs up down left right)",
    "pad stick left|right X Y       tilt a virtual stick (-1..1, up positive); 0 0 centres it",
    "pad trigger left|right VALUE   pull a virtual trigger (0..1)",
    "pad release                    centre and release the virtual controller",
    "pad status                     connected controllers, the active one and its state",
];

/// The controllers and the state the game last acted on.
#[derive(Default)]
pub(super) struct Pads {
    #[cfg(target_os = "android")]
    pub(super) custom_keys: std::collections::HashSet<KeyCode>,
    #[cfg(target_os = "android")]
    pub(super) hardware_keys: std::collections::HashSet<KeyCode>,
    #[cfg(target_os = "android")]
    last_input_mode: Option<horizon_client::android_gamepad::InputMode>,
    #[cfg(target_os = "android")]
    pub(super) touch_shield: bool,
    #[cfg(target_os = "android")]
    pub(super) touch: horizon_client::touch::TouchControls,
    /// Keep a released tap's ray until the tick consumed its queued click.
    #[cfg(target_os = "android")]
    pub(super) touch_aim: Option<[f32; 2]>,
    #[cfg(target_os = "android")]
    pub(super) touch_ui: horizon_client::touch_ui::TouchUi,
    /// Splitscreen: this player's pad, read by `PadRouter` for every game, instead of the pads.
    pub(super) feed: Option<PadState>,
    /// The real controllers and the DualSense's output.
    #[cfg(all(feature = "controllers", not(target_os = "android")))]
    devices: hardware::Devices,
    /// The virtual pad of `horizon-ctl pad`.
    injected: PadState,
    /// The virtual pad was in use at the last poll.
    virtual_input: bool,
    /// Buttons a `pad ... tap` releases at the next poll.
    taps: Vec<Button>,
    edges: Edges,
    /// The game tick each button went down in, and releases held back until
    /// a tick has passed: vanilla samples keys once per tick, so a click
    /// shorter than that (a quick stick click for sprint) would be lost.
    pressed_at: [Option<u64>; 16],
    late_releases: Vec<Button>,
    /// The merged state of the last poll.
    pub(super) state: PadState,
    /// The pad drove the game last (the mouse and keyboard hand control back).
    pub(super) in_use: bool,
    pub(super) style: GlyphStyle,
    /// The cursor the pad drives in menus.
    pub(super) menu: super::pad_menu::MenuCursor,
    /// The last vibration asked for, for `pad status`.
    last_rumble: Option<(&'static str, gamepad::Pulse)>,
    /// The player's health between polls (a drop is damage taken).
    health: HealthWatch,
}

#[cfg(all(feature = "controllers", not(target_os = "android")))]
mod hardware;
pub(super) use hardware::{PadId, PadRouter};

/// What a controller did since the last frame.
#[cfg_attr(not(feature = "controllers"), allow(dead_code))]
pub(in crate::game) enum PadEvent {
    Press(PadId, Button),
    /// Disconnected (out of battery, unplugged, out of range).
    Gone(PadId),
    /// Connected again under the same id (Windows keeps it; macOS gives a new one). On Linux
    /// gilrs gives a new device the slot of a disconnected one of the same model, so there this
    /// can be another controller of that model.
    Back(PadId),
}

/// A build without the `controllers` feature (the server build, which must not need libudev): no
/// hardware pads, only `horizon-ctl pad`'s virtual one.
#[cfg(any(not(feature = "controllers"), target_os = "android"))]
mod hardware {
    use super::{super::Game, Pads};
    use horizon_client::gamepad::{PadState, Pulse, Rumble};
    use serde_json::{Value, json};

    impl Pads {
        pub(super) fn read(&mut self) -> PadState {
            #[cfg(target_os = "android")]
            return horizon_client::android_runtime::keypad_state()
                .merge(&horizon_client::android_runtime::gamepad_state());
            #[cfg(not(target_os = "android"))]
            PadState::default()
        }
        pub(in crate::game) fn release_hardware(&mut self) {}
    }

    pub(in crate::game) type PadId = usize;
    #[derive(Default)]
    pub(in crate::game) struct PadRouter {}
    impl PadRouter {
        pub(in crate::game) fn events(&mut self) -> Vec<super::PadEvent> {
            Vec::new()
        }
        pub(in crate::game) fn lend_output(&mut self, _: &mut Pads, _: PadId) {}
        pub(in crate::game) fn reclaim_output(&mut self, _: &mut Pads) {}
        pub(in crate::game) fn connected(&self, _: PadId) -> bool {
            false
        }
        pub(in crate::game) fn uuid(&self, _: PadId) -> [u8; 16] {
            [0; 16]
        }
        pub(in crate::game) fn state(&self, _: PadId) -> PadState {
            PadState::default()
        }
        pub(in crate::game) fn name(&self, _: PadId) -> String {
            String::new()
        }
        pub(in crate::game) fn style(&self, _: PadId) -> horizon_client::gamepad::GlyphStyle {
            horizon_client::gamepad::GlyphStyle::Auto
        }
    }

    impl Game<'_> {
        pub(super) fn pad_output_off(&mut self) {
            #[cfg(target_os = "android")]
            crate::platform::cancel_vibration();
        }
        pub(super) fn pad_output_frame(&mut self) {
            #[cfg(target_os = "android")]
            if !self.settings.controller.vibration {
                crate::platform::cancel_vibration();
            }
        }
        pub(in crate::game) fn pad_close_output(&mut self) {
            self.pad_output_off();
        }
        pub(super) fn pad_vibrate(&mut self, _: Rumble, pulse: Pulse) {
            #[cfg(target_os = "android")]
            crate::platform::vibrate(pulse);
            #[cfg(not(target_os = "android"))]
            let _ = pulse;
        }
        pub(super) fn pad_devices_status(&self) -> Value {
            json!({ "backend": cfg!(target_os = "android"), "controllers": [], "android_native_input": cfg!(target_os = "android") })
        }
    }
}

impl Pads {
    /// Include releases delayed until the next simulation tick.
    pub(super) fn holds_action(&self, settings: &gamepad::PadSettings, action: &str) -> bool {
        settings.binding(action).is_some_and(|button| {
            self.edges.is_held(button) || self.late_releases.contains(&button)
        })
    }

    /// A new player entity took over (spawn, respawn, a server joined, a
    /// dimension change): its health is not compared with the old one's.
    pub(super) fn player_replaced(&mut self) {
        self.health.forget();
    }
}

impl Game<'_> {
    #[cfg(target_os = "android")]
    pub(super) fn touch_shield_available(&self) -> bool {
        self.held()
            .into_iter()
            .chain(self.offhand.as_ref())
            .any(|s| self.blocks_attacks(s).is_some())
    }
    #[cfg(target_os = "android")]
    pub(super) fn android_inventory_controls(&self) -> bool {
        self.settings.touch.inventory_controls
            && matches!(self.screen, Screen::Inventory | Screen::Creative)
    }
    /// Once per loop iteration: read the pads and act on button changes.
    pub(super) fn poll_pads(&mut self, event_loop: &dyn EventLoopCtl) {
        #[cfg(target_os = "android")]
        if !matches!(self.screen, Screen::Playing | Screen::Options)
            && self.pad.touch.editor.is_some()
        {
            self.pad.touch.cancel_editor();
        }
        #[cfg(target_os = "android")]
        {
            crate::performance::configure_gpu_profile(
                crate::device_runtime::preferences().gpu_profile,
                &crate::backend::game_home(),
            );
            self.pad.touch.inventory_mode = self.android_inventory_controls();
            if !self.pad.touch.inventory_mode {
                self.pad.touch.inventory_shift = false;
            }
            self.android_virtual_mouse_clicks(event_loop);
            let active = horizon_client::android_runtime::active_input();
            if self.pad.last_input_mode.replace(active).is_some_and(|old| {
                old == horizon_client::android_gamepad::InputMode::KeyboardMouse && old != active
            }) {
                for key in std::mem::take(&mut self.pad.hardware_keys) {
                    self.android_key_up(key);
                }
                horizon_client::android_runtime::clear_keypad();
            }
            if (self.pad_in_menu() && !self.android_inventory_controls())
                || !self.focused
                || (!crate::touch_visible() && !self.settings.keypad.enabled)
            {
                self.pad.touch.clear();
                horizon_client::android_runtime::clear_keypad();
            }
            self.android_custom_events();
        }
        #[cfg(target_os = "android")]
        if self.pad_in_menu() && self.android_inventory_controls() {
            self.pad.touch.clear_world_gesture();
        }
        let settings = &self.settings.controller;
        if !settings.enabled && !cfg!(target_os = "android") {
            self.pad_output_off();
            if self.pad.state != PadState::default() {
                self.pad.state = PadState::default();
                for (button, down) in self.pad.edges.release_all() {
                    self.pad_edge(event_loop, button, down);
                }
            }
            // The keyboard's menu focus follows its widget all the same.
            self.pad_menu_frame(event_loop);
            return;
        }
        #[cfg(target_os = "android")]
        {
            crate::platform::poll_gamepad_connections();
            if !self.settings.controller.enabled || !self.focused {
                horizon_client::android_runtime::clear_gamepads();
            }
            self.pad.touch.force_crosshair =
                !crate::touch_visible() || self.settings.keypad.enabled;
        }
        let hardware = match self.pad.feed {
            Some(state) => state,
            None => self.pad.read(),
        };
        self.ui.options.pad_style = self.settings.controller.glyphs.resolve(self.pad.style);
        let state = hardware.merge(&self.pad.injected);
        #[cfg(target_os = "android")]
        let state = {
            self.pad.touch.shield_available = self.touch_shield_available();
            // #71: a crouch toggled on land would keep sinking, descending or
            // dismounting once the player swims, flies or rides; there the
            // button holds, so the toggle ends.
            if crate::touch_visible()
                && self.android_touch_descending()
                && self.toggles.drop_pad_toggle(SNEAK)
            {
                self.sync_toggle(SNEAK);
            }
            let mut touch = self.pad.touch.poll_state();
            self.pad.touch_shield = crate::touch_visible()
                && !self.pad_in_menu()
                && self.touch_shield_available()
                && if self.settings.touch.shield_button {
                    self.pad.touch.shield_held()
                } else {
                    self.pad.touch.sneaking || touch.held(Button::RightStick)
                };
            if self.pad.touch_shield {
                touch.triggers[0] = 1.0;
            }
            if let Some(point) = self.pad.touch.aim {
                self.pad.touch_aim = Some(point);
            }
            if self.pad.touch.aim.is_some()
                && self.pad.touch.holding
                && (self.pad.touch.holding_use
                    || self.touch_item_held_use()
                    || self.touch_entity_accepts_use())
            {
                touch.triggers = [1.0, 0.0];
                self.pad.touch.latch_use();
            }
            if let Some(held_use) = self.pad.touch.take_tap() {
                if self.entity_targeted() && self.using.is_none() && self.screen == Screen::Playing
                {
                    // Resolve the entity attack now, never as held LMB. A mob
                    // disappearing before the next tick cannot expose a block.
                    if self.remote.is_some() {
                        self.remote_start_attack();
                    } else {
                        self.attack();
                    }
                } else if !held_use {
                    // Preserve normal block placement and empty-hand interaction.
                    // Items with a use action wait for the long-press instead.
                    touch.triggers[0] = 1.0;
                }
            }
            state.merge(&touch)
        };
        self.pad.virtual_input = self.pad.injected != PadState::default();
        for button in std::mem::take(&mut self.pad.taps) {
            self.pad.injected.set(button, false);
        }
        let threshold = self.settings.controller.trigger_threshold;
        let edges = self.pad.edges.update(&state, threshold);
        if state.active(&self.settings.controller) {
            self.last_input = Instant::now();
            self.pad.in_use = true;
        }
        self.pad.state = state;
        self.pad_output_frame();
        // Damage from any source, here or on a server; nothing counts on
        // the title screen.
        let health = self
            .player
            .as_ref()
            .filter(|_| self.title.is_none())
            .map(|p| p.health.health());
        if let Some(damage) = self.pad.health.observe(health) {
            self.pad_rumble(Rumble::Hurt(damage));
        }
        let ticking = self.title.is_none() && self.screen == Screen::Playing && !self.pause;
        for button in std::mem::take(&mut self.pad.late_releases) {
            if !ticking || self.pad.pressed_at[button as usize] != Some(self.ticks) {
                self.pad_button_logged(event_loop, button, false);
            } else {
                self.pad.late_releases.push(button);
            }
        }
        for (button, down) in edges {
            self.pad_edge(event_loop, button, down);
        }
        self.pad_menu_frame(event_loop);
        #[cfg(target_os = "android")]
        if state.triggers == [0.0; 2] && self.pad.late_releases.is_empty() {
            self.pad.touch_aim = None;
        }
    }

    fn pad_edge(&mut self, event_loop: &dyn EventLoopCtl, button: Button, down: bool) {
        let ticking = self.title.is_none() && self.screen == Screen::Playing && !self.pause;
        if down {
            // Pressed again before its held-back release went out.
            if let Some(i) = self.pad.late_releases.iter().position(|b| *b == button) {
                self.pad.late_releases.remove(i);
                self.pad_button_logged(event_loop, button, false);
            }
            self.pad.pressed_at[button as usize] = Some(self.ticks);
        } else if ticking && self.pad.pressed_at[button as usize] == Some(self.ticks) {
            self.pad.late_releases.push(button);
            return;
        }
        self.pad_button_logged(event_loop, button, down);
    }

    fn pad_button_logged(&mut self, event_loop: &dyn EventLoopCtl, button: Button, down: bool) {
        if let Err(e) = self.pad_button(event_loop, button, down) {
            log::warn!("Horizon: controller {}: {e}", button.name());
        }
    }

    /// Input from the pad counts while the window has focus, or always for
    /// the virtual pad (a script can't focus the window).
    fn pad_focused(&self) -> bool {
        self.focused || self.pad.virtual_input
    }

    fn pad_button(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        button: Button,
        down: bool,
    ) -> Result<()> {
        if down && !self.pad_focused() {
            return Ok(());
        }
        let playing = !self.pad_in_menu();
        if down && playing && !self.captured && self.focused {
            // Like a click into the window: take the pointer.
            self.capture(true);
        }
        if !playing {
            self.pad_menu_button(event_loop, button, down)?;
            if down {
                return Ok(());
            }
        } else if button == Button::Start {
            #[cfg(target_os = "android")]
            if down {
                self.audio.play_button_click();
            }
            return if down { self.pad_back() } else { Ok(()) };
        }
        #[cfg(target_os = "android")]
        if down && playing && button == Button::North {
            self.audio.play_button_click();
        }
        // Releases always reach the mappings, so nothing stays held when a
        // screen opened in between.
        for action in self.settings.controller.actions_of(button) {
            self.pad_action(action, down)?;
        }
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub(super) fn touch_entity_accepts_use(&self) -> bool {
        if [
            self.boat_hit().map(|h| h.1),
            self.cart_hit().map(|h| h.1),
            self.armor_stand_hit().map(|h| h.1),
            self.frame_hit().map(|h| h.1),
        ]
        .into_iter()
        .flatten()
        .any(|distance| self.before_block(distance))
        {
            return true;
        }
        self.mob_hit()
            .filter(|(_, distance)| self.before_block(*distance))
            .is_some_and(|(i, _)| {
                self.remote_mob_takes(i, horizon_client::first_person::Hand::Main)
                    || self.remote_mob_takes(i, horizon_client::first_person::Hand::Off)
            })
    }

    #[cfg(target_os = "android")]
    pub(super) fn touch_context_use(&self) -> bool {
        if self.touch_entity_accepts_use()
            || [
                self.boat_hit().map(|h| h.1),
                self.cart_hit().map(|h| h.1),
                self.armor_stand_hit().map(|h| h.1),
                self.frame_hit().map(|h| h.1),
            ]
            .into_iter()
            .flatten()
            .any(|d| self.before_block(d))
        {
            return true;
        }
        let Some(hit) = self.pick() else {
            return false;
        };
        let state = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        let b = self.res.behaviours();
        let data = b.data(state);
        let p = b.props;
        let table = &b.table;
        use horizon_world::level::classes::Class;
        data.chain.iter().any(|c| match c {
            Class::Door | Class::TrapDoor => {
                data.set_type.as_ref().is_some_and(|s| s.can_open_by_hand)
            }
            Class::Cake | Class::CandleCake => {
                self.creative
                    || self
                        .player
                        .as_ref()
                        .is_some_and(|p| p.health.food.food < 20)
            }
            Class::Composter => table.int(state, p.level) == 8,
            Class::CaveVines | Class::CaveVinesPlant => table.bool(state, p.berries),
            Class::SweetBerryBush => table.int(state, p.age) > 1,
            Class::Repeater | Class::Comparator | Class::DaylightDetector | Class::RedstoneWire => {
                self.mode != horizon_client::command::args::GameMode::Adventure
            }
            Class::FenceGate
            | Class::Button
            | Class::Lever
            | Class::Note
            | Class::Bell
            | Class::FlowerPot
            | Class::DragonEgg
            | Class::AbstractBed
            | Class::RespawnAnchor
            | Class::AbstractFurnace
            | Class::Anvil
            | Class::Barrel
            | Class::Beacon
            | Class::BrewingStand
            | Class::CartographyTable
            | Class::Chest
            | Class::Crafter
            | Class::CraftingTable
            | Class::Dispenser
            | Class::EnchantingTable
            | Class::EnderChest
            | Class::Grindstone
            | Class::Hopper
            | Class::Loom
            | Class::ShulkerBox
            | Class::Sign
            | Class::SmithingTable
            | Class::Stonecutter
            | Class::Lectern => true,
            _ => false,
        })
    }

    /// Touch fly/swim down and dismount are held even when ground sneaking
    /// is toggled. Any water counts, not only the sprint-swim pose: a toggled
    /// crouch there sinks the player (#71).
    fn android_touch_descending(&self) -> bool {
        #[cfg(target_os = "android")]
        {
            self.pad.touch.vertical_movement
                || self.pad.touch.swimming
                || self.pad.touch.mounted
                || self
                    .player
                    .as_ref()
                    .is_some_and(|p| p.fluids.was_touching_water)
        }
        #[cfg(not(target_os = "android"))]
        {
            false
        }
    }

    /// A bound action: a vanilla mapping acts as its default key or mouse
    /// button, like a rebound keyboard key.
    fn pad_action(&mut self, action: &str, down: bool) -> Result<()> {
        match action {
            gamepad::HOTBAR_PREVIOUS | gamepad::HOTBAR_NEXT => {
                if down && self.screen == Screen::Playing {
                    self.selected =
                        gamepad::hotbar_step(self.selected, action == gamepad::HOTBAR_NEXT);
                }
                Ok(())
            }
            #[cfg(target_os = "android")]
            "key.sprint" => {
                self.mapped_key(KeyCode::ControlLeft, down, false)?;
                if !down
                    && crate::touch_visible()
                    && self.settings.touch.sprint_toggle
                    && let Some(player) = &mut self.player
                {
                    player.sprinting = false;
                }
                Ok(())
            }
            "key.sneak"
                if self.settings.controller.toggle_sneak
                    && !self.settings.toggle_crouch
                    && !self.android_touch_descending() =>
            {
                // A stick click flips sneaking instead of holding it, as
                // vanilla's toggle crouch does: a screen opening releases it
                // and closing restores it (ToggleKeyMapping).
                if !down {
                    return Ok(());
                }
                let screen_open = self.screen != Screen::Playing || self.title.is_some();
                self.toggles.pad_toggle(SNEAK, screen_open);
                self.sync_toggle(SNEAK);
                self.key_pressed(KeyCode::ShiftLeft)
            }
            _ => {
                let default = KeyBinds::default_of(action);
                if let Some(button) = mouse_button(default) {
                    // A menu may open between press and release. Always deliver
                    // the release so mining/use cannot remain held behind Options.
                    if !down || (self.title.is_none() && self.screen == Screen::Playing) {
                        self.play_mouse(button, down)?;
                    }
                    Ok(())
                } else if let Some(key) = key_code(default) {
                    self.mapped_key(key, down, false)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// B and Start: what Escape does where the pad is.
    pub(super) fn pad_back(&mut self) -> Result<()> {
        // The options open over the title screen too: they come first, as
        // for the keyboard's Escape.
        if self.screen == Screen::Options {
            return self.options_key(KeyCode::Escape, None);
        }
        if let Some(title) = self.title.as_mut() {
            title.escape();
            self.save_title_options();
            return Ok(());
        }
        if self.sign_edit.is_some() {
            self.close_sign_editor();
            return Ok(());
        }
        if self.book.is_some() {
            self.close_book();
            return Ok(());
        }
        self.key_pressed(KeyCode::Escape)
    }

    /// The left stick as the move vector, while playing.
    pub(super) fn pad_move(&self) -> Option<[f32; 2]> {
        let settings = &self.settings.controller;
        if (!settings.enabled && !cfg!(target_os = "android"))
            || self.pad_in_menu()
            || !self.pad_focused()
        {
            return None;
        }
        #[cfg(target_os = "android")]
        {
            if horizon_client::android_runtime::active_input()
                == horizon_client::android_gamepad::InputMode::Controller
                || (self.pad.touch.settings.analog_movement
                    && self.pad.touch.settings.mode != horizon_client::touch::Mode::Dpad)
            {
                gamepad::move_vector(self.pad.state.left, settings.move_deadzone)
            } else {
                gamepad::java_move_vector(self.pad.state.left, settings.move_deadzone)
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            gamepad::move_vector(self.pad.state.left, settings.move_deadzone)
        }
    }

    /// The right stick turns the view, once per frame (`Entity.turn`).
    pub(super) fn pad_look(&mut self, seconds: f32) {
        #[cfg(target_os = "android")]
        {
            let enabled = self.settings.touch.gyroscope
                && !self.pad_in_menu()
                && self.player.is_some()
                && self.pad_focused()
                && crate::touch_visible();
            let rotation = if enabled {
                crate::platform::display_rotation()
            } else {
                0
            };
            let delta = crate::device_runtime::gyro(
                enabled,
                self.settings.touch.gyro_sensitivity,
                rotation,
            );
            if enabled {
                self.tutorial
                    .mouse(f64::from(delta[0]), f64::from(delta[1]));
                self.turn_player(f64::from(delta[0] / 0.15), f64::from(delta[1] / 0.15));
            }
        }
        let settings = &self.settings.controller;
        if !settings.enabled || self.pad_in_menu() || self.player.is_none() || !self.pad_focused() {
            return;
        }
        // A long frame (a hitch) must not spin the view.
        let (yaw, pitch) = gamepad::look_delta(
            self.pad.state.right,
            settings,
            seconds.min(0.1),
            self.scoping(),
        );
        if yaw == 0.0 && pitch == 0.0 {
            return;
        }
        self.tutorial.mouse(f64::from(yaw), f64::from(pitch));
        self.turn_player(f64::from(yaw / 0.15), f64::from(pitch / 0.15));
    }

    /// Vibrate the active pad, when vibration is on and the pad is the one
    /// playing (a pad lying idle while the mouse plays stays still).
    pub(super) fn pad_rumble(&mut self, rumble: Rumble) {
        let settings = &self.settings.controller;
        if (!settings.enabled && !cfg!(target_os = "android"))
            || !settings.vibration
            || !self.pad.in_use
        {
            return;
        }
        let Some(pulse) = rumble.pulse() else {
            return;
        };
        self.pad.last_rumble = Some((rumble.name(), pulse));
        self.pad_vibrate(rumble, pulse);
    }

    /// `horizon-ctl pad ...`.
    pub(super) fn pad_command(&mut self, words: &[&str]) -> Result<Value> {
        let arg = |i: usize| -> Result<&str> {
            words
                .get(i)
                .copied()
                .ok_or_else(|| format!("usage: {}", HELP[0]).into())
        };
        let number = |i: usize| -> Result<f32> {
            let v: f32 = arg(i)?.parse()?;
            if v.is_finite() {
                Ok(v)
            } else {
                Err("not a number".into())
            }
        };
        match arg(1)? {
            "status" => {
                let s = &self.pad.state;
                let held: Vec<&str> = Button::ALL
                    .into_iter()
                    .filter(|b| self.pad.edges.is_held(*b))
                    .map(Button::name)
                    .collect();
                let mut status = json!({
                    "enabled": self.settings.controller.enabled,
                    "glyphs": self.pad.style.name(),
                    "last_rumble": self.pad.last_rumble.map(|(why, p)| json!({
                        "reason": why, "strong": p.strong, "weak": p.weak, "ms": p.millis,
                    })),
                    "in_use": self.pad.in_use,
                    "menu": self.pad_menu_status(),
                    "held": held,
                    "left": s.left,
                    "right": s.right,
                    "triggers": s.triggers,
                });
                if let (Some(status), Value::Object(devices)) =
                    (status.as_object_mut(), self.pad_devices_status())
                {
                    status.extend(devices);
                }
                Ok(status)
            }
            "release" => {
                self.pad.injected = PadState::default();
                self.pad.taps.clear();
                Ok(json!({ "ok": true }))
            }
            "stick" => {
                let v = [number(3)?.clamp(-1.0, 1.0), number(4)?.clamp(-1.0, 1.0)];
                match arg(2)? {
                    "left" => self.pad.injected.left = v,
                    "right" => self.pad.injected.right = v,
                    other => return Err(format!("stick must be left or right, not {other}").into()),
                }
                Ok(json!({ "ok": true }))
            }
            "trigger" => {
                let v = number(3)?.clamp(0.0, 1.0);
                match arg(2)? {
                    "left" => self.pad.injected.triggers[0] = v,
                    "right" => self.pad.injected.triggers[1] = v,
                    other => {
                        return Err(format!("trigger must be left or right, not {other}").into());
                    }
                }
                Ok(json!({ "ok": true }))
            }
            name => {
                let button = Button::parse(name)
                    .ok_or_else(|| format!("unknown pad button {name}; {}", HELP[0]))?;
                match arg(2)? {
                    "down" => self.pad.injected.set(button, true),
                    "up" => self.pad.injected.set(button, false),
                    "tap" => {
                        self.pad.injected.set(button, true);
                        self.pad.taps.push(button);
                    }
                    other => {
                        return Err(
                            format!("pad action must be down, up or tap, not {other}").into()
                        );
                    }
                }
                Ok(json!({ "ok": true }))
            }
        }
    }
}
