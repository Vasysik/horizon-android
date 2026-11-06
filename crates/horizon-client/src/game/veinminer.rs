// Native adapter for Horizon 028f60881678a9a0e3e198f693a82858fa475c76.
use super::{Game, Result};

pub(super) const ID: &str = "veinminer";
mod engine {
    // Original Rust implementation of the documented vein-mining behavior.
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type Position = [i32; 3];
    pub type Actor = [i32; 4];

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
    pub struct Settings {
        pub enabled: bool,
        pub cooldown: i64,
        pub must_sneak: bool,
        pub max_chain: usize,
        pub need_correct_tool: bool,
        pub search_radius: i32,
        pub decrease_durability: bool,
        pub hunger_per_block: f32,
        pub separate_group_mining: bool,
        pub permission_restricted: bool,
    }
    impl Default for Settings {
        fn default() -> Self {
            Self {
                enabled: true,
                cooldown: 20,
                must_sneak: false,
                max_chain: 100,
                need_correct_tool: true,
                search_radius: 1,
                decrease_durability: true,
                hunger_per_block: 0.0,
                separate_group_mining: false,
                permission_restricted: false,
            }
        }
    }
    impl Settings {
        pub fn validate(&self) -> Result<(), String> {
            if !(1..=4096).contains(&self.max_chain) {
                return Err("maxChain must be 1..4096 (including the original block)".into());
            }
            if !(1..=3).contains(&self.search_radius) {
                return Err("searchRadius must be 1..3".into());
            }
            if !(0..=72_000).contains(&self.cooldown) {
                return Err("cooldown must be 0..72000 ticks".into());
            }
            if !self.hunger_per_block.is_finite() || !(0.0..=40.0).contains(&self.hunger_per_block)
            {
                return Err("hungerPerBlock must be finite and 0..40".into());
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
    pub struct Overrides {
        pub cooldown: Option<i64>,
        pub must_sneak: Option<bool>,
        pub max_chain: Option<usize>,
        pub need_correct_tool: Option<bool>,
        pub search_radius: Option<i32>,
        pub decrease_durability: Option<bool>,
        pub hunger_per_block: Option<f32>,
        pub separate_group_mining: Option<bool>,
        pub permission_restricted: Option<bool>,
    }
    impl Overrides {
        fn apply(&self, settings: &mut Settings) {
            macro_rules! fields { ($($field:ident),*) => { $(if let Some(value) = self.$field { settings.$field = value; })* }; }
            fields!(
                cooldown,
                must_sneak,
                max_chain,
                need_correct_tool,
                search_radius,
                decrease_durability,
                hunger_per_block,
                separate_group_mining,
                permission_restricted
            );
        }
    }
    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Group {
        pub id: String,
        pub blocks: BTreeSet<String>,
        #[serde(default)]
        pub tools: BTreeSet<String>,
        #[serde(default)]
        pub overrides: Overrides,
    }
    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        pub settings: Settings,
        pub blocks: BTreeSet<String>,
        pub groups: Vec<Group>,
        pub players: BTreeSet<String>,
    }
    fn id_valid(s: &str) -> bool {
        let Some((namespace, path)) = s.split_once(':') else {
            return false;
        };
        !namespace.is_empty()
            && !path.is_empty()
            && namespace
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
            && path
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"/_.-".contains(&b))
    }
    impl Config {
        pub fn from_json(source: &str) -> Result<Self, String> {
            let config: Self = serde_json::from_str(source).map_err(|e| e.to_string())?;
            config.validate()?;
            Ok(config)
        }
        pub fn validate(&self) -> Result<(), String> {
            self.settings.validate()?;
            if self.groups.len() > 256 || self.blocks.len() > 4096 {
                return Err("too many groups or blocks".into());
            }
            let mut ids = BTreeSet::new();
            for group in &self.groups {
                if group.id.is_empty() || !ids.insert(&group.id) {
                    return Err(format!("empty/duplicate group id: {}", group.id));
                }
                if group.blocks.is_empty() || group.blocks.len() > 4096 || group.tools.len() > 4096
                {
                    return Err(format!("group {} has invalid block/tool count", group.id));
                }
                let mut settings = self.settings.clone();
                group.overrides.apply(&mut settings);
                settings
                    .validate()
                    .map_err(|e| format!("group {}: {e}", group.id))?;
            }
            for id in self.blocks.iter().chain(
                self.groups
                    .iter()
                    .flat_map(|g| g.blocks.iter().chain(&g.tools)),
            ) {
                if !id_valid(id) {
                    return Err(format!(
                        "expected explicit namespaced block/item id, got {id:?}"
                    ));
                }
            }
            Ok(())
        }
        pub fn action(
            &self,
            name: &str,
            tool: Option<&str>,
        ) -> Option<(Settings, BTreeSet<String>)> {
            let groups: Vec<_> = self
                .groups
                .iter()
                .filter(|g| g.blocks.contains(name))
                .collect();
            if groups.is_empty() && !self.blocks.contains(name) {
                return None;
            }
            let mut settings = self.settings.clone();
            if let Some(first) = groups.first() {
                first.overrides.apply(&mut settings);
            }
            let tools: BTreeSet<_> = groups.iter().flat_map(|g| &g.tools).collect();
            if !tools.is_empty()
                && tool.is_none_or(|id| !tools.iter().any(|allowed| allowed.as_str() == id))
            {
                return None;
            }
            let targets = if settings.separate_group_mining || groups.is_empty() {
                BTreeSet::from([name.to_owned()])
            } else {
                groups
                    .iter()
                    .flat_map(|g| g.blocks.iter().cloned())
                    .collect()
            };
            Some((settings, targets))
        }
    }
    impl Default for Config {
        fn default() -> Self {
            let mut groups = Vec::new();
            for ore in [
                "coal", "copper", "iron", "gold", "redstone", "lapis", "diamond", "emerald",
            ] {
                groups.push(Group {
                    id: format!("{ore}-ore"),
                    blocks: [
                        format!("minecraft:{ore}_ore"),
                        format!("minecraft:deepslate_{ore}_ore"),
                    ]
                    .into(),
                    tools: BTreeSet::new(),
                    overrides: Overrides::default(),
                });
            }
            for wood in [
                "oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry",
                "pale_oak",
            ] {
                groups.push(Group {
                    id: format!("{wood}-logs"),
                    blocks: [
                        format!("minecraft:{wood}_log"),
                        format!("minecraft:{wood}_wood"),
                    ]
                    .into(),
                    tools: BTreeSet::new(),
                    overrides: Overrides::default(),
                });
            }
            Self {
                settings: Settings::default(),
                blocks: [
                    "minecraft:nether_quartz_ore".into(),
                    "minecraft:nether_gold_ore".into(),
                    "minecraft:ancient_debris".into(),
                ]
                .into(),
                groups,
                players: BTreeSet::new(),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Block {
        pub name: String,
        pub state: u32,
        pub breakable: bool,
    }
    #[derive(Clone, Debug)]
    pub struct Player {
        pub actor: Actor,
        pub name: String,
        pub tick: i64,
        pub survival: bool,
        pub sneaking: bool,
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Tool {
        pub item: String,
        pub remaining: Option<u32>,
    }

    pub trait Host {
        type Error;
        fn player(&mut self) -> Player;
        // None means unloaded/outside build bounds. This must never load a chunk.
        fn block(&self, position: Position) -> Option<Block>;
        fn tool(&self) -> Option<Tool>;
        fn can_harvest(&self, block: &Block) -> bool;
        fn has_food(&self) -> bool;
        // Must use the game's ordinary player-mining path, never /setblock.
        fn mine(
            &mut self,
            position: Position,
            preserve_durability: bool,
        ) -> Result<bool, Self::Error>;
        fn exhaust(&mut self, amount: f32);
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Hit {
        pub position: Position,
        pub block: Block,
    }
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Report {
        pub planned: usize,
        pub extra_broken: usize,
        pub changed: usize,
        pub stopped: bool,
    }

    pub fn preview<H: Host>(
        host: &H,
        origin: Position,
        targets: &BTreeSet<String>,
        settings: &Settings,
    ) -> Vec<Hit> {
        let mut hits = Vec::new();
        if settings.validate().is_err() {
            return hits;
        }
        let mut seen = BTreeSet::from([origin]);
        let mut queue = VecDeque::from([origin]);
        // This bound caps reads of nonmatching neighbours as well as accepted blocks.
        let max_examined = settings.max_chain.saturating_mul(343).min(1_000_000);
        while let Some(position) = queue.pop_front() {
            if hits.len() >= settings.max_chain {
                break;
            }
            let Some(block) = host.block(position) else {
                continue;
            };
            if !block.breakable
                || !targets.contains(&block.name)
                || settings.need_correct_tool && !host.can_harvest(&block)
            {
                continue;
            }
            hits.push(Hit { position, block });
            let r = settings.search_radius;
            for x in -r..=r {
                for y in -r..=r {
                    for z in -r..=r {
                        let (Some(x), Some(y), Some(z)) = (
                            position[0].checked_add(x),
                            position[1].checked_add(y),
                            position[2].checked_add(z),
                        ) else {
                            continue;
                        };
                        let next = [x, y, z];
                        if seen.len() < max_examined && seen.insert(next) {
                            queue.push_back(next);
                        }
                    }
                }
            }
        }
        hits
    }

    #[derive(Default)]
    pub struct Runtime {
        cooldowns: BTreeMap<Actor, i64>,
        disabled: BTreeSet<Actor>,
    }
    impl Runtime {
        pub fn enabled(&self, actor: Actor) -> bool {
            !self.disabled.contains(&actor)
        }
        pub fn set_enabled(&mut self, actor: Actor, enabled: bool) {
            if enabled {
                self.disabled.remove(&actor);
            } else {
                self.disabled.insert(actor);
            }
        }
        pub fn mine<H: Host>(
            &mut self,
            host: &mut H,
            config: &Config,
            origin: Position,
        ) -> Result<Report, H::Error> {
            let player = host.player();
            let tool = host.tool();
            let seed = host.block(origin);
            let action = seed
                .as_ref()
                .filter(|b| b.breakable)
                .and_then(|b| config.action(&b.name, tool.as_ref().map(|t| t.item.as_str())));
            let action = action.filter(|(settings, _)| {
                settings.enabled
                    && player.survival
                    && self.enabled(player.actor)
                    && (!settings.must_sneak || player.sneaking)
                    && (!settings.permission_restricted || config.players.contains(&player.name))
                    && self
                        .cooldowns
                        .get(&player.actor)
                        .is_none_or(|end| player.tick >= *end)
            });
            let hits = if let Some((settings, targets)) = &action {
                preview(host, origin, targets, settings)
            } else {
                Vec::new()
            };
            let mut report = Report {
                planned: hits.len(),
                ..Report::default()
            };
            // The original click always follows Horizon's original behavior, even if the mod is off.
            if !host.mine(origin, false)? || hits.len() <= 1 {
                return Ok(report);
            }
            let Some((settings, _)) = action else {
                return Ok(report);
            };
            self.cooldowns.retain(|_, end| *end > player.tick);
            self.cooldowns
                .insert(player.actor, player.tick.saturating_add(settings.cooldown));
            for hit in hits.into_iter().skip(1) {
                if host.tool() != tool
                    && host.tool().as_ref().map(|t| &t.item) != tool.as_ref().map(|t| &t.item)
                {
                    report.stopped = true;
                    break;
                }
                if settings.decrease_durability
                    && host
                        .tool()
                        .as_ref()
                        .and_then(|t| t.remaining)
                        .is_some_and(|n| n <= 1)
                    || settings.hunger_per_block > 0.0 && !host.has_food()
                {
                    report.stopped = true;
                    break;
                }
                if host.block(hit.position).as_ref() != Some(&hit.block) {
                    report.changed += 1;
                    continue;
                }
                if settings.need_correct_tool && !host.can_harvest(&hit.block) {
                    report.stopped = true;
                    break;
                }
                if !host.mine(hit.position, !settings.decrease_durability)? {
                    report.stopped = true;
                    break;
                }
                host.exhaust(settings.hunger_per_block);
                report.extra_broken += 1;
            }
            Ok(report)
        }
    }
}
use engine::{Block, Config, Host, Player, Position, Runtime, Tool};

#[derive(Default)]
pub(super) struct State {
    runtime: Runtime,
    config: Option<Config>,
}

thread_local! {
    static STATES: std::cell::RefCell<std::collections::BTreeMap<std::path::PathBuf, State>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}
fn with_state<T>(game: &mut Game<'_>, run: impl FnOnce(&mut Game<'_>, &mut State) -> T) -> T {
    struct Lease {
        key: std::path::PathBuf,
        state: State,
    }
    impl Drop for Lease {
        fn drop(&mut self) {
            STATES.with(|states| {
                states
                    .borrow_mut()
                    .insert(self.key.clone(), std::mem::take(&mut self.state));
            });
        }
    }
    let key = game.world_root.clone();
    let state = STATES.with(|states| states.borrow_mut().remove(&key).unwrap_or_default());
    let mut lease = Lease { key, state };
    run(game, &mut lease.state)
}

fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HORIZON_VEINMINER_CONFIG").map(Into::into)
}
fn load_config() -> std::result::Result<Config, String> {
    match config_path() {
        Some(path) => Config::from_json(
            &std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        ),
        None => Ok(Config::default()),
    }
}

struct Native<'g, 'r> {
    game: &'g mut Game<'r>,
}
impl Host for Native<'_, '_> {
    type Error = Box<dyn std::error::Error>;
    fn player(&mut self) -> Player {
        Player {
            actor: self.game.acting_player_uuid(),
            name: self
                .game
                .lan_acting_name
                .as_deref()
                .unwrap_or(super::chat::PLAYER_NAME)
                .to_owned(),
            tick: self.game.world.game_time,
            survival: self.game.remote.is_none()
                && self.game.player.is_some()
                && self.game.may_build()
                && !self.game.creative,
            sneaking: self
                .game
                .keys
                .contains(&winit::keyboard::KeyCode::ShiftLeft),
        }
    }
    fn block(&self, p: Position) -> Option<Block> {
        if p[1] < self.game.world.shape.min_y
            || p[1] >= self.game.world.shape.max_y()
            || !self.game.world.loaded(p[0], p[2])
        {
            return None;
        }
        let state = self.game.world.block(p[0], p[1], p[2]);
        let facts = self.game.res.facts().get(state);
        Some(Block {
            name: facts.name.to_string(),
            state: state.0,
            breakable: facts.hardness >= 0.0 && !self.game.world.is_air(state),
        })
    }
    fn tool(&self) -> Option<Tool> {
        let stack = self.game.held()?;
        let max = self
            .game
            .res
            .items()
            .get(&stack.item)
            .map_or(0, |i| i.max_damage);
        Some(Tool {
            item: stack.item.to_string(),
            remaining: (max > 0).then_some(max.saturating_sub(stack.damage)),
        })
    }
    fn can_harvest(&self, block: &Block) -> bool {
        let facts = self
            .game
            .res
            .facts()
            .get(horizon_world::registry::BlockStateId(block.state));
        !facts.requires_tool
            || self
                .game
                .res
                .items()
                .correct_for_drops(self.game.held(), &block.name)
    }
    fn has_food(&self) -> bool {
        self.game
            .player
            .as_ref()
            .is_some_and(|p| p.health.food.food > 0 || p.health.food.saturation > 0.0)
    }
    fn mine(&mut self, p: Position, preserve: bool) -> Result<bool> {
        let before = self.block(p);
        let held = preserve.then(|| self.game.inventory[self.game.selected].clone());
        // Temporarily remove pre-existing damage, then restore damage (and a tool
        // destroyed by this one operation) without changing other mining effects.
        if preserve && let Some(stack) = self.game.inventory[self.game.selected].as_mut() {
            stack.damage = 0;
        }
        let result = self.game.break_block_as_player(p);
        if let Some(original) = held {
            match (self.game.inventory[self.game.selected].as_mut(), original) {
                (Some(now), Some(was)) if now.item == was.item => now.damage = was.damage,
                (None, Some(was)) => self.game.inventory[self.game.selected] = Some(was),
                _ => {}
            }
        }
        result?;
        Ok(before.is_some() && self.block(p) != before)
    }
    fn exhaust(&mut self, amount: f32) {
        if amount > 0.0
            && let Some(p) = self.game.player.as_mut()
        {
            p.cause_food_exhaustion(amount);
        }
    }
}
impl Game<'_> {
    pub(super) fn veinminer_set_global_enabled(&mut self, enabled: bool) {
        self.native_set_setting(ID, "enabled", if enabled { "true" } else { "false" });
    }

    pub(super) fn veinminer_break_block(&mut self, pos: Position) -> Result<()> {
        with_state(self, |game, state| {
            if state.config.is_none() {
                state.config = Some(load_config().unwrap_or_else(|error| {
                    eprintln!("Veinminer disabled: {error}");
                    let mut config = Config::default();
                    config.settings.enabled = false;
                    config
                }));
            }
            let enabled = game.native_setting(ID, "enabled").as_deref() != Some("false");
            let mut config = state.config.as_ref().unwrap().clone();
            if !enabled {
                config.settings.enabled = false;
            }
            state
                .runtime
                .mine(&mut Native { game }, &config, pos)
                .map(|_| ())
        })
    }
    pub(super) fn veinminer_command(&mut self, command: &str) -> Option<bool> {
        with_state(self, |game, state| {
            let mut words = command.split_whitespace();
            if words.next()? != "veinminer" {
                return None;
            }
            let args: Vec<_> = words.collect();
            let actor = game.acting_player_uuid();
            let globally_enabled = game.native_setting(ID, "enabled").as_deref() != Some("false");
            let message = match args.as_slice() {
                [] | ["status"] => {
                    if !globally_enabled {
                        "Veinminer: off (disabled in Options > Mods)".into()
                    } else {
                        format!(
                            "Veinminer: {}",
                            if state.runtime.enabled(actor) {
                                "on"
                            } else {
                                "off"
                            }
                        )
                    }
                }
                ["toggle"] | ["on"] | ["off"] => {
                    let on = match args[0] {
                        "on" => true,
                        "off" => false,
                        _ => !state.runtime.enabled(actor),
                    };
                    state.runtime.set_enabled(actor, on);
                    if !globally_enabled {
                        format!(
                            "Veinminer player preference: {} (global setting is OFF)",
                            if on { "on" } else { "off" }
                        )
                    } else {
                        format!("Veinminer: {}", if on { "on" } else { "off" })
                    }
                }
                ["reload"] => {
                    if game.ui.registries.permission < 2 {
                        game.command_failure(horizon_client::text::Text::literal(
                            "Veinminer reload requires permission level 2",
                        ));
                        return Some(false);
                    }
                    match load_config() {
                        Ok(config) => {
                            state.config = Some(config);
                            "Veinminer configuration reloaded".into()
                        }
                        Err(error) => {
                            game.command_failure(horizon_client::text::Text::literal(error));
                            return Some(false);
                        }
                    }
                }
                _ => {
                    game.command_failure(horizon_client::text::Text::literal(
                        "Usage: /veinminer [status|toggle|on|off|reload]",
                    ));
                    return Some(false);
                }
            };
            game.system_message(horizon_client::text::Text::literal(message));
            Some(true)
        })
    }
}

#[cfg(test)]
mod engine_tests {
    use super::engine as horizon_veinminer;
    use horizon_veinminer::*;
    use std::collections::{BTreeMap, BTreeSet};

