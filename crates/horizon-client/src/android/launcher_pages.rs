use super::*;
use horizon_client::osk::Output;
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Page(Page),
    Play,
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
    PerformanceHints,
    GpuProfile,
}

/// Material 3 Expressive colour roles for the launcher: the dark baseline scheme,
/// which sits on Horizon's violet accent.
pub(super) mod m3 {
    use horizon_client::ui_theme::color::hex;
    pub const SURFACE: u32 = hex(0x141218, 255);
    pub const SURFACE_LOW: u32 = hex(0x1D1B20, 255);
    pub const CONTAINER: u32 = hex(0x211F26, 255);
    pub const CONTAINER_HIGH: u32 = hex(0x2B2930, 255);
    pub const CONTAINER_HIGHEST: u32 = hex(0x36343B, 255);
    pub const ON_SURFACE: u32 = hex(0xE6E0E9, 255);
    pub const ON_SURFACE_VARIANT: u32 = hex(0xCAC4D0, 255);
    pub const OUTLINE: u32 = hex(0x938F99, 255);
    pub const OUTLINE_VARIANT: u32 = hex(0x49454F, 255);
    pub const PRIMARY: u32 = hex(0xD0BCFF, 255);
    pub const ON_PRIMARY: u32 = hex(0x381E72, 255);
    pub const PRIMARY_CONTAINER: u32 = hex(0x4F378B, 255);
    pub const ON_PRIMARY_CONTAINER: u32 = hex(0xEADDFF, 255);
    pub const SECONDARY_CONTAINER: u32 = hex(0x4A4458, 255);
    pub const ON_SECONDARY_CONTAINER: u32 = hex(0xE8DEF8, 255);
    pub const TERTIARY_CONTAINER: u32 = hex(0x633B48, 255);
    pub const ON_TERTIARY_CONTAINER: u32 = hex(0xFFD8E4, 255);
    pub const ERROR: u32 = hex(0xF2B8B5, 255);
    pub const ERROR_CONTAINER: u32 = hex(0x8C1D18, 255);
    pub const SCRIM: u32 = hex(0x000000, 160);
    pub const STATE_LAYER: u32 = hex(0xFFFFFF, 30);
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Style {
    Filled,
    Tonal,
    Outlined,
    Field,
}

/// The top of a `size` text line centred in a box of height `h` at `y`.
pub(super) fn label_y(y: f32, h: f32, size: f32) -> f32 {
    y + ((h - (size * 1.45).round()) / 2.0).max(0.0)
}

fn centered(ui: &mut Ui, face: Face, size: f32, cx: f32, y: f32, color: u32, text: &str) {
    let width = ui.measure(face, size, text);
    ui.text(face, size, cx - width / 2.0, y, color, text);
}

/// The label size `pill` uses for a box of this size.
pub(super) fn pill_text_size(w: f32, h: f32) -> f32 {
    if h >= 56.0 {
        18.0
    } else if w < 90.0 {
        13.0
    } else {
        15.0
    }
}

/// A Material 3 button: fully rounded, optional leading icon, centred label
/// (fields keep a left-aligned value in a rounded box).
pub(super) fn pill(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    icon: Option<&str>,
    label: &str,
    action: Action,
    style: Style,
) {
    let [x, y, w, h] = rect;
    let radius = if style == Style::Field {
        16.0f32.min(h / 2.0)
    } else {
        h / 2.0
    };
    let (background, foreground) = match style {
        Style::Filled => (m3::PRIMARY, m3::ON_PRIMARY),
        Style::Tonal => (m3::SECONDARY_CONTAINER, m3::ON_SECONDARY_CONTAINER),
        Style::Outlined => (0, m3::PRIMARY),
        Style::Field => (m3::CONTAINER_HIGHEST, m3::ON_SURFACE),
    };
    ui.rounded(x, y, w, h, radius, background);
    if style == Style::Outlined {
        ui.border(x, y, w, h, radius, m3::OUTLINE);
    }
    let size = pill_text_size(w, h);
    let icon_size = if h >= 56.0 { 24.0 } else { 20.0 };
    let pad = if style == Style::Field {
        16.0
    } else {
        (h / 2.0).clamp(10.0, 24.0)
    };
    let gap = 8.0;
    let icon_room = icon.map_or(0.0, |_| icon_size + gap);
    let text = ui.fit_text(
        Face::Medium,
        size,
        label,
        (w - 2.0 * pad - icon_room).max(0.0),
    );
    let content = ui.measure(Face::Medium, size, &text) + icon_room;
    let mut cx = if style == Style::Field {
        x + pad
    } else {
        x + ((w - content) / 2.0).max(pad.min(w / 2.0))
    };
    if let Some(icon) = icon {
        super::icons::draw(
            ui,
            icon,
            cx,
            y + (h - icon_size) / 2.0,
            icon_size,
            foreground,
        );
        cx += icon_room;
    }
    ui.text(
        Face::Medium,
        size,
        cx,
        label_y(y, h, size),
        foreground,
        &text,
    );
    targets.push((rect, action));
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    let style = if accent {
        Style::Filled
    } else if matches!(action, Action::Field(_)) {
        Style::Field
    } else {
        Style::Tonal
    };
    pill(ui, targets, rect, None, label, action, style);
}

/// A Material 3 switch (52 × 32) with its top-left corner at (`x`, `y`).
pub(super) fn switch(ui: &mut Ui, x: f32, y: f32, on: bool) {
    if on {
        ui.rounded(x, y, 52.0, 32.0, 16.0, m3::PRIMARY);
        ui.rounded(x + 24.0, y + 4.0, 24.0, 24.0, 12.0, m3::ON_PRIMARY);
        super::icons::draw(ui, "check", x + 28.0, y + 8.0, 16.0, m3::PRIMARY);
    } else {
        ui.rounded(x, y, 52.0, 32.0, 16.0, m3::CONTAINER_HIGHEST);
        ui.border(x, y, 52.0, 32.0, 16.0, m3::OUTLINE);
        ui.rounded(x + 8.0, y + 8.0, 16.0, 16.0, 8.0, m3::OUTLINE);
    }
}

fn page_icon(page: Page) -> &'static str {
    match page {
        Page::Play => "play",
        Page::Mods => "package",
        Page::Packs => "image",
        Page::Shaders => "zap",
        Page::Installed => "folder",
        Page::Settings => "sliders",
    }
}

