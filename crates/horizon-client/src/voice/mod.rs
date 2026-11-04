//! Simple Voice Chat (SVC) Client Subsystem for Horizon.

pub mod audio_pipeline;
pub mod config;
pub mod hardware_io;
pub mod protocol;
pub mod udp_worker;
pub mod ui;

pub use protocol::parse_voice_host;

use audio_pipeline::{FRAME_SIZE, VoiceCapturePipeline, VoicePlaybackPipeline};
use config::SvcProtocolConfig;
use hardware_io::VoiceHardwareIo;
use protocol::*;
use std::collections::HashMap;
use std::path::Path;
use udp_worker::{IncomingSound, UdpConnectionState, UdpWorker, UdpWorkerHandle};
use ui::{VoiceChatScreenState, VoiceHudStatus};

pub struct VoiceChatClient {
    pub config: SvcProtocolConfig,
    pub udp_handle: Option<UdpWorkerHandle>,
    pub secret: Option<SecretPacket>,
    pub capture_pipeline: VoiceCapturePipeline,
    pub playback_pipeline: VoicePlaybackPipeline,
    pub hardware_io: VoiceHardwareIo,
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
            hardware_io: VoiceHardwareIo::new(),
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

    pub fn set_server_host(&mut self, host: &str) {
        self.current_server_host = host.to_string();
    }

    pub fn initiate_handshake(&mut self) -> (String, Vec<u8>) {
        self.fallback_timer = Some(std::time::Instant::now());
        let pkt = RequestSecretPacket::new(self.config.compatibility_version);
        (CHANNEL_REQUEST_SECRET.to_string(), pkt.to_bytes())
    }

    pub fn check_handshake_timeout(&mut self) -> Option<(String, Vec<u8>)> {
        if let Some(timer) = self.fallback_timer
            && self.secret.is_none()
            && timer.elapsed() >= std::time::Duration::from_secs(4)
        {
            let next_version = self.config.next_fallback_version();
            let _ = self.config.save_to_dir(&self.config_dir);
            self.fallback_timer = Some(std::time::Instant::now());
            log::info!(
                "SVC: Handshake timed out, retrying with fallback compatibility version {next_version}"
            );
            let pkt = RequestSecretPacket::new(next_version);
            return Some((CHANNEL_REQUEST_SECRET.to_string(), pkt.to_bytes()));
        }
        None
    }

