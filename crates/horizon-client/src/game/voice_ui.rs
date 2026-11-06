//! In-game Voice Chat UI and HUD overlays for Horizon.
//!
//! Provides the Voice Chat modal screen (Keybind 'V' / Android Voice button)
//! and bottom-left HUD indicators.

use super::{Game, Screen};
use horizon_client::hud::HudBatch;
use horizon_client::ui;
use horizon_client::voice::PlayerSvcStatus;
use horizon_client::voice::udp_worker::UdpConnectionState;
use horizon_client::voice::ui::VoiceHudStatus;
use horizon_protocol::packets::configuration::CustomPayload;
use horizon_protocol::packets::play::Serverbound as S;

impl<'a> Game<'a> {
    /// Compiles and sorts players for the Adjust Player Volumes menu based on filter setting.
    pub(crate) fn voice_filtered_players(&self) -> Vec<(u128, String, PlayerSvcStatus)> {
        let mut list: Vec<(u128, String, PlayerSvcStatus)> = self
            .voice
            .known_players
            .iter()
            .map(|(u, n)| (*u, n.clone(), self.voice.player_svc_status(*u, n)))
            .filter(|(_, _, status)| {
                if !self.voice.filter_svc_only {
                    true
                } else {
                    *status != PlayerSvcStatus::NoSvc
                }
            })
            .collect();

        list.sort_by(|a, b| {
            let is_no_svc = |s: PlayerSvcStatus| s == PlayerSvcStatus::NoSvc;
            (is_no_svc(a.2), a.1.to_ascii_lowercase(), a.0).cmp(&(
                is_no_svc(b.2),
                b.1.to_ascii_lowercase(),
                b.0,
            ))
        });
        list
    }

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

        let modal_w = 360.0;
        let modal_h = 240.0;
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

        // Helper closure to render a button
        let draw_btn = |batch: &mut HudBatch,
                        atlas: &horizon_client::hud::HudAtlas,
                        cursor: (f32, f32),
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
                0xFF552222
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

        if self.voice.show_player_volumes {
            // ================= PAGE 2: PLAYER VOLUMES =================
            let title = "Adjust Player Volumes (Speaker)";
            let tw = HudBatch::text_width(&self.gui.atlas, title);
            batch.text(
                &self.gui.atlas,
                title,
                x + (modal_w - tw) / 2.0,
                y + 8.0,
                0xFFFFFFFF,
                true,
            );

            let filtered_players = self.voice_filtered_players();
            let total_players = filtered_players.len();
            const PLAYERS_PER_PAGE: usize = 5;
            let total_pages = if total_players == 0 {
                1
            } else {
                (total_players + PLAYERS_PER_PAGE - 1) / PLAYERS_PER_PAGE
            };
            let current_page = self
                .voice
                .player_list_page
                .min(total_pages.saturating_sub(1));

            let page_info = format!(
                "Page {} of {} | {} player(s)",
                current_page + 1,
                total_pages,
                total_players
            );
            batch.text(
                &self.gui.atlas,
                &page_info,
                x + 15.0,
                y + 24.0,
                0xFFAAAAAA,
                true,
            );

            // Filter button toggle
            let filter_lbl = if self.voice.filter_svc_only {
                "Filter: SVC Users"
            } else {
                "Filter: All Players"
            };
            let filter_btn_w = 110.0;
            let filter_btn_x = x + modal_w - 15.0 - filter_btn_w;
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                filter_btn_x,
                y + 20.0,
                filter_btn_w,
                16.0,
                filter_lbl,
                self.voice.filter_svc_only,
            );

