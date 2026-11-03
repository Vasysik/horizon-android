//! Fishing, to pass the time while the game is set up (setup.rs), seen from above: water over the
//! whole screen. Hold a finger on it to cast and keep it there; a fish comes toward the bobber, a
//! trail of water across the surface, and bites: lift the finger then. Too soon and it swims off;
//! too late and it gets away, and another comes while the finger stays down. What to do is written
//! until the first catch. Its sounds are made here, soft: the game's own are part of what is being
//! set up.
use horizon_client::ui_theme::{Face, Ui, color};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// The water's pixel art: one cell, in dp, and its four shades, darkest first.
const CELL: f32 = 12.0;
const WATER: [u32; 4] = [
    color::hex(0x2654b0, 255),
    color::hex(0x2e62c4, 255),
    color::hex(0x3a72d4, 255),
    color::hex(0x4e88e2, 255),
];
const RIPPLE: u32 = color::hex(0x6ea6ee, 255);
const SPRAY: u32 = color::hex(0xe6f2ff, 255);
const LINE: u32 = color::hex(0x14161e, 255);
/// Our own float from above, 8x8: an outline, a red cap with its shine, a white body.
const BOBBER: [&str; 8] = [
    "..oooo..", ".oRRRRo.", "oRRhRRRo", "oRRRRRRo", "oWWWWWWo", "oWWWWWWo", ".oWWWWo.", "..oooo..",
];
/// How long the bobber flies out, and how long a bite lasts.
const CAST: Duration = Duration::from_millis(500);
const BITE: Duration = Duration::from_millis(750);
/// How long the last catch (or miss) stays written.
const SAID: Duration = Duration::from_millis(1600);

/// What a cast brings up, and how often in a hundred (vanilla's fish, junk and treasure).
const CATCHES: &[(&str, u32)] = &[
    ("Raw Cod", 51),
    ("Raw Salmon", 21),
    ("Pufferfish", 11),
    ("Tropical Fish", 2),
    ("an Old Boot", 6),
    ("a Stick", 4),
    ("a Name Tag", 2),
    ("an Enchanted Book", 2),
    ("a Nautilus Shell", 1),
];

/// A fish from above, 12x6 cells facing left: a shadow under the water with a light eye. The
/// second row of each pair is the tail's other beat.
const FISH: [&str; 6] = [
    "...ddddd....",
    ".dddddddd..d",
    "dedddddddddd",
    "dddddddddddd",
    ".dddddddd..d",
    "...ddddd....",
];
const FISH_SWISH: [&str; 6] = [
    "...ddddd....",
    ".dddddddd...",
    "deddddddddd.",
    "ddddddddddd.",
    ".dddddddd.dd",
    "...ddddd...d",
];
const FISH_CELL: f32 = 4.0;

/// What the fishing sounds like.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::game) enum Sound {
    /// The line thrown.
    Cast,
    /// The bobber on the water, or a fish gone.
    Plop,
    /// A bubble as the fish comes.
    Blip,
    /// It bites.
    Splash,
    /// Caught.
    Catch,
}

/// The samples' rate.
pub(in crate::game) const RATE: u32 = 22_050;

impl Sound {
    /// Mono samples at `RATE`, made once.
    pub(in crate::game) fn samples(self) -> Arc<Vec<f32>> {
        static MADE: [OnceLock<Arc<Vec<f32>>>; 5] = [const { OnceLock::new() }; 5];
        MADE[self as usize]
            .get_or_init(|| Arc::new(self.make()))
            .clone()
    }