    pub fn handle_custom_payload(
        &mut self,
        channel: &str,
        data: &[u8],
    ) -> Result<Option<(String, Vec<u8>)>, String> {
        match channel {
            CHANNEL_SECRET => {
                log::info!("SVC: Received SecretPacket payload ({} bytes)", data.len());
                let secret = SecretPacket::from_bytes(data)
                    .map_err(|e| format!("Failed to parse SecretPacket: {e}"))?;
                if !(1..=65535).contains(&secret.server_port) {
                    return Err(format!(
                        "Invalid server port in SecretPacket: {}",
                        secret.server_port
                    ));
                }
                log::info!(
                    "SVC: Received secret from server, port: {}, codec: {}, mtu: {}",
                    secret.server_port,
                    secret.codec,
                    secret.mtu_size
                );

                self.fallback_timer = None;

                if let Some(handle) = self.udp_handle.take() {
                    handle.stop();
                }

                let handle = UdpWorker::spawn(secret.clone(), &self.current_server_host)?;
                self.udp_handle = Some(handle);
                self.secret = Some(secret);

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

    pub fn toggle_mute(&mut self) -> (String, Vec<u8>) {
        self.config.muted = !self.config.muted;
        let _ = self.config.save_to_dir(&self.config_dir);
        let pkt = UpdateStatePacket {
            muted: self.config.muted,
            disabled: self.config.disabled,
        };
        (CHANNEL_CLIENT_STATE.to_string(), pkt.to_bytes())
    }

    pub fn toggle_disable(&mut self) -> (String, Vec<u8>) {
        self.config.disabled = !self.config.disabled;
        let _ = self.config.save_to_dir(&self.config_dir);
        let pkt = UpdateStatePacket {
            muted: self.config.muted,
            disabled: self.config.disabled,
        };
        (CHANNEL_CLIENT_STATE.to_string(), pkt.to_bytes())
    }

    pub fn set_ptt_pressed(&mut self, pressed: bool) {
        self.ptt_pressed = pressed;
    }

    pub fn set_whispering(&mut self, whispering: bool) {
        self.whispering = whispering;
    }

    /// Process a single 960-sample microphone frame and queue it to the UDP worker.
    pub fn process_mic_audio(&mut self, pcm_in: &[f32; FRAME_SIZE]) {
        if self.config.disabled || self.config.muted {
            if self.is_talking {
                self.capture_pipeline.reset_encoder();
            }
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
                let _ = handle.outgoing_mic_tx.try_send(mic);
            }
        } else {
            if self.is_talking {
                self.capture_pipeline.reset_encoder();
            }
            self.is_talking = false;
        }
    }

    /// Called every game tick: polls hardware microphone frames when active and
    /// decodes + mixes incoming UDP voice packets with 3D spatial attenuation.
    pub fn tick_audio<F>(
        &mut self,
        listener_pos: [f64; 3],
        listener_yaw: f32,
        resolve_player_pos: F,
    ) where
        F: Fn(u128) -> Option<[f64; 3]>,
    {
        let is_connected = self.udp_handle.as_ref().is_some_and(|h| h.is_connected());
        if !is_connected || self.config.disabled {
            self.is_talking = false;
            return;
        }

        // 1. Capture & send microphone frames if unmuted and (voice activation OR PTT held)
        if !self.config.muted && (self.config.voice_activation || self.ptt_pressed) {
            let mut processed_any = false;
            while let Some(frame) = self.hardware_io.poll_mic_frame() {
                processed_any = true;
                self.process_mic_audio(&frame);
            }
            if !processed_any && !self.ptt_pressed && !self.config.voice_activation {
                self.is_talking = false;
            }
        } else {
            if self.is_talking {
                self.capture_pipeline.reset_encoder();
            }
            self.is_talking = false;
        }

        // Prune sender decoders that have been idle > DECODER_IDLE_TIMEOUT
        self.playback_pipeline.prune_idle_decoders();

        // 2. Drain incoming UDP voice packets and play them on the speaker with 3D spatial gain
        let default_max_dist = self
            .secret
            .as_ref()
            .map(|s| s.voice_chat_distance as f32)
            .unwrap_or(48.0);

        let mut incoming = Vec::new();
        if let Some(ref handle) = self.udp_handle {
            while let Ok(pkt) = handle.incoming_sound_rx.try_recv() {
                incoming.push(pkt);
            }
        }

        for sound in incoming {
            match sound {
                IncomingSound::Player(pkt) => {
                    let pcm = self.playback_pipeline.decode_sender_frame(
                        pkt.sender_uuid,
                        pkt.sequence_number,
                        &pkt.opus_data,
                    );
                    if pcm.is_empty() {
                        continue;
                    }
                    let max_dist = if pkt.distance > 0.0 {
                        pkt.distance
                    } else {
                        default_max_dist
                    };
                    let (left, right) = if let Some(src_pos) = resolve_player_pos(pkt.sender_uuid) {
                        VoicePlaybackPipeline::calculate_spatial_gain(
                            listener_pos,
                            listener_yaw,
                            src_pos,
                            max_dist,
                        )
                    } else {
                        (0.8, 0.8)
                    };
                    self.hardware_io.push_spatial_audio(&pcm, left, right);
                }
                IncomingSound::Group(pkt) => {
                    let pcm = self.playback_pipeline.decode_sender_frame(
                        pkt.sender_uuid,
                        pkt.sequence_number,
                        &pkt.opus_data,
                    );
                    if pcm.is_empty() {
                        continue;
                    }
                    self.hardware_io.push_spatial_audio(&pcm, 1.0, 1.0);
                }
                IncomingSound::Location(pkt) => {
                    let pcm = self.playback_pipeline.decode_sender_frame(
                        pkt.sender_uuid,
                        pkt.sequence_number,
                        &pkt.opus_data,
                    );
                    if pcm.is_empty() {
                        continue;
                    }
                    let max_dist = if pkt.distance > 0.0 {
                        pkt.distance
                    } else {
                        default_max_dist
                    };
                    let (left, right) = VoicePlaybackPipeline::calculate_spatial_gain(
                        listener_pos,
                        listener_yaw,
                        pkt.location,
                        max_dist,
                    );
                    self.hardware_io.push_spatial_audio(&pcm, left, right);
                }
            }
        }
    }

    pub fn connection_state(&self) -> UdpConnectionState {
        if let Some(ref handle) = self.udp_handle {
            handle
                .state
                .read()
                .map(|s| s.clone())
                .unwrap_or(UdpConnectionState::Disconnected)
        } else {
            UdpConnectionState::Disconnected
        }
    }

    pub fn hud_status(&self) -> VoiceHudStatus {
        let conn = self.connection_state();
        ui::determine_hud_status(&conn, &self.config, self.is_talking, self.whispering)
    }

    pub fn disconnect(&mut self) {
        if let Some(handle) = self.udp_handle.take() {
            handle.stop();
        }
        self.hardware_io.stop();
        self.capture_pipeline.reset_encoder();
        self.playback_pipeline.clear_decoders();
        self.secret = None;
        self.player_states.clear();
        self.groups.clear();
        self.is_talking = false;
    }
}
