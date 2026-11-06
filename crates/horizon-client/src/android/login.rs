//! Native account panel; authentication is shared with the desktop launcher.
use super::pages::{Action, button};
use crate::account::{self, Account, Accounts, Secrets, SignIn, SignInState};
use horizon_client::ui_theme::{Face, Icon, Ui, color};

fn login_surface() -> u32 {
    horizon_client::ui_theme::active_theme().surface
}
fn login_surface_high() -> u32 {
    horizon_client::ui_theme::active_theme().surface_bright
}
fn login_border() -> u32 {
    horizon_client::ui_theme::active_theme().border
}
fn login_text() -> u32 {
    horizon_client::ui_theme::active_theme().text_primary
}
fn login_muted() -> u32 {
    horizon_client::ui_theme::active_theme().text_muted
}
fn login_accent() -> u32 {
    horizon_client::ui_theme::active_theme().accent
}
fn login_accent_soft() -> u32 {
    horizon_client::ui_theme::active_theme().accent_soft
}

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
    fn provider_name(&self) -> &'static str {
        self.profile
            .as_deref()
            .and_then(|name| self.saved_accounts.iter().find(|a| a.name == name))
            .map(|a| match a.provider {
                account::Provider::Ely => "Ely.by",
                account::Provider::Microsoft => "Microsoft",
            })
            .unwrap_or("Microsoft")
    }

    pub fn status(&self, offline_name: &str) -> String {
        match (&self.profile, self.signed_in) {
            (Some(name), true) => {
                format!("Already signed in as {name} · {}", self.provider_name())
            }
            (Some(name), false) => format!("Sign-in required for {name}"),
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
    #[allow(clippy::too_many_arguments)]
    fn account_row(
        &self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        rect: [f32; 4],
        name: &str,
        detail: &str,
        current: bool,
        action: Action,
    ) {
        let [x, y, w, h] = rect;
        ui.rounded(x, y, w, h, 14.0, login_surface());
        ui.border(
            x,
            y,
            w,
            h,
            14.0,
            if current {
                login_accent()
            } else {
                login_border()
            },
        );
        let avatar = (h - 12.0).clamp(30.0, 40.0);
        ui.avatar_head_2d(x + 10.0, y + (h - avatar) * 0.5, avatar, name);
        let tx = x + avatar + 22.0;
        let fitted = ui.fit_text(Face::SemiBold, 14.0, name, (w - avatar - 56.0).max(60.0));
        ui.text(Face::SemiBold, 14.0, tx, y + 7.0, login_text(), &fitted);
        let detail = if current {
            format!("{detail} · Current")
        } else {
            detail.to_owned()
        };
        ui.text(
            Face::Regular,
            11.5,
            tx,
            y + h - 21.0,
            if current {
                login_accent()
            } else {
                login_muted()
            },
            &detail,
        );
        if current {
            ui.icon(
                Icon::Check,
                x + w - 30.0,
                y + (h - 18.0) * 0.5,
                18.0,
                login_accent(),
            );
        }
        targets.push(([x, y, w, h], action));
    }

    pub fn draw(
        &self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        offline_name: &str,
        editing: Option<&str>,
        modal_anim: f32,
    ) {
        targets.clear();
        let modal_t = modal_anim.clamp(0.2, 1.0);
        let scale_in = 0.95 + 0.05 * modal_t;
        let w = (ui.w - 24.0).min(620.0) * scale_in;
        let h = (ui.h - 20.0).min(600.0) * scale_in;
        let x = (ui.w - w) * 0.5;
        let mut y = (ui.h - h) * 0.5 + (1.0 - modal_t) * 14.0;
        if editing.is_some()
            && let Some(bottom_px) = crate::native_ime::viewport_bottom()
        {
            let ime_top_dp = bottom_px as f32 / ui.density;
            if ime_top_dp < ui.h - 16.0 {
                y = y.min((ime_top_dp - h - 8.0).max(6.0));
            }
        }
        let theme = horizon_client::ui_theme::active_theme();
        ui.fill(
            0.0,
            0.0,
            ui.w,
            ui.h,
            color::with_alpha(theme.bg_solid, 0.65 * modal_t),
        );
        ui.rounded(
            x + 2.0,
            y + 8.0,
            w,
            h,
            26.0,
            color::hex(0x000000, (90.0 * modal_t) as u8),
        );
        ui.rounded(x, y, w, h, 26.0, theme.bg_solid);
        ui.border(x, y, w, h, 26.0, login_border());

        let (title, subtitle): (&str, String) = if self.providers {
            ("Accounts", "Choose how you want to play".into())
        } else if self.choosing {
            let line = if let Some(name) = &self.profile {
                if self.signed_in {
                    format!("Signed in as {name} with {}", self.provider_name())
                } else {
                    format!("Sign in required for {name}")
                }
            } else {
                "Choose an account or play offline".into()
            };
            ("Your accounts", line)
        } else if self.ely {
            (
                "Ely.by account",
                "Credentials go only to authserver.ely.by".into(),
            )
        } else if self.offline {
            (
                "Offline profile",
                "A local name for single-player and LAN".into(),
            )
        } else {
            let line = if let Some(name) = &self.profile {
                if self.signed_in {
                    format!("Already signed in as {name}")
                } else {
                    format!("Sign in to use {name}")
                }
            } else {
                "Xbox and Minecraft profile".into()
            };
            ("Microsoft account", line)
        };
        if !self.providers {
            let back = [x + 16.0, y + 16.0, 38.0, 38.0];
            ui.rounded(
                back[0],
                back[1],
                back[2],
                back[3],
                12.0,
                login_surface_high(),
            );
            ui.icon(
                Icon::ArrowLeft,
                back[0] + 10.0,
                back[1] + 10.0,
                18.0,
                login_text(),
            );
            targets.push((back, Action::Account));
        }
        let close = [x + w - 54.0, y + 16.0, 38.0, 38.0];
        ui.rounded(
            close[0],
            close[1],
            close[2],
            close[3],
            12.0,
            login_surface_high(),
        );
        ui.icon(
            Icon::Close,
            close[0] + 11.0,
            close[1] + 11.0,
            16.0,
            login_text(),
        );
        targets.push((close, Action::CloseLogin));
        let text_x = if self.providers { x + 22.0 } else { x + 64.0 };
        ui.text(Face::SemiBold, 20.0, text_x, y + 14.0, login_text(), title);
        let fitted = ui.fit_text(
            Face::Regular,
            12.5,
            &subtitle,
            (x + w - 70.0 - text_x).max(70.0),
        );
        ui.text(
            Face::Regular,
            12.5,
            text_x,
            y + 42.0,
            login_muted(),
            &fitted,
        );
        ui.rounded(x + 20.0, y + 72.0, w - 40.0, 1.0, 0.5, login_border());

        let content_y = y + 88.0;
        let content_h = (y + h - 20.0 - content_y).max(60.0);
        let footer_y = y + h - 58.0;
        let gap = 8.0;
        let field_h = ((content_h - 116.0) / 3.0 - gap).clamp(42.0, 56.0);

        if self.providers {
            let gap = 12.0;
            let cw = ((w - 40.0 - gap) * 0.5).max(120.0);
            let ch = ((content_h - 54.0 - gap) * 0.5).clamp(84.0, 96.0);
            for (i, (label, note, action, mark)) in [
                (
                    "Microsoft",
                    "Xbox and Minecraft",
                    Action::MicrosoftAccount,
                    "M",
                ),
                ("Ely.by", "Alternative auth", Action::ElySignIn, "E"),
                (
                    "Offline",
                    "Play without signing in",
                    Action::OfflineAccount,
                    "O",
                ),
                (
                    "Saved accounts",
                    "Switch account",
                    Action::SavedAccounts,
                    "S",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let cx = x + 20.0 + (i % 2) as f32 * (cw + gap);
                let cy = content_y + (i / 2) as f32 * (ch + gap);
                ui.rounded(cx, cy, cw, ch, 16.0, login_surface());
                ui.border(cx, cy, cw, ch, 16.0, login_border());
                ui.rounded(cx + 14.0, cy + 14.0, 34.0, 34.0, 11.0, login_accent_soft());
                ui.text(
                    Face::SemiBold,
                    15.0,
                    cx + 23.0,
                    cy + 21.0,
                    login_accent(),
                    mark,
                );
                ui.text(
                    Face::SemiBold,
                    15.0,
                    cx + 14.0,
                    cy + 50.0,
                    login_text(),
                    label,
                );
                let note = ui.fit_text(Face::Regular, 11.5, note, cw - 28.0);
                ui.text(
                    Face::Regular,
                    11.5,
                    cx + 14.0,
                    cy + 68.0,
                    login_muted(),
                    &note,
                );
                targets.push(([cx, cy, cw, ch], action));
            }
            button(
                ui,
                targets,
                [x + 20.0, footer_y, w - 40.0, 42.0],
                "Close",
                Action::CloseLogin,
                false,
            );
            return;
        }

        if self.choosing {
            let row_h = ((footer_y - content_y - 26.0) / 4.0).clamp(36.0, 56.0);
            let offline_current = self.profile.is_none();
            let offline_label = if offline_name.trim().is_empty() {
                "Offline player"
            } else {
                offline_name
            };
            self.account_row(
                ui,
                targets,
                [x + 20.0, content_y, w - 40.0, row_h],
                offline_label,
                "Offline profile",
                offline_current,
                Action::SelectOffline,
            );
            for (row, (index, account)) in self
                .saved_accounts
                .iter()
                .enumerate()
                .skip(self.account_offset)
                .take(3)
                .enumerate()
            {
                let detail = if account.provider == account::Provider::Ely {
                    "Ely.by"
                } else {
                    "Microsoft"
                };
                let current = self.profile.as_deref() == Some(account.name.as_str());
                let cy = content_y + (row + 1) as f32 * (row_h + 6.0);
                self.account_row(
                    ui,
                    targets,
                    [x + 20.0, cy, w - 40.0, row_h],
                    &account.name,
                    detail,
                    current,
                    Action::SelectAccount(index),
                );
            }
            let nav = 36.0;
            button(
                ui,
                targets,
                [x + 20.0, footer_y, nav, 38.0],
                "<",
                Action::SavedPage(false),
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0 + nav + 6.0, footer_y, nav, 38.0],
                ">",
                Action::SavedPage(true),
                false,
            );
            let after = x + 20.0 + 2.0 * (nav + 6.0) + 8.0;
            let rest = x + w - 20.0 - after;
            let add_w = (rest - 8.0) * 0.5;
            button(
                ui,
                targets,
                [after, footer_y, add_w, 38.0],
                "Add account",
                Action::AddAccount,
                false,
            );
            button(
                ui,
                targets,
                [after + add_w + 8.0, footer_y, rest - add_w - 8.0, 38.0],
                "Done",
                Action::CloseLogin,
                true,
            );
            return;
        }

        if self.ely {
            for (i, (name, label, value, secret, placeholder)) in [
                (
                    "ely_username",
                    "Username or email",
                    self.username.as_str(),
                    false,
                    "Username or email",
                ),
                (
                    "ely_password",
                    "Password",
                    self.password.as_str(),
                    true,
                    "Password",
                ),
                (
                    "ely_otp",
                    "Two-factor code",
                    self.otp.as_str(),
                    true,
                    "Optional",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let fy = content_y + i as f32 * (field_h + gap);
                let display = if secret {
                    "*".repeat(value.chars().count().min(40))
                } else {
                    value.to_owned()
                };
                let focused = editing == Some(name);
                ui.text_field(
                    x + 20.0,
                    fy,
                    w - 40.0,
                    field_h,
                    label,
                    &display,
                    placeholder,
                    focused,
                );
                targets.push(([x + 20.0, fy, w - 40.0, field_h], Action::Field(name)));
            }
            if editing.is_some_and(|field| field.starts_with("ely_")) {
                button(
                    ui,
                    targets,
                    [x + 20.0, footer_y, w - 40.0, 42.0],
                    "Done editing",
                    Action::FinishNameEdit,
                    true,
                );
                return;
            }
            let note_y = content_y + 3.0 * (field_h + gap) + 4.0;
            if !self.message.is_empty() {
                paragraph(
                    ui,
                    x + 20.0,
                    note_y,
                    w - 40.0,
                    &self.message,
                    login_text(),
                    2,
                );
            }
            button(
                ui,
                targets,
                [x + 20.0, footer_y, w - 40.0, 42.0],
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
                [x + 20.0, footer_y - 48.0, w - 40.0, 36.0],
                "Back to accounts",
                Action::Account,
                false,
            );
            return;
        }

        if self.offline {
            let focused = editing == Some("name");
            ui.text_field(
                x + 20.0,
                content_y,
                w - 40.0,
                58.0,
                "Player name",
                offline_name,
                "Enter a name",
                focused,
            );
            targets.push(([x + 20.0, content_y, w - 40.0, 58.0], Action::Field("name")));
            let count = format!("{}/16 characters", offline_name.chars().count());
            ui.text(
                Face::Regular,
                11.5,
                x + 20.0,
                content_y + 66.0,
                login_muted(),
                &count,
            );
            ui.hint_line(
                x + 20.0,
                content_y + 88.0,
                w - 40.0,
                "This name is local; it is not an online identity.",
                login_muted(),
            );
            if focused {
                button(
                    ui,
                    targets,
                    [x + 20.0, footer_y, w - 40.0, 42.0],
                    "Done editing",
                    Action::FinishNameEdit,
                    true,
                );
                return;
            }
            button(
                ui,
                targets,
                [x + 20.0, footer_y, w - 40.0, 42.0],
                "Use offline account",
                Action::SelectOffline,
                true,
            );
            button(
                ui,
                targets,
                [x + 20.0, footer_y - 48.0, w - 40.0, 36.0],
                "Back to accounts",
                Action::Account,
                false,
            );
            return;
        }

        // Microsoft
        let card_h = 84.0;
        let card = [x + 20.0, content_y, w - 40.0, card_h];
        ui.rounded(card[0], card[1], card[2], card[3], 16.0, login_surface());
        ui.border(card[0], card[1], card[2], card[3], 16.0, login_border());
        let display_name = self.profile.as_deref().unwrap_or(offline_name);
        ui.avatar_head_2d(card[0] + 14.0, card[1] + 14.0, 56.0, display_name);
        let name_x = card[0] + 84.0;
        let fitted = ui.fit_text(Face::SemiBold, 17.0, display_name, card[2] - 100.0);
        ui.text(
            Face::SemiBold,
            17.0,
            name_x,
            card[1] + 14.0,
            login_text(),
            &fitted,
        );
        let status = self.status(offline_name);
        let fitted = ui.fit_text(Face::Regular, 12.5, &status, card[2] - 100.0);
        ui.text(
            Face::Regular,
            12.5,
            name_x,
            card[1] + 40.0,
            if self.signed_in {
                login_accent()
            } else {
                login_muted()
            },
            &fitted,
        );
        ui.text(
            Face::Regular,
            11.0,
            name_x,
            card[1] + 60.0,
            login_muted(),
            if self.signed_in {
                "Microsoft · Connected"
            } else {
                "Microsoft · Not connected"
            },
        );

        let mut row_y = content_y + card_h + 12.0;
        if let Some(SignInState::Code(code)) = &self.state {
            let code_h = 84.0;
            ui.rounded(
                x + 20.0,
                row_y,
                w - 40.0,
                code_h,
                14.0,
                login_surface_high(),
            );
            ui.border(x + 20.0, row_y, w - 40.0, code_h, 14.0, login_border());
            ui.text(
                Face::Regular,
                11.0,
                x + 34.0,
                row_y + 12.0,
                login_muted(),
                "SIGN-IN CODE",
            );
            ui.text(
                Face::SemiBold,
                28.0,
                x + 34.0,
                row_y + 30.0,
                login_text(),
                &code.user_code,
            );
            row_y += code_h + 10.0;
            let half = (w - 52.0) * 0.5;
            button(
                ui,
                targets,
                [x + 20.0, row_y, half, 38.0],
                "Copy code",
                Action::CopyLoginCode,
                false,
            );
            button(
                ui,
                targets,
                [x + 32.0 + half, row_y, half, 38.0],
                "Open browser",
                Action::LoginLink,
                false,
            );
            row_y += 46.0;
        } else {
            let text = if self.signed_in {
                "Your account is saved on this device. Sessions renew automatically."
            } else {
                "You sign in inside Microsoft's browser; this app never sees your password."
            };
            row_y = paragraph(ui, x + 20.0, row_y, w - 40.0, text, login_muted(), 2) + 8.0;
        }
        let message = if matches!(&self.state, Some(SignInState::Starting)) {
            "Requesting a sign-in code..."
        } else {
            self.message.as_str()
        };
        if !message.is_empty() {
            row_y = paragraph(ui, x + 20.0, row_y, w - 40.0, message, login_text(), 2) + 4.0;
        }
        if self.message.contains("not approved by Mojang") && row_y + 30.0 < footer_y - 54.0 {
            button(
                ui,
                targets,
                [x + 20.0, row_y, w - 40.0, 30.0],
                "Request Minecraft API approval",
                Action::ReviewApp,
                false,
            );
        }
        let (label, action) = if matches!(&self.state, Some(SignInState::Code(_))) {
            ("Copy code and open browser", Action::LoginLink)
        } else if self.pending() {
            ("Restart browser sign-in", Action::SignIn)
        } else if self.signed_in {
            ("Switch Microsoft account", Action::SignIn)
        } else {
            ("Sign in with browser", Action::SignIn)
        };
        button(
            ui,
            targets,
            [x + 20.0, footer_y, w - 40.0, 42.0],
            label,
            action,
            true,
        );
        let half = (w - 52.0) * 0.5;
        button(
            ui,
            targets,
            [x + 20.0, footer_y - 48.0, half, 38.0],
            "Device code",
            Action::DeviceSignIn,
            false,
        );
        let (second_label, second_action) = if self.pending() {
            ("Cancel", Action::CancelLogin)
        } else if self.profile.is_some() {
            ("Sign out", Action::SignOut)
        } else {
            ("Back", Action::Account)
        };
        button(
            ui,
            targets,
            [x + 32.0 + half, footer_y - 48.0, half, 38.0],
            second_label,
            second_action,
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

fn paragraph(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str, tone: u32, max_lines: usize) -> f32 {
    let limit = (w / 6.8).max(16.0) as usize;
    let mut line = String::new();
    let mut row = 0usize;
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + word.len() + 1 > limit {
            ui.text(Face::Regular, 12.5, x, y + row as f32 * 18.0, tone, &line);
            row += 1;
            line.clear();
            if row >= max_lines {
                break;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if row < max_lines && !line.is_empty() {
        ui.text(Face::Regular, 12.5, x, y + row as f32 * 18.0, tone, &line);
        row += 1;
    }
    y + row as f32 * 18.0
}
