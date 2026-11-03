// Copyright (c) 2026 d3grando
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: 184b92bab7851684bbe6b840c5b8cacc10fa05f2. Android static-bundle adaptation.
// SmartF3's game-independent logic: the modules, what each one shows, the
// two columns and where each line goes, the configuration file, the settings
// page and `/smartf3`. Injected into the game as `crate::game::smartf3`
// (see horizon-mixin.toml) and, unchanged, this repository's Cargo library,
// so all of it is unit tested without the game. The adapter that reads the
// game and draws is native.rs.

use serde_json::{Map, Value, json};

pub const ID: &str = "smartf3";
pub const VERSION: &str = "0.1.0";
pub const NAME: &str = "SmartF3";
pub const DESCRIPTION: &str =
    "A tidier F3 debug screen: the same information in coloured modules you can hide and reorder.";
/// The file in the mod's settings folder (`native_setting`).
pub const CONFIG_FILE: &str = "config.json";
/// `Font.lineHeight`, the step between two lines of the debug screen.
pub const LINE_HEIGHT: i32 = 9;
/// The vanilla debug screen's text colour (ARGB), under the modules' colours.
pub const TEXT_ARGB: u32 = 0xFFE0_E0E0;

// ---------------------------------------------------------------- colours

/// The colours a click on the settings page steps through: the sixteen chat
/// colours, and the vanilla debug screen's own grey.
pub const PALETTE: [(&str, &str, u32); 17] = [
    ("white", "White", 0xFF_FFFF),
    ("f3_gray", "F3 Grey", 0xE0_E0E0),
    ("gray", "Grey", 0xAA_AAAA),
    ("dark_gray", "Dark Grey", 0x55_5555),
    ("black", "Black", 0x00_0000),
    ("red", "Red", 0xFF_5555),
    ("gold", "Gold", 0xFF_AA00),
    ("yellow", "Yellow", 0xFF_FF55),
    ("green", "Green", 0x55_FF55),
    ("aqua", "Aqua", 0x55_FFFF),
    ("blue", "Blue", 0x55_55FF),
    ("light_purple", "Light Purple", 0xFF_55FF),
    ("dark_red", "Dark Red", 0xAA_0000),
    ("dark_green", "Dark Green", 0x00_AA00),
    ("dark_aqua", "Dark Aqua", 0x00_AAAA),
    ("dark_blue", "Dark Blue", 0x00_00AA),
    ("dark_purple", "Dark Purple", 0xAA_00AA),
];

/// A palette name (`gold`, any case) or `#RRGGBB`, as RGB.
pub fn parse_color(text: &str) -> Option<u32> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u32::from_str_radix(hex, 16).ok();
        }
        return None;
    }
    PALETTE
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(text))
        .map(|&(_, _, rgb)| rgb)
}
/// How the file writes a colour: its palette name, else `#RRGGBB`.
pub fn color_key(rgb: u32) -> String {
    match PALETTE.iter().find(|p| p.2 == rgb) {
        Some((key, _, _)) => (*key).to_owned(),
        None => format!("#{rgb:06X}"),
    }
}
/// How the settings page shows a colour.
pub fn color_name(rgb: u32) -> String {
    match PALETTE.iter().find(|p| p.2 == rgb) {
        Some((_, name, _)) => (*name).to_owned(),
        None => format!("#{rgb:06X}"),
    }
}
/// The palette colour after `rgb` (the first one for a colour not in it).
pub fn next_color(rgb: u32) -> u32 {
    match PALETTE.iter().position(|p| p.2 == rgb) {
        Some(i) => PALETTE[(i + 1) % PALETTE.len()].2,
        None => PALETTE[0].2,
    }
}

// ---------------------------------------------------------------- modules

/// The modules, in the order the settings page steps through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Fps,
    Server,
    Coordinates,
    Chunk,
    Facing,
    World,
    Biome,
    Light,
    Charts,
    System,
    Rendering,
    TargetBlock,
    TargetFluid,
    TargetEntity,
}

