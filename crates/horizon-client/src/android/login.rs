//! Native account panel; authentication is shared with the desktop launcher.
use super::pages::{Action, button};
use crate::account::{self, Account, Accounts, Secrets, SignIn, SignInState};
use horizon_client::ui_theme::{Face, Ui, color};

pub(super) struct Login {
    pub choosing: bool,
    pub providers: bool,
    pub offline: bool,
    pub saved_accounts: Vec<Account>,
    pub account_offset: usize,
    pub ely: bool,
    pub username: String,
    pub password: String,
    pub otp: String,
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
            choosing: false,
            providers: false,
            offline: false,
            saved_accounts: vec![],
            account_offset: 0,
            ely: false,
            username: String::new(),
            password: String::new(),
            otp: String::new(),
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
        let accounts = Accounts::load();
        let account = accounts.selected().cloned();
        self.saved_accounts = accounts.accounts;
        self.profile = account.as_ref().map(|a| a.name.clone());
        self.signed_in = account.is_some_and(|a| {
            account::store()
                .get(&a.key())
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

    pub fn pending(&self) -> bool {
        self.task.is_some()
    }
    pub fn hide(&mut self) {
        self.password.clear();
        self.otp.clear();
        self.visible = false;
    }
    pub fn close(&mut self) {
        self.password.clear();
        self.otp.clear();
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
        self.ely = false;
        self.choosing = false;
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
    /// Reports completion so views can switch their provider-scoped selection,
    /// including when the account panel was closed during browser sign-in.
    pub fn poll(&mut self) -> bool {
        let mut completed = false;
        if let Some(task) = &self.task {
            self.state = Some(task.state());
            if let Some(SignInState::Done(result)) = &self.state {
                self.message = match result {
                    Ok(a) => format!("Signed in as {}", a.name),
                    Err(e) => e.clone(),
                };
                self.task = None;
                self.refresh_status();
                completed = true;
            }
        }
        if self.auto_open_device && matches!(self.state, Some(SignInState::Code(_))) {
            self.auto_open_device = false;
            self.open_link();
        }
        completed
    }
    pub fn start_ely(&mut self) {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.message.clear();
        self.task = Some(SignIn::start_ely(
            self.username.clone(),
            std::mem::take(&mut self.password),
            std::mem::take(&mut self.otp),
        ));
        self.state = Some(SignInState::Starting);
    }
    pub fn field(&self, name: &str) -> Option<&str> {
        match name {
            "ely_username" => Some(&self.username),
            "ely_password" => Some(&self.password),
            "ely_otp" => Some(&self.otp),
            _ => None,
        }
    }
    pub fn edit(&mut self, field: Option<&str>, output: &horizon_client::osk::Output) -> bool {
        let value = match field {
            Some("ely_username") => &mut self.username,
            Some("ely_password") => &mut self.password,
            Some("ely_otp") => &mut self.otp,
            _ => return false,
        };
        match output {
            horizon_client::osk::Output::Text(text) if value.len() + text.len() <= 1024 => {
                value.push_str(text);
            }
            horizon_client::osk::Output::Backspace => {
                value.pop();
            }
            _ => {}
        }
        true
    }
    pub fn sign_out(&mut self) {
        self.close();
        self.visible = true;
        let mut accounts = Accounts::load();
        if let Some(a) = accounts.selected().cloned() {
            self.message = match accounts
                .remove_when_idle(account::store(), &a.key())
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
        account::session::selected_session(id.trim())
    }
    pub fn draw(
        &self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        offline_name: &str,
        editing: Option<&str>,
    ) {
        targets.clear();
        let w = (ui.w - 32.0).min(720.0);
        let h = (ui.h - 24.0).min(550.0);
        let x = (ui.w - w) / 2.0;
        let y = (ui.h - h) / 2.0;
        ui.rounded(x, y, w, h, 8.0, color::BG);
        if self.providers {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 20.0,
                color::TEXT,
                "Accounts",
            );
            for (index, (label, action)) in [
                ("Offline", Action::OfflineAccount),
                ("Microsoft", Action::MicrosoftAccount),
                ("Ely.by", Action::ElySignIn),
                ("Saved accounts", Action::SavedAccounts),
            ]
            .into_iter()
            .enumerate()
            {
                button(
                    ui,
                    targets,
                    [x + 20.0, y + 62.0 + index as f32 * 42.0, w - 40.0, 36.0],
                    label,
                    action,
                    false,
                );
            }
            button(
                ui,
                targets,
                [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                "Close",
                Action::CloseLogin,
                false,
            );
            return;
        }
        if self.choosing {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 20.0,
                color::TEXT,
                "Choose account",
            );
            button(
                ui,
                targets,
                [x + 20.0, y + 62.0, w - 40.0, 36.0],
                &format!("Offline: {offline_name}"),
                Action::SelectOffline,
                false,
            );
            for (index, account) in self
                .saved_accounts
                .iter()
                .enumerate()
                .skip(self.account_offset)
                .take(5)
            {
                let label = format!(
                    "{} · {}",
                    account.name,
                    if account.provider == account::Provider::Ely {
                        "Ely.by"
                    } else {
                        "Microsoft"
                    }
                );
                button(
                    ui,
                    targets,
                    [
                        x + 20.0,
                        y + 110.0 + (index - self.account_offset) as f32 * 42.0,
                        w - 40.0,
                        36.0,
                    ],
                    &label,
                    Action::SelectAccount(index),
                    false,
                );
            }
            button(
                ui,
                targets,
                [x + 20.0, y + h - 96.0, (w - 48.0) * 0.5, 32.0],
                "Previous",
                Action::SavedPage(false),
                false,
            );
            button(
                ui,
                targets,
                [x + w * 0.5 + 4.0, y + h - 96.0, (w - 48.0) * 0.5, 32.0],
                "Next",
                Action::SavedPage(true),
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                "Back",
                Action::Account,
                false,
            );
            return;
        }
        if self.ely {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 18.0,
                color::TEXT,
                "Ely.by account",
            );
            ui.text(
                Face::Regular,
                13.0,
                x + 20.0,
                y + 54.0,
                color::MUTED,
                "Credentials go only to authserver.ely.by.",
            );
            for (index, (label, name, value, secret)) in [
                (
                    "Username or email",
                    "ely_username",
                    self.username.as_str(),
                    false,
                ),
                (
                    "Password (not saved)",
                    "ely_password",
                    self.password.as_str(),
                    true,
                ),
                (
                    "Two-factor code (optional)",
                    "ely_otp",
                    self.otp.as_str(),
                    true,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let active = editing.is_some_and(|field| field.starts_with("ely_"));
                if active && editing != Some(name) {
                    continue;
                }
                let top = y + 86.0 + if active { 0.0 } else { index as f32 * 72.0 };
                ui.text(Face::Regular, 13.0, x + 20.0, top, color::MUTED, label);
                let display = if secret {
                    "*".repeat(value.chars().count().min(40))
                } else {
                    value.into()
                };
                button(
                    ui,
                    targets,
                    [x + 20.0, top + 20.0, w - 40.0, 38.0],
                    &display,
                    Action::Field(name),
                    false,
                );
            }
            if editing.is_some_and(|field| field.starts_with("ely_")) {
                button(
                    ui,
                    targets,
                    [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                    "Done editing",
                    Action::FinishNameEdit,
                    true,
                );
                return;
            }
            let message = ui.fit_text(Face::Regular, 13.0, &self.message, w - 40.0);
            ui.text(
                Face::Regular,
                13.0,
                x + 20.0,
                y + 310.0,
                color::TEXT,
                &message,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 142.0, w - 40.0, 38.0],
                if self.pending() {
                    "Signing in..."
                } else {
                    "Sign in to Ely.by"
                },
                Action::SubmitEly,
                true,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 96.0, w - 40.0, 36.0],
                "Back to accounts",
                Action::Account,
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                "Done",
                Action::CloseLogin,
                false,
            );
            return;
        }
        if self.offline {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 20.0,
                color::TEXT,
                "Offline player name",
            );
            ui.text(
                Face::Regular,
                16.0,
                x + 20.0,
                y + 66.0,
                color::MUTED,
                "Maximum 16 characters.",
            );
            button(
                ui,
                targets,
                [x + 20.0, y + 100.0, w - 40.0, 44.0],
                if offline_name.is_empty() {
                    "Enter an offline name"
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 56.0, w - 40.0, 36.0],
                "Use offline account",
                Action::SelectOffline,
                true,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 100.0, w - 40.0, 36.0],
                "Back to accounts",
                Action::Account,
                false,
            );
            return;
        }

        ui.text(
            Face::Medium,
            24.0,
            x + 20.0,
            y + 18.0,
            color::TEXT,
            "Microsoft account",
        );
        ui.text(
            Face::Medium,
            18.0,
            x + 20.0,
            y + 58.0,
            if self.signed_in {
                color::ACCENT
            } else {
                color::MUTED
            },
            &self.status(offline_name),
        );
        if let Some(SignInState::Code(code)) = &self.state {
            button(
                ui,
                targets,
                [x + 20.0, y + 96.0, w - 40.0, 48.0],
                &format!("{}    /    Copy code", code.user_code),
                Action::CopyLoginCode,
                false,
            );
        } else {
            ui.text(
                Face::Regular,
                16.0,
                x + 20.0,
                y + 104.0,
                color::TEXT,
                if self.signed_in {
                    "Account saved securely. Sessions renew automatically."
                } else {
                    "Your password is entered in Microsoft's browser."
                },
            );
        }
        let text = match &self.state {
            Some(SignInState::Starting) => "Requesting a sign-in code...",
            _ => &self.message,
        };
        let limit = ((w - 40.0) / 7.5).max(20.0) as usize;
        let mut line = String::new();
        let mut row = 0;
        for word in text.split_whitespace() {
            if line.len() + word.len() + 1 > limit {
                ui.text(
                    Face::Regular,
                    14.0,
                    x + 20.0,
                    y + 166.0 + row as f32 * 20.0,
                    color::TEXT,
                    &line,
                );
                row += 1;
                line.clear();
                if row >= 5 {
                    break;
                }
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if row < 5 {
            ui.text(
                Face::Regular,
                14.0,
                x + 20.0,
                y + 166.0 + row as f32 * 20.0,
                color::TEXT,
                &line,
            );
        }
        let half = (w - 52.0) / 2.0;
        if self.message.contains("not approved by Mojang") {
            button(
                ui,
                targets,
                [x + 20.0, y + h - 183.0, w - 40.0, 30.0],
                "Request Minecraft API approval",
                Action::ReviewApp,
                false,
            );
        }
        let (label, action) = if matches!(self.state, Some(SignInState::Code(_))) {
            ("Copy code and open browser", Action::LoginLink)
        } else if self.pending() {
            ("Restart browser sign-in", Action::SignIn)
        } else {
            (
                if self.signed_in {
                    "Switch Microsoft account"
                } else {
                    "Sign in with browser"
                },
                Action::SignIn,
            )
        };
        button(
            ui,
            targets,
            [x + 20.0, y + h - 148.0, w - 40.0, 42.0],
            label,
            action,
            true,
        );
        button(
            ui,
            targets,
            [x + 20.0, y + h - 98.0, half, 38.0],
            "Use a device code instead",
            Action::DeviceSignIn,
            false,
        );
        button(
            ui,
            targets,
            [x + 32.0 + half, y + h - 98.0, half, 38.0],
            if self.pending() {
                "Cancel sign-in"
            } else {
                if self.profile.is_some() {
                    "Sign out"
                } else {
                    "Close"
                }
            },
            if self.pending() {
                Action::CancelLogin
            } else {
                if self.profile.is_some() {
                    Action::SignOut
                } else {
                    Action::CloseLogin
                }
            },
            false,
        );
        button(
            ui,
            targets,
            [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
            if self.pending() {
                "Back (sign-in continues)"
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
