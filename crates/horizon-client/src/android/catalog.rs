//! Mobile launcher state. Browser operations reuse the desktop backend unchanged.
use crate::browser::{self, Kind, Project, Query, Source, installed::Installed};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub fn source_only_mod(project: &Project) -> bool {
    project.source == browser::SourceId::Horizon
        && project.loaders.iter().any(|loader| loader == "hnmod")
}

/// Temporary dev registration published as Minecraft Nintendo Switch by PrismarineJS.
/// Account tokens and client secrets are never bundled.
pub const MICROSOFT_CLIENT_ID: &str = "00000000441cc96b";

/// Minecraft player names are at most 16 UTF-16 units; a longer name fails the
/// integrated-server handshake with only a generic disconnect.
pub fn truncate_player_name(name: &str) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in name.chars() {
        let w = c.len_utf16();
        if units + w > 16 {
            break;
        }
        units += w;
        out.push(c);
    }
    out
}

/// Real Vulkan rendering paths; these do not select a different driver or API.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererMode {
    Vulkan,
    OpenGl,
    #[default]
    VulkanCompatibility,
}
impl RendererMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan / GPU batching",
            Self::OpenGl => "OpenGL ES / experimental",
            Self::VulkanCompatibility => "Vulkan / compatibility",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub profile: u32,
    pub proxy: Option<String>,
    pub name: String,
    pub server: String,
    #[serde(skip)]
    pub microsoft_client_id: String,
    pub renderer: RendererMode,
    pub render_scale: crate::display::RenderScale,
    pub orientation: Orientation,
    #[serde(flatten)]
    pub screen: crate::display::ScreenPreferences,
    pub input_host: InputHost,
    pub keypad_controls: bool,
    pub record_performance: bool,
    pub worker_limit: u32,
    pub cache_budget_mib: u32,
    #[serde(flatten)]
    pub device: crate::device_preferences::Settings,
    pub shader_source: crate::shader_catalog::Origin,
    pub mod_source: crate::shader_catalog::Origin,
    #[serde(default)]
    pub ui_scale_percent: u32,
    #[serde(default)]
    pub super_perf_mode: bool,
    #[serde(default)]
    pub battery_perf_mode: bool,
    #[serde(default)]
    pub max_fps_mode: bool,
    #[serde(default = "default_true")]
    pub zero_latency_touch: bool,
    #[serde(default = "default_true")]
    pub fast_chunk_upload: bool,
    #[serde(default)]
    pub experimental_leaf_culling: bool,
    #[serde(default)]
    pub disable_launcher_animations: bool,
    #[serde(default)]
    pub disable_game_animations: bool,
    #[serde(default)]
    pub last_name_changed: String,
    #[serde(default)]
    pub custom_profile_name: String,
    #[serde(default)]
    pub custom_profile_desc: String,
    #[serde(default)]
    pub preset_edit_mode: String,
    #[serde(default)]
    pub theme: horizon_client::ui_theme::LauncherThemeConfig,
}

fn default_true() -> bool {
    true
}

pub use horizon_client::android_gamepad::InputMode as InputHost;

/// Android ActivityInfo orientations, independent of the device's natural axis.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
    ReverseLandscape,
    ReversePortrait,
    Auto,
}
impl Orientation {
    pub const ALL: [Self; 5] = [
        Self::Landscape,
        Self::Portrait,
        Self::ReverseLandscape,
        Self::ReversePortrait,
        Self::Auto,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Landscape => "Landscape (sensor)",
            Self::Portrait => "Portrait (sensor)",
            Self::ReverseLandscape => "Landscape (reversed)",
            Self::ReversePortrait => "Portrait (reversed)",
            Self::Auto => "Automatic rotation",
        }
    }
    pub fn android_value(self) -> i32 {
        match self {
            Self::Landscape => 6,
            Self::Portrait => 7,
            Self::ReverseLandscape => 8,
            Self::ReversePortrait => 9,
            Self::Auto => 10,
        }
    }
}
impl Config {
    pub fn load(home: &Path) -> Self {
        let value: serde_json::Value = std::fs::read(home.join("android-launcher.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut config: Self = serde_json::from_value(value.clone()).unwrap_or_default();
        if cfg!(target_pointer_width = "32") && value.get("renderer").is_none() {
            config.renderer = RendererMode::VulkanCompatibility;
        }
        // Ignore saved app registrations while this dev build uses the fixed ID.
        config.microsoft_client_id =
            crate::account::client_id().unwrap_or_else(|| MICROSOFT_CLIENT_ID.into());
        if value.get("renderer").is_none() && value["gpu_batching"].as_bool() == Some(true) {
            config.renderer = RendererMode::Vulkan;
        }
        let legacy = std::fs::read(config.game_home(home).join("horizon-touch.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .unwrap_or_default();
        config.device.migrate_missing(&value, &legacy);
        config.name = truncate_player_name(&config.name);
        config.theme.sanitize();
        horizon_client::ui_theme::set_active_theme(config.theme.resolve());
        horizon_client::ui_theme::motion::set_reduced_motion(
            config.disable_launcher_animations
                || config.super_perf_mode
                || config.battery_perf_mode,
        );
        horizon_client::ui_theme::motion::set_game_animations_disabled(
            config.disable_game_animations,
        );
        config
    }
    pub fn save(&self, home: &Path) -> Result<(), String> {
        let part = home.join("android-launcher.partial");
        std::fs::write(
            &part,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(part, home.join("android-launcher.json")).map_err(|e| e.to_string())
    }
    pub fn profile_home(&self, home: &Path) -> PathBuf {
        if self.profile == 0 {
            home.into()
        } else {
            home.join("profiles")
                .join(format!("profile-{}", self.profile))
        }
    }
    pub fn game_home(&self, home: &Path) -> PathBuf {
        // Legacy modpack selections are ignored; their files remain untouched.
        self.profile_home(home)
    }
    pub fn touch_name_changed(&mut self) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let days = secs / 86_400;
        let rem = secs % 86_400;
        let hh = rem / 3600;
        let mm = (rem % 3600) / 60;
        // Convert days since 1970-01-01 to YYYY-MM-DD (civil calendar algorithm)
        let z = days as i64 + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = (z - era * 146_097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if m <= 2 { y + 1 } else { y };
        self.last_name_changed = format!("{year:04}-{m:02}-{d:02} at {hh:02}:{mm:02} UTC");
    }
    pub fn last_name_changed_label(&self) -> &str {
        if self.last_name_changed.trim().is_empty() {
            "Default profile (Never changed)"
        } else {
            &self.last_name_changed
        }
    }
    pub fn ui_scale_label(&self) -> &'static str {
        match self.ui_scale_percent {
            85 => "85% (Compact)",
            100 => "100% (Default DPI)",
            115 => "115% (Large)",
            130 => "130% (Extra Large)",
            _ => "Auto (Phone DPI)",
        }
    }
    pub fn apply_experimental_to_game(&self, home: &Path) {
        horizon_client::android_runtime::set_experimental_leaf_culling(
            self.experimental_leaf_culling,
        );
        let game = self.game_home(home);
        let _ = std::fs::create_dir_all(&game);
        let options_path = game.join("options.txt");
        let mut settings = horizon_client::settings::Settings::load(&options_path);
        horizon_client::ui_theme::motion::set_game_animations_disabled(
            self.disable_game_animations,
        );
        if self.disable_game_animations {
            settings.horizon.animated_textures = false;
            settings.chunk_fade = 0.0;
            settings.view_bobbing = false;
            settings.menu_blur = 0;
        }
        if self.super_perf_mode {
            settings.render_distance = settings.render_distance.min(8);
            settings.simulation_distance = settings.simulation_distance.min(6);
            settings.menu_blur = 0;
            settings.clouds = "false".into();
            settings.particles = 2;
            settings.entity_shadows = false;
            settings.cutout_leaves = false;
            settings.smooth_lighting = false;
            settings.graphics_preset = "fast".into();
            settings.horizon.upload_budget_kib = 1536;
        } else if self.battery_perf_mode {
            settings.render_distance = settings.render_distance.min(8);
            settings.simulation_distance = settings.simulation_distance.min(5);
            settings.max_fps = 60;
            settings.vsync = true;
            settings.menu_blur = 0;
        } else if self.max_fps_mode {
            settings.max_fps = 260;
            settings.vsync = false;
        }
        if self.fast_chunk_upload {
            settings.horizon.upload_budget_kib = settings.horizon.upload_budget_kib.max(4096);
        }
        settings.horizon.cave_culling = true;
        settings.horizon.entity_culling = true;
        settings.horizon.resolution_scale = self.render_scale.percent().clamp(25, 100);
        settings.save(&options_path);
        let touch_path = game.join("horizon-touch.json");
        let mut touch_cfg = horizon_client::touch::Settings::load_from(&touch_path);
        touch_cfg.disable_game_animations = self.disable_game_animations;
        touch_cfg.resolution_scale_percent = self.render_scale.percent().clamp(25, 100);
        let _ = touch_cfg.save_to(&touch_path);
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Play,
    Mods,
    Packs,
    Shaders,
    Installed,
    Settings,
    Skins,
}
impl Page {
    pub const ALL: [Self; 7] = [
        Self::Play,
        Self::Mods,
        Self::Packs,
        Self::Shaders,
        Self::Installed,
        Self::Settings,
        Self::Skins,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Mods => "Mods",
            Self::Packs => "Resource Pack",
            Self::Shaders => "Shaders",
            Self::Installed => "Installed",
            Self::Settings => "Settings",
            Self::Skins => "Skins",
        }
    }
    pub fn is_catalog(self) -> bool {
        matches!(self, Self::Packs | Self::Mods | Self::Shaders)
    }
}
enum Reply {
    Done(String, horizon_client::content_files::Kind),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    #[default]
    Overview,
    Video,
    Launcher,
    Theme,
    Controls,
    Experimental,
    Diagnostics,
    Account,
}
impl SettingsCategory {
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "Settings",
            Self::Video => "Video Settings",
            Self::Launcher => "Launcher Settings",
            Self::Theme => "Theme Customization",
            Self::Controls => "Controls & HUD Layout",
            Self::Experimental => "Experimental Stuff",
            Self::Diagnostics => "Diagnostics & Logs",
            Self::Account => "Account",
        }
    }
}

#[derive(Clone, Copy)]
pub struct TransferTracker {
    pub started_at: std::time::Instant,
    pub last_sample_at: std::time::Instant,
    pub last_mib: f64,
    pub speed_mib_s: f64,
    pub paused: bool,
}
impl Default for TransferTracker {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            started_at: now,
            last_sample_at: now,
            last_mib: 0.0,
            speed_mib_s: 0.0,
            paused: false,
        }
    }
}
impl TransferTracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn observe(&mut self, done_mib: f64, paused: bool) {
        let now = std::time::Instant::now();
        self.paused = paused;
        if paused {
            self.last_sample_at = now;
            self.last_mib = done_mib;
            self.speed_mib_s = 0.0;
            return;
        }
        let dt = now.duration_since(self.last_sample_at).as_secs_f64();
        if dt >= 0.18 {
            let delta = (done_mib - self.last_mib).max(0.0);
            let inst = delta / dt;
            if inst > 0.0 {
                self.speed_mib_s = if self.speed_mib_s <= 0.01 {
                    inst
                } else {
                    self.speed_mib_s * 0.68 + inst * 0.32
                };
            } else if dt > 1.2 {
                let avg_dt = now.duration_since(self.started_at).as_secs_f64().max(0.5);
                self.speed_mib_s = (done_mib / avg_dt).max(0.4);
            }
            self.last_sample_at = now;
            self.last_mib = done_mib;
        } else if self.speed_mib_s <= 0.01 && done_mib > 0.0 {
            let avg_dt = now.duration_since(self.started_at).as_secs_f64().max(0.25);
            self.speed_mib_s = (done_mib / avg_dt).clamp(0.5, 95.0);
        }
    }
    pub fn speed_label(&self) -> String {
        if self.paused {
            return "0.0 MiB/s".into();
        }
        let s = self.speed_mib_s.max(0.0);
        if s < 0.05 {
            "Calculating...".into()
        } else if s < 1.0 {
            format!("{:.0} KiB/s", s * 1024.0)
        } else {
            format!("{s:.1} MiB/s")
        }
    }
    pub fn eta_label(&self, done_mib: f64, total_mib: f64) -> String {
        if self.paused {
            return "Paused".into();
        }
        let rem = (total_mib - done_mib).max(0.0);
        if rem <= 0.05 {
            return "ETA <1s".into();
        }
        let speed = self.speed_mib_s;
        if speed <= 0.05 {
            return "ETA --".into();
        }
        let secs = (rem / speed).ceil() as u64;
        if secs >= 60 {
            format!("ETA {}m {:02}s", secs / 60, secs % 60)
        } else {
            format!("ETA {secs}s")
        }
    }
}

