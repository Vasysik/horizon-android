//! In-game Voice Chat UI and HUD overlays for Horizon.
//!
//! Provides the Voice Chat modal screen (Keybind 'V' / Android Voice button)
//! and bottom-left HUD indicators.

use super::{Game, Screen};
use horizon_client::hud::HudBatch;
use horizon_client::ui;
use horizon_client::voice::udp_worker::UdpConnectionState;
use horizon_client::voice::ui::VoiceHudStatus;
use horizon_protocol::packets::configuration::CustomPayload;
use horizon_protocol::packets::play::Serverbound as S;

impl<'a> Game<'a> {
    /// Renders bottom-left voice chat status indicator badge.
    pub(super) fn draw_voice_hud_indicator(&mut self, batch: &mut HudBatch) {
        if self.screen != Screen::Playing {
            return;
        }

        let hud_status = self.voice.hud_status();
        let (label, color) = match hud_status {
            VoiceHudStatus::Disconnected => ("[SVC: Disconnected]", 0xFFAAAAAA),
            VoiceHudStatus::Disabled => ("[SVC: Disabled]", 0xFFFF5555),
            VoiceHudStatus::Muted => ("[SVC: Muted]", 0xFFFFAA00),
            VoiceHudStatus::Speaking => ("[SVC: Speaking]", 0xFF55FF55),
            VoiceHudStatus::Whispering => ("[SVC: Whispering]", 0xFF55FFFF),
            VoiceHudStatus::Idle => {
                if let Some(ping) = self.voice.udp_handle.as_ref().and_then(|h| h.ping_ms()) {
                    let s = format!("[SVC: {ping}ms]");
                    let tw = HudBatch::text_width(&self.gui.atlas, &s);
                    let x = 6.0;
                    let y = batch.height - 16.0;
                    batch.fill(
                        &self.gui.atlas,
                        x - 2.0,
                        y - 2.0,
                        tw + 4.0,
                        13.0,
                        0x88000000,
                    );
                    batch.text(&self.gui.atlas, &s, x, y, 0xFFFFFFFF, true);
                    return;
                } else {
                    ("[SVC: Ready]", 0xFFFFFFFF)
                }
            }
        };

        let tw = HudBatch::text_width(&self.gui.atlas, label);
        let x = 6.0;
        let y = batch.height - 16.0;

        batch.fill(
            &self.gui.atlas,
            x - 2.0,
            y - 2.0,
            tw + 4.0,
            13.0,
            0x88000000,
        );
        batch.text(&self.gui.atlas, label, x, y, color, true);
    }

