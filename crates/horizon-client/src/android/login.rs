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
        let w = (ui.w - 32.0).min(720.0);
        let h = (ui.h - 24.0).min(550.0);
        let x = (ui.w - w) / 2.0;
        let y = (ui.h - h) / 2.0;
        ui.rounded(x, y, w, h, 8.0, color::BG);
        if editing_name {
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
                "Used when signed out of Microsoft.",
            );
            button(
                ui,
                targets,
                [x + 20.0, y + 100.0, w - 40.0, 44.0],
                offline_name,
                Action::Field("name"),
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 56.0, w - 40.0, 36.0],
                "Done",
                Action::FinishNameEdit,
                true,
            );
            return;
        }

        ui.text(
            Face::Medium,
            24.0,
            x + 20.0,
            y + 18.0,
            color::TEXT,
            "Accounts",
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
        if !self.pending() {
            ui.text(
                Face::Regular,
                14.0,
                x + 20.0,
                y + h - 247.0,
                color::MUTED,
                "Offline player name",
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 226.0, w - 40.0, 38.0],
                if offline_name.is_empty() {
                    "Enter an offline name"
                } else {
                    offline_name
                },
                Action::Field("name"),
                false,
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
