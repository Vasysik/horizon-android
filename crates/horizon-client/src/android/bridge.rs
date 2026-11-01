//! The Compose launcher's bridge (`dev.horizon.android.NativeBridge`).
//!
//! The launcher Activity runs in the app's main process and the game runs in its
//! own `:game` process. This bridge gives the Kotlin UI the Rust launcher's state
//! as one JSON snapshot and takes its actions by name. Every call comes from the
//! Android main thread; slow work stays on the same worker threads the native
//! launcher uses.
use crate::catalog::{Catalog, Page};
use crate::launcher::login::Login;
use horizon_client::content_files::Kind;
use jni::{
    JNIEnv,
    objects::{JClass, JObject, JString},
    sys::jstring,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};

struct Controller {
    home: PathBuf,
    catalog: Catalog,
    login: Login,
    install: Arc<Mutex<horizon_client::official_install::InstallProgress>>,
    worker: Option<mpsc::Receiver<Result<(), String>>>,
    diagnostic_export: Option<mpsc::Receiver<Result<String, String>>>,
    error: Option<String>,
}

thread_local! {
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}

/// The file the `:game` process reads to start the game without its own launcher.
pub(crate) const LAUNCH_REQUEST: &str = "launch-request.json";

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Mods => "mods",
        Kind::Packs => "packs",
        Kind::Shaders => "shaders",
    }
}
fn parse_kind(name: &str) -> Option<Kind> {
    Kind::ALL.into_iter().find(|k| kind_name(*k) == name)
}
fn page_name(page: Page) -> &'static str {
    match page {
        Page::Play => "play",
        Page::Mods => "mods",
        Page::Packs => "packs",
        Page::Shaders => "shaders",
        Page::Installed => "installed",
        Page::Settings => "settings",
    }
}

fn profile_dir(home: &std::path::Path, id: u32) -> PathBuf {
    if id == 0 {
        home.into()
    } else {
        home.join("profiles").join(format!("profile-{id}"))
    }
}

/// Player-chosen profile names live beside the profile's saves.
const PROFILE_NAME_FILE: &str = "android-profile-name.txt";

fn profile_name(dir: &std::path::Path, id: u32) -> String {
    std::fs::read_to_string(dir.join(PROFILE_NAME_FILE))
        .ok()
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            if id == 0 {
                "Default".into()
            } else {
                format!("Profile {id}")
            }
        })
}

fn write_profile_name(dir: &std::path::Path, name: &str) -> Result<(), String> {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(32)
        .collect();
    if name.is_empty() {
        let _ = std::fs::remove_file(dir.join(PROFILE_NAME_FILE));
        return Ok(());
    }
    std::fs::write(dir.join(PROFILE_NAME_FILE), name).map_err(|e| e.to_string())
}

fn choice(
    label: impl Into<String>,
    detail: impl Into<String>,
    value: impl ToString,
    selected: bool,
) -> Value {
    json!({"label": label.into(), "detail": detail.into(), "value": value.to_string(), "selected": selected})
}

impl Controller {
    fn new(home: PathBuf) -> Self {
        let cleanup_home = home.clone();
        std::thread::spawn(move || {
            if let Err(error) =
                horizon_client::official_install::cleanup_completed_downloads(&cleanup_home)
            {
                log::warn!("Could not clean completed Minecraft downloads: {error}");
            }
        });
        let mut catalog = Catalog::new(&home);
        if crate::diagnostics::previous_unclean(&home) {
            catalog.status = crate::diagnostics::summary(&home);
        }
        horizon_client::android_runtime::graphics_startup::configure(home.clone());
        if horizon_client::android_runtime::graphics_startup::recover(&home) {
            catalog.config.renderer = crate::catalog::RendererMode::OpenGl;
            if let Err(error) = catalog.config.save(&home) {
                log::warn!("Could not save graphics recovery choice: {error}");
            }
            catalog.status = "Previous Vulkan startup was interrupted. Using OpenGL ES recovery; you can change Rendering mode in Settings.".into();
        }
        let mut controller = Self {
            home,
            catalog,
            login: Login::new(),
            install: Arc::default(),
            worker: None,
            diagnostic_export: None,
            error: None,
        };
        controller.resume();
        controller
    }

