//! Simple Voice Chat (SVC) Audio Pipeline for Horizon.
//!
//! Handles 48kHz mono PCM frames (20ms = 960 samples), dual 480-sample sub-frame
//! RNNoise neural noise suppression, Voice Activity Detection (VAD),
//! stateful Opus encoding/decoding (VOIP mode), Packet Loss Concealment (PLC),
//! and 3D spatial gain (attenuation & stereo panning).

use audiopus::{
    Application, Bitrate, Channels, SampleRate,
    coder::{Decoder as OpusDecoder, Encoder as OpusEncoder, GenericCtl},
};
use nnnoiseless::DenoiseState;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: usize = 48000;
pub const FRAME_SIZE: usize = (SAMPLE_RATE / 1000) * 20; // 960 samples (20ms @ 48kHz)
pub const SUBFRAME_SIZE: usize = FRAME_SIZE / 2; // 480 samples
pub const MAX_OPUS_PACKET_SIZE: usize = 1275;
pub const MAX_PLC_FRAMES: i64 = 5; // Conceal up to 100ms (5 frames) of consecutive packet loss
pub const DECODER_IDLE_TIMEOUT: Duration = Duration::from_secs(10);

#[inline]
fn soft_clip_mic(x: f32) -> f32 {
    if x.abs() <= 0.70 {
        x
    } else {
        x.signum() * (0.70 + 0.30 * ((x.abs() - 0.70) * 2.5).tanh())
    }
}

/// Audio capture processing pipeline (Microphone -> RNNoise -> VAD -> Opus Encoder)
pub struct VoiceCapturePipeline {
    denoiser: Box<DenoiseState<'static>>,
    encoder: OpusEncoder,
    sequence_number: i64,
    last_vad_probability: f32,
    pub agc_gain: f32,
}

impl Default for VoiceCapturePipeline {
    fn default() -> Self {
        let mut encoder = OpusEncoder::new(SampleRate::Hz48000, Channels::Mono, Application::Voip)
            .expect("Failed to initialize 48kHz mono VOIP Opus encoder");
        let _ = encoder.set_bitrate(Bitrate::BitsPerSecond(32_000));
        Self {
            denoiser: DenoiseState::new(),
            encoder,
            sequence_number: 0,
            last_vad_probability: 0.0,
            agc_gain: 1.5,
        }
    }
}

