//! Android's native entry point. The simulation and renderer are shared with desktop.
#![cfg(target_os = "android")]

// The game and mod API compile into this crate, not horizon-client's library,
// so their `crate::` paths reach the Android platform modules below.
extern crate self as horizon_client;
pub use horizon_client_lib::*;

#[allow(dead_code, clippy::collapsible_match)]
// Shared vanilla match structure and desktop harnesses.
pub mod game;
pub mod mod_api;
#[path = "android/game_launch.rs"]
mod game_launch;
#[path = "android/launcher.rs"]
mod launcher;
// The fork's launcher search, compatibility checks, downloads, dependency
// resolution, Ely.by accounts and Java content porting; GPUI is desktop-only.
// Kept as Android's own copy: upstream's launcher dropped the Ely.by provider.
#[path = "android/dns.rs"]
mod android_dns;
#[path = "android/catalog.rs"]
mod catalog;
#[path = "android/device_preferences.rs"]
mod device_preferences;
#[path = "android/device_runtime.rs"]
mod device_runtime;
#[path = "android/diagnostics.rs"]
mod diagnostics;
#[path = "android/display.rs"]
mod display;
// The desktop launcher's own worlds, screenshots and data packs; the Android side of each is
// `android/launcher_home.rs`.
#[allow(dead_code)]
#[path = "android/launcher_datapacks.rs"]
mod datapacks;
mod desktop_seams {
    /// Android has no trash the app can reach: what the player confirms is deleted.
    pub fn to_trash(path: &std::path::Path) -> Result<(), String> {
        crate::launcher_data::remove(path)
    }
}
#[path = "android/launcher_data.rs"]
mod launcher_data;
#[allow(dead_code)]
#[path = "android/launcher_glass.rs"]
mod launcher_glass;
#[allow(dead_code)]
#[path = "android/launcher_scene.rs"]
mod launcher_scene;
#[allow(dead_code)]
#[path = "android/launcher_servers.rs"]
mod server_list;
#[allow(dead_code)]
#[path = "android/launcher_tokens.rs"]
mod launcher_tokens;
#[allow(dead_code)]
#[path = "android/launcher_view.rs"]
mod launcher_view;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../../launcher/src/screenshots.rs"]
mod screenshots;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../../launcher/src/worlds.rs"]
mod worlds;
#[path = "android/document_bridge.rs"]
mod document_bridge;
#[path = "android/document_provider.rs"]
mod document_provider;
#[path = "android/document_store.rs"]
mod document_store;
#[path = "android/documents.rs"]
mod documents;
#[path = "android/exit_reports.rs"]
mod exit_reports;
#[path = "android/gallery.rs"]
mod gallery;
#[path = "android/native_ime.rs"]
mod native_ime;
#[path = "android/performance.rs"]
mod performance;
#[path = "android/platform.rs"]
mod platform;
#[path = "android/search_queue.rs"]
mod search_queue;
#[path = "android/secure_store.rs"]
mod secure_store;
#[path = "android/shader_catalog.rs"]
mod shader_catalog;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "android/launcher_shared"]
mod launcher_shared {
    pub mod account;
    pub mod browser;
    pub mod proxy;
    pub mod skins;
}
use launcher_shared::{account, browser, proxy};
// The launcher's wardrobe and the client's skin code share the name `skins`.
pub mod skins {
    pub use crate::launcher_shared::skins::*;
    pub use horizon_client_lib::skins::*;
}
// The shared account protocol needs only the launcher's version and data path.
mod backend {
    pub const VERSION: &str = if crate::PUBLIC_BUILD {
        "0.7.7"
    } else {
        "0.7.7-dev"
    };
    static HOME: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    pub fn initialize(home: std::path::PathBuf) {
        let _ = HOME.set(home);
    }
    pub fn game_home() -> std::path::PathBuf {
        data_dir()
    }
    pub fn game_mod_api_level() -> Option<u32> {
        Some(horizon_mods::api_level::LEVEL)
    }
    pub fn data_dir() -> std::path::PathBuf {
        HOME.get().expect("android_main sets account home").clone()
    }
}

use std::cell::RefCell;
pub(crate) fn keyboard_mouse_input() -> bool {
    horizon_client::android_runtime::active_input() == catalog::InputHost::KeyboardMouse
}
pub(crate) fn configure_input_mode(mode: catalog::InputHost) {
    horizon_client::android_runtime::configure_input(mode);
}
pub(crate) fn touch_visible() -> bool {
    horizon_client::android_runtime::active_input() == catalog::InputHost::Touch
}
pub(crate) fn touch_input(touch: bool) {
    horizon_client::android_runtime::note_input(if touch {
        catalog::InputHost::Touch
    } else {
        catalog::InputHost::KeyboardMouse
    });
}
pub(crate) fn save_input_mode() {
    APP.with(|app| {
        if let Some(home) = app.borrow().as_ref().and_then(|a| a.internal_data_path()) {
            let mut config = catalog::Config::load(&home);
            let mode = horizon_client::android_runtime::input_mode();
            if config.input_host != mode {
                config.input_host = mode;
                if let Err(e) = config.save(&home) {
                    log::warn!("Saving input mode: {e}");
                }
            }
        }
    });
}
pub(crate) const PUBLIC_BUILD: bool = match option_env!("HORIZON_ANDROID_PUBLIC") {
    Some(value) => value.len() == 1 && value.as_bytes()[0] == b'1',
    None => false,
};

