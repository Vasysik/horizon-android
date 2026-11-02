use super::*;
use horizon_client::{
    osk::Output,
    ui_theme::{Icon, color},
};
use std::sync::atomic::Ordering;
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

/// Material You 3 dark violet-black palette matching the Horizon Android revamp design.
const MY3_BG: u32 = color::hex(0x0D0C14, 255);
const MY3_SURFACE: u32 = color::hex(0x171522, 255);
const MY3_SURFACE_HIGH: u32 = color::hex(0x232032, 255);
const MY3_BORDER: u32 = color::hex(0x282538, 255);
const MY3_NAV_ACTIVE: u32 = color::hex(0x5B4898, 255);
const MY3_ACCENT: u32 = color::hex(0x9A7BFF, 255);
const MY3_ON_ACCENT: u32 = color::hex(0x17112E, 255);
const MY3_TRACK: u32 = color::hex(0x2B2937, 255);
const MY3_PAUSE_BG: u32 = color::hex(0x252333, 255);
const MY3_CANCEL_BG: u32 = color::hex(0x382E4C, 255);
const MY3_TEXT: u32 = color::hex(0xF4F0FC, 255);
const MY3_MUTED: u32 = color::hex(0x9C96B0, 255);

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Page(Page),
    Play,
    PauseInstall,
    CancelInstall,
    PauseCatalogDownload,
    CancelCatalogDownload,
    Search,
    ShaderSource(crate::shader_catalog::Origin),
    ModSource(crate::shader_catalog::Origin),
    Field(&'static str),
    FinishNameEdit,
    Install(usize),
    Import(horizon_client::content_files::Kind),
    OpenFolder(horizon_client::content_files::Kind),
    ToggleSection(usize),
    ToggleLocal(usize),
    DeleteLocal(usize),
    ConfirmDeleteLocal,
    ConfirmExit,
    Next,
    Previous,
    Profile,
    Logs,
    RecordPerformance,
    ExportPerformance,
    ExportDiagnostics,
    Account,
    SignIn,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    Repository,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    ChooseProfile(u32),
    Dismiss,
    Immersive,
    Cutout,
    Aspect,
    ChooseAspect(crate::display::Aspect),
    DisplayHz,
    ChooseDisplayHz(u16),
    CacheBudget,
    ChooseCacheBudget(u32),
    ToggleBundled(&'static str),
    WorldFiles,
    GameFiles,
    WorkerLimit,
    ChooseWorkerLimit(u32),
    PerformanceHints,
    GpuProfile,
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    let [x, y, w, h] = rect;
    let radius = (h * 0.42).clamp(10.0, 22.0);
    ui.rounded(
        x,
        y,
        w,
        h,
        radius,
        if accent {
            MY3_ACCENT
        } else {
            MY3_SURFACE_HIGH
        },
    );
    if !accent {
        ui.border(x, y, w, h, radius, MY3_BORDER);
    }
    let padding = (w * 0.10).clamp(8.0, 14.0);
    let font = if w < 90.0 {
        13.0
    } else if h <= 36.0 {
        14.0
    } else {
        15.0
    };
    let fitted = ui.fit_text(Face::Medium, font, label, (w - 2.0 * padding).max(16.0));
    let text_w = ui.measure(Face::Medium, font, &fitted);
    let left_align = matches!(action, Action::Field(_)) || label.contains("v");
    let tx = if left_align {
        x + padding
    } else {
        x + ((w - text_w) * 0.5).max(padding)
    };
    let ty = y + ((h - font * 1.45) * 0.5).max(2.0);
    ui.text(
        Face::Medium,
        font,
        tx,
        ty,
        if accent { MY3_ON_ACCENT } else { MY3_TEXT },
        &fitted,
    );
    targets.push((rect, action));
}

fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
) {
    let [x, y, w, h] = rect;
    ui.text(
        Face::Regular,
        14.0,
        x,
        y + (h - 20.0) * 0.5,
        MY3_TEXT,
        if on { "On" } else { "Off" },
    );
    let track_x = x + w - 52.0;
    let track_y = y + (h - 26.0) * 0.5;
    ui.rounded(
        track_x,
        track_y,
        50.0,
        26.0,
        13.0,
        if on { MY3_ACCENT } else { MY3_TRACK },
    );
    ui.rounded(
        track_x + if on { 26.0 } else { 4.0 },
        track_y + 3.0,
        20.0,
        20.0,
        10.0,
        if on { MY3_ON_ACCENT } else { MY3_MUTED },
    );
    targets.push(([x, y, w, h], action));
}

fn page_icon(page: Page) -> Icon {
    match page {
        Page::Play => Icon::Play,
        Page::Mods => Icon::Mods,
        Page::Packs => Icon::Packs,
        Page::Shaders => Icon::Shaders,
        Page::Installed => Icon::Installed,
        Page::Settings => Icon::Settings,
    }
}

fn icon_action_button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    bg: u32,
    icon: Icon,
    action: Action,
) {
    let [x, y, w, h] = rect;
    ui.rounded(x, y, w, h, 15.0, bg);
    let icon_size = 18.0;
    ui.icon(
        icon,
        x + (w - icon_size) * 0.5,
        y + (h - icon_size) * 0.5,
        icon_size,
        MY3_TEXT,
    );
    targets.push((rect, action));
}

#[allow(clippy::too_many_arguments)]
fn draw_my3_progress_view(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    content_x: f32,
    content_w: f32,
    h: f32,
    title: &str,
    subtitle_left: &str,
    subtitle_right: &str,
    fraction: f32,
    stage_text: &str,
    paused: bool,
    pause_action: Action,
    cancel_action: Action,
) {
    let center_y = (h * 0.45).clamp(180.0, h - 120.0);
    let title_fitted = ui.fit_text(Face::SemiBold, 24.0, title, content_w);
    ui.text(
        Face::SemiBold,
        24.0,
        content_x,
        center_y - 58.0,
        MY3_TEXT,
        &title_fitted,
    );

    let right_w = ui.measure(Face::Regular, 14.0, subtitle_right);
    let left_max = (content_w - right_w - 16.0).max(80.0);
    let left_fitted = ui.fit_text(Face::Regular, 14.0, subtitle_left, left_max);
    ui.text(
        Face::Regular,
        14.0,
        content_x,
        center_y - 22.0,
        MY3_MUTED,
        &left_fitted,
    );
    ui.text(
        Face::Regular,
        14.0,
        content_x + content_w - right_w,
        center_y - 22.0,
        MY3_MUTED,
        subtitle_right,
    );

    // Thick Material You 3 rounded progress bar (#9A7BFF on #2B2937).
    let bar_y = center_y + 12.0;
    let bar_h = 12.0;
    ui.rounded(content_x, bar_y, content_w, bar_h, 6.0, MY3_TRACK);
    let clamped = fraction.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_w = (content_w * clamped).max(bar_h);
        ui.rounded(content_x, bar_y, fill_w.min(content_w), bar_h, 6.0, MY3_ACCENT);
    }

    let stage_line = if paused {
        format!("Paused • {stage_text}")
    } else {
        stage_text.to_owned()
    };
    let stage_fitted = ui.fit_text(Face::Regular, 13.5, &stage_line, content_w);
    ui.text(
        Face::Regular,
        13.5,
        content_x,
        bar_y + 24.0,
        MY3_MUTED,
        &stage_fitted,
    );

    // Bottom-right MY3 Pause (||) and Cancel (X) rounded square buttons.
    let btn_y = (h - 68.0).max(bar_y + 52.0);
    let cancel_x = content_x + content_w - 46.0;
    let pause_x = cancel_x - 56.0;
    icon_action_button(
        ui,
        targets,
        [pause_x, btn_y, 46.0, 46.0],
        MY3_PAUSE_BG,
        if paused { Icon::Play } else { Icon::Pause },
        pause_action,
    );
    icon_action_button(
        ui,
        targets,
        [cancel_x, btn_y, 46.0, 46.0],
        MY3_CANCEL_BG,
        Icon::Close,
        cancel_action,
    );
}

