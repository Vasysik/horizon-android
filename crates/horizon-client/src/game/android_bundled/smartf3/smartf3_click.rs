// Copyright (c) 2026 d3grando
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: 184b92bab7851684bbe6b840c5b8cacc10fa05f2. Android static-bundle adaptation.
// native_hooks::Game::native_settings_click (a hook point): a click on
// SmartF3's settings page changes the setting, keeps it and applies it.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::smartf3::ID {
        game.smartf3_settings_click(button);
    }
}
