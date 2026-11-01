//! Simple Voice Chat (SVC) Client Subsystem for Horizon.
//!
//! Provides full protocol parity with Simple Voice Chat Java Edition (version 21),
//! including AES-128-GCM encrypted UDP transport, 14 CustomPayload TCP channels,
//! RNNoise dual sub-frame neural noise reduction, 3D spatial gain audio,
//! dynamic protocol negotiation, desktop keybinds, and Android touch controls.

pub mod audio_pipeline;
pub mod config;
pub mod protocol;
pub mod udp_worker;
pub mod ui;

use audio_pipeline::{VoiceCapturePipeline, VoicePlaybackPipeline, FRAME_SIZE};
use config::SvcProtocolConfig;
use protocol::*;
use std::collections::HashMap;
use std::path::Path;
use udp_worker::{UdpConnectionState, UdpWorker, UdpWorkerHandle};
use ui::{VoiceChatScreenState, VoiceHudStatus};

pub struct VoiceChatClient {
    pub config: SvcProtocolConfig,
    pub udp_handle: Option<UdpWorkerHandle>,
    pub secret: Option<SecretPacket>,
    pub capture_pipeline: VoiceCapturePipeline,
    pub playback_pipeline: VoicePlaybackPipeline,
    pub player_states: HashMap<u128, PlayerState>,
    pub groups: HashMap<u128, ClientGroup>,
    pub screen_state: VoiceChatScreenState,
    pub ptt_pressed: bool,
    pub whispering: bool,
    pub is_talking: bool,
    pub config_dir: std::path::PathBuf,
    pub fallback_timer: Option<std::time::Instant>,
    pub current_server_host: String,
}

impl VoiceChatClient {
    pub fn new(config_dir: &Path) -> Self {
        let config = SvcProtocolConfig::load_from_dir(config_dir);
        let screen_state = VoiceChatScreenState::new(config.compatibility_version);
        Self {
            config,
            udp_handle: None,
            secret: None,
            capture_pipeline: VoiceCapturePipeline::new(),
            playback_pipeline: VoicePlaybackPipeline::new(),
            player_states: HashMap::new(),
            groups: HashMap::new(),
            screen_state,
            ptt_pressed: false,
            whispering: false,
            is_talking: false,
            config_dir: config_dir.to_path_buf(),
            fallback_timer: None,
            current_server_host: String::new(),
        }
    }

    /// Set current connected server hostname for UDP fallback
    pub fn set_server_host(&mut self, host: &str) {
        self.current_server_host = host.to_string();
    }

    /// Build RequestSecretPacket payload to initiate voice chat handshake on server join.
    pub fn initiate_handshake(&mut self) -> (String, Vec<u8>) {
        self.fallback_timer = Some(std::time::Instant::now());
        let pkt = RequestSecretPacket::new(self.config.compatibility_version);
        (CHANNEL_REQUEST_SECRET.to_string(), pkt.to_bytes())
    }

    /// Check for 4-second timeout during initial handshake and cycle fallback version if needed.
    pub fn check_handshake_timeout(&mut self) -> Option<(String, Vec<u8>)> {
        if let Some(timer) = self.fallback_timer {
            if self.secret.is_none() && timer.elapsed() >= std::time::Duration::from_secs(4) {
                let next_version = self.config.next_fallback_version();
                let _ = self.config.save_to_dir(&self.config_dir);
                self.fallback_timer = Some(std::time::Instant::now());
                log::info!("SVC: Handshake timed out, retrying with fallback compatibility version {next_version}");
                let pkt = RequestSecretPacket::new(next_version);
                return Some((CHANNEL_REQUEST_SECRET.to_string(), pkt.to_bytes()));
            }
        }
        None
    }

