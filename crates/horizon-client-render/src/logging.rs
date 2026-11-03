//! The game's log, as vanilla's log4j configuration writes it (client-1.21.2.xml, the one
//! 26.3's version manifest names): `logs/latest.log` in the game folder with the pattern
//! `[%d{HH:mm:ss}] [%t/%level]: %msg%n`, rolled over at start (OnStartupTriggeringPolicy, a
//! non-empty file only) and when the day changes (TimeBasedTriggeringPolicy) to
//! `logs/%d{yyyy-MM-dd}-%i.log.gz`, dated by the old file's last write. DefaultRolloverStrategy
//! keeps 7 archives a day, the newest with the highest index.
//!
//! Unlike log4j, the start's rollover waits for the run's first line, and it also keeps the last
//! run's log as it was in `logs/previous.log`: a player sends that one when the game has been
//! opened again since (Android starts it again by itself right after ending it).
//!
//! Messages go through the `log` facade. Horizon's own still print their plain text on stderr,
//! as they did before the log existed; other crates' errors only reach the file (their
//! warnings are mostly the Vulkan loader's startup chatter).
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate};

/// DefaultRolloverStrategy's default `max`: archives kept for one date.
pub const MAX_ARCHIVES_PER_DAY: u32 = 7;
/// The last run's log, uncompressed beside latest.log.
pub const PREVIOUS_LOG: &str = "previous.log";
/// Formatted lines kept in memory for crash reports.
const RECENT: usize = 50;
/// Warnings and errors one line of code may log in a [`FLOOD_WINDOW`]; past that they are counted,
/// not written, so a warning repeated every packet can't fill the disk. Vanilla has no such limit:
/// below it the log is vanilla's, line for line.
const FLOOD_LINES: u32 = 100;
const FLOOD_WINDOW: Duration = Duration::from_secs(60);

/// The platform data folder (what the `dirs` crate's data_dir gives).
pub fn data_dir() -> PathBuf {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let home = env("HOME").or_else(|| env("USERPROFILE"));
    if cfg!(windows) {
        if let Some(d) = env("APPDATA") {
            return d;
        }
    } else if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            return h.join("Library/Application Support");
        }
    } else if let Some(d) = env("XDG_DATA_HOME") {
        return d;
    } else if let Some(h) = &home {
        return h.join(".local/share");
    }
    std::env::temp_dir()
}

/// `--home DIR`, set once at start; it wins over `HORIZON_HOME`.
static HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Makes `game_home` return `home` for the rest of the run (`--home`).
pub fn set_game_home(home: PathBuf) {
    let _ = HOME.set(home);
}

/// `--player-dir DIR` (#4952), set once at start: one player's own files, when several games
/// play in one game folder at the same time (the launcher's `play --account`).
static PLAYER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Makes [`player_dir`] return `dir` for the rest of the run (`--player-dir`), creating it.
/// What is a player's own goes there: `options.txt` and the settings files beside it,
/// `logs/`, `crash-reports/`, `horizon-player.txt`, `servers.dat`, `hotbar.nbt` and
/// `screenshots/`. What the game folder's players share stays in it: saves, resource and
/// shader packs, mods and their config, caches.
pub fn set_player_dir(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let _ = PLAYER.set(dir);
}

/// The folder of this player's own files: `--player-dir`, else the game folder (the working
/// directory, where the launcher runs the game).
pub fn player_dir() -> PathBuf {
    PLAYER
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// `--player-dir`, if given.
pub fn player_dir_override() -> Option<&'static Path> {
    PLAYER.get().map(PathBuf::as_path)
}

/// Where the files all players share live, for settings kept beside `options.txt` in
/// `options_dir` (the enabled mods): that folder, unless `--player-dir` moved `options.txt`
/// out of the game folder.
pub fn shared_dir(options_dir: &Path) -> PathBuf {
    shared_dir_of(PLAYER.get().map(PathBuf::as_path), options_dir, || {
        std::env::current_dir().unwrap_or_default()
    })
}

/// [`shared_dir`] for a given `--player-dir` and game folder.
fn shared_dir_of(
    player: Option<&Path>,
    options_dir: &Path,
    game_dir: impl FnOnce() -> PathBuf,
) -> PathBuf {
    match player {
        Some(player) if player == options_dir => game_dir(),
        _ => options_dir.to_path_buf(),
    }
}

