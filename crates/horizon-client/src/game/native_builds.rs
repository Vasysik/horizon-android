//! Android: native mods (docs/starling-native-mods.md). A native mod is compiled into the game, so
//! playing with one is playing another build of this version's game library, made by GitHub
//! (native-mods.yml) and published as assets of the native-builds release, the organisation's
//! repository's and Starling's own both asked (starling_releases). This card lists the
//! builds for this version, downloads one (its SHA-256 checked against the build's JSON), and
//! chooses it for the next start: StarlingActivity loads it in place of the APK's library
//! (NativeMods.java). It also stops using one, asks GitHub for another set (an issue the player
//! submits), and says when the activity set a build aside.
use super::Game;
use horizon_client::{
    hud::HudBatch,
    starling_releases::{self as releases, NativeBuild as Build},
    ui_theme::{Face, Ui, color},
};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use winit::event::{Touch, TouchPhase};

/// The mod build service (packaging/android/mod_build_server.py).
const BUILD_SERVER: &str = "https://191-234-193-205.sslip.io";
/// A build's folder here starts with this, and a new one takes the place of the last.
const COMPILED: &str = "compiled-";
/// The service's waits: between looks at the job, and how long it may go on at all.
const POLL: Duration = Duration::from_secs(2);
const LONGEST: Duration = Duration::from_secs(40 * 60);
const WAKING: Duration = Duration::from_secs(5 * 60);
/// Tries in a row that brought nothing before a look or a download gives up.
const QUIET_TRIES: u32 = 15;

/// The library's name in a build's folder, as NativeMods.java loads it.
const LIBRARY: &str = "libhorizon_android.so";
/// Rows of builds the card shows.
const SHOWN: usize = 4;
/// Options > Mods' row that opens the card, after the native mods' own rows: a native mod's page
/// with one button. (Not in native_hooks.rs, which keeps no `cfg`.)
const ID: &str = "starling-native-builds";

#[derive(Clone, PartialEq)]
enum Stage {
    Loading,
    List,
    Downloading(String),
    /// Chosen: it plays from the next start.
    Chosen(String),
    Failed(String),
    /// What compiling mod `.0` means, before it starts.
    Warn(String),
    /// On the build service: the water to fish in while it takes.
    Compiling,
}

#[derive(Default)]
struct Shared {
    builds: Option<Result<Vec<Build>, String>>,
    done: u64,
    total: u64,
    downloaded: Option<Result<String, String>>,
    /// A compile is under way (its card may be hidden), and what it says.
    compiling: bool,
    status: String,
    /// When the service began compiling: the card counts from it, each frame.
    building_since: Option<Instant>,
}

#[derive(Default)]
pub(super) struct NativeBuilds {
    stage: Option<Stage>,
    shared: Arc<Mutex<Shared>>,
    /// The buttons as last drawn (UI units), with what each does.
    buttons: std::cell::RefCell<Vec<([f32; 4], Act)>>,
    /// The activity's notice was shown this run.
    noticed: Cell<bool>,
    fishing: super::setup::fishing::Fishing,
    mining: super::build_mining::BuildMining,
    /// Under a build: digging, not fishing.
    digging: bool,
}

#[derive(Clone, PartialEq)]
enum Act {
    Use(Build),
    StopUsing,
    Restart,
    Retry,
    Close,
    Compile,
    /// Under a build: from the water to the rock and back.
    Switch,
}

fn dir() -> PathBuf {
    horizon_client::logging::private_home().join("native")
}

/// The build chosen for the next start (files/native/current), if any.
fn chosen() -> Option<String> {
    let name = std::fs::read_to_string(dir().join("current")).ok()?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// The build this game runs, when it is one (StarlingActivity names it).
fn running() -> Option<String> {
    let path = PathBuf::from(std::env::var_os("STARLING_GAME_LIBRARY")?);
    Some(path.parent()?.file_name()?.to_string_lossy().into_owned())
}

/// `v0.1.4__wrench__carryblock`'s mods as the card says them: "Wrench, Carryblock".
fn mods_label(mods: &[String]) -> String {
    let word = |m: &String| {
        let mut c = m.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect::<String>())
            .unwrap_or_default()
    };
    mods.iter().map(word).collect::<Vec<_>>().join(", ")
}

/// A name of the release's assets: lowercase letters, digits, `.`, `_` and `-`.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

