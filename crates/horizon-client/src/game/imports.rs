//! Files handed to the game from outside its folder (Android's "Open with", share and file picker,
//! Horizon Android's folder): the inbox installed on a worker (horizon_client::import), told by a
//! banner across the top of every screen, the setup's too. It drops in with a sound as a file
//! arrives, a spinner turning while it installs, then says what each file added with a chime (or
//! what went wrong, lower), its bar running out before it goes back up. Its sounds are made here,
//! as the fishing's are: the game's own may not be there yet.
use horizon_client::{
    hud::{HudAtlas, HudBatch},
    import::{self, Event, Inbox, Kind, Places},
    ui_theme::{Face, Ui, color},
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(400);
/// How long a finished import's banner stays down.
const SHOWN: Duration = Duration::from_millis(4500);
/// The banner's way down (with a little bounce) and back up.
const ENTER: Duration = Duration::from_millis(360);
const LEAVE: Duration = Duration::from_millis(260);
/// The icon's pop as an import ends.
const POP: Duration = Duration::from_millis(340);
/// Two banners' sounds of one kind this close are one.
const AGAIN: Duration = Duration::from_millis(900);
/// The made sounds' sample rate.
pub(super) const RATE: u32 = 22_050;
/// The banner's second line's size.
const DETAIL: f32 = 15.5;

/// What the banner says the import is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    /// A file arriving or installing: the spinner turns.
    Working,
    /// Installed: what it was, for its icon (None for a note of the caller's own).
    Done(Option<Kind>),
    Failed,
}

impl State {
    fn color(self) -> u32 {
        match self {
            State::Working => color::ACCENT,
            State::Done(_) => color::SUCCESS,
            State::Failed => color::DANGER,
        }
    }
}

/// The banner's sounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Chime {
    /// It drops in: a soft rising note.
    Arrive,
    /// Imported: two soft round notes going up.
    Done,
    /// It failed: two low notes going down.
    Failed,
}

impl Chime {
    pub(super) fn gain(self) -> f32 {
        match self {
            Chime::Arrive => 0.22,
            Chime::Done => 0.28,
            Chime::Failed => 0.3,
        }
    }

    /// Mono samples at `RATE`, made once.
    pub(super) fn samples(self) -> Arc<Vec<f32>> {
        static MADE: [OnceLock<Arc<Vec<f32>>>; 3] = [const { OnceLock::new() }; 3];
        MADE[self as usize]
            .get_or_init(|| Arc::new(self.make()))
            .clone()
    }

    fn make(self) -> Vec<f32> {
        use std::f32::consts::{PI, TAU};
        let rate = RATE as f32;
        let seconds = match self {
            Chime::Arrive => 0.16,
            Chime::Done => 0.85,
            Chime::Failed => 0.42,
        };
        let n = (seconds * rate) as usize;
        let mut phase = 0.0f32;
        let mut out = Vec::with_capacity(n);
        // A soft round note, as a mallet on wood: its tone, a breath of the octave gone at once,
        // swelling in over a few milliseconds and dying away slowly.
        let bell = |f: f32, t: f32| {
            if t < 0.0 {
                return 0.0;
            }
            let attack = (t / 0.018).min(1.0);
            let swell = attack * attack * (3.0 - 2.0 * attack);
            swell
                * (-t * 4.2).exp()
                * ((TAU * f * t).sin() + 0.12 * (TAU * 2.0 * f * t).sin() * (-t * 18.0).exp())
        };
        // A soft low note: its tone and an odd overtone, like a rounded square.
        let low = |f: f32, t: f32, length: f32| {
            if !(0.0..length).contains(&t) {
                return 0.0;
            }
            let attack = (t / 0.006).min(1.0);
            let release = ((length - t) / 0.03).min(1.0);
            attack
                * release
                * (-t * 4.0).exp()
                * ((TAU * f * t).sin() + 0.3 * (TAU * 3.0 * f * t).sin())
        };
        for i in 0..n {
            let t = i as f32 / rate;
            let k = t / seconds;
            let sample = match self {
                Chime::Arrive => {
                    phase += (420.0 + 520.0 * k) / rate;
                    (phase * TAU).sin() * (PI * k).sin().powi(2) * 0.8
                }
                // E5, then A5 a beat later.
                Chime::Done => 0.5 * bell(659.3, t) + 0.5 * bell(880.0, t - 0.12),
                // E4, then B3.
                Chime::Failed => 0.45 * low(329.6, t, 0.17) + 0.45 * low(246.9, t - 0.19, 0.2),
            };
            out.push(sample.clamp(-1.0, 1.0));
        }
        out
    }
}

