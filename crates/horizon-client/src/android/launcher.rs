//! A native Rust installer and version screen, sharing the game's Vulkan renderer.
use crate::catalog::{Catalog, Page};
use crate::game;
#[path = "chooser.rs"]
mod chooser;
#[path = "launcher_gesture.rs"]
mod gesture;
#[path = "login.rs"]
mod login;
#[path = "launcher_pages.rs"]
mod pages;
#[path = "wardrobe.rs"]
mod wardrobe;
use horizon_client::{
    Result,
    gfx::Renderer,
    hud::{HudAtlas, HudBatch},
    ui_theme::{Face, Ui},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowId},
};

struct PreparedAccount {
    key: String,
    file: PathBuf,
    appearance: crate::skins::cloud::Cache,
}
pub(super) struct Launcher {
    home: PathBuf,
    controller_edges: horizon_client::gamepad::Edges,
    controller_repeat: Option<(horizon_client::gamepad::Button, Instant, bool)>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui: Option<game::UiResources>,
    pub(super) audio: Option<horizon_client::audio::Audio>,
    game: Option<Box<dyn ApplicationHandler>>,
    cursor: (f64, f64),
    mouse_down: bool,
    feedback: Option<(pages::Action, Instant)>,
    install: Arc<Mutex<horizon_client::official_install::InstallProgress>>,
    install_paused: Arc<std::sync::atomic::AtomicBool>,
    install_cancelled: Arc<std::sync::atomic::AtomicBool>,
    install_transfer: crate::catalog::TransferTracker,
    pub(super) anim: pages::AnimState,
    pub(super) control_editor: Option<horizon_client::touch::TouchControls>,
    worker: Option<mpsc::Receiver<std::result::Result<(), String>>>,
    error: Option<String>,
    catalog: Catalog,
    targets: Vec<([f32; 4], pages::Action)>,
    list_targets: Vec<([f32; 4], pages::Action)>,
    list_view: Option<[f32; 3]>,
    pending_focus: Option<pages::Action>,
    drag: Option<gesture::Drag>,
    chooser: Option<chooser::Chooser>,
    exit_confirmation: bool,
    login: login::Login,
    wardrobe: wardrobe::Wardrobe,
    session_worker: Option<mpsc::Receiver<std::result::Result<PreparedAccount, String>>>,
    session_ready: bool,
    prepared_session: Option<PathBuf>,
    pub(super) launching: bool,
    pub(super) launch_pending_tick: bool,
    pub(super) diagnostics_modal_open: bool,
    focus: Option<usize>,
    diagnostic_export: Option<mpsc::Receiver<std::result::Result<String, String>>>,
}

impl Launcher {
    pub(super) fn refresh_wardrobe_account(&mut self) {
        if self.catalog.page == Page::Skins {
            self.catalog.scroll = 0.0;
        }
        // The header shows this appearance even when another page is open.
        self.wardrobe.open(
            &self.catalog.config.name,
            &self.catalog.config.microsoft_client_id,
        );
    }

    fn edit_text(&mut self, output: horizon_client::osk::Output) {
        if let Some(tc) = &mut self.control_editor
            && horizon_client::touch_ui::apply_editor_or_preset_output(
                tc.editor.as_mut(),
                None,
                &output,
            )
        {
            let game_home = self.catalog.config.game_home(&self.home);
            let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
            if horizon_client::touch_ui::output_dismisses(&output) {
                crate::platform::hide();
            }
            return;
        }
        if self.login.edit(self.catalog.editing, &output) {
            if matches!(
                output,
                horizon_client::osk::Output::Enter | horizon_client::osk::Output::Close
            ) {
                self.catalog.editing = None;
                crate::platform::hide();
            }
        } else {
            self.catalog.edit(output, &self.home);
        }
    }