    /// The launcher Activity came back to the front, usually from the game.
    fn resume(&mut self) {
        self.catalog.config = crate::catalog::Config::load(&self.home);
        self.catalog.refresh(&self.home);
        self.login.refresh_status();
        if let Ok(message) = std::fs::read_to_string(self.home.join("android-error.txt")) {
            self.error = Some(message);
        }
    }

    fn poll(&mut self) {
        if let Some(result) = self.worker.as_ref().and_then(|w| w.try_recv().ok()) {
            self.worker = None;
            if let Err(error) = result {
                self.error = Some(
                    if error.contains("Dns Failed") || error.contains("lookup address") {
                        "Minecraft download interrupted: DNS could not resolve the download host. Check your connection, VPN or Private DNS, then retry Download. Completed files are kept.".into()
                    } else {
                        error
                    },
                );
                crate::diagnostics::record_error(self.error.as_deref().unwrap_or_default());
            }
        }
        if let Some(result) = self
            .diagnostic_export
            .as_ref()
            .and_then(|r| r.try_recv().ok())
        {
            self.diagnostic_export = None;
            self.catalog.status = match result {
                Ok(name) => {
                    let _ = crate::platform::open_documents("reports");
                    format!(
                        "Saved {name} in Reports. Review the ZIP before sharing; nothing was uploaded."
                    )
                }
                Err(error) => format!("Diagnostic export failed: {error}"),
            };
        }
        self.catalog.poll(&self.home);
        if let Some(notice) = crate::documents::poll() {
            self.catalog.status = notice.text;
            if notice.changed {
                self.catalog.refresh(&self.home);
            }
        }
        for result in crate::gallery::poll() {
            self.catalog.status = result.unwrap_or_else(|e| format!("File export failed: {e}"));
        }
        self.login.poll();
    }

    fn settings(&self) -> Value {
        let c = &self.catalog.config;
        let row = |id: &str, title: &str, value: String, kind: &str| json!({"id": id, "title": title, "value": value, "kind": kind});
        let switch = |id: &str, title: &str, value: &str, on: bool| json!({"id": id, "title": title, "value": value, "kind": "switch", "on": on});
        json!([
            {"group": "Graphics", "rows": [
                row("renderer", "Rendering", c.renderer.label().into(), "choice"),
                row("render_scale", "Client resolution", format!("{}%", c.render_scale.percent()), "choice"),
                row("display_hz", "Display refresh rate", c.device.display_label(), "choice"),
                switch("performance_hints", "Performance hints", if c.device.performance_hints { "On (ADPF)" } else { "Off" }, c.device.performance_hints),
            ]},
            {"group": "Screen", "rows": [
                row("orientation", "Orientation", c.orientation.label().into(), "choice"),
                row("aspect", "Aspect ratio", c.screen.aspect.label().into(), "choice"),
                switch("immersive", "Immersive fullscreen", if c.screen.immersive { "On" } else { "Off" }, c.screen.immersive),
                switch("cutout", "Avoid camera cutout", if c.screen.avoid_cutout { "On" } else { "Off" }, c.screen.avoid_cutout),
            ]},
            {"group": "Game", "rows": [
                row("profile", "Profile", profile_name(&profile_dir(&self.home, c.profile), c.profile), "choice"),
                row("input_host", "Input", c.input_host.label().into(), "choice"),
                row("name", "Offline player name", if c.name.is_empty() { "Not set".into() } else { c.name.clone() }, "text"),
                row("server", "Server", if c.server.is_empty() { "Singleplayer".into() } else { c.server.clone() }, "text"),
            ]},
            {"group": "Diagnostics", "rows": [
                switch("gpu_profile", "GPU profiling", if c.device.gpu_profile { "On / included in report" } else { "Off" }, c.device.gpu_profile),
                switch("record_performance", "Performance recording", if c.record_performance { "On / up to 30 min" } else { "Off" }, c.record_performance),
                row("export_diagnostics", "Diagnostic report", "Export one ZIP to share".into(), "action"),
                row("export_performance", "Raw performance data", "Advanced / JSONL in Reports".into(), "action"),
                row("logs", "Last session summary", "Show what happened last time".into(), "action"),
            ]},
        ])
    }