            let row_start_y = y + 40.0;
            if total_players == 0 {
                let empty_msg = if self.voice.filter_svc_only {
                    "No active SVC players detected (Click Filter: All)"
                } else {
                    "No players detected in voice chat yet"
                };
                batch.text(
                    &self.gui.atlas,
                    empty_msg,
                    x + 20.0,
                    y + 80.0,
                    0xFF888888,
                    true,
                );
            } else {
                let start = current_page * PLAYERS_PER_PAGE;
                let slice: Vec<(u128, String, PlayerSvcStatus, f32, bool)> = filtered_players
                    .iter()
                    .skip(start)
                    .take(PLAYERS_PER_PAGE)
                    .map(|(u, n, st)| {
                        (
                            *u,
                            n.clone(),
                            *st,
                            self.voice.config.player_volume(*u),
                            self.voice.config.is_player_muted(*u),
                        )
                    })
                    .collect();

                for (idx, (uuid, name, status, vol, muted)) in slice.into_iter().enumerate() {
                    let ry = row_start_y + (idx as f32) * 31.0;
                    batch.fill(&self.gui.atlas, x + 12.0, ry, 336.0, 27.0, 0x2A2A2A2A);
                    batch.fill(&self.gui.atlas, x + 12.0, ry + 26.0, 336.0, 1.0, 0xFF3A3A3A);

                    // 1. 2D Player Head (16x16) & Gamertag from SkinsRestorer / Tab List hook
                    let head_x = x + 16.0;
                    let head_y = ry + 5.0;
                    let (real_name, (skin_texture, _slim)) =
                        self.voice_player_name_and_skin(uuid, &name);
                    let skin_id = self.face_skin(uuid, &skin_texture);
                    let mut face_draws = Vec::with_capacity(2);
                    horizon_client::tab_list::face(
                        &mut face_draws,
                        &skin_id,
                        head_x as i32,
                        head_y as i32,
                        16,
                        true,
                        false,
                        0xFFFFFFFF,
                    );
                    horizon_client::hud_layout::paint(batch, &self.gui.atlas, &face_draws);

                    // 2. Status badge, Gamertag & volume status
                    let status_badge = match status {
                        PlayerSvcStatus::Talking => "[LIVE] ",
                        PlayerSvcStatus::Connected => "[SVC] ",
                        PlayerSvcStatus::Disabled => "[OFF] ",
                        PlayerSvcStatus::NoSvc => "",
                    };
                    let label = format!(
                        "{}{}: {:.0}%{}",
                        status_badge,
                        real_name,
                        vol * 100.0,
                        if muted { " [MUTED]" } else { "" }
                    );
                    let text_color = if muted {
                        0xFFFF6666
                    } else {
                        match status {
                            PlayerSvcStatus::Talking => 0xFF55FF55,
                            PlayerSvcStatus::Connected => 0xFFFFFFFF,
                            PlayerSvcStatus::Disabled => 0xFFFFAA55,
                            PlayerSvcStatus::NoSvc => 0xFFAAAAAA,
                        }
                    };
                    batch.text(
                        &self.gui.atlas,
                        &label,
                        head_x + 22.0,
                        ry + 9.0,
                        text_color,
                        true,
                    );

                    // 3. Interactive buttons
                    let mute_btn_w = 48.0;
                    let plus_btn_w = 20.0;
                    let minus_btn_w = 20.0;
                    let btn_h = 17.0;
                    let btn_y = ry + 5.0;
                    let mute_btn_x = x + 360.0 - 15.0 - mute_btn_w;
                    let plus_btn_x = mute_btn_x - 5.0 - plus_btn_w;
                    let minus_btn_x = plus_btn_x - 4.0 - minus_btn_w;

                    draw_btn(
                        batch,
                        &self.gui.atlas,
                        self.cursor,
                        minus_btn_x,
                        btn_y,
                        minus_btn_w,
                        btn_h,
                        "-",
                        false,
                    );
                    draw_btn(
                        batch,
                        &self.gui.atlas,
                        self.cursor,
                        plus_btn_x,
                        btn_y,
                        plus_btn_w,
                        btn_h,
                        "+",
                        false,
                    );
                    let mute_lbl = if muted { "Unmute" } else { "Mute" };
                    draw_btn(
                        batch,
                        &self.gui.atlas,
                        self.cursor,
                        mute_btn_x,
                        btn_y,
                        mute_btn_w,
                        btn_h,
                        mute_lbl,
                        muted,
                    );
                }
            }

