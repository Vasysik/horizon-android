//! Explicit, scrollable selections. Values never cycle on repeated button taps.
use super::{
    Launcher,
    pages::{Action, button},
};
use crate::catalog::RendererMode;
use horizon_client::ui_theme::{Face, Ui, color};

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
                    (0u32, "Auto (Phone DPI)", "Automatically matches your display's DPI density"),
                    (85, "85% (Compact)", "Fits more rows on screen"),
                    (100, "100% (Default DPI)", "Standard phone DPI scaling"),
                    (115, "115% (Large)", "Larger buttons, cards and text"),
                    (130, "130% (Extra Large)", "Maximum touch comfort on high-DPI phones"),
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
                let touch = horizon_client::touch::Settings::load_from(&game_home.join("horizon-touch.json"));
                let mut profiles = Vec::new();
                for (id, _, _) in horizon_client::touch::Settings::PRESETS {
                    profiles.push((*id).to_owned());
                }
                for saved in horizon_client::touch::Settings::list_profiles(&game_home) {
                    if !profiles.iter().any(|p| p.eq_ignore_ascii_case(&saved)) {
                        profiles.push(saved);
                    }
                }
                let choices = profiles
                    .into_iter()
                    .enumerate()
                    .map(|(idx, name)| {
                        let active = touch.active_profile.eq_ignore_ascii_case(&name);
                        let preset_info = horizon_client::touch::Settings::PRESETS
                            .iter()
                            .find(|(id, _, _)| id.eq_ignore_ascii_case(&name));
                        let (title, sub) = if let Some((_, title, desc)) = preset_info {
                            ((*title).to_owned(), (*desc).to_owned())
                        } else {
                            (
                                format!("Custom Layout: {name}"),
                                if active {
                                    "Currently active custom HUD layout".into()
                                } else {
                                    "Saved custom layout in controlmap/".into()
                                },
                            )
                        };
                        (
                            title,
                            sub,
                            Action::ChooseControlProfile(idx as u32),
                            active,
                        )
                    })
                    .collect();
                (
                    "HUD Control Presets & Profiles",
                    "Select a built-in Pocket Edition / Pojav preset or a custom saved layout.",
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
                        28.0,
                        108.0 + i as f32 * 64.0 - self.scroll,
                        width - 56.0,
                        58.0,
                    ],
                    *action,
                )
            })
            .collect()
    }
    pub(super) fn viewport(&self, height: f32) -> [f32; 3] {
        let top = 108.0;
        let bottom = (height - 64.0).max(top + 60.0);
        [
            top,
            bottom,
            (self.choices.len() as f32 * 64.0 - (bottom - top)).max(0.0),
        ]
    }
    pub fn draw(&mut self, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>) {
        let (w, h) = (ui.w, ui.h);
        targets.clear();
        ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x0D0C14, 255));
        ui.text(Face::SemiBold, 22.0, 28.0, 20.0, color::TEXT, self.title);
        let fitted_help = ui.fit_text(Face::Regular, 13.5, self.help, w - 56.0);
        ui.text(Face::Regular, 13.5, 28.0, 54.0, color::MUTED, &fitted_help);
        if let Some(device) = &self.device {
            let fitted_dev = ui.fit_text(Face::Regular, 12.5, device, w - 56.0);
            ui.text(Face::Regular, 12.5, 28.0, 78.0, color::MUTED, &fitted_dev);
        }
        let top = 108.0;
        let bottom = (h - 64.0).max(top + 60.0);
        self.scroll = self.scroll.clamp(
            0.0,
            (self.choices.len() as f32 * 64.0 - (bottom - top)).max(0.0),
        );
        ui.set_clip(Some([0.0, top, w, bottom - top]));
        for (i, (label, detail, action, selected)) in self.choices.iter().enumerate() {
            let y = top + i as f32 * 64.0 - self.scroll;
            if y + 58.0 < top || y > bottom {
                continue;
            }
            let bg = if *selected {
                color::hex(0x282142, 255)
            } else {
                color::hex(0x171621, 255)
            };
            let border = if *selected {
                color::hex(0x9A7BFF, 255)
            } else {
                color::hex(0x262435, 255)
            };
            ui.rounded(28.0, y, w - 56.0, 56.0, 14.0, bg);
            ui.border(28.0, y, w - 56.0, 56.0, 14.0, border);
            ui.rounded(
                44.0,
                y + 18.0,
                20.0,
                20.0,
                10.0,
                if *selected {
                    color::hex(0x9A7BFF, 255)
                } else {
                    color::hex(0x353248, 255)
                },
            );
            ui.rounded(46.0, y + 20.0, 16.0, 16.0, 8.0, bg);
            if *selected {
                ui.rounded(50.0, y + 24.0, 8.0, 8.0, 4.0, color::hex(0x9A7BFF, 255));
            }
            let has_detail = !detail.trim().is_empty();
            let fitted_lbl = ui.fit_text(Face::SemiBold, 15.5, label, w - 120.0);
            ui.text(
                Face::SemiBold,
                15.5,
                76.0,
                y + if has_detail { 8.0 } else { 17.0 },
                color::TEXT,
                &fitted_lbl,
            );
            if has_detail {
                let fitted_det = ui.fit_text(Face::Regular, 12.5, detail, w - 120.0);
                ui.text(Face::Regular, 12.5, 76.0, y + 31.0, color::MUTED, &fitted_det);
            }
            let cy0 = y.max(top);
            let cy1 = (y + 56.0).min(bottom);
            if cy1 > cy0 + 8.0 {
                targets.push(([28.0, cy0, w - 56.0, cy1 - cy0], *action));
            }
        }
        ui.set_clip(None);
        button(
            ui,
            targets,
            [28.0, h - 50.0, 136.0, 38.0],
            "Cancel",
            Action::Dismiss,
            false,
        );
    }
}
