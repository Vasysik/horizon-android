//! Per-sender jitter buffers and a real-time mixer for Simple Voice Chat playback.
//!
//! Data flow:
//! - UDP worker thread: decodes packets and calls [`VoiceMixer::push`] (48 kHz mono PCM per sender).
//! - Game tick: calls [`VoiceMixer::sources`] / [`VoiceMixer::apply_gains`] to refresh 3D gains.
//! - Audio callback: pulls [`VoiceMixer::next_frame`] through an [`OutputResampler`]; every active
//!   sender is SUMMED (never appended one after another), gains are smoothed per sample and an
//!   underrun fades out instead of cutting to silence.

use super::audio_pipeline::{FRAME_SIZE, SAMPLE_RATE};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Playback of a sender starts once this much audio is buffered (4 frames = 80 ms).
pub const PREBUFFER_SAMPLES: usize = FRAME_SIZE * 4;
/// Hard cap per sender (20 frames = 400 ms). Overflow drops ONE frame per push.
pub const MAX_BUFFER_SAMPLES: usize = FRAME_SIZE * 20;
/// Fade-in / fade-out length (5 ms @ 48 kHz).
pub const FADE_SAMPLES: usize = 240;
/// One-pole gain smoothing coefficient per sample (about 10 ms time constant).
const GAIN_SMOOTHING: f32 = 0.002;
/// Gain used until the game tick has computed a real 3D gain.
const DEFAULT_GAIN: f32 = 0.8;

/// Where the 3D gain of a sender comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GainSource {
    Player { max_dist: f32 },
    Group,
    Location { pos: [f64; 3], max_dist: f32 },
}

/// Counters for diagnostics (reset every time they are taken).
#[derive(Debug, Clone, Copy, Default)]
pub struct MixerStats {
    pub pushes: u64,
    pub underruns: u64,
    pub overflow_drops: u64,
    pub max_queue_samples: usize,
}

struct SenderStream {
    buf: VecDeque<f32>,
    playing: bool,
    ending: bool,
    fade_pos: usize,
    tail: usize,
    last: f32,
    gain_l: f32,
    gain_r: f32,
    target_l: f32,
    target_r: f32,
    culled: bool,
    source: GainSource,
    last_push: Instant,
}

impl SenderStream {
    fn new(source: GainSource) -> Self {
        Self {
            buf: VecDeque::with_capacity(FRAME_SIZE * 8),
            playing: false,
            ending: false,
            fade_pos: 0,
            tail: 0,
            last: 0.0,
            gain_l: DEFAULT_GAIN,
            gain_r: DEFAULT_GAIN,
            target_l: DEFAULT_GAIN,
            target_r: DEFAULT_GAIN,
            culled: false,
            source,
            last_push: Instant::now(),
        }
    }

    /// Linear decay of the last played sample after an underrun or the end of speech.
    fn decay_tail(&mut self) -> f32 {
        if self.tail == 0 {
            self.last = 0.0;
            return 0.0;
        }
        let v = self.last * (self.tail as f32 / FADE_SAMPLES as f32);
        self.tail -= 1;
        v
    }

    fn next_mono(&mut self, underruns: &mut u64) -> f32 {
        if !self.playing {
            let ready =
                self.buf.len() >= PREBUFFER_SAMPLES || (self.ending && !self.buf.is_empty());
            if !ready {
                return self.decay_tail();
            }
            self.playing = true;
            self.fade_pos = 0;
        }
        match self.buf.pop_front() {
            Some(s) => {
                let g = if self.fade_pos < FADE_SAMPLES {
                    let f = self.fade_pos as f32 / FADE_SAMPLES as f32;
                    self.fade_pos += 1;
                    f
                } else {
                    1.0
                };
                let v = s * g;
                self.last = v;
                v
            }
            None => {
                self.playing = false;
                if !self.ending {
                    *underruns += 1;
                }
                self.tail = FADE_SAMPLES;
                self.decay_tail()
            }
        }
    }

