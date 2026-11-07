//! Android: setting the game up on the phone when its resources are not there (or did not
//! load), in place of the loading failure. Sign in with the Microsoft account that owns the game
//! (accounts.rs) or play offline, the official files fetched, then prepared on the phone
//! (preparer.rs); or the resources prepared on a computer, picked as before. Drawn with the
//! embedded Horizon font: the game's own is part of what is missing. While it waits, water to fish in (fishing.rs), and the
//! game's settings, drawn in the low-budget skin the app carries (packaging/lowbudget).
pub(super) mod fishing;

use super::{Audio, Loading, UiResources};
use horizon_client::{
    accounts::{self, Files, State},
    hud::HudBatch,
    preparer::{self, Stage},
    ui_theme::{Face, Ui, color},
};
use std::cell::Cell;
use std::time::Instant;
use winit::event::{Touch, TouchPhase};

/// What the screen offers now.
#[derive(Clone, Debug, PartialEq)]
enum Step {
    /// Before anything: the Starling folder the title screen offers too (files_offer.rs), so the
    /// game's files are where a player can reach them, and a bug report's log with them.
    #[cfg(target_os = "android")]
    Folder(super::files_offer::Stage),
    /// Horizon Android is on the phone: its worlds and resource packs can come over
    /// (HorizonImport.java). True once its folder was asked for: they are arriving.
    #[cfg(target_os = "android")]
    Horizon(bool),
    /// Nothing under way: sign in, play offline, or bring resources from a computer.
    Start,
    /// Play offline chosen: whether the player is sure (most servers need a sign-in).
    Offline,
    /// Asking Microsoft (or renewing the saved sign-in).
    Talking,
    /// The player signs in elsewhere with `code`.
    Code {
        code: String,
        link: String,
    },
    /// The official files on their way: files and bytes so far, and in all.
    Download([u64; 4]),
    /// Java on its way: bytes so far, and in all.
    Java(u64, u64),
    Unpacking,
    /// horizon-prepare's last line.
    Preparing(String),
    /// Prepared: the game loads.
    Done,
    /// The last attempt failed, and why.
    Failed(String),
    /// The sign-in failed (an account without the game, say): playing offline is offered too.
    SignInFailed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    /// The Starling folder: Android's setting that allows it, or, allowed, the restart that moves
    /// the files.
    #[cfg(target_os = "android")]
    Folder,
    /// Not now (asked again on the title screen only if not turned down), or Later.
    #[cfg(target_os = "android")]
    FolderLater,
    /// Horizon Android's folder in the system's picker (again, if the player came back without
    /// picking it).
    #[cfg(target_os = "android")]
    Horizon,
    /// Skip, or Next once its files are on their way: not offered again.
    #[cfg(target_os = "android")]
    HorizonDone,
    SignIn,
    /// Play offline: asks first, then the files are fetched without a sign-in.
    Offline,
    PlayOffline,
    /// Back from the question to the start.
    NotOffline,
    OpenLink,
    CopyCode,
    /// Pick the resources prepared on a computer (as the loading failure did).
    Pick,
    /// After a failure: the step that failed, again.
    Retry,
    /// While it waits: to the water, back from it, or to the game's settings.
    Fish,
    Back,
    Settings,
}

#[derive(Default)]
pub(super) struct Setup {
    /// The saved sign-in renewed, once.
    resumed: bool,
    /// The Starling folder's setting opened, waiting for the player to come back from it.
    #[cfg(target_os = "android")]
    folder_asked: bool,
    /// The Starling folder put off for this start (Later).
    #[cfg(target_os = "android")]
    folder_later: bool,
    /// Whether Horizon Android is installed, asked of Android once.
    #[cfg(target_os = "android")]
    horizon_installed: Cell<Option<bool>>,
    /// Its folder asked for: its worlds and packs are on their way to the import inbox.
    #[cfg(target_os = "android")]
    horizon_picked: bool,
    /// Play offline tapped: the question shows.
    offline_asked: bool,
    /// Prepared once already: what it made did not load either (the failure says why), so it is
    /// not prepared again unasked.
    prepared: bool,
    /// Prepared and loading: the setup stays up until the menu's font is there (the loading
    /// screen without it draws every letter as an empty box).
    pub(super) starting: bool,
    /// The buttons as last drawn (dp): rectangle and what it does.
    buttons: Cell<[Option<([f32; 4], Action)>; 3]>,
    fishing: fishing::Fishing,
    /// The water as last drawn (dp), while there is a wait to pass.
    pond: Cell<Option<[f32; 4]>>,
    /// Out fishing (the water over the screen), not on the card.
    fishing_open: bool,
    /// The low-budget skin put on the menus, once.
    skinned: bool,
}

