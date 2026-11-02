//! Mobile launcher state. Browser operations reuse the desktop backend unchanged.
use crate::browser::{self, Kind, Project, Query, Source, installed::Installed};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};
pub fn source_only_mod(project: &Project) -> bool {
    project.source == browser::SourceId::Horizon
        && project.loaders.iter().any(|loader| loader == "hnmod")
}

/// Temporary dev registration published as Minecraft Nintendo Switch by PrismarineJS.
/// Account tokens and client secrets are never bundled.
pub const MICROSOFT_CLIENT_ID: &str = "00000000441cc96b";

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
    /// Set when this load applied the low-power-GPU defaults; the launcher
    /// shows one status note about it. Never serialized.
    #[serde(skip)]
    pub low_end_tuned: bool,
    /// The device ships no Vulkan driver, so the built-in OpenGL ES renderer
    /// is used regardless of the saved mode. Never serialized.
    #[serde(skip)]
    pub no_vulkan: bool,
    pub renderer: RendererMode,
    pub render_scale: crate::display::RenderScale,
    pub orientation: Orientation,
    #[serde(flatten)]
    pub screen: crate::display::ScreenPreferences,
    pub input_host: InputHost,
    pub keypad_controls: bool,
    pub record_performance: bool,
    #[serde(flatten)]
    pub device: crate::device_preferences::Settings,
    pub shader_source: crate::shader_catalog::Origin,
    pub mod_source: crate::shader_catalog::Origin,
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
            config.renderer = RendererMode::OpenGl;
        }
        // Devices with no Vulkan driver at all rely on Oxide's built-in
        // OpenGL ES renderer; PowerVR GE-series boards (Helio A22 class)
        // have a Vulkan driver that renders but glitches, so they get the
        // same stable default. Fresh installs only; saved choices win.
        let no_vulkan = !crate::device_runtime::vulkan_available();
        config.no_vulkan = no_vulkan;
        let mut low_end = false;
        if value.get("renderer").is_none() && (no_vulkan || crate::device_runtime::low_end_gpu())
        {
            config.renderer = RendererMode::OpenGl;
            low_end = true;
        }
        let tuned = low_end || no_vulkan;
        if tuned && value.get("render_scale").is_none() {
            config.render_scale = crate::display::RenderScale::new(67);
        }
        // Ignore saved app registrations while this dev build uses the fixed ID.
        config.microsoft_client_id =
            crate::account::client_id().unwrap_or_else(|| MICROSOFT_CLIENT_ID.into());
        if value.get("renderer").is_none() && value["gpu_batching"].as_bool() == Some(true) {
            config.renderer = RendererMode::Vulkan;
        }
        config.low_end_tuned = low_end && config.renderer == RendererMode::OpenGl;
        let legacy = std::fs::read(config.game_home(home).join("horizon-touch.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .unwrap_or_default();
        config.device.migrate_missing(&value, &legacy);
        // Weak-GPU defaults: cap the panel refresh request at 60 Hz (a 90 or
        // 120 Hz panel otherwise doubles the frame budget of a GE8300) and
        // let ADPF performance hints boost the game thread. Applied only
        // while the player has saved neither choice.
        if tuned {
            if value.get("display_hz").is_none() && legacy.get("display_hz").is_none() {
                config.device.display_hz = 60;
            }
            if value.get("performance_hints").is_none()
                && legacy.get("performance_hints").is_none()
            {
                config.device.performance_hints = true;
            }
        }
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
}
#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Play,
    Mods,
    Packs,
    Shaders,
    Installed,
    Settings,
}
impl Page {
    pub const ALL: [Self; 6] = [
        Self::Play,
        Self::Mods,
        Self::Packs,
        Self::Shaders,
        Self::Installed,
        Self::Settings,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Mods => "Mods",
            Self::Packs => "Packs",
            Self::Shaders => "Shaders",
            Self::Installed => "Installed",
            Self::Settings => "Settings",
        }
    }
    pub fn is_catalog(self) -> bool {
        matches!(self, Self::Packs | Self::Mods | Self::Shaders)
    }
}
enum Reply {
    Done(String, horizon_client::content_files::Kind),
}
pub struct Catalog {
    pub local: Vec<LocalItem>,
    pub sections: [bool; 3],
    pub pending_delete: Option<LocalItem>,
    downloaded: Vec<(browser::SourceId, String)>,
    downloading: Option<(browser::SourceId, String)>,
    progress: Arc<Mutex<f32>>,
    failed: Option<(browser::SourceId, String)>,
    pub page: Page,
    pub query: String,
    pub results: Vec<Project>,
    pub status: String,
    pub offset: usize,
    pub scroll: f32,
    pub config: Config,
    pub editing: Option<&'static str>,
    pub render_scale_text: String,
    worker: Option<mpsc::Receiver<Result<Reply, String>>>,
    search_worker: Option<mpsc::Receiver<Result<Vec<Project>, String>>>,
    searches: crate::search_queue::Queue<(Page, browser::SourceId, String, usize), Vec<Project>>,
}
#[derive(Clone)]
pub struct LocalItem {
    pub kind: horizon_client::content_files::Kind,
    pub file: String,
    pub title: String,
    pub enabled: bool,
    /// Set for Java mod JARs (Fabric/Forge/Quilt/NeoForge) found in the mods
    /// folder: a loader+name summary. Oxide is a Rust client, so these are
    /// listed for explanation and removal, never loaded.
    pub java: Option<String>,
}