impl ModuleId {
    pub const ALL: [ModuleId; 14] = [
        ModuleId::Fps,
        ModuleId::Server,
        ModuleId::Coordinates,
        ModuleId::Chunk,
        ModuleId::Facing,
        ModuleId::World,
        ModuleId::Biome,
        ModuleId::Light,
        ModuleId::Charts,
        ModuleId::System,
        ModuleId::Rendering,
        ModuleId::TargetBlock,
        ModuleId::TargetFluid,
        ModuleId::TargetEntity,
    ];
    /// Its key in the file.
    pub fn key(self) -> &'static str {
        match self {
            ModuleId::Fps => "fps",
            ModuleId::Server => "server",
            ModuleId::Coordinates => "coordinates",
            ModuleId::Chunk => "chunk",
            ModuleId::Facing => "facing",
            ModuleId::World => "world",
            ModuleId::Biome => "biome",
            ModuleId::Light => "light",
            ModuleId::Charts => "charts",
            ModuleId::System => "system",
            ModuleId::Rendering => "rendering",
            ModuleId::TargetBlock => "target_block",
            ModuleId::TargetFluid => "target_fluid",
            ModuleId::TargetEntity => "target_entity",
        }
    }
    /// Its name on the settings page.
    pub fn title(self) -> &'static str {
        match self {
            ModuleId::Fps => "FPS",
            ModuleId::Server => "Server",
            ModuleId::Coordinates => "Coordinates",
            ModuleId::Chunk => "Chunk",
            ModuleId::Facing => "Facing",
            ModuleId::World => "World",
            ModuleId::Biome => "Biome",
            ModuleId::Light => "Light",
            ModuleId::Charts => "Chart keys",
            ModuleId::System => "System (memory, CPU, GPU)",
            ModuleId::Rendering => "Rendering",
            ModuleId::TargetBlock => "Targeted block",
            ModuleId::TargetFluid => "Targeted fluid",
            ModuleId::TargetEntity => "Targeted entity",
        }
    }
    pub fn from_key(key: &str) -> Option<ModuleId> {
        ModuleId::ALL.into_iter().find(|m| m.key() == key)
    }
    /// The defaults: the vanilla screen's split (where you are on the left,
    /// the machine and what you look at on the right), one label colour each.
    fn default_config(self) -> ModuleConfig {
        let (column, label) = match self {
            ModuleId::Fps => (Column::Left, "green"),
            ModuleId::Server => (Column::Left, "light_purple"),
            ModuleId::Coordinates => (Column::Left, "red"),
            ModuleId::Chunk => (Column::Left, "yellow"),
            ModuleId::Facing => (Column::Left, "aqua"),
            ModuleId::World => (Column::Left, "dark_aqua"),
            ModuleId::Biome => (Column::Left, "dark_green"),
            ModuleId::Light => (Column::Left, "gold"),
            ModuleId::Charts => (Column::Left, "gray"),
            ModuleId::System => (Column::Right, "gold"),
            ModuleId::Rendering => (Column::Right, "gray"),
            ModuleId::TargetBlock => (Column::Right, "blue"),
            ModuleId::TargetFluid => (Column::Right, "aqua"),
            ModuleId::TargetEntity => (Column::Right, "red"),
        };
        ModuleConfig {
            id: self,
            shown: true,
            column,
            label: parse_color(label).unwrap_or(0xFF_FFFF),
            value: parse_color("f3_gray").unwrap_or(0xE0_E0E0),
        }
    }
    /// Left out while debug info is reduced (the server's
    /// `reducedDebugInfo` rule or the option), as vanilla leaves out where
    /// you are and what you look at.
    fn reduced_hides(self) -> bool {
        matches!(
            self,
            ModuleId::Coordinates
                | ModuleId::Facing
                | ModuleId::TargetBlock
                | ModuleId::TargetFluid
                | ModuleId::TargetEntity
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Left,
    Right,
}
impl Column {
    fn key(self) -> &'static str {
        match self {
            Column::Left => "left",
            Column::Right => "right",
        }
    }
    fn index(self) -> usize {
        match self {
            Column::Left => 0,
            Column::Right => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    /// The vanilla box behind each line (ARGB 0x90505050).
    Vanilla,
    /// A darker box (ARGB 0xA0000000).
    Dark,
    None,
}
impl Background {
    const ALL: [Background; 3] = [Background::Vanilla, Background::Dark, Background::None];
    fn key(self) -> &'static str {
        match self {
            Background::Vanilla => "vanilla",
            Background::Dark => "dark",
            Background::None => "none",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Background::Vanilla => "grey (vanilla)",
            Background::Dark => "dark",
            Background::None => "none",
        }
    }
    /// The box's ARGB colour, none for no box.
    pub fn argb(self) -> Option<u32> {
        match self {
            Background::Vanilla => Some(0x9050_5050),
            Background::Dark => Some(0xA000_0000),
            Background::None => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleConfig {
    pub id: ModuleId,
    pub shown: bool,
    pub column: Column,
    /// RGB of the labels (`XYZ:`) and of the values.
    pub label: u32,
    pub value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// SmartF3 draws F3; false leaves the vanilla debug screen.
    pub enabled: bool,
    /// Fewer, shorter lines.
    pub compact: bool,
    pub shadow: bool,
    pub background: Background,
    /// An empty line between two modules.
    pub spacing: bool,
    /// Every module once, in drawing order (top to bottom in its column).
    pub modules: Vec<ModuleConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            compact: false,
            shadow: true,
            background: Background::Vanilla,
            spacing: true,
            modules: ModuleId::ALL.map(ModuleId::default_config).to_vec(),
        }
    }
}

/// The configuration as read, or why it could not be.
pub type Loaded = Result<Config, String>;

const KEYS: [&str; 6] = [
    "enabled",
    "compact",
    "shadow",
    "background",
    "spacing",
    "modules",
];
const MODULE_KEYS: [&str; 5] = ["id", "shown", "column", "label", "value"];

fn flag(object: &Map<String, Value>, key: &str, default: bool) -> Result<bool, String> {
    match object.get(key) {
        None => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("\"{key}\" must be true or false")),
    }
}
fn text<'v>(object: &'v Map<String, Value>, key: &str) -> Result<Option<&'v str>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(format!("\"{key}\" must be a string")),
    }
}

impl Config {
    /// Reads `config.json`. Every key is optional; an unknown key or value
    /// is an error. Modules the file leaves out keep their defaults and come
    /// after the ones it lists.
    pub fn from_json(source: &str) -> Result<Config, String> {
        let value: Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
        let Value::Object(object) = value else {
            return Err("the file must hold a JSON object".to_owned());
        };
        if let Some(key) = object.keys().find(|k| !KEYS.contains(&k.as_str())) {
            return Err(format!("unknown key \"{key}\""));
        }
        let defaults = Config::default();
        let background = match text(&object, "background")? {
            None => defaults.background,
            Some(b) => Background::ALL
                .into_iter()
                .find(|x| x.key() == b)
                .ok_or_else(|| format!("unknown background \"{b}\" (vanilla, dark, none)"))?,
        };
        let mut modules: Vec<ModuleConfig> = Vec::new();
        match object.get("modules") {
            None => {}
            Some(Value::Array(list)) => {
                for entry in list {
                    let module = Self::module_from_json(entry)?;
                    if modules.iter().any(|m| m.id == module.id) {
                        return Err(format!("module \"{}\" is listed twice", module.id.key()));
                    }
                    modules.push(module);
                }
            }
            Some(_) => return Err("\"modules\" must be a list".to_owned()),
        }
        for id in ModuleId::ALL {
            if !modules.iter().any(|m| m.id == id) {
                modules.push(id.default_config());
            }
        }
        Ok(Config {
            enabled: flag(&object, "enabled", defaults.enabled)?,
            compact: flag(&object, "compact", defaults.compact)?,
            shadow: flag(&object, "shadow", defaults.shadow)?,
            background,
            spacing: flag(&object, "spacing", defaults.spacing)?,
            modules,
        })
    }

    fn module_from_json(entry: &Value) -> Result<ModuleConfig, String> {
        let Value::Object(object) = entry else {
            return Err("each module must be a JSON object".to_owned());
        };
        if let Some(key) = object.keys().find(|k| !MODULE_KEYS.contains(&k.as_str())) {
            return Err(format!("unknown module key \"{key}\""));
        }
        let key = text(object, "id")?.ok_or("a module has no \"id\"")?;
        let id = ModuleId::from_key(key).ok_or_else(|| format!("unknown module \"{key}\""))?;
        let mut module = id.default_config();
        module.shown = flag(object, "shown", module.shown)?;
        if let Some(column) = text(object, "column")? {
            module.column = match column {
                "left" => Column::Left,
                "right" => Column::Right,
                other => return Err(format!("{key}: unknown column \"{other}\" (left, right)")),
            };
        }
        for (field, slot) in [("label", &mut module.label), ("value", &mut module.value)] {
            if let Some(color) = text(object, field)? {
                *slot = parse_color(color)
                    .ok_or_else(|| format!("{key}: unknown {field} colour \"{color}\""))?;
            }
        }
        Ok(module)
    }

