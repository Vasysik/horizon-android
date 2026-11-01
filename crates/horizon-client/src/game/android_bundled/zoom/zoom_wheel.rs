// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// While the key is held, the wheel zooms instead of moving the hotbar.
pub(super) fn run(game: &crate::game::Game<'_>, steps: f32, claimed: &mut bool) {
    use crate::game::official_zoom as zoom;
    if game.native_on(zoom::ID) && zoom::wheel(steps) {
        *claimed = true;
    }
}
