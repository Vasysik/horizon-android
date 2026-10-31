//! The debug screen's entries (26.3 `DebugScreenEntries`, `DebugScreenEntryList`):
//! what each F3 line block and debug renderer is, whether it shows always,
//! only in the overlay or never, the two built-in profiles, and
//! `debug-profile.json` beside `options.txt`, which keeps them.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `DebugScreenEntryStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    AlwaysOn,
    InOverlay,
    Never,
}

impl Status {
    /// `getSerializedName`.
    pub fn name(self) -> &'static str {
        match self {
            Status::AlwaysOn => "alwaysOn",
            Status::InOverlay => "inOverlay",
            Status::Never => "never",
        }
    }
    fn parse(name: &str) -> Option<Self> {
        [Status::AlwaysOn, Status::InOverlay, Status::Never]
            .into_iter()
            .find(|s| s.name() == name)
    }
}

/// `DebugScreenProfile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Default,
    Performance,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::Default => "default",
            Profile::Performance => "performance",
        }
    }
    pub fn translation_key(self) -> &'static str {
        match self {
            Profile::Default => "debug.options.profile.default",
            Profile::Performance => "debug.options.profile.performance",
        }
    }
    fn parse(name: &str) -> Option<Self> {
        [Profile::Default, Profile::Performance]
            .into_iter()
            .find(|p| p.name() == name)
    }
    /// `DebugScreenEntries.PROFILES`.
    fn statuses(self) -> &'static [(&'static str, Status)] {
        use Status::*;
        match self {
            Profile::Default => &[
                ("3d_crosshair", InOverlay),
                ("game_version", InOverlay),
                ("tps", InOverlay),
                ("fps", InOverlay),
                ("memory", InOverlay),
                ("system_specs", InOverlay),
                ("player_position", InOverlay),
                ("player_section_position", InOverlay),
                ("simple_performance_impactors", InOverlay),
            ],
            Profile::Performance => &[
                ("tps", InOverlay),
                ("fps", AlwaysOn),
                ("gpu_utilization", InOverlay),
                ("memory", InOverlay),
                ("simple_performance_impactors", InOverlay),
            ],
        }
    }
}

/// `DebugEntryCategory`: the screen's two sections, by `sortKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    /// `SCREEN_TEXT`, `debug.options.category.text`.
    Text,
    /// `RENDERER` (`DebugEntryNoop`), `debug.options.category.renderer`.
    Renderer,
}

impl Category {
    pub fn translation_key(self) -> &'static str {
        match self {
            Category::Text => "debug.options.category.text",
            Category::Renderer => "debug.options.category.renderer",
        }
    }
}

/// One registered entry: its path in the `minecraft` namespace, its section,
/// and whether it shows while debug info is reduced (`isAllowed(true)`).
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub path: &'static str,
    pub category: Category,
    pub reduced: bool,
}

const fn text(path: &'static str, reduced: bool) -> Entry {
    Entry {
        path,
        category: Category::Text,
        reduced,
    }
}
const fn renderer(path: &'static str) -> Entry {
    Entry {
        path,
        category: Category::Renderer,
        reduced: false,
    }
}

/// `DebugScreenEntries`' registrations, in their order.
pub const ENTRIES: [Entry; 46] = [
    text("game_version", true),
    text("fps", true),
    text("tps", true),
    text("memory", true),
    text("detailed_memory", true),
    text("system_specs", true),
    text("looking_at_block_state", false),
    text("looking_at_block_tags", false),
    text("looking_at_fluid_state", false),
    text("looking_at_fluid_tags", false),
    text("looking_at_entity", false),
    text("looking_at_entity_tags", false),
    text("chunk_render_stats", true),
    text("chunk_generation_stats", false),
    text("entity_render_stats", true),
    text("particle_render_stats", false),
    text("chunk_source_stats", true),
    text("player_position", false),
    text("player_section_position", true),
    text("player_speed", false),
    text("light_levels", false),
    text("heightmap", false),
    text("biome", false),
    text("local_difficulty", false),
    text("day_count", false),
    text("entity_spawn_counts", false),
    text("sound_mood", false),
    text("sound_cache", true),
    text("post_effects", false),
    renderer("entity_hitboxes"),
    renderer("chunk_borders"),
    renderer("3d_crosshair"),
    renderer("chunk_section_paths"),
    text("gpu_utilization", true),
    text("simple_performance_impactors", true),
    renderer("chunk_section_octree"),
    renderer("visualize_water_levels"),
    renderer("visualize_heightmap"),
    renderer("visualize_collision_boxes"),
    renderer("visualize_entity_supporting_blocks"),
    renderer("visualize_block_light_levels"),
    renderer("visualize_sky_light_levels"),
    renderer("visualize_solid_faces"),
    renderer("visualize_chunks_on_server"),
    renderer("visualize_sky_light_sections"),
    renderer("chunk_section_visibility"),
];