impl Setup {
    /// How soon the screen draws again: often while the water or the settings show.
    pub(super) fn frame(&self, settings_open: bool) -> std::time::Duration {
        let lively = self.pond.get().is_some() || settings_open;
        std::time::Duration::from_millis(if lively { 33 } else { 400 })
    }
}

/// Whether the step is a wait worth passing (the water shows).
fn waiting(step: &Step) -> bool {
    matches!(
        step,
        Step::Download(_) | Step::Java(..) | Step::Unpacking | Step::Preparing(_)
    )
}

fn step(setup: &Setup, failure: &str) -> Step {
    if setup.starting {
        return Step::Done;
    }
    if setup.prepared && preparer::stage() == Stage::Idle {
        return Step::Failed(format!("The prepared game files did not load: {failure}"));
    }
    match preparer::stage() {
        Stage::Java(done, total) => return Step::Java(done, total),
        Stage::Unpacking => return Step::Unpacking,
        Stage::Preparing(line) => return Step::Preparing(line),
        Stage::Done => return Step::Done,
        Stage::Failed(e) => return Step::Failed(e),
        Stage::Idle => {}
    }
    match accounts::files() {
        Files::Checking => return Step::Download([0; 4]),
        Files::Downloading(p) => {
            return Step::Download([p.files as u64, p.files_total as u64, p.bytes, p.bytes_total]);
        }
        Files::Failed(e) => return Step::Failed(e),
        // Ready: the preparing starts by itself (poll_setup).
        Files::Ready => return Step::Java(0, 0),
        Files::Missing => {}
    }
    match accounts::state() {
        // Play offline tapped, at the start or after a failed sign-in: the question.
        State::Offline | State::Failed(_) if setup.offline_asked => Step::Offline,
        State::Offline => {
            #[cfg(target_os = "android")]
            if let Some(stage) = folder_offer(setup) {
                return Step::Folder(stage);
            }
            // After the folder: what comes over lands in the game folder that stays.
            #[cfg(target_os = "android")]
            if horizon_offer(setup) {
                return Step::Horizon(setup.horizon_picked);
            }
            Step::Start
        }
        State::Resuming | State::Starting | State::SignedIn(_) => Step::Talking,
        State::Code { code, link } => Step::Code { code, link },
        State::Failed(e) => Step::SignInFailed(e),
    }
}

/// The Starling folder's offer where it stands, while the game folder is still the app's own
/// files and the player has not turned it down.
#[cfg(target_os = "android")]
fn folder_offer(setup: &Setup) -> Option<super::files_offer::Stage> {
    use super::files_offer::{Stage, declined};
    let private = horizon_client::logging::game_home() == horizon_client::logging::private_home();
    if !private || setup.folder_later || declined().exists() {
        return None;
    }
    Some(if crate::android::file_access_allowed() {
        Stage::Restart
    } else if setup.folder_asked {
        Stage::Waiting
    } else {
        Stage::Ask
    })
}

/// Written once the player is done with the offer of Horizon Android's worlds (Next or Skip): it
/// is not made again.
#[cfg(target_os = "android")]
fn horizon_answered() -> std::path::PathBuf {
    horizon_client::logging::private_home().join("horizon-android-import-answered")
}

/// Whether to offer Horizon Android's worlds and packs: it is installed, and not answered yet.
#[cfg(target_os = "android")]
fn horizon_offer(setup: &Setup) -> bool {
    if horizon_answered().exists() {
        return false;
    }
    setup.horizon_installed.get().unwrap_or_else(|| {
        let installed = crate::android::horizon_android_installed();
        setup.horizon_installed.set(Some(installed));
        installed
    })
}

fn mb(bytes: u64) -> String {
    format!("{:.0}", bytes as f64 / 1_000_000.0)
}

