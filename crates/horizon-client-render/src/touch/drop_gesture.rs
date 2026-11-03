//! Touch timing only; inventory mutation remains the game's existing Drop operation.
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Wait,
    Tap,
    Drag,
    Drop,
    Cancel,
}
pub struct DropGesture {
    since: Instant,
    next: Instant,
    origin: [f32; 2],
    repeating: bool,
    finished: bool,
}
pub const SINGLE: Duration = Duration::from_millis(300);
pub const REPEAT: Duration = Duration::from_millis(750);
pub const INTERVAL: Duration = Duration::from_millis(125);
impl DropGesture {
    pub fn new(now: Instant, origin: [f32; 2]) -> Self {
        Self {
            since: now,
            next: now + REPEAT,
            origin,
            repeating: false,
            finished: false,
        }
    }
    pub fn progress(&self, now: Instant) -> [f32; 2] {
        let elapsed = now.duration_since(self.since).as_secs_f32();
        [
            (elapsed / SINGLE.as_secs_f32()).min(1.0),
            (elapsed / REPEAT.as_secs_f32()).min(1.0),
        ]
    }
    pub fn update(
        &mut self,
        now: Instant,
        point: [f32; 2],
        ended: bool,
        cancelled: bool,
    ) -> Outcome {
        if self.finished {
            return Outcome::Cancel;
        }
        if cancelled {
            self.finished = true;
            return Outcome::Cancel;
        }
        if (point[0] - self.origin[0]).hypot(point[1] - self.origin[1]) > 10.0 {
            self.finished = true;
            return if self.repeating {
                Outcome::Cancel
            } else {
                Outcome::Drag
            };
        }
        if ended {
            self.finished = true;
            return if self.repeating {
                Outcome::Cancel
            } else if now.duration_since(self.since) >= SINGLE {
                Outcome::Drop
            } else {
                Outcome::Tap
            };
        }
        if now >= self.next {
            self.repeating = true;
            // Never catch up missed frames by dropping a burst of items.
            self.next = now + INTERVAL;
            return Outcome::Drop;
        }
        Outcome::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_drops_once_and_never_duplicates_a_repeat() {
        let now = Instant::now();
        for duration in [300, 749, 750, 2000] {
            let mut g = DropGesture::new(now, [0.0; 2]);
            assert_eq!(
                g.update(now + Duration::from_millis(duration), [0.0; 2], true, false),
                Outcome::Drop
            );
            assert_eq!(
                g.update(now + Duration::from_secs(5), [0.0; 2], true, false),
                Outcome::Cancel
            );
        }
        let mut g = DropGesture::new(now, [0.0; 2]);
        assert_eq!(
            g.update(now + REPEAT, [0.0; 2], false, false),
            Outcome::Drop
        );
        assert_eq!(
            g.update(now + REPEAT + INTERVAL / 2, [0.0; 2], false, false),
            Outcome::Wait
        );
        assert_eq!(
            g.update(now + REPEAT + INTERVAL, [0.0; 2], false, false),
            Outcome::Drop
        );
        assert_eq!(
            g.update(now + REPEAT + INTERVAL, [0.0; 2], true, false),
            Outcome::Cancel
        );
    }
    #[test]
    fn drag_cancel_and_tap_do_not_drop() {
        let now = Instant::now();
        for (point, end, cancel, expected) in [
            ([0.0; 2], true, false, Outcome::Tap),
            ([12.0, 0.0], false, false, Outcome::Drag),
            ([0.0; 2], false, true, Outcome::Cancel),
        ] {
            let mut g = DropGesture::new(now, [0.0; 2]);
            assert_eq!(
                g.update(now + Duration::from_millis(50), point, end, cancel),
                expected
            );
            assert_ne!(g.update(now + REPEAT, point, false, false), Outcome::Drop);
        }
    }
}