fn page_title(page: Page, wide: bool) -> &'static str {
    match page {
        Page::Play => "Play",
        Page::Mods => "Horizon Mods",
        Page::Packs if wide => "Resource Packs",
        Page::Packs => "Packs",
        Page::Shaders if wide => "Shader Packs",
        Page::Shaders => "Shaders",
        Page::Installed if wide => "Installed content",
        Page::Installed => "Installed",
        Page::Settings if wide => "Launch settings",
        Page::Settings => "Settings",
    }
}

/// A navigation rail or navigation bar destination: the active indicator pill,
/// its icon and the label below.
fn destination(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    page: Page,
    selected: bool,
    label: &str,
) {
    let [x, y, w, h] = rect;
    let indicator = 56.0f32.min(w - 8.0);
    let block = 32.0 + 4.0 + 17.0;
    let top = y + ((h - block) / 2.0).max(0.0);
    let cx = x + w / 2.0;
    if selected {
        ui.rounded(
            cx - indicator / 2.0,
            top,
            indicator,
            32.0,
            16.0,
            m3::SECONDARY_CONTAINER,
        );
    }
    super::icons::draw(
        ui,
        page_icon(page),
        cx - 12.0,
        top + 4.0,
        24.0,
        if selected {
            m3::ON_SECONDARY_CONTAINER
        } else {
            m3::ON_SURFACE_VARIANT
        },
    );
    let label = ui.fit_text(Face::Medium, 12.0, label, w - 4.0);
    centered(
        ui,
        Face::Medium,
        12.0,
        cx,
        top + 36.0,
        if selected {
            m3::ON_SURFACE
        } else {
            m3::ON_SURFACE_VARIANT
        },
        &label,
    );
    targets.push((rect, Action::Page(page)));
}

/// The account chip: an avatar, then the sign-in state.
fn account_chip(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    signed_in: bool,
) {
    let [x, y, w, h] = rect;
    ui.rounded(x, y, w, h, h / 2.0, m3::CONTAINER_HIGH);
    let avatar = h - 12.0;
    ui.rounded(
        x + 6.0,
        y + 6.0,
        avatar,
        avatar,
        avatar / 2.0,
        if signed_in {
            m3::PRIMARY
        } else {
            m3::PRIMARY_CONTAINER
        },
    );
    let icon = (avatar * 0.6).round();
    super::icons::draw(
        ui,
        "user",
        x + 6.0 + (avatar - icon) / 2.0,
        y + 6.0 + (avatar - icon) / 2.0,
        icon,
        if signed_in {
            m3::ON_PRIMARY
        } else {
            m3::ON_PRIMARY_CONTAINER
        },
    );
    let text = ui.fit_text(Face::Medium, 14.0, label, w - avatar - 30.0);
    ui.text(
        Face::Medium,
        14.0,
        x + avatar + 16.0,
        label_y(y, h, 14.0),
        m3::ON_SURFACE,
        &text,
    );
    targets.push((rect, Action::Account));
}