/// Where the launcher runs the game and keeps its saves: `--home`, else `HORIZON_HOME`, else
/// `horizon` in the platform data folder (launcher/src/backend.rs `game_home`).
pub fn game_home() -> PathBuf {
    if let Some(home) = HOME.get() {
        return home.clone();
    }
    match std::env::var_os("HORIZON_HOME").filter(|h| !h.is_empty()) {
        Some(h) => PathBuf::from(h),
        None => data_dir().join("horizon"),
    }
}

/// What the game keeps apart from the player's folder: `HORIZON_PRIVATE`, else the game folder.
/// Android shows the game folder in shared storage and keeps the prepared resources, the caches,
/// the import inbox and the control endpoint in the app's own files.
pub fn private_home() -> PathBuf {
    match std::env::var_os("HORIZON_PRIVATE").filter(|h| !h.is_empty()) {
        Some(h) => PathBuf::from(h),
        None => game_home(),
    }
}

/// Derived caches (`.horizon-cache`): in the folder the game runs in, or in `HORIZON_PRIVATE`
/// when that is set apart (shared storage on Android has no symlinks for pack overlays).
pub fn cache_dir() -> PathBuf {
    match std::env::var_os("HORIZON_PRIVATE").filter(|h| !h.is_empty()) {
        Some(h) => PathBuf::from(h).join(".horizon-cache"),
        None => std::env::current_dir()
            .unwrap_or_default()
            .join(".horizon-cache"),
    }
}

/// `logs/` and `crash-reports/` of a game folder.
pub fn logs_dir(home: &Path) -> PathBuf {
    home.join("logs")
}
pub fn crash_reports_dir(home: &Path) -> PathBuf {
    home.join("crash-reports")
}

/// `2026-10-14-3.log.gz`
pub fn archive_name(date: NaiveDate, index: u32) -> String {
    format!("{}-{index}.log.gz", date.format("%Y-%m-%d"))
}

/// The archive index of `name` if it is one of `date`'s archives.
fn archive_index(name: &str, date: NaiveDate) -> Option<u32> {
    let prefix = format!("{}-", date.format("%Y-%m-%d"));
    name.strip_prefix(&prefix)?
        .strip_suffix(".log.gz")?
        .parse()
        .ok()
        .filter(|&i| i > 0)
}

/// What one rollover does to the archives already there.
#[derive(Debug, PartialEq, Eq)]
pub struct Rollover {
    pub delete: Option<String>,
    /// In order: each target is free when its rename runs.
    pub renames: Vec<(String, String)>,
    pub archive: String,
}

/// DefaultRolloverStrategy with fileIndex "max": the new archive takes the next index; once
/// `max` are there the lowest goes and the others move down one.
pub fn rollover(existing: &[String], date: NaiveDate, max: u32) -> Rollover {
    let mut present: Vec<u32> = existing
        .iter()
        .filter_map(|n| archive_index(n, date))
        .collect();
    present.sort_unstable();
    present.dedup();
    let highest = present.last().copied().unwrap_or(0);
    if highest < max {
        return Rollover {
            delete: None,
            renames: Vec::new(),
            archive: archive_name(date, highest + 1),
        };
    }
    let delete = Some(archive_name(date, present[0]));
    let kept = &present[1..];
    let renames = kept
        .iter()
        .enumerate()
        .map(|(i, &from)| (from, i as u32 + 1))
        .filter(|(from, to)| from != to)
        .map(|(from, to)| (archive_name(date, from), archive_name(date, to)))
        .collect();
    Rollover {
        delete,
        renames,
        archive: archive_name(date, kept.len() as u32 + 1),
    }
}

/// `[12:04:31] [Render thread/INFO]: message`
pub fn format_line(
    time: &DateTime<Local>,
    thread: &str,
    level: log::Level,
    message: &str,
) -> String {
    format!(
        "[{}] [{thread}/{level}]: {message}",
        time.format("%H:%M:%S")
    )
}

