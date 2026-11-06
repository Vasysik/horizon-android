//! Explicit, scrollable selections. Values never cycle on repeated button taps.
use super::{Launcher, pages::Action};
use crate::catalog::RendererMode;
use crate::launcher_glass::{Art, Kind as Look, Plate, clip, fitted, paragraph, white};
use crate::launcher_tokens::{fill, ink, radius};
use crate::launcher_view::{head, scrim};
use horizon_client::ui_theme::{Face, Ui, color::hex};

/// A choice's height.
const ROW: f32 = 58.0;

pub(super) enum Kind {
    Aspect,
    DisplayHz,
    CacheBudget,
    WorkerLimit,
    Renderer,
    RenderScale,
    Orientation,
    InputHost,
}
pub(super) struct Chooser {
    title: &'static str,
    help: &'static str,
    device: Option<String>,
    choices: Vec<(String, String, Action, bool)>,
    pub scroll: f32,
    /// Where the list was drawn: its top and bottom, and its left and width.
    top: f32,
    bottom: f32,
    frame: [f32; 2],
}
impl Launcher {
    pub(super) fn open_chooser(&mut self, kind: Kind) {
        crate::platform::hide();
        self.catalog.editing = None;
        let device = if matches!(kind, Kind::Renderer) {
            self.renderer.as_ref().map(|r| {
                format!(
                    "GPU: {} / {} {}",
                    r.info.name, r.info.driver, r.info.driver_info
                )
            })
        } else {
            None
        };
        let c = &self.catalog.config;
        let (title, help, choices) = match kind {
            Kind::Aspect => (
                "Aspect ratio",
                "Fits the entire picture without stretching. Touch coordinates follow the picture. Portrait swaps width and height.",
                [
                    crate::display::Aspect::Native,
                    crate::display::Aspect::Wide,
                    crate::display::Aspect::Classic,
                    crate::display::Aspect::Square,
                ]
                .into_iter()
                .map(|a| {
                    (
                        a.label().into(),
                        String::new(),
                        Action::ChooseAspect(a),
                        c.screen.aspect == a,
                    )
                })
                .collect(),
            ),
            Kind::CacheBudget => (
                "Reusable RAM cache",
                "Decoded sounds (80%) and empty mesh buffers (20%). Recreated at the same quality. Smaller budgets can increase CPU work. Not total RAM/VRAM. Applies on next Play.",
                [0, 16, 32, 64, 128, 256]
                    .into_iter()
                    .map(|mib| {
                        (
                            if mib == 0 {
                                "Engine default (80 MiB)".into()
                            } else {
                                format!("{mib} MiB")
                            },
                            String::new(),
                            Action::ChooseCacheBudget(mib),
                            c.cache_budget_mib == mib,
                        )
                    })
                    .collect(),
            ),
            Kind::WorkerLimit => (
                "Chunk worker limit",
                "Ceiling for generation and meshing workers, not all app threads. Lower values can reduce CPU contention but slow loading. Applies on next Play.",
                (0..=64u32)
                    .map(|limit| {
                        (
                            if limit == 0 {
                                "No additional limit".into()
                            } else {
                                format!("{limit} worker(s)")
                            },
                            String::new(),
                            Action::ChooseWorkerLimit(limit),
                            c.worker_limit == limit,
                        )
                    })
                    .collect(),
            ),
            Kind::DisplayHz => (
                "Display refresh rate",
                "An Android display preference, not a game FPS cap. Unsupported rates may be ignored.",
                crate::device_preferences::DISPLAY_RATES
                    .into_iter()
                    .map(|hz| {
                        (
                            if hz == 0 {
                                "System default".into()
                            } else {
                                format!("{hz} Hz")
                            },
                            String::new(),
                            Action::ChooseDisplayHz(hz),
                            c.device.display_hz == hz,
                        )
                    })
                    .collect(),
            ),
            Kind::RenderScale => {
                let mut choices: Vec<_> = crate::display::RenderScale::PRESETS
                    .into_iter()
                    .map(|percent| {
                        (
                            format!("{percent}%"),
                            if percent == 100 {
                                "Native resolution".into()
                            } else {
                                "Lower resolution for the whole picture".into()
                            },
                            Action::ChooseRenderScale(crate::display::RenderScale::new(percent)),
                            c.render_scale.percent() == percent,
                        )
                    })
                    .collect();
                choices.push((
                    "Custom percentage...".into(),
                    "Enter a value from 25 to 100".into(),
                    Action::Field("render_scale"),
                    false,
                ));
                (
                    "Client resolution",
                    "Small screens retain a usable menu size.",
                    choices,
                )
            }
            Kind::InputHost => (
                "Input mode",
                "Mouse mode uses a physical keyboard. Touch mode keeps the Android keyboard.",
                crate::catalog::InputHost::ALL
                    .into_iter()
                    .map(|mode| {
                        (
                            mode.label().into(),
                            String::new(),
                            Action::ChooseInputHost(mode),
                            c.input_host == mode,
                        )
                    })
                    .collect(),
            ),
            Kind::Orientation => (
                "Screen orientation",
                "Applies to the launcher and game. Does not change touch bindings.",
                crate::catalog::Orientation::ALL
                    .into_iter()
                    .map(|mode| {
                        (
                            mode.label().into(),
                            String::new(),
                            Action::ChooseOrientation(mode),
                            c.orientation == mode,
                        )
                    })
                    .collect(),
            ),
            Kind::Renderer => (
                "Rendering mode",
                "Uses the system GPU driver. Applies on the next game launch.",
                vec![
                    (
                        "OpenGL ES / experimental".into(),
                        "For older GPUs. Requires ES 3.0+ and sufficient driver limits.".into(),
                        Action::ChooseRenderer(RendererMode::OpenGl),
                        c.renderer == RendererMode::OpenGl,
                    ),
                    (
                        "Vulkan / GPU batching".into(),
                        "Groups terrain draws and enables supported GPU culling.".into(),
                        Action::ChooseRenderer(RendererMode::Vulkan),
                        c.renderer == RendererMode::Vulkan,
                    ),
                    (
                        "Vulkan / compatibility".into(),
                        "Direct terrain draws for drivers with missing or transparent blocks."
                            .into(),
                        Action::ChooseRenderer(RendererMode::VulkanCompatibility),
                        c.renderer == RendererMode::VulkanCompatibility,
                    ),
                ],
            ),
        };
        self.chooser = Some(Chooser {
            title,
            help,
            choices,
            device,
            scroll: 0.0,
            top: 0.0,
            bottom: 0.0,
            frame: [0.0, 0.0],
        });
    }
}
impl Chooser {
    /// A list made by the caller.
    pub(super) fn list(
        title: &'static str,
        help: &'static str,
        choices: Vec<(String, String, Action, bool)>,
    ) -> Self {
        Self {
            title,
            help,
            device: None,
            choices,
            scroll: 0.0,
            top: 0.0,
            bottom: 0.0,
            frame: [0.0, 0.0],
        }
    }
    pub(super) fn confirm_delete(name: String) -> Self {
        Self {
            title: "Remove add-on?",
            help: "Deletes only the selected file from this profile. This cannot be undone.",
            device: Some(name),
            choices: vec![
                (
                    "Remove".into(),
                    String::new(),
                    Action::ConfirmDeleteLocal,
                    false,
                ),
                ("Cancel".into(), String::new(), Action::Dismiss, true),
            ],
            scroll: 0.0,
            top: 0.0,
            bottom: 0.0,
            frame: [0.0, 0.0],
        }
    }
    pub(super) fn navigation(&self, _width: f32) -> Vec<([f32; 4], Action)> {
        let [x, w] = self.frame;
        self.choices
            .iter()
            .enumerate()
            .map(|(i, (_, _, action, _))| {
                (
                    [x, self.top + i as f32 * ROW - self.scroll, w, ROW - 2.0],
                    *action,
                )
            })
            .collect()
    }
    pub(super) fn viewport(&self, _height: f32) -> [f32; 3] {
        [
            self.top,
            self.bottom,
            (self.choices.len() as f32 * ROW - (self.bottom - self.top)).max(0.0),
        ]
    }
    pub fn draw(&mut self, art: &Art, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>) {
        targets.clear();
        scrim(ui);
        let w = (ui.w - 24.0).min(520.0);
        let h = (ui.h - 24.0).min(680.0);
        let (x, y) = ((ui.w - w) / 2.0, (ui.h - h) / 2.0);
        art.plate(ui, [x, y, w, h], radius::MODAL, Plate::Float);
        let mut top = head(ui, x + 24.0, y + 20.0, w - 48.0, self.title, self.help);
        if let Some(device) = &self.device {
            top += paragraph(
                ui,
                Face::Regular,
                13.0,
                x + 24.0,
                top,
                w - 48.0,
                white(ink::QUIET),
                device,
            ) + 4.0;
        }
        top += 6.0;
        let bottom = y + h - 74.0;
        self.top = top;
        self.bottom = bottom;
        self.frame = [x + 12.0, w - 24.0];
        self.scroll = self.scroll.clamp(
            0.0,
            (self.choices.len() as f32 * ROW - (bottom - top)).max(0.0),
        );
        let start = ui.batch.vertices.len();
        for (i, (label, detail, action, selected)) in self.choices.iter().enumerate() {
            let row = top + i as f32 * ROW - self.scroll;
            if row + ROW < top || row > bottom {
                continue;
            }
            let rect = [x + 12.0, row, w - 24.0, ROW - 2.0];
            if *selected {
                ui.rounded(
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    radius::ROW,
                    white(fill::SELECTED),
                );
                art.icon(
                    ui,
                    "tick-02",
                    rect[0] + rect[2] - 34.0,
                    row + (ROW - 22.0) / 2.0,
                    20.0,
                    hex(art.look.accent_soft, 255),
                );
            }
            let room = rect[2] - 56.0;
            let text_y = if detail.is_empty() {
                row + 16.0
            } else {
                row + 7.0
            };
            fitted(
                ui,
                Face::Medium,
                15.0,
                x + 24.0,
                text_y,
                room,
                white(ink::FULL),
                label,
            );
            fitted(
                ui,
                Face::Regular,
                13.0,
                x + 24.0,
                row + 29.0,
                room,
                white(ink::QUIET),
                detail,
            );
            let (y0, y1) = (rect[1].max(top), (rect[1] + rect[3]).min(bottom));
            if y1 - y0 >= 8.0 {
                targets.push(([rect[0], y0, rect[2], y1 - y0], *action));
            }
        }
        clip(ui, start, [x, top, w, bottom - top]);
        art.button(
            ui,
            targets,
            [x + 24.0, y + h - 60.0, w - 48.0, 42.0],
            "Cancel",
            Action::Dismiss,
            Look::Secondary,
        );
    }
}
