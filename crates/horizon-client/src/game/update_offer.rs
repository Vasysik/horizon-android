//! Android: the prompt, over the title screen, when GitHub has a newer Starling release than this
//! one: the newest of the organisation's repository and Starling's own (starling_releases, which
//! also keeps Horizon Android's releases and preview builds out). Update downloads its APK and
//! hands it to Android's installer, which asks the player (and, the first time, to allow
//! installing from Starling) and offers to open the game again once it is in; Tomorrow asks again
//! in a day; No, at the next start. Only a release asks: a test build is signed with another key,
//! and the release could not install over it (`STARLING_UPDATE_TEST=<version>` makes one ask as
//! that version, to try the prompt).
use super::Game;
use horizon_client::{
    hud::HudBatch,
    starling_releases::{self as releases, Update as Release},
    ui_theme::{Face, Ui, color},
};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use winit::event::{Touch, TouchPhase};

/// How long Tomorrow keeps the prompt away.
const LATER: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, PartialEq)]
enum Stage {
    Ask,
    Downloading,
    /// The installer was shown; it may be shown again.
    Installing,
    Failed(String),
}

/// What the check and the download threads hand the game.
#[derive(Default)]
struct Shared {
    release: Option<Release>,
    done: u64,
    total: u64,
    /// The APK's name in files/updates/ once downloaded, or why it is not.
    downloaded: Option<Result<String, String>>,
}

#[derive(Default)]
pub(super) struct UpdateOffer {
    started: bool,
    stage: Option<Stage>,
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
    /// The buttons as last drawn (UI units), at most three.
    buttons: Cell<[Option<[f32; 4]>; 3]>,
}

fn updates() -> PathBuf {
    horizon_client::logging::private_home().join("updates")
}