impl VoiceCapturePipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes a single 960-sample mono f32 frame into Opus wire bytes.
    pub fn encode_frame(&mut self, pcm: &[f32; FRAME_SIZE]) -> Option<Vec<u8>> {
        let mut out = vec![0u8; MAX_OPUS_PACKET_SIZE];
        match self.encoder.encode_float(pcm, &mut out) {
            Ok(len) if len > 0 => {
                out.truncate(len);
                Some(out)
            }
            Ok(_) => None,
            Err(e) => {
                log::warn!("SVC: Opus encode error: {e}");
                None
            }
        }
    }

    /// Resets the internal Opus encoder state and AGC (e.g., when speech stops).
    pub fn reset_encoder(&mut self) {
        let _ = self.encoder.reset_state();
        self.agc_gain = 1.5;
    }

    /// Process a 960-sample PCM frame (-1.0 .. 1.0).
    ///
    /// Runs 2 sub-frames of 480 samples through RNNoise scaled to [-32768.0, 32767.0],
    /// extracts VAD probability, and applies Voice-Gated AGC only if speech is detected.
    pub fn process_frame(
        &mut self,
        pcm_in: &[f32; FRAME_SIZE],
        rnnoise_enabled: bool,
        mic_gain: f32,
        voice_activation: bool,
        vad_threshold: f32,
        ptt_pressed: bool,
    ) -> Option<Vec<u8>> {
        if !voice_activation && !ptt_pressed {
            return None;
        }

        let mut denoised_pcm = [0.0f32; FRAME_SIZE];

        let base_gain = (mic_gain.clamp(0.0, 4.0) * 1.8).clamp(0.0, 7.2);
        let vad_max = if rnnoise_enabled {
            let mut sub1_in = [0.0f32; SUBFRAME_SIZE];
            let mut sub1_out = [0.0f32; SUBFRAME_SIZE];
            for i in 0..SUBFRAME_SIZE {
                sub1_in[i] = (pcm_in[i] * base_gain * 32768.0).clamp(-32768.0, 32767.0);
            }
            let vad1 = self.denoiser.process_frame(&mut sub1_out, &sub1_in);

            let mut sub2_in = [0.0f32; SUBFRAME_SIZE];
            let mut sub2_out = [0.0f32; SUBFRAME_SIZE];
            for i in 0..SUBFRAME_SIZE {
                sub2_in[i] =
                    (pcm_in[SUBFRAME_SIZE + i] * base_gain * 32768.0).clamp(-32768.0, 32767.0);
            }
            let vad2 = self.denoiser.process_frame(&mut sub2_out, &sub2_in);

            for i in 0..SUBFRAME_SIZE {
                denoised_pcm[i] = (sub1_out[i] / 32768.0).clamp(-1.0, 1.0);
                denoised_pcm[SUBFRAME_SIZE + i] = (sub2_out[i] / 32768.0).clamp(-1.0, 1.0);
            }
            vad1.max(vad2)
        } else {
            for i in 0..FRAME_SIZE {
                denoised_pcm[i] = (pcm_in[i] * base_gain).clamp(-1.0, 1.0);
            }
            let sum_sq: f32 = denoised_pcm.iter().map(|s| s * s).sum();
            let rms = (sum_sq / (FRAME_SIZE as f32)).sqrt();
            (rms * 5.0).clamp(0.0, 1.0)
        };

        self.last_vad_probability = vad_max;

        let vad_passed = !voice_activation || vad_max >= vad_threshold;
        if !vad_passed {
            self.agc_gain = self.agc_gain * 0.95 + 1.5 * 0.05;
            return None;
        }

        // Step 2: Voice-Gated AGC — only runs on voice-confirmed frames
        let mut frame_peak = 0.0f32;
        for &s in denoised_pcm.iter() {
            frame_peak = frame_peak.max(s.abs());
        }

        if frame_peak >= 0.02 {
            let desired_agc = (0.65 / frame_peak).clamp(1.0, 4.0);
            if desired_agc < self.agc_gain {
                // Fast attack to prevent clipping when suddenly speaking loudly
                self.agc_gain = self.agc_gain * 0.50 + desired_agc * 0.50;
            } else {
                // Smooth release to elevate quieter speech
                self.agc_gain = self.agc_gain * 0.85 + desired_agc * 0.15;
            }
        }

        for sample in denoised_pcm.iter_mut() {
            *sample = soft_clip_mic(*sample * self.agc_gain).clamp(-1.0, 1.0);
        }

        let opus_data = self.encode_frame(&denoised_pcm)?;
        self.sequence_number += 1;
        Some(opus_data)
    }

    pub fn last_vad(&self) -> f32 {
        self.last_vad_probability
    }

    pub fn current_sequence(&self) -> i64 {
        self.sequence_number
    }
}

struct SenderDecoder {
    decoder: OpusDecoder,
    last_sequence: Option<i64>,
    last_active: Instant,
}

impl SenderDecoder {
    fn new() -> Option<Self> {
        let decoder = OpusDecoder::new(SampleRate::Hz48000, Channels::Mono).ok()?;
        Some(Self {
            decoder,
            last_sequence: None,
            last_active: Instant::now(),
        })
    }

    fn reset(&mut self) {
        let _ = self.decoder.reset_state();
        self.last_sequence = None;
        self.last_active = Instant::now();
    }

    fn decode_packet(&mut self, opus_data: &[u8], out: &mut [f32; FRAME_SIZE]) -> Option<usize> {
        self.decoder
            .decode_float(Some(opus_data), &mut out[..], false)
            .ok()
    }

    fn decode_plc(&mut self, out: &mut [f32; FRAME_SIZE]) -> Option<usize> {
        self.decoder
            .decode_float(None::<&[u8]>, &mut out[..], false)
            .ok()
    }
}

/// Decodes received Opus packets per sender UUID and calculates 3D spatial audio gains.
#[derive(Default)]
pub struct VoicePlaybackPipeline {
    decoders: HashMap<u128, SenderDecoder>,
}

impl VoicePlaybackPipeline {
    pub fn new() -> Self {
        Self {
            decoders: HashMap::new(),
        }
    }

