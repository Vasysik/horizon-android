//! Button gesture ownership. Time is supplied by the caller for deterministic tests.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Binding {
    pub token: String,
    #[serde(default)]
    pub toggle: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Bindings {
    pub single: Vec<Binding>,
    pub double: Vec<Binding>,
    pub hold: Vec<Binding>,
    /// Immediate single is additive: double/hold never retract its action.
    /// Deferred single is suppressed by double or hold.
    pub immediate_single: bool,
    pub double_ms: u64,
    pub hold_ms: u64,
}
impl Default for Bindings {
    fn default() -> Self {
        Self {
            single: vec![],
            double: vec![],
            hold: vec![],
            immediate_single: true,
            double_ms: 300,
            hold_ms: 350,
        }
    }
}
impl Bindings {
    pub fn sanitize(&mut self, valid: impl Fn(&str) -> bool) {
        self.double_ms = self.double_ms.clamp(100, 700);
        self.hold_ms = self.hold_ms.clamp(150, 2000);
        for bindings in [&mut self.single, &mut self.double, &mut self.hold] {
            bindings.truncate(8);
            let mut seen = BTreeSet::new();
            bindings.retain(|b| {
                valid(&b.token) && b.token != "key.keyboard.unknown" && seen.insert(b.token.clone())
            });
        }
    }
}
#[derive(Clone)]
struct Press {
    index: usize,
    since: Instant,
    bindings: Bindings,
    held: bool,
    double: bool,
    origin: Option<[f32; 2]>,
    dragged: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Owner {
    Finger(u64, u8),
    Toggle(usize, u8),
    Pulse(u64),
}
#[derive(Default)]
pub(super) struct Runtime {
    presses: BTreeMap<u64, Press>,
    sources: BTreeMap<Owner, BTreeSet<String>>,
    pending: BTreeMap<usize, (Instant, Bindings)>,
    last_tap: BTreeMap<usize, Instant>,
    emitted: BTreeSet<String>,
    dispatched: BTreeSet<String>,
    events: Vec<(String, bool)>,
    pulse: u64,
    release_pulses: BTreeSet<u64>,
}
impl Runtime {
    fn sync(&mut self) {
        let next: BTreeSet<_> = self.sources.values().flatten().cloned().collect();
        self.events
            .extend(self.emitted.difference(&next).map(|k| (k.clone(), false)));
        self.events
            .extend(next.difference(&self.emitted).map(|k| (k.clone(), true)));
        self.emitted = next;
    }
    fn activate(&mut self, owner: Owner, index: usize, gesture: u8, bindings: &[Binding]) {
        let mut held = BTreeSet::new();
        let toggle_owner = Owner::Toggle(index, gesture);
        for b in bindings {
            if b.toggle {
                let tokens = self.sources.entry(toggle_owner).or_default();
                if !tokens.remove(&b.token) {
                    tokens.insert(b.token.clone());
                }
            } else {
                held.insert(b.token.clone());
            }
        }
        self.sources.insert(owner, held);
        self.sync();
    }
    fn pulse(&mut self, index: usize, bindings: &[Binding]) {
        self.pulse = self.pulse.wrapping_add(1);
        self.activate(Owner::Pulse(self.pulse), index, 0, bindings);
    }
    pub fn owns(&self, id: u64) -> bool {
        self.presses.contains_key(&id) || self.sources.contains_key(&Owner::Finger(id, 0))
    }
    pub fn tokens(&self) -> impl Iterator<Item = &str> {
        self.emitted.iter().map(String::as_str)
    }
    pub fn finger_has(&self, id: u64, token: &str) -> bool {
        self.sources.iter().any(|(owner, tokens)| {
            matches!(owner, Owner::Finger(f, _) if *f == id) && tokens.contains(token)
        })
    }
    pub fn direct(&mut self, id: u64, token: &str) {
        self.sources
            .insert(Owner::Finger(id, 0), [token.into()].into());
        self.sync();
    }
    pub fn start(&mut self, id: u64, index: usize, bindings: Bindings, now: Instant) {
        if self.owns(id) {
            return;
        }
        let double = !bindings.double.is_empty()
            && self.last_tap.remove(&index).is_some_and(|at| {
                now.duration_since(at) <= Duration::from_millis(bindings.double_ms)
            });
        if double {
            self.pending.remove(&index);
        }
        if bindings.immediate_single {
            self.activate(Owner::Finger(id, 0), index, 0, &bindings.single);
        }
        if double {
            self.activate(Owner::Finger(id, 1), index, 1, &bindings.double);
        }
        self.presses.insert(
            id,
            Press {
                index,
                since: now,
                bindings,
                held: false,
                double,
                origin: None,
                dragged: false,
            },
        );
    }
    pub fn end(&mut self, id: u64, now: Instant) {
        let matured = self.presses.get_mut(&id).and_then(|p| {
            if !p.held
                && !p.bindings.hold.is_empty()
                && now.duration_since(p.since) >= Duration::from_millis(p.bindings.hold_ms)
            {
                p.held = true;
                Some((p.index, p.bindings.hold.clone()))
            } else {
                None
            }
        });
        if let Some((index, bindings)) = matured {
            self.activate(Owner::Finger(id, 2), index, 2, &bindings);
        }
        if let Some(p) = self.presses.remove(&id)
            && !p.held
            && !p.double
            && !p.dragged
        {
            self.last_tap.insert(p.index, now);
            if !p.bindings.immediate_single {
                if p.bindings.double.is_empty() {
                    self.pulse(p.index, &p.bindings.single);
                } else {
                    self.pending.insert(p.index, (now, p.bindings));
                }
            }
        }
        self.sources
            .retain(|owner, _| !matches!(owner, Owner::Finger(f, _) if *f == id));
        self.sync();
    }
    pub fn poll(&mut self, now: Instant) -> Vec<(String, bool)> {
        self.sources.retain(
            |owner, _| !matches!(owner, Owner::Pulse(id) if self.release_pulses.contains(id)),
        );
        self.sync();
        let expired: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, (at, b))| now.duration_since(*at) >= Duration::from_millis(b.double_ms))
            .map(|(i, _)| *i)
            .collect();
        for index in expired {
            let (_, bindings) = self.pending.remove(&index).unwrap();
            self.pulse(index, &bindings.single);
            self.last_tap.remove(&index);
        }
        let held: Vec<_> = self
            .presses
            .iter_mut()
            .filter_map(|(id, p)| {
                if !p.held
                    && !p.bindings.hold.is_empty()
                    && now.duration_since(p.since) >= Duration::from_millis(p.bindings.hold_ms)
                {
                    p.held = true;
                    Some((*id, p.index, p.bindings.hold.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (id, index, bindings) in held {
            self.pending.remove(&index);
            self.last_tap.remove(&index);
            self.activate(Owner::Finger(id, 2), index, 2, &bindings);
        }
        self.release_pulses = self
            .sources
            .keys()
            .filter_map(|owner| match owner {
                Owner::Pulse(id) => Some(*id),
                _ => None,
            })
            .collect();
        self.dispatched = self.emitted.clone();
        std::mem::take(&mut self.events)
    }
    pub fn clear(&mut self) {
        self.events.clear();
        self.emitted = self.dispatched.clone();
        self.presses.clear();
        self.sources.clear();
        self.pending.clear();
        self.last_tap.clear();
        self.release_pulses.clear();
        self.sync();
    }
    pub fn clear_toggle_tokens(&mut self, matches: impl Fn(&str) -> bool) {
        for (owner, tokens) in &mut self.sources {
            if matches!(owner, Owner::Toggle(_, _)) {
                tokens.retain(|token| !matches(token));
            }
        }
        for (id, press) in &self.presses {
            for (gesture, bindings, active) in [
                (0, &press.bindings.single, press.bindings.immediate_single),
                (1, &press.bindings.double, press.double),
                (2, &press.bindings.hold, press.held),
            ] {
                if active {
                    self.sources
                        .entry(Owner::Finger(*id, gesture))
                        .or_default()
                        .extend(
                            bindings
                                .iter()
                                .filter(|b| matches(&b.token))
                                .map(|b| b.token.clone()),
                        );
                }
            }
        }
        self.sync();
    }
    pub fn motion(&mut self, id: u64, point: [f32; 2], aspect: f32) {
        if let Some(p) = self.presses.get_mut(&id) {
            let origin = *p.origin.get_or_insert(point);
            p.dragged |= ((point[0] - origin[0]) * aspect).hypot(point[1] - origin[1]) > 0.018;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bind(token: &str, toggle: bool) -> Binding {
        Binding {
            token: token.into(),
            toggle,
        }
    }
    fn config() -> Bindings {
        Bindings {
            single: vec![bind("shift", false), bind("attack", false)],
            ..Bindings::default()
        }
    }
    #[test]
    fn multiple_buttons_and_toggles_share_ownership() {
        let now = Instant::now();
        let mut r = Runtime::default();
        r.start(1, 0, config(), now);
        r.start(2, 1, config(), now);
        assert_eq!(
            r.poll(now),
            [("attack".into(), true), ("shift".into(), true)]
        );
        r.end(1, now);
        assert!(r.poll(now).is_empty());
        let b = Bindings {
            single: vec![bind("shift", true)],
            ..Bindings::default()
        };
        r.start(3, 2, b.clone(), now);
        r.end(3, now);
        r.end(2, now);
        assert_eq!(r.poll(now), [("attack".into(), false)]);
        r.start(4, 2, b, now);
        r.end(4, now);
        assert_eq!(r.poll(now), [("shift".into(), false)]);
    }
    #[test]
    fn descending_drops_latched_sneak_but_keeps_pressed_fingers() {
        let now = Instant::now();
        let mut r = Runtime::default();
        let b = Bindings {
            single: vec![bind("sneak", true)],
            ..Bindings::default()
        };
        r.start(1, 0, b.clone(), now);
        r.end(1, now);
        r.poll(now);
        r.clear_toggle_tokens(|token| token == "sneak");
        assert_eq!(r.poll(now), [("sneak".into(), false)]);
        r.start(2, 0, b, now);
        r.clear_toggle_tokens(|token| token == "sneak");
        assert_eq!(r.poll(now), [("sneak".into(), true)]);
        r.direct(3, "sneak");
        r.end(2, now);
        assert!(r.poll(now).is_empty());
        r.end(3, now);
        assert_eq!(r.poll(now), [("sneak".into(), false)]);
    }
    #[test]
    fn cancelled_unpolled_press_and_dragged_deferred_tap_do_not_fire() {
        let now = Instant::now();
        let mut r = Runtime::default();
        r.start(1, 0, config(), now);
        r.clear();
        assert!(r.poll(now).is_empty());
        let mut b = config();
        b.immediate_single = false;
        r.start(2, 0, b, now);
        r.motion(2, [0.5, 0.5], 2.0);
        r.motion(2, [0.6, 0.5], 2.0);
        r.end(2, now + Duration::from_millis(30));
        assert!(r.poll(now + Duration::from_secs(1)).is_empty());
    }
    #[test]
    fn hold_is_recognized_even_when_a_frame_was_missed() {
        let now = Instant::now();
        let mut r = Runtime::default();
        let b = Bindings {
            immediate_single: false,
            single: vec![bind("single", false)],
            hold: vec![bind("held", false)],
            ..Bindings::default()
        };
        r.start(1, 0, b, now);
        r.end(1, now + Duration::from_secs(1));
        assert_eq!(
            r.poll(now + Duration::from_secs(1)),
            [("held".into(), true), ("held".into(), false)]
        );
    }
    #[test]
    fn immediate_single_is_additive_and_deferred_single_is_exclusive() {
        let now = Instant::now();
        for immediate in [true, false] {
            let mut r = Runtime::default();
            let mut b = config();
            b.immediate_single = immediate;
            b.double = vec![bind("jump", false)];
            r.start(1, 0, b.clone(), now);
            assert_eq!(r.tokens().any(|k| k == "attack"), immediate);
            r.end(1, now + Duration::from_millis(20));
            r.start(2, 0, b, now + Duration::from_millis(100));
            let events = r.poll(now + Duration::from_millis(100));
            assert!(events.contains(&("jump".into(), true)));
            assert_eq!(events.contains(&("attack".into(), true)), immediate);
            r.end(2, now + Duration::from_millis(120));
            r.poll(now + Duration::from_secs(1));
            assert_eq!(r.tokens().count(), 0);
        }
    }
    #[test]
    fn deferred_single_hold_and_cancel_have_no_ghost_actions() {
        let now = Instant::now();
        let mut r = Runtime::default();
        let mut b = config();
        b.immediate_single = false;
        b.double = vec![bind("double", false)];
        b.hold = vec![bind("hold", false)];
        r.start(1, 0, b.clone(), now);
        r.end(1, now + Duration::from_millis(20));
        assert!(r.poll(now + Duration::from_millis(200)).is_empty());
        assert!(
            r.poll(now + Duration::from_millis(320))
                .contains(&("attack".into(), true))
        );
        assert!(
            r.poll(now + Duration::from_millis(321))
                .contains(&("attack".into(), false))
        );
        r.start(2, 0, b.clone(), now + Duration::from_secs(1));
        let held = r.poll(now + Duration::from_millis(1400));
        assert_eq!(held, [("hold".into(), true)]);
        r.end(2, now + Duration::from_millis(1500));
        assert_eq!(
            r.poll(now + Duration::from_secs(2)),
            [("hold".into(), false)]
        );
        r.start(3, 0, b, now + Duration::from_secs(3));
        r.clear();
        assert!(r.poll(now + Duration::from_secs(5)).is_empty());
        assert_eq!(r.tokens().count(), 0);
    }
}