/// What the step says: title, body, and the line under it.
fn texts(step: &Step) -> (&'static str, String, Option<String>) {
    match step {
        #[cfg(target_os = "android")]
        Step::Folder(stage) => {
            let [title, body, ..] = super::files_offer::texts(*stage);
            (title, body.into(), None)
        }
        #[cfg(target_os = "android")]
        Step::Horizon(false) => (
            "Coming from Horizon Android?",
            "Horizon Android is part of Starling now: bring your worlds and resource packs over. \
             Close Horizon Android first, then, in the folder that opens, tap Use this folder and \
             Allow."
                .into(),
            Some("They are copied: Horizon Android keeps its own.".into()),
        ),
        #[cfg(target_os = "android")]
        Step::Horizon(true) => (
            "Coming from Horizon Android?",
            "Your worlds and resource packs are on their way: the note at the top says what each \
             one added. Tap Next when they are in."
                .into(),
            None,
        ),
        Step::Start => (
            "Set up Starling",
            "Sign in with the Microsoft account that owns Minecraft: Java Edition, or play \
             offline: Starling downloads the game from Mojang and prepares it on this phone, once \
             (about 1.5 GB, a few minutes)."
                .into(),
            None,
        ),
        Step::Offline => (
            "Are you sure?",
            "You will not be able to play on most servers! We highly recommend signing in with \
             Microsoft."
                .into(),
            None,
        ),
        Step::Talking => ("Set up Starling", "Talking to Microsoft...".into(), None),
        Step::Code { link, .. } => (
            "Sign in with Microsoft",
            format!("On {link}, enter this code:"),
            Some("Starling goes on by itself once you have signed in.".into()),
        ),
        Step::Download([files, files_total, bytes, bytes_total]) => (
            "Downloading the game",
            "From Mojang's servers, checked file by file.".into(),
            Some(if *bytes_total > 0 {
                format!(
                    "{files} of {files_total} files, {} of {} MB",
                    mb(*bytes),
                    mb(*bytes_total)
                )
            } else {
                "Checking the files".into()
            }),
        ),
        Step::Java(done, total) => (
            "Preparing the game",
            "Downloading Java, which prepares the game files.".into(),
            (*total > 0).then(|| format!("{} of {} MB", mb(*done), mb(*total))),
        ),
        Step::Unpacking => ("Preparing the game", "Unpacking Java.".into(), None),
        Step::Preparing(line) => (
            "Preparing the game",
            "This takes a few minutes. Keep Starling open.".into(),
            Some(line.clone()),
        ),
        Step::Done => (
            "Starting Starling",
            "The game files are ready.".into(),
            None,
        ),
        Step::Failed(e) | Step::SignInFailed(e) => ("Something went wrong", e.clone(), None),
    }
}

/// The fraction done, for the bar.
fn fraction(step: &Step) -> Option<f32> {
    match step {
        Step::Download([_, _, bytes, total]) | Step::Java(bytes, total) if *total > 0 => {
            Some((*bytes as f64 / *total as f64) as f32)
        }
        _ => None,
    }
}