    /// Handle incoming Minecraft CustomPayload packets for all voicechat:* channels.
    pub fn handle_custom_payload(
        &mut self,
        channel: &str,
        data: &[u8],
    ) -> Result<Option<(String, Vec<u8>)>, String> {
        match channel {
            CHANNEL_SECRET => {
                let secret = SecretPacket::from_bytes(data)
                    .map_err(|e| format!("Failed to parse SecretPacket: {e}"))?;
                log::info!(
                    "SVC: Received secret from server, port: {}, codec: {}, mtu: {}",
                    secret.server_port,
                    secret.codec,
                    secret.mtu_size
                );

                // Stop previous UDP worker if any
                if let Some(handle) = self.udp_handle.take() {
                    handle.stop();
                }

                let handle = UdpWorker::spawn(secret.clone(), &self.current_server_host)?;
                self.udp_handle = Some(handle);
                self.secret = Some(secret);
                self.fallback_timer = None;

                // Send back initial client state
                let state_pkt = UpdateStatePacket {
                    muted: self.config.muted,
                    disabled: self.config.disabled,
                };
                return Ok(Some((
                    CHANNEL_CLIENT_STATE.to_string(),
                    state_pkt.to_bytes(),
                )));
            }
            CHANNEL_PLAYER_STATE => {
                if let Ok(pkt) = PlayerStatePacket::from_bytes(data) {
                    self.player_states.insert(pkt.state.uuid, pkt.state);
                }
            }
            CHANNEL_PLAYER_STATES => {
                if let Ok(pkt) = PlayerStatesPacket::from_bytes(data) {
                    for s in pkt.states {
                        self.player_states.insert(s.uuid, s);
                    }
                }
            }
            CHANNEL_REMOVE_STATE => {
                if let Ok(pkt) = RemovePlayerStatePacket::from_bytes(data) {
                    self.player_states.remove(&pkt.player_uuid);
                }
            }
            CHANNEL_ADD_GROUP => {
                if let Ok(pkt) = AddGroupPacket::from_bytes(data) {
                    self.groups.insert(pkt.group.id, pkt.group);
                }
            }
            CHANNEL_REMOVE_GROUP => {
                if let Ok(pkt) = RemoveGroupPacket::from_bytes(data) {
                    self.groups.remove(&pkt.group_id);
                }
            }
            CHANNEL_JOINED_GROUP => {
                if let Ok(pkt) = JoinedGroupPacket::from_bytes(data) {
                    if pkt.wrong_password {
                        self.screen_state.status_message =
                            Some("Incorrect group password".to_string());
                    } else {
                        self.config.group_id = pkt.group_id;
                        let _ = self.config.save_to_dir(&self.config_dir);
                        self.screen_state.status_message = if pkt.group_id.is_some() {
                            Some("Joined group successfully".to_string())
                        } else {
                            Some("Left group".to_string())
                        };
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    /// Toggle mute microphone state
    pub fn toggle_mute(&mut self) -> (String, Vec<u8>) {
        self.config.muted = !self.config.muted;
        let _ = self.config.save_to_dir(&self.config_dir);
        let pkt = UpdateStatePacket {
            muted: self.config.muted,
            disabled: self.config.disabled,
        };
        (CHANNEL_CLIENT_STATE.to_string(), pkt.to_bytes())
    }

    /// Toggle disable voice chat state
    pub fn toggle_disable(&mut self) -> (String, Vec<u8>) {
        self.config.disabled = !self.config.disabled;
        let _ = self.config.save_to_dir(&self.config_dir);
        let pkt = UpdateStatePacket {
            muted: self.config.muted,
            disabled: self.config.disabled,
        };
        (CHANNEL_CLIENT_STATE.to_string(), pkt.to_bytes())
    }

    /// Set Push-to-Talk key held state
    pub fn set_ptt_pressed(&mut self, pressed: bool) {
        self.ptt_pressed = pressed;
    }

    /// Set whispering key held state
    pub fn set_whispering(&mut self, whispering: bool) {
        self.whispering = whispering;
    }

    /// Poll and process microphone input audio buffer
    pub fn process_mic_audio(&mut self, pcm_in: &[f32; FRAME_SIZE]) {
        if self.config.disabled || self.config.muted {
            self.is_talking = false;
            return;
        }

        if let Some(opus_data) = self.capture_pipeline.process_frame(
            pcm_in,
            self.config.rnnoise_enabled,
            self.config.mic_gain,
            self.config.voice_activation,
            self.config.vad_threshold,
            self.ptt_pressed,
        ) {
            self.is_talking = true;
            if let Some(ref handle) = self.udp_handle {
                let mic = MicPacket {
                    opus_data,
                    sequence_number: self.capture_pipeline.current_sequence(),
                    whispering: self.whispering,
                };
                let _ = handle.outgoing_mic_tx.send(mic);
            }
        } else {
            self.is_talking = false;
        }
    }

    /// Retrieve active connection state
    pub fn connection_state(&self) -> UdpConnectionState {
        if let Some(ref handle) = self.udp_handle {
            handle.state.read().unwrap().clone()
        } else {
            UdpConnectionState::Disconnected
        }
    }

    /// Determine HUD status icon to render
    pub fn hud_status(&self) -> VoiceHudStatus {
        let conn = self.connection_state();
        ui::determine_hud_status(&conn, &self.config, self.is_talking, self.whispering)
    }

    /// Close active connection and reset state on server disconnect
    pub fn disconnect(&mut self) {
        if let Some(handle) = self.udp_handle.take() {
            handle.stop();
        }
        self.secret = None;
        self.player_states.clear();
        self.groups.clear();
        self.is_talking = false;
    }
}
