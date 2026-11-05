//! Explicit, scrollable selections. Values never cycle on repeated button taps.
use super::{
    Launcher,
    pages::{Action, button},
};
use crate::catalog::RendererMode;
use horizon_client::ui_theme::{Face, Ui};

pub(super) enum Kind {
    Aspect,
    DisplayHz,
    CacheBudget,
    WorkerLimit,
    Renderer,
    RenderScale,
    Profile,
    Orientation,
    InputHost,
    UiScale,
    ControlProfile,
}
pub(super) struct Chooser {
    title: &'static str,
    help: &'static str,
    device: Option<String>,
    choices: Vec<(String, String, Action, bool)>,
    pub scroll: f32,
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
                        "Optimized Vulkan pipeline with per-page draw batching and high-throughput chunk streaming.".into(),
                        Action::ChooseRenderer(RendererMode::Vulkan),
                        c.renderer == RendererMode::Vulkan,
                    ),
                    (
                        "Vulkan / compatibility".into(),
                        "Standard Vulkan rendering path with direct per-section draws."
                            .into(),
                        Action::ChooseRenderer(RendererMode::VulkanCompatibility),
                        c.renderer == RendererMode::VulkanCompatibility,
                    ),
                ],
            ),
            Kind::Profile => {
                let mut ids = vec![0, c.profile];
                ids.extend(
                    std::fs::read_dir(self.home.join("profiles"))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.path().is_dir())
                        .filter_map(|e| {
                            e.file_name()
                                .to_str()?
                                .strip_prefix("profile-")?
                                .parse::<u32>()
                                .ok()
                        }),
                );
                ids.sort_unstable();
                ids.dedup();
                let next = ids.last().copied().unwrap_or(0).checked_add(1);
                let mut choices: Vec<_> = ids
                    .into_iter()
                    .map(|id| {
                        (
                            format!("Profile {id}"),
                            if id == 0 {
                                "Default profile".into()
                            } else {
                                "Separate saves, mods and packs".into()
                            },
                            Action::ChooseProfile(id),
                            id == c.profile,
                        )
                    })
                    .collect();
                if let Some(id) = next {
                    choices.push((
                        "Create profile".into(),
                        format!("Create Profile {id}"),
                        Action::ChooseProfile(id),
                        false,
                    ));
                }
                (
                    "Choose profile",
                    "Each profile keeps its own game settings and saves.",
                    choices,
                )
            }
            Kind::UiScale => (
                "Launcher UI scale",
                "Adapts the launcher to your phone's DPI. Increase or decrease if touch controls and text feel too small or too large.",
                [
                    (
                        0u32,
                        "Auto (Phone DPI)",
                        "Automatically matches your display's DPI density",
                    ),
                    (85, "85% (Compact)", "Fits more rows on screen"),
                    (100, "100% (Default DPI)", "Standard phone DPI scaling"),
                    (115, "115% (Large)", "Larger buttons, cards and text"),
                    (
                        130,
                        "130% (Extra Large)",
                        "Maximum touch comfort on high-DPI phones",
                    ),
                ]
                .into_iter()
                .map(|(pct, title, desc)| {
                    (
                        title.into(),
                        desc.into(),
                        Action::ChooseUiScale(pct),
                        c.ui_scale_percent == pct,
                    )
                })
                .collect(),
            ),
            Kind::ControlProfile => {
                let game_home = c.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let style = touch.control_style;
                let presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                let choices = presets
                    .into_iter()
                    .enumerate()
                    .map(|(idx, meta)| {
                        let active = touch.preset_id.eq_ignore_ascii_case(&meta.id);
                        let mut title = meta.name.clone();
                        if meta.is_default {
                            title.push_str("  [Protected Default]");
                        }
                        let sub = format!(
                            "{}  •  controls/{}/{}.json",
                            meta.description,
                            style.storage_subdir(),
                            meta.id
                        );
                        (title, sub, Action::ChooseControlProfile(idx as u32), active)
                    })
                    .collect();
                (
                    match style {
                        horizon_client::touch::ControlStyle::Mcpe => "MCPE Control Presets",
                        horizon_client::touch::ControlStyle::GoyControl => {
                            "GoyControl Control Presets"
                        }
                    },
                    match style {
                        horizon_client::touch::ControlStyle::Mcpe => {
                            "Independent MCPE presets stored in controls/mcpe/. Default is protected."
                        }
                        horizon_client::touch::ControlStyle::GoyControl => {
                            "Independent GoyControl presets stored in controls/goycontrol/. Default is protected."
                        }
                    },
                    choices,
                )
            }
        };
        self.chooser = Some(Chooser {
            title,
            help,
            choices,
            device,
            scroll: 0.0,
        });
    }
}
impl Chooser {
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
        }
    }
    pub(super) fn navigation(&self, width: f32) -> Vec<([f32; 4], Action)> {
        self.choices
            .iter()
            .enumerate()
            .map(|(i, (_, _, action, _))| {
                (
                    [
                        32.0,
                        137.0 + i as f32 * 84.0 - self.scroll,
                        width - 64.0,
                        76.0,
                    ],
                    *action,
                )
            })
            .collect()
    }
    pub(super) fn viewport(&self, height: f32) -> [f32; 3] {
        [
            137.0,
            height - 100.0,
            (self.choices.len() as f32 * 84.0 - (height - 237.0)).max(0.0),
        ]
    }
    pub fn draw(&mut self, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>, modal_anim: f32) {
        let (w, h) = (ui.w, ui.h);
        let theme = horizon_client::ui_theme::active_theme();
        let slide = (1.0 - modal_anim.clamp(0.0, 1.0)) * 16.0;
        targets.clear();
        ui.draw_theme_background(0.0, 0.0, w, h, &theme);
        ui.text(
            Face::SemiBold,
            24.0,
            32.0,
            32.0 + slide,
            theme.text_primary,
            self.title,
        );
        ui.text(
            Face::Regular,
            14.0,
            32.0,
            74.0 + slide,
            theme.text_muted,
            self.help,
        );
        if let Some(device) = &self.device {
            ui.hint_line(32.0, 100.0 + slide, w - 64.0, device, theme.accent);
        }
        let top = 132.0 + slide;
        let bottom = h - 94.0;
        self.scroll = self.scroll.clamp(
            0.0,
            (self.choices.len() as f32 * 84.0 - (bottom - top)).max(0.0),
        );
        ui.set_clip(Some([0.0, top, w, (bottom - top).max(1.0)]));
        for (i, (label, detail, action, selected)) in self.choices.iter().enumerate() {
            let y = top + i as f32 * 84.0 - self.scroll;
            if y + 76.0 < top || y > bottom {
                continue;
            }
            ui.rounded(
                32.0,
                y,
                w - 64.0,
                74.0,
                14.0,
                if *selected {
                    theme.surface_selected
                } else {
                    theme.surface
                },
            );
            ui.stroke_rounded(
                32.0,
                y,
                w - 64.0,
                74.0,
                14.0,
                if *selected { 1.6 } else { 1.0 },
                if *selected {
                    theme.accent
                } else {
                    theme.border
                },
            );
            ui.stroke_rounded(
                48.0,
                y + 26.0,
                22.0,
                22.0,
                11.0,
                1.8,
                if *selected {
                    theme.accent
                } else {
                    theme.text_muted
                },
            );
            if *selected {
                ui.rounded(53.0, y + 31.0, 12.0, 12.0, 6.0, theme.accent);
            }
            ui.text(
                Face::SemiBold,
                17.0,
                84.0,
                y + 11.0,
                theme.text_primary,
                label,
            );
            ui.text(
                Face::Regular,
                13.0,
                84.0,
                y + 40.0,
                theme.text_muted,
                detail,
            );
            if let Some(c) = ui.clip_rect([32.0, y, w - 64.0, 74.0]) {
                targets.push((c, *action));
            }
        }
        ui.set_clip(None);
        button(
            ui,
            targets,
            [32.0, h - 74.0, 160.0, 44.0],
            "Cancel",
            Action::Dismiss,
            false,
        );
    }
}