    fn make(self) -> Vec<f32> {
        let rate = RATE as f32;
        let mut seed = 0x2f6b_4e1du32;
        let mut noise = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32 * 2.0 - 1.0
        };
        let (seconds, mut low, mut phase) = (
            match self {
                Sound::Cast => 0.28,
                Sound::Plop => 0.14,
                Sound::Blip => 0.07,
                Sound::Splash => 0.4,
                Sound::Catch => 0.3,
            },
            0.0f32,
            0.0f32,
        );
        let n = (seconds * rate) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / rate;
            let k = t / seconds;
            let sample = match self {
                // Air rushing past: softened noise, up and down.
                Sound::Cast => {
                    low += (noise() - low) * (0.08 + 0.25 * k);
                    low * (std::f32::consts::PI * k).sin() * 0.9
                }
                // A drop into water: a tone falling fast.
                Sound::Plop => {
                    phase += (520.0 - 340.0 * k) / rate;
                    (phase * std::f32::consts::TAU).sin() * (-t * 32.0).exp() * 0.7
                }
                Sound::Blip => {
                    phase += (700.0 + 700.0 * k) / rate;
                    (phase * std::f32::consts::TAU).sin()
                        * (std::f32::consts::PI * k).sin().powi(2)
                        * 0.35
                }
                Sound::Splash => {
                    low += (noise() - low) * 0.3;
                    phase += (240.0 - 120.0 * k) / rate;
                    low * (-t * 9.0).exp() * 0.9
                        + (phase * std::f32::consts::TAU).sin() * (-t * 20.0).exp() * 0.4
                }
                // A pop, then a short bright ring.
                Sound::Catch => {
                    let f = if t < 0.08 { 600.0 + 7500.0 * t } else { 980.0 };
                    phase += f / rate;
                    (phase * std::f32::consts::TAU).sin()
                        * (t / 0.01).min(1.0)
                        * (-t * 9.0).exp()
                        * 0.5
                }
            };
            out.push(sample);
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// The line in: a finger on the water casts.
    Idle,
    /// The bobber on its way out.
    Cast,
    /// Floating; a fish comes at `next`.
    Waiting { next: Instant },
    /// A fish swims to the bobber from `from` (dp from it, across the surface), arriving at `at`.
    Coming { from: [f32; 2], at: Instant },
    /// It bites until `until`: lift now.
    Bite { until: Instant },
}

pub(in crate::game) struct Fishing {
    phase: Phase,
    /// When the phase began.
    since: Instant,
    /// Where the bobber lands (dp): where the finger went down.
    spot: [f32; 2],
    caught: u32,
    said: Option<(String, Instant)>,
    seed: u64,
    /// When it began, for the water's moves.
    started: Instant,
    /// Sounds to play, and when the fish last blew a bubble.
    sounds: Vec<Sound>,
    blip: Instant,
}

impl Default for Fishing {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)
            | 1;
        Self {
            phase: Phase::Idle,
            since: Instant::now(),
            spot: [0.0, 0.0],
            caught: 0,
            said: None,
            seed,
            started: Instant::now(),
            sounds: Vec::new(),
            blip: Instant::now(),
        }
    }
}

/// 0..1 for a cell, the same every frame.
fn grain(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b9) ^ (y as u32).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