pub struct ProjectModal {
    pub project: Project,
    pub bundled_id: Option<&'static str>,
    pub catalog_index: Option<usize>,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub followers: u64,
    pub license: String,
    pub categories: Vec<String>,
    pub versions: Vec<browser::Version>,
    pub selected_version: usize,
    pub screenshots: Vec<(String, u32, u32, Vec<u8>)>,
    pub loading: bool,
    pub scroll: f32,
}

pub enum ModalUpdate {
    Metadata {
        description: String,
        author: String,
        downloads: u64,
        followers: u64,
        license: String,
        categories: Vec<String>,
        versions: Vec<browser::Version>,
    },
    Icon(String, u32, u32, Vec<u8>),
    Screenshot(String, u32, u32, Vec<u8>),
    Finished,
}

type IconData = (String, u32, u32, Vec<u8>, Option<String>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstalledSubTab {
    #[default]
    Library,
    FileManager,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstalledFilter {
    #[default]
    All,
    Mods,
    Packs,
    Shaders,
}

impl InstalledFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Mods, Self::Packs, Self::Shaders];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Mods => "Mods",
            Self::Packs => "Resource Packs",
            Self::Shaders => "Shaders",
        }
    }
    pub fn matches(self, kind: horizon_client::content_files::Kind) -> bool {
        match self {
            Self::All => true,
            Self::Mods => kind == horizon_client::content_files::Kind::Mods,
            Self::Packs => kind == horizon_client::content_files::Kind::Packs,
            Self::Shaders => kind == horizon_client::content_files::Kind::Shaders,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstalledSort {
    #[default]
    Name,
    Type,
    Size,
    Status,
}

impl InstalledSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Sort: Name",
            Self::Type => "Sort: Type",
            Self::Size => "Sort: Size",
            Self::Status => "Sort: Status",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Type,
            Self::Type => Self::Size,
            Self::Size => Self::Status,
            Self::Status => Self::Name,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FmModal {
    ConfirmDelete { paths: Vec<String> },
    Rename { target_rel: String },
    NewFolder,
    Details(crate::document_store::FmFileDetails),
}

pub fn format_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub struct Catalog {
    pub local: Vec<LocalItem>,
    pub bundled_enabled: std::collections::BTreeSet<String>,
    pub sections: [bool; 3],
    pub pending_delete: Option<LocalItem>,
    pub installed_sub_tab: InstalledSubTab,
    pub installed_filter: InstalledFilter,
    pub installed_sort: InstalledSort,
    pub installed_query: String,
    pub fm_current_dir: String,
    pub fm_back_stack: Vec<String>,
    pub fm_forward_stack: Vec<String>,
    pub fm_entries: Vec<crate::document_store::FmEntry>,
    pub fm_selected: std::collections::BTreeSet<String>,
    pub fm_clipboard: Option<(Vec<PathBuf>, bool)>,
    pub fm_modal: Option<FmModal>,
    pub fm_input_text: String,
    downloaded: Vec<(browser::SourceId, String)>,
    pub downloading: Option<(browser::SourceId, String)>,
    pub downloading_title: String,
    pub downloading_size_mib: f32,
    progress: Arc<Mutex<f32>>,
    progress_stage: Arc<Mutex<String>>,
    paused: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    failed: Option<(browser::SourceId, String)>,
    pub page: Page,
    pub query: String,
    pub results: Vec<Project>,
    pub icons: HashMap<String, (u32, u32, Vec<u8>)>,
    pub enriched_summaries: HashMap<String, String>,
    pub project_modal: Option<ProjectModal>,
    pub modal_worker: Option<mpsc::Receiver<ModalUpdate>>,
    pub settings_category: SettingsCategory,
    pub transfer: TransferTracker,
    pub status: String,
    pub offset: usize,
    pub scroll: f32,
    pub config: Config,
    pub editing: Option<&'static str>,
    pub render_scale_text: String,
    worker: Option<mpsc::Receiver<Result<Reply, String>>>,
    search_worker: Option<mpsc::Receiver<Result<Vec<Project>, String>>>,
    icon_worker: Option<mpsc::Receiver<IconData>>,
    searches: crate::search_queue::Queue<(Page, browser::SourceId, String, usize), Vec<Project>>,
}
#[derive(Clone)]
pub struct LocalItem {
    pub kind: horizon_client::content_files::Kind,
    pub file: String,
    pub title: String,
    pub enabled: bool,
    pub size_bytes: u64,
    pub badge: &'static str,
    pub subtitle: String,
}
impl Catalog {
    pub fn new(home: &Path) -> Self {
        let config = Config::load(home);
        let mut result = Self {
            local: vec![],
            bundled_enabled: Default::default(),
            sections: [true, true, true],
            pending_delete: None,
            installed_sub_tab: InstalledSubTab::Library,
            installed_filter: InstalledFilter::All,
            installed_sort: InstalledSort::Name,
            installed_query: String::new(),
            fm_current_dir: String::new(),
            fm_back_stack: Vec::new(),
            fm_forward_stack: Vec::new(),
            fm_entries: Vec::new(),
            fm_selected: Default::default(),
            fm_clipboard: None,
            fm_modal: None,
            fm_input_text: String::new(),
            downloaded: vec![],
            downloading: None,
            downloading_title: String::new(),
            downloading_size_mib: 4.8,
            progress: Arc::new(Mutex::new(0.0)),
            progress_stage: Arc::new(Mutex::new(String::new())),
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
            failed: None,
            page: Page::Play,
            query: String::new(),
            results: vec![],
            icons: HashMap::new(),
            enriched_summaries: HashMap::new(),
            project_modal: None,
            modal_worker: None,
            settings_category: SettingsCategory::Overview,
            transfer: TransferTracker::default(),
            status: String::new(),
            offset: 0,
            scroll: 0.0,
            config,
            editing: None,
            render_scale_text: String::new(),
            worker: None,
            search_worker: None,
            icon_worker: None,
            searches: crate::search_queue::Queue::new(),
        };
        result.refresh(home);
        result
    }
    pub fn show_installed(&mut self, kind: Option<horizon_client::content_files::Kind>) {
        self.searches.cancel_interest();
        self.page = Page::Installed;
        self.scroll = 0.0;
        self.editing = None;
        if let Some(k) = kind {
            self.installed_sub_tab = InstalledSubTab::Library;
            self.installed_filter = match k {
                horizon_client::content_files::Kind::Mods => InstalledFilter::Mods,
                horizon_client::content_files::Kind::Packs => InstalledFilter::Packs,
                horizon_client::content_files::Kind::Shaders => InstalledFilter::Shaders,
            };
        }
        self.sections = [true; 3];
        if let Some(i) = horizon_client::content_files::Kind::ALL
            .iter()
            .position(|k| Some(*k) == kind)
        {
            self.sections[i] = true;
        }
    }
    pub fn refresh(&mut self, home: &Path) {
        let game = self.config.game_home(home);
        let installed = Installed::load(&game);
        self.downloaded = installed
            .entries
            .iter()
            .filter(|e| game.join(&e.file).exists())
            .map(|e| (e.source, e.project.clone()))
            .collect();
        let on = horizon_client::mod_list::ModList::load(&game);
        self.bundled_enabled = on.enabled.difference(&on.disabled).cloned().collect();
        let settings = horizon_client::settings::Settings::load(&game.join("options.txt"));
        self.local.clear();
        for kind in horizon_client::content_files::Kind::ALL {
            let dir = game.join(kind.folder());
            if std::fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
                continue;
            }
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let valid = horizon_client::content_files::plain_name(&name)
                    && (kind.accepts(&name)
                        || (kind == horizon_client::content_files::Kind::Mods
                            && name.to_ascii_lowercase().ends_with(".jar")));
                if !valid {
                    continue;
                }
                let enabled = match kind {
                    horizon_client::content_files::Kind::Packs => {
                        settings.resource_packs.contains(&format!("file/{name}"))
                    }
                    horizon_client::content_files::Kind::Shaders => {
                        settings.horizon.shader_pack == name
                    }
                    horizon_client::content_files::Kind::Mods => name
                        .strip_suffix(".hmod")
                        .or_else(|| name.strip_suffix(".jar"))
                        .is_some_and(|id| on.enabled.contains(id)),
                };
                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let badge = match kind {
                    horizon_client::content_files::Kind::Mods => "MOD",
                    horizon_client::content_files::Kind::Packs => "PACK",
                    horizon_client::content_files::Kind::Shaders => "SHADER",
                };
                let ext_tag = if name.ends_with(".hmod") {
                    "Horizon Mod (.hmod)"
                } else if name.ends_with(".jar") {
                    "Java Mod (.jar)"
                } else if name.ends_with(".hshader") {
                    "Horizon Shader (.hshader)"
                } else if kind == horizon_client::content_files::Kind::Shaders {
                    "Shader Pack (.zip)"
                } else {
                    "Resource Pack (.zip)"
                };
                let subtitle = format!("{ext_tag}  •  {}  •  {name}", format_file_size(size_bytes));
                let clean_title = name
                    .strip_suffix(".hmod")
                    .or_else(|| name.strip_suffix(".jar"))
                    .or_else(|| name.strip_suffix(".hshader"))
                    .or_else(|| name.strip_suffix(".zip"))
                    .unwrap_or(&name)
                    .to_owned();
                self.local.push(LocalItem {
                    kind,
                    file: name,
                    title: clean_title,
                    enabled,
                    size_bytes,
                    badge,
                    subtitle,
                });
            }
        }
        // Include ported Modrinth folder mods and native .hnmod packages from Installed.
        for e in &installed.entries {
            if e.kind != Kind::Mod || !game.join(&e.file).exists() {
                continue;
            }
            let file_name = e
                .file
                .strip_prefix("horizon-mods/")
                .unwrap_or(&e.file)
                .to_owned();
            if self
                .local
                .iter()
                .any(|item| item.file == file_name || item.file == e.file)
            {
                continue;
            }
            let mod_id = file_name
                .strip_prefix("horizon-native-mods/")
                .unwrap_or(&file_name)
                .strip_suffix(".hnmod")
                .or_else(|| file_name.strip_suffix(".hmod"))
                .or_else(|| file_name.strip_suffix(".jar"))
                .unwrap_or(&file_name);
            let full_path = game.join(&e.file);
            let size_bytes = std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
            let subtitle = format!(
                "Mod Package ({})  •  {}  •  {}",
                e.number,
                format_file_size(size_bytes),
                file_name
            );
            self.local.push(LocalItem {
                kind: horizon_client::content_files::Kind::Mods,
                file: e.file.clone(),
                title: format!("{} ({})", e.title, e.version),
                enabled: on.enabled.contains(mod_id) && !on.disabled.contains(mod_id),
                size_bytes,
                badge: "MOD",
                subtitle,
            });
        }
        // Also scan horizon-native-mods/ for any .hnmod or .hmod files on disk.
        let native_dir = game.join("horizon-native-mods");
        if std::fs::symlink_metadata(&native_dir).is_ok_and(|m| !m.file_type().is_symlink()) {
            for entry in std::fs::read_dir(&native_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if !horizon_client::content_files::plain_name(&name)
                    || (!name.ends_with(".hnmod")
                        && !name.ends_with(".hmod")
                        && !name.ends_with(".jar"))
                {
                    continue;
                }
                let rel = format!("horizon-native-mods/{name}");
                if self
                    .local
                    .iter()
                    .any(|item| item.file == name || item.file == rel)
                {
                    continue;
                }
                let mod_id = name
                    .strip_suffix(".hnmod")
                    .or_else(|| name.strip_suffix(".hmod"))
                    .or_else(|| name.strip_suffix(".jar"))
                    .unwrap_or(&name);
                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                self.local.push(LocalItem {
                    kind: horizon_client::content_files::Kind::Mods,
                    file: rel,
                    title: mod_id.to_owned(),
                    enabled: on.enabled.contains(mod_id) && !on.disabled.contains(mod_id),
                    size_bytes,
                    badge: "MOD",
                    subtitle: format!(
                        "Horizon Native Mod (.hnmod)  •  {}  •  {name}",
                        format_file_size(size_bytes)
                    ),
                });
            }
        }
        // Include enabled built-in Horizon mods (ANDROID_BUNDLED) in Installed > Mods.
        for &(b_id, b_title, b_desc) in &horizon_client::mod_list::ANDROID_BUNDLED {
            if on.enabled.contains(b_id) && !on.disabled.contains(b_id) {
                let b_file = format!("builtin:{b_id}");
                if !self.local.iter().any(|item| item.file == b_file) {
                    self.local.push(LocalItem {
                        kind: horizon_client::content_files::Kind::Mods,
                        file: b_file,
                        title: b_title.to_owned(),
                        enabled: true,
                        size_bytes: 0,
                        badge: "MOD",
                        subtitle: format!("Built-in Horizon Mod  •  {b_desc}  •  {b_id}"),
                    });
                }
            }
        }
        self.sort_local_items();
        self.refresh_fm(home);
    }

    pub fn sort_local_items(&mut self) {
        match self.installed_sort {
            InstalledSort::Name => {
                self.local
                    .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            InstalledSort::Type => {
                self.local.sort_by(|a, b| {
                    a.badge
                        .cmp(b.badge)
                        .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                });
            }
            InstalledSort::Size => {
                self.local.sort_by(|a, b| {
                    b.size_bytes
                        .cmp(&a.size_bytes)
                        .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                });
            }
            InstalledSort::Status => {
                self.local.sort_by(|a, b| {
                    b.enabled
                        .cmp(&a.enabled)
                        .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                });
            }
        }
    }

    pub fn search_worker_active(&self) -> bool {
        self.search_worker.is_some()
    }

    pub fn refresh_fm(&mut self, home: &Path) {
        let game = self.config.game_home(home);
        match crate::document_store::list_fm_dir(home, &game, &self.fm_current_dir) {
            Ok(entries) => {
                self.fm_selected
                    .retain(|sel| entries.iter().any(|e| &e.rel_path == sel));
                self.fm_entries = entries;
            }
            Err(_) => {
                self.fm_current_dir.clear();
                self.fm_selected.clear();
                self.fm_entries =
                    crate::document_store::list_fm_dir(home, &game, "").unwrap_or_default();
            }
        }
    }

    pub fn fm_navigate(&mut self, home: &Path, target_rel: String) {
        let clean = target_rel.trim().trim_matches('/').to_owned();
        if clean != self.fm_current_dir {
            self.fm_back_stack.push(self.fm_current_dir.clone());
            self.fm_forward_stack.clear();
            self.fm_current_dir = clean;
            self.fm_selected.clear();
            self.scroll = 0.0;
        }
        self.refresh_fm(home);
    }

    pub fn fm_back(&mut self, home: &Path) -> bool {
        let Some(prev) = self.fm_back_stack.pop() else {
            return false;
        };
        self.fm_forward_stack.push(self.fm_current_dir.clone());
        self.fm_current_dir = prev;
        self.fm_selected.clear();
        self.scroll = 0.0;
        self.refresh_fm(home);
        true
    }

    pub fn fm_forward(&mut self, home: &Path) -> bool {
        let Some(next) = self.fm_forward_stack.pop() else {
            return false;
        };
        self.fm_back_stack.push(self.fm_current_dir.clone());
        self.fm_current_dir = next;
        self.fm_selected.clear();
        self.scroll = 0.0;
        self.refresh_fm(home);
        true
    }

    pub fn fm_up(&mut self, home: &Path) {
        if self.fm_current_dir.is_empty() {
            return;
        }
        let parent = self
            .fm_current_dir
            .rsplit_once('/')
            .map(|(p, _)| p.to_owned())
            .unwrap_or_default();
        self.fm_navigate(home, parent);
    }

    pub fn fm_copy_or_cut(&mut self, home: &Path, is_cut: bool) {
        if self.fm_selected.is_empty() {
            self.status = "Select one or more files or folders first.".into();
            return;
        }
        let game = self.config.game_home(home);
        let mut paths = Vec::new();
        for rel in &self.fm_selected {
            if is_cut && crate::document_store::is_protected_root_folder(rel) {
                self.status = format!("Built-in root folder '{rel}' cannot be moved.");
                return;
            }
            match crate::document_store::resolve_fm_path(home, &game, rel) {
                Ok(p) => paths.push(p),
                Err(e) => {
                    self.status = e;
                    return;
                }
            }
        }
        let count = paths.len();
        self.fm_clipboard = Some((paths, is_cut));
        self.status = format!(
            "{} {count} item(s) to clipboard. Open destination folder and tap Paste.",
            if is_cut { "Cut" } else { "Copied" }
        );
    }

    pub fn fm_paste_here(&mut self, home: &Path) {
        let Some((sources, is_cut)) = self.fm_clipboard.clone() else {
            self.status = "Clipboard is empty.".into();
            return;
        };
        let game = self.config.game_home(home);
        match crate::document_store::fm_paste(home, &game, &sources, is_cut, &self.fm_current_dir) {
            Ok(count) => {
                if is_cut {
                    self.fm_clipboard = None;
                }
                self.fm_selected.clear();
                self.status = format!(
                    "{} {count} item(s) into '/{}'.",
                    if is_cut { "Moved" } else { "Pasted" },
                    self.fm_current_dir
                );
                self.refresh(home);
            }
            Err(e) => {
                self.status = e;
            }
        }
    }

    pub fn fm_confirm_delete(&mut self, home: &Path) {
        let Some(FmModal::ConfirmDelete { paths }) = self.fm_modal.take() else {
            return;
        };
        let game = self.config.game_home(home);
        match crate::document_store::fm_delete(home, &game, &paths) {
            Ok(count) => {
                self.fm_selected.clear();
                self.status = format!("Deleted {count} item(s).");
                self.refresh(home);
            }
            Err(e) => {
                self.status = e;
            }
        }
    }

    pub fn fm_submit_rename(&mut self, home: &Path) {
        let Some(FmModal::Rename { target_rel }) = self.fm_modal.clone() else {
            return;
        };
        let game = self.config.game_home(home);
        let new_name = self.fm_input_text.trim().to_owned();
        match crate::document_store::fm_rename(home, &game, &target_rel, &new_name) {
            Ok(_) => {
                self.fm_modal = None;
                self.editing = None;
                self.fm_selected.clear();
                crate::platform::hide();
                self.status = format!("Renamed to '{new_name}'.");
                self.refresh(home);
            }
            Err(e) => {
                self.status = e;
            }
        }
    }

    pub fn fm_submit_new_folder(&mut self, home: &Path) {
        if !matches!(self.fm_modal, Some(FmModal::NewFolder)) {
            return;
        }
        let game = self.config.game_home(home);
        let folder_name = self.fm_input_text.trim().to_owned();
        match crate::document_store::fm_mkdir(home, &game, &self.fm_current_dir, &folder_name) {
            Ok(_) => {
                self.fm_modal = None;
                self.editing = None;
                crate::platform::hide();
                self.status = format!("Created folder '{folder_name}'.");
                self.refresh(home);
            }
            Err(e) => {
                self.status = e;
            }
        }
    }
    pub fn busy(&self) -> bool {
        self.worker.is_some()
    }
    pub fn progress_fraction(&self) -> f32 {
        *self.progress.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn progress_stage(&self) -> String {
        let s = self
            .progress_stage
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if s.is_empty() { self.status.clone() } else { s }
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    pub fn toggle_pause(&mut self) {
        let next = !self.is_paused();
        self.paused.store(next, Ordering::Relaxed);
    }
    pub fn cancel_download(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        self.worker = None;
        self.downloading = None;
        self.downloading_title.clear();
        self.status = "Download cancelled.".into();
    }
    pub fn download_label(&self, project: &Project) -> Option<String> {
        if source_only_mod(project) {
            return Some("Unsupported".into());
        }
        if self
            .downloading
            .as_ref()
            .is_some_and(|(source, id)| *source == project.source && *id == project.id)
        {
            return Some(format!(
                "{:.0}%",
                *self.progress.lock().unwrap_or_else(|e| e.into_inner()) * 100.0
            ));
        }
        if self
            .downloaded
            .iter()
            .any(|(source, id)| *source == project.source && *id == project.id)
            || (project.kind == Kind::Mod
                && self.local.iter().any(|item| {
                    item.kind == horizon_client::content_files::Kind::Mods
                        && (item.file == format!("{}.hmod", project.id)
                            || item.file == format!("{}.jar", project.slug)
                            || item.file == format!("horizon-mods/{}", project.id)
                            || item.file == format!("horizon-mods/{}", project.slug)
                            || item.file == format!("horizon-native-mods/{}.hnmod", project.id))
                }))
        {
            return Some("Installed".into());
        }
        None
    }
    pub fn install_button(&self, project: &Project) -> &'static str {
        if self
            .failed
            .as_ref()
            .is_some_and(|(source, id)| *source == project.source && *id == project.id)
        {
            "Retry"
        } else {
            "Install"
        }
    }
    fn job(&mut self, status: &str, job: impl FnOnce() -> Result<Reply, String> + Send + 'static) {
        if self.busy() {
            return;
        }
        self.status = status.into();
        *self
            .progress_stage
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = status.into();
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(job());
        });
    }
    pub fn poll(&mut self, home: &Path) {
        if let Some(rx) = &self.icon_worker {
            while let Ok((key, w, h, rgba, summary)) = rx.try_recv() {
                self.icons.insert(key.clone(), (w, h, rgba));
                if let Some(s) = summary.filter(|s| !s.trim().is_empty()) {
                    self.enriched_summaries.insert(key, s);
                }
            }
        }
        if let Some(rx) = &self.modal_worker {
            while let Ok(update) = rx.try_recv() {
                match update {
                    ModalUpdate::Metadata {
                        description,
                        author,
                        downloads,
                        followers,
                        license,
                        categories,
                        versions,
                    } => {
                        if let Some(modal) = &mut self.project_modal {
                            if !description.trim().is_empty() {
                                modal.description = description;
                            }
                            if !author.trim().is_empty() {
                                modal.author = author;
                            }
                            if downloads > 0 {
                                modal.downloads = downloads;
                            }
                            if followers > 0 {
                                modal.followers = followers;
                            }
                            if !license.trim().is_empty() {
                                modal.license = license;
                            }
                            if !categories.is_empty() {
                                modal.categories = categories;
                            }
                            if !versions.is_empty() {
                                modal.versions = versions;
                                modal.selected_version = 0;
                            }
                        }
                    }
                    ModalUpdate::Icon(id, w, h, rgba) => {
                        self.icons.insert(id, (w, h, rgba));
                    }
                    ModalUpdate::Screenshot(title, w, h, rgba) => {
                        if let Some(modal) = &mut self.project_modal {
                            modal.screenshots.push((title, w, h, rgba));
                        }
                    }
                    ModalUpdate::Finished => {
                        if let Some(modal) = &mut self.project_modal {
                            modal.loading = false;
                        }
                    }
                }
            }
        }
        if self.downloading.is_some() && self.busy() {
            let frac = self.progress_fraction().clamp(0.0, 1.0);
            let total = f64::from(self.downloading_size_mib.max(1.0));
            self.transfer
                .observe(total * f64::from(frac), self.is_paused());
        }
        let reply = self.worker.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Launcher worker stopped".into())),
            Err(_) => None,
        });
        if let Some(reply) = reply {
            self.worker = None;
            let downloading = self.downloading.take();
            self.downloading_title.clear();
            if reply.is_err() {
                self.failed = downloading;
            }
            match reply {
                Ok(Reply::Done(s, kind)) => {
                    self.status = s;
                    self.show_installed(Some(kind));
                }
                Err(e) => {
                    log::warn!("Add-on operation failed: {e}");
                    self.status = e;
                }
            }
            self.refresh(home);
        }
        let result = self
            .search_worker
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(v) => Some(v),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Search worker stopped".into())),
                Err(_) => None,
            });
        if let Some(result) = result {
            self.search_worker = None;
            let visible = self.searches.finish(result.as_ref().ok().cloned());
            if visible && self.page.is_catalog() && !self.busy() {
                match result {
                    Ok(items) => {
                        self.show_results(items);
                        self.fetch_icons(home);
                    }
                    Err(error) => self.status = error,
                }
            }
        }
        self.start_search();
    }
    fn fetch_icons(&mut self, home: &Path) {
        // Collect both direct icon URLs and Horizon/bundled items that need a Modrinth name search
        // for their PFP and description.
        let mut tasks: Vec<(String, String, Option<String>, Kind)> = Vec::new();
        if self.page == Page::Mods
            && self.config.mod_source == crate::shader_catalog::Origin::Horizon
        {
            for (id, title, _) in horizon_client::mod_list::ANDROID_BUNDLED {
                if !self.icons.contains_key(id) {
                    tasks.push((id.to_owned(), title.to_owned(), None, Kind::Mod));
                }
            }
        }
        for p in &self.results {
            if self.icons.contains_key(&p.id) {
                continue;
            }
            let direct = p
                .icon
                .as_ref()
                .map(|u| u.trim().to_owned())
                .filter(|u| !u.is_empty());
            tasks.push((p.id.clone(), p.title.clone(), direct, p.kind));
        }
        if tasks.is_empty() {
            return;
        }
        let home = home.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.icon_worker = Some(rx);
        std::thread::spawn(move || {
            for (id, title, direct_url, kind) in tasks {
                let mut icon_url = direct_url;
                let mut found_summary = None;
                if icon_url.is_none() {
                    if let Some(hit) = lookup_modrinth_by_name(&title, kind) {
                        icon_url = hit.icon;
                        if !hit.summary.trim().is_empty() {
                            found_summary = Some(hit.summary);
                        }
                    }
                }
                if let Some(url) = icon_url
                    && let Ok(path) = browser::images::fetch(&home, &url, browser::images::ROW)
                    && let Ok((w, h, rgba)) = horizon_client::hud::decode(&path)
                {
                    let _ = tx.send((id, w, h, rgba, found_summary));
                }
            }
        });
    }

    pub fn open_project_modal(
        &mut self,
        home: &Path,
        project: Project,
        bundled_id: Option<&'static str>,
        catalog_index: Option<usize>,
    ) {
        let initial_desc = self
            .enriched_summaries
            .get(&project.id)
            .cloned()
            .unwrap_or_else(|| project.summary.clone());
        let initial_versions = vec![browser::Version {
            id: format!("{}-latest", project.id),
            number: "26.3-latest".into(),
            published: "2026".into(),
            game_versions: vec![browser::GAME.into()],
            loaders: if project.loaders.is_empty() {
                vec![project.source.label().to_lowercase()]
            } else {
                project.loaders.clone()
            },
            files: Vec::new(),
            required: Vec::new(),
            blocked: None,
        }];
        self.project_modal = Some(ProjectModal {
            author: if project.author.is_empty() {
                project.source.label().to_owned()
            } else {
                project.author.clone()
            },
            downloads: project.downloads,
            followers: 0,
            license: project.license.clone(),
            categories: project.loaders.clone(),
            description: initial_desc,
            versions: initial_versions,
            selected_version: 0,
            screenshots: Vec::new(),
            loading: true,
            scroll: 0.0,
            project: project.clone(),
            bundled_id,
            catalog_index,
        });
        let home = home.to_path_buf();
        let need_icon = !self.icons.contains_key(&project.id);
        let (tx, rx) = mpsc::channel();
        self.modal_worker = Some(rx);
        std::thread::spawn(move || {
            let mut description = project.summary.clone();
            let mut author = project.author.clone();
            let mut downloads = project.downloads;
            let mut followers = 0;
            let mut license = project.license.clone();
            let mut categories = project.loaders.clone();
            let mut versions = Vec::new();
            let mut gallery = Vec::new();
            let mut icon_url = project.icon.clone();

            if let Some(source) = browser::source(project.source) {
                if let Ok(v) = source.versions(&project)
                    && !v.is_empty()
                {
                    versions = v;
                }
                if let Ok(d) = source.details(&project) {
                    if !d.body.trim().is_empty() {
                        description = clean_markdown_body(&d.body);
                    }
                    if !d.authors.is_empty() {
                        author = d.authors.join(", ");
                    }
                    if d.followers > 0 {
                        followers = d.followers;
                    }
                    if !d.categories.is_empty() {
                        categories = d.categories;
                    }
                    gallery = d.gallery;
                }
            }

            // For Horizon list items (or bundled mods), search Modrinth by the project's name
            // to enrich with Modrinth's full description, PFP icon, and gallery screenshots!
            if project.source == browser::SourceId::Horizon
                || gallery.is_empty()
                || icon_url.is_none()
            {
                if let Some(m_hit) = lookup_modrinth_by_name(&project.title, project.kind) {
                    if icon_url.is_none() {
                        icon_url = m_hit.icon.clone();
                    }
                    if downloads == 0 {
                        downloads = m_hit.downloads;
                    }
                    if author.is_empty() && !m_hit.author.is_empty() {
                        author = m_hit.author.clone();
                    }
                    if license.is_empty() && !m_hit.license.is_empty() {
                        license = m_hit.license.clone();
                    }
                    if let Ok(m_details) = browser::modrinth::Modrinth.details(&m_hit) {
                        if !m_details.body.trim().is_empty() {
                            let body_clean = clean_markdown_body(&m_details.body);
                            if project.source == browser::SourceId::Horizon {
                                let caps = browser::horizon::trust_and_capabilities(&project.id);
                                if caps.is_empty() {
                                    description = body_clean;
                                } else {
                                    description = format!(
                                        "{}

{}",
                                        caps.join(" • "),
                                        body_clean
                                    );
                                }
                            } else {
                                description = body_clean;
                            }
                        }
                        if followers == 0 {
                            followers = m_details.followers;
                        }
                        if !m_details.categories.is_empty() {
                            categories = m_details.categories;
                        }
                        if gallery.is_empty() {
                            gallery = m_details.gallery;
                        }
                    } else if description.len() < m_hit.summary.len() {
                        description = m_hit.summary.clone();
                    }
                    if versions.is_empty()
                        && let Ok(m_vers) = browser::modrinth::Modrinth.versions(&m_hit)
                    {
                        versions = m_vers;
                    }
                }
            }

            let _ = tx.send(ModalUpdate::Metadata {
                description,
                author,
                downloads,
                followers,
                license,
                categories,
                versions,
            });

            if need_icon
                && let Some(url) = icon_url
                && let Ok(path) = browser::images::fetch(&home, &url, browser::images::ROW)
                && let Ok((w, h, rgba)) = horizon_client::hud::decode(&path)
            {
                let _ = tx.send(ModalUpdate::Icon(project.id.clone(), w, h, rgba));
            }

            for (idx, img) in gallery.into_iter().take(3).enumerate() {
                if let Ok(path) = browser::images::fetch(&home, &img.url, 240)
                    && let Ok((w, h, rgba)) = horizon_client::hud::decode(&path)
                {
                    let label = if img.title.trim().is_empty() {
                        format!("Screenshot {}", idx + 1)
                    } else {
                        img.title
                    };
                    let _ = tx.send(ModalUpdate::Screenshot(label, w, h, rgba));
                }
            }
            let _ = tx.send(ModalUpdate::Finished);
        });
    }
    pub fn search(&mut self, home: &Path) {
        self.request_search(home, true);
    }
    pub fn browse(&mut self, home: &Path) {
        self.searches.cancel_interest();
        if self.page.is_catalog() {
            self.request_search(home, false);
        }
    }
    fn request_search(&mut self, home: &Path, refresh: bool) {
        if !self.page.is_catalog() {
            return;
        }
        if self.page == Page::Mods && self.query.trim().starts_with("https://") {
            self.searches.cancel_interest();
            if refresh {
                self.import_native(home);
            } else {
                self.results.clear();
                self.status = "Press Import .hmod to download this HTTPS link".into();
            }
            return;
        }
        self.scroll = 0.0;
        let source = match self.page {
            Page::Packs => browser::SourceId::Modrinth,
            Page::Shaders => self.config.shader_source.source(),
            Page::Mods => self.config.mod_source.source(),
            _ => browser::SourceId::Horizon,
        };
        let key = (self.page, source, self.query.trim().to_owned(), self.offset);
        if let Some(items) = self.searches.request(key, refresh) {
            self.show_results(items);
            self.fetch_icons(home);
        } else {
            self.results.clear();
            self.status = "Searching... You can switch tabs while results load.".into();
        }
        self.start_search();
    }
    fn show_results(&mut self, items: Vec<Project>) {
        self.status = if self.page == Page::Shaders
            && items.is_empty()
            && self.config.shader_source == crate::shader_catalog::Origin::Horizon
        {
            "No Horizon shader packs match this search. Try another query or import a .hshader."
                .into()
        } else if self.page == Page::Mods
            && self.config.mod_source == crate::shader_catalog::Origin::Modrinth
        {
            format!(
                "{} Java mods - browsing only; this Rust client cannot run Java loaders",
                items.len()
            )
        } else if self.page == Page::Mods {
            format!(
                "{} Horizon mods available (playhorizon.dev/mods) - select Install to download",
                items.len()
            )
        } else {
            format!("{} results - select Install to download", items.len())
        };
        self.results = items;
    }
    fn start_search(&mut self) {
        if self.busy() || self.search_worker.is_some() {
            return;
        }
        let Some((page, source, text, offset)) = self.searches.take_next() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.search_worker = Some(rx);
        std::thread::spawn(move || {
            let kind = match page {
                Page::Mods => Kind::Mod,
                Page::Shaders => Kind::Shader,
                _ => Kind::ResourcePack,
            };
            let mut query = Query::new(&text, kind);
            query.offset = offset;
            query.limit = 100;
            query.everything = true;
            let result = if page == Page::Shaders && source == browser::SourceId::Modrinth {
                query.everything = true;
                browser::modrinth::search_shader_packs(&query)
            } else if page == Page::Packs
                || (page == Page::Mods && source == browser::SourceId::Modrinth)
            {
                query.everything = true;
                browser::modrinth::Modrinth.search(&query)
            } else {
                browser::horizon::Registry.search(&query)
            };
            let _ = tx.send(result);
        });
    }
    pub fn import_native(&mut self, home: &Path) {
        let url = self.query.trim().to_owned();
        let game = self.config.game_home(home);
        self.downloading = Some((browser::SourceId::Horizon, "https-import".into()));
        self.downloading_title = "Custom .hmod Package".into();
        self.downloading_size_mib = 2.4;
        self.paused.store(false, Ordering::Relaxed);
        self.cancelled.store(false, Ordering::Relaxed);
        *self.progress.lock().unwrap_or_else(|e| e.into_inner()) = 0.15;
        let progress = self.progress.clone();
        self.job("Downloading sandboxed .hmod over HTTPS...", move || {
            use std::io::Read;
            let mut bytes = Vec::new();
            ureq::AgentBuilder::new()
                .https_only(true)
                .timeout_connect(std::time::Duration::from_secs(20))
                .timeout_read(std::time::Duration::from_secs(90))
                .build()
                .get(&url)
                .call()
                .map_err(|e| e.to_string())?
                .into_reader()
                .take(32 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 32 * 1024 * 1024 {
                return Err("Horizon mod exceeds 32 MiB".into());
            }
            *progress.lock().unwrap_or_else(|e| e.into_inner()) = 0.85;
            let dir = game.join("horizon-mods");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let stage = dir.join("import.partial");
            std::fs::write(&stage, bytes).map_err(|e| e.to_string())?;
            let result = (|| {
                let package = horizon_mods::Package::read(&stage)?;
                let destination = dir.join(format!("{}.hmod", package.manifest.id));
                if destination.exists() {
                    std::fs::copy(&destination, destination.with_extension("hmod.previous"))
                        .map_err(|e| e.to_string())?;
                }
                std::fs::rename(&stage, destination).map_err(|e| e.to_string())?;
                *progress.lock().unwrap_or_else(|e| e.into_inner()) = 1.0;
                Ok(Reply::Done(
                    format!(
                        "Installed {}. Enable it in Installed after reviewing its capabilities in game Mods.",
                        package.manifest.name
                    ),
                    horizon_client::content_files::Kind::Mods,
                ))
            })();
            let _ = std::fs::remove_file(stage);
            result
        });
    }
    pub fn install(&mut self, home: &Path, index: usize) {
        let Some(project) = self.results.get(index).cloned() else {
            return;
        };

        if source_only_mod(&project) {
            self.status = "Native .hnmod mods are temporarily unsupported on Android. Built-in mods remain available.".into();
            return;
        }
        if self.busy() || self.download_label(&project).is_some() {
            return;
        }
        let game = self.config.game_home(home);
        if !matches!(project.kind, Kind::ResourcePack | Kind::Shader | Kind::Mod) {
            return;
        }
        self.downloading = Some((project.source, project.id.clone()));
        self.downloading_title = project.title.clone();
        self.downloading_size_mib = match project.kind {
            Kind::ResourcePack => 14.6,
            Kind::Shader => 6.4,
            Kind::Mod => 3.8,
            _ => 5.0,
        };
        self.transfer.reset();
        self.failed = None;
        self.paused.store(false, Ordering::Relaxed);
        self.cancelled.store(false, Ordering::Relaxed);
        *self.progress.lock().unwrap_or_else(|e| e.into_inner()) = 0.0;
        let progress = self.progress.clone();
        let progress_stage = self.progress_stage.clone();
        let paused = self.paused.clone();
        let cancelled = self.cancelled.clone();
        let status = match (project.kind, project.source) {
            (Kind::Shader, browser::SourceId::Modrinth) => "Downloading shader pack ZIP...",
            (Kind::Mod, browser::SourceId::Modrinth) => "Downloading and porting Modrinth mod...",
            (Kind::Mod, _) => "Downloading Horizon mod package...",
            _ => "Checking compatibility and downloading...",
        };
        self.job(status, move || {
            std::fs::create_dir_all(&game).map_err(|e| e.to_string())?;
            let report = |stage: &str, p: f32| {
                while paused.load(Ordering::Relaxed) && !cancelled.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                if !stage.is_empty() {
                    *progress_stage.lock().unwrap_or_else(|e| e.into_inner()) = stage.to_owned();
                }
                if p.is_finite() {
                    *progress.lock().unwrap_or_else(|e| e.into_inner()) = p.clamp(0.0, 1.0);
                }
            };
            let source = browser::source(project.source).ok_or("Unsupported download source")?;
            let count = if project.kind == Kind::Shader
                && project.source == browser::SourceId::Modrinth
            {
                crate::shader_catalog::install(source.as_ref(), &project, &game, &report)?
            } else {
                match browser::installed::install(source.as_ref(), &project, true, &game, &report) {
                    Ok(entries) => entries.len(),
                    Err(first_err)
                        if project.kind == Kind::Mod
                            && project.source == browser::SourceId::Modrinth =>
                    {
                        // Fallback for Modrinth Java mods without port-pack content: download the
                        // primary .jar into horizon-mods/ and register it in Installed.
                        install_modrinth_jar_fallback(source.as_ref(), &project, &game, &report)
                            .map_err(|_| first_err)?
                    }
                    Err(e) => return Err(e),
                }
            };
            if cancelled.load(Ordering::Relaxed) {
                return Err("Download cancelled".into());
            }
            Ok(Reply::Done(
                format!("Installed {count} item(s). Open Play to load this profile."),
                match project.kind {
                    Kind::Mod => horizon_client::content_files::Kind::Mods,
                    Kind::Shader => horizon_client::content_files::Kind::Shaders,
                    _ => horizon_client::content_files::Kind::Packs,
                },
            ))
        });
    }
    pub fn toggle_local(&mut self, home: &Path, index: usize) {
        let Some(item) = self.local.get(index).cloned() else {
            return;
        };
        let game = self.config.game_home(home);
        let path = game.join("options.txt");
        let mut settings = horizon_client::settings::Settings::load(&path);
        match item.kind {
            horizon_client::content_files::Kind::Mods => {
                if !cfg!(target_pointer_width = "64") {
                    self.status = "Sandboxed code mods require a 64-bit APK".into();
                    return;
                }
                let mut on = horizon_client::mod_list::ModList::load(&game);
                if let Some(bid) = item.file.strip_prefix("builtin:") {
                    if item.enabled {
                        on.enabled.remove(bid);
                        on.disabled.insert(bid.to_owned());
                    } else {
                        on.disabled.remove(bid);
                        on.enabled.insert(bid.to_owned());
                    }
                    on.save(&game);
                    self.status = "Selection saved for the next game launch".into();
                    self.refresh(home);
                    return;
                }
                let id = item
                    .file
                    .strip_prefix("horizon-native-mods/")
                    .or_else(|| item.file.strip_prefix("horizon-mods/"))
                    .unwrap_or(&item.file);
                let id = id
                    .strip_suffix(".hmod")
                    .or_else(|| id.strip_suffix(".hnmod"))
                    .or_else(|| id.strip_suffix(".jar"))
                    .unwrap_or(id);
                if item.enabled {
                    on.enabled.remove(id);
                } else {
                    on.enabled.insert(id.to_owned());
                }
                on.save(&game);
            }
            horizon_client::content_files::Kind::Packs => {
                let id = format!("file/{}", item.file);
                if item.enabled {
                    settings.resource_packs.retain(|p| p != &id);
                } else {
                    settings.resource_packs.push(id);
                }
                settings.save(&path);
            }
            horizon_client::content_files::Kind::Shaders => {
                settings.horizon.shader_pack = if item.enabled {
                    String::new()
                } else {
                    item.file
                };
                settings.save(&path);
            }
        }
        self.status = "Selection saved for the next game launch".into();
        self.refresh(home);
    }
    pub fn remove_local(&mut self, home: &Path) {
        let Some(item) = self.pending_delete.take() else {
            return;
        };
        let game = self.config.game_home(home);
        let result = (|| -> Result<(), String> {
            if let Some(bid) = item.file.strip_prefix("builtin:") {
                let mut on = horizon_client::mod_list::ModList::load(&game);
                on.enabled.remove(bid);
                on.disabled.insert(bid.to_owned());
                on.save(&game);
                return Ok(());
            }
            if item.file.starts_with("horizon-mods/")
                || item.file.starts_with("horizon-native-mods/")
            {
                let target = game.join(&item.file);
                let meta = std::fs::symlink_metadata(&target).map_err(|e| e.to_string())?;
                if meta.file_type().is_symlink() {
                    return Err("Refusing a redirected content path".into());
                }
                if meta.is_dir() {
                    std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_file(&target).map_err(|e| e.to_string())?;
                }
                let mut installed = Installed::load(&game);
                installed.entries.retain(|e| e.file != item.file);
                installed.save(&game)?;
                let mut on = horizon_client::mod_list::ModList::load(&game);
                let id = item
                    .file
                    .strip_prefix("horizon-native-mods/")
                    .or_else(|| item.file.strip_prefix("horizon-mods/"))
                    .unwrap_or(&item.file);
                let id = id
                    .strip_suffix(".hmod")
                    .or_else(|| id.strip_suffix(".hnmod"))
                    .or_else(|| id.strip_suffix(".jar"))
                    .unwrap_or(id);
                on.enabled.remove(id);
                on.save(&game);
                return Ok(());
            }
            if !horizon_client::content_files::plain_name(&item.file) {
                return Err("Invalid content filename".into());
            }
            let dir = game.join(item.kind.folder());
            if std::fs::symlink_metadata(&dir)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Refusing a redirected content directory".into());
            }
            let target = dir.join(&item.file);
            let meta = std::fs::symlink_metadata(&target).map_err(|e| e.to_string())?;
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(
                    "Only an explicitly selected regular add-on file can be removed".into(),
                );
            }
            std::fs::remove_file(target).map_err(|e| e.to_string())?;
            let relative = format!("{}/{}", item.kind.folder(), item.file);
            let mut installed = Installed::load(&game);
            installed.entries.retain(|e| e.file != relative);
            installed.save(&game)?;
            let path = game.join("options.txt");
            let mut settings = horizon_client::settings::Settings::load(&path);
            if item.kind == horizon_client::content_files::Kind::Packs {
                settings
                    .resource_packs
                    .retain(|p| p != &format!("file/{}", item.file));
            }
            if item.kind == horizon_client::content_files::Kind::Shaders
                && settings.horizon.shader_pack == item.file
            {
                settings.horizon.shader_pack.clear();
            }
            settings.save(&path);
            if item.kind == horizon_client::content_files::Kind::Mods {
                let mut on = horizon_client::mod_list::ModList::load(&game);
                if let Some(id) = item
                    .file
                    .strip_suffix(".hmod")
                    .or_else(|| item.file.strip_suffix(".jar"))
                {
                    on.enabled.remove(id);
                }
                on.save(&game);
            }
            Ok(())
        })();
        self.status = match result {
            Ok(()) => format!("Removed {}", item.title),
            Err(e) => e,
        };
        self.refresh(home);
    }
    pub fn edit(&mut self, out: horizon_client::osk::Output, home: &Path) {
        use horizon_client::osk::Output;
        let is_name = self.editing == Some("name");
        let is_profile_name = self.editing == Some("profile_name");
        let is_theme_field = matches!(
            self.editing,
            Some(
                "theme_bg"
                    | "theme_surface"
                    | "theme_accent"
                    | "theme_grad_start"
                    | "theme_grad_mid"
                    | "theme_grad_end"
            )
        );
        if is_theme_field {
            self.config.theme.preset = horizon_client::ui_theme::ThemePreset::Custom;
        }
        let field = match self.editing {
            Some("name") => &mut self.config.name,
            Some("profile_name") => &mut self.config.custom_profile_name,
            Some("profile_desc") => &mut self.config.custom_profile_desc,
            Some("server") => &mut self.config.server,
            Some("render_scale") => &mut self.render_scale_text,
            Some("client_id") => &mut self.config.microsoft_client_id,
            Some("theme_bg") => &mut self.config.theme.custom_bg_hex,
            Some("theme_surface") => &mut self.config.theme.custom_surface_hex,
            Some("theme_accent") => &mut self.config.theme.custom_accent_hex,
            Some("theme_grad_start") => &mut self.config.theme.gradient_start_hex,
            Some("theme_grad_mid") => &mut self.config.theme.gradient_mid_hex,
            Some("theme_grad_end") => &mut self.config.theme.gradient_end_hex,
            Some("installed_search") => &mut self.installed_query,
            Some("fm_rename" | "fm_new_folder") => &mut self.fm_input_text,
            _ => &mut self.query,
        };
        match out {
            Output::Text(s) => {
                if is_name {
                    let combined = format!("{}{}", field, s);
                    *field = truncate_player_name(&combined);
                    self.config.touch_name_changed();
                } else {
                    field.push_str(&s);
                }
            }
            Output::Backspace => {
                field.pop();
                if is_name {
                    self.config.touch_name_changed();
                }
            }
            Output::Key(_) => {}
            Output::Enter | Output::Close => {
                if self.editing == Some("fm_rename") {
                    if matches!(out, Output::Enter) {
                        self.fm_submit_rename(home);
                    } else {
                        self.editing = None;
                    }
                    return;
                } else if self.editing == Some("fm_new_folder") {
                    if matches!(out, Output::Enter) {
                        self.fm_submit_new_folder(home);
                    } else {
                        self.editing = None;
                    }
                    return;
                } else if self.editing == Some("installed_search") {
                    self.editing = None;
                    self.scroll = 0.0;
                    return;
                } else if self.editing == Some("render_scale") {
                    match crate::display::RenderScale::parse(&self.render_scale_text) {
                        Ok(scale) => {
                            self.config.render_scale = scale;
                            crate::display::set_scale(scale);
                        }
                        Err(error) => {
                            self.status = error.into();
                            return;
                        }
                    }
                } else if is_name {
                    self.config.touch_name_changed();
                } else if is_profile_name {
                    if matches!(out, Output::Enter)
                        && (self.config.preset_edit_mode == "copy"
                            || self.config.preset_edit_mode == "edit")
                    {
                        self.editing = Some("profile_desc");
                        let _ = self.config.save(home);
                        crate::platform::show(&self.config.custom_profile_desc);
                        self.status =
                            "Step 2/2: Edit preset description and press Enter to save.".into();
                        return;
                    }
                    let game_home = self.config.game_home(home);
                    let clean = horizon_client::touch::Settings::sanitize_profile_name(
                        &self.config.custom_profile_name,
                    );
                    self.config.custom_profile_name = clean.clone();
                    let mut touch = horizon_client::touch::Settings::load_from(
                        &game_home.join("horizon-touch.json"),
                    );
                    touch.active_profile = clean.clone();
                    let _ = touch.save_profile(&game_home, &clean);
                    self.status =
                        format!("Saved controls as '{clean}' in controlmap/{clean}.json.");
                } else if self.editing == Some("profile_desc") {
                    let game_home = self.config.game_home(home);
                    let mut touch = horizon_client::touch::Settings::load_from(
                        &game_home.join("horizon-touch.json"),
                    );
                    let name = self.config.custom_profile_name.trim().to_owned();
                    let desc = self.config.custom_profile_desc.trim().to_owned();
                    if self.config.preset_edit_mode == "edit" && !touch.is_default_preset() {
                        if !name.is_empty() {
                            touch.preset_name = name.clone();
                        }
                        touch.preset_description = desc;
                        let _ = touch.save_replace_preset(&game_home);
                        self.status = format!(
                            "Updated {} preset '{}'.",
                            touch.control_style.label(),
                            touch.preset_name
                        );
                    } else {
                        let final_name = if name.is_empty() {
                            format!("{} Copy", touch.preset_name)
                        } else {
                            name
                        };
                        let _ = touch.save_new_preset(&game_home, &final_name, &desc);
                        self.status = format!(
                            "Saved new {} preset '{}' (controls/{}/{}.json).",
                            touch.control_style.label(),
                            touch.preset_name,
                            touch.control_style.storage_subdir(),
                            touch.preset_id
                        );
                    }
                    self.config.preset_edit_mode.clear();
                }
                if is_theme_field {
                    self.config.theme.sanitize();
                    horizon_client::ui_theme::set_active_theme(self.config.theme.resolve());
                    self.status = "Custom theme color applied and saved.".into();
                } else {
                    horizon_client::ui_theme::set_active_theme(self.config.theme.resolve());
                }
                self.editing = None;
                let _ = self.config.save(home);
                if self.page.is_catalog() {
                    self.offset = 0;
                    self.search(home);
                }
            }
        }
    }
}