    fn options(&self, id: &str) -> Value {
        let c = &self.catalog.config;
        let (title, help, choices): (&str, &str, Vec<Value>) = match id {
            "renderer" => (
                "Rendering mode",
                "Uses the system GPU driver. Applies on the next game launch.",
                vec![
                    choice(
                        "OpenGL ES / experimental",
                        "For older GPUs. Requires ES 3.0+ and sufficient driver limits.",
                        "opengl",
                        c.renderer == crate::catalog::RendererMode::OpenGl,
                    ),
                    choice(
                        "Vulkan / GPU batching",
                        "Groups terrain draws and enables supported GPU culling.",
                        "vulkan",
                        c.renderer == crate::catalog::RendererMode::Vulkan,
                    ),
                    choice(
                        "Vulkan / compatibility",
                        "Direct terrain draws for drivers with missing or transparent blocks.",
                        "vulkan_compat",
                        c.renderer == crate::catalog::RendererMode::VulkanCompatibility,
                    ),
                ],
            ),
            "render_scale" => (
                "Client resolution",
                "Lower values render fewer pixels for the whole picture.",
                crate::display::RenderScale::PRESETS
                    .into_iter()
                    .map(|p| {
                        choice(
                            format!("{p}%"),
                            if p == 100 {
                                "Native resolution"
                            } else {
                                "Lower resolution for the whole picture"
                            },
                            p,
                            c.render_scale.percent() == p,
                        )
                    })
                    .collect(),
            ),
            "display_hz" => (
                "Display refresh rate",
                "An Android display preference, not a game FPS cap. Unsupported rates may be ignored.",
                crate::device_preferences::DISPLAY_RATES
                    .into_iter()
                    .map(|hz| {
                        choice(
                            if hz == 0 {
                                "System default".into()
                            } else {
                                format!("{hz} Hz")
                            },
                            "",
                            hz,
                            c.device.display_hz == hz,
                        )
                    })
                    .collect(),
            ),
            "orientation" => (
                "Screen orientation",
                "Applies to the game. Does not change touch bindings.",
                crate::catalog::Orientation::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(i, o)| choice(o.label(), "", i, c.orientation == o))
                    .collect(),
            ),
            "aspect" => (
                "Aspect ratio",
                "Fits the entire picture without stretching. Portrait swaps width and height.",
                ASPECTS
                    .into_iter()
                    .enumerate()
                    .map(|(i, a)| choice(a.label(), "", i, c.screen.aspect == a))
                    .collect(),
            ),
            "input_host" => (
                "Input mode",
                "Mouse mode uses a physical keyboard. Touch mode keeps the Android keyboard.",
                crate::catalog::InputHost::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(i, m)| choice(m.label(), "", i, c.input_host == m))
                    .collect(),
            ),
            "profile" => {
                let mut ids = vec![0, c.profile];
                ids.extend(
                    std::fs::read_dir(self.home.join("profiles"))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.path().is_dir())
                        .filter_map(|e| {
                            e.file_name()
                                .to_str()?
                                .strip_prefix("profile-")?
                                .parse::<u32>()
                                .ok()
                        }),
                );
                ids.sort_unstable();
                ids.dedup();
                let next = ids.last().copied().unwrap_or(0).checked_add(1);
                let mut choices: Vec<_> = ids
                    .into_iter()
                    .map(|id| {
                        choice(
                            format!("Profile {id}"),
                            if id == 0 {
                                "Default profile"
                            } else {
                                "Separate saves, mods and packs"
                            },
                            id,
                            id == c.profile,
                        )
                    })
                    .collect();
                if let Some(id) = next {
                    choices.push(choice(
                        "Create profile",
                        format!("Create Profile {id}"),
                        id,
                        false,
                    ));
                }
                (
                    "Choose profile",
                    "Each profile keeps its own game settings and saves.",
                    choices,
                )
            }
            _ => ("", "", vec![]),
        };
        json!({"title": title, "help": help, "choices": choices})
    }

    fn snapshot(&mut self) -> Value {
        use horizon_client::official_install::InstallStage;
        self.poll();
        let installed = horizon_client::official_install::ready(&self.home);
        let progress = *self.install.lock().unwrap_or_else(|e| e.into_inner());
        let current = InstallStage::ALL
            .iter()
            .position(|s| *s == progress.stage)
            .unwrap_or(0);
        let fraction = if progress.total > 0 {
            progress.done as f32 / progress.total as f32
        } else {
            0.0
        };
        let detail = match progress.stage {
            InstallStage::Download => format!(
                "{:.1} / {:.1} MiB · verified downloads",
                progress.done as f64 / 1048576.0,
                progress.total as f64 / 1048576.0
            ),
            InstallStage::Controls => {
                format!(
                    "{} / {} control files verified",
                    progress.done, progress.total
                )
            }
            _ => "Please keep Horizon open. Downloaded files are kept for retry.".into(),
        };
        let c = &self.catalog.config;
        let results: Vec<Value> = self
            .catalog
            .results
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let state = self.catalog.download_label(p);
                let java_mod = p.kind == crate::browser::Kind::Mod
                    && p.source == crate::browser::SourceId::Modrinth;
                let note = if java_mod {
                    Some("Java mod")
                } else if crate::catalog::source_only_mod(p) {
                    Some(".hnmod: later")
                } else if p.kind == crate::browser::Kind::Mod && !cfg!(target_pointer_width = "64")
                {
                    Some("64-bit required")
                } else {
                    None
                };
                json!({
                    "index": i,
                    "title": p.title,
                    "summary": p.summary,
                    "author": p.author,
                    "downloads": p.downloads,
                    "state": state,
                    "note": note,
                    "button": self.catalog.install_button(p),
                })
            })
            .collect();
        let local: Vec<Value> = self
            .catalog
            .local
            .iter()
            .enumerate()
            .map(|(i, item)| {
                json!({
                    "index": i,
                    "kind": kind_name(item.kind),
                    "title": item.title,
                    "file": item.file,
                    "enabled": item.enabled,
                    "toggle": item.kind != Kind::Mods || cfg!(target_pointer_width = "64"),
                })
            })
            .collect();
        let code = match self.login.state() {
            Some(crate::account::SignInState::Code(code)) => {
                Some(json!({"user_code": code.user_code, "uri": code.verification_uri}))
            }
            _ => None,
        };
        json!({
            "version": crate::backend::VERSION,
            "status": self.catalog.status,
            "error": self.error,
            "install": {
                "installed": installed,
                "installing": self.worker.is_some(),
                "stage": progress.stage.label(),
                "stage_index": current,
                "stages": InstallStage::ALL.iter().map(|s| s.label()).collect::<Vec<_>>(),
                "overall": (current as f32 + fraction.clamp(0.0, 1.0)) / InstallStage::ALL.len() as f32,
                "detail": detail,
            },
            "account": {
                "status": self.login.status(&c.name),
                "signed_in": self.login.signed_in,
                "profile": self.login.profile,
                "pending": self.login.pending(),
                "message": self.login.message(),
                "code": code,
                "needs_review": self.login.message().contains("not approved by Mojang"),
            },
            "config": {
                "name": c.name,
                "server": c.server,
                "profile": c.profile,
                "profile_name": profile_name(&profile_dir(&self.home, c.profile), c.profile),
                "renderer": c.renderer.label(),
                "render_scale": c.render_scale.percent(),
                "input": c.input_host.label(),
                "mod_source": if c.mod_source == crate::shader_catalog::Origin::Modrinth { "modrinth" } else { "horizon" },
                "shader_source": if c.shader_source == crate::shader_catalog::Origin::Modrinth { "modrinth" } else { "horizon" },
            },
            "settings": self.settings(),
            "catalog": {
                "page": page_name(self.catalog.page),
                "query": self.catalog.query,
                "results": results,
                "offset": self.catalog.offset,
                "busy": self.catalog.busy(),
            },
            "local": local,
            "documents": crate::documents::overlay(),
            "is_64_bit": cfg!(target_pointer_width = "64"),
        })
    }

    fn save(&mut self) -> Result<(), String> {
        self.catalog.config.save(&self.home)
    }

    /// Profile 0 plus every `profiles/profile-N` folder, sorted.
    fn profile_ids(&self) -> Vec<u32> {
        let mut ids = vec![0, self.catalog.config.profile];
        ids.extend(
            std::fs::read_dir(self.home.join("profiles"))
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()?
                        .strip_prefix("profile-")?
                        .parse::<u32>()
                        .ok()
                }),
        );
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// What the profile manager shows: names and what each profile holds.
    fn profiles(&self) -> Value {
        let count = |dir: PathBuf| {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .count()
        };
        Value::Array(
            self.profile_ids()
                .into_iter()
                .map(|id| {
                    let dir = profile_dir(&self.home, id);
                    json!({
                        "id": id,
                        "name": profile_name(&dir, id),
                        "active": id == self.catalog.config.profile,
                        "worlds": count(dir.join("saves")),
                        "mods": count(dir.join("horizon-mods")),
                        "packs": count(dir.join("resourcepacks")),
                        "shaders": count(dir.join("shaderpacks")),
                    })
                })
                .collect(),
        )
    }

    fn call(&mut self, method: &str, arg: &str) -> Result<Value, String> {
        let home = self.home.clone();
        match method {
            "resume" => self.resume(),
            "dismiss_error" => {
                self.error = None;
                let _ = std::fs::remove_file(home.join("android-error.txt"));
            }
            "dismiss_status" => self.catalog.status.clear(),
            "page" => {
                let page = Page::ALL
                    .into_iter()
                    .find(|p| page_name(*p) == arg)
                    .ok_or("Unknown page")?;
                self.catalog.page = page;
                self.catalog.offset = 0;
                self.error = None;
                self.catalog.browse(&home);
            }
            "search" => {
                self.catalog.query = arg.to_owned();
                self.catalog.offset = 0;
                self.catalog.search(&home);
            }
            "next" => {
                self.catalog.offset += 12;
                self.catalog.search(&home);
            }
            "previous" => {
                self.catalog.offset = self.catalog.offset.saturating_sub(12);
                self.catalog.search(&home);
            }
            "source" => {
                let origin = if arg == "modrinth" {
                    crate::shader_catalog::Origin::Modrinth
                } else {
                    crate::shader_catalog::Origin::Horizon
                };
                if self.catalog.page == Page::Mods {
                    self.catalog.config.mod_source = origin;
                } else {
                    self.catalog.config.shader_source = origin;
                }
                self.save()?;
                self.catalog.offset = 0;
                self.catalog.browse(&home);
            }
            "install" => {
                let index = arg.parse::<usize>().map_err(|e| e.to_string())?;
                self.catalog.install(&home, index);
            }
            "toggle_local" => {
                let index = arg.parse::<usize>().map_err(|e| e.to_string())?;
                self.catalog.toggle_local(&home, index);
            }
            "remove_local" => {
                let index = arg.parse::<usize>().map_err(|e| e.to_string())?;
                self.catalog.pending_delete = self.catalog.local.get(index).cloned();
                self.catalog.remove_local(&home);
            }
            "open_folder" => {
                let folder = match parse_kind(arg).ok_or("Unknown content kind")? {
                    Kind::Mods => "mods",
                    Kind::Packs => "packs",
                    Kind::Shaders => "shaders",
                };
                let id = crate::document_store::folder_id(self.catalog.config.profile, folder);
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Opened in Files. Return here after copying files.".into()
                } else {
                    "Open the system Files app and choose Horizon Android.".into()
                };
            }
            "import_conflict" => crate::documents::resolve(match arg {
                "keep_both" => crate::documents::Choice::KeepBoth,
                "replace" => crate::documents::Choice::Replace,
                _ => crate::documents::Choice::Cancel,
            }),
            "import_cancel" => crate::documents::cancel(),
            "choose" => {
                let (id, value) = arg.split_once('=').ok_or("Expected id=value")?;
                self.choose(id, value)?;
            }
            "toggle" => self.toggle(arg)?,
            "text" => {
                let (id, value) = arg.split_once('=').ok_or("Expected id=value")?;
                let value = value.trim().to_owned();
                match id {
                    "name" => self.catalog.config.name = value.chars().take(16).collect(),
                    "server" => self.catalog.config.server = value,
                    "render_scale" => {
                        let percent = value
                            .trim_end_matches('%')
                            .parse::<u32>()
                            .map_err(|_| "Enter a percentage from 25 to 100")?;
                        self.catalog.config.render_scale =
                            crate::display::RenderScale::new(percent);
                    }
                    "client_id" => self.catalog.config.microsoft_client_id = value,
                    _ => return Err("Unknown field".into()),
                }
                self.save()?;
            }
            "sign_in" => self.login.start(&self.catalog.config.microsoft_client_id),
            "device_sign_in" => self
                .login
                .start_device(&self.catalog.config.microsoft_client_id),
            "copy_code" => self.login.copy_code(),
            "open_link" => self.login.open_link(),
            "cancel_login" => self.login.close(),
            "sign_out" => self.login.sign_out(),
            "review_app" => self.login.review_app(),
            "repository" => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            "logs" => {
                self.catalog.status = crate::diagnostics::summary(&home);
                if let Ok(error) = std::fs::read_to_string(home.join("android-error.txt")) {
                    self.catalog.status = format!(
                        "Last handled error: {}. {}",
                        crate::diagnostics::redact(&error),
                        self.catalog.status
                    );
                }
            }
            "export_diagnostics" if self.diagnostic_export.is_none() => {
                let (sender, receiver) = mpsc::channel();
                self.diagnostic_export = Some(receiver);
                self.catalog.status =
                    "Preparing local diagnostic ZIP. No upload; review before sharing.".into();
                std::thread::spawn(move || {
                    let result = crate::diagnostics::export(&home).map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                    let _ = sender.send(result);
                });
            }
            "export_diagnostics" => {}
            "export_performance" => {
                self.catalog.status = match crate::performance::export(&home) {
                    Ok(()) => "Reports opened in Files. Copy or share a report from there.".into(),
                    Err(e) => e,
                };
            }
            "play" => return self.play(),
            "options" => return Ok(self.options(arg)),
            "profiles" => return Ok(self.profiles()),
            "profile_create" => {
                let id = self
                    .profile_ids()
                    .last()
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or("Too many profiles")?;
                let dir = profile_dir(&home, id);
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                write_profile_name(&dir, arg)?;
                self.choose("profile", &id.to_string())?;
            }
            "profile_rename" => {
                let (id, name) = arg.split_once('=').ok_or("Expected id=name")?;
                let id = id.parse::<u32>().map_err(|e| e.to_string())?;
                write_profile_name(&profile_dir(&home, id), name)?;
            }
            "profile_delete" => {
                let id = arg.parse::<u32>().map_err(|e| e.to_string())?;
                if id == 0 {
                    return Err("The default profile cannot be deleted".into());
                }
                if id == self.catalog.config.profile {
                    return Err("Switch to another profile before deleting this one".into());
                }
                std::fs::remove_dir_all(profile_dir(&home, id)).map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Unknown launcher action {method}")),
        }
        Ok(Value::Null)
    }

    fn choose(&mut self, id: &str, value: &str) -> Result<(), String> {
        let index = |len: usize| -> Result<usize, String> {
            value
                .parse::<usize>()
                .ok()
                .filter(|i| *i < len)
                .ok_or_else(|| "Unknown choice".to_owned())
        };
        let c = &mut self.catalog.config;
        match id {
            "renderer" => {
                c.renderer = match value {
                    "opengl" => crate::catalog::RendererMode::OpenGl,
                    "vulkan_compat" => crate::catalog::RendererMode::VulkanCompatibility,
                    _ => crate::catalog::RendererMode::Vulkan,
                }
            }
            "render_scale" => {
                c.render_scale = crate::display::RenderScale::new(
                    value.parse::<u32>().map_err(|e| e.to_string())?,
                )
            }
            "display_hz" => {
                c.device.display_hz = value.parse::<u16>().map_err(|e| e.to_string())?;
                c.device.sanitize();
            }
            "orientation" => {
                c.orientation =
                    crate::catalog::Orientation::ALL[index(crate::catalog::Orientation::ALL.len())?]
            }
            "aspect" => c.screen.aspect = ASPECTS[index(ASPECTS.len())?],
            "input_host" => {
                c.input_host =
                    crate::catalog::InputHost::ALL[index(crate::catalog::InputHost::ALL.len())?]
            }
            "profile" => {
                let profile = value.parse::<u32>().map_err(|e| e.to_string())?;
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })
                .map_err(|e| e.to_string())?;
                c.profile = profile;
                self.save()?;
                self.catalog.refresh(&self.home);
                return Ok(());
            }
            _ => return Err("Unknown setting".into()),
        }
        self.save()
    }

    fn toggle(&mut self, id: &str) -> Result<(), String> {
        let c = &mut self.catalog.config;
        match id {
            "immersive" => c.screen.immersive ^= true,
            "cutout" => c.screen.avoid_cutout ^= true,
            "performance_hints" => {
                c.device.performance_hints ^= true;
                self.catalog.status = "ADPF gives Android CPU frame-work hints on supported devices. It does not cap FPS or guarantee a speedup.".into();
            }
            "gpu_profile" => {
                c.device.gpu_profile ^= true;
                self.catalog.status = "Detailed GPU profiling records diagnostics and has overhead. Reports remain on this device.".into();
            }
            "record_performance" => {
                c.record_performance ^= true;
                self.catalog.status = if c.record_performance {
                    "Recording starts on Play. Detailed timings add overhead. Return here to save the report."
                } else {
                    "Performance recording off."
                }
                .into();
            }
            _ => return Err("Unknown setting".into()),
        }
        self.save()
    }

    /// Starts the install, or asks the Activity to start the game process.
    fn play(&mut self) -> Result<Value, String> {
        if self.worker.is_some() || self.login.pending() {
            return Ok(json!({"launch": false}));
        }
        if horizon_client::official_install::ready(&self.home) {
            self.save()?;
            self.error = None;
            let _ = std::fs::remove_file(self.home.join("android-error.txt"));
            std::fs::write(self.home.join(LAUNCH_REQUEST), br#"{"play":true}"#)
                .map_err(|e| e.to_string())?;
            crate::diagnostics::heartbeat(
                "play_requested",
                || json!({"renderer": self.catalog.config.renderer.label(), "launcher": "compose"}),
            );
            return Ok(json!({"launch": true}));
        }
        if !self.home.join("engine-data.zip").is_file() {
            return Err("This APK is missing its engine data. Install a complete build.".into());
        }
        self.error = None;
        *self.install.lock().unwrap_or_else(|e| e.into_inner()) = Default::default();
        let home = self.home.clone();
        let install = self.install.clone();
        let (sender, receiver) = mpsc::channel();
        self.worker = Some(receiver);
        std::thread::spawn(move || {
            let result = horizon_client::official_install::install_with_progress(&home, |update| {
                let mut current = install.lock().unwrap_or_else(|e| e.into_inner());
                let done = if current.stage == update.stage {
                    current.done.max(update.done)
                } else {
                    update.done
                };
                *current = horizon_client::official_install::InstallProgress { done, ..update };
            });
            let _ = sender.send(result.map_err(|e| e.to_string()));
        });
        Ok(json!({"launch": false}))
    }
}

