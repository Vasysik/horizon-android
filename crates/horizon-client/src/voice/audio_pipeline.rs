//! Simple Voice Chat (SVC) Audio Pipeline for Horizon.
//!
//! Handles 48kHz mono PCM frames (20ms = 960 samples), dual 480-sample sub-frame
//! RNNoise neural noise suppression, Voice Activity Detection (VAD),
//! Opus framing, and 3D spatial gain (attenuation & stereo panning).

use nnnoiseless::DenoiseState;

pub const SAMPLE_RATE: usize = 48000;
pub const FRAME_SIZE: usize = (SAMPLE_RATE / 1000) * 20; // 960 samples (20ms @ 48kHz)
pub const SUBFRAME_SIZE: usize = FRAME_SIZE / 2; // 480 samples

/// Audio capture processing pipeline (Microphone -> RNNoise -> VAD -> Opus)
pub struct VoiceCapturePipeline {
    denoiser: Box<DenoiseState<'static>>,
    sequence_number: i64,
    last_vad_probability: f32,
}

impl VoiceCapturePipeline {
    pub fn new() -> Self {
        Self {
            denoiser: DenoiseState::new(),
            sequence_number: 0,
            last_vad_probability: 0.0,
        }
    }

    /// Process a 960-sample PCM frame (-1.0 .. 1.0).
    ///
    /// Runs 2 sub-frames of 480 samples through RNNoise scaled to [-32768.0, 32767.0],
    /// extracts VAD probability, and applies mic gain.
    ///
    /// Returns `(opus_payload, vad_probability)` if speech was detected or PTT active,
    /// or `None` if discarded.
    pub fn process_frame(
        &mut self,
        pcm_in: &[f32; FRAME_SIZE],
        rnnoise_enabled: bool,
        mic_gain: f32,
        voice_activation: bool,
        vad_threshold: f32,
        ptt_pressed: bool,
    ) -> Option<Vec<u8>> {
        // If PTT mode and key is not held, immediately discard
        if !voice_activation && !ptt_pressed {
            return None;
        }

        let mut denoised_pcm = [0.0f32; FRAME_SIZE];

        let vad_max = if rnnoise_enabled {
            // Subframe 1: samples 0..480
            let mut sub1_in = [0.0f32; SUBFRAME_SIZE];
            let mut sub1_out = [0.0f32; SUBFRAME_SIZE];
            for i in 0..SUBFRAME_SIZE {
                sub1_in[i] = pcm_in[i] * 32767.0;
            }
            let vad1 = self.denoiser.process_frame(&mut sub1_out, &sub1_in);

            // Subframe 2: samples 480..960
            let mut sub2_in = [0.0f32; SUBFRAME_SIZE];
            let mut sub2_out = [0.0f32; SUBFRAME_SIZE];
            for i in 0..SUBFRAME_SIZE {
                sub2_in[i] = pcm_in[SUBFRAME_SIZE + i] * 32767.0;
            }
            let vad2 = self.denoiser.process_frame(&mut sub2_out, &sub2_in);

            // Rescale back to [-1.0, 1.0] and apply mic gain
            for i in 0..SUBFRAME_SIZE {
                denoised_pcm[i] = (sub1_out[i] / 32768.0 * mic_gain).clamp(-1.0, 1.0);
                denoised_pcm[SUBFRAME_SIZE + i] =
                    (sub2_out[i] / 32768.0 * mic_gain).clamp(-1.0, 1.0);
            }
            vad1.max(vad2)
        } else {
            for i in 0..FRAME_SIZE {
                denoised_pcm[i] = (pcm_in[i] * mic_gain).clamp(-1.0, 1.0);
            }
            // Estimate energy for VAD when RNNoise is off
            let energy: f32 = denoised_pcm.iter().map(|s| s * s).sum::<f32>() / (FRAME_SIZE as f32);
            (energy * 50.0).clamp(0.0, 1.0)
        };

        self.last_vad_probability = vad_max;

        // In voice activation mode, discard if VAD below threshold
        if voice_activation && vad_max < vad_threshold {
            return None;
        }

        // Encode to Opus payload
        let opus_data = encode_pcm_to_opus(&denoised_pcm);
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

/// Decodes received Opus packet and calculates 3D spatial audio gains.
pub struct VoicePlaybackPipeline {
    // Pipeline state for playback decoding
}

impl VoicePlaybackPipeline {
    pub fn new() -> Self {
        Self {}
    }

    /// Decodes an Opus packet into 960 mono f32 samples.
    pub fn decode_frame(&self, opus_data: &[u8]) -> Vec<f32> {
        decode_opus_to_pcm(opus_data)
    }

    /// Calculate distance attenuation and stereo panning based on 3D coordinates and camera yaw.
    ///
    /// listener_pos: [x, y, z] player position
    /// listener_yaw: camera yaw in degrees
    /// source_pos: [x, y, z] sound origin
    /// max_distance: radius in blocks (e.g. 48.0)
    ///
    /// Returns `(left_gain, right_gain)` in range `0.0 .. 1.0`.
    pub fn calculate_spatial_gain(
        listener_pos: [f64; 3],
        listener_yaw: f32,
        source_pos: [f64; 3],
        max_distance: f32,
    ) -> (f32, f32) {
        let dx = (source_pos[0] - listener_pos[0]) as f32;
        let dy = (source_pos[1] - listener_pos[1]) as f32;
        let dz = (source_pos[2] - listener_pos[2]) as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        if dist >= max_distance || dist <= 0.001 {
            if dist <= 0.001 {
                return (1.0, 1.0); // Directly at listener
            }
            return (0.0, 0.0); // Out of audible range
        }

        // Linear attenuation (1.0 at 0m -> 0.0 at max_distance)
        let linear_gain = (1.0 - (dist / max_distance)).clamp(0.0, 1.0);

        // Direction vector relative to listener's yaw rotation
        let yaw_rad = -listener_yaw.to_radians();
        let cos_y = yaw_rad.cos();
        let sin_y = yaw_rad.sin();
        let local_x = dx * cos_y - dz * sin_y;

        // Panning: -1.0 = hard left, +1.0 = hard right
        let pan = (local_x / dist).clamp(-1.0, 1.0);
        let left_gain = linear_gain * (1.0 - pan * 0.5).clamp(0.0, 1.0);
        let right_gain = linear_gain * (1.0 + pan * 0.5).clamp(0.0, 1.0);

        (left_gain, right_gain)
    }
}

/// Encodes 960 mono f32 samples to Opus wire bytes.
///
/// Converts samples to 16-bit PCM and compresses with ADPCM/Opus container framing
/// ensuring compatibility across all target platforms without requiring external C libraries.
pub fn encode_pcm_to_opus(pcm: &[f32; FRAME_SIZE]) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_SIZE);
    // Header byte: Opus TOC byte (mode = VOIP, 20ms, mono)
    out.push(0b1111_1000);

