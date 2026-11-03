//! Opt-in local performance reports. No account, chat, world names or positions.
use horizon_client::{control::FrameStats, spans};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_SECONDS: u64 = 1800;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
struct Recorder {
    start: Instant,
    interval: Instant,
    frames: FrameStats,
    slow: Vec<Value>,
    sender: mpsc::SyncSender<Value>,
    worker: std::thread::JoinHandle<std::io::Result<()>>,
    dropped: u64,
}

pub fn start(home: &Path, metadata: Value) -> horizon_client::Result<()> {
    stop();
    PROFILE_OWNS_RECORDER.store(false, Ordering::Relaxed);
    let dir = home.join("performance");
    std::fs::create_dir_all(&dir)?;
    let mut files = reports(home);
    while files.len() >= 3 {
        std::fs::remove_file(files.remove(0))?;
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("horizon-performance-{stamp}.jsonl")))?;
    let (sender, receiver) = mpsc::sync_channel::<Value>(8);
    let worker = std::thread::Builder::new().name("Performance report".into()).spawn(move || {
        let mut file = std::io::BufWriter::new(file);
        let header = json!({"type":"header", "schema":1, "version":crate::backend::VERSION,
            "max_seconds":MAX_SECONDS, "metadata":metadata,
            "timing_notes":"CPU spans are nested and must not be summed. Worker totals span concurrent jobs. GPU timestamps are delayed and may be unavailable. Wall minus thread CPU includes waiting and scheduling; it is not proof of lock contention."});
        let mut bytes = write_line(&mut file, &header)?;
        file.flush()?;
        for row in receiver {
            let encoded = serde_json::to_vec(&row)?;
            if bytes + encoded.len() + 1 + 128 > MAX_BYTES {
                write_line(&mut file, &json!({"type":"end","reason":"size_limit"}))?;
                return file.flush();
            }
            file.write_all(&encoded)?;
            file.write_all(b"\n")?;
            bytes += encoded.len() + 1;
            file.flush()?;
        }
        write_line(&mut file, &json!({"type":"end","reason":"stopped"}))?;
        file.flush()
    })?;
    *RECORDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(Recorder {
        start: Instant::now(),
        interval: Instant::now(),
        frames: Default::default(),
        slow: vec![],
        sender,
        worker,
        dropped: 0,
    });
    spans::record_timings(true);
    spans::enable(true);
    ACTIVE.store(true, Ordering::Relaxed);
    Ok(())
}
fn write_line(file: &mut impl Write, row: &Value) -> std::io::Result<usize> {
    let bytes = serde_json::to_vec(row)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(bytes.len() + 1)
}
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}
pub fn slow_frame(cpu: f32, timings: &[spans::Span]) {
    if !active() || cpu < 50.0 {
        return;
    }
    if let Some(r) = RECORDER.lock().unwrap_or_else(|e| e.into_inner()).as_mut()
        && r.slow.len() < 16
    {
        r.slow.push(
            json!({"type":"cpu_spike", "t_ms":r.start.elapsed().as_millis(),
            "cpu_wall_ms":cpu, "stages":timings.iter().take(24).map(|s| json!({
                "stage":s.name,"wall_ms":s.ms,"thread_cpu_ms":s.cpu_ms})).collect::<Vec<_>>()}),
        );
    }
}
pub fn record(frame: f32, cpu: f32, gpu: &[f32], phases: [f32; 4]) -> Option<Value> {
    if !active() {
        return None;
    }
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let r = guard.as_mut()?;
    if r.start.elapsed().as_secs() >= MAX_SECONDS {
        ACTIVE.store(false, Ordering::Relaxed);
        spans::record_timings(false);
        spans::enable(false);
        return None;
    }
    r.frames.record(frame);
    r.frames.record_cpu(cpu);
    r.frames.record_gpu(gpu);
    r.frames.record_phases(phases);
    if frame >= 50.0 && r.slow.len() < 16 {
        r.slow.push(
            json!({"type":"slow_frame","t_ms":r.start.elapsed().as_millis(),
            "frame_ms":frame,"cpu_ms":cpu,"gpu_ms":gpu.last(),"phases_ms":phases}),
        );
    }
    if r.interval.elapsed() < Duration::from_secs(1) {
        return None;
    }
    let row = json!({"type":"sample","t_ms":r.start.elapsed().as_millis(),
        "window_wall_ms":r.interval.elapsed().as_millis(),"frames":r.frames.summary(),
        "timings":spans::drain_timings(),"slow_events":std::mem::take(&mut r.slow),
        "dropped_samples":r.dropped});
    r.frames.reset();
    r.interval = Instant::now();
    Some(row)
}
pub fn submit(mut row: Value, context: Value) {
    row["context"] = context;
    if let Some(r) = RECORDER.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        match r.sender.try_send(row) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => r.dropped += 1,
            Err(mpsc::TrySendError::Disconnected(_)) => {
                ACTIVE.store(false, Ordering::Relaxed);
                spans::record_timings(false);
                spans::enable(false);
            }
        }
    }
}
pub fn stop() {
    ACTIVE.store(false, Ordering::Relaxed);
    PROFILE_SWITCH.store(false, Ordering::Relaxed);
    PROFILE_OWNS_RECORDER.store(false, Ordering::Relaxed);
    spans::record_timings(false);
    spans::enable(false);
    if let Some(r) = RECORDER.lock().unwrap_or_else(|e| e.into_inner()).take() {
        drop(r.sender);
        if !matches!(r.worker.join(), Ok(Ok(()))) {
            log::warn!("Performance report writer failed");
        }
    }
}
fn reports(home: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(home.join("performance"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                n.to_string_lossy().starts_with("horizon-performance-")
                    && p.extension().is_some_and(|e| e == "jsonl")
            })
        })
        .collect();
    files.sort();
    files
}
pub fn export(home: &Path) -> std::result::Result<(), String> {
    stop();
    reports(home)
        .pop()
        .ok_or("No performance report. Enable recording, then launch and play first.")?;
    if crate::platform::open_documents("reports") {
        Ok(())
    } else {
        Err("Open the system Files app, then Horizon Android / Reports.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_stages_spikes_retention_and_size_limit() {
        let home = std::env::temp_dir().join(format!(
            "horizon-perf-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(record(16.0, 5.0, &[], [0.0; 4]).is_none());
        assert!(export(&home).is_err());
        for _ in 0..4 {
            start(&home, json!({"renderer":"test"})).unwrap();
            spans::job("mesh_section", || {
                std::thread::sleep(Duration::from_millis(3))
            });
            slow_frame(
                80.0,
                &[spans::Span {
                    name: "tick_blocks",
                    ms: 70.0,
                    cpu_ms: 65.0,
                }],
            );
            RECORDER.lock().unwrap().as_mut().unwrap().interval =
                Instant::now() - Duration::from_secs(2);
            let sample = record(60.0, 12.0, &[40.0], [2.0, 3.0, 4.0, 3.0]).unwrap();
            submit(sample, json!({"pending_uploads":4}));
            stop();
        }
        let files = reports(&home);
        assert_eq!(files.len(), 3);
        let text = std::fs::read_to_string(files.last().unwrap()).unwrap();
        let rows: Vec<Value> = text
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows[0]["schema"], 1);
        let sample = &rows[1];
        assert_eq!(sample["frames"]["gpu"]["avg_ms"], 40.0);
        assert_eq!(sample["context"]["pending_uploads"], 4);
        let events = sample["slow_events"].as_array().unwrap();
        assert!(events.iter().any(|e| e["frame_ms"] == 60.0));
        assert!(
            events
                .iter()
                .any(|e| e["stages"][0]["stage"] == "tick_blocks")
        );
        assert!(
            sample["timings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["stage"] == "mesh_section" && v["wall_ms"].as_f64().unwrap() >= 2.0)
        );
        assert_eq!(rows.last().unwrap()["type"], "end");
        start(&home, json!({})).unwrap();
        submit(
            json!({"type":"test"}),
            json!({"padding":"x".repeat(MAX_BYTES)}),
        );
        stop();
        let last = reports(&home).pop().unwrap();
        assert!(std::fs::metadata(&last).unwrap().len() < MAX_BYTES as u64);
        assert!(
            std::fs::read_to_string(last)
                .unwrap()
                .contains("size_limit")
        );
        start(&home, json!({})).unwrap();
        RECORDER.lock().unwrap().as_mut().unwrap().start =
            Instant::now() - Duration::from_secs(MAX_SECONDS + 1);
        assert!(record(16.0, 4.0, &[], [0.0; 4]).is_none());
        assert!(!active());
        stop();
        // GPU profiling joins the existing writer, and disabling it must not
        // stop an independently requested CPU/performance report.
        start(&home, json!({"purpose":"combined"})).unwrap();
        configure_gpu_profile(true, &home);
        assert!(active());
        assert!(!PROFILE_OWNS_RECORDER.load(Ordering::Relaxed));
        configure_gpu_profile(false, &home);
        assert!(active());
        stop();
        configure_gpu_profile(true, &home);
        assert!(active());
        assert!(PROFILE_OWNS_RECORDER.load(Ordering::Relaxed));
        configure_gpu_profile(false, &home);
        assert!(!active());
        configure_gpu_profile(true, &home);
        start(&home, json!({"purpose":"replace_gpu_only"})).unwrap();
        configure_gpu_profile(true, &home);
        configure_gpu_profile(false, &home);
        assert!(active());
        stop();
        // Returning to the launcher resets the switch for the next Play.
        configure_gpu_profile(true, &home);
        assert!(active());
        stop();
        std::fs::remove_dir_all(home).unwrap();
    }
}

static PROFILE_SWITCH: AtomicBool = AtomicBool::new(false);
static PROFILE_OWNS_RECORDER: AtomicBool = AtomicBool::new(false);
pub fn configure_gpu_profile(on: bool, home: &Path) {
    let old = PROFILE_SWITCH.swap(on, Ordering::Relaxed);
    if on && !old && !active() {
        match start(
            home,
            json!({"purpose":"detailed_gpu_profile","quantiles":"rolling 120 completed GPU samples per stage; delayed readback","limits":"No GPU clocks or vendor hardware counters; correlation is not proof of cause."}),
        ) {
            Ok(()) => {
                PROFILE_SWITCH.store(true, Ordering::Relaxed);
                PROFILE_OWNS_RECORDER.store(true, Ordering::Relaxed);
            }
            Err(error) => log::warn!("Cannot start GPU profile: {error}"),
        }
    } else if !on && old && PROFILE_OWNS_RECORDER.swap(false, Ordering::Relaxed) {
        stop();
    }
}