    /// The file's text: every key, modules in their order.
    pub fn to_json(&self) -> String {
        let modules: Vec<Value> = self
            .modules
            .iter()
            .map(|m| {
                json!({
                    "id": m.id.key(),
                    "shown": m.shown,
                    "column": m.column.key(),
                    "label": color_key(m.label),
                    "value": color_key(m.value),
                })
            })
            .collect();
        let value = json!({
            "enabled": self.enabled,
            "compact": self.compact,
            "shadow": self.shadow,
            "background": self.background.key(),
            "spacing": self.spacing,
            "modules": modules,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }

    pub fn module(&self, id: ModuleId) -> Option<&ModuleConfig> {
        self.modules.iter().find(|m| m.id == id)
    }
    fn module_mut(&mut self, id: ModuleId) -> Option<&mut ModuleConfig> {
        self.modules.iter_mut().find(|m| m.id == id)
    }
    /// Whether the screen shows module `id` (the adapter reads only what
    /// the shown modules need).
    pub fn wants(&self, id: ModuleId) -> bool {
        self.module(id).is_some_and(|m| m.shown)
    }
    /// Moves module `id` one place up (`up`) or down among the modules of
    /// its column; false when it is already first or last there.
    pub fn shift(&mut self, id: ModuleId, up: bool) -> bool {
        let Some(at) = self.modules.iter().position(|m| m.id == id) else {
            return false;
        };
        let column = self.modules[at].column;
        let same = |m: &ModuleConfig| m.column == column;
        let other = if up {
            self.modules[..at].iter().rposition(same)
        } else {
            self.modules[at + 1..]
                .iter()
                .position(same)
                .map(|i| at + 1 + i)
        };
        match other {
            Some(other) => {
                self.modules.swap(at, other);
                true
            }
            None => false,
        }
    }
    /// Module `id`'s place in its column, from 1, and how many share it.
    pub fn place(&self, id: ModuleId) -> (usize, usize) {
        let Some(column) = self.module(id).map(|m| m.column) else {
            return (0, 0);
        };
        let mut place = 0;
        let mut count = 0;
        for m in self.modules.iter().filter(|m| m.column == column) {
            count += 1;
            if m.id == id {
                place = count;
            }
        }
        (place, count)
    }
}

// ---------------------------------------------------------------- what the game reports

/// What the adapter read from the game for one frame. Fields a hidden
/// module needs are left empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// Debug info is reduced: the modules of where you are and what you
    /// look at are left out.
    pub reduced: bool,
    pub fps: Option<Fps>,
    /// `26.3 (26.3/horizon)`.
    pub version: String,
    pub server: Option<Server>,
    pub player: Option<Player>,
    /// `minecraft:overworld`.
    pub dimension: String,
    /// Force-loaded chunks (`FC`).
    pub forced_chunks: usize,
    pub day_time: Option<i64>,
    pub biome: Option<String>,
    /// Sky and block light at your feet.
    pub light: Option<[u8; 2]>,
    pub charts: Vec<String>,
    pub system: System,
    pub rendering: Vec<String>,
    pub target_block: Option<TargetBlock>,
    pub target_fluid: Option<TargetFluid>,
    /// The entity type under the crosshair (`minecraft:zombie`).
    pub target_entity: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fps {
    pub fps: i32,
    /// The frame-rate limit; None when unlimited.
    pub limit: Option<i32>,
    /// ` (fifo)`, as vanilla writes it (may be empty).
    pub present: String,
    /// `165` (Hz).
    pub refresh: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Server {
    /// None for single player's integrated server, else the server's brand.
    pub brand: Option<String>,
    /// Milliseconds per tick of the integrated server.
    pub tick_ms: Option<f32>,
    /// The target milliseconds per tick (`50.0`, `-` while sprinting).
    pub target: String,
    /// ` (frozen)`, ` (sprinting)`... or empty.
    pub status: String,
    pub tx: i64,
    pub rx: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Player {
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct System {
    /// The game's resident memory in bytes, when it could be read.
    pub memory: Option<u64>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    /// The graphics API and driver.
    pub driver: Option<String>,
    /// `1920x1080 (NVIDIA Corporation)`.
    pub display: Option<String>,
    /// `960x540 (2.00x pixel density)`.
    pub window: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetBlock {
    pub pos: [i32; 3],
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetFluid {
    pub pos: [i32; 3],
    pub lava: bool,
    /// 1..=8; 8 is a source or a falling column.
    pub amount: u8,
    pub falling: bool,
    pub source: bool,
}

// ---------------------------------------------------------------- lines

/// One line of a module: `label: value`, or the value alone when the label
/// is empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub label: String,
    pub value: String,
}
fn line(label: &str, value: impl Into<String>) -> Line {
    Line {
        label: label.to_owned(),
        value: value.into(),
    }
}

/// `Mth.wrapDegrees`.
pub fn wrap_degrees(angle: f32) -> f32 {
    let wrapped = angle % 360.0;
    if wrapped >= 180.0 {
        wrapped - 360.0
    } else if wrapped < -180.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}
/// `Direction.fromYRot`: the compass direction and the axis it faces.
pub fn facing(yaw: f32) -> (&'static str, &'static str) {
    match ((f64::from(yaw) / 90.0 + 0.5).floor() as i32) & 3 {
        0 => ("south", "+Z"),
        1 => ("west", "-X"),
        2 => ("north", "-Z"),
        _ => ("east", "+X"),
    }
}
/// `minecraft:plains` as `plains`; other namespaces are kept.
pub fn short_id(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}
/// The day and the time of day (`06:00` at tick 0, as a bed's clock).
pub fn day_clock(day_time: i64) -> (i64, String) {
    let day = day_time.div_euclid(24_000);
    let tick = day_time.rem_euclid(24_000);
    let minutes = (tick * 60 / 1000 + 6 * 60) % (24 * 60);
    (day, format!("{:02}:{:02}", minutes / 60, minutes % 60))
}
/// A byte count as MiB.
pub fn mebibytes(bytes: u64) -> String {
    format!("{} MiB", bytes / (1024 * 1024))
}
/// `tasklist /FO CSV /NH`'s last field (`"512,345 K"`, in any locale's
/// digit grouping) as bytes.
pub fn parse_tasklist(output: &str) -> Option<u64> {
    let row = output.lines().find(|l| l.starts_with('"'))?;
    let field = row
        .trim_end()
        .trim_end_matches('"')
        .rsplit("\",\"")
        .next()?;
    let digits: String = field.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<u64>().ok().map(|kib| kib * 1024)
}

fn xyz(p: [i32; 3]) -> String {
    format!("{}, {}, {}", p[0], p[1], p[2])
}

/// Module `id`'s lines for this frame; none when it has nothing to show
/// (nothing targeted, no world).
pub fn module_lines(id: ModuleId, s: &Snapshot, compact: bool) -> Vec<Line> {
    if s.reduced && id.reduced_hides() {
        return Vec::new();
    }
    let mut out = Vec::new();
    match id {
        ModuleId::Fps => {
            if let Some(f) = &s.fps {
                if compact {
                    out.push(line("FPS", f.fps.to_string()));
                } else {
                    let limit = f.limit.map_or("inf".to_owned(), |l| l.to_string());
                    out.push(line(
                        "FPS",
                        format!("{} (T: {limit}{} @{}Hz)", f.fps, f.present, f.refresh),
                    ));
                }
            }
        }
        ModuleId::Server => {
            if !compact {
                out.push(line("Minecraft", s.version.clone()));
            }
            if let Some(v) = &s.server {
                let what = match (&v.brand, v.tick_ms) {
                    (None, Some(ms)) if compact => format!("{ms:.1} ms/tick{}", v.status),
                    (None, Some(ms)) => format!(
                        "integrated @ {ms:.1}/{} ms{}, {} tx, {} rx",
                        v.target, v.status, v.tx, v.rx
                    ),
                    (Some(brand), _) if compact => format!("\"{brand}\"{}", v.status),
                    (brand, _) => format!(
                        "\"{}\"{}, {} tx, {} rx",
                        brand.as_deref().unwrap_or("?"),
                        v.status,
                        v.tx,
                        v.rx
                    ),
                };
                out.push(line("Server", what));
            }
        }
        ModuleId::Coordinates => {
            if let Some(p) = &s.player {
                let [x, y, z] = p.pos;
                if compact {
                    out.push(line("XYZ", format!("{x:.1} / {y:.1} / {z:.1}")));
                } else {
                    let b = p.pos.map(|v| v.floor() as i32);
                    out.push(line("XYZ", format!("{x:.3} / {y:.5} / {z:.3}")));
                    out.push(line("Block", format!("{} {} {}", b[0], b[1], b[2])));
                }
            }
        }
        ModuleId::Chunk => {
            if let Some(p) = &s.player {
                let b = p.pos.map(|v| v.floor() as i32);
                let (cx, sy, cz) = (b[0] >> 4, b[1] >> 4, b[2] >> 4);
                let relative = format!("{:02} {:02} {:02}", b[0] & 15, b[1] & 15, b[2] & 15);
                if s.reduced {
                    out.push(line("Section-relative", relative));
                } else if compact {
                    out.push(line(
                        "Chunk",
                        format!("{cx} {sy} {cz} in r.{}.{}", cx >> 5, cz >> 5),
                    ));
                } else {
                    out.push(line("Chunk", format!("{cx} {sy} {cz}")));
                    out.push(line(
                        "Region",
                        format!("r.{}.{}.mca [{} {}]", cx >> 5, cz >> 5, cx & 31, cz & 31),
                    ));
                    out.push(line("Section-relative", relative));
                }
            }
        }
        ModuleId::Facing => {
            if let Some(p) = &s.player {
                let (direction, axis) = facing(p.yaw);
                if compact {
                    out.push(line("Facing", format!("{direction} ({axis})")));
                } else {
                    out.push(line(
                        "Facing",
                        format!(
                            "{direction} ({axis}) ({:.1} / {:.1})",
                            wrap_degrees(p.yaw),
                            wrap_degrees(p.pitch)
                        ),
                    ));
                }
            }
        }
        ModuleId::World => {
            if s.player.is_some() {
                let clock = s.day_time.map(day_clock);
                if compact {
                    let day = clock.map_or(String::new(), |(d, t)| format!(", day {d} {t}"));
                    out.push(line("World", format!("{}{day}", short_id(&s.dimension))));
                } else {
                    out.push(line("Dimension", s.dimension.clone()));
                    out.push(line("Forced chunks", s.forced_chunks.to_string()));
                    if let Some((day, time)) = clock {
                        out.push(line("Day", format!("{day} ({time})")));
                    }
                }
            }
        }
        ModuleId::Biome => {
            if let Some(b) = &s.biome {
                let name = if compact { short_id(b) } else { b };
                out.push(line("Biome", name));
            }
        }
        ModuleId::Light => {
            if let Some([sky, block]) = s.light {
                if compact {
                    out.push(line("Light", format!("{} ({sky}/{block})", sky.max(block))));
                } else {
                    out.push(line("Light", format!("{sky} sky, {block} block")));
                }
            }
        }
        ModuleId::Charts => {
            if !compact {
                out.extend(s.charts.iter().map(|c| line("", c.clone())));
            }
        }
        ModuleId::System => {
            let sys = &s.system;
            if let Some(bytes) = sys.memory {
                out.push(line("Memory", mebibytes(bytes)));
            }
            if !compact && let Some(cpu) = &sys.cpu {
                out.push(line("CPU", cpu.clone()));
            }
            if let Some(gpu) = &sys.gpu {
                out.push(line("GPU", gpu.clone()));
            }
            if !compact {
                for (label, v) in [
                    ("Driver", &sys.driver),
                    ("Display", &sys.display),
                    ("Window", &sys.window),
                ] {
                    if let Some(v) = v {
                        out.push(line(label, v.clone()));
                    }
                }
            }
        }
        ModuleId::Rendering => {
            if !compact {
                out.extend(s.rendering.iter().map(|r| line("", r.clone())));
            }
        }
        ModuleId::TargetBlock => {
            if let Some(t) = &s.target_block {
                if compact {
                    let at = t.pos;
                    out.push(line(
                        "Block",
                        format!("{} @ {} {} {}", short_id(&t.name), at[0], at[1], at[2]),
                    ));
                } else {
                    out.push(line("Targeted Block", xyz(t.pos)));
                    out.push(line("", t.name.clone()));
                    out.extend(t.properties.iter().map(|(k, v)| line(k, v.clone())));
                }
            }
        }
        ModuleId::TargetFluid => {
            if let Some(t) = &s.target_fluid {
                let name = match (t.lava, t.source) {
                    (false, true) => "minecraft:water",
                    (false, false) => "minecraft:flowing_water",
                    (true, true) => "minecraft:lava",
                    (true, false) => "minecraft:flowing_lava",
                };
                if compact {
                    let at = t.pos;
                    out.push(line(
                        "Fluid",
                        format!("{} @ {} {} {}", short_id(name), at[0], at[1], at[2]),
                    ));
                } else {
                    out.push(line("Targeted Fluid", xyz(t.pos)));
                    out.push(line("", name));
                    out.push(line("amount", t.amount.to_string()));
                    out.push(line("falling", t.falling.to_string()));
                }
            }
        }
        ModuleId::TargetEntity => {
            if let Some(e) = &s.target_entity {
                if compact {
                    out.push(line("Entity", short_id(e)));
                } else {
                    out.push(line("Targeted Entity", e.clone()));
                }
            }
        }
    }
    out
}

/// A line as drawn: its label (with `: `) and value, in their colours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Styled {
    pub label: String,
    pub value: String,
    pub label_rgb: u32,
    pub value_rgb: u32,
}

/// The two columns, top to bottom; None is an empty line between modules.
pub fn columns(config: &Config, s: &Snapshot) -> [Vec<Option<Styled>>; 2] {
    let mut out: [Vec<Option<Styled>>; 2] = [Vec::new(), Vec::new()];
    for m in config.modules.iter().filter(|m| m.shown) {
        let lines = module_lines(m.id, s, config.compact);
        if lines.is_empty() {
            continue;
        }
        let column = &mut out[m.column.index()];
        if config.spacing && !column.is_empty() {
            column.push(None);
        }
        column.extend(lines.into_iter().map(|l| {
            Some(Styled {
                label: if l.label.is_empty() {
                    String::new()
                } else {
                    format!("{}: ", l.label)
                },
                value: l.value,
                label_rgb: m.label,
                value_rgb: m.value,
            })
        }));
    }
    out
}

/// A line placed on the screen (debug-scale GUI pixels).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub line: Styled,
}

/// Where each line goes, as the vanilla screen places them: from 2 pixels
/// down, a line every 9; the left column 2 pixels from the left edge, the
/// right one right-aligned 2 pixels from the right edge of a screen
/// `screen_width` wide. `width_of` measures a text.
pub fn layout(
    columns: &[Vec<Option<Styled>>; 2],
    screen_width: i32,
    width_of: impl Fn(&str) -> i32,
) -> Vec<Placed> {
    let mut out = Vec::new();
    for (c, lines) in columns.iter().enumerate() {
        for (i, l) in lines.iter().enumerate() {
            let Some(l) = l else { continue };
            let width = width_of(&format!("{}{}", l.label, l.value));
            let x = if c == 0 { 2 } else { screen_width - 2 - width };
            out.push(Placed {
                x,
                y: 2 + LINE_HEIGHT * i as i32,
                width,
                line: l.clone(),
            });
        }
    }
    out
}

// ---------------------------------------------------------------- settings page

/// What a click on the settings page does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Keep this configuration and put it in effect.
    Save(Config),
    /// Read the file again.
    Reload,
    /// Show this module's buttons.
    Select(ModuleId),
}

fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}
fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// The settings page's buttons, the module `selected`'s among them.
pub fn labels(loaded: &Loaded, selected: ModuleId) -> Vec<String> {
    let config = match loaded {
        Ok(config) => config,
        Err(error) => {
            return vec![
                format!("{CONFIG_FILE}: {error} (click to read it again)"),
                "Reset to defaults".to_owned(),
            ];
        }
    };
    let m = config
        .module(selected)
        .cloned()
        .unwrap_or_else(|| selected.default_config());
    let index = ModuleId::ALL
        .iter()
        .position(|&x| x == selected)
        .unwrap_or(0);
    let title = selected.title();
    let (place, count) = config.place(selected);
    let column = match m.column {
        Column::Left => "left",
        Column::Right => "right",
    };
    vec![
        format!(
            "Debug screen: {}",
            if config.enabled { NAME } else { "vanilla F3" }
        ),
        format!(
            "Layout: {}",
            if config.compact { "compact" } else { "full" }
        ),
        format!("Text shadow: {}", on_off(config.shadow)),
        format!("Background: {}", config.background.name()),
        format!("Gap between modules: {}", on_off(config.spacing)),
        format!(
            "Module {}/{}: {title} (click for the next)",
            index + 1,
            ModuleId::ALL.len()
        ),
        format!("  {title}: {}", if m.shown { "shown" } else { "hidden" }),
        format!("  Label colour: {}", color_name(m.label)),
        format!("  Value colour: {}", color_name(m.value)),
        format!("  Column: {column}"),
        if place > 1 {
            format!(
                "  Move up ({} of {count} in the {column} column)",
                ordinal(place)
            )
        } else {
            format!("  Move up (already first in the {column} column)")
        },
        if place < count {
            format!(
                "  Move down ({} of {count} in the {column} column)",
                ordinal(place)
            )
        } else {
            format!("  Move down (already last in the {column} column)")
        },
        "Reset to defaults".to_owned(),
    ]
}

/// A click on button `button` of the page `labels` made.
pub fn click(loaded: &Loaded, selected: ModuleId, button: usize) -> Option<Effect> {
    let config = match loaded {
        Ok(config) => config,
        Err(_) => {
            return match button {
                0 => Some(Effect::Reload),
                1 => Some(Effect::Save(Config::default())),
                _ => None,
            };
        }
    };
    let mut next = config.clone();
    match button {
        0 => next.enabled = !next.enabled,
        1 => next.compact = !next.compact,
        2 => next.shadow = !next.shadow,
        3 => {
            let at = Background::ALL
                .iter()
                .position(|&b| b == next.background)
                .unwrap_or(0);
            next.background = Background::ALL[(at + 1) % Background::ALL.len()];
        }
        4 => next.spacing = !next.spacing,
        5 => {
            let at = ModuleId::ALL
                .iter()
                .position(|&x| x == selected)
                .unwrap_or(0);
            return Some(Effect::Select(
                ModuleId::ALL[(at + 1) % ModuleId::ALL.len()],
            ));
        }
        6..=9 => {
            let m = next.module_mut(selected)?;
            match button {
                6 => m.shown = !m.shown,
                7 => m.label = next_color(m.label),
                8 => m.value = next_color(m.value),
                _ => {
                    m.column = match m.column {
                        Column::Left => Column::Right,
                        Column::Right => Column::Left,
                    }
                }
            }
        }
        10 | 11 => {
            if !next.shift(selected, button == 10) {
                return None;
            }
        }
        12 => next = Config::default(),
        _ => return None,
    }
    Some(Effect::Save(next))
}

// ---------------------------------------------------------------- /smartf3

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    /// SmartF3 draws F3.
    On,
    /// The vanilla debug screen.
    Off,
    Toggle,
    Compact,
    Reload,
}

pub const USAGE: &str = "Usage: /smartf3 [status|on|off|toggle|compact|reload]";

/// `smartf3 ...` (the line without its slash): None when it is not ours,
/// Err with the usage when its argument is unknown.
pub fn parse_command(command: &str) -> Option<Result<Command, String>> {
    let mut words = command.split_whitespace();
    if words.next()? != ID {
        return None;
    }
    let parsed = match (words.next(), words.next()) {
        (None | Some("status"), None) => Ok(Command::Status),
        (Some("on"), None) => Ok(Command::On),
        (Some("off" | "vanilla"), None) => Ok(Command::Off),
        (Some("toggle"), None) => Ok(Command::Toggle),
        (Some("compact"), None) => Ok(Command::Compact),
        (Some("reload"), None) => Ok(Command::Reload),
        _ => Err(USAGE.to_owned()),
    };
    Some(parsed)
}

/// What the screen is now, for `/smartf3 status`.
pub fn status(config: &Config) -> String {
    if config.enabled {
        format!(
            "{NAME} draws the F3 screen ({} layout). /smartf3 off for vanilla F3.",
            if config.compact { "compact" } else { "full" }
        )
    } else {
        format!("F3 shows the vanilla debug screen. /smartf3 on for {NAME}.")
    }
}

/// Runs a command other than `Reload` on `config`: the configuration to
/// keep (None: unchanged) and the answer.
pub fn run_command(config: &Config, command: Command) -> (Option<Config>, String) {
    let mut next = config.clone();
    match command {
        Command::Status | Command::Reload => return (None, status(config)),
        Command::On => next.enabled = true,
        Command::Off => next.enabled = false,
        Command::Toggle => next.enabled = !next.enabled,
        Command::Compact => next.compact = !next.compact,
    }
    let answer = status(&next);
    (Some(next), answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            reduced: false,
            fps: Some(Fps {
                fps: 112,
                limit: Some(120),
                present: " (fifo)".into(),
                refresh: "165".into(),
            }),
            version: "26.3 (26.3/horizon)".into(),
            server: Some(Server {
                brand: None,
                tick_ms: Some(6.6),
                target: "50.0".into(),
                status: String::new(),
                tx: 28,
                rx: 492,
            }),
            player: Some(Player {
                pos: [108.5, 104.0, -51.5],
                yaw: -1.0,
                pitch: 7.1,
            }),
            dimension: "minecraft:overworld".into(),
            forced_chunks: 0,
            day_time: Some(30_000),
            biome: Some("minecraft:plains".into()),
            light: Some([15, 3]),
            charts: vec!["Debug charts: ...".into()],
            system: System {
                memory: Some(812 * 1024 * 1024),
                cpu: Some("12x Ryzen".into()),
                gpu: Some("RTX (dGPU)".into()),
                driver: Some("Vulkan 580".into()),
                display: Some("1920x1080 (NVIDIA Corporation)".into()),
                window: None,
            },
            rendering: vec!["Filtering: None".into()],
            target_block: Some(TargetBlock {
                pos: [108, 103, -52],
                name: "minecraft:oak_log".into(),
                properties: vec![("axis".into(), "y".into())],
            }),
            target_fluid: None,
            target_entity: Some("minecraft:zombie".into()),
        }
    }

