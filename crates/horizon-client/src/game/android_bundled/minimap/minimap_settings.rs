// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The mod's settings page: the WebAssembly mod's buttons.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::official_minimap as m;
    if mod_id != m::ID {
        return;
    }
    let game = &*game;
    buttons.extend(m::settings(|key| game.native_setting(m::ID, key)).labels());
}
