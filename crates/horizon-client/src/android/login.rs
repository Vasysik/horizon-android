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
        targets.clear();
        let w = (ui.w - 36.0).clamp(280.0, 520.0);
        let h = if editing_name {
            (ui.h - 32.0).clamp(210.0, 250.0)
        } else {
            (ui.h - 24.0).clamp(286.0, 344.0)
        };
        let x = (ui.w - w) * 0.5;
        let y = (ui.h - h) * 0.5;

        // Fullscreen dimmed backdrop scrim + Material You 3 modal surface
        ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, color::hex(0x06050B, 212));
        ui.rounded(x, y, w, h, 22.0, color::hex(0x171522, 255));
        ui.border(x, y, w, h, 22.0, color::hex(0x2E2A42, 255));

        if editing_name {
            ui.text(
                Face::SemiBold,
                20.0,
                x + 22.0,
                y + 20.0,
                color::TEXT,
                "Offline player name",
            );
            ui.text(
                Face::Regular,
                13.5,
                x + 22.0,
                y + 52.0,
                color::MUTED,
                "Used for singleplayer and offline LAN worlds when signed out of Microsoft.",
            );
            button(
                ui,
                targets,
                [x + 22.0, y + 88.0, w - 44.0, 42.0],
                offline_name,
                Action::Field("name"),
                false,
            );
            button(
                ui,
                targets,
                [x + 22.0, y + h - 54.0, w - 44.0, 38.0],
                "Save & Done",
                Action::FinishNameEdit,
                true,
            );
            return;
        }

        // Top Header: Avatar + "Accounts" + Status Pill + Close (X) button
        ui.avatar_head(x + 22.0, y + 18.0, 28.0);
        ui.text(
            Face::SemiBold,
            20.0,
            x + 58.0,
            y + 18.0,
            color::TEXT,
            "Accounts",
        );

        let status_str = self.status(offline_name);
        let status_col = if self.signed_in {
            color::hex(0x9A7BFF, 255)
        } else {
            color::MUTED
        };
        let status_fit = ui.fit_text(Face::Medium, 13.5, &status_str, (w - 130.0).max(80.0));
        ui.text(
            Face::Medium,
            13.5,
            x + 58.0,
            y + 43.0,
            status_col,
            &status_fit,
        );

        // Top-right Close button
        let close_rect = [x + w - 54.0, y + 16.0, 34.0, 34.0];
        ui.rounded(
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
            12.0,
            color::hex(0x232032, 255),
        );
        ui.icon(
            horizon_client::ui_theme::Icon::Close,
            close_rect[0] + 9.0,
            close_rect[1] + 9.0,
            16.0,
            color::TEXT,
        );
        targets.push((close_rect, Action::CloseLogin));

        let mut cy = y + 72.0;

        // Status / Device Code Card (strictly sequential cy so nothing ever overlaps!)
        if let Some(SignInState::Code(code)) = &self.state {
            button(
                ui,
                targets,
                [x + 22.0, cy, w - 44.0, 42.0],
                &format!("Code: {}   •   Tap to copy", code.user_code),
                Action::CopyLoginCode,
                false,
            );
            cy += 48.0;
        } else {
            let box_h = 46.0;
            ui.rounded(
                x + 22.0,
                cy,
                w - 44.0,
                box_h,
                12.0,
                color::hex(0x1E1B2E, 255),
            );
            ui.border(
                x + 22.0,
                cy,
                w - 44.0,
                box_h,
                12.0,
                color::hex(0x2D2942, 255),
            );
            let primary_note = if self.signed_in {
                "Account saved securely. Sessions renew automatically."
            } else if matches!(self.state, Some(SignInState::Starting)) {
                "Requesting a Microsoft device sign-in code..."
            } else {
                "Sign in with Microsoft in your browser, or play offline below."
            };
            let fit_note = ui.fit_text(Face::Regular, 13.0, primary_note, w - 68.0);
            let has_extra = !self.message.trim().is_empty();
            ui.text(
                Face::Regular,
                13.0,
                x + 34.0,
                cy + if has_extra { 6.0 } else { 13.0 },
                color::TEXT,
                &fit_note,
            );
            if has_extra {
                let fit_msg = ui.fit_text(Face::Medium, 12.0, self.message.trim(), w - 68.0);
                ui.text(
                    Face::Medium,
                    12.0,
                    x + 34.0,
                    cy + 24.0,
                    color::hex(0x9A7BFF, 255),
                    &fit_msg,
                );
            }
            cy += box_h + 10.0;
        }

        // Offline player name field (when not actively waiting on browser sign-in)
        if !self.pending() {
            ui.text(
                Face::Medium,
                12.5,
                x + 22.0,
                cy,
                color::MUTED,
                "Offline player name",
            );
            cy += 20.0;
            button(
                ui,
                targets,
                [x + 22.0, cy, w - 44.0, 36.0],
                if offline_name.trim().is_empty() {
                    "Tap to enter an offline player name..."
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
            );
        } else if !self.message.trim().is_empty() {
            let fit_msg = ui.fit_text(Face::Regular, 13.0, self.message.trim(), w - 44.0);
            ui.text(Face::Regular, 13.0, x + 22.0, cy + 6.0, color::TEXT, &fit_msg);
        }

        if self.message.contains("not approved by Mojang") {
            button(
                ui,
                targets,
                [x + 22.0, y + h - 138.0, w - 44.0, 28.0],
                "Request Minecraft API approval",
                Action::ReviewApp,
                false,
            );
        }

        // Bottom Buttons Area
        let (primary_label, primary_action) = if matches!(self.state, Some(SignInState::Code(_))) {
            ("Copy code & open browser", Action::LoginLink)
        } else if self.pending() {
            ("Restart browser sign-in", Action::SignIn)
        } else if self.signed_in {
            ("Switch Microsoft account", Action::SignIn)
        } else {
            ("Sign in with Microsoft", Action::SignIn)
        };

        let btn_primary_y = y + h - 100.0;
        button(
            ui,
            targets,
            [x + 22.0, btn_primary_y, w - 44.0, 38.0],
            primary_label,
            primary_action,
            true,
        );

        let row2_y = y + h - 52.0;
        let col_gap = 10.0;
        let col3_w = (w - 44.0 - col_gap * 2.0) / 3.0;
        button(
            ui,
            targets,
            [x + 22.0, row2_y, col3_w, 36.0],
            "Device code",
            Action::DeviceSignIn,
            false,
        );
        button(
            ui,
            targets,
            [x + 22.0 + col3_w + col_gap, row2_y, col3_w, 36.0],
            if self.pending() {
                "Cancel"
            } else if self.profile.is_some() {
                "Sign out"
            } else {
                "Clear"
            },
            if self.pending() {
                Action::CancelLogin
            } else if self.profile.is_some() {
                Action::SignOut
            } else {
                Action::CloseLogin
            },
            false,
        );
        button(
            ui,
            targets,
            [x + 22.0 + (col3_w + col_gap) * 2.0, row2_y, col3_w, 36.0],
            if self.pending() { "Hide" } else { "Done" },
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
