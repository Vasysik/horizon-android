// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The player's click, search or key on the panel, when it is this mod's.
pub(super) fn run(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    event: &crate::game::native_hooks::NativePanelEvent,
) {
    use crate::game::official_recipes as m;
    if mod_id != m::ID || !game.native_on(m::ID) {
        return;
    }
    crate::game::official_recipes_glue::event(game, event);
}
