<<<<<<< HEAD
//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on, the snapshot they
//! read each tick, their hooks (tick, frame, chat, commands, connection, keys, payloads, damage)
//! and what they ask for (chat lines, sounds, payloads, screens, commands). With no mod on,
//! `horizon_mods::Mods` holds no engine and every hook here returns after one check. Starling's
//! drawing and screens are in `mod_ui.rs`; a Horizon `ui` mod's screen and settings page are
//! here, its `render` shapes in `mod_render.rs` and its `input` in `mod_input.rs`.
//!
//! Starling's mods need the mod runtime: the client builds with its `mods` feature.
use super::{Game, Screen, execution_role::ExecutionRole, lan::LanHost, server_mods::ModEvent};
||||||| f8988d5a7
//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on,
//! their tick, frame, chat and command hooks, their HUD panels and what
//! they print. With no mod on, `horizon_mods::Mods` holds no engine and
//! every hook here returns after one check.
//!
//! Code mods run only in builds with the `mods` feature (release builds):
//! without it the game links no wasmtime, and every hook does nothing.
//! Content mods (`content_root`) load in every build.
use super::Game;
=======
//! Mods in the game (`docs/mods.md`): content mods' blocks, items, data and
//! assets (`content_root`), and the Mods page's rows. Code mods are native
//! (`native_hooks.rs`); WebAssembly code mods don't run any more.
use super::Game;
>>>>>>> 31f83499b
use horizon_client::{
<<<<<<< HEAD
    command::{Dispatcher, vanilla},
    extras::Corner,
    extras_hud::{self, Corners},
    hud::HudBatch,
    items::ItemStack,
    keybinds,
    mobs::finalize,
    mod_list::{self, Installed},
    options_ui::Page,
    text::{self, Text},
    ui,
};
use horizon_mods::{
    BlockInfo, Capability, ChatVerdict, Clock, Controls, CreativeCatalog, Effect, Entity,
    EntityInfo, GameMode, Gui, ItemCatalog, ItemInfo, Measure, ModKey, Mods, Output, Player, Runs,
    ServerPlayer, Snapshot, Status, Target, UiScreen, UiWidget, WorldInfo,
||||||| f8988d5a7
    command::{Dispatcher, vanilla},
    extras_hud::Corners,
    hud::HudBatch,
    mod_list,
};
use std::{path::PathBuf, time::Duration};
#[cfg(feature = "mods")]
use {
    super::Screen,
    super::{execution_role::ExecutionRole, lan::LanHost, server_mods::ModEvent},
    horizon_client::{
        extras::Corner,
        extras_hud,
        items::ItemStack,
        keybinds,
        mod_list::Installed,
        options_ui::Page,
        text::{self, Text},
        ui,
    },
    horizon_mods::{
        Clock, ModKey, Mods, Output, Player, Runs, ServerPlayer, Snapshot, Status, UiScreen,
        UiWidget,
    },
    std::time::Instant,
    winit::{event::MouseButton, keyboard::KeyCode},
=======
    mod_list::{self, Installed},
    text::Text,
>>>>>>> 31f83499b
};
<<<<<<< HEAD
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError},
    },
    time::{Duration, Instant},
};
use winit::{event::MouseButton, keyboard::KeyCode};
||||||| f8988d5a7
=======
use std::path::PathBuf;
>>>>>>> 31f83499b

#[derive(Default)]
pub(super) struct ModsState {
<<<<<<< HEAD
    pub(super) mods: Mods,
    /// Horizon's capability `input`: a claimed item's use while the button is down.
    pub(super) using: Option<super::mod_input::ModUse>,
    /// Horizon's capability `render`'s camera shakes.
    pub(super) shake: super::mod_render::Shake,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
||||||| f8988d5a7
    #[cfg(feature = "mods")]
    pub(super) mods: Mods,
    /// Capability `input`: a claimed item's use while the button is down.
    #[cfg(feature = "mods")]
    pub(super) using: Option<super::mod_input::ModUse>,
    /// Capability `render`'s camera shakes.
    #[cfg(feature = "mods")]
    pub(super) shake: super::mod_render::Shake,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
=======
    /// `--dev-mod PATH`: always on.
>>>>>>> 31f83499b
    pub(super) dev: Vec<PathBuf>,
<<<<<<< HEAD
    /// When dev mods were last checked for a rebuild.
    checked: Option<Instant>,
    /// When the mods stopped on quick code were last looked at ([`Mods::restart_on_full_code`]).
    retried: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    tree: Option<Dispatcher>,
    /// A server mod runs here: what players do is queued for it.
    pub(super) server: bool,
    /// The players server mods saw last tick, for joins and leaves.
    pub(super) players: Vec<ServerPlayer>,
    /// What players did this tick, for server mods at its end.
    pub(super) events: Vec<ModEvent>,
    /// A running server mod has capability `world`: its calls get the
    /// levels' chunks to read.
    pub(super) world: bool,
    /// Blocks server mods set, by dimension, made before that level's next
    /// tick.
    pub(super) block_writes: Vec<(String, horizon_mods::BlockChange)>,
    /// On a server, its tree (`Remote::commands`) with the running mods' commands it lacks.
    remote_tree: Option<Dispatcher>,
    /// Horizon's `command.run` lines for this window's own world, run on the next tick: run at
    /// once, their chat lines would reach the mods while their output is still being shown, and
    /// a mod answering its own lines would recurse.
    commands: Vec<String>,
    /// `command.run-quiet` batches for this window's own world, by mod id,
    /// run on the next tick as `commands` are.
    quiet: Vec<(String, Vec<String>)>,
    /// A Horizon `ui` mod's screen or settings page while it is up (`Screen::Mod`).
    ui_screen: Option<ModScreen>,
    /// The HUD font's advances for Horizon's `render.text-width`, made once.
    font_widths: Arc<[f32]>,
    /// The mod screen open now, `<mod id>:<name>`, over everything else.
    pub(super) screen: Option<String>,
    /// That screen stands in for the creative screen (`screens.replace`).
    pub(super) replacing: bool,
    /// The screen the mods were last told of (`screen-opened` / `screen-closed`).
    pub(super) shown: Option<String>,
    /// The mouse button held on a screen, for drag events.
    pub(super) held: Option<u8>,
    /// The mod screen asked for the on-screen keyboard (`screens.keyboard`).
    pub(super) keyboard: bool,
    /// Where on the mod screen a finger drags instead of scrolling (`mobile.drag-areas`), in
    /// GUI pixels.
    pub(super) drag_areas: Vec<[f32; 4]>,
    /// Where the mod screen draws the open menu's slots (`screens.set-slots`): menu slot and the
    /// top left of its square, in GUI pixels.
    pub(super) screen_slots: Vec<(u32, f32, f32)>,
    /// The world the mods were last told of: None out of a world, Some(server address).
    joined: Option<Option<String>>,
    dimension: Option<String>,
    /// The player's health last tick, for `player-hurt`.
    health: Option<f32>,
    /// The mods' keybinds held now, as `<mod id>:<keybind id>`.
    pub(super) keys_down: Vec<String>,
    /// A mod's button on the game's screen held down by the mouse: mod id, button id.
    pub(super) pressed_button: Option<(String, String)>,
    /// The game's controls for `keys.is-control-down` and `keys.presses`, as last framed.
    controls: Controls,
    /// Controls whose press since the last frame was counted as it came ([`ModsState::press`]).
    counted: [bool; 9],
    /// The mods holding sneak (slot 0) or sprint (slot 1) for the player (`input.hold`), by id.
    holds: Vec<(String, usize)>,
    /// Every block state's name, for `world-read` (made once).
    states: Option<Arc<Vec<String>>>,
    /// What `level.get-block-info` says of every block state, made once for the language its
    /// names are in.
    block_facts: Option<(String, Arc<Vec<BlockFact>>)>,
    /// The blocks needing a tool that the held item (its id here) harvests.
    harvest: Option<(String, Arc<HashSet<Arc<str>>>)>,
    biomes: Option<Arc<Vec<String>>>,
    /// `level.get-spawn-spots`' table for a zombie (`spawn_rules::spot_table`) and every
    /// state's outline top, made once.
    spots: Option<Arc<Spots>>,
    /// The `world-draw` labels' texts as read, by what the mods wrote.
    pub(super) label_texts: std::cell::RefCell<std::collections::HashMap<String, Text>>,
    items: Option<Arc<dyn ItemCatalog>>,
    /// The creative tabs for `creative`, made once for the language their names are in.
    pub(super) creative: Option<(String, Arc<dyn CreativeCatalog>)>,
    /// The recipes for mods (Starling's `recipes`), for the language their names are in.
    pub(super) recipes: Option<(String, Arc<dyn horizon_mods::RecipeCatalog>)>,
    /// Horizon's capability `recipes`'s tables, made once a mod reads them.
    pub(super) horizon_recipes: Option<Arc<horizon_mods::Recipes>>,
    /// Extras that are official mods now, being installed.
    pub(super) moved: super::moved_extras::MovedState,
    /// Playing at the last frame, for `mod_input_frame`.
    pub(super) playing: bool,
    measure: Option<Measure>,
    /// Mods loading on their own thread ([`Game::start_mods`]), and since when.
    loading: Option<(Receiver<Mods>, Instant)>,
    /// The mods folder as last looked at (each mod with its size and time), and when: a mod
    /// copied in, replaced or removed (`cargo horizon-mod push`, a file manager) restarts them.
    folder: Option<Vec<FolderMod>>,
    /// A change seen once: the mods restart when the next look finds the folder the same
    /// (replacing a file on Android's shared storage shows it missing for a moment).
    changed: Option<Vec<FolderMod>>,
    watched: Option<Instant>,
    /// The load running restarts the mods after the folder changed: say so when it ends.
    reloading: bool,
    /// The "Starting mods" note is up.
    announced: bool,
}
impl ModsState {
    /// The mods are starting (on their own thread): a world waits for them to open.
    pub(super) fn starting(&self) -> bool {
        self.loading.is_some()
    }
    /// A press of `control` (one of [`horizon_mods::CONTROLS`]) counted as it comes: a click can
    /// start and end between two frames.
    pub(super) fn press(&mut self, control: &str) {
        if let Some(i) = horizon_mods::CONTROLS.iter().position(|c| *c == control) {
            self.controls[i].1 += 1;
            self.counted[i] = true;
        }
    }
    /// A mod's screen is open in the creative screen's place.
    pub(super) fn standing_in(&self) -> bool {
        self.replacing && self.screen.is_some()
    }
    /// A mod holds sneak (slot 0) or sprint (slot 1): the player's toggle state, as it were.
    pub(super) fn holding(&self, slot: usize) -> bool {
        self.holds.iter().any(|(_, s)| *s == slot)
    }
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        self.tree
            .as_ref()
            .unwrap_or_else(|| vanilla::game_dispatcher())
    }
    /// The server's tree with the mods' commands, apart from the one the server sent
    /// ([`Game::merge_remote_tree`]).
    pub(super) fn remote_dispatcher(&self) -> Option<&Dispatcher> {
        self.remote_tree.as_ref()
    }
    /// Whether a mod draws a HUD panel or a corner panel.
    pub(super) fn has_panels(&self) -> bool {
        self.mods.panels().next().is_some() || self.mods.corner_panels().next().is_some()
    }
||||||| f8988d5a7
    /// When dev mods were last checked for a rebuild.
    #[cfg(feature = "mods")]
    checked: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    #[cfg(feature = "mods")]
    tree: Option<Dispatcher>,
    /// A server mod runs here: what players do is queued for it.
    #[cfg(feature = "mods")]
    pub(super) server: bool,
    /// The players server mods saw last tick, for joins and leaves.
    #[cfg(feature = "mods")]
    pub(super) players: Vec<ServerPlayer>,
    /// What players did this tick, for server mods at its end.
    #[cfg(feature = "mods")]
    pub(super) events: Vec<ModEvent>,
    /// A running server mod has capability `world`: its calls get the
    /// levels' chunks to read.
    #[cfg(feature = "mods")]
    pub(super) world: bool,
    /// Blocks server mods set, by dimension, made before that level's next
    /// tick.
    #[cfg(feature = "mods")]
    pub(super) block_writes: Vec<(String, horizon_mods::BlockChange)>,
    /// `command.run` lines for this window's own world, run on the next tick:
    /// run at once, their chat lines would reach the mods while their output
    /// is still being shown, and a mod answering its own lines would recurse.
    #[cfg(feature = "mods")]
    commands: Vec<String>,
    /// `command.run-quiet` batches for this window's own world, by mod id,
    /// run on the next tick as `commands` are.
    #[cfg(feature = "mods")]
    quiet: Vec<(String, Vec<String>)>,
    /// The server's command tree with the running mods' commands added.
    #[cfg(feature = "mods")]
    remote_tree: Option<Dispatcher>,
    /// A `ui` mod's screen or settings page while it is up.
    #[cfg(feature = "mods")]
    screen: Option<ModScreen>,
    /// The HUD font's advances for `render.text-width`, made once.
    #[cfg(feature = "mods")]
    font_widths: std::sync::Arc<[f32]>,
    /// Capability `recipes`'s tables, made once a mod reads them.
    #[cfg(feature = "mods")]
    pub(super) recipes: Option<std::sync::Arc<horizon_mods::Recipes>>,
    /// Extras that are official mods now, being installed.
    #[cfg(feature = "mods")]
    pub(super) moved: super::moved_extras::MovedState,
    /// Playing at the last frame, for `mod_input_frame`.
    #[cfg(feature = "mods")]
    pub(super) playing: bool,
=======
    /// The WebAssembly mods found that no longer run, for the one-time notice.
    stopped: Vec<String>,
>>>>>>> 31f83499b
}

<<<<<<< HEAD
/// The mappings Horizon's `input.mapping` answers for, with the default key their binding is
/// translated to (none: a mouse button, attack or use).
const MAPPINGS: [(&str, Option<KeyCode>); 9] = [
    ("key.forward", Some(KeyCode::KeyW)),
    ("key.left", Some(KeyCode::KeyA)),
    ("key.back", Some(KeyCode::KeyS)),
    ("key.right", Some(KeyCode::KeyD)),
    ("key.jump", Some(KeyCode::Space)),
    ("key.sneak", Some(KeyCode::ShiftLeft)),
    ("key.sprint", Some(KeyCode::ControlLeft)),
    ("key.attack", None),
    ("key.use", None),
];
/// Characters Horizon's `render.text-width` knows exactly: U+0000 to U+024F.
const FONT_CHARS: u32 = 0x250;