    /// Decodes an incoming Opus packet for a specific `sender_uuid` and `sequence_number`.
    ///
    /// - If `opus_data` is empty: the sender stopped talking; resets that sender's decoder
    ///   and returns an empty `Vec<f32>` (no samples decoded).
    /// - If packets were lost (`sequence_number > last_sequence + 1`): runs Opus PLC
    ///   (decoding with `None`) for missing frames up to `MAX_PLC_FRAMES` before decoding
    ///   the current packet.
    /// - If a packet is late/duplicate (`sequence_number <= last_sequence`): drops it.
    pub fn decode_sender_frame(
        &mut self,
        sender_uuid: u128,
        sequence_number: i64,
        opus_data: &[u8],
    ) -> Vec<f32> {
        if opus_data.is_empty() {
            if let Some(sender) = self.decoders.get_mut(&sender_uuid) {
                sender.reset();
            }
            return Vec::new();
        }

        if !self.decoders.contains_key(&sender_uuid) {
            let Some(new_decoder) = SenderDecoder::new() else {
                log::warn!("SVC: Failed to create Opus decoder for sender {sender_uuid:032x}");
                return Vec::new();
            };
            self.decoders.insert(sender_uuid, new_decoder);
        }

        let Some(sender) = self.decoders.get_mut(&sender_uuid) else {
            return Vec::new();
        };
        sender.last_active = Instant::now();

        let mut output = Vec::with_capacity(FRAME_SIZE);

        if let Some(last_seq) = sender.last_sequence {
            let gap = sequence_number.wrapping_sub(last_seq);
            if gap <= 0 {
                // If sequence jumped far backwards (e.g. sender reconnected), reset decoder
                if gap < -100 {
                    sender.reset();
                } else {
                    // Late or duplicate packet: ignore to preserve decoder state
                    return Vec::new();
                }
            } else if gap > 1 {
                let missing = (gap - 1).min(MAX_PLC_FRAMES);
                let mut plc_buf = [0.0f32; FRAME_SIZE];
                for _ in 0..missing {
                    if let Some(decoded_len) = sender.decode_plc(&mut plc_buf) {
                        output.extend_from_slice(&plc_buf[..decoded_len.min(FRAME_SIZE)]);
                    }
                }
            }
        }

        let mut pcm_buf = [0.0f32; FRAME_SIZE];
        if let Some(decoded_len) = sender.decode_packet(opus_data, &mut pcm_buf) {
            sender.last_sequence = Some(sequence_number);
            output.extend_from_slice(&pcm_buf[..decoded_len.min(FRAME_SIZE)]);
        }

        output
    }

