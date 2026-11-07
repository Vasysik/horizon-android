// Clicking the single button flips and persists the global Veinminer setting.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    let id = crate::game::veinminer::ID;
    if mod_id != id || button != 0 {
        return;
    }
    let enabled = game.native_setting(id, "enabled").as_deref() != Some("false");
    game.veinminer_set_global_enabled(!enabled);
}
