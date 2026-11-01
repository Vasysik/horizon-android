// Copyright (c) 2026 d3grando
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: 184b92bab7851684bbe6b840c5b8cacc10fa05f2. Android static-bundle adaptation.
// native_hooks::Game::native_command (a hook point): run `/smartf3`, leave
// everything else to the game (and to other mods).
pub(super) fn run(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if !*ran && game.smartf3_command(command) {
        *ran = true;
    }
}