/// Gzips `text` into the next archive for `date` in `dir`.
fn archive(dir: &Path, date: NaiveDate, text: &[u8]) -> io::Result<()> {
    let names: Vec<String> = fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    let plan = rollover(&names, date, MAX_ARCHIVES_PER_DAY);
    if let Some(name) = &plan.delete {
        fs::remove_file(dir.join(name))?;
    }
    for (from, to) in &plan.renames {
        fs::rename(dir.join(from), dir.join(to))?;
    }
    let mut gz = flate2::write::GzEncoder::new(
        File::create(dir.join(&plan.archive))?,
        flate2::Compression::default(),
    );
    gz.write_all(text)?;
    gz.finish()?.sync_all()
}

/// Moves what `file` holds into an archive dated `date` and empties it; with `previous`, a copy
/// stays as `previous.log` too. The file stays open (and locked), so this works on Windows too.
fn roll(file: &mut File, dir: &Path, date: NaiveDate, previous: bool) -> io::Result<()> {
    let mut text = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut text)?;
    // Only a convenience: the archive goes on without it.
    let copied = if previous && !text.is_empty() {
        fs::write(dir.join(PREVIOUS_LOG), &text)
    } else {
        Ok(())
    };
    if !text.is_empty() {
        archive(dir, date, &text)?;
    }
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    copied
}

/// The open `latest.log` and the lock that keeps it this game's.
pub struct Latest {
    pub file: File,
    _lock: File,
    dir: PathBuf,
    /// The date of the last run's lines still in the file.
    last_run: Option<NaiveDate>,
}

impl Latest {
    /// Archives the last run's lines (keeping them as previous.log), once, before this run's first
    /// line: until then they stay in latest.log, so a run that writes nothing (Android starting the
    /// app again right after it ended it) does not leave it empty.
    pub fn archive_last_run(&mut self) -> io::Result<()> {
        let Some(date) = self.last_run.take() else {
            return Ok(());
        };
        let rolled = roll(&mut self.file, &self.dir, date, true);
        if rolled.is_err() {
            // This run's lines go after the last run's then, not over them.
            self.file.seek(SeekFrom::End(0))?;
        }
        rolled
    }
}

/// Opens `dir/latest.log` for this run; the last run's lines in it are archived when this run
/// writes ([`Latest::archive_last_run`]). None when another Horizon
/// using the same folder is writing it: that game keeps it, this one logs to stderr only (where
/// files cannot be locked at all, it is opened unlocked). The
/// lock is a separate file (`.lock`): Windows locks are mandatory, and a locked latest.log could
/// not be read while the game runs.
pub fn open_latest(dir: &Path) -> io::Result<Option<Latest>> {
    fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(".lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => return Ok(None),
        // A filesystem without file locks (Android's shared storage, ENOSYS): log unlocked,
        // rather than not at all.
        Err(fs::TryLockError::Error(_)) => {}
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("latest.log"))?;
    let metadata = file.metadata()?;
    // OnStartupTriggeringPolicy's minSize: an empty file is not archived.
    let last_run = if metadata.len() > 0 {
        Some(DateTime::<Local>::from(metadata.modified()?).date_naive())
    } else {
        None
    };
    Ok(Some(Latest {
        file,
        _lock: lock,
        dir: dir.to_owned(),
        last_run,
    }))
}

/// What the writer thread is told.
enum Out {
    /// Lines for the file, dated, and the message for stderr (Horizon's own).
    Write {
        date: NaiveDate,
        lines: Vec<String>,
        stderr: Option<String>,
    },
    /// Answer once everything before is written.
    Flush(mpsc::SyncSender<()>),
}

/// The file side of the log, on its own thread: a line costs the thread that logs it a
/// format and a channel send, never a write that can wait (a full stderr pipe, a slow disk):
/// written on the game thread, a line could hold the tick for milliseconds while the CPU sat
/// idle.
struct Writer {
    file: Option<Latest>,
    dir: PathBuf,
    /// The day the open file's lines belong to.
    date: NaiveDate,
}

