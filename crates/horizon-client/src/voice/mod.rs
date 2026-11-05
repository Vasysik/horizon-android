//! Simple Voice Chat (SVC) Client Subsystem for Horizon.

pub mod audio_pipeline;
pub mod config;
pub mod hardware_io;
pub mod mixer;
pub mod protocol;
pub mod udp_worker;
pub mod ui;

pub use protocol::parse_voice_host;

use audio_pipeline::{FRAME_SIZE, VoiceCapturePipeline, VoicePlaybackPipeline};
use config::SvcProtocolConfig;
use hardware_io::VoiceHardwareIo;
use mixer::GainSource;
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
    pub known_players: Vec<(u128, String)>,
    pub show_player_volumes: bool,
    pub player_list_page: usize,
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
            known_players: Vec::new(),
            show_player_volumes: false,
            player_list_page: 0,
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
            && timer.elapsed() >= std::time::Duration::from_secs(8)
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

    pub fn delay_handshake_timer(&mut self) {
        if self.fallback_timer.is_some() && self.secret.is_none() {
            self.fallback_timer = Some(std::time::Instant::now());
        }
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

                let handle = UdpWorker::spawn(
                    secret.clone(),
                    &self.current_server_host,
                    self.hardware_io.mixer(),
                )?;
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
            CHANNEL_PLAYER_STATE | "voicechat:player_state" => {
                if let Ok(pkt) = PlayerStatePacket::from_bytes(data) {
                    let local_uuid = self.secret.as_ref().map(|s| s.player_uuid);
                    if pkt.state.uuid != 0 && Some(pkt.state.uuid) != local_uuid {
                        self.register_known_player(pkt.state.uuid, pkt.state.name.clone());
                    }
                    self.player_states.insert(pkt.state.uuid, pkt.state);
                }
            }
            CHANNEL_PLAYER_STATES | "voicechat:player_states" => {
                if let Ok(pkt) = PlayerStatesPacket::from_bytes(data) {
                    let local_uuid = self.secret.as_ref().map(|s| s.player_uuid);
                    for s in pkt.states {
                        if s.uuid != 0 && Some(s.uuid) != local_uuid {
                            self.register_known_player(s.uuid, s.name.clone());
                        }
                        self.player_states.insert(s.uuid, s);
                    }
                }
            }
            CHANNEL_REMOVE_STATE | "voicechat:remove_player_state" | "voicechat:remove_state" => {
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
        self.hardware_io.drop_input_stream();
        let _ = self.config.save_to_dir(&self.config_dir);
        let pkt = UpdateStatePacket {
            muted: self.config.muted,
            disabled: self.config.disabled,
        };
        (CHANNEL_CLIENT_STATE.to_string(), pkt.to_bytes())
    }

    pub fn set_muted(&mut self, muted: bool) -> (String, Vec<u8>) {
        self.config.muted = muted;
        self.hardware_io.drop_input_stream();
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
        if !pressed && !self.config.voice_activation && self.ptt_pressed {
            self.hardware_io.stop_mic_capture();
        }
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

        // 2. Playback: decoding and jitter buffering run on the UDP thread and the audio callback.
        //    The game tick only refreshes the 3D gain targets (listener/source positions).
        self.hardware_io.ensure_output_started();
        let mixer = self.hardware_io.mixer();
        let sources = match mixer.lock() {
            Ok(m) => m.sources(),
            Err(_) => return,
        };

        const SPEAKER_MAKEUP_BOOST: f32 = 2.6;
        let mic_active_compensation = if self.hardware_io.is_input_stream_open() {
            1.25
        } else {
            1.0
        };
        let default_max_dist = self
            .secret
            .as_ref()
            .map(|s| s.voice_chat_distance as f32)
            .unwrap_or(48.0);

        let mut gains: Vec<(u128, (f32, f32), Option<[f32; 3]>)> =
            Vec::with_capacity(sources.len());

        for (uuid, kind, last_known_pos) in sources {
            self.register_known_player(uuid, String::new());

            if self.config.is_player_muted(uuid) || self.config.disabled {
                gains.push((uuid, (0.0, 0.0), None));
                continue;
            }

            let player_vol = self.config.player_volume(uuid);
            let combined_gain = (self.config.output_volume.clamp(0.0, 3.0)
                * player_vol
                * SPEAKER_MAKEUP_BOOST
                * mic_active_compensation)
                .clamp(0.0, 8.0);

            let (base_l, base_r, updated_pos) = match kind {
                VoiceSourceKind::Group => (1.0, 1.0, None),
                VoiceSourceKind::Location { pos, max_distance } => {
                    let max_dist = if max_distance > 0.0 {
                        max_distance
                    } else {
                        default_max_dist
                    };
                    let (l, r) = VoicePlaybackPipeline::compute_spatial_gains(
                        listener_pos,
                        listener_yaw,
                        pos,
                        max_dist,
                    );
                    (l, r, None)
                }
                VoiceSourceKind::Player { max_distance } => {
                    let max_dist = max_distance.unwrap_or(default_max_dist);
                    if let Some(pos64) = resolve_player_pos(uuid) {
                        let pos = [pos64[0] as f32, pos64[1] as f32, pos64[2] as f32];
                        let (l, r) = VoicePlaybackPipeline::compute_spatial_gains(
                            listener_pos,
                            listener_yaw,
                            pos,
                            max_dist,
                        );
                        (l, r, Some(pos))
                    } else if let Some(last_pos) = last_known_pos {
                        let (l, r) = VoicePlaybackPipeline::compute_spatial_gains(
                            listener_pos,
                            listener_yaw,
                            last_pos,
                            max_dist,
                        );
                        (l, r, None)
                    } else {
                        (0.20, 0.20, None)
                    }
                }
            };

            let gain_l = (base_l * combined_gain).clamp(0.0, 8.0);
            let gain_r = (base_r * combined_gain).clamp(0.0, 8.0);
            gains.push((uuid, (gain_l, gain_r), updated_pos));
        }

        if let Ok(mut m) = mixer.lock() {
            m.apply_gains(&gains);
            if let Some((stats, streams)) = m.take_stats_if_due(std::time::Duration::from_secs(5)) {
                let pushes = stats.pushes;
                let underruns = stats.underruns;
                let drops = stats.overflow_drops;
                let queue_ms = stats.max_queue_samples / 48;
                log::info!(
                    "SVC audio: streams={streams} pushes={pushes} underruns={underruns} overflow_drops={drops} max_queue_ms={queue_ms}"
                );
            }
        }
    }

    pub fn register_known_player(&mut self, uuid: u128, name: String) {
        if uuid == 0 {
            return;
        }
        if let Some(entry) = self.known_players.iter_mut().find(|(id, _)| *id == uuid) {
            if !name.is_empty()
                && (entry.1.is_empty() || entry.1.starts_with("Player ") || entry.1 != name)
            {
                entry.1 = name;
            }
            return;
        }
        if self.known_players.len() >= 128 {
            self.known_players.remove(0);
        }
        let display = if name.is_empty() {
            format!("Player {:04x}", (uuid & 0xFFFF) as u16)
        } else {
            name
        };
        self.known_players.push((uuid, display));
    }

    pub fn set_output_volume(&mut self, volume: f32) {
        self.config.output_volume = (volume.clamp(0.0, 3.0) * 20.0).round() / 20.0;
        let _ = self.config.save_to_dir(&self.config_dir);
    }

    pub fn set_mic_gain(&mut self, gain: f32) {
        self.config.mic_gain = (gain.clamp(0.0, 4.0) * 20.0).round() / 20.0;
        let _ = self.config.save_to_dir(&self.config_dir);
    }

    pub fn set_vad_threshold(&mut self, threshold: f32) {
        self.config.vad_threshold = (threshold.clamp(0.05, 0.95) * 20.0).round() / 20.0;
        let _ = self.config.save_to_dir(&self.config_dir);
    }

    pub fn adjust_player_volume(&mut self, uuid: u128, delta: f32) {
        let current = self.config.player_volume(uuid);
        self.config.set_player_volume(uuid, current + delta);
        let _ = self.config.save_to_dir(&self.config_dir);
    }

    pub fn toggle_player_mute(&mut self, uuid: u128) -> bool {
        let muted = self.config.toggle_player_mute(uuid);
        let _ = self.config.save_to_dir(&self.config_dir);
        muted
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
        self.known_players.clear();
        self.show_player_volumes = false;
        self.player_list_page = 0;
        self.is_talking = false;
    }
}