/// The buttons, the main one first (drawn on the right).
fn buttons(step: &Step) -> [Option<(&'static str, Action)>; 3] {
    match step {
        #[cfg(target_os = "android")]
        Step::Folder(stage) => {
            let [_, _, primary, other] = super::files_offer::texts(*stage);
            [
                Some((primary, Action::Folder)),
                Some((other, Action::FolderLater)),
                None,
            ]
        }
        #[cfg(target_os = "android")]
        Step::Horizon(false) => [
            Some(("Bring them over", Action::Horizon)),
            Some(("Skip", Action::HorizonDone)),
            None,
        ],
        #[cfg(target_os = "android")]
        Step::Horizon(true) => [
            Some(("Next", Action::HorizonDone)),
            Some(("Pick the folder again", Action::Horizon)),
            None,
        ],
        Step::Start => [
            Some(("Sign in with Microsoft", Action::SignIn)),
            Some(("Play offline", Action::Offline)),
            Some(("Files from a computer", Action::Pick)),
        ],
        Step::Offline => [
            Some(("Sign in with Microsoft", Action::SignIn)),
            Some(("Play offline", Action::PlayOffline)),
            Some(("Back", Action::NotOffline)),
        ],
        Step::Code { .. } => [
            Some(("Open the page", Action::OpenLink)),
            Some(("Copy the code", Action::CopyCode)),
            None,
        ],
        Step::Failed(_) => [
            Some(("Try again", Action::Retry)),
            Some(("Files from a computer", Action::Pick)),
            None,
        ],
        Step::SignInFailed(_) => [
            Some(("Try again", Action::Retry)),
            Some(("Play offline", Action::Offline)),
            Some(("Files from a computer", Action::Pick)),
        ],
        step if waiting(step) => [
            Some(("Go fishing", Action::Fish)),
            Some(("Settings", Action::Settings)),
            None,
        ],
        _ => [None, None, None],
    }
}

/// Words of `text` in lines no wider than `max` dp; a word wider than that (a path) is cut where
/// it reaches the edge.
fn wrap(ui: &Ui<'_>, size: f32, text: &str, max: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        let mut words = Vec::new();
        for word in paragraph.split(' ') {
            let mut piece = String::new();
            for ch in word.chars() {
                piece.push(ch);
                if piece.chars().count() > 1 && ui.measure(Face::Regular, size, &piece) > max {
                    piece.pop();
                    words.push(std::mem::replace(&mut piece, ch.to_string()));
                }
            }
            words.push(piece);
        }
        for word in &words {
            let next = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && ui.measure(Face::Regular, size, &next) > max {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = next;
            }
        }
        lines.push(line);
    }
    lines
}

impl Loading {
    /// Every frame on the setup: a saved sign-in renewed, the download after a sign-in, the
    /// preparing once the files are there, and the game loaded once they are prepared.
    pub(super) fn poll_setup(&mut self) {
        self.setup.fishing.update(Instant::now());
        // The fishing's sounds, on an output of its own: the game's sounds are not there yet.
        if self.audio.is_none() && self.setup.pond.get().is_some() {
            let mut audio = Audio::with_registry(&self.vanilla_root, Default::default());
            audio.master = self.settings.master_volume;
            self.audio = Some(audio);
        }
        for sound in self.setup.fishing.take_sounds() {
            if let Some(audio) = &mut self.audio {
                audio.play_samples(sound.samples(), fishing::RATE, 0.3);
            }
        }
        if !self.setup.resumed {
            self.setup.resumed = true;
            accounts::resume();
        }
        // The menus in the skin the app carries: their own resources are what is missing.
        if !self.setup.skinned {
            self.setup.skinned = true;
            if let Some(root) = preparer::skin() {
                match UiResources::load(&root, &root) {
                    Ok(gui) => self.set_ui(gui, root),
                    Err(e) => log::warn!("Horizon: the setup's skin did not load: {e}"),
                }
            }
        }
        if preparer::stage() == Stage::Done {
            self.setup.prepared = true;
            preparer::clear();
            // The game opens its sound output with the sounds just prepared.
            self.audio = None;
            match self.retry() {
                Ok(()) => self.setup.starting = true,
                Err(e) => self.failure = Some(e.to_string()),
            }
            return;
        }
        if preparer::running() {
            return;
        }
        match (accounts::files(), accounts::state()) {
            (Files::Ready, _) if preparer::stage() == Stage::Idle && !self.setup.prepared => {
                preparer::start(self.vanilla_root.clone());
            }
            (Files::Missing, State::SignedIn(_)) => accounts::download_files(),
            _ => {}
        }
    }

