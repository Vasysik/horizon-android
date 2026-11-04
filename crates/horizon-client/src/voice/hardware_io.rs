//! Hardware Microphone Capture & Speaker Playback for Simple Voice Chat (SVC).
//!
//! Uses `cpal` to capture microphone input (resampling to 48kHz mono 960-sample frames)
//! and mix decoded 3D spatial voice streams to the default output device.

use super::audio_pipeline::{FRAME_SIZE, SAMPLE_RATE};
#[cfg(feature = "audio")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// ~60ms prebuffer at 48kHz (3 frames * 960 samples = 2880 stereo samples).
pub const PREBUFFER_SAMPLES: usize = FRAME_SIZE * 3;

/// Stateful speaker playback resampler (48000 Hz -> hardware `out_rate`) with 60ms prebuffering
/// and linear interpolation.
pub struct SpeakerResampler {
    prebuffering: bool,
    phase: f64,
    prev: (f32, f32),
    curr: (f32, f32),
}

impl Default for SpeakerResampler {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeakerResampler {
    pub fn new() -> Self {
        Self {
            prebuffering: true,
            phase: 0.0,
            prev: (0.0, 0.0),
            curr: (0.0, 0.0),
        }
    }

    fn enter_prebuffering(&mut self) {
        self.prebuffering = true;
        self.phase = 0.0;
        self.prev = (0.0, 0.0);
        self.curr = (0.0, 0.0);
    }

    /// Fills `data` (interleaved `out_channels` @ `out_rate` Hz) from `queue` (48kHz stereo).
    pub fn fill_f32(
        &mut self,
        queue: &mut VecDeque<(f32, f32)>,
        data: &mut [f32],
        out_channels: usize,
        out_rate: usize,
    ) {
        let out_channels = out_channels.max(1);
        if self.prebuffering {
            if queue.len() < PREBUFFER_SAMPLES {
                data.fill(0.0);
                return;
            }
            self.prebuffering = false;
            self.phase = 0.0;
            self.prev = queue.pop_front().unwrap_or((0.0, 0.0));
            self.curr = queue.pop_front().unwrap_or(self.prev);
        }

        let step = (SAMPLE_RATE as f64) / (out_rate.max(1) as f64);

        for frame in data.chunks_mut(out_channels) {
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                self.prev = self.curr;
                if let Some(next) = queue.pop_front() {
                    self.curr = next;
                } else {
                    // Buffer underrun: stop continuous (0,0) stepping and re-prebuffer
                    self.enter_prebuffering();
                    frame.fill(0.0);
                    continue;
                }
            }

            if self.prebuffering {
                frame.fill(0.0);
                continue;
            }

            let frac = self.phase as f32;
            let left = self.prev.0 + (self.curr.0 - self.prev.0) * frac;
            let right = self.prev.1 + (self.curr.1 - self.prev.1) * frac;

            if out_channels == 1 {
                frame[0] = (left + right) * 0.5;
            } else {
                frame[0] = left;
                frame[1] = right;
                for extra in &mut frame[2..] {
                    *extra = 0.0;
                }
            }

            self.phase += step;
        }
    }

    /// Fills `data` (i16 interleaved `out_channels` @ `out_rate` Hz) from `queue` (48kHz stereo).
    pub fn fill_i16(
        &mut self,
        queue: &mut VecDeque<(f32, f32)>,
        data: &mut [i16],
        out_channels: usize,
        out_rate: usize,
    ) {
        let out_channels = out_channels.max(1);
        if self.prebuffering {
            if queue.len() < PREBUFFER_SAMPLES {
                data.fill(0);
                return;
            }
            self.prebuffering = false;
            self.phase = 0.0;
            self.prev = queue.pop_front().unwrap_or((0.0, 0.0));
            self.curr = queue.pop_front().unwrap_or(self.prev);
        }

        let step = (SAMPLE_RATE as f64) / (out_rate.max(1) as f64);

        for frame in data.chunks_mut(out_channels) {
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                self.prev = self.curr;
                if let Some(next) = queue.pop_front() {
                    self.curr = next;
                } else {
                    self.enter_prebuffering();
                    frame.fill(0);
                    continue;
                }
            }

            if self.prebuffering {
                frame.fill(0);
                continue;
            }

            let frac = self.phase as f32;
            let left = (self.prev.0 + (self.curr.0 - self.prev.0) * frac).clamp(-1.0, 1.0);
            let right = (self.prev.1 + (self.curr.1 - self.prev.1) * frac).clamp(-1.0, 1.0);

            if out_channels == 1 {
                let mono = ((left + right) * 0.5).clamp(-1.0, 1.0);
                frame[0] = (mono * 32767.0) as i16;
            } else {
                frame[0] = (left * 32767.0) as i16;
                frame[1] = (right * 32767.0) as i16;
                for extra in &mut frame[2..] {
                    *extra = 0;
                }
            }

            self.phase += step;
        }
    }
}

