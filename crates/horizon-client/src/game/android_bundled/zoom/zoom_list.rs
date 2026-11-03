// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// Zoom's row in Options > Mods, with its key.
pub(super) fn run(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::official_zoom as zoom;
    mods.push(crate::game::native_hooks::NativeMod {
        id: zoom::ID.into(),
        title: format!("Zoom {}", zoom::VERSION),
        description: zoom::DESCRIPTION.into(),
        keys: vec![[
            zoom::KEY.into(),
            "Zoom key".into(),
            zoom::DEFAULT_KEY.into(),
        ]],
    });
}