/// A Horizon `ui` mod's own screen, or its settings page from Options > Mods: the widgets the
/// mod pushed, laid out and drawn with the menus' own widgets.
struct ModScreen {
    /// The running mod's index in `mods.mods.entries()`.
    owner: usize,
    /// Opened from Options > Mods: Done and Esc return there.
    settings: bool,
    /// What the mod's screen replaced (Playing, Chat, ...), to restore.
    under: Screen,
    /// One scroll per `list` widget, in widget order.
    scrolls: Vec<ui::Scroll>,
    /// A key row waiting for the key to bind: the mod key's name.
    binding: Option<String>,
||||||| f8988d5a7
/// The mappings `input.mapping` answers for, with the default key their
/// binding is translated to (none: a mouse button, attack or use).
#[cfg(feature = "mods")]
const MAPPINGS: [(&str, Option<winit::keyboard::KeyCode>); 9] = {
    use winit::keyboard::KeyCode as K;
    [
        ("key.forward", Some(K::KeyW)),
        ("key.left", Some(K::KeyA)),
        ("key.back", Some(K::KeyS)),
        ("key.right", Some(K::KeyD)),
        ("key.jump", Some(K::Space)),
        ("key.sneak", Some(K::ShiftLeft)),
        ("key.sprint", Some(K::ControlLeft)),
        ("key.attack", None),
        ("key.use", None),
    ]
};
/// Characters `render.text-width` knows exactly: U+0000 to U+024F.
#[cfg(feature = "mods")]
const FONT_CHARS: u32 = 0x250;

/// A `ui` mod's own screen, or its settings page from Options > Mods: the
/// widgets the mod pushed, laid out and drawn with the menus' own widgets.
#[cfg(feature = "mods")]
struct ModScreen {
    /// The running mod's index in `mods.mods.entries()`.
    owner: usize,
    /// Opened from Options > Mods: Done and Esc return there.
    settings: bool,
    /// What the mod's screen replaced (Playing, Chat, ...), to restore.
    under: Screen,
    /// One scroll per `list` widget, in widget order.
    scrolls: Vec<ui::Scroll>,
    /// A key row waiting for the key to bind: the mod key's name.
    binding: Option<String>,
=======
/// Freeze native component IDs before reading content or opening a connection.
/// With no mod registering any, nothing is installed, which every reader
/// treats as no custom components, and `registries.json` isn't read.
pub(super) fn register_components(root: &std::path::Path) -> horizon_client::Result<()> {
    let mut names = Vec::new();
    Game::native_component_types(&mut names);
    if names.is_empty() {
        return Ok(());
    }
    let path =
        horizon_protocol::registry::Builtin::find(root).ok_or("registries.json not found")?;
    let builtin = horizon_protocol::registry::Builtin::load(&path)?;
    let types = builtin
        .get("minecraft:data_component_type")
        .ok_or("missing data component registry")?;
    let first = i32::try_from(types.names.len())?;
    horizon_protocol::item::custom::install(first, names)
        .map_err(|error| format!("invalid native component registration: {error}"))?;
    Ok(())
>>>>>>> 31f83499b
}
<<<<<<< HEAD
/// A mod in the folder as the watcher saw it: path, size, file id and change time. On Android's
/// shared storage a replaced file's modification time goes back to the old file's a moment
/// later (and the change time's fraction of a second with it), so Unix uses the inode and the
/// change time's seconds, which a replacement sets.
type FolderMod = (PathBuf, u64, u64, i128);

/// How often dev mods are checked for a rebuild.
const RELOAD_CHECK: Duration = Duration::from_secs(1);
/// How long mods may take to start before a note says so (a large one compiling the first time).
const SLOW_START: Duration = Duration::from_millis(700);
/// How far around the player `world-read` mods see blocks.
const LEVEL_CHUNKS: i32 = 2;
||||||| f8988d5a7
impl ModsState {
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.tree {
            return tree;
        }
        vanilla::game_dispatcher()
    }
    /// The server's tree plus the mods' commands, when a mod owns any.
    pub(super) fn remote_dispatcher(&self) -> Option<&Dispatcher> {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.remote_tree {
            return Some(tree);
        }
        None
    }
    /// Whether a mod runs: the chat keeps its lines for them.
    pub(super) fn running(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.is_running() {
            return true;
        }
        false
    }
    /// Whether a mod draws a HUD panel or a corner panel.
    pub(super) fn has_panels(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.panels().next().is_some() || self.mods.corner_panels().next().is_some() {
            return true;
        }
        false
    }
}
/// How often dev mods are checked for a rebuild.
#[cfg(feature = "mods")]
const RELOAD_CHECK: Duration = Duration::from_secs(1);
=======
>>>>>>> 31f83499b

<<<<<<< HEAD
/// A vanilla command name: mods may not take it.
fn vanilla_command(name: &str) -> bool {
    let d = vanilla::game_dispatcher();
    d.nodes[0]
        .children
        .iter()
        .any(|&c| d.nodes[c].name() == name)
||||||| f8988d5a7
/// A vanilla command name: mods may not take it.
#[cfg(feature = "mods")]
fn vanilla_command(name: &str) -> bool {
    let d = vanilla::game_dispatcher();
    d.nodes[0]
        .children
        .iter()
        .any(|&c| d.nodes[c].name() == name)
=======
/// The blocks and items native mods declare in code (`native_blocks`,
/// `native_items`): asked once, so every content root of this run, a pack
/// reload's included, has the same ones.
fn native_declared() -> &'static (
    Vec<horizon_mods::content::Block>,
    Vec<horizon_mods::content::Item>,
) {
    static DECLARED: std::sync::OnceLock<(
        Vec<horizon_mods::content::Block>,
        Vec<horizon_mods::content::Item>,
    )> = std::sync::OnceLock::new();
    DECLARED.get_or_init(|| {
        let (mut blocks, mut items) = (Vec::new(), Vec::new());
        Game::native_blocks(&mut blocks);
        Game::native_items(&mut items);
        (blocks, items)
    })
}
/// The native mods' standalone models (`native_standalone_models`), set once
/// before the resources first load.
pub(super) fn register_standalone_models() {
    let mut names = Vec::new();
    Game::native_standalone_models(&mut names);
    horizon_client::models::register_standalone(names);
>>>>>>> 31f83499b
}

/// A grid of stacks a mod shows in a tooltip (`tooltips.add-grid`).
pub(super) struct TooltipGrid {
    pub(super) columns: usize,
    pub(super) stacks: Vec<Option<ItemStack>>,
    /// ARGB under the slots, 0 for none.
    pub(super) color: u32,
}
impl TooltipGrid {
    pub(super) fn width(&self) -> f32 {
        (self.columns.min(self.stacks.len().max(1)) * 18) as f32
    }
    pub(super) fn height(&self) -> f32 {
        (self.stacks.len().div_ceil(self.columns) * 18) as f32
    }
}

/// The loaded chunks around the player, for `world-read` mods.
struct LevelView {
    chunks: Vec<((i32, i32), Arc<horizon_client::chunk::Chunk>)>,
    states: Arc<Vec<String>>,
    biomes: Arc<Vec<String>>,
    facts: Arc<Vec<BlockFact>>,
    harvest: Arc<HashSet<Arc<str>>>,
    spots: Arc<Spots>,
}
/// What `level.get-spawn-spots` reads of each block state: its `spawn_rules::SPOT_*` bits for a
/// zombie, and how high its outline reaches.
struct Spots {
    bits: Vec<u8>,
    tops: Vec<f32>,
}
/// A block state as `level.get-block-info` tells it, but for the held item.
struct BlockFact {
    /// `minecraft:stone`.
    block: Arc<str>,
    name: String,
    item: Option<String>,
    hardness: f32,
    tool: Option<&'static str>,
    needs_tool: bool,
}
impl LevelView {
    fn chunk(&self, pos: [i32; 3]) -> Option<&horizon_client::chunk::Chunk> {
        let key = (pos[0] >> 4, pos[2] >> 4);
        self.chunks
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, c)| &**c)
    }
}
impl horizon_mods::LevelView for LevelView {
    fn block(&self, pos: [i32; 3]) -> Option<String> {
        let state = self.chunk(pos)?.block(pos[0] & 15, pos[1], pos[2] & 15);
        self.states.get(state.0 as usize).cloned()
    }
    fn biome(&self, pos: [i32; 3]) -> Option<String> {
        let biome = self.chunk(pos)?.biome(pos[0], pos[1], pos[2]);
        self.biomes.get(biome as usize).cloned()
    }
    fn light(&self, pos: [i32; 3]) -> Option<(u8, u8)> {
        let light = self.chunk(pos)?.light(pos[0] & 15, pos[1], pos[2] & 15);
        Some((light & 15, light >> 4))
    }
    fn spawn_spots(&self, from: [i32; 3], to: [i32; 3]) -> horizon_mods::SpawnSpots {
        use horizon_client::mobs::spawn_rules::{
            SPOT_BELOW, SPOT_FEET, SPOT_FLOOR, SPOT_HEAD, SPOT_ROOM,
        };
        let (min, max) = (
            horizon_client::chunk::MIN_Y,
            horizon_client::chunk::MIN_Y + horizon_client::chunk::HEIGHT,
        );
        let mut out = Vec::new();
        for z in from[2]..=to[2] {
            for x in from[0]..=to[0] {
                let Some(c) = self.chunk([x, 0, z]) else {
                    continue;
                };
                let (cx, cz) = (x & 15, z & 15);
                let bits = |y: i32| {
                    if (min..max).contains(&y) {
                        let state = c.block(cx, y, cz).0 as usize;
                        (self.spots.bits.get(state).copied().unwrap_or(0), state)
                    } else {
                        (0, 0)
                    }
                };
                for y in from[1]..=to[1] {
                    let (below, here, above) = (bits(y - 1).0, bits(y), bits(y + 1).0);
                    let ok = below & (SPOT_FLOOR | SPOT_BELOW) == SPOT_FLOOR
                        && here.0 & (SPOT_ROOM | SPOT_FEET) == SPOT_ROOM
                        && above & (SPOT_ROOM | SPOT_HEAD) == SPOT_ROOM;
                    if ok {
                        let top = self.spots.tops.get(here.1).copied().unwrap_or(0.0);
                        out.push(([x, y, z], c.light(cx, y, cz), top));
                    }
                }
            }
        }
        // The lowest first.
        out.sort_by_key(|(pos, _, _)| pos[1]);
        let mut spots = horizon_mods::SpawnSpots::default();
        for (pos, light, top) in out {
            spots.push(pos, light & 15, light >> 4, top);
        }
        spots
    }
    fn block_info(&self, pos: [i32; 3]) -> Option<BlockInfo> {
        let state = self.chunk(pos)?.block(pos[0] & 15, pos[1], pos[2] & 15).0 as usize;
        let fact = self.facts.get(state)?;
        Some(BlockInfo {
            block: self.states.get(state)?.clone(),
            name: fact.name.clone(),
            item: fact.item.clone(),
            hardness: fact.hardness,
            tool: fact.tool.map(str::to_owned),
            needs_tool: fact.needs_tool,
            harvestable: !fact.needs_tool || self.harvest.contains(&fact.block),
        })
    }
}

/// The game's items, for `inventory.get-item-info`.
struct Catalog(Vec<(String, ItemInfo)>);
impl ItemCatalog for Catalog {
    fn info(&self, id: &str) -> Option<ItemInfo> {
        let id = if id.contains(':') {
            id.to_owned()
        } else {
            format!("minecraft:{id}")
        };
        self.0
            .binary_search_by(|(k, _)| k.as_str().cmp(&id))
            .ok()
            .map(|i| self.0[i].1.clone())
    }
}

/// What [`content_root`] tells a loading screen while it works.
pub(super) enum ContentNote<'a> {
    /// A mod's assets coming from Modrinth: its name, and the bytes so far and in all.
    Fetching {
        name: &'a str,
        done: u64,
        total: u64,
    },
    /// The mods' root being built: their ids.
    Building(&'a str),
}

impl ContentNote<'_> {
    /// The loading screen's line for it.
    pub(super) fn text(&self) -> String {
        let mb = |bytes: u64| bytes as f64 / 1_000_000.0;
        match self {
            ContentNote::Fetching { name, done, total } => format!(
                "Downloading {name}'s textures and models from Modrinth: {:.1} of {:.1} MB",
                mb(*done),
                mb(*total)
            ),
            ContentNote::Building(names) => {
                format!("Preparing mods: {names} (the first time takes a minute)")
            }
        }
    }
}