const ASPECTS: [crate::display::Aspect; 4] = [
    crate::display::Aspect::Native,
    crate::display::Aspect::Wide,
    crate::display::Aspect::Classic,
    crate::display::Aspect::Square,
];

fn string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> String {
    if value.is_null() {
        return String::new();
    }
    env.get_string(value).map(String::from).unwrap_or_else(|_| {
        let _ = env.exception_clear();
        String::new()
    })
}
fn reply(env: &mut JNIEnv<'_>, value: &Value) -> jstring {
    env.new_string(value.to_string())
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `NativeBridge.start(Activity, String home)`: once per Activity instance.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_horizon_android_NativeBridge_start(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    activity: JObject<'_>,
    home: JString<'_>,
) {
    let home = PathBuf::from(string(&mut env, &home));
    let (Ok(vm), Ok(global)) = (env.get_java_vm(), env.new_global_ref(&activity)) else {
        let _ = env.exception_clear();
        return;
    };
    crate::platform::set_bridge_context(vm, global);
    if CONTROLLER.with(|c| c.borrow().is_some()) {
        return;
    }
    crate::init_logging();
    crate::backend::initialize(home.clone());
    if let Err(error) = crate::diagnostics::initialize(&home, json!({"launcher": "compose"})) {
        log::error!("Cannot initialize local diagnostics: {error}");
    }
    if let Ok(vm) = env.get_java_vm() {
        crate::secure_store::initialize(vm);
    }
    if let Some(cache) = crate::platform::cache_dir() {
        horizon_client::android_runtime::configure_cache_dir(cache.clone());
        horizon_client::official_install::configure_download_cache_dir(cache);
    }
    if let Err(error) =
        std::fs::create_dir_all(&home).and_then(|()| std::env::set_current_dir(&home))
    {
        log::error!("Cannot open {}: {error}", home.display());
    }
    horizon_client::crash::install(home.clone(), "launcher");
    let controller = Controller::new(home);
    CONTROLLER.with(|c| *c.borrow_mut() = Some(controller));
}

/// `NativeBridge.snapshot()`: the whole launcher state as JSON.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_horizon_android_NativeBridge_snapshot(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    let value = CONTROLLER
        .with(|c| c.borrow_mut().as_mut().map(Controller::snapshot))
        .unwrap_or(Value::Null);
    reply(&mut env, &value)
}