    fn values(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| {
                if l.label.is_empty() {
                    l.value.clone()
                } else {
                    format!("{}: {}", l.label, l.value)
                }
            })
            .collect()
    }

    #[test]
    fn position_modules_match_the_vanilla_numbers() {
        let s = snapshot();
        assert_eq!(
            values(&module_lines(ModuleId::Coordinates, &s, false)),
            ["XYZ: 108.500 / 104.00000 / -51.500", "Block: 108 104 -52"]
        );
        assert_eq!(
            values(&module_lines(ModuleId::Chunk, &s, false)),
            [
                "Chunk: 6 6 -4",
                "Region: r.0.-1.mca [6 28]",
                "Section-relative: 12 08 12"
            ]
        );
        assert_eq!(
            values(&module_lines(ModuleId::Facing, &s, false)),
            ["Facing: south (+Z) (-1.0 / 7.1)"]
        );
        assert_eq!(facing(450.0), ("west", "-X"));
        assert_eq!(facing(180.0), ("north", "-Z"));
        assert_eq!(facing(-90.0), ("east", "+X"));
    }

    #[test]
    fn compact_mode_shortens_every_module() {
        let s = snapshot();
        let compact = |id| values(&module_lines(id, &s, true));
        assert_eq!(compact(ModuleId::Fps), ["FPS: 112"]);
        assert_eq!(
            compact(ModuleId::Coordinates),
            ["XYZ: 108.5 / 104.0 / -51.5"]
        );
        assert_eq!(compact(ModuleId::Chunk), ["Chunk: 6 6 -4 in r.0.-1"]);
        assert_eq!(compact(ModuleId::Facing), ["Facing: south (+Z)"]);
        assert_eq!(compact(ModuleId::World), ["World: overworld, day 1 12:00"]);
        assert_eq!(compact(ModuleId::Biome), ["Biome: plains"]);
        assert_eq!(compact(ModuleId::Light), ["Light: 15 (15/3)"]);
        assert_eq!(compact(ModuleId::Server), ["Server: 6.6 ms/tick"]);
        assert_eq!(
            compact(ModuleId::System),
            ["Memory: 812 MiB", "GPU: RTX (dGPU)"]
        );
        assert_eq!(
            compact(ModuleId::TargetBlock),
            ["Block: oak_log @ 108 103 -52"]
        );
        assert_eq!(compact(ModuleId::TargetEntity), ["Entity: zombie"]);
        assert!(compact(ModuleId::Charts).is_empty());
        assert!(compact(ModuleId::Rendering).is_empty());
    }

    #[test]
    fn full_modules_carry_the_vanilla_information() {
        let s = snapshot();
        let full = |id| values(&module_lines(id, &s, false));
        assert_eq!(full(ModuleId::Fps), ["FPS: 112 (T: 120 (fifo) @165Hz)"]);
        assert_eq!(
            full(ModuleId::Server),
            [
                "Minecraft: 26.3 (26.3/horizon)",
                "Server: integrated @ 6.6/50.0 ms, 28 tx, 492 rx"
            ]
        );
        assert_eq!(
            full(ModuleId::World),
            [
                "Dimension: minecraft:overworld",
                "Forced chunks: 0",
                "Day: 1 (12:00)"
            ]
        );
        assert_eq!(full(ModuleId::Light), ["Light: 15 sky, 3 block"]);
        assert_eq!(
            full(ModuleId::TargetBlock),
            [
                "Targeted Block: 108, 103, -52",
                "minecraft:oak_log",
                "axis: y"
            ]
        );
        assert_eq!(
            full(ModuleId::System),
            [
                "Memory: 812 MiB",
                "CPU: 12x Ryzen",
                "GPU: RTX (dGPU)",
                "Driver: Vulkan 580",
                "Display: 1920x1080 (NVIDIA Corporation)"
            ]
        );
        let mut remote = s.clone();
        remote.server = Some(Server {
            brand: Some("vanilla".into()),
            tick_ms: None,
            target: "50.0".into(),
            status: " (frozen)".into(),
            tx: 1,
            rx: 2,
        });
        assert_eq!(
            values(&module_lines(ModuleId::Server, &remote, false))[1],
            "Server: \"vanilla\" (frozen), 1 tx, 2 rx"
        );
        let mut fluid = s.clone();
        fluid.target_fluid = Some(TargetFluid {
            pos: [1, 2, 3],
            lava: false,
            amount: 5,
            falling: false,
            source: false,
        });
        assert_eq!(
            values(&module_lines(ModuleId::TargetFluid, &fluid, false)),
            [
                "Targeted Fluid: 1, 2, 3",
                "minecraft:flowing_water",
                "amount: 5",
                "falling: false"
            ]
        );
    }

    #[test]
    fn nothing_targeted_or_no_world_shows_nothing() {
        let s = Snapshot::default();
        for id in ModuleId::ALL {
            if id != ModuleId::Server {
                assert!(module_lines(id, &s, false).is_empty(), "{id:?}");
            }
        }
    }

    #[test]
    fn reduced_debug_info_hides_where_you_are() {
        let mut s = snapshot();
        s.reduced = true;
        assert!(module_lines(ModuleId::Coordinates, &s, false).is_empty());
        assert!(module_lines(ModuleId::Facing, &s, false).is_empty());
        assert!(module_lines(ModuleId::TargetBlock, &s, false).is_empty());
        assert!(module_lines(ModuleId::TargetEntity, &s, false).is_empty());
        assert_eq!(
            values(&module_lines(ModuleId::Chunk, &s, false)),
            ["Section-relative: 12 08 12"]
        );
        assert!(!module_lines(ModuleId::Fps, &s, false).is_empty());
    }

    #[test]
    fn columns_follow_the_order_and_gap_setting() {
        let s = snapshot();
        let mut config = Config::default();
        for m in &mut config.modules {
            m.shown = matches!(
                m.id,
                ModuleId::Fps | ModuleId::Biome | ModuleId::TargetEntity
            );
        }
        let [left, right] = columns(&config, &s);
        let text = |c: &[Option<Styled>]| -> Vec<String> {
            c.iter()
                .map(|l| {
                    l.as_ref()
                        .map_or(String::new(), |l| l.label.clone() + &l.value)
                })
                .collect()
        };
        assert_eq!(
            text(&left),
            [
                "FPS: 112 (T: 120 (fifo) @165Hz)",
                "",
                "Biome: minecraft:plains"
            ]
        );
        assert_eq!(text(&right), ["Targeted Entity: minecraft:zombie"]);
        assert_eq!(left[0].as_ref().map(|l| l.label_rgb), Some(0x55_FF55));
        assert_eq!(left[0].as_ref().map(|l| l.value_rgb), Some(0xE0_E0E0));

        config.spacing = false;
        // Up past the hidden modules too: the page counts every module of a column.
        while config.shift(ModuleId::Biome, true) {}
        let [left, _] = columns(&config, &s);
        assert_eq!(
            text(&left),
            ["Biome: minecraft:plains", "FPS: 112 (T: 120 (fifo) @165Hz)"]
        );
    }

    #[test]
    fn layout_places_lines_as_vanilla() {
        let mut config = Config::default();
        for m in &mut config.modules {
            m.shown = matches!(
                m.id,
                ModuleId::Fps | ModuleId::Biome | ModuleId::TargetEntity
            );
        }
        let placed = layout(&columns(&config, &snapshot()), 400, |t| t.len() as i32);
        let at: Vec<(i32, i32, i32)> = placed.iter().map(|p| (p.x, p.y, p.width)).collect();
        assert_eq!(at, [(2, 2, 31), (2, 20, 23), (400 - 2 - 33, 2, 33)]);
    }

    #[test]
    fn shifting_stays_within_a_column() {
        let mut config = Config::default();
        assert_eq!(config.place(ModuleId::Fps), (1, 9));
        assert!(!config.shift(ModuleId::Fps, true));
        assert_eq!(config.place(ModuleId::System), (1, 5));
        // Server is second on the left; System (right) sits between nothing.
        assert!(config.shift(ModuleId::Server, true));
        assert_eq!(config.place(ModuleId::Server), (1, 9));
        assert!(!config.shift(ModuleId::TargetEntity, false));
        assert!(config.shift(ModuleId::System, false));
        assert_eq!(config.place(ModuleId::System), (2, 5));
        assert_eq!(config.place(ModuleId::Rendering), (1, 5));
        assert_eq!(config.place(ModuleId::Charts), (9, 9));
    }

    #[test]
    fn the_file_round_trips_and_fills_in_defaults() {
        let mut config = Config {
            compact: true,
            background: Background::Dark,
            ..Config::default()
        };
        config.modules[0].label = 0x12_3456;
        config.modules.swap(0, 1);
        let text = config.to_json();
        assert_eq!(Config::from_json(&text), Ok(config));
        assert!(text.contains("\"label\": \"#123456\""));
        assert!(text.contains("\"label\": \"light_purple\""));

        assert_eq!(Config::from_json("{}"), Ok(Config::default()));
        let partial = Config::from_json(
            r#"{"modules": [{"id": "biome", "column": "right", "value": "GOLD"}]}"#,
        )
        .unwrap();
        assert_eq!(partial.modules[0].id, ModuleId::Biome);
        assert_eq!(partial.modules[0].column, Column::Right);
        assert_eq!(partial.modules[0].value, 0xFF_AA00);
        assert_eq!(partial.modules.len(), ModuleId::ALL.len());
    }

    #[test]
    fn a_wrong_file_is_refused_with_a_reason() {
        let error = |text: &str| Config::from_json(text).unwrap_err();
        assert_eq!(error(r#"{"colour": 1}"#), "unknown key \"colour\"");
        assert_eq!(
            error(r#"{"shadow": "yes"}"#),
            "\"shadow\" must be true or false"
        );
        assert!(error(r#"{"background": "pink"}"#).contains("unknown background"));
        assert_eq!(
            error(r#"{"modules": [{"id": "fps"}, {"id": "fps"}]}"#),
            "module \"fps\" is listed twice"
        );
        assert_eq!(
            error(r#"{"modules": [{"id": "map"}]}"#),
            "unknown module \"map\""
        );
        assert_eq!(
            error(r##"{"modules": [{"id": "fps", "label": "#12345"}]}"##),
            "fps: unknown label colour \"#12345\""
        );
        assert!(error("[]").contains("JSON object"));
        assert!(Config::from_json("{").is_err());
    }

    #[test]
    fn colours_parse_and_cycle() {
        assert_eq!(parse_color("gold"), Some(0xFF_AA00));
        assert_eq!(parse_color("Dark_Aqua"), Some(0x00_AAAA));
        assert_eq!(parse_color("#00ff7f"), Some(0x00_FF7F));
        assert_eq!(parse_color("#00ff7"), None);
        assert_eq!(parse_color("pink"), None);
        assert_eq!(color_name(0xE0_E0E0), "F3 Grey");
        assert_eq!(color_name(0x12_3456), "#123456");
        assert_eq!(next_color(0xFF_FFFF), 0xE0_E0E0);
        assert_eq!(next_color(0xAA_00AA), 0xFF_FFFF);
        assert_eq!(next_color(0x12_3456), 0xFF_FFFF);
    }

    #[test]
    fn the_settings_page_changes_the_selected_module() {
        let loaded: Loaded = Ok(Config::default());
        let page = labels(&loaded, ModuleId::Biome);
        assert_eq!(page.len(), 13);
        assert_eq!(page[0], "Debug screen: SmartF3");
        assert_eq!(page[5], "Module 7/14: Biome (click for the next)");
        assert_eq!(page[7], "  Label colour: Dark Green");
        assert_eq!(page[10], "  Move up (7th of 9 in the left column)");
        assert_eq!(
            click(&loaded, ModuleId::Biome, 5),
            Some(Effect::Select(ModuleId::Light))
        );
        assert_eq!(
            click(&loaded, ModuleId::TargetEntity, 5),
            Some(Effect::Select(ModuleId::Fps))
        );
        let Some(Effect::Save(c)) = click(&loaded, ModuleId::Biome, 6) else {
            panic!("hide")
        };
        assert!(!c.wants(ModuleId::Biome));
        let Some(Effect::Save(c)) = click(&loaded, ModuleId::Biome, 9) else {
            panic!("column")
        };
        assert_eq!(
            c.module(ModuleId::Biome).map(|m| m.column),
            Some(Column::Right)
        );
        let Some(Effect::Save(c)) = click(&loaded, ModuleId::Biome, 0) else {
            panic!("vanilla")
        };
        assert!(!c.enabled);
        assert_eq!(labels(&Ok(c), ModuleId::Fps)[0], "Debug screen: vanilla F3");
        let Some(Effect::Save(c)) = click(&loaded, ModuleId::Fps, 3) else {
            panic!("background")
        };
        assert_eq!(c.background, Background::Dark);
        assert_eq!(click(&loaded, ModuleId::Fps, 10), None);
        assert_eq!(
            labels(&loaded, ModuleId::Fps)[10],
            "  Move up (already first in the left column)"
        );
        assert_eq!(
            click(&loaded, ModuleId::Fps, 12),
            Some(Effect::Save(Config::default()))
        );
        assert_eq!(click(&loaded, ModuleId::Fps, 13), None);
    }

    #[test]
    fn a_broken_file_offers_a_reload_and_a_reset() {
        let loaded: Loaded = Err("unknown key \"x\"".into());
        assert_eq!(
            labels(&loaded, ModuleId::Fps),
            [
                "config.json: unknown key \"x\" (click to read it again)",
                "Reset to defaults"
            ]
        );
        assert_eq!(click(&loaded, ModuleId::Fps, 0), Some(Effect::Reload));
        assert_eq!(
            click(&loaded, ModuleId::Fps, 1),
            Some(Effect::Save(Config::default()))
        );
    }

    #[test]
    fn commands_switch_between_the_screens() {
        assert_eq!(parse_command("smartf3"), Some(Ok(Command::Status)));
        assert_eq!(parse_command("smartf3  off"), Some(Ok(Command::Off)));
        assert_eq!(parse_command("smartf3 vanilla"), Some(Ok(Command::Off)));
        assert_eq!(parse_command("smartf3 compact"), Some(Ok(Command::Compact)));
        assert_eq!(parse_command("smartf3 on now"), Some(Err(USAGE.into())));
        assert_eq!(parse_command("smartf3x"), None);
        assert_eq!(parse_command("tp 0 0 0"), None);
        let config = Config::default();
        let (next, answer) = run_command(&config, Command::Toggle);
        assert_eq!(next.as_ref().map(|c| c.enabled), Some(false));
        assert!(answer.contains("vanilla"));
        let (next, _) = run_command(&config, Command::Compact);
        assert_eq!(next.map(|c| c.compact), Some(true));
        assert_eq!(run_command(&config, Command::Status).0, None);
    }

    #[test]
    fn small_helpers() {
        assert_eq!(day_clock(0), (0, "06:00".into()));
        assert_eq!(day_clock(18_000), (0, "00:00".into()));
        assert_eq!(day_clock(-1000), (-1, "05:00".into()));
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(mebibytes(3 * 1024 * 1024 + 5), "3 MiB");
        assert_eq!(
            parse_tasklist("\"horizon-client.exe\",\"1234\",\"Console\",\"1\",\"512,345 K\"\r\n"),
            Some(512_345 * 1024)
        );
        assert_eq!(
            parse_tasklist("\"horizon.exe\",\"1\",\"Console\",\"1\",\"1\u{a0}024 KB\""),
            Some(1024 * 1024)
        );
        assert_eq!(parse_tasklist("INFO: No tasks are running."), None);
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(22), "22nd");
    }
}
