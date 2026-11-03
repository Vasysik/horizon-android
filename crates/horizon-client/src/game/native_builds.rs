//! Android: native mods (docs/starling-native-mods.md). A native mod is compiled into the game, so
//! playing with one is playing another build of this version's game library, made by GitHub
//! (native-mods.yml) and published as assets of the native-builds release. This card lists the
//! builds for this version, downloads one (its SHA-256 checked against the build's JSON), and
//! chooses it for the next start: StarlingActivity loads it in place of the APK's library
//! (NativeMods.java). It also stops using one, asks GitHub for another set (an issue the player
//! submits), and says when the activity set a build aside.
use super::Game;
use horizon_client::{
    hud::HudBatch,
    ui_theme::{Face, Ui, color},
};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use winit::event::{Touch, TouchPhase};

const RELEASE: &str = "https://api.github.com/repos/JuanLuci/starling/releases/tags/native-builds";
/// The issue form that asks for a build (.github/ISSUE_TEMPLATE/native-build.yml), its version
/// filled in.
const REQUEST: &str = "https://github.com/JuanLuci/starling/issues/new?template=native-build.yml";
/// The library's name in a build's folder, as NativeMods.java loads it.
const LIBRARY: &str = "libhorizon_android.so";
/// Rows of builds the card shows.
const SHOWN: usize = 4;
/// Options > Mods' row that opens the card, after the native mods' own rows: a native mod's page
/// with one button. (Not in native_hooks.rs, which keeps no `cfg`.)
const ID: &str = "starling-native-builds";

/// A build for this version in the native-builds release.
#[derive(Clone, PartialEq)]
struct Build {
    /// `v0.1.4__wrench`: its assets' names, and its folder here.
    name: String,
    mods: Vec<String>,
    json: String,
    library: String,
}

#[derive(Clone, PartialEq)]
enum Stage {
    Loading,
    List,
    Downloading(String),
    /// Chosen: it plays from the next start.
    Chosen(String),
    Failed(String),
}

#[derive(Default)]
struct Shared {
    builds: Option<Result<Vec<Build>, String>>,
    done: u64,
    total: u64,
    downloaded: Option<Result<String, String>>,
}

#[derive(Default)]
pub(super) struct NativeBuilds {
    stage: Option<Stage>,
    shared: Arc<Mutex<Shared>>,
    /// The buttons as last drawn (UI units), with what each does.
    buttons: std::cell::RefCell<Vec<([f32; 4], Act)>>,
    /// The activity's notice was shown this run.
    noticed: Cell<bool>,
}

#[derive(Clone, PartialEq)]
enum Act {
    Use(Build),
    StopUsing,
    Request,
    Restart,
    Retry,
    Close,
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

/// The builds made for `version`.
fn list(version: &str) -> Result<Vec<Build>, String> {
    let text = get(RELEASE, version)
        .map_err(|e| {
            if e.contains("404") {
                "No native mod builds have been made yet.".to_owned()
            } else {
                e
            }
        })?
        .into_string()
        .map_err(|e| e.to_string())?;
    let release: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let url = |name: &str| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(name))
            .and_then(|a| a["browser_download_url"].as_str())
            .filter(|u| u.starts_with("https://"))
            .map(str::to_owned)
    };
    let prefix = format!("v{version}__");
    let mut builds: Vec<Build> = assets
        .iter()
        .filter_map(|a| a["name"].as_str())
        .filter_map(|n| n.strip_suffix(".json"))
        .filter(|n| n.starts_with(&prefix) && plain(n))
        .filter_map(|name| {
            Some(Build {
                name: name.to_owned(),
                mods: name[prefix.len()..]
                    .split("__")
                    .map(str::to_owned)
                    .collect(),
                json: url(&format!("{name}.json"))?,
                library: url(&format!("{name}.so"))?,
            })
        })
        .collect();
    builds.sort_by(|a, b| (a.mods.len(), &a.name).cmp(&(b.mods.len(), &b.name)));
    Ok(builds)
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

impl Game<'_> {
    /// Options > Mods' Native mods row, put back after `list_native_mods` made the native rows.
    pub(super) fn list_native_builds_row(&mut self) {
        let installed = &mut self.settings.mods.installed;
        installed.retain(|row| row.id != ID);
        installed.push(horizon_client::mod_list::Installed {
            id: ID.into(),
            title: "Native mods".into(),
            description: "This game with native mods compiled in, built by GitHub: choose a \
                          build, or play without."
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
        let ours = self
            .settings
            .mods
            .installed
            .get(i)
            .is_some_and(|row| row.id == ID);
        if ours {
            self.open_native_builds();
        }
        ours
    }

    /// Shows the card: the builds for this version, looked up now.
    pub(super) fn open_native_builds(&mut self) {
        let Some((version, _)) = crate::app_version() else {
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
        if matches!(card.stage, Some(Stage::Downloading(_)))
            && let Some(result) = shared.downloaded.take()
        {
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
            builds
                .iter()
                .find(|b| b.name == name)
                .map_or_else(|| name.to_owned(), |b| mods_label(&b.mods))
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
                acts.push(("Ask for another set".into(), Act::Request));
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
                        "{playing}{found} Native mods run with the game's full access, unlike \
                         Starling mods: only those in Starling's native-mods folder are built, \
                         each read before it was added."
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
                    format!("{} is ready", label(name)),
                    "It plays from the next start. If it closes before it shows, Starling starts \
                     without it the time after."
                        .into(),
                )
            }
            Stage::Failed(e) => {
                acts.push(("Try again".into(), Act::Retry));
                ("Native mods".to_owned(), e.clone())
            }
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

    /// While the card shows it takes every touch; a lifted finger on a button acts.
    pub(super) fn native_builds_touch(&mut self, touch: &Touch) -> bool {
        if !self.native_builds_shown() {
            return false;
        }
        if touch.phase != TouchPhase::Ended {
            return true;
        }
        let [w, h] = self.touch_size();
        let d = density(w as f32, h as f32);
        self.native_builds_press([touch.location.x as f32 / d, touch.location.y as f32 / d]);
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
        let Some((version, _)) = crate::app_version() else {
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
            Act::Request => {
                super::packs::open_uri(&format!(
                    "{REQUEST}&title=%5Bnative+build%5D+&version={version}"
                ));
            }
            Act::Restart => crate::restart_app(),
            Act::Retry => self.open_native_builds(),
            Act::Close => self.native_builds.stage = None,
        }
    }
}
