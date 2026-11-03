//! Bounded local diagnostics, independent of opt-in frame profiling.
//! No signal handlers: Android owns native tombstones and low-memory exits.
#[path = "native_tombstone.rs"]
mod native_tombstone;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const LOG_LIMIT: u64 = 512 * 1024;
const FILE_LIMIT: u64 = 256 * 1024;
const BUNDLE_LIMIT: usize = 4 * 1024 * 1024;
static DROPPED_LOGS: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<Option<State>> = Mutex::new(None);
static EARLY: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
struct State {
    sender: mpsc::SyncSender<Event>,
    session: Value,
    heartbeat: Instant,
}
enum Event {
    Log(String),
    Session(Value),
    Flush(mpsc::SyncSender<()>),
}
pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn reason_name(reason: i32) -> &'static str {
    match reason {
        1 => "exit_self",
        2 => "signaled",
        3 => "low_memory",
        4 => "java_crash",
        5 => "native_crash",
        6 => "anr",
        7 => "initialization_failure",
        8 => "permission_change",
        9 => "excessive_resource_usage",
        10 => "user_requested",
        11 => "user_stopped",
        12 => "dependency_died",
        13 => "other",
        14 => "freezer",
        15 => "package_state_change",
        16 => "package_updated",
        _ => "unknown",
    }
}
pub fn tail(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    file.seek(SeekFrom::Start(size.saturating_sub(limit)))?;
    let mut out = Vec::new();
    file.take(limit).read_to_end(&mut out)?;
    Ok(out)
}
fn atomic_json(path: &Path, value: &Value) -> io::Result<()> {
    let temp = path.with_extension("partial");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)
}
pub fn initialize(home: &Path, device: Value) -> io::Result<()> {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if state.is_some() {
        return Ok(());
    }
    let dir = home.join("performance");
    fs::create_dir_all(&dir)?;
    for (old, new) in [
        ("current-session.json", "previous-session.json"),
        ("diagnostic.log", "previous-diagnostic.log"),
    ] {
        if let Ok(data) = tail(&dir.join(old), LOG_LIMIT) {
            fs::write(dir.join(new), data)?;
        }
    }
    let session = json!({"schema":1,"pid":std::process::id(),"start_ms":timestamp(),
        "version":crate::backend::VERSION,"source_commit":option_env!("HORIZON_ANDROID_BUILD_COMMIT"),
        "source_dirty":option_env!("HORIZON_ANDROID_BUILD_DIRTY"),"abi":std::env::consts::ARCH,
        "phase":"native_start","clean_exit":false,"device":device});
    atomic_json(&dir.join("current-session.json"), &session)?;
    let (sender, receiver) = mpsc::sync_channel(128);
    std::thread::Builder::new()
        .name("Local diagnostics".into())
        .spawn(move || {
            let mut log = File::create(dir.join("diagnostic.log")).ok();
            let mut size = 0u64;
            for event in receiver {
                match event {
                    Event::Log(line) => {
                        let line = redact(&line);
                        if size + line.len() as u64 + 1 > LOG_LIMIT {
                            drop(log.take());
                            let _ = fs::rename(
                                dir.join("diagnostic.log"),
                                dir.join("diagnostic-rotated.log"),
                            );
                            log = File::create(dir.join("diagnostic.log")).ok();
                            size = 0;
                        }
                        if let Some(file) = log.as_mut() {
                            let _ = writeln!(file, "{line}");
                        }
                        size += line.len() as u64 + 1;
                    }
                    Event::Session(mut value) => {
                        value["memory"] = memory();
                        let _ = atomic_json(&dir.join("current-session.json"), &value);
                    }
                    Event::Flush(done) => {
                        if let Some(file) = log.as_mut() {
                            let _ = file.flush();
                        }
                        let _ = done.send(());
                    }
                }
            }
        })?;
    for line in EARLY.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
        let _ = sender.try_send(Event::Log(line));
    }
    *state = Some(State {
        sender,
        session,
        heartbeat: Instant::now(),
    });
    Ok(())
}
/// Called by the logger; never waits for disk, the state lock or a full queue.
pub fn log_line(level: log::Level, target: &str, message: &str) {
    if level > log::Level::Info {
        return;
    }
    let line = format!(
        "{} [{level}] {target}: {}",
        timestamp(),
        message.chars().take(4096).collect::<String>()
    );
    let guard = match STATE.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => {
            DROPPED_LOGS.fetch_add(1, Ordering::Relaxed);
            return;
        }
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    if let Some(state) = guard.as_ref() {
        if state.sender.try_send(Event::Log(line)).is_err() {
            DROPPED_LOGS.fetch_add(1, Ordering::Relaxed);
        }
        return;
    }
    drop(guard);
    let mut early = EARLY.lock().unwrap_or_else(|e| e.into_inner());
    if early.len() == 32 {
        early.pop_front();
    }
    early.push_back(line);
}
fn memory() -> Value {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let mut values = serde_json::Map::new();
    for line in status.lines() {
        if let Some((key, value)) = line.split_once(':')
            && matches!(key, "VmRSS" | "VmHWM" | "VmSize" | "VmPeak" | "Threads")
        {
            values.insert(key.into(), value.trim().into());
        }
    }
    json!(values)
}
/// Frame-safe heartbeat: a busy diagnostic state skips this update rather than
/// making rendering wait. Memory collection and disk writes happen off-thread.
pub fn heartbeat(phase: &str, context: impl FnOnce() -> Value) {
    let mut guard = match STATE.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    if let Some(state) = guard.as_mut() {
        if state.session["phase"] == phase && state.heartbeat.elapsed() < Duration::from_secs(5) {
            return;
        }
        state.heartbeat = Instant::now();
        state.session["phase"] = phase.into();
        state.session["updated_ms"] = timestamp().into();
        state.session["dropped_log_lines"] = DROPPED_LOGS.load(Ordering::Relaxed).into();
        state.session["context"] = context();
        let _ = state.sender.try_send(Event::Session(state.session.clone()));
    }
}
pub fn record_error(error: &str) {
    if let Some(state) = STATE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        state.session["last_handled_error"] =
            redact(&error.chars().take(4096).collect::<String>()).into();
        state.session["error_ms"] = timestamp().into();
        let _ = state.sender.try_send(Event::Session(state.session.clone()));
    }
}
pub fn finish() {
    if let Some(state) = STATE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        state.session["clean_exit"] = true.into();
        state.session["updated_ms"] = timestamp().into();
        let _ = state.sender.try_send(Event::Session(state.session.clone()));
    }
    flush();
}
/// Logger flush must not make startup/rendering wait for the diagnostic disk worker.
pub fn request_flush() {
    let (tx, _rx) = mpsc::sync_channel(1);
    let guard = match STATE.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    if let Some(state) = guard.as_ref() {
        let _ = state.sender.try_send(Event::Flush(tx));
    }
}
pub fn flush() {
    let (tx, rx) = mpsc::sync_channel(1);
    if let Some(state) = STATE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = state.sender.try_send(Event::Flush(tx));
    }
    let _ = rx.recv_timeout(Duration::from_millis(500));
}
pub fn previous_unclean(home: &Path) -> bool {
    fs::read(home.join("performance/previous-session.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .is_some_and(|v| v["clean_exit"] != true)
}
pub fn summary(home: &Path) -> String {
    let previous = fs::read(home.join("performance/previous-session.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    match previous {
        Some(v) if v["clean_exit"] != true => format!("Previous session ended without a clean exit; last phase: {}. This may be a crash or an Android process kill. Export diagnostics before reporting.", v["phase"].as_str().unwrap_or("unknown")),
        _ => "Export one diagnostic ZIP to send with your report. It includes local logs, Android exit history and retained performance recordings. Review before sharing.".into(),
    }
}
/// Best-effort redaction, not a promise that arbitrary third-party text is public.
pub fn redact(text: &str) -> String {
    static PRIVATE: OnceLock<regex::Regex> = OnceLock::new();
    static ADDRESS: OnceLock<regex::Regex> = OnceLock::new();
    static DOMAIN: OnceLock<regex::Regex> = OnceLock::new();
    let private = PRIVATE.get_or_init(|| regex::Regex::new(r"(?i)access.?token|refresh.?token|authorization|bearer |password|client.?secret|xbl3\.0|chat message|connecting to|joining server|player name|username").unwrap());
    let address = ADDRESS.get_or_init(|| regex::Regex::new(r#"(?i)https?://[^\s\"<>]+|\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?::[0-9]+)?|/data/(?:user|user_de)/[^\s\"<>]+|/storage/emulated/[^\s\"<>]+|[a-z]:\\Users\\[^\\\s]+"#).unwrap());
    let domain = DOMAIN.get_or_init(|| {
        regex::Regex::new(r"(?i)\b(?:[a-z0-9_-]+\.)+[a-z]{2,}\b|\[[a-f0-9:]+\](?::[0-9]+)?")
            .unwrap()
    });
    text.lines()
        .map(|line| {
            if private.is_match(line) {
                "[private log line removed]".into()
            } else {
                let line = address.replace_all(line, "[redacted]");
                domain
                    .replace_all(&line, |caps: &regex::Captures<'_>| {
                        let name = &caps[0];
                        let extension = name.rsplit('.').next().unwrap_or_default();
                        if matches!(
                            extension,
                            "rs" | "so"
                                | "dll"
                                | "wgsl"
                                | "json"
                                | "jsonl"
                                | "txt"
                                | "java"
                                | "kt"
                                | "toml"
                                | "trace"
                                | "log"
                                | "zip"
                                | "apk"
                        ) {
                            name.to_owned()
                        } else {
                            "[address removed]".into()
                        }
                    })
                    .into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
/// Explicit user action only. No accounts/configuration/worlds, no uploads.
pub fn export(home: &Path) -> Result<PathBuf, String> {
    flush();
    let dir = home.join("performance");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("horizon-diagnostics-{}.zip", timestamp()));
    let stage = path.with_extension("partial");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        for folder in [&dir, &home.join("crash-reports")] {
            if let Ok(entries) = fs::read_dir(folder) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if entry.file_type()?.is_file()
                        && !entry.file_type()?.is_symlink()
                        && (name.ends_with(".log")
                            || name.ends_with(".trace")
                            || (name.starts_with("horizon-performance-")
                                && name.ends_with(".jsonl"))
                            || name.starts_with("crash-")
                            || matches!(
                                name.as_str(),
                                "android-exits.json"
                                    | "current-session.json"
                                    | "previous-session.json"
                                    | "last-rust-panic.txt"
                                    | "previous-error.txt"
                            ))
                    {
                        files.push(entry.path());
                    }
                }
            }
        }
        files.sort_by_key(|p| std::cmp::Reverse(p.file_name().map(|s| s.to_owned())));
        let mut archive = zip::ZipWriter::new(File::create(&stage)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut included = Vec::new();
        let mut omitted = Vec::new();
        let mut details = Vec::new();
        let mut total = 0;
        for file in files.into_iter().take(32) {
            let mut name = file.file_name().unwrap().to_string_lossy().into_owned();
            let metadata = file.metadata()?;
            let source_size = metadata.len();
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis());
            let data = if name.ends_with(".trace") {
                let mut data = Vec::new();
                File::open(&file)?
                    .take(1024 * 1024)
                    .read_to_end(&mut data)?;
                data
            } else {
                tail(&file, FILE_LIMIT)?
            };
            let binary_trace = name.ends_with(".trace")
                && (std::str::from_utf8(&data).is_err()
                    || data.iter().any(|b| *b < 32 && !matches!(b, 9 | 10 | 13)));
            let text = if binary_trace {
                omitted.push(json!({"file":name,"reason":"binary system trace: original retained privately on device"}));
                let Some(summary) = native_tombstone::summarize(&data) else {
                    continue;
                };
                name.push_str("-summary.txt");
                redact(&summary)
            } else {
                redact(&String::from_utf8_lossy(&data))
            };
            if total + text.len() > BUNDLE_LIMIT {
                omitted.push(json!({"file":name,"reason":"bundle size limit"}));
                continue;
            }
            if included.contains(&name) {
                continue;
            }
            archive.start_file(&name, options)?;
            archive.write_all(text.as_bytes())?;
            total += text.len();
            details.push(json!({"file":name,"source_bytes":source_size,"input_truncated_by_export":source_size>if name.ends_with("summary.txt") { 1024*1024 } else { FILE_LIMIT },"source_modified_ms":modified_ms}));
            included.push(name);
        }
        archive.start_file("manifest.json", options)?;
        archive.write_all(&serde_json::to_vec_pretty(&json!({"schema":1,"version":crate::backend::VERSION,"source_commit":option_env!("HORIZON_ANDROID_BUILD_COMMIT"),"included":included,"files":details,"omitted":omitted,"note":"Logs are bounded tails with best-effort redaction. Review before sharing. Binary Android tombstones contribute a bounded crashed-thread summary; original memory/register dumps are excluded and remain private on device. No account files, worlds or configuration are included."}))?)?;
        archive.finish()?;
        fs::rename(&stage, &path)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(stage);
        return Err(error.to_string());
    }
    let mut old: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("horizon-diagnostics-"))
                && p.extension().is_some_and(|s| s == "zip")
        })
        .collect();
    old.sort();
    let excess = old.len().saturating_sub(3);
    for file in old.into_iter().take(excess) {
        let _ = fs::remove_file(file);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn home() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "horizon-diagnostics-unit-{}-{}",
            std::process::id(),
            timestamp()
        ));
        fs::create_dir_all(path.join("performance")).unwrap();
        path
    }
    #[test]
    fn redaction_keeps_native_frames_but_removes_credentials_and_addresses() {
        let text = redact(
            "access_token=sealed-token-value\nconnection 192.0.2.7:25565 leo.spcfy.eu https://example.org/private\n/data/user/0/dev.horizon.android/files/saves/PrivateWorld\nC:\\Users\\Alice\\source\\game.rs\n#00 pc 00004abc libvulkan.so (BuildId: abcdef123456)",
        );
        for secret in [
            "sealed-token-value",
            "192.0.2.7",
            "leo.spcfy.eu",
            "example.org",
            "PrivateWorld",
            "Alice",
        ] {
            assert!(!text.contains(secret), "{secret}: {text}");
        }
        assert!(text.contains("libvulkan.so"));
        assert!(text.contains("abcdef123456"));
    }
    #[test]
    fn export_allowlist_bounds_and_binary_trace_manifest() {
        let home = home();
        let dir = home.join("performance");
        fs::write(home.join("account.json"), b"secret-account").unwrap();
        fs::write(dir.join("account.json"), b"secret-account").unwrap();
        fs::write(
            dir.join("diagnostic.log"),
            "Vulkan initialized\nAuthorization: secret\n",
        )
        .unwrap();
        fs::write(dir.join("exit-1.trace"), [0xff, 0, 0xff]).unwrap();
        fs::write(
            dir.join("previous-session.json"),
            r#"{"phase":"game_loading","clean_exit":false}"#,
        )
        .unwrap();
        assert!(previous_unclean(&home));
        assert!(summary(&home).contains("game_loading"));
        let file = export(&home).unwrap();
        let mut zip = zip::ZipArchive::new(File::open(file).unwrap()).unwrap();
        assert!(zip.by_name("account.json").is_err());
        assert!(zip.by_name("exit-1.trace").is_err());
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("binary system trace"));
        let mut log = String::new();
        zip.by_name("diagnostic.log")
            .unwrap()
            .read_to_string(&mut log)
            .unwrap();
        assert!(log.contains("Vulkan initialized"));
        assert!(!log.contains("secret"));
        drop(zip);
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn writer_survives_errors_and_heartbeat_is_lazy() {
        let home = home();
        fs::write(
            home.join("performance/current-session.json"),
            r#"{"phase":"prior_world","clean_exit":false}"#,
        )
        .unwrap();
        log_line(log::Level::Info, "probe", "before initialization");
        initialize(&home, json!({"model":"probe"})).unwrap();
        heartbeat("in_world", || json!({"backend":"Vulkan"}));
        heartbeat("in_world", || {
            panic!("context must not be built each frame")
        });
        {
            // A frame must never wait for a diagnostic lock held elsewhere.
            let _guard = STATE.lock().unwrap();
            heartbeat("blocked", || panic!("busy heartbeat must skip context"));
            log_line(log::Level::Info, "probe", "busy diagnostic state");
            request_flush();
        }
        record_error("VK_ERROR_DEVICE_LOST");
        log_line(log::Level::Error, "probe", "VK_ERROR_DEVICE_LOST");
        finish();
        let current: Value = serde_json::from_slice(
            &fs::read(home.join("performance/current-session.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(current["clean_exit"], true);
        assert_eq!(current["last_handled_error"], "VK_ERROR_DEVICE_LOST");
        assert_eq!(current["context"]["backend"], "Vulkan");
        let text = fs::read_to_string(home.join("performance/diagnostic.log")).unwrap();
        assert!(text.contains("before initialization"));
        assert!(text.contains("VK_ERROR_DEVICE_LOST"));
        assert!(summary(&home).contains("prior_world"));
        STATE.lock().unwrap().take();
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn exit_reasons_distinguish_native_java_and_low_memory() {
        assert_eq!(reason_name(3), "low_memory");
        assert_eq!(reason_name(4), "java_crash");
        assert_eq!(reason_name(5), "native_crash");
        assert_eq!(reason_name(6), "anr");
        assert_eq!(reason_name(999), "unknown");
    }
}