    struct World {
        blocks: BTreeMap<Position, Block>,
        player: Player,
        tool: Option<Tool>,
        food: bool,
        harvest: bool,
        calls: Vec<(Position, bool)>,
        drops: usize,
        xp: usize,
        exhaustion: f32,
        failed: BTreeSet<Position>,
        error: Option<Position>,
        change_after_origin: Option<Position>,
    }
    impl World {
        fn new(points: &[Position]) -> Self {
            Self {
                blocks: points
                    .iter()
                    .map(|p| (*p, block("minecraft:iron_ore", 1)))
                    .collect(),
                player: Player {
                    actor: [0, 0, 0, 1],
                    name: "Alex".into(),
                    tick: 0,
                    survival: true,
                    sneaking: false,
                },
                tool: Some(Tool {
                    item: "minecraft:iron_pickaxe".into(),
                    remaining: Some(1000),
                }),
                food: true,
                harvest: true,
                calls: Vec::new(),
                drops: 0,
                xp: 0,
                exhaustion: 0.0,
                failed: BTreeSet::new(),
                error: None,
                change_after_origin: None,
            }
        }
    }
    fn block(name: &str, state: u32) -> Block {
        Block {
            name: name.into(),
            state,
            breakable: true,
        }
    }
    impl Host for World {
        type Error = &'static str;
        fn player(&mut self) -> Player {
            self.player.clone()
        }
        fn block(&self, p: Position) -> Option<Block> {
            self.blocks.get(&p).cloned()
        }
        fn tool(&self) -> Option<Tool> {
            self.tool.clone()
        }
        fn can_harvest(&self, _: &Block) -> bool {
            self.harvest
        }
        fn has_food(&self) -> bool {
            self.food
        }
        fn mine(&mut self, p: Position, preserve: bool) -> Result<bool, Self::Error> {
            self.calls.push((p, preserve));
            if self.error == Some(p) {
                return Err("native mining failed");
            }
            if self.failed.contains(&p) {
                return Ok(false);
            }
            let removed = self.blocks.remove(&p).is_some();
            if removed {
                self.drops += 1;
                self.xp += 2;
                if !preserve
                    && let Some(t) = self.tool.as_mut()
                    && let Some(n) = t.remaining.as_mut()
                {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        self.tool = None;
                    }
                }
            }
            if self.calls.len() == 1
                && let Some(p) = self.change_after_origin
            {
                self.blocks.insert(p, block("minecraft:diamond_ore", 99));
            }
            Ok(removed)
        }
        fn exhaust(&mut self, n: f32) {
            self.exhaustion += n;
        }
    }
    fn mine(world: &mut World, config: &Config) -> Report {
        Runtime::default().mine(world, config, [0, 0, 0]).unwrap()
    }