/// `root` with the content mods that are on layered over it
/// (docs/modpacks.md): their blocks, items, data and assets. Read once at
/// start, before the resources load; a mod whose content is refused is left
/// out here and its row on the Mods page says why.
///
/// Mods that add blocks or items are tied to the world (world_content.rs): with
/// `registries`, those are the ones loaded, whatever `enabled` says (a restart
/// into a world brings its set; a pack reload keeps the start's, since the
/// registries can't change). Without, the last world's set (`last_start`). The ones loaded are what worlds compare theirs
/// with. `on_note` hears a mod's assets coming from Modrinth (mod_assets.rs) the first time it is
/// on, and the mods' names before their root is built (none when it is reused).
pub(super) fn content_root(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    dev: &[std::path::PathBuf],
    registries: Option<&std::collections::BTreeSet<String>>,
    on_note: &mut dyn FnMut(ContentNote),
) -> std::path::PathBuf {
    use horizon_client::mod_content::{self, ContentMod};
    use horizon_mods::{Content, Package};
    let here = std::env::current_dir().unwrap_or_default();
    let mut paths = horizon_mods::find(&here.join(mod_list::DIR));
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    let mut known = None;
    let mut mods: Vec<ContentMod> = Vec::new();
<<<<<<< HEAD
    let mut enabled_tied = std::collections::BTreeSet::new();
    let mut installed_tied = std::collections::BTreeSet::new();
    let packages: Vec<_> = paths
        .iter()
        .filter_map(|path| Some((path, Package::read(path).ok()?)))
        .collect();
    for (path, package) in &packages {
||||||| f8988d5a7
    for path in &paths {
        let Ok(package) = Package::read(path) else {
            continue;
        };
=======
    // The first load's list stays (`set_local`): a reload does not hash the files again.
    let announce = !horizon_protocol::mod_list::is_set();
    let mut announced = Vec::new();
    for path in &paths {
        let Ok(package) = Package::read(path) else {
            continue;
        };
>>>>>>> 31f83499b
        let m = &package.manifest;
        if m.capabilities.contains(&Capability::Registries) {
            installed_tied.insert(m.id.clone());
            if enabled.contains(&m.id) || dev.contains(path) {
                enabled_tied.insert(m.id.clone());
            }
        }
    }
    // A start the player made (no restart asked for a set): the last world's mods, so opening it
    // again needs no restart, while Options > Mods turns on what it did then.
    let last = match registries {
        Some(_) => None,
        None => super::world_content::last_start(&enabled_tied),
    };
    if let Some(last) = &last {
        log::info!("Horizon: starting with the last world's mods {last:?}");
    }
    let registries = registries.or(last.as_ref());
    for (path, package) in &packages {
        let path = *path;
        let m = &package.manifest;
        let tied = m.capabilities.contains(&Capability::Registries);
        let on = match registries {
            Some(set) if tied => set.contains(&m.id),
            _ => enabled.contains(&m.id),
        } || dev.contains(path);
        if !on || mods.iter().any(|c| c.id == m.id) {
            continue;
        }
<<<<<<< HEAD
        // Its assets from Modrinth, the first time it is on.
        let cache = horizon_client::logging::cache_dir();
        let fetched = m
            .assets
            .as_ref()
            .map(|a| horizon_mods::fetch::dir(&cache, &a.fetch));
        if let (Some(dir), Some(a)) = (&fetched, &m.assets)
            && !horizon_mods::fetch::ready(dir, &a.fetch)
        {
            log::info!(
                "Horizon: fetching {}'s assets: {} from Modrinth ({} bytes)",
                m.id,
                a.fetch.file,
                a.fetch.size
            );
            let name = m.name.as_str();
            let got = super::mod_assets::get(m, dir, &mut |done, total| {
                on_note(ContentNote::Fetching { name, done, total })
            });
            match got {
                Ok(n) => log::info!("Horizon: {} assets of {} unpacked", n, m.id),
                Err(e) => log::warn!("Horizon: {}'s assets not fetched: {e}", m.id),
            }
        }
        // A mod without registries brings its own GUI sprites at most (its buttons' icons).
        let content = Content::read_with(path, m, fetched.as_deref()).and_then(|content| {
            // The first of two mods owning one namespace stays on.
            let own = (m.id.as_str(), content.namespaces.as_slice());
            match mods
                .iter()
                .find_map(|c| horizon_mods::content::clash((&c.id, &c.content.namespaces), own))
            {
                Some(e) => Err(e),
                None => Ok(content),
||||||| f8988d5a7
        let content = Content::read(path, m).and_then(|content| {
            // The first of two mods owning one namespace stays on.
            let own = (m.id.as_str(), content.namespaces.as_slice());
            match mods
                .iter()
                .find_map(|c| horizon_mods::content::clash((&c.id, &c.content.namespaces), own))
            {
                Some(e) => Err(e),
                None => Ok(content),
=======
        // The items this mod's native code declares may be a file item's crafting remainder.
        let declared: Vec<String> = native_declared()
            .1
            .iter()
            .filter(|i| i.id.split_once(':').is_some_and(|(ns, _)| ns == m.id))
            .map(|i| i.id.clone())
            .collect();
        let content = match Content::read_declaring(path, m, &declared) {
            Ok(content) => content,
            Err(e) => {
                log::warn!("Horizon: mod {} content not loaded: {e}", m.id);
                continue;
>>>>>>> 31f83499b
            }
<<<<<<< HEAD
        });
        match content {
            Ok(content) if content.is_empty() => {}
            Ok(content) => mods.push(ContentMod {
                id: m.id.clone(),
                content,
            }),
||||||| f8988d5a7
        });
        match content {
            Ok(content) => mods.push(ContentMod {
                id: m.id.clone(),
                content,
            }),
=======
        };
        let known = match &mut known {
            Some(types) => types,
            None => match mod_content::known_components(root) {
                Ok(types) => known.insert(types),
                Err(e) => {
                    log::warn!("Horizon: content mods not loaded: {e}");
                    register_mob_effects(&[]);
                    return root.to_owned();
                }
            },
        };
        let candidate = ContentMod {
            id: m.id.clone(),
            content,
        };
        match mod_content::accept_mod(&mut mods, candidate, known) {
            Ok(_) if announce => announced.push(announced_entry(m, path)),
            Ok(_) => {}
>>>>>>> 31f83499b
            Err(e) => log::warn!("Horizon: mod {} content not loaded: {e}", m.id),
        }
    }
<<<<<<< HEAD
    let names = mods
        .iter()
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    super::world_content::set_enabled(enabled_tied);
    super::world_content::set_installed(installed_tied);
    // The world-tied ones: what adds blocks or items.
    let tied: std::collections::BTreeSet<String> = mods
        .iter()
        .filter(|m| !m.content.blocks.is_empty() || !m.content.items.is_empty())
        .map(|m| m.id.clone())
        .collect();
    let built = mod_content::resource_root_with(
        root,
        &mods,
        &horizon_client::logging::cache_dir(),
        &mut || on_note(ContentNote::Building(&names)),
    );
    match built {
||||||| f8988d5a7
    match mod_content::resource_root(root, &mods, &here.join(".horizon-cache")) {
=======
    let (blocks, items) = native_declared();
    if !blocks.is_empty() || !items.is_empty() {
        if known.is_none() {
            match mod_content::known_components(root) {
                Ok(types) => known = Some(types),
                Err(e) => log::warn!("Horizon: native mods' blocks and items not loaded: {e}"),
            }
        }
        if let Some(known) = &known {
            for e in mod_content::accept_declared(&mut mods, blocks, items, known) {
                log::warn!("Horizon: {e}");
            }
            // A file item's remainder named a code-declared item that was refused: that mod
            // alone is left out, not every content mod through the overlay's check.
            for e in mod_content::drop_dangling_remainders(&mut mods) {
                log::warn!("Horizon: {e}");
            }
        }
    }
    match mod_content::resource_root(root, &mods, &here.join(".horizon-cache")) {
>>>>>>> 31f83499b
        Ok(out) => {
            if announce {
                horizon_protocol::mod_list::set_local(announced);
            }
            if out != root {
                log::info!(
                    "Horizon: content from {} mod(s) in {}",
                    mods.len(),
                    out.display()
                );
            }
<<<<<<< HEAD
            super::world_content::set_running(tied);
||||||| f8988d5a7
=======
            register_mob_effects(&mods);
            register_entity_types(&mods);
            register_menu_types(&mods);
>>>>>>> 31f83499b
            out
        }
        Err(e) => {
            log::warn!("Horizon: content mods not loaded: {e}");
<<<<<<< HEAD
            super::world_content::set_running(Default::default());
||||||| f8988d5a7
=======
            register_mob_effects(&[]);
            register_entity_types(&[]);
            register_menu_types(&[]);
>>>>>>> 31f83499b
            root.to_owned()
        }
    }
}

<<<<<<< HEAD
/// A UUID as text, with dashes.
fn uuid_text(id: u128) -> String {
    let h = format!("{id:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Rich text from a mod: a JSON text component when it starts with `{` or `[`, else plain text
/// with legacy `§` codes.
pub(super) fn rich_text(text: &str) -> Text {
    let t = text.trim_start();
    if (t.starts_with('{') || t.starts_with('['))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(t)
    {
        return horizon_client::scoreboard::comp_text(&v);
    }
    if text.contains('§') {
        Text::legacy(text)
    } else {
        Text::literal(text)
||||||| f8988d5a7
/// A build without the `mods` feature: no mod runs.
#[cfg(not(feature = "mods"))]
impl Game<'_> {
    /// Says why the installed and dev mods don't run.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let found = horizon_mods::find(&dir).len() + self.mods.dev.len();
        if found > 0 {
            log::warn!(
                "Horizon: {found} mod(s) not started: this build has no mod runtime (build with --features mods)"
            );
        }
        self.list_native_mods();
    }
    pub(super) fn merge_remote_tree(&mut self) {}
    pub(super) fn sync_mods(&mut self) {}
    pub(super) fn tick_mods(&mut self) {}
    pub(super) fn mods_frame(&mut self, _elapsed: Duration) {}
    pub(super) fn mod_command(&mut self, _command: &str) -> bool {
        false
    }
    pub(super) fn mods_chat(&mut self, _line: &str) {}
    pub(super) fn draw_mod_panels(&self, _batch: &mut HudBatch, _corners: &mut Corners) {}
    pub(super) fn open_mod_screen(&mut self, _owner: usize, _settings: bool) {}
    pub(super) fn close_mod_screen(&mut self, _tell: bool) {}
    pub(super) fn draw_mod_screen(&mut self, _batch: &mut HudBatch) {}
    pub(super) fn mod_screen_click(&mut self, _button: winit::event::MouseButton) {}
    pub(super) fn mod_screen_key(&mut self, _key: winit::keyboard::KeyCode) -> bool {
        false
    }
    pub(super) fn mod_screen_scroll(&mut self, _delta: f32) {}
=======
/// A loaded mod as a server tells joining clients (`horizon_protocol::mod_list`): its manifest's
/// id, version and side, and the SHA-256 of its `.hmod` (none for a folder).
fn announced_entry(
    m: &horizon_mods::Manifest,
    path: &std::path::Path,
) -> horizon_protocol::mod_list::Entry {
    use horizon_mods::Side;
    use horizon_protocol::mod_list;
    use sha2::{Digest, Sha256};
    let sha256 = if path.is_file() {
        std::fs::read(path)
            .ok()
            .map(|bytes| Sha256::digest(bytes).into())
    } else {
        None
    };
    mod_list::Entry {
        id: m.id.clone(),
        version: m.version.clone(),
        sha256,
        side: match m.side {
            Side::Both => mod_list::Side::Both,
            Side::Server => mod_list::Side::Server,
            Side::Client => mod_list::Side::Client,
        },
>>>>>>> 31f83499b
    }
}

<<<<<<< HEAD
impl Game<'_> {
    /// Reads `horizon-mods/` and the dev mods, and starts the ones this game runs that are turned
    /// on, on a thread of their own: compiling a mod takes seconds the first time (its code is
    /// kept in the cache after), which on the render thread would freeze the game and get it
    /// closed on Android. The mods running until then keep running; [`Game::poll_mods`] puts the
    /// new ones in place.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let dev = self.mods.dev.clone();
        let enabled = self.settings.mods.enabled.clone();
        // A native mod of the same id (one built into this game) takes over.
        let mut native = Vec::new();
        self.native_mods(&mut native);
        let native: Vec<String> = native.into_iter().map(|n| n.id).collect();
        let cache = horizon_client::logging::cache_dir().join("compiled-mods");
        let runs = self.mod_runs();
        let load = move || {
            let mut paths = horizon_mods::find(&dir);
            paths.retain(|p| !dev.contains(p));
            paths.extend(dev.iter().cloned());
            // A dev mod is on whatever the list says.
            let dev_ids: Vec<String> = dev
                .iter()
                .filter_map(|p| horizon_mods::Package::read(p).ok())
                .map(|p| p.manifest.id)
                .collect();
            let on = |id: &str| {
                (enabled.contains(id) || dev_ids.iter().any(|d| d == id))
                    && !native.iter().any(|n| n == id)
            };
            Mods::load_cached_for(runs, &paths, on, vanilla_command, Some(&cache))
||||||| f8988d5a7
#[cfg(feature = "mods")]
impl Game<'_> {
    /// Reads `horizon-mods/` and the dev mods, and starts the ones turned on.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let dev = &self.mods.dev;
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !dev.contains(p));
        paths.extend(dev.iter().cloned());
        // A dev mod is on whatever the list says.
        let dev_ids: Vec<String> = dev
            .iter()
            .filter_map(|p| horizon_mods::Package::read(p).ok())
            .map(|p| p.manifest.id)
            .collect();
        // A native mod of the same id (one built into this game) takes over.
        let mut native = Vec::new();
        self.native_mods(&mut native);
        let enabled = &self.settings.mods.enabled;
        let on = |id: &str| {
            (enabled.contains(id) || dev_ids.iter().any(|d| d == id))
                && !native.iter().any(|n| n.id == id)
=======
/// The mods' mob effects join vanilla's (`brewing::effect`), once the root
/// holding their network ids is built. The startup root's effects, none
/// included, stay until restart: a pack reload after a Mods switch builds a
/// root with others.
fn register_mob_effects(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::brewing::{self, Category};
    use horizon_mods::content::EffectCategory;
    let effects = horizon_client::mod_content::mob_effects(mods);
    let ids: Vec<String> = effects.iter().map(|e| e.id.clone()).collect();
    let set = brewing::register_mod_effects(effects.into_iter().map(|e| {
        let category = match e.category {
            EffectCategory::Beneficial => Category::Beneficial,
            EffectCategory::Harmful => Category::Harmful,
            EffectCategory::Neutral => Category::Neutral,
>>>>>>> 31f83499b
        };
<<<<<<< HEAD
        // A server's game has no frames to put them in place in: its server mods start here,
        // before its world ticks.
        if !self.options.execution_role.is_client() {
            let mods = load();
            self.put_mods_in_place(mods);
            return;
        }
        let (send, receive) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("mods".into())
            .spawn(move || {
                let _ = send.send(load());
            });
        match spawned {
            // A load already running is dropped: this one reads the newer list.
            Ok(_) => self.mods.loading = Some((receive, Instant::now())),
            Err(e) => log::error!("Horizon: could not start mods: {e}"),
        }
    }
    /// The mods folder's mods, each with its size and time (a folder mod's by its component).
    fn mods_folder() -> Vec<FolderMod> {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        horizon_mods::find(&dir)
            .into_iter()
            .map(|path| {
                let file = if path.is_dir() {
                    path.join(horizon_mods::manifest::WASM)
                } else {
                    path.clone()
                };
                let Ok(meta) = std::fs::metadata(&file) else {
                    return (path, 0, 0, 0);
                };
                #[cfg(unix)]
                let (id, time) = {
                    use std::os::unix::fs::MetadataExt;
                    // Whole seconds: the fraction too changes back a moment after a copy.
                    (meta.ino(), i128::from(meta.ctime()))
                };
                #[cfg(not(unix))]
                let (id, time) = (
                    0,
                    meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_nanos() as i128),
                );
                (path, meta.len(), id, time)
            })
            .collect()
    }
    /// Once a second, in menus and in a world: the mods start again when the folder changed.
    fn watch_mods_folder(&mut self) {
        if self
            .mods
            .watched
            .is_some_and(|t| t.elapsed() < RELOAD_CHECK)
        {
            return;
        }
        self.mods.watched = Some(Instant::now());
        if self.mods.loading.is_some() {
            return;
        }
        let now = Self::mods_folder();
        match &self.mods.folder {
            Some(was) if *was != now => {
                if self.mods.changed.as_ref() != Some(&now) {
                    self.mods.changed = Some(now);
                    return;
                }
                log::info!("Horizon: the mods folder changed; starting the mods again");
                self.mods.changed = None;
                self.mods.folder = Some(now);
                self.mods.reloading = true;
                self.start_mods();
            }
            Some(_) => self.mods.changed = None,
            None => self.mods.folder = Some(now),
        }
    }
    /// Each frame: the mods a load finished, in place of the ones running.
    pub(super) fn poll_mods(&mut self) {
        self.watch_mods_folder();
        let Some((receive, started)) = &self.mods.loading else {
            return;
        };
        let mods = match receive.try_recv() {
            Ok(mods) => mods,
            Err(TryRecvError::Empty) => {
                if !self.mods.announced && started.elapsed() > SLOW_START {
                    self.mods.announced = true;
                    self.imports.working("Starting mods...".into());
                }
                return;
            }
            Err(TryRecvError::Disconnected) => {
                log::error!("Horizon: the mods' thread stopped");
                self.mods.loading = None;
                return;
            }
        };
        log::info!(
            "Horizon: mods started in {:?}",
            self.mods
                .loading
                .take()
                .map_or(Duration::ZERO, |(_, t)| t.elapsed())
        );
        if std::mem::take(&mut self.mods.reloading) {
            self.imports.say("Mods reloaded".into());
            self.mods.announced = false;
        } else if std::mem::take(&mut self.mods.announced) {
            self.imports.say("Mods started".into());
        }
        self.put_mods_in_place(mods);
    }
    /// Mods a load made, in place of the ones running.
    fn put_mods_in_place(&mut self, mods: Mods) {
        self.mods.mods = mods;
        let chosen = &self.settings.mods;
        self.mods.mods.set_bindings(|id| chosen.keys_of(id));
        self.mods.screen = None;
        // Entries were replaced wholesale: a Horizon screen's owner index is stale.
        self.mods.ui_screen = None;
        // Content mods may have changed the items: Horizon's tables are remade on the next read.
        self.mods.horizon_recipes = None;
        self.mods.joined = None;
        self.release_mod_holds(|_| false);
        self.list_mods();
        self.sync_mod_commands();
        self.register_mod_channels();
        // What they read until the next tick, now: a game paused while they started has none.
        self.refresh_mod_snapshot();
        self.show_mod_output();
||||||| f8988d5a7
        let mods = Mods::load_for(self.mod_runs(), &paths, on, vanilla_command);
        self.mods.mods = mods;
        let chosen = &self.settings.mods;
        self.mods.mods.set_bindings(|id| chosen.keys_of(id));
        // Entries were replaced wholesale: a screen's owner index is stale.
        self.mods.screen = None;
        // Content mods may have changed the items: remade on the next read.
        self.mods.recipes = None;
        self.list_mods();
        self.sync_mod_commands();
        self.show_mod_output();
=======
        (e.id.clone(), e.color, category)
    }));
    if !set
        && brewing::mod_effects()
            .iter()
            .map(|e| e.id)
            .ne(ids.iter().map(String::as_str))
    {
        log::warn!("Horizon: the mods' mob effects changed; restart the game to use them");
>>>>>>> 31f83499b
    }
}

/// the mods' thrown entity types, as their mob effects: set once, from the
/// startup root.
fn register_entity_types(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::projectiles;
    let types = horizon_client::mod_content::entity_types(mods);
    let set = projectiles::register_thrown_types(
        types
            .iter()
            .map(|t| (t.id.clone(), t.item.clone(), t.damage)),
    );
    if !set
        && projectiles::thrown_types()
            .iter()
            .map(|t| t.id)
            .ne(types.iter().map(|t| t.id.as_str()))
    {
        log::warn!("Horizon: the mods' entity types changed; restart the game to use them");
    }
}

/// the mods' menu types, as their entity types: set once, from the startup
/// root.
fn register_menu_types(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::containers;
    let menus = horizon_client::mod_content::menu_types(mods);
    let set = containers::register_mod_menu_types(menus.iter().map(|m| {
        let results = m.results.iter().map(|&r| usize::from(r)).collect();
        (m.id.clone(), m.slots.into(), m.data_slots.into(), results)
    }));
    if !set
        && containers::mod_menu_types()
            .iter()
            .map(|m| m.id)
            .ne(menus.iter().map(|m| m.id.as_str()))
    {
        log::warn!("Horizon: the mods' menu types changed; restart the game to use them");
    }
}

/// What the launcher writes beside a game it built with native mods: their ids under `mods`.
const BUILD_INFO: &str = "native-mods.json";

/// The native mods the launcher built this game with, `id version, id version`, for the crash
/// report (`built`: its [`BUILD_INFO`]). None for a release or Nightly, which have no mods.
fn native_mods_detail(built: &str) -> Option<String> {
    let info: serde_json::Value = serde_json::from_str(built).unwrap_or_default();
    let list = info["mods"]
        .as_array()?
        .iter()
        .map(|m| {
            let text = |key: &str| m[key].as_str().unwrap_or("?").to_owned();
            format!("{} {}", text("id"), text("version"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    (!list.is_empty()).then_some(list)
}

/// A row for each native mod the launcher installed in `dir` (`<id>.hnmod`) that this game wasn't
/// built with (`built`: its [`BUILD_INFO`], empty for a release or Nightly) and that has no row
/// yet. The page said "No mods" before, and players copied the files into `horizon-mods` (#4770).
/// Its toggle is the `mod.<id>` line the launcher's next build follows.
fn not_built(dir: &std::path::Path, built: &str, rows: &[Installed]) -> Vec<Installed> {
    let info: serde_json::Value = serde_json::from_str(built).unwrap_or_default();
    let in_build = |id: &str| {
        info["mods"]
            .as_array()
            .is_some_and(|mods| mods.iter().any(|m| m["id"] == id))
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.retain(|p| p.extension().is_some_and(|e| e == "hnmod"));
    paths.sort();
    let mut out: Vec<Installed> = Vec::new();
    for path in paths {
        let Ok(bundle) = horizon_mods::index::native::Bundle::read(&path) else {
            continue;
        };
        let m = bundle.manifest;
        let listed = rows.iter().chain(&out).any(|row| row.id == m.id);
        if !listed && !in_build(&m.id) {
            out.push(Installed {
                title: format!("{} {}", m.id, m.version),
                id: m.id,
                description: String::new(),
                problem: Some(mod_list::NOT_BUILT.to_owned()),
                dev: false,
                native: None,
            });
        }
    }
    out
}

/// A WebAssembly code mod's row on the Mods page.
const STOPPED: &str = "Doesn't run any more: needs a native port";
/// The player was told once about the WebAssembly mods that stopped (`horizon-mods/<this>`).
const NOTICE_SEEN: &str = ".native-notice";

impl Game<'_> {
    /// Lists the installed mods on the Mods page. Content mods load with the resources
    /// (`content_root`); native ones are built into the game (`native_hooks.rs`). WebAssembly code
    /// mods don't run any more: their rows say so, and the player hears it once in the chat
    /// (`tell_native_now`).
    pub(super) fn start_mods(&mut self) {
        let dir = Self::game_dir().join(mod_list::DIR);
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !self.mods.dev.contains(p));
        paths.extend(self.mods.dev.iter().cloned());
        self.settings.mods.installed = paths
            .iter()
<<<<<<< HEAD
            .map(|e| {
                let file = e.path.file_name().unwrap_or_default().to_string_lossy();
                let problem = match &e.status {
                    Status::Failed(reason) => Some(format!("Stopped: {reason}")),
                    _ => None,
                };
                let Some(m) = &e.manifest else {
                    return Installed {
                        id: String::new(),
                        title: file.into_owned(),
                        description: String::new(),
                        summary: String::new(),
                        config: None,
                        problem,
                        dev: false,
                        has_settings: false,
                        native: None,
                    };
                };
                // What it may do first: the row cuts long descriptions.
                let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                let may = if may.is_empty() {
                    "Asks for nothing.".to_owned()
                } else {
                    format!("May {}.", may.join(", "))
||||||| f8988d5a7
            .map(|e| {
                let file = e.path.file_name().unwrap_or_default().to_string_lossy();
                let problem = match &e.status {
                    Status::Failed(reason) => Some(format!("Stopped: {reason}")),
                    _ => None,
                };
                let Some(m) = &e.manifest else {
                    return Installed {
                        id: String::new(),
                        title: file.into_owned(),
                        description: String::new(),
                        problem,
                        dev: false,
                        has_settings: false,
                        native: None,
                    };
                };
                // What it may do first: the row cuts long descriptions.
                let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                let may = if may.is_empty() {
                    "Asks for nothing.".to_owned()
                } else {
                    format!("May {}.", may.join(", "))
=======
            .map(|path| {
                let file = path.file_name().unwrap_or_default().to_string_lossy();
                let package = match horizon_mods::Package::read(path) {
                    Ok(package) => package,
                    Err(e) => {
                        return Installed {
                            id: String::new(),
                            title: file.into_owned(),
                            description: String::new(),
                            problem: Some(format!("Can't be read: {e}")),
                            dev: false,
                            native: None,
                        };
                    }
>>>>>>> 31f83499b
                };
<<<<<<< HEAD
                // Assets from Modrinth: where from, and why they are not here if they are not.
                let fetched = m.assets.as_ref().map_or_else(String::new, |a| {
                    format!(
                        " Its textures, models and sounds come from Modrinth ({}, {:.1} MB), \
                         downloaded the first time it is on.",
                        a.fetch.file,
                        a.fetch.size as f64 / 1_000_000.0
                    )
                });
                let problem = problem.or_else(|| {
                    super::mod_assets::failed(&m.id)
                        .map(|e| format!("Its assets could not be downloaded: {e}"))
                });
||||||| f8988d5a7
=======
                let m = &package.manifest;
                let problem = package.wasm.is_some().then(|| STOPPED.to_owned());
>>>>>>> 31f83499b
                Installed {
                    id: m.id.clone(),
                    title: format!("{} {}", m.name, m.version),
<<<<<<< HEAD
                    description: format!("{may}{fetched} {}", m.description)
                        .trim()
                        .to_owned(),
                    summary: m.description.clone(),
                    config: self.mods.mods.config_screen(&m.id),
||||||| f8988d5a7
                    description: format!("{may} {}", m.description).trim().to_owned(),
=======
                    description: m.description.clone(),
>>>>>>> 31f83499b
                    problem,
                    dev: self.mods.dev.contains(path),
                    native: None,
                }
            })
            .collect();
        // A mod with a native version built in says so instead, and isn't news.
        self.list_native_mods();
<<<<<<< HEAD
        #[cfg(target_os = "android")]
        self.list_native_builds_row();
    }
    /// The chat's tree after the running mods changed.
    fn sync_mod_commands(&mut self) {
        // In-frame, the window runs server mods too: their names, free text
        // here (the server parses their arguments).
        let server = self.mods.mods.server_commands().map(|c| c.name.as_str());
        let mut names = self.mods.mods.commands().chain(server).peekable();
        self.mods.tree = names
            .peek()
            .is_some()
            .then(|| vanilla::game_dispatcher().with_mod_commands(names));
        self.merge_remote_tree();
    }
    /// On a server, the chat's tree: the server's, with the running mods' commands it does not
    /// have (the server's own win). None while no mod adds one.
    pub(super) fn merge_remote_tree(&mut self) {
        let tree = self.remote.as_ref().and_then(|r| r.commands.as_ref());
        self.mods.remote_tree = tree.and_then(|tree| {
            let mut names = self
                .mods
                .mods
                .commands()
                .filter(|name| tree.find(name).is_none())
                .peekable();
            names
                .peek()
                .is_some()
                .then(|| tree.with_mod_commands(names))
        });
    }
    /// `/command` typed in chat on a server: true when a client mod (native or WebAssembly) owns
    /// it and the server has no command of that name.
    pub(super) fn remote_mod_command(&mut self, command: &str) -> bool {
        let name = command.split_whitespace().next().unwrap_or_default();
        let server_has = self
            .remote
            .as_ref()
            .and_then(|r| r.commands.as_ref())
            .is_some_and(|tree| tree.find(name).is_some());
        !server_has && self.client_mod_command(command)
    }
    /// On a server, tells it the mods' payload channels (`minecraft:register`, NUL-separated),
    /// so its plugins send on them.
    pub(super) fn register_mod_channels(&mut self) {
        let channels = self.mods.mods.channels();
        if channels.is_empty() || self.remote.is_none() {
            return;
        }
        self.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload {
                channel: "minecraft:register".into(),
                data: channels.join("\0").into_bytes(),
            },
        ));
    }
    /// After the options changed: restart the mods if the list of the ones
    /// turned on did.
    pub(super) fn sync_mods(&mut self) {
        let running: Vec<&str> = self
            .mods
            .mods
            .entries()
||||||| f8988d5a7
    }
    /// The chat's tree after the running mods changed.
    fn sync_mod_commands(&mut self) {
        // In-frame, the window runs server mods too: their names, free text
        // here (the server parses their arguments).
        let server = self.mods.mods.server_commands().map(|c| c.name.as_str());
        let mut names = self.mods.mods.commands().chain(server).peekable();
        self.mods.tree = names
            .peek()
            .is_some()
            .then(|| vanilla::game_dispatcher().with_mod_commands(names));
        self.merge_remote_tree();
    }

    /// The server's command tree with the mods' commands, for the chat.
    pub(super) fn merge_remote_tree(&mut self) {
        let names: Vec<String> = self.mods.mods.commands().map(str::to_owned).collect();
        self.mods.remote_tree = self
            .remote
            .as_ref()
            .and_then(|r| r.commands.as_ref())
            .filter(|_| !names.is_empty())
            .map(|t| t.with_mod_commands(names.iter().map(String::as_str)));
    }

    /// After the options changed: restart the mods if the list of the ones
    /// turned on did.
    pub(super) fn sync_mods(&mut self) {
        let running: Vec<&str> = self
            .mods
            .mods
            .entries()
=======
        let built = std::env::current_exe()
            .ok()
            .and_then(|exe| std::fs::read_to_string(exe.with_file_name(BUILD_INFO)).ok())
            .unwrap_or_default();
        let built_mods = native_mods_detail(&built);
        let built_with_mods = built_mods.is_some();
        if let Some(list) = built_mods {
            horizon_client::crash::set_detail("Native Mods", list);
        }
        horizon_client::gfx::native_render::set_mods_built_in(
            built_with_mods
                || self
                    .settings
                    .mods
                    .installed
                    .iter()
                    .any(|m| m.native.is_some()),
        );
        let native_dir = Self::game_dir().join(mod_list::NATIVE_DIR);
        let unbuilt = not_built(&native_dir, &built, &self.settings.mods.installed);
        self.settings.mods.installed.extend(unbuilt);
        let installed = &self.settings.mods.installed;
        self.mods.stopped = installed
>>>>>>> 31f83499b
            .iter()
            .filter(|row| row.problem.as_deref() == Some(STOPPED))
            .map(|row| row.title.clone())
            .collect();
    }
<<<<<<< HEAD
    /// A game stack as mods see it.
    pub(super) fn mod_stack(&self, s: &ItemStack) -> horizon_mods::ItemStack {
        let components = horizon_client::item_data::encode_components(s)
            .map(|c| horizon_client::item_data::canon_tag(&c))
            .unwrap_or_default();
        horizon_mods::ItemStack {
            id: s.item.to_string(),
            count: s.count,
            damage: s.damage,
            max_damage: self
                .res
                .items()
                .items
                .get(&*s.item)
                .map_or(0, |i| i.max_damage),
            name: self.stack_hover_name(s).plain(),
            components,
        }
    }
    /// A mod's stack as a game stack (its saved components read back).
    pub(super) fn game_stack(&self, s: &horizon_mods::ItemStack) -> Option<ItemStack> {
        let components = if s.components.is_empty() {
            String::new()
        } else {
            format!(",components:{}", s.components)
        };
        let snbt = format!(
            "{{id:\"{}\",count:{}{components}}}",
            s.id.replace('"', ""),
            s.count.max(1)
        );
        horizon_client::item_data::parse_stack(&snbt)
            .ok()
            .flatten()
            .or_else(|| {
                self.res
                    .items()
                    .items
                    .contains_key(s.id.as_str())
                    .then(|| ItemStack::new(&s.id, s.count.max(1)))
            })
    }
    /// What mods read, taken now (between ticks), once the world's resources have loaded: mods
    /// cached from an earlier start run before they do, on the loading menus.
    pub(super) fn refresh_mod_snapshot(&mut self) {
        if self.mods.mods.is_running() && self.res.rest_ready() {
            let snapshot = self.mod_snapshot();
            self.mods.mods.set_snapshot(snapshot);
        }
    }
    /// What mods read this tick (or frame, for mods with a `frame` hook): the parts their
    /// capabilities reach, and the server's players as they were at the last tick's end.
    fn mod_snapshot(&mut self) -> Snapshot {
        let players = if self.mods.server {
            self.mods.players.clone()
        } else {
            Vec::new()
        };
        self.mod_snapshot_with(players)
    }
    /// What mods read, with these players on the server.
    pub(super) fn mod_snapshot_with(&mut self, players: Vec<ServerPlayer>) -> Snapshot {
        let mods = &self.mods.mods;
        let (inventory_read, world_read, game_read) = (
            mods.uses(Capability::InventoryRead),
            mods.uses(Capability::WorldRead),
            mods.uses(Capability::GameRead) || mods.uses(Capability::Net),
        );
        let remote = self.remote.as_ref();
        let (name, uuid) = match remote {
            Some(r) => (r.session.name.clone(), uuid_text(r.profile)),
            None => (
                super::chat::PLAYER_NAME.to_owned(),
                self.level
                    .player_uuid
                    .map(|u| {
                        uuid_text(
                            u.iter()
                                .fold(0u128, |acc, &w| acc << 32 | u128::from(w as u32)),
                        )
                    })
                    .unwrap_or_default(),
            ),
        };
        // LivingEntity.getMaxHealth / getArmorValue (attributes read through the live map).
        let (max_health, armor) = self.player.as_mut().map_or((20.0, 0.0), |p| {
            (
                p.attribute("minecraft:max_health").unwrap_or(20.0),
                p.attribute("minecraft:armor")
                    .or_else(|_| p.attribute("armor"))
                    .unwrap_or(0.0),
            )
        });
        let player = self.player.as_ref().map(|p| Player {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
            velocity_x: p.velocity.x,
            velocity_y: p.velocity.y,
            velocity_z: p.velocity.z,
            yaw: self.yaw,
            pitch: self.pitch,
            health: p.health.health(),
            max_health: max_health as f32,
            absorption: p.health.absorption(),
            food: p.health.food.food.max(0) as u32,
            saturation: p.health.food.saturation,
            exhaustion: if remote.is_some() {
                0.0
            } else {
                p.health.food.exhaustion
            },
            air: p.air,
            max_air: 300,
            armor: armor.max(0.0) as u32,
            experience_level: self.experience.level.max(0) as u32,
            experience_progress: self.experience.progress,
            selected_slot: self.selected as u32,
            game_mode: match self.mode_id() {
                1 => GameMode::Creative,
                2 => GameMode::Adventure,
                3 => GameMode::Spectator,
                _ => GameMode::Survival,
            },
            on_ground: p.ground.on_ground,
            sneaking: p.shift_key_down,
            sprinting: p.sprinting,
            swimming: p.swimming,
            flying: p.flying,
            gliding: p.fall_flying,
            dimension: self.dim.id().to_owned(),
            name,
            uuid,
        });
        let in_world = player.is_some();
        let clock = in_world.then(|| Clock {
            game_time: self.world.game_time.max(0) as u64,
            day_time: self.world.day_time.max(0) as u64,
        });
        let effects = if game_read && in_world {
            self.potions
                .effects
                .list
                .iter()
                .map(|e| Effect {
                    id: format!("minecraft:{}", e.id),
                    amplifier: e.amplifier.max(0) as u32,
                    duration: e.duration,
                    ambient: e.ambient,
                    visible: e.visible,
                })
                .collect()
        } else {
            Vec::new()
        };
        let weather = &self.world.weather;
        let world = in_world.then(|| WorldInfo {
            multiplayer: remote.is_some() && self.integrated.is_none(),
            server: remote
                .filter(|_| self.integrated.is_none())
                .map(|r| r.session.address.clone()),
            difficulty: u32::from(self.ui.difficulty),
            hardcore: self.level.hardcore,
            raining: weather.rain_level.1 > 0.2,
            thundering: weather.thunder_level.1 > 0.9,
        });
        let target = (game_read && in_world).then(|| self.mod_target()).flatten();
        let mining = self
            .mining
            .as_ref()
            .filter(|_| game_read && in_world)
            .map(|m| horizon_mods::Mining {
                pos: horizon_mods::BlockPos {
                    x: m.pos[0],
                    y: m.pos[1],
                    z: m.pos[2],
                },
                progress: m.progress.clamp(0.0, 1.0),
            });
        let (gw, gh) = self.gui_size();
        let gui = Gui {
            width: gw as u32,
            height: gh as u32,
            scale: self.renderer.as_ref().map_or(1.0, |r| {
                HudBatch::new([r.config.width, r.config.height]).scale
            }),
        };
        let mut snapshot = Snapshot {
            player,
            effects,
            clock,
            world,
            target,
            mining,
            screen: self.mod_screen_name(),
            gui: Some(gui),
            fps: self.fps.round().max(0.0) as u32,
            keys_down: self.mods.keys_down.clone(),
            touch: self.has_touch(),
            measure: Some(self.mod_measure()),
            gui_size: (gw, gh),
            held: self.mod_held(),
            ..Snapshot::default()
        };
        // Horizon's `render` and `input`: the frame counter, the clicks, the movement and click
        // mappings as the keystroke overlay reads them (keys by the default key their binding is
        // translated to), and the HUD font's advances.
        if self.mods.mods.uses(Capability::Render) || self.mods.mods.uses(Capability::Input) {
            snapshot.frame = self.frame_reading();
            let (cps, mouse) = self.mouse_reading();
            snapshot.cps = cps;
            snapshot.mappings = MAPPINGS
                .iter()
                .map(|&(name, key)| {
                    let label = keybinds::display_name(self.settings.keys.get(name)).plain();
                    let down = match key {
                        Some(k) => self.keys.contains(&k),
                        None => mouse[usize::from(name == "key.use")],
                    };
                    (name, label, down)
                })
                .collect();
            if self.mods.font_widths.is_empty() {
                let atlas = &self.gui.atlas;
                self.mods.font_widths = (0..FONT_CHARS)
                    .map(|c| char::from_u32(c).map_or(6.0, |c| atlas.glyph_cell(c).1))
                    .collect();
            }
            snapshot.font_widths = self.mods.font_widths.clone();
        }
        if inventory_read {
            let mut slots: Vec<Option<horizon_mods::ItemStack>> = self.inventory[..36]
                .iter()
                .map(|s| s.as_ref().map(|s| self.mod_stack(s)))
                .collect();
            // Feet to head, as vanilla's Inventory and the API say (the game keeps head first).
            slots.extend(
                self.armor
                    .iter()
                    .rev()
                    .map(|s| s.as_ref().map(|s| self.mod_stack(s))),
            );
            slots.push(self.offhand.as_ref().map(|s| self.mod_stack(s)));
            snapshot.inventory = slots;
            snapshot.cursor = self.carried.as_ref().map(|s| self.mod_stack(s));
            snapshot.hovered = self.hovered_stack().map(|s| self.mod_stack(&s));
            snapshot.container = self.mod_container();
        }
        let creative = self.mods.mods.uses(Capability::Creative);
        // `creative` checks the stacks it is given against the items.
        if inventory_read || creative {
            snapshot.items = Some(self.mod_items());
        }
        // The tabs answer `creative` and `inventory.get-item-order`.
        if (creative || inventory_read) && self.res.rest_ready() {
            snapshot.creative = Some(self.mod_creative_tabs());
        }
        if self.mods.mods.uses(Capability::InventoryClick) {
            snapshot.menu = self.mod_menu();
        }
        if self.mods.mods.uses(Capability::Recipes) && self.res.rest_ready() {
            snapshot.recipes = Some(self.mod_recipe_catalog());
            snapshot.crafting = self.mod_crafting();
        }
        if self.mods.mods.uses(Capability::Ui) {
            snapshot.window = self.mod_window();
        }
        if world_read && in_world {
            snapshot.level = Some(self.mod_level());
            (snapshot.entities, snapshot.entity_info) = self.mod_entities().into_iter().unzip();
        }
        snapshot.players = players;
        snapshot.view_hit = self.mod_view_hit();
        // Horizon's capability `recipes`: its tables and the book's known displays (nothing while
        // no mod reads them), the screen taking recipes, and creative mode for `give`.
        (snapshot.horizon_recipes, snapshot.known_recipes) = self.mod_recipes();
        snapshot.placing = self.mod_recipe_screen();
        snapshot.creative_mode = self.creative;
        snapshot
    }
    /// What the crosshair is on, for mods.
    fn mod_target(&self) -> Option<Target> {
        use horizon_mods::{BlockHit, BlockPos, Direction, EntityHit};
        let block = self.pick().map(|hit| {
            let state = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
            let face = match hit.face {
                [0, -1, 0] => Direction::Down,
                [0, 1, 0] => Direction::Up,
                [0, 0, -1] => Direction::North,
                [0, 0, 1] => Direction::South,
                [-1, 0, 0] => Direction::West,
                _ => Direction::East,
            };
            let eye = self.eye_position().unwrap_or(hit.point);
            let d = ((hit.point[0] - eye[0]).powi(2)
                + (hit.point[1] - eye[1]).powi(2)
                + (hit.point[2] - eye[2]).powi(2))
            .sqrt();
            (
                BlockHit {
                    pos: BlockPos {
                        x: hit.pos[0],
                        y: hit.pos[1],
                        z: hit.pos[2],
                    },
                    face,
                    block: self.state_string(state),
                },
                d,
            )
        });
        let entity = self.mob_hit().map(|(i, d)| {
            let m = &self.mobs[i];
            (
                EntityHit {
                    id: m.id as i32,
                    kind: m.kind.id.to_string(),
                    x: m.position.x,
                    y: m.position.y,
                    z: m.position.z,
                },
                d,
            )
        });
        match (block, entity) {
            (Some((_, bd)), Some((e, ed))) if ed <= bd => Some(Target::Entity(e)),
            (None, Some((e, _))) => Some(Target::Entity(e)),
            (Some((b, _)), _) => Some(Target::Block(b)),
            (None, None) => None,
        }
    }
    /// The open container's own slots, for mods.
    fn mod_container(&mut self) -> Option<horizon_mods::Container> {
        if self.screen != Screen::Inventory {
            return None;
        }
        let (kind, title) = match self.remote.as_ref().and_then(|r| r.menu.clone()) {
            // The server names it in full already (`minecraft:generic_9x3`).
            Some((_, menu)) => (
                menu,
                self.remote
                    .as_ref()
                    .and_then(|r| r.menu_title.as_ref())
                    .map(|t| t.plain())
                    .unwrap_or_default(),
            ),
            None if self.container.is_some() => ("minecraft:generic_9x3".to_owned(), String::new()),
            None => return None,
        };
        let own: Vec<usize> = self
            .slots()
            .into_iter()
            .filter(|i| (300..500).contains(i))
            .collect();
        let mut slots = Vec::with_capacity(own.len());
        for i in own {
            let stack = self.slot_view(i);
            slots.push(stack.map(|s| self.mod_stack(&s)));
        }
        Some(horizon_mods::Container { kind, title, slots })
    }
    /// The open menu by vanilla's slot ids (`slots.get-menu`): the screen's, or the player's
    /// inventory with none open; None on a screen without slots.
    fn mod_menu(&mut self) -> Option<horizon_mods::Menu> {
        let slotted = matches!(self.screen, Screen::Inventory | Screen::Creative)
            || (self.screen == Screen::Playing && self.player.is_some());
        if !slotted || self.title.is_some() {
            return None;
        }
        let mut read = None;
        // Read, nothing kept.
        self.menu_session(|m, _, _| {
            read = Some((m.kind(), super::remote::menu_items(m)));
            Some(false)
        });
        let (kind, items) = read?;
        let kind = match self.remote.as_ref().and_then(|r| r.menu.as_ref()) {
            Some((_, name)) if self.screen == Screen::Inventory => name.clone(),
            _ => kind.registry_name(),
        };
        Some(horizon_mods::Menu {
            kind,
            slots: items
                .iter()
                .map(|s| s.as_ref().map(|s| self.mod_stack(s)))
                .collect(),
            carried: self.carried.as_ref().map(|s| self.mod_stack(s)),
        })
    }
    /// `slots.click`: a click on the open menu's slot `slot` (vanilla's id), as the mouse's: on
    /// a server predicted and sent, here done on the menu.
    pub(super) fn mod_slot_click(&mut self, slot: i32, button: u8, kind: horizon_mods::ClickKind) {
        use horizon_client::menu::Input;
        use horizon_mods::ClickKind;
        let input = match kind {
            ClickKind::Pickup => Input::Pickup,
            ClickKind::QuickMove => Input::QuickMove,
            ClickKind::Swap => Input::Swap,
            ClickKind::Clone => Input::Clone,
            ClickKind::Throw => Input::Throw,
            ClickKind::PickupAll => Input::PickupAll,
        };
        let button = i32::from(button);
        // The creative screen's clicks are this client's, sent as set slots (as vanilla's).
        if self.remote.is_some() && !self.creative_menu_open() {
            let Ok(slot) = i16::try_from(slot) else {
                return;
            };
            self.remote_menu_clicks(&[(slot, button, input)]);
            return;
        }
        let done = self.menu_session(|m, env, _| Some(m.click(env, slot, button, input).is_ok()));
        if done && let Some(key) = self.container {
            self.container_changed(key);
        }
    }
    /// `slots.drag`: the mouse's drag across slots (AbstractContainerScreen's QUICK_CRAFT
    /// clicks: its start outside the window, each slot, its end).
    pub(super) fn mod_slot_drag(&mut self, slots: &[i32], button: u8) {
        use horizon_client::menu::Input;
        // AbstractContainerMenu.getQuickcraftMask: the phase, and the drag's kind above it.
        let mask = |phase: i32| phase | (i32::from(button) << 2);
        let clicks: Vec<(i32, i32)> = std::iter::once((-999, mask(0)))
            .chain(slots.iter().map(|&s| (s, mask(1))))
            .chain(std::iter::once((-999, mask(2))))
            .collect();
        if self.remote.is_some() && !self.creative_menu_open() {
            let clicks: Vec<(i16, i32, Input)> = clicks
                .iter()
                .filter_map(|&(s, b)| Some((i16::try_from(s).ok()?, b, Input::QuickCraft)))
                .collect();
            self.remote_menu_clicks(&clicks);
            return;
        }
        let done = self.menu_session(|m, env, _| {
            let mut done = false;
            for &(slot, b) in &clicks {
                done |= m.click(env, slot, b, Input::QuickCraft).is_ok();
            }
            Some(done)
        });
        if done && let Some(key) = self.container {
            self.container_changed(key);
        }
    }
    /// The stack under the cursor in a screen.
    fn hovered_stack(&mut self) -> Option<ItemStack> {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative)
            || self.mods.screen.is_some()
        {
            return None;
        }
        let renderer = self.renderer.as_ref()?;
        let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        let slot = self.slot_at(&batch, self.cursor.0, self.cursor.1)?;
        self.slot_view(slot)
    }
    fn mod_items(&mut self) -> Arc<dyn ItemCatalog> {
        if let Some(items) = &self.mods.items {
            return items.clone();
        }
        let mut list: Vec<(String, ItemInfo)> = self
            .res
            .items()
            .items
            .iter()
            .map(|(id, info)| {
                let food = info.food.map(|(nutrition, saturation)| horizon_mods::Food {
                    nutrition,
                    saturation,
                    can_always_eat: info.can_always_eat,
                });
                (
                    id.to_string(),
                    ItemInfo {
                        id: id.to_string(),
                        name: self.item_display_name(id).plain(),
                        max_stack_size: info.max_stack.max(1),
                        max_damage: info.max_damage,
                        food,
                    },
                )
            })
            .collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        let items: Arc<dyn ItemCatalog> = Arc::new(Catalog(list));
        self.mods.items = Some(items.clone());
        items
    }
    fn mod_measure(&mut self) -> Measure {
        if let Some(m) = &self.mods.measure {
            return m.clone();
        }
        // The font is the render thread's: mods measure with a copy of its advances
        // (Basic Multilingual Plane), bold adding a pixel as Font.width does.
        let font = &self.gui.atlas.font;
        let advances: Vec<f32> = (0..0x3000u32)
            .map(|c| char::from_u32(c).map_or(0.0, |ch| font.width(ch.encode_utf8(&mut [0; 4]))))
            .collect();
        let measure: Measure = Arc::new(move |s: &str| {
            rich_text(s)
                .resolve_legacy()
                .chars()
                .map(|(ch, style)| {
                    let advance = advances.get(ch as usize).copied().unwrap_or(6.0);
                    advance + if style.is_bold() { 1.0 } else { 0.0 }
                })
                .sum()
        });
        self.mods.measure = Some(measure.clone());
        measure
    }
    fn mod_level(&mut self) -> Arc<dyn horizon_mods::LevelView> {
        let states = match &self.mods.states {
            Some(s) => s.clone(),
            None => {
                let registry = &self.pack.assets.registry;
                let names: Vec<String> = (0..registry.len())
                    .map(|i| self.state_string(horizon_world::registry::BlockStateId(i as u32)))
                    .collect();
                let names = Arc::new(names);
                self.mods.states = Some(names.clone());
                names
            }
        };
        let biomes = self
            .mods
            .biomes
            .get_or_insert_with(|| {
                Arc::new(
                    self.res
                        .biomes()
                        .list
                        .iter()
                        .map(|b| b.name.clone())
                        .collect(),
                )
            })
            .clone();
        let (cx, cz) = self.player.as_ref().map_or((0, 0), |p| {
            (
                (p.position.x.floor() as i32) >> 4,
                (p.position.z.floor() as i32) >> 4,
            )
        });
        let mut chunks = Vec::new();
        for x in cx - LEVEL_CHUNKS..=cx + LEVEL_CHUNKS {
            for z in cz - LEVEL_CHUNKS..=cz + LEVEL_CHUNKS {
                if let Some(c) = self.world.chunks.get(&(x, z)) {
                    chunks.push(((x, z), c.clone()));
                }
            }
        }
        Arc::new(LevelView {
            chunks,
            states,
            biomes,
            facts: self.mod_block_facts(),
            harvest: self.mod_harvest(),
            spots: self.mod_spots(),
        })
    }
    /// `level.get-spawn-spots`' table, made the first time a `world-read` mod runs.
    fn mod_spots(&mut self) -> Arc<Spots> {
        if let Some(spots) = &self.mods.spots {
            return spots.clone();
        }
        let facts = self.res.facts();
        let (width, height) = self
            .res
            .mob_types()
            .get("minecraft:zombie")
            .map_or((0.6, 1.95), |t| (t.width as f32, t.height as f32));
        let bits = horizon_client::mobs::spawn_rules::spot_table(
            horizon_client::mobs::Species::Zombie,
            width,
            height,
            facts,
            self.res.mob_data(),
            self.collision,
            &self.res.items().block_tags,
        );
        let tops = (0..bits.len())
            .map(|i| {
                let f = facts.get(horizon_world::registry::BlockStateId(i as u32));
                f.outline.iter().map(|b| b[4] as f32).fold(0.0, f32::max)
            })
            .collect();
        let spots = Arc::new(Spots { bits, tops });
        self.mods.spots = Some(spots.clone());
        spots
    }
    /// Every block state's name, item, hardness and tool, in the player's language.
    fn mod_block_facts(&mut self) -> Arc<Vec<BlockFact>> {
        let language = horizon_client::lang::code();
        if let Some((made_in, facts)) = &self.mods.block_facts
            && *made_in == language
        {
            return facts.clone();
        }
        let tags = &self.res.items().block_tags;
        let tool = |block: &str| {
            ["pickaxe", "axe", "shovel", "hoe"]
                .into_iter()
                .find(|tool| {
                    tags.get(&format!("minecraft:mineable/{tool}"))
                        .is_some_and(|set| set.contains(block))
                })
        };
        let registry = &self.pack.assets.registry;
        let facts: Vec<BlockFact> = (0..registry.len())
            .map(|i| {
                let f = self
                    .res
                    .facts()
                    .get(horizon_world::registry::BlockStateId(i as u32));
                let (namespace, path) = f.name.split_once(':').unwrap_or(("minecraft", &f.name));
                BlockFact {
                    block: f.name.clone(),
                    name: horizon_client::lang::get(&format!("block.{namespace}.{path}")),
                    item: (&*f.item != "minecraft:air").then(|| f.item.to_string()),
                    hardness: f.hardness,
                    tool: tool(&f.name),
                    needs_tool: f.requires_tool,
                }
            })
            .collect();
        let facts = Arc::new(facts);
        self.mods.block_facts = Some((language, facts.clone()));
        facts
    }
    /// The blocks needing a tool that the held item would make drop (`Tool.isCorrectForDrops`),
    /// made again when the held item changes.
    fn mod_harvest(&mut self) -> Arc<HashSet<Arc<str>>> {
        let held = self.held().map_or_else(String::new, |s| s.item.to_string());
        if let Some((item, set)) = &self.mods.harvest
            && *item == held
        {
            return set.clone();
        }
        let facts = self.mod_block_facts();
        let items = self.res.items();
        let stack = self.held();
        let set: HashSet<Arc<str>> = facts
            .iter()
            .filter(|f| f.needs_tool && items.correct_for_drops(stack, &f.block))
            .map(|f| f.block.clone())
            .collect();
        let set = Arc::new(set);
        self.mods.harvest = Some((held, set.clone()));
        set
    }
    /// Entities near the player, nearest first, for mods, with what the game shows of them.
    fn mod_entities(&self) -> Vec<(Entity, EntityInfo)> {
        let Some(p) = self.player.as_ref() else {
            return Vec::new();
        };
        let mut list: Vec<(f64, (Entity, EntityInfo))> = self
            .mobs
            .iter()
            .map(|m| {
                let d = m.position.distance_squared(p.position);
                let entity = Entity {
                    id: m.id as i32,
                    kind: m.kind.id.to_string(),
                    x: m.position.x,
                    y: m.position.y,
                    z: m.position.z,
                    yaw: m.yaw,
                    health: Some(m.health),
                    name: m.custom_name.clone(),
                };
                let info = EntityInfo {
                    id: m.id as i32,
                    kind: m.kind.id.to_string(),
                    name: m.custom_name.clone().unwrap_or_else(|| {
                        horizon_client::lang::get(&format!(
                            "entity.{}",
                            m.kind.id.replace(':', ".")
                        ))
                    }),
                    health: Some(m.health),
                    max_health: Some(finalize::max_health(m)),
                    armor: finalize::value(m, finalize::Attr::Armor) as f32,
                    baby: m.is_baby(),
                };
                (d, (entity, info))
            })
            .filter(|(d, _)| *d <= 64.0 * 64.0)
            .collect();
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        list.truncate(128);
        list.into_iter().map(|(_, e)| e).collect()
    }
    /// The first block along the player's view within
    /// [`horizon_mods::RAYCAST_MAX`] and its distance from the eye
    /// (`game.raycast`): through the loaded chunks as the crosshair's ray
    /// goes (fluids pass), then across the distant terrain's ground heights,
    /// a block at a time.
    fn mod_view_hit(&self) -> Option<(f32, horizon_mods::Hit)> {
        if !self.mods.mods.wants_raycast() {
            return None;
        }
        let (eye, dir) = self.aim()?;
        let reach = f64::from(horizon_mods::RAYCAST_MAX);
        let (pos, point, state) = match self.clip_ray(eye, dir, reach) {
            Some(hit) => (
                hit.pos,
                hit.point,
                self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]),
            ),
            None => self.lod_ray(eye, dir, reach)?,
        };
        let distance = (0..3).map(|i| (point[i] - eye[i]).powi(2)).sum::<f64>();
        let hit = horizon_mods::Hit {
            block_x: pos[0],
            block_y: pos[1],
            block_z: pos[2],
            x: point[0],
            y: point[1],
            z: point[2],
            block: self.res.facts().get(state).name.to_string(),
        };
        Some((distance.sqrt() as f32, hit))
    }
    /// Past the resident chunks: the view walked across the distant
    /// terrain's ground heights (`Lod::ground`), a block at a time. Columns in
    /// loaded chunks are left to `clip_ray`, which has already missed them.
    fn lod_ray(
        &self,
        eye: [f64; 3],
        dir: [f64; 3],
        reach: f64,
    ) -> Option<([i32; 3], [f64; 3], horizon_world::registry::BlockStateId)> {
        let lod = self.lod.as_ref()?;
        let mut t = 0.0;
        while t <= reach {
            let p = [0, 1, 2].map(|i| eye[i] + dir[i] * t);
            let (x, z) = (p[0].floor() as i32, p[2].floor() as i32);
            if !self.world.loaded(x, z)
                && let Some((height, state)) = lod.ground(x, z)
                && p[1] <= f64::from(height)
            {
                return Some(([x, height - 1, z], [p[0], f64::from(height), p[2]], state));
            }
            t += 1.0;
        }
        None
    }
    /// A mod's `command.run-quiet` lines (vanilla's commands only, as
    /// `command.run`): to a server that runs them quietly as one payload
    /// (`horizon_protocol::quiet`), to any other as chat commands, or for this
    /// window's own world next tick.
    fn mod_quiet_commands(&mut self, mod_id: String, lines: Vec<String>) {
        let lines: Vec<String> = lines
            .into_iter()
            .filter(|l| vanilla_command(l.split(' ').next().unwrap_or_default()))
            .collect();
        match self.remote.as_ref().map(|r| r.quiet_commands) {
            Some(true) => {
                for data in horizon_protocol::quiet::encode(&mod_id, &lines) {
                    self.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
                        horizon_protocol::packets::configuration::CustomPayload {
                            channel: horizon_protocol::quiet::CHANNEL.into(),
                            data,
                        },
                    ));
                }
            }
            // The server checks the player's permissions; its feedback shows.
            Some(false) => {
                for line in lines {
                    self.send_chat(&format!("/{line}"));
                }
            }
            None => self.mods.quiet.push((mod_id, lines)),
        }
    }
    /// A quiet batch in this window's own world, as a server runs one
    /// (game/lan_quiet.rs): each line as the player typing it, no chat, a
    /// chat message refused; one line in the log.
    fn run_quiet_batch(&mut self, mod_id: &str, lines: &[String]) {
        let permission = self.ui.registries.permission;
        let mut failed = 0;
        for line in lines {
            let ok = self.signable_arguments(permission, line).is_empty()
                && self.run_command_quietly(line);
            failed += usize::from(!ok);
        }
        log::info!(
            "[mod {mod_id}] ran {} quiet command(s): {failed} failed",
            lines.len()
        );
    }
    /// A game tick: the snapshot, the connection and damage events, then the tick hook.
    pub(super) fn tick_mods(&mut self) {
        // Here, at the player's level: shown inside a console call (a control socket command set
        // the hook off), they would run as the console.
        for line in std::mem::take(&mut self.mods.commands) {
            self.run_command(&line);
        }
        for (mod_id, lines) in std::mem::take(&mut self.mods.quiet) {
            self.run_quiet_batch(&mod_id, &lines);
        }
        // A mod that stopped lets go of what it held.
        if !self.mods.holds.is_empty() {
            let running: Vec<String> = self
                .mods
                .mods
                .entries()
                .iter()
                .filter(|e| e.status == Status::Running)
                .filter_map(|e| e.manifest.as_ref().map(|m| m.id.clone()))
                .collect();
            self.release_mod_holds(|id| running.iter().any(|r| r == id));
        }
        if !self.mods.mods.is_running() {
||||||| f8988d5a7
    /// What mods read this tick (or frame, for mods with a `frame` hook): the
    /// server's players as they were at the last tick's end.
    fn mod_snapshot(&mut self) -> Snapshot {
        let players = if self.mods.server {
            self.mods.players.clone()
        } else {
            Vec::new()
        };
        self.mod_snapshot_with(players)
    }
    /// What mods read, with these players on the server.
    pub(super) fn mod_snapshot_with(&mut self, players: Vec<ServerPlayer>) -> Snapshot {
        let player = self.player.as_ref().map(|p| Player {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
            yaw: self.yaw,
            pitch: self.pitch,
            health: p.health.health(),
            food: p.health.food.food.max(0) as u32,
            dimension: self.dim.id().to_owned(),
        });
        let clock = player.is_some().then(|| Clock {
            game_time: self.world.game_time.max(0) as u64,
            day_time: self.world.day_time.max(0) as u64,
        });
        let (cps, mouse) = self.mouse_reading();
        // The movement and click mappings, as the keystroke overlay reads them:
        // keys by the default key their binding is translated to.
        let mappings = MAPPINGS
            .iter()
            .map(|&(name, key)| {
                let label = keybinds::display_name(self.settings.keys.get(name)).plain();
                let down = match key {
                    Some(k) => self.keys.contains(&k),
                    None => mouse[usize::from(name == "key.use")],
                };
                (name, label, down)
            })
            .collect();
        if self.mods.font_widths.is_empty() {
            let atlas = &self.gui.atlas;
            self.mods.font_widths = (0..FONT_CHARS)
                .map(|c| char::from_u32(c).map_or(6.0, |c| atlas.glyph_cell(c).1))
                .collect();
        }
        let (recipes, known_recipes) = self.mod_recipes();
        Snapshot {
            player,
            clock,
            gui_size: self.gui_size(),
            held: self.mod_held(),
            frame: self.frame_reading(),
            mappings,
            cps,
            font_widths: self.mods.font_widths.clone(),
            players,
            view_hit: self.mod_view_hit(),
            recipes,
            known_recipes,
            placing: self.mod_recipe_screen(),
            creative: self.creative,
        }
    }
    /// The first block along the player's view within
    /// [`horizon_mods::RAYCAST_MAX`] and its distance from the eye
    /// (`game.raycast`): through the loaded chunks as the crosshair's ray
    /// goes (fluids pass), then across the distant terrain's ground heights,
    /// a block at a time.
    fn mod_view_hit(&self) -> Option<(f32, horizon_mods::Hit)> {
        if !self.mods.mods.wants_raycast() {
            return None;
        }
        let (eye, dir) = self.aim()?;
        let reach = f64::from(horizon_mods::RAYCAST_MAX);
        let (pos, point, state) = match self.clip_ray(eye, dir, reach) {
            Some(hit) => (
                hit.pos,
                hit.point,
                self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]),
            ),
            None => self.lod_ray(eye, dir, reach)?,
        };
        let distance = (0..3).map(|i| (point[i] - eye[i]).powi(2)).sum::<f64>();
        let hit = horizon_mods::Hit {
            block_x: pos[0],
            block_y: pos[1],
            block_z: pos[2],
            x: point[0],
            y: point[1],
            z: point[2],
            block: self.res.facts().get(state).name.to_string(),
        };
        Some((distance.sqrt() as f32, hit))
    }
    /// Past the resident chunks: the view walked across the distant
    /// terrain's ground heights (`Lod::ground`), a block at a time. Columns in
    /// loaded chunks are left to `clip_ray`, which has already missed them.
    fn lod_ray(
        &self,
        eye: [f64; 3],
        dir: [f64; 3],
        reach: f64,
    ) -> Option<([i32; 3], [f64; 3], horizon_world::registry::BlockStateId)> {
        let lod = self.lod.as_ref()?;
        let mut t = 0.0;
        while t <= reach {
            let p = [0, 1, 2].map(|i| eye[i] + dir[i] * t);
            let (x, z) = (p[0].floor() as i32, p[2].floor() as i32);
            if !self.world.loaded(x, z)
                && let Some((height, state)) = lod.ground(x, z)
                && p[1] <= f64::from(height)
            {
                return Some(([x, height - 1, z], [p[0], f64::from(height), p[2]], state));
            }
            t += 1.0;
        }
        None
    }
    /// A mod's `command.run-quiet` lines (vanilla's commands only, as
    /// `command.run`): to a server that runs them quietly as one payload
    /// (`horizon_protocol::quiet`), to any other as chat commands, or for this
    /// window's own world next tick.
    fn mod_quiet_commands(&mut self, mod_id: String, lines: Vec<String>) {
        let lines: Vec<String> = lines
            .into_iter()
            .filter(|l| vanilla_command(l.split(' ').next().unwrap_or_default()))
            .collect();
        match self.remote.as_ref().map(|r| r.quiet_commands) {
            Some(true) => {
                for data in horizon_protocol::quiet::encode(&mod_id, &lines) {
                    self.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
                        horizon_protocol::packets::configuration::CustomPayload {
                            channel: horizon_protocol::quiet::CHANNEL.into(),
                            data,
                        },
                    ));
                }
            }
            // The server checks the player's permissions; its feedback shows.
            Some(false) => {
                for line in lines {
                    self.send_chat(&format!("/{line}"));
                }
            }
            None => self.mods.quiet.push((mod_id, lines)),
        }
    }
    /// A quiet batch in this window's own world, as a server runs one
    /// (game/lan_quiet.rs): each line as the player typing it, no chat, a
    /// chat message refused; one line in the log.
    fn run_quiet_batch(&mut self, mod_id: &str, lines: &[String]) {
        let permission = self.ui.registries.permission;
        let mut failed = 0;
        for line in lines {
            let ok = self.signable_arguments(permission, line).is_empty()
                && self.run_command_quietly(line);
            failed += usize::from(!ok);
        }
        log::info!(
            "[mod {mod_id}] ran {} quiet command(s): {failed} failed",
            lines.len()
        );
    }
    /// A game tick.
    pub(super) fn tick_mods(&mut self) {
        // Here, at the player's level: shown inside a console call (a control
        // socket command set the hook off), they would run as the console.
        for line in std::mem::take(&mut self.mods.commands) {
            self.run_command(&line);
        }
        for (mod_id, lines) in std::mem::take(&mut self.mods.quiet) {
            self.run_quiet_batch(&mod_id, &lines);
        }
        if !self.mods.mods.is_running() {
=======
    /// Once, in the first world after the update: which WebAssembly mods stopped.
    pub(super) fn tell_native_now(&mut self) {
        if self.mods.stopped.is_empty() || self.player.is_none() {
>>>>>>> 31f83499b
            return;
        }
<<<<<<< HEAD
        self.tick_mod_use();
        let snapshot = self.mod_snapshot();
        let joined = snapshot
            .player
            .as_ref()
            .map(|_| snapshot.world.as_ref().and_then(|w| w.server.clone()));
        let dimension = snapshot.player.as_ref().map(|p| p.dimension.clone());
        let health = snapshot.player.as_ref().map(|p| p.health);
        self.mods.mods.set_snapshot(snapshot);
        // ClientPlayConnectionEvents: the world entered or left, a dimension changed.
        if joined != self.mods.joined {
            if self.mods.joined.is_some() {
                self.mods.mods.left();
            }
            if let Some(server) = &joined {
                self.mods.mods.joined(server.as_deref());
            }
            self.mods.joined = joined;
            self.mods.dimension = None;
        }
        if dimension.is_some() && dimension != self.mods.dimension {
            if self.mods.dimension.is_some()
                && let Some(d) = &dimension
            {
                self.mods.mods.dimension_changed(d);
            }
            self.mods.dimension = dimension;
        }
        if let (Some(before), Some(now)) = (self.mods.health, health)
            && now < before
        {
            self.mods.mods.player_hurt(before - now);
        }
        self.mods.health = health;
        self.mods.mods.tick(self.ticks);
        self.show_mod_output();
    }
    /// Restarts the dev mods rebuilt since the last check, once a second.
    pub(super) fn reload_dev_mods(&mut self) {
        if self.mods.dev.is_empty()
            || self
                .mods
                .checked
                .is_some_and(|t| t.elapsed() < RELOAD_CHECK)
        {
||||||| f8988d5a7
        self.tick_mod_use();
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        self.mods.mods.tick(self.ticks);
        self.show_mod_output();
    }
    /// Restarts the dev mods rebuilt since the last check, once a second.
    pub(super) fn reload_dev_mods(&mut self) {
        if self.mods.dev.is_empty()
            || self
                .mods
                .checked
                .is_some_and(|t| t.elapsed() < RELOAD_CHECK)
        {
=======
        let names = std::mem::take(&mut self.mods.stopped).join(", ");
        let seen = Self::game_dir().join(mod_list::DIR).join(NOTICE_SEEN);
        if seen.exists() {
>>>>>>> 31f83499b
            return;
        }
<<<<<<< HEAD
        self.mods.checked = Some(Instant::now());
        if self.mods.mods.reload_changed(vanilla_command) {
            let chosen = &self.settings.mods;
            self.mods.mods.set_bindings(|id| chosen.keys_of(id));
            self.list_mods();
            self.sync_mod_commands();
            if self.options.execution_role.is_client() {
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            } else {
                log::info!("Horizon: mods reloaded");
            }
        }
    }
    /// Once per frame: the frame hook, dev mods' rebuilds, and the mods stopped on quick code.
    pub(super) fn mods_frame(&mut self, elapsed: Duration) {
        self.reload_dev_mods();
        // A mod stopped on the quick compiler's code starts again on full code once it is kept.
        if self
            .mods
            .retried
            .is_none_or(|t| t.elapsed() >= RELOAD_CHECK)
        {
            self.mods.retried = Some(Instant::now());
            if self.mods.mods.restart_on_full_code(vanilla_command) {
                self.list_mods();
                self.sync_mod_commands();
            }
        }
        self.swap_creative_screen();
        if !self.mods.mods.is_running() {
            return;
        }
        self.mod_input_frame();
        self.mods_screen_events();
        self.frame_mod_controls();
        // Horizon's `input.mapping` reads the keys as of this frame for a mod with a frame hook.
        if self.mods.mods.wants_frames()
            && (self.mods.mods.uses(Capability::Render) || self.mods.mods.uses(Capability::Input))
            && self.res.rest_ready()
        {
            let snapshot = self.mod_snapshot();
            self.mods.mods.set_snapshot(snapshot);
        }
        self.mods.mods.frame(elapsed.as_secs_f32());
        self.show_mod_output();
    }
    /// The game's controls for mods this frame: held (behind a screen, none), and a press counted
    /// each time one goes down.
    fn frame_mod_controls(&mut self) {
        let playing = self.screen == Screen::Playing && self.title.is_none();
        let key = |k| playing && self.keys.contains(&k);
        let [left, forward] = self.pad_move().filter(|_| playing).unwrap_or([0.0, 0.0]);
        let held = [
            key(KeyCode::KeyW) || forward > 0.3,
            key(KeyCode::KeyS) || forward < -0.3,
            key(KeyCode::KeyA) || left > 0.3,
            key(KeyCode::KeyD) || left < -0.3,
            key(KeyCode::Space),
            key(KeyCode::ShiftLeft),
            key(KeyCode::ControlLeft),
            playing && self.left,
            playing && self.right,
        ];
        let counted = std::mem::take(&mut self.mods.counted);
        for ((control, down), counted) in self.mods.controls.iter_mut().zip(held).zip(counted) {
            if down && !control.0 && !counted {
                control.1 += 1;
            }
            control.0 = down;
        }
        self.mods.mods.set_controls(self.mods.controls);
    }
    /// Lets go of what mods hold for the player but those `keep` says (when they stop or reload).
    pub(super) fn release_mod_holds(&mut self, keep: impl Fn(&str) -> bool) {
        let before = self.mods.holds.len();
        self.mods.holds.retain(|(id, _)| keep(id));
        if self.mods.holds.len() != before {
            self.sync_toggle(0);
            self.sync_toggle(1);
        }
    }
    /// `/command` typed in chat: true when a mod owns it.
    pub(super) fn mod_command(&mut self, command: &str) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        // The command may follow a teleport in the same tick.
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        if !self.mods.mods.command(command) {
            return false;
        }
        self.show_mod_output();
        true
    }
    /// A chat line the player receives: what the mods make of it (None: hidden).
    pub(super) fn mods_chat(&mut self, line: Text) -> Option<Text> {
        if !self.mods.mods.is_running() {
            return Some(line);
        }
        let component = line.component().to_string();
        let verdict = self.mods.mods.chat(&line.plain(), &component);
        self.show_mod_output();
        match verdict {
            ChatVerdict::Keep => Some(line),
            ChatVerdict::Hide => None,
            ChatVerdict::Replace(text) => Some(rich_text(&text)),
        }
    }
    /// A line the player is about to send: what the mods make of it (None: cancelled).
    pub(super) fn mods_chat_sending(&mut self, message: &str) -> Option<String> {
        if !self.mods.mods.is_running() {
            return Some(message.to_owned());
        }
        let out = self.mods.mods.chat_sending(message);
        self.show_mod_output();
        out
    }
    /// A custom payload from the server, for the mods on its channel.
    pub(super) fn mods_payload(&mut self, channel: &str, data: &[u8]) {
        if !self.mods.mods.is_running() {
            return;
        }
        self.mods.mods.payload(channel, data);
        self.show_mod_output();
    }
    /// The lines and the grids of stacks mods add to `stack`'s tooltip, and whether a grid
    /// stands instead of the game's lines of a container's contents.
    pub(super) fn mods_tooltip_grids(
        &mut self,
        stack: &ItemStack,
    ) -> (Vec<Text>, Vec<TooltipGrid>, bool) {
        if !self.mods.mods.is_running() || !self.mods.mods.wants(|h| h.tooltips) {
            return (Vec::new(), Vec::new(), false);
        }
        let stack = self.mod_stack(stack);
        let (lines, grids, instead) = self.mods.mods.tooltip_grids(&stack);
        self.show_mod_output();
        let grids = grids
            .into_iter()
            .map(|g| TooltipGrid {
                columns: g.columns.clamp(1, 16) as usize,
                stacks: g
                    .stacks
                    .iter()
                    .map(|s| s.as_ref().and_then(|s| self.game_stack(s)))
                    .collect(),
                color: g.color,
            })
            .collect();
        (lines.iter().map(|l| rich_text(l)).collect(), grids, instead)
    }
    /// A key or mouse button for the mods' keybinds (while playing): pressed or released.
    pub(super) fn mods_key(&mut self, name: &str, pressed: bool) {
        if !self.mods.mods.is_running() {
            return;
        }
        let binds = self.mods.mods.keybinds();
        for (id, bind) in binds.iter().filter(|(_, b)| b.key == name) {
            self.mod_bind(id, &bind.id, pressed);
        }
        self.show_mod_output();
    }
    /// Mod `id`'s keybind `bind` held or let go (a key, or its touch button).
    pub(super) fn mod_bind(&mut self, id: &str, bind: &str, pressed: bool) {
        let full = format!("{id}:{bind}");
        let held = self.mods.keys_down.contains(&full);
        if pressed == held {
            return;
        }
        if pressed {
            self.mods.keys_down.push(full);
            // A press on a paused game's screen (a mod's button there) comes between no ticks.
            self.refresh_mod_snapshot();
        } else {
            self.mods.keys_down.retain(|k| *k != full);
        }
        self.mods.mods.keybind(id, bind, pressed);
        self.show_mod_output();
    }
    /// Minecraft.startAttack: whether the mods let it go ahead, on the entity or block under
    /// the crosshair.
    pub(super) fn mods_allow_attack(&mut self) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        if let Some((i, distance)) = self.mob_hit()
            && self.before_block(distance)
        {
            let id = self.mobs[i].id as i32;
            return self.mods_attack_entity(id);
        }
        match self.pick() {
            Some(hit) => self.mods_attack_block(hit.pos, hit.face),
            None => true,
        }
    }
    /// Minecraft.startUseItem: whether the mods let it go ahead, on the block under the
    /// crosshair or with the held item.
    pub(super) fn mods_allow_use(&mut self) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        if let Some(hit) = self.pick()
            && !self.entity_targeted()
            && !self.mods_use_block(hit.pos, hit.face, false)
        {
            return false;
        }
        self.mods_use_item(false)
    }
    /// `UseItemCallback`: false when a mod cancels it.
    pub(super) fn mods_use_item(&mut self, off_hand: bool) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        let hand = if off_hand {
            horizon_mods::Hand::OffHand
        } else {
            horizon_mods::Hand::MainHand
        };
        let allowed = self.mods.mods.use_item(hand);
        self.show_mod_output();
        allowed
    }
    /// `UseBlockCallback`.
    pub(super) fn mods_use_block(&mut self, pos: [i32; 3], face: [i32; 3], off_hand: bool) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        let hand = if off_hand {
            horizon_mods::Hand::OffHand
        } else {
            horizon_mods::Hand::MainHand
        };
        let allowed = self
            .mods
            .mods
            .use_block(block_pos(pos), direction(face), hand);
        self.show_mod_output();
        allowed
    }
    /// `AttackEntityCallback`.
    pub(super) fn mods_attack_entity(&mut self, id: i32) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        let allowed = self.mods.mods.attack_entity(id);
        self.show_mod_output();
        allowed
    }
    /// `AttackBlockCallback`.
    pub(super) fn mods_attack_block(&mut self, pos: [i32; 3], face: [i32; 3]) -> bool {
        if !self.mods.mods.wants(|h| h.interaction) {
            return true;
        }
        let allowed = self.mods.mods.attack_block(block_pos(pos), direction(face));
        self.show_mod_output();
        allowed
    }
    /// `ClientPlayerBlockBreakEvents.AFTER`.
    pub(super) fn mods_block_broken(
        &mut self,
        pos: [i32; 3],
        state: horizon_world::registry::BlockStateId,
    ) {
        if !self.mods.mods.wants(|h| h.interaction) {
            return;
        }
        let block = self.state_string(state);
        self.mods.mods.client_block_broken(block_pos(pos), &block);
        self.show_mod_output();
    }
    /// What mods showed and asked for: client mods' lines into the chat, server mods' to their
    /// players.
    pub(super) fn show_mod_output(&mut self) {
        self.route_mod_output(None);
    }
    /// [`Game::show_mod_output`], with the hub when it is out of `self.lan`.
    pub(super) fn route_mod_output(&mut self, mut host: Option<&mut LanHost>) {
        // Slots a mod clicked or set: its next hook reads them as they are now.
        let mut clicked = false;
        for output in self.mods.mods.drain() {
            let line =
                match output {
                    Output::Chat { name, text } => {
                        let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                        line.push(rich_text(&text));
                        line
                    }
                    Output::Feedback { text, error: false } => Text::literal(text),
                    Output::Feedback { text, error: true } => {
                        Text::literal(text).with_color(text::RED)
                    }
                    // Horizon's `command.run`: vanilla's commands only, as a mod running a mod
                    // command could loop.
                    Output::Command { line } => {
                        let name = line.split(' ').next().unwrap_or_default();
                        if !vanilla_command(name) {
                            // Dropped.
                        } else if self.remote.is_some() {
                            // The server checks the player's permissions.
                            self.send_chat(&format!("/{line}"));
                        } else {
                            self.mods.commands.push(line);
                        }
                        continue;
                    }
                    Output::QuietCommands { mod_id, lines } => {
                        self.mod_quiet_commands(mod_id, lines);
                        continue;
                    }
                    Output::OpenUiScreen { owner } => {
                        self.open_mod_ui(owner, false);
                        continue;
                    }
                    Output::CloseUiScreen { owner } => {
                        if self
                            .mods
                            .ui_screen
                            .as_ref()
                            .is_some_and(|s| s.owner == owner)
                        {
                            self.close_mod_ui(false);
                        }
                        continue;
                    }
                    Output::Shake { strength, seconds } => {
                        self.mods.shake.add(strength, seconds);
                        continue;
                    }
                    Output::PlaceRecipe { book_id, all } => {
                        self.mod_place_recipe(book_id, all);
                        continue;
                    }
                    Output::Give {
                        item,
                        count,
                        components,
                    } => {
                        self.mod_give(&item, count, &components);
                        continue;
                    }
                    // A Horizon mod's screen goes with it: the entry's run is gone, so
                    // `mod_ui_view` finds no widgets and the screen closes.
                    Output::Stopped { name, reason } => {
                        self.list_mods();
                        self.sync_mod_commands();
                        if self
                            .mods
                            .screen
                            .as_ref()
                            .is_some_and(|s| !self.mods.mods.wants(|_| true) || s.is_empty())
                        {
                            self.mods.screen = None;
                        }
                        let line = format!("Mod {name} stopped: {reason}");
                        if self.options.dedicated.is_some() {
                            super::dedicated::log("WARN", &line);
                        }
                        Text::literal(line).with_color(text::RED)
                    }
                    server @ (Output::Tell { .. }
                    | Output::Broadcast { .. }
                    | Output::Reply { .. }
                    | Output::SetBlocks { .. }) => {
                        self.route_server_output(host.as_deref_mut(), server);
                        continue;
                    }
                    Output::Overlay { text } => {
                        self.titles.set_overlay(rich_text(&text), false);
                        continue;
                    }
                    Output::Send { message } => {
                        self.mod_send(&message);
                        continue;
                    }
                    Output::Sound { id, volume, pitch } => {
                        self.audio.play(&id, None, volume, pitch);
                        continue;
                    }
                    Output::Payload { channel, data } => {
                        self.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
                            horizon_protocol::packets::configuration::CustomPayload {
                                channel,
                                data,
                            },
                        ));
                        continue;
                    }
                    Output::OpenScreen { screen } => {
                        self.open_mod_screen(screen);
                        continue;
                    }
                    Output::Click {
                        slot, button, kind, ..
                    } => {
                        self.mod_slot_click(slot, button, kind);
                        clicked = true;
                        continue;
                    }
                    Output::Drag { slots, button, .. } => {
                        self.mod_slot_drag(&slots, button);
                        clicked = true;
                        continue;
                    }
                    Output::DragAreas { id, areas } => {
                        if self.mods.screen.as_ref().is_some_and(|s| {
                            s.split_once(':').is_some_and(|(owner, _)| owner == id)
                        }) {
                            self.mods.drag_areas = areas;
                        }
                        continue;
                    }
                    Output::FillRecipe { recipe, all, .. } => {
                        self.fill_recipe(&recipe, all);
                        continue;
                    }
                    Output::ScreenSlots { id, slots } => {
                        if self.mods.screen.as_ref().is_some_and(|s| {
                            s.split_once(':').is_some_and(|(owner, _)| owner == id)
                        }) {
                            self.mods.screen_slots = slots;
                        }
                        continue;
                    }
                    Output::CreativeClick {
                        tab,
                        search,
                        index,
                        button,
                        kind,
                        ..
                    } => {
                        self.mod_creative_click(&tab, &search, index, button, kind);
                        clicked = true;
                        continue;
                    }
                    Output::CreativeSlot { slot, stack, .. } => {
                        self.mod_creative_slot(slot, stack.as_ref());
                        clicked = true;
                        continue;
                    }
                    Output::CreativeCarried { stack, .. } => {
                        self.mod_creative_carried(stack.as_ref());
                        clicked = true;
                        continue;
                    }
                    Output::Hold { id, control, held } => {
                        let slot = usize::from(control == "key.sprint");
                        self.mods
                            .holds
                            .retain(|(owner, s)| !(*owner == id && *s == slot));
                        if held {
                            self.mods.holds.push((id, slot));
                        }
                        self.sync_toggle(slot);
                        continue;
                    }
                    Output::Keyboard { id, shown } => {
                        if self.mods.screen.as_ref().is_some_and(|s| {
                            s.split_once(':').is_some_and(|(owner, _)| owner == id)
                        }) {
                            self.mods.keyboard = shown;
                        }
                        continue;
                    }
                    Output::CloseScreen { id } => {
                        if self.mods.screen.as_ref().is_some_and(|s| {
                            s.split_once(':').is_some_and(|(owner, _)| owner == id)
                        }) {
                            self.close_mod_screen();
                        }
                        continue;
                    }
                };
            self.add_mod_chat_line(line);
        }
        if clicked {
            self.refresh_mod_snapshot();
        }
        // A Horizon mod may set or clear its settings page in any call, not only while loading:
        // its row's Settings button follows.
        let rows = self.settings.mods.installed.iter_mut();
        for (row, e) in rows.zip(self.mods.mods.entries()) {
            row.has_settings = e.has_ui_settings();
        }
    }
    /// `chat-send.send`: a line or `/command` as the player typed it, past the chat hooks.
    fn mod_send(&mut self, message: &str) {
        if self.remote.is_some() {
            self.send_chat(message);
        } else if let Some(command) = message.strip_prefix('/') {
            self.run_command(command);
        } else {
            self.handle_chat_input(message);
        }
    }
    /// A mod's line in the window's chat; not through system_message: a mod
    /// must not hear its own lines.
    /// Written by this client, so the Chat option doesn't hide it.
    pub(super) fn add_mod_chat_line(&mut self, line: Text) {
        let focused = self.screen == Screen::Chat;
        self.ui.chat.add(
            &self.gui.atlas,
            line,
            None,
            horizon_client::chat::ChatSource::Client,
            self.ticks,
            focused,
        );
    }
    /// The mods' HUD panels, stacked with the extras in their corners.
    pub(super) fn draw_mod_panels(&self, batch: &mut HudBatch, corners: &mut Corners) {
        for panel in self.mods.mods.panels() {
            let corner = match panel.corner {
                horizon_mods::Corner::TopLeft => Corner::TopLeft,
                horizon_mods::Corner::TopRight => Corner::TopRight,
                horizon_mods::Corner::BottomLeft => Corner::BottomLeft,
                horizon_mods::Corner::BottomRight => Corner::BottomRight,
            };
            // F3 takes the top left, as it does from the extras.
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let lines: Vec<(&str, u32)> = panel
                .lines
                .iter()
                .map(|l| (l.text.as_str(), l.color))
                .collect();
            extras_hud::draw_lines(batch, &self.gui.atlas, corners, corner, &lines);
        }
        // Horizon's capability `render`'s corner panels, placed the same way.
        for panel in self.mods.mods.corner_panels() {
            let corner = match panel.corner {
                horizon_mods::RenderCorner::TopLeft => Corner::TopLeft,
                horizon_mods::RenderCorner::TopRight => Corner::TopRight,
                horizon_mods::RenderCorner::BottomLeft => Corner::BottomLeft,
                horizon_mods::RenderCorner::BottomRight => Corner::BottomRight,
            };
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let at = corners.place(corner, panel.width, panel.height);
            self.draw_shapes(batch, &panel.shapes, at);
        }
    }

    /// A Horizon `ui` screen's title and widgets as the mod last pushed them; None when the
    /// surface is gone (the mod stopped or closed it). Borrows `mods` only, so
    /// `self.mods.ui_screen` may stay borrowed for the scrolls.
    fn mod_ui_view<'a>(mods: &'a Mods, ms: &ModScreen) -> Option<(&'a str, &'a [UiWidget])> {
        if ms.settings {
            let widgets = mods.settings_widgets(ms.owner);
            if widgets.is_empty() && mods.keys_of(ms.owner).is_empty() {
                return None;
            }
            let name = mods
                .entries()
                .get(ms.owner)
                .and_then(|e| e.manifest.as_ref())
                .map_or("Mod", |m| m.name.as_str());
            Some((name, widgets))
        } else {
            let s: &UiScreen = mods.screen(ms.owner)?;
            Some((s.title.as_str(), s.widgets.as_slice()))
        }
    }
    /// The keys a settings page lists first: the mod's own (`input`).
    fn mod_page_keys<'a>(mods: &'a Mods, ms: &ModScreen) -> &'a [ModKey] {
        if ms.settings {
            mods.keys_of(ms.owner)
        } else {
            &[]
        }
    }
    /// A key row's label as Key Binds shows one, "Zoom: Z", marked
    /// "> Zoom: Z <" in yellow while it waits for the new key.
    fn mod_key_label(k: &ModKey, waiting: bool) -> Text {
        let value = format!("{}: {}", k.label, keybinds::display_name(&k.key).plain());
        if waiting {
            Text::literal(format!("> {value} <")).with_color(text::YELLOW)
        } else {
            Text::literal(value)
        }
    }
    /// The player bound the key row that waited to `key`: the mod's key
    /// follows it now and at the next start (`horizon-mods.txt`).
    fn bind_mod_key(&mut self, key: String) {
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            return;
        };
        let Some(name) = ms.binding.take() else {
            return;
        };
        let owner = ms.owner;
        if !self.mods.mods.rebind(owner, &name, &key) {
            return;
        }
        let Some(id) = self
            .mods
            .mods
            .entries()
            .get(owner)
            .and_then(|e| e.manifest.as_ref())
            .map(|m| m.id.clone())
        else {
            return;
        };
        self.settings
            .mods
            .keys
            .entry(id)
            .or_default()
            .insert(name, key);
        self.settings.save(&Self::options_path());
    }
    /// A key while a Horizon mod screen is up: true when a key row waited for it (Key Binds:
    /// Escape binds none).
    pub(super) fn mod_screen_key(&mut self, key: KeyCode) -> bool {
        if self
            .mods
            .ui_screen
            .as_ref()
            .is_none_or(|ms| ms.binding.is_none())
        {
            return false;
        }
        let value = match key {
            KeyCode::Escape => None,
            k => keybinds::key_name(k),
        };
        self.bind_mod_key(value.unwrap_or("key.keyboard.unknown").to_owned());
        true
    }
    /// The layout the mod screen's widgets get at `h` below `top`: (widget, top, height), the
    /// column being `MOD_UI_COLUMN` wide down the middle.
    fn mod_ui_layout(h: f32, top: f32, widgets: &[UiWidget]) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(widgets.len());
        let mut y = top;
        for w in widgets {
            let wh = match w {
                UiWidget::Text(_) => 12.0,
                UiWidget::Item(_) | UiWidget::Button(_) => 20.0,
                UiWidget::List(l) => (l.rows.len().min(6) as f32 * 12.0) + 8.0,
            };
            // Nothing may cover Done; a crowded mod loses what won't fit.
            if y + wh > h - 34.0 {
                break;
            }
            out.push((y, wh));
            y += wh + 6.0;
        }
        out
    }
    /// A Horizon mod pushes its screen (`ui.open-screen`), or Options > Mods opens its settings
    /// page (`settings`, the mod's `installed` index). Starling's mod screens are
    /// `open_mod_screen` (`mod_ui.rs`).
    pub(super) fn open_mod_ui(&mut self, owner: usize, settings: bool) {
        if self.title.is_some() {
            return;
        }
        if settings {
            self.mods.mods.ui_settings_opened(owner);
            let keys = self.mods.mods.keys_of(owner);
            if self.mods.mods.settings_widgets(owner).is_empty() && keys.is_empty() {
                self.show_mod_output();
                return;
            }
        }
        self.mods.ui_screen = Some(ModScreen {
            owner,
            settings,
            under: self.screen,
            scrolls: Vec::new(),
            binding: None,
        });
        self.screen = Screen::Mod;
        if self.player.is_some() {
            self.capture(false);
        }
        self.show_mod_output();
    }
    /// Leaves the Horizon mod screen: back to Options for a settings page, to what it covered
    /// for a mod's own. `tell` is Esc/Done, heard as `closed`.
    pub(super) fn close_mod_ui(&mut self, tell: bool) {
        let Some(ms) = self.mods.ui_screen.take() else {
            return;
        };
        if ms.settings {
            self.open_options_at(self.options_from_title, Page::Mods);
        } else {
            self.screen = ms.under;
            if self.screen == Screen::Playing && self.player.is_some() {
                self.capture(true);
            }
        }
        if tell {
            self.mods.mods.ui_closed(ms.owner);
            self.show_mod_output();
        }
    }
    /// The Horizon mod's screen or settings page: its title over widgets stacked down the
    /// middle, a Done button, Esc to leave (vanilla's menus).
    pub(super) fn draw_mod_screen(&mut self, batch: &mut HudBatch) {
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            // The screen was dropped while it was up: back to the world, mouse held again like
            // any return to Playing.
            self.screen = Screen::Playing;
            if self.player.is_some() {
                self.capture(true);
            }
            return;
        };
        let Some((title, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            // The mod's surface is gone (it stopped or cleared it): leave the screen like Done
            // did, without `closed` — the mod ended it.
            self.close_mod_ui(false);
            return;
        };
        // Item icons draw in a second pass: `draw_item` needs `&mut self`, so `atlas` may not
        // stay borrowed across it.
        let mut items: Vec<(String, f32, f32)> = Vec::new();
        let atlas = &self.gui.atlas;
        let in_world = self.player.is_some();
        ui::menu_background(batch, atlas, 0.0, 0.0, batch.width, batch.height, in_world);
        ui::centered_text(
            batch,
            atlas,
            &Text::literal(title),
            0.0,
            15.0,
            batch.width,
            u32::MAX,
        );
        let cx = (batch.width / 2.0).floor();
        let x = cx - MOD_UI_COLUMN / 2.0;
        // A settings page starts with the mod's keys, as Key Binds rows.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        for (i, k) in keys.iter().enumerate() {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
            let waiting = ms.binding.as_deref() == Some(k.name.as_str());
            let label = Self::mod_key_label(k, waiting);
            ui::button(
                batch,
                atlas,
                x,
                wy,
                MOD_UI_COLUMN,
                20.0,
                &label,
                true,
                hover,
            );
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(batch.height, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            match w {
                UiWidget::Button(b) => {
                    let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
                    let label = Text::literal(b.label.clone());
                    ui::button(
                        batch,
                        atlas,
                        x,
                        wy,
                        MOD_UI_COLUMN,
                        20.0,
                        &label,
                        true,
                        hover,
                    );
                }
                UiWidget::Text(t) => {
                    let color = if t.color == 0 {
                        u32::MAX
                    } else {
                        t.color | 0xFF00_0000
                    };
                    ui::centered_text(
                        batch,
                        atlas,
                        &Text::literal(t.text.clone()),
                        x,
                        wy + 1.0,
                        MOD_UI_COLUMN,
                        color,
                    );
                }
                UiWidget::Item(i) => {
                    if self.res.items().get(&i.item).is_some() {
                        items.push((i.item.clone(), x + 2.0, wy + 2.0));
                    } else {
                        batch.text(atlas, "?", x + 6.0, wy + 6.0, u32::MAX, true);
                    }
                    let name = if i.name.is_empty() {
                        i.item.rsplit(':').next().unwrap_or(&i.item).to_string()
                    } else {
                        i.name.clone()
                    };
                    batch.text_styled(
                        atlas,
                        &Text::literal(name),
                        x + 22.0,
                        wy + 6.0,
                        u32::MAX,
                        true,
                    );
                }
                UiWidget::List(l) => {
                    ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
                    let scroll = ms.scrolls[list_i];
                    list_i += 1;
                    let rows_top = wy + 4.0;
                    let rows_bottom = wy + wh - 4.0;
                    batch.fill(atlas, x, wy, MOD_UI_COLUMN, wh, 0x8000_0000);
                    for (b, yy) in [(x, wy), (x, wy + wh - 1.0)] {
                        batch.fill(atlas, b, yy, MOD_UI_COLUMN, 1.0, 0xFF50_5050);
                    }
                    for (xx, yy) in [(x, wy), (x + MOD_UI_COLUMN - 1.0, wy)] {
                        batch.fill(atlas, xx, yy, 1.0, wh, 0xFF50_5050);
                    }
                    let content = l.rows.len() as f32 * 12.0;
                    let first = (scroll.amount / 12.0) as usize;
                    let last = (first + ((rows_bottom - rows_top) / 12.0).ceil() as usize + 1)
                        .min(l.rows.len());
                    for (vi, ri) in (first..last).enumerate() {
                        let ry = rows_top + vi as f32 * 12.0 - (scroll.amount % 12.0);
                        if ui::hovered(self.cursor, x, ry, MOD_UI_COLUMN - 6.0, 12.0) {
                            batch.fill(atlas, x + 1.0, ry, MOD_UI_COLUMN - 8.0, 12.0, 0x20FF_FFFF);
                        }
                        if l.selected == Some(ri as u32) {
                            batch.fill(atlas, x + 1.0, ry, 3.0, 12.0, 0xFFFF_FFFF);
                        }
                        batch.text_styled(
                            atlas,
                            &Text::literal(l.rows[ri].clone()),
                            x + 6.0,
                            ry + 2.0,
                            u32::MAX,
                            true,
                        );
                    }
                    ui::scrollbar(
                        batch,
                        atlas,
                        x + MOD_UI_COLUMN - 5.0,
                        rows_top,
                        rows_bottom,
                        content,
                        scroll.amount,
                    );
                }
            }
        }
        let hover = ui::hovered(self.cursor, cx - 100.0, batch.height - 27.0, 200.0, 20.0);
        ui::button(
            batch,
            atlas,
            cx - 100.0,
            batch.height - 27.0,
            200.0,
            20.0,
            &Text::literal("Done"),
            true,
            hover,
        );
        for (id, ix, iy) in items {
            let stack = ItemStack::new(id.as_str(), 1);
            self.draw_item(batch, &stack, ix, iy);
        }
    }
    /// A click on the Horizon mod screen: Done leaves it, a `list` row is the mod's
    /// `ui-events::select`.
    pub(super) fn mod_screen_click(&mut self, button: MouseButton) {
        // Key Binds: a mouse button pressed while waiting binds it.
        if self
            .mods
            .ui_screen
            .as_ref()
            .is_some_and(|ms| ms.binding.is_some())
        {
            self.bind_mod_key(keybinds::mouse_name(button));
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        if ui::hovered(cursor, (w / 2.0).floor() - 100.0, h - 27.0, 200.0, 20.0) {
            self.audio.play_button_click();
            self.close_mod_ui(true);
            return;
        }
        let Some(ms) = self.mods.ui_screen.as_ref() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let owner = ms.owner;
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        // A key row waits for the key to bind.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        let picked = (0..keys.len()).find(|&i| {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
        });
        if let Some(i) = picked {
            let name = keys[i].name.clone();
            self.audio.play_button_click();
            if let Some(ms) = self.mods.ui_screen.as_mut() {
                ms.binding = Some(name);
            }
            return;
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        // The `ui_select` call needs `&mut`: find the pick first.
        let mut hit = None;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            // A button is `select(id, 0)`.
            if let UiWidget::Button(b) = w
                && ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
            {
                self.audio.play_button_click();
                let owner = self.mods.ui_screen.as_ref().map_or(0, |ms| ms.owner);
                self.mods.mods.ui_select(owner, b.id, 0);
                self.show_mod_output();
                return;
            }
            let UiWidget::List(l) = w else { continue };
            let scroll = ms.scrolls.get(list_i).copied().unwrap_or_default();
            list_i += 1;
            let row = ((cursor.1 - (wy + 4.0) + scroll.amount) / 12.0) as i64;
            if ui::hovered(cursor, x + 1.0, wy + 1.0, MOD_UI_COLUMN - 7.0, wh - 2.0)
                && row >= 0
                && (row as usize) < l.rows.len()
            {
                hit = Some((l.id, row as u32));
                break;
            }
        }
        if let Some((id, row)) = hit {
            self.audio.play_button_click();
            self.mods.mods.ui_select(owner, id, row);
            self.show_mod_output();
        }
    }
    /// The wheel over a Horizon mod screen's `list` scrolls it.
    pub(super) fn mod_screen_scroll(&mut self, delta: f32) {
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        let top =
            MOD_UI_TOP + Self::mod_page_keys(&self.mods.mods, ms).len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            let UiWidget::List(l) = w else { continue };
            let over = ui::hovered(cursor, x, wy, MOD_UI_COLUMN, wh);
            ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
            if over {
                let content = l.rows.len() as f32 * 12.0;
                ms.scrolls[list_i].wheel(delta, 12.0, wy + 4.0, wy + wh - 4.0, content);
                return;
            }
            list_i += 1;
        }
||||||| f8988d5a7
        self.mods.checked = Some(Instant::now());
        if self.mods.mods.reload_changed(vanilla_command) {
            let chosen = &self.settings.mods;
            self.mods.mods.set_bindings(|id| chosen.keys_of(id));
            self.list_mods();
            self.sync_mod_commands();
            if self.options.execution_role.is_client() {
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            } else {
                log::info!("Horizon: mods reloaded");
            }
        }
    }
    /// Once per frame: the frame hook, and dev mods' rebuilds.
    pub(super) fn mods_frame(&mut self, elapsed: Duration) {
        self.reload_dev_mods();
        if !self.mods.mods.is_running() {
            return;
        }
        self.mod_input_frame();
        if self.mods.mods.wants_frames() {
            let snapshot = self.mod_snapshot();
            self.mods.mods.set_snapshot(snapshot);
        }
        self.mods.mods.frame(elapsed.as_secs_f32());
        self.show_mod_output();
    }
    /// `/command` typed in chat: true when a mod owns it.
    pub(super) fn mod_command(&mut self, command: &str) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        // The command may follow a teleport in the same tick.
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        if !self.mods.mods.command(command) {
            return false;
        }
        self.show_mod_output();
        true
    }
    /// A chat line the player received.
    pub(super) fn mods_chat(&mut self, line: &str) {
        self.mods.mods.chat(line);
        self.show_mod_output();
    }
    /// What mods showed: client mods' lines into the chat, server mods'
    /// to their players.
    pub(super) fn show_mod_output(&mut self) {
        self.route_mod_output(None);
    }
    /// [`Game::show_mod_output`], with the hub when it is out of `self.lan`.
    pub(super) fn route_mod_output(&mut self, mut host: Option<&mut LanHost>) {
        for output in self.mods.mods.drain() {
            let line = match output {
                Output::Chat { name, text } => {
                    let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                    line.push(Text::literal(text));
                    line
                }
                Output::Feedback { text, error: false } => Text::literal(text),
                Output::Feedback { text, error: true } => Text::literal(text).with_color(text::RED),
                // Vanilla's commands only: a mod running a mod command could loop.
                Output::Command { line } => {
                    let name = line.split(' ').next().unwrap_or_default();
                    if !vanilla_command(name) {
                        // Dropped.
                    } else if self.remote.is_some() {
                        // The server checks the player's permissions.
                        self.send_chat(&format!("/{line}"));
                    } else {
                        self.mods.commands.push(line);
                    }
                    continue;
                }
                Output::QuietCommands { mod_id, lines } => {
                    self.mod_quiet_commands(mod_id, lines);
                    continue;
                }
                Output::Stopped { name, reason } => {
                    // Its screen goes with it: the entry's run is gone, so
                    // `mod_ui_view` finds no widgets and the screen closes.
                    self.list_mods();
                    self.sync_mod_commands();
                    let line = format!("Mod {name} stopped: {reason}");
                    if self.options.dedicated.is_some() {
                        super::dedicated::log("WARN", &line);
                    }
                    Text::literal(line).with_color(text::RED)
                }
                server @ (Output::Tell { .. }
                | Output::Broadcast { .. }
                | Output::Reply { .. }
                | Output::SetBlocks { .. }) => {
                    self.route_server_output(host.as_deref_mut(), server);
                    continue;
                }
                Output::OpenScreen { owner } => {
                    self.open_mod_screen(owner, false);
                    continue;
                }
                Output::Shake { strength, seconds } => {
                    self.mods.shake.add(strength, seconds);
                    continue;
                }
                Output::PlaceRecipe { book_id, all } => {
                    self.mod_place_recipe(book_id, all);
                    continue;
                }
                Output::Give {
                    item,
                    count,
                    components,
                } => {
                    self.mod_give(&item, count, &components);
                    continue;
                }
                Output::CloseScreen { owner } => {
                    if self.mods.screen.as_ref().is_some_and(|s| s.owner == owner) {
                        self.close_mod_screen(false);
                    }
                    continue;
                }
            };
            self.add_mod_chat_line(line);
        }
        // A mod may set or clear its settings page in any call, not only
        // while loading: its row's Settings button follows.
        let rows = self.settings.mods.installed.iter_mut();
        for (row, e) in rows.zip(self.mods.mods.entries()) {
            row.has_settings = e.has_ui_settings();
        }
    }
    /// A mod's line in the window's chat; not through system_message: a mod
    /// must not hear its own lines.
    /// Written by this client, so the Chat option doesn't hide it.
    pub(super) fn add_mod_chat_line(&mut self, line: Text) {
        let focused = self.screen == Screen::Chat;
        self.ui.chat.add(
            &self.gui.atlas,
            line,
            None,
            horizon_client::chat::ChatSource::Client,
            self.ticks,
            focused,
        );
    }
    /// The mods' HUD panels, stacked with the extras in their corners.
    pub(super) fn draw_mod_panels(&self, batch: &mut HudBatch, corners: &mut Corners) {
        for panel in self.mods.mods.panels() {
            let corner = match panel.corner {
                horizon_mods::Corner::TopLeft => Corner::TopLeft,
                horizon_mods::Corner::TopRight => Corner::TopRight,
                horizon_mods::Corner::BottomLeft => Corner::BottomLeft,
                horizon_mods::Corner::BottomRight => Corner::BottomRight,
            };
            // F3 takes the top left, as it does from the extras.
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let lines: Vec<(&str, u32)> = panel
                .lines
                .iter()
                .map(|l| (l.text.as_str(), l.color))
                .collect();
            extras_hud::draw_lines(batch, &self.gui.atlas, corners, corner, &lines);
        }
        // Capability `render`'s corner panels, placed the same way.
        for panel in self.mods.mods.corner_panels() {
            let corner = match panel.corner {
                horizon_mods::RenderCorner::TopLeft => Corner::TopLeft,
                horizon_mods::RenderCorner::TopRight => Corner::TopRight,
                horizon_mods::RenderCorner::BottomLeft => Corner::BottomLeft,
                horizon_mods::RenderCorner::BottomRight => Corner::BottomRight,
            };
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let at = corners.place(corner, panel.width, panel.height);
            self.draw_shapes(batch, &panel.shapes, at);
        }
    }

    /// A `ui` screen's title and widgets as the mod last pushed them; None
    /// when the surface is gone (the mod stopped or closed it). Borrows
    /// `mods` only, so `self.mods.screen` may stay borrowed for the scrolls.
    fn mod_ui_view<'a>(mods: &'a Mods, ms: &ModScreen) -> Option<(&'a str, &'a [UiWidget])> {
        if ms.settings {
            let widgets = mods.settings_widgets(ms.owner);
            if widgets.is_empty() && mods.keys_of(ms.owner).is_empty() {
                return None;
            }
            let name = mods
                .entries()
                .get(ms.owner)
                .and_then(|e| e.manifest.as_ref())
                .map_or("Mod", |m| m.name.as_str());
            Some((name, widgets))
        } else {
            let s: &UiScreen = mods.screen(ms.owner)?;
            Some((s.title.as_str(), s.widgets.as_slice()))
        }
    }
    /// The keys a settings page lists first: the mod's own (`input`).
    fn mod_page_keys<'a>(mods: &'a Mods, ms: &ModScreen) -> &'a [ModKey] {
        if ms.settings {
            mods.keys_of(ms.owner)
        } else {
            &[]
        }
    }
    /// A key row's label as Key Binds shows one, "Zoom: Z", marked
    /// "> Zoom: Z <" in yellow while it waits for the new key.
    fn mod_key_label(k: &ModKey, waiting: bool) -> Text {
        let value = format!("{}: {}", k.label, keybinds::display_name(&k.key).plain());
        if waiting {
            Text::literal(format!("> {value} <")).with_color(text::YELLOW)
        } else {
            Text::literal(value)
        }
    }
    /// The player bound the key row that waited to `key`: the mod's key
    /// follows it now and at the next start (`horizon-mods.txt`).
    fn bind_mod_key(&mut self, key: String) {
        let Some(ms) = self.mods.screen.as_mut() else {
            return;
        };
        let Some(name) = ms.binding.take() else {
            return;
        };
        let owner = ms.owner;
        if !self.mods.mods.rebind(owner, &name, &key) {
            return;
        }
        let Some(id) = self
            .mods
            .mods
            .entries()
            .get(owner)
            .and_then(|e| e.manifest.as_ref())
            .map(|m| m.id.clone())
        else {
            return;
        };
        self.settings
            .mods
            .keys
            .entry(id)
            .or_default()
            .insert(name, key);
        self.settings.save(&Self::options_path());
    }
    /// A key while a mod screen is up: true when a key row waited for
    /// it (Key Binds: Escape binds none).
    pub(super) fn mod_screen_key(&mut self, key: KeyCode) -> bool {
        if self
            .mods
            .screen
            .as_ref()
            .is_none_or(|ms| ms.binding.is_none())
        {
            return false;
        }
        let value = match key {
            KeyCode::Escape => None,
            k => keybinds::key_name(k),
        };
        self.bind_mod_key(value.unwrap_or("key.keyboard.unknown").to_owned());
        true
    }
    /// The layout the mod screen's widgets get at `h`: (widget, top,
    /// height), the column being `COLUMN` wide down the middle.
    fn mod_ui_layout(h: f32, top: f32, widgets: &[UiWidget]) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(widgets.len());
        let mut y = top;
        for w in widgets {
            let wh = match w {
                UiWidget::Text(_) => 12.0,
                UiWidget::Item(_) | UiWidget::Button(_) => 20.0,
                UiWidget::List(l) => (l.rows.len().min(6) as f32 * 12.0) + 8.0,
            };
            // Nothing may cover Done; a crowded mod loses what won't fit.
            if y + wh > h - 34.0 {
                break;
            }
            out.push((y, wh));
            y += wh + 6.0;
        }
        out
    }
    /// A mod pushes its screen (`ui.open-screen`), or Options > Mods opens
    /// its settings page (`settings`, the mod's `installed` index).
    pub(super) fn open_mod_screen(&mut self, owner: usize, settings: bool) {
        if self.title.is_some() {
            return;
        }
        if settings {
            self.mods.mods.ui_settings_opened(owner);
            let keys = self.mods.mods.keys_of(owner);
            if self.mods.mods.settings_widgets(owner).is_empty() && keys.is_empty() {
                self.show_mod_output();
                return;
            }
        }
        self.mods.screen = Some(ModScreen {
            owner,
            settings,
            under: self.screen,
            scrolls: Vec::new(),
            binding: None,
        });
        self.screen = Screen::Mod;
        if self.player.is_some() {
            self.capture(false);
        }
        self.show_mod_output();
    }
    /// Leaves the mod screen: back to Options for a settings page, to what
    /// it covered for a mod's own. `tell` is Esc/Done, heard as `closed`.
    pub(super) fn close_mod_screen(&mut self, tell: bool) {
        let Some(ms) = self.mods.screen.take() else {
            return;
        };
        if ms.settings {
            self.open_options_at(self.options_from_title, Page::Mods);
        } else {
            self.screen = ms.under;
            if self.screen == Screen::Playing && self.player.is_some() {
                self.capture(true);
            }
        }
        if tell {
            self.mods.mods.ui_closed(ms.owner);
            self.show_mod_output();
        }
    }
    /// The mod's screen or settings page: its title over widgets stacked
    /// down the middle, a Done button, Esc to leave (vanilla's menus).
    pub(super) fn draw_mod_screen(&mut self, batch: &mut HudBatch) {
        let Some(ms) = self.mods.screen.as_mut() else {
            // The screen was dropped while it was up: back to the world,
            // mouse held again like any return to Playing.
            self.screen = Screen::Playing;
            if self.player.is_some() {
                self.capture(true);
            }
            return;
        };
        let Some((title, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            // The mod's surface is gone (it stopped or cleared it): leave
            // the screen like Done did, without `closed` — the mod ended it.
            self.close_mod_screen(false);
            return;
        };
        // Item icons draw in a second pass: `draw_item` needs `&mut self`,
        // so `atlas` may not stay borrowed across it.
        let mut items: Vec<(String, f32, f32)> = Vec::new();
        let atlas = &self.gui.atlas;
        let in_world = self.player.is_some();
        ui::menu_background(batch, atlas, 0.0, 0.0, batch.width, batch.height, in_world);
        ui::centered_text(
            batch,
            atlas,
            &Text::literal(title),
            0.0,
            15.0,
            batch.width,
            u32::MAX,
        );
        let cx = (batch.width / 2.0).floor();
        let x = cx - MOD_UI_COLUMN / 2.0;
        // A settings page starts with the mod's keys, as Key Binds rows.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        for (i, k) in keys.iter().enumerate() {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
            let waiting = ms.binding.as_deref() == Some(k.name.as_str());
            let label = Self::mod_key_label(k, waiting);
            ui::button(
                batch,
                atlas,
                x,
                wy,
                MOD_UI_COLUMN,
                20.0,
                &label,
                true,
                hover,
            );
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(batch.height, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            match w {
                UiWidget::Button(b) => {
                    let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
                    let label = Text::literal(b.label.clone());
                    ui::button(
                        batch,
                        atlas,
                        x,
                        wy,
                        MOD_UI_COLUMN,
                        20.0,
                        &label,
                        true,
                        hover,
                    );
                }
                UiWidget::Text(t) => {
                    let color = if t.color == 0 {
                        u32::MAX
                    } else {
                        t.color | 0xFF00_0000
                    };
                    ui::centered_text(
                        batch,
                        atlas,
                        &Text::literal(t.text.clone()),
                        x,
                        wy + 1.0,
                        MOD_UI_COLUMN,
                        color,
                    );
                }
                UiWidget::Item(i) => {
                    if self.res.items().get(&i.item).is_some() {
                        items.push((i.item.clone(), x + 2.0, wy + 2.0));
                    } else {
                        batch.text(atlas, "?", x + 6.0, wy + 6.0, u32::MAX, true);
                    }
                    let name = if i.name.is_empty() {
                        i.item.rsplit(':').next().unwrap_or(&i.item).to_string()
                    } else {
                        i.name.clone()
                    };
                    batch.text_styled(
                        atlas,
                        &Text::literal(name),
                        x + 22.0,
                        wy + 6.0,
                        u32::MAX,
                        true,
                    );
                }
                UiWidget::List(l) => {
                    ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
                    let scroll = ms.scrolls[list_i];
                    list_i += 1;
                    let rows_top = wy + 4.0;
                    let rows_bottom = wy + wh - 4.0;
                    batch.fill(atlas, x, wy, MOD_UI_COLUMN, wh, 0x8000_0000);
                    for (b, yy) in [(x, wy), (x, wy + wh - 1.0)] {
                        batch.fill(atlas, b, yy, MOD_UI_COLUMN, 1.0, 0xFF50_5050);
                    }
                    for (xx, yy) in [(x, wy), (x + MOD_UI_COLUMN - 1.0, wy)] {
                        batch.fill(atlas, xx, yy, 1.0, wh, 0xFF50_5050);
                    }
                    let content = l.rows.len() as f32 * 12.0;
                    let first = (scroll.amount / 12.0) as usize;
                    let last = (first + ((rows_bottom - rows_top) / 12.0).ceil() as usize + 1)
                        .min(l.rows.len());
                    for (vi, ri) in (first..last).enumerate() {
                        let ry = rows_top + vi as f32 * 12.0 - (scroll.amount % 12.0);
                        if ui::hovered(self.cursor, x, ry, MOD_UI_COLUMN - 6.0, 12.0) {
                            batch.fill(atlas, x + 1.0, ry, MOD_UI_COLUMN - 8.0, 12.0, 0x20FF_FFFF);
                        }
                        if l.selected == Some(ri as u32) {
                            batch.fill(atlas, x + 1.0, ry, 3.0, 12.0, 0xFFFF_FFFF);
                        }
                        batch.text_styled(
                            atlas,
                            &Text::literal(l.rows[ri].clone()),
                            x + 6.0,
                            ry + 2.0,
                            u32::MAX,
                            true,
                        );
                    }
                    ui::scrollbar(
                        batch,
                        atlas,
                        x + MOD_UI_COLUMN - 5.0,
                        rows_top,
                        rows_bottom,
                        content,
                        scroll.amount,
                    );
                }
            }
        }
        let hover = ui::hovered(self.cursor, cx - 100.0, batch.height - 27.0, 200.0, 20.0);
        ui::button(
            batch,
            atlas,
            cx - 100.0,
            batch.height - 27.0,
            200.0,
            20.0,
            &Text::literal("Done"),
            true,
            hover,
        );
        for (id, ix, iy) in items {
            let stack = ItemStack::new(id.as_str(), 1);
            self.draw_item(batch, &stack, ix, iy);
        }
    }
    /// A click on the mod screen: Done leaves it, a `list` row is the mod's
    /// `ui-events::select`.
    pub(super) fn mod_screen_click(&mut self, button: MouseButton) {
        // Key Binds: a mouse button pressed while waiting binds it.
        if self
            .mods
            .screen
            .as_ref()
            .is_some_and(|ms| ms.binding.is_some())
        {
            self.bind_mod_key(keybinds::mouse_name(button));
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        if ui::hovered(cursor, (w / 2.0).floor() - 100.0, h - 27.0, 200.0, 20.0) {
            self.audio.play_button_click();
            self.close_mod_screen(true);
            return;
        }
        let Some(ms) = self.mods.screen.as_ref() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let owner = ms.owner;
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        // A key row waits for the key to bind.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        let picked = (0..keys.len()).find(|&i| {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
        });
        if let Some(i) = picked {
            let name = keys[i].name.clone();
            self.audio.play_button_click();
            if let Some(ms) = self.mods.screen.as_mut() {
                ms.binding = Some(name);
            }
            return;
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        // The `ui_select` call needs `&mut`: find the pick first.
        let mut hit = None;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            // A button is `select(id, 0)`.
            if let UiWidget::Button(b) = w
                && ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
            {
                self.audio.play_button_click();
                let owner = self.mods.screen.as_ref().map_or(0, |ms| ms.owner);
                self.mods.mods.ui_select(owner, b.id, 0);
                self.show_mod_output();
                return;
            }
            let UiWidget::List(l) = w else { continue };
            let scroll = ms.scrolls.get(list_i).copied().unwrap_or_default();
            list_i += 1;
            let row = ((cursor.1 - (wy + 4.0) + scroll.amount) / 12.0) as i64;
            if ui::hovered(cursor, x + 1.0, wy + 1.0, MOD_UI_COLUMN - 7.0, wh - 2.0)
                && row >= 0
                && (row as usize) < l.rows.len()
            {
                hit = Some((l.id, row as u32));
                break;
            }
        }
        if let Some((id, row)) = hit {
            self.audio.play_button_click();
            self.mods.mods.ui_select(owner, id, row);
            self.show_mod_output();
        }
    }
    /// The wheel over a `list` scrolls it.
    pub(super) fn mod_screen_scroll(&mut self, delta: f32) {
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        let Some(ms) = self.mods.screen.as_mut() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        let top =
            MOD_UI_TOP + Self::mod_page_keys(&self.mods.mods, ms).len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            let UiWidget::List(l) = w else { continue };
            let over = ui::hovered(cursor, x, wy, MOD_UI_COLUMN, wh);
            ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
            if over {
                let content = l.rows.len() as f32 * 12.0;
                ms.scrolls[list_i].wheel(delta, 12.0, wy + 4.0, wy + wh - 4.0, content);
                return;
            }
            list_i += 1;
        }