fn get(url: &str, version: &str) -> Result<ureq::Response, String> {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build()
        .get(url)
        .set("User-Agent", &format!("Starling/{version}"))
        .call()
        .map_err(|e| e.to_string())
}

/// The builds made for `version`, in every repository's native-builds release (a repository
/// without one has none).
fn list(version: &str) -> Result<Vec<Build>, String> {
    let mut lists = Vec::new();
    let mut errors = Vec::new();
    for repository in releases::REPOSITORIES {
        let release = get(&releases::native_builds_url(repository), version)
            .and_then(|r| r.into_string().map_err(|e| e.to_string()))
            .and_then(|text| {
                serde_json::from_str::<serde_json::Value>(&text).map_err(|e| e.to_string())
            });
        match release {
            Ok(release) => lists.push(releases::native_builds_in(&release, version)),
            Err(e) if e.contains("404") => {}
            Err(e) => errors.push(e),
        }
    }
    if lists.is_empty() {
        return Err(errors
            .into_iter()
            .next()
            .unwrap_or_else(|| "No native mod builds have been made yet.".to_owned()));
    }
    Ok(releases::merge_native_builds(lists))
}

/// The build's library into files/native/NAME/, checked against its JSON, and chosen for the next
/// start.
fn download(build: &Build, version: &str, shared: &Mutex<Shared>) -> Result<String, String> {
    let text = get(&build.json, version)?
        .into_string()
        .map_err(|e| e.to_string())?;
    let about: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if about["version"].as_str() != Some(version) {
        return Err(format!("That build is not for Starling {version}."));
    }
    let want = about["sha256"].as_str().unwrap_or_default().to_owned();
    if want.len() != 64 {
        return Err("That build does not say its SHA-256.".into());
    }
    let folder = dir().join(&build.name);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let part = folder.join(format!("{LIBRARY}.part"));
    let response = get(&build.library, version)?;
    let total = response
        .header("Content-Length")
        .and_then(|l| l.parse().ok())
        .or_else(|| about["size"].as_u64())
        .unwrap_or(0);
    shared.lock().unwrap_or_else(PoisonError::into_inner).total = total;
    let mut body = response.into_reader();
    let mut file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 256 * 1024];
    let mut done = 0;
    loop {
        let n = body.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        file.write_all(&buffer[..n])
            .map_err(|e| format!("{}: {e}", part.display()))?;
        done += n as u64;
        shared.lock().unwrap_or_else(PoisonError::into_inner).done = done;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != want {
        let _ = std::fs::remove_file(&part);
        return Err("The download came out damaged. Try again.".into());
    }
    // A rename: a library this game runs stays mapped as it was.
    std::fs::rename(&part, folder.join(LIBRARY)).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("for"), version).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("build.json"), &text).map_err(|e| e.to_string())?;
    std::fs::write(dir().join("current"), &build.name).map_err(|e| e.to_string())?;
    Ok(build.name.clone())
}

/// The UI's scale for a window of `w` x `h` pixels (as `Ui::new`).
fn density(w: f32, h: f32) -> f32 {
    (w / 1040.0).min(h / 700.0).clamp(1.0, 4.0)
}

/// What the build this game runs says of itself (its `build.json`: the mods in it).
pub(super) fn running_build_info() -> Option<String> {
    std::fs::read_to_string(dir().join(running()?).join("build.json")).ok()
}

/// The `horizon-native-mods` folder's mods, each a length and its file, for the service.
fn bundles_body() -> Result<Vec<u8>, String> {
    let folder = Game::game_dir().join(horizon_client::mod_list::NATIVE_DIR);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&folder)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.retain(|p| p.extension().is_some_and(|e| e == "hnmod"));
    paths.sort();
    let mut body = Vec::new();
    for path in paths {
        if horizon_mods::index::native::Bundle::read(&path).is_err() {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        body.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        body.extend_from_slice(&bytes);
    }
    if body.is_empty() {
        return Err("There is no native mod to compile.".into());
    }
    Ok(body)
}

/// What a refused request says (the service's JSON `error`), or that it can't be reached.
fn service_error(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, response) => response
            .into_string()
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("The build server answered {code}.")),
        ureq::Error::Transport(_) => {
            "The build server can't be reached right now. Try again later.".into()
        }
    }
}