    #[test]
    fn connected_diagonals_group_variants_and_original_once_keep_drops_and_xp() {
        let mut w = World::new(&[[0, 0, 0], [1, 1, 1], [2, 1, 1], [20, 0, 0]]);
        w.blocks
            .insert([2, 1, 1], block("minecraft:deepslate_iron_ore", 2));
        let report = mine(&mut w, &Config::default());
        assert_eq!((report.planned, report.extra_broken), (3, 2));
        assert_eq!((w.calls.len(), w.drops, w.xp), (3, 3, 6));
        assert_eq!(w.calls.iter().filter(|(p, _)| *p == [0, 0, 0]).count(), 1);
        assert!(w.blocks.contains_key(&[20, 0, 0]));
        assert_eq!(w.tool.unwrap().remaining, Some(997));
    }
    #[test]
    fn plan_is_pure_deterministic_bounded_and_max_chain_includes_origin() {
        let points: Vec<_> = (-8..=8)
            .flat_map(|x| (-8..=8).map(move |z| [x, 0, z]))
            .collect();
        let mut w = World::new(&points);
        let mut c = Config::default();
        c.settings.max_chain = 7;
        let targets = BTreeSet::from(["minecraft:iron_ore".into()]);
        let a = preview(&w, [0, 0, 0], &targets, &c.settings);
        let b = preview(&w, [0, 0, 0], &targets, &c.settings);
        assert_eq!(a, b);
        assert_eq!(a.len(), 7);
        assert_eq!(w.calls.len(), 0);
        assert_eq!(mine(&mut w, &c).extra_broken, 6);
        assert_eq!(w.calls.len(), 7);
    }
    #[test]
    fn loaded_matching_blocks_only_no_generated_chunks_or_unbreakable_bridge() {
        let mut w = World::new(&[[0, 0, 0], [2, 0, 0]]);
        w.blocks.insert([1, 0, 0], block("minecraft:stone", 2));
        assert_eq!(mine(&mut w, &Config::default()).extra_broken, 0);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.blocks.get_mut(&[1, 0, 0]).unwrap().breakable = false;
        assert_eq!(mine(&mut w, &Config::default()).extra_broken, 0);
        let mut w = World::new(&[[0, 0, 0], [2, 0, 0]]);
        let mut c = Config::default();
        c.settings.search_radius = 2;
        assert_eq!(mine(&mut w, &c).extra_broken, 1);
    }
    #[test]
    fn last_durability_point_is_kept_and_broken_initial_tool_stops_chain() {
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]);
        w.tool.as_mut().unwrap().remaining = Some(3);
        let r = mine(&mut w, &Config::default());
        assert_eq!(r.extra_broken, 1);
        assert!(r.stopped);
        assert_eq!(w.tool.unwrap().remaining, Some(1));
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.tool.as_mut().unwrap().remaining = Some(1);
        assert_eq!(mine(&mut w, &Config::default()).extra_broken, 0);
        assert!(w.tool.is_none());
    }
    #[test]
    fn preservation_and_additional_exhaustion_only_apply_to_extra_blocks() {
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.tool.as_mut().unwrap().remaining = Some(2);
        let mut c = Config::default();
        c.settings.decrease_durability = false;
        c.settings.hunger_per_block = 0.5;
        assert_eq!(mine(&mut w, &c).extra_broken, 2);
        assert_eq!(
            w.calls,
            vec![([0, 0, 0], false), ([1, 0, 0], true), ([2, 0, 0], true)]
        );
        assert_eq!(w.tool.unwrap().remaining, Some(1));
        assert_eq!(w.exhaustion, 1.0);
    }
    #[test]
    fn stale_blocks_are_not_mined_even_when_the_new_block_is_whitelisted() {
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.change_after_origin = Some([1, 0, 0]);
        let r = mine(&mut w, &Config::default());
        assert_eq!((r.changed, r.extra_broken), (1, 1));
        assert_eq!(w.blocks[&[1, 0, 0]].name, "minecraft:diamond_ore");
    }
    #[test]
    fn original_failure_never_triggers_extras_and_extra_failure_aborts() {
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.failed.insert([0, 0, 0]);
        assert_eq!(mine(&mut w, &Config::default()).extra_broken, 0);
        assert_eq!(w.calls.len(), 1);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.failed.insert([1, 0, 0]);
        let r = mine(&mut w, &Config::default());
        assert!(r.stopped);
        assert_eq!(w.calls.len(), 2);
    }
    #[test]
    fn native_errors_propagate_without_attempting_later_blocks() {
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        w.error = Some([1, 0, 0]);
        assert_eq!(
            Runtime::default().mine(&mut w, &Config::default(), [0, 0, 0]),
            Err("native mining failed")
        );
        assert_eq!(w.calls.len(), 2);
    }
    #[test]
    fn creative_adventure_sneak_tool_food_permissions_and_global_disable_gate_only_extras() {
        for gate in 0..7 {
            let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
            let mut c = Config::default();
            match gate {
                0 => w.player.survival = false,
                1 => c.settings.must_sneak = true,
                2 => w.harvest = false,
                3 => {
                    c.settings.hunger_per_block = 0.2;
                    w.food = false;
                }
                4 => c.settings.permission_restricted = true,
                5 => c.settings.enabled = false,
                _ => {
                    c.groups.clear();
                    c.blocks.clear();
                }
            }
            assert_eq!(mine(&mut w, &c).extra_broken, 0, "gate {gate}");
            assert_eq!(w.calls.len(), 1);
        }
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.player.sneaking = true;
        let mut c = Config::default();
        c.settings.must_sneak = true;
        c.settings.permission_restricted = true;
        c.players.insert("Alex".into());
        assert_eq!(mine(&mut w, &c).extra_broken, 1);
    }
    #[test]
    fn cooldown_and_toggles_are_per_player_and_original_mining_is_never_cancelled() {
        let mut rt = Runtime::default();
        let c = Config::default();
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 1);
        for tick in [0, 19] {
            let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
            w.player.tick = tick;
            assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 0);
            assert_eq!(w.calls.len(), 1);
        }
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.player.actor = [0, 0, 0, 2];
        assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 1);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.player.tick = 20;
        assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 1);
        rt.set_enabled([0, 0, 0, 1], false);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.player.tick = 100;
        assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 0);
        rt.set_enabled(w.player.actor, true);
        w.blocks.insert([0, 0, 0], block("minecraft:iron_ore", 1));
        assert_eq!(rt.mine(&mut w, &c, [0, 0, 0]).unwrap().extra_broken, 1);
    }
    #[test]
    fn group_tools_overrides_and_separate_group_mining_have_defined_precedence() {
        let mut c = Config::default();
        let g = c.groups.iter_mut().find(|g| g.id == "iron-ore").unwrap();
        g.tools.insert("minecraft:diamond_pickaxe".into());
        g.overrides.max_chain = Some(2);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        assert_eq!(mine(&mut w, &c).extra_broken, 0);
        w.blocks.insert([0, 0, 0], block("minecraft:iron_ore", 1));
        w.tool.as_mut().unwrap().item = "minecraft:diamond_pickaxe".into();
        assert_eq!(mine(&mut w, &c).extra_broken, 1);
        let mut w = World::new(&[[0, 0, 0], [1, 0, 0]]);
        w.blocks
            .insert([1, 0, 0], block("minecraft:deepslate_iron_ore", 2));
        let mut c = Config::default();
        c.settings.separate_group_mining = true;
        assert_eq!(mine(&mut w, &c).extra_broken, 0);
    }
    #[test]
    fn malformed_unknown_and_unbounded_configuration_fails_closed() {
        assert!(Config::default().validate().is_ok());
        for source in [
            r#"{"settings":{"maxChain":0}}"#,
            r#"{"settings":{"searchRadius":100}}"#,
            r#"{"settings":{"hungerPerBlock":-1}}"#,
            r#"{"settings":{"delay":5}}"#,
            r#"{"groups":[{"id":"x","blocks":["no-namespace"]}]}"#,
            r#"{"groups":[{"id":"x","blocks":["minecraft:stone"]},{"id":"x","blocks":["minecraft:dirt"]}]}"#,
        ] {
            assert!(Config::from_json(source).is_err(), "{source}");
        }
        let c = Config::default();
        let json = serde_json::to_string(&c).unwrap();
        assert!(Config::from_json(&json).is_ok());
    }
    #[test]
    fn coordinates_at_integer_limits_do_not_wrap_to_other_chunks() {
        let p = [i32::MAX, 0, i32::MIN];
        let w = World::new(&[p]);
        assert_eq!(
            preview(
                &w,
                p,
                &["minecraft:iron_ore".into()].into(),
                &Settings::default()
            )
            .len(),
            1
        );
    }
}