/// Summarises a Java mod JAR so the launcher can list it and explain why it
/// cannot run: the loader flavour plus the name/version from its metadata.
fn java_mod_summary(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > 64 << 20 {
        return None;
    }
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut meta: Option<(&'static str, String)> = None;
    for (loader, entry) in [
        ("Fabric", "fabric.mod.json"),
        ("Quilt", "quilt.mod.json"),
        ("NeoForge", "META-INF/neoforge.mods.toml"),
        ("Forge", "META-INF/mods.toml"),
        ("Forge", "mcmod.info"),
    ] {
        if let Ok(mut entry) = zip.by_name(entry) {
            let mut raw = Vec::new();
            if entry.take(256 * 1024).read_to_end(&mut raw).is_ok() {
                meta = Some((loader, String::from_utf8_lossy(&raw).into_owned()));
                break;
            }
        }
    }
    let (loader, text) = meta?;
    let (name, version) = if matches!(loader, "Forge" | "NeoForge")
        && !text.trim_start().starts_with('[')
    {
        (toml_field(&text, "displayName"), toml_field(&text, "version"))
    } else {
        let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let field = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .or_else(|| {
                    value
                        .pointer(&format!("/quilt_loader/metadata/{key}"))
                        .and_then(|v| v.as_str())
                })
                .or_else(|| {
                    value
                        .pointer(&format!("/quilt_loader/{key}"))
                        .and_then(|v| v.as_str())
                })
                .or_else(|| value.get(0).and_then(|v| v.get(key)).and_then(|v| v.as_str()))
                .map(str::to_owned)
        };
        (field("name"), field("version"))
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = name.filter(|n| !n.is_empty()).unwrap_or(stem);
    let mut summary = format!("{loader} mod: {name}");
    if let Some(version) = version.filter(|v| !v.is_empty() && v.len() <= 40) {
        summary.push(' ');
        summary.push_str(&version);
    }
    Some(summary)
}

/// Reads a `key = "value"` line from a Forge/NeoForge mods.toml.
fn toml_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix(key) {
            if let Some(rest) = rest.trim_start().strip_prefix('=') {
                let value = rest.trim().trim_matches(['"', '\'']);
                if !value.is_empty() {
                    return Some(value.to_owned());
                }
            }
        }
    }
    None
}

/// Release assets of Oxide's first-party mods (Copper, the mobile
/// performance suite): an index plus the validated `.hmod` packages.
const OXIDE_RELEASE_BASE: &str =
    "https://raw.githubusercontent.com/kushaladhikari982648-lgtm/Oxide/copper-v1/";

