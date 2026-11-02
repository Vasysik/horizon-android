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

type IconData = (String, u32, u32, Vec<u8>);

pub struct Catalog {
    pub local: Vec<LocalItem>,
    pub bundled_enabled: std::collections::BTreeSet<String>,
    pub sections: [bool; 3],
    pub pending_delete: Option<LocalItem>,
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
}
impl Catalog {
    pub fn new(home: &Path) -> Self {
        let config = Config::load(home);
        let mut result = Self {
            local: vec![],
            bundled_enabled: Default::default(),
            sections: [false, false, false],
            pending_delete: None,
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
                self.local.push(LocalItem {
                    kind,
                    file: name.clone(),
                    title: name,
                    enabled,
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
            if self.local.iter().any(|item| item.file == file_name || item.file == e.file) {
                continue;
            }
            let mod_id = file_name
                .strip_prefix("horizon-native-mods/")
                .unwrap_or(&file_name)
                .strip_suffix(".hnmod")
                .or_else(|| file_name.strip_suffix(".hmod"))
                .or_else(|| file_name.strip_suffix(".jar"))
                .unwrap_or(&file_name);
            self.local.push(LocalItem {
                kind: horizon_client::content_files::Kind::Mods,
                file: e.file.clone(),
                title: format!("{} ({})", e.title, e.version),
                enabled: on.enabled.contains(mod_id),
            });
        }
        self.local.sort_by_key(|e| e.title.to_lowercase());
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
        if s.is_empty() {
            self.status.clone()
        } else {
            s
        }
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
            while let Ok((key, w, h, rgba)) = rx.try_recv() {
                self.icons.insert(key, (w, h, rgba));
            }
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
        let missing: Vec<(String, String)> = self
            .results
            .iter()
            .filter_map(|p| {
                let url = p.icon.as_ref()?.trim();
                if url.is_empty() || self.icons.contains_key(&p.id) {
                    return None;
                }
                Some((p.id.clone(), url.to_owned()))
            })
            .collect();
        if missing.is_empty() {
            return;
        }
        let home = home.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.icon_worker = Some(rx);
        std::thread::spawn(move || {
            for (id, url) in missing {
                if let Ok(path) = browser::images::fetch(&home, &url, browser::images::ROW)
                    && let Ok((w, h, rgba)) = horizon_client::hud::decode(&path)
                {
                    let _ = tx.send((id, w, h, rgba));
                }
            }
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
                "{} Modrinth mods ready to port and install",
                items.len()
            )
        } else if self.page == Page::Mods {
            format!(
                "{} Horizon mods available to install",
                items.len()
            )
        } else {
            format!(
                "{} results - select Install to download",
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
        if self.busy() || self.download_label(&project).is_some() {
            return;
        }
        let game = self.config.game_home(home);
        if !matches!(
            project.kind,
            Kind::ResourcePack | Kind::Shader | Kind::Mod
        ) {
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