    pub(super) fn new(home: PathBuf) -> Self {
        // Upgrade existing installations and pre-verify structure transforms in the background
        // so pressing Play launches immediately without main-thread I/O stalls.
        let cleanup_home = home.clone();
        std::thread::spawn(move || {
            if let Err(error) =
                horizon_client::official_install::cleanup_completed_downloads(&cleanup_home)
            {
                log::warn!("Could not clean completed Minecraft downloads: {error}");
            }
            if horizon_client::official_install::ready(&cleanup_home) {
                let _ = horizon_client::android_resources::repair_structure_transforms(
                    &cleanup_home.join("engine-data.zip"),
                    &cleanup_home
                        .join(".local/horizon-client")
                        .join(horizon_core::VANILLA_VERSION),
                );
            }
        });
        let mut catalog = Catalog::new(&home);
        if crate::diagnostics::previous_unclean(&home) {
            catalog.status = crate::diagnostics::summary(&home);
        }
        horizon_client::android_runtime::graphics_startup::configure(home.clone());
        if horizon_client::android_runtime::graphics_startup::recover(&home) {
            catalog.config.renderer = crate::catalog::RendererMode::VulkanCompatibility;
            if let Err(error) = catalog.config.save(&home) {
                log::warn!("Could not save graphics recovery choice: {error}");
            }
            catalog.status = "Previous Vulkan startup was interrupted. Using Vulkan compatibility; you can change Rendering mode in Settings.".into();
        }
        crate::configure_input_mode(catalog.config.input_host);
        crate::device_runtime::configure(catalog.config.device);
        horizon_client::ui_theme::set_ui_scale_percent(catalog.config.ui_scale_percent);
        horizon_client::ui_theme::motion::set_launcher_animations_disabled(
            catalog.config.disable_launcher_animations,
        );
        horizon_client::ui_theme::motion::set_game_animations_disabled(
            catalog.config.disable_game_animations,
        );
        // First run: ensure independent MCPE and GoyControl storage directories exist,
        // and initialize MCPE Controls as the default active control system on fresh installs.
        let game_home = catalog.config.game_home(&home);
        horizon_client::touch::Settings::ensure_system_storage(
            &game_home,
            horizon_client::touch::ControlStyle::Mcpe,
        );
        horizon_client::touch::Settings::ensure_system_storage(
            &game_home,
            horizon_client::touch::ControlStyle::GoyControl,
        );
        let touch_file = game_home.join("horizon-touch.json");
        if !touch_file.exists() {
            let touch = horizon_client::touch::Settings::default_mcpe();
            if let Some(dir) = touch_file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(error) = touch.save_to(&touch_file) {
                log::warn!("Cannot write the factory touch layout: {error}");
            }
        }
        // Pre-warm Microsoft session in background at startup so pressing Play doesn't stall.
        let prewarm_session_worker = if crate::account::Accounts::load().selected().is_some() {
            let id = catalog.config.microsoft_client_id.clone();
            let game_home = catalog.config.game_home(&home);
            let wardrobe_home = home.join("wardrobe");
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let res = (|| {
                    let (account, secrets) = login::Login::session(&id)?
                        .ok_or("The selected account changed. Choose Play again.")?;
                    let appearance = crate::skins::cloud::fetch(&account, &secrets, &wardrobe_home)
                        .unwrap_or_default();
                    let file = crate::account::write_session_file(
                        &game_home.join("sessions"),
                        &account,
                        &secrets,
                    )?;
                    Ok(PreparedAccount {
                        key: account.key(),
                        file,
                        appearance,
                    })
                })();
                if let Err(unsent) = tx.send(res)
                    && let Ok(prepared) = unsent.0
                {
                    let _ = std::fs::remove_file(prepared.file);
                }
            });
            Some(rx)
        } else {
            None
        };
        // Load the selected wardrobe and its last head before the first draw:
        // the account pill must have the real face instantly, not after a page visit.
        let mut wardrobe = wardrobe::Wardrobe::new(&home);
        wardrobe.open(&catalog.config.name, &catalog.config.microsoft_client_id);
        Self {
            wardrobe,
            home,
            controller_edges: Default::default(),
            controller_repeat: None,
            window: None,
            renderer: None,
            gui: None,
            audio: None,
            game: None,
            cursor: (0.0, 0.0),
            mouse_down: false,
            feedback: None,
            install: Arc::default(),
            install_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            install_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            install_transfer: Default::default(),
            anim: Default::default(),
            control_editor: None,
            worker: None,
            error: None,
            catalog,
            targets: vec![],
            list_targets: vec![],
            list_view: None,
            pending_focus: None,
            drag: None,
            chooser: None,
            exit_confirmation: false,
            login: login::Login::new(),
            session_worker: prewarm_session_worker,
            session_ready: false,
            prepared_session: None,
            launching: false,
            launch_pending_tick: false,
            diagnostics_modal_open: false,
            focus: None,
            diagnostic_export: None,
        }
    }

    fn restore_launcher(&mut self, events: &ActiveEventLoop) {
        if !horizon_client::android_runtime::take_launcher_return() {
            return;
        }
        if let Some(mut game) = self.game.take() {
            game.exiting(events);
            drop(game);
        }
        crate::game::android_reset_bundled_mods();
        crate::document_store::set_game_active(false);
        crate::performance::stop();
        let _ = crate::platform::capture_mouse(false);
        crate::platform::hide();
        horizon_client::auth::clear_current();
        self.session_ready = false;
        self.prepared_session = None;
        self.launching = false;
        self.launch_pending_tick = false;
        horizon_client::discord_presence::stop();
        horizon_client::ui_theme::reset_atlas();
        self.cursor = (0.0, 0.0);
        self.drag = None;
        self.chooser = None;
        self.exit_confirmation = false;
        self.focus = None;
        self.pending_focus = None;
        if let Err(error) = std::env::set_current_dir(&self.home) {
            self.fail(error);
            return;
        }
        horizon_client::android_runtime::clear_gamepads();
        self.controller_edges = Default::default();
        self.controller_repeat = None;
        self.catalog.config.input_host = horizon_client::android_runtime::input_mode();
        self.catalog.refresh(&self.home);
        self.login.refresh_status();
        crate::diagnostics::heartbeat("returned_to_launcher", || serde_json::json!({}));
        self.catalog.page = Page::Play;
        if let Ok(message) = std::fs::read_to_string(self.home.join("android-error.txt")) {
            self.error = Some(message);
        }
        if let Err(error) = self.open(events) {
            self.fail(error);
            events.exit();
        }
    }
    pub(super) fn configure_screen(&self) {
        crate::native_ime::configure_screen(
            self.catalog.config.screen.immersive,
            self.catalog.config.screen.avoid_cutout,
        );
        winit::platform::android::set_native_content_aspect(
            self.catalog.config.screen.aspect.ratio(),
        );
    }
    fn fail(&mut self, error: impl std::fmt::Display) {
        log::error!("{error}");
        self.launching = false;
        self.launch_pending_tick = false;
        crate::diagnostics::record_error(&error.to_string());
        self.error = Some(error.to_string());
        let _ = std::fs::write(self.home.join("android-error.txt"), error.to_string());
    }

    fn open(&mut self, events: &ActiveEventLoop) -> Result<()> {
        crate::platform::set_orientation(self.catalog.config.orientation);
        self.configure_screen();
        horizon_client::ui_theme::set_android_dpi(crate::platform::display_density());
        horizon_client::ui_theme::set_ui_scale_percent(self.catalog.config.ui_scale_percent);
        crate::display::set_scale(self.catalog.config.render_scale);
        horizon_client::android_runtime::configure_gpu(false);
        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
            if let Some(size) = crate::display::prepare(window)? {
                renderer.resize(size[0], size[1]);
            }
            renderer.recreate_surface()?;
            let size = window.inner_size();
            renderer.resize(size.width, size.height);
            return Ok(());
        }
        let window = Arc::new(
            events.create_window(Window::default_attributes().with_title("Horizon Android"))?,
        );
        crate::display::prepare(&window)?;
        let vanilla_dir = self
            .home
            .join(".local/horizon-client")
            .join(horizon_core::VANILLA_VERSION);
        let atlas_root = if vanilla_dir.join("assets/minecraft/textures/gui").is_dir() {
            vanilla_dir
        } else {
            self.home.join("installer")
        };
        // Discover the adapter limit before packing any shared sprites.
        let mut renderer = game::loading_renderer(
            window.clone(),
            false,
            None,
            None,
            None,
            if self.catalog.config.renderer == crate::catalog::RendererMode::OpenGl {
                horizon_client::graphics::GraphicsApi::OpenGl
            } else {
                horizon_client::graphics::GraphicsApi::Auto
            },
        )?;
        let gui = game::UiResources {
            atlas: HudAtlas::build(&atlas_root)?,
            panorama: Arc::new(horizon_client::gfx::panorama::Faces {
                size: 1,
                pixels: [0u8; 4].repeat(6),
            }),
            sounds: Some(horizon_client::sound_events::SoundRegistry::load(
                &atlas_root,
            )),
        };
        renderer.set_loading_ui(&gui.atlas, gui.panorama.as_ref())?;
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.gui = Some(gui);
        Ok(())
    }

    fn request_back(&mut self) {
        if crate::platform::active() && self.control_editor.is_some() {
            crate::platform::hide();
            return;
        }
        crate::platform::hide();
        if let Some(tc) = &mut self.control_editor {
            let game_home = self.catalog.config.game_home(&self.home);
            if tc.request_editor_back() {
                let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
                self.control_editor = None;
            }
            return;
        }
        if !self.anim.intro_done {
            self.anim.intro_done = true;
            return;
        }
        if self.diagnostics_modal_open {
            self.diagnostics_modal_open = false;
        } else if self.exit_confirmation {
            self.exit_confirmation = false;
        } else if self.catalog.fm_modal.take().is_some() {
            self.catalog.editing = None;
        } else if self.catalog.project_modal.take().is_some() {
            self.catalog.scroll = 0.0;
        } else if self.login.visible {
            self.login.hide();
        } else if self.chooser.take().is_some() {
            // Back dismisses the current dialog, including the exit question.
        } else if self.catalog.page == Page::Installed
            && self.catalog.installed_sub_tab == crate::catalog::InstalledSubTab::FileManager
            && self.catalog.fm_back(&self.home)
        {
            // Back in File Manager pops the directory history stack.
        } else if self.catalog.page == Page::Settings
            && self.catalog.settings_category != crate::catalog::SettingsCategory::Overview
        {
            self.catalog.settings_category =
                if self.catalog.settings_category == crate::catalog::SettingsCategory::Theme {
                    crate::catalog::SettingsCategory::Launcher
                } else {
                    crate::catalog::SettingsCategory::Overview
                };
            self.catalog.scroll = 0.0;
            self.anim.trigger_page_transition();
        } else if self.catalog.editing.take().is_none() {
            self.exit_confirmation = true;
        }
        self.focus = None;
        self.pending_focus = self.exit_confirmation.then_some(pages::Action::Dismiss);
        self.targets.clear();
        self.list_targets.clear();
        self.drag = None;
        if let Err(error) = self.catalog.config.save(&self.home) {
            self.fail(error);
        }
    }

    fn action(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.exit_confirmation
            || self.chooser.is_some()
            || self.worker.is_some()
            || self.session_worker.is_some()
            || self.launch_pending_tick
        {
            return Ok(());
        }
        if self.login.pending() {
            return Ok(());
        }
        crate::diagnostics::heartbeat(
            "play_requested",
            || serde_json::json!({"renderer":self.catalog.config.renderer.label(),"resolution_percent":self.catalog.config.render_scale.percent()}),
        );
        horizon_client::android_runtime::configure_worker_limit(self.catalog.config.worker_limit);
        horizon_client::android_runtime::configure_cache_budget(
            self.catalog.config.cache_budget_mib,
        );
        crate::configure_input_mode(self.catalog.config.input_host);
        crate::device_runtime::configure(self.catalog.config.device);
        if horizon_client::official_install::ready(&self.home) {
            if !self.launching {
                self.launching = true;
                self.error = None;
                self.catalog.status = "Launching Minecraft 26.3...".into();
                let game_home = self.catalog.config.game_home(&self.home);
                if !self.session_ready && crate::account::Accounts::load().selected().is_some() {
                    let id = self.catalog.config.microsoft_client_id.clone();
                    let wardrobe_home = self.home.join("wardrobe");
                    let (sender, receiver) = mpsc::channel();
                    self.session_worker = Some(receiver);
                    self.catalog.status = "Preparing account session and appearance...".into();
                    std::thread::spawn(move || {
                        let result = (|| {
                            let (account, secrets) = login::Login::session(&id)?
                                .ok_or("The selected account changed. Choose Play again.")?;
                            let appearance =
                                crate::skins::cloud::fetch(&account, &secrets, &wardrobe_home)
                                    .unwrap_or_default();
                            let file = crate::account::write_session_file(
                                &game_home.join("sessions"),
                                &account,
                                &secrets,
                            )?;
                            Ok(PreparedAccount {
                                key: account.key(),
                                file,
                                appearance,
                            })
                        })();
                        if let Err(unsent) = sender.send(result)
                            && let Ok(prepared) = unsent.0
                        {
                            let _ = std::fs::remove_file(prepared.file);
                        }
                    });
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return Ok(());
                }
                self.launch_pending_tick = true;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return Ok(());
            }
            horizon_client::android_resources::repair_structure_transforms(
                &self.home.join("engine-data.zip"),
                &self
                    .home
                    .join(".local/horizon-client")
                    .join(horizon_core::VANILLA_VERSION),
            )?;
            crate::platform::hide();
            self.catalog.config.save(&self.home)?;
            self.catalog.config.apply_experimental_to_game(&self.home);
            // Migrate the earlier launcher preset once; subsequent edits belong to Key Binds.
            let keypad_path = self
                .catalog
                .config
                .game_home(&self.home)
                .join("horizon-keypad.json");
            if self.catalog.config.keypad_controls && !keypad_path.exists() {
                if let Some(dir) = keypad_path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                horizon_client::keypad::Settings {
                    enabled: true,
                    ..Default::default()
                }
                .save(&keypad_path)?;
            }
            self.catalog.config.keypad_controls = false;
            let game_home = self.catalog.config.game_home(&self.home);
            if !self.session_ready && crate::account::Accounts::load().selected().is_some() {
                let id = self.catalog.config.microsoft_client_id.clone();
                let wardrobe_home = self.home.join("wardrobe");
                let (sender, receiver) = mpsc::channel();
                self.session_worker = Some(receiver);
                self.catalog.status = "Preparing account session and appearance...".into();
                std::thread::spawn(move || {
                    let result = (|| {
                        let (account, secrets) = login::Login::session(&id)?
                            .ok_or("The selected account changed. Choose Play again.")?;
                        let appearance =
                            crate::skins::cloud::fetch(&account, &secrets, &wardrobe_home)
                                .unwrap_or_default();
                        let file = crate::account::write_session_file(
                            &game_home.join("sessions"),
                            &account,
                            &secrets,
                        )?;
                        Ok(PreparedAccount {
                            key: account.key(),
                            file,
                            appearance,
                        })
                    })();
                    if let Err(unsent) = sender.send(result)
                        && let Ok(prepared) = unsent.0
                    {
                        let _ = std::fs::remove_file(prepared.file);
                    }
                });
                return Ok(());
            }
            self.session_ready = false;
            std::fs::create_dir_all(&game_home)?;
            if crate::account::Accounts::load().selected().is_none() {
                self.wardrobe
                    .prepare_game(&self.catalog.config.name, &game_home)?;
            }
            let extra_arguments = if crate::PUBLIC_BUILD {
                Vec::new()
            } else {
                match std::fs::read(self.home.join("android-launch.json")) {
                    Ok(bytes) => serde_json::from_slice::<Vec<String>>(&bytes)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                    Err(e) => return Err(e.into()),
                }
            };

            self.audio = None;
            self.error = None;
            let _ = std::fs::remove_file(self.home.join("android-error.txt"));
            if self.catalog.config.record_performance || self.catalog.config.device.gpu_profile {
                crate::performance::start(
                    &self.home,
                    serde_json::json!({
                        "renderer": self.catalog.config.renderer.label(),
                        "resolution_percent": self.catalog.config.render_scale.percent(),
                        "source_commit": option_env!("HORIZON_ANDROID_BUILD_COMMIT"),
                        "source_dirty": option_env!("HORIZON_ANDROID_BUILD_DIRTY"),
                        "performance_recording": self.catalog.config.record_performance,
                        "gpu_profiling": self.catalog.config.device.gpu_profile,
                    }),
                )?;
            }
            // The A press that launches the game must not accept its first menu too.
            horizon_client::android_runtime::clear_gamepads();
            self.controller_edges = Default::default();
            self.controller_repeat = None;
            let request = crate::game_launch::LaunchRequest {
                resources: self
                    .home
                    .join(".local/horizon-client")
                    .join(horizon_core::VANILLA_VERSION),
                game_home,
                session_file: self.prepared_session.take(),
                offline_name: crate::catalog::truncate_player_name(&self.catalog.config.name),
                server: self.catalog.config.server.clone(),
                control_socket: (!crate::PUBLIC_BUILD).then(|| self.home.join("control.json")),
                extra_arguments,
                open_gl: self.catalog.config.renderer == crate::catalog::RendererMode::OpenGl,
                gpu_batching: self.catalog.config.renderer == crate::catalog::RendererMode::Vulkan,
            };
            crate::document_store::set_game_active(true);
            let game = match crate::game_launch::start(request) {
                Ok(game) => game,
                Err(error) => {
                    crate::document_store::set_game_active(false);
                    crate::performance::stop();
                    return Err(error);
                }
            };
            // Android has one native window; release its surface before handing it over.
            self.renderer = None;
            self.window = None;
            self.gui = None;
            horizon_client::ui_theme::reset_atlas();
            self.game = Some(game);
            self.game.as_mut().unwrap().resumed(events);
        } else {
            if !self.home.join("engine-data.zip").is_file() {
                return Err(
                    "This APK is missing its engine data. Install a complete build.".into(),
                );
            }
            self.error = None;
            *self
                .install
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Default::default();
            self.install_paused
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.install_cancelled
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.install_transfer.reset();
            let home = self.home.clone();
            let install = self.install.clone();
            let paused = self.install_paused.clone();
            let cancelled = self.install_cancelled.clone();
            let (sender, receiver) = mpsc::channel();
            self.worker = Some(receiver);
            std::thread::spawn(move || {
                let result =
                    horizon_client::official_install::install_with_progress(&home, |update| {
                        while paused.load(std::sync::atomic::Ordering::Relaxed)
                            && !cancelled.load(std::sync::atomic::Ordering::Relaxed)
                        {
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        let mut current = install
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let done = if current.stage == update.stage {
                            current.done.max(update.done)
                        } else {
                            update.done
                        };
                        *current =
                            horizon_client::official_install::InstallProgress { done, ..update };
                    });
                let _ = sender.send(result.map_err(|e| e.to_string()));
            });
        }
        Ok(())
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        self.configure_screen();
        if let Some(game) = &mut self.game {
            return game.resumed(events);
        }
        if let Err(error) = self.open(events) {
            self.fail(error);
            events.exit();
        }
    }
    fn suspended(&mut self, events: &ActiveEventLoop) {
        crate::diagnostics::heartbeat("suspended", || serde_json::json!({}));
        if let Some(game) = &mut self.game {
            return game.suspended(events);
        }
        self.drag = None;
        if let Some(renderer) = &mut self.renderer {
            renderer.suspend_surface();
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if crate::documents::handle_event(&event) {
            return;
        }
        if matches!(event, WindowEvent::Resized(_)) {
            let _ = crate::native_ime::viewport_bottom();
        }
        if matches!(event, WindowEvent::Focused(true)) {
            self.configure_screen();
        }
        if let Some(game) = &mut self.game {
            game.window_event(events, id, event);
            self.restore_launcher(events);
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                self.request_back();
                Ok(())
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
                Ok(())
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.focus = None;
                self.pending_focus = None;
                self.cursor = (position.x, position.y);
                Ok(())
            }
            WindowEvent::Touch(touch) => self.touch_event(events, touch),
            WindowEvent::Focused(false) => {
                crate::diagnostics::heartbeat("launcher_background", || serde_json::json!({}));
                horizon_client::android_runtime::clear_gamepads();
                self.controller_repeat = None;
                self.mouse_down = false;
                self.drag = None;
                Ok(())
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if state == ElementState::Pressed {
                    self.mouse_down = true;
                    Ok(())
                } else if std::mem::take(&mut self.mouse_down) {
                    self.activate(events)
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.exit_confirmation {
                    return;
                }
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 42.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / self.ui_density(),
                };
                if let Some(chooser) = &mut self.chooser {
                    chooser.scroll = (chooser.scroll - y).max(0.0);
                } else {
                    self.catalog.scroll = (self.catalog.scroll - y).max(0.0);
                }
                Ok(())
            }
            WindowEvent::RedrawRequested => self.draw_pages(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.physical_key
                    == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
                    || event.logical_key
                        == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
                {
                    self.type_key(event);
                    Ok(())
                } else if self.navigate(&event) {
                    Ok(())
                } else if self.catalog.editing.is_none()
                    && !event.repeat
                    && matches!(
                        event.logical_key,
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter)
                    )
                {
                    self.activate(events)
                } else {
                    self.type_key(event);
                    Ok(())
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.edit_text(horizon_client::osk::Output::Text(text));
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(error);
        }
    }
    fn device_event(&mut self, events: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        if matches!(event, DeviceEvent::Removed) {
            self.mouse_down = false;
            self.drag = None;
        }
        if let Some(game) = &mut self.game {
            game.device_event(events, id, event);
        } else {
            horizon_client::android_runtime::gamepad_event(
                winit::platform::android::native_device_id(id),
                &event,
            );
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            game.about_to_wait(events);
            self.restore_launcher(events);
            return;
        }
        crate::diagnostics::heartbeat("launcher", || serde_json::json!({}));
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
        crate::platform::poll_gamepad_connections();
        if let Err(error) = self.controller_frame(events) {
            self.fail(error);
        }
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer)
            && let Err(error) = crate::display::apply(window, renderer)
        {
            crate::display::set_scale(crate::display::RenderScale::default());
            self.fail(error);
        }
        if let Some(result) = self.worker.as_ref().and_then(|w| w.try_recv().ok()) {
            self.worker = None;
            if let Err(error) = result {
                let message = error.to_string();
                if message.contains("Dns Failed") || message.contains("lookup address") {
                    self.fail("Minecraft download interrupted: DNS could not resolve the download host. Check your connection, VPN or Private DNS, then retry Download. Completed files are kept.");
                } else {
                    self.fail(error);
                }
            }
        }
        if let Some(tc) = &mut self.control_editor
            && let Some(ed) = &mut tc.editor
            && std::mem::take(&mut ed.keyboard_request)
        {
            let text = if ed.editing_save_desc {
                ed.save_desc_input.clone()
            } else {
                ed.save_name_input.clone()
            };
            crate::platform::show(&text);
        }
        for out in crate::platform::poll() {
            if matches!(out, horizon_client::osk::Output::Enter) {
                crate::platform::hide();
            }
            self.edit_text(out);
        }
        if let Some(audio) = &mut self.audio {
            audio.tick(false, &Default::default());
        }
        self.catalog.poll(&self.home);
        self.wardrobe.poll();
        if let Some(notice) = crate::documents::poll() {
            self.wardrobe.reload();
            self.catalog.status = notice.text;
            if notice.changed {
                self.catalog.refresh(&self.home);
                self.catalog.show_installed(notice.kind);
            }
        }
        for result in crate::gallery::poll() {
            self.catalog.status = result.unwrap_or_else(|e| format!("File export failed: {e}"));
        }
        if self.login.poll() {
            self.session_worker = None;
            self.prepared_session = None;
            self.session_ready = false;
            self.refresh_wardrobe_account();
        }
        if let Some(result) = self.session_worker.as_ref().and_then(|w| w.try_recv().ok()) {
            let launch_pending = self.launching
                || self.catalog.status == "Preparing account session and appearance...";
            self.session_worker = None;
            match result {
                Ok(prepared) => {
                    // Only the current selection may publish the game handoff.
                    if crate::account::Accounts::load()
                        .selected()
                        .is_none_or(|a| a.key() != prepared.key)
                    {
                        self.launching = false;
                        self.launch_pending_tick = false;
                        let _ = std::fs::remove_file(prepared.file);
                        self.catalog.status =
                            "The selected account changed. Choose Play again.".into();
                    } else {
                        let root = self.home.join("wardrobe");
                        let cloud = prepared.appearance.skin.as_deref().map(|p| {
                            (
                                p,
                                prepared.appearance.slim,
                                prepared.appearance.cape.as_deref(),
                            )
                        });
                        let appearance =
                            crate::skins::library::Library::load(&root).and_then(|library| {
                                library.prepare_game(
                                    &root,
                                    &prepared.key,
                                    &self.catalog.config.game_home(&self.home),
                                    cloud,
                                )
                            });
                        match appearance {
                            Ok(()) => {
                                self.prepared_session = Some(prepared.file);
                                self.session_ready = true;
                                if launch_pending {
                                    self.catalog.status.clear();
                                    if let Err(e) = self.action(events) {
                                        self.fail(e);
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = std::fs::remove_file(prepared.file);
                                self.fail(error);
                            }
                        }
                    }
                }
                Err(e) => {
                    if launch_pending {
                        self.fail(e);
                    } else {
                        log::warn!("Background session pre-warm notice: {e}");
                    }
                }
            }
        }

        if self.launch_pending_tick {
            self.launch_pending_tick = false;
            if let Err(error) = self.action(events) {
                self.fail(error);
            }
            if self.game.is_some() {
                return;
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        let frame_ms = if self.worker.is_some()
            || self.launching
            || self.session_worker.is_some()
            || self.catalog.busy()
            || self.drag.is_some()
            || self.login.pending()
            || self.control_editor.is_some()
            || self
                .catalog
                .project_modal
                .as_ref()
                .is_some_and(|m| m.loading)
            || self.anim.is_active()
            || (self.catalog.page == Page::Skins && self.wardrobe.is_animating())
            || self
                .feedback
                .is_some_and(|(_, t)| t.elapsed() < Duration::from_millis(180))
        {
            16
        } else {
            33
        };
        events.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(frame_ms),
        ));
    }
    fn exiting(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            game.exiting(events);
        }
    }
}

impl Launcher {
    fn controller_frame(&mut self, events: &ActiveEventLoop) -> Result<()> {
        use horizon_client::gamepad::Button;
        use winit::keyboard::{Key, NamedKey};
        let mut state = horizon_client::android_runtime::gamepad_state();
        for (b, held) in [
            (Button::DPadLeft, state.left[0] < -0.55),
            (Button::DPadRight, state.left[0] > 0.55),
            (Button::DPadUp, state.left[1] > 0.55),
            (Button::DPadDown, state.left[1] < -0.55),
        ] {
            state.set(b, held || state.held(b));
        }
        let now = Instant::now();
        let mut pressed: Vec<_> = self
            .controller_edges
            .update(&state, 0.5)
            .into_iter()
            .filter_map(|(b, down)| down.then_some(b))
            .collect();
        if let Some((button, last, repeating)) = self.controller_repeat {
            if !state.held(button) {
                self.controller_repeat = None;
            } else if now.duration_since(last).as_secs_f32() >= if repeating { 0.09 } else { 0.35 }
            {
                pressed.push(button);
                self.controller_repeat = Some((button, now, true));
            }
        }
        for b in pressed {
            let direction = match b {
                Button::DPadUp => Some(NamedKey::ArrowUp),
                Button::DPadDown => Some(NamedKey::ArrowDown),
                Button::DPadLeft => Some(NamedKey::ArrowLeft),
                Button::DPadRight => Some(NamedKey::ArrowRight),
                _ => None,
            };
            if let Some(key) = direction {
                self.navigate_key(&Key::Named(key));
                if self.controller_repeat.is_none_or(|(old, _, _)| old != b) {
                    self.controller_repeat = Some((b, now, false));
                }
            } else if b == Button::South {
                self.activate(events)?;
            } else if b == Button::East {
                crate::platform::hide();
                self.catalog.editing = None;
                if self.chooser.take().is_none() {
                    if self.login.visible {
                        self.login.visible = false;
                    } else {
                        self.catalog.page = Page::Play;
                    }
                }
                self.focus = None;
            }
        }
        Ok(())
    }
}
