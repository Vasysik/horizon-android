// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The mod's settings page: the WebAssembly mod's one button.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::official_recipes as m;
    if mod_id != m::ID {
        return;
    }
    buttons.push(m::give_label(m::gives(game.native_setting(m::ID, "give"))));
}