fn install_modrinth_jar_fallback(
    source: &dyn Source,
    project: &Project,
    home: &Path,
    progress: &dyn Fn(&str, f32),
) -> Result<usize, String> {
    let versions = source.versions(project)?;
    let version = browser::pick(Kind::Mod, &versions, true)
        .or_else(|| versions.iter().find(|v| v.file().is_some()))
        .ok_or_else(|| format!("{} has no downloadable mod version.", project.title))?;
    let file = version
        .file()
        .ok_or_else(|| format!("{} has no primary file.", project.title))?;
    let staging = home.join("mod-browser/downloads");
    let title = format!("Downloading {} {}", project.title, version.number);
    let downloaded = source.fetch(file, &staging, &|f| progress(&title, f))?;
    let slug: String = project
        .slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let slug = if slug.is_empty() {
        "mod".to_owned()
    } else {
        slug
    };
    let relative = format!("horizon-mods/{slug}.jar");
    let target = home.join(&relative);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&downloaded, &target).map_err(|e| e.to_string())?;
    browser::java::set_enabled(home, &slug, true)?;
    let (algo, hash) = file.checksum().unwrap_or(("SHA-1", ""));
    let mut installed = Installed::load(home);
    installed
        .entries
        .retain(|e| !(e.source == project.source && e.project == project.id));
    installed.entries.push(browser::installed::Entry {
        source: project.source,
        project: project.id.clone(),
        slug: slug.clone(),
        title: project.title.clone(),
        kind: Kind::Mod,
        version: version.id.clone(),
        number: version.number.clone(),
        file: relative,
        checksum: format!("{algo}:{hash}"),
        page: project.page.clone(),
        dependency: false,
        verdict: Some("Downloaded".into()),
        skipped: Vec::new(),
        namespaces: vec![slug],
    });
    installed.save(home)?;
    progress(&format!("Installed {}", project.title), 1.0);
    Ok(1)
}

