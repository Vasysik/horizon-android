//! Simple Voice Chat (SVC) In-Game GUI & HUD Overlay.
//!
//! Provides the in-game settings modal (Keybind 'V' / Android Voice button)
//! and bottom-left HUD status indicators matching Simple Voice Chat Java.

use super::config::SvcProtocolConfig;
use super::udp_worker::UdpConnectionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceTab {
    General,
    Settings,
    Groups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceHudStatus {
    Disconnected,
    Disabled,
    Muted,
    Speaking,
    Whispering,
    Idle,
}

/// State of the In-Game Voice Chat Menu
#[derive(Debug, Clone)]
pub struct VoiceChatScreenState {
    pub active_tab: VoiceTab,
    pub input_version: i32,
    pub new_group_name: String,
    pub new_group_password: String,
    pub new_group_type: i16,
    pub status_message: Option<String>,
}

impl VoiceChatScreenState {
    pub fn new(current_version: i32) -> Self {
        Self {
            active_tab: VoiceTab::General,
            input_version: current_version,
            new_group_name: String::new(),
            new_group_password: String::new(),
            new_group_type: 0,
            status_message: None,
        }
    }
}

/// Helper that determines the active HUD icon to display on the HUD bottom-left.
pub fn determine_hud_status(
    connection: &UdpConnectionState,
    config: &SvcProtocolConfig,
    is_talking: bool,
    is_whispering: bool,
) -> VoiceHudStatus {
    match connection {
        UdpConnectionState::Disconnected | UdpConnectionState::Failed(_) => {
            VoiceHudStatus::Disconnected
        }
        UdpConnectionState::Connecting | UdpConnectionState::Authenticating => {
            VoiceHudStatus::Disconnected
        }
        UdpConnectionState::Connected { .. } => {
            if config.disabled {
                VoiceHudStatus::Disabled
            } else if config.muted {
                VoiceHudStatus::Muted
            } else if is_whispering && is_talking {
                VoiceHudStatus::Whispering
            } else if is_talking {
                VoiceHudStatus::Speaking
            } else {
                VoiceHudStatus::Idle
            }
        }
    }
}

/// Generates localized string labels and tooltips for the Voice Chat UI.
pub struct VoiceChatUiLabels;

impl VoiceChatUiLabels {
    pub const TITLE: &'static str = "Voice Chat";
    pub const TAB_GENERAL: &'static str = "General";
    pub const TAB_SETTINGS: &'static str = "Settings";
    pub const TAB_GROUPS: &'static str = "Groups";

    pub const MUTE_MIC: &'static str = "Microphone: ";
    pub const DISABLE_VOICE: &'static str = "Voice Chat: ";
    pub const ACTIVATION_TYPE: &'static str = "Activation: ";
    pub const PTT_MODE: &'static str = "Push-to-Talk";
    pub const VOICE_ACTIVATION_MODE: &'static str = "Voice Activation";
    pub const RNNOISE_TOGGLE: &'static str = "Noise Suppression (RNNoise): ";
    pub const MIC_GAIN: &'static str = "Mic Amplification: ";
    pub const VAD_THRESHOLD: &'static str = "VAD Threshold: ";
    pub const PROTOCOL_VERSION: &'static str = "Compatibility Version: ";
    pub const NEXT_FALLBACK: &'static str = "Cycle Fallback Version";
    pub const LEAVE_GROUP: &'static str = "Leave Group";
    pub const CREATE_GROUP: &'static str = "Create Group";
    pub const JOIN_GROUP: &'static str = "Join";
}