    fn next_frame(&mut self, underruns: &mut u64) -> (f32, f32) {
        let m = self.next_mono(underruns);
        self.gain_l += (self.target_l - self.gain_l) * GAIN_SMOOTHING;
        self.gain_r += (self.target_r - self.gain_r) * GAIN_SMOOTHING;
        (m * self.gain_l, m * self.gain_r)
    }
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    if x.abs() <= 0.75 {
        x
    } else {
        x.signum() * (0.75 + 0.25 * ((x.abs() - 0.75) * 2.0).tanh())
    }
}

/// Sums every sender into one 48 kHz stereo stream.
pub struct VoiceMixer {
    streams: HashMap<u128, SenderStream>,
    stats: MixerStats,
    last_stats: Instant,
}

impl Default for VoiceMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceMixer {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            stats: MixerStats::default(),
            last_stats: Instant::now(),
        }
    }

    /// Queues decoded 48 kHz mono samples of `uuid` (called from the UDP thread).
    pub fn push(&mut self, uuid: u128, source: GainSource, pcm: &[f32]) {
        if pcm.is_empty() {
            return;
        }
        let stream = self
            .streams
            .entry(uuid)
            .or_insert_with(|| SenderStream::new(source));
        stream.source = source;
        stream.ending = false;
        stream.last_push = Instant::now();
        if stream.culled {
            return;
        }
        stream.buf.extend(pcm.iter().copied());
        if stream.buf.len() > MAX_BUFFER_SAMPLES {
            let drop_count = FRAME_SIZE.min(stream.buf.len());
            stream.buf.drain(..drop_count);
            self.stats.overflow_drops += 1;
        }
        self.stats.pushes += 1;
        let queued = stream.buf.len();
        if queued > self.stats.max_queue_samples {
            self.stats.max_queue_samples = queued;
        }
    }

    /// Checks if incoming audio packets from `sender_uuid` should be decoded.
    /// Returns false if the sender is currently culled (muted, out of range, or beyond top-8).
    pub fn should_decode_sender(&self, sender_uuid: u128) -> bool {
        match self.streams.get(&sender_uuid) {
            Some(stream) => !stream.culled,
            None => true,
        }
    }

    /// Updates active timestamp and source for a culled sender without decoding or buffering audio.
    pub fn touch_culled_sender(&mut self, uuid: u128, source: GainSource) {
        let stream = self
            .streams
            .entry(uuid)
            .or_insert_with(|| SenderStream::new(source));
        stream.source = source;
        stream.ending = false;
        stream.last_push = Instant::now();
    }

    /// The sender stopped talking (empty Opus packet): play out what is buffered, then fade.
    pub fn end_stream(&mut self, uuid: u128) {
        if let Some(stream) = self.streams.get_mut(&uuid) {
            stream.ending = true;
        }
    }

    /// Snapshot of every sender and where its 3D gain comes from (for the game tick).
    pub fn sources(&self) -> Vec<(u128, GainSource)> {
        self.streams.iter().map(|(u, s)| (*u, s.source)).collect()
    }

    /// Sets the 3D gain targets and culling state computed by the game tick (smoothed per sample).
    pub fn apply_gains(&mut self, gains: &[(u128, (f32, f32), bool)]) {
        for (uuid, (left, right), culled) in gains {
            if let Some(stream) = self.streams.get_mut(uuid) {
                stream.target_l = *left;
                stream.target_r = *right;
                stream.culled = *culled;
                if *culled {
                    stream.buf.clear();
                    stream.playing = false;
                }
            }
        }
    }

    /// Removes silent senders that have not pushed audio for `max_idle`.
    pub fn prune_idle(&mut self, max_idle: Duration) {
        self.streams
            .retain(|_, s| s.playing || s.last_push.elapsed() <= max_idle);
    }

    /// Returns and resets the counters when `every` has elapsed and there was activity.
    pub fn take_stats_if_due(&mut self, every: Duration) -> Option<(MixerStats, usize)> {
        if self.last_stats.elapsed() < every {
            return None;
        }
        self.last_stats = Instant::now();
        let stats = std::mem::take(&mut self.stats);
        if stats.pushes == 0 && stats.underruns == 0 {
            return None;
        }
        Some((stats, self.streams.len()))
    }

    pub fn clear(&mut self) {
        self.streams.clear();
        self.stats = MixerStats::default();
    }

    /// Next 48 kHz stereo frame: the sum of all senders (called from the audio callback).
    pub fn next_frame(&mut self) -> (f32, f32) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;
        for stream in self.streams.values_mut() {
            let (l, r) = stream.next_frame(&mut self.stats.underruns);
            left += l;
            right += r;
        }
        (
            soft_limit(left).clamp(-1.0, 1.0),
            soft_limit(right).clamp(-1.0, 1.0),
        )
    }
}

