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
        editing_name: bool,
    ) {
        use horizon_client::ui_theme::Icon;
        targets.clear();
        let (sw, sh) = (ui.w, ui.h);
        ui.rounded(0.0, 0.0, sw, sh, 0.0, color::hex(0x06050A, 196));

        let w = (sw - 28.0).min(488.0);
        let h = (sh - 20.0).min(368.0).max(248.0);
        let x = (sw - w) * 0.5;
        let y = (sh - h) * 0.5;

        ui.rounded(x, y, w, h, 22.0, color::hex(0x161421, 255));
        ui.border(x, y, w, h, 22.0, color::hex(0x2E2A42, 255));

        let inner_x = x + 20.0;
        let inner_w = w - 40.0;

        // Header Row (Avatar + Title + Status Badge + Close Button)
        let header_y = y + 14.0;
        ui.avatar_head(inner_x, header_y + 2.0, 30.0);
        ui.text(
            Face::SemiBold,
            17.5,
            inner_x + 38.0,
            header_y + 4.0,
            color::TEXT,
            "Accounts & Sign-In",
        );

        let close_rect = [x + w - 50.0, header_y, 32.0, 32.0];
        ui.rounded(
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
            10.0,
            color::hex(0x262235, 255),
        );
        ui.icon(
            Icon::Close,
            close_rect[0] + 9.0,
            close_rect[1] + 9.0,
            14.0,
            color::TEXT,
        );
        targets.push((close_rect, Action::CloseLogin));

        let status_label = self.status(offline_name);
        let badge_w = (ui.measure(Face::Medium, 11.0, &status_label) + 18.0).min(156.0);
        let badge_x = (close_rect[0] - badge_w - 8.0).max(inner_x + 190.0);
        if badge_x + badge_w <= close_rect[0] - 4.0 {
            ui.rounded(
                badge_x,
                header_y + 4.0,
                badge_w,
                24.0,
                12.0,
                if self.signed_in {
                    color::hex(0x233B2E, 255)
                } else {
                    color::hex(0x262235, 255)
                },
            );
            ui.border(
                badge_x,
                header_y + 4.0,
                badge_w,
                24.0,
                12.0,
                if self.signed_in {
                    color::hex(0x4E8E6E, 255)
                } else {
                    color::hex(0x38334C, 255)
                },
            );
            ui.centered_fit(
                Face::Medium,
                10.5,
                [badge_x + 4.0, header_y + 4.0, badge_w - 8.0, 24.0],
                if self.signed_in {
                    color::hex(0x7FE3AF, 255)
                } else {
                    color::MUTED
                },
                &status_label,
            );
        }

        let mut cy = header_y + 38.0;
        ui.rounded(inner_x, cy, inner_w, 1.0, 0.0, color::hex(0x29253B, 255));
        cy += 10.0;

        if editing_name {
            ui.text(
                Face::SemiBold,
                15.0,
                inner_x,
                cy + 6.0,
                color::TEXT,
                "Offline player name",
            );
            ui.text(
                Face::Regular,
                12.5,
                inner_x,
                cy + 30.0,
                color::MUTED,
                "Used when signed out of Microsoft.",
            );
            button(
                ui,
                targets,
                [inner_x, cy + 56.0, inner_w, 40.0],
                offline_name,
                Action::Field("name"),
                false,
            );
            button(
                ui,
                targets,
                [inner_x, y + h - 50.0, inner_w, 36.0],
                "Done",
                Action::FinishNameEdit,
                true,
            );
            return;
        }

        // Reserve bottom space for Offline Player Name + Action Buttons so nothing ever overlaps!
        let row2_y = y + h - 44.0;
        let row1_y = row2_y - 38.0;
        let offline_y = row1_y - 54.0;

        // Section 1: Microsoft Account Status / Device Code Box
        let box_h = (offline_y - cy - 6.0).clamp(44.0, 96.0);
        ui.rounded(inner_x, cy, inner_w, box_h, 12.0, color::hex(0x1E1B2B, 255));
        ui.border(inner_x, cy, inner_w, box_h, 12.0, color::hex(0x2D2940, 255));

        if let Some(SignInState::Code(code)) = &self.state {
            button(
                ui,
                targets,
                [inner_x + 8.0, cy + 6.0, inner_w - 16.0, 32.0],
                &format!("Code: {}  •  Tap to Copy", code.user_code),
                Action::CopyLoginCode,
                false,
            );
        } else {
            let subtitle = if self.signed_in {
                "Account saved securely. Sessions renew automatically."
            } else {
                "Your password is entered in Microsoft's browser."
            };
            let fitted_sub = ui.fit_text(Face::Medium, 12.0, subtitle, inner_w - 20.0);
            ui.text(Face::Medium, 12.0, inner_x + 10.0, cy + 6.0, color::TEXT, &fitted_sub);
        }

        let msg_text = match &self.state {
            Some(SignInState::Starting) => "Requesting a sign-in code...",
            _ => &self.message,
        };
        if !msg_text.is_empty() && box_h >= 46.0 {
            let fitted_msg = ui.fit_text(Face::Regular, 11.5, msg_text, inner_w - 20.0);
            ui.text(
                Face::Regular,
                11.5,
                inner_x + 10.0,
                cy + box_h - 22.0,
                color::MUTED,
                &fitted_msg,
            );
        }

        // Section 2: Offline Player Name Input (never overlaps status box!)
        if !self.pending() && offline_y >= cy + box_h + 2.0 {
            ui.text(
                Face::Medium,
                11.5,
                inner_x,
                offline_y,
                color::MUTED,
                "Offline player name",
            );
            button(
                ui,
                targets,
                [inner_x, offline_y + 16.0, inner_w, 32.0],
                if offline_name.is_empty() {
                    "Enter an offline name"
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
            );
        }

        // Section 3: Action Buttons (2 compact rows at bottom)
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
            if self.pending() { "Back (continues)" } else { "Done" },
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