#[unsafe(no_mangle)]
fn android_on_create(state: &winit::platform::android::activity::OnCreateState<'_>) {
    init_logging();
    horizon_protocol::install_system_srv_lookup(android_dns::query_srv);
    if let Err(error) =
        unsafe { native_ime::initialize(state.vm_as_ptr().cast(), state.activity_as_ptr().cast()) }
    {
        log::error!("Android system keyboard initialization: {error}");
    }
}
use winit::{
    event_loop::EventLoop,
    platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp},
};

thread_local! {
    static APP: RefCell<Option<AndroidApp>> = const { RefCell::new(None) };
}

pub(crate) fn android_event_loop() -> std::result::Result<EventLoop<()>, winit::error::EventLoopError> {
    APP.with(|app| {
        EventLoop::builder()
            .with_android_app(
                app.borrow()
                    .as_ref()
                    .expect("android_main sets APP")
                    .clone(),
            )
            .build()
    })
}

fn init_logging() {
    let trace_gles = !PUBLIC_BUILD && std::env::var_os("HORIZON_GLES_TRACE").is_some();
    let mut logger = android_logger::Config::default()
        .with_max_level(if trace_gles {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        })
        .with_tag("Horizon");
    if trace_gles {
        logger = logger.with_filter(
            android_logger::FilterBuilder::new()
                .parse("info,wgpu_hal::gles=trace,wgpu_core::device=debug")
                .build(),
        );
    }
    struct LocalLogger(android_logger::AndroidLogger);
    impl log::Log for LocalLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            log::Log::enabled(&self.0, metadata)
        }
        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            log::Log::log(&self.0, record);
            let line = record.args().to_string();
            launcher_data::remember_log(record.level(), record.target(), &line);
            diagnostics::log_line(record.level(), record.target(), &line);
        }
        fn flush(&self) {
            diagnostics::request_flush();
        }
    }
    static LOGGER: std::sync::OnceLock<LocalLogger> = std::sync::OnceLock::new();
    let logger = LOGGER.get_or_init(|| LocalLogger(android_logger::AndroidLogger::new(logger)));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(if trace_gles {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        });
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    use winit::platform::android::activity::input::Axis;
    for axis in [
        Axis::Vscroll,
        Axis::Hscroll,
        Axis::RelativeX,
        Axis::RelativeY,
    ] {
        app.enable_motion_axis(axis);
    }
    init_logging();
    let Some(home) = app.internal_data_path() else {
        log::error!("Android supplied no application data directory");
        return;
    };
    backend::initialize(home.clone());
    if let Err(error) = diagnostics::initialize(&home, serde_json::json!({})) {
        log::error!("Cannot initialize local diagnostics: {error}");
    }
    if let Ok(vm) = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) } {
        secure_store::initialize(vm);
    }
    let engine_data = app.asset_manager().open(c"engine-data.zip");
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    let Some(cache) = platform::cache_dir() else {
        log::error!("Android supplied no application cache directory");
        return;
    };
    horizon_client::android_runtime::configure_cache_dir(cache.clone());
    horizon_client::official_install::configure_download_cache_dir(cache);
    horizon_client::android_runtime::install_clipboard(
        horizon_client::android_runtime::ClipboardHooks {
            get: platform::clipboard_get,
            set: platform::clipboard_set,
        },
    );
    if let Err(error) =
        std::fs::create_dir_all(&home).and_then(|()| std::env::set_current_dir(&home))
    {
        log::error!("Cannot open {}: {error}", home.display());
        return;
    }
    #[cfg(debug_assertions)]
    if secure_store::development_probe() {
        std::process::exit(0);
    }
    horizon_client::crash::install(home.clone(), "client");
    exit_reports::install(&home);
    let _ = std::fs::remove_file(home.join("android-error.txt"));
    if let Some(mut data) = engine_data {
        let copied = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(home.join("engine-data.partial"))?;
            std::io::copy(&mut data, &mut file)?;
            drop(file);
            std::fs::rename(
                home.join("engine-data.partial"),
                home.join("engine-data.zip"),
            )
        })();
        if let Err(error) = copied {
            log::error!("Cannot unpack engine data: {error}");
        }
    }
    let mut launcher = launcher::Launcher::new(home.clone());
    let result = android_event_loop()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
        .and_then(|events| events.run_app(&mut launcher).map_err(Into::into));
    if let Err(error) = result {
        log::error!("Horizon: {error}");
        diagnostics::record_error(&error.to_string());
        horizon_client::crash::report_error(&home, "client", &*error);
        let _ = std::fs::write(home.join("android-error.txt"), error.to_string());
    }
    // Winit permits one EventLoop per process. NativeActivity otherwise keeps
    // this process cached after Quit, making the next launch fail. run_app's
    // exiting callback has completed the world save before we get here.
    drop(launcher);
    diagnostics::finish();
    platform::finish();
    APP.with(|slot| *slot.borrow_mut() = None);
    std::process::exit(0);
}