/// Linear-interpolation resampler from the mixer's 48 kHz to the hardware rate.
#[derive(Default)]
pub struct OutputResampler {
    phase: f64,
    prev: (f32, f32),
    curr: (f32, f32),
    primed: bool,
}

impl OutputResampler {
    pub fn new() -> Self {
        Self::default()
    }

    fn next(&mut self, mixer: &mut VoiceMixer, step: f64) -> (f32, f32) {
        if !self.primed {
            self.prev = mixer.next_frame();
            self.curr = mixer.next_frame();
            self.phase = 0.0;
            self.primed = true;
        }
        while self.phase >= 1.0 {
            self.phase -= 1.0;
            self.prev = self.curr;
            self.curr = mixer.next_frame();
        }
        let frac = self.phase as f32;
        let left = self.prev.0 + (self.curr.0 - self.prev.0) * frac;
        let right = self.prev.1 + (self.curr.1 - self.prev.1) * frac;
        self.phase += step;
        (left, right)
    }

    /// Fills interleaved `data` (`out_channels` @ `out_rate` Hz) from the mixer.
    pub fn fill_f32(
        &mut self,
        mixer: &mut VoiceMixer,
        data: &mut [f32],
        out_channels: usize,
        out_rate: usize,
    ) {
        let out_channels = out_channels.max(1);
        let step = SAMPLE_RATE as f64 / out_rate.max(1) as f64;
        for frame in data.chunks_mut(out_channels) {
            let (left, right) = self.next(mixer, step);
            if out_channels == 1 || frame.len() < 2 {
                frame[0] = (left + right) * 0.5;
            } else {
                frame[0] = left;
                frame[1] = right;
                for extra in &mut frame[2..] {
                    *extra = 0.0;
                }
            }
        }
    }