/// The Oxide source's catalogue: first-party mods from this repository's
/// release assets. The download URL rides in `Project::icon` because these
/// mods have no shared browser source to resolve files through.
fn oxide_search(text: &str, offset: usize) -> Result<Vec<Project>, String> {
    use std::io::Read;
    let mut raw = Vec::new();
    ureq::AgentBuilder::new()
        .https_only(true)
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(30))
        .build()
        .get(&format!("{OXIDE_RELEASE_BASE}index.json"))
        .call()
        .map_err(|e| e.to_string())?
        .into_reader()
        .take(1024 * 1024)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let mut all: Vec<Project> = value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let text = |key: &str| entry.get(key).and_then(|v| v.as_str());
                    let file = text("file")?;
                    let id = text("id")?;
                    Some(Project {
                        source: browser::SourceId::Horizon,
                        id: id.to_owned(),
                        slug: id.to_owned(),
                        title: text("name")?.to_owned(),
                        summary: text("summary").unwrap_or_default().to_owned(),
                        author: text("author").unwrap_or("Oxide").to_owned(),
                        downloads: entry.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                        page: text("page").unwrap_or_default().to_owned(),
                        icon: Some(format!("{OXIDE_RELEASE_BASE}{file}")),
                        license: "MIT".into(),
                        kind: Kind::Mod,
                        game_versions: vec![browser::GAME.into()],
                        loaders: vec!["horizon".into()],
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let needle = text.trim().to_lowercase();
    if !needle.is_empty() {
        all.retain(|project| {
            project.title.to_lowercase().contains(&needle)
                || project.summary.to_lowercase().contains(&needle)
        });
    }
    Ok(all.into_iter().skip(offset).take(12).collect())
}

/// Explains, for a Java mod found on a mod source, that Oxide cannot run it
/// and what the native equivalent of the popular ones is.
fn java_mod_status(project: &browser::Project) -> String {
    let loader = project
        .loaders
        .iter()
        .find(|l| {
            matches!(
                l.as_str(),
                "fabric" | "forge" | "quilt" | "neoforge" | "liteloader"
            )
        })
        .map(String::as_str)
        .unwrap_or("Java");
    let slug = project.slug.to_ascii_lowercase();
    let equivalent = if slug.contains("sodium")
        || slug.contains("rubidium")
        || slug.contains("embeddium")
        || slug.contains("magnesium")
    {
        "Oxide's renderer is already native Rust; tune Client resolution and Rendering in Settings for performance."
    } else if slug.contains("lithium") || slug.contains("pheromone") || slug.contains("krypton") {
        "Oxide's game loop and networking are native Rust rewrites; no optimizer mod is needed."
    } else if slug.contains("iris") || slug.contains("optifine") || slug.contains("oculus") {
        "For shader packs use the Shaders page: Horizon .hshader packs and experimental shader ZIPs."
    } else if slug.contains("starlight") || slug.contains("phosphor") {
        "Oxide's lighting engine is a native Rust rewrite; no mod is needed."
    } else {
        "Browse the Horizon source on the Mods page for sandboxed .hmod mods, or see docs/mods.md to build one."
    };
    format!(
        "{} is a {loader} (Java) mod: Oxide is a Rust client and cannot run Java bytecode. {equivalent}",
        project.title
    )
}
impl Catalog {
    /// Anything the launcher paints as movement: a running download, search
    /// or their workers. The idle redraw pacing keeps 10 Hz while this is true.
    pub fn animating(&self) -> bool {
        self.downloading.is_some() || self.worker.is_some() || self.search_worker.is_some()
    }
    pub fn new(home: &Path) -> Self {
        let config = Config::load(home);
        let mut result = Self {
            local: vec![],
            sections: [false, false, false],
            pending_delete: None,
            downloaded: vec![],
            downloading: None,
            progress: Arc::new(Mutex::new(0.0)),
            failed: None,
            page: Page::Play,
            query: String::new(),
            results: vec![],
            status: String::new(),
            offset: 0,
            scroll: 0.0,
            config,
            editing: None,
            render_scale_text: String::new(),
            worker: None,
            search_worker: None,
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
        self.sections = [false; 3];
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
            .filter(|e| game.join(&e.file).is_file())
            .map(|e| (e.source, e.project.clone()))
            .collect();
        let on = horizon_client::mod_list::ModList::load(&game);
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
                if !horizon_client::content_files::plain_name(&name) {
                    continue;
                }
                // Java mod JARs are listed (never loaded) so players get an
                // explanation instead of a mod that silently does nothing.
                let java = if kind == horizon_client::content_files::Kind::Mods
                    && name.to_ascii_lowercase().ends_with(".jar")
                {
                    java_mod_summary(&entry.path())
                } else {
                    None
                };
                if java.is_none() && !kind.accepts(&name) {
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
                        .is_some_and(|id| on.enabled.contains(id)),
                };
                self.local.push(LocalItem {
                    kind,
                    file: name.clone(),
                    title: name,
                    enabled,
                    java,
                });
            }
        }
        self.local.sort_by_key(|e| e.title.to_lowercase());
    }
    pub fn busy(&self) -> bool {
        self.worker.is_some()
    }
    pub fn download_label(&self, project: &Project) -> Option<String> {
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
                        && item.file == format!("{}.hmod", project.id)
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
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(job());
        });
    }
    pub fn poll(&mut self, home: &Path) {
        let reply = self.worker.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Launcher worker stopped".into())),
            Err(_) => None,
        });
        if let Some(reply) = reply {
            self.worker = None;
            let downloading = self.downloading.take();
            if reply.is_err() {
                self.failed = downloading;
            }
            match reply {
                Ok(Reply::Done(s, kind)) => {
                    self.status = s;
                    self.show_installed(Some(kind));
                }
                Err(e) => self.status = e,
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
                    Ok(items) => self.show_results(items),
                    Err(error) => self.status = error,
                }
            }
        }
        self.start_search();
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
            let native = items.iter().filter(|p| source_only_mod(p)).count();
            format!(
                "{} .hmod / {} source-only .hnmod (not supported yet)",
                items.len() - native,
                native
            )
        } else {
            format!(
                "{} results - select Install to check compatibility",
                items.len()
            )
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
            query.limit = 12;
            let result = if page == Page::Shaders && source == browser::SourceId::Modrinth {
                query.everything = true;
                browser::modrinth::search_shader_packs(&query)
            } else if page == Page::Packs
                || (page == Page::Mods && source == browser::SourceId::Modrinth)
            {
                query.everything = true;
                browser::modrinth::Modrinth.search(&query)
            } else if page == Page::Mods && source == browser::SourceId::CurseForge {
                // The Oxide source sentinel: first-party mods, own index.
                oxide_search(&text, offset)
            } else {
                browser::horizon::Registry.search(&query)
            };
            let _ = tx.send(result);
        });
    }
    pub fn import_native(&mut self, home: &Path) {
        let url = self.query.trim().to_owned();
        let game = self.config.game_home(home);
        self.job("Downloading sandboxed .hmod over HTTPS...",move||{
            use std::io::Read;
            let mut bytes=Vec::new();
            ureq::AgentBuilder::new().https_only(true).timeout_connect(std::time::Duration::from_secs(20)).timeout_read(std::time::Duration::from_secs(90)).build().get(&url).call().map_err(|e|e.to_string())?.into_reader().take(32*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
            if bytes.len()>32*1024*1024 {return Err("Horizon mod exceeds 32 MiB".into());}
            let dir=game.join("horizon-mods");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
            let stage=dir.join("import.partial");std::fs::write(&stage,bytes).map_err(|e|e.to_string())?;
            let result=(||{
                let package=horizon_mods::Package::read(&stage)?;
                let destination=dir.join(format!("{}.hmod",package.manifest.id));
                if destination.exists(){std::fs::copy(&destination,destination.with_extension("hmod.previous")).map_err(|e|e.to_string())?;}
                std::fs::rename(&stage,destination).map_err(|e|e.to_string())?;
                Ok(Reply::Done(format!("Installed {}. Enable it in Installed after reviewing its capabilities in game Mods.",package.manifest.name), horizon_client::content_files::Kind::Mods))
            })();
            let _=std::fs::remove_file(stage);result
        });
    }
    /// Downloads a first-party Oxide mod (Copper) over HTTPS and validates
    /// it exactly like an imported `.hmod` before it replaces an old copy.
    fn oxide_install(&mut self, home: &Path, url: String) {
        let game = self.config.game_home(home);
        self.job("Downloading Oxide mod over HTTPS...", move || {
            use std::io::Read;
            let mut bytes = Vec::new();
            ureq::AgentBuilder::new().https_only(true).timeout_connect(std::time::Duration::from_secs(20)).timeout_read(std::time::Duration::from_secs(90)).build().get(&url).call().map_err(|e|e.to_string())?.into_reader().take(32*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
            if bytes.len() > 32 * 1024 * 1024 {
                return Err("Oxide mod exceeds 32 MiB".into());
            }
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
                Ok(Reply::Done(
                    format!(
                        "Installed {}. Enable it in Installed, then Play: Copper paces chunk streaming from the next world load.",
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
            self.status = "This .hnmod needs a separately compiled client. Android support is planned; sandboxed .hmod mods can be installed now.".into();
            return;
        }
        if project.kind == Kind::Mod && project.source != browser::SourceId::Horizon {
            self.status = java_mod_status(&project);
            return;
        }
        if self.busy() || self.download_label(&project).is_some() {
            return;
        }
        // First-party Oxide mods (Copper): a direct HTTPS .hmod from this
        // repository's release assets, validated like an imported one.
        if let Some(url) = project
            .icon
            .clone()
            .filter(|icon| icon.starts_with(OXIDE_RELEASE_BASE))
        {
            self.oxide_install(home, url);
            return;
        }
        let game = self.config.game_home(home);
        if project.kind != Kind::ResourcePack
            && !(project.kind == Kind::Shader
                && matches!(
                    project.source,
                    browser::SourceId::Horizon | browser::SourceId::Modrinth
                ))
            && !(cfg!(target_pointer_width = "64")
                && project.kind == Kind::Mod
                && project.source == browser::SourceId::Horizon)
        {
            return;
        }
        self.downloading = Some((project.source, project.id.clone()));
        self.failed = None;
        *self.progress.lock().unwrap_or_else(|e| e.into_inner()) = 0.0;
        let progress = self.progress.clone();
        let status =
            if project.kind == Kind::Shader && project.source == browser::SourceId::Modrinth {
                "Downloading experimental shader ZIP..."
            } else {
                "Checking compatibility and downloading..."
            };
        self.job(status, move || {
            std::fs::create_dir_all(&game).map_err(|e| e.to_string())?;
            let source = browser::source(project.source).ok_or("Unsupported download source")?;
            let count = if project.kind == Kind::Shader
                && project.source == browser::SourceId::Modrinth
            {
                crate::shader_catalog::install(source.as_ref(), &project, &game, &|_name, p| {
                    if p.is_finite() {
                        *progress.lock().unwrap_or_else(|e| e.into_inner()) = p.clamp(0.0, 1.0);
                    }
                })?
            } else {
                browser::installed::install(source.as_ref(), &project, true, &game, &|_name, p| {
                    if p.is_finite() {
                        *progress.lock().unwrap_or_else(|e| e.into_inner()) = p.clamp(0.0, 1.0);
                    }
                })?
                .len()
            };
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
        if let Some(summary) = &item.java {
            self.status = format!("{summary}: Java mods cannot run in Oxide, a Rust client. Delete it here; docs/mods.md lists the native equivalents.");
            return;
        }
        let game = self.config.game_home(home);
        let path = game.join("options.txt");
        let mut settings = horizon_client::settings::Settings::load(&path);
        match item.kind {
            horizon_client::content_files::Kind::Mods => {
                if !cfg!(target_pointer_width = "64") {
                    // Content mods (api 0.2, no code) load in every build;
                    // sandboxed code mods need wasmtime, which has no 32-bit
                    // ARM target. Copper and friends are content packages.
                    let content_only = horizon_mods::Package::read(&game.join(&item.file))
                        .is_ok_and(|package| package.manifest.api == "0.2");
                    if !content_only {
                        self.status = "Sandboxed code mods require a 64-bit APK".into();
                        return;
                    }
                }
                let mut on = horizon_client::mod_list::ModList::load(&game);
                if let Some(id) = item.file.strip_suffix(".hmod") {
                    if item.enabled {
                        on.enabled.remove(id);
                    } else {
                        on.enabled.insert(id.to_owned());
                    }
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
                if let Some(id) = item.file.strip_suffix(".hmod") {
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
        let field = match self.editing {
            Some("name") => &mut self.config.name,
            Some("server") => &mut self.config.server,
            Some("render_scale") => &mut self.render_scale_text,
            Some("client_id") => &mut self.config.microsoft_client_id,
            _ => &mut self.query,
        };
        match out {
            Output::Text(s) => field.push_str(&s),
            Output::Backspace => {
                field.pop();
            }
            Output::Enter | Output::Close => {
                if self.editing == Some("render_scale") {
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