/// What the banner shows, and its moves.
struct Banner {
    title: String,
    detail: String,
    state: State,
    /// When it began coming down.
    entered: Instant,
    /// When the import ended (Done or Failed): the pop, then the time it stays.
    ended: Option<Instant>,
}

/// `easeOutBack`: past the end a little, then back.
fn ease_out_back(x: f32) -> f32 {
    let (c1, c3) = (1.70158, 2.70158);
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

impl Banner {
    /// Going back up, or gone.
    fn leaving(&self, now: Instant) -> bool {
        self.ended.is_some_and(|e| now >= e + SHOWN)
    }

    fn gone(&self, now: Instant) -> bool {
        self.ended.is_some_and(|e| now >= e + SHOWN + LEAVE)
    }

    /// How far down it is: 0 above the screen, 1 in place (a little more as it bounces); None
    /// once gone.
    fn drop(&self, now: Instant) -> Option<f32> {
        if self.gone(now) {
            return None;
        }
        let down = now.saturating_duration_since(self.entered).as_secs_f32() / ENTER.as_secs_f32();
        let mut drop = ease_out_back(down.min(1.0));
        if let Some(ended) = self.ended
            && now >= ended + SHOWN
        {
            let up =
                now.saturating_duration_since(ended + SHOWN).as_secs_f32() / LEAVE.as_secs_f32();
            drop = drop.min(1.0) * (1.0 - up.min(1.0).powi(3));
        }
        Some(drop)
    }

    /// The icon's size, 1 at rest: it pops as the import ends.
    fn pop(&self, now: Instant) -> f32 {
        match self.ended {
            Some(ended) => {
                let k = now.saturating_duration_since(ended).as_secs_f32() / POP.as_secs_f32();
                0.55 + 0.45 * ease_out_back(k.min(1.0))
            }
            None => 1.0,
        }
    }

    /// What is left of its time down, for the bar: 1 while it works.
    fn left(&self, now: Instant) -> f32 {
        self.ended.map_or(1.0, |e| {
            1.0 - (now.saturating_duration_since(e).as_secs_f32() / SHOWN.as_secs_f32()).min(1.0)
        })
    }
}

/// The inbox and its banner: held by the loading screen and the game alike.
#[derive(Default)]
pub(super) struct Imports {
    inbox: Option<Inbox>,
    polled: Option<Instant>,
    banner: Option<Banner>,
    /// A skin the inbox just put in `skins/`, for the game to put on.
    skin: Option<PathBuf>,
    /// Sounds for the caller to play (`take_sounds`).
    sounds: Vec<Chime>,
    /// The last sound of each kind, so a burst of imports does not ring over itself.
    rung: [Option<Instant>; 3],
}

impl Imports {
    /// Look at the inbox where a platform fills it (Android); the kind of an import that just
    /// finished well. `resources` is the prepared resources' folder (the vanilla root).
    pub(super) fn poll(&mut self, resources: &Path) -> Option<Kind> {
        if !horizon_client::platform::registered()
            || self.polled.is_some_and(|t| t.elapsed() < POLL)
        {
            return None;
        }
        self.polled = Some(Instant::now());
        let inbox = self.inbox.get_or_insert_with(|| {
            Inbox::new(
                import::inbox_dir(),
                Places {
                    game: horizon_client::logging::game_home(),
                    resources: resources.to_path_buf(),
                },
            )
        });
        let event = inbox.poll()?;
        let now = Instant::now();
        let mut finished = None;
        match event {
            Event::Receiving { name, bytes } => self.show(
                now,
                "Receiving a file",
                format!("{} · {:.0} MB", shown(&name), bytes as f64 / 1e6),
                State::Working,
            ),
            Event::Installing { name } => {
                self.show(now, "Importing", shown(&name).to_owned(), State::Working)
            }
            Event::Done(installed) => {
                log::info!("Horizon: import: {}", installed.message);
                finished = Some(installed.kind);
                if installed.kind == Kind::Skin {
                    self.skin = installed.path.clone();
                }
                self.show(
                    now,
                    done_title(installed.kind),
                    installed.message,
                    State::Done(Some(installed.kind)),
                );
            }
            Event::Failed { name, error } => {
                log::warn!("Horizon: import of {name} failed: {error}");
                self.show(now, "Could not import", error, State::Failed);
            }
        }
        finished
    }

    /// The banner says `title` and `detail` as `state`: it drops in (with its sound) if it is not
    /// down already, and an ending rings.
    fn show(&mut self, now: Instant, title: &str, detail: String, state: State) {
        let fresh = self.banner.as_ref().is_none_or(|b| b.leaving(now));
        let ended = (state != State::Working).then_some(now);
        let entered = match &self.banner {
            Some(b) if !fresh => b.entered,
            _ => now,
        };
        let chime = match state {
            State::Working if fresh => Some(Chime::Arrive),
            State::Working => None,
            State::Done(_) => Some(Chime::Done),
            State::Failed => Some(Chime::Failed),
        };
        if let Some(chime) = chime {
            let last = &mut self.rung[chime as usize];
            if last.is_none_or(|t| now.saturating_duration_since(t) >= AGAIN) {
                self.sounds.push(chime);
                *last = Some(now);
            }
        }
        self.banner = Some(Banner {
            title: title.to_owned(),
            detail: sentence(detail),
            state,
            entered,
            ended,
        });
    }

    /// The banner says something went well, in the caller's words (a picture taken from the
    /// inbox, the resources needing a restart).
    pub(super) fn say(&mut self, title: &str, detail: String) {
        self.show(Instant::now(), title, detail, State::Done(None));
    }

    /// The banner says what went wrong.
    pub(super) fn say_failed(&mut self, detail: String) {
        self.show(Instant::now(), "Could not import", detail, State::Failed);
    }

    /// The sounds since the last call, for the caller's audio.
    pub(super) fn take_sounds(&mut self) -> Vec<Chime> {
        std::mem::take(&mut self.sounds)
    }

    /// Whether the banner is on screen, so the caller draws often enough for it to move.
    pub(super) fn animating(&self) -> bool {
        self.banner
            .as_ref()
            .is_some_and(|b| !b.gone(Instant::now()))
    }

    /// The banner across the top of the screen, while an import runs and for a while after.
    pub(super) fn draw(&self, atlas: &HudAtlas, batch: &mut HudBatch) {
        let Some(banner) = &self.banner else { return };
        let now = Instant::now();
        let Some(drop) = banner.drop(now) else { return };
        let mut ui = Ui::new(atlas, batch, (-1.0, -1.0));
        let w = ui.w;
        let (pad, tile, bar) = (28.0, 60.0, 6.0);
        let text_x = pad + tile + 20.0;
        let lines = fit(&ui, &banner.detail, w - text_x - pad, 2);
        let h = if lines.len() > 1 { 118.0 } else { 96.0 };
        let shadow = 12.0;
        let y = -(h + shadow) * (1.0 - drop);
        let accent = banner.state.color();
        // Reaching above the top, so its bounce shows no gap; tinted with its state's colour, so it
        // stands out from any screen under it.
        ui.fill(
            0.0,
            y - 30.0,
            w,
            h + 30.0,
            color::with_alpha(color::BG, 0.98),
        );
        ui.fill(0.0, y - 30.0, w, h + 30.0, color::with_alpha(accent, 0.18));
        for (i, alpha) in [0.32, 0.18, 0.09, 0.04].into_iter().enumerate() {
            ui.fill(
                0.0,
                y + h + i as f32 * 3.0,
                w,
                3.0,
                color::hex(0x000000, (alpha * 255.0) as u8),
            );
        }
        // The bar along its bottom: a light running across while it works, then the time left.
        let bar_y = y + h - bar;
        if banner.state == State::Working {
            ui.fill(0.0, bar_y, w, bar, color::with_alpha(accent, 0.3));
            let run = now.saturating_duration_since(banner.entered).as_secs_f32() % 1.2 / 1.2;
            let width = w * 0.28;
            let x = -width + run * (w + width);
            ui.fill(
                x.max(0.0),
                bar_y,
                (x + width).min(w) - x.max(0.0),
                bar,
                accent,
            );
        } else {
            ui.fill(0.0, bar_y, w * banner.left(now), bar, accent);
        }
        // The icon on its tile: a spinner while it works, what came in (with a tick), or "!".
        let mut tile_x = pad;
        let tile_y = y + (h - bar - tile) / 2.0;
        let since_end = banner
            .ended
            .map(|e| now.saturating_duration_since(e).as_secs_f32());
        if banner.state == State::Failed
            && let Some(t) = since_end.filter(|t| *t < 0.4)
        {
            // A shake, as a "no".
            tile_x += (t * 55.0).sin() * 7.0 * (1.0 - t / 0.4);
        }
        ui.rounded(
            tile_x,
            tile_y,
            tile,
            tile,
            15.0,
            color::with_alpha(accent, 0.3),
        );
        let (cx, cy) = (tile_x + tile / 2.0, tile_y + tile / 2.0);
        let scale = banner.pop(now);
        match banner.state {
            State::Working => spinner(
                &mut ui,
                cx,
                cy,
                now.saturating_duration_since(banner.entered),
                accent,
            ),
            State::Done(kind) => {
                let (art, cell) = kind.map_or((CHECK, 4.6), |k| (icon(k), 4.0));
                pixels(&mut ui, art, cx, cy, cell * scale);
                if kind.is_some() {
                    // The tick, popping a little after the icon.
                    let late = since_end.map_or(1.0, |t| ((t - 0.12) / 0.25).clamp(0.0, 1.0));
                    if late > 0.0 {
                        let s = 0.5 + 0.5 * ease_out_back(late);
                        let (bx, by) = (tile_x + tile - 8.0, tile_y + tile - 8.0);
                        let r = 12.0 * s;
                        ui.rounded(bx - r, by - r, 2.0 * r, 2.0 * r, r, color::SUCCESS);
                        pixels(&mut ui, TICK, bx, by, 2.4 * s);
                    }
                }
            }
            State::Failed => pixels(&mut ui, BANG, cx, cy, 4.6 * scale),
        }
        let title_y = y + 18.0;
        ui.text(
            Face::Semibold,
            20.0,
            text_x,
            title_y,
            color::TEXT,
            &banner.title,
        );
        for (i, line) in lines.iter().enumerate() {
            let ly = title_y + 31.0 + i as f32 * 21.0;
            ui.text(Face::Regular, DETAIL, text_x, ly, color::MUTED, line);
        }
        ui.finish();
    }
}

/// `text` with a capital first letter, as the banner's line starts (an import's error begins
/// lower case, written to follow "could not import ...: ").
fn sentence(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => first.to_uppercase().chain(chars).collect(),
        _ => text,
    }
}

