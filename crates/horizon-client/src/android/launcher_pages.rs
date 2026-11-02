use super::*;
use crate::mc_ui::{self, Kind, color};
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

/// Is `point` (dp) inside `rect`?
fn hit(rect: [f32; 4], point: [f32; 2]) -> bool {
    point[0] >= rect[0]
        && point[0] <= rect[0] + rect[2]
        && point[1] >= rect[1]
        && point[1] <= rect[1] + rect[3]
}

/// The bevel kind an action's button wears: green for the primary action,
/// red for destructive confirmations, dark plates for tabs and quiet links,
/// stone for everything else.
fn button_kind(action: Action, accent: bool) -> Kind {
    if accent {
        return Kind::Green;
    }
    match action {
        Action::Page(_)
        | Action::Repository
        | Action::ShaderSource(_)
        | Action::ModSource(_)
        | Action::ToggleSection(_) => Kind::Dark,
        Action::DeleteLocal(_) | Action::ConfirmDeleteLocal => Kind::Red,
        _ => Kind::Stone,
    }
}

/// The pixel-face label size for a button rect: bigger for the tall Play
/// button, smaller for narrow ones.
fn button_font(rect: [f32; 4]) -> f32 {
    let [_, _, w, h] = rect;
    if h >= 56.0 {
        20.0
    } else if w < 80.0 {
        13.0
    } else {
        16.0
    }
}