impl Writer {
    fn run(mut self, out: mpsc::Receiver<Out>) {
        let stderr = io::stderr();
        for msg in out {
            match msg {
                Out::Write {
                    date,
                    lines,
                    stderr: message,
                } => {
                    if let Some(message) = message {
                        let _ = writeln!(stderr.lock(), "{message}");
                    }
                    if let Some(latest) = &mut self.file
                        && let Err(e) = latest.archive_last_run()
                    {
                        let _ = writeln!(
                            stderr.lock(),
                            "Horizon: could not archive the last run's log: {e}"
                        );
                    }
                    if date != self.date {
                        if let Some(latest) = &mut self.file
                            && let Err(e) = roll(&mut latest.file, &self.dir, self.date, false)
                        {
                            let _ = writeln!(
                                stderr.lock(),
                                "Horizon: could not roll over the log: {e}"
                            );
                        }
                        self.date = date;
                    }
                    if let Some(latest) = &mut self.file {
                        let mut text = lines.join("\n");
                        text.push('\n');
                        let _ = latest.file.write_all(text.as_bytes());
                    }
                }
                Out::Flush(done) => {
                    if let Some(latest) = &mut self.file {
                        let _ = latest.file.flush();
                    }
                    let _ = done.send(());
                }
            }
        }
    }
}

struct Logger {
    out: mpsc::Sender<Out>,
    /// The last lines, kept here (not on the writer) so a crash report has them at once.
    recent: Mutex<VecDeque<String>>,
    /// What the main thread is called in the log: "Render thread", or "Server thread" for the
    /// dedicated server, as in vanilla.
    main_thread: &'static str,
    floods: Mutex<Floods>,
}

/// What the flood limit does with one warning or error.
#[derive(Debug, PartialEq, Eq)]
enum Gate {
    /// Write it, after saying how many lines like it were left out in the window before.
    Write {
        left_out: u32,
    },
    /// Write it, then say the next ones from this line of code wait for the next window.
    Last,
    Skip,
}

/// Warnings and errors per line of code, in windows of [`FLOOD_WINDOW`].
#[derive(Default)]
struct Floods {
    sites: HashMap<(String, u32), Flood>,
}

struct Flood {
    since: Instant,
    written: u32,
    left_out: u32,
}

impl Floods {
    fn gate(&mut self, file: &str, line: u32, now: Instant) -> Gate {
        let flood = self.sites.entry((file.to_owned(), line)).or_insert(Flood {
            since: now,
            written: 0,
            left_out: 0,
        });
        let mut left_out = 0;
        if now.duration_since(flood.since) >= FLOOD_WINDOW {
            left_out = flood.left_out;
            *flood = Flood {
                since: now,
                written: 0,
                left_out: 0,
            };
        }
        flood.written += 1;
        match flood.written.cmp(&FLOOD_LINES) {
            std::cmp::Ordering::Less => Gate::Write { left_out },
            std::cmp::Ordering::Equal => Gate::Last,
            std::cmp::Ordering::Greater => {
                flood.left_out += 1;
                Gate::Skip
            }
        }
    }
}

static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

/// Where messages are printed instead of stderr, which a platform may discard (Android's
/// logcat, set by horizon-android). It gets every message the file gets, and other crates'
/// warnings too (the graphics driver's, wgpu's), since nothing else shows them there.
static CONSOLE: std::sync::OnceLock<fn(log::Level, &str)> = std::sync::OnceLock::new();

/// Prints messages with `console` from now on (before `init`).
pub fn set_console(console: fn(log::Level, &str)) {
    let _ = CONSOLE.set(console);
}

fn ours(target: &str) -> bool {
    target.starts_with("horizon")
}

impl log::Log for Logger {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        meta.level() <= log::Level::Info
            && (ours(meta.target()) || meta.level() <= log::Level::Error)
    }

    fn log(&self, record: &log::Record) {
        let console = CONSOLE.get();
        if !self.enabled(record.metadata()) {
            if let Some(console) = console
                && record.level() <= log::Level::Warn
            {
                console(
                    record.level(),
                    &format!("[{}] {}", record.target(), record.args()),
                );
            }
            return;
        }
        let message = record.args().to_string();
        // A platform console (Android's logcat) gets the message in place of stderr.
        if let Some(console) = console {
            console(record.level(), &message);
        }
        let current = std::thread::current();
        let thread = match current.name() {
            Some("main") => self.main_thread,
            Some(name) => name,
            None => "unnamed",
        };
        let gate = if record.level() <= log::Level::Warn {
            let mut floods = self.floods.lock().unwrap_or_else(|e| e.into_inner());
            floods.gate(
                record.file().unwrap_or(record.target()),
                record.line().unwrap_or(0),
                Instant::now(),
            )
        } else {
            Gate::Write { left_out: 0 }
        };
        let level = record.level();
        match gate {
            Gate::Skip => return,
            Gate::Write { left_out } if left_out > 0 => self.write(
                thread,
                level,
                &format!(
                    "{left_out} more lines like the next one were not logged in the last minute"
                ),
            ),
            _ => {}
        }
        let stderr = (console.is_none() && ours(record.target())).then(|| message.clone());
        self.send(thread, level, &message, stderr);
        if gate == Gate::Last {
            self.write(
                thread,
                level,
                &format!("{FLOOD_LINES} lines like this one in a minute: the next ones are counted, not logged"),
            );
        }
    }

    /// Waits (a second at most) until every line logged before is in the file.
    fn flush(&self) {
        let (done, wait) = mpsc::sync_channel(1);
        if self.out.send(Out::Flush(done)).is_ok() {
            let _ = wait.recv_timeout(std::time::Duration::from_secs(1));
        }
    }
}