/// `DebugScreenEntries.getEntry` by path.
pub fn entry(path: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.path == path)
}

/// `DebugOptionsScreen.OptionList.updateSearch`: the entries whose path
/// contains `search`, by category then identifier.
pub fn search(search: &str) -> Vec<&'static Entry> {
    let mut all: Vec<&Entry> = ENTRIES.iter().filter(|e| e.path.contains(search)).collect();
    all.sort_by(|a, b| a.category.cmp(&b.category).then(a.path.cmp(b.path)));
    all
}

/// `DebugScreenEntryList`: every entry's status (by full identifier, as the
/// file keeps them) and the profile they came from if unchanged. Whether the
/// F3 overlay is open (`isOverlayVisible`) is the game's `debug`.
#[derive(Clone, Debug)]
pub struct DebugEntries {
    statuses: BTreeMap<String, Status>,
    profile: Option<Profile>,
    file: Option<PathBuf>,
}

impl Default for DebugEntries {
    fn default() -> Self {
        let mut entries = Self {
            statuses: BTreeMap::new(),
            profile: None,
            file: None,
        };
        entries.reset_to_profile(Profile::Default);
        entries
    }
}

fn identifier(path: &str) -> String {
    format!("minecraft:{path}")
}

impl DebugEntries {
    /// `DebugScreenEntryList.load` from `debug-profile.json` in `dir`: the
    /// default profile without a file; a file that does not parse is reset
    /// to it and saved again.
    pub fn load(dir: &Path) -> Self {
        let file = dir.join("debug-profile.json");
        let mut entries = Self {
            file: Some(file.clone()),
            ..Self::default()
        };
        let Ok(text) = std::fs::read_to_string(&file) else {
            return entries;
        };
        match Self::parse(&text) {
            Some((Some(profile), _)) => entries.reset_to_profile(profile),
            Some((None, custom)) => {
                entries.statuses = custom;
                entries.profile = None;
            }
            None => {
                log::error!(
                    "Couldn't read debug profile file {}, resetting to default",
                    file.display()
                );
                entries.save();
            }
        }
        entries
    }
    /// `SerializedOptions.CODEC`: an optional `profile` name and an optional
    /// `custom` map of identifiers to statuses.
    fn parse(text: &str) -> Option<(Option<Profile>, BTreeMap<String, Status>)> {
        let json: serde_json::Value = serde_json::from_str(text).ok()?;
        let object = json.as_object()?;
        let profile = match object.get("profile") {
            Some(name) => Some(Profile::parse(name.as_str()?)?),
            None => None,
        };
        let mut custom = BTreeMap::new();
        if let Some(map) = object.get("custom") {
            for (key, status) in map.as_object()? {
                // Identifier.CODEC: a bare path is in the minecraft namespace.
                let key = if key.contains(':') {
                    key.clone()
                } else {
                    identifier(key)
                };
                custom.insert(key, Status::parse(status.as_str()?)?);
            }
        }
        Some((profile, custom))
    }
    /// `DebugScreenEntryList.save`: the profile's name while it is unchanged,
    /// else every status, with the data version the codec wraps it in.
    pub fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let mut json = serde_json::Map::new();
        json.insert(
            "DataVersion".into(),
            horizon_core::version::DATA_VERSION.into(),
        );
        match self.profile {
            Some(profile) => {
                json.insert("profile".into(), profile.name().into());
            }
            None => {
                let custom: serde_json::Map<String, serde_json::Value> = self
                    .statuses
                    .iter()
                    .map(|(k, s)| (k.clone(), s.name().into()))
                    .collect();
                json.insert("custom".into(), custom.into());
            }
        }
        if let Err(e) = std::fs::write(file, serde_json::Value::Object(json).to_string()) {
            log::error!("Failed to save debug profile file {}: {e}", file.display());
        }
    }
    fn reset_to_profile(&mut self, profile: Profile) {
        self.profile = Some(profile);
        self.statuses = profile
            .statuses()
            .iter()
            .map(|(path, status)| (identifier(path), *status))
            .collect();
    }
    /// `loadProfile` then `save` (the screen's profile buttons).
    pub fn load_profile(&mut self, profile: Profile) {
        self.reset_to_profile(profile);
        self.save();
    }
    pub fn is_using_profile(&self, profile: Profile) -> bool {
        self.profile == Some(profile)
    }
    /// `getStatus`: never for an entry the statuses leave out.
    pub fn status(&self, path: &str) -> Status {
        self.statuses
            .get(&identifier(path))
            .copied()
            .unwrap_or(Status::Never)
    }
    /// `setStatus`: the statuses are custom from now on, and saved.
    pub fn set_status(&mut self, path: &str, status: Status) {
        self.profile = None;
        self.statuses.insert(identifier(path), status);
        self.save();
    }
    /// `toggleStatus` (F3+B, F3+G) with the overlay open or not: whether the
    /// entry is on afterwards.
    pub fn toggle_status(&mut self, path: &str, overlay: bool) -> bool {
        let (status, on) = match self.statuses.get(&identifier(path)) {
            Some(Status::AlwaysOn) => (Status::Never, false),
            Some(Status::InOverlay) if overlay => (Status::Never, false),
            Some(Status::Never) if overlay => (Status::InOverlay, true),
            _ => (Status::AlwaysOn, true),
        };
        self.set_status(path, status);
        on
    }
    /// `isCurrentlyEnabled` (`rebuildCurrentList`): on always, or in the
    /// `overlay` while it is open, and allowed with the debug info `reduced`.
    pub fn enabled(&self, path: &str, overlay: bool, reduced: bool) -> bool {
        let shown = match self.status(path) {
            Status::AlwaysOn => true,
            Status::InOverlay => overlay,
            Status::Never => false,
        };
        shown && entry(path).is_some_and(|e| e.reduced || !reduced)
    }
    /// `getCurrentlyEnabled().isEmpty()` negated: something to draw.
    pub fn any_enabled(&self, overlay: bool, reduced: bool) -> bool {
        ENTRIES
            .iter()
            .any(|e| self.enabled(e.path, overlay, reduced))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_follows_the_overlay() {
        let mut e = DebugEntries::default();
        // Not in the default profile: on always while F3 is closed.
        assert!(e.toggle_status("entity_hitboxes", false));
        assert_eq!(e.status("entity_hitboxes"), Status::AlwaysOn);
        assert!(!e.toggle_status("entity_hitboxes", false));
        assert_eq!(e.status("entity_hitboxes"), Status::Never);
        // With F3 open, a hidden entry goes in the overlay only.
        assert!(e.toggle_status("chunk_borders", true));
        assert_eq!(e.status("chunk_borders"), Status::AlwaysOn);
        assert!(!e.toggle_status("chunk_borders", true));
        assert_eq!(e.status("chunk_borders"), Status::Never);
        assert!(e.toggle_status("chunk_borders", true));
        assert_eq!(e.status("chunk_borders"), Status::InOverlay);
        assert!(!e.toggle_status("chunk_borders", true));
        assert!(!e.is_using_profile(Profile::Default));
    }

    #[test]
    fn enabled_needs_the_overlay_and_full_debug_info() {
        let mut e = DebugEntries::default();
        assert!(!e.any_enabled(false, false));
        assert!(e.enabled("player_position", true, false));
        // Reduced debug info hides the position, not the section.
        assert!(!e.enabled("player_position", true, true));
        assert!(e.enabled("player_section_position", true, true));
        e.load_profile(Profile::Performance);
        assert!(e.enabled("fps", false, false));
        assert!(!e.enabled("memory", false, false));
    }

    #[test]
    fn the_file_keeps_a_profile_or_the_statuses() {
        let dir =
            std::env::temp_dir().join(format!("horizon-debug-profile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut e = DebugEntries::load(&dir);
        assert!(e.is_using_profile(Profile::Default));
        e.load_profile(Profile::Performance);
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("debug-profile.json")).unwrap())
                .unwrap();
        assert_eq!(saved["profile"], "performance");
        assert!(DebugEntries::load(&dir).is_using_profile(Profile::Performance));
        e.set_status("biome", Status::AlwaysOn);
        let loaded = DebugEntries::load(&dir);
        assert_eq!(loaded.status("biome"), Status::AlwaysOn);
        assert_eq!(loaded.status("fps"), Status::AlwaysOn);
        assert!(!loaded.is_using_profile(Profile::Performance));
        // A broken file is reset to the default profile.
        std::fs::write(dir.join("debug-profile.json"), "{\"profile\":\"nope\"}").unwrap();
        assert!(DebugEntries::load(&dir).is_using_profile(Profile::Default));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_list_sorts_text_before_renderers() {
        let all = search("");
        assert_eq!(all.len(), 46);
        assert_eq!(all[0].path, "biome");
        let first_renderer = all
            .iter()
            .position(|e| e.category == Category::Renderer)
            .unwrap();
        assert_eq!(all[first_renderer].path, "3d_crosshair");
        assert!(
            all[first_renderer..]
                .iter()
                .all(|e| e.category == Category::Renderer)
        );
        assert_eq!(search("chunk_b").len(), 1);
    }
}
