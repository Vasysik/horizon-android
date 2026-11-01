// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// Horizon's official Zoom, formerly the WebAssembly mod and before that a
// built-in extra: hold the key (Z by default) to zoom in, and turn the mouse
// wheel while holding it to zoom further in or out. Its settings are the
// WebAssembly mod's files (`horizon-mods/.horizon-mod-data/zoom/level`,
// `smooth`, `sensitivity`) and its key `key.zoom.zoom` in `horizon-mods.txt`,
// so a player keeps them. This file is both the mod's Cargo library (its
// tests) and the module the bundle adds to the game.
use std::cell::RefCell;

/// The mod's id: its settings folder and its key's line.
pub const ID: &str = "zoom";
pub const VERSION: &str = "2.0.0";
pub const DESCRIPTION: &str =
    "Hold a key to zoom in; the mouse wheel changes the zoom while you hold it. Horizon's own.";
/// The key's name, and its default.
pub const KEY: &str = "zoom";
pub const DEFAULT_KEY: &str = "key.keyboard.z";
/// The zooms the starting zoom cycles through, and the wheel's limits.
pub const LEVELS: [u8; 6] = [2, 3, 4, 6, 8, 10];
const MIN: f32 = 1.25;
const MAX: f32 = 10.0;
/// Each wheel step multiplies the zoom by this.
const STEP: f32 = 1.25;

/// The settings, as the WebAssembly Zoom kept them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// The zoom the key starts at, one of `LEVELS`.
    pub level: u8,
    /// Ease in and out like the spyglass.
    pub smooth: bool,
    /// Turn slower the more you zoom in.
    pub sensitivity: bool,
}

impl Settings {
    /// From the stored values (`get` reads one key's file); the extra's
    /// defaults (4x, smooth, slower turning) for the ones missing.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let level = get("level")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .map_or(4, |v| v.clamp(2, MAX as u8));
        let flag = |key: &str| get(key).is_none_or(|v| v.trim() != "false");
        Self {
            level,
            smooth: flag("smooth"),
            sensitivity: flag("sensitivity"),
        }
    }
    /// The settings page's buttons, as the WebAssembly Zoom labelled them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Starting zoom: {}x", self.level),
            format!("Smooth zoom: {}", on(self.smooth)),
            format!("Slower turning: {}", on(self.sensitivity)),
        ]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => {
                // As the extra cycled: an unknown level counts as the first.
                let at = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(at + 1) % LEVELS.len()];
            }
            1 => self.smooth = !self.smooth,
            2 => self.sensitivity = !self.sensitivity,
            _ => {}
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 3] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("level", self.level.to_string()),
            ("smooth", flag(self.smooth)),
            ("sensitivity", flag(self.sensitivity)),
        ]
    }
}

#[derive(Default)]
struct State {
    /// Read on the first key press, then kept.
    settings: Option<Settings>,
    /// The zoom while the key is held; None when it is up.
    held: Option<f32>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

/// Replaces the settings (the settings page changed them).
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

/// The zoom key went down (only while playing) or up.
pub fn key(playing: bool, pressed: bool, get: impl Fn(&str) -> Option<String>) {
    if pressed && !playing {
        return;
    }
    let level = settings(get).level;
    STATE.with_borrow_mut(|s| {
        s.held = pressed.then(|| f32::from(level).clamp(MIN, MAX));
    });
}

/// Wheel steps: true when the key is held and they zoom.
pub fn wheel(steps: f32) -> bool {
    STATE.with_borrow_mut(|s| {
        let Some(z) = &mut s.held else {
            return false;
        };
        *z = (*z * STEP.powf(steps)).clamp(MIN, MAX);
        true
    })
}

/// The zoom in effect: (factor, smooth, turning scale), none when the key
/// is up.
pub fn view() -> Option<(f32, bool, f32)> {
    STATE.with_borrow(|s| {
        let factor = s.held?;
        let settings = s.settings?;
        let turn = if settings.sensitivity {
            1.0 / factor
        } else {
            1.0
        };
        Some((factor, settings.smooth, turn))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    #[test]
    fn reads_the_webassembly_mods_settings() {
        assert_eq!(
            Settings::read(stored(&[])),
            Settings {
                level: 4,
                smooth: true,
                sensitivity: true
            }
        );
        let s = Settings::read(stored(&[
            ("level", "8\n"),
            ("smooth", "false"),
            ("sensitivity", "true"),
        ]));
        assert_eq!(
            s,
            Settings {
                level: 8,
                smooth: false,
                sensitivity: true
            }
        );
        assert_eq!(Settings::read(stored(&[("level", "99")])).level, 10);
        let round: Vec<(&str, String)> = s.stored().into();
        let back: Vec<(&str, &str)> = round.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&back)), s);
    }

    #[test]
    fn buttons_cycle_as_the_extra_did() {
        let mut s = Settings {
            level: 10,
            smooth: true,
            sensitivity: false,
        };
        s.click(0);
        assert_eq!(s.level, 2);
        s.level = 5;
        s.click(0);
        assert_eq!(s.level, 3);
        s.click(1);
        s.click(2);
        assert_eq!(
            s.labels(),
            [
                "Starting zoom: 3x",
                "Smooth zoom: OFF",
                "Slower turning: ON"
            ]
        );
    }

    #[test]
    fn holds_zooms_with_the_wheel_and_lets_go() {
        let none = stored(&[]);
        key(false, true, &none);
        assert_eq!(view(), None, "not while a screen is open");
        assert!(!wheel(1.0));
        key(true, true, &none);
        assert_eq!(view(), Some((4.0, true, 0.25)));
        assert!(wheel(1.0));
        assert_eq!(view().map(|v| v.0), Some(5.0));
        assert!(wheel(40.0));
        assert_eq!(view().map(|v| v.0), Some(10.0));
        key(false, false, &none);
        assert_eq!(view(), None);
        assert!(!wheel(1.0));
    }
}

/// Reset profile-scoped state when Android returns from its launcher.
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