impl Logger {
    fn write(&self, thread: &str, level: log::Level, message: &str) {
        self.send(thread, level, message, None);
    }

    fn send(&self, thread: &str, level: log::Level, message: &str, stderr: Option<String>) {
        let now = Local::now();
        // As log4j writes a multi-line message: the pattern once, the rest as it is.
        let lines: Vec<String> = message
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    format_line(&now, thread, level, line)
                } else {
                    line.to_owned()
                }
            })
            .collect();
        {
            let mut recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
            for line in &lines {
                if recent.len() == RECENT {
                    recent.pop_front();
                }
                recent.push_back(line.clone());
            }
        }
        let msg = Out::Write {
            date: now.date_naive(),
            lines,
            stderr,
        };
        if let Err(mpsc::SendError(Out::Write {
            stderr: Some(message),
            ..
        })) = self.out.send(msg)
        {
            // No writer (it could not start): the terminal still gets Horizon's messages.
            eprintln!("{message}");
        }
    }
}

/// Starts the log in `home/logs`. The main thread shows as `main_thread` in it. Messages are
/// still printed when the file cannot be opened.
pub fn init(home: &Path, main_thread: &'static str) {
    let dir = logs_dir(home);
    let file = match open_latest(&dir) {
        Ok(Some(latest)) => Some(latest),
        Ok(None) => {
            eprintln!(
                "Horizon: another game is writing {}; this one logs to the terminal only",
                dir.join("latest.log").display()
            );
            None
        }
        Err(e) => {
            eprintln!(
                "Horizon: could not open {}: {e}",
                dir.join("latest.log").display()
            );
            None
        }
    };
    let logger = LOGGER.get_or_init(|| {
        let (out, receive) = mpsc::channel();
        let writer = Writer {
            file,
            dir,
            date: Local::now().date_naive(),
        };
        if let Err(e) = std::thread::Builder::new()
            .name("Log writer".into())
            .spawn(move || writer.run(receive))
        {
            eprintln!("Horizon: could not start the log writer: {e}");
        }
        Logger {
            out,
            recent: Mutex::new(VecDeque::new()),
            main_thread,
            floods: Mutex::new(Floods::default()),
        }
    });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// A line in the log from code that prints it elsewhere itself (the dedicated server console).
pub fn write_only(level: log::Level, message: &str) {
    if let Some(logger) = LOGGER.get() {
        let current = std::thread::current();
        let thread = match current.name() {
            Some("main") | None => logger.main_thread,
            Some(name) => name,
        };
        logger.write(thread, level, message);
    }
}

/// Writes `message` to the log file only, under the thread vanilla names for it (`Server thread` for
/// a login refused on a connection's own thread), as the console line beside it shows.
pub fn write_as(thread: &str, level: log::Level, message: &str) {
    if let Some(logger) = LOGGER.get() {
        logger.write(thread, level, message);
    }
}

/// The last lines written, oldest first (for crash reports).
pub fn recent_lines() -> Vec<String> {
    LOGGER.get().map_or_else(Vec::new, |l| {
        l.recent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    })
}

/// Flushes the log file (before the process exits).
pub fn flush() {
    if let Some(logger) = LOGGER.get() {
        log::Log::flush(logger);
    }
}

/// The last `lines` lines of a text file.
pub fn tail(path: &Path, lines: usize) -> io::Result<Vec<String>> {
    let text = fs::read(path)?;
    let text = String::from_utf8_lossy(&text);
    let all: Vec<&str> = text.lines().collect();
    Ok(all[all.len().saturating_sub(lines)..]
        .iter()
        .map(|l| l.to_string())
        .collect())
}

/// The newest `crash-*.txt` in a game folder's crash-reports (names sort by time).
pub fn newest_crash_report(home: &Path) -> Option<PathBuf> {
    fs::read_dir(crash_reports_dir(home))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt"))
        })
        .max_by_key(|p| p.file_name().map(|n| n.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_dir_keeps_the_shared_files_in_the_game_folder() {
        let (game, player) = (Path::new("/games/shared"), Path::new("/games/players/alex"));
        // options.txt moved to the player's folder: the enabled mods stay the game folder's.
        assert_eq!(shared_dir_of(Some(player), player, || game.into()), game);
        // No --player-dir, or a splitscreen profile's own folder: beside options.txt as before.
        assert_eq!(shared_dir_of(None, game, || unreachable!()), game);
        let profile = Path::new("/games/shared/splitscreen/profiles/P2");
        assert_eq!(
            shared_dir_of(Some(player), profile, || unreachable!()),
            profile
        );
    }

    #[test]
    fn a_warning_repeated_every_packet_is_counted_past_a_hundred_a_minute() {
        let (mut floods, start) = (Floods::default(), Instant::now());
        let gates: Vec<Gate> = (0..140)
            .map(|i| floods.gate("lan_vehicles.rs", 345, start + Duration::from_millis(i * 7)))
            .collect();
        assert!(
            gates[..99]
                .iter()
                .all(|g| *g == Gate::Write { left_out: 0 })
        );
        assert_eq!(gates[99], Gate::Last);
        assert!(gates[100..].iter().all(|g| *g == Gate::Skip));
        // Another line of code has its own count.
        assert_eq!(
            floods.gate("lan_vehicles.rs", 330, start),
            Gate::Write { left_out: 0 }
        );
        // The next minute writes again, and says how many were left out.
        assert_eq!(
            floods.gate("lan_vehicles.rs", 345, start + FLOOD_WINDOW),
            Gate::Write { left_out: 40 }
        );
        assert_eq!(
            floods.gate("lan_vehicles.rs", 345, start + FLOOD_WINDOW),
            Gate::Write { left_out: 0 }
        );
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    #[test]
    fn archives_are_named_like_vanillas() {
        assert_eq!(archive_name(day(4), 1), "2026-10-04-1.log.gz");
        let none: Vec<String> = Vec::new();
        assert_eq!(
            rollover(&none, day(14), 7),
            Rollover {
                delete: None,
                renames: vec![],
                archive: "2026-10-14-1.log.gz".into()
            }
        );
        // Another day's archives and unrelated files do not count.
        let names: Vec<String> = [
            "2026-10-13-1.log.gz",
            "2026-10-13-2.log.gz",
            "2026-10-14-1.log.gz",
            "2026-10-14-2.log.gz",
            "latest.log",
            "debug-1.log.gz",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(rollover(&names, day(14), 7).archive, "2026-10-14-3.log.gz");
        assert_eq!(rollover(&names, day(13), 7).archive, "2026-10-13-3.log.gz");
        assert_eq!(rollover(&names, day(15), 7).archive, "2026-10-15-1.log.gz");
    }

    #[test]
    fn the_eighth_archive_of_a_day_drops_the_oldest() {
        let names: Vec<String> = (1..=7).map(|i| archive_name(day(14), i)).collect();
        let plan = rollover(&names, day(14), 7);
        assert_eq!(plan.delete.as_deref(), Some("2026-10-14-1.log.gz"));
        assert_eq!(plan.renames.len(), 6);
        assert_eq!(
            plan.renames[0],
            ("2026-10-14-2.log.gz".into(), "2026-10-14-1.log.gz".into())
        );
        assert_eq!(
            plan.renames[5],
            ("2026-10-14-7.log.gz".into(), "2026-10-14-6.log.gz".into())
        );
        assert_eq!(plan.archive, "2026-10-14-7.log.gz");
    }

    #[test]
    fn lines_look_like_log4js_pattern() {
        let time = chrono::TimeZone::with_ymd_and_hms(&Local, 2026, 10, 14, 9, 5, 7).unwrap();
        assert_eq!(
            format_line(
                &time,
                "Render thread",
                log::Level::Info,
                "load assets: 1.20s"
            ),
            "[09:05:07] [Render thread/INFO]: load assets: 1.20s"
        );
        assert_eq!(
            format_line(&time, "Worker-1", log::Level::Warn, "x"),
            "[09:05:07] [Worker-1/WARN]: x"
        );
    }

    fn gunzip(path: &Path) -> String {
        let mut text = String::new();
        flate2::read::GzDecoder::new(File::open(path).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        text
    }

    #[test]
    fn startup_archives_the_last_runs_log() {
        let dir = std::env::temp_dir().join(format!("horizon-logs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Empty: nothing is archived (OnStartupTriggeringPolicy's minSize is 1 byte).
        open_latest(&dir)
            .unwrap()
            .unwrap()
            .archive_last_run()
            .unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        let first = "[10:00:00] [Render thread/INFO]: first run\n";
        fs::write(dir.join("latest.log"), first).unwrap();
        let mut latest = open_latest(&dir).unwrap().unwrap();
        // A second game in the same folder leaves the file alone.
        assert!(open_latest(&dir).unwrap().is_none());
        latest.archive_last_run().unwrap();
        writeln!(latest.file, "second run").unwrap();
        // Once only.
        latest.archive_last_run().unwrap();
        writeln!(latest.file, "more").unwrap();
        drop(latest);
        let today = Local::now().date_naive();
        assert_eq!(gunzip(&dir.join(archive_name(today, 1))), first);
        assert!(!dir.join(archive_name(today, 2)).exists());
        assert_eq!(fs::read_to_string(dir.join(PREVIOUS_LOG)).unwrap(), first);
        assert_eq!(
            fs::read_to_string(dir.join("latest.log")).unwrap(),
            "second run\nmore\n"
        );
        assert_eq!(
            tail(&dir.join("latest.log"), 100).unwrap(),
            vec!["second run", "more"]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_that_writes_nothing_leaves_the_last_runs_log() {
        let dir =
            std::env::temp_dir().join(format!("horizon-logs-quiet-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let killed = "[10:00:00] [Render thread/INFO]: the run that was ended\n";
        fs::write(dir.join("latest.log"), killed).unwrap();
        // Android starts the app again right after ending it, and that run ends before a line.
        drop(open_latest(&dir).unwrap().unwrap());
        assert_eq!(fs::read_to_string(dir.join("latest.log")).unwrap(), killed);
        assert!(!dir.join(PREVIOUS_LOG).exists());
        // The run after that writes: the ended run's log is archived once, and kept plain.
        let mut latest = open_latest(&dir).unwrap().unwrap();
        latest.archive_last_run().unwrap();
        writeln!(latest.file, "next run").unwrap();
        drop(latest);
        let today = Local::now().date_naive();
        assert_eq!(gunzip(&dir.join(archive_name(today, 1))), killed);
        assert!(!dir.join(archive_name(today, 2)).exists());
        assert_eq!(fs::read_to_string(dir.join(PREVIOUS_LOG)).unwrap(), killed);
        // A rollover in the run (the day changing) keeps previous.log as it is.
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.join("latest.log"))
            .unwrap();
        file.seek(SeekFrom::End(0)).unwrap();
        roll(&mut file, &dir, today, false).unwrap();
        assert_eq!(gunzip(&dir.join(archive_name(today, 2))), "next run\n");
        assert_eq!(fs::read_to_string(dir.join(PREVIOUS_LOG)).unwrap(), killed);
        assert_eq!(fs::read_to_string(dir.join("latest.log")).unwrap(), "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_newest_crash_report_is_found_by_name() {
        let home = std::env::temp_dir().join(format!("horizon-crash-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        assert!(newest_crash_report(&home).is_none());
        let dir = crash_reports_dir(&home);
        fs::create_dir_all(&dir).unwrap();
        for name in [
            "crash-2026-10-13_23.59.59-client.txt",
            "crash-2026-10-14_08.00.01-client.txt",
            "notes.txt",
        ] {
            fs::write(dir.join(name), "x").unwrap();
        }
        assert_eq!(
            newest_crash_report(&home).unwrap(),
            dir.join("crash-2026-10-14_08.00.01-client.txt")
        );
        let _ = fs::remove_dir_all(&home);
    }
}