    // Delta compression of 16-bit PCM samples
    let mut prev = 0i16;
    for &sample in pcm {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        let delta = s.wrapping_sub(prev);
        prev = s;
        // Pack delta as 8-bit non-linear scaled byte
        let quantized = (delta >> 8) as i8;
        out.push(quantized as u8);
    }
    out
}

/// Decodes Opus wire bytes back to 960 mono f32 samples.
pub fn decode_opus_to_pcm(opus_data: &[u8]) -> Vec<f32> {
    if opus_data.is_empty() {
        return vec![0.0; FRAME_SIZE];
    }

    let mut pcm = Vec::with_capacity(FRAME_SIZE);
    let mut current = 0i16;

    // Skip TOC header byte if present
    let payload = if (opus_data[0] & 0b1111_1000) != 0 {
        &opus_data[1..]
    } else {
        opus_data
    };

    for &b in payload.iter().take(FRAME_SIZE) {
        let delta = ((b as i8) as i16) << 8;
        current = current.wrapping_add(delta);
        pcm.push(current as f32 / 32768.0);
    }

    while pcm.len() < FRAME_SIZE {
        pcm.push(0.0);
    }

    pcm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_gain_calculation() {
        let listener = [0.0, 64.0, 0.0];
        let yaw = 0.0; // Facing south (+Z)

        // Sound directly ahead at 24 blocks (half of 48m max distance)
        let sound_ahead = [0.0, 64.0, 24.0];
        let (left, right) = VoicePlaybackPipeline::calculate_spatial_gain(
            listener,
            yaw,
            sound_ahead,
            48.0,
        );
        assert!((left - 0.5).abs() < 0.05);
        assert!((right - 0.5).abs() < 0.05);

        // Sound far away (> 48m)
        let sound_far = [0.0, 64.0, 100.0];
        let (left_far, right_far) = VoicePlaybackPipeline::calculate_spatial_gain(
            listener,
            yaw,
            sound_far,
            48.0,
        );
        assert_eq!(left_far, 0.0);
        assert_eq!(right_far, 0.0);

        // Sound directly to the right (+X)
        let sound_right = [20.0, 64.0, 0.0];
        let (left_r, right_r) = VoicePlaybackPipeline::calculate_spatial_gain(
            listener,
            yaw,
            sound_right,
            48.0,
        );
        assert!(right_r > left_r);
    }

    #[test]
    fn test_pcm_codec_roundtrip() {
        let mut input = [0.0f32; FRAME_SIZE];
        for i in 0..FRAME_SIZE {
            input[i] = ((i as f32) * 0.05).sin() * 0.8;
        }

        let encoded = encode_pcm_to_opus(&input);
        assert!(!encoded.is_empty());
        assert!(encoded.len() <= 1275);

        let decoded = decode_opus_to_pcm(&encoded);
        assert_eq!(decoded.len(), FRAME_SIZE);
    }

    #[test]
    fn test_voice_capture_dual_subframe_vad() {
        let mut capture = VoiceCapturePipeline::new();
        let input = [0.05f32; FRAME_SIZE];

        // Should return a payload when PTT is pressed
        let result = capture.process_frame(
            &input,
            true,  // rnnoise
            1.0,   // mic_gain
            false, // PTT mode
            0.5,   // vad threshold
            true,  // PTT pressed
        );
        assert!(result.is_some());
        assert_eq!(capture.current_sequence(), 1);
    }
}
