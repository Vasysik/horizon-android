//! Simple Voice Chat (SVC) Dynamic Protocol Configuration.
//!
//! Allows runtime configuration of protocol compatibility versions and voice chat settings
//! without requiring binary recompilation or APK rebuilds.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_COMPATIBILITY_VERSION: i32 = 21;
pub const DEFAULT_FALLBACK_VERSIONS: [i32; 5] = [21, 20, 19, 18, 22];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvcProtocolConfig {
    /// Active protocol compatibility version sent to the server (default: 21)
    pub compatibility_version: i32,
    /// List of fallback versions to attempt during auto-negotiation
    pub fallback_versions: Vec<i32>,
    /// Index into fallback_versions if auto-negotiating
    #[serde(default)]
    pub fallback_index: usize,
    /// Microphone mute toggle
    pub muted: bool,
    /// Audio playback disable toggle
    pub disabled: bool,
    /// Activation mode: false = Push-to-Talk, true = Voice Activation
    pub voice_activation: bool,
    /// Voice Activity Detection (VAD) threshold (0.0 .. 1.0)
    pub vad_threshold: f32,
    /// Microphone gain multiplier (e.g. 1.0 = 100%)
    pub mic_gain: f32,
    /// Active RNNoise neural denoiser toggle
    pub rnnoise_enabled: bool,
    /// Desktop PTT key name
    pub ptt_key: String,
    /// Currently joined group UUID (if any)
    pub group_id: Option<u128>,
}

impl Default for SvcProtocolConfig {
    fn default() -> Self {
        Self {
            compatibility_version: DEFAULT_COMPATIBILITY_VERSION,
            fallback_versions: DEFAULT_FALLBACK_VERSIONS.to_vec(),
            fallback_index: 0,
            muted: false,
            disabled: false,
            voice_activation: false,
            vad_threshold: 0.5,
            mic_gain: 1.0,
            rnnoise_enabled: true,
            ptt_key: "Caps Lock".to_string(),
            group_id: None,
        }
    }
}

impl SvcProtocolConfig {
    pub const FILE_NAME: &'static str = "svc_protocol.json";

    /// Load config from the given directory, or return default if missing or corrupted.
    pub fn load_from_dir(dir: &Path) -> Self {
        let path = dir.join(Self::FILE_NAME);
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Self {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(config) = serde_json::from_slice::<Self>(&bytes) {
                return config;
            }
        }
        let config = Self::default();
        let _ = config.save_to_path(path);
        config
    }

    /// Save config to the given directory.
    pub fn save_to_dir(&self, dir: &Path) -> std::io::Result<()> {
        let path = dir.join(Self::FILE_NAME);
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, data)
    }

    /// Select the next fallback version in order. Returns the newly selected version.
    pub fn next_fallback_version(&mut self) -> i32 {
        if self.fallback_versions.is_empty() {
            return self.compatibility_version;
        }
        self.fallback_index = (self.fallback_index + 1) % self.fallback_versions.len();
        self.compatibility_version = self.fallback_versions[self.fallback_index];
        self.compatibility_version
    }

    /// Explicitly set the compatibility version (e.g. from in-game GUI).
    pub fn set_version(&mut self, version: i32) {
        self.compatibility_version = version;
        if let Some(pos) = self.fallback_versions.iter().position(|&v| v == version) {
            self.fallback_index = pos;
        }
    }
}
