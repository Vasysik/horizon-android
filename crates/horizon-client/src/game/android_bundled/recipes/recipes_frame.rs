// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// Each frame: the item panel follows the world and the mod's on/off.
pub(super) fn run(game: &mut crate::game::Game<'_>) {
    crate::game::official_recipes_glue::frame(game);
}