/// Written by Tomorrow: the time (Unix seconds) before which the prompt is not made.
fn later() -> PathBuf {
    horizon_client::logging::private_home().join("update-later")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A repository's latest release, as GitHub's API serves it.
fn latest(url: &str, current: &str) -> Result<serde_json::Value, String> {
    let text = ureq::get(url)
        .set("User-Agent", &format!("Starling/{current}"))
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// The newest Starling release on GitHub, when it is newer than `current` and has its APK: every
/// repository asked, an error only when none answered.
fn check(current: &str) -> Result<Option<Release>, String> {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for repository in releases::REPOSITORIES {
        match latest(&releases::latest_url(repository), current) {
            Ok(release) => found.extend(releases::update_in(&release, current)),
            Err(e) => errors.push(format!("{repository}: {e}")),
        }
    }
    if errors.len() == releases::REPOSITORIES.len() {
        return Err(errors.join("; "));
    }
    Ok(releases::newest(found))
}

/// The release's APK into files/updates/, checked; its name there.
fn download(
    release: &Release,
    shared: &Mutex<Shared>,
    cancel: &AtomicBool,
) -> Result<String, String> {
    let ok = |n: &str| {
        !n.is_empty()
            && n.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    if !ok(&release.name) || !release.url.starts_with("https://") {
        return Err(format!(
            "the release's APK is named oddly: {}",
            release.name
        ));
    }
    let dir = updates();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let part = dir.join(format!("{}.part", release.name));
    // No limit on the whole download (a slow network), only on each wait.
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build()
        .get(&release.url)
        .call()
        .map_err(|e| e.to_string())?;
    let total = response
        .header("Content-Length")
        .and_then(|l| l.parse().ok())
        .unwrap_or(release.size);
    shared.lock().unwrap_or_else(PoisonError::into_inner).total = total;
    let mut body = response.into_reader();
    let mut file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 256 * 1024];
    let mut done = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
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
    if release.sha256.as_ref().is_some_and(|want| *want != got) || done == 0 {
        let _ = std::fs::remove_file(&part);
        return Err("The download came out damaged. Try again.".into());
    }
    std::fs::rename(&part, dir.join(&release.name)).map_err(|e| e.to_string())?;
    Ok(release.name.clone())
}

/// The UI's scale for a window of `w` x `h` pixels (as `Ui::new`).
fn density(w: f32, h: f32) -> f32 {
    (w / 1040.0).min(h / 700.0).clamp(1.0, 4.0)
}

impl Game<'_> {
    /// Once at start (the check, in the background), then the download while it runs.
    pub(super) fn poll_update_offer(&mut self) {
        let offer = &mut self.update_offer;
        if !offer.started {
            offer.started = true;
            // An update installed, or one left: the APK goes.
            let _ = std::fs::remove_dir_all(updates());
            let test = std::env::var("STARLING_UPDATE_TEST")
                .ok()
                .filter(|v| !v.is_empty());
            let current = match (crate::android::app_version(), test) {
                (_, Some(pretend)) => pretend,
                (Some((version, true)), None) => version,
                _ => return,
            };
            let until = std::fs::read_to_string(later())
                .ok()
                .and_then(|t| t.trim().parse::<u64>().ok());
            if until.is_some_and(|t| now() < t) {
                return;
            }
            let shared = offer.shared.clone();
            let _ = std::thread::Builder::new()
                .name("update check".into())
                .spawn(move || match check(&current) {
                    Ok(release) => {
                        if let Some(r) = &release {
                            log::info!(
                                "Horizon: Starling {} is out (this is {current})",
                                r.version
                            );
                        }
                        shared
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .release = release;
                    }
                    Err(e) => log::info!("Horizon: could not look for a newer Starling: {e}"),
                });
            return;
        }
        let mut shared = offer.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if offer.stage.is_none()
            && shared.release.is_some()
            && !offer.cancel.load(Ordering::Relaxed)
        {
            offer.stage = Some(Stage::Ask);
        }
        if offer.stage == Some(Stage::Downloading)
            && let Some(result) = shared.downloaded.take()
        {
            offer.stage = Some(match result {
                Ok(name) if crate::android::install_update(&name) => Stage::Installing,
                Ok(_) => Stage::Failed("Android has no installer that takes the update.".into()),
                Err(e) => Stage::Failed(e),
            });
        }
    }

    /// Over the title only, and not over the offer of the Starling folder.
    fn update_offer_shown(&self) -> Option<Stage> {
        if self.title.is_none() || self.files_offer_shown().is_some() {
            return None;
        }
        self.update_offer.stage.clone()
    }

    fn release(&self) -> Option<Release> {
        self.update_offer
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .release
            .clone()
    }

    /// A card in the middle of the title screen, the rest dimmed, as the folder's offer.
    pub(super) fn draw_update_offer(&self, batch: &mut HudBatch) {
        let Some(stage) = self.update_offer_shown() else {
            return;
        };
        let Some(release) = self.release() else {
            return;
        };
        let mb = |bytes: u64| format!("{:.0}", bytes as f64 / 1_000_000.0);
        let (done, total) = {
            let shared = self
                .update_offer
                .shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            (shared.done, shared.total.max(release.size))
        };
        let version = &release.version;
        let (title, body, buttons): (String, String, &[&str]) = match &stage {
            Stage::Ask => (
                format!("Starling {version} is out"),
                format!(
                    "Update now? Starling downloads it ({} MB) and Android installs it over this \
                     one: your worlds and settings stay.",
                    mb(release.size)
                ),
                &["Update", "Tomorrow", "No"],
            ),
            Stage::Downloading => (
                format!("Downloading Starling {version}"),
                format!("{} of {} MB", mb(done), mb(total)),
                &["Cancel"],
            ),
            Stage::Installing => (
                format!("Install Starling {version}"),
                "Android asks to install the update (the first time, to allow installs from \
                 Starling). Once it is in, open Starling again."
                    .into(),
                &["Install", "Later"],
            ),
            Stage::Failed(e) => (
                "The update did not download".into(),
                e.clone(),
                &["Try again", "Later"],
            ),
        };
        let mut ui = Ui::new(&self.gui.atlas, batch, (-1.0, -1.0));
        let (w, h) = (ui.w, ui.h);
        ui.fill(0.0, 0.0, w, h, color::with_alpha(color::BG, 0.72));
        let card_w = (w - 48.0).min(470.0);
        let pad = 22.0;
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in body.split(' ') {
            let next = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && ui.measure(Face::Regular, 14.0, &next) > card_w - 2.0 * pad {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = next;
            }
        }
        lines.push(line);
        let bar = stage == Stage::Downloading;
        let button_h = 40.0;
        let card_h = pad
            + 26.0
            + 12.0
            + lines.len() as f32 * 21.0
            + if bar { 22.0 } else { 0.0 }
            + 22.0
            + button_h
            + pad;
        let (x, y) = ((w - card_w) / 2.0, ((h - card_h) / 2.0).max(8.0));
        ui.rounded(x, y, card_w, card_h, 14.0, color::BG);
        ui.border(x, y, card_w, card_h, 14.0, color::BORDER);
        ui.text(Face::Semibold, 18.0, x + pad, y + pad, color::TEXT, &title);
        let mut ty = y + pad + 26.0 + 12.0;
        for line in &lines {
            ui.text(Face::Regular, 14.0, x + pad, ty, color::MUTED, line);
            ty += 21.0;
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
        }
        // The first button is the primary one, on the right; the others to its left.
        let by = y + card_h - pad - button_h;
        let gap = 12.0;
        let n = buttons.len() as f32;
        let bw = (card_w - 2.0 * pad - gap * (n - 1.0)) / n;
        let mut drawn = [None; 3];
        for (i, text) in buttons.iter().enumerate() {
            let bx = x + card_w - pad - bw - i as f32 * (bw + gap);
            let rect = [bx, by, bw, button_h];
            let (fill, ink) = if i == 0 && buttons.len() > 1 {
                (color::ACCENT, color::ON_ACCENT)
            } else {
                (color::RAISED, color::TEXT)
            };
            ui.rounded(rect[0], rect[1], rect[2], rect[3], 10.0, fill);
            ui.text_centered(
                Face::Semibold,
                15.0,
                rect[0] + rect[2] / 2.0,
                rect[1] + (rect[3] - 15.0) / 2.0 - 2.0,
                ink,
                text,
            );
            drawn[i] = Some(rect);
        }
        ui.finish();
        self.update_offer.buttons.set(drawn);
    }

    /// While the card shows it takes every touch; a lifted finger on a button acts.
    pub(super) fn update_offer_touch(&mut self, touch: &Touch) -> bool {
        let Some(stage) = self.update_offer_shown() else {
            return false;
        };
        if touch.phase != TouchPhase::Ended {
            return true;
        }
        let [w, h] = self.touch_size();
        let d = density(w as f32, h as f32);
        let p = [touch.location.x as f32 / d, touch.location.y as f32 / d];
        let inside = |r: [f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };
        let Some(button) = self
            .update_offer
            .buttons
            .get()
            .iter()
            .position(|r| r.is_some_and(inside))
        else {
            return true;
        };
        let release = self.release();
        let offer = &mut self.update_offer;
        match (stage, button) {
            (Stage::Ask | Stage::Failed(_), 0) => {
                if let Some(release) = release {
                    offer.stage = Some(Stage::Downloading);
                    offer.cancel.store(false, Ordering::Relaxed);
                    let shared = offer.shared.clone();
                    let cancel = offer.cancel.clone();
                    {
                        let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
                        s.done = 0;
                        s.downloaded = None;
                    }
                    let _ = std::thread::Builder::new()
                        .name("update download".into())
                        .spawn(move || {
                            let result = download(&release, &shared, &cancel);
                            if let Err(e) = &result {
                                log::warn!("Horizon: the update did not download: {e}");
                            }
                            shared
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .downloaded = Some(result);
                        });
                }
            }
            (Stage::Ask, 1) => {
                let _ = std::fs::write(later(), (now() + LATER.as_secs()).to_string());
                offer.stage = None;
                offer.cancel.store(true, Ordering::Relaxed);
            }
            (Stage::Installing, 0) => {
                if let Some(release) = release
                    && !crate::android::install_update(&release.name)
                {
                    offer.stage = Some(Stage::Failed(
                        "Android has no installer that takes the update.".into(),
                    ));
                }
            }
            // Cancel, No, Later: asked again at the next start.
            _ => {
                offer.cancel.store(true, Ordering::Relaxed);
                offer.stage = None;
            }
        }
        true
    }
}