=======
        self.system_message(Text::literal(format!(
            "Horizon mods are native now: {names} stopped running until they have a native version. The launcher's Mods page has the native ones."
        )));
        let _ = std::fs::write(&seen, "");
>>>>>>> 31f83499b
    }
}

<<<<<<< HEAD
/// Where a Horizon mod screen's widgets start, and their column's width.
const MOD_UI_TOP: f32 = 40.0;
const MOD_UI_COLUMN: f32 = 200.0;
/// A key row's height with its gap, as the widgets below it space out.
const MOD_UI_KEY_ROW: f32 = 26.0;

fn block_pos(p: [i32; 3]) -> horizon_mods::BlockPos {
    horizon_mods::BlockPos {
        x: p[0],
        y: p[1],
        z: p[2],
    }
}
fn direction(face: [i32; 3]) -> horizon_mods::Direction {
    use horizon_mods::Direction;
    match face {
        [0, -1, 0] => Direction::Down,
        [0, 1, 0] => Direction::Up,
        [0, 0, -1] => Direction::North,
        [0, 0, 1] => Direction::South,
        [-1, 0, 0] => Direction::West,
        _ => Direction::East,
||||||| f8988d5a7
#[cfg(feature = "mods")]
const MOD_UI_TOP: f32 = 40.0;
#[cfg(feature = "mods")]
const MOD_UI_COLUMN: f32 = 200.0;
/// A key row's height with its gap, as the widgets below it space out.
#[cfg(feature = "mods")]
const MOD_UI_KEY_ROW: f32 = 26.0;
=======
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A minimal `.hnmod` the registry accepts: its manifest and its mixin's.
    fn hnmod(dir: &std::path::Path, id: &str) {
        let horizon = "a".repeat(40);
        let manifest = serde_json::json!({
            "schema": 1, "id": id, "version": "0.3.0", "game": "26.3",
            "horizon": horizon, "mixins": "b".repeat(40),
        });
        let mixin = format!(
            "[mod]\nid = \"{id}\"\nversion = \"0.3.0\"\n[compatibility]\nhorizon_revision = \"{horizon}\"\n"
        );
        let file = std::fs::File::create(dir.join(format!("{id}.hnmod"))).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("horizon-native.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file(format!("mixins/{id}/horizon-mixin.toml"), options)
            .unwrap();
        zip.write_all(mixin.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn the_crash_report_lists_the_builds_native_mods() {
        let built = r#"{"mods":[{"id":"machine_sample","version":"0.1.0","sha256":null},{"id":"pipes","version":"2.0"}]}"#;
        assert_eq!(
            native_mods_detail(built).as_deref(),
            Some("machine_sample 0.1.0, pipes 2.0")
        );
        // A release has no file; a build without mods has an empty list.
        assert_eq!(native_mods_detail(""), None);
        assert_eq!(native_mods_detail(r#"{"mods":[]}"#), None);
    }

    #[test]
    fn native_mods_installed_but_not_built_get_a_row_saying_so() {
        let dir = std::env::temp_dir().join(format!("horizon-not-built-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for id in ["veinminer", "zoom", "minimap"] {
            hnmod(&dir, id);
        }
        std::fs::write(dir.join("notes.txt"), "not a mod").unwrap();
        std::fs::write(dir.join("broken.hnmod"), "not a zip").unwrap();
        // A release: none is built in; the page lists them, sorted, instead of "No mods".
        let rows = not_built(&dir, "", &[]);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["minimap", "veinminer", "zoom"]);
        assert_eq!(rows[1].title, "veinminer 0.3.0");
        assert_eq!(rows[1].problem.as_deref(), Some(mod_list::NOT_BUILT));
        assert!(rows.iter().all(|r| r.native.is_none() && !r.dev));
        // A native build with zoom, and minimap's row already listed by its own hook.
        let built =
            r#"{"base":"nightly-2026-10-30-1da354b","mods":[{"id":"zoom","version":"0.3.0"}]}"#;
        let listed = Installed {
            id: "minimap".into(),
            ..rows[0].clone()
        };
        let rows = not_built(&dir, built, &[listed]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "veinminer");
        // No folder: no rows.
        assert!(not_built(&dir.join("missing"), "", &[]).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
>>>>>>> 31f83499b
    }
}