impl Fishing {
    /// 0..1, xorshift.
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / (1u64 << 24) as f32
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.random()
    }

    fn after(&mut self, now: Instant, low: f32, high: f32) -> Instant {
        now + Duration::from_secs_f32(self.between(low, high))
    }

    fn set(&mut self, phase: Phase, now: Instant) {
        self.phase = phase;
        self.since = now;
    }

    fn say(&mut self, text: String, now: Instant) {
        self.said = Some((text, now));
    }

    /// A finger down on the water at `at` (dp): the bobber goes there.
    pub(in crate::game) fn press(&mut self, now: Instant, at: [f32; 2]) {
        if self.phase == Phase::Idle {
            self.spot = at;
            self.sounds.push(Sound::Cast);
            self.set(Phase::Cast, now);
        }
    }

    /// The sounds since the last call.
    pub(in crate::game) fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// The finger lifted.
    pub(in crate::game) fn release(&mut self, now: Instant) {
        match self.phase {
            Phase::Bite { .. } => {
                let mut roll = self.random() * 100.0;
                let catch = CATCHES
                    .iter()
                    .find(|(_, weight)| {
                        roll -= *weight as f32;
                        roll < 0.0
                    })
                    .map_or(CATCHES[0].0, |c| c.0);
                self.caught += 1;
                self.sounds.push(Sound::Catch);
                self.say(format!("You caught {catch}!"), now);
            }
            Phase::Coming { .. } => self.say("Too soon: it swam off.".into(), now),
            Phase::Idle => return,
            Phase::Cast | Phase::Waiting { .. } => {}
        }
        self.set(Phase::Idle, now);
    }

    /// The fish's moves, every frame.
    pub(in crate::game) fn update(&mut self, now: Instant) {
        if self
            .said
            .as_ref()
            .is_some_and(|(_, at)| now.duration_since(*at) > SAID)
        {
            self.said = None;
        }
        match self.phase {
            Phase::Cast if now.duration_since(self.since) >= CAST => {
                self.sounds.push(Sound::Plop);
                let next = self.after(now, 1.5, 6.0);
                self.set(Phase::Waiting { next }, now);
            }
            Phase::Waiting { next } if now >= next => {
                let angle = self.between(0.0, std::f32::consts::TAU);
                let distance = self.between(220.0, 380.0);
                let from = [angle.cos() * distance, angle.sin() * distance * 0.6];
                let at = self.after(now, 1.4, 2.4);
                self.set(Phase::Coming { from, at }, now);
            }
            Phase::Coming { at, .. } if now >= at => {
                self.sounds.push(Sound::Splash);
                self.set(Phase::Bite { until: now + BITE }, now);
            }
            Phase::Coming { .. } if now.duration_since(self.blip).as_secs_f32() > 0.3 => {
                self.blip = now;
                self.sounds.push(Sound::Blip);
            }
            Phase::Bite { until } if now >= until => {
                self.sounds.push(Sound::Plop);
                self.say("It got away.".into(), now);
                let next = self.after(now, 1.5, 5.0);
                self.set(Phase::Waiting { next }, now);
            }
            _ => {}
        }
    }

    /// The water under everything, `w` x `h` dp.
    pub(in crate::game) fn draw_water(&self, ui: &mut Ui<'_>, w: f32, h: f32, now: Instant) {
        let t = now.duration_since(self.started).as_secs_f32();
        ui.fill(0.0, 0.0, w, h, WATER[1]);
        let (cols, rows) = ((w / CELL).ceil() as i32, (h / CELL).ceil() as i32);
        for gy in 0..rows {
            for gx in 0..cols {
                let (x, y) = (gx as f32, gy as f32);
                let v = (x * 0.11 + (y * 0.23 + t * 0.5).sin() * 2.2 + t * 0.35).sin() * 0.55
                    + (x * 0.05 - y * 0.17 + t * 0.25).sin() * 0.35
                    + (grain(gx, gy) - 0.5) * 0.35;
                let shade = match v {
                    v if v < -0.45 => 0,
                    v if v < 0.25 => continue,
                    v if v < 0.7 => 2,
                    _ => 3,
                };
                ui.fill(x * CELL, y * CELL, CELL, CELL, WATER[shade]);
            }
        }
    }

    /// The line, the bobber, the fish and the splash over the water, and what to do, written at
    /// `hint_y` (dp); the catches counted at the top right.
    pub(in crate::game) fn draw(&self, ui: &mut Ui<'_>, w: f32, h: f32, hint_y: f32, now: Instant) {
        let t = now.duration_since(self.since).as_secs_f32();
        let landed = self.spot;
        // The player is below the screen, a little left of the middle.
        let hand = [w * 0.42, h + 12.0];
        let (bobber, size) = match self.phase {
            Phase::Idle => (None, 1.0),
            Phase::Cast => {
                let k = (t / CAST.as_secs_f32()).min(1.0);
                let lift = 1.0 - (2.0 * k - 1.0).powi(2);
                (
                    Some([
                        hand[0] + (landed[0] - hand[0]) * k,
                        hand[1] + (landed[1] - hand[1]) * k,
                    ]),
                    1.0 + lift * 0.6,
                )
            }
            _ => (Some(landed), 1.0),
        };
        if let Some(b) = bobber {
            // Ripples: slow while it floats, quick rings when it bites.
            let bite = matches!(self.phase, Phase::Bite { .. });
            if !matches!(self.phase, Phase::Cast) {
                let rings: &[f32] = if bite { &[0.0, 0.33, 0.66] } else { &[0.0] };
                for &delay in rings {
                    let period = if bite { 0.5 } else { 2.2 };
                    let k = ((t / period + delay) % 1.0).max(0.0);
                    ring(ui, b, 18.0 + k * if bite { 60.0 } else { 40.0 }, 1.0 - k);
                }
            }
            // The line in steps, from the hand to the bobber.
            let steps = ((b[0] - hand[0]).hypot(b[1] - hand[1]) / 4.0) as i32;
            for i in 0..steps {
                let k = i as f32 / steps as f32;
                let px = hand[0] + (b[0] - hand[0]) * k;
                let py = hand[1] + (b[1] - hand[1]) * k;
                ui.fill(px - 1.5, py - 1.5, 3.0, 3.0, LINE);
            }
        }
        // The fish swims to the bobber under the water, rings and bubbles behind it; biting, it
        // stays under the bobber, its tail beating fast.
        match (self.phase, bobber) {
            (Phase::Coming { from, at }, Some(b)) => {
                let total = at.duration_since(self.since).as_secs_f32().max(0.1);
                let path = |time: f32| {
                    let k = (time / total).clamp(0.0, 1.0);
                    let k = k * k * (3.0 - 2.0 * k);
                    let across = (time * 5.0).sin() * 10.0 * (1.0 - k);
                    let len = from[0].hypot(from[1]).max(1.0);
                    [
                        b[0] + from[0] * (1.0 - k) - from[1] / len * across,
                        b[1] + from[1] * (1.0 - k) + from[0] / len * across,
                    ]
                };
                for j in 1..=4 {
                    let then = t - j as f32 * 0.3;
                    if then > 0.0 {
                        ring(ui, path(then), 6.0 + j as f32 * 9.0, 0.55 - j as f32 * 0.12);
                    }
                }
                let at = path(t);
                for i in 0..3 {
                    let rise = ((t * 2.0 + i as f32 * 0.37) % 1.0) * 14.0;
                    let x = at[0] + [-14.0, 6.0, 16.0][i] * if from[0] > 0.0 { 1.0 } else { -1.0 };
                    let s = [3.0, 4.0, 3.0][i];
                    ui.rounded(
                        x,
                        at[1] - 8.0 - rise,
                        s,
                        s,
                        s / 2.0,
                        color::with_alpha(SPRAY, 0.8),
                    );
                }
                fish(ui, at, from[0] > 0.0, (t * 6.0) as i32);
            }
            (Phase::Bite { .. }, Some(b)) => {
                fish(ui, [b[0] + 10.0, b[1] + 8.0], true, (t * 16.0) as i32)
            }
            _ => {}
        }
        if let Some(b) = bobber {
            let bite = matches!(self.phase, Phase::Bite { .. });
            if bite {
                // Spray as it is pulled under.
                for i in 0..18 {
                    let a = i as f32 * 2.4;
                    let r = 16.0 + (i % 5) as f32 * 11.0 + (t * 60.0).min(20.0);
                    let px = b[0] + a.cos() * r;
                    let py = b[1] + a.sin() * r * 0.8 - (t * 40.0).min(16.0);
                    let s = [4.0, 6.0, 8.0][i % 3];
                    ui.fill(px, py, s, s, color::with_alpha(SPRAY, 0.9));
                }
            }
            // Pulled under, only the red cap shows.
            let rows = if bite { 4 } else { 8 };
            let px = 6.0 * size;
            let top = [
                b[0] - 4.0 * px,
                b[1] - 4.0 * px + if bite { 14.0 } else { 0.0 },
            ];
            for (y, row) in BOBBER.iter().enumerate().take(rows) {
                for (x, c) in row.bytes().enumerate() {
                    let ink = match c {
                        b'o' => color::hex(0x1c181e, 255),
                        b'R' => color::hex(0xce3028, 255),
                        b'h' => color::hex(0xff8c78, 255),
                        b'W' => color::hex(0xecece4, 255),
                        _ => continue,
                    };
                    ui.fill(top[0] + x as f32 * px, top[1] + y as f32 * px, px, px, ink);
                }
            }
        }
        // What to do, until the first catch; then only what came up.
        let hint = match (&self.said, self.phase) {
            (Some((text, _)), _) => Some(text.as_str()),
            _ if self.caught > 0 => None,
            (None, Phase::Idle) => Some("Hold a finger on the water to fish"),
            (None, Phase::Cast | Phase::Waiting { .. }) => Some("Keep holding... wait for a bite"),
            (None, Phase::Coming { .. }) => Some("Something is coming..."),
            (None, Phase::Bite { .. }) => Some("Now! Lift your finger!"),
        };
        if let Some(hint) = hint {
            pill(ui, w / 2.0, hint_y, hint, false);
        }
        if self.caught > 0 {
            let count = format!("Caught: {}", self.caught);
            let cw = ui.measure(Face::Semibold, 15.0, &count);
            pill(ui, w - 16.0 - cw / 2.0 - 16.0, 16.0, &count, true);
        }
    }
}