    /// Same as [`Self::fill_f32`] for 16-bit output.
    pub fn fill_i16(
        &mut self,
        mixer: &mut VoiceMixer,
        data: &mut [i16],
        out_channels: usize,
        out_rate: usize,
    ) {
        let out_channels = out_channels.max(1);
        let step = SAMPLE_RATE as f64 / out_rate.max(1) as f64;
        for frame in data.chunks_mut(out_channels) {
            let (left, right) = self.next(mixer, step);
            if out_channels == 1 || frame.len() < 2 {
                let mono = ((left + right) * 0.5).clamp(-1.0, 1.0);
                frame[0] = (mono * 32767.0) as i16;
            } else {
                frame[0] = (left.clamp(-1.0, 1.0) * 32767.0) as i16;
                frame[1] = (right.clamp(-1.0, 1.0) * 32767.0) as i16;
                for extra in &mut frame[2..] {
                    *extra = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u128 = 1;
    const B: u128 = 2;

    fn constant(value: f32, frames: usize) -> Vec<f32> {
        vec![value; FRAME_SIZE * frames]
    }

    #[test]
    fn two_senders_are_summed_not_interleaved() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.2, 10));
        mixer.push(B, GainSource::Group, &constant(0.2, 10));
        let mut last = (0.0, 0.0);
        for _ in 0..(FADE_SAMPLES + 100) {
            last = mixer.next_frame();
        }
        // 0.2 * 0.8 (default gain) * 2 senders = 0.32
        assert!((last.0 - 0.32).abs() < 1e-4, "left was {}", last.0);
        assert!((last.1 - 0.32).abs() < 1e-4, "right was {}", last.1);
    }

    #[test]
    fn stays_silent_until_prebuffer_is_full() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.5, 2));
        for _ in 0..200 {
            assert_eq!(mixer.next_frame(), (0.0, 0.0));
        }
        mixer.push(A, GainSource::Group, &constant(0.5, 2));
        let mut heard = false;
        for _ in 0..(FADE_SAMPLES * 2) {
            let (l, _) = mixer.next_frame();
            if l > 0.1 {
                heard = true;
            }
        }
        assert!(heard);
    }

    #[test]
    fn underrun_fades_out_without_a_click() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.5, 4));
        let mut prev = 0.0f32;
        let mut max_step = 0.0f32;
        for _ in 0..(FRAME_SIZE * 4 + FADE_SAMPLES * 3) {
            let (l, _) = mixer.next_frame();
            max_step = max_step.max((l - prev).abs());
            prev = l;
        }
        assert!(max_step < 0.005, "max step {max_step}");
        assert_eq!(prev, 0.0);
        let (stats, streams) = mixer.take_stats_if_due(Duration::ZERO).unwrap();
        assert_eq!(stats.underruns, 1);
        assert_eq!(streams, 1);
    }

    #[test]
    fn end_of_speech_is_not_counted_as_underrun() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.5, 1));
        mixer.end_stream(A);
        let mut heard = false;
        for _ in 0..(FRAME_SIZE * 2) {
            if mixer.next_frame().0 > 0.1 {
                heard = true;
            }
        }
        assert!(heard);
        let (stats, _) = mixer.take_stats_if_due(Duration::ZERO).unwrap();
        assert_eq!(stats.underruns, 0);
    }

    #[test]
    fn overflow_drops_one_frame_per_push() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.1, 21));
        let (stats, _) = mixer.take_stats_if_due(Duration::ZERO).unwrap();
        assert_eq!(stats.overflow_drops, 1);
        assert_eq!(stats.max_queue_samples, FRAME_SIZE * 20);
    }

    #[test]
    fn gain_follows_target_smoothly() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.5, 50));
        mixer.apply_gains(&[(A, (0.0, 0.0), false)]);
        let mut prev = 0.0f32;
        let mut max_step = 0.0f32;
        let mut last = 1.0f32;
        for i in 0..8000 {
            let (l, _) = mixer.next_frame();
            if i > FADE_SAMPLES {
                max_step = max_step.max((l - prev).abs());
            }
            prev = l;
            last = l;
        }
        assert!(max_step < 0.002, "max step {max_step}");
        assert!(last.abs() < 0.001, "last {last}");
    }

    #[test]
    fn test_culling_clears_buffer_and_drops_samples() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.5, 4));
        assert!(mixer.should_decode_sender(A));

        // Cull sender A
        mixer.apply_gains(&[(A, (0.0, 0.0), true)]);
        assert!(!mixer.should_decode_sender(A));

        // Pushing samples to culled sender should not buffer audio
        mixer.push(A, GainSource::Group, &constant(0.5, 4));
        assert_eq!(mixer.next_frame(), (0.0, 0.0));

        // Touching culled sender keeps it active
        mixer.touch_culled_sender(A, GainSource::Group);
        assert!(!mixer.should_decode_sender(A));
    }

    #[test]
    fn soft_limit_is_identity_below_knee_and_bounded_above() {
        assert_eq!(soft_limit(0.5), 0.5);
        assert_eq!(soft_limit(-0.5), -0.5);
        assert!(soft_limit(1.5) > 0.75 && soft_limit(1.5) < 1.0);
        assert!(soft_limit(-1.5) < -0.75 && soft_limit(-1.5) > -1.0);
    }

    #[test]
    fn resampler_44100_stays_fed_and_audible() {
        let mut mixer = VoiceMixer::new();
        mixer.push(A, GainSource::Group, &constant(0.3, 40));
        let mut resampler = OutputResampler::new();
        // 0.5 s of stereo output at 44.1 kHz pulls about 25 frames of the 40 queued.
        let mut out = vec![0.0f32; 22050 * 2];
        resampler.fill_f32(&mut mixer, &mut out, 2, 44100);
        let (stats, _) = mixer.take_stats_if_due(Duration::ZERO).unwrap();
        assert_eq!(stats.underruns, 0);
        assert!(out.iter().any(|&s| s > 0.1));
    }
}
