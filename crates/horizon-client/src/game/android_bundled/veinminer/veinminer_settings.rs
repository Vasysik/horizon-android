// Persistent ON/OFF setting shown on Options > Mods > Veinminer.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    let id = crate::game::veinminer::ID;
    if mod_id != id {
        return;
    }
    let enabled = game.native_setting(id, "enabled").as_deref() != Some("false");
    buttons.push(format!(
        "Enable Veinminer: {}",
        if enabled { "ON" } else { "OFF" }
    ));
}