    /// Renders the Voice Chat modal screen (when Screen::VoiceChat).
    pub(super) fn draw_voice_chat_screen(&mut self, batch: &mut HudBatch) {
        // Semi-transparent overlay over the whole screen
        batch.fill(
            &self.gui.atlas,
            0.0,
            0.0,
            batch.width,
            batch.height,
            0x99000000,
        );

        let modal_w = 460.0;
        let modal_h = 250.0;
        let x = ((batch.width - modal_w) / 2.0).floor();
        let y = ((batch.height - modal_h) / 2.0).floor();

        // Main modal container box
        batch.fill(&self.gui.atlas, x, y, modal_w, modal_h, 0xEE1E1E1E);
        batch.fill(&self.gui.atlas, x, y, modal_w, 2.0, 0xFF555555);
        batch.fill(
            &self.gui.atlas,
            x,
            y + modal_h - 2.0,
            modal_w,
            2.0,
            0xFF555555,
        );
        batch.fill(&self.gui.atlas, x, y, 2.0, modal_h, 0xFF555555);
        batch.fill(
            &self.gui.atlas,
            x + modal_w - 2.0,
            y,
            2.0,
            modal_h,
            0xFF555555,
        );

        // Column divider
        batch.fill(&self.gui.atlas, x + 230.0, y + 26.0, 1.0, 186.0, 0xFF444444);

        // Title
        let title = "Simple Voice Chat";
        let tw = HudBatch::text_width(&self.gui.atlas, title);
        batch.text(
            &self.gui.atlas,
            title,
            x + (modal_w - tw) / 2.0,
            y + 8.0,
            0xFFFFFFFF,
            true,
        );

        let col1_x = x + 12.0;
        let col1_w = 210.0;

        // Connection status line
        let status_str = match self.voice.connection_state() {
            UdpConnectionState::Disconnected => "Status: Disconnected".to_string(),
            UdpConnectionState::Connecting => "Status: Connecting...".to_string(),
            UdpConnectionState::Authenticating => "Status: Authenticating...".to_string(),
            UdpConnectionState::Connected { ping_ms } => {
                format!(
                    "Connected ({}ms) | v{}",
                    ping_ms, self.voice.config.compatibility_version
                )
            }
            UdpConnectionState::Failed(msg) => format!("Error: {msg}"),
        };
        batch.text(
            &self.gui.atlas,
            &status_str,
            col1_x,
            y + 26.0,
            0xFFAAAAAA,
            true,
        );

        // Helper closure to render a button
        let draw_btn = |batch: &mut HudBatch,
                        atlas: &horizon_client::packs::AtlasSources,
                        cursor: [f64; 2],
                        bx: f32,
                        by: f32,
                        bw: f32,
                        bh: f32,
                        text: &str,
                        warn: bool| {
            let hovered = ui::hovered(cursor, bx, by, bw, bh);
            let bg_color = if hovered {
                0xFF444444
            } else if warn {
                0xFF442222
            } else {
                0xFF2A2A2A
            };
            batch.fill(atlas, bx, by, bw, bh, bg_color);
            let tw = HudBatch::text_width(atlas, text);
            let tx = bx + (bw - tw) / 2.0;
            let ty = by + (bh - 10.0) / 2.0;
            batch.text(
                atlas,
                text,
                tx,
                ty,
                if hovered { 0xFFFFFF55 } else { 0xFFFFFFFF },
                true,
            );
        };

        // Left Column:
        // Row 1: Mute Mic & Deafen buttons
        let mute_txt = if self.voice.config.muted {
            "Mic: MUTED"
        } else {
            "Mic: ACTIVE"
        };
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 42.0,
            102.0,
            18.0,
            mute_txt,
            self.voice.config.muted,
        );

