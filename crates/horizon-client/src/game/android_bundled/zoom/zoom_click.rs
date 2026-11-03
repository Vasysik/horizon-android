// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// A click on Zoom's settings page: change the setting and keep it.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::official_zoom as zoom;
    if mod_id != zoom::ID {
        return;
    }
    let mut settings = zoom::settings(|key| game.native_setting(zoom::ID, key));
    settings.click(button);
    zoom::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(zoom::ID, key, &value);
    }
}
