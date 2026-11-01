//! In-game Voice Chat UI and HUD overlays for Horizon.
//!
//! Provides the Voice Chat modal screen (Keybind 'V' / Android Voice button)
//! and bottom-left HUD indicators.

use super::{Game, Screen};
use horizon_client::voice::ui::VoiceHudStatus;
use horizon_client::voice::udp_worker::UdpConnectionState;
use horizon_client::hud::HudBatch;
use horizon_client::ui;
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
                    batch.fill(&self.gui.atlas, x - 2.0, y - 2.0, tw + 4.0, 13.0, 0x88000000);
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

        batch.fill(&self.gui.atlas, x - 2.0, y - 2.0, tw + 4.0, 13.0, 0x88000000);
        batch.text(&self.gui.atlas, label, x, y, color, true);
    }

    /// Renders the Voice Chat modal screen (when Screen::VoiceChat).
    pub(super) fn draw_voice_chat_screen(&mut self, batch: &mut HudBatch) {
        // Semi-transparent overlay over the whole screen
        batch.fill(&self.gui.atlas, 0.0, 0.0, batch.width, batch.height, 0x99000000);

        let modal_w = 280.0;
        let modal_h = 210.0;
        let x = ((batch.width - modal_w) / 2.0).floor();
        let y = ((batch.height - modal_h) / 2.0).floor();

        // Main modal container box
        batch.fill(&self.gui.atlas, x, y, modal_w, modal_h, 0xEE1E1E1E);
        batch.fill(&self.gui.atlas, x, y, modal_w, 2.0, 0xFF555555);
        batch.fill(&self.gui.atlas, x, y + modal_h - 2.0, modal_w, 2.0, 0xFF555555);
        batch.fill(&self.gui.atlas, x, y, 2.0, modal_h, 0xFF555555);
        batch.fill(&self.gui.atlas, x + modal_w - 2.0, y, 2.0, modal_h, 0xFF555555);

        // Title
        let title = "Simple Voice Chat";
        let tw = HudBatch::text_width(&self.gui.atlas, title);
        batch.text(&self.gui.atlas, title, x + (modal_w - tw) / 2.0, y + 10.0, 0xFFFFFFFF, true);

        // Connection status line
        let status_str = match self.voice.connection_state() {
            UdpConnectionState::Disconnected => "Status: Disconnected".to_string(),
            UdpConnectionState::Connecting => "Status: Connecting...".to_string(),
            UdpConnectionState::Authenticating => "Status: Authenticating...".to_string(),
            UdpConnectionState::Connected { ping_ms } => {
                format!("Status: Connected ({ping_ms} ms)")
            }
            UdpConnectionState::Failed(msg) => format!("Status: Error ({msg})"),
        };
        batch.text(&self.gui.atlas, &status_str, x + 15.0, y + 28.0, 0xFFAAAAAA, true);

        let ver_str = format!("Protocol Version: {}", self.voice.config.compatibility_version);
        batch.text(&self.gui.atlas, &ver_str, x + 15.0, y + 42.0, 0xFFAAAAAA, true);

        // Buttons
        let btn_w = 250.0;
        let btn_h = 20.0;
        let btn_x = x + 15.0;

        let buttons: [(f32, String, bool); 6] = [
            (
                y + 60.0,
                format!(
                    "Microphone: {}",
                    if self.voice.config.muted {
                        "MUTED (M)"
                    } else {
                        "ACTIVE"
                    }
                ),
                self.voice.config.muted,
            ),
            (
                y + 85.0,
                format!(
                    "Voice Chat: {}",
                    if self.voice.config.disabled {
                        "DISABLED (N)"
                    } else {
                        "ENABLED"
                    }
                ),
                self.voice.config.disabled,
            ),
            (
                y + 110.0,
                format!(
                    "Activation: {}",
                    if self.voice.config.voice_activation {
                        "Voice Activity"
                    } else {
                        "Push-To-Talk (CapsLock)"
                    }
                ),
                false,
            ),
            (
                y + 135.0,
                format!(
                    "RNNoise AI Filter: {}",
                    if self.voice.config.rnnoise_enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                false,
            ),
            (
                y + 160.0,
                "Cycle Fallback Version (21, 20, 19, 18, 22)".to_string(),
                false,
            ),
            (
                y + 185.0,
                "Done (Esc / V)".to_string(),
                false,
            ),
        ];

        for (by, text, warn) in &buttons {
            let hovered = ui::hovered(self.cursor, btn_x, *by, btn_w, btn_h);
            let bg_color = if hovered {
                0xFF444444
            } else if *warn {
                0xFF442222
            } else {
                0xFF2A2A2A
            };
            batch.fill(&self.gui.atlas, btn_x, *by, btn_w, btn_h, bg_color);
            let tw = HudBatch::text_width(&self.gui.atlas, text);
            let tx = btn_x + (btn_w - tw) / 2.0;
            let ty = by + 5.0;
            batch.text(&self.gui.atlas, text, tx, ty, if hovered { 0xFFFFFF55 } else { 0xFFFFFFFF }, true);
        }
    }

    /// Handles mouse click on the Voice Chat modal screen.
    pub(super) fn voice_chat_click(&mut self) {
        let (w, h) = self.gui_size();
        let modal_w = 280.0;
        let modal_h = 210.0;
        let x = ((w - modal_w) / 2.0).floor();
        let y = ((h - modal_h) / 2.0).floor();

        let btn_w = 250.0;
        let btn_h = 20.0;
        let btn_x = x + 15.0;

        // Button 1: Toggle Mute
        if ui::hovered(self.cursor, btn_x, y + 60.0, btn_w, btn_h) {
            self.voice_toggle_mute();
        }
        // Button 2: Toggle Disable
        else if ui::hovered(self.cursor, btn_x, y + 85.0, btn_w, btn_h) {
            self.voice_toggle_disable();
        }
        // Button 3: Toggle Activation (PTT vs Voice Activity)
        else if ui::hovered(self.cursor, btn_x, y + 110.0, btn_w, btn_h) {
            self.voice.config.voice_activation = !self.voice.config.voice_activation;
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
        }
        // Button 4: Toggle RNNoise
        else if ui::hovered(self.cursor, btn_x, y + 135.0, btn_w, btn_h) {
            self.voice.config.rnnoise_enabled = !self.voice.config.rnnoise_enabled;
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
        }
        // Button 5: Cycle Fallback Version
        else if ui::hovered(self.cursor, btn_x, y + 160.0, btn_w, btn_h) {
            let next = self.voice.config.next_fallback_version();
            let _ = self.voice.config.save_to_dir(&self.voice.config_dir);
            log::info!("SVC: Manually cycled compatibility version to {next}");
            if self.remote.is_some() {
                self.initiate_voice_handshake();
            }
        }
        // Button 6: Done
        else if ui::hovered(self.cursor, btn_x, y + 185.0, btn_w, btn_h) {
            self.screen = Screen::Playing;
            self.capture(true);
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
            self.send(S::CustomPayload(CustomPayload {
                channel: ch,
                data,
            }));
        }
    }

    /// Toggles disable voice chat state.
    pub(super) fn voice_toggle_disable(&mut self) {
        let (ch, data) = self.voice.toggle_disable();
        if self.remote.is_some() {
            self.send(S::CustomPayload(CustomPayload {
                channel: ch,
                data,
            }));
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
