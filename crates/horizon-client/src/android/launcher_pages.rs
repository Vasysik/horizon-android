use super::*;
use crate::catalog::SettingsCategory;
use horizon_client::{
    gfx::Renderer,
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
const MY3_CARD_BG: u32 = color::hex(0x171621, 255);
const MY3_CARD_BORDER: u32 = color::hex(0x262435, 255);
const MY3_SURFACE_HIGH: u32 = color::hex(0x232032, 255);
const MY3_BORDER: u32 = color::hex(0x282538, 255);
const MY3_NAV_ACTIVE: u32 = color::hex(0x5B4898, 255);
const MY3_ACCENT: u32 = color::hex(0x9A7BFF, 255);
const MY3_ACCENT_SOFT: u32 = color::hex(0x332854, 255);
const MY3_ON_ACCENT: u32 = color::hex(0x17112E, 255);
const MY3_TRACK: u32 = color::hex(0x2B2937, 255);
const MY3_PAUSE_BG: u32 = color::hex(0x252333, 255);
const MY3_CANCEL_BG: u32 = color::hex(0x382E4C, 255);
const MY3_TEXT: u32 = color::hex(0xF4F0FC, 255);
const MY3_MUTED: u32 = color::hex(0x9C96B0, 255);

/// Smooth 60fps UI animation state across navigation, pages, progress bars, and modals.
pub(super) struct AnimState {
    pub last_tick: std::time::Instant,
    pub last_page: Page,
    pub last_category: SettingsCategory,
    pub nav_pill_y: f32,
    pub page_fade: f32,
    pub page_slide: f32,
    pub progress_smooth: f32,
    pub modal_anim: f32,
    pub shimmer_phase: f32,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            last_tick: std::time::Instant::now(),
            last_page: Page::Play,
            last_category: SettingsCategory::Overview,
            nav_pill_y: 74.0,
            page_fade: 1.0,
            page_slide: 0.0,
            progress_smooth: 0.0,
            modal_anim: 0.0,
            shimmer_phase: 0.0,
        }
    }
}

impl AnimState {
    pub fn step(
        &mut self,
        page: Page,
        category: SettingsCategory,
        target_nav_y: f32,
        has_modal: bool,
    ) {
        let now = std::time::Instant::now();
        let dt = now
            .duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.001, 0.08);
        self.last_tick = now;

        if page != self.last_page || (page == Page::Settings && category != self.last_category) {
            self.last_page = page;
            self.last_category = category;
            self.page_fade = 0.0;
            self.page_slide = 14.0;
        }