/// `NativeBridge.call(String method, String arg)`: `{"ok":…}` or `{"error":…}`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_horizon_android_NativeBridge_call(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    method: JString<'_>,
    arg: JString<'_>,
) -> jstring {
    let method = string(&mut env, &method);
    let arg = string(&mut env, &arg);
    let result = CONTROLLER.with(|c| match c.borrow_mut().as_mut() {
        Some(controller) => controller.call(&method, &arg),
        None => Err("Launcher is not initialized".into()),
    });
    let value = match result {
        Ok(value) => json!({"ok": value}),
        Err(error) => {
            CONTROLLER.with(|c| {
                if let Some(controller) = c.borrow_mut().as_mut() {
                    controller.catalog.status = error.clone();
                }
            });
            json!({"error": error})
        }
    };
    reply(&mut env, &value)
}

/// `NativeBridge.importDocument(Activity, String kind, String uri)`: a picker result.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_horizon_android_NativeBridge_importDocument(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    activity: JObject<'_>,
    kind: JString<'_>,
    uri: JString<'_>,
) {
    let kind = string(&mut env, &kind);
    let uri = (!uri.is_null()).then(|| string(&mut env, &uri));
    let Some(kind) = parse_kind(&kind) else {
        return;
    };
    let Some(home) = CONTROLLER.with(|c| {
        c.borrow()
            .as_ref()
            .map(|c| c.catalog.config.game_home(&c.home))
    }) else {
        return;
    };
    crate::documents::import_external(&mut env, &activity, kind, home, uri);
}