/// The library of job `job` into `part`, going on from what is there, checked by `want`.
fn fetch_library(
    agent: &ureq::Agent,
    url: &str,
    part: &std::path::Path,
    size: u64,
    want: &str,
    shared: &Mutex<Shared>,
) -> Result<(), String> {
    let on_disk = |p: &std::path::Path| std::fs::metadata(p).map_or(0, |m| m.len());
    let mut quiet = 0;
    while on_disk(part) < size {
        let have = on_disk(part);
        let tried = (|| -> Result<(), String> {
            let response = agent
                .get(url)
                .set("Range", &format!("bytes={have}-"))
                .call()
                .map_err(service_error)?;
            let resumed = response.status() == 206;
            let mut file = if resumed {
                std::fs::OpenOptions::new().append(true).open(part)
            } else {
                std::fs::File::create(part)
            }
            .map_err(|e| format!("{}: {e}", part.display()))?;
            let mut done = if resumed { have } else { 0 };
            let mut body = response.into_reader();
            let mut buffer = vec![0; 256 * 1024];
            loop {
                let n = body.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    return Ok(());
                }
                file.write_all(&buffer[..n])
                    .map_err(|e| format!("{}: {e}", part.display()))?;
                done += n as u64;
                shared.lock().unwrap_or_else(PoisonError::into_inner).status = format!(
                    "Downloading: {:.0} of {:.0} MB",
                    done as f64 / 1e6,
                    size as f64 / 1e6
                );
            }
        })();
        quiet = if on_disk(part) > have { 0 } else { quiet + 1 };
        if let Err(e) = tried {
            log::warn!("Horizon: the compiled game stopped downloading ({e}); trying again");
            if quiet >= QUIET_TRIES {
                return Err(e);
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    }
    let mut hash = Sha256::new();
    let mut file = std::fs::File::open(part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != want {
        let _ = std::fs::remove_file(part);
        return Err("The download came out damaged. Try again.".into());
    }
    Ok(())
}

/// This version's game compiled with the folder's mods on the build service, downloaded and chosen
/// for the next start: the folder's name.
fn compile(version: &str, shared: &Mutex<Shared>) -> Result<String, String> {
    let say = |text: String| {
        let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
        s.status = text;
        s.building_since = None;
    };
    let server = std::env::var("HORIZON_BUILD_SERVER").unwrap_or_else(|_| BUILD_SERVER.into());
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(40))
        .build();
    say("Sending the mod to the build server...".into());
    let body = bundles_body()?;
    let started = Instant::now();
    // The machine that builds sleeps until it is asked: the gateway says so while it wakes.
    let job = loop {
        let answer: serde_json::Value = agent
            .post(&format!("{server}/v1/build?version={version}"))
            .set("Content-Type", "application/octet-stream")
            .send_bytes(&body)
            .map_err(service_error)?
            .into_json()
            .map_err(|e| e.to_string())?;
        if answer["starting"].as_bool() != Some(true) {
            break answer["job"].as_str().unwrap_or_default().to_owned();
        }
        if started.elapsed() > WAKING {
            return Err("The build server did not wake up. Try again later.".into());
        }
        say("Waking the build server up: about a minute...".into());
        std::thread::sleep(POLL * 3);
    };
    if !plain(&job) {
        return Err("The build server gave no job.".into());
    }
    let mut quiet = 0;
    let finished = loop {
        if started.elapsed() > LONGEST {
            return Err("The build took too long. Try again later.".into());
        }
        std::thread::sleep(POLL);
        let looked = agent
            .get(&format!("{server}/v1/job/{job}"))
            .call()
            .map_err(service_error)
            .and_then(|r| {
                r.into_json::<serde_json::Value>()
                    .map_err(|e| e.to_string())
            });
        let status = match looked {
            Ok(status) => status,
            Err(e) => {
                quiet += 1;
                if quiet >= QUIET_TRIES {
                    return Err(e);
                }
                continue;
            }
        };
        quiet = 0;
        match status["state"].as_str() {
            Some("starting") => say("Waking the build server up: about a minute...".into()),
            Some("queued") => say(format!(
                "In line: {} before yours",
                status["ahead"].as_u64().unwrap_or(0)
            )),
            Some("building") => {
                let seconds = status["seconds"].as_u64().unwrap_or(0);
                let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
                if s.building_since.is_none() {
                    s.building_since = Instant::now().checked_sub(Duration::from_secs(seconds));
                }
            }
            Some("done") => break status,
            Some("failed") => {
                return Err(status["error"]
                    .as_str()
                    .unwrap_or("The build failed.")
                    .to_owned());
            }
            _ => return Err("The build server lost the job. Try again.".into()),
        }
    };
    let want = finished["sha256"].as_str().unwrap_or_default().to_owned();
    let size = finished["size"].as_u64().unwrap_or(0);
    if want.len() != 64 || size == 0 {
        return Err("The build does not say its SHA-256.".into());
    }
    say("Downloading...".into());
    let name = format!("{COMPILED}{job}");
    let folder = dir().join(&name);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let part = folder.join(format!("{LIBRARY}.part"));
    let url = format!("{server}/v1/job/{job}/lib");
    fetch_library(&agent, &url, &part, size, &want, shared)?;
    // A rename: a library this game runs stays mapped as it was.
    std::fs::rename(&part, folder.join(LIBRARY)).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("for"), version).map_err(|e| e.to_string())?;
    std::fs::write(
        folder.join("build.json"),
        finished["build"].to_string().as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(dir().join("current"), &name).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(dir().join("starts"));
    // The compiled builds before it: 60 MB each.
    for entry in std::fs::read_dir(dir()).into_iter().flatten().flatten() {
        let other = entry.file_name().to_string_lossy().into_owned();
        if other.starts_with(COMPILED) && other != name {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(name)
}

impl Game<'_> {
    /// The Mods page's mods not in this game get a Compile button (`native_builds_button`).
    pub(super) fn offer_compile(&mut self) {
        for row in &mut self.settings.mods.installed {
            if row.problem.as_deref() == Some(horizon_client::mod_list::NOT_BUILT) {
                row.native = Some(horizon_client::mod_list::NativePage {
                    keys: Vec::new(),
                    buttons: vec!["Compile".into()],
                });
            }
        }
    }

    /// Options > Mods' Native mods row, put back after `list_native_mods` made the native rows.
    pub(super) fn list_native_builds_row(&mut self) {
        let installed = &mut self.settings.mods.installed;
        installed.retain(|row| row.id != ID);
        installed.push(horizon_client::mod_list::Installed {
            id: ID.into(),
            title: "Native mods".into(),
            description: "This game with native mods compiled in: choose a build, or play \
                          without."
                .into(),
            problem: None,
            dev: false,
            native: Some(horizon_client::mod_list::NativePage {
                keys: Vec::new(),
                buttons: vec!["Choose a build...".into()],
            }),
        });
    }

    /// A button of the native mod page `i` of Options > Mods: true when it was the Native mods
    /// row's, which opens the card.
    pub(super) fn native_builds_button(&mut self, i: usize) -> bool {
        let Some(row) = self.settings.mods.installed.get(i) else {
            return false;
        };
        if row.id == ID {
            self.open_native_builds();
            return true;
        }
        if row.problem.as_deref() == Some(horizon_client::mod_list::NOT_BUILT) {
            let id = row.id.clone();
            let card = &mut self.native_builds;
            // One under way (its card hidden) comes back; its warning was read.
            let compiling = card
                .shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .compiling;
            card.stage = Some(if compiling {
                Stage::Compiling
            } else {
                Stage::Warn(id)
            });
            return true;
        }
        false
    }

    /// Shows the card: the builds for this version, looked up now.
    pub(super) fn open_native_builds(&mut self) {
        let Some((version, _)) = crate::android::app_version() else {
            return;
        };
        let card = &mut self.native_builds;
        card.stage = Some(Stage::Loading);
        let shared = card.shared.clone();
        shared.lock().unwrap_or_else(PoisonError::into_inner).builds = None;
        let _ = std::thread::Builder::new()
            .name("native builds".into())
            .spawn(move || {
                let builds = list(&version);
                if let Err(e) = &builds {
                    log::info!("Horizon: no native mod builds listed: {e}");
                }
                shared.lock().unwrap_or_else(PoisonError::into_inner).builds = Some(builds);
            });
    }

    /// The activity's notice, once: a build it set aside (NativeMods.java), on the title screen.
    pub(super) fn poll_native_builds(&mut self) {
        if self.native_builds.stage == Some(Stage::Compiling) {
            self.native_builds.fishing.update(Instant::now());
            let [w, h] = self.touch_size();
            let d = density(w as f32, h as f32);
            self.native_builds
                .mining
                .prepare(w as f32 / d, h as f32 / d);
            let mut sounds = self.native_builds.fishing.take_sounds();
            sounds.extend(self.native_builds.mining.take_sounds());
            for sound in sounds {
                self.audio
                    .play_samples(sound.samples(), super::setup::fishing::RATE, 0.3);
            }
        }
        let card = &mut self.native_builds;
        if !card.noticed.get() && self.title.is_some() && card.stage.is_none() {
            card.noticed.set(true);
            let notice = dir().join("notice");
            if let Ok(text) = std::fs::read_to_string(&notice) {
                let _ = std::fs::remove_file(&notice);
                card.stage = Some(Stage::Failed(text.trim().to_owned()));
            }
        }
        let mut shared = card.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if card.stage == Some(Stage::Loading)
            && let Some(builds) = &shared.builds
        {
            card.stage = Some(match builds {
                Ok(_) => Stage::List,
                Err(e) => Stage::Failed(e.clone()),
            });
        }
        if (matches!(card.stage, Some(Stage::Downloading(_))) || shared.compiling)
            && let Some(result) = shared.downloaded.take()
        {
            shared.compiling = false;
            card.stage = Some(match result {
                Ok(name) => Stage::Chosen(name),
                Err(e) => Stage::Failed(e),
            });
        }
    }

    pub(super) fn native_builds_shown(&self) -> bool {
        self.native_builds.stage.is_some()
    }

    /// A card in the middle of the screen, the rest dimmed, as the update's.
    pub(super) fn draw_native_builds(&self, batch: &mut HudBatch) {
        let Some(stage) = self.native_builds.stage.clone() else {
            return;
        };
        if stage == Stage::Compiling {
            self.draw_compiling(batch);
            return;
        }
        let (builds, done, total) = {
            let shared = self
                .native_builds
                .shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let builds = match &shared.builds {
                Some(Ok(b)) => b.clone(),
                _ => Vec::new(),
            };
            (builds, shared.done, shared.total)
        };
        let label = |name: &str| {
            builds.iter().find(|b| b.name == name).map_or_else(
                || {
                    if name.starts_with(COMPILED) {
                        "your native mods".to_owned()
                    } else {
                        name.to_owned()
                    }
                },
                |b| mods_label(&b.mods),
            )
        };
        let current = chosen();
        let now = running();
        let mut acts: Vec<(String, Act)> = Vec::new();
        let (title, body) = match &stage {
            Stage::Loading => ("Native mods".to_owned(), "Looking for builds...".to_owned()),
            Stage::List => {
                for b in builds.iter().take(SHOWN) {
                    let text = if current.as_deref() == Some(&b.name) {
                        format!("{} (chosen)", mods_label(&b.mods))
                    } else {
                        format!("Play with {}", mods_label(&b.mods))
                    };
                    acts.push((text, Act::Use(b.clone())));
                }
                if current.is_some() {
                    acts.push(("Play without native mods".into(), Act::StopUsing));
                }
                let playing = match now {
                    Some(n) => format!("This game has {}. ", label(&n)),
                    None => String::new(),
                };
                let found = if builds.is_empty() {
                    "No builds for this version yet."
                } else {
                    "Each is this version's game with those mods compiled in."
                };
                (
                    "Native mods".to_owned(),
                    format!(
                        "{playing}{found} Native mods run with the game's full access. To add \
                         one, open its .hnmod with Starling and press Compile on its row."
                    ),
                )
            }
            Stage::Downloading(name) => (
                format!("Downloading {}", label(name)),
                format!(
                    "{:.0} of {:.0} MB",
                    done as f64 / 1e6,
                    total.max(done) as f64 / 1e6
                ),
            ),
            Stage::Chosen(name) => {
                acts.push(("Restart now".into(), Act::Restart));
                (
                    if name.starts_with(COMPILED) {
                        "Your native mods are ready".to_owned()
                    } else {
                        format!("{} is ready", label(name))
                    },
                    "It plays from the next start. If it closes before it shows, Starling starts \
                     without it the time after."
                        .into(),
                )
            }
            Stage::Failed(e) => {
                acts.push(("Try again".into(), Act::Retry));
                ("Native mods".to_owned(), e.clone())
            }
            Stage::Warn(id) => {
                acts.push(("Compile".into(), Act::Compile));
                (
                    format!("Compile {id}?"),
                    "A native mod runs with the game's full access: it can do anything the game \
                     can on this phone, so only compile mods you trust. It is sent to Starling's \
                     build server, which compiles the game with your native mods in a few \
                     minutes; then the game restarts with them. If it closes before it shows, \
                     Starling starts without it the time after."
                        .to_owned(),
                )
            }
            Stage::Compiling => unreachable!("drawn by draw_compiling"),
        };
        acts.push(("Close".into(), Act::Close));
        let mut ui = Ui::new(&self.gui.atlas, batch, (-1.0, -1.0));
        let (w, h) = (ui.w, ui.h);
        ui.fill(0.0, 0.0, w, h, color::with_alpha(color::BG, 0.72));
        let card_w = (w - 48.0).min(470.0);
        let pad = 20.0;
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in body.split(' ') {
            let next = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && ui.measure(Face::Regular, 13.0, &next) > card_w - 2.0 * pad {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = next;
            }
        }
        lines.push(line);
        let bar = matches!(stage, Stage::Downloading(_));
        let (button_h, gap) = (34.0, 8.0);
        let card_h = pad
            + 24.0
            + 10.0
            + lines.len() as f32 * 19.0
            + if bar { 20.0 } else { 0.0 }
            + 14.0
            + acts.len() as f32 * (button_h + gap)
            - gap
            + pad;
        let (x, y) = ((w - card_w) / 2.0, ((h - card_h) / 2.0).max(4.0));
        ui.rounded(x, y, card_w, card_h, 14.0, color::BG);
        ui.border(x, y, card_w, card_h, 14.0, color::BORDER);
        ui.text(Face::Semibold, 17.0, x + pad, y + pad, color::TEXT, &title);
        let mut ty = y + pad + 24.0 + 10.0;
        for line in &lines {
            ui.text(Face::Regular, 13.0, x + pad, ty, color::MUTED, line);
            ty += 19.0;
        }
        if bar {
            let bw = card_w - 2.0 * pad;
            let f = if total > 0 {
                (done as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            ui.rounded(x + pad, ty + 6.0, bw, 8.0, 4.0, color::RAISED);
            ui.rounded(
                x + pad,
                ty + 6.0,
                (bw * f).max(8.0),
                8.0,
                4.0,
                color::ACCENT,
            );
            ty += 20.0;
        }
        let mut by = ty + 14.0;
        let mut drawn = Vec::new();
        for (i, (text, act)) in acts.into_iter().enumerate() {
            let rect = [x + pad, by, card_w - 2.0 * pad, button_h];
            let (fill, ink) = if i == 0 && act != Act::Close {
                (color::ACCENT, color::ON_ACCENT)
            } else {
                (color::RAISED, color::TEXT)
            };
            ui.rounded(rect[0], rect[1], rect[2], rect[3], 10.0, fill);
            ui.text_centered(
                Face::Semibold,
                14.0,
                rect[0] + rect[2] / 2.0,
                rect[1] + (rect[3] - 14.0) / 2.0 - 2.0,
                ink,
                &text,
            );
            drawn.push((rect, act));
            by += button_h + gap;
        }
        ui.finish();
        *self.native_builds.buttons.borrow_mut() = drawn;
    }

    /// The water over the screen while the build goes on: what it says, and a way out of it.
    fn draw_compiling(&self, batch: &mut HudBatch) {
        let status = {
            let s = self
                .native_builds
                .shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match s.building_since {
                Some(since) => {
                    let seconds = since.elapsed().as_secs();
                    format!("Compiling: {}:{:02}", seconds / 60, seconds % 60)
                }
                None => s.status.clone(),
            }
        };
        let now = Instant::now();
        let card = &self.native_builds;
        let mut ui = Ui::new(&self.gui.atlas, batch, (-1.0, -1.0));
        let (w, h) = (ui.w, ui.h);
        if card.digging {
            card.mining.draw(&mut ui, w, h, now);
        } else {
            card.fishing.draw_water(&mut ui, w, h, now);
            card.fishing.draw(&mut ui, w, h, h - 58.0, now);
        }
        let (pad, bar_h, hide_w) = (16.0, 44.0, 84.0);
        ui.rounded(pad, pad, w - 2.0 * pad, bar_h, 12.0, color::BG);
        ui.text(
            Face::Regular,
            14.0,
            pad + 14.0,
            pad + 13.0,
            color::TEXT,
            &status,
        );
        let rect = [w - pad - hide_w - 6.0, pad + 6.0, hide_w, bar_h - 12.0];
        ui.rounded(rect[0], rect[1], rect[2], rect[3], 8.0, color::RAISED);
        ui.text_centered(
            Face::Semibold,
            13.0,
            rect[0] + rect[2] / 2.0,
            rect[1] + (rect[3] - 13.0) / 2.0 - 2.0,
            color::TEXT,
            "Hide",
        );
        let other = [rect[0] - hide_w - 8.0, rect[1], hide_w, rect[3]];
        ui.rounded(other[0], other[1], other[2], other[3], 8.0, color::RAISED);
        ui.text_centered(
            Face::Semibold,
            13.0,
            other[0] + other[2] / 2.0,
            other[1] + (other[3] - 13.0) / 2.0 - 2.0,
            color::TEXT,
            if card.digging { "Fish" } else { "Mine" },
        );
        ui.finish();
        *self.native_builds.buttons.borrow_mut() = vec![(rect, Act::Close), (other, Act::Switch)];
    }

    /// While the card shows it takes every touch; a lifted finger on a button acts. Under a build
    /// a finger anywhere else fishes.
    pub(super) fn native_builds_touch(&mut self, touch: &Touch) -> bool {
        if !self.native_builds_shown() {
            return false;
        }
        let [w, h] = self.touch_size();
        let d = density(w as f32, h as f32);
        let at = [touch.location.x as f32 / d, touch.location.y as f32 / d];
        if self.native_builds.stage == Some(Stage::Compiling) {
            let on_button = self.native_builds.buttons.borrow().iter().any(|(r, _)| {
                at[0] >= r[0] && at[0] <= r[0] + r[2] && at[1] >= r[1] && at[1] <= r[1] + r[3]
            });
            let now = Instant::now();
            let digging = self.native_builds.digging;
            match touch.phase {
                TouchPhase::Started if !on_button && digging => {
                    self.native_builds
                        .mining
                        .press(at, w as f32 / d, h as f32 / d);
                }
                TouchPhase::Started if !on_button => self.native_builds.fishing.press(now, at),
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if !digging {
                        self.native_builds.fishing.release(now);
                    }
                    if on_button {
                        self.native_builds_press(at);
                    }
                }
                _ => {}
            }
            return true;
        }
        if touch.phase != TouchPhase::Ended {
            return true;
        }
        self.native_builds_press(at);
        true
    }

    /// A press at `p` (UI units) on the card's buttons.
    pub(super) fn native_builds_press(&mut self, p: [f32; 2]) {
        let inside = |r: &[f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };
        let act = self
            .native_builds
            .buttons
            .borrow()
            .iter()
            .find(|(r, _)| inside(r))
            .map(|(_, a)| a.clone());
        let Some(act) = act else {
            return;
        };
        let Some((version, _)) = crate::android::app_version() else {
            return;
        };
        match act {
            Act::Use(build) => {
                let card = &mut self.native_builds;
                card.stage = Some(Stage::Downloading(build.name.clone()));
                let shared = card.shared.clone();
                {
                    let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    s.done = 0;
                    s.total = 0;
                    s.downloaded = None;
                }
                let _ = std::thread::Builder::new()
                    .name("native build download".into())
                    .spawn(move || {
                        let result = download(&build, &version, &shared);
                        if let Err(e) = &result {
                            log::warn!("Horizon: the native mod build did not download: {e}");
                        }
                        shared
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .downloaded = Some(result);
                    });
            }
            Act::StopUsing => {
                let _ = std::fs::remove_file(dir().join("current"));
                self.native_builds.stage = Some(Stage::List);
            }
            Act::Restart => crate::android::restart_app(),
            Act::Retry => self.open_native_builds(),
            Act::Close => self.native_builds.stage = None,
            Act::Switch => self.native_builds.digging = !self.native_builds.digging,
            Act::Compile => {
                let card = &mut self.native_builds;
                card.stage = Some(Stage::Compiling);
                let shared = card.shared.clone();
                {
                    let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    if s.compiling {
                        return;
                    }
                    s.compiling = true;
                    s.downloaded = None;
                    s.status = String::new();
                }
                let _ = std::thread::Builder::new()
                    .name("native mod compile".into())
                    .spawn(move || {
                        let result = compile(&version, &shared);
                        if let Err(e) = &result {
                            log::warn!("Horizon: the native mods did not compile: {e}");
                        }
                        shared
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .downloaded = Some(result);
                    });
            }
        }
    }
}
