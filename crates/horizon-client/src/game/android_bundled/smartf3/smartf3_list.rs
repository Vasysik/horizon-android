// Copyright (c) 2026 d3grando
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: 184b92bab7851684bbe6b840c5b8cacc10fa05f2. Android static-bundle adaptation.
// native_hooks::Game::native_mods (a hook point): SmartF3's row in Options > Mods.
pub(super) fn run(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::smartf3 as sf;
    mods.push(crate::game::native_hooks::NativeMod {
        id: sf::ID.into(),
        title: format!("{} {}", sf::NAME, sf::VERSION),
        description: sf::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