    pub(super) fn draw_setup(&self, batch: &mut HudBatch, failure: &str) {
        let Some(gui) = &self.gui else { return };
        let step = step(&self.setup, failure);
        let (title, body, note) = texts(&step);
        let mut ui = Ui::new(&gui.atlas, batch, (-1.0, -1.0));
        let (w, h) = (ui.w, ui.h);
        let now = Instant::now();
        let pond = (waiting(&step) && self.setup.fishing_open).then_some([0.0, 0.0, w, h]);
        if pond.is_some() {
            self.setup.fishing.draw_water(&mut ui, w, h, now);
        } else {
            ui.fill(0.0, 0.0, w, h, color::BG);
        }
        // Out fishing, the card only says how far it is; Back is in the corner.
        let actions = if pond.is_some() {
            [None, None, None]
        } else {
            buttons(&step)
        };
        // Wider for three buttons, so "Sign in with Microsoft" fits in a third.
        let three = actions.iter().all(Option::is_some);
        let card_w = (w - 48.0).min(if three { 660.0 } else { 520.0 });
        let pad = 24.0;
        let inner = card_w - 2.0 * pad;
        let mut lines = wrap(&ui, 14.0, &body, inner);
        if lines.len() > 9 {
            lines.truncate(8);
            lines.push("...".into());
        }
        let note = note.map(|n| wrap(&ui, 12.0, &n, inner));
        let code = match &step {
            Step::Code { code, .. } => Some(code.clone()),
            _ => None,
        };
        let bar = fraction(&step);
        let button_h = 40.0;
        let mut card_h = pad + 26.0 + 12.0 + lines.len() as f32 * 21.0;
        if code.is_some() {
            card_h += 52.0;
        }
        if bar.is_some() {
            card_h += 22.0;
        }
        if let Some(note) = &note {
            card_h += 8.0 + note.len() as f32 * 18.0;
        }
        if actions.iter().any(Option::is_some) {
            card_h += 22.0 + button_h;
        }
        card_h += pad;
        // With a wait to pass, the card goes up, over the water.
        let (x, y) = if pond.is_some() {
            ((w - card_w) / 2.0, 16.0)
        } else {
            ((w - card_w) / 2.0, ((h - card_h) / 2.0).max(8.0))
        };
        let card = if pond.is_some() {
            color::with_alpha(color::BG, 0.88)
        } else {
            color::BG
        };
        ui.rounded(x, y, card_w, card_h, 14.0, card);
        ui.border(x, y, card_w, card_h, 14.0, color::BORDER);
        ui.text(Face::Semibold, 18.0, x + pad, y + pad, color::TEXT, title);
        let mut ty = y + pad + 26.0 + 12.0;
        for line in &lines {
            ui.text(Face::Regular, 14.0, x + pad, ty, color::MUTED, line);
            ty += 21.0;
        }
        if let Some(code) = &code {
            ui.text_centered(
                Face::Semibold,
                30.0,
                x + card_w / 2.0,
                ty + 8.0,
                color::TEXT,
                code,
            );
            ty += 52.0;
        }
        if let Some(done) = bar {
            ty += 6.0;
            ui.rounded(x + pad, ty, inner, 8.0, 4.0, color::RAISED);
            ui.rounded(
                x + pad,
                ty,
                inner * done.clamp(0.0, 1.0),
                8.0,
                4.0,
                color::ACCENT,
            );
            ty += 16.0;
        }
        if let Some(note) = &note {
            ty += 8.0;
            for line in note {
                ui.text(Face::Regular, 12.0, x + pad, ty, color::MUTED, line);
                ty += 18.0;
            }
        }
        let mut drawn = [None, None, None];
        let shown: Vec<_> = actions.iter().flatten().collect();
        if !shown.is_empty() {
            let by = y + card_h - pad - button_h;
            let bw = (inner - 12.0 * (shown.len() - 1) as f32) / shown.len() as f32;
            for (i, (text, action)) in shown.iter().enumerate() {
                // The first is the main one, on the right.
                let bx = x + card_w - pad - bw - (bw + 12.0) * i as f32;
                let rect = [bx, by, bw, button_h];
                let (fill, ink) = if i == 0 {
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
                drawn[i] = Some((rect, *action));
            }
        }
        // Why the game could not start, small, under the card: for a report. Not on a first start,
        // where the game files are simply not there yet.
        if matches!(step, Step::Start) && self.vanilla_root.exists() {
            for (i, line) in wrap(&ui, 11.0, failure, card_w).iter().take(2).enumerate() {
                let ly = y + card_h + 14.0 + i as f32 * 15.0;
                ui.text(
                    Face::Regular,
                    11.0,
                    x,
                    ly,
                    color::with_alpha(color::MUTED, 0.6),
                    line,
                );
            }
        }
        if pond.is_some() {
            self.setup.fishing.draw(&mut ui, w, h, h - 58.0, now);
            let rect = [16.0, 16.0, 92.0, 36.0];
            ui.rounded(
                rect[0],
                rect[1],
                rect[2],
                rect[3],
                18.0,
                color::with_alpha(color::BG, 0.88),
            );
            ui.text_centered(
                Face::Semibold,
                15.0,
                rect[0] + rect[2] / 2.0,
                rect[1] + 9.0,
                color::TEXT,
                "Back",
            );
            drawn[0] = Some((rect, Action::Back));
        }
        ui.finish();
        self.setup.buttons.set(drawn);
        // Fishing under the card: the card itself takes no cast.
        let below = y + card_h + 8.0;
        self.setup
            .pond
            .set(pond.map(|_| [0.0, below, w, h - below]));
        // The inbox's banner over it all, since the setup polls the inbox too: what its file
        // picker, or Horizon Android's folder, added.
        self.imports.draw(&gui.atlas, batch);
    }

    /// A finger lifted on a button acts.
    pub(super) fn setup_touch(&mut self, touch: &Touch) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let (pw, ph) = (size.width as f32, size.height as f32);
        let density = (pw / 1040.0).min(ph / 700.0).clamp(1.0, 4.0);
        let p = [
            touch.location.x as f32 / density,
            touch.location.y as f32 / density,
        ];
        let inside = |r: [f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };
        let on_button = self
            .setup
            .buttons
            .get()
            .into_iter()
            .flatten()
            .any(|(r, _)| inside(r));
        // The water: a finger down casts, lifted it reels in.
        match touch.phase {
            TouchPhase::Started if !on_button && self.setup.pond.get().is_some_and(inside) => {
                self.setup.fishing.press(Instant::now(), p);
                return;
            }
            TouchPhase::Ended | TouchPhase::Cancelled => self.setup.fishing.release(Instant::now()),
            _ => {}
        }
        if touch.phase != TouchPhase::Ended {
            return;
        }
        let hit = self
            .setup
            .buttons
            .get()
            .into_iter()
            .flatten()
            .find(|(r, _)| inside(*r));
        let Some((_, action)) = hit else { return };
        match action {
            #[cfg(target_os = "android")]
            Action::Folder => match folder_offer(&self.setup) {
                Some(super::files_offer::Stage::Restart) => crate::android::restart_app(),
                _ => {
                    crate::android::open_file_access_settings();
                    self.setup.folder_asked = true;
                }
            },
            #[cfg(target_os = "android")]
            Action::FolderLater => {
                // Not now: as on the title screen, not offered again (Later, once allowed: the
                // next start moves the files anyway).
                if folder_offer(&self.setup) != Some(super::files_offer::Stage::Restart) {
                    let _ = std::fs::write(super::files_offer::declined(), b"");
                }
                self.setup.folder_later = true;
            }
            #[cfg(target_os = "android")]
            Action::Horizon => {
                if crate::android::import_horizon_android() {
                    self.setup.horizon_picked = true;
                } else {
                    log::warn!("Horizon: no folder picker for Horizon Android's files");
                }
            }
            #[cfg(target_os = "android")]
            Action::HorizonDone => {
                let _ = std::fs::write(horizon_answered(), b"");
            }
            Action::SignIn => {
                self.setup.offline_asked = false;
                accounts::sign_in();
            }
            Action::Offline => self.setup.offline_asked = true,
            Action::PlayOffline => {
                self.setup.offline_asked = false;
                accounts::download_files();
            }
            Action::NotOffline => self.setup.offline_asked = false,
            Action::OpenLink => {
                if let State::Code { link, .. } = accounts::state()
                    && !horizon_client::platform::open_link(&link).unwrap_or(false)
                {
                    log::warn!("Horizon: no browser opened {link}");
                }
            }
            Action::CopyCode => {
                if let State::Code { code, .. } = accounts::state() {
                    let _ = horizon_client::platform::clipboard_set(&code);
                }
            }
            Action::Pick => {
                if !horizon_client::platform::pick_file() {
                    log::warn!("Horizon: no file picker to import resources with");
                }
            }
            Action::Fish => self.setup.fishing_open = true,
            Action::Back => {
                self.setup.fishing.release(Instant::now());
                self.setup.fishing_open = false;
            }
            Action::Settings => self.open_menu_options(horizon_client::options_ui::Page::Main),
            Action::Retry => match (preparer::stage(), accounts::files()) {
                (Stage::Failed(_) | Stage::Idle, Files::Ready) => {
                    preparer::start(self.vanilla_root.clone())
                }
                (_, Files::Failed(_)) => accounts::download_files(),
                _ => accounts::sign_in(),
            },
        }
    }
}