/// Draws a widget for `action` and registers its tap target. `press` is the
/// rect being held (or tapped in the last 160 ms), so the widget can draw
/// vanilla's pressed bevel in the same pass. Field actions draw an edit box
/// instead of a button.
pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
    press: Option<[f32; 4]>,
) {
    if matches!(action, Action::Field(_)) {
        mc_ui::field(ui, rect, label, 15.0, false);
        targets.push((rect, action));
        return;
    }
    let kind = button_kind(action, accent);
    let pressed = press == Some(rect);
    let base = button_font(rect);
    let font = if matches!(action, Action::Page(_)) {
        let [_, _, w, _] = rect;
        (base * ((w - 16.0) / ui.measure(Face::Pixel, base, label).max(1.0)).min(1.0)).max(9.0)
    } else {
        base
    };
    mc_ui::button(ui, rect, label, kind, pressed, font);
    targets.push((rect, action));
}
fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
    press: Option<[f32; 4]>,
) {
    // Vanilla's ON/OFF button: a green plate when on, stone when off.
    mc_ui::button(
        ui,
        rect,
        if on { "ON" } else { "OFF" },
        if on { Kind::Green } else { Kind::Stone },
        press == Some(rect),
        14.0,
    );
    targets.push((rect, action));
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
        // The previous frame's targets: the layout only moves when a drag
        // scrolls it, so hit-testing last frame's rects finds the widget under
        // a held finger and lets it draw vanilla's pressed bevel in one pass.
        let previous = std::mem::take(&mut self.targets);
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
        let point = [
            self.cursor.0 as f32 / ui.density,
            self.cursor.1 as f32 / ui.density,
        ];
        let holding = self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved);
        let press = if holding {
            previous
                .iter()
                .rev()
                .find(|(rect, _)| hit(*rect, point))
                .map(|(rect, _)| *rect)
        } else {
            self.feedback
                .filter(|(_, time)| time.elapsed() < std::time::Duration::from_millis(160))
                .and_then(|(action, _)| {
                    previous
                        .iter()
                        .find(|(_, a)| *a == action)
                        .map(|(rect, _)| *rect)
                })
        };
        // Hand the allocation back: the frame rebuilds the same list.
        let mut previous = previous;
        previous.clear();
        self.targets = previous;
        let scale = ui.density / ui.batch.scale;
        ui.batch
            .fill(&gui.atlas, 0.0, 0.0, w * scale, h * scale, color::PAGE);
        let title_size = if w < 400.0 { 18.0 } else if w < 700.0 { 22.0 } else { 28.0 };
        let title_y = if w < 700.0 { 12.0 } else { 28.0 };
        mc_ui::grass_mark(&mut ui, 32.0, title_y + 1.0, title_size);
        mc_ui::shadow_text(
            &mut ui,
            Face::Pixel,
            title_size,
            32.0 + title_size + 10.0,
            title_y,
            color::TEXT,
            &format!("Oxide {}", crate::backend::VERSION),
        );
        button(
            &mut ui,
            &mut self.targets,
            [
                if w < 700.0 { 32.0 } else { w - 320.0 },
                if w < 700.0 { 47.0 } else { 24.0 },
                if w < 700.0 { w - 64.0 } else { 288.0 },
                if w < 700.0 { 34.0 } else { 46.0 },
            ],
            &self.login.status(&self.catalog.config.name),
            Action::Account,
            self.login.signed_in,
            press,
        );
        let tabs = Page::ALL.len() as f32;
        let tab_margin = if w < 700.0 { 16.0 } else { 32.0 };
        let tab_width = (w - 2.0 * tab_margin) / tabs;
        for (i, page) in Page::ALL.iter().enumerate() {
            let x = tab_margin + i as f32 * tab_width;
            button(
                &mut ui,
                &mut self.targets,
                [x, 88.0, tab_width - 4.0, 42.0],
                if w < 700.0 {
                    match page {
                        Page::Installed => "Files",
                        Page::Settings => "Setup",
                        _ => page.name(),
                    }
                } else {
                    page.name()
                },
                Action::Page(*page),
                *page == self.catalog.page,
                press,
            );
        }
        let compact = w < 700.0;
        let page = self.catalog.page;
        if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self.install.lock().unwrap();
            let top = 160.0;
            mc_ui::shadow_text(
                &mut ui,
                Face::Pixel,
                28.0,
                32.0,
                top,
                color::TEXT,
                if installing {
                    "Getting your game ready"
                } else if installed {
                    "Ready to play"
                } else if compact {
                    "Your Java worlds"
                } else {
                    "Your Java worlds, on Android"
                },
            );
            ui.text(
                Face::Regular,
                17.0,
                32.0,
                top + 47.0,
                color::MUTED,
                &format!("Minecraft 26.3  /  Profile {}", self.catalog.config.profile),
            );
            if installing {
                let current = InstallStage::ALL
                    .iter()
                    .position(|s| *s == progress.stage)
                    .unwrap_or(0);
                let row_height = ((h - 360.0) / 6.0).clamp(25.0, 40.0);
                if h >= 620.0 {
                    for (index, stage) in InstallStage::ALL.iter().enumerate() {
                        let y = top + 90.0 + index as f32 * row_height;
                        // The stage number in the pixel face: green once done,
                        // white while running, dim ahead.
                        let number = format!("{:02}", index + 1);
                        ui.text(
                            Face::Pixel,
                            15.0,
                            32.0,
                            y,
                            if index < current {
                                color::GREEN_TEXT
                            } else if index == current {
                                color::TEXT
                            } else {
                                color::FAINT
                            },
                            &number,
                        );
                        ui.text(
                            Face::Regular,
                            16.0,
                            66.0,
                            y + 1.0,
                            if index == current {
                                color::TEXT
                            } else if index < current {
                                color::MUTED
                            } else {
                                color::FAINT
                            },
                            stage.label(),
                        );
                    }
                } else {
                    ui.text(
                        Face::Pixel,
                        17.0,
                        32.0,
                        top + 100.0,
                        color::TEXT,
                        progress.stage.label(),
                    );
                    ui.text(
                        Face::Regular,
                        15.0,
                        32.0,
                        top + 138.0,
                        color::MUTED,
                        &format!("Step {} of 6", current + 1),
                    );
                }
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                let overall = (current as f32 + fraction.clamp(0.0, 1.0)) / 6.0;
                let bar_y = h - 156.0;
                mc_ui::progress(&mut ui, [32.0, bar_y, w - 64.0, 12.0], overall);
                let detail = match progress.stage {
                    InstallStage::Download => format!(
                        "{:.1} / {:.1} MiB  - verified downloads",
                        progress.done as f64 / 1048576.0,
                        progress.total as f64 / 1048576.0
                    ),
                    InstallStage::Controls => format!(
                        "{} / {} control files verified",
                        progress.done, progress.total
                    ),
                    _ => "Please keep Oxide open. Downloaded files are kept for retry.".into(),
                };
                ui.text(
                    Face::Regular,
                    14.0,
                    32.0,
                    bar_y + 20.0,
                    color::MUTED,
                    &detail,
                );
            } else {
                let rows = if installed {
                    [
                        ("01", "Open a world or join a Java server"),
                        ("02", "Touch setup: Options > Controls"),
                        ("03", "Find packs and .hmod in the tabs"),
                    ]
                } else {
                    [
                        ("01", "Download resources from Mojang"),
                        ("02", "Install and verify on this phone"),
                        ("03", "Play offline or sign in for servers"),
                    ]
                };
                for (index, (number, line)) in rows.iter().enumerate() {
                    let y = top + 100.0 + index as f32 * 42.0;
                    ui.text(Face::Pixel, 16.0, 32.0, y, color::GREEN_TEXT, number);
                    ui.text(Face::Regular, 16.0, 70.0, y + 1.0, color::MUTED, line);
                }
            }
            if installing {
                mc_ui::shadow_text(
                    &mut ui,
                    Face::Pixel,
                    18.0,
                    32.0,
                    h - 92.0,
                    color::TEXT,
                    progress.stage.label(),
                );
            } else {
                button(
                    &mut ui,
                    &mut self.targets,
                    [32.0, h - 100.0, 320.0, 60.0],
                    if installed {
                        "Play"
                    } else if self.error.is_some() {
                        "Retry installation"
                    } else {
                        "Download Minecraft 26.3"
                    },
                    Action::Play,
                    true,
                    press,
                );
            }
        } else if page.is_catalog() {
            mc_ui::shadow_text(
                &mut ui,
                Face::Pixel,
                20.0,
                32.0,
                142.0,
                color::TEXT,
                if page == Page::Mods {
                    "Horizon Mods"
                } else if page == Page::Shaders {
                    if w < 360.0 { "Shaders" } else { "Shader Packs" }
                } else {
                    "Resource Packs"
                },
            );
            button(
                &mut ui,
                &mut self.targets,
                [32.0, 174.0, w - 240.0, 44.0],
                &format!(
                    "{}{}",
                    if page == Page::Mods {
                        "Search / .hmod URL: "
                    } else {
                        "Search: "
                    },
                    self.catalog.query
                ),
                Action::Field("search"),
                false,
                press,
            );
            button(
                &mut ui,
                &mut self.targets,
                [w - 190.0, 174.0, 158.0, 44.0],
                if page == Page::Mods && self.catalog.query.trim().starts_with("https://") {
                    "Import .hmod"
                } else {
                    "Search"
                },
                Action::Search,
                true,
                press,
            );
            if matches!(page, Page::Shaders | Page::Mods) {
                use crate::shader_catalog::Origin;
                let origins: &[Origin] = if page == Page::Mods {
                    &[Origin::Horizon, Origin::Modrinth, Origin::Oxide]
                } else {
                    &[Origin::Horizon, Origin::Modrinth]
                };
                let width = ((w - 185.0) / origins.len() as f32).clamp(48.0, 120.0);
                for (i, origin) in origins.iter().enumerate() {
                    let rect = [
                        w - 32.0 - (origins.len() - i) as f32 * width,
                        134.0,
                        width - 5.0,
                        32.0,
                    ];
                    button(
                        &mut ui,
                        &mut self.targets,
                        rect,
                        origin.label(),
                        if page == Page::Mods {
                            Action::ModSource(*origin)
                        } else {
                            Action::ShaderSource(*origin)
                        },
                        false,
                        press,
                    );
                    if (if page == Page::Mods {
                        self.catalog.config.mod_source
                    } else {
                        self.catalog.config.shader_source
                    }) == *origin
                    {
                        // The chosen source wears a green frame, like a
                        // selected inventory slot.
                        ui.border(
                            rect[0] - 2.0,
                            rect[1] - 2.0,
                            rect[2] + 4.0,
                            rect[3] + 4.0,
                            0.0,
                            color::GREEN_FACE,
                        );
                    }
                }
            }
            let hint = match page {
                Page::Mods
                    if self.catalog.config.mod_source
                        == crate::shader_catalog::Origin::Modrinth =>
                {
                    "Modrinth Java mods are browse-only; Fabric/Forge/NeoForge cannot run here"
                }
                Page::Mods
                    if self.catalog.config.mod_source
                        == crate::shader_catalog::Origin::Oxide =>
                {
                    "Oxide first-party mods: Copper, the mobile performance suite"
                }
                Page::Mods => "Search Horizon mods or paste an HTTPS .hmod link",
                Page::Shaders
                    if self.catalog.config.shader_source
                        == crate::shader_catalog::Origin::Modrinth =>
                {
                    "Modrinth shader ZIPs / experimental compatibility; some packs may not work"
                }
                Page::Shaders => "Horizon native .hshader packs",
                _ => "Verified downloads / enable in Installed",
            };
            let hint = ui.fit_text(Face::Regular, 14.0, hint, w - 64.0);
            ui.text(Face::Regular, 14.0, 32.0, 270.0, color::MUTED, &hint);
            let kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            let file_button_width = ((w - 72.0) / 2.0).min(158.0);
            button(
                &mut ui,
                &mut self.targets,
                [32.0, 222.0, file_button_width, 38.0],
                "Open folder",
                Action::OpenFolder(kind),
                false,
                press,
            );
            button(
                &mut ui,
                &mut self.targets,
                [w - 32.0 - file_button_width, 222.0, file_button_width, 38.0],
                "Import file",
                Action::Import(kind),
                false,
                press,
            );
            let top = 296.0;
            let bottom = h - 112.0;
            self.catalog.scroll = self.catalog.scroll.clamp(
                0.0,
                (self.catalog.results.len() as f32 * 78.0 - (bottom - top)).max(0.0),
            );
            self.list_view = Some([
                top,
                bottom,
                (self.catalog.results.len() as f32 * 78.0 - (bottom - top)).max(0.0),
            ]);
            for (i, p) in self.catalog.results.iter().enumerate() {
                let y = top + i as f32 * 78.0 - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                if download.is_none()
                    && !crate::catalog::source_only_mod(p)
                    && !(p.kind == crate::browser::Kind::Mod
                        && p.source == crate::browser::SourceId::Modrinth)
                {
                    self.list_targets
                        .push(([w - 150.0, y, 118.0, 70.0], Action::Install(i)));
                }
                if y < top || y + 70.0 > bottom {
                    continue;
                }
                // Each result sits in a slot, like a vanilla list entry.
                mc_ui::slot(&mut ui, [28.0, y - 2.0, w - 56.0, 74.0], false);
                let title = ui.fit_text(Face::Pixel, 18.0, &p.title, w - 222.0);
                let summary = ui.fit_text(Face::Regular, 14.0, &p.summary, w - 222.0);
                ui.text(Face::Pixel, 18.0, 42.0, y + 6.0, color::TEXT, &title);
                ui.text(
                    Face::Regular,
                    14.0,
                    42.0,
                    y + 38.0,
                    color::MUTED,
                    &summary,
                );
                if let Some(state) = download {
                    ui.text(
                        Face::Pixel,
                        15.0,
                        w - 150.0,
                        y + 12.0,
                        if state == "Installed" {
                            color::GREEN_TEXT
                        } else {
                            color::TEXT
                        },
                        &state,
                    );
                } else if p.kind == crate::browser::Kind::Mod
                    && p.source == crate::browser::SourceId::Modrinth
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "Java mod",
                    );
                } else if crate::catalog::source_only_mod(p) {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        ".hnmod: later",
                    );
                } else if p.kind == crate::browser::Kind::Mod && !cfg!(target_pointer_width = "64")
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "64-bit required",
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 150.0, y, 118.0, 42.0],
                        self.catalog.install_button(p),
                        Action::Install(i),
                        false,
                        press,
                    );
                }
            }
            button(
                &mut ui,
                &mut self.targets,
                [32.0, h - 100.0, 120.0, 40.0],
                "Previous",
                Action::Previous,
                false,
                press,
            );
            button(
                &mut ui,
                &mut self.targets,
                [166.0, h - 100.0, 120.0, 40.0],
                "Next",
                Action::Next,
                false,
                press,
            );
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            let top = 156.0;
            let bottom = h - 100.0;
            let row_height = if compact { 110.0 } else { 72.0 };
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
                        "-"
                    } else {
                        "+"
                    },
                    kind.label()
                );
                self.list_targets
                    .push(([32.0, y, w - 210.0, 40.0], Action::ToggleSection(section)));
                self.list_targets
                    .push(([w - 165.0, y, 133.0, 40.0], Action::Import(kind)));
                if y >= top && y + 40.0 <= bottom {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [32.0, y, w - 210.0, 40.0],
                        &label,
                        Action::ToggleSection(section),
                        false,
                        press,
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y, 133.0, 40.0],
                        "Import",
                        Action::Import(kind),
                        false,
                        press,
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
                    self.list_targets.push((
                        [
                            if compact { 32.0 } else { w - 300.0 },
                            y,
                            120.0,
                            row_height - 4.0,
                        ],
                        Action::ToggleLocal(i),
                    ));
                    self.list_targets.push((
                        [w - 165.0, y, 133.0, row_height - 4.0],
                        Action::DeleteLocal(i),
                    ));
                    if y < top || y + row_height - 4.0 > bottom {
                        continue;
                    }
                    mc_ui::slot(
                        &mut ui,
                        [28.0, y - 2.0, w - 56.0, row_height - 4.0],
                        item.enabled,
                    );
                    let title = ui.fit_text(
                        Face::Pixel,
                        18.0,
                        &item.title,
                        if compact { w - 84.0 } else { w - 384.0 },
                    );
                    ui.text(Face::Pixel, 18.0, 42.0, y + 4.0, color::TEXT, &title);
                    ui.text(
                        Face::Regular,
                        13.0,
                        42.0,
                        y + 32.0,
                        color::MUTED,
                        item.java.as_deref().unwrap_or_else(|| kind.label()),
                    );
                    if item.java.is_some() {
                        ui.text(
                            Face::Regular,
                            14.0,
                            if compact { 32.0 } else { w - 300.0 },
                            y + if compact { 62.0 } else { 12.0 },
                            color::MUTED,
                            "Java mod: cannot run",
                        );
                    } else if kind == Kind::Mods && !cfg!(target_pointer_width = "64") {
                        ui.text(
                            Face::Regular,
                            14.0,
                            if compact { 32.0 } else { w - 300.0 },
                            y + if compact { 62.0 } else { 12.0 },
                            color::MUTED,
                            "64-bit required",
                        );
                    } else {
                        toggle(
                            &mut ui,
                            &mut self.targets,
                            [
                                if compact { 32.0 } else { w - 300.0 },
                                y + if compact { 55.0 } else { 0.0 },
                                120.0,
                                40.0,
                            ],
                            item.enabled,
                            Action::ToggleLocal(i),
                            press,
                        );
                    }
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y + if compact { 55.0 } else { 0.0 }, 133.0, 40.0],
                        "Remove",
                        Action::DeleteLocal(i),
                        false,
                        press,
                    );
                }
            }
        } else {
            let c = &self.catalog.config;
            mc_ui::shadow_text(
                &mut ui,
                Face::Pixel,
                22.0,
                32.0,
                156.0,
                color::TEXT,
                "Launch settings",
            );
            let row_height = if compact { 82.0 } else { 66.0 };
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (15.0 * row_height - (h - 130.0 - 195.0)).max(0.0));
            self.list_view = Some([
                195.0,
                h - 130.0,
                (15.0 * row_height - (h - 130.0 - 195.0)).max(0.0),
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
                ("Profile", format!("Profile {}", c.profile), Action::Profile),
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
                let y = 195.0 + i as f32 * row_height - self.catalog.scroll;
                self.list_targets.push((
                    [
                        if compact { 32.0 } else { w * 0.36 },
                        y,
                        if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                        if compact { 70.0 } else { 46.0 },
                    ],
                    action,
                ));
                if y < 185.0 || y + if compact { 70.0 } else { 46.0 } > h - 130.0 {
                    continue;
                }
                ui.text(
                    Face::Regular,
                    16.0,
                    32.0,
                    y + if compact { 0.0 } else { 13.0 },
                    color::MUTED,
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
                            if compact { 32.0 } else { w * 0.36 },
                            y + if compact { 24.0 } else { 0.0 },
                            if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                            46.0,
                        ],
                        match action {
                            Action::PerformanceHints => c.device.performance_hints,
                            Action::Immersive => c.screen.immersive,
                            Action::Cutout => c.screen.avoid_cutout,
                            _ => c.device.gpu_profile,
                        },
                        action,
                        press,
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [
                            if compact { 32.0 } else { w * 0.36 },
                            y + if compact { 24.0 } else { 0.0 },
                            if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                            46.0,
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
                        press,
                    );
                }
                mc_ui::divider(&mut ui, 32.0, y + if compact { 78.0 } else { 62.0 }, w - 64.0);
            }
            button(
                &mut ui,
                &mut self.targets,
                [w * 0.5 + 6.0, h - 114.0, w * 0.5 - 38.0, 42.0],
                "Microsoft / Xbox account",
                Action::Account,
                true,
                press,
            );
            button(
                &mut ui,
                &mut self.targets,
                [32.0, h - 114.0, w * 0.5 - 38.0, 42.0],
                "Diagnostics",
                Action::Logs,
                false,
                press,
            );
        }
        let status = self.error.as_deref().unwrap_or(&self.catalog.status);
        let status_color = if self.error.is_some() {
            color::ERROR
        } else {
            color::MUTED
        };
        for (i, line) in status
            .chars()
            .collect::<Vec<_>>()
            .chunks(110)
            .take(2)
            .enumerate()
        {
            ui.text(
                Face::Regular,
                14.0,
                32.0,
                h - 47.0 + i as f32 * 18.0,
                status_color,
                &ui.fit_text(
                    Face::Regular,
                    14.0,
                    &line.iter().collect::<String>(),
                    (w - 210.0).max(40.0),
                ),
            );
        }
        button(
            &mut ui,
            &mut self.targets,
            [w - 142.0, h - 52.0, 110.0, 32.0],
            "GitHub",
            Action::Repository,
            false,
            press,
        );
        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing == Some("name"),
                press,
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets, press);
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
            mc_ui::scrollbar(&mut ui, w - 14.0, top, height, y, thumb);
        }
        let caret_visible = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok_and(|t| t.as_millis() % 1000 < 500);
        if caret_visible && let Some(name) = self.catalog.editing {
            let value = match name {
                "name" => self.catalog.config.name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => format!("Search: {}", self.catalog.query),
            };
            for ([x, y, w, h], action) in &self.targets {
                if *action == Action::Field(name) {
                    // Match the edit box's metrics: Geist 15, 10dp left inset.
                    let text = ui.fit_text(Face::Regular, 15.0, &value, (w - 20.0).max(8.0));
                    let caret_x = x + 10.0 + ui.measure(Face::Regular, 15.0, &text);
                    mc_ui::fill(
                        &mut ui,
                        caret_x.min(x + w - 8.0),
                        y + (h - 18.0) / 2.0,
                        2.0,
                        18.0,
                        color::TEXT,
                    );
                } else if name == "render_scale" && *action == Action::RenderScale {
                    // While editing, the scale button doubles as the field;
                    // its centred pixel label decides the caret's place.
                    let font = button_font([*x, *y, *w, *h]);
                    let text = ui.fit_text(Face::Pixel, font, &value, (w - 16.0).max(8.0));
                    let width = ui.measure(Face::Pixel, font, &text);
                    let caret_x = x + ((w - width) / 2.0).max(8.0) + width + 2.0;
                    mc_ui::fill(
                        &mut ui,
                        caret_x.min(x + w - 8.0),
                        y + (h - 18.0) / 2.0,
                        2.0,
                        18.0,
                        color::TEXT,
                    );
                }
            }
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            mc_ui::focus_ring(&mut ui, [*x, *y, *w, *h]);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
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