/// The fish centred on `at`: facing left when it swims left (`toward_left`), the tail on its
/// `beat`.
fn fish(ui: &mut Ui<'_>, at: [f32; 2], toward_left: bool, beat: i32) {
    let rows = if beat % 2 == 0 { &FISH } else { &FISH_SWISH };
    let (cols, n) = (rows[0].len(), rows.len());
    let left = at[0] - cols as f32 * FISH_CELL / 2.0;
    let top = at[1] - n as f32 * FISH_CELL / 2.0;
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            let ink = match c {
                b'd' => color::with_alpha(color::hex(0x0b2350, 255), 0.62),
                b'e' => color::with_alpha(color::hex(0xb8d8ff, 255), 0.9),
                _ => continue,
            };
            let x = if toward_left { x } else { cols - 1 - x };
            ui.fill(
                left + x as f32 * FISH_CELL,
                top + y as f32 * FISH_CELL,
                FISH_CELL,
                FISH_CELL,
                ink,
            );
        }
    }
}

/// A ripple, snapped to the water's cells: brighter while `fade` is high.
fn ring(ui: &mut Ui<'_>, at: [f32; 2], r: f32, fade: f32) {
    let n = (r * 0.6) as i32 + 8;
    let mut last = None;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        let cell = (
            ((at[0] + a.cos() * r) / CELL * 2.0).round(),
            ((at[1] + a.sin() * r * 0.8) / CELL * 2.0).round(),
        );
        if last == Some(cell) {
            continue;
        }
        last = Some(cell);
        let half = CELL / 2.0;
        ui.fill(
            cell.0 * half,
            cell.1 * half,
            half,
            half,
            color::with_alpha(RIPPLE, fade.clamp(0.0, 1.0)),
        );
    }
}