/// A file's name as the banner shows it: a zip's without ".zip" (a world zipped on its way in is
/// still its folder's name).
fn shown(name: &str) -> &str {
    name.strip_suffix(".zip").unwrap_or(name)
}

/// The banner's title for what an import added.
fn done_title(kind: Kind) -> &'static str {
    match kind {
        Kind::World => "World imported",
        Kind::ResourcePack => "Resource pack imported",
        Kind::DataPack => "Data pack imported",
        Kind::ShaderPack => "Shader pack imported",
        Kind::Mod | Kind::NativeMod => "Mod imported",
        Kind::Skin => "Skin imported",
        Kind::Resources => "Minecraft resources imported",
        Kind::Bedrock | Kind::Unknown => "Imported",
    }
}

/// `text` in at most `most` lines no wider than `max`, the last cut short with "..." if it does
/// not all fit.
fn fit(ui: &Ui<'_>, text: &str, max: f32, most: usize) -> Vec<String> {
    let wide = |s: &str| ui.measure(Face::Regular, DETAIL, s) > max;
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if line.is_empty() || !wide(&next) {
            line = next;
            continue;
        }
        if lines.len() + 1 == most {
            // The last line: as much as fits, then "...".
            let mut rest = next;
            while !rest.is_empty() && wide(&format!("{rest}...")) {
                rest.pop();
            }
            lines.push(format!("{}...", rest.trim_end()));
            return lines;
        }
        lines.push(std::mem::replace(&mut line, word.to_owned()));
    }
    if !line.is_empty() {
        if wide(&line) {
            let mut rest = line;
            while !rest.is_empty() && wide(&format!("{rest}...")) {
                rest.pop();
            }
            line = format!("{rest}...");
        }
        lines.push(line);
    }
    lines
}

