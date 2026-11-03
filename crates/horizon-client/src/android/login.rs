//! Native account panel; authentication is shared with the desktop launcher.
use super::pages::{Action, button};
use crate::account::{self, Account, Accounts, Secrets, SignIn, SignInState};
use horizon_client::ui_theme::{Face, Ui, color};

pub(super) struct Login {
    pub visible: bool,
    pub signed_in: bool,
    pub profile: Option<String>,
    task: Option<SignIn>,
    auto_open_device: bool,
    state: Option<SignInState>,
    message: String,
}
impl Login {
    pub fn new() -> Self {
        let mut login = Self {
            visible: false,
            signed_in: false,
            profile: None,
            task: None,
            auto_open_device: false,
            state: None,
            message: String::new(),
        };
        login.refresh_status();
        login
    }
    pub(super) fn refresh_status(&mut self) {
        let account = Accounts::load().selected().cloned();
        self.profile = account.as_ref().map(|a| a.name.clone());
        self.signed_in = account.is_some_and(|a| {
            account::store()
                .get(&a.uuid)
                .ok()
                .flatten()
                .is_some_and(|s| serde_json::from_str::<Secrets>(&s).is_ok())
        });
    }
    pub fn status(&self, offline_name: &str) -> String {
        match (&self.profile, self.signed_in) {
            (Some(name), true) => format!("Signed in: {name}"),
            (Some(name), false) => format!("Sign in required: {name}"),
            _ if self.pending() => "Signing in...".into(),
            _ if !offline_name.trim().is_empty() => format!("Offline: {offline_name}"),
            _ => "Offline / Sign in".into(),
        }
    }
    pub fn account_label(&self, offline_name: &str) -> String {
        if let Some(name) = &self.profile {
            name.clone()
        } else if self.pending() {
            "Signing in...".into()
        } else if !offline_name.trim().is_empty() {
            offline_name.trim().to_owned()
        } else {
            "vasysik1".into()
        }
    }