/// A list row's leading icon in a tonal tile.
fn icon_tile(ui: &mut Ui, icon: &str, x: f32, y: f32, size: f32, background: u32, color: u32) {
    ui.rounded(x, y, size, size, (size * 0.3).round(), background);
    let glyph = (size * 0.5).round();
    super::icons::draw(
        ui,
        icon,
        x + (size - glyph) / 2.0,
        y + (size - glyph) / 2.0,
        glyph,
        color,
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
        let (Some(renderer), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        if renderer.suspended {
            return Ok(());
        }
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;
        let mut batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0));
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
        let (surface_w, surface_h) = (ui.batch.width, ui.batch.height);
        ui.batch
            .fill(&gui.atlas, 0.0, 0.0, surface_w, surface_h, m3::SURFACE);
        let wide = w >= 700.0;
        let page = self.catalog.page;
        // Where the value of the field being edited starts: x, line top, room, text size.
        let mut caret: Option<[f32; 4]> = None;
        let account = self.login.status(&self.catalog.config.name);
        // Navigation: a rail on wide screens, a bottom bar on phones held upright.
        let (l, r, t, b, status_y) = if wide {
            const RAIL: f32 = 96.0;
            ui.rounded(0.0, 0.0, RAIL, h, 0.0, m3::SURFACE_LOW);
            ui.rounded(24.0, 20.0, 48.0, 48.0, 16.0, m3::PRIMARY_CONTAINER);
            centered(
                &mut ui,
                Face::SemiBold,
                24.0,
                48.0,
                label_y(20.0, 48.0, 24.0),
                m3::ON_PRIMARY_CONTAINER,
                "H",
            );
            let step = ((h - 92.0 - 84.0) / Page::ALL.len() as f32).clamp(56.0, 76.0);
            for (i, p) in Page::ALL.iter().enumerate() {
                destination(
                    &mut ui,
                    &mut self.targets,
                    [0.0, 92.0 + i as f32 * step, RAIL, step],
                    *p,
                    *p == page,
                    p.name(),
                );
            }
            ui.rounded(24.0, h - 72.0, 48.0, 48.0, 24.0, m3::CONTAINER_HIGH);
            super::icons::draw(
                &mut ui,
                "external-link",
                36.0,
                h - 60.0,
                24.0,
                m3::ON_SURFACE_VARIANT,
            );
            self.targets
                .push(([24.0, h - 72.0, 48.0, 48.0], Action::Repository));
            let (l, r) = (RAIL + 28.0, w - 28.0);
            let title = ui.fit_text(Face::SemiBold, 30.0, page_title(page, true), r - l - 300.0);
            ui.text(Face::SemiBold, 30.0, l, 20.0, m3::ON_SURFACE, &title);
            account_chip(
                &mut ui,
                &mut self.targets,
                [r - 280.0, 18.0, 280.0, 52.0],
                &account,
                self.login.signed_in,
            );
            (l, r, 92.0, h - 68.0, h - 56.0)
        } else {
            const BAR: f32 = 80.0;
            let chip = (w * 0.48).min(220.0);
            let title = ui.fit_text(
                Face::SemiBold,
                24.0,
                page_title(page, false),
                w - chip - 52.0,
            );
            ui.text(Face::SemiBold, 24.0, 20.0, 20.0, m3::ON_SURFACE, &title);
            account_chip(
                &mut ui,
                &mut self.targets,
                [w - 20.0 - chip, 14.0, chip, 48.0],
                &account,
                self.login.signed_in,
            );
            ui.rounded(0.0, h - BAR, w, BAR, 0.0, m3::SURFACE_LOW);
            let item = w / Page::ALL.len() as f32;
            for (i, p) in Page::ALL.iter().enumerate() {
                destination(
                    &mut ui,
                    &mut self.targets,
                    [i as f32 * item, h - BAR + 4.0, item, BAR - 8.0],
                    *p,
                    *p == page,
                    match p {
                        Page::Installed => "Files",
                        Page::Settings => "Setup",
                        _ => p.name(),
                    },
                );
            }
            (20.0, w - 20.0, 80.0, h - BAR - 58.0, h - BAR - 50.0)
        };
        let compact = r - l < 600.0;
        if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self.install.lock().unwrap();
            let current = InstallStage::ALL
                .iter()
                .position(|s| *s == progress.stage)
                .unwrap_or(0);
            let fraction = if progress.total > 0 {
                progress.done as f32 / progress.total as f32
            } else {
                0.0
            };
            let overall = (current as f32 + fraction.clamp(0.0, 1.0)) / 6.0;
            let c = &self.catalog.config;
            // The hero card on the left (top on phones); account, version and the
            // launch button in the side panel.
            let (hero, side) = if wide {
                let panel = ((r - l) * 0.36).clamp(280.0, 360.0);
                (
                    [l, t, r - l - panel - 16.0, b - t],
                    [r - panel, t, panel, b - t],
                )
            } else {
                let side_h = 88.0 + 12.0 + 76.0;
                (
                    [l, t, r - l, b - t - side_h - 16.0],
                    [l, b - side_h, r - l, side_h],
                )
            };
            let [hx, hy, hw, hh] = hero;
            ui.rounded(hx, hy, hw, hh, 28.0, m3::CONTAINER);
            // Expressive accent: a large circle and a soft pill, top right.
            let art = if hw > 560.0 {
                (hh * 0.32).clamp(96.0, 136.0)
            } else {
                0.0
            };
            if art > 0.0 {
                let ax = hx + hw - art - 28.0;
                ui.rounded(
                    ax - art * 0.35,
                    hy + 28.0 + art * 0.55,
                    art * 0.7,
                    art * 0.45,
                    art * 0.225,
                    m3::TERTIARY_CONTAINER,
                );
                ui.rounded(ax, hy + 28.0, art, art, art / 2.0, m3::PRIMARY_CONTAINER);
                let glyph = (art * 0.45).round();
                super::icons::draw(
                    &mut ui,
                    if installing {
                        "download"
                    } else if installed {
                        "play"
                    } else {
                        "gamepad"
                    },
                    ax + (art - glyph) / 2.0,
                    hy + 28.0 + (art - glyph) / 2.0,
                    glyph,
                    m3::ON_PRIMARY_CONTAINER,
                );
            }
            let text_w = hw - 56.0 - if art > 0.0 { art * 1.4 + 16.0 } else { 0.0 };
            ui.text(
                Face::Medium,
                12.0,
                hx + 28.0,
                hy + 28.0,
                m3::PRIMARY,
                &format!("HORIZON ANDROID {}", crate::backend::VERSION),
            );
            let headline_size = if compact { 28.0 } else { 36.0 };
            let headline = ui.fit_text(
                Face::SemiBold,
                headline_size,
                if installing {
                    "Getting your game ready"
                } else if installed {
                    "Ready to play"
                } else if text_w < 420.0 {
                    "Your Java worlds"
                } else {
                    "Your Java worlds, on Android"
                },
                text_w,
            );
            ui.text(
                Face::SemiBold,
                headline_size,
                hx + 28.0,
                hy + 48.0,
                m3::ON_SURFACE,
                &headline,
            );
            let support = ui.fit_text(
                Face::Regular,
                15.0,
                &format!(
                    "Minecraft 26.3  ·  Profile {}  ·  {}",
                    c.profile,
                    c.renderer.label()
                ),
                text_w,
            );
            let support_y = hy + 52.0 + (headline_size * 1.45).round();
            ui.text(
                Face::Regular,
                15.0,
                hx + 28.0,
                support_y,
                m3::ON_SURFACE_VARIANT,
                &support,
            );
            let list_top = support_y + 40.0;
            let list_bottom = hy + hh - if installing { 56.0 } else { 24.0 };
            if installing {
                let step = ((list_bottom - list_top) / 6.0).min(40.0);
                if step >= 26.0 {
                    for (index, stage) in InstallStage::ALL.iter().enumerate() {
                        let y = list_top + index as f32 * step;
                        if index < current {
                            super::icons::draw(
                                &mut ui,
                                "check",
                                hx + 28.0,
                                y + 2.0,
                                20.0,
                                m3::PRIMARY,
                            );
                        } else if index == current {
                            ui.rounded(hx + 31.0, y + 5.0, 14.0, 14.0, 7.0, m3::PRIMARY);
                        } else {
                            ui.border(hx + 31.0, y + 5.0, 14.0, 14.0, 7.0, m3::OUTLINE);
                        }
                        ui.text(
                            if index == current {
                                Face::Medium
                            } else {
                                Face::Regular
                            },
                            16.0,
                            hx + 60.0,
                            y,
                            if index == current {
                                m3::ON_SURFACE
                            } else {
                                m3::ON_SURFACE_VARIANT
                            },
                            stage.label(),
                        );
                    }
                } else {
                    ui.text(
                        Face::Medium,
                        18.0,
                        hx + 28.0,
                        list_top,
                        m3::ON_SURFACE,
                        progress.stage.label(),
                    );
                    ui.text(
                        Face::Regular,
                        15.0,
                        hx + 28.0,
                        list_top + 30.0,
                        m3::ON_SURFACE_VARIANT,
                        &format!("Step {} of 6", current + 1),
                    );
                }
                let detail = match progress.stage {
                    InstallStage::Download => format!(
                        "{:.1} / {:.1} MiB  ·  verified downloads",
                        progress.done as f64 / 1048576.0,
                        progress.total as f64 / 1048576.0
                    ),
                    InstallStage::Controls => format!(
                        "{} / {} control files verified",
                        progress.done, progress.total
                    ),
                    _ => "Please keep Horizon open. Downloaded files are kept for retry.".into(),
                };
                let detail = ui.fit_text(Face::Regular, 14.0, &detail, hw - 56.0);
                ui.text(
                    Face::Regular,
                    14.0,
                    hx + 28.0,
                    hy + hh - 44.0,
                    m3::ON_SURFACE_VARIANT,
                    &detail,
                );
            } else {
                let rows = if installed {
                    [
                        "Open a world or join a Java server",
                        "Touch setup: Options > Controls",
                        "Find packs and .hmod in the tabs",
                    ]
                } else {
                    [
                        "Download resources from Mojang",
                        "Install and verify on this phone",
                        "Play offline or sign in for servers",
                    ]
                };
                let step = ((list_bottom - list_top) / 3.0).min(56.0);
                if step >= 36.0 {
                    for (index, line) in rows.iter().enumerate() {
                        let y = list_top + index as f32 * step;
                        ui.rounded(hx + 28.0, y, 32.0, 32.0, 16.0, m3::SECONDARY_CONTAINER);
                        centered(
                            &mut ui,
                            Face::Medium,
                            14.0,
                            hx + 44.0,
                            label_y(y, 32.0, 14.0),
                            m3::ON_SECONDARY_CONTAINER,
                            &(index + 1).to_string(),
                        );
                        let line = ui.fit_text(Face::Regular, 16.0, line, hw - 112.0);
                        ui.text(
                            Face::Regular,
                            16.0,
                            hx + 76.0,
                            label_y(y, 32.0, 16.0),
                            m3::ON_SURFACE,
                            &line,
                        );
                    }
                }
            }
            // The side panel.
            let [sx, sy, sw, sh] = side;
            let launch_h = if wide { 96.0 } else { 76.0 };
            let card_h = if wide { 104.0 } else { 88.0 };
            let card = [sx, sy, sw, card_h];
            ui.rounded(card[0], card[1], card[2], card[3], 28.0, m3::CONTAINER_HIGH);
            let avatar = card_h - 40.0;
            ui.rounded(
                sx + 20.0,
                sy + 20.0,
                avatar,
                avatar,
                avatar / 2.0,
                if self.login.signed_in {
                    m3::PRIMARY
                } else {
                    m3::PRIMARY_CONTAINER
                },
            );
            super::icons::draw(
                &mut ui,
                "user",
                sx + 20.0 + avatar * 0.2,
                sy + 20.0 + avatar * 0.2,
                (avatar * 0.6).round(),
                if self.login.signed_in {
                    m3::ON_PRIMARY
                } else {
                    m3::ON_PRIMARY_CONTAINER
                },
            );
            let name = if c.name.trim().is_empty() {
                "Player".to_owned()
            } else {
                c.name.clone()
            };
            let text_x = sx + 36.0 + avatar;
            let name = ui.fit_text(Face::SemiBold, 18.0, &name, sx + sw - 20.0 - text_x);
            ui.text(
                Face::SemiBold,
                18.0,
                text_x,
                sy + card_h / 2.0 - 26.0,
                m3::ON_SURFACE,
                &name,
            );
            let state = ui.fit_text(Face::Regular, 13.0, &account, sx + sw - 20.0 - text_x);
            ui.text(
                Face::Regular,
                13.0,
                text_x,
                sy + card_h / 2.0 + 2.0,
                m3::ON_SURFACE_VARIANT,
                &state,
            );
            self.targets.push((card, Action::Account));
            if wide && sh >= card_h * 2.0 + launch_h + 24.0 {
                let vy = sy + card_h + 12.0;
                let version = [sx, vy, sw, card_h];
                ui.rounded(sx, vy, sw, card_h, 28.0, m3::CONTAINER_HIGH);
                icon_tile(
                    &mut ui,
                    "box",
                    sx + 20.0,
                    vy + 20.0,
                    card_h - 40.0,
                    m3::TERTIARY_CONTAINER,
                    m3::ON_TERTIARY_CONTAINER,
                );
                ui.text(
                    Face::SemiBold,
                    18.0,
                    text_x,
                    vy + card_h / 2.0 - 26.0,
                    m3::ON_SURFACE,
                    "Minecraft 26.3",
                );
                let detail = ui.fit_text(
                    Face::Regular,
                    13.0,
                    &format!(
                        "Profile {}  ·  {}%  ·  {}",
                        c.profile,
                        c.render_scale.percent(),
                        c.input_host.label()
                    ),
                    sx + sw - 20.0 - text_x,
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    text_x,
                    vy + card_h / 2.0 + 2.0,
                    m3::ON_SURFACE_VARIANT,
                    &detail,
                );
                self.targets.push((version, Action::Profile));
            }
            let launch = [sx, sy + sh - launch_h, sw, launch_h];
            let [lx, ly, lw, lh] = launch;
            if installing {
                ui.rounded(lx, ly, lw, lh, 28.0, m3::CONTAINER_HIGH);
                let label = ui.fit_text(Face::Medium, 16.0, progress.stage.label(), lw - 120.0);
                ui.text(
                    Face::Medium,
                    16.0,
                    lx + 24.0,
                    ly + lh / 2.0 - 26.0,
                    m3::ON_SURFACE,
                    &label,
                );
                let percent = format!("{}%", (overall * 100.0).round() as u32);
                let pw = ui.measure(Face::Medium, 16.0, &percent);
                ui.text(
                    Face::Medium,
                    16.0,
                    lx + lw - 24.0 - pw,
                    ly + lh / 2.0 - 26.0,
                    m3::PRIMARY,
                    &percent,
                );
                let bar_y = ly + lh / 2.0 + 6.0;
                let track = lw - 48.0;
                ui.rounded(lx + 24.0, bar_y, track, 10.0, 5.0, m3::SECONDARY_CONTAINER);
                if overall > 0.0 {
                    ui.rounded(
                        lx + 24.0,
                        bar_y,
                        (track * overall).max(10.0),
                        10.0,
                        5.0,
                        m3::PRIMARY,
                    );
                }
            } else {
                ui.rounded(lx, ly, lw, lh, 28.0, m3::PRIMARY);
                let label = if installed {
                    "Play"
                } else if self.error.is_some() {
                    "Retry installation"
                } else if lw < 300.0 {
                    "Download 26.3"
                } else {
                    "Download Minecraft 26.3"
                };
                let size = if installed { 26.0 } else { 20.0 };
                let icon = if installed {
                    "play"
                } else if self.error.is_some() {
                    "reload"
                } else {
                    "download"
                };
                let label = ui.fit_text(Face::SemiBold, size, label, lw - 96.0);
                let content = 32.0 + 12.0 + ui.measure(Face::SemiBold, size, &label);
                let cx = lx + (lw - content) / 2.0;
                super::icons::draw(
                    &mut ui,
                    icon,
                    cx,
                    ly + (lh - 32.0) / 2.0,
                    32.0,
                    m3::ON_PRIMARY,
                );
                ui.text(
                    Face::SemiBold,
                    size,
                    cx + 44.0,
                    label_y(ly, lh, size),
                    m3::ON_PRIMARY,
                    &label,
                );
                self.targets.push((launch, Action::Play));
            }
        } else if page.is_catalog() {
            use crate::shader_catalog::Origin;
            // Search field and its action.
            let action_w = if compact { 112.0 } else { 150.0 };
            let field = [l, t, r - l - action_w - 12.0, 52.0];
            ui.rounded(
                field[0],
                field[1],
                field[2],
                field[3],
                26.0,
                m3::CONTAINER_HIGHEST,
            );
            super::icons::draw(
                &mut ui,
                "search",
                l + 18.0,
                t + 14.0,
                24.0,
                m3::ON_SURFACE_VARIANT,
            );
            let editing_search = self.catalog.editing == Some("search");
            let (value, value_color) = if !self.catalog.query.is_empty() || editing_search {
                (self.catalog.query.clone(), m3::ON_SURFACE)
            } else if page == Page::Mods {
                (
                    "Search mods or paste an .hmod URL".to_owned(),
                    m3::ON_SURFACE_VARIANT,
                )
            } else {
                ("Search".to_owned(), m3::ON_SURFACE_VARIANT)
            };
            let room = field[2] - 74.0;
            let value = ui.fit_text(Face::Regular, 16.0, &value, room);
            ui.text(
                Face::Regular,
                16.0,
                l + 56.0,
                label_y(t, 52.0, 16.0),
                value_color,
                &value,
            );
            if editing_search {
                caret = Some([l + 56.0, label_y(t, 52.0, 16.0), room, 16.0]);
                ui.border(field[0], field[1], field[2], field[3], 26.0, m3::PRIMARY);
            }
            self.targets.push((field, Action::Field("search")));
            let import_url =
                page == Page::Mods && self.catalog.query.trim().starts_with("https://");
            pill(
                &mut ui,
                &mut self.targets,
                [r - action_w, t, action_w, 52.0],
                Some(if import_url { "download" } else { "search" }),
                if import_url { "Import" } else { "Search" },
                Action::Search,
                Style::Filled,
            );
            // Source segments and file actions.
            let row = t + 64.0;
            let file_w = if compact {
                ((r - l) * 0.27).min(120.0)
            } else {
                150.0
            };
            let file_kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            if matches!(page, Page::Shaders | Page::Mods) {
                let selected = if page == Page::Mods {
                    self.catalog.config.mod_source
                } else {
                    self.catalog.config.shader_source
                };
                let segments = (r - l - 2.0 * file_w - 24.0).min(300.0);
                let half = segments / 2.0;
                for (i, origin) in [Origin::Horizon, Origin::Modrinth].into_iter().enumerate() {
                    let x = l + i as f32 * half;
                    let on = selected == origin;
                    let rect = [x, row, half, 40.0];
                    ui.rounded(
                        x,
                        row,
                        half,
                        40.0,
                        20.0,
                        if on { m3::SECONDARY_CONTAINER } else { 0 },
                    );
                    if !on {
                        ui.border(x, row, half, 40.0, 20.0, m3::OUTLINE);
                    }
                    let label = origin.source().label();
                    let icon_room = if on { 26.0 } else { 0.0 };
                    let text = ui.fit_text(Face::Medium, 14.0, label, half - 20.0 - icon_room);
                    let content = ui.measure(Face::Medium, 14.0, &text) + icon_room;
                    let cx = x + (half - content) / 2.0;
                    let fg = if on {
                        m3::ON_SECONDARY_CONTAINER
                    } else {
                        m3::ON_SURFACE
                    };
                    if on {
                        super::icons::draw(&mut ui, "check", cx, row + 11.0, 18.0, fg);
                    }
                    ui.text(
                        Face::Medium,
                        14.0,
                        cx + icon_room,
                        label_y(row, 40.0, 14.0),
                        fg,
                        &text,
                    );
                    self.targets.push((
                        rect,
                        if page == Page::Mods {
                            Action::ModSource(origin)
                        } else {
                            Action::ShaderSource(origin)
                        },
                    ));
                }
            }
            pill(
                &mut ui,
                &mut self.targets,
                [r - 2.0 * file_w - 12.0, row, file_w, 40.0],
                Some("folder"),
                if compact { "Folder" } else { "Open folder" },
                Action::OpenFolder(file_kind),
                Style::Tonal,
            );
            pill(
                &mut ui,
                &mut self.targets,
                [r - file_w, row, file_w, 40.0],
                Some("plus"),
                if compact { "Import" } else { "Import file" },
                Action::Import(file_kind),
                Style::Tonal,
            );
            let hint = match page {
                Page::Mods if self.catalog.config.mod_source == Origin::Modrinth => {
                    "Modrinth Java mods are browse-only; Fabric/Forge/NeoForge cannot run here"
                }
                Page::Mods => "Search Horizon mods or paste an HTTPS .hmod link",
                Page::Shaders if self.catalog.config.shader_source == Origin::Modrinth => {
                    "Modrinth shader ZIPs / experimental compatibility; some packs may not work"
                }
                Page::Shaders => "Horizon native .hshader packs",
                _ => "Verified downloads / enable in Installed",
            };
            let hint = ui.fit_text(Face::Regular, 14.0, hint, r - l);
            ui.text(
                Face::Regular,
                14.0,
                l,
                row + 52.0,
                m3::ON_SURFACE_VARIANT,
                &hint,
            );
            // Results.
            let top = row + 84.0;
            let bottom = b - 60.0;
            const STRIDE: f32 = 92.0;
            let maximum =
                (self.catalog.results.len() as f32 * STRIDE - 8.0 - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            for (i, p) in self.catalog.results.iter().enumerate() {
                let y = top + i as f32 * STRIDE - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                let install = [r - 16.0 - 124.0, y + 22.0, 124.0, 40.0];
                if download.is_none()
                    && !crate::catalog::source_only_mod(p)
                    && !(p.kind == crate::browser::Kind::Mod
                        && p.source == crate::browser::SourceId::Modrinth)
                {
                    self.list_targets.push((install, Action::Install(i)));
                }
                if y < top || y + 84.0 > bottom {
                    continue;
                }
                ui.rounded(l, y, r - l, 84.0, 20.0, m3::CONTAINER);
                icon_tile(
                    &mut ui,
                    page_icon(page),
                    l + 16.0,
                    y + 16.0,
                    52.0,
                    m3::PRIMARY_CONTAINER,
                    m3::ON_PRIMARY_CONTAINER,
                );
                let text_room = r - l - 84.0 - 156.0;
                let title = ui.fit_text(Face::Medium, 17.0, &p.title, text_room);
                let summary = ui.fit_text(Face::Regular, 14.0, &p.summary, text_room);
                ui.text(
                    Face::Medium,
                    17.0,
                    l + 84.0,
                    y + 16.0,
                    m3::ON_SURFACE,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    14.0,
                    l + 84.0,
                    y + 44.0,
                    m3::ON_SURFACE_VARIANT,
                    &summary,
                );
                let note = |ui: &mut Ui, text: &str, color: u32| {
                    let text = ui.fit_text(Face::Medium, 14.0, text, 124.0);
                    ui.text(
                        Face::Medium,
                        14.0,
                        install[0],
                        label_y(install[1], 40.0, 14.0),
                        color,
                        &text,
                    );
                };
                if let Some(state) = download {
                    note(&mut ui, &state, m3::PRIMARY);
                } else if p.kind == crate::browser::Kind::Mod
                    && p.source == crate::browser::SourceId::Modrinth
                {
                    note(&mut ui, "Java mod", m3::ON_SURFACE_VARIANT);
                } else if crate::catalog::source_only_mod(p) {
                    note(&mut ui, ".hnmod: later", m3::ON_SURFACE_VARIANT);
                } else if p.kind == crate::browser::Kind::Mod && !cfg!(target_pointer_width = "64")
                {
                    note(&mut ui, "64-bit required", m3::ON_SURFACE_VARIANT);
                } else {
                    pill(
                        &mut ui,
                        &mut self.targets,
                        install,
                        Some("download"),
                        self.catalog.install_button(p),
                        Action::Install(i),
                        Style::Tonal,
                    );
                }
            }
            if self.catalog.results.is_empty() {
                centered(
                    &mut ui,
                    Face::Regular,
                    15.0,
                    (l + r) / 2.0,
                    top + 40.0,
                    m3::ON_SURFACE_VARIANT,
                    "Nothing here yet. Search or change the source.",
                );
            }
            pill(
                &mut ui,
                &mut self.targets,
                [l, b - 48.0, 132.0, 48.0],
                Some("arrow-left"),
                "Previous",
                Action::Previous,
                Style::Outlined,
            );
            pill(
                &mut ui,
                &mut self.targets,
                [l + 144.0, b - 48.0, 132.0, 48.0],
                Some("chevron-right"),
                "Next",
                Action::Next,
                Style::Outlined,
            );
            let page_label = format!("Page {}", self.catalog.offset / 12 + 1);
            let pw = ui.measure(Face::Medium, 14.0, &page_label);
            if l + 290.0 + pw < r {
                ui.text(
                    Face::Medium,
                    14.0,
                    r - pw,
                    label_y(b - 48.0, 48.0, 14.0),
                    m3::ON_SURFACE_VARIANT,
                    &page_label,
                );
            }
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            let top = t;
            let bottom = b;
            let row_height = if compact { 116.0 } else { 72.0 };
            let total = Kind::ALL
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    64.0 + if self.catalog.sections[i] {
                        self.catalog.local.iter().filter(|e| e.kind == *k).count() as f32
                            * (row_height + 8.0)
                    } else {
                        0.0
                    }
                })
                .sum::<f32>();
            let maximum = (total - 8.0 - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            let mut offset = 0.0;
            for (section, kind) in Kind::ALL.into_iter().enumerate() {
                let count = self.catalog.local.iter().filter(|e| e.kind == kind).count();
                let open = self.catalog.sections[section];
                let y = top + offset - self.catalog.scroll;
                let header = [l, y, r - l - 156.0, 56.0];
                let import = [r - 12.0 - 132.0, y + 8.0, 132.0, 40.0];
                self.list_targets
                    .push((header, Action::ToggleSection(section)));
                self.list_targets.push((import, Action::Import(kind)));
                if y >= top && y + 56.0 <= bottom {
                    ui.rounded(l, y, r - l, 56.0, 28.0, m3::CONTAINER_HIGH);
                    super::icons::draw(
                        &mut ui,
                        if open {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        l + 16.0,
                        y + 16.0,
                        24.0,
                        m3::ON_SURFACE_VARIANT,
                    );
                    let label = kind.label();
                    ui.text(
                        Face::Medium,
                        17.0,
                        l + 52.0,
                        label_y(y, 56.0, 17.0),
                        m3::ON_SURFACE,
                        label,
                    );
                    let badge_x = l + 64.0 + ui.measure(Face::Medium, 17.0, label);
                    let number = count.to_string();
                    let badge_w = ui.measure(Face::Medium, 12.0, &number) + 16.0;
                    ui.rounded(
                        badge_x,
                        y + 18.0,
                        badge_w.max(24.0),
                        20.0,
                        10.0,
                        m3::PRIMARY_CONTAINER,
                    );
                    centered(
                        &mut ui,
                        Face::Medium,
                        12.0,
                        badge_x + badge_w.max(24.0) / 2.0,
                        label_y(y + 18.0, 20.0, 12.0),
                        m3::ON_PRIMARY_CONTAINER,
                        &number,
                    );
                    self.targets.push((header, Action::ToggleSection(section)));
                    pill(
                        &mut ui,
                        &mut self.targets,
                        import,
                        Some("plus"),
                        "Import",
                        Action::Import(kind),
                        Style::Tonal,
                    );
                }
                offset += 64.0;
                if !open {
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
                    offset += row_height + 8.0;
                    let control_y = y + if compact { 64.0 } else { 16.0 };
                    let switch_x = if compact {
                        l + 20.0
                    } else {
                        r - 16.0 - 132.0 - 24.0 - 100.0
                    };
                    let toggle_rect = [switch_x, control_y, 100.0, 40.0];
                    let remove = [r - 16.0 - 132.0, control_y, 132.0, 40.0];
                    self.list_targets
                        .push((toggle_rect, Action::ToggleLocal(i)));
                    self.list_targets.push((remove, Action::DeleteLocal(i)));
                    if y < top || y + row_height > bottom {
                        continue;
                    }
                    ui.rounded(l, y, r - l, row_height, 20.0, m3::CONTAINER);
                    let room = if compact {
                        r - l - 40.0
                    } else {
                        switch_x - l - 36.0
                    };
                    let title = ui.fit_text(Face::Medium, 17.0, &item.title, room);
                    ui.text(
                        Face::Medium,
                        17.0,
                        l + 20.0,
                        y + 12.0,
                        m3::ON_SURFACE,
                        &title,
                    );
                    ui.text(
                        Face::Regular,
                        13.0,
                        l + 20.0,
                        y + 40.0,
                        m3::ON_SURFACE_VARIANT,
                        kind.label(),
                    );
                    if kind == Kind::Mods && !cfg!(target_pointer_width = "64") {
                        ui.text(
                            Face::Regular,
                            14.0,
                            switch_x,
                            label_y(control_y, 40.0, 14.0),
                            m3::ON_SURFACE_VARIANT,
                            "64-bit required",
                        );
                    } else {
                        switch(&mut ui, switch_x, control_y + 4.0, item.enabled);
                        ui.text(
                            Face::Medium,
                            14.0,
                            switch_x + 62.0,
                            label_y(control_y, 40.0, 14.0),
                            m3::ON_SURFACE_VARIANT,
                            if item.enabled { "On" } else { "Off" },
                        );
                        self.targets.push((toggle_rect, Action::ToggleLocal(i)));
                    }
                    pill(
                        &mut ui,
                        &mut self.targets,
                        remove,
                        Some("trash"),
                        "Remove",
                        Action::DeleteLocal(i),
                        Style::Outlined,
                    );
                }
            }
        } else {
            let c = &self.catalog.config;
            let rows = [
                (
                    "Rendering",
                    c.renderer.label().to_owned(),
                    Action::Renderer,
                    "monitor",
                ),
                (
                    "Client resolution",
                    if self.catalog.editing == Some("render_scale") {
                        self.catalog.render_scale_text.clone()
                    } else {
                        format!("{}%", c.render_scale.percent())
                    },
                    Action::RenderScale,
                    "image",
                ),
                (
                    "Profile",
                    format!("Profile {}", c.profile),
                    Action::Profile,
                    "user",
                ),
                (
                    "Orientation",
                    c.orientation.label().to_owned(),
                    Action::Orientation,
                    "reload",
                ),
                (
                    "Input",
                    c.input_host.label().to_owned(),
                    Action::InputHost,
                    "gamepad",
                ),
                (
                    "Immersive fullscreen",
                    if c.screen.immersive { "On" } else { "Off" }.into(),
                    Action::Immersive,
                    "monitor",
                ),
                (
                    "Avoid camera cutout",
                    if c.screen.avoid_cutout { "On" } else { "Off" }.into(),
                    Action::Cutout,
                    "box",
                ),
                (
                    "Aspect ratio",
                    c.screen.aspect.label().into(),
                    Action::Aspect,
                    "image",
                ),
                (
                    "Display refresh rate",
                    c.device.display_label(),
                    Action::DisplayHz,
                    "clock",
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
                    "zap",
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
                    "bug",
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
                    "clock",
                ),
                (
                    "Diagnostic report",
                    "Export one ZIP to share".into(),
                    Action::ExportDiagnostics,
                    "archive",
                ),
                (
                    "Raw performance data",
                    "Advanced / JSONL in Reports".into(),
                    Action::ExportPerformance,
                    "notes",
                ),
                (
                    "Server",
                    if c.server.is_empty() && self.catalog.editing != Some("server") {
                        "Singleplayer / tap to enter server".into()
                    } else {
                        c.server.clone()
                    },
                    Action::Field("server"),
                    "server",
                ),
            ];
            const ROW: f32 = 72.0;
            const GAP: f32 = 6.0;
            let top = t;
            let bottom = b - 64.0;
            let maximum = (rows.len() as f32 * (ROW + GAP) - GAP - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            let count = rows.len();
            for (i, (title, value, action, icon)) in rows.into_iter().enumerate() {
                let y = top + i as f32 * (ROW + GAP) - self.catalog.scroll;
                let rect = [l, y, r - l, ROW];
                self.list_targets.push((rect, action));
                if y < top || y + ROW > bottom {
                    continue;
                }
                // First and last rows of the group take the larger corner.
                let radius = if i == 0 || i + 1 == count { 24.0 } else { 12.0 };
                ui.rounded(l, y, r - l, ROW, radius, m3::CONTAINER);
                icon_tile(
                    &mut ui,
                    icon,
                    l + 16.0,
                    y + 16.0,
                    40.0,
                    m3::SECONDARY_CONTAINER,
                    m3::ON_SECONDARY_CONTAINER,
                );
                let is_switch = matches!(
                    action,
                    Action::PerformanceHints
                        | Action::Immersive
                        | Action::Cutout
                        | Action::GpuProfile
                        | Action::RecordPerformance
                );
                let room = r - l - 72.0 - 88.0;
                let title = ui.fit_text(Face::Medium, 16.0, title, room);
                ui.text(
                    Face::Medium,
                    16.0,
                    l + 72.0,
                    y + 12.0,
                    m3::ON_SURFACE,
                    &title,
                );
                let editing = (action == Action::Field("server")
                    && self.catalog.editing == Some("server"))
                    || (action == Action::RenderScale
                        && self.catalog.editing == Some("render_scale"));
                let shown = ui.fit_text(Face::Regular, 14.0, &value, room);
                ui.text(
                    Face::Regular,
                    14.0,
                    l + 72.0,
                    y + 38.0,
                    if editing {
                        m3::PRIMARY
                    } else {
                        m3::ON_SURFACE_VARIANT
                    },
                    &shown,
                );
                if editing {
                    caret = Some([l + 72.0, y + 38.0, room, 14.0]);
                    ui.border(l, y, r - l, ROW, radius, m3::PRIMARY);
                }
                if is_switch {
                    switch(
                        &mut ui,
                        r - 16.0 - 52.0,
                        y + 20.0,
                        match action {
                            Action::PerformanceHints => c.device.performance_hints,
                            Action::Immersive => c.screen.immersive,
                            Action::Cutout => c.screen.avoid_cutout,
                            Action::GpuProfile => c.device.gpu_profile,
                            _ => c.record_performance,
                        },
                    );
                } else {
                    super::icons::draw(
                        &mut ui,
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
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        r - 16.0 - 24.0,
                        y + 24.0,
                        24.0,
                        m3::ON_SURFACE_VARIANT,
                    );
                }
                self.targets.push((rect, action));
            }
            let half = (r - l - 12.0) / 2.0;
            pill(
                &mut ui,
                &mut self.targets,
                [l, b - 52.0, half, 52.0],
                Some("bug"),
                "Diagnostics",
                Action::Logs,
                Style::Tonal,
            );
            pill(
                &mut ui,
                &mut self.targets,
                [l + half + 12.0, b - 52.0, half, 52.0],
                Some("user"),
                if compact {
                    "Microsoft account"
                } else {
                    "Microsoft / Xbox account"
                },
                Action::Account,
                Style::Filled,
            );
        }
        // Status, as a snackbar-like strip.
        let status = self.error.as_deref().unwrap_or(&self.catalog.status);
        if !status.trim().is_empty() {
            let error = self.error.is_some();
            let strip_w = r - l;
            ui.rounded(
                l,
                status_y,
                strip_w,
                44.0,
                14.0,
                if error {
                    m3::ERROR_CONTAINER
                } else {
                    m3::CONTAINER_HIGH
                },
            );
            super::icons::draw(
                &mut ui,
                "info-box",
                l + 12.0,
                status_y + 12.0,
                20.0,
                if error { m3::ERROR } else { m3::PRIMARY },
            );
            let room = strip_w - 56.0;
            let per_line = ((room / 6.6) as usize).max(16);
            let chars: Vec<char> = status.chars().collect();
            let lines: Vec<String> = chars
                .chunks(per_line)
                .take(2)
                .map(|line| line.iter().collect())
                .collect();
            let size = 13.0;
            let line_h = (size * 1.45f32).round();
            let first = status_y + (44.0 - line_h * lines.len() as f32) / 2.0;
            for (i, line) in lines.iter().enumerate() {
                let line = ui.fit_text(Face::Regular, size, line, room);
                ui.text(
                    Face::Regular,
                    size,
                    l + 44.0,
                    first + i as f32 * line_h,
                    if error {
                        m3::ON_SURFACE
                    } else {
                        m3::ON_SURFACE_VARIANT
                    },
                    &line,
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
            caret = None;
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
            caret = None;
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
            let x = if self.chooser.is_some() {
                w - 12.0
            } else {
                r + 8.0
            };
            ui.rounded(x, top, 4.0, height, 2.0, m3::OUTLINE_VARIANT);
            ui.rounded(x, y, 4.0, thumb, 2.0, m3::PRIMARY);
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
            // Fields drawn by the dialogs: the value sits where `pill` puts a field's text.
            let anchor = caret.or_else(|| {
                self.targets
                    .iter()
                    .find(|(_, action)| *action == Action::Field(name))
                    .map(|([x, y, w, h], _)| {
                        let size = pill_text_size(*w, *h);
                        [x + 16.0, label_y(*y, *h, size), w - 32.0, size]
                    })
            });
            if let Some([x, y, room, size]) = anchor {
                // Pages draw values in Regular; dialog fields come from `pill` (Medium).
                let face = if caret.is_some() {
                    Face::Regular
                } else {
                    Face::Medium
                };
                let text = ui.fit_text(face, size, &value, room);
                let caret_x = x + ui.measure(face, size, &text);
                ui.rounded(
                    caret_x.min(x + room),
                    y + 2.0,
                    2.0,
                    (size * 1.45).round() - 4.0,
                    1.0,
                    m3::PRIMARY,
                );
            }
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            let radius = (h / 2.0).min(24.0);
            ui.border(
                *x - 3.0,
                *y - 3.0,
                *w + 6.0,
                *h + 6.0,
                radius + 3.0,
                m3::PRIMARY,
            );
            ui.border(
                *x - 2.0,
                *y - 2.0,
                *w + 4.0,
                *h + 4.0,
                radius + 2.0,
                m3::PRIMARY,
            );
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
                ui.rounded(*x, *y, *w, *h, (h / 2.0).min(24.0), m3::STATE_LAYER);
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(160)
            && let Some(([x, y, w, h], _)) = self.targets.iter().find(|(_, a)| *a == action)
        {
            ui.rounded(*x, *y, *w, *h, (h / 2.0).min(24.0), m3::STATE_LAYER);
        }
        crate::documents::draw_ui(&mut ui);
        ui.finish();

        if !renderer.render(&game::loading_frame(&batch))? && !batch.patches.is_empty() {
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        Ok(())
    }
    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() {
            return Ok(());
        }
        let Some(r) = &self.renderer else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&self.gui.as_ref().unwrap().atlas, &mut b, (0.0, 0.0));
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
        if self.catalog.busy() || crate::documents::busy() {
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
            Some(Action::DeviceSignIn) => self
                .login
                .start_device(&self.catalog.config.microsoft_client_id),
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => self.login.start(&self.catalog.config.microsoft_client_id),
            Some(Action::SignOut) => self.login.sign_out(),
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
            Some(Action::Dismiss) => self.chooser = None,
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
            if self.login.visible {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
                return;
            }
            if self.chooser.take().is_some() {
                return;
            }
            crate::platform::hide();
            self.catalog.editing = None;
            let _ = self.catalog.config.save(&self.home);
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