/// Eight dots round the tile's middle, the brightest going round.
fn spinner(ui: &mut Ui<'_>, cx: f32, cy: f32, running: Duration, ink: u32) {
    let turn = running.as_secs_f32() / 0.9;
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let behind = (turn * 8.0 - i as f32).rem_euclid(8.0) / 8.0;
        let alpha = 0.25 + 0.75 * (1.0 - behind).powi(2);
        let (x, y) = (cx + a.cos() * 17.0, cy + a.sin() * 17.0);
        ui.rounded(
            x - 3.7,
            y - 3.7,
            7.4,
            7.4,
            3.7,
            color::with_alpha(ink, alpha),
        );
    }
}

/// Our own pixel icons, a cell to a character (palette in `shade`; '.' is clear).
const WORLD: &[&str] = &[
    "GGGGGGGGGG",
    "GgGGGgGGgG",
    "GGgGGGGgGG",
    "gDgGgDgGDg",
    "DDdDDDdDDD",
    "DdDDdDDDdD",
    "DDDdDDdDDd",
    "dDDDDdDDDD",
    "DDdDDDDdDD",
    "DDDDdDDDDD",
];
const PACK: &[&str] = &[
    "KKKKKKKKKK",
    "KSSSSSSSSK",
    "KSSSSSSYSK",
    "KSSSSSSSSK",
    "KSSSMSSSSK",
    "KSSMMMSSSK",
    "KSMMMMMSSK",
    "KGMMMMMMGK",
    "KGGGGGGGGK",
    "KKKKKKKKKK",
];
const GEAR: &[&str] = &[
    "....OO....",
    ".O.OOOO.O.",
    "..OOOOOO..",
    ".OOO..OOO.",
    "OOO....OOO",
    "OOO....OOO",
    ".OOO..OOO.",
    "..OOOOOO..",
    ".O.OOOO.O.",
    "....OO....",
];
const FACE: &[&str] = &[
    "HHHHHHHHHH",
    "HHHHHHHHHH",
    "HSSSSSSSSH",
    "SSSSSSSSSS",
    "SWESSSSEWS",
    "SSSSSSSSSS",
    "SSSSSSSSSS",
    "SSSmmmmSSS",
    "SSSSSSSSSS",
    "SSSSSSSSSS",
];
const BOX: &[&str] = &[
    "..BBBBBB..",
    ".BbbbTbbB.",
    "BBBBBTBBBB",
    "BbbbbTbbbB",
    "BbbbbTbbbB",
    "BbbbbbbbbB",
    "BbbbbbbbbB",
    "BbbbbbbbbB",
    "BBBBBBBBBB",
];
const CHECK: &[&str] = &[
    ".........C",
    "........CC",
    ".......CC.",
    "C.....CC..",
    "CC...CC...",
    ".CC.CC....",
    "..CCC.....",
    "...C......",
];
const TICK: &[&str] = &["....W", "...WW", "W.WW.", "WWW..", ".W..."];
const BANG: &[&str] = &[
    "..RR..", "..RR..", "..RR..", "..RR..", "..RR..", "......", "..RR..", "..RR..",
];