/// Text on a dark pill centred on `cx`, its top at `y`.
fn pill(ui: &mut Ui<'_>, cx: f32, y: f32, text: &str, bold: bool) {
    let face = if bold { Face::Semibold } else { Face::Regular };
    let tw = ui.measure(face, 15.0, text);
    ui.rounded(
        cx - tw / 2.0 - 16.0,
        y,
        tw + 32.0,
        34.0,
        17.0,
        color::with_alpha(color::BG, 0.82),
    );
    ui.text_centered(face, 15.0, cx, y + 8.0, color::TEXT, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fish_is_caught_only_while_it_bites() {
        let mut f = Fishing::default();
        let mut now = Instant::now();
        f.press(now, [200.0, 200.0]);
        f.release(now + Duration::from_millis(100));
        assert_eq!(
            (f.caught, f.phase),
            (0, Phase::Idle),
            "lifted at once: nothing"
        );
        f.press(now, [200.0, 200.0]);
        // Out, a fish comes, it bites: the moves arrive frame by frame.
        let mut bitten = false;
        for _ in 0..2000 {
            now += Duration::from_millis(16);
            f.update(now);
            if let Phase::Bite { .. } = f.phase {
                bitten = true;
                break;
            }
        }
        assert!(bitten, "a fish bites within half a minute");
        f.release(now);
        assert_eq!(f.caught, 1);
        assert!(
            f.said
                .as_ref()
                .is_some_and(|(s, _)| s.starts_with("You caught"))
        );
    }

    #[test]
    fn its_sounds_are_short_and_soft() {
        for sound in [
            Sound::Cast,
            Sound::Plop,
            Sound::Blip,
            Sound::Splash,
            Sound::Catch,
        ] {
            let samples = sound.samples();
            assert!(
                !samples.is_empty() && samples.len() < RATE as usize / 2,
                "{sound:?}"
            );
            assert!(samples.iter().all(|s| s.abs() <= 1.0), "{sound:?} clips");
        }
    }

    #[test]
    fn the_catches_add_up_to_a_hundred() {
        assert_eq!(CATCHES.iter().map(|c| c.1).sum::<u32>(), 100);
    }
}
