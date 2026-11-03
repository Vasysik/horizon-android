// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The zoom in effect, unless another mod's came first.
pub(super) fn run(
    game: &crate::game::Game<'_>,
    zoom: &mut Option<crate::game::native_hooks::NativeZoom>,
) {
    if zoom.is_none()
        && game.native_on(crate::game::official_zoom::ID)
        && let Some((factor, smooth, turn)) = crate::game::official_zoom::view()
    {
        *zoom = Some(crate::game::native_hooks::NativeZoom {
            factor,
            smooth,
            turn,
        });
    }
}