    /// Explicitly generates a single 960-sample PLC frame for `sender_uuid` when a packet is lost.
    pub fn conceal_packet_loss(&mut self, sender_uuid: u128) -> Vec<f32> {
        let Some(sender) = self.decoders.get_mut(&sender_uuid) else {
            return Vec::new();
        };
        sender.last_active = Instant::now();
        let mut plc_buf = [0.0f32; FRAME_SIZE];
        if let Some(decoded_len) = sender.decode_plc(&mut plc_buf) {
            if let Some(ref mut seq) = sender.last_sequence {
                *seq = seq.wrapping_add(1);
            }
            plc_buf[..decoded_len.min(FRAME_SIZE)].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Removes decoders for senders that have been idle longer than `DECODER_IDLE_TIMEOUT`.
    pub fn prune_idle_decoders(&mut self) {
        self.decoders
            .retain(|_, state| state.last_active.elapsed() <= DECODER_IDLE_TIMEOUT);
    }

    /// Clears all active sender decoders (e.g. on disconnect).
    pub fn clear_decoders(&mut self) {
        self.decoders.clear();
    }

    pub fn active_decoder_count(&self) -> usize {
        self.decoders.len()
    }

    pub fn active_sender_uuids(&self) -> Vec<u128> {
        let mut uuids: Vec<u128> = self.decoders.keys().copied().collect();
        uuids.sort_unstable();
        uuids
    }

    /// Calculate distance attenuation and stereo panning using vanilla Simple Voice Chat distance curve.
    pub fn compute_spatial_gains(
        listener_pos: [f64; 3],
        listener_yaw: f32,
        source_pos: [f32; 3],
        max_distance: f32,
    ) -> (f32, f32) {
        let dx = source_pos[0] - listener_pos[0] as f32;
        let dy = source_pos[1] - listener_pos[1] as f32;
        let dz = source_pos[2] - listener_pos[2] as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        if distance >= max_distance || distance <= 0.001 {
            if distance <= 0.001 {
                return (1.0, 1.0);
            }
            return (0.0, 0.0);
        }

        let fade_distance = (max_distance * 0.50).max(4.0);
        let attenuation = if distance <= fade_distance {
            1.0
        } else {
            1.0 - ((distance - fade_distance) / (max_distance - fade_distance).max(0.001))
                .clamp(0.0, 1.0)
        };

        let yaw_rad = -listener_yaw.to_radians();
        let cos_y = yaw_rad.cos();
        let sin_y = yaw_rad.sin();
        let local_x = dx * cos_y - dz * sin_y;

        let pan = (local_x / distance).clamp(-1.0, 1.0);
        // Constant-Power Panning: center pan (0.0) gives full 1.0 gain in both channels
        let left_pan = (1.0 - pan.max(0.0) * 0.7).clamp(0.2, 1.0);
        let right_pan = (1.0 + pan.min(0.0) * 0.7).clamp(0.2, 1.0);

        let left_gain = (attenuation * left_pan).clamp(0.0, 1.0);
        let right_gain = (attenuation * right_pan).clamp(0.0, 1.0);

        (left_gain, right_gain)
    }

    /// Calculate distance attenuation and stereo panning based on 3D coordinates and camera yaw.
    pub fn calculate_spatial_gain(
        listener_pos: [f64; 3],
        listener_yaw: f32,
        source_pos: [f64; 3],
        max_distance: f32,
    ) -> (f32, f32) {
        Self::compute_spatial_gains(
            listener_pos,
            listener_yaw,
            [
                source_pos[0] as f32,
                source_pos[1] as f32,
                source_pos[2] as f32,
            ],
            max_distance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn pearson_correlation(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        let n = a.len() as f32;
        let mean_a = a.iter().sum::<f32>() / n;
        let mean_b = b.iter().sum::<f32>() / n;
        let mut num = 0.0f32;
        let mut den_a = 0.0f32;
        let mut den_b = 0.0f32;
        for (&xa, &xb) in a.iter().zip(b.iter()) {
            let da = xa - mean_a;
            let db = xb - mean_b;
            num += da * db;
            den_a += da * da;
            den_b += db * db;
        }
        if den_a <= 1e-12 || den_b <= 1e-12 {
            return 0.0;
        }
        num / (den_a.sqrt() * den_b.sqrt())
    }

    #[test]
    fn test_spatial_gain_calculation() {
        let listener = [0.0, 64.0, 0.0];
        let yaw = 0.0;

        let sound_ahead = [0.0, 64.0, 24.0];
        let (left, right) =
            VoicePlaybackPipeline::calculate_spatial_gain(listener, yaw, sound_ahead, 48.0);
        assert_eq!(left, 1.0);
        assert_eq!(right, 1.0);

        let sound_far = [0.0, 64.0, 100.0];
        let (left_far, right_far) =
            VoicePlaybackPipeline::calculate_spatial_gain(listener, yaw, sound_far, 48.0);
        assert_eq!(left_far, 0.0);
        assert_eq!(right_far, 0.0);

        let sound_right = [20.0, 64.0, 0.0];
        let (left_r, right_r) =
            VoicePlaybackPipeline::calculate_spatial_gain(listener, yaw, sound_right, 48.0);
        assert!(right_r > left_r);
    }

    #[test]
    fn test_opus_440hz_sine_roundtrip() {
        let mut capture = VoiceCapturePipeline::new();
        let mut playback = VoicePlaybackPipeline::new();
        let sender_uuid: u128 = 0x1234_5678_9ABC_DEF0_1122_3344_5566_7788;

        // Generate 10 consecutive 20ms frames (200ms = 9600 samples) of a 440 Hz sine wave at 48 kHz
        let num_frames = 10;
        let total_samples = FRAME_SIZE * num_frames;
        let mut input_stream = Vec::with_capacity(total_samples);
        let mut decoded_stream = Vec::with_capacity(total_samples);

        for frame_idx in 0..num_frames {
            let mut frame = [0.0f32; FRAME_SIZE];
            for i in 0..FRAME_SIZE {
                let sample_idx = frame_idx * FRAME_SIZE + i;
                let t = (sample_idx as f32) / (SAMPLE_RATE as f32);
                frame[i] = (TAU * 440.0 * t).sin() * 0.8;
            }
            input_stream.extend_from_slice(&frame);

            let encoded = capture
                .encode_frame(&frame)
                .expect("Opus encoder must produce packet");
            assert!(!encoded.is_empty());
            assert!(encoded.len() < FRAME_SIZE);

            let decoded = playback.decode_sender_frame(sender_uuid, frame_idx as i64, &encoded);
            assert_eq!(decoded.len(), FRAME_SIZE);
            decoded_stream.extend_from_slice(&decoded);
        }

        // Opus VOIP at 48kHz has a standard lookahead (algorithmic delay) of ~312 samples (6.5ms).
        // Evaluate correlation after warm-up (frames 2..10) compensated for algorithmic delay.
        let warmup = FRAME_SIZE * 2;
        let mut best_corr = -1.0f32;
        let mut best_lag = 0usize;
        for lag in 0..=400 {
            let len = total_samples - warmup - lag;
            let corr = pearson_correlation(
                &input_stream[warmup..warmup + len],
                &decoded_stream[warmup + lag..warmup + lag + len],
            );
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }

        println!(
            "Opus 440Hz roundtrip correlation: r = {:.6} (lookahead lag = {} samples)",
            best_corr, best_lag
        );
        assert!(
            best_corr > 0.95,
            "Expected high Opus 440Hz sine correlation > 0.95, got {best_corr}"
        );
    }

    #[test]
    fn test_empty_opus_data_resets_decoder_without_decoding() {
        let mut capture = VoiceCapturePipeline::new();
        let mut playback = VoicePlaybackPipeline::new();
        let sender = 0xAA_u128;

        let frame = [0.25f32; FRAME_SIZE];
        let pkt = capture.encode_frame(&frame).unwrap();
        let pcm = playback.decode_sender_frame(sender, 1, &pkt);
        assert_eq!(pcm.len(), FRAME_SIZE);

        // Empty opus_data = sender stopped speaking -> reset decoder, return empty Vec
        let stop_pcm = playback.decode_sender_frame(sender, 2, &[]);
        assert!(stop_pcm.is_empty());
    }

    #[test]
    fn test_plc_and_per_sender_decoder_isolation() {
        let mut capture = VoiceCapturePipeline::new();
        let mut playback = VoicePlaybackPipeline::new();
        let sender_a = 1u128;
        let sender_b = 2u128;

        let mut frame = [0.0f32; FRAME_SIZE];
        for (i, sample) in frame.iter_mut().enumerate() {
            *sample = ((i as f32) * 0.05).sin() * 0.5;
        }
        let pkt1 = capture.encode_frame(&frame).unwrap();
        let pkt2 = capture.encode_frame(&frame).unwrap();
        let pkt3 = capture.encode_frame(&frame).unwrap();

        assert_eq!(
            playback.decode_sender_frame(sender_a, 1, &pkt1).len(),
            FRAME_SIZE
        );
        assert_eq!(
            playback.decode_sender_frame(sender_b, 10, &pkt2).len(),
            FRAME_SIZE
        );
        assert_eq!(playback.active_decoder_count(), 2);

        // Sequence jump 1 -> 3 for sender_a (packet 2 lost): triggers 1 PLC frame + 1 decoded frame
        let recovered = playback.decode_sender_frame(sender_a, 3, &pkt3);
        assert_eq!(recovered.len(), FRAME_SIZE * 2);

        // Late packet (seq 2 after seq 3) is ignored
        let late = playback.decode_sender_frame(sender_a, 2, &pkt2);
        assert!(late.is_empty());
    }

    #[test]
    fn test_voice_capture_dual_subframe_vad() {
        let mut capture = VoiceCapturePipeline::new();
        let input = [0.05f32; FRAME_SIZE];

        let result = capture.process_frame(&input, true, 1.0, false, 0.5, true);
        assert!(result.is_some());
        assert_eq!(capture.current_sequence(), 1);
    }
}