        let speed = (dt * 16.0).min(1.0);
        self.nav_pill_y += (target_nav_y - self.nav_pill_y) * (dt * 18.0).min(1.0);
        self.page_fade = (self.page_fade + dt * 7.5).min(1.0);
        self.page_slide *= (1.0 - speed).max(0.0);
        let modal_target = if has_modal { 1.0 } else { 0.0 };
        self.modal_anim += (modal_target - self.modal_anim) * (dt * 15.0).min(1.0);
        self.shimmer_phase = (self.shimmer_phase + dt * 0.85) % 1.0;
    }

    pub fn trigger_page_transition(&mut self) {
        self.page_fade = 0.0;
        self.page_slide = 14.0;
    }

    pub fn is_active(&self) -> bool {
        self.page_fade < 0.99
            || self.page_slide.abs() > 0.4
            || (self.modal_anim > 0.01 && self.modal_anim < 0.99)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Page(Page),
    SettingsCat(SettingsCategory),
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
    OpenProject(usize),
    OpenBundledProject(&'static str),
    CloseProjectModal,
    SelectModalVersion(usize),
    InstallModalProject,
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
    UiScale,
    ChooseUiScale(u32),
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
    // Video settings actions (options.txt)
    RenderDistDelta(i32),
    SimDistDelta(i32),
    CycleMaxFps,
    CycleGraphicsPreset,
    ToggleVsync,
    ToggleSmoothLighting,
    CycleClouds,
    ToggleMenuBlur,
    FovDelta(f32),
    // Experimental settings actions
    ToggleSuperPerf,
    ToggleBatteryPerf,
    ToggleMaxFpsMode,
    ToggleZeroLatencyTouch,
    ToggleFastChunkUpload,
    // Controls & Control Editor actions
    OpenControlEditor,
    CloseControlEditor,
    ControlProfileChooser,
    ChooseControlProfile(u32),
    SaveControlProfileAsNew,
    ExportControlProfile,
    ImportControlProfile,
    MouseSensDelta(f32),
    TouchSensDelta(f32),
    VirtualMouseSpeedDelta(f32),
    ButtonScaleDelta(f32),
    ToggleControlScheme,
    SelectControlMode(horizon_client::touch::ControlStyle),
    CopyActivePreset,
    EditActivePresetMeta,
    ToggleSplitControls,
    ToggleSprintLatch,
    ToggleLeftHanded,
    ToggleInvertY,
    ToggleGyro,
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
        if accent { MY3_ACCENT } else { MY3_SURFACE_HIGH },
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
    let left_align = matches!(action, Action::Field(_)) || label.ends_with(" v");
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
    if let Some(c) = ui.clip_rect(rect) {
        targets.push((c, action));
    }
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
        Face::Medium,
        13.5,
        x,
        y + (h - 20.0) * 0.5,
        if on { MY3_ACCENT } else { MY3_MUTED },
        if on { "Enabled" } else { "Off" },
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
    if let Some(c) = ui.clip_rect([x, y, w, h]) {
        targets.push((c, action));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_setting_card(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    list_targets: &mut Vec<([f32; 4], Action)>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    subtitle: &str,
    right_control: SettingControl<'_>,
) {
    ui.rounded(x, y, w, h, 14.0, MY3_CARD_BG);
    ui.border(x, y, w, h, 14.0, MY3_CARD_BORDER);

    let text_max = (w - 220.0).max(90.0);
    let fitted_title = ui.fit_text(Face::SemiBold, 15.0, title, text_max);
    ui.text(
        Face::SemiBold,
        15.0,
        x + 16.0,
        y + 10.0,
        MY3_TEXT,
        &fitted_title,
    );
    let fitted_sub = ui.fit_text(Face::Regular, 12.5, subtitle, text_max);
    ui.text(
        Face::Regular,
        12.5,
        x + 16.0,
        y + 32.0,
        MY3_MUTED,
        &fitted_sub,
    );

    match right_control {
        SettingControl::Toggle(on, action) => {
            let rect = [x + w - 122.0, y + (h - 38.0) * 0.5, 108.0, 38.0];
            list_targets.push((rect, action));
            toggle(ui, targets, rect, on, action);
        }
        SettingControl::Button(label, action, accent) => {
            let bw = 176.0f32.min(w * 0.44);
            let rect = [x + w - bw - 14.0, y + (h - 38.0) * 0.5, bw, 38.0];
            list_targets.push((rect, action));
            button(ui, targets, rect, label, action, accent);
        }
        SettingControl::Stepper(value_label, minus_action, plus_action) => {
            let right_x = x + w - 14.0;
            let plus_rect = [right_x - 36.0, y + (h - 36.0) * 0.5, 36.0, 36.0];
            let val_w = 88.0;
            let val_x = plus_rect[0] - val_w - 6.0;
            let minus_rect = [val_x - 36.0 - 6.0, y + (h - 36.0) * 0.5, 36.0, 36.0];

            ui.rounded(val_x, y + (h - 36.0) * 0.5, val_w, 36.0, 12.0, MY3_SURFACE);
            ui.border(val_x, y + (h - 36.0) * 0.5, val_w, 36.0, 12.0, MY3_BORDER);
            let fitted_v = ui.fit_text(Face::SemiBold, 13.5, value_label, val_w - 10.0);
            let tw = ui.measure(Face::SemiBold, 13.5, &fitted_v);
            ui.text(
                Face::SemiBold,
                13.5,
                val_x + (val_w - tw) * 0.5,
                y + (h - 20.0) * 0.5,
                MY3_ACCENT,
                &fitted_v,
            );
            list_targets.push((minus_rect, minus_action));
            list_targets.push((plus_rect, plus_action));
            button(ui, targets, minus_rect, "-", minus_action, false);
            button(ui, targets, plus_rect, "+", plus_action, true);
        }
    }
}

enum SettingControl<'a> {
    Toggle(bool, Action),
    Button(&'a str, Action, bool),
    Stepper(&'a str, Action, Action),
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
    speed_label: &str,
    eta_label: &str,
    step_index: usize,
    steps: &[&str],
    fraction: f32,
    shimmer_phase: f32,
    stage_text: &str,
    paused: bool,
    pause_action: Action,
    cancel_action: Action,
) {
    let center_y = (h * 0.44).clamp(168.0, h - 135.0);
    let title_fitted = ui.fit_text(Face::SemiBold, 23.0, title, content_w);
    ui.text(
        Face::SemiBold,
        23.0,
        content_x,
        center_y - 68.0,
        MY3_TEXT,
        &title_fitted,
    );

    // Step indicator pill badge next to subtitle
    let step_badge = format!(
        "Step {} of {}",
        (step_index + 1).min(steps.len()),
        steps.len().max(1)
    );
    let badge_w = (ui.measure(Face::SemiBold, 12.0, &step_badge) + 18.0).max(76.0);
    ui.rounded(
        content_x,
        center_y - 34.0,
        badge_w,
        22.0,
        11.0,
        MY3_ACCENT_SOFT,
    );
    ui.text(
        Face::SemiBold,
        12.0,
        content_x + 9.0,
        center_y - 31.0,
        MY3_ACCENT,
        &step_badge,
    );

    let right_w = ui.measure(Face::Medium, 13.5, subtitle_right);
    let left_x = content_x + badge_w + 10.0;
    let left_max = (content_w - badge_w - right_w - 24.0).max(60.0);
    let left_fitted = ui.fit_text(Face::Regular, 13.5, subtitle_left, left_max);
    ui.text(
        Face::Regular,
        13.5,
        left_x,
        center_y - 32.0,
        MY3_MUTED,
        &left_fitted,
    );
    ui.text(
        Face::Medium,
        13.5,
        content_x + content_w - right_w,
        center_y - 32.0,
        MY3_TEXT,
        subtitle_right,
    );

    // Thick Material You 3 rounded progress bar (#9A7BFF on #2B2937) with animated shimmer.
    let bar_y = center_y + 2.0;
    let bar_h = 12.0;
    ui.rounded(content_x, bar_y, content_w, bar_h, 6.0, MY3_TRACK);
    let clamped = fraction.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_w = (content_w * clamped).max(bar_h).min(content_w);
        ui.rounded(content_x, bar_y, fill_w, bar_h, 6.0, MY3_ACCENT);
        if !paused && fill_w > 36.0 {
            let sw = (fill_w * 0.25).clamp(24.0, 72.0);
            let sx = content_x + (fill_w - sw) * shimmer_phase.clamp(0.0, 1.0);
            ui.rounded(
                sx,
                bar_y + 2.0,
                sw,
                bar_h - 4.0,
                4.0,
                color::hex(0xC9B8FF, 165),
            );
        }
    }

    // Stage description on left + Download Speed & ETA on right
    let metrics_line = format!("{speed_label}  •  {eta_label}");
    let metrics_w = ui.measure(Face::Medium, 13.0, &metrics_line);
    let stage_line = if paused {
        format!("Paused • {stage_text}")
    } else {
        stage_text.to_owned()
    };
    let stage_max = (content_w - metrics_w - 16.0).max(80.0);
    let stage_fitted = ui.fit_text(Face::Regular, 13.0, &stage_line, stage_max);
    ui.text(
        Face::Regular,
        13.0,
        content_x,
        bar_y + 20.0,
        MY3_MUTED,
        &stage_fitted,
    );
    ui.text(
        Face::Medium,
        13.0,
        content_x + content_w - metrics_w,
        bar_y + 20.0,
        MY3_ACCENT,
        &metrics_line,
    );

    // Multi-step visual progress pills row
    if !steps.is_empty() {
        let step_y = bar_y + 48.0;
        let gap = 6.0;
        let n = steps.len() as f32;
        let pill_w = ((content_w - gap * (n - 1.0)) / n).max(36.0);
        for (idx, name) in steps.iter().enumerate() {
            let px = content_x + idx as f32 * (pill_w + gap);
            let done = idx < step_index;
            let active = idx == step_index;
            let bg = if active {
                MY3_ACCENT_SOFT
            } else if done {
                color::hex(0x1E2832, 255)
            } else {
                MY3_SURFACE
            };
            let border_col = if active {
                MY3_ACCENT
            } else if done {
                color::hex(0x4E8E6E, 255)
            } else {
                MY3_BORDER
            };
            ui.rounded(px, step_y, pill_w, 26.0, 8.0, bg);
            ui.border(px, step_y, pill_w, 26.0, 8.0, border_col);
            let prefix = if done { "✓ " } else { "" };
            let step_txt = format!("{prefix}{name}");
            let fitted = ui.fit_text(Face::Medium, 11.5, &step_txt, pill_w - 10.0);
            let tw = ui.measure(Face::Medium, 11.5, &fitted);
            ui.text(
                Face::Medium,
                11.5,
                px + (pill_w - tw) * 0.5,
                step_y + 5.0,
                if active || done { MY3_TEXT } else { MY3_MUTED },
                &fitted,
            );
        }
    }

    // Bottom-right MY3 Pause (||) and Cancel (X) rounded square buttons.
    let btn_y = (h - 62.0).max(bar_y + 82.0);
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
                } else if let Some(modal) = &mut self.catalog.project_modal {
                    &mut modal.scroll
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
        // Page drawing borrows `self` mutably (settings and the project modal do), so lend the
        // renderer and its resources out as plain locals while the frame is built, and put them
        // back whatever the result is.
        let Some(mut r) = self.renderer.take() else {
            return Ok(());
        };
        let Some(gui) = self.gui.take() else {
            self.renderer = Some(r);
            return Ok(());
        };
        let result = self.draw_pages_with(&mut r, &gui);
        self.renderer = Some(r);
        self.gui = Some(gui);
        result
    }

    /// Draws one launcher frame with the renderer lent out (see `draw_pages`).
    fn draw_pages_with(&mut self, r: &mut Renderer, gui: &game::UiResources) -> Result<()> {
        if r.suspended {
            return Ok(());
        }
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;

        // Fullscreen MCPE + GoyControl Editor with Draggable Floating Menu Pod
        if let Some(tc) = &mut self.control_editor {
            let mut b = HudBatch::new([r.config.width, r.config.height]);
            tc.viewport(r.config.width, r.config.height);
            let profile_name = tc.settings.active_profile.clone();
            horizon_client::touch_ui::draw_hud_editor(tc, &mut b, &gui.atlas, &profile_name, None);
            if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
                horizon_client::ui_theme::reset_atlas();
            }
            return Ok(());
        }

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
        ui.horizon_logo(20.0, 15.0, 25.0);
        ui.rounded(55.0, 18.0, 1.5, 20.0, 0.75, MY3_BORDER);
        let header_title = format!("Horizon Android {}", crate::backend::VERSION);
        let account_name = self.login.account_label(&self.catalog.config.name);
        let account_text_w = ui.measure(Face::Medium, 13.5, &account_name).min(136.0);
        let pill_w = (account_text_w + 64.0).clamp(114.0, 204.0);
        let pill_x = (w - 18.0 - pill_w).max(190.0);
        let pill_y = 11.0;
        let pill_h = 36.0;

        let eq_btn_rect = [pill_x - 44.0, pill_y, 36.0, pill_h];
        ui.rounded(
            eq_btn_rect[0],
            eq_btn_rect[1],
            eq_btn_rect[2],
            eq_btn_rect[3],
            pill_h * 0.5,
            color::hex(0x1C1929, 255),
        );
        ui.border(
            eq_btn_rect[0],
            eq_btn_rect[1],
            eq_btn_rect[2],
            eq_btn_rect[3],
            pill_h * 0.5,
            color::hex(0x312C45, 255),
        );
        ui.icon(
            Icon::MenuEquals,
            eq_btn_rect[0] + 9.0,
            eq_btn_rect[1] + 9.0,
            18.0,
            MY3_ACCENT,
        );
        self.targets.push((eq_btn_rect, Action::OpenControlEditor));

        let max_title_w = (eq_btn_rect[0] - 76.0).max(80.0);
        let fitted_header = ui.fit_text(Face::SemiBold, 15.5, &header_title, max_title_w);
        ui.text(Face::SemiBold, 15.5, 67.0, 17.0, MY3_TEXT, &fitted_header);

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
        ui.avatar_head_3d(pill_x + 6.0, pill_y + 4.0, 28.0, &account_name);
        let fitted_acct = ui.fit_text(Face::Medium, 13.5, &account_name, pill_w - 62.0);
        ui.text(
            Face::Medium,
            13.5,
            pill_x + 37.0,
            pill_y + 8.0,
            MY3_TEXT,
            &fitted_acct,
        );
        ui.icon(
            Icon::ChevronDown,
            pill_x + pill_w - 22.0,
            pill_y + 12.0,
            12.0,
            MY3_MUTED,
        );
        self.targets
            .push(([pill_x, pill_y, pill_w, pill_h], Action::Account));

        // ---- Left Vertical Navigation Rail + Smooth Gliding Active Pill ----
        let sidebar_w = if w < 560.0 { 136.0 } else { 184.0 };
        ui.rounded(sidebar_w, 56.0, 1.0, (h - 68.0).max(10.0), 0.0, MY3_BORDER);
        let nav_start_y = 62.0;
        let nav_step = ((h - nav_start_y - 14.0) / 6.0).clamp(36.0, 48.0);
        let nav_h = (nav_step - 5.0).clamp(32.0, 41.0);
        let selected_idx = Page::ALL
            .iter()
            .position(|p| *p == self.catalog.page)
            .unwrap_or(0);
        let target_nav_y = nav_start_y + selected_idx as f32 * nav_step;
        self.anim.step(
            self.catalog.page,
            self.catalog.settings_category,
            target_nav_y,
            self.catalog.project_modal.is_some(),
        );

        // Draw gliding MY3 active indicator pill
        ui.rounded(
            10.0,
            self.anim.nav_pill_y,
            sidebar_w - 20.0,
            nav_h,
            nav_h * 0.5,
            MY3_NAV_ACTIVE,
        );

        for (i, page) in Page::ALL.iter().enumerate() {
            let nx = 10.0;
            let ny = nav_start_y + i as f32 * nav_step;
            let nw = sidebar_w - 20.0;
            let selected = *page == self.catalog.page;
            let fg = if selected {
                MY3_TEXT
            } else {
                color::hex(0xCFC9E2, 255)
            };
            ui.icon(
                page_icon(*page),
                nx + 14.0,
                ny + (nav_h - 17.0) * 0.5,
                17.0,
                fg,
            );
            let label = ui.fit_text(Face::Medium, 13.5, page.name(), nw - 42.0);
            ui.text(
                Face::Medium,
                13.5,
                nx + 39.0,
                ny + (nav_h - 19.0) * 0.5,
                fg,
                &label,
            );
            self.targets
                .push(([nx, ny, nw, nav_h], Action::Page(*page)));
        }

        // ---- Main Content Area (with smooth slide transition offset) ----
        let content_x = sidebar_w + 22.0;
        let content_w = (w - content_x - 22.0).max(160.0);
        let compact = content_w < 430.0;
        let page = self.catalog.page;
        let slide_y = self.anim.page_slide;

        // When any catalog item (Mod, Pack, Shader) is downloading/installing, take the entire
        // middle screen to show the MY3 downloading view with Speed, ETA, and Steps.
        if let Some((source_id, _)) = self
            .catalog
            .downloading
            .as_ref()
            .filter(|_| self.catalog.busy())
        {
            let frac = self.catalog.progress_fraction().clamp(0.0, 1.0);
            self.anim.progress_smooth += (frac - self.anim.progress_smooth) * 0.25;
            let smooth_frac = self.anim.progress_smooth.clamp(0.04, 1.0);
            let total_mib = f64::from(self.catalog.downloading_size_mib.max(1.0));
            let done_mib = (total_mib * f64::from(frac)).clamp(0.0, total_mib);
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
            let sub_right = format!("{done_mib:.1} / {total_mib:.1} MiB ({:.0}%)", frac * 100.0);
            let stage = self.catalog.progress_stage();
            let step_idx = if frac < 0.15 {
                0
            } else if frac < 0.72 {
                1
            } else if frac < 0.92 {
                2
            } else {
                3
            };
            draw_my3_progress_view(
                &mut ui,
                &mut self.targets,
                content_x,
                content_w,
                h,
                &title,
                &sub_left,
                &sub_right,
                &self.catalog.transfer.speed_label(),
                &self.catalog.transfer.eta_label(done_mib, total_mib),
                step_idx,
                &["Resolve", "Download", "Verify", "Install"],
                smooth_frac,
                self.anim.shimmer_phase,
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
                self.anim.progress_smooth += (overall - self.anim.progress_smooth) * 0.25;
                let smooth_overall = self.anim.progress_smooth.clamp(0.02, 1.0);
                let (done_mib, total_mib) =
                    if progress.stage == InstallStage::Download && progress.total > 0 {
                        (
                            progress.done as f64 / 1_048_576.0,
                            progress.total as f64 / 1_048_576.0,
                        )
                    } else {
                        (55.3 * f64::from(overall), 55.3)
                    };
                let paused = self.install_paused.load(Ordering::Relaxed);
                self.install_transfer.observe(done_mib, paused);
                let sub_left = format!(
                    "Minecraft 26.3 • Profile {}",
                    self.catalog.config.profile + 1
                );
                let sub_right = format!(
                    "{done_mib:.1} / {total_mib:.1} MiB ({:.0}%)",
                    overall * 100.0
                );
                let stage_text = match progress.stage {
                    InstallStage::Download => "Downloading Java Edition 26.3 client.jar",
                    other => other.label(),
                };
                draw_my3_progress_view(
                    &mut ui,
                    &mut self.targets,
                    content_x,
                    content_w,
                    h,
                    "Getting your game ready",
                    &sub_left,
                    &sub_right,
                    &self.install_transfer.speed_label(),
                    &self.install_transfer.eta_label(done_mib, total_mib),
                    current,
                    &[
                        "Manifest",
                        "Metadata",
                        "Client JAR",
                        "Verify",
                        "Extract",
                        "Ready",
                    ],
                    smooth_overall,
                    self.anim.shimmer_phase,
                    stage_text,
                    paused,
                    Action::PauseInstall,
                    Action::CancelInstall,
                );
            } else {
                let card_y = 66.0 + slide_y;
                let card_h = (h - 178.0).clamp(148.0, 236.0);
                ui.rounded(content_x, card_y, content_w, card_h, 20.0, MY3_SURFACE);
                ui.border(content_x, card_y, content_w, card_h, 20.0, MY3_BORDER);

                ui.text(
                    Face::SemiBold,
                    23.0,
                    content_x + 22.0,
                    card_y + 18.0,
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
                    14.0,
                    content_x + 22.0,
                    card_y + 52.0,
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
                        "• Browse Mods, Resource Packs & Shaders with Modrinth previews",
                        "• Customize MCPE & GoyControl HUD layouts in Settings → Controls",
                    ]
                } else {
                    [
                        "• Downloads official Minecraft 26.3 assets directly from Mojang",
                        "• Live download speed, ETA and 6-step verification",
                        "• Play offline or sign in with your Microsoft account",
                    ]
                };
                for (index, line) in rows.iter().enumerate() {
                    let fitted =
                        ui.fit_text(Face::Regular, 13.5, line, (content_w - 44.0).max(80.0));
                    ui.text(
                        Face::Regular,
                        13.5,
                        content_x + 22.0,
                        card_y + 86.0 + index as f32 * 25.0,
                        MY3_MUTED,
                        &fitted,
                    );
                }

                let btn_y = (h - 92.0).max(card_y + card_h + 12.0);
                let play_w = content_w.min(256.0);
                button(
                    &mut ui,
                    &mut self.targets,
                    [content_x, btn_y, play_w, 46.0],
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
                if content_w > 390.0 {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x + play_w + 12.0, btn_y, 134.0, 46.0],
                        &format!("Profile {} v", self.catalog.config.profile + 1),
                        Action::Profile,
                        false,
                    );
                }
            }
        } else if page.is_catalog() {
            let header_y = 62.0 + slide_y;
            ui.text(
                Face::SemiBold,
                19.0,
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
                let seg_w = 100.0;
                for (i, origin) in [Origin::Horizon, Origin::Modrinth].into_iter().enumerate() {
                    let rx = content_x + content_w - (2 - i) as f32 * (seg_w + 6.0) + 6.0;
                    let rect = [rx, header_y - 2.0, seg_w, 30.0];
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
            let search_y = 98.0 + slide_y;
            let search_btn_w = 98.0;
            let field_w = (content_w - search_btn_w - 8.0).max(120.0);
            ui.rounded(content_x, search_y, field_w, 36.0, 18.0, MY3_SURFACE);
            ui.border(content_x, search_y, field_w, 36.0, 18.0, MY3_BORDER);
            ui.icon(
                Icon::Search,
                content_x + 12.0,
                search_y + 9.0,
                17.0,
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
            let fitted_q = ui.fit_text(Face::Regular, 13.5, prompt, field_w - 42.0);
            ui.text(
                Face::Regular,
                13.5,
                content_x + 35.0,
                search_y + 8.0,
                if self.catalog.query.is_empty() {
                    MY3_MUTED
                } else {
                    MY3_TEXT
                },
                &fitted_q,
            );
            self.targets.push((
                [content_x, search_y, field_w, 36.0],
                Action::Field("search"),
            ));

            button(
                &mut ui,
                &mut self.targets,
                [content_x + field_w + 8.0, search_y, search_btn_w, 36.0],
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
            let sub_y = 140.0 + slide_y;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, sub_y, 114.0, 29.0],
                "Open folder",
                Action::OpenFolder(kind),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 122.0, sub_y, 114.0, 29.0],
                "Import file",
                Action::Import(kind),
                false,
            );
            if !compact {
                let status_line = self.error.as_deref().unwrap_or(&self.catalog.status);
                let fitted_status = ui.fit_text(
                    Face::Regular,
                    12.5,
                    status_line,
                    (content_w - 248.0).max(60.0),
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 246.0,
                    sub_y + 5.0,
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
            let row_h = 72.0;
            let row_count = bundled.len() + catalog_rows.len();
            let top = 176.0;
            let bottom = h - 46.0;
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
            ui.set_clip(Some([
                content_x - 2.0,
                top,
                content_w + 4.0,
                (bottom - top).max(1.0),
            ]));

            for (row, (id, title, description)) in bundled.iter().enumerate() {
                let y = top + row as f32 * row_h - self.catalog.scroll;
                let enabled = self.catalog.bundled_enabled.contains(*id);
                let card_rect = [content_x, y, content_w - 118.0, 64.0];
                let btn_rect = [content_x + content_w - 112.0, y + 13.0, 100.0, 38.0];
                self.list_targets
                    .push((card_rect, Action::OpenBundledProject(id)));
                self.list_targets
                    .push((btn_rect, Action::ToggleBundled(id)));
                if y + 64.0 < top || y > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 64.0, 14.0, MY3_CARD_BG);
                ui.border(content_x, y, content_w, 64.0, 14.0, MY3_CARD_BORDER);

                // Project icon: fetched Modrinth PFP if available, else MY3 vector icon badge.
                let drawn_icon = if let Some((iw, ih, rgba)) = self.catalog.icons.get(*id) {
                    ui.image(id, *iw, *ih, rgba, content_x + 10.0, y + 10.0, 44.0, 44.0)
                } else {
                    false
                };
                if !drawn_icon {
                    ui.rounded(
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                        12.0,
                        color::hex(0x27213C, 255),
                    );
                    ui.icon(fallback_icon, content_x + 21.0, y + 21.0, 22.0, MY3_ACCENT);
                }

                // Badge chip ("Built-in • Tap for info")
                let text_max = (content_w - 190.0).max(80.0);
                let fitted_title = ui.fit_text(Face::SemiBold, 15.0, title, text_max);
                let rich_desc = self
                    .catalog
                    .enriched_summaries
                    .get(*id)
                    .map_or(*description, String::as_str);
                let summary = ui.fit_text(Face::Regular, 12.5, rich_desc, text_max);
                ui.text(
                    Face::SemiBold,
                    15.0,
                    content_x + 64.0,
                    y + 8.0,
                    MY3_TEXT,
                    &fitted_title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 64.0,
                    y + 28.0,
                    MY3_MUTED,
                    &summary,
                );
                ui.text(
                    Face::Medium,
                    11.0,
                    content_x + 64.0,
                    y + 45.0,
                    MY3_ACCENT,
                    "Horizon Built-in • Tap for versions, Modrinth info & screenshots",
                );
                if let Some(c) = ui.clip_rect(card_rect) {
                    self.targets.push((c, Action::OpenBundledProject(id)));
                }
                button(
                    &mut ui,
                    &mut self.targets,
                    btn_rect,
                    if enabled { "Installed" } else { "Install" },
                    Action::ToggleBundled(id),
                    !enabled,
                );
            }

            for (row, (i, p)) in catalog_rows.into_iter().enumerate() {
                let y = top + (row + bundled.len()) as f32 * row_h - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                let card_rect = [content_x, y, content_w - 118.0, 64.0];
                let btn_rect = [content_x + content_w - 112.0, y + 13.0, 100.0, 38.0];
                self.list_targets.push((card_rect, Action::OpenProject(i)));
                if download.is_none() {
                    self.list_targets.push((btn_rect, Action::Install(i)));
                }
                if y + 64.0 < top || y > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 64.0, 14.0, MY3_CARD_BG);
                ui.border(content_x, y, content_w, 64.0, 14.0, MY3_CARD_BORDER);

                // Project icon: fetched RGBA icon if ready, else MY3 vector icon badge.
                let drawn_icon = if let Some((iw, ih, rgba)) = self.catalog.icons.get(&p.id) {
                    ui.image(
                        &p.id,
                        *iw,
                        *ih,
                        rgba,
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                    )
                } else {
                    false
                };
                if !drawn_icon {
                    ui.rounded(
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                        12.0,
                        color::hex(0x27213C, 255),
                    );
                    ui.icon(fallback_icon, content_x + 21.0, y + 21.0, 22.0, MY3_ACCENT);
                }

                let text_max = (content_w - 190.0).max(80.0);
                let title = ui.fit_text(Face::SemiBold, 15.0, &p.title, text_max);
                let rich_summary = self
                    .catalog
                    .enriched_summaries
                    .get(&p.id)
                    .map_or(p.summary.as_str(), String::as_str);
                let summary = ui.fit_text(Face::Regular, 12.5, rich_summary, text_max);
                ui.text(
                    Face::SemiBold,
                    15.0,
                    content_x + 64.0,
                    y + 8.0,
                    MY3_TEXT,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 64.0,
                    y + 28.0,
                    MY3_MUTED,
                    &summary,
                );
                let meta_tag = if p.downloads > 0 {
                    format!(
                        "{} • {} downloads • Tap for versions & screenshots",
                        p.source.label(),
                        format_downloads(p.downloads)
                    )
                } else {
                    format!(
                        "{} • Tap for versions, description & screenshots",
                        p.source.label()
                    )
                };
                let fitted_meta = ui.fit_text(Face::Medium, 11.0, &meta_tag, text_max);
                ui.text(
                    Face::Medium,
                    11.0,
                    content_x + 64.0,
                    y + 45.0,
                    MY3_ACCENT,
                    &fitted_meta,
                );
                if let Some(c) = ui.clip_rect(card_rect) {
                    self.targets.push((c, Action::OpenProject(i)));
                }

                if let Some(state) = download {
                    ui.rounded(
                        btn_rect[0],
                        btn_rect[1],
                        btn_rect[2],
                        btn_rect[3],
                        19.0,
                        MY3_SURFACE_HIGH,
                    );
                    ui.icon(
                        Icon::Check,
                        btn_rect[0] + 10.0,
                        btn_rect[1] + 11.0,
                        15.0,
                        MY3_ACCENT,
                    );
                    ui.text(
                        Face::Medium,
                        13.0,
                        btn_rect[0] + 28.0,
                        btn_rect[1] + 9.0,
                        MY3_TEXT,
                        &state,
                    );
                    if let Some(c) = ui.clip_rect(btn_rect) {
                        self.targets.push((c, Action::OpenProject(i)));
                    }
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

            ui.set_clip(None);
            let pager_y = h - 42.0;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, pager_y, 102.0, 32.0],
                "Previous",
                Action::Previous,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 110.0, pager_y, 102.0, 32.0],
                "Next",
                Action::Next,
                false,
            );
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            ui.text(
                Face::SemiBold,
                19.0,
                content_x,
                62.0 + slide_y,
                MY3_TEXT,
                "Installed Add-ons",
            );
            let top = 96.0;
            let bottom = h - 46.0;
            let row_height = if compact { 92.0 } else { 62.0 };
            let total = Kind::ALL
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    44.0 + if self.catalog.sections[i] {
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
            ui.set_clip(Some([
                content_x - 2.0,
                top,
                content_w + 4.0,
                (bottom - top).max(1.0),
            ]));
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
                let sec_w = (content_w - 118.0).max(120.0);
                self.list_targets
                    .push(([content_x, y, sec_w, 36.0], Action::ToggleSection(section)));
                self.list_targets.push((
                    [content_x + sec_w + 8.0, y, 110.0, 36.0],
                    Action::Import(kind),
                ));
                if y + 36.0 >= top && y <= bottom {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x, y, sec_w, 36.0],
                        &label,
                        Action::ToggleSection(section),
                        self.catalog.sections[section],
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [content_x + sec_w + 8.0, y, 110.0, 36.0],
                        "Import",
                        Action::Import(kind),
                        false,
                    );
                }
                offset += 44.0;
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
                        content_x + content_w - 230.0
                    };
                    let remove_x = content_x + content_w - 106.0;
                    self.list_targets.push((
                        [toggle_x, y, 108.0, row_height - 6.0],
                        Action::ToggleLocal(i),
                    ));
                    self.list_targets.push((
                        [remove_x, y, 94.0, row_height - 6.0],
                        Action::DeleteLocal(i),
                    ));
                    if y + row_height < top || y > bottom {
                        continue;
                    }
                    ui.rounded(content_x, y, content_w, row_height - 6.0, 14.0, MY3_CARD_BG);
                    ui.border(
                        content_x,
                        y,
                        content_w,
                        row_height - 6.0,
                        14.0,
                        MY3_CARD_BORDER,
                    );
                    let title = ui.fit_text(
                        Face::SemiBold,
                        15.0,
                        &item.title,
                        if compact {
                            content_w - 28.0
                        } else {
                            content_w - 250.0
                        },
                    );
                    ui.text(
                        Face::SemiBold,
                        15.0,
                        content_x + 16.0,
                        y + 8.0,
                        MY3_TEXT,
                        &title,
                    );
                    ui.text(
                        Face::Regular,
                        12.5,
                        content_x + 16.0,
                        y + 30.0,
                        MY3_MUTED,
                        kind.label(),
                    );
                    toggle(
                        &mut ui,
                        &mut self.targets,
                        [toggle_x, y + if compact { 44.0 } else { 8.0 }, 108.0, 38.0],
                        item.enabled,
                        Action::ToggleLocal(i),
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [remove_x, y + if compact { 44.0 } else { 9.0 }, 94.0, 36.0],
                        "Remove",
                        Action::DeleteLocal(i),
                        false,
                    );
                }
            }
            ui.set_clip(None);
        } else {
            self.draw_settings_page(&mut ui, content_x, content_w, h, compact, slide_y);
        }

        // Bottom status strip when not in a full-screen progress dialog.
        let in_progress_dialog = (page == Page::Play && self.worker.is_some())
            || (self.catalog.downloading.is_some() && self.catalog.busy());
        if !in_progress_dialog {
            let status = self.error.as_deref().unwrap_or(&self.catalog.status);
            if !status.is_empty() && !page.is_catalog() {
                let fitted =
                    ui.fit_text(Face::Regular, 12.5, status, (content_w - 116.0).max(60.0));
                ui.text(Face::Regular, 12.5, content_x, h - 32.0, MY3_MUTED, &fitted);
            }
            if page == Page::Play
                || (page == Page::Settings
                    && self.catalog.settings_category == SettingsCategory::Overview)
            {
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 118.0, h - 38.0, 92.0, 28.0],
                    "GitHub",
                    Action::Repository,
                    false,
                );
            }
        }

        // Material You 3 Project Detail Modal (Versions, Modrinth Description, Gallery Screenshots)
        if self.catalog.project_modal.is_some() {
            self.draw_project_modal(&mut ui, w, h);
        }

        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.config.last_name_changed_label(),
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
            && self.catalog.project_modal.is_none()
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
                "profile_name" => self.catalog.config.custom_profile_name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => self.catalog.query.clone(),
            };
            for ([x, y, w, _], action) in &self.targets {
                if *action == Action::Field(name)
                    || (name == "render_scale" && *action == Action::RenderScale)
                {
                    let pad = if name == "search" { 35.0 } else { 14.0 };
                    let text = ui.fit_text(Face::Regular, 13.5, &value, w - pad - 12.0);
                    let caret_x = x + pad + ui.measure(Face::Regular, 13.5, &text);
                    ui.rounded(
                        caret_x.min(x + w - 10.0),
                        y + 9.0,
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

    /// Material You 3 Project Detail Dialog showing Modrinth PFP, Description,
    /// Gallery Screenshots, and selectable Versions to Download.
    fn draw_project_modal(&mut self, ui: &mut Ui, w: f32, h: f32) {
        // The install label asks the catalog about the project the modal is showing, so work it out
        // before the modal is borrowed mutably for drawing.
        let Some(install_label) = self.catalog.project_modal.as_ref().map(|modal| {
            let version = modal
                .versions
                .get(modal.selected_version)
                .map_or_else(|| "Latest".to_owned(), |v| v.number.clone());
            if let Some(bundled) = modal.bundled_id {
                if self.catalog.bundled_enabled.contains(bundled) {
                    "Installed (Tap to Disable)".to_owned()
                } else {
                    format!("Enable Built-in ({version})")
                }
            } else if let Some(state) = self.catalog.download_label(&modal.project) {
                state
            } else {
                format!("Download {} ({version})", modal.project.kind.label())
            }
        }) else {
            return;
        };
        let Some(modal) = &mut self.catalog.project_modal else {
            return;
        };
        self.targets.clear();
        self.list_targets.clear();

        // Scrim backdrop
        let alpha = (185.0 * self.anim.modal_anim.clamp(0.2, 1.0)) as u8;
        ui.rounded(
            0.0,
            0.0,
            w,
            h,
            0.0,
            horizon_client::hud::rgba(6, 5, 12, alpha),
        );

        let scale_in = 0.94 + 0.06 * self.anim.modal_anim.clamp(0.0, 1.0);
        let mw = ((w - 44.0).min(640.0) * scale_in).max(280.0);
        let mh = ((h - 36.0).min(385.0) * scale_in).max(230.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5;

        ui.rounded(mx, my, mw, mh, 22.0, MY3_SURFACE);
        ui.border(mx, my, mw, mh, 22.0, MY3_BORDER);

        // Modal Top Header (PFP + Title + Author/Downloads + Close Button)
        let icon_drawn = if let Some((iw, ih, rgba)) = self.catalog.icons.get(&modal.project.id) {
            ui.image(
                &modal.project.id,
                *iw,
                *ih,
                rgba,
                mx + 18.0,
                my + 16.0,
                50.0,
                50.0,
            )
        } else {
            false
        };
        if !icon_drawn {
            ui.rounded(
                mx + 18.0,
                my + 16.0,
                50.0,
                50.0,
                14.0,
                color::hex(0x27213C, 255),
            );
            ui.icon(
                page_icon(self.catalog.page),
                mx + 31.0,
                my + 29.0,
                24.0,
                MY3_ACCENT,
            );
        }

        let close_rect = [mx + mw - 54.0, my + 16.0, 38.0, 38.0];
        icon_action_button(
            ui,
            &mut self.targets,
            close_rect,
            MY3_SURFACE_HIGH,
            Icon::Close,
            Action::CloseProjectModal,
        );

        let title_max = (mw - 146.0).max(100.0);
        let fitted_title = ui.fit_text(Face::SemiBold, 18.5, &modal.project.title, title_max);
        ui.text(
            Face::SemiBold,
            18.5,
            mx + 78.0,
            my + 16.0,
            MY3_TEXT,
            &fitted_title,
        );

        let mut meta_parts = vec![format!("by {}", modal.author)];
        if modal.downloads > 0 {
            meta_parts.push(format!("{} downloads", format_downloads(modal.downloads)));
        }
        if !modal.license.is_empty() {
            meta_parts.push(modal.license.clone());
        }
        if modal.loading {
            meta_parts.push("Syncing Modrinth details...".into());
        }
        let meta_line = meta_parts.join("  •  ");
        let fitted_meta = ui.fit_text(Face::Regular, 12.5, &meta_line, title_max);
        ui.text(
            Face::Regular,
            12.5,
            mx + 78.0,
            my + 41.0,
            MY3_MUTED,
            &fitted_meta,
        );

        // Divider under modal header
        ui.rounded(mx + 16.0, my + 74.0, mw - 32.0, 1.0, 0.0, MY3_BORDER);

        // Scrollable Body Area (Description + Gallery Screenshots + Versions List)
        let body_top = my + 82.0;
        let body_bottom = my + mh - 62.0;
        let inner_x = mx + 18.0;
        let inner_w = mw - 36.0;

        let desc_lines =
            wrap_text_lines(ui, Face::Regular, 13.0, &modal.description, inner_w - 8.0);
        let desc_h = (desc_lines.len() as f32 * 18.5).max(24.0);
        let gallery_h = if modal.screenshots.is_empty() {
            0.0
        } else {
            118.0
        };
        let versions_h = 28.0 + (modal.versions.len().min(6) as f32) * 38.0;
        let total_content_h = 24.0 + desc_h + gallery_h + versions_h + 16.0;
        let max_scroll = (total_content_h - (body_bottom - body_top)).max(0.0);
        modal.scroll = modal.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([body_top, body_bottom, max_scroll]);

        ui.set_clip(Some([
            mx + 4.0,
            body_top,
            mw - 8.0,
            (body_bottom - body_top).max(1.0),
        ]));
        let mut cy = body_top - modal.scroll;

        // Section 1: About / Description
        if cy + 20.0 >= body_top && cy <= body_bottom {
            ui.text(
                Face::SemiBold,
                14.0,
                inner_x,
                cy,
                MY3_ACCENT,
                "About Project",
            );
        }
        cy += 22.0;
        for line in &desc_lines {
            if cy + 18.0 >= body_top && cy <= body_bottom {
                ui.text(Face::Regular, 13.0, inner_x, cy, MY3_TEXT, line);
            }
            cy += 18.5;
        }
        cy += 8.0;

        // Section 2: Screenshots (from Modrinth gallery)
        if !modal.screenshots.is_empty() {
            if cy + 20.0 >= body_top && cy <= body_bottom {
                ui.text(Face::SemiBold, 14.0, inner_x, cy, MY3_ACCENT, "Screenshots");
            }
            cy += 22.0;
            let count = modal.screenshots.len().min(3) as f32;
            let gap = 10.0;
            let card_w = ((inner_w - gap * (count - 1.0)) / count).clamp(110.0, 200.0);
            let card_h = 84.0;
            if cy + card_h >= body_top && cy <= body_bottom {
                for (idx, (caption, sw, sh, rgba)) in modal.screenshots.iter().take(3).enumerate() {
                    let sx = inner_x + idx as f32 * (card_w + gap);
                    ui.rounded(sx, cy, card_w, card_h, 12.0, MY3_CARD_BG);
                    ui.border(sx, cy, card_w, card_h, 12.0, MY3_CARD_BORDER);
                    let key = format!("shot:{}:{idx}", modal.project.id);
                    let _ = ui.image(
                        &key,
                        *sw,
                        *sh,
                        rgba,
                        sx + 3.0,
                        cy + 3.0,
                        card_w - 6.0,
                        card_h - 22.0,
                    );
                    let cap_fit = ui.fit_text(Face::Regular, 11.0, caption, card_w - 12.0);
                    ui.text(
                        Face::Regular,
                        11.0,
                        sx + 6.0,
                        cy + card_h - 17.0,
                        MY3_MUTED,
                        &cap_fit,
                    );
                }
            }
            cy += card_h + 12.0;
        }

        // Section 3: Selectable Versions to Download
        if cy + 20.0 >= body_top && cy <= body_bottom {
            ui.text(
                Face::SemiBold,
                14.0,
                inner_x,
                cy,
                MY3_ACCENT,
                "Available Versions",
            );
        }
        cy += 24.0;
        for (v_idx, ver) in modal.versions.iter().take(6).enumerate() {
            let selected = modal.selected_version == v_idx;
            let row_rect = [inner_x, cy, inner_w, 34.0];
            self.list_targets
                .push((row_rect, Action::SelectModalVersion(v_idx)));
            if cy + 34.0 >= body_top && cy <= body_bottom {
                ui.rounded(
                    inner_x,
                    cy,
                    inner_w,
                    34.0,
                    10.0,
                    if selected {
                        MY3_ACCENT_SOFT
                    } else {
                        MY3_CARD_BG
                    },
                );
                ui.border(
                    inner_x,
                    cy,
                    inner_w,
                    34.0,
                    10.0,
                    if selected {
                        MY3_ACCENT
                    } else {
                        MY3_CARD_BORDER
                    },
                );
                let ver_title = format!(
                    "{}v{}  •  MC {}  ({})",
                    if selected { "● " } else { "○ " },
                    ver.number,
                    ver.game_versions.first().map_or("26.3", String::as_str),
                    if ver.loaders.is_empty() {
                        modal.project.source.label().to_owned()
                    } else {
                        ver.loaders.join(", ")
                    }
                );
                let fitted_v = ui.fit_text(Face::Medium, 13.0, &ver_title, inner_w - 24.0);
                ui.text(
                    Face::Medium,
                    13.0,
                    inner_x + 12.0,
                    cy + 7.0,
                    if selected { MY3_TEXT } else { MY3_MUTED },
                    &fitted_v,
                );
                if let Some(c) = ui.clip_rect(row_rect) {
                    self.targets.push((c, Action::SelectModalVersion(v_idx)));
                }
            }
            cy += 38.0;
        }

        ui.set_clip(None);
        // Modal Bottom Action Bar
        ui.rounded(mx + 16.0, my + mh - 56.0, mw - 32.0, 1.0, 0.0, MY3_BORDER);

        button(
            ui,
            &mut self.targets,
            [mx + 18.0, my + mh - 48.0, 108.0, 38.0],
            "Close",
            Action::CloseProjectModal,
            false,
        );
        button(
            ui,
            &mut self.targets,
            [mx + 136.0, my + mh - 48.0, mw - 154.0, 38.0],
            &install_label,
            Action::InstallModalProject,
            true,
        );
    }

    /// Revamped Settings tab matching `45062.jpg` with 2-column MY3 category cards
    /// and dedicated sub-pages for Video, Launcher, Controls, Experimental, and Diagnostics.
    fn draw_settings_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        let cat = self.catalog.settings_category;
        if cat == SettingsCategory::Overview {
            ui.text(
                Face::SemiBold,
                20.0,
                content_x,
                62.0 + slide_y,
                MY3_TEXT,
                "Settings",
            );

            let cards: [(Icon, &str, &str, Action); 6] = [
                (
                    Icon::Monitor,
                    "Video Settings",
                    "Rendering, resolution and performance",
                    Action::SettingsCat(SettingsCategory::Video),
                ),
                (
                    Icon::Gear,
                    "Launcher Settings",
                    "Profiles, orientation and app preferences",
                    Action::SettingsCat(SettingsCategory::Launcher),
                ),
                (
                    Icon::Gamepad,
                    "Controls",
                    "HUD touch layout, buttons, sensitivity and profiles",
                    Action::SettingsCat(SettingsCategory::Controls),
                ),
                (
                    Icon::Flask,
                    "Experimental Stuff",
                    "Unstable features. We put the weird shit here.",
                    Action::SettingsCat(SettingsCategory::Experimental),
                ),
                (
                    Icon::Terminal,
                    "Diagnostics",
                    "Logs, device info and troubleshooting",
                    Action::SettingsCat(SettingsCategory::Diagnostics),
                ),
                (
                    Icon::MicrosoftGrid,
                    "Account",
                    "Microsoft and Xbox sign-in",
                    Action::Account,
                ),
            ];

            let grid_top = 98.0 + slide_y;
            let gap_x = 14.0;
            let gap_y = 12.0;
            let cols = if compact { 1 } else { 2 };
            let card_w = if cols == 2 {
                (content_w - gap_x) * 0.5
            } else {
                content_w
            };
            let card_h = ((h - grid_top - 52.0 - gap_y * 2.0) / 3.0).clamp(68.0, 92.0);

            for (idx, (icon, title, subtitle, action)) in cards.into_iter().enumerate() {
                let col = idx % cols;
                let row = idx / cols;
                let cx = content_x + col as f32 * (card_w + gap_x);
                let cy = grid_top + row as f32 * (card_h + gap_y);
                let rect = [cx, cy, card_w, card_h];

                ui.rounded(cx, cy, card_w, card_h, 16.0, MY3_CARD_BG);
                ui.border(cx, cy, card_w, card_h, 16.0, MY3_CARD_BORDER);

                // Left vector icon + vertical divider line matching 45062.jpg
                let icon_size = 20.0;
                ui.icon(
                    icon,
                    cx + 20.0,
                    cy + (card_h - icon_size) * 0.5,
                    icon_size,
                    color::hex(0xD5CEE8, 255),
                );
                ui.rounded(
                    cx + 54.0,
                    cy + card_h * 0.26,
                    1.5,
                    card_h * 0.48,
                    0.75,
                    color::hex(0x2E2B40, 255),
                );

                let text_x = cx + 68.0;
                let text_max = (card_w - 82.0).max(60.0);
                let fitted_title = ui.fit_text(Face::SemiBold, 15.5, title, text_max);
                let fitted_sub = ui.fit_text(Face::Regular, 12.5, subtitle, text_max);
                let text_block_y = cy + (card_h - 40.0) * 0.5;
                ui.text(
                    Face::SemiBold,
                    15.5,
                    text_x,
                    text_block_y,
                    MY3_TEXT,
                    &fitted_title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    text_x,
                    text_block_y + 22.0,
                    MY3_MUTED,
                    &fitted_sub,
                );

                self.list_targets.push((rect, action));
                self.targets.push((rect, action));
            }
            return;
        }

        // Sub-category Header Bar with Back button
        let header_y = 60.0 + slide_y;
        button(
            ui,
            &mut self.targets,
            [content_x, header_y, 88.0, 32.0],
            "← Back",
            Action::SettingsCat(SettingsCategory::Overview),
            false,
        );
        let fitted_cat = ui.fit_text(
            Face::SemiBold,
            18.5,
            cat.title(),
            (content_w - 104.0).max(80.0),
        );
        ui.text(
            Face::SemiBold,
            18.5,
            content_x + 100.0,
            header_y + 3.0,
            MY3_TEXT,
            &fitted_cat,
        );

        let list_top = 102.0;
        let list_bottom = h - 42.0;
        let card_h = 58.0;
        let step_y = 66.0;

        let game_home = self.catalog.config.game_home(&self.home);
        let opts = horizon_client::settings::Settings::load(&game_home.join("options.txt"));
        let touch_cfg =
            horizon_client::touch::Settings::load_from(&game_home.join("horizon-touch.json"));

        match cat {
            SettingsCategory::Video => {
                let c = &self.catalog.config;
                let count = 8;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let renderer_str = format!("{} v", c.renderer.label());
                let res_str = if self.catalog.editing == Some("render_scale") {
                    self.catalog.render_scale_text.clone()
                } else {
                    format!("{}% v", c.render_scale.percent())
                };
                let hz_str = format!("{} v", c.device.display_label());
                let aspect_str = format!("{} v", c.screen.aspect.label());
                let orient_str = format!("{} v", c.orientation.label());
                let ui_scale_str = format!("{} v", c.ui_scale_label());

                let rows: [(&str, &str, SettingControl<'_>); 8] = [
                    (
                        "Rendering backend (Vulkan / GLES)",
                        "Hardware GPU driver pipeline used when launching Minecraft",
                        SettingControl::Button(&renderer_str, Action::Renderer, true),
                    ),
                    (
                        "Client resolution scale",
                        "3D framebuffer resolution percentage (lower boosts FPS on high-DPI phones)",
                        SettingControl::Button(&res_str, Action::RenderScale, false),
                    ),
                    (
                        "Display refresh rate",
                        "Preferred Android screen refresh rate (60Hz / 90Hz / 120Hz)",
                        SettingControl::Button(&hz_str, Action::DisplayHz, false),
                    ),
                    (
                        "Display aspect ratio",
                        "Native fullscreen, 16:9 widescreen, 4:3 classic or square",
                        SettingControl::Button(&aspect_str, Action::Aspect, false),
                    ),
                    (
                        "Screen orientation",
                        "Sensor landscape, locked landscape or portrait layout",
                        SettingControl::Button(&orient_str, Action::Orientation, false),
                    ),
                    (
                        "Immersive fullscreen",
                        "Hide Android status and navigation bars during play",
                        SettingControl::Toggle(c.screen.immersive, Action::Immersive),
                    ),
                    (
                        "Avoid camera cutout / notch",
                        "Inset HUD away from punch-hole cameras and curved display edges",
                        SettingControl::Toggle(c.screen.avoid_cutout, Action::Cutout),
                    ),
                    (
                        "Launcher UI scale (Phone DPI)",
                        "Adapts launcher cards, text and buttons to your screen DPI",
                        SettingControl::Button(&ui_scale_str, Action::UiScale, false),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Launcher => {
                let c = &self.catalog.config;
                let count = 12;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let renderer_str = format!("{} v", c.renderer.label());
                let ui_scale_str = format!("{} v", c.ui_scale_label());
                let profile_str = format!("Profile {} v", c.profile + 1);
                let orient_str = format!("{} v", c.orientation.label());
                let aspect_str = format!("{} v", c.screen.aspect.label());
                let input_str = format!("{} v", c.input_host.label());
                let worker_str = if c.worker_limit == 0 {
                    "Auto (All cores) v".to_owned()
                } else {
                    format!("{} threads v", c.worker_limit)
                };
                let cache_str = if c.cache_budget_mib == 0 {
                    "Default (80 MiB) v".to_owned()
                } else {
                    format!("{} MiB v", c.cache_budget_mib)
                };
                let server_str = if c.server.is_empty() {
                    "Singleplayer / Tap to set".to_owned()
                } else {
                    c.server.clone()
                };

                let rows: [(&str, &str, SettingControl<'_>); 12] = [
                    (
                        "Rendering backend (Vulkan / GLES)",
                        "Hardware GPU driver pipeline used when launching Minecraft",
                        SettingControl::Button(&renderer_str, Action::Renderer, true),
                    ),
                    (
                        "Launcher UI scale (Phone DPI)",
                        "Adapts launcher cards, text and buttons to your screen DPI",
                        SettingControl::Button(&ui_scale_str, Action::UiScale, false),
                    ),
                    (
                        "Active game profile",
                        "Each profile isolates its own worlds, mods and options",
                        SettingControl::Button(&profile_str, Action::Profile, false),
                    ),
                    (
                        "Screen orientation",
                        "Sensor landscape, locked landscape or portrait layout",
                        SettingControl::Button(&orient_str, Action::Orientation, false),
                    ),
                    (
                        "Display aspect ratio",
                        "Native fullscreen, 16:9 widescreen, 4:3 classic or square",
                        SettingControl::Button(&aspect_str, Action::Aspect, false),
                    ),
                    (
                        "Immersive fullscreen",
                        "Hide Android status and navigation bars during play",
                        SettingControl::Toggle(c.screen.immersive, Action::Immersive),
                    ),
                    (
                        "Avoid camera cutout / notch",
                        "Inset HUD away from punch-hole cameras and curved edges",
                        SettingControl::Toggle(c.screen.avoid_cutout, Action::Cutout),
                    ),
                    (
                        "Input host mode",
                        "Switch between touchscreen HUD and physical mouse/keyboard",
                        SettingControl::Button(&input_str, Action::InputHost, false),
                    ),
                    (
                        "Chunk worker thread ceiling",
                        "Background worldgen and chunk meshing worker limit",
                        SettingControl::Button(&worker_str, Action::WorkerLimit, false),
                    ),
                    (
                        "Reusable RAM cache budget",
                        "Pooled audio & mesh buffer cache ceiling",
                        SettingControl::Button(&cache_str, Action::CacheBudget, false),
                    ),
                    (
                        "Quick-join server address",
                        "Optional server IP:port to connect immediately on Play",
                        SettingControl::Button(&server_str, Action::Field("server"), false),
                    ),
                    (
                        "Open game & world folders",
                        "Manage saves, resource packs and mods in Android Files",
                        SettingControl::Button("Open Files", Action::GameFiles, false),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Controls => {
                let count = 15;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let style = touch_cfg.control_style;
                let is_mcpe = style == horizon_client::touch::ControlStyle::Mcpe;
                let is_goy = style == horizon_client::touch::ControlStyle::GoyControl;
                let sys_name = style.label();
                let sys_presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                let preset_label = format!(
                    "{} ({} presets) v",
                    touch_cfg.preset_name,
                    sys_presets.len()
                );
                let preset_sub = format!(
                    "{}  •  controls/{}/{}.json{}",
                    touch_cfg.preset_description,
                    style.storage_subdir(),
                    touch_cfg.preset_id,
                    if touch_cfg.is_default_preset() {
                        " (Protected Default)"
                    } else {
                        ""
                    }
                );
                let editor_title = format!("{} Control Editor", sys_name);
                let editor_sub = match style {
                    horizon_client::touch::ControlStyle::Mcpe => {
                        "Customize MCPE controls with exact Minecraft sprites, size, opacity & alignment"
                    }
                    horizon_client::touch::ControlStyle::GoyControl => {
                        "Customize GoyControl buttons, custom keys, toggle, swipe & pass-through"
                    }
                };
                let preset_row_title = format!("{} Preset Selector", sys_name);
                let copy_title = format!("Copy {} Preset ('{}')", sys_name, touch_cfg.preset_name);
                let copy_btn_label = if self.catalog.editing == Some("profile_name")
                    && self.catalog.config.preset_edit_mode == "copy"
                {
                    "Typing Name..."
                } else if self.catalog.editing == Some("profile_desc")
                    && self.catalog.config.preset_edit_mode == "copy"
                {
                    "Typing Desc..."
                } else {
                    "Copy Preset"
                };
                let edit_meta_title = format!("Edit {} Preset Name & Description", sys_name);
                let edit_meta_sub = if touch_cfg.is_default_preset() {
                    "Default layout is protected and cannot be renamed or overwritten; use Copy Preset"
                } else {
                    "Pencil / Edit: rename or update description of the selected user preset"
                };
                let edit_meta_btn = if touch_cfg.is_default_preset() {
                    "Protected"
                } else if self.catalog.editing == Some("profile_name")
                    && self.catalog.config.preset_edit_mode == "edit"
                {
                    "Typing Name..."
                } else if self.catalog.editing == Some("profile_desc")
                    && self.catalog.config.preset_edit_mode == "edit"
                {
                    "Typing Desc..."
                } else {
                    "Edit (Pencil)"
                };

                let mouse_sens = format!("{:.0}%", opts.sensitivity * 200.0);
                let touch_sens = format!("{:.0}%", touch_cfg.sensitivity * 100.0);
                let vmouse_spd = format!("{:.0}%", touch_cfg.virtual_mouse_sensitivity * 100.0);
                let btn_scale = format!("{:.0}%", touch_cfg.button_scale * 100.0);
                let scheme_str = touch_cfg.mode.label();
                let invert_y_active = opts.invert_y || touch_cfg.invert_y;

                let rows: [(&str, &str, SettingControl<'_>); 15] = [
                    (
                        "Control Mode: MCPE (Default)",
                        "Vanilla Minecraft/MCPE-style touch controls (default on fresh install)",
                        SettingControl::Button(
                            if is_mcpe {
                                "● Active"
                            } else {
                                "○ Select MCPE"
                            },
                            Action::SelectControlMode(horizon_client::touch::ControlStyle::Mcpe),
                            is_mcpe,
                        ),
                    ),
                    (
                        "Control Mode: GoyControl",
                        "Independent custom touch control system with its own layouts & editor",
                        SettingControl::Button(
                            if is_goy {
                                "● Active"
                            } else {
                                "○ Select GoyControl"
                            },
                            Action::SelectControlMode(
                                horizon_client::touch::ControlStyle::GoyControl,
                            ),
                            is_goy,
                        ),
                    ),
                    (
                        &editor_title,
                        editor_sub,
                        SettingControl::Button(
                            "Customize Controls",
                            Action::OpenControlEditor,
                            true,
                        ),
                    ),
                    (
                        &preset_row_title,
                        &preset_sub,
                        SettingControl::Button(&preset_label, Action::ControlProfileChooser, false),
                    ),
                    (
                        &copy_title,
                        "Duplicate selected layout into a new independent preset with custom name & description",
                        SettingControl::Button(copy_btn_label, Action::CopyActivePreset, false),
                    ),
                    (
                        &edit_meta_title,
                        edit_meta_sub,
                        SettingControl::Button(edit_meta_btn, Action::EditActivePresetMeta, false),
                    ),
                    (
                        "Sensitivity (Touch Look Speed)",
                        "Direct finger drag camera rotation speed multiplier",
                        SettingControl::Stepper(
                            &touch_sens,
                            Action::TouchSensDelta(-0.10),
                            Action::TouchSensDelta(0.10),
                        ),
                    ),
                    (
                        "Invert Y-Axis",
                        "Inverts vertical camera look direction for touch and mouse",
                        SettingControl::Toggle(invert_y_active, Action::ToggleInvertY),
                    ),
                    (
                        "Split Touch (Aim & Look)",
                        "Separates aiming and camera look with center crosshair",
                        SettingControl::Toggle(
                            touch_cfg.split_controls,
                            Action::ToggleSplitControls,
                        ),
                    ),
                    (
                        "Virtual Mouse Pointer Speed",
                        "Zero-lag virtual cursor speed in menus, inventories and HUD",
                        SettingControl::Stepper(
                            &vmouse_spd,
                            Action::VirtualMouseSpeedDelta(-0.15),
                            Action::VirtualMouseSpeedDelta(0.15),
                        ),
                    ),
                    (
                        "HUD Button Scale",
                        "Global size multiplier for on-screen touch buttons",
                        SettingControl::Stepper(
                            &btn_scale,
                            Action::ButtonScaleDelta(-0.10),
                            Action::ButtonScaleDelta(0.10),
                        ),
                    ),
                    (
                        "Movement Control Scheme",
                        "Switch between classic D-Pad buttons and analog thumb joystick",
                        SettingControl::Button(scheme_str, Action::ToggleControlScheme, false),
                    ),
                    (
                        "Left-Handed HUD Mirroring",
                        "Mirrors movement controls to the right side of the screen",
                        SettingControl::Toggle(touch_cfg.left_handed, Action::ToggleLeftHanded),
                    ),
                    (
                        "Gyroscope Aiming",
                        "Tilt your phone for fine bow and crosshair aiming adjustments",
                        SettingControl::Toggle(touch_cfg.gyroscope, Action::ToggleGyro),
                    ),
                    (
                        "3D Camera Base Sensitivity",
                        "Base in-game 3D look sensitivity stored in options.txt",
                        SettingControl::Stepper(
                            &mouse_sens,
                            Action::MouseSensDelta(-0.05),
                            Action::MouseSensDelta(0.05),
                        ),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Experimental => {
                let c = &self.catalog.config;
                let count = 7;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let rows: [(&str, &str, SettingControl<'_>); 7] = [
                    (
                        "Super Performance Mode",
                        "Aggressive chunk culling, fast clouds, minimal particles & Vulkan batching",
                        SettingControl::Toggle(c.super_perf_mode, Action::ToggleSuperPerf),
                    ),
                    (
                        "Battery Performance Mode",
                        "Caps framerate to 60 FPS with VSync and reduces background worker wakeups",
                        SettingControl::Toggle(c.battery_perf_mode, Action::ToggleBatteryPerf),
                    ),
                    (
                        "Max FPS Mode (Uncapped 260 FPS)",
                        "Disables VSync, unlocks framerate ceiling and requests maximum display Hz",
                        SettingControl::Toggle(c.max_fps_mode, Action::ToggleMaxFpsMode),
                    ),
                    (
                        "Zero-Latency Touch Pipeline",
                        "Bypasses redundant pointer filtering for instant camera & virtual mouse response",
                        SettingControl::Toggle(
                            c.zero_latency_touch,
                            Action::ToggleZeroLatencyTouch,
                        ),
                    ),
                    (
                        "Fast Chunk Mesh Upload Boost",
                        "Prioritizes visible chunk uploads to eliminate singleplayer movement stutter",
                        SettingControl::Toggle(c.fast_chunk_upload, Action::ToggleFastChunkUpload),
                    ),
                    (
                        "Android ADPF Performance Hints",
                        "Sends real-time CPU thermal & frame-work hints to the Android scheduler",
                        SettingControl::Toggle(
                            c.device.performance_hints,
                            Action::PerformanceHints,
                        ),
                    ),
                    (
                        "Detailed GPU Pass Profiling",
                        "Records per-pass GPU timings in diagnostic reports (adds minor overhead)",
                        SettingControl::Toggle(c.device.gpu_profile, Action::GpuProfile),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Diagnostics => {
                let c = &self.catalog.config;
                let count = 5;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let rows: [(&str, &str, SettingControl<'_>); 5] = [
                    (
                        "Inspect logs & device summary",
                        "Check last handled error, renderer info and runtime health",
                        SettingControl::Button("View Summary", Action::Logs, true),
                    ),
                    (
                        "Export diagnostic report (.zip)",
                        "Packages logs and device info into a local ZIP for troubleshooting",
                        SettingControl::Button("Export ZIP", Action::ExportDiagnostics, false),
                    ),
                    (
                        "Record gameplay performance",
                        "Captures frame-time and chunk-build telemetry during Play (up to 30 min)",
                        SettingControl::Toggle(c.record_performance, Action::RecordPerformance),
                    ),
                    (
                        "Export raw performance data (.jsonl)",
                        "Share or inspect recorded frame telemetry in Android Files",
                        SettingControl::Button("Export JSONL", Action::ExportPerformance, false),
                    ),
                    (
                        "World backup & file manager",
                        "Open worlds folder in system Files to back up or copy saves",
                        SettingControl::Button("World Files", Action::WorldFiles, false),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            // Not reachable from the category grid yet.
            SettingsCategory::Overview | SettingsCategory::Account => {}
        }
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
            .rev()
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
            Some(Action::SettingsCat(cat)) => {
                self.catalog.settings_category = cat;
                self.catalog.scroll = 0.0;
            }
            Some(Action::OpenProject(index)) => {
                if let Some(project) = self.catalog.results.get(index).cloned() {
                    self.catalog
                        .open_project_modal(&self.home, project, None, Some(index));
                }
            }
            Some(Action::OpenBundledProject(id)) => {
                if let Some((_, title, desc)) = horizon_client::mod_list::ANDROID_BUNDLED
                    .iter()
                    .find(|(bid, _, _)| *bid == id)
                {
                    let project = crate::browser::Project {
                        source: crate::browser::SourceId::Horizon,
                        id: (*id).to_owned(),
                        slug: (*id).to_owned(),
                        title: (*title).to_owned(),
                        summary: (*desc).to_owned(),
                        author: "Horizon Built-in".into(),
                        downloads: 0,
                        icon: None,
                        page: String::new(),
                        kind: crate::browser::Kind::Mod,
                        license: "MIT / Apache-2.0".into(),
                        game_versions: vec![crate::browser::GAME.into()],
                        loaders: vec!["horizon".into()],
                    };
                    self.catalog
                        .open_project_modal(&self.home, project, Some(id), None);
                }
            }
            Some(Action::CloseProjectModal) => {
                self.catalog.project_modal = None;
                self.catalog.modal_worker = None;
            }
            Some(Action::SelectModalVersion(v_idx)) => {
                if let Some(modal) = &mut self.catalog.project_modal {
                    modal.selected_version = v_idx;
                }
            }
            Some(Action::InstallModalProject) => {
                if let Some(modal) = self.catalog.project_modal.take() {
                    self.catalog.modal_worker = None;
                    if let Some(id) = modal.bundled_id {
                        self.toggle_bundled_mod(id)?;
                    } else if let Some(idx) = modal.catalog_index {
                        self.catalog.install(&self.home, idx);
                    }
                }
            }
            Some(Action::UiScale) => self.open_chooser(chooser::Kind::UiScale),
            Some(Action::ChooseUiScale(pct)) => {
                self.catalog.config.ui_scale_percent = pct;
                horizon_client::ui_theme::set_ui_scale_percent(pct);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = format!(
                    "Launcher UI scale set to {}.",
                    self.catalog.config.ui_scale_label()
                );
            }
            // Video settings mutations
            Some(Action::RenderDistDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.render_distance = (s.render_distance + delta).clamp(2, 32);
                });
            }
            Some(Action::SimDistDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.simulation_distance = (s.simulation_distance + delta).clamp(5, 32);
                });
            }
            Some(Action::FovDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.fov = (s.fov + delta).clamp(30.0, 110.0);
                });
            }
            Some(Action::CycleMaxFps) => {
                self.modify_game_settings(|s| {
                    s.max_fps = match s.max_fps {
                        0..=45 => 60,
                        46..=75 => 90,
                        76..=105 => 120,
                        106..=200 => 260,
                        _ => 30,
                    };
                });
            }
            Some(Action::CycleGraphicsPreset) => {
                self.modify_game_settings(|s| {
                    s.graphics_preset = match s.graphics_preset.as_str() {
                        "fast" => "fancy".into(),
                        "fancy" => "fabulous".into(),
                        _ => "fast".into(),
                    };
                });
            }
            Some(Action::ToggleVsync) => {
                self.modify_game_settings(|s| s.vsync = !s.vsync);
            }
            Some(Action::ToggleSmoothLighting) => {
                self.modify_game_settings(|s| s.smooth_lighting = !s.smooth_lighting);
            }
            Some(Action::CycleClouds) => {
                self.modify_game_settings(|s| {
                    s.clouds = match s.clouds.as_str() {
                        "false" => "fast".into(),
                        "fast" => "true".into(),
                        _ => "false".into(),
                    };
                });
            }
            Some(Action::ToggleMenuBlur) => {
                self.modify_game_settings(|s| {
                    s.menu_blur = if s.menu_blur > 0 { 0 } else { 4 };
                });
            }
            // Experimental settings mutations
            Some(Action::ToggleSuperPerf) => {
                self.catalog.config.super_perf_mode = !self.catalog.config.super_perf_mode;
                if self.catalog.config.super_perf_mode {
                    self.catalog.config.battery_perf_mode = false;
                    self.catalog.config.renderer = crate::catalog::RendererMode::Vulkan;
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.super_perf_mode {
                    "Super Performance Mode enabled: fast chunk culling, minimal particles & Vulkan batching."
                } else {
                    "Super Performance Mode disabled."
                }
                .into();
            }
            Some(Action::ToggleBatteryPerf) => {
                self.catalog.config.battery_perf_mode = !self.catalog.config.battery_perf_mode;
                if self.catalog.config.battery_perf_mode {
                    self.catalog.config.super_perf_mode = false;
                    self.catalog.config.max_fps_mode = false;
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.battery_perf_mode {
                    "Battery Performance Mode enabled: 60 FPS cap with VSync and balanced workers."
                } else {
                    "Battery Performance Mode disabled."
                }
                .into();
            }
            Some(Action::ToggleMaxFpsMode) => {
                self.catalog.config.max_fps_mode = !self.catalog.config.max_fps_mode;
                if self.catalog.config.max_fps_mode {
                    self.catalog.config.battery_perf_mode = false;
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.max_fps_mode {
                    "Max FPS Mode enabled: 260 FPS uncapped with VSync disabled."
                } else {
                    "Max FPS Mode disabled."
                }
                .into();
            }
            Some(Action::ToggleZeroLatencyTouch) => {
                self.catalog.config.zero_latency_touch = !self.catalog.config.zero_latency_touch;
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.zero_latency_touch {
                    "Zero-Latency Touch Pipeline active."
                } else {
                    "Standard touch pipeline active."
                }
                .into();
            }
            Some(Action::ToggleFastChunkUpload) => {
                self.catalog.config.fast_chunk_upload = !self.catalog.config.fast_chunk_upload;
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.fast_chunk_upload {
                    "Fast Chunk Mesh Upload Boost active."
                } else {
                    "Standard chunk mesh upload active."
                }
                .into();
            }
            // Controls & Control Editor actions
            Some(Action::SelectControlMode(style)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                touch.switch_control_system(&game_home, style);
                self.catalog.status = format!(
                    "Control Mode set to {} (Active preset: '{}').",
                    style.label(),
                    touch.preset_name
                );
            }
            Some(Action::OpenControlEditor) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut tc = horizon_client::touch::TouchControls::default();
                tc.settings = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                tc.open_editor();
                self.control_editor = Some(tc);
            }
            Some(Action::CloseControlEditor) => {
                if let Some(mut tc) = self.control_editor.take() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    if !tc.settings.is_default_preset() {
                        let _ = tc.settings.save_replace_preset(&game_home);
                    } else {
                        let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
                    }
                    self.catalog.status = format!(
                        "Saved {} preset '{}'.",
                        tc.settings.control_style.label(),
                        tc.settings.preset_name
                    );
                }
            }
            Some(Action::ControlProfileChooser) => {
                self.open_chooser(chooser::Kind::ControlProfile);
            }
            Some(Action::ChooseControlProfile(idx)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let style = touch.control_style;
                let presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                if let Some(meta) = presets.get(idx as usize) {
                    touch.select_preset(&game_home, style, &meta.id);
                    self.catalog.status = format!(
                        "Selected {} preset '{}' ({}).",
                        style.label(),
                        meta.name,
                        meta.description
                    );
                }
                self.chooser = None;
            }
            Some(Action::CopyActivePreset) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                self.catalog.config.preset_edit_mode = "copy".into();
                self.catalog.config.custom_profile_name = format!("{} Copy", touch.preset_name);
                self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                self.catalog.editing = Some("profile_name");
                crate::platform::show(&self.catalog.config.custom_profile_name);
                self.catalog.status = format!(
                    "Step 1/2: Enter new {} preset name and press Enter.",
                    touch.control_style.label()
                );
            }
            Some(Action::EditActivePresetMeta) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                if touch.is_default_preset() {
                    self.catalog.status = format!(
                        "{} Default preset is protected and cannot be renamed or overwritten. Use 'Copy Preset' to create an editable copy.",
                        touch.control_style.label()
                    );
                } else {
                    self.catalog.config.preset_edit_mode = "edit".into();
                    self.catalog.config.custom_profile_name = touch.preset_name.clone();
                    self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                    self.catalog.editing = Some("profile_name");
                    crate::platform::show(&self.catalog.config.custom_profile_name);
                    self.catalog.status = format!(
                        "Step 1/2: Edit {} preset name and press Enter.",
                        touch.control_style.label()
                    );
                }
            }
            Some(Action::SaveControlProfileAsNew) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                self.catalog.config.preset_edit_mode = "copy".into();
                self.catalog.config.custom_profile_name = format!("{} Copy", touch.preset_name);
                self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                self.catalog.editing = Some("profile_name");
                crate::platform::show(&self.catalog.config.custom_profile_name);
                self.catalog.status = format!(
                    "Step 1/2: Enter new {} preset name and press Enter.",
                    touch.control_style.label()
                );
            }
            Some(Action::ExportControlProfile) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                match touch.export_profile(&self.home, &game_home, &touch.preset_id) {
                    Ok(path) => {
                        let file_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let _ = crate::platform::open_documents("reports");
                        self.catalog.status = format!(
                            "Exported '{}' to Reports & controls/{}/{}.json.",
                            file_name,
                            touch.control_style.storage_subdir(),
                            touch.preset_id
                        );
                    }
                    Err(e) => self.catalog.status = e.to_string(),
                }
            }
            Some(Action::ImportControlProfile) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let _ = std::fs::create_dir_all(
                    game_home
                        .join("controls")
                        .join(touch.control_style.storage_subdir()),
                );
                let _ = crate::platform::open_documents(crate::document_store::ROOT);
                self.catalog.status = format!(
                    "Place any .json preset file in controls/{}/ and select it under {} Preset Selector.",
                    touch.control_style.storage_subdir(),
                    touch.control_style.label()
                );
            }
            Some(Action::MouseSensDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.sensitivity = (s.sensitivity + delta).clamp(0.05, 1.0);
                });
            }
            Some(Action::TouchSensDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.sensitivity = (t.sensitivity + delta).clamp(0.25, 2.5);
                });
            }
            Some(Action::VirtualMouseSpeedDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_sensitivity =
                        (t.virtual_mouse_sensitivity + delta).clamp(0.25, 3.0);
                });
            }
            Some(Action::ButtonScaleDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.button_scale = (t.button_scale + delta).clamp(0.60, 1.60);
                });
            }
            Some(Action::ToggleControlScheme) => {
                self.modify_touch_settings(|t| {
                    t.mode = match t.mode {
                        horizon_client::touch::Mode::Crosshair => horizon_client::touch::Mode::Tap,
                        horizon_client::touch::Mode::Tap => horizon_client::touch::Mode::Dpad,
                        horizon_client::touch::Mode::Dpad => horizon_client::touch::Mode::Crosshair,
                    };
                });
            }
            Some(Action::ToggleSplitControls) => {
                self.modify_touch_settings(|t| t.split_controls = !t.split_controls);
            }
            Some(Action::ToggleSprintLatch) => {
                self.modify_touch_settings(|t| t.sprint_toggle = !t.sprint_toggle);
            }
            Some(Action::ToggleLeftHanded) => {
                self.modify_touch_settings(|t| t.left_handed = !t.left_handed);
            }
            Some(Action::ToggleInvertY) => {
                let mut next = false;
                self.modify_game_settings(|s| {
                    s.invert_y = !s.invert_y;
                    next = s.invert_y;
                });
                self.modify_touch_settings(|t| t.invert_y = next);
            }
            Some(Action::ToggleGyro) => {
                self.modify_touch_settings(|t| t.gyroscope = !t.gyroscope);
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
                self.toggle_bundled_mod(id)?;
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
                self.catalog.project_modal = None;
                self.catalog.modal_worker = None;
                if crate::platform::active() {
                    crate::platform::hide();
                }
                if page == Page::Settings && self.catalog.page == Page::Settings {
                    self.catalog.settings_category = SettingsCategory::Overview;
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
                self.catalog.config.touch_name_changed();
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
                    "profile_name" => &self.catalog.config.custom_profile_name,
                    "profile_desc" => &self.catalog.config.custom_profile_desc,
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

    fn toggle_bundled_mod(&mut self, id: &'static str) -> Result<()> {
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
            if enabled {
                "disabled"
            } else {
                "installed & enabled"
            }
        );
        Ok(())
    }

    fn modify_game_settings(&mut self, f: impl FnOnce(&mut horizon_client::settings::Settings)) {
        let game_home = self.catalog.config.game_home(&self.home);
        let _ = std::fs::create_dir_all(&game_home);
        let path = game_home.join("options.txt");
        let mut s = horizon_client::settings::Settings::load(&path);
        f(&mut s);
        s.save(&path);
    }

    fn modify_touch_settings(&mut self, f: impl FnOnce(&mut horizon_client::touch::Settings)) {
        let game_home = self.catalog.config.game_home(&self.home);
        let _ = std::fs::create_dir_all(&game_home);
        let mut t =
            horizon_client::touch::Settings::load_from(&game_home.join("horizon-touch.json"));
        f(&mut t);
        if !t.is_default_preset() {
            let _ = t.save_replace_preset(&game_home);
        } else {
            let _ = t.save_to(&game_home.join("horizon-touch.json"));
        }
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

        // Route all touches in the MCPE + GoyControl Editor through touch_hud_editor
        // and persist changes directly to the active profile's game_home.
        if self.control_editor.is_some() {
            let (rw, rh) = self.renderer.as_ref().map_or((1920, 1080), |r| {
                (r.config.width.max(1), r.config.height.max(1))
            });
            let game_home = self.catalog.config.game_home(&self.home);
            let prev_cwd = std::env::current_dir().ok();
            let _ = std::env::set_current_dir(&game_home);
            if let (Some(tc), Some(gui)) = (&mut self.control_editor, &self.gui) {
                tc.viewport(rw, rh);
                let mut b = HudBatch::new([rw, rh]);
                let norm = [
                    (t.location.x as f32 / rw as f32).clamp(0.0, 1.0),
                    (t.location.y as f32 / rh as f32).clamp(0.0, 1.0),
                ];
                let ended = matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
                let closed = horizon_client::touch_ui::touch_hud_editor(
                    tc,
                    &mut b,
                    &gui.atlas,
                    t.id,
                    norm,
                    t.phase == TouchPhase::Started,
                    ended,
                );
                if ended || closed {
                    let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
                }
                if closed || tc.editor.is_none() {
                    self.catalog.status = format!(
                        "Saved {} preset '{}' (controls/{}/{}.json).",
                        tc.settings.control_style.label(),
                        tc.settings.preset_name,
                        tc.settings.control_style.storage_subdir(),
                        tc.settings.preset_id
                    );
                    self.control_editor = None;
                }
            }
            if let Some(prev) = prev_cwd {
                let _ = std::env::set_current_dir(prev);
            }
            return Ok(());
        }

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
                        } else if let Some(modal) = &mut self.catalog.project_modal {
                            modal.scroll = (modal.scroll + delta).max(0.0);
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

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn wrap_text_lines(ui: &Ui, face: Face, size: f32, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if lines.last().is_some_and(|l: &String| !l.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }
        let mut current = String::new();
        for word in trimmed.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if ui.measure(face, size, &candidate) <= max_w || current.is_empty() {
                current = candidate;
            } else {
                lines.push(current);
                current = word.to_owned();
            }
            if lines.len() >= 16 {
                break;
            }
        }
        if !current.is_empty() && lines.len() < 16 {
            lines.push(current);
        }
        if lines.len() >= 16 {
            break;
        }
    }
    if lines.is_empty() {
        lines.push("No additional description available.".into());
    }
    lines
}
