//! Team rules in gameplay for differential tests against vanilla
//! (tools/test_team_rules_parity.py): friendly fire, alliances, collision
//! rules, seeing invisible team mates and name tag visibility, from
//! `horizon_client::scoreboard`, in VanillaTeamRulesOracle's JSON lines.
//!
//! usage: team_rules_fixture INPUTS OUTPUT
use horizon_client::scoreboard::{self as sb, CollisionRule, NameView, Scoreboard, Visibility};
use serde_json::{Value, json};
use std::{fs, io::Write};

pub fn main() {
    let args: Vec<String> = crate::args().collect();
    let mut out = std::io::BufWriter::new(fs::File::create(&args[2]).expect("output"));
    for line in fs::read_to_string(&args[1]).expect("inputs").lines() {
        let c: Value = serde_json::from_str(line).expect("case");
        let mut board = Scoreboard::default();
        for name in ["A", "B"] {
            let o = &c[name];
            let t = board.add_team(name);
            t.allow_friendly_fire = o["ff"].as_bool().unwrap_or(true);
            t.see_friendly_invisibles = o["si"].as_bool().unwrap_or(true);
            t.collision_rule =
                CollisionRule::by_name(o["collision"].as_str().unwrap_or("")).expect("rule");
            t.nametag_visibility =
                Visibility::by_name(o["nametag"].as_str().unwrap_or("")).expect("visibility");
        }
        if let Some(name) = c["other"].as_str() {
            board.add_to_team("HiddenNPC", name);
        }
        if let Some(name) = c["own"].as_str() {
            board.add_to_team(&horizon_client::save::uuid_text([0, 0, 0, 7]), name);
        }
        let profile_team = board.entity_team(Some("HiddenNPC"), Some([0, 0, 0, 7]));
        let team = |k: &str| c[k].as_str().and_then(|n| board.team(n));
        let flag = |k: &str| c[k].as_bool().unwrap_or(false);
        let (own, other, viewer) = (team("own"), team("other"), team("viewer"));
        let invisible =
            sb::invisible_to(other, viewer, flag("viewer_spectator"), flag("invisible"));
        let view = NameView {
            discrete: flag("discrete"),
            distance_sqr: c["distance"].as_f64().unwrap_or(0.0),
            invisible_to_viewer: invisible,
            hud_hidden: flag("hud_hidden"),
            is_camera: flag("camera_is_entity"),
            is_vehicle: flag("vehicle"),
        };
        let r = json!({
            "case": c["case"],
            "profile_team": profile_team.map(|t| t.name.as_str()),
            "profile_pushes": sb::pushable_by(own, profile_team, flag("pushable")),
            "damage": sb::teams_allow_damage(own, other),
            "allied": sb::teams_allied(own, other),
            "pushes": !flag("spectator") && sb::pushable_by(own, other, flag("pushable")),
            "invisible": invisible,
            "name": sb::name_shown(other, viewer, view),
        });
        writeln!(out, "{r}").expect("write");
    }
}
