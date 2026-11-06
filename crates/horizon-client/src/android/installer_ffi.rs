use serde_json::json;
use std::ffi::{CStr, c_char};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const STATE_IDLE: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_DONE: u8 = 2;
const STATE_FAILED: u8 = 3;

struct Status {
    state: u8,
    stage: u8,
    done: u64,
    total: u64,
    error: String,
    last_write: Option<Instant>,
}

static STATUS: Mutex<Status> = Mutex::new(Status {
    state: STATE_IDLE,
    stage: 0,
    done: 0,
    total: 0,
    error: String::new(),
    last_write: None,
});

static STATUS_HOME: OnceLock<PathBuf> = OnceLock::new();

fn stage_id(stage: horizon_client::official_install::InstallStage) -> u8 {
    use horizon_client::official_install::InstallStage;
    match stage {
        InstallStage::Metadata => 0,
        InstallStage::Download => 1,
        InstallStage::Extract => 2,
        InstallStage::WorldData => 3,
        InstallStage::Controls => 4,
        InstallStage::Finalize => 5,
    }
}

fn stage_name(stage: u8) -> &'static str {
    match stage {
        0 => "metadata",
        1 => "download",
        2 => "extract",
        3 => "world-data",
        4 => "controls",
        5 => "finalize",
        _ => "idle",
    }
}

fn unit(stage: u8) -> &'static str {
    match stage {
        1 => "bytes",
        4 => "files",
        _ => "none",
    }
}

fn write_status(status: &Status) {
    let Some(home) = STATUS_HOME.get() else {
        return;
    };
    let body = json!({
        "state": match status.state {
            STATE_RUNNING => "running",
            STATE_DONE => "done",
            STATE_FAILED => "failed",
            _ => "idle",
        },
        "stage": stage_name(status.stage),
        "stage_id": status.stage,
        "done": status.done,
        "total": status.total,
        "unit": unit(status.stage),
        "error": status.error,
    });
    let path = home.join("horizon-install-status.json");
    let partial = home.join("horizon-install-status.partial");
    if let Ok(bytes) = serde_json::to_vec(&body)
        && std::fs::write(&partial, bytes).is_ok()
    {
        let _ = std::fs::rename(&partial, &path);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn horizon_install_start(home: *const c_char, cache: *const c_char) -> i32 {
    if home.is_null() || cache.is_null() {
        return -1;
    }
    let home = unsafe { CStr::from_ptr(home) }
        .to_string_lossy()
        .into_owned();
    let cache = unsafe { CStr::from_ptr(cache) }
        .to_string_lossy()
        .into_owned();
    let home = PathBuf::from(home);
    let cache = PathBuf::from(cache);
    if !home.is_absolute() || !cache.is_absolute() {
        return -1;
    }
    if !home.join("engine-data.zip").is_file() {
        return -2;
    }
    let _ = STATUS_HOME.set(home.clone());
    {
        let mut status = STATUS.lock().unwrap_or_else(|error| error.into_inner());
        if status.state == STATE_RUNNING {
            return -3;
        }
        *status = Status {
            state: STATE_RUNNING,
            stage: 0,
            done: 0,
            total: 0,
            error: String::new(),
            last_write: None,
        };
        write_status(&status);
    }
    horizon_client::official_install::configure_download_cache_dir(cache);
    std::thread::spawn(move || {
        let result = horizon_client::official_install::install_with_progress(&home, |progress| {
            let mut status = STATUS.lock().unwrap_or_else(|error| error.into_inner());
            let stage = stage_id(progress.stage);
            let changed = status.stage != stage;
            status.stage = stage;
            status.done = progress.done;
            status.total = progress.total;
            let due = status
                .last_write
                .is_none_or(|last| last.elapsed() >= Duration::from_millis(300));
            if changed || due {
                status.last_write = Some(Instant::now());
                write_status(&status);
            }
        });
        let mut status = STATUS.lock().unwrap_or_else(|error| error.into_inner());
        match result {
            Ok(()) => {
                status.state = STATE_DONE;
                status.stage = 5;
                status.done = status.total;
                status.error.clear();
            }
            Err(error) => {
                status.state = STATE_FAILED;
                status.error = error.to_string();
            }
        }
        status.last_write = Some(Instant::now());
        write_status(&status);
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn horizon_install_ready(home: *const c_char) -> i32 {
    if home.is_null() {
        return 0;
    }
    let home = unsafe { CStr::from_ptr(home) }
        .to_string_lossy()
        .into_owned();
    let home = Path::new(&home);
    i32::from(home.is_absolute() && horizon_client::official_install::ready(home))
}
