// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// A click on the mod's settings page: "Click gives items" flips, kept where
// the WebAssembly mod kept it.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::official_recipes as m;
    if mod_id != m::ID || button != 0 {
        return;
    }
    let give = !m::gives(game.native_setting(m::ID, "give"));
    game.native_set_setting(m::ID, "give", if give { "true" } else { "false" });
}