impl Launcher {
    /// The same spatial focus algorithm as the game's controller menus.
    pub(super) fn navigate(&mut self, key: &KeyEvent) -> bool {
        self.navigate_key(&key.logical_key)
    }
    pub(super) fn navigate_key(&mut self, key: &winit::keyboard::Key) -> bool {
        use horizon_client::gamepad::{self, Button};
        use winit::keyboard::{Key, NamedKey};
        if self.pending_focus.is_some() {
            return true;
        }
        if self.catalog.editing.is_some() || self.targets.is_empty() {
            return false;
        }
        let button = match key {
            Key::Named(NamedKey::ArrowUp) => Button::DPadUp,
            Key::Named(NamedKey::ArrowDown) => Button::DPadDown,
            Key::Named(NamedKey::ArrowLeft) => Button::DPadLeft,
            Key::Named(NamedKey::ArrowRight) => Button::DPadRight,
            Key::Named(NamedKey::Tab) => {
                self.focus = Some(self.focus.map_or(0, |i| (i + 1) % self.targets.len()));
                return true;
            }
            _ => return false,
        };
        if matches!(button, Button::DPadUp | Button::DPadDown)
            && let Some((_, action)) = self.focus.and_then(|i| self.targets.get(i))
            && let Some(index) = self.list_targets.iter().position(|(_, a)| a == action)
        {
            let rects: Vec<_> = self.list_targets.iter().map(|(r, _)| *r).collect();
            if let Some(next) = gamepad::focus_step_strict(rects[index], button, &rects)
                && let Some(view) = self.list_view
            {
                let (rect, action) = self.list_targets[next];
                let scroll = if let Some(chooser) = &mut self.chooser {
                    &mut chooser.scroll
                } else {
                    &mut self.catalog.scroll
                };
                *scroll = super::gesture::reveal(*scroll, rect, view);
                self.pending_focus = Some(action);
                self.focus = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return true;
            }
        }
        self.focus = Some(
            if let Some(index) = self.focus.filter(|i| *i < self.targets.len()) {
                let rects: Vec<_> = self.targets.iter().map(|(rect, _)| *rect).collect();
                gamepad::focus_step_rect(rects[index], button, &rects).unwrap_or(index)
            } else {
                0
            },
        );
        true
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        let (Some(r), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        if r.suspended {
            return Ok(());
        }
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        // OEM navigation bars can remain visible even in fullscreen (keypad phones).
        // Inset the launcher layout only; keep surface and input coordinates intact.
        if let Some(window) = &self.window {
            use winit::platform::android::WindowExtAndroid;
            let rect = window.content_rect();
            let input = winit::platform::android::native_input_scale();
            if rect.bottom > 0 {
                ui.h = ui.h.min(rect.bottom as f32 * input[1] as f32 / ui.density);
            }
            if rect.right > 0 {
                ui.w = ui.w.min(rect.right as f32 * input[0] as f32 / ui.density);
            }
        }
        if let Some(bottom) = crate::native_ime::viewport_bottom() {
            ui.h = ui.h.min(bottom as f32 / ui.density);
        }
        let (w, h) = (ui.w, ui.h);
        let scale = ui.density / ui.batch.scale;
        ui.batch
            .fill(&gui.atlas, 0.0, 0.0, w * scale, h * scale, MY3_BG);

        // ---- Top Header Bar (H Logo | Horizon Android <version> ... Account Pill) ----
        ui.horizon_logo(22.0, 19.0, 26.0);
        ui.rounded(59.0, 22.0, 1.5, 21.0, 0.75, MY3_BORDER);
        let header_title = format!("Horizon Android {}", crate::backend::VERSION);
        let account_name = self.login.account_label(&self.catalog.config.name);
        let account_text_w = ui.measure(Face::Medium, 14.0, &account_name).min(140.0);
        let pill_w = (account_text_w + 68.0).clamp(118.0, 210.0);
        let pill_x = (w - 22.0 - pill_w).max(200.0);
        let pill_y = 14.0;
        let pill_h = 38.0;

        let max_title_w = (pill_x - 84.0).max(80.0);
        let fitted_header = ui.fit_text(Face::SemiBold, 16.0, &header_title, max_title_w);
        ui.text(Face::SemiBold, 16.0, 72.0, 21.0, MY3_TEXT, &fitted_header);

        // Top-right Account Selector Pill with circular avatar head, username, and chevron.
        ui.rounded(
            pill_x,
            pill_y,
            pill_w,
            pill_h,
            pill_h * 0.5,
            color::hex(0x1C1929, 255),
        );
        ui.border(
            pill_x,
            pill_y,
            pill_w,
            pill_h,
            pill_h * 0.5,
            color::hex(0x312C45, 255),
        );
        ui.avatar_head(pill_x + 7.0, pill_y + 7.0, 24.0);
        let fitted_acct = ui.fit_text(Face::Medium, 14.0, &account_name, pill_w - 64.0);
        ui.text(
            Face::Medium,
            14.0,
            pill_x + 38.0,
            pill_y + 9.0,
            MY3_TEXT,
            &fitted_acct,
        );
        ui.icon(
            Icon::ChevronDown,
            pill_x + pill_w - 24.0,
            pill_y + 13.0,
            12.0,
            MY3_MUTED,
        );
        self.targets
            .push(([pill_x, pill_y, pill_w, pill_h], Action::Account));

        // ---- Left Vertical Navigation Rail + Divider Line ----
        let sidebar_w = if w < 560.0 { 132.0 } else { 196.0 };
        ui.rounded(
            sidebar_w,
            66.0,
            1.0,
            (h - 82.0).max(10.0),
            0.0,
            MY3_BORDER,
        );
        let nav_start_y = 74.0;
        let nav_step = ((h - nav_start_y - 20.0) / 6.0).clamp(42.0, 50.0);
        let nav_h = (nav_step - 6.0).clamp(36.0, 42.0);
        for (i, page) in Page::ALL.iter().enumerate() {
            let nx = 12.0;
            let ny = nav_start_y + i as f32 * nav_step;
            let nw = sidebar_w - 24.0;
            let selected = *page == self.catalog.page;
            if selected {
                ui.rounded(nx, ny, nw, nav_h, nav_h * 0.5, MY3_NAV_ACTIVE);
            }
            let fg = if selected {
                MY3_TEXT
            } else {
                color::hex(0xCFC9E2, 255)
            };
            ui.icon(page_icon(*page), nx + 16.0, ny + (nav_h - 18.0) * 0.5, 18.0, fg);
            let label = ui.fit_text(Face::Medium, 14.5, page.name(), nw - 48.0);
            ui.text(
                Face::Medium,
                14.5,
                nx + 44.0,
                ny + (nav_h - 21.0) * 0.5,
                fg,
                &label,
            );
            self.targets.push(([nx, ny, nw, nav_h], Action::Page(*page)));
        }

        // ---- Main Content Area ----
        let content_x = sidebar_w + 28.0;
        let content_w = (w - content_x - 28.0).max(160.0);
        let compact = content_w < 460.0;
        let page = self.catalog.page;

        // When any catalog item (Mod, Pack, Shader) is downloading/installing, take the entire
        // middle screen to show the MY3 downloading view matching the Play preparation screen.
        if let Some((source_id, _)) = self.catalog.downloading.as_ref().filter(|_| self.catalog.busy()) {
            let frac = self.catalog.progress_fraction().clamp(0.0, 1.0);
            let total_mib = self.catalog.downloading_size_mib.max(1.0);
            let done_mib = (total_mib * frac).clamp(0.0, total_mib);
            let title = if self.catalog.downloading_title.is_empty() {
                "Downloading add-on...".to_owned()
            } else {
                format!("Installing {}", self.catalog.downloading_title)
            };
            let sub_left = format!(
                "Minecraft 26.3 • {} • Profile {}",
                source_id.label(),
                self.catalog.config.profile + 1
            );
            let sub_right = format!("{done_mib:.1} / {total_mib:.1} MiB");
            let stage = self.catalog.progress_stage();
            draw_my3_progress_view(
                &mut ui,
                &mut self.targets,
                content_x,
                content_w,
                h,
                &title,
                &sub_left,
                &sub_right,
                frac.max(0.04),
                &stage,
                self.catalog.is_paused(),
                Action::PauseCatalogDownload,
                Action::CancelCatalogDownload,
            );
        } else if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self
                .install
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if installing {
                let current = InstallStage::ALL
                    .iter()
                    .position(|s| *s == progress.stage)
                    .unwrap_or(0);
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                let overall = ((current as f32 + fraction.clamp(0.0, 1.0)) / 6.0).clamp(0.02, 1.0);
                let (done_mib, total_mib) = if progress.stage == InstallStage::Download
                    && progress.total > 0
                {
                    (
                        progress.done as f64 / 1_048_576.0,
                        progress.total as f64 / 1_048_576.0,
                    )
                } else {
                    (55.3 * f64::from(overall), 55.3)
                };
                let sub_left = format!(
                    "Minecraft 26.3 • Profile {}",
                    self.catalog.config.profile + 1
                );
                let sub_right = format!("{done_mib:.1} / {total_mib:.1} MiB");
                let stage_text = match progress.stage {
                    InstallStage::Download => "Downloading Java Edition 26.3 client.jar",
                    other => other.label(),
                };
                let paused = self.install_paused.load(Ordering::Relaxed);
                draw_my3_progress_view(
                    &mut ui,
                    &mut self.targets,
                    content_x,
                    content_w,
                    h,
                    "Getting your game ready",
                    &sub_left,
                    &sub_right,
                    overall,
                    stage_text,
                    paused,
                    Action::PauseInstall,
                    Action::CancelInstall,
                );
            } else {
                let card_y = 84.0;
                let card_h = (h - 210.0).clamp(170.0, 260.0);
                ui.rounded(content_x, card_y, content_w, card_h, 20.0, MY3_SURFACE);
                ui.border(content_x, card_y, content_w, card_h, 20.0, MY3_BORDER);

                ui.text(
                    Face::SemiBold,
                    26.0,
                    content_x + 24.0,
                    card_y + 22.0,
                    MY3_TEXT,
                    if installed {
                        "Ready to play"
                    } else if compact {
                        "Your Java worlds"
                    } else {
                        "Your Java worlds, on Android"
                    },
                );
                ui.text(
                    Face::Regular,
                    15.0,
                    content_x + 24.0,
                    card_y + 62.0,
                    MY3_MUTED,
                    &format!(
                        "Minecraft 26.3 • Profile {} • {}",
                        self.catalog.config.profile + 1,
                        self.catalog.config.renderer.label()
                    ),
                );

                let rows = if installed {
                    [
                        "• Native Rust client with hardware Vulkan & OpenGL ES rendering",
                        "• Install Horizon .hmod, .hnmod and Modrinth mods in the Mods tab",
                        "• Touch camera, mouse sensitivity and virtual pointer ready",
                    ]
                } else {
                    [
                        "• Downloads official Minecraft 26.3 assets directly from Mojang",
                        "• Verifies and prepares resources locally on your device",
                        "• Play offline or sign in with your Microsoft account",
                    ]
                };
                for (index, line) in rows.iter().enumerate() {
                    let fitted =
                        ui.fit_text(Face::Regular, 14.0, line, (content_w - 48.0).max(80.0));
                    ui.text(
                        Face::Regular,
                        14.0,
                        content_x + 24.0,
                        card_y + 102.0 + index as f32 * 28.0,
                        MY3_MUTED,
                        &fitted,
                    );
                }

                let btn_y = (h - 102.0).max(card_y + card_h + 16.0);
                let play_w = content_w.min(280.0);
                button(
                    &mut ui,
                    &mut self.targets,
                    [content_x, btn_y, play_w, 52.0],
                    if installed {
                        "Play Minecraft 26.3"
                    } else if self.error.is_some() {
                        "Retry installation"
                    } else {
                        "Download Minecraft 26.3"
                    },
                    Action::Play,
                    true,
                );
                if content_w > 420.0 {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x + play_w + 14.0, btn_y, 140.0, 52.0],
                        &format!("Profile {} v", self.catalog.config.profile + 1),
                        Action::Profile,
                        false,
                    );
                }
            }
        } else if page.is_catalog() {
            let header_y = 76.0;
            ui.text(
                Face::SemiBold,
                20.0,
                content_x,
                header_y,
                MY3_TEXT,
                if page == Page::Mods {
                    "Mods"
                } else if page == Page::Shaders {
                    "Shader Packs"
                } else {
                    "Resource Packs"
                },
            );
            if matches!(page, Page::Shaders | Page::Mods) {
                use crate::shader_catalog::Origin;
                let seg_w = 104.0;
                for (i, origin) in [Origin::Horizon, Origin::Modrinth].into_iter().enumerate() {
                    let rx = content_x + content_w - (2 - i) as f32 * (seg_w + 6.0) + 6.0;
                    let rect = [rx, header_y - 2.0, seg_w, 32.0];
                    let active = (if page == Page::Mods {
                        self.catalog.config.mod_source
                    } else {
                        self.catalog.config.shader_source
                    }) == origin;
                    button(
                        &mut ui,
                        &mut self.targets,
                        rect,
                        origin.source().label(),
                        if page == Page::Mods {
                            Action::ModSource(origin)
                        } else {
                            Action::ShaderSource(origin)
                        },
                        active,
                    );
                }
            }

            // Search bar + Search button + Open folder / Import file pills
            let search_y = 116.0;
            let search_btn_w = 108.0;
            let field_w = (content_w - search_btn_w - 10.0).max(120.0);
            ui.rounded(content_x, search_y, field_w, 40.0, 20.0, MY3_SURFACE);
            ui.border(content_x, search_y, field_w, 40.0, 20.0, MY3_BORDER);
            ui.icon(
                Icon::Search,
                content_x + 14.0,
                search_y + 11.0,
                18.0,
                MY3_MUTED,
            );
            let prompt = if self.catalog.query.is_empty() {
                if page == Page::Mods {
                    "Search mods or paste .hmod HTTPS link..."
                } else {
                    "Search catalog..."
                }
            } else {
                &self.catalog.query
            };
            let fitted_q = ui.fit_text(Face::Regular, 14.0, prompt, field_w - 46.0);
            ui.text(
                Face::Regular,
                14.0,
                content_x + 38.0,
                search_y + 10.0,
                if self.catalog.query.is_empty() {
                    MY3_MUTED
                } else {
                    MY3_TEXT
                },
                &fitted_q,
            );
            self.targets
                .push(([content_x, search_y, field_w, 40.0], Action::Field("search")));

            button(
                &mut ui,
                &mut self.targets,
                [
                    content_x + field_w + 10.0,
                    search_y,
                    search_btn_w,
                    40.0,
                ],
                if page == Page::Mods && self.catalog.query.trim().starts_with("https://") {
                    "Import .hmod"
                } else {
                    "Search"
                },
                Action::Search,
                true,
            );

            let kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            let sub_y = 164.0;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, sub_y, 124.0, 32.0],
                "Open folder",
                Action::OpenFolder(kind),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 132.0, sub_y, 124.0, 32.0],
                "Import file",
                Action::Import(kind),
                false,
            );
            if !compact {
                let status_line = self.error.as_deref().unwrap_or(&self.catalog.status);
                let fitted_status = ui.fit_text(
                    Face::Regular,
                    13.0,
                    status_line,
                    (content_w - 270.0).max(60.0),
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    content_x + 268.0,
                    sub_y + 6.0,
                    MY3_MUTED,
                    &fitted_status,
                );
            }

            let bundled: Vec<_> = if page == Page::Mods
                && self.catalog.config.mod_source == crate::shader_catalog::Origin::Horizon
            {
                let query = self.catalog.query.to_lowercase();
                horizon_client::mod_list::ANDROID_BUNDLED
                    .into_iter()
                    .filter(|(id, title, _)| {
                        id.contains(&query) || title.to_lowercase().contains(&query)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let catalog_rows: Vec<_> = self
                .catalog
                .results
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !(page == Page::Mods
                        && p.source == crate::browser::SourceId::Horizon
                        && horizon_client::mod_list::ANDROID_BUNDLED
                            .iter()
                            .any(|(id, _, _)| *id == p.id))
                })
                .collect();
            let row_h = 74.0;
            let row_count = bundled.len() + catalog_rows.len();
            let top = 204.0;
            let bottom = h - 62.0;
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (row_count as f32 * row_h - (bottom - top)).max(0.0));
            self.list_view = Some([
                top,
                bottom,
                (row_count as f32 * row_h - (bottom - top)).max(0.0),
            ]);

            let fallback_icon = page_icon(page);

            for (row, (id, title, description)) in bundled.iter().enumerate() {
                let y = top + row as f32 * row_h - self.catalog.scroll;
                let enabled = self.catalog.bundled_enabled.contains(*id);
                self.list_targets.push((
                    [content_x + content_w - 118.0, y + 13.0, 106.0, 40.0],
                    Action::ToggleBundled(id),
                ));
                if y < top - 4.0 || y + 66.0 > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 66.0, 14.0, MY3_SURFACE);
                ui.border(content_x, y, content_w, 66.0, 14.0, MY3_BORDER);

                // Project icon badge
                ui.rounded(
                    content_x + 11.0,
                    y + 11.0,
                    44.0,
                    44.0,
                    12.0,
                    color::hex(0x27213C, 255),
                );
                ui.icon(
                    fallback_icon,
                    content_x + 22.0,
                    y + 22.0,
                    22.0,
                    MY3_ACCENT,
                );

                let text_max = (content_w - 196.0).max(80.0);
                let title = ui.fit_text(Face::SemiBold, 16.0, title, text_max);
                let summary = ui.fit_text(Face::Regular, 13.0, description, text_max);
                ui.text(
                    Face::SemiBold,
                    16.0,
                    content_x + 66.0,
                    y + 10.0,
                    MY3_TEXT,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    content_x + 66.0,
                    y + 34.0,
                    MY3_MUTED,
                    &summary,
                );
                button(
                    &mut ui,
                    &mut self.targets,
                    [content_x + content_w - 118.0, y + 13.0, 106.0, 40.0],
                    if enabled { "Installed" } else { "Install" },
                    Action::ToggleBundled(id),
                    !enabled,
                );
            }

            for (row, (i, p)) in catalog_rows.into_iter().enumerate() {
                let y = top + (row + bundled.len()) as f32 * row_h - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                let btn_rect = [content_x + content_w - 118.0, y + 13.0, 106.0, 40.0];
                if download.is_none() {
                    self.list_targets.push((btn_rect, Action::Install(i)));
                }
                if y < top - 4.0 || y + 66.0 > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 66.0, 14.0, MY3_SURFACE);
                ui.border(content_x, y, content_w, 66.0, 14.0, MY3_BORDER);

                // Project icon: fetched RGBA icon if ready, else MY3 vector icon badge.
                let drawn_icon =
                    if let Some((iw, ih, rgba)) = self.catalog.icons.get(&p.id) {
                        ui.image(
                            &p.id,
                            *iw,
                            *ih,
                            rgba,
                            content_x + 11.0,
                            y + 11.0,
                            44.0,
                            44.0,
                        )
                    } else {
                        false
                    };
                if !drawn_icon {
                    ui.rounded(
                        content_x + 11.0,
                        y + 11.0,
                        44.0,
                        44.0,
                        12.0,
                        color::hex(0x27213C, 255),
                    );
                    ui.icon(
                        fallback_icon,
                        content_x + 22.0,
                        y + 22.0,
                        22.0,
                        MY3_ACCENT,
                    );
                }

                let text_max = (content_w - 196.0).max(80.0);
                let title = ui.fit_text(Face::SemiBold, 16.0, &p.title, text_max);
                let summary = ui.fit_text(Face::Regular, 13.0, &p.summary, text_max);
                ui.text(
                    Face::SemiBold,
                    16.0,
                    content_x + 66.0,
                    y + 10.0,
                    MY3_TEXT,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    content_x + 66.0,
                    y + 34.0,
                    MY3_MUTED,
                    &summary,
                );

                if let Some(state) = download {
                    ui.rounded(
                        btn_rect[0],
                        btn_rect[1],
                        btn_rect[2],
                        btn_rect[3],
                        20.0,
                        MY3_SURFACE_HIGH,
                    );
                    ui.icon(
                        Icon::Check,
                        btn_rect[0] + 12.0,
                        btn_rect[1] + 12.0,
                        15.0,
                        MY3_ACCENT,
                    );
                    ui.text(
                        Face::Medium,
                        13.5,
                        btn_rect[0] + 31.0,
                        btn_rect[1] + 10.0,
                        MY3_TEXT,
                        &state,
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        btn_rect,
                        self.catalog.install_button(p),
                        Action::Install(i),
                        true,
                    );
                }
            }

            let pager_y = h - 50.0;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, pager_y, 108.0, 36.0],
                "Previous",
                Action::Previous,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 118.0, pager_y, 108.0, 36.0],
                "Next",
                Action::Next,
                false,
            );
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            ui.text(
                Face::SemiBold,
                20.0,
                content_x,
                76.0,
                MY3_TEXT,
                "Installed Add-ons",
            );
            let top = 114.0;
            let bottom = h - 54.0;
            let row_height = if compact { 96.0 } else { 66.0 };
            let total = Kind::ALL
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    48.0 + if self.catalog.sections[i] {
                        self.catalog.local.iter().filter(|e| e.kind == *k).count() as f32
                            * row_height
                    } else {
                        0.0
                    }
                })
                .sum::<f32>();
            let maximum = (total - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            let mut offset = 0.0;
            for (section, kind) in Kind::ALL.into_iter().enumerate() {
                let count = self.catalog.local.iter().filter(|e| e.kind == kind).count();
                let y = top + offset - self.catalog.scroll;
                let label = format!(
                    "{} {} ({count})",
                    if self.catalog.sections[section] {
                        "▾"
                    } else {
                        "▸"
                    },
                    kind.label()
                );
                let sec_w = (content_w - 124.0).max(120.0);
                self.list_targets
                    .push(([content_x, y, sec_w, 40.0], Action::ToggleSection(section)));
                self.list_targets.push((
                    [content_x + sec_w + 10.0, y, 114.0, 40.0],
                    Action::Import(kind),
                ));
                if y >= top - 4.0 && y + 40.0 <= bottom {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x, y, sec_w, 40.0],
                        &label,
                        Action::ToggleSection(section),
                        self.catalog.sections[section],
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x + sec_w + 10.0, y, 114.0, 40.0],
                        "Import",
                        Action::Import(kind),
                        false,
                    );
                }
                offset += 48.0;
                if !self.catalog.sections[section] {
                    continue;
                }
                for (i, item) in self
                    .catalog
                    .local
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.kind == kind)
                {
                    let y = top + offset - self.catalog.scroll;
                    offset += row_height;
                    let toggle_x = if compact {
                        content_x + 14.0
                    } else {
                        content_x + content_w - 236.0
                    };
                    let remove_x = content_x + content_w - 110.0;
                    self.list_targets.push((
                        [toggle_x, y, 110.0, row_height - 6.0],
                        Action::ToggleLocal(i),
                    ));
                    self.list_targets.push((
                        [remove_x, y, 98.0, row_height - 6.0],
                        Action::DeleteLocal(i),
                    ));
                    if y < top - 4.0 || y + row_height - 4.0 > bottom {
                        continue;
                    }
                    ui.rounded(content_x, y, content_w, row_height - 6.0, 14.0, MY3_SURFACE);
                    ui.border(content_x, y, content_w, row_height - 6.0, 14.0, MY3_BORDER);
                    let title = ui.fit_text(
                        Face::SemiBold,
                        15.5,
                        &item.title,
                        if compact {
                            content_w - 28.0
                        } else {
                            content_w - 260.0
                        },
                    );
                    ui.text(Face::SemiBold, 15.5, content_x + 16.0, y + 8.0, MY3_TEXT, &title);
                    ui.text(
                        Face::Regular,
                        13.0,
                        content_x + 16.0,
                        y + 32.0,
                        MY3_MUTED,
                        kind.label(),
                    );
                    toggle(
                        &mut ui,
                        &mut self.targets,
                        [
                            toggle_x,
                            y + if compact { 48.0 } else { 10.0 },
                            110.0,
                            40.0,
                        ],
                        item.enabled,
                        Action::ToggleLocal(i),
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [
                            remove_x,
                            y + if compact { 48.0 } else { 11.0 },
                            98.0,
                            38.0,
                        ],
                        "Remove",
                        Action::DeleteLocal(i),
                        false,
                    );
                }
            }
        } else {
            let c = &self.catalog.config;
            ui.text(
                Face::SemiBold,
                20.0,
                content_x,
                76.0,
                MY3_TEXT,
                "Settings",
            );
            let row_height = if compact { 78.0 } else { 58.0 };
            let list_top = 112.0;
            let list_bottom = h - 96.0;
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (18.0 * row_height - (list_bottom - list_top)).max(0.0));
            self.list_view = Some([
                list_top,
                list_bottom,
                (18.0 * row_height - (list_bottom - list_top)).max(0.0),
            ]);
            for (i, (title, value, action)) in [
                ("Rendering", c.renderer.label().to_owned(), Action::Renderer),
                (
                    "Client resolution",
                    if self.catalog.editing == Some("render_scale") {
                        self.catalog.render_scale_text.clone()
                    } else {
                        format!("{}%", c.render_scale.percent())
                    },
                    Action::RenderScale,
                ),
                (
                    "Chunk worker limit",
                    if c.worker_limit == 0 {
                        "No additional limit".into()
                    } else {
                        c.worker_limit.to_string()
                    },
                    Action::WorkerLimit,
                ),
                (
                    "Reusable RAM cache",
                    if c.cache_budget_mib == 0 {
                        "Engine default (80 MiB)".into()
                    } else {
                        format!("{} MiB", c.cache_budget_mib)
                    },
                    Action::CacheBudget,
                ),
                (
                    "Game files",
                    "Open in system Files".into(),
                    Action::GameFiles,
                ),
                (
                    "World files",
                    "Import / copy / back up in Files".into(),
                    Action::WorldFiles,
                ),
                (
                    "Profile",
                    format!("Profile {}", c.profile + 1),
                    Action::Profile,
                ),
                (
                    "Orientation",
                    c.orientation.label().to_owned(),
                    Action::Orientation,
                ),
                ("Input", c.input_host.label().to_owned(), Action::InputHost),
                (
                    "Immersive fullscreen",
                    if c.screen.immersive { "On" } else { "Off" }.into(),
                    Action::Immersive,
                ),
                (
                    "Avoid camera cutout",
                    if c.screen.avoid_cutout { "On" } else { "Off" }.into(),
                    Action::Cutout,
                ),
                (
                    "Aspect ratio",
                    c.screen.aspect.label().into(),
                    Action::Aspect,
                ),
                (
                    "Display refresh rate",
                    c.device.display_label(),
                    Action::DisplayHz,
                ),
                (
                    "Performance hints",
                    if c.device.performance_hints {
                        "On (ADPF)"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::PerformanceHints,
                ),
                (
                    "GPU profiling",
                    if c.device.gpu_profile {
                        "On / included in report"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::GpuProfile,
                ),
                (
                    "Performance recording",
                    if c.record_performance {
                        "On / up to 30 min"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::RecordPerformance,
                ),
                (
                    "Diagnostic report",
                    "Export one ZIP to share".into(),
                    Action::ExportDiagnostics,
                ),
                (
                    "Raw performance data",
                    "Advanced / JSONL in Reports".into(),
                    Action::ExportPerformance,
                ),
                (
                    "Server",
                    if c.server.is_empty() {
                        "Singleplayer / tap to enter server".into()
                    } else {
                        c.server.clone()
                    },
                    Action::Field("server"),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let y = list_top + i as f32 * row_height - self.catalog.scroll;
                let ctrl_x = if compact {
                    content_x
                } else {
                    content_x + content_w * 0.42
                };
                let ctrl_w = if compact {
                    content_w
                } else {
                    content_w * 0.58
                };
                self.list_targets.push((
                    [ctrl_x, y, ctrl_w, if compact { 66.0 } else { 42.0 }],
                    action,
                ));
                if y < list_top - 6.0 || y + if compact { 66.0 } else { 42.0 } > list_bottom {
                    continue;
                }
                ui.text(
                    Face::Regular,
                    15.0,
                    content_x,
                    y + if compact { 0.0 } else { 10.0 },
                    MY3_MUTED,
                    title,
                );
                if matches!(
                    action,
                    Action::PerformanceHints | Action::Immersive | Action::Cutout
                ) {
                    toggle(
                        &mut ui,
                        &mut self.targets,
                        [
                            ctrl_x,
                            y + if compact { 22.0 } else { 0.0 },
                            ctrl_w,
                            42.0,
                        ],
                        match action {
                            Action::PerformanceHints => c.device.performance_hints,
                            Action::Immersive => c.screen.immersive,
                            Action::Cutout => c.screen.avoid_cutout,
                            _ => c.device.gpu_profile,
                        },
                        action,
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [
                            ctrl_x,
                            y + if compact { 22.0 } else { 0.0 },
                            ctrl_w,
                            42.0,
                        ],
                        &format!(
                            "{} {}",
                            value,
                            if matches!(
                                action,
                                Action::Renderer
                                    | Action::Aspect
                                    | Action::DisplayHz
                                    | Action::RenderScale
                                    | Action::Profile
                                    | Action::Orientation
                                    | Action::InputHost
                            ) {
                                "v"
                            } else {
                                ""
                            }
                        ),
                        action,
                        false,
                    );
                }
                ui.rounded(
                    content_x,
                    y + if compact { 72.0 } else { 51.0 },
                    content_w,
                    1.0,
                    0.0,
                    MY3_BORDER,
                );
            }
            let half = (content_w - 12.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, h - 86.0, half, 38.0],
                "Diagnostics",
                Action::Logs,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + half + 12.0, h - 86.0, half, 38.0],
                "Microsoft / Xbox account",
                Action::Account,
                true,
            );
        }

        // Bottom status strip when not in a full-screen progress dialog.
        let in_progress_dialog = (page == Page::Play && self.worker.is_some())
            || (self.catalog.downloading.is_some() && self.catalog.busy());
        if !in_progress_dialog {
            let status = self.error.as_deref().unwrap_or(&self.catalog.status);
            if !status.is_empty() && page != Page::Mods && page != Page::Packs && page != Page::Shaders {
                let fitted = ui.fit_text(
                    Face::Regular,
                    13.0,
                    status,
                    (content_w - 120.0).max(60.0),
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    content_x,
                    h - 38.0,
                    MY3_MUTED,
                    &fitted,
                );
            }
            if page == Page::Play || page == Page::Settings {
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 126.0, h - 44.0, 98.0, 30.0],
                    "GitHub",
                    Action::Repository,
                    false,
                );
            }
        }

        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing == Some("name"),
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 10.0, top, 4.0, height, 2.0, MY3_BORDER);
            ui.rounded(w - 10.0, y, 4.0, thumb, 2.0, MY3_MUTED);
        }
        let caret_visible = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok_and(|t| t.as_millis() % 1000 < 500);
        if caret_visible && let Some(name) = self.catalog.editing {
            let value = match name {
                "name" => self.catalog.config.name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => self.catalog.query.clone(),
            };
            for ([x, y, w, _], action) in &self.targets {
                if *action == Action::Field(name)
                    || (name == "render_scale" && *action == Action::RenderScale)
                {
                    let pad = if name == "search" { 38.0 } else { 14.0 };
                    let text = ui.fit_text(Face::Regular, 14.0, &value, w - pad - 12.0);
                    let caret_x = x + pad + ui.measure(Face::Regular, 14.0, &text);
                    ui.rounded(
                        caret_x.min(x + w - 10.0),
                        y + 11.0,
                        1.5,
                        18.0,
                        0.5,
                        MY3_ACCENT,
                    );
                }
            }
        }
        if self.exit_confirmation {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(0.0, 0.0, w, h, 0.0, horizon_client::hud::rgba(0, 0, 0, 180));
            let width = (w - 32.0).min(360.0);
            let x = (w - width) * 0.5;
            let y = (h - 160.0) * 0.5;
            ui.rounded(x, y, width, 160.0, 20.0, MY3_SURFACE);
            ui.border(x, y, width, 160.0, 20.0, MY3_BORDER);
            ui.text(
                Face::SemiBold,
                20.0,
                x + 22.0,
                y + 24.0,
                MY3_TEXT,
                "Exit Horizon Android?",
            );
            let bw = (width - 52.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [x + 20.0, y + 94.0, bw, 42.0],
                "Cancel",
                Action::Dismiss,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [x + 32.0 + bw, y + 94.0, bw, 42.0],
                "Exit",
                Action::ConfirmExit,
                true,
            );
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            ui.border(*x - 2.0, *y - 2.0, *w + 4.0, *h + 4.0, 12.0, MY3_ACCENT);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        if self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved) {
            let point = [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ];
            if let Some(([x, y, w, h], _)) = self.targets.iter().rev().find(|(rect, _)| {
                point[0] >= rect[0]
                    && point[0] <= rect[0] + rect[2]
                    && point[1] >= rect[1]
                    && point[1] <= rect[1] + rect[3]
            }) {
                ui.rounded(
                    *x,
                    *y,
                    *w,
                    *h,
                    (*h * 0.42).clamp(10.0, 22.0),
                    horizon_client::hud::rgba(255, 255, 255, 36),
                );
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(160)
            && let Some(([x, y, w, h], _)) = self.targets.iter().find(|(_, a)| *a == action)
        {
            ui.rounded(
                *x,
                *y,
                *w,
                *h,
                (*h * 0.42).clamp(10.0, 22.0),
                horizon_client::hud::rgba(255, 255, 255, 36),
            );
        }
        crate::documents::draw_ui(&mut ui);
        ui.finish();

        if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        Ok(())
    }
    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() && self.chooser.is_none() && !self.exit_confirmation {
            return Ok(());
        }
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        let p = self.focus.and_then(|index| self.targets.get(index)).map_or(
            [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ],
            |([x, y, w, h], _)| [x + w / 2.0, y + h / 2.0],
        );
        let action = self
            .targets
            .iter()
            .find(|([x, y, w, h], _)| {
                (*x..*x + *w).contains(&p[0]) && (*y..*y + *h).contains(&p[1])
            })
            .map(|(_, a)| *a);
        if crate::documents::busy() {
            return Ok(());
        }
        if self.catalog.busy()
            && !matches!(
                action,
                Some(
                    Action::PauseCatalogDownload
                        | Action::CancelCatalogDownload
                        | Action::Page(_)
                        | Action::Account
                )
            )
        {
            return Ok(());
        }
        self.feedback = action.map(|action| (action, std::time::Instant::now()));
        self.focus = None;
        if action.is_some() && horizon_client::official_install::ready(&self.home) {
            let audio = self.audio.get_or_insert_with(|| {
                let mut audio = horizon_client::audio::Audio::new(
                    &self.home.join(".local/horizon-client/26.3"),
                );
                let settings = horizon_client::settings::Settings::load(
                    &self
                        .catalog
                        .config
                        .game_home(&self.home)
                        .join("options.txt"),
                );
                audio.master = settings.volumes[0];
                let mut volumes = settings.volumes;
                volumes[0] = 1.0;
                horizon_client::settings::set_source_volumes(volumes);
                audio
            });
            audio.play_button_click();
        }
        match action {
            Some(Action::PauseInstall) => {
                let next = !self.install_paused.load(Ordering::Relaxed);
                self.install_paused.store(next, Ordering::Relaxed);
            }
            Some(Action::CancelInstall) => {
                self.install_cancelled.store(true, Ordering::Relaxed);
                self.install_paused.store(false, Ordering::Relaxed);
                self.worker = None;
                self.catalog.status = "Game preparation cancelled.".into();
            }
            Some(Action::PauseCatalogDownload) => {
                self.catalog.toggle_pause();
            }
            Some(Action::CancelCatalogDownload) => {
                self.catalog.cancel_download();
            }
            Some(Action::ConfirmExit) => {
                events.exit();
            }
            Some(Action::Immersive) => {
                self.catalog.config.screen.immersive ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Cutout) => {
                self.catalog.config.screen.avoid_cutout ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Aspect) => self.open_chooser(chooser::Kind::Aspect),
            Some(Action::ChooseAspect(aspect)) => {
                self.catalog.config.screen.aspect = aspect;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
                self.chooser = None;
            }
            Some(Action::CacheBudget) => self.open_chooser(chooser::Kind::CacheBudget),
            Some(Action::ChooseCacheBudget(mib)) => {
                self.catalog.config.cache_budget_mib = mib.min(1024);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Disposable cache budget saved. Applies on next Play; not a total RAM or VRAM limit.".into();
            }
            Some(Action::ToggleBundled(id)) => {
                let game = self.catalog.config.game_home(&self.home);
                let mut list = horizon_client::mod_list::ModList::load(&game);
                let enabled = list.enabled.contains(id) && !list.disabled.contains(id);
                if enabled {
                    list.enabled.remove(id);
                    list.disabled.insert(id.into());
                } else {
                    list.disabled.remove(id);
                    list.enabled.insert(id.into());
                }
                std::fs::create_dir_all(&game)?;
                std::fs::write(game.join(horizon_client::mod_list::FILE), list.serialize())?;
                self.catalog.refresh(&self.home);
                self.catalog.status = format!(
                    "Bundled {id} {}. Applies on Play; settings are in Options > Mods.",
                    if enabled { "disabled" } else { "installed & enabled" }
                );
            }
            Some(Action::GameFiles) => {
                self.catalog.status = if crate::platform::open_documents(crate::document_store::ROOT) {
                    "Game files opened. Worlds, mods, packs, shaders and reports are available here."
                } else { "Open system Files > Horizon Android." }.into();
            }
            Some(Action::WorldFiles) => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "worlds");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Copy a complete world folder to back up or import. Finish copying before Play; never replace an existing world silently."
                } else { "Open system Files > Horizon Android > Worlds. Copy complete world folders while the game is closed." }.into();
            }
            Some(Action::WorkerLimit) => self.open_chooser(chooser::Kind::WorkerLimit),
            Some(Action::ChooseWorkerLimit(limit)) => {
                self.catalog.config.worker_limit = limit.min(64);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Chunk worker ceiling saved. Applies on the next Play; other engine threads are unchanged.".into();
            }
            Some(Action::DisplayHz) => self.open_chooser(chooser::Kind::DisplayHz),
            Some(Action::ChooseDisplayHz(hz)) => {
                self.catalog.config.device.display_hz = hz;
                self.catalog.config.device.sanitize();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
                self.chooser = None;
            }
            Some(Action::PerformanceHints) => {
                self.catalog.config.device.performance_hints ^= true;
                self.catalog.status="ADPF gives Android CPU frame-work hints on supported devices. It does not cap FPS or guarantee a speedup.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }
            Some(Action::GpuProfile) => {
                self.catalog.config.device.gpu_profile ^= true;
                self.catalog.status="Detailed GPU profiling records diagnostics and has overhead. Reports remain on this device.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }

            Some(Action::ModSource(origin)) => {
                self.catalog.config.mod_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::ShaderSource(origin)) => {
                self.catalog.config.shader_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::OpenFolder(kind)) => {
                crate::platform::hide();
                let folder = match kind {
                    horizon_client::content_files::Kind::Mods => "mods",
                    horizon_client::content_files::Kind::Packs => "packs",
                    horizon_client::content_files::Kind::Shaders => "shaders",
                };
                let id = crate::document_store::folder_id(self.catalog.config.profile, folder);
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Opened in Files. Return here after copying files.".into()
                } else {
                    "Open the system Files app and choose Horizon Android.".into()
                };
            }
            Some(Action::Import(kind)) => {
                crate::platform::hide();
                let home = self.catalog.config.game_home(&self.home);
                if let Err(e) = crate::documents::begin(kind, home) {
                    self.catalog.status = e;
                }
            }
            Some(Action::ToggleSection(i)) => {
                self.catalog.sections[i] = !self.catalog.sections[i];
                self.catalog.scroll = 0.0;
            }
            Some(Action::ToggleLocal(i)) => self.catalog.toggle_local(&self.home, i),
            Some(Action::DeleteLocal(i)) => {
                self.catalog.pending_delete = self.catalog.local.get(i).cloned();
                if let Some(item) = &self.catalog.pending_delete {
                    self.chooser = Some(super::chooser::Chooser::confirm_delete(item.file.clone()));
                }
            }
            Some(Action::ConfirmDeleteLocal) => {
                self.catalog.remove_local(&self.home);
                self.chooser = None;
            }
            Some(Action::Repository) => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            Some(Action::Account) => {
                self.login.visible = true;
            }
            Some(Action::CloseLogin) => {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
            }
            Some(Action::DeviceSignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.login
                    .start_device(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.login.start(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::SignOut) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.login.sign_out();
            }
            Some(Action::LoginLink) => self.login.open_link(),
            Some(Action::Page(page)) => {
                self.catalog.editing = None;
                if crate::platform::active() {
                    crate::platform::hide();
                }
                self.catalog.page = page;
                self.catalog.scroll = 0.0;
                self.error = None;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::Play) => self.action(events)?,
            Some(Action::Search) => {
                crate::platform::hide();
                self.catalog.offset = 0;
                self.catalog.search(&self.home);
            }
            Some(Action::FinishNameEdit) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::Field(name)) => {
                if name == "render_scale" {
                    self.catalog.render_scale_text =
                        self.catalog.config.render_scale.percent().to_string();
                    self.chooser = None;
                }
                self.catalog.editing = Some(name);
                let initial = match name {
                    "name" => &self.catalog.config.name,
                    "server" => &self.catalog.config.server,
                    "render_scale" => &self.catalog.render_scale_text,
                    "client_id" => &self.catalog.config.microsoft_client_id,
                    _ => &self.catalog.query,
                };
                crate::platform::show(initial);
            }
            Some(Action::Install(i)) => self.catalog.install(&self.home, i),
            Some(Action::Next) => {
                self.catalog.offset += 12;
                self.catalog.search(&self.home);
            }
            Some(Action::Previous) => {
                self.catalog.offset = self.catalog.offset.saturating_sub(12);
                self.catalog.search(&self.home);
            }
            Some(Action::Profile) => self.open_chooser(chooser::Kind::Profile),
            Some(Action::Renderer) => self.open_chooser(chooser::Kind::Renderer),
            Some(Action::RenderScale) => self.open_chooser(chooser::Kind::RenderScale),
            Some(Action::ChooseRenderScale(scale)) => {
                self.catalog.config.render_scale = scale;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::display::set_scale(scale);
            }
            Some(Action::Orientation) => self.open_chooser(chooser::Kind::Orientation),
            Some(Action::InputHost) => self.open_chooser(chooser::Kind::InputHost),
            Some(Action::ChooseInputHost(mode)) => {
                self.catalog.config.input_host = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::configure_input_mode(mode);
            }
            Some(Action::ChooseOrientation(mode)) => {
                self.catalog.config.orientation = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::platform::set_orientation(mode);
            }
            Some(Action::ChooseRenderer(mode)) => {
                self.catalog.config.renderer = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
            }
            Some(Action::ChooseProfile(profile)) => {
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })?;
                self.catalog.config.profile = profile;
                self.catalog.config.save(&self.home)?;
                self.catalog.refresh(&self.home);
                self.chooser = None;
            }
            Some(Action::Dismiss) => {
                self.exit_confirmation = false;
                self.chooser = None;
                if self.session_worker.is_none() {
                    self.session_ready = false;
                    self.prepared_session = None;
                }
            }
            Some(Action::Logs) => {
                self.catalog.status = crate::diagnostics::summary(&self.home);
                if let Ok(error) = std::fs::read_to_string(self.home.join("android-error.txt")) {
                    self.catalog.status = format!(
                        "Last handled error: {}. {}",
                        crate::diagnostics::redact(&error),
                        self.catalog.status
                    );
                }
            }
            Some(Action::ExportDiagnostics) if self.diagnostic_export.is_none() => {
                let home = self.home.clone();
                let (sender, receiver) = mpsc::channel();
                self.diagnostic_export = Some(receiver);
                self.catalog.status =
                    "Preparing local diagnostic ZIP. No upload; review before sharing.".into();
                std::thread::spawn(move || {
                    let result = crate::diagnostics::export(&home).map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                    let _ = sender.send(result);
                });
            }
            Some(Action::ExportDiagnostics) => {}
            Some(Action::RecordPerformance) => {
                self.catalog.config.record_performance = !self.catalog.config.record_performance;
                if !self.catalog.config.record_performance {
                    crate::performance::stop();
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.record_performance {
                    "Recording starts on Play. Detailed timings add overhead. Return here to save the report."
                } else { "Performance recording off." }.into();
            }
            Some(Action::ExportPerformance) => {
                self.catalog.status = match crate::performance::export(&self.home) {
                    Ok(()) => "Reports opened in Files. Copy or share a report from there.".into(),
                    Err(e) => e,
                };
            }
            None => {}
        }
        Ok(())
    }
    pub(super) fn ui_density(&self) -> f32 {
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return 1.0;
        };
        let mut batch = HudBatch::new([r.config.width, r.config.height]);
        Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0)).density
    }

    pub(super) fn touch_event(&mut self, events: &ActiveEventLoop, t: Touch) -> Result<()> {
        self.focus = None;
        self.pending_focus = None;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.cursor = (t.location.x, t.location.y);
        match t.phase {
            TouchPhase::Started => {
                self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
            }
            TouchPhase::Moved => {
                let density = self.ui_density() as f64;
                if let Some(drag) = &mut self.drag {
                    let delta = drag.update(t.location.x, t.location.y, density);
                    if drag.moved {
                        if let Some(chooser) = &mut self.chooser {
                            chooser.scroll = (chooser.scroll + delta).max(0.0);
                        } else if self
                            .list_view
                            .is_some_and(|view| drag.origin_y() / density >= view[0] as f64)
                        {
                            self.catalog.scroll = (self.catalog.scroll + delta).max(0.0);
                        }
                    }
                }
            }
            TouchPhase::Ended => {
                if self.drag.take().is_some_and(|drag| !drag.moved) {
                    self.activate(events)?;
                }
            }
            TouchPhase::Cancelled => {
                self.drag = None;
            }
        }
        Ok(())
    }
    pub(super) fn type_key(&mut self, key: KeyEvent) {
        if key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            || key.logical_key
                == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
        {
            if !key.repeat {
                self.request_back();
            }
            return;
        }
        if self.catalog.editing.is_none() {
            return;
        }
        let out = match key.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => Some(Output::Backspace),
            PhysicalKey::Code(KeyCode::Enter) => {
                crate::platform::hide();
                Some(Output::Enter)
            }
            _ => key.text.map(|s| Output::Text(s.into())),
        };
        if let Some(out) = out {
            self.catalog.edit(out, &self.home);
        }
    }
}
