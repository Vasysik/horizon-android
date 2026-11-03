// Wrench's adapter to the game: what the rules in `crate::game::wrench`
// (runtime.rs) decide, done in the game that simulates the world (single
// player's server thread, a LAN host's, in-frame single player). Injected as
// `crate::game::wrench_native`. A window joined to a server does nothing: the
// server's world is not its own.
use super::wrench::{self as wr};
use super::{Game, GuestUse};
use horizon_client::command::args::GameMode;
use horizon_client::first_person::Hand;
use horizon_client::text::Text;
use horizon_core::position::BlockPos;
use horizon_protocol::packets::play::{self, Clientbound as C};
use horizon_world::chunk::BlockState;
use horizon_world::level::flags;

impl Game<'_> {
    /// `Game::use_item`'s HEAD: Some (the hand) when the wrench took the
    /// click, None to let the game use it as usual. Runs for the player
    /// acting: a guest in their turn (single player's own player is one on
    /// the server thread), or in-frame single player's player.
    pub(super) fn wrench_use(&mut self) -> Option<Option<Hand>> {
        if self.remote.is_some()
            || self.player.is_none()
            || self.spectator()
            || !self.native_on(wr::ID)
        {
            return None;
        }
        // A server's own (console) use is nobody's.
        if self.lan_acting.is_none() && !self.native_window() {
            return None;
        }
        // The main hand, on a block: a guest's packet names which.
        match self.guest_use {
            Some(GuestUse::OnBlock(Hand::Main)) => {}
            None if !self.entity_targeted() => {}
            _ => return None,
        }
        if !matches!(self.mode, GameMode::Survival | GameMode::Creative) {
            return None;
        }
        let wrench = wr::wrench_item(self.native_setting(wr::ID, wr::ITEM_SETTING).as_deref());
        let held = self.inventory.get(self.selected).and_then(Option::as_ref)?;
        if *held.item != *wrench {
            return None;
        }
        let hit = self.pick()?;
        if !self.wr_may_change(hit.pos) {
            return None;
        }
        let done = if self.wr_sneaking() {
            let on = wr::take_apart_on(
                self.native_setting(wr::ID, wr::TAKE_APART_SETTING)
                    .as_deref(),
            );
            on && self.with_player_actor(|game| game.wr_take_apart(hit.pos))
        } else {
            self.with_player_actor(|game| game.wr_turn(hit.pos, hit.face))
        };
        if !done {
            return None;
        }
        self.swing_arm();
        Some(Some(Hand::Main))
    }

    /// Sneaking: the left shift key, onto which a guest's sneak flag is
    /// mapped for their turn.
    fn wr_sneaking(&self) -> bool {
        self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
            || self.player.as_ref().is_some_and(|p| p.shift_key_down)
    }

    /// What the game checks before a player changes the block at `pos`: in
    /// the level and loaded, inside the world border, within reach of their
    /// eyes (a guest names the block their own client aimed at). Single
    /// player and LAN have no spawn protection.
    fn wr_may_change(&self, pos: [i32; 3]) -> bool {
        let layout = self.world.layout();
        let in_level = pos[1] >= layout.min_y()
            && pos[1] < layout.max_y()
            && self.world.loaded(pos[0], pos[2]);
        let inside_border = self.border.contains(f64::from(pos[0]), f64::from(pos[2]));
        let reach = self.reach() + 1.0;
        let reachable = self.eye_position().is_some_and(|eye| {
            let d: f64 = (0..3)
                .map(|k| (f64::from(pos[k]) + 0.5 - eye[k]).powi(2))
                .sum();
            d <= reach * reach
        });
        in_level && inside_border && reachable
    }

    /// Turns the block at `pos`, its face `face` clicked: the first of
    /// [`wr::turns`] that is a state of the block and can stay there, shaped
    /// to its neighbours (stairs' corners). Its block entity stays (the
    /// block is the same). False when nothing turned.
    fn wr_turn(&mut self, pos: [i32; 3], face: [i32; 3]) -> bool {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let registry = self.pack.assets.registry;
        let Some(now) = registry.state(state) else {
            return false;
        };
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let current = now
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for properties in wr::turns(&now.name, &current, face) {
            let candidate = BlockState {
                name: now.name.clone(),
                properties: properties.into(),
            };
            let Some(turned) = registry.id(&candidate) else {
                continue;
            };
            let shaped = self.with_level(|b, level, ticks| {
                b.can_survive(level, ticks, at, turned)
                    .then(|| b.update_from_neighbour_shapes(level, ticks, at, turned))
            });
            let Some(shaped) = shaped.filter(|s| *s != state && !self.world.is_air(*s)) else {
                continue;
            };
            let placed = self.with_level(|b, level, ticks| {
                b.set_block(level, ticks, at, shaped, flags::UPDATE_ALL)
            });
            if placed {
                let _ = self.process_level_events();
                return true;
            }
        }
        false
    }

    /// Takes the block at `pos` apart when it is a machine
    /// ([`wr::takes_apart`]): it breaks as if mined with the right tool, so
    /// it drops itself and what it held (a creative player gets nothing, as
    /// when mining). False when it is not one.
    fn wr_take_apart(&mut self, pos: [i32; 3]) -> bool {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let registry = self.pack.assets.registry;
        let Some(name) = registry.state(state).map(|s| s.name.clone()) else {
            return false;
        };
        if !wr::takes_apart(&name) {
            return false;
        }
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let drop = !self.creative;
        let broken = self.with_level(|b, level, ticks| b.destroy_block(level, ticks, at, drop));
        let _ = self.process_level_events();
        if !broken {
            self.wr_tell("That can't be taken apart right now");
        }
        broken
    }

    /// A line in the acting player's action bar: a guest gets it at the end
    /// of their turn.
    fn wr_tell(&mut self, text: &str) {
        let line = Text::literal(text.to_owned());
        if self.lan_acting.is_some() {
            self.guest_packets.push(C::SystemChat(play::SystemChat {
                content: horizon_client::lan::data::component_nbt(&line.component()),
                overlay: true,
            }));
        } else {
            self.titles.set_overlay(line, false);
        }
    }

    /// Options > Mods' buttons for the wrench's page.
    pub(super) fn wrench_settings(&self) -> Vec<String> {
        let item = wr::wrench_item(self.native_setting(wr::ID, wr::ITEM_SETTING).as_deref());
        let apart = wr::take_apart_on(
            self.native_setting(wr::ID, wr::TAKE_APART_SETTING)
                .as_deref(),
        );
        vec![
            format!("Wrench: {}", wr::item_label(&item)),
            format!(
                "Sneak takes machines apart: {}",
                if apart { "ON" } else { "OFF" }
            ),
        ]
    }
    /// A click on the page: the next wrench item, or taking apart on or off.
    pub(super) fn wrench_settings_click(&self, button: usize) {
        match button {
            0 => {
                let item =
                    wr::wrench_item(self.native_setting(wr::ID, wr::ITEM_SETTING).as_deref());
                self.native_set_setting(wr::ID, wr::ITEM_SETTING, wr::next_item(&item));
            }
            1 => {
                let on = wr::take_apart_on(
                    self.native_setting(wr::ID, wr::TAKE_APART_SETTING)
                        .as_deref(),
                );
                self.native_set_setting(
                    wr::ID,
                    wr::TAKE_APART_SETTING,
                    if on { "off" } else { "on" },
                );
            }
            _ => {}
        }
    }
}