            // Footer navigation buttons
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                x + 15.0,
                y + 204.0,
                95.0,
                22.0,
                "< Back",
                false,
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                x + 130.0,
                y + 204.0,
                95.0,
                22.0,
                "< Prev",
                false,
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                x + 245.0,
                y + 204.0,
                100.0,
                22.0,
                "Next >",
                false,
            );
        } else {
            // ================= PAGE 1: SVC GENERAL SETTINGS =================
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

            // Protocol version badge at top-right of modal header
            let badge_w = 38.0;
            let badge_h = 16.0;
            let badge_x = x + modal_w - 15.0 - badge_w;
            let badge_y = y + 6.0;
            let cycle_txt = format!("v{}", self.voice.config.compatibility_version);
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                badge_x,
                badge_y,
                badge_w,
                badge_h,
                &cycle_txt,
                false,
            );

            let col1_x = x + 15.0;
            let col1_w = 330.0;

            // Connection status line
            let status_str = match self.voice.connection_state() {
                UdpConnectionState::Disconnected => "Status: Disconnected".to_string(),
                UdpConnectionState::Connecting => "Status: Connecting...".to_string(),
                UdpConnectionState::Authenticating => "Status: Authenticating...".to_string(),
                UdpConnectionState::Connected { ping_ms } => {
                    format!(
                        "Connected ({} ms) · v{}",
                        ping_ms, self.voice.config.compatibility_version
                    )
                }
                UdpConnectionState::Failed(msg) => format!("Error: {msg}"),
            };
            batch.text(
                &self.gui.atlas,
                &status_str,
                col1_x,
                y + 24.0,
                0xFFAAAAAA,
                true,
            );

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
                y + 40.0,
                160.0,
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
                col1_x + 170.0,
                y + 40.0,
                160.0,
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
                y + 62.0,
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
                y + 84.0,
                col1_w,
                18.0,
                &rnn_txt,
                false,
            );

            // Row 4: Speaker Volume [-] [ Speaker Volume: 100% ] [+]
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x,
                y + 106.0,
                26.0,
                18.0,
                "-",
                false,
            );
            let vol_txt = format!(
                "Speaker Volume: {:.0}%",
                self.voice.config.output_volume * 100.0
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 30.0,
                y + 106.0,
                270.0,
                18.0,
                &vol_txt,
                false,
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 304.0,
                y + 106.0,
                26.0,
                18.0,
                "+",
                false,
            );

            // Row 5: Mic Gain [-] [ Mic Gain: 100% ] [+]
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x,
                y + 128.0,
                26.0,
                18.0,
                "-",
                false,
            );
            let gain_txt = format!("Mic Gain: {:.0}%", self.voice.config.mic_gain * 100.0);
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 30.0,
                y + 128.0,
                270.0,
                18.0,
                &gain_txt,
                false,
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 304.0,
                y + 128.0,
                26.0,
                18.0,
                "+",
                false,
            );

            // Row 6: VAD Threshold [-] [ VAD Sensitivity: 50% ] [+]
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x,
                y + 150.0,
                26.0,
                18.0,
                "-",
                false,
            );
            let vad_txt = format!(
                "VAD Sensitivity: {:.0}%",
                self.voice.config.vad_threshold * 100.0
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 30.0,
                y + 150.0,
                270.0,
                18.0,
                &vad_txt,
                false,
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x + 304.0,
                y + 150.0,
                26.0,
                18.0,
                "+",
                false,
            );

            // Row 7: Sub-menu button for Player Volumes
            let adj_txt = format!(
                "Adjust Player Volumes ({}) >",
                self.voice.known_players.len()
            );
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x,
                y + 174.0,
                col1_w,
                20.0,
                &adj_txt,
                false,
            );

            // Row 8: Done Button (expanded to full width 330px)
            draw_btn(
                batch,
                &self.gui.atlas,
                self.cursor,
                col1_x,
                y + 202.0,
                col1_w,
                22.0,
                "Done (Esc / V)",
                false,
            );
        }
    }

    /// Handles mouse click on the Voice Chat modal screen.
    pub(super) fn voice_chat_click(&mut self) {
        let (w, h) = self.gui_size();
        let modal_w = 360.0;
        let modal_h = 240.0;
        let x = ((w - modal_w) / 2.0).floor();
        let y = ((h - modal_h) / 2.0).floor();

        if self.voice.show_player_volumes {
            let filter_btn_w = 110.0;
            let filter_btn_x = x + modal_w - 15.0 - filter_btn_w;
            if ui::hovered(self.cursor, filter_btn_x, y + 20.0, filter_btn_w, 16.0) {
                self.voice.filter_svc_only = !self.voice.filter_svc_only;
                self.voice.player_list_page = 0;
                return;
            }

            // 1. Back button
            if ui::hovered(self.cursor, x + 15.0, y + 204.0, 95.0, 22.0) {
                self.voice.show_player_volumes = false;
                return;
            }
            // 2. Prev button
            if ui::hovered(self.cursor, x + 130.0, y + 204.0, 95.0, 22.0) {
                if self.voice.player_list_page > 0 {
                    self.voice.player_list_page -= 1;
                }
                return;
            }
            // 3. Next button
            let filtered_players = self.voice_filtered_players();
            let total_players = filtered_players.len();
            let total_pages = if total_players == 0 {
                1
            } else {
                (total_players + 4) / 5
            };
            if ui::hovered(self.cursor, x + 245.0, y + 204.0, 100.0, 22.0) {
                if self.voice.player_list_page + 1 < total_pages {
                    self.voice.player_list_page += 1;
                }
                return;
            }
            // 4. Per-player volume & mute buttons
            let current_page = self.voice.player_list_page;
            let row_start_y = y + 40.0;
            for idx in 0..5 {
                let player_idx = current_page * 5 + idx;
                if player_idx >= total_players {
                    break;
                }
                let uuid = filtered_players[player_idx].0;
                let ry = row_start_y + (idx as f32) * 31.0;
                let btn_y = ry + 5.0;
                let mute_btn_w = 48.0;
                let plus_btn_w = 20.0;
                let minus_btn_w = 20.0;
                let mute_btn_x = x + 360.0 - 15.0 - mute_btn_w;
                let plus_btn_x = mute_btn_x - 5.0 - plus_btn_w;
                let minus_btn_x = plus_btn_x - 4.0 - minus_btn_w;

                if ui::hovered(self.cursor, minus_btn_x, btn_y, minus_btn_w, 17.0) {
                    self.voice.adjust_player_volume(uuid, -0.10);
                    return;
                } else if ui::hovered(self.cursor, plus_btn_x, btn_y, plus_btn_w, 17.0) {
                    self.voice.adjust_player_volume(uuid, 0.10);
                    return;
                } else if ui::hovered(self.cursor, mute_btn_x, btn_y, mute_btn_w, 17.0) {
                    self.voice.toggle_player_mute(uuid);
                    return;
                }
            }
        } else {
            let col1_x = x + 15.0;
            let col1_w = 330.0;

            // Row 1: Mic Mute & Deafen
            if ui::hovered(self.cursor, col1_x, y + 40.0, 160.0, 18.0) {
                self.voice_toggle_mute();
            } else if ui::hovered(self.cursor, col1_x + 170.0, y + 40.0, 160.0, 18.0) {
                self.voice_toggle_disable();
            }
            // Row 2: Activation Mode
            else if ui::hovered(self.cursor, col1_x, y + 62.0, col1_w, 18.0) {
                self.voice.config.voice_activation = !self.voice.config.voice_activation;
                let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
            }
            // Row 3: RNNoise Denoiser
            else if ui::hovered(self.cursor, col1_x, y + 84.0, col1_w, 18.0) {
                self.voice.config.rnnoise_enabled = !self.voice.config.rnnoise_enabled;
                let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
            }
            // Row 4: Speaker Volume [-] / [+]
            else if ui::hovered(self.cursor, col1_x, y + 106.0, 26.0, 18.0) {
                self.voice
                    .set_output_volume(self.voice.config.output_volume - 0.10);
            } else if ui::hovered(self.cursor, col1_x + 304.0, y + 106.0, 26.0, 18.0) {
                self.voice
                    .set_output_volume(self.voice.config.output_volume + 0.10);
            }
            // Row 5: Mic Gain [-] / [+]
            else if ui::hovered(self.cursor, col1_x, y + 128.0, 26.0, 18.0) {
                self.voice.set_mic_gain(self.voice.config.mic_gain - 0.10);
            } else if ui::hovered(self.cursor, col1_x + 304.0, y + 128.0, 26.0, 18.0) {
                self.voice.set_mic_gain(self.voice.config.mic_gain + 0.10);
            }
            // Row 6: VAD Threshold [-] / [+]
            else if ui::hovered(self.cursor, col1_x, y + 150.0, 26.0, 18.0) {
                self.voice
                    .set_vad_threshold(self.voice.config.vad_threshold - 0.05);
            } else if ui::hovered(self.cursor, col1_x + 304.0, y + 150.0, 26.0, 18.0) {
                self.voice
                    .set_vad_threshold(self.voice.config.vad_threshold + 0.05);
            }
            // Row 7: Adjust Player Volumes Sub-menu
            else if ui::hovered(self.cursor, col1_x, y + 174.0, col1_w, 20.0) {
                self.voice.show_player_volumes = true;
                self.voice.player_list_page = 0;
            }
            // Header Badge: Cycle Fallback Version (disabled when connected to prevent muscle memory accidents)
            else if ui::hovered(self.cursor, x + modal_w - 15.0 - 38.0, y + 6.0, 38.0, 16.0) {
                let is_connected = matches!(
                    self.voice.connection_state(),
                    UdpConnectionState::Connected { .. }
                );
                if !is_connected {
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
            }
            // Row 8: Done (expanded to full width)
            else if ui::hovered(self.cursor, col1_x, y + 202.0, col1_w, 22.0) {
                self.screen = Screen::Playing;
                self.capture(true);
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