fn icon(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::World => WORLD,
        Kind::ResourcePack | Kind::DataPack | Kind::ShaderPack => PACK,
        Kind::Mod | Kind::NativeMod => GEAR,
        Kind::Skin => FACE,
        Kind::Resources | Kind::Bedrock | Kind::Unknown => BOX,
    }
}

fn shade(c: u8) -> Option<u32> {
    Some(match c {
        b'G' => color::hex(0x6abf40, 255),
        b'g' => color::hex(0x4f9a2c, 255),
        b'D' => color::hex(0x8f5d37, 255),
        b'd' => color::hex(0x704529, 255),
        b'K' => color::hex(0x7a5530, 255),
        b'S' => color::hex(0x86c8f2, 255),
        b'Y' => color::hex(0xffd84a, 255),
        b'M' => color::hex(0x8d94a6, 255),
        b'O' => color::hex(0xc3cad6, 255),
        b'H' => color::hex(0x5b3a24, 255),
        b'E' => color::hex(0x2f4a8c, 255),
        b'm' => color::hex(0x9c4b3c, 255),
        b'B' => color::hex(0xa8763e, 255),
        b'b' => color::hex(0xc99554, 255),
        b'T' => color::hex(0xeadbb0, 255),
        b'W' => color::WHITE,
        b'C' => color::SUCCESS,
        b'R' => color::DANGER,
        _ => return None,
    })
}