pub struct VoiceHardwareIo {
    #[cfg(feature = "audio")]
    _input_stream: Option<cpal::Stream>,
    #[cfg(feature = "audio")]
    _output_stream: Option<cpal::Stream>,
    mic_buffer: Arc<Mutex<VecDeque<f32>>>,
    speaker_buffer: Arc<Mutex<VecDeque<(f32, f32)>>>,
    input_initialized: bool,
    output_initialized: bool,
}

impl Default for VoiceHardwareIo {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceHardwareIo {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "audio")]
            _input_stream: None,
            #[cfg(feature = "audio")]
            _output_stream: None,
            mic_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(FRAME_SIZE * 10))),
            speaker_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(FRAME_SIZE * 20))),
            input_initialized: false,
            output_initialized: false,
        }
    }

    /// Lazily starts microphone capture stream when the player unmutes or holds PTT.
    pub fn ensure_input_started(&mut self) {
        if self.input_initialized {
            return;
        }
        self.input_initialized = true;

        #[cfg(feature = "audio")]
        {
            let host = cpal::default_host();
            let Some(device) = host.default_input_device() else {
                log::warn!("SVC: No default microphone input device found");
                return;
            };
            let Ok(supported) = device.default_input_config() else {
                log::warn!("SVC: Failed to query default microphone input config");
                return;
            };

            let config = supported.config();
            let in_rate = config.sample_rate.0 as usize;
            let in_channels = (config.channels as usize).max(1);
            let mic_buf = Arc::clone(&self.mic_buffer);

            let push_samples = move |mono_samples: &[f32]| {
                let Ok(mut guard) = mic_buf.lock() else {
                    return;
                };
                // Prevent unbounded lag (keep max ~200ms)
                if guard.len() > FRAME_SIZE * 10 {
                    let drop_count = guard.len() - FRAME_SIZE * 4;
                    guard.drain(..drop_count);
                }
                if in_rate == SAMPLE_RATE {
                    guard.extend(mono_samples.iter().copied());
                } else if in_rate > 0 && !mono_samples.is_empty() {
                    // Linear interpolation resample from hardware sample rate to 48000 Hz
                    let ratio = in_rate as f64 / SAMPLE_RATE as f64;
                    let out_len = ((mono_samples.len() as f64) / ratio) as usize;
                    for i in 0..out_len {
                        let src_pos = i as f64 * ratio;
                        let src_idx = src_pos as usize;
                        let frac = (src_pos - src_idx as f64) as f32;
                        if src_idx + 1 < mono_samples.len() {
                            let s0 = mono_samples[src_idx];
                            let s1 = mono_samples[src_idx + 1];
                            guard.push_back(s0 + (s1 - s0) * frac);
                        } else if src_idx < mono_samples.len() {
                            guard.push_back(mono_samples[src_idx]);
                        }
                    }
                }
            };

            let err_fn = |err| log::warn!("SVC: Microphone input stream error: {err}");

            let stream_res = match supported.sample_format() {
                cpal::SampleFormat::F32 => {
                    let cb = push_samples.clone();
                    device.build_input_stream(
                        &config,
                        move |data: &[f32], _| {
                            let mono: Vec<f32> = data
                                .chunks(in_channels)
                                .map(|ch| ch.iter().copied().sum::<f32>() / (ch.len() as f32))
                                .collect();
                            cb(&mono);
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let cb = push_samples.clone();
                    device.build_input_stream(
                        &config,
                        move |data: &[i16], _| {
                            let mono: Vec<f32> = data
                                .chunks(in_channels)
                                .map(|ch| {
                                    ch.iter().map(|&s| s as f32 / 32768.0).sum::<f32>()
                                        / (ch.len() as f32)
                                })
                                .collect();
                            cb(&mono);
                        },
                        err_fn,
                        None,
                    )
                }
                _ => {
                    log::warn!("SVC: Unsupported microphone sample format");
                    return;
                }
            };

            match stream_res {
                Ok(stream) => {
                    if let Err(e) = stream.play() {
                        log::warn!("SVC: Failed to start microphone stream: {e}");
                    } else {
                        log::info!(
                            "SVC: Microphone capture active ({} Hz, {} ch)",
                            in_rate,
                            in_channels
                        );
                        self._input_stream = Some(stream);
                    }
                }
                Err(e) => log::warn!("SVC: Failed to build microphone stream: {e}"),
            }
        }
    }

    /// Lazily starts speaker output stream when incoming voice packets arrive.
    pub fn ensure_output_started(&mut self) {
        if self.output_initialized {
            return;
        }
        self.output_initialized = true;

        #[cfg(feature = "audio")]
        {
            let host = cpal::default_host();
            let Some(device) = host.default_output_device() else {
                log::warn!("SVC: No default speaker output device found");
                return;
            };
            let Ok(supported) = device.default_output_config() else {
                log::warn!("SVC: Failed to query default speaker output config");
                return;
            };

            let config = supported.config();
            let out_rate = config.sample_rate.0 as usize;
            let out_channels = (config.channels as usize).max(1);
            let spk_buf = Arc::clone(&self.speaker_buffer);

            let err_fn = |err| log::warn!("SVC: Speaker output stream error: {err}");

            let stream_res = match supported.sample_format() {
                cpal::SampleFormat::F32 => {
                    let mut resampler = SpeakerResampler::new();
                    device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _| {
                            let Ok(mut guard) = spk_buf.lock() else {
                                data.fill(0.0);
                                return;
                            };
                            resampler.fill_f32(&mut guard, data, out_channels, out_rate);
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let mut resampler = SpeakerResampler::new();
                    device.build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            let Ok(mut guard) = spk_buf.lock() else {
                                data.fill(0);
                                return;
                            };
                            resampler.fill_i16(&mut guard, data, out_channels, out_rate);
                        },
                        err_fn,
                        None,
                    )
                }
                _ => return,
            };

            if let Ok(stream) = stream_res {
                if stream.play().is_ok() {
                    log::info!(
                        "SVC: Speaker output active ({} Hz, {} ch)",
                        out_rate,
                        out_channels
                    );
                    self._output_stream = Some(stream);
                }
            }
        }
    }

    /// Pops one 960-sample (20ms @ 48kHz) frame from the microphone ring buffer if available.
    pub fn poll_mic_frame(&mut self) -> Option<[f32; FRAME_SIZE]> {
        self.ensure_input_started();
        let mut guard = self.mic_buffer.lock().ok()?;
        if guard.len() < FRAME_SIZE {
            return None;
        }
        let mut frame = [0.0f32; FRAME_SIZE];
        for sample in &mut frame {
            *sample = guard.pop_front().unwrap_or(0.0);
        }
        Some(frame)
    }

    /// Queues decoded 48kHz mono samples with stereo spatial gains into the speaker buffer.
    pub fn push_spatial_audio(&mut self, mono_pcm: &[f32], left_gain: f32, right_gain: f32) {
        if left_gain <= 0.0001 && right_gain <= 0.0001 {
            return;
        }
        self.ensure_output_started();
        let Ok(mut guard) = self.speaker_buffer.lock() else {
            return;
        };
        // Cap speaker queue at ~300ms (15 frames) to keep latency low
        if guard.len() > FRAME_SIZE * 15 {
            let drop_count = guard.len() - FRAME_SIZE * 5;
            guard.drain(..drop_count);
        }
        for &s in mono_pcm {
            guard.push_back((
                (s * left_gain).clamp(-1.0, 1.0),
                (s * right_gain).clamp(-1.0, 1.0),
            ));
        }
    }

    pub fn stop(&mut self) {
        #[cfg(feature = "audio")]
        {
            self._input_stream = None;
            self._output_stream = None;
        }
        self.input_initialized = false;
        self.output_initialized = false;
        if let Ok(mut g) = self.mic_buffer.lock() {
            g.clear();
        }
        if let Ok(mut g) = self.speaker_buffer.lock() {
            g.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prebuffer_and_underrun_recovery() {
        let mut resampler = SpeakerResampler::new();
        let mut queue = VecDeque::new();
        let mut out = vec![1.0f32; 480 * 2]; // 10ms stereo buffer

        // Push only 2 frames (40ms < 60ms prebuffer): output must remain silent and queue untouched
        for _ in 0..(FRAME_SIZE * 2) {
            queue.push_back((0.5, -0.5));
        }
        resampler.fill_f32(&mut queue, &mut out, 2, 48000);
        assert!(out.iter().all(|&s| s == 0.0));
        assert_eq!(queue.len(), FRAME_SIZE * 2);

        // Push 3rd frame to reach 60ms (2880 samples): playback starts
        for _ in 0..FRAME_SIZE {
            queue.push_back((0.5, -0.5));
        }
        resampler.fill_f32(&mut queue, &mut out, 2, 48000);
        assert!(out.iter().any(|&s| s != 0.0));

        // Drain remaining samples until empty, verify resampler re-enters prebuffering
        let mut drain_buf = vec![0.0f32; PREBUFFER_SAMPLES * 4];
        resampler.fill_f32(&mut queue, &mut drain_buf, 2, 48000);
        assert!(queue.is_empty());
        assert!(resampler.prebuffering);
    }

    #[test]
    fn test_linear_interpolation_resampling() {
        let mut resampler = SpeakerResampler::new();
        let mut queue = VecDeque::new();
        for i in 0..PREBUFFER_SAMPLES {
            let v = (i as f32) / (PREBUFFER_SAMPLES as f32);
            queue.push_back((v, v));
        }

        // Resample 48000 -> 96000 Hz (step = 0.5): midpoint must be linearly interpolated
        let mut out = vec![0.0f32; 8]; // 4 stereo frames
        resampler.fill_f32(&mut queue, &mut out, 2, 96000);
        let s0 = 0.0f32;
        let s1 = 1.0f32 / (PREBUFFER_SAMPLES as f32);
        let mid = (s0 + s1) * 0.5;
        assert!((out[0] - s0).abs() < 1e-6);
        assert!((out[2] - mid).abs() < 1e-6);
        assert!((out[4] - s1).abs() < 1e-6);
    }
}
