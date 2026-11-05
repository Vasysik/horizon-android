//! Hardware Microphone Capture & Speaker Playback for Simple Voice Chat (SVC).
//!
//! Uses `cpal` to capture microphone input (resampling to 48kHz mono 960-sample frames)
//! and mix decoded 3D spatial voice streams to the default output device.

use super::audio_pipeline::{FRAME_SIZE, SAMPLE_RATE};
#[cfg(feature = "audio")]
use super::mixer::OutputResampler;
use super::mixer::VoiceMixer;
#[cfg(feature = "audio")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct VoiceHardwareIo {
    #[cfg(feature = "audio")]
    _input_stream: Option<cpal::Stream>,
    #[cfg(feature = "audio")]
    _output_stream: Option<cpal::Stream>,
    mic_buffer: Arc<Mutex<VecDeque<f32>>>,
    mixer: Arc<Mutex<VoiceMixer>>,
    input_initialized: bool,
    output_initialized: bool,
    last_mic_frame_at: Option<Instant>,
    last_valid_mic_sample_at: Option<Instant>,
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
            mixer: Arc::new(Mutex::new(VoiceMixer::new())),
            input_initialized: false,
            output_initialized: false,
            last_mic_frame_at: None,
            last_valid_mic_sample_at: None,
        }
    }

    /// Drops the current microphone stream and resets capture state.
    pub fn drop_input_stream(&mut self) {
        #[cfg(feature = "audio")]
        {
            self._input_stream = None;
        }
        self.input_initialized = false;
        self.last_mic_frame_at = None;
        self.last_valid_mic_sample_at = None;
        if let Ok(mut g) = self.mic_buffer.lock() {
            g.clear();
        }
    }

    /// Stops microphone capture and releases audio focus.
    pub fn stop_mic_capture(&mut self) {
        self.drop_input_stream();
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

            // 1A. Prioritize 1-channel (mono) 48kHz or 44.1kHz config before falling back
            let supported = device
                .supported_input_configs()
                .ok()
                .and_then(|configs| {
                    let list: Vec<_> = configs.collect();
                    // Priority 1: Mono F32 with 48000 Hz
                    if let Some(c) = list.iter().find(|c| {
                        c.channels() == 1
                            && c.sample_format() == cpal::SampleFormat::F32
                            && (c.min_sample_rate().0..=c.max_sample_rate().0).contains(&48000)
                    }) {
                        return c.try_with_sample_rate(cpal::SampleRate(48000));
                    }
                    // Priority 2: Mono F32 with 44100 Hz
                    if let Some(c) = list.iter().find(|c| {
                        c.channels() == 1
                            && c.sample_format() == cpal::SampleFormat::F32
                            && (c.min_sample_rate().0..=c.max_sample_rate().0).contains(&44100)
                    }) {
                        return c.try_with_sample_rate(cpal::SampleRate(44100));
                    }
                    // Priority 3: Any Mono F32
                    if let Some(c) = list
                        .iter()
                        .find(|c| c.channels() == 1 && c.sample_format() == cpal::SampleFormat::F32)
                    {
                        let rate = c
                            .max_sample_rate()
                            .min(cpal::SampleRate(48000))
                            .max(c.min_sample_rate());
                        return c.try_with_sample_rate(rate);
                    }
                    // Priority 4: Any Mono (e.g. I16)
                    if let Some(c) = list.iter().find(|c| c.channels() == 1) {
                        let rate = if (c.min_sample_rate().0..=c.max_sample_rate().0)
                            .contains(&48000)
                        {
                            cpal::SampleRate(48000)
                        } else if (c.min_sample_rate().0..=c.max_sample_rate().0).contains(&44100) {
                            cpal::SampleRate(44100)
                        } else {
                            c.max_sample_rate()
                                .min(cpal::SampleRate(48000))
                                .max(c.min_sample_rate())
                        };
                        return c.try_with_sample_rate(rate);
                    }
                    None
                })
                .or_else(|| device.default_input_config().ok());

            let Some(supported) = supported else {
                log::warn!("SVC: Failed to query microphone input config");
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
                            // 1B. Avoid 50% cut & phase cancellation between ch0 (bottom mic) and ch1 (top mic)
                            let mono: Vec<f32> = data
                                .chunks(in_channels)
                                .map(|frame| {
                                    if in_channels == 1 {
                                        frame[0]
                                    } else {
                                        let ch0 = frame[0];
                                        let ch1 = frame.get(1).copied().unwrap_or(0.0);
                                        if ch0.abs() >= ch1.abs() * 0.7 {
                                            ch0
                                        } else {
                                            ch1
                                        }
                                    }
                                })
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
                            // 1B. Avoid 50% cut & phase cancellation between ch0 (bottom mic) and ch1 (top mic)
                            let mono: Vec<f32> = data
                                .chunks(in_channels)
                                .map(|frame| {
                                    if in_channels == 1 {
                                        frame[0] as f32 / 32768.0
                                    } else {
                                        let ch0 = frame[0] as f32 / 32768.0;
                                        let ch1 = frame
                                            .get(1)
                                            .map(|&s| s as f32 / 32768.0)
                                            .unwrap_or(0.0);
                                        if ch0.abs() >= ch1.abs() * 0.7 {
                                            ch0
                                        } else {
                                            ch1
                                        }
                                    }
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
                        self.last_mic_frame_at = Some(Instant::now());
                        self.last_valid_mic_sample_at = Some(Instant::now());
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
            let mixer = Arc::clone(&self.mixer);

            let err_fn = |err| log::warn!("SVC: Speaker output stream error: {err}");

            let stream_res = match supported.sample_format() {
                cpal::SampleFormat::F32 => {
                    let mut resampler = OutputResampler::new();
                    device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _| {
                            let Ok(mut guard) = mixer.lock() else {
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
                    let mut resampler = OutputResampler::new();
                    device.build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            let Ok(mut guard) = mixer.lock() else {
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

        let now = Instant::now();

        // 1C. Watchdog: check if microphone stream has stalled (>1.5s) or silenced by ScreenRecorder (>3.0s)
        #[cfg(feature = "audio")]
        if self._input_stream.is_some() {
            let stalled = self
                .last_mic_frame_at
                .is_some_and(|t| now.duration_since(t) > std::time::Duration::from_millis(1500));
            let silenced = self
                .last_valid_mic_sample_at
                .is_some_and(|t| now.duration_since(t) > std::time::Duration::from_secs(3));

            if stalled || silenced {
                log::warn!(
                    "SVC: Microphone stream stalled or silenced by OS/ScreenRecorder, re-initializing..."
                );
                self.drop_input_stream();
                self.ensure_input_started();
                return None;
            }
        }

        let mut guard = self.mic_buffer.lock().ok()?;
        if guard.len() < FRAME_SIZE {
            return None;
        }

        self.last_mic_frame_at = Some(now);

        let mut frame = [0.0f32; FRAME_SIZE];
        let mut max_abs = 0.0f32;
        for sample in &mut frame {
            let s = guard.pop_front().unwrap_or(0.0);
            *sample = s;
            max_abs = max_abs.max(s.abs());
        }

        if max_abs > 1e-5 {
            self.last_valid_mic_sample_at = Some(now);
        }

        Some(frame)
    }

    /// Shared mixer: the UDP thread pushes decoded voice into it, the audio callback plays it.
    pub fn mixer(&self) -> Arc<Mutex<VoiceMixer>> {
        Arc::clone(&self.mixer)
    }

    pub fn stop(&mut self) {
        #[cfg(feature = "audio")]
        {
            self._input_stream = None;
            self._output_stream = None;
        }
        self.input_initialized = false;
        self.output_initialized = false;
        self.last_mic_frame_at = None;
        self.last_valid_mic_sample_at = None;
        if let Ok(mut g) = self.mic_buffer.lock() {
            g.clear();
        }
        if let Ok(mut m) = self.mixer.lock() {
            m.clear();
        }
    }
}