/// A pixel icon centred on (`cx`, `cy`), `cell` UI units to a pixel.
fn pixels(ui: &mut Ui<'_>, art: &[&str], cx: f32, cy: f32, cell: f32) {
    let rows = art.len() as f32;
    let cols = art.first().map_or(0, |r| r.len()) as f32;
    let (x0, y0) = (cx - cols * cell / 2.0, cy - rows * cell / 2.0);
    for (r, row) in art.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if let Some(ink) = shade(ch) {
                let (x, y) = (x0 + c as f32 * cell, y0 + r as f32 * cell);
                // A hair over a cell, so no seam shows between them.
                ui.fill(x, y, cell + 0.05, cell + 0.05, ink);
            }
        }
    }
}

impl super::Game<'_> {
    /// The inbox, a few times a second, from the frame; the banner's sounds.
    pub(super) fn poll_imports(&mut self) {
        if !self.button_image_wait() {
            match self.imports.poll(&self.ui.vanilla_root.clone()) {
                Some(Kind::Resources) => self.imports.say(
                    "Minecraft resources imported",
                    "Restart the game to use them".into(),
                ),
                Some(Kind::ResourcePack) => self.refresh_pack_lists(),
                // The Mods page lists it at once; a mod with code runs as soon as it is turned on.
                Some(Kind::Mod | Kind::NativeMod) => self.start_mods(),
                // A skin is worn at once (Skin Customization's Choose a File...).
                Some(Kind::Skin) => {
                    if let Some(path) = self.imports.skin.take() {
                        self.own_skin_imported(&path);
                    }
                }
                _ => {}
            }
        }
        for chime in self.imports.take_sounds() {
            self.audio.play_samples(chime.samples(), RATE, chime.gain());
        }
    }

    pub(super) fn draw_imports(&self, batch: &mut HudBatch) {
        self.imports.draw(&self.gui.atlas, batch);
    }
}