        let deafen_txt = if self.voice.config.disabled {
            "Deafen: ON"
        } else {
            "Deafen: OFF"
        };
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 108.0,
            y + 42.0,
            102.0,
            18.0,
            deafen_txt,
            self.voice.config.disabled,
        );

        // Row 2: Activation Mode
        let mode_txt = if self.voice.config.voice_activation {
            "Mode: Voice Activation"
        } else {
            "Mode: Push-To-Talk (Caps)"
        };
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 64.0,
            col1_w,
            18.0,
            mode_txt,
            false,
        );

        // Row 3: RNNoise Denoiser
        let rnn_txt = format!(
            "RNNoise Denoiser: {}",
            if self.voice.config.rnnoise_enabled {
                "ON"
            } else {
                "OFF"
            }
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 86.0,
            col1_w,
            18.0,
            &rnn_txt,
            false,
        );

        // Row 4: Master Volume [-] [100%] [+]
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 108.0,
            22.0,
            18.0,
            "-",
            false,
        );
        let vol_txt = format!(
            "Master Volume: {:.0}%",
            self.voice.config.output_volume * 100.0
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 25.0,
            y + 108.0,
            160.0,
            18.0,
            &vol_txt,
            false,
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 188.0,
            y + 108.0,
            22.0,
            18.0,
            "+",
            false,
        );

        // Row 5: Mic Amplification / Gain [-] [100%] [+]
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 130.0,
            22.0,
            18.0,
            "-",
            false,
        );
        let gain_txt = format!("Mic Gain: {:.0}%", self.voice.config.mic_gain * 100.0);
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 25.0,
            y + 130.0,
            160.0,
            18.0,
            &gain_txt,
            false,
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 188.0,
            y + 130.0,
            22.0,
            18.0,
            "+",
            false,
        );

        // Row 6: VAD Threshold [-] [50%] [+]
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 152.0,
            22.0,
            18.0,
            "-",
            false,
        );
        let vad_txt = format!(
            "VAD Threshold: {:.0}%",
            self.voice.config.vad_threshold * 100.0
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 25.0,
            y + 152.0,
            160.0,
            18.0,
            &vad_txt,
            false,
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x + 188.0,
            y + 152.0,
            22.0,
            18.0,
            "+",
            false,
        );

        // Row 7: Cycle Version
        let cycle_txt = format!(
            "Cycle Version (v{})",
            self.voice.config.compatibility_version
        );
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            col1_x,
            y + 174.0,
            col1_w,
            18.0,
            &cycle_txt,
            false,
        );

        // Right Column: Player Volumes
        let col2_x = x + 238.0;
        batch.text(
            &self.gui.atlas,
            "Player Volumes",
            col2_x,
            y + 26.0,
            0xFFFFFFFF,
            true,
        );

        if self.voice.known_players.is_empty() {
            batch.text(
                &self.gui.atlas,
                "No players detected in voice chat yet",
                col2_x,
                y + 50.0,
                0xFF888888,
                true,
            );
        } else {
            let max_shown = self.voice.known_players.len().min(5);
            for idx in 0..max_shown {
                let (uuid, ref name) = self.voice.known_players[idx];
                let vol = self.voice.config.player_volume(uuid);
                let muted = self.voice.config.is_player_muted(uuid);
                let row_y = y + 42.0 + (idx as f32) * 30.0;

                let row_label = format!(
                    "{}: {:.0}%{}",
                    name,
                    vol * 100.0,
                    if muted { " [MUTED]" } else { "" }
                );
                batch.text(
                    &self.gui.atlas,
                    &row_label,
                    col2_x,
                    row_y,
                    if muted { 0xFFFF5555 } else { 0xFFCCCCCC },
                    true,
                );

                let btn_y = row_y + 12.0;
                draw_btn(
                    batch,
                    &self.gui.atlas,
                    self.cursor,
                    col2_x,
                    btn_y,
                    22.0,
                    15.0,
                    "-",
                    false,
                );
                draw_btn(
                    batch,
                    &self.gui.atlas,
                    self.cursor,
                    col2_x + 25.0,
                    btn_y,
                    22.0,
                    15.0,
                    "+",
                    false,
                );
                let mute_lbl = if muted { "Unmute" } else { "Mute" };
                draw_btn(
                    batch,
                    &self.gui.atlas,
                    self.cursor,
                    col2_x + 50.0,
                    btn_y,
                    52.0,
                    15.0,
                    mute_lbl,
                    muted,
                );
            }
        }

        // Bottom Row: Done Button
        let done_w = 180.0;
        let done_x = x + (modal_w - done_w) / 2.0;
        draw_btn(
            batch,
            &self.gui.atlas,
            self.cursor,
            done_x,
            y + 218.0,
            done_w,
            22.0,
            "Done (Esc / V)",
            false,
        );
    }

    /// Handles mouse click on the Voice Chat modal screen.
    pub(super) fn voice_chat_click(&mut self) {
        let (w, h) = self.gui_size();
        let modal_w = 460.0;
        let modal_h = 250.0;
        let x = ((w - modal_w) / 2.0).floor();
        let y = ((h - modal_h) / 2.0).floor();

        let col1_x = x + 12.0;
        let col1_w = 210.0;
        let col2_x = x + 238.0;

        // Button 1: Toggle Mute
        if ui::hovered(self.cursor, col1_x, y + 42.0, 102.0, 18.0) {
            self.voice_toggle_mute();
        }
        // Button 2: Toggle Deafen / Disable
        else if ui::hovered(self.cursor, col1_x + 108.0, y + 42.0, 102.0, 18.0) {
            self.voice_toggle_disable();
        }
        // Button 3: Toggle Activation (PTT vs Voice Activity)
        else if ui::hovered(self.cursor, col1_x, y + 64.0, col1_w, 18.0) {
            self.voice.config.voice_activation = !self.voice.config.voice_activation;
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
        }
        // Button 4: Toggle RNNoise
        else if ui::hovered(self.cursor, col1_x, y + 86.0, col1_w, 18.0) {
            self.voice.config.rnnoise_enabled = !self.voice.config.rnnoise_enabled;
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
        }
        // Button 5: Master Volume [-]
        else if ui::hovered(self.cursor, col1_x, y + 108.0, 22.0, 18.0) {
            self.voice
                .set_output_volume(self.voice.config.output_volume - 0.10);
        }
        // Button 6: Master Volume [+]
        else if ui::hovered(self.cursor, col1_x + 188.0, y + 108.0, 22.0, 18.0) {
            self.voice
                .set_output_volume(self.voice.config.output_volume + 0.10);
        }
        // Button 7: Mic Gain [-]
        else if ui::hovered(self.cursor, col1_x, y + 130.0, 22.0, 18.0) {
            self.voice.set_mic_gain(self.voice.config.mic_gain - 0.10);
        }
        // Button 8: Mic Gain [+]
        else if ui::hovered(self.cursor, col1_x + 188.0, y + 130.0, 22.0, 18.0) {
            self.voice.set_mic_gain(self.voice.config.mic_gain + 0.10);
        }
        // Button 9: VAD Threshold [-]
        else if ui::hovered(self.cursor, col1_x, y + 152.0, 22.0, 18.0) {
            self.voice
                .set_vad_threshold(self.voice.config.vad_threshold - 0.05);
        }
        // Button 10: VAD Threshold [+]
        else if ui::hovered(self.cursor, col1_x + 188.0, y + 152.0, 22.0, 18.0) {
            self.voice
                .set_vad_threshold(self.voice.config.vad_threshold + 0.05);
        }
        // Button 11: Cycle Fallback Version
        else if ui::hovered(self.cursor, col1_x, y + 174.0, col1_w, 18.0) {
            let next = self.voice.config.next_fallback_version();
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
            log::info!("SVC: Manually cycled compatibility version to {next}");
            self.voice.secret = None;
            if let Some(h) = self.voice.udp_handle.take() {
                h.stop();
            }
            if self.remote.is_some() {
                self.initiate_voice_handshake();
            }
        }
        // Button 12: Done
        else if ui::hovered(
            self.cursor,
            x + (modal_w - 180.0) / 2.0,
            y + 218.0,
            180.0,
            22.0,
        ) {
            self.screen = Screen::Playing;
            self.capture(true);
        }
        // Player Volume & Mute actions in Column 2
        else {
            let max_shown = self.voice.known_players.len().min(5);
            for idx in 0..max_shown {
                let uuid = self.voice.known_players[idx].0;
                let row_y = y + 42.0 + (idx as f32) * 30.0;
                let btn_y = row_y + 12.0;

                if ui::hovered(self.cursor, col2_x, btn_y, 22.0, 15.0) {
                    self.voice.adjust_player_volume(uuid, -0.10);
                    break;
                } else if ui::hovered(self.cursor, col2_x + 25.0, btn_y, 22.0, 15.0) {
                    self.voice.adjust_player_volume(uuid, 0.10);
                    break;
                } else if ui::hovered(self.cursor, col2_x + 50.0, btn_y, 52.0, 15.0) {
                    self.voice.toggle_player_mute(uuid);
                    break;
                }
            }
        }
    }

    /// Toggles mute microphone state with Android permission check.
    pub(super) fn voice_toggle_mute(&mut self) {
        #[cfg(target_os = "android")]
        {
            if self.voice.config.muted && !crate::platform::ensure_microphone_permission() {
                log::warn!("SVC: Microphone permission not granted on unmute");
                return;
            }
        }
        let (ch, data) = self.voice.toggle_mute();
        if self.remote.is_some() {
            self.send(S::CustomPayload(CustomPayload { channel: ch, data }));
        }
    }

    /// Toggles disable voice chat state.
    pub(super) fn voice_toggle_disable(&mut self) {
        let (ch, data) = self.voice.toggle_disable();
        if self.remote.is_some() {
            self.send(S::CustomPayload(CustomPayload { channel: ch, data }));
        }
    }

    /// Updates push-to-talk key status with Android permission check.
    pub(super) fn voice_ptt(&mut self, pressed: bool) {
        if pressed {
            #[cfg(target_os = "android")]
            {
                if !crate::platform::ensure_microphone_permission() {
                    log::warn!("SVC: Microphone permission requested on PTT");
                    return;
                }
            }
        }
        self.voice.set_ptt_pressed(pressed);
    }
}