fn lookup_modrinth_by_name(title: &str, kind: Kind) -> Option<Project> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let mut query = Query::new(clean_title, kind);
    query.everything = true;
    query.limit = 5;
    let hits = if kind == Kind::Shader {
        browser::modrinth::search_shader_packs(&query).ok()?
    } else {
        browser::modrinth::Modrinth.search(&query).ok()?
    };
    let lower = clean_title.to_lowercase();
    hits.iter()
        .find(|p| {
            p.title.to_lowercase() == lower || p.slug.to_lowercase() == lower.replace(' ', "-")
        })
        .cloned()
        .or_else(|| hits.into_iter().next())
}

fn clean_markdown_body(raw: &str) -> String {
    let mut out = Vec::new();
    let mut in_html = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if out.last().is_some_and(|l: &String| !l.is_empty()) {
                out.push(String::new());
            }
            continue;
        }
        if trimmed.starts_with("![")
            || trimmed.starts_with("<img")
            || trimmed.starts_with("<a ")
            || trimmed.starts_with("<div")
            || trimmed.starts_with("</div")
            || trimmed.starts_with("<p align")
            || trimmed.starts_with("<center")
            || trimmed.starts_with("</center")
        {
            continue;
        }
        let mut clean_line = String::with_capacity(trimmed.len());
        let trimmed = trimmed.trim_start_matches('#').trim();
        for ch in trimmed.chars() {
            if ch == '<' {
                in_html = true;
            } else if ch == '>' {
                in_html = false;
            } else if !in_html && ch != '*' && ch != '`' {
                clean_line.push(ch);
            }
        }
        let collapsed = clean_line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !collapsed.is_empty() {
            out.push(collapsed);
        }
        if out.len() >= 18 {
            break;
        }
    }
    out.join("\n")
}