impl super::Loading {
    /// The banner's sounds while the game is set up: an output of its own, as the fishing's, the
    /// game's own sounds not being there yet.
    pub(super) fn play_import_sounds(&mut self) {
        let chimes = self.imports.take_sounds();
        if chimes.is_empty() {
            return;
        }
        let audio = self.audio.get_or_insert_with(|| {
            let mut audio = super::Audio::with_registry(&self.vanilla_root, Default::default());
            audio.master = self.settings.master_volume;
            audio
        });
        for chime in chimes {
            audio.play_samples(chime.samples(), RATE, chime.gain());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sounds_are_short_and_never_clip() {
        for chime in [Chime::Arrive, Chime::Done, Chime::Failed] {
            let samples = chime.samples();
            assert!(
                !samples.is_empty() && samples.len() < RATE as usize,
                "{chime:?}"
            );
            assert!(samples.iter().all(|s| s.abs() <= 1.0), "{chime:?}");
            assert!(
                samples.iter().any(|s| s.abs() > 0.2),
                "{chime:?} is audible"
            );
        }
    }

    #[test]
    fn it_drops_in_with_a_sound_rings_once_and_goes_back_up() {
        let mut imports = Imports::default();
        let t0 = Instant::now();
        imports.show(t0, "Importing", "a".into(), State::Working);
        assert_eq!(imports.take_sounds(), [Chime::Arrive]);
        // Above the screen as it starts, past its place as it bounces, then in place.
        let banner = imports.banner.as_ref().unwrap();
        assert!(banner.drop(t0).unwrap().abs() < 1e-4);
        let bounce = banner.drop(t0 + ENTER.mul_f32(0.7)).unwrap();
        assert!(bounce > 1.0, "{bounce}");
        assert_eq!(banner.drop(t0 + ENTER), Some(1.0));
        // Still working: no second drop, no sound.
        let t1 = t0 + Duration::from_millis(500);
        imports.show(t1, "Importing", "a".into(), State::Working);
        assert!(imports.take_sounds().is_empty());
        let t2 = t0 + Duration::from_secs(1);
        imports.show(
            t2,
            "World imported",
            "b".into(),
            State::Done(Some(Kind::World)),
        );
        assert_eq!(imports.take_sounds(), [Chime::Done]);
        let banner = imports.banner.as_ref().unwrap();
        assert_eq!(banner.entered, t0, "it stays down");
        assert!(banner.pop(t2) < 0.6 && (banner.pop(t2 + POP) - 1.0).abs() < 1e-4);
        assert_eq!(banner.drop(t2 + SHOWN), Some(1.0));
        assert!(banner.drop(t2 + SHOWN + LEAVE.mul_f32(0.5)).unwrap() < 1.0);
        assert_eq!(banner.drop(t2 + SHOWN + LEAVE), None);
        // The next import, once it went up, drops in again.
        let t3 = t2 + SHOWN + LEAVE;
        imports.show(t3, "Importing", "c".into(), State::Working);
        assert_eq!(imports.take_sounds(), [Chime::Arrive]);
        assert_eq!(imports.banner.as_ref().unwrap().entered, t3);
    }

    #[test]
    fn a_burst_of_imports_does_not_ring_over_itself() {
        let mut imports = Imports::default();
        let t0 = Instant::now();
        imports.show(
            t0,
            "World imported",
            "a".into(),
            State::Done(Some(Kind::World)),
        );
        imports.show(
            t0 + Duration::from_millis(200),
            "World imported",
            "b".into(),
            State::Done(None),
        );
        assert_eq!(imports.take_sounds(), [Chime::Done]);
        imports.show(t0 + AGAIN, "Could not import", "c".into(), State::Failed);
        imports.show(
            t0 + AGAIN * 2,
            "World imported",
            "d".into(),
            State::Done(None),
        );
        assert_eq!(imports.take_sounds(), [Chime::Failed, Chime::Done]);
    }

    #[test]
    fn its_lines_start_with_a_capital_and_zips_go_by_their_names() {
        assert_eq!(
            sentence("the game does not know".into()),
            "The game does not know"
        );
        assert_eq!(sentence("World \"a\" added".into()), "World \"a\" added");
        assert_eq!(sentence(String::new()), "");
        assert_eq!(shown("New World.zip"), "New World");
        assert_eq!(shown("skin.png"), "skin.png");
    }

    #[test]
    fn every_icon_is_square_enough_and_painted() {
        for art in [WORLD, PACK, GEAR, FACE, BOX, CHECK, TICK, BANG] {
            let width = art[0].len();
            assert!(art.iter().all(|r| r.len() == width), "{art:?}");
            for row in art {
                for c in row.bytes() {
                    assert!(c == b'.' || shade(c).is_some(), "{} in {art:?}", c as char);
                }
            }
        }
    }
}