    pub fn pending(&self) -> bool {
        self.task.is_some()
    }
    pub fn hide(&mut self) {
        self.visible = false;
    }
    pub fn close(&mut self) {
        self.auto_open_device = false;
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.state = None;
        self.visible = false;
    }
    pub fn start(&mut self, id: &str) {
        self.start_device(id);
        self.auto_open_device = true;
    }
    pub fn start_device(&mut self, id: &str) {
        self.close();
        self.visible = true;
        let id = id.trim();
        if !account::legacy_client(id)
            && (id.len() != 36
                || !id.chars().enumerate().all(|(i, c)| {
                    if [8, 13, 18, 23].contains(&i) {
                        c == '-'
                    } else {
                        c.is_ascii_hexdigit()
                    }
                }))
        {
            self.message =
                "Register our Microsoft public client application first, then enter its client ID."
                    .into();
            return;
        }
        self.message.clear();
        self.task = Some(SignIn::start_with_client_id(Some(id.into())));
        self.state = Some(SignInState::Starting);
    }
    pub fn poll(&mut self) {
        if let Some(task) = &self.task {
            self.state = Some(task.state());
            if let Some(SignInState::Done(result)) = &self.state {
                self.message = match result {
                    Ok(a) => format!("Signed in as {}", a.name),
                    Err(e) => e.clone(),
                };
                self.task = None;
                self.refresh_status();
            }
        }
        if self.auto_open_device && matches!(self.state, Some(SignInState::Code(_))) {
            self.auto_open_device = false;
            self.open_link();
        }
    }
    pub fn sign_out(&mut self) {
        self.close();
        self.visible = true;
        let mut accounts = Accounts::load();
        if let Some(a) = accounts.selected().cloned() {
            self.message = match accounts
                .remove(account::store(), &a.uuid)
                .and_then(|_| accounts.save())
            {
                Ok(()) => "Signed out. Offline play is available.".into(),
                Err(e) => e,
            };
        }
        self.refresh_status();
    }
    pub fn copy_code(&mut self) {
        if let Some(SignInState::Code(code)) = &self.state {
            self.message = if crate::platform::copy_login_code(&code.user_code) {
                "Code copied. Paste it on the Microsoft page.".into()
            } else {
                "Could not copy the code. Try again.".into()
            };
        }
    }
    pub fn open_link(&mut self) {
        if let Some(SignInState::Code(code)) = &self.state {
            let mut url = url::Url::parse(&code.verification_uri).ok();
            if let Some(uri) = &mut url {
                uri.query_pairs_mut().append_pair("otc", &code.user_code);
            }
            let url = url
                .map(String::from)
                .unwrap_or_else(|| code.verification_uri.clone());
            self.copy_code();
            if let Err(e) = crate::platform::open_login_url(&url) {
                self.message = e;
            }
        }
    }
    pub fn review_app(&mut self) {
        if let Err(e) = crate::platform::open_login_url("https://aka.ms/mce-reviewappid") {
            self.message = e;
        }
    }
    pub fn session(id: &str) -> Result<Option<(Account, Secrets)>, String> {
        let mut accounts = Accounts::load();
        let Some(a) = accounts.selected().cloned() else {
            return Ok(None);
        };
        let secrets = account::store()
            .get(&a.uuid)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or("Session ended. Sign in again, or sign out to play offline.")?;
        let (a, secrets) = account::fresh(&account::Endpoints::live(), id.trim(), &a, secrets)?;
        accounts.add(account::store(), a.clone(), &secrets)?;
        accounts.save()?;
        Ok(Some((a, secrets)))
    }
    pub fn draw(
        &self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        offline_name: &str,
        last_name_changed: &str,
        editing_name: bool,
    ) {
        use horizon_client::ui_theme::Icon;
        targets.clear();
        let (sw, sh) = (ui.w, ui.h);
        ui.rounded(0.0, 0.0, sw, sh, 0.0, color::hex(0x05040A, 205));

        let w = (sw - 24.0).min(528.0);
        let h = (sh - 16.0).min(392.0).max(264.0);
        let x = (sw - w) * 0.5;
        let y = (sh - h) * 0.5;

        // Material You 3 Extra-Large Modal Surface + Elevation Shadow
        ui.rounded(x + 2.0, y + 6.0, w, h, 24.0, color::hex(0x000000, 150));
        ui.rounded(x, y, w, h, 24.0, color::hex(0x151320, 255));
        ui.stroke_rounded(x, y, w, h, 24.0, 1.5, color::hex(0x36304E, 255));

        let inner_x = x + 18.0;
        let inner_w = w - 36.0;

        // Top Bar: Title + Status Pill + Close Button
        let header_y = y + 12.0;
        ui.rounded(
            inner_x,
            header_y + 2.0,
            28.0,
            28.0,
            9.0,
            color::hex(0x332854, 255),
        );
        ui.icon(
            Icon::MicrosoftGrid,
            inner_x + 6.0,
            header_y + 8.0,
            16.0,
            color::hex(0x9A7BFF, 255),
        );
        ui.text(
            Face::SemiBold,
            16.5,
            inner_x + 36.0,
            header_y + 3.0,
            color::TEXT,
            "Player Identity & Microsoft Account",
        );

        let close_rect = [x + w - 48.0, header_y, 30.0, 30.0];
        ui.rounded(
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
            15.0,
            color::hex(0x252136, 255),
        );
        ui.border(
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
            15.0,
            color::hex(0x3B3554, 255),
        );
        ui.icon(
            Icon::Close,
            close_rect[0] + 8.0,
            close_rect[1] + 8.0,
            14.0,
            color::TEXT,
        );
        targets.push((close_rect, Action::CloseLogin));

        let active_name = self.account_label(offline_name);
        let mut cy = header_y + 36.0;

        if editing_name {
            // Focused Name Editing View with 3D Head Preview
            let hero_h = 84.0;
            ui.rounded(
                inner_x,
                cy,
                inner_w,
                hero_h,
                16.0,
                color::hex(0x1E1B2E, 255),
            );
            ui.border(
                inner_x,
                cy,
                inner_w,
                hero_h,
                16.0,
                color::hex(0x9A7BFF, 255),
            );
            ui.avatar_head_3d(inner_x + 12.0, cy + 10.0, 64.0, &active_name);
            ui.text(
                Face::SemiBold,
                15.5,
                inner_x + 88.0,
                cy + 12.0,
                color::TEXT,
                "Editing Player Name",
            );
            ui.text(
                Face::Regular,
                12.0,
                inner_x + 88.0,
                cy + 34.0,
                color::MUTED,
                "Updates your 3D Minecraft head preview and offline profile identity.",
            );
            let change_line = format!("Last changed: {last_name_changed}");
            let fitted_chg = ui.fit_text(Face::Medium, 11.5, &change_line, inner_w - 98.0);
            ui.text(
                Face::Medium,
                11.5,
                inner_x + 88.0,
                cy + 54.0,
                color::hex(0x9A7BFF, 255),
                &fitted_chg,
            );

            button(
                ui,
                targets,
                [inner_x, cy + hero_h + 14.0, inner_w, 40.0],
                if offline_name.is_empty() {
                    "Type player name..."
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
            );
            button(
                ui,
                targets,
                [inner_x, y + h - 48.0, inner_w, 36.0],
                "Save Player Name",
                Action::FinishNameEdit,
                true,
            );
            return;
        }

        // Section 1: 3D Minecraft Head Identity Hero Card (3D Head + Username + Status Badge + Last Name Change + Quick Edit)
        let hero_h = 82.0;
        ui.rounded(
            inner_x,
            cy,
            inner_w,
            hero_h,
            16.0,
            color::hex(0x1D1A2B, 255),
        );
        ui.border(
            inner_x,
            cy,
            inner_w,
            hero_h,
            16.0,
            color::hex(0x322D4A, 255),
        );

        // 3D Isometric Minecraft Head on the left
        ui.avatar_head_3d(inner_x + 10.0, cy + 9.0, 64.0, &active_name);

        let info_x = inner_x + 84.0;
        let edit_btn_w = 96.0;
        let info_max_w = (inner_w - 94.0 - edit_btn_w - 8.0).max(90.0);

        let fitted_name = ui.fit_text(Face::SemiBold, 16.5, &active_name, info_max_w);
        ui.text(
            Face::SemiBold,
            16.5,
            info_x,
            cy + 8.0,
            color::TEXT,
            &fitted_name,
        );

        let acct_type = if self.signed_in {
            "Microsoft Xbox Live • Session Active"
        } else if self.profile.is_some() {
            "Microsoft Account • Renewal Needed"
        } else {
            "Offline / Local Profile • Ready to Play"
        };
        let fitted_type = ui.fit_text(Face::Medium, 11.5, acct_type, info_max_w);
        ui.text(
            Face::Medium,
            11.5,
            info_x,
            cy + 31.0,
            if self.signed_in {
                color::hex(0x6EE7B7, 255)
            } else {
                color::hex(0x9A7BFF, 255)
            },
            &fitted_type,
        );

        let history_line = format!("Last name change: {last_name_changed}");
        let fitted_hist = ui.fit_text(Face::Regular, 11.0, &history_line, inner_w - 94.0);
        ui.text(
            Face::Regular,
            11.0,
            info_x,
            cy + 52.0,
            color::MUTED,
            &fitted_hist,
        );

        // Quick "Change Name" button inside Hero Card
        let edit_rect = [
            inner_x + inner_w - edit_btn_w - 10.0,
            cy + 10.0,
            edit_btn_w,
            30.0,
        ];
        button(
            ui,
            targets,
            edit_rect,
            "Change Name",
            Action::Field("name"),
            false,
        );

        cy += hero_h + 8.0;

        // Reserve bottom space for Offline Name Card + 2 Action Button Rows
        let row2_y = y + h - 42.0;
        let row1_y = row2_y - 36.0;
        let name_row_y = row1_y - 42.0;

        // Section 2: Microsoft Session & Device Code Card
        let box_h = (name_row_y - cy - 6.0).clamp(38.0, 78.0);
        ui.rounded(inner_x, cy, inner_w, box_h, 14.0, color::hex(0x1B1828, 255));
        ui.border(inner_x, cy, inner_w, box_h, 14.0, color::hex(0x2D2942, 255));

        if let Some(SignInState::Code(code)) = &self.state {
            button(
                ui,
                targets,
                [inner_x + 8.0, cy + 5.0, inner_w - 16.0, 28.0],
                &format!("Device Code: {}  •  Tap to Copy", code.user_code),
                Action::CopyLoginCode,
                true,
            );
        } else {
            let subtitle = if self.signed_in {
                "Microsoft tokens are stored in encrypted storage and renew automatically."
            } else {
                "Sign in with Microsoft Browser or Device Code for official online servers."
            };
            let fitted_sub = ui.fit_text(Face::Medium, 11.5, subtitle, inner_w - 20.0);
            ui.text(
                Face::Medium,
                11.5,
                inner_x + 10.0,
                cy + 6.0,
                color::TEXT,
                &fitted_sub,
            );
        }

        let msg_text = match &self.state {
            Some(SignInState::Starting) => "Requesting Microsoft sign-in code...",
            _ => &self.message,
        };
        if !msg_text.is_empty() && box_h >= 42.0 {
            let fitted_msg = ui.fit_text(Face::Regular, 11.0, msg_text, inner_w - 20.0);
            ui.text(
                Face::Regular,
                11.0,
                inner_x + 10.0,
                cy + box_h - 20.0,
                color::hex(0xC4B5FD, 255),
                &fitted_msg,
            );
        }

        // Section 3: Direct Offline Name Pill Bar
        if !self.pending() && name_row_y >= cy + box_h + 2.0 {
            let lbl_w = 112.0;
            ui.text(
                Face::SemiBold,
                11.5,
                inner_x + 4.0,
                name_row_y + 7.0,
                color::MUTED,
                "Offline Username:",
            );
            button(
                ui,
                targets,
                [inner_x + lbl_w, name_row_y, inner_w - lbl_w, 32.0],
                if offline_name.is_empty() {
                    "Tap to set offline player name..."
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
            );
        }

        // Section 4: Action Buttons (2 M3 rows at bottom)
        let (primary_label, primary_action) = if matches!(self.state, Some(SignInState::Code(_))) {
            ("Copy code & open browser", Action::LoginLink)
        } else if self.pending() {
            ("Restart browser sign-in", Action::SignIn)
        } else if self.signed_in {
            ("Switch Microsoft account", Action::SignIn)
        } else {
            ("Sign in with browser", Action::SignIn)
        };
        let half = (inner_w - 8.0) * 0.5;
        button(
            ui,
            targets,
            [inner_x, row1_y, half, 32.0],
            primary_label,
            primary_action,
            true,
        );
        button(
            ui,
            targets,
            [inner_x + half + 8.0, row1_y, half, 32.0],
            "Use device code",
            Action::DeviceSignIn,
            false,
        );

        let (sec_label, sec_action) = if self.pending() {
            ("Cancel sign-in", Action::CancelLogin)
        } else if self.profile.is_some() {
            ("Sign out", Action::SignOut)
        } else if self.message.contains("not approved by Mojang") {
            ("Request API approval", Action::ReviewApp)
        } else {
            ("Close", Action::CloseLogin)
        };
        button(
            ui,
            targets,
            [inner_x, row2_y, half, 32.0],
            sec_label,
            sec_action,
            false,
        );
        button(
            ui,
            targets,
            [inner_x + half + 8.0, row2_y, half, 32.0],
            if self.pending() {
                "Back (continues)"
            } else {
                "Done"
            },
            Action::CloseLogin,
            false,
        );
    }
}
impl Drop for Login {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.cancel();
        }
    }
}
