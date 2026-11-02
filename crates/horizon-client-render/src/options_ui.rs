//! The options screens: OptionsScreen, Video Settings, Music & Sounds,
//! Controls, Key Binds, Mouse Settings, Chat Settings, Accessibility,
//! Language and Resource Packs, laid out like vanilla's HeaderAndFooter /
//! OptionsList screens and drawn with the shared widgets.
use std::time::Instant;

pub use crate::settings::Opt;

use crate::{
    gamepad::{self, GlyphStyle},
    hud::{HudAtlas, HudBatch, rgba},
    keybinds::{self, CATEGORIES, KeyBinds, MAPPINGS},
    lang::{self, LanguageInfo},
    settings::{SOUND_SOURCES, Settings},
    text::{self, Style, Text},
    ui,
};

fn tr(key: &str) -> Text {
    Text::translate(key, &[])
}
/// `text` cut to `chars` characters, `...` marking the cut.
fn clipped(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_owned();
    }
    let cut: String = text.chars().take(chars - 3).collect();
    format!("{}...", cut.trim_end())
}
/// Where the Shader Packs page's folder is, as a player finds it: on a phone, from the storage's
/// root (`Starling/shaderpacks`), or in the app's own files, which Open Pack Folder sets up a
/// reachable folder for.
fn shader_folder_note() -> String {
    let dir = std::env::current_dir()
        .unwrap_or_default()
        .join("shaderpacks");
    if cfg!(target_os = "android") {
        if crate::logging::game_home() == crate::logging::private_home() {
            return "In the app's own files: Open Pack Folder to reach them".into();
        }
        if let Ok(shared) = dir.strip_prefix("/storage/emulated/0") {
            return format!("Folder: {}", shared.display());
        }
    }
    clipped(&format!("Folder: {}", dir.display()), 52)
}
fn generic(caption: &str, value: Text) -> Text {
    Text::translate("options.generic_value", &[tr(caption), value])
}
fn percent(caption: &str, value: f32) -> Text {
    Text::translate(
        "options.percent_value",
        &[
            tr(caption),
            Text::literal(((f64::from(value) * 100.0) as i32).to_string()),
        ],
    )
}
fn on_off(v: bool) -> Text {
    tr(if v { "options.on" } else { "options.off" })
}
fn chat_px(caption: &str, px: f32) -> Text {
    Text::translate(
        "options.pixel_value",
        &[tr(caption), Text::literal((px as i32).to_string())],
    )
}
pub(crate) enum Kind {
    /// Slider position 0..1.
    Slider(f32),
    Cycle,
}
/// `PlayerModelPart` keys, in vanilla's bit order (`modelPart_*` in options.txt).
const MODEL_PART_KEYS: [&str; 7] = [
    "modelPart_cape",
    "modelPart_jacket",
    "modelPart_left_sleeve",
    "modelPart_right_sleeve",
    "modelPart_left_pants_leg",
    "modelPart_right_pants_leg",
    "modelPart_hat",
];
/// The Accessibility Settings page:
/// `AccessibilityOptionsScreen.options`, with the Controls link inserted separately.
pub(crate) const ACCESSIBILITY_OPTIONS: &[Opt] = &[
    Opt::Narrator,
    Opt::ShowSubtitles,
    Opt::HighContrast,
    Opt::MenuBlur,
    Opt::TextBackgroundOpacity,
    Opt::TextBackground,
    Opt::ChatOpacity,
    Opt::ChatLineSpacing,
    Opt::ChatDelay,
    Opt::NotificationDisplayTime,
    Opt::ViewBobbing,
    Opt::DistortionEffects,
    Opt::FovEffects,
    Opt::DarknessPulsing,
    Opt::DamageTilt,
    Opt::GlintSpeed,
    Opt::GlintStrength,
    Opt::HideLightningFlashes,
    Opt::MonochromeLogo,
    Opt::PanoramaSpeed,
    Opt::HideSplashTexts,
    Opt::NarratorHotkey,
    Opt::RotateWithMinecarts,
    Opt::HighContrastBlockOutlines,
];

/// `MouseSettingsScreen.options` in Java Edition 26.3, in vanilla order.
pub(crate) const MOUSE_OPTIONS: &[Opt] = &[
    Opt::Sensitivity,
    Opt::WheelSensitivity,
    Opt::DiscreteScroll,
    Opt::InvertX,
    Opt::InvertY,
    Opt::AllowCursorChanges,
];

impl Opt {
    /// Keep controls visible while their underlying systems are still missing.
    pub(crate) fn available(self) -> bool {
        !matches!(
            self,
            Opt::Narrator
                | Opt::NarratorHotkey
                | Opt::ChatDelay
                // Stored, but there are no blocked players, signed chat or
                // chat drafts yet.
                | Opt::HideMatchedNames
                | Opt::OnlySecureChat
                | Opt::SaveChatDrafts
                | Opt::MonochromeLogo
                | Opt::RotateWithMinecarts
                // Saved, but no Horizon system reads them yet: no Realms, no
                // friends list, no directional audio, no now-playing toast.
                | Opt::RealmsNotifications
                | Opt::InGameNotification
                | Opt::SharePresence
                | Opt::DirectionalAudio
                | Opt::MusicToast
                // Saved, but nothing in rendering reads them yet.
                | Opt::ChunkFade
                | Opt::ImprovedTransparency
                | Opt::ChunkBuilder
        )
    }
    fn file_key(self) -> Option<&'static str> {
        Some(match self {
            Opt::Narrator => "narrator",
            Opt::TextBackground => "backgroundForChatOnly",
            Opt::ChatDelay => "chatDelay",
            Opt::HideMatchedNames => "hideMatchedNames",
            Opt::OnlySecureChat => "onlyShowSecureChat",
            Opt::SaveChatDrafts => "saveChatDrafts",
            Opt::DistortionEffects => "screenEffectScale",
            Opt::FovEffects => "fovEffectScale",
            Opt::DarknessPulsing => "darknessEffectScale",
            Opt::DamageTilt => "damageTiltStrength",
            Opt::GlintSpeed => "glintSpeed",
            Opt::GlintStrength => "glintStrength",
            Opt::HideLightningFlashes => "hideLightningFlashes",
            Opt::MonochromeLogo => "darkMojangStudiosBackground",
            Opt::PanoramaSpeed => "panoramaScrollSpeed",
            Opt::HideSplashTexts => "hideSplashTexts",
            Opt::NarratorHotkey => "narratorHotkey",
            Opt::RotateWithMinecarts => "rotateWithMinecart",
            Opt::HighContrastBlockOutlines => "highContrastBlockOutline",
            Opt::MusicFrequency => "musicFrequency",
            Opt::ModelPart(i) => MODEL_PART_KEYS[i],
            Opt::AllowServerListing => "allowServerListing",
            Opt::RealmsNotifications => "realmsNotifications",
            Opt::InGameNotification => "inGameNotification",
            Opt::SharePresence => "sharePresence",
            Opt::DirectionalAudio => "directionalAudio",
            Opt::MusicToast => "musicToast",
            _ => return None,
        })
    }

    pub(crate) fn caption(self) -> &'static str {
        match self {
            Opt::Fov => "options.fov",
            Opt::RenderDistance => "options.renderDistance",
            Opt::SimulationDistance => "options.simulationDistance",
            Opt::Sensitivity => "options.sensitivity",
            Opt::WheelSensitivity => "options.mouseWheelSensitivity",
            Opt::InvertX => "options.invertMouseX",
            Opt::InvertY => "options.invertMouseY",
            Opt::DiscreteScroll => "options.discrete_mouse_scroll",
            Opt::AllowCursorChanges => "options.allowCursorChanges",
            Opt::Volume(_) => "soundCategory.master",
            Opt::GuiScale => "options.guiScale",
            Opt::ViewBobbing => "options.viewBobbing",
            Opt::Gamma => "options.gamma",
            Opt::MaxFps => "options.framerateLimit",
            Opt::Vsync => "options.vsync",
            Opt::Fullscreen => "options.fullscreen",
            Opt::SmoothLighting => "options.ao",
            Opt::Clouds => "options.renderClouds",
            Opt::CloudRange => "options.renderCloudsDistance",
            Opt::Particles => "options.particles",
            Opt::EntityDistance => "options.entityDistanceScaling",
            Opt::EntityShadows => "options.entityShadows",
            Opt::BiomeBlend => "options.biomeBlendRadius",
            Opt::Preset => "options.graphics.preset",
            Opt::Mipmaps => "options.mipmapLevels",
            Opt::WeatherRadius => "options.weatherRadius",
            Opt::ChunkFade => "options.chunkFade",
            Opt::Vignette => "options.vignette",
            Opt::Autosave => "options.autosaveIndicator",
            Opt::CutoutLeaves => "options.cutoutLeaves",
            Opt::ImprovedTransparency => "options.improvedTransparency",
            Opt::AttackIndicator => "options.attackIndicator",
            Opt::InactivityFps => "options.inactivityFpsLimit",
            Opt::ChunkBuilder => "options.prioritizeChunkUpdates",
            Opt::MenuBlur => "options.accessibility.menu_background_blurriness",
            Opt::ToggleCrouch => "key.sneak",
            Opt::ToggleSprint => "key.sprint",
            Opt::AutoJump => "options.autoJump",
            Opt::ChatVisibility => "options.chat.visibility",
            Opt::ChatColors => "options.chat.color",
            Opt::ChatLinks => "options.chat.links",
            Opt::ChatLinksPrompt => "options.chat.links.prompt",
            Opt::ChatOpacity => "options.chat.opacity",
            Opt::TextBackgroundOpacity => "options.accessibility.text_background_opacity",
            Opt::ChatScale => "options.chat.scale",
            Opt::ChatWidth => "options.chat.width",
            Opt::ChatHeightFocused => "options.chat.height.focused",
            Opt::ChatHeightUnfocused => "options.chat.height.unfocused",
            Opt::ChatLineSpacing => "options.chat.line_spacing",
            Opt::AutoSuggestions => "options.autoSuggestCommands",
            Opt::HideMatchedNames => "options.hideMatchedNames",
            Opt::OnlySecureChat => "options.onlyShowSecureChat",
            Opt::SaveChatDrafts => "options.chat.drafts",
            Opt::ReducedDebugInfo => "options.reducedDebugInfo",
            Opt::HighContrast => "options.accessibility.high_contrast",
            Opt::ShowSubtitles => "options.showSubtitles",
            Opt::MusicFrequency => "options.music_frequency",
            Opt::NotificationDisplayTime => "options.notifications.display_time",
            Opt::Narrator => "options.narrator",
            Opt::TextBackground => "options.accessibility.text_background",
            Opt::ChatDelay => "Chat Delay",
            Opt::DistortionEffects => "options.screenEffectScale",
            Opt::FovEffects => "options.fovEffectScale",
            Opt::DarknessPulsing => "options.darknessEffectScale",
            Opt::DamageTilt => "options.damageTiltStrength",
            Opt::GlintSpeed => "options.glintSpeed",
            Opt::GlintStrength => "options.glintStrength",
            Opt::HideLightningFlashes => "options.hideLightningFlashes",
            Opt::MonochromeLogo => "options.darkMojangStudiosBackgroundColor",
            Opt::PanoramaSpeed => "options.accessibility.panorama_speed",
            Opt::HideSplashTexts => "options.hideSplashTexts",
            Opt::NarratorHotkey => "options.accessibility.narrator_hotkey",
            Opt::RotateWithMinecarts => "options.rotateWithMinecart",
            Opt::HighContrastBlockOutlines => "options.accessibility.high_contrast_block_outline",
            Opt::ForceUnicode => "options.forceUnicodeFont",
            Opt::JapaneseGlyphs => "options.japaneseGlyphVariants",
            // Horizon's own options: vanilla has no keys for them, so they read
            // the namespace the client bundles (assets/horizon/lang).
            Opt::PadEnabled => "horizon.options.controller",
            Opt::PadVibration => "horizon.options.vibration",
            Opt::PadAdaptiveTriggers => "horizon.options.adaptive_triggers",
            Opt::PadLightbar => "horizon.options.lightbar",
            Opt::PadLookSensitivity => "horizon.options.look_sensitivity",
            Opt::PadInvertX => "horizon.options.invert_x",
            Opt::PadInvertY => "horizon.options.invert_y",
            Opt::PadToggleSneak => "horizon.options.sneak_button",
            Opt::PadServerStick => "horizon.options.server_stick",
            Opt::PadStickPrompt => "horizon.options.stick_prompt",
            Opt::PadMoveDeadzone => "horizon.options.move_deadzone",
            Opt::PadLookDeadzone => "horizon.options.look_deadzone",
            Opt::PadGlyphs => "horizon.options.button_glyphs",
            Opt::PadMenuNav => "horizon.options.menu_navigation",
            Opt::PadKeyboard => "horizon.options.on_screen_keyboard",
            Opt::ModelPart(i) => [
                "options.modelPart.cape",
                "options.modelPart.jacket",
                "options.modelPart.left_sleeve",
                "options.modelPart.right_sleeve",
                "options.modelPart.left_pants_leg",
                "options.modelPart.right_pants_leg",
                "options.modelPart.hat",
            ][i],
            Opt::MainHand => "options.mainHand",
            Opt::AllowServerListing => "options.allowServerListing",
            Opt::RealmsNotifications => "options.realmsNotifications.button",
            Opt::InGameNotification => "options.inGameNotification",
            Opt::SharePresence => "options.sharePresence",
            Opt::DirectionalAudio => "options.directionalAudio",
            Opt::MusicToast => "options.musicToast",
        }
    }
    pub(crate) fn kind(self, s: &Settings) -> Kind {
        let unit = |v: f32, lo: f32, hi: f32| Kind::Slider(((v - lo) / (hi - lo)).clamp(0.0, 1.0));
        match self {
            Opt::FovEffects | Opt::DarknessPulsing => {
                Kind::Slider(s.file.double(self.file_key().unwrap()).sqrt() as f32)
            }
            Opt::DistortionEffects
            | Opt::DamageTilt
            | Opt::GlintSpeed
            | Opt::GlintStrength
            | Opt::PanoramaSpeed => Kind::Slider(s.file.double(self.file_key().unwrap()) as f32),
            Opt::Fov => unit(s.fov, 30.0, 110.0),
            Opt::RenderDistance => unit(s.render_distance as f32, 2.0, 64.0),
            Opt::SimulationDistance => unit(s.simulation_distance as f32, 5.0, 32.0),
            Opt::Sensitivity => Kind::Slider(s.sensitivity),
            Opt::WheelSensitivity => unit(s.mouse_wheel_sensitivity, 0.01, 10.0),
            Opt::Volume(i) => Kind::Slider(if i == 0 {
                s.master_volume
            } else {
                s.volumes[i]
            }),
            Opt::Gamma => Kind::Slider(s.gamma),
            Opt::MaxFps => unit(s.max_fps as f32, 10.0, 260.0),
            Opt::CloudRange => unit(s.cloud_range as f32, 2.0, 128.0),
            Opt::EntityDistance => unit(s.entity_distance, 0.5, 5.0),
            Opt::BiomeBlend => unit(s.biome_blend as f32, 0.0, 7.0),
            Opt::Preset => unit(preset_index(&s.graphics_preset) as f32, 0.0, 3.0),
            Opt::Mipmaps => unit(s.mipmap_levels as f32, 0.0, 4.0),
            Opt::WeatherRadius => unit(s.weather_radius as f32, 3.0, 10.0),
            Opt::ChunkFade => unit(s.chunk_fade, 0.0, 2.0),
            Opt::MenuBlur => unit(s.menu_blur as f32, 0.0, 10.0),
            Opt::NotificationDisplayTime => unit(s.notification_display_time, 0.5, 10.0),
            Opt::ChatOpacity => Kind::Slider(s.chat_opacity),
            Opt::TextBackgroundOpacity => Kind::Slider(s.text_background_opacity),
            Opt::ChatScale => Kind::Slider(s.chat_scale),
            Opt::ChatWidth => Kind::Slider(s.chat_width),
            Opt::ChatHeightFocused => Kind::Slider(s.chat_height_focused),
            Opt::ChatHeightUnfocused => Kind::Slider(s.chat_height_unfocused),
            Opt::ChatLineSpacing => Kind::Slider(s.chat_line_spacing),
            Opt::PadLookSensitivity => Kind::Slider(s.controller.look_sensitivity),
            Opt::PadMoveDeadzone => unit(s.controller.move_deadzone, 0.0, 0.5),
            Opt::PadLookDeadzone => unit(s.controller.look_deadzone, 0.0, 0.5),
            _ => Kind::Cycle,
        }
    }
    /// OptionInstance's static or value-dependent tooltip translation.
    fn tooltip_key(self, s: &Settings) -> String {
        let key = match self {
            Opt::InactivityFps => {
                if s.inactivity_fps_limit == "minimized" {
                    "options.inactivityFpsLimit.minimized"
                } else {
                    "options.inactivityFpsLimit.afk"
                }
            }
            Opt::ChunkBuilder => [
                "options.prioritizeChunkUpdates.none",
                "options.prioritizeChunkUpdates.byPlayer",
                "options.prioritizeChunkUpdates.nearby",
            ][usize::from(s.prioritize_chunk_updates.min(2))],
            _ => self.caption(),
        };
        format!("{key}.tooltip")
    }
    /// A slider's position (0..1), or `None` for a cycle button.
    pub fn slider_position(self, s: &Settings) -> Option<f32> {
        match self.kind(s) {
            Kind::Slider(v) => Some(v),
            Kind::Cycle => None,
        }
    }
    /// The widget message (OptionInstance.toString / CycleButton full name).
    pub fn label(self, s: &Settings) -> Text {
        let c = self.caption();
        match self {
            Opt::ChatDelay => tr("options.chat.delay_none"),
            Opt::FovEffects
            | Opt::DarknessPulsing
            | Opt::DistortionEffects
            | Opt::DamageTilt
            | Opt::GlintSpeed
            | Opt::GlintStrength => {
                let value = s.file.double(self.file_key().unwrap()) as f32;
                if value == 0.0 {
                    generic(c, tr("options.off"))
                } else {
                    percent(c, value)
                }
            }
            Opt::PanoramaSpeed => percent(c, s.file.double("panoramaScrollSpeed") as f32),
            Opt::Fov => match s.fov as i32 {
                70 => generic(c, tr("options.fov.min")),
                110 => generic(c, tr("options.fov.max")),
                v => generic(c, Text::literal(v.to_string())),
            },
            Opt::RenderDistance => generic(c, Text::tr("options.chunks", &[&s.render_distance])),
            Opt::SimulationDistance => {
                generic(c, Text::tr("options.chunks", &[&s.simulation_distance]))
            }
            Opt::Sensitivity => {
                if s.sensitivity == 0.0 {
                    generic(c, tr("options.sensitivity.min"))
                } else if s.sensitivity == 1.0 {
                    generic(c, tr("options.sensitivity.max"))
                } else {
                    percent(c, 2.0 * s.sensitivity)
                }
            }
            Opt::WheelSensitivity => generic(
                c,
                Text::literal(format!("{:.2}", s.mouse_wheel_sensitivity)),
            ),
            Opt::Volume(i) => {
                let v = if i == 0 {
                    s.master_volume
                } else {
                    s.volumes[i]
                };
                let caption = format!("soundCategory.{}", SOUND_SOURCES[i]);
                if v == 0.0 {
                    generic(&caption, tr("options.off"))
                } else {
                    percent(&caption, v)
                }
            }
            Opt::GuiScale => {
                let value = if s.gui_scale == 0 {
                    tr("options.guiScale.auto")
                } else {
                    Text::literal(s.gui_scale.to_string())
                };
                generic(c, value)
            }
            Opt::Gamma => match (f64::from(s.gamma) * 100.0) as i32 {
                0 => generic(c, tr("options.gamma.min")),
                50 => generic(c, tr("options.gamma.default")),
                100 => generic(c, tr("options.gamma.max")),
                v => generic(c, Text::literal(v.to_string())),
            },
            Opt::MaxFps => {
                if s.max_fps >= 260 {
                    generic(c, tr("options.framerateLimit.max"))
                } else {
                    generic(c, Text::tr("options.framerate", &[&s.max_fps]))
                }
            }
            Opt::CloudRange => generic(c, Text::tr("options.chunks", &[&s.cloud_range])),
            Opt::EntityDistance => percent(c, s.entity_distance),
            Opt::BiomeBlend => generic(
                c,
                tr(&format!(
                    "options.biomeBlendRadius.{}",
                    s.biome_blend * 2 + 1
                )),
            ),
            Opt::Preset => generic(c, tr(&format!("options.graphics.{}", s.graphics_preset))),
            Opt::Mipmaps => {
                if s.mipmap_levels == 0 {
                    generic(c, tr("options.off"))
                } else {
                    generic(c, Text::literal(s.mipmap_levels.to_string()))
                }
            }
            Opt::WeatherRadius => generic(c, Text::tr("options.blocks", &[&s.weather_radius])),
            Opt::ChunkFade => {
                if s.chunk_fade <= 0.0 {
                    tr("options.chunkFade.none")
                } else {
                    Text::tr(
                        "options.chunkFade.seconds",
                        &[&format!("{:.2}", s.chunk_fade)],
                    )
                }
            }
            Opt::MenuBlur => {
                if s.menu_blur == 0 {
                    generic(c, tr("options.off"))
                } else {
                    generic(c, Text::literal(s.menu_blur.to_string()))
                }
            }
            Opt::ChatOpacity => percent(c, s.chat_opacity * 0.9 + 0.1),
            Opt::TextBackgroundOpacity => percent(c, s.text_background_opacity),
            Opt::ChatScale => {
                if s.chat_scale == 0.0 {
                    generic(c, tr("options.off"))
                } else {
                    percent(c, s.chat_scale)
                }
            }
            Opt::ChatWidth => chat_px(c, (s.chat_width * 280.0 + 40.0).floor()),
            Opt::ChatHeightFocused => chat_px(c, (s.chat_height_focused * 160.0 + 20.0).floor()),
            Opt::ChatHeightUnfocused => {
                chat_px(c, (s.chat_height_unfocused * 160.0 + 20.0).floor())
            }
            Opt::ChatLineSpacing => percent(c, s.chat_line_spacing),
            Opt::NotificationDisplayTime => generic(
                c,
                Text::tr(
                    "options.multiplier",
                    &[&format!("{:.1}", s.notification_display_time)],
                ),
            ),
            Opt::PadLookSensitivity => percent(c, 2.0 * s.controller.look_sensitivity),
            Opt::PadMoveDeadzone | Opt::PadLookDeadzone => {
                let v = if self == Opt::PadMoveDeadzone {
                    s.controller.move_deadzone
                } else {
                    s.controller.look_deadzone
                };
                Text::translate(
                    "options.percent_value",
                    &[
                        tr(c),
                        Text::literal(((v * 100.0).round() as i32).to_string()),
                    ],
                )
            }
            _ => generic(c, self.value_text(s)),
        }
    }
    fn value_text(self, s: &Settings) -> Text {
        match self {
            Opt::TextBackground => tr(if s.file.bool("backgroundForChatOnly") {
                "options.accessibility.text_background.chat"
            } else {
                "options.accessibility.text_background.everywhere"
            }),
            Opt::Narrator => tr("options.narrator.off"),
            Opt::ChatDelay => tr("options.chat.delay_none"),
            Opt::HideLightningFlashes => on_off(s.file.bool("hideLightningFlashes")),
            Opt::MonochromeLogo => on_off(s.file.bool("darkMojangStudiosBackground")),
            Opt::HideSplashTexts => on_off(s.file.bool("hideSplashTexts")),
            Opt::HideMatchedNames
            | Opt::OnlySecureChat
            | Opt::SaveChatDrafts
            | Opt::ModelPart(_)
            | Opt::AllowServerListing
            | Opt::RealmsNotifications
            | Opt::InGameNotification
            | Opt::DirectionalAudio => on_off(s.file.bool(self.file_key().unwrap())),
            Opt::MainHand => tr(if s.main_hand_left {
                "options.mainHand.left"
            } else {
                "options.mainHand.right"
            }),
            Opt::SharePresence => tr([
                "options.sharePresence.none",
                "options.sharePresence.limited",
                "options.sharePresence.all",
            ][s.file.int("sharePresence").clamp(0, 2) as usize]),
            Opt::MusicToast => tr([
                "options.musicToast.never",
                "options.musicToast.pauseMenu",
                "options.musicToast.pauseMenuAndToast",
            ][s.file.int("musicToast").clamp(0, 2) as usize]),
            Opt::NarratorHotkey => on_off(s.file.bool("narratorHotkey")),
            Opt::RotateWithMinecarts => on_off(s.file.bool("rotateWithMinecart")),
            Opt::HighContrastBlockOutlines => on_off(s.file.bool("highContrastBlockOutline")),
            Opt::InvertX => on_off(s.invert_x),
            Opt::InvertY => on_off(s.invert_y),
            Opt::DiscreteScroll => on_off(s.discrete_scroll),
            Opt::AllowCursorChanges => on_off(s.allow_cursor_changes),
            Opt::ViewBobbing => on_off(s.view_bobbing),
            Opt::Vsync => on_off(s.vsync),
            Opt::Fullscreen => on_off(s.fullscreen),
            Opt::SmoothLighting => on_off(s.smooth_lighting),
            Opt::Clouds => tr(match s.clouds.as_str() {
                "false" => "options.off",
                "fast" => "options.clouds.fast",
                _ => "options.clouds.fancy",
            }),
            Opt::Particles => tr([
                "options.particles.all",
                "options.particles.decreased",
                "options.particles.minimal",
            ][usize::from(s.particles.min(2))]),
            Opt::EntityShadows => on_off(s.entity_shadows),
            Opt::Vignette => on_off(s.vignette),
            Opt::Autosave => on_off(s.autosave_indicator),
            Opt::CutoutLeaves => on_off(s.cutout_leaves),
            Opt::ImprovedTransparency => on_off(s.improved_transparency),
            Opt::AttackIndicator => tr([
                "options.off",
                "options.attack.crosshair",
                "options.attack.hotbar",
            ][usize::from(s.attack_indicator.min(2))]),
            Opt::InactivityFps => tr(if s.inactivity_fps_limit == "minimized" {
                "options.inactivityFpsLimit.minimized"
            } else {
                "options.inactivityFpsLimit.afk"
            }),
            Opt::ChunkBuilder => tr([
                "options.prioritizeChunkUpdates.none",
                "options.prioritizeChunkUpdates.byPlayer",
                "options.prioritizeChunkUpdates.nearby",
            ][usize::from(s.prioritize_chunk_updates.min(2))]),
            Opt::ToggleCrouch => tr(if s.toggle_crouch {
                "options.key.toggle"
            } else {
                "options.key.hold"
            }),
            Opt::ToggleSprint => tr(if s.toggle_sprint {
                "options.key.toggle"
            } else {
                "options.key.hold"
            }),
            Opt::AutoJump => on_off(s.auto_jump),
            Opt::ChatVisibility => tr([
                "options.chat.visibility.full",
                "options.chat.visibility.system",
                "options.chat.visibility.hidden",
            ][usize::from(s.chat_visibility.min(2))]),
            Opt::ChatColors => on_off(s.chat_colors),
            Opt::ChatLinks => on_off(s.chat_links),
            Opt::ChatLinksPrompt => on_off(s.chat_links_prompt),
            Opt::AutoSuggestions => on_off(s.auto_suggestions),
            Opt::ReducedDebugInfo => on_off(s.reduced_debug_info),
            Opt::HighContrast => on_off(s.high_contrast),
            Opt::ShowSubtitles => on_off(s.show_subtitles),
            Opt::MusicFrequency => tr([
                "options.music_frequency.default",
                "options.music_frequency.frequent",
                "options.music_frequency.constant",
            ][s.file.int("musicFrequency").clamp(0, 2) as usize]),
            Opt::ForceUnicode => on_off(s.force_unicode),
            Opt::JapaneseGlyphs => on_off(s.japanese_glyphs),
            Opt::PadEnabled => on_off(s.controller.enabled),
            Opt::PadVibration => on_off(s.controller.vibration),
            Opt::PadAdaptiveTriggers => on_off(s.controller.adaptive_triggers),
            Opt::PadInvertX => on_off(s.controller.invert_x),
            Opt::PadInvertY => on_off(s.controller.invert_y),
            Opt::PadServerStick => tr(&s.controller.server_stick.lang_key()),
            Opt::PadStickPrompt => on_off(s.controller.stick_prompt),
            Opt::PadToggleSneak => tr(if s.controller.toggle_sneak {
                "options.key.toggle"
            } else {
                "options.key.hold"
            }),
            Opt::PadLightbar => Text::literal(match s.controller.lightbar {
                gamepad::Lightbar::Health => "Health",
                gamepad::Lightbar::Dimension => "Dimension",
                gamepad::Lightbar::Off => "Off",
            }),
            Opt::PadGlyphs => Text::literal(match s.controller.glyphs {
                GlyphStyle::Auto => "Auto",
                GlyphStyle::Xbox => "Xbox",
                GlyphStyle::PlayStation => "PlayStation",
                GlyphStyle::Nintendo => "Nintendo",
            }),
            Opt::PadMenuNav => Text::literal(match s.controller.menu_nav {
                gamepad::MenuNav::Focus => "Focus",
                gamepad::MenuNav::Pointer => "Pointer",
            }),
            Opt::PadKeyboard => Text::literal(match s.controller.keyboard {
                gamepad::TextKeyboard::Auto => "Auto",
                gamepad::TextKeyboard::Horizon => "Horizon",
                gamepad::TextKeyboard::Steam => "Steam",
            }),
            _ => Text::empty(),
        }
    }
    /// Set a slider from its 0..1 position (OptionInstance value sets).
    pub fn set_slider(self, s: &mut Settings, v: f32) {
        if !self.available() {
            return;
        }
        let v = v.clamp(0.0, 1.0);
        let step = |lo: f32, hi: f32| (lo + v * (hi - lo)).round();
        match self {
            Opt::FovEffects | Opt::DarknessPulsing => s.file.set(
                self.file_key().unwrap(),
                crate::options_file::V::Double(f64::from(v).powi(2)),
            ),
            Opt::DistortionEffects
            | Opt::DamageTilt
            | Opt::PanoramaSpeed
            | Opt::GlintSpeed
            | Opt::GlintStrength => s.file.set(
                self.file_key().unwrap(),
                crate::options_file::V::Double(f64::from(v)),
            ),
            Opt::Fov => s.fov = step(30.0, 110.0),
            Opt::RenderDistance => s.render_distance = step(2.0, 64.0) as i32,
            Opt::SimulationDistance => s.simulation_distance = step(5.0, 32.0) as i32,
            Opt::Sensitivity => s.sensitivity = v,
            Opt::WheelSensitivity => s.mouse_wheel_sensitivity = 0.01 + v * 9.99,
            Opt::Volume(0) => s.master_volume = v,
            Opt::Volume(i) => s.volumes[i] = v,
            Opt::Gamma => s.gamma = v,
            Opt::MaxFps => s.max_fps = (step(1.0, 26.0) as u32) * 10,
            Opt::CloudRange => s.cloud_range = step(2.0, 128.0) as i32,
            Opt::EntityDistance => s.entity_distance = step(2.0, 20.0) / 4.0,
            Opt::BiomeBlend => s.biome_blend = step(0.0, 7.0) as i32,
            Opt::Preset => s.graphics_preset = PRESETS[step(0.0, 3.0) as usize].into(),
            Opt::Mipmaps => s.mipmap_levels = step(0.0, 4.0) as i32,
            Opt::WeatherRadius => s.weather_radius = step(3.0, 10.0) as i32,
            Opt::ChunkFade => s.chunk_fade = step(0.0, 40.0) / 20.0,
            Opt::MenuBlur => s.menu_blur = step(0.0, 10.0) as i32,
            Opt::NotificationDisplayTime => s.notification_display_time = step(5.0, 100.0) / 10.0,
            Opt::ChatOpacity => s.chat_opacity = v,
            Opt::TextBackgroundOpacity => s.text_background_opacity = v,
            Opt::ChatScale => s.chat_scale = v,
            Opt::ChatWidth => s.chat_width = v,
            Opt::ChatHeightFocused => s.chat_height_focused = v,
            Opt::ChatHeightUnfocused => s.chat_height_unfocused = v,
            Opt::ChatLineSpacing => s.chat_line_spacing = v,
            Opt::PadLookSensitivity => s.controller.look_sensitivity = v,
            Opt::PadMoveDeadzone => s.controller.move_deadzone = step(0.0, 50.0) / 100.0,
            Opt::PadLookDeadzone => s.controller.look_deadzone = step(0.0, 50.0) / 100.0,
            _ => {}
        }
        if self.affects_preset() {
            s.graphics_preset = "custom".into();
        }
    }
    fn affects_preset(self) -> bool {
        matches!(
            self,
            Opt::RenderDistance
                | Opt::SimulationDistance
                | Opt::SmoothLighting
                | Opt::Clouds
                | Opt::CloudRange
                | Opt::Particles
                | Opt::EntityDistance
                | Opt::EntityShadows
                | Opt::BiomeBlend
                | Opt::Mipmaps
                | Opt::WeatherRadius
                | Opt::CutoutLeaves
                | Opt::ImprovedTransparency
                | Opt::MenuBlur
                | Opt::ChunkBuilder
        )
    }
    /// Advance a cycle button (CycleButton.cycleValue).
    pub fn cycle(self, s: &mut Settings, back: bool, framebuffer: [i32; 2]) {
        if !self.available() {
            return;
        }
        let next = |n: u8, count: u8| {
            if back {
                (n + count - 1) % count
            } else {
                (n + 1) % count
            }
        };
        match self {
            Opt::GuiScale => {
                let max = crate::window::calculate_scale(framebuffer, 0, s.force_unicode) as u32;
                let value = s.gui_scale.min(max);
                s.gui_scale = if back {
                    if value == 0 { max } else { value - 1 }
                } else if value == max {
                    0
                } else {
                    value + 1
                };
            }
            Opt::TextBackground
            | Opt::HideLightningFlashes
            | Opt::HideSplashTexts
            | Opt::HighContrastBlockOutlines
            | Opt::HideMatchedNames
            | Opt::OnlySecureChat
            | Opt::SaveChatDrafts
            | Opt::ModelPart(_)
            | Opt::AllowServerListing
            | Opt::RealmsNotifications
            | Opt::InGameNotification
            | Opt::DirectionalAudio => {
                let key = self.file_key().unwrap();
                s.file
                    .set(key, crate::options_file::V::Bool(!s.file.bool(key)));
            }
            Opt::InvertX => s.invert_x = !s.invert_x,
            Opt::InvertY => s.invert_y = !s.invert_y,
            Opt::DiscreteScroll => s.discrete_scroll = !s.discrete_scroll,
            Opt::AllowCursorChanges => s.allow_cursor_changes = !s.allow_cursor_changes,
            Opt::ViewBobbing => s.view_bobbing = !s.view_bobbing,
            Opt::Vsync => s.vsync = !s.vsync,
            Opt::Fullscreen => s.fullscreen = !s.fullscreen,
            Opt::SmoothLighting => s.smooth_lighting = !s.smooth_lighting,
            Opt::Clouds => {
                let order = ["false", "fast", "true"];
                let i = order.iter().position(|c| *c == s.clouds).unwrap_or(2) as u8;
                s.clouds = order[usize::from(next(i, 3))].into();
            }
            Opt::Particles => s.particles = next(s.particles, 3),
            Opt::EntityShadows => s.entity_shadows = !s.entity_shadows,
            Opt::Vignette => s.vignette = !s.vignette,
            Opt::Autosave => s.autosave_indicator = !s.autosave_indicator,
            Opt::CutoutLeaves => s.cutout_leaves = !s.cutout_leaves,
            Opt::ImprovedTransparency => s.improved_transparency = !s.improved_transparency,
            Opt::AttackIndicator => s.attack_indicator = next(s.attack_indicator, 3),
            Opt::InactivityFps => {
                s.inactivity_fps_limit = if s.inactivity_fps_limit == "afk" {
                    "minimized".into()
                } else {
                    "afk".into()
                }
            }
            Opt::ChunkBuilder => s.prioritize_chunk_updates = next(s.prioritize_chunk_updates, 3),
            Opt::ToggleCrouch => s.toggle_crouch = !s.toggle_crouch,
            Opt::ToggleSprint => s.toggle_sprint = !s.toggle_sprint,
            Opt::AutoJump => s.auto_jump = !s.auto_jump,
            Opt::ChatVisibility => s.chat_visibility = next(s.chat_visibility, 3),
            Opt::ChatColors => s.chat_colors = !s.chat_colors,
            Opt::ChatLinks => s.chat_links = !s.chat_links,
            Opt::ChatLinksPrompt => s.chat_links_prompt = !s.chat_links_prompt,
            Opt::AutoSuggestions => s.auto_suggestions = !s.auto_suggestions,
            Opt::ReducedDebugInfo => s.reduced_debug_info = !s.reduced_debug_info,
            Opt::HighContrast => s.high_contrast = !s.high_contrast,
            Opt::ShowSubtitles => s.show_subtitles = !s.show_subtitles,
            Opt::MusicFrequency => {
                let n = next(s.file.int("musicFrequency").clamp(0, 2) as u8, 3);
                s.file
                    .set("musicFrequency", crate::options_file::V::Int(i32::from(n)));
            }
            Opt::MainHand => s.main_hand_left = !s.main_hand_left,
            Opt::SharePresence => {
                let n = next(s.file.int("sharePresence").clamp(0, 2) as u8, 3);
                s.file
                    .set("sharePresence", crate::options_file::V::Int(i32::from(n)));
            }
            Opt::MusicToast => {
                let n = next(s.file.int("musicToast").clamp(0, 2) as u8, 3);
                s.file
                    .set("musicToast", crate::options_file::V::Int(i32::from(n)));
            }
            Opt::ForceUnicode => s.force_unicode = !s.force_unicode,
            Opt::JapaneseGlyphs => s.japanese_glyphs = !s.japanese_glyphs,
            Opt::PadEnabled => s.controller.enabled = !s.controller.enabled,
            Opt::PadVibration => s.controller.vibration = !s.controller.vibration,
            Opt::PadAdaptiveTriggers => {
                s.controller.adaptive_triggers = !s.controller.adaptive_triggers
            }
            Opt::PadInvertX => s.controller.invert_x = !s.controller.invert_x,
            Opt::PadInvertY => s.controller.invert_y = !s.controller.invert_y,
            Opt::PadToggleSneak => s.controller.toggle_sneak = !s.controller.toggle_sneak,
            Opt::PadServerStick => s.controller.server_stick = s.controller.server_stick.next(),
            Opt::PadStickPrompt => s.controller.stick_prompt = !s.controller.stick_prompt,
            Opt::PadLightbar => {
                let all = gamepad::Lightbar::ALL;
                let i = all
                    .iter()
                    .position(|l| *l == s.controller.lightbar)
                    .unwrap_or(0) as u8;
                s.controller.lightbar = all[usize::from(next(i, 3))];
            }
            Opt::PadGlyphs => {
                let i = GlyphStyle::ALL
                    .iter()
                    .position(|g| *g == s.controller.glyphs)
                    .unwrap_or(0) as u8;
                s.controller.glyphs = GlyphStyle::ALL[usize::from(next(i, 4))];
            }
            Opt::PadMenuNav => {
                s.controller.menu_nav = match s.controller.menu_nav {
                    gamepad::MenuNav::Focus => gamepad::MenuNav::Pointer,
                    gamepad::MenuNav::Pointer => gamepad::MenuNav::Focus,
                }
            }
            Opt::PadKeyboard => {
                let all = gamepad::TextKeyboard::ALL;
                let i = all
                    .iter()
                    .position(|k| *k == s.controller.keyboard)
                    .unwrap_or(0) as u8;
                s.controller.keyboard = all[usize::from(next(i, 3))];
            }
            _ => {}
        }
        if self.affects_preset() {
            s.graphics_preset = "custom".into();
        }
    }
}
const PRESETS: [&str; 4] = ["fast", "fancy", "fabulous", "custom"];
fn preset_index(p: &str) -> usize {
    PRESETS.iter().position(|x| *x == p).unwrap_or(1)
}
/// A slider moved to `v` (OptionInstance.set): the graphics preset applies
/// its options when its value changes (`Options.applyGraphicsPreset`).
pub fn set_slider(opt: Opt, s: &mut Settings, v: f32) {
    let preset = preset_index(&s.graphics_preset);
    opt.set_slider(s, v);
    if opt == Opt::Preset && preset_index(&s.graphics_preset) != preset {
        apply_preset(s);
    }
}
/// GraphicsPreset.apply: the options Fast, Fancy and Fabulous set (texture
/// filtering and anisotropy have no option here yet).
pub fn apply_preset(s: &mut Settings) {
    let fabulous = s.graphics_preset == "fabulous";
    match s.graphics_preset.as_str() {
        "fast" => {
            s.biome_blend = 1;
            s.render_distance = 8;
            s.prioritize_chunk_updates = 0;
            s.simulation_distance = 6;
            s.smooth_lighting = false;
            s.clouds = "fast".into();
            s.particles = 1;
            s.mipmap_levels = 2;
            s.entity_shadows = false;
            s.entity_distance = 0.75;
            s.menu_blur = 2;
            s.cloud_range = 32;
            s.cutout_leaves = false;
            s.improved_transparency = false;
            s.weather_radius = 5;
        }
        "fancy" | "fabulous" => {
            s.biome_blend = 2;
            s.render_distance = if fabulous { 32 } else { 16 };
            s.prioritize_chunk_updates = 1;
            s.simulation_distance = 12;
            s.smooth_lighting = true;
            s.clouds = "true".into();
            s.particles = 0;
            s.mipmap_levels = 4;
            s.entity_shadows = true;
            s.entity_distance = if fabulous { 1.25 } else { 1.0 };
            s.menu_blur = 5;
            s.cloud_range = if fabulous { 128 } else { 64 };
            s.cutout_leaves = true;
            s.improved_transparency = fabulous;
            s.weather_radius = 10;
        }
        _ => {}
    }
}

/// `selected_key` while a native mod's key waits (`selected_native_key`).
const NATIVE_KEY: &str = "horizon.nativeModKey";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Main,
    Video,
    Sound,
    Controls,
    KeyBinds,
    Mouse,
    Chat,
    Accessibility,
    Language,
    Font,
    Packs,
    Mods,
    /// A native mod's settings page: its index in `mods.installed`.
    NativeMod(usize),
    /// Horizon's shader packs, either kind (`shader_pack_list`): one on or none, and a
    /// `shaders/` pack's profile.
    ShaderPacks,
    /// The picked `shaders/` pack's options (`shader_options`), one of its screens at a time.
    ShaderOptions,
    /// The player's own touch buttons (`Settings::custom_buttons`), and what one of them does.
    CustomButtons,
    CustomButton(u32),
    /// Horizon's graphics settings (`graphics_ui`).
    Graphics,
    GraphicsAdvanced,
    /// Horizon's controller settings and button binds.
    Controller,
    PadBinds,
    Extras,
    /// CreditsAndAttributionScreen.
    Credits,
    /// Horizon Settings: every page Horizon adds, as a mod's settings
    /// screen hangs off vanilla's Options.
    Horizon,
    /// AI Agent: agent control on or off, whether an agent is connected, the connection details.
    Agent,
    /// SkinCustomizationScreen: the skin layers and Main Hand, and in the game's own options
    /// (`skin_page`) the player's name and own skin (`own_skin`) before them.
    Skin,
    /// OnlineOptionsScreen: server listing, Realms and friends notifications.
    Online,
}
#[derive(Clone, Debug, PartialEq)]
enum Act {
    None,
    Push(Page),
    Done,
    Cycle(Opt),
    Extra(crate::extras::Item),
    Key(&'static str),
    ResetKey(&'static str),
    ResetKeys,
    /// Controller binds: wait for a button, reset one, reset all.
    PadKey(&'static str),
    ResetPadKey(&'static str),
    ResetPadKeys,
    ToggleMod(usize),
    /// Pick `s.shader_packs[i]`, or turn it off when it is the pick (None: Off).
    PickShaderPack(Option<usize>),
    /// The next profile of the chosen pack.
    ShaderProfile,
    OpenShaderPackFolder,
    /// The next of the resolutions the Shader Packs page offers.
    ShaderResolution,
    /// The next of the frame limits the Shader Packs page offers.
    ShaderFpsLimit,
    /// The Custom Buttons page, a new button there, one to change or take away, and a key for it.
    CustomButtons,
    CustomAdd,
    CustomEdit(u32),
    CustomDelete(u32),
    CustomKey(u32, &'static str),
    /// Open the Shader Options page.
    ShaderOptions,
    /// A pack's option, its next value.
    ShaderOption(String),
    /// One of the pack's screens.
    ShaderScreen(String),
    /// The next of the profiles on the pack's own screen.
    ShaderOptionsProfile,
    /// All the pack's options back at what the profile says.
    ShaderReset,
    /// The mod's settings screen (`screens.set-config`, Mod Menu's config button).
    ModConfig(usize),
    /// The installed mod's own settings page (capability `ui`), shown as a
    /// screen: `Event::ModSettings` has the game open it.
    ModSettings(usize),
    /// A native mod's key (mod, key) waits for a key; a button of its page.
    NativeKey(usize, usize),
    NativeButton(usize, usize),
    OpenModFolder,
    /// The touch layout editor (phones).
    TouchLayout,
    /// Credits & Attribution's Credits button (WinScreen without the poem).
    Credits,
    CopyAgentDetails,
    /// Skin Customization: the next skin of `skins/`, the other model, the system's file picker
    /// (phones), the folder, Ely.by's skins.
    NextSkin,
    SkinModel,
    PickSkin,
    OpenSkinFolder,
    ElyBySkins,
}
enum WKind {
    Button {
        label: Text,
        active: bool,
        act: Act,
    },
    Slider {
        label: Text,
        value: f32,
        opt: Opt,
    },
    /// A pack's option shown as a slider (`sliders=`): its place in its list of values.
    ShaderSlider {
        label: Text,
        value: f32,
        name: String,
    },
    Text {
        text: Text,
        color: u32,
        centered: bool,
    },
    LanguageRow {
        label: Text,
        selected: bool,
        code: String,
    },
}
struct Widget {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    kind: WKind,
    /// Scrolls with the list (clipped to the list area).
    list: bool,
}
/// What the game must do after an interaction.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// An accepted button press or the release of a slider.
    ButtonClick,
    /// Close the options screens (back to the pause menu or title).
    Close,
    /// Settings changed: apply them.
    Changed,
    /// The language changed (reload translations).
    Language(String),
    /// The pack selection changed (reload resources).
    Packs(Vec<String>),
    OpenPackFolder,
    OpenModFolder,
    /// Open mod `i`'s settings screen (`screens.set-config`) over the Mods page.
    ModConfig(usize),
    /// The Shader Packs page opened: list `shaderpacks/` into `shader_packs`.
    ListShaderPacks,
    /// A shader pack or its profile was picked, or the pack turned off: save and apply it.
    ShaderPack,
    /// The Shader Options page opened: read the picked pack's options into `shader_options`.
    OpenShaderOptions,
    /// The Shader Options page closed with changes: save them and load the pack again.
    ShaderOptions,
    /// The Shader Packs page's folder (on a phone without the Starling folder, its offer).
    OpenShaderPackFolder,
    /// Open the mod's own settings page (capability `ui`): the installed
    /// mod's index in `mods.installed`.
    ModSettings(usize),
    /// Button `.1` of native mod `.0`'s page was pressed: the game tells the
    /// mod and refreshes the page.
    NativeButton(usize, usize),
    /// Close the options and edit the touch controls' layout over the world.
    TouchLayout,
    /// The Custom Buttons page opened: `Settings::custom_buttons` takes what the layout has.
    OpenCustomButtons,
    /// The player's own touch buttons changed: the layout takes them.
    CustomButtons,
    /// Roll the credits (`CreditsAndAttributionScreen.openCreditsScreen`).
    Credits,
    /// Copy how an agent connects (the AI Agent page).
    CopyAgentDetails,
}
/// What Skin Customization asks of the game (`OptionsUi::take_skin_actions`), apart from
/// [`Event`]: the menus before the game, which don't show the page, need not know them.
#[derive(Clone, Debug, PartialEq)]
pub enum SkinAction {
    /// The name, checked (`offline_name::is_valid_choice`): keep it.
    Name(String),
    /// The skin or model changed: keep it and put it on.
    Choice(crate::own_skin::SkinChoice),
    /// The skins folder (on a phone without the Starling folder, its offer).
    OpenFolder,
    /// Ask the system for a skin file; it arrives through the import inbox.
    PickFile,
    /// Ely.by Skins turned on or off (`elyby`): keep it.
    ElyBy(bool),
}
/// `SliderableValueSet.applyValueImmediately`: these apply 600 ms after
/// the last change (`OptionInstanceSliderButton.delayedApplyAt`).
fn delays_apply(opt: Opt) -> bool {
    matches!(
        opt,
        Opt::RenderDistance | Opt::SimulationDistance | Opt::BiomeBlend | Opt::WheelSensitivity
    )
}
/// Whether a slider still shows its own position: while held, or for a
/// delayed one until its value applies (both then `resetValue`).
fn slider_unsnapped(opt: Opt, held: bool, moved: Instant) -> bool {
    if delays_apply(opt) {
        moved.elapsed().as_millis() < 600
    } else {
        held
    }
}
pub struct OptionsUi {
    pub stack: Vec<Page>,
    scroll: f32,
    pub(crate) dragging: Option<Opt>,
    /// The pack's option slider held, and where the finger holds it.
    shader_dragging: Option<(String, f32)>,
    /// The pressed slider's own position (`AbstractSliderButton.value`) and
    /// when it last moved: the handle follows the pointer while the option
    /// keeps its stepped value, then snaps to it (`resetValue`).
    slider_value: Option<(Opt, f32, Instant)>,
    /// The pointer's last y while a scrollbar is held.
    pub(crate) scrollbar_drag_y: Option<f32>,
    /// Key Binds: the mapping waiting for a key ([`NATIVE_KEY`]: a native
    /// mod's, `selected_native_key`).
    pub selected_key: Option<&'static str>,
    /// The native mod's key waiting: (mod, key) in `mods.installed`.
    selected_native_key: Option<(usize, usize)>,
    /// Controller Binds: the action waiting for a pad button.
    pub selected_pad_action: Option<&'static str>,
    /// The glyphs the binds list shows (the setting, else the pad's family).
    pub pad_style: GlyphStyle,
    pub languages: Vec<LanguageInfo>,
    language_choice: Option<String>,
    language_search: ui::EditBox,
    pub(crate) settings_search: ui::EditBox,
    /// What the custom button on its page sends.
    custom_send: ui::EditBox,
    /// Packs: the packs found and the selection the screen opens with.
    pub pack_infos: Vec<crate::pack_screen::PackInfo>,
    pub pack_selection: Vec<String>,
    /// The open pack screen, made at its first frame.
    pub pack_screen: Option<crate::pack_screen::PackScreen>,
    /// A reload failure, dismissed before returning to the current page.
    pub resource_error: Option<String>,
    /// When the pack screen's search box took the focus (its cursor blink).
    pack_focus: Instant,
    pub in_world: bool,
    /// Scroll the selected language into the middle on the next layout.
    center_language: bool,
    pub graphics: crate::graphics_ui::GraphicsUi,
    /// The AI Agent page's status lines (set by the game while it shows).
    pub agent_status: Vec<String>,
    /// The Shader Packs page's chosen pack's profiles ("" first: its defaults).
    shader_profiles: Vec<String>,
    /// Whether Options opens Skin Customization: the game's options do; the menus before the
    /// game, which can't put a skin on, keep vanilla's greyed button.
    pub skin_page: bool,
    skin: SkinPage,
}
/// The Skin Customization page: the name being typed and the skins to choose from, read from
/// beside options.txt when the page opens.
struct SkinPage {
    name: ui::EditBox,
    /// The name worlds open with now (`offline_name::for_worlds`).
    saved: String,
    /// The signed-in account's name, which servers get.
    account: Option<String>,
    /// The PNGs of `skins/` (`own_skin::list`).
    files: Vec<String>,
    choice: crate::own_skin::SkinChoice,
    /// Ely.by Skins (`elyby::enabled`).
    elyby: bool,
    /// A name refused, or a skin the game could not put on.
    warning: Option<String>,
    /// For the game, oldest first.
    actions: Vec<SkinAction>,
}
impl Default for SkinPage {
    fn default() -> Self {
        let mut name = ui::EditBox::new(0.0, 0.0, 270.0, 20.0);
        name.max_length = 16;
        name.focused = false;
        Self {
            name,
            saved: String::new(),
            account: None,
            files: Vec::new(),
            choice: Default::default(),
            elyby: true,
            warning: None,
            actions: Vec::new(),
        }
    }
}
impl Default for OptionsUi {
    fn default() -> Self {
        let mut search = ui::EditBox::new(0.0, 0.0, 200.0, 15.0);
        search.max_length = 32;
        let mut settings_search = ui::EditBox::new(0.0, 0.0, 200.0, 15.0);
        settings_search.focused = false;
        let mut custom_send = ui::EditBox::new(0.0, 0.0, 310.0, 20.0);
        custom_send.max_length = 256;
        custom_send.focused = false;
        Self {
            stack: vec![Page::Main],
            scroll: 0.0,
            dragging: None,
            shader_dragging: None,
            slider_value: None,
            scrollbar_drag_y: None,
            selected_key: None,
            selected_native_key: None,
            selected_pad_action: None,
            pad_style: GlyphStyle::Xbox,
            languages: Vec::new(),
            language_choice: None,
            language_search: search,
            settings_search,
            custom_send,
            pack_infos: Vec::new(),
            pack_selection: Vec::new(),
            pack_screen: None,
            resource_error: None,
            pack_focus: Instant::now(),
            in_world: false,
            center_language: false,
            graphics: Default::default(),
            agent_status: Vec::new(),
            shader_profiles: Vec::new(),
            skin_page: false,
            skin: SkinPage::default(),
        }
    }
}
fn button(x: f32, y: f32, w: f32, label: Text, act: Act) -> Widget {
    Widget {
        x,
        y,
        w,
        h: 20.0,
        kind: WKind::Button {
            label,
            active: true,
            act,
        },
        list: false,
    }
}
fn inactive(mut w: Widget) -> Widget {
    if let WKind::Button { active, .. } = &mut w.kind {
        *active = false;
    }
    w
}
fn option_widget(s: &Settings, opt: Opt, x: f32, y: f32, w: f32) -> Widget {
    if !opt.available() {
        return inactive(button(x, y, w, opt.label(s), Act::Cycle(opt)));
    }
    match opt.kind(s) {
        Kind::Slider(v) => Widget {
            x,
            y,
            w,
            h: 20.0,
            kind: WKind::Slider {
                label: opt.label(s),
                value: v,
                opt,
            },
            list: false,
        },
        Kind::Cycle => button(x, y, w, opt.label(s), Act::Cycle(opt)),
    }
}
/// What a button opening one of Horizon's pages says on hover.
fn horizon_page_tooltip(page: Page) -> Option<&'static str> {
    Some(match page {
        Page::Horizon => "Horizon's own settings: graphics, controllers, extras and mods.",
        Page::Graphics => "Graphics, simply: each setting says what it costs.",
        Page::GraphicsAdvanced => {
            "Every graphics setting, Horizon's included, with what each costs."
        }
        Page::Controller => "Controllers: their buttons, sticks, vibration and menu navigation.",
        Page::Extras => {
            "Optional additions, all off by default: the game stays vanilla until you turn one on."
        }
        Page::Agent => "Let your own AI agent play: horizon-ctl and MCP clients on this computer.",
        Page::ShaderPacks => {
            "Shader packs from the shaderpacks folder: Horizon's own and those in the shaders/ format. One runs at a time."
        }
        Page::Mods => {
            "Mods made for Horizon, from the horizon-mods folder. Each runs in a sandbox and stays off until you turn it on."
        }
        _ => return None,
    })
}
/// The open screen of the picked pack: its title, then its items, one to a row on a phone, where
/// long names fit and a finger finds them, else as the pack lays them out.
fn shader_option_rows(s: &Settings) -> Vec<Row> {
    let Some(options) = &s.shader_options else {
        return vec![Row::Header("Reading the shader pack's options...")];
    };
    let columns = if cfg!(target_os = "android") {
        1
    } else {
        options
            .screens
            .screen(options.screen())
            .map_or(2, |(_, c)| c)
    };
    let width = if columns == 1 { 310.0 } else { 150.0 };
    let widget = |item: &horizon_shaders::ScreenItem| -> Option<Widget> {
        use horizon_shaders::ScreenItem;
        Some(match item {
            ScreenItem::Empty if columns != 2 => return None,
            ScreenItem::Empty => Widget {
                x: 0.0,
                y: 0.0,
                w: 150.0,
                h: 20.0,
                kind: WKind::Text {
                    text: Text::literal(""),
                    color: 0,
                    centered: false,
                },
                list: false,
            },
            ScreenItem::Profile => button(
                0.0,
                0.0,
                width,
                options.profile_label(),
                Act::ShaderOptionsProfile,
            ),
            ScreenItem::Screen(name) => button(
                0.0,
                0.0,
                width,
                options.screen_label(name),
                Act::ShaderScreen(name.clone()),
            ),
            ScreenItem::Option(name) => match options.slider_position(name) {
                Some(value) => Widget {
                    x: 0.0,
                    y: 0.0,
                    w: width,
                    h: 20.0,
                    kind: WKind::ShaderSlider {
                        label: options.option_label(name),
                        value,
                        name: name.clone(),
                    },
                    list: false,
                },
                None => button(
                    0.0,
                    0.0,
                    width,
                    options.option_label(name),
                    Act::ShaderOption(name.clone()),
                ),
            },
        })
    };
    let widgets: Vec<Widget> = options.items().iter().filter_map(widget).collect();
    let mut rows = vec![Row::Note(options.title().strip_legacy().plain())];
    if columns == 1 {
        rows.extend(widgets.into_iter().map(|w| Row::Buttons(w, None)));
        return rows;
    }
    let mut widgets = widgets.into_iter();
    while let Some(a) = widgets.next() {
        rows.push(Row::Buttons(a, widgets.next()));
    }
    rows
}
/// Rows of an OptionsList: (big?, options) or a header.
enum Row {
    Big(Opt),
    Pair(Opt, Option<Opt>),
    Header(&'static str),
    /// A line of grey text (a path).
    Note(String),
    /// A button, and another after it.
    Buttons(Widget, Option<Widget>),
    /// A line of grey text under the row before (a mod's description).
    About(String),
    /// Skin Customization's name box, after its label.
    Field,
    /// What a custom button sends.
    SendField,
    /// A line of red text: what was refused.
    Warning(String),
}
fn pairs(opts: &[Opt]) -> Vec<Row> {
    opts.chunks(2)
        .map(|c| Row::Pair(c[0], c.get(1).copied()))
        .collect()
}
impl OptionsUi {
    /// A switch of `horizon-extras.txt` as the Extras page draws it: "Caption: On".
    fn extra_button(&self, s: &Settings, item: crate::extras::Item) -> Widget {
        let label = format!("{}: {}", item.caption(), item.value(&s.extras));
        button(0.0, 0.0, 150.0, Text::literal(label), Act::Extra(item))
    }
    fn settings_search_active(&self) -> bool {
        self.settings_search.focused
            && matches!(self.page(), Page::Video | Page::Controls | Page::KeyBinds)
    }
    /// Whether a text field shown now has the focus: the language and settings searches, the
    /// pack search, or Skin Customization's name.
    pub fn text_focused(&self) -> bool {
        match self.page() {
            Page::Language => self.language_search.focused,
            Page::Packs => self
                .pack_screen
                .as_ref()
                .is_some_and(|p| p.search_focused()),
            Page::Skin => self.skin.name.focused,
            Page::CustomButton(_) => self.custom_send.focused,
            _ => self.settings_search_active(),
        }
    }
    /// Whether `(x, y)` (GUI pixels) is on the focused text field shown now.
    pub fn text_field_at(&self, x: f32, y: f32) -> bool {
        let on = |e: &ui::EditBox| {
            e.focused && x >= e.x && x < e.x + e.width && y >= e.y && y < e.y + e.height
        };
        match self.page() {
            Page::Language => on(&self.language_search),
            Page::Packs => self.pack_screen.as_ref().is_some_and(|p| p.search_at(x, y)),
            Page::Skin => on(&self.skin.name),
            Page::CustomButton(_) => on(&self.custom_send),
            _ => self.settings_search_active() && on(&self.settings_search),
        }
    }
    /// Read the Skin Customization page's name, skins and choice again (it opened, or a skin
    /// came through the import inbox).
    pub fn refresh_skin_page(&mut self) {
        let dir = std::env::current_dir().unwrap_or_default();
        let page = &mut self.skin;
        page.saved = crate::offline_name::for_worlds(&dir);
        page.account = crate::auth::current().map(|a| a.name.clone());
        page.files = crate::own_skin::list(&dir.join(crate::own_skin::DIR));
        page.choice = crate::own_skin::SkinChoice::load(&dir);
        page.elyby = crate::elyby::enabled(&dir);
        page.warning = None;
        if !page.name.focused {
            page.name.set_value(&page.saved);
        }
    }
    /// A line in red on the Skin Customization page (a skin the game could not put on).
    pub fn skin_warning(&mut self, text: String) {
        self.skin.warning = Some(text);
    }
    /// What Skin Customization asked of the game since the last call.
    pub fn take_skin_actions(&mut self) -> Vec<SkinAction> {
        std::mem::take(&mut self.skin.actions)
    }
    /// The name box loses the focus: a changed name is kept when it is one, else refused and
    /// the old one back in the box.
    fn commit_name(&mut self) {
        let page = &mut self.skin;
        page.name.focused = false;
        let typed = page.name.value.clone();
        if typed == page.saved {
            return;
        }
        if crate::offline_name::is_valid_choice(&typed) {
            page.saved = typed.clone();
            page.warning = None;
            page.actions.push(SkinAction::Name(typed));
            return;
        }
        page.warning = Some(format!(
            "Not a name: 3 to 16 letters, digits or _ ({} kept)",
            page.saved
        ));
        page.name.set_value(&page.saved);
    }
    pub fn page(&self) -> Page {
        *self.stack.last().unwrap_or(&Page::Main)
    }
    /// Jump straight to a page (control socket / tests).
    pub fn show(&mut self, page: Page) {
        if page != Page::Main {
            self.push(page);
        }
    }
    pub fn open(&mut self, in_world: bool) {
        self.open_at(in_world, Page::Main);
    }
    /// Open a page whose Done/Escape returns directly to the calling screen.
    pub fn open_at(&mut self, in_world: bool, page: Page) {
        self.stack.clear();
        self.push(page);
        self.in_world = in_world;
        self.selected_key = None;
    }
    fn push(&mut self, page: Page) {
        self.stack.push(page);
        self.scroll = 0.0;
        self.scrollbar_drag_y = None;
        if matches!(page, Page::Video | Page::Controls | Page::KeyBinds) {
            self.settings_search.set_value("");
            self.settings_search.focused = false;
        }
        if page == Page::Language {
            self.language_choice = None;
            self.language_search.set_value("");
            self.center_language = true;
        }
        if page == Page::Packs {
            self.pack_screen = None;
        }
        if matches!(page, Page::Graphics | Page::GraphicsAdvanced) {
            self.graphics.reset();
        }
        if page == Page::Skin {
            self.skin.name.focused = false;
            self.refresh_skin_page();
        }
    }
    /// The graphics screen level of `page`, if it is one.
    fn graphics_level(page: Page) -> Option<crate::graphics_ui::Level> {
        match page {
            Page::Graphics => Some(crate::graphics_ui::Level::Simple),
            Page::GraphicsAdvanced => Some(crate::graphics_ui::Level::Advanced),
            _ => None,
        }
    }
    fn title(page: Page) -> &'static str {
        match page {
            Page::Main => "options.title",
            Page::Video => "options.videoTitle",
            Page::Sound => "options.sounds.title",
            Page::Skin => "options.skinCustomisation.title",
            Page::Online => "options.online.title",
            Page::Controls => "controls.title",
            Page::KeyBinds => "controls.keybinds.title",
            Page::Mouse => "options.mouse_settings.title",
            Page::Chat => "options.chat.title",
            Page::Accessibility => "options.accessibility.title",
            Page::Language => "options.language.title",
            Page::Font => "options.font.title",
            Page::Packs => "resourcePack.title",
            Page::Mods => "Mods",
            Page::NativeMod(_) => "Mod Settings",
            Page::ShaderPacks => "Shader Packs",
            Page::ShaderOptions => "Shader Options",
            Page::CustomButtons | Page::CustomButton(_) => "Custom Buttons",
            Page::Graphics | Page::GraphicsAdvanced => "options.videoTitle",
            Page::Controller => "Controller Settings",
            Page::PadBinds => "Controller Binds",
            Page::Extras => "Horizon Extras",
            Page::Credits => "credits_and_attribution.screen.title",
            Page::Horizon => "Horizon Settings",
            Page::Agent => "AI Agent",
        }
    }
    fn scrollbar_x(&self, width: f32) -> f32 {
        (width / 2.0).floor()
            + match self.page() {
                Page::KeyBinds | Page::PadBinds => 178.0,
                Page::Language => 118.0,
                _ => 163.0,
            }
    }
    fn filter_option_rows(&self, rows: Vec<Row>) -> Vec<Row> {
        let query = self.settings_search.value.to_lowercase();
        if query.is_empty() {
            return rows;
        }
        let mut filtered = Vec::new();
        let mut pending_header = None;
        for row in rows {
            let matches = |opt: Opt| lang::get(opt.caption()).to_lowercase().contains(&query);
            let visible = match row {
                Row::Header(key) => {
                    pending_header = Some(key);
                    continue;
                }
                Row::Big(opt) if matches(opt) => vec![Row::Big(opt)],
                Row::Pair(a, b) => pairs(
                    &[Some(a), b]
                        .into_iter()
                        .flatten()
                        .filter(|opt| matches(*opt))
                        .collect::<Vec<_>>(),
                ),
                // Keep navigation available regardless of the query.
                Row::Buttons(a, b) => {
                    filtered.push(Row::Buttons(a, b));
                    continue;
                }
                _ => Vec::new(),
            };
            if !visible.is_empty() {
                // Emit a section heading only when its first matching option is found.
                if let Some(key) = pending_header.take() {
                    filtered.push(Row::Header(key));
                }
                filtered.extend(visible);
            }
        }
        filtered
    }

    /// A native mod's page: its name, its keys as Key Binds rows (the
    /// player's choice, else the default), then its buttons.
    fn native_rows(&self, s: &Settings, i: usize) -> Vec<Row> {
        let Some(m) = s.mods.installed.get(i) else {
            return Vec::new();
        };
        let mut rows = vec![Row::Note(m.title.clone())];
        let Some(page) = &m.native else {
            return rows;
        };
        let chosen = s.mods.keys.get(&m.id);
        for (k, [name, label, default]) in page.keys.iter().enumerate() {
            let key = chosen.and_then(|c| c.get(name)).unwrap_or(default);
            let mut message = keybinds::display_name(key);
            if self.selected_native_key == Some((i, k)) {
                message = Text::literal("> ")
                    .append(message.with_color(text::WHITE))
                    .append(" <")
                    .with_color(text::YELLOW);
            }
            let caption = button(0.0, 0.0, 150.0, Text::literal(label.clone()), Act::None);
            let change = button(0.0, 0.0, 150.0, message, Act::NativeKey(i, k));
            rows.push(Row::Buttons(inactive(caption), Some(change)));
        }
        for (b, label) in page.buttons.iter().enumerate() {
            let widget = button(
                0.0,
                0.0,
                310.0,
                Text::literal(label.clone()),
                Act::NativeButton(i, b),
            );
            rows.push(Row::Buttons(widget, None));
        }
        rows
    }

    fn list_rows(&self, s: &Settings) -> Vec<Row> {
        match self.page() {
            Page::Extras => {
                // Each section's switches two to a row, as vanilla's
                // OptionsList.addSmall lays out an options screen.
                let extra = |item: crate::extras::Item| {
                    let value = format!("{}: {}", item.caption(), item.value(&s.extras));
                    let label = if item.key_name().is_some() && self.selected_key == item.key_name()
                    {
                        Text::literal(format!("> {value} <")).with_color(text::YELLOW)
                    } else {
                        Text::literal(value)
                    };
                    let widget = button(0.0, 0.0, 150.0, label, Act::Extra(item));
                    // A setting of a switch that is off does nothing: inactive,
                    // as vanilla greys out an option that can't apply.
                    if item
                        .parent()
                        .is_some_and(|parent| parent.switch(&s.extras) == Some(false))
                    {
                        inactive(widget)
                    } else {
                        widget
                    }
                };
                let mut rows = Vec::new();
                let mut section = Vec::new();
                let flush = |rows: &mut Vec<Row>, section: &mut Vec<crate::extras::Item>| {
                    rows.extend(
                        section
                            .chunks(2)
                            .map(|c| Row::Buttons(extra(c[0]), c.get(1).map(|&b| extra(b)))),
                    );
                    section.clear();
                };
                for row in crate::extras::rows() {
                    match row {
                        Err(header) => {
                            flush(&mut rows, &mut section);
                            rows.push(Row::Header(header));
                        }
                        Ok(item) => section.push(item),
                    }
                }
                flush(&mut rows, &mut section);
                rows
            }
            Page::Horizon => {
                // Two to a row, as vanilla's OptionsList.addSmall lays out
                // buttons; each opens a page built from vanilla's widgets.
                let open = |label: &str, page: Page| {
                    button(0.0, 0.0, 150.0, Text::literal(label), Act::Push(page))
                };
                vec![
                    Row::Buttons(
                        open("Graphics...", Page::Graphics),
                        Some(open("Advanced Graphics...", Page::GraphicsAdvanced)),
                    ),
                    Row::Buttons(
                        open("Shader Packs...", Page::ShaderPacks),
                        Some(open("Controller Settings...", Page::Controller)),
                    ),
                    Row::Buttons(
                        open("Extras...", Page::Extras),
                        Some(open("Mods...", Page::Mods)),
                    ),
                    // No AI hides the AI Agent page and everything AI.
                    {
                        let ai = if s.extras.no_ai {
                            self.extra_button(s, crate::extras::Item::NoAi)
                        } else {
                            open("AI Agent...", Page::Agent)
                        };
                        // A phone has no Discord client to show the game in.
                        if cfg!(target_os = "android") {
                            Row::Buttons(ai, None)
                        } else {
                            Row::Buttons(
                                self.extra_button(s, crate::extras::Item::DiscordPresence),
                                Some(ai),
                            )
                        }
                    },
                ]
                .into_iter()
                .chain(
                    (!s.extras.no_ai).then(|| {
                        Row::Buttons(self.extra_button(s, crate::extras::Item::NoAi), None)
                    }),
                )
                .collect()
            }
            Page::Agent => {
                let mut rows = vec![Row::Buttons(
                    self.extra_button(s, crate::extras::Item::AgentControl),
                    Some(button(
                        0.0,
                        0.0,
                        150.0,
                        Text::literal("Copy Connection Details"),
                        Act::CopyAgentDetails,
                    )),
                )];
                rows.extend(self.agent_status.iter().cloned().map(Row::Note));
                rows
            }
            Page::Skin => {
                // SkinCustomizationScreen: the seven layers, then Main Hand; in the game's own
                // options, the player's name and skin before them.
                let mut opts: Vec<Opt> = (0..MODEL_PART_KEYS.len()).map(Opt::ModelPart).collect();
                opts.push(Opt::MainHand);
                let layers = pairs(&opts);
                if !self.skin_page {
                    return layers;
                }
                let page = &self.skin;
                let mut rows = vec![Row::Field];
                rows.push(match (&page.warning, &page.account) {
                    (Some(warning), _) => Row::Warning(warning.clone()),
                    // Online-mode servers take only the account's name, and the game can't tell
                    // them from the others before it joins.
                    (None, Some(account)) => {
                        Row::Note(format!("Worlds use this name; servers use {account}"))
                    }
                    (None, None) => {
                        Row::Note("Your name in worlds and on servers, from the next one".into())
                    }
                });
                let skin = page
                    .choice
                    .file
                    .as_deref()
                    .map_or("Default", crate::own_skin::shown_name);
                let skin = button(
                    0.0,
                    0.0,
                    150.0,
                    Text::literal(format!("Skin: {}", clipped(skin, 18))),
                    Act::NextSkin,
                );
                let model = if page.choice.slim { "Slim" } else { "Classic" };
                let model = button(
                    0.0,
                    0.0,
                    150.0,
                    Text::literal(format!("Model: {model}")),
                    Act::SkinModel,
                );
                // The default skin's model goes with it (Steve's or Alex's, by the UUID).
                let model = if page.choice.file.is_some() {
                    model
                } else {
                    inactive(model)
                };
                rows.push(Row::Buttons(skin, Some(model)));
                let elyby = |width: f32| {
                    let on = if page.elyby { "ON" } else { "OFF" };
                    button(
                        0.0,
                        0.0,
                        width,
                        Text::literal(format!("Ely.by Skins: {on}")),
                        Act::ElyBySkins,
                    )
                };
                // A phone's own file picker: the file chosen comes through the import inbox.
                if cfg!(target_os = "android") {
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal("Choose a File..."),
                            Act::PickSkin,
                        ),
                        Some(elyby(150.0)),
                    ));
                } else {
                    rows.push(Row::Buttons(elyby(310.0), None));
                }
                if page.files.is_empty() {
                    rows.push(Row::Note(
                        "No skins yet: put 64x64 PNGs in the skins folder".into(),
                    ));
                }
                // Vanilla's protocol: other players get the skin of the profile from the server.
                rows.push(Row::Note(
                    "Only you see this skin. On a server, others see".into(),
                ));
                rows.push(Row::Note(
                    "the skin Mojang has for your account, or the default".into(),
                ));
                // Offline play's skins: by the player's name, where nothing else gives one.
                if page.elyby {
                    rows.push(Row::Note(
                        "Ely.by Skins: a player without a skin shows the one".into(),
                    ));
                    rows.push(Row::Note(
                        "their name has on ely.by (yours too, on Default)".into(),
                    ));
                }
                rows.extend(layers);
                rows
            }
            Page::Video => {
                // Horizon's graphics screens, Simple and Advanced.
                let mut rows = vec![Row::Buttons(
                    button(
                        0.0,
                        0.0,
                        150.0,
                        Text::literal("Horizon Graphics..."),
                        Act::Push(Page::Graphics),
                    ),
                    Some(button(
                        0.0,
                        0.0,
                        150.0,
                        Text::literal("Advanced Graphics..."),
                        Act::Push(Page::GraphicsAdvanced),
                    )),
                )];
                rows.push(Row::Buttons(
                    button(
                        0.0,
                        0.0,
                        310.0,
                        Text::literal("Shader Packs..."),
                        Act::Push(Page::ShaderPacks),
                    ),
                    None,
                ));
                rows.push(Row::Header("options.video.display.header"));
                // A phone's app always fills its screen.
                let display: &[Opt] = if cfg!(target_os = "android") {
                    &[
                        Opt::MaxFps,
                        Opt::Vsync,
                        Opt::InactivityFps,
                        Opt::GuiScale,
                        Opt::Gamma,
                    ]
                } else {
                    &[
                        Opt::MaxFps,
                        Opt::Vsync,
                        Opt::InactivityFps,
                        Opt::GuiScale,
                        Opt::Fullscreen,
                        Opt::Gamma,
                    ]
                };
                rows.extend(pairs(display));
                rows.push(Row::Header("options.video.quality.header"));
                rows.push(Row::Big(Opt::Preset));
                rows.extend(pairs(&[
                    Opt::BiomeBlend,
                    Opt::RenderDistance,
                    Opt::ChunkBuilder,
                    Opt::SimulationDistance,
                    Opt::SmoothLighting,
                    Opt::Clouds,
                    Opt::Particles,
                    Opt::Mipmaps,
                    Opt::EntityShadows,
                    Opt::EntityDistance,
                    Opt::MenuBlur,
                    Opt::CloudRange,
                    Opt::CutoutLeaves,
                    Opt::ImprovedTransparency,
                    Opt::WeatherRadius,
                ]));
                rows.push(Row::Header("options.video.preferences.header"));
                rows.extend(pairs(&[
                    Opt::Autosave,
                    Opt::Vignette,
                    Opt::AttackIndicator,
                    Opt::ChunkFade,
                ]));
                self.filter_option_rows(rows)
            }
            Page::Sound => {
                // SoundOptionsScreen, without Audio Device: Horizon opens the
                // output device itself and cannot switch it.
                let mut rows = vec![Row::Big(Opt::Volume(0))];
                let others: Vec<Opt> = (1..SOUND_SOURCES.len()).map(Opt::Volume).collect();
                rows.extend(pairs(&others));
                rows.extend(pairs(&[Opt::ShowSubtitles, Opt::DirectionalAudio]));
                rows.extend(pairs(&[Opt::MusicFrequency, Opt::MusicToast]));
                rows
            }
            Page::Online => {
                // OnlineOptionsScreen without the Xbox and friends-list parts.
                let mut rows = vec![Row::Header("options.online.friends.header")];
                rows.extend(pairs(&[Opt::InGameNotification, Opt::SharePresence]));
                rows.push(Row::Header("options.online.servers.header"));
                rows.push(Row::Big(Opt::AllowServerListing));
                rows.push(Row::Header("options.online.realms.header"));
                rows.push(Row::Big(Opt::RealmsNotifications));
                rows
            }
            Page::Controls => {
                let mut rows = vec![Row::Buttons(
                    button(
                        0.0,
                        0.0,
                        150.0,
                        tr("options.mouse_settings"),
                        Act::Push(Page::Mouse),
                    ),
                    Some(button(
                        0.0,
                        0.0,
                        150.0,
                        tr("controls.keybinds"),
                        Act::Push(Page::KeyBinds),
                    )),
                )];
                rows.extend(pairs(&[
                    Opt::ToggleCrouch,
                    Opt::ToggleSprint,
                    Opt::AutoJump,
                ]));
                // Horizon's controller page, after vanilla's rows.
                rows.push(Row::Buttons(
                    button(
                        0.0,
                        0.0,
                        310.0,
                        Text::literal("Controller Settings..."),
                        Act::Push(Page::Controller),
                    ),
                    None,
                ));
                // The touch controls' layout, where there is a touch screen to play on.
                if cfg!(target_os = "android") {
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            310.0,
                            Text::literal("Touch Controls..."),
                            Act::TouchLayout,
                        ),
                        None,
                    ));
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            310.0,
                            Text::literal("Custom Buttons..."),
                            Act::CustomButtons,
                        ),
                        None,
                    ));
                }
                self.filter_option_rows(rows)
            }
            Page::Controller => {
                let mut rows = vec![Row::Buttons(
                    button(
                        0.0,
                        0.0,
                        310.0,
                        Text::literal("Controller Binds..."),
                        Act::Push(Page::PadBinds),
                    ),
                    None,
                )];
                // A phone reads its controllers through Android (game/pad/android.rs): the
                // DualSense's own output (its triggers and lightbar) and Steam's keyboard are
                // the computer's only.
                let opts: &[Opt] = if cfg!(target_os = "android") {
                    &[
                        Opt::PadEnabled,
                        Opt::PadVibration,
                        Opt::PadLookSensitivity,
                        Opt::PadToggleSneak,
                        Opt::PadServerStick,
                        Opt::PadStickPrompt,
                        Opt::PadInvertX,
                        Opt::PadInvertY,
                        Opt::PadMoveDeadzone,
                        Opt::PadLookDeadzone,
                        Opt::PadGlyphs,
                        Opt::PadMenuNav,
                    ]
                } else {
                    &[
                        Opt::PadEnabled,
                        Opt::PadVibration,
                        Opt::PadAdaptiveTriggers,
                        Opt::PadLightbar,
                        Opt::PadLookSensitivity,
                        Opt::PadToggleSneak,
                        Opt::PadServerStick,
                        Opt::PadStickPrompt,
                        Opt::PadInvertX,
                        Opt::PadInvertY,
                        Opt::PadMoveDeadzone,
                        Opt::PadLookDeadzone,
                        Opt::PadGlyphs,
                        Opt::PadMenuNav,
                        Opt::PadKeyboard,
                    ]
                };
                rows.extend(pairs(opts));
                rows
            }
            Page::Mouse => pairs(MOUSE_OPTIONS),
            Page::Chat => pairs(&[
                Opt::ChatVisibility,
                Opt::ChatColors,
                Opt::ChatLinks,
                Opt::ChatLinksPrompt,
                Opt::ChatOpacity,
                Opt::TextBackgroundOpacity,
                Opt::ChatScale,
                Opt::ChatLineSpacing,
                Opt::ChatWidth,
                Opt::ChatHeightFocused,
                Opt::ChatHeightUnfocused,
                Opt::AutoSuggestions,
                Opt::HideMatchedNames,
                Opt::ReducedDebugInfo,
                Opt::OnlySecureChat,
                Opt::SaveChatDrafts,
            ]),
            Page::Accessibility => {
                let mut rows = vec![Row::Buttons(
                    option_widget(s, Opt::Narrator, 0.0, 0.0, 150.0),
                    Some(button(
                        0.0,
                        0.0,
                        150.0,
                        tr("options.controls"),
                        Act::Push(Page::Controls),
                    )),
                )];
                rows.extend(pairs(&ACCESSIBILITY_OPTIONS[1..]));
                rows
            }
            Page::Font => pairs(&[Opt::ForceUnicode, Opt::JapaneseGlyphs]),
            Page::Mods => {
                if s.mods.installed.is_empty() {
                    return vec![Row::Header(
                        "No mods: put .hmod files in the horizon-mods folder",
                    )];
                }
                // Each mod: its switch (and its settings: Mod Menu's config button, or the page
                // of a mod with capability `ui` or of a native mod), then what it is, readable
                // without a mouse to hover.
                let mut rows = Vec::new();
                for (i, installed) in s.mods.installed.iter().enumerate() {
                    let mut label = Text::literal(format!("{}: ", installed.title))
                        .append(on_off(s.mods.is_on(i)));
                    if installed.problem.is_some() {
                        label = label.append(" (Stopped)");
                    } else if installed.dev {
                        label = label.append(" (Dev)");
                    }
                    let dead = installed.dev || installed.id.is_empty();
                    let (width, side) = if installed.config.is_some() {
                        let config =
                            button(0.0, 0.0, 64.0, Text::literal("Config"), Act::ModConfig(i));
                        (236.0, Some(config))
                    } else if installed.has_settings || installed.native.is_some() {
                        let settings = button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal("Settings"),
                            Act::ModSettings(i),
                        );
                        (150.0, Some(settings))
                    } else {
                        (310.0, None)
                    };
                    let widget = button(0.0, 0.0, width, label, Act::ToggleMod(i));
                    rows.push(Row::Buttons(
                        if dead { inactive(widget) } else { widget },
                        side,
                    ));
                    let about = installed.problem.as_deref().unwrap_or(&installed.summary);
                    if !about.is_empty() {
                        rows.push(Row::About(clipped(about, 52)));
                    }
                }
                rows
            }
            Page::NativeMod(i) => self.native_rows(s, i),
            Page::ShaderPacks => {
                use crate::shader_pack_list::Kind as PackKind;
                let choice = s.horizon.shader_choice();
                let pick = |on: bool, name: &str, act: Act| {
                    let label = if on {
                        format!("> {name} <")
                    } else {
                        name.to_owned()
                    };
                    Row::Buttons(button(0.0, 0.0, 310.0, Text::literal(label), act), None)
                };
                let mut rows = Vec::new();
                // Packs cost per pixel: what matters most, above however many packs there are.
                if let Some((pack, profile)) = &choice {
                    let scale = s.horizon.resolution_scale;
                    let limit = (crate::graphics::auto_shader_scale() * 100.0).round() as u32;
                    let limited = profile == crate::graphics::AUTO_PROFILE
                        && !crate::graphics::light_shader_pack(pack)
                        && scale > limit;
                    let resolution = if limited {
                        format!("Resolution: {limit}% (Auto)")
                    } else {
                        format!("Resolution: {scale}%")
                    };
                    let fps = match s.max_fps {
                        fps if fps >= 260 => "Off".to_owned(),
                        fps => fps.to_string(),
                    };
                    rows.push(Row::Note("Settings of the pack on".into()));
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal(resolution),
                            Act::ShaderResolution,
                        ),
                        Some(button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal(format!("FPS Limit: {fps}")),
                            Act::ShaderFpsLimit,
                        )),
                    ));
                    // Profiles are a `shaders/` pack's; Horizon's own have none.
                    if !s
                        .shader_packs
                        .iter()
                        .any(|p| p.file == *pack && p.kind == PackKind::Horizon)
                    {
                        let shown = match profile.as_str() {
                            "" => "Pack Default",
                            crate::graphics::AUTO_PROFILE => "Auto",
                            profile => profile,
                        };
                        rows.push(Row::Buttons(
                            button(
                                0.0,
                                0.0,
                                150.0,
                                Text::literal(format!("Profile: {shown}")),
                                Act::ShaderProfile,
                            ),
                            Some(button(
                                0.0,
                                0.0,
                                150.0,
                                Text::literal("Shader Options..."),
                                Act::ShaderOptions,
                            )),
                        ));
                    }
                }
                rows.push(Row::Note(
                    "Horizon's own packs, and Iris and OptiFine ones".into(),
                ));
                rows.push(Row::Note(shader_folder_note()));
                rows.push(pick(choice.is_none(), "Off", Act::PickShaderPack(None)));
                if s.shader_packs.is_empty() {
                    rows.push(Row::Header(
                        "No shader packs: put them in the shaderpacks folder",
                    ));
                }
                for (i, pack) in s.shader_packs.iter().enumerate() {
                    let on = choice.as_ref().is_some_and(|(p, _)| *p == pack.file);
                    let name = match pack.kind {
                        PackKind::Horizon => format!("{} (Horizon)", pack.title),
                        PackKind::Classic => pack.title.clone(),
                    };
                    rows.push(pick(on, &name, Act::PickShaderPack(Some(i))));
                }
                rows
            }
            Page::ShaderOptions => shader_option_rows(s),
            Page::CustomButtons => {
                let mut rows = vec![Row::Note(
                    "Buttons of your own: place them in Touch Controls".into(),
                )];
                for b in &s.custom_buttons {
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal(b.label()),
                            Act::CustomEdit(b.id),
                        ),
                        Some(button(
                            0.0,
                            0.0,
                            150.0,
                            Text::literal("Delete"),
                            Act::CustomDelete(b.id),
                        )),
                    ));
                }
                rows.push(Row::Buttons(
                    button(0.0, 0.0, 310.0, Text::literal("Add Button"), Act::CustomAdd),
                    None,
                ));
                rows
            }
            Page::CustomButton(id) => {
                let held =
                    s.custom_buttons
                        .iter()
                        .find(|b| b.id == id)
                        .and_then(|b| match &b.action {
                            crate::settings::CustomAction::Key(name) => Some(name.as_str()),
                            crate::settings::CustomAction::Send(_) => None,
                        });
                let mut rows = vec![
                    Row::Note("Sends a /command or a chat line when pressed".into()),
                    Row::SendField,
                    Row::Note("Or holds one of the keys of the game".into()),
                ];
                let mut mappings: Vec<(&'static str, &'static str)> =
                    MAPPINGS.iter().map(|(n, _, c)| (*n, *c)).collect();
                mappings.retain(|(name, _)| keybinds::key_code(s.keys.get(name)).is_some());
                mappings.sort_by(|a, b| {
                    let ca = CATEGORIES.iter().position(|c| *c == a.1).unwrap_or(99);
                    let cb = CATEGORIES.iter().position(|c| *c == b.1).unwrap_or(99);
                    ca.cmp(&cb)
                        .then_with(|| lang::get(a.0).cmp(&lang::get(b.0)))
                });
                for (name, _) in mappings {
                    let label = if held == Some(name) {
                        format!("> {} <", lang::get(name))
                    } else {
                        lang::get(name)
                    };
                    rows.push(Row::Buttons(
                        button(
                            0.0,
                            0.0,
                            310.0,
                            Text::literal(label),
                            Act::CustomKey(id, name),
                        ),
                        None,
                    ));
                }
                rows
            }
            _ => {
                let _ = s;
                Vec::new()
            }
        }
    }
    /// The chosen pack's profiles (`profile.<name>` in its shaders.properties, in
    /// `shaderpacks/` beside `options.txt`), for the Shader Packs page.
    fn refresh_shader_profiles(&mut self, s: &Settings) {
        let dir = std::env::current_dir()
            .unwrap_or_default()
            .join(crate::shader_pack_list::DIR);
        self.shader_profiles = vec![String::new()];
        if cfg!(target_os = "android") {
            self.shader_profiles
                .insert(0, crate::graphics::AUTO_PROFILE.to_owned());
        }
        if let Some((pack, _)) = s.horizon.shader_choice()
            && let Ok(pack) = horizon_shaders::Pack::open(&dir.join(pack))
        {
            let text = pack.read_text("shaders.properties").unwrap_or_default();
            let macros = horizon_shaders::Macros::standard();
            let properties = horizon_shaders::Properties::parse(&text, &macros);
            self.shader_profiles.extend(
                properties
                    .entries
                    .iter()
                    .filter_map(|e| e.key.strip_prefix("profile.").map(str::to_owned)),
            );
        }
    }
    /// Build every widget of the current page.
    fn layout(&mut self, s: &Settings, w: f32, h: f32) -> (Vec<Widget>, f32, f32, f32) {
        let mut out = Vec::new();
        let page = self.page();
        let cx = (w / 2.0).floor();
        if matches!(page, Page::Video | Page::Controls | Page::KeyBinds) {
            self.settings_search.x = cx - 100.0;
            self.settings_search.y = 17.0;
        }
        let done = |y: f32| button(cx - 100.0, y, 200.0, tr("gui.done"), Act::Done);
        // (list top, list bottom, content height)
        let (mut top, mut bottom, mut content) = (33.0, h - 33.0, 0.0);
        match page {
            Page::Main => {
                out.push(Widget {
                    x: 0.0,
                    y: 12.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr("options.title"),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                out.push(option_widget(s, Opt::Fov, cx - 154.0, 29.0, 150.0));
                out.push(button(
                    cx + 4.0,
                    29.0,
                    150.0,
                    tr("options.online"),
                    Act::Push(Page::Online),
                ));
                let entries = [
                    // Everywhere for the layers and Main Hand; `skin_page` adds the name and skin.
                    ("options.skinCustomisation", Some(Page::Skin)),
                    ("options.sounds", Some(Page::Sound)),
                    ("options.video", Some(Page::Video)),
                    ("options.controls", Some(Page::Controls)),
                    ("options.language", Some(Page::Language)),
                    ("options.chat", Some(Page::Chat)),
                    ("options.resourcepack", Some(Page::Packs)),
                    ("options.accessibility", Some(Page::Accessibility)),
                    // Vanilla's Telemetry Data and Credits slots: Horizon's
                    // settings and its mods, as a mod adds its button here.
                    // Credits & Attribution (WinScreen's credits) follows them.
                    ("Horizon Settings...", Some(Page::Horizon)),
                    ("Mods...", Some(Page::Mods)),
                    ("options.credits_and_attribution", Some(Page::Credits)),
                ];
                let rows = entries.len().div_ceil(2) as f32;
                let gy = (61.0 + 30.0f32).min(h - 33.0 - rows * 24.0);
                for (i, (key, page)) in entries.iter().enumerate() {
                    let x = cx - 158.0 + 4.0 + (i % 2) as f32 * 158.0;
                    let y = gy + (i / 2) as f32 * 24.0;
                    let b = button(x, y, 150.0, tr(key), page.map_or(Act::None, Act::Push));
                    out.push(if page.is_some() { b } else { inactive(b) });
                }
                out.push(done(h - 27.0));
                top = 0.0;
                bottom = 0.0;
            }
            Page::Credits => {
                // CreditsAndAttributionScreen: the title header, three 210-wide
                // buttons 8 apart (HeaderAndFooterLayout: 30 below the header
                // unless the footer is closer), Done.
                out.push(Widget {
                    x: 0.0,
                    y: 12.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr(Self::title(page)),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                let y = (33.0 + 30.0f32).min(h - 33.0 - 76.0);
                let x = cx - 105.0;
                out.push(button(
                    x,
                    y,
                    210.0,
                    tr("credits_and_attribution.button.credits"),
                    Act::Credits,
                ));
                // ConfirmLinkScreen to minecraft.net: Horizon opens no links.
                for (i, key) in [
                    "credits_and_attribution.button.attribution",
                    "credits_and_attribution.button.licenses",
                ]
                .into_iter()
                .enumerate()
                {
                    let y = y + 28.0 * (i + 1) as f32;
                    out.push(inactive(button(x, y, 210.0, tr(key), Act::None)));
                }
                out.push(done(h - 27.0));
                top = 0.0;
                bottom = 0.0;
            }
            Page::KeyBinds => {
                top = 38.0;
                out.push(Widget {
                    x: 0.0,
                    y: 4.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr(Self::title(page)),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                let mut mappings: Vec<(&str, &str)> =
                    MAPPINGS.iter().map(|(n, _, c)| (*n, *c)).collect();
                let query = self.settings_search.value.to_lowercase();
                mappings.retain(|(name, _)| {
                    query.is_empty() || lang::get(name).to_lowercase().contains(&query)
                });
                mappings.sort_by(|a, b| {
                    let ca = CATEGORIES.iter().position(|c| *c == a.1).unwrap_or(99);
                    let cb = CATEGORIES.iter().position(|c| *c == b.1).unwrap_or(99);
                    ca.cmp(&cb)
                        .then_with(|| lang::get(a.0).cmp(&lang::get(b.0)))
                });
                let row_left = cx - 170.0;
                let scrollbar_x = row_left + 340.0 + 6.0 + 2.0;
                let list_start = out.len();
                let mut y = top + 2.0 - self.scroll;
                let mut last_category = "";
                for (name, category) in mappings {
                    if category != last_category {
                        last_category = category;
                        let label =
                            Text::translate(&format!("key.category.minecraft.{category}"), &[])
                                .with_style(&Style {
                                    bold: Some(true),
                                    underlined: Some(true),
                                    ..Style::default()
                                });
                        out.push(Widget {
                            x: 0.0,
                            y: y + 7.0,
                            w,
                            h: 9.0,
                            kind: WKind::Text {
                                text: label,
                                color: u32::MAX,
                                centered: true,
                            },
                            list: false,
                        });
                        y += 20.0;
                    }
                    out.push(Widget {
                        x: row_left + 2.0,
                        y: y + 6.0,
                        w: 200.0,
                        h: 9.0,
                        kind: WKind::Text {
                            text: tr(name),
                            color: u32::MAX,
                            centered: false,
                        },
                        list: false,
                    });
                    let key = s.keys.get(name).to_owned();
                    let mut message = keybinds::display_name(&key);
                    let conflicts = s.keys.conflicts(name);
                    if !conflicts.is_empty() {
                        message = Text::literal("[ ")
                            .append(message.with_color(text::WHITE))
                            .append(" ]")
                            .with_color(text::YELLOW);
                    }
                    if self.selected_key == Some(name) {
                        message = Text::literal("> ")
                            .append(message.with_style(&Style {
                                color: Some(text::WHITE),
                                underlined: Some(true),
                                ..Style::default()
                            }))
                            .append(" <")
                            .with_color(text::YELLOW);
                    }
                    let reset_x = scrollbar_x - 50.0 - 10.0;
                    let change_x = reset_x - 5.0 - 75.0;
                    let name_static = MAPPINGS.iter().find(|m| m.0 == name).map_or("", |m| m.0);
                    out.push(button(change_x, y, 75.0, message, Act::Key(name_static)));
                    if !conflicts.is_empty() {
                        out.push(Widget {
                            x: change_x - 6.0,
                            y: y + 1.0,
                            w: 3.0,
                            h: 18.0,
                            kind: WKind::Text {
                                text: Text::empty(),
                                color: rgba(255, 255, 0, 255),
                                centered: false,
                            },
                            list: false,
                        });
                    }
                    let reset = button(
                        reset_x,
                        y,
                        50.0,
                        tr("controls.reset"),
                        Act::ResetKey(name_static),
                    );
                    out.push(if s.keys.is_default(name) {
                        inactive(reset)
                    } else {
                        reset
                    });
                    y += 20.0;
                }
                content = y + self.scroll - top + 2.0;
                out[list_start..].iter_mut().for_each(|w| w.list = true);
                out.push(button(
                    cx - 154.0,
                    h - 27.0,
                    150.0,
                    tr("controls.resetAll"),
                    Act::ResetKeys,
                ));
                out.push(button(cx + 4.0, h - 27.0, 150.0, tr("gui.done"), Act::Done));
            }
            Page::PadBinds => {
                out.push(Widget {
                    x: 0.0,
                    y: 12.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr(Self::title(page)),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                let row_left = cx - 170.0;
                let scrollbar_x = row_left + 340.0 + 6.0 + 2.0;
                let list_start = out.len();
                let mut y = top + 2.0 - self.scroll;
                let pad = &s.controller;
                for (action, _) in gamepad::ACTIONS {
                    out.push(Widget {
                        x: row_left + 2.0,
                        y: y + 6.0,
                        w: 200.0,
                        h: 9.0,
                        kind: WKind::Text {
                            text: tr(gamepad::action_caption(action)),
                            color: u32::MAX,
                            centered: false,
                        },
                        list: false,
                    });
                    let mut message = match pad.binding(action) {
                        Some(b) => {
                            let [r, g, bl] = b.color(self.pad_style);
                            Text::literal(b.label(self.pad_style))
                                .with_color(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(bl))
                        }
                        None => tr("key.keyboard.unknown"),
                    };
                    if pad.conflicts(action) {
                        message = Text::literal("[ ")
                            .append(message)
                            .append(" ]")
                            .with_color(text::YELLOW);
                    }
                    if self.selected_pad_action == Some(*action) {
                        message = Text::literal("> ")
                            .append(message.with_style(&Style {
                                underlined: Some(true),
                                ..Style::default()
                            }))
                            .append(" <")
                            .with_color(text::YELLOW);
                    }
                    let reset_x = scrollbar_x - 50.0 - 10.0;
                    let change_x = reset_x - 5.0 - 75.0;
                    out.push(button(change_x, y, 75.0, message, Act::PadKey(action)));
                    let reset = button(
                        reset_x,
                        y,
                        50.0,
                        tr("controls.reset"),
                        Act::ResetPadKey(action),
                    );
                    out.push(if pad.is_default(action) {
                        inactive(reset)
                    } else {
                        reset
                    });
                    y += 20.0;
                }
                content = y + self.scroll - top + 2.0;
                out[list_start..].iter_mut().for_each(|w| w.list = true);
                out.push(button(
                    cx - 154.0,
                    h - 27.0,
                    150.0,
                    tr("controls.resetAll"),
                    Act::ResetPadKeys,
                ));
                out.push(button(cx + 4.0, h - 27.0, 150.0, tr("gui.done"), Act::Done));
            }
            Page::Language => {
                top = 36.0;
                bottom = h - 53.0;
                out.push(Widget {
                    x: 0.0,
                    y: 4.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr(Self::title(page)),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                self.language_search.x = cx - 100.0;
                self.language_search.y = 17.0;
                let query = self.language_search.value.to_lowercase();
                let current = self
                    .language_choice
                    .clone()
                    .unwrap_or_else(|| s.language.clone());
                if std::mem::take(&mut self.center_language)
                    && let Some(i) = self.languages.iter().position(|l| l.code == current)
                {
                    // AbstractSelectionList.centerScrollOn.
                    self.scroll = (i as f32 * 18.0 + 9.0 - (bottom - top) / 2.0).max(0.0);
                }
                let list_start = out.len();
                let mut y = top + 2.0 - self.scroll;
                for l in &self.languages {
                    if !query.is_empty()
                        && !l.label().to_lowercase().contains(&query)
                        && !l.code.contains(&query)
                    {
                        continue;
                    }
                    out.push(Widget {
                        x: cx - 110.0,
                        y,
                        w: 220.0,
                        h: 18.0,
                        kind: WKind::LanguageRow {
                            label: Text::literal(l.label()),
                            selected: l.code == current,
                            code: l.code.clone(),
                        },
                        list: false,
                    });
                    y += 18.0;
                }
                content = y + self.scroll - top + 2.0;
                out[list_start..].iter_mut().for_each(|w| w.list = true);
                out.push(Widget {
                    x: 0.0,
                    y: h - 45.0,
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr("options.languageAccuracyWarning"),
                        color: rgba(0xBA, 0xBA, 0xBA, 255),
                        centered: true,
                    },
                    list: false,
                });
                out.push(button(
                    cx - 154.0,
                    h - 28.0,
                    150.0,
                    tr("options.font"),
                    Act::Push(Page::Font),
                ));
                out.push(button(cx + 4.0, h - 28.0, 150.0, tr("gui.done"), Act::Done));
            }
            // Drawn by `pack_screen`.
            Page::Packs => {}
            _ => {
                if matches!(page, Page::Video | Page::Controls) {
                    top = 38.0;
                }
                out.push(Widget {
                    x: 0.0,
                    y: if matches!(page, Page::Video | Page::Controls) {
                        4.0
                    } else {
                        12.0
                    },
                    w,
                    h: 9.0,
                    kind: WKind::Text {
                        text: tr(Self::title(page)),
                        color: u32::MAX,
                        centered: true,
                    },
                    list: false,
                });
                let list_start = out.len();
                let mut y = top + 2.0 - self.scroll;
                for row in self.list_rows(s) {
                    match row {
                        Row::Header(key) => {
                            let padding = if y + self.scroll <= top + 2.0 {
                                0.0
                            } else {
                                18.0
                            };
                            let label = tr(key).with_style(&Style {
                                bold: Some(true),
                                underlined: Some(true),
                                ..Style::default()
                            });
                            out.push(Widget {
                                x: 0.0,
                                y: y + 2.0 + padding + 4.0,
                                w,
                                h: 9.0,
                                kind: WKind::Text {
                                    text: label,
                                    color: u32::MAX,
                                    centered: true,
                                },
                                list: false,
                            });
                            y += padding + 18.0;
                        }
                        Row::Note(text) => {
                            out.push(Widget {
                                x: 0.0,
                                y: y + 4.0,
                                w,
                                h: 9.0,
                                kind: WKind::Text {
                                    text: Text::literal(text),
                                    color: 0xFFA0A0A0,
                                    centered: true,
                                },
                                list: false,
                            });
                            y += 16.0;
                        }
                        Row::Big(opt) => {
                            out.push(option_widget(s, opt, cx - 155.0, y + 2.0, 310.0));
                            y += 25.0;
                        }
                        Row::Pair(a, b) => {
                            out.push(option_widget(s, a, cx - 155.0, y + 2.0, 150.0));
                            if let Some(b) = b {
                                out.push(option_widget(s, b, cx + 5.0, y + 2.0, 150.0));
                            }
                            y += 25.0;
                        }
                        Row::Buttons(mut a, b) => {
                            a.x = cx - 155.0;
                            a.y = y + 2.0;
                            let after = a.x + a.w + 10.0;
                            out.push(a);
                            if let Some(mut b) = b {
                                b.x = after;
                                b.y = y + 2.0;
                                out.push(b);
                            }
                            y += 25.0;
                        }
                        Row::About(text) => {
                            out.push(Widget {
                                x: cx - 153.0,
                                y,
                                w: 306.0,
                                h: 9.0,
                                kind: WKind::Text {
                                    text: Text::literal(text),
                                    color: 0xFFA0_A0A0,
                                    centered: false,
                                },
                                list: false,
                            });
                            y += 13.0;
                        }
                        Row::Field => {
                            out.push(Widget {
                                x: cx - 155.0,
                                y: y + 8.0,
                                w: 40.0,
                                h: 9.0,
                                kind: WKind::Text {
                                    text: Text::literal("Name"),
                                    color: 0xFFA0_A0A0,
                                    centered: false,
                                },
                                list: false,
                            });
                            // Drawn and edited by the page itself (`skin.name`).
                            let field = &mut self.skin.name;
                            (field.x, field.y) = (cx - 115.0, y + 2.0);
                            (field.width, field.height) = (270.0, 20.0);
                            y += 25.0;
                        }
                        Row::SendField => {
                            // Drawn and edited by the page itself.
                            let field = &mut self.custom_send;
                            (field.x, field.y) = (cx - 155.0, y + 2.0);
                            (field.width, field.height) = (310.0, 20.0);
                            y += 25.0;
                        }
                        Row::Warning(text) => {
                            out.push(Widget {
                                x: 0.0,
                                y: y + 4.0,
                                w,
                                h: 9.0,
                                kind: WKind::Text {
                                    text: Text::literal(text),
                                    color: rgba(255, 85, 85, 255),
                                    centered: true,
                                },
                                list: false,
                            });
                            y += 16.0;
                        }
                    }
                }
                content = y + self.scroll - top + 2.0;
                out[list_start..].iter_mut().for_each(|w| w.list = true);
                let folder = match page {
                    Page::Mods => Some(("Open Mod Folder", Act::OpenModFolder)),
                    Page::ShaderPacks => Some(("Open Pack Folder", Act::OpenShaderPackFolder)),
                    Page::ShaderOptions => Some(("Reset", Act::ShaderReset)),
                    Page::Skin if self.skin_page => Some(("Open Skin Folder", Act::OpenSkinFolder)),
                    _ => None,
                };
                if let Some((label, act)) = folder {
                    out.push(button(
                        cx - 154.0,
                        h - 27.0,
                        150.0,
                        Text::literal(label),
                        act,
                    ));
                    out.push(button(cx + 4.0, h - 27.0, 150.0, tr("gui.done"), Act::Done));
                } else {
                    out.push(done(h - 27.0));
                }
            }
        }
        // Layout may change the offset (language centering), or the list may
        // shrink after a resize/search. Correct the rows in this same frame.
        let previous_scroll = self.scroll;
        self.clamp_scroll(top, bottom, content);
        for widget in &mut out {
            if widget.list {
                widget.y += previous_scroll - self.scroll;
            }
        }
        if page == Page::Skin && self.skin_page {
            self.skin.name.y += previous_scroll - self.scroll;
        }
        (out, top, bottom, content)
    }
    fn clamp_scroll(&mut self, top: f32, bottom: f32, content: f32) {
        let max = (content - (bottom - top)).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max);
    }
    pub fn draw(
        &mut self,
        s: &Settings,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
    ) {
        let (w, h) = (batch.width, batch.height);
        if let Some(error) = &self.resource_error {
            ui::menu_background(batch, atlas, 0.0, 0.0, w, h, self.in_world);
            let title = tr("resourcePack.load_fail");
            batch.text_styled(
                atlas,
                &title,
                ((w - atlas.font.width_text(&title)) / 2.0).floor(),
                30.0,
                u32::MAX,
                true,
            );
            let lines = atlas.font.split(&Text::literal(error), (w - 50.0).max(1.0));
            let limit = ((h - 110.0) / 9.0).max(1.0) as usize;
            for (i, line) in lines.iter().take(limit).enumerate() {
                batch.text_styled(
                    atlas,
                    line,
                    ((w - atlas.font.width_text(line)) / 2.0).floor(),
                    60.0 + i as f32 * 9.0,
                    u32::MAX,
                    true,
                );
            }
            ui::button(
                batch,
                atlas,
                w / 2.0 - 100.0,
                h - 28.0,
                200.0,
                20.0,
                &tr("gui.done"),
                true,
                ui::hovered(cursor, w / 2.0 - 100.0, h - 28.0, 200.0, 20.0),
            );
            return;
        }
        if let Some(level) = Self::graphics_level(self.page()) {
            self.graphics
                .draw(level, s, batch, atlas, cursor, self.in_world);
            return;
        }
        if self.page() == Page::Packs {
            ui::menu_background(batch, atlas, 0.0, 0.0, w, h, self.in_world);
            let blink = self.pack_focus.elapsed().as_millis() as u64;
            let m = crate::screen_draw::Measure(&atlas.font);
            let mouse = (cursor.0.floor() as i32, cursor.1.floor() as i32);
            let packs = self.packs(atlas, (w, h));
            let mut draws = packs.draw(&m, mouse, blink);
            packs.bind_icons(&mut draws, atlas, &mut batch.patches);
            crate::screen_draw::paint(batch, atlas, &draws);
            return;
        }
        let (widgets, top, bottom, content) = self.layout(s, w, h);
        let in_world = self.in_world;
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, in_world);
        let list = bottom > top;
        if list {
            ui::list_background(batch, atlas, top, bottom, in_world);
        }
        let visible = |wd: &Widget| !wd.list || (wd.y < bottom && wd.y + wd.h > top);
        let mut tooltip = None;
        for wd in &widgets {
            if !visible(wd) {
                continue;
            }
            let start = batch.vertices.len();
            let hover = ui::hovered(cursor, wd.x, wd.y, wd.w, wd.h)
                && (!wd.list || cursor.1 >= top && cursor.1 < bottom);
            if hover {
                if let WKind::Button {
                    act: Act::Extra(item),
                    ..
                } = &wd.kind
                {
                    tooltip = Some(atlas.font.split(&Text::literal(item.description()), 170.0));
                }
                if let WKind::Button {
                    act: Act::Push(page),
                    ..
                } = &wd.kind
                    && let Some(text) = horizon_page_tooltip(*page)
                {
                    tooltip = Some(atlas.font.split(&Text::literal(text), 170.0));
                }
                if let Some(options) = &s.shader_options {
                    let comment = match &wd.kind {
                        WKind::Button {
                            act: Act::ShaderOption(name),
                            ..
                        }
                        | WKind::ShaderSlider { name, .. } => {
                            options.comment(&format!("option.{name}"))
                        }
                        WKind::Button {
                            act: Act::ShaderScreen(screen),
                            ..
                        } => options.comment(&format!("screen.{screen}")),
                        WKind::Button {
                            act: Act::ShaderOptionsProfile,
                            ..
                        } => options.comment("profile"),
                        _ => None,
                    };
                    if let Some(comment) = comment {
                        tooltip = Some(atlas.font.split(&comment, 200.0));
                    }
                }
                let key = match &wd.kind {
                    WKind::Button {
                        act: Act::Cycle(opt),
                        ..
                    }
                    | WKind::Slider { opt, .. } => Some(opt.tooltip_key(s)),
                    WKind::Button {
                        act: Act::Push(Page::Language),
                        ..
                    } => Some("options.language.tooltip".into()),
                    WKind::Button {
                        act: Act::Push(Page::Accessibility),
                        ..
                    } => Some("options.accessibility.tooltip".into()),
                    _ => None,
                };
                if let Some(key) = key.filter(|key| lang::has(key)) {
                    tooltip = Some(atlas.font.split(&tr(&key), 170.0));
                }
                if let WKind::Button {
                    act: Act::Cycle(opt),
                    ..
                } = &wd.kind
                    && !opt.available()
                {
                    tooltip = Some(atlas.font.split(
                        &Text::literal("This feature is not yet available in Horizon."),
                        170.0,
                    ));
                }
                if let WKind::Button {
                    act: Act::ToggleMod(i),
                    ..
                } = &wd.kind
                    && let Some(installed) = s.mods.installed.get(*i)
                {
                    let description = installed
                        .problem
                        .as_deref()
                        .unwrap_or(&installed.description);
                    tooltip = Some(atlas.font.split(
                        &Text::literal(format!("{}\n{description}", installed.title)),
                        170.0,
                    ));
                }
            }
            match &wd.kind {
                WKind::Button { label, active, act } => {
                    ui::button(batch, atlas, wd.x, wd.y, wd.w, wd.h, label, *active, hover);
                    if hover && let Act::Key(name) = act {
                        let conflicts = s.keys.conflicts(name);
                        if !conflicts.is_empty() {
                            let names = conflicts
                                .iter()
                                .map(|c| lang::get(c))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let full = Text::translate(
                                "controls.keybinds.duplicateKeybinds",
                                &[Text::literal(names)],
                            );
                            tooltip = Some(
                                full.plain()
                                    .split('\n')
                                    .map(|l| Text::literal(l.to_owned()))
                                    .collect::<Vec<_>>(),
                            );
                        }
                    }
                }
                WKind::ShaderSlider { label, value, name } => {
                    let held = self.shader_dragging.as_ref().filter(|(n, _)| n == name);
                    let dragging = held.is_some();
                    ui::slider(
                        batch,
                        atlas,
                        wd.x,
                        wd.y,
                        wd.w,
                        label,
                        held.map_or(*value, |h| h.1),
                        hover || dragging,
                        true,
                    );
                }
                WKind::Slider { label, value, opt } => {
                    let dragging = self.dragging == Some(*opt);
                    let shown = self.slider_shown(*opt, *value);
                    ui::slider(
                        batch,
                        atlas,
                        wd.x,
                        wd.y,
                        wd.w,
                        label,
                        shown,
                        hover || dragging,
                        true,
                    );
                }
                WKind::Text {
                    text,
                    color,
                    centered,
                } => {
                    if text.is_empty() {
                        batch.fill(atlas, wd.x, wd.y, wd.w, wd.h, *color);
                    } else if *centered {
                        let tw = atlas.font.width_text(text);
                        batch.text_styled(
                            atlas,
                            text,
                            wd.x + ((wd.w - tw) / 2.0).floor(),
                            wd.y,
                            *color,
                            true,
                        );
                    } else {
                        batch.text_styled(atlas, text, wd.x, wd.y, *color, true);
                    }
                }
                WKind::LanguageRow {
                    label, selected, ..
                } => {
                    let rect = [wd.x, wd.y, wd.w, wd.h];
                    crate::widgets::record_as(
                        &label.plain(),
                        rect,
                        true,
                        crate::widgets::Kind::Row,
                    );
                    crate::widgets::target_as(rect, crate::widgets::Kind::Row);
                    if *selected {
                        batch.fill(atlas, wd.x, wd.y, wd.w, wd.h, u32::MAX);
                        batch.fill(
                            atlas,
                            wd.x + 1.0,
                            wd.y + 1.0,
                            wd.w - 2.0,
                            wd.h - 2.0,
                            rgba(0, 0, 0, 255),
                        );
                    }
                    let tw = atlas.font.width_text(label);
                    batch.text_styled(
                        atlas,
                        label,
                        wd.x + ((wd.w - tw) / 2.0).floor(),
                        wd.y + 2.0 + 7.0 - 4.0,
                        u32::MAX,
                        true,
                    );
                }
            }
            if wd.list && (wd.y < top || wd.y + wd.h > bottom) {
                batch.clip_since(start, 0.0, top, w, bottom);
            }
        }
        if list {
            ui::separators(batch, atlas, top, bottom, in_world);
            let sx = self.scrollbar_x(w);
            ui::scrollbar(batch, atlas, sx, top, bottom, content, self.scroll);
        }
        if self.page() == Page::Language {
            let mut search = self.language_search.clone();
            search.draw(batch, atlas, None);
            if search.value.is_empty() && !search.focused {
                batch.text_styled(
                    atlas,
                    &tr("gui.language.search").with_color(text::GRAY).italic(),
                    search.text_x(),
                    search.text_y(),
                    u32::MAX,
                    true,
                );
            }
            self.language_search = search;
        }
        if matches!(self.page(), Page::Video | Page::Controls | Page::KeyBinds) {
            let mut search = self.settings_search.clone();
            search.draw(batch, atlas, None);
            if search.value.is_empty() && !search.focused {
                batch.text_styled(
                    atlas,
                    &Text::literal("Search settings...")
                        .with_color(text::GRAY)
                        .italic(),
                    search.text_x(),
                    search.text_y(),
                    u32::MAX,
                    true,
                );
            }
            self.settings_search = search;
        }
        if self.page() == Page::Skin && self.skin_page {
            let mut name = self.skin.name.clone();
            if name.y >= top && name.y + name.height <= bottom {
                // Red while what is typed is no name, which leaving the box refuses.
                name.text_color = if crate::offline_name::is_valid_choice(&name.value) {
                    rgba(224, 224, 224, 255)
                } else {
                    rgba(255, 85, 85, 255)
                };
                name.draw(batch, atlas, None);
            }
            self.skin.name = name;
        }
        if matches!(self.page(), Page::CustomButton(_)) {
            let mut field = self.custom_send.clone();
            if field.y >= top && field.y + field.height <= bottom {
                field.draw(batch, atlas, None);
            }
            self.custom_send = field;
        }
        if let Some(lines) = tooltip {
            ui::tooltip(batch, atlas, &lines, cursor);
        }
    }
    /// Mouse press/drag. Returns events for the game.
    pub fn click(
        &mut self,
        s: &mut Settings,
        atlas: &HudAtlas,
        x: f32,
        y: f32,
        size: (f32, f32),
        framebuffer: [i32; 2],
        drag: bool,
        right: bool,
    ) -> Vec<Event> {
        if self.resource_error.is_some() {
            if !drag
                && !right
                && ui::hovered((x, y), size.0 / 2.0 - 100.0, size.1 - 28.0, 200.0, 20.0)
            {
                self.resource_error = None;
                return vec![Event::ButtonClick];
            }
            return Vec::new();
        }
        if let Some(level) = Self::graphics_level(self.page()) {
            use crate::graphics_ui::Action;
            let mut events = Vec::new();
            for action in self
                .graphics
                .click(level, s, (x, y), size, framebuffer, drag, right)
            {
                match action {
                    Action::ButtonClick => events.push(Event::ButtonClick),
                    Action::Changed => events.push(Event::Changed),
                    Action::OpenAdvanced => self.push(Page::GraphicsAdvanced),
                    Action::OpenSimple => {
                        if self.stack.len() >= 2
                            && self.stack[self.stack.len() - 2] == Page::Graphics
                        {
                            events.extend(self.back(s));
                        } else {
                            self.stack.pop();
                            self.push(Page::Graphics);
                        }
                    }
                    Action::Done => events.extend(self.back(s)),
                }
            }
            return events;
        }
        if self.page() == Page::Packs {
            let mut events = Vec::new();
            if drag {
                self.packs(atlas, size).drag(y as f64);
                return events;
            }
            if right {
                return events;
            }
            let m = crate::screen_draw::Measure(&atlas.font);
            let p = self.packs(atlas, size);
            if p.button_at(&m, x as f64, y as f64) {
                events.push(Event::ButtonClick);
            }
            let before = p.search_focused();
            let action = p.click(&m, x as f64, y as f64);
            if !before && p.search_focused() {
                self.pack_focus = Instant::now();
            }
            match action {
                crate::pack_screen::Action::OpenFolder => events.push(Event::OpenPackFolder),
                crate::pack_screen::Action::Done => events.extend(self.back(s)),
                crate::pack_screen::Action::None => {}
            }
            return events;
        }
        let (widgets, top, bottom, content) = self.layout(s, size.0, size.1);
        let mut events = Vec::new();
        if !drag && matches!(self.page(), Page::Video | Page::Controls | Page::KeyBinds) {
            let search = &mut self.settings_search;
            search.focused = ui::hovered((x, y), search.x, search.y, search.width, search.height);
            if search.focused {
                search.click(atlas, x, false, false);
                self.scroll = 0.0;
                events.push(Event::ButtonClick);
                return events;
            }
        }
        if !drag && !right && self.page() == Page::Skin && self.skin_page {
            let field = &mut self.skin.name;
            if ui::hovered((x, y), field.x, field.y, field.width, field.height)
                && y >= top
                && y < bottom
            {
                if !field.focused {
                    field.reset_blink();
                }
                field.focused = true;
                field.click(atlas, x, false, false);
                return events;
            }
            // A press anywhere else leaves the box.
            if field.focused {
                self.commit_name();
            }
        }
        if !drag && !right && matches!(self.page(), Page::CustomButton(_)) {
            let field = &mut self.custom_send;
            if ui::hovered((x, y), field.x, field.y, field.width, field.height)
                && y >= top
                && y < bottom
            {
                if !field.focused {
                    field.reset_blink();
                }
                field.focused = true;
                field.click(atlas, x, false, false);
                return events;
            }
            field.focused = false;
        }
        if drag {
            if let Some(previous_y) = self.scrollbar_drag_y {
                if let Some((_, thumb)) = ui::scrollbar_thumb(top, bottom, content, self.scroll) {
                    let max = content - (bottom - top);
                    self.scroll = if y < top {
                        0.0
                    } else if y > bottom {
                        max
                    } else {
                        self.scroll + (y - previous_y) * max / (bottom - top - thumb)
                    };
                    self.clamp_scroll(top, bottom, content);
                    self.scrollbar_drag_y = Some(y);
                } else {
                    self.scrollbar_drag_y = None;
                }
                return events;
            }
            if let Some((name, at)) = &mut self.shader_dragging
                && let Some(wd) = widgets
                    .iter()
                    .find(|wd| matches!(&wd.kind, WKind::ShaderSlider { name: n, .. } if n == name))
                && let Some(options) = &mut s.shader_options
            {
                *at = ui::slider_value(wd.x, wd.w, x);
                options.set_position(name, *at);
            }
            if let Some(opt) = self.dragging
                && let Some(wd) = widgets
                    .iter()
                    .find(|wd| matches!(&wd.kind, WKind::Slider { opt: o, .. } if *o == opt))
            {
                let v = ui::slider_value(wd.x, wd.w, x);
                self.slider_value = Some((opt, v, Instant::now()));
                set_slider(opt, s, v);
                events.push(Event::Changed);
            }
            return events;
        }
        // Controller Binds: a click cancels the wait.
        if self.selected_pad_action.take().is_some() {
            return events;
        }
        // Key Binds: a mouse button pressed while waiting binds it.
        if self.selected_key.is_some() {
            let button = if right {
                winit::event::MouseButton::Right
            } else {
                winit::event::MouseButton::Left
            };
            return self.bind_mouse(s, button);
        }
        // AbstractWidget accepts only the primary button. Rebinding above
        // accepts other mouse buttons, but is not a widget activation.
        if right {
            return events;
        }
        if ui::scrollbar_thumb(top, bottom, content, self.scroll).is_some()
            && ui::hovered((x, y), self.scrollbar_x(size.0), top, 6.0, bottom - top)
        {
            self.scrollbar_drag_y = Some(y);
            return events;
        }
        for wd in widgets {
            if !ui::hovered((x, y), wd.x, wd.y, wd.w, wd.h) {
                continue;
            }
            if wd.list && (y < top || y >= bottom) {
                continue;
            }
            match wd.kind {
                WKind::Button {
                    active: true, act, ..
                } => {
                    events.push(Event::ButtonClick);
                    match act {
                        Act::Push(p) => {
                            if p == Page::ShaderPacks {
                                events.push(Event::ListShaderPacks);
                                self.refresh_shader_profiles(s);
                            }
                            self.push(p)
                        }
                        Act::PickShaderPack(i) => {
                            let pick = match i {
                                Some(i) => s.shader_packs.get(i).map(|p| p.file.clone()),
                                None => Some(String::new()),
                            };
                            if let Some(pick) = pick {
                                s.horizon.shader_pack = if s.horizon.shader_pack == pick {
                                    String::new()
                                } else {
                                    pick
                                };
                                s.horizon.shader_profile.clear();
                                self.refresh_shader_profiles(s);
                                events.push(Event::ShaderPack);
                            }
                        }
                        Act::ShaderProfile => {
                            let now = s.horizon.shader_choice().map(|c| c.1).unwrap_or_default();
                            let list = &self.shader_profiles;
                            let at = list.iter().position(|p| *p == now).unwrap_or(0);
                            if let Some(next) = list.get((at + 1) % list.len().max(1)) {
                                // Empty would fall back to the variable's: the pack's own
                                // defaults are `default`, and empty is what means Auto.
                                s.horizon.shader_profile = match next.as_str() {
                                    "" => "default".into(),
                                    crate::graphics::AUTO_PROFILE => String::new(),
                                    next => next.to_owned(),
                                };
                            }
                            events.push(Event::ShaderPack);
                        }
                        Act::Done => events.extend(self.back(s)),
                        Act::Cycle(opt) => {
                            opt.cycle(s, right, framebuffer);
                            if opt == Opt::Preset {
                                apply_preset(s);
                            }
                            if opt == Opt::HighContrast {
                                // Options.highContrast: the built-in pack follows the option.
                                let on = s.resource_packs.iter().any(|id| id == "high_contrast");
                                if on != s.high_contrast {
                                    s.resource_packs.retain(|id| id != "high_contrast");
                                    if s.high_contrast {
                                        s.resource_packs.push("high_contrast".into());
                                    }
                                    // PackSelectionScreen reads the repository each time it
                                    // opens: the next one starts from this selection.
                                    self.pack_selection = s.resource_packs.clone();
                                    self.pack_screen = None;
                                    events.push(Event::Packs(s.resource_packs.clone()));
                                }
                            }
                            events.push(Event::Changed);
                        }
                        Act::Extra(item) => {
                            if let Some(name) = item.key_name() {
                                self.selected_key = Some(name);
                            } else {
                                item.cycle(&mut s.extras, right);
                                events.push(Event::Changed);
                            }
                        }
                        Act::Key(name) => self.selected_key = Some(name),
                        Act::ResetKey(name) => {
                            s.keys.set(name, KeyBinds::default_of(name));
                            events.push(Event::Changed);
                        }
                        Act::ResetKeys => {
                            s.keys.reset_all();
                            events.push(Event::Changed);
                        }
                        Act::PadKey(action) => self.selected_pad_action = Some(action),
                        Act::ResetPadKey(action) => {
                            s.controller
                                .bind(action, gamepad::PadSettings::default_of(action));
                            events.push(Event::Changed);
                        }
                        Act::ResetPadKeys => {
                            s.controller.reset_bindings();
                            events.push(Event::Changed);
                        }
                        Act::ToggleMod(i) => {
                            s.mods.toggle(i);
                            events.push(Event::Changed);
                        }
                        Act::ModConfig(i) => events.push(Event::ModConfig(i)),
                        Act::ModSettings(i) => {
                            events.push(Event::ButtonClick);
                            if s.mods.installed.get(i).is_some_and(|m| m.native.is_some()) {
                                self.push(Page::NativeMod(i));
                            } else {
                                events.push(Event::ModSettings(i));
                            }
                        }
                        Act::NativeKey(i, k) => {
                            self.selected_key = Some(NATIVE_KEY);
                            self.selected_native_key = Some((i, k));
                        }
                        Act::NativeButton(i, b) => events.push(Event::NativeButton(i, b)),
                        Act::OpenModFolder => events.push(Event::OpenModFolder),
                        Act::OpenShaderPackFolder => events.push(Event::OpenShaderPackFolder),
                        Act::ShaderResolution => {
                            const STEPS: [u32; 6] = [100, 75, 50, 40, 30, 25];
                            let now = s.horizon.resolution_scale;
                            let next = STEPS.iter().position(|&v| v == now).map_or(2, |i| i + 1);
                            s.horizon.resolution_scale = STEPS[next % STEPS.len()];
                            events.push(Event::ShaderPack);
                            events.push(Event::Changed);
                        }
                        Act::ShaderFpsLimit => {
                            const STEPS: [u32; 4] = [30, 40, 60, 260];
                            let next = STEPS
                                .iter()
                                .position(|&v| v == s.max_fps)
                                .map_or(0, |i| i + 1);
                            s.max_fps = STEPS[next % STEPS.len()];
                            events.push(Event::Changed);
                        }
                        Act::CustomButtons => {
                            self.push(Page::CustomButtons);
                            events.push(Event::OpenCustomButtons);
                        }
                        Act::CustomAdd => {
                            let id = s.custom_buttons.iter().map(|b| b.id + 1).max().unwrap_or(1);
                            s.custom_buttons.push(crate::settings::CustomButton {
                                id,
                                action: crate::settings::CustomAction::Send(String::new()),
                            });
                            self.custom_send.set_value("");
                            self.custom_send.focused = false;
                            self.push(Page::CustomButton(id));
                        }
                        Act::CustomEdit(id) => {
                            let line =
                                s.custom_buttons.iter().find(|b| b.id == id).map(|b| {
                                    match &b.action {
                                        crate::settings::CustomAction::Send(line) => line.as_str(),
                                        crate::settings::CustomAction::Key(_) => "",
                                    }
                                });
                            self.custom_send.set_value(line.unwrap_or_default());
                            self.custom_send.focused = false;
                            self.push(Page::CustomButton(id));
                        }
                        Act::CustomDelete(id) => {
                            s.custom_buttons.retain(|b| b.id != id);
                            events.push(Event::CustomButtons);
                        }
                        Act::CustomKey(id, name) => {
                            if let Some(b) = s.custom_buttons.iter_mut().find(|b| b.id == id) {
                                b.action = crate::settings::CustomAction::Key(name.to_owned());
                            }
                            self.custom_send.set_value("");
                            self.custom_send.focused = false;
                        }
                        Act::ShaderOptions => {
                            s.shader_options = None;
                            self.push(Page::ShaderOptions);
                            events.push(Event::OpenShaderOptions);
                        }
                        Act::ShaderOption(name) => {
                            if let Some(options) = &mut s.shader_options {
                                options.cycle(&name, right);
                            }
                        }
                        Act::ShaderScreen(screen) => {
                            if let Some(options) = &mut s.shader_options {
                                options.path.push(screen);
                                self.scroll = 0.0;
                            }
                        }
                        Act::ShaderOptionsProfile => {
                            if let Some(options) = &mut s.shader_options {
                                options.next_profile();
                            }
                        }
                        Act::ShaderReset => {
                            if let Some(options) = &mut s.shader_options {
                                options.reset();
                            }
                        }
                        Act::TouchLayout => events.push(Event::TouchLayout),
                        Act::Credits => events.push(Event::Credits),
                        Act::CopyAgentDetails => events.push(Event::CopyAgentDetails),
                        Act::NextSkin => {
                            let page = &mut self.skin;
                            page.choice.file =
                                crate::own_skin::next(&page.files, page.choice.file.as_deref());
                            page.warning = None;
                            page.actions.push(SkinAction::Choice(page.choice.clone()));
                        }
                        Act::SkinModel => {
                            let page = &mut self.skin;
                            page.choice.slim = !page.choice.slim;
                            page.actions.push(SkinAction::Choice(page.choice.clone()));
                        }
                        Act::PickSkin => self.skin.actions.push(SkinAction::PickFile),
                        Act::ElyBySkins => {
                            let page = &mut self.skin;
                            page.elyby = !page.elyby;
                            page.actions.push(SkinAction::ElyBy(page.elyby));
                        }
                        Act::OpenSkinFolder => self.skin.actions.push(SkinAction::OpenFolder),
                        Act::None => {}
                    }
                }
                WKind::ShaderSlider { name, .. } => {
                    let at = ui::slider_value(wd.x, wd.w, x);
                    if let Some(options) = &mut s.shader_options {
                        options.set_position(&name, at);
                    }
                    self.shader_dragging = Some((name, at));
                }
                WKind::Slider { opt, .. } => {
                    self.dragging = Some(opt);
                    let v = ui::slider_value(wd.x, wd.w, x);
                    self.slider_value = Some((opt, v, Instant::now()));
                    set_slider(opt, s, v);
                    events.push(Event::Changed);
                }
                WKind::LanguageRow { code, .. } => {
                    if self.language_choice.as_deref() == Some(code.as_str()) {
                        events.extend(self.back(s));
                    } else {
                        self.language_choice = Some(code);
                    }
                }
                _ => {}
            }
            break;
        }
        events
    }
    /// Where a slider's handle sits: its own position while held (or until
    /// a delayed one applies), else the option's stepped value.
    fn slider_shown(&self, opt: Opt, value: f32) -> f32 {
        match self.slider_value {
            Some((o, v, at)) if o == opt && slider_unsnapped(o, self.dragging == Some(o), at) => v,
            _ => value,
        }
    }
    pub fn release(&mut self) -> Vec<Event> {
        self.scrollbar_drag_y = None;
        if let Some(p) = &mut self.pack_screen {
            p.release();
        }
        // AbstractSliderButton suppresses the press sound and plays once on
        // release, even if the pointer has left the widget or its value stayed.
        let slider = self.dragging.take().is_some() | self.shader_dragging.take().is_some();
        let graphics_slider = self.graphics.release();
        if slider || graphics_slider {
            vec![Event::ButtonClick]
        } else {
            Vec::new()
        }
    }
    pub fn scroll(&mut self, s: &Settings, size: (f32, f32), amount: f32) {
        let (_, top, bottom, content) = self.layout(s, size.0, size.1);
        self.scroll -= amount * 18.0;
        self.clamp_scroll(top, bottom, content);
    }
    /// The mouse wheel at the cursor.
    pub fn scroll_at(&mut self, s: &Settings, size: (f32, f32), cursor: (f32, f32), amount: f32) {
        if self.resource_error.is_some() {
            return;
        }
        if let Some(level) = Self::graphics_level(self.page()) {
            self.graphics.scroll(level, size.1, amount);
            return;
        }
        match &mut self.pack_screen {
            Some(p) if self.stack.last() == Some(&Page::Packs) => {
                p.scroll(cursor.0 as f64, cursor.1 as f64, amount as f64)
            }
            _ => self.scroll(s, size, amount),
        }
    }
    /// The pack screen at this GUI size, made on first use.
    fn packs(&mut self, atlas: &HudAtlas, size: (f32, f32)) -> &mut crate::pack_screen::PackScreen {
        let m = crate::screen_draw::Measure(&atlas.font);
        let (w, h) = (size.0 as i32, size.1 as i32);
        match &mut self.pack_screen {
            Some(p) => {
                if p.width != w || p.height != h {
                    p.resize(&m, w, h);
                }
            }
            None => {
                self.pack_focus = Instant::now();
                self.pack_screen = Some(crate::pack_screen::PackScreen::new(
                    &m,
                    w,
                    h,
                    self.in_world,
                    self.pack_infos.clone(),
                    &self.pack_selection,
                ));
            }
        }
        self.pack_screen.as_mut().expect("made above")
    }
    /// Escape or Done: leave the current page, applying page results.
    pub fn back(&mut self, s: &mut Settings) -> Vec<Event> {
        let mut events = Vec::new();
        // A pack's screen goes back to the one that opened it.
        if self.page() == Page::ShaderOptions
            && let Some(options) = &mut s.shader_options
            && options.path.pop().is_some()
        {
            self.scroll = 0.0;
            return events;
        }
        let page = self.stack.pop().unwrap_or(Page::Main);
        self.scroll = 0.0;
        self.scrollbar_drag_y = None;
        self.dragging = None;
        self.slider_value = None;
        self.selected_key = None;
        self.selected_pad_action = None;
        match page {
            Page::Language => {
                if let Some(code) = self.language_choice.take()
                    && code != s.language
                {
                    s.language = code.clone();
                    events.push(Event::Language(code));
                }
            }
            Page::Packs => {
                // PackSelectionScreen.onClose: PackSelectionModel.commit.
                if let Some(p) = self.pack_screen.take()
                    && p.changed
                {
                    let ids = p.commit();
                    s.resource_packs = ids.clone();
                    self.pack_selection = ids.clone();
                    events.push(Event::Packs(ids));
                }
            }
            Page::ShaderOptions => {
                if s.shader_options.as_ref().is_some_and(|o| o.changed) {
                    events.push(Event::ShaderOptions);
                } else {
                    s.shader_options = None;
                }
            }
            // Done or Escape with the name box focused keeps what it holds, as leaving it does.
            Page::Skin if self.skin.name.focused => self.commit_name(),
            // What the box holds is what it sends; one that does nothing goes.
            Page::CustomButton(id) => {
                let line = self.custom_send.value.trim().to_owned();
                self.custom_send.focused = false;
                if let Some(b) = s.custom_buttons.iter_mut().find(|b| b.id == id)
                    && (!line.is_empty()
                        || matches!(b.action, crate::settings::CustomAction::Send(_)))
                {
                    b.action = crate::settings::CustomAction::Send(line);
                }
                s.custom_buttons
                    .retain(|b| b.action != crate::settings::CustomAction::Send(String::new()));
                events.push(Event::CustomButtons);
            }
            _ => {}
        }
        events.push(Event::Changed);
        if self.stack.is_empty() {
            events.push(Event::Close);
        }
        events
    }
    /// Typed text (the language search box).
    pub fn type_text(&mut self, atlas: &HudAtlas, text: &str) {
        if self.resource_error.is_some() {
            return;
        }
        if self.page() == Page::Packs
            && let Some(p) = &mut self.pack_screen
        {
            p.type_text(&crate::screen_draw::Measure(&atlas.font), text);
            return;
        }
        if self.page() == Page::Language {
            self.language_search.insert_text(text);
            self.scroll = 0.0;
        } else if self.settings_search_active() {
            self.settings_search.insert_text(text);
            self.scroll = 0.0;
        } else if self.page() == Page::Skin && self.skin.name.focused {
            self.skin.name.insert_text(text);
        } else if matches!(self.page(), Page::CustomButton(_)) && self.custom_send.focused {
            self.custom_send.insert_text(text);
        }
    }
    /// Ctrl+A/C/V/X in the language or pack search box; false when neither
    /// is shown with the focus.
    pub fn shortcut(
        &mut self,
        atlas: &HudAtlas,
        shortcut: crate::clipboard::EditShortcut,
        clipboard: &mut dyn crate::clipboard::Clipboard,
    ) -> bool {
        if self.resource_error.is_some() || self.selected_key.is_some() {
            return false;
        }
        if self.page() == Page::Packs
            && let Some(p) = &mut self.pack_screen
        {
            let m = crate::screen_draw::Measure(&atlas.font);
            return p.shortcut(&m, shortcut, clipboard);
        }
        if self.page() == Page::Language {
            if self.language_search.shortcut(shortcut, clipboard) {
                self.scroll = 0.0;
            }
            return true;
        }
        if self.settings_search_active() {
            if self.settings_search.shortcut(shortcut, clipboard) {
                self.scroll = 0.0;
            }
            return true;
        }
        if self.page() == Page::Skin && self.skin.name.focused {
            self.skin.name.shortcut(shortcut, clipboard);
            return true;
        }
        if matches!(self.page(), Page::CustomButton(_)) && self.custom_send.focused {
            self.custom_send.shortcut(shortcut, clipboard);
            return true;
        }
        false
    }
    fn bind_key(&mut self, s: &mut Settings, value: String) -> Vec<Event> {
        let Some(name) = self.selected_key.take() else {
            return Vec::new();
        };
        if name == NATIVE_KEY {
            let (i, k) = self.selected_native_key.take().unwrap_or_default();
            let Some(m) = s.mods.installed.get(i) else {
                return Vec::new();
            };
            let Some([key, ..]) = m.native.as_ref().and_then(|p| p.keys.get(k)) else {
                return Vec::new();
            };
            let keys = s.mods.keys.entry(m.id.clone()).or_default();
            keys.insert(key.clone(), value);
            return vec![Event::Changed];
        }
        if !s.extras.set_key(name, value.clone()) {
            s.keys.set(name, &value);
        }
        vec![Event::Changed]
    }
    /// Mouse bindings use the same settings and persistence as keyboard bindings.
    pub fn bind_mouse(
        &mut self,
        s: &mut Settings,
        button: winit::event::MouseButton,
    ) -> Vec<Event> {
        self.bind_key(s, keybinds::mouse_name(button))
    }
    /// Keys while the options are open. Returns events; `handled` false lets
    /// Escape fall through.
    pub fn key(
        &mut self,
        s: &mut Settings,
        atlas: &HudAtlas,
        key: winit::keyboard::KeyCode,
        text: Option<&str>,
    ) -> Vec<Event> {
        use winit::keyboard::KeyCode as K;
        if self.resource_error.is_some() {
            if matches!(key, K::Escape | K::Enter | K::NumpadEnter) {
                self.resource_error = None;
            }
            return Vec::new();
        }
        if self.page() == Page::Packs
            && let Some(p) = &mut self.pack_screen
        {
            let m = crate::screen_draw::Measure(&atlas.font);
            match key {
                // ConfirmScreen.keyPressed: Escape answers no.
                K::Escape if p.confirming() => {
                    p.cancel_confirm(&m);
                    return Vec::new();
                }
                K::Backspace => {
                    p.erase(&m, 1);
                    return Vec::new();
                }
                _ => {}
            }
        }
        if let Some(action) = self.selected_pad_action.take() {
            // As on Key Binds, Escape unbinds; other keys cancel.
            if key == K::Escape {
                s.controller.bind(action, None);
            }
            return vec![Event::Changed];
        }
        if self.selected_key.is_some() {
            // KeyBindsScreen.keyPressed: Escape unbinds.
            let value = if key == K::Escape {
                "key.keyboard.unknown".to_owned()
            } else {
                keybinds::key_name(key)
                    .unwrap_or("key.keyboard.unknown")
                    .to_owned()
            };
            return self.bind_key(s, value);
        }
        if key == K::Escape {
            return self.back(s);
        }
        if self.page() == Page::Language {
            if !self.language_search.key(key, false, false)
                && let Some(t) = text
            {
                let typed: String = t.chars().filter(|c| !c.is_control()).collect();
                if !typed.is_empty() {
                    self.language_search.insert_text(&typed);
                }
            }
            self.scroll = 0.0;
        } else if self.settings_search_active() {
            if !self.settings_search.key(key, false, false)
                && let Some(t) = text
            {
                let typed: String = t.chars().filter(|c| !c.is_control()).collect();
                if !typed.is_empty() {
                    self.settings_search.insert_text(&typed);
                }
            }
            self.scroll = 0.0;
        } else if self.page() == Page::Skin && self.skin.name.focused {
            // Enter (the on-screen keyboard's Done) keeps the name.
            if matches!(key, K::Enter | K::NumpadEnter) {
                self.commit_name();
                return Vec::new();
            }
            let field = &mut self.skin.name;
            if !field.key(key, false, false)
                && let Some(t) = text
            {
                let typed: String = t.chars().filter(|c| !c.is_control()).collect();
                if !typed.is_empty() {
                    field.insert_text(&typed);
                }
            }
            field.reset_blink();
        } else if matches!(self.page(), Page::CustomButton(_)) && self.custom_send.focused {
            // Enter (the on-screen keyboard's Done) leaves the box; Done on the page keeps it.
            let field = &mut self.custom_send;
            if matches!(key, K::Enter | K::NumpadEnter) {
                field.focused = false;
                return Vec::new();
            }
            if !field.key(key, false, false)
                && let Some(t) = text
            {
                let typed: String = t.chars().filter(|c| !c.is_control()).collect();
                if !typed.is_empty() {
                    field.insert_text(&typed);
                }
            }
            field.reset_blink();
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_sliders_and_toggles_survive_options_round_trip() {
        let mut s = Settings::default();
        for opt in [
            Opt::FovEffects,
            Opt::DarknessPulsing,
            Opt::DamageTilt,
            Opt::PanoramaSpeed,
        ] {
            for position in [0.0, 0.5, 1.0] {
                opt.set_slider(&mut s, position);
                let loaded = Settings::parse(&s.serialize());
                assert_eq!(opt.slider_position(&loaded), Some(position));
                let expected = if matches!(opt, Opt::FovEffects | Opt::DarknessPulsing) {
                    f64::from(position).powi(2)
                } else {
                    f64::from(position)
                };
                assert_eq!(loaded.file.double(opt.file_key().unwrap()), expected);
            }
        }
        for opt in [
            Opt::TextBackground,
            Opt::HideLightningFlashes,
            Opt::HideSplashTexts,
        ] {
            let key = opt.file_key().unwrap();
            let before = s.file.bool(key);
            opt.cycle(&mut s, false, [854, 480]);
            assert_eq!(Settings::parse(&s.serialize()).file.bool(key), !before);
        }
        Opt::TextBackground.cycle(&mut s, false, [854, 480]);
        let chat_only = s.text_background(0.0);
        Opt::TextBackground.cycle(&mut s, false, [854, 480]);
        assert_ne!(s.text_background(0.0), chat_only);
    }

    #[test]
    fn mouse_settings_match_java_26_3_option_order() {
        let mut ui = OptionsUi::default();
        ui.show(Page::Mouse);
        let mut options = Vec::new();
        for row in ui.list_rows(&Settings::default()) {
            if let Row::Pair(a, b) = row {
                options.push(a);
                if let Some(b) = b {
                    options.push(b);
                }
            }
        }
        assert_eq!(
            options,
            vec![
                Opt::Sensitivity,
                Opt::WheelSensitivity,
                Opt::DiscreteScroll,
                Opt::InvertX,
                Opt::InvertY,
                Opt::AllowCursorChanges,
            ]
        );
    }

    #[test]
    fn mouse_settings_round_trip_with_java_defaults() {
        let mut settings = Settings::default();
        assert!(!settings.invert_x);
        assert!(settings.allow_cursor_changes);
        Opt::InvertX.cycle(&mut settings, false, [854, 480]);
        Opt::AllowCursorChanges.cycle(&mut settings, false, [854, 480]);
        let loaded = Settings::parse(&settings.serialize());
        assert!(loaded.invert_x);
        assert!(!loaded.allow_cursor_changes);
        assert!(!loaded.file.bool("allowCursorChanges"));
    }

    #[test]
    fn accessibility_has_all_options_and_disabled_placeholders() {
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Accessibility);
        let (widgets, _, _, _) = ui.layout(&s, 480.0, 1200.0);
        let options: Vec<_> = widgets
            .iter()
            .filter_map(|w| match w.kind {
                WKind::Button {
                    act: Act::Cycle(opt),
                    ..
                }
                | WKind::Slider { opt, .. } => Some(opt),
                _ => None,
            })
            .collect();
        assert_eq!(options, ACCESSIBILITY_OPTIONS);
        for &opt in ACCESSIBILITY_OPTIONS {
            let widget = widgets
                .iter()
                .find(|w| match w.kind {
                    WKind::Button {
                        act: Act::Cycle(o), ..
                    }
                    | WKind::Slider { opt: o, .. } => o == opt,
                    _ => false,
                })
                .expect("every accessibility option has a control");
            if !opt.available() {
                assert!(matches!(widget.kind, WKind::Button { active: false, .. }));
                let before = s.serialize();
                opt.cycle(&mut s, false, [854, 480]);
                opt.set_slider(&mut s, 0.0);
                assert_eq!(s.serialize(), before);
            }
        }
        assert!(widgets.iter().any(|w| matches!(
            w.kind,
            WKind::Button {
                act: Act::Push(Page::Controls),
                active: true,
                ..
            }
        )));
    }

    #[test]
    fn render_distance_slider_round_trips_through_extended_options_range() {
        let mut s = Settings::default();
        for (position, distance) in [(0.0, 2), (0.5, 33), (1.0, 64)] {
            set_slider(Opt::RenderDistance, &mut s, position);
            assert_eq!(s.render_distance, distance);
            let reloaded = Settings::parse(&s.serialize());
            assert_eq!(reloaded.render_distance, distance);
            assert_eq!(
                Opt::RenderDistance.slider_position(&reloaded),
                Some(position)
            );
        }
        let mut file = crate::options_file::OptionsFile::default();
        file.set_text("renderDistance", "64").unwrap();
        assert!(file.set_text("renderDistance", "65").is_err());
        assert!(file.set_text("renderDistance", "1").is_err());
        assert_eq!(file.int("renderDistance"), 64);
    }
    fn atlas() -> HudAtlas {
        HudAtlas::build(std::path::Path::new("/nonexistent/horizon-test-assets")).unwrap()
    }

    #[test]
    fn resource_reload_error_blocks_options_until_dismissed() {
        use winit::keyboard::KeyCode;
        let atlas = atlas();
        let mut settings = Settings::default();
        let mut ui = OptionsUi {
            resource_error: Some("broken pack".into()),
            ..Default::default()
        };
        let stack = ui.stack.clone();
        assert!(
            ui.click(
                &mut settings,
                &atlas,
                10.0,
                10.0,
                (480.0, 270.0),
                [960, 540],
                false,
                false
            )
            .is_empty()
        );
        assert!(ui.resource_error.is_some());
        assert!(
            ui.key(&mut settings, &atlas, KeyCode::KeyW, None)
                .is_empty()
        );
        assert!(ui.resource_error.is_some());
        ui.key(&mut settings, &atlas, KeyCode::Escape, None);
        assert!(ui.resource_error.is_none());
        assert_eq!(ui.stack, stack);
        ui.resource_error = Some("broken pack".into());
        assert_eq!(
            ui.click(
                &mut settings,
                &atlas,
                240.0,
                250.0,
                (480.0, 270.0),
                [960, 540],
                false,
                false
            ),
            vec![Event::ButtonClick]
        );
        assert!(ui.resource_error.is_none());
    }

    #[test]
    fn mods_page_shows_what_a_mod_is_and_opens_its_settings() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        let size = (480.0, 240.0);
        ui.show(Page::Mods);
        s.mods.installed.push(crate::mod_list::Installed {
            id: "keystrokes".into(),
            title: "Keystrokes 0.1.0".into(),
            description: "May draw on your screen.".into(),
            summary: "Keys lit while held.".into(),
            config: Some("keystrokes:settings".into()),
            problem: None,
            dev: false,
            has_settings: false,
            native: None,
        });
        let (widgets, ..) = ui.layout(&s, size.0, size.1);
        assert!(widgets.iter().any(|w| {
            matches!(&w.kind, WKind::Text { text, .. } if text.plain() == "Keys lit while held.")
        }));
        let config = widgets
            .iter()
            .find(|w| {
                matches!(
                    w.kind,
                    WKind::Button {
                        act: Act::ModConfig(0),
                        ..
                    }
                )
            })
            .expect("a Config button beside the mod");
        assert_eq!(
            ui.click(
                &mut s,
                &atlas,
                config.x + 1.0,
                config.y + 1.0,
                size,
                [480, 240],
                false,
                false,
            ),
            vec![Event::ButtonClick, Event::ModConfig(0)]
        );
    }

    #[test]
    fn classic_mods_page_toggles_mods_and_keeps_dev_and_invalid_entries_inactive() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        let size = (480.0, 240.0);
        assert!(ui.layout(&s, size.0, size.1).0.iter().any(|w| {
            matches!(
                w.kind,
                WKind::Button {
                    active: true,
                    act: Act::Push(Page::Mods),
                    ..
                }
            )
        }));
        ui.show(Page::Mods);
        let empty = ui.layout(&s, size.0, size.1).0;
        assert!(empty.iter().any(|w| {
            matches!(&w.kind, WKind::Text { text, .. } if text.plain().contains(".hmod"))
        }));
        let open_folder = empty
            .iter()
            .find(|w| {
                matches!(
                    &w.kind,
                    WKind::Button {
                        act: Act::OpenModFolder,
                        ..
                    }
                )
            })
            .expect("Mods page has an Open Mod Folder button");
        assert_eq!(
            ui.click(
                &mut s,
                &atlas,
                open_folder.x + 1.0,
                open_folder.y + 1.0,
                size,
                [480, 240],
                false,
                false,
            ),
            vec![Event::ButtonClick, Event::OpenModFolder]
        );
        for (id, dev) in [("normal", false), ("dev", true), ("", false)] {
            s.mods.installed.push(crate::mod_list::Installed {
                id: id.into(),
                title: id.into(),
                description: "May show HUD panels.".into(),
                summary: String::new(),
                config: None,
                problem: id.is_empty().then(|| "Invalid package".into()),
                dev,
                has_settings: false,
                native: None,
            });
        }
        let (widgets, ..) = ui.layout(&s, size.0, size.1);
        let rows: Vec<_> = widgets
            .iter()
            .filter(|w| {
                matches!(
                    w.kind,
                    WKind::Button {
                        act: Act::ToggleMod(_),
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            let events = ui.click(
                &mut s,
                &atlas,
                row.x + 1.0,
                row.y + 1.0,
                size,
                [480, 240],
                false,
                false,
            );
            if i == 0 {
                assert!(events.contains(&Event::Changed));
                assert!(events.contains(&Event::ButtonClick));
            } else {
                assert!(events.is_empty());
            }
        }
        assert!(s.mods.is_on(0));
        assert!(s.mods.is_on(1));
        assert!(!s.mods.is_on(2));
        assert_eq!(s.mods.serialize(), "mod.normal:true\n");
        let row = rows[0];
        ui.click(
            &mut s,
            &atlas,
            row.x + 1.0,
            row.y + 1.0,
            size,
            [480, 240],
            false,
            false,
        );
        assert!(!s.mods.is_on(0));
        assert!(!ui.back(&mut s).contains(&Event::Close));
        assert_eq!(ui.page(), Page::Main);
    }

    #[test]
    fn option_tooltip_keys_follow_value_dependent_descriptions() {
        let mut s = Settings::default();
        assert_eq!(
            Opt::InactivityFps.tooltip_key(&s),
            "options.inactivityFpsLimit.afk.tooltip"
        );
        s.inactivity_fps_limit = "minimized".into();
        assert_eq!(
            Opt::InactivityFps.tooltip_key(&s),
            "options.inactivityFpsLimit.minimized.tooltip"
        );
        for (value, name) in ["none", "byPlayer", "nearby"].iter().enumerate() {
            s.prioritize_chunk_updates = value as u8;
            assert_eq!(
                Opt::ChunkBuilder.tooltip_key(&s),
                format!("options.prioritizeChunkUpdates.{name}.tooltip")
            );
        }
        assert_eq!(
            Opt::MenuBlur.tooltip_key(&s),
            "options.accessibility.menu_background_blurriness.tooltip"
        );
    }

    #[test]
    #[ignore = "requires prepared language data in HORIZON_TEST_RESOURCES; run alone"]
    fn classic_hover_draws_the_selected_options_wrapped_tooltip() {
        let root = std::path::PathBuf::from(
            std::env::var_os("HORIZON_TEST_RESOURCES").expect("HORIZON_TEST_RESOURCES"),
        );
        lang::set(lang::Language::load(&[&root], "en_us"));
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Video);
        for mode in ["afk", "minimized"] {
            s.inactivity_fps_limit = mode.into();
            let (widgets, _, _, _) = ui.layout(&s, 480.0, 480.0);
            let option = widgets
                .iter()
                .find(|w| {
                    matches!(
                        w.kind,
                        WKind::Button {
                            act: Act::Cycle(Opt::InactivityFps),
                            ..
                        }
                    )
                })
                .unwrap();
            let cursor = (option.x + 1.0, option.y + 1.0);
            let mut actual = HudBatch::new([480, 480]);
            ui.draw(&s, &mut actual, &atlas, cursor);
            let key = format!("options.inactivityFpsLimit.{mode}.tooltip");
            assert!(lang::has(&key));
            let mut expected = HudBatch::new([480, 480]);
            ui::tooltip(
                &mut expected,
                &atlas,
                &atlas.font.split(&tr(&key), 170.0),
                cursor,
            );
            assert!(!expected.vertices.is_empty());
            let tail = &actual.vertices[actual.vertices.len() - expected.vertices.len()..];
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(tail),
                bytemuck::cast_slice::<_, u8>(&expected.vertices)
            );
        }
    }

    #[test]
    fn classic_video_search_keeps_only_headings_with_matching_options() {
        let mut ui = OptionsUi::default();
        ui.show(Page::Video);
        let settings = Settings::default();
        let headers = |rows: Vec<Row>| {
            rows.into_iter()
                .filter_map(|row| match row {
                    Row::Header(key) => Some(key),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let all_headers = vec![
            "options.video.display.header",
            "options.video.quality.header",
            "options.video.preferences.header",
        ];
        assert_eq!(headers(ui.list_rows(&settings)), all_headers);
        for (opt, header) in [
            (Opt::Fullscreen, all_headers[0]),
            (Opt::RenderDistance, all_headers[1]),
            (Opt::Preset, all_headers[1]),
            (Opt::Autosave, all_headers[2]),
        ] {
            ui.settings_search
                .set_value(&lang::get(opt.caption()).to_uppercase());
            let rows = ui.list_rows(&settings);
            assert_eq!(headers(ui.list_rows(&settings)), vec![header]);
            assert!(rows.iter().any(|row| match row {
                Row::Big(found) => *found == opt,
                Row::Pair(a, b) => *a == opt || *b == Some(opt),
                _ => false,
            }));
            assert!(rows.iter().any(|row| matches!(row, Row::Buttons(..))));
        }
        ui.settings_search.set_value("no matching video setting");
        let rows = ui.list_rows(&settings);
        assert!(headers(ui.list_rows(&settings)).is_empty());
        assert!(rows.iter().all(|row| matches!(row, Row::Buttons(..))));
        ui.settings_search.set_value("");
        assert_eq!(headers(ui.list_rows(&settings)), all_headers);
    }

    #[test]
    fn classic_search_emits_each_matching_heading_once_in_section_order() {
        let mut ui = OptionsUi::default();
        ui.settings_search
            .set_value(&lang::get(Opt::Fullscreen.caption()));
        let rows = ui.filter_option_rows(vec![
            Row::Header("empty"),
            Row::Header("display"),
            Row::Pair(Opt::Fullscreen, Some(Opt::Fullscreen)),
            Row::Big(Opt::Fullscreen),
            Row::Header("quality"),
            Row::Big(Opt::Fullscreen),
            Row::Header("trailing empty"),
        ]);
        assert!(matches!(
            rows.as_slice(),
            [
                Row::Header("display"),
                Row::Pair(Opt::Fullscreen, Some(Opt::Fullscreen)),
                Row::Big(Opt::Fullscreen),
                Row::Header("quality"),
                Row::Big(Opt::Fullscreen),
            ]
        ));
    }

    #[test]
    fn classic_video_gui_scale_cycles_on_press_without_dragging() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Video);
        let size = (480.0, 480.0);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let button = widgets
            .iter()
            .find(|wd| {
                matches!(
                    wd.kind,
                    WKind::Button {
                        active: true,
                        act: Act::Cycle(Opt::GuiScale),
                        ..
                    }
                )
            })
            .expect("GUI Scale must be a cycle button in classic Video Settings");
        let (x, y) = (button.x + 1.0, button.y + 1.0);
        assert!(Opt::GuiScale.slider_position(&s).is_none());
        assert!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], false, true)
                .is_empty()
        );
        assert_eq!(s.gui_scale, 0);
        for expected in [1, 2, 0] {
            assert_eq!(
                ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
                vec![Event::ButtonClick, Event::Changed]
            );
            assert_eq!(s.gui_scale, expected);
            assert!(
                ui.click(&mut s, &atlas, x + 100.0, y, size, [854, 480], true, false)
                    .is_empty()
            );
            assert_eq!(s.gui_scale, expected);
            assert!(ui.release().is_empty());
        }
    }

    #[test]
    fn extras_from_horizon_settings_edit_shared_settings_and_return() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        let size = (480.0, 480.0);
        let click = |ui: &mut OptionsUi, s: &mut Settings, act: Act| {
            let (widgets, _, _, _) = ui.layout(s, size.0, size.1);
            let widget = widgets
                .iter()
                .find(
                    |w| matches!(&w.kind, WKind::Button { act: a, active: true, .. } if *a == act),
                )
                .unwrap();
            ui.click(
                s,
                &atlas,
                widget.x + 1.0,
                widget.y + 1.0,
                size,
                [960, 960],
                false,
                false,
            )
        };
        for page in [Page::Horizon, Page::Extras] {
            assert_eq!(
                click(&mut ui, &mut s, Act::Push(page)),
                vec![Event::ButtonClick]
            );
            assert_eq!(ui.page(), page);
        }
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        assert_eq!(
            widgets
                .iter()
                .filter(|w| matches!(
                    w.kind,
                    WKind::Button {
                        act: Act::Extra(_),
                        ..
                    }
                ))
                .count(),
            crate::extras::rows().iter().filter(|r| r.is_ok()).count()
        );
        assert_eq!(
            click(&mut ui, &mut s, Act::Extra(crate::extras::Item::Sort)),
            vec![Event::ButtonClick, Event::Changed]
        );
        assert!(s.extras.sort);
        assert_eq!(
            click(&mut ui, &mut s, Act::Extra(crate::extras::Item::SortKey)),
            vec![Event::ButtonClick]
        );
        assert_eq!(ui.selected_key, Some(crate::extras::SORT_KEY));
        assert_eq!(
            ui.key(&mut s, &atlas, winit::keyboard::KeyCode::KeyC, None),
            vec![Event::Changed]
        );
        assert_eq!(s.extras.sort_key, "key.keyboard.c");
        assert_eq!(
            crate::extras::Extras::parse(&s.extras.serialize()),
            s.extras
        );
        // In a window too short for the page, the wheel scrolls to its end.
        let short = (480.0, 180.0);
        ui.scroll(&s, short, -1000.0);
        let (_, top, bottom, content) = ui.layout(&s, short.0, short.1);
        assert!(content > bottom - top);
        assert_eq!(ui.scroll, content - (bottom - top));
        assert_eq!(
            ui.key(&mut s, &atlas, winit::keyboard::KeyCode::Escape, None),
            vec![Event::Changed]
        );
        assert_eq!(ui.page(), Page::Horizon);
    }

    #[test]
    fn classic_graphics_presets_set_the_distances() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Video);
        let size = (480.0, 480.0);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let preset = widgets
            .iter()
            .find(|wd| {
                matches!(
                    wd.kind,
                    WKind::Slider {
                        opt: Opt::Preset,
                        ..
                    }
                )
            })
            .expect("the Graphics preset slider is in classic Video Settings");
        // Dragging the slider across its range (GraphicsPreset.apply on each change).
        let y = preset.y + 1.0;
        let mut seen = Vec::new();
        ui.click(
            &mut s,
            &atlas,
            preset.x + 1.0,
            y,
            size,
            [854, 480],
            false,
            false,
        );
        for step in 0..=20 {
            let x = preset.x + preset.w * step as f32 / 20.0;
            ui.click(&mut s, &atlas, x, y, size, [854, 480], true, false);
            seen.push((
                s.graphics_preset.clone(),
                s.render_distance,
                s.simulation_distance,
                s.cloud_range,
            ));
        }
        ui.release();
        for expected in [
            ("fast", 8, 6, 32),
            ("fancy", 16, 12, 64),
            ("fabulous", 32, 12, 128),
        ] {
            assert!(
                seen.iter()
                    .any(|(p, r, d, c)| (p.as_str(), *r, *d, *c) == expected),
                "{expected:?} in {seen:?}"
            );
        }
    }

    #[test]
    fn gui_scale_cycle_handles_reverse_and_out_of_range_saved_values() {
        for (framebuffer, force_unicode, max) in [
            ([854, 480], false, 2),
            ([1920, 1080], false, 4),
            ([1280, 720], true, 4),
            ([3840, 2160], false, 9),
            ([320, 240], false, 1),
        ] {
            let mut s = Settings {
                force_unicode,
                ..Settings::default()
            };
            for expected in (0..=max).rev() {
                Opt::GuiScale.cycle(&mut s, true, framebuffer);
                assert_eq!(s.gui_scale, expected);
            }
            for expected in (1..=max).chain([0]) {
                Opt::GuiScale.cycle(&mut s, false, framebuffer);
                assert_eq!(s.gui_scale, expected);
            }
            s.gui_scale = u32::MAX;
            Opt::GuiScale.cycle(&mut s, false, framebuffer);
            assert_eq!(s.gui_scale, 0);
            s.gui_scale = u32::MAX;
            Opt::GuiScale.cycle(&mut s, true, framebuffer);
            assert_eq!(s.gui_scale, max - 1);
        }
        let mut s = Settings {
            gui_scale: 4,
            ..Settings::default()
        };
        // A resized window supplies a new maximum on the next interaction.
        Opt::GuiScale.cycle(&mut s, true, [854, 480]);
        assert_eq!(s.gui_scale, 1);
    }

    #[test]
    fn classic_buttons_sound_but_disabled_and_nonprimary_clicks_are_silent() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Mouse);
        let size = (480.0, 320.0);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let button = widgets
            .iter()
            .find(|wd| {
                matches!(
                    wd.kind,
                    WKind::Button {
                        active: true,
                        act: Act::Cycle(_),
                        ..
                    }
                )
            })
            .unwrap();
        let (x, y) = (button.x + 1.0, button.y + 1.0);
        assert!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], false, true)
                .is_empty()
        );
        assert_eq!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
            vec![Event::ButtonClick, Event::Changed]
        );
        assert!(ui.release().is_empty());
        assert!(
            ui.click(&mut s, &atlas, 0.0, 0.0, size, [854, 480], false, false)
                .is_empty()
        );
        ui.show(Page::KeyBinds);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let disabled = widgets
            .iter()
            .find(|wd| matches!(wd.kind, WKind::Button { active: false, .. }))
            .unwrap();
        assert!(
            ui.click(
                &mut s,
                &atlas,
                disabled.x + 1.0,
                disabled.y + 1.0,
                size,
                [854, 480],
                false,
                false
            )
            .is_empty()
        );
    }

    #[test]
    fn classic_sliders_sound_once_on_release_including_unchanged_values() {
        let atlas = atlas();
        let size = (480.0, 320.0);
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Sound);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let slider = widgets
            .iter()
            .find(|wd| {
                matches!(
                    wd.kind,
                    WKind::Slider {
                        opt: Opt::Volume(0),
                        ..
                    }
                )
            })
            .unwrap();
        let (x, y) = (slider.x + slider.w - 1.0, slider.y + 1.0);
        assert_eq!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
            vec![Event::Changed]
        );
        assert_eq!(s.master_volume, 1.0);
        assert_eq!(ui.release(), vec![Event::ButtonClick]);
        assert!(ui.release().is_empty());
        assert_eq!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
            vec![Event::Changed]
        );
        for cx in [-100.0, 250.0, 1000.0] {
            assert_eq!(
                ui.click(&mut s, &atlas, cx, -100.0, size, [854, 480], true, false),
                vec![Event::Changed]
            );
        }
        assert_eq!(ui.release(), vec![Event::ButtonClick]);
        assert!(
            ui.click(&mut s, &atlas, x, y, size, [854, 480], true, false)
                .is_empty()
        );
    }

    #[test]
    fn classic_sliders_follow_the_pointer_and_snap_on_release() {
        let atlas = atlas();
        let size = (480.0, 320.0);
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Sound);
        let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
        let slider = widgets
            .iter()
            .find(|wd| {
                matches!(
                    wd.kind,
                    WKind::Slider {
                        opt: Opt::Volume(0),
                        ..
                    }
                )
            })
            .unwrap();
        let (x, y) = (slider.x + 4.0 + (slider.w - 8.0) * 0.3, slider.y + 1.0);
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false);
        let raw = ui::slider_value(slider.x, slider.w, x);
        // Held: the handle stays under the pointer whatever the stepped value.
        assert_eq!(ui.slider_shown(Opt::Volume(0), 0.25), raw);
        assert_eq!(ui.slider_shown(Opt::Volume(1), 0.25), 0.25);
        ui.click(
            &mut s,
            &atlas,
            x + 10.0,
            -50.0,
            size,
            [854, 480],
            true,
            false,
        );
        let raw = ui::slider_value(slider.x, slider.w, x + 10.0);
        assert_eq!(ui.slider_shown(Opt::Volume(0), 0.25), raw);
        ui.release();
        assert_eq!(ui.slider_shown(Opt::Volume(0), 0.25), 0.25);
        // Render distance applies late (applyValueImmediately = false) and
        // keeps its position until then, even once released.
        ui.slider_value = Some((Opt::RenderDistance, 0.51, Instant::now()));
        assert_eq!(ui.slider_shown(Opt::RenderDistance, 0.5), 0.51);
        ui.slider_value = Some((
            Opt::RenderDistance,
            0.51,
            Instant::now() - std::time::Duration::from_millis(600),
        ));
        assert_eq!(ui.slider_shown(Opt::RenderDistance, 0.5), 0.5);
    }

    #[test]
    fn classic_language_rows_and_text_fields_are_silent() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi {
            languages: languages(),
            ..Default::default()
        };
        ui.show(Page::Language);
        let size = (480.0, 320.0);
        let (widgets, top, bottom, _) = ui.layout(&s, size.0, size.1);
        let row = widgets
            .iter()
            .find(|wd| {
                matches!(wd.kind, WKind::LanguageRow { .. }) && wd.y >= top && wd.y + wd.h <= bottom
            })
            .unwrap();
        assert!(
            ui.click(
                &mut s,
                &atlas,
                row.x + 1.0,
                row.y + 1.0,
                size,
                [854, 480],
                false,
                false
            )
            .is_empty()
        );
        assert!(ui.release().is_empty());
        let search = ui.language_search.clone();
        assert!(
            ui.click(
                &mut s,
                &atlas,
                search.x + 1.0,
                search.y + 1.0,
                size,
                [854, 480],
                false,
                false
            )
            .is_empty()
        );
    }

    #[test]
    fn language_search_hint_does_not_overlap_the_focused_caret() {
        let atlas = atlas();
        let settings = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Language);
        let draw_hint = |ui: &mut OptionsUi| {
            let mut batch = HudBatch::new([960, 640]);
            ui.draw(&settings, &mut batch, &atlas, (-1.0, -1.0));
            let search = &ui.language_search;
            batch.vertices.iter().any(|v| {
                v.color == rgba(170, 170, 170, 255)
                    && v.pos[0] >= search.text_x()
                    && v.pos[0] < search.x + search.width
                    && v.pos[1] >= search.text_y()
                    && v.pos[1] < search.y + search.height
            })
        };
        ui.language_search.focused = false;
        assert!(draw_hint(&mut ui));
        ui.language_search.focused = true;
        assert!(!draw_hint(&mut ui));
        ui.language_search.focused = false;
        ui.language_search.set_value("english");
        assert!(!draw_hint(&mut ui));
        ui.language_search.set_value("");
        assert!(draw_hint(&mut ui));
    }

    fn languages() -> Vec<LanguageInfo> {
        (0..30)
            .map(|i| LanguageInfo {
                code: format!("language_{i}"),
                name: format!("Language {i}"),
                region: String::new(),
                bidirectional: false,
            })
            .collect()
    }

    #[test]
    fn classic_scroll_stops_at_both_limits_before_layout() {
        let s = Settings::default();
        let size = (480.0, 120.0);
        for page in [Page::KeyBinds, Page::Chat, Page::Sound, Page::Language] {
            let mut ui = OptionsUi {
                languages: languages(),
                ..Default::default()
            };
            ui.show(page);
            let (_, top, bottom, content) = ui.layout(&s, size.0, size.1);
            let max = content - (bottom - top);
            assert!(max > 4.5, "{page:?}");
            for _ in 0..3 {
                ui.scroll_at(&s, size, (240.0, 60.0), 1000.0);
                assert_eq!(ui.scroll, 0.0, "{page:?}");
            }
            for _ in 0..3 {
                ui.scroll_at(&s, size, (240.0, 60.0), -1000.0);
                assert_eq!(ui.scroll, max, "{page:?}");
            }
            ui.scroll_at(&s, size, (240.0, 60.0), 0.25);
            assert_eq!(ui.scroll, max - 4.5, "{page:?}");
            ui.scroll_at(&s, size, (240.0, 60.0), 1000.0);
            ui.scroll_at(&s, size, (240.0, 60.0), -0.25);
            assert_eq!(ui.scroll, 4.5, "{page:?}");
        }
    }

    #[test]
    fn partially_visible_keybind_rows_are_clipped_and_clickable() {
        let mut atlas = atlas();
        let button = crate::hud::Rect {
            u0: 0.25,
            v0: 0.25,
            u1: 0.5,
            v1: 0.5,
            w: 20,
            h: 20,
        };
        for name in ["button", "button_disabled", "button_highlighted"] {
            atlas
                .sprites
                .insert(format!("gui/sprites/widget/{name}"), button);
        }
        for above in [true, false] {
            let mut s = Settings::default();
            let mut ui = OptionsUi::default();
            ui.show(Page::KeyBinds);
            let mut batch = HudBatch::new([480, 240]);
            let size = (batch.width, batch.height);
            let (widgets, top, bottom, _) = ui.layout(&s, size.0, size.1);
            let row = widgets
                .iter()
                .filter(|w| {
                    matches!(
                        w.kind,
                        WKind::Button {
                            act: Act::Key(_),
                            ..
                        }
                    )
                })
                .nth(10)
                .unwrap();
            let WKind::Button {
                act: Act::Key(key), ..
            } = row.kind
            else {
                unreachable!()
            };
            let row_y = if above { top - 5.0 } else { bottom - 16.0 };
            ui.scroll = row.y - row_y;
            let (x, width, height) = (row.x, row.w, row.h);
            ui.draw(&s, &mut batch, &atlas, (-1.0, -1.0));
            let vertices: Vec<_> = batch
                .vertices
                .iter()
                .filter(|v| {
                    (0.25..=0.5).contains(&v.uv[0])
                        && (0.25..=0.5).contains(&v.uv[1])
                        && v.pos[0] >= x
                        && v.pos[0] <= x + width
                        && v.pos[1] >= row_y
                        && v.pos[1] <= row_y + height
                })
                .collect();
            assert!(!vertices.is_empty());
            let lo = vertices
                .iter()
                .map(|v| v.pos[1])
                .fold(f32::INFINITY, f32::min);
            let hi = vertices
                .iter()
                .map(|v| v.pos[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(lo, row_y.max(top));
            assert_eq!(hi, (row_y + height).min(bottom));
            let hidden_y = if above {
                row_y + 1.0
            } else {
                row_y + height - 1.0
            };
            assert!(
                ui.click(
                    &mut s,
                    &atlas,
                    x + 1.0,
                    hidden_y,
                    size,
                    [480, 240],
                    false,
                    false
                )
                .is_empty()
            );
            assert_eq!(ui.selected_key, None);
            assert_eq!(
                ui.click(
                    &mut s,
                    &atlas,
                    x + 1.0,
                    (lo + hi) / 2.0,
                    size,
                    [480, 240],
                    false,
                    false
                ),
                vec![Event::ButtonClick]
            );
            assert_eq!(ui.selected_key, Some(key));
        }
    }

    #[test]
    fn classic_scrollbar_drag_is_bounded_silent_and_stops_on_release() {
        let atlas = atlas();
        let size = (480.0, 160.0);
        for page in [Page::KeyBinds, Page::PadBinds, Page::Language, Page::Chat] {
            let mut s = Settings::default();
            let mut ui = OptionsUi {
                languages: languages(),
                ..Default::default()
            };
            ui.show(page);
            let (_, top, bottom, content) = ui.layout(&s, size.0, size.1);
            let max = content - (bottom - top);
            assert!(max > 0.0, "{page:?}");
            let x = ui.scrollbar_x(size.0) + 2.0;
            let (_, thumb) = ui::scrollbar_thumb(top, bottom, content, ui.scroll).unwrap();
            let y = top + thumb / 2.0;
            assert!(
                ui.click(&mut s, &atlas, x, y, size, [960, 480], false, true)
                    .is_empty()
            );
            assert_eq!(ui.scrollbar_drag_y, None);
            assert!(
                ui.click(&mut s, &atlas, x, y, size, [960, 480], false, false)
                    .is_empty()
            );
            assert_eq!(ui.scrollbar_drag_y, Some(y));
            let delta = 5.0;
            assert!(
                ui.click(
                    &mut s,
                    &atlas,
                    x - 100.0,
                    y + delta,
                    size,
                    [960, 480],
                    true,
                    false
                )
                .is_empty()
            );
            assert!((ui.scroll - delta * max / (bottom - top - thumb)).abs() < 0.001);
            for (y, expected) in [(bottom + 100.0, max), (top - 100.0, 0.0)] {
                assert!(
                    ui.click(&mut s, &atlas, x, y, size, [960, 480], true, false)
                        .is_empty()
                );
                assert!((ui.scroll - expected).abs() < 0.001, "{page:?}");
            }
            assert!(ui.release().is_empty());
            assert_eq!(ui.scrollbar_drag_y, None);
            assert!(
                ui.click(
                    &mut s,
                    &atlas,
                    x,
                    bottom + 100.0,
                    size,
                    [960, 480],
                    true,
                    false
                )
                .is_empty()
            );
            assert_eq!(ui.scroll, 0.0);
            assert_eq!(s.keys, Settings::default().keys);
            ui.click(&mut s, &atlas, x, top + 1.0, size, [960, 480], false, false);
            assert!(ui.scrollbar_drag_y.is_some());
            ui.show(Page::Mouse);
            assert_eq!(ui.scrollbar_drag_y, None);
        }
    }

    #[test]
    fn extras_and_vanilla_bindings_accept_mouse_buttons() {
        use crate::extras::{Extras, SORT_KEY};
        use winit::event::MouseButton;
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ] {
            for name in [SORT_KEY, "key.jump"] {
                let mut ui = OptionsUi::default();
                let mut s = Settings::default();
                ui.selected_key = Some(name);
                assert_eq!(ui.bind_mouse(&mut s, button), vec![Event::Changed]);
                assert_eq!(ui.selected_key, None);
                let value = keybinds::mouse_name(button);
                match name {
                    SORT_KEY => assert_eq!(s.extras.sort_key, value),
                    _ => assert_eq!(s.keys.get(name), value),
                }
                assert_eq!(Extras::parse(&s.extras.serialize()), s.extras);
                if name != "key.jump" {
                    assert_eq!(s.keys, Settings::default().keys);
                }
                assert!(ui.bind_mouse(&mut s, button).is_empty());
            }
        }
    }

    #[test]
    fn horizon_extra_mouse_binding_consumes_the_click() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::Extras);
        for (right, expected) in [(false, "key.mouse.left"), (true, "key.mouse.right")] {
            ui.selected_key = Some(crate::extras::SORT_KEY);
            assert_eq!(
                ui.click(
                    &mut s,
                    &atlas,
                    2.0,
                    2.0,
                    (480.0, 240.0),
                    [960, 480],
                    false,
                    right
                ),
                vec![Event::Changed]
            );
            assert_eq!(s.extras.sort_key, expected);
            assert_eq!(ui.selected_key, None);
            assert_eq!(ui.page(), Page::Extras);
        }
    }

    #[test]
    fn key_rebinding_takes_precedence_over_scrollbar_dragging() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::KeyBinds);
        ui.selected_key = Some("key.jump");
        let x = ui.scrollbar_x(480.0) + 2.0;
        assert_eq!(
            ui.click(
                &mut s,
                &atlas,
                x,
                50.0,
                (480.0, 240.0),
                [960, 480],
                false,
                false
            ),
            vec![Event::Changed]
        );
        assert_eq!(s.keys.get("key.jump"), "key.mouse.left");
        assert_eq!(ui.scrollbar_drag_y, None);
        assert!(ui.release().is_empty());
    }

    #[test]
    fn classic_pages_without_overflow_do_not_scroll() {
        let s = Settings::default();
        for page in [Page::Main, Page::Font, Page::Mouse] {
            let mut ui = OptionsUi::default();
            ui.show(page);
            for amount in [-1000.0, 1000.0, -0.25] {
                ui.scroll(&s, (480.0, 720.0), amount);
                assert_eq!(ui.scroll, 0.0, "{page:?}");
            }
        }
    }

    #[test]
    fn classic_resize_positions_rows_at_the_clamped_offset() {
        let s = Settings::default();
        let mut ui = OptionsUi::default();
        ui.show(Page::KeyBinds);
        ui.scroll(&s, (480.0, 120.0), -1000.0);
        assert!(ui.scroll > 0.0);
        let (widgets, _, _, _) = ui.layout(&s, 480.0, 4000.0);
        assert_eq!(ui.scroll, 0.0);
        let (expected, _, _, _) = ui.layout(&s, 480.0, 4000.0);
        assert_eq!(
            widgets.iter().map(|w| w.y).collect::<Vec<_>>(),
            expected.iter().map(|w| w.y).collect::<Vec<_>>()
        );
    }

    #[test]
    fn language_centering_and_filtering_use_bounded_row_positions() {
        let s = Settings {
            language: "language_29".into(),
            ..Default::default()
        };
        let mut ui = OptionsUi {
            languages: languages(),
            ..Default::default()
        };
        ui.show(Page::Language);
        let (widgets, top, bottom, content) = ui.layout(&s, 480.0, 240.0);
        assert_eq!(ui.scroll, content - (bottom - top));
        let (expected, _, _, _) = ui.layout(&s, 480.0, 240.0);
        assert_eq!(
            widgets.iter().map(|w| w.y).collect::<Vec<_>>(),
            expected.iter().map(|w| w.y).collect::<Vec<_>>()
        );
        ui.language_search.set_value("language_29");
        let (widgets, top, _, _) = ui.layout(&s, 480.0, 240.0);
        assert_eq!(ui.scroll, 0.0);
        assert_eq!(widgets.iter().find(|w| w.list).unwrap().y, top + 2.0);
    }

    #[test]
    fn title_shortcuts_initialize_pages_and_return_directly_to_the_title() {
        let mut ui = OptionsUi::default();
        let mut s = Settings::default();
        for page in [Page::Accessibility, Page::Language] {
            ui.language_choice = Some("de_de".into());
            ui.language_search.set_value("old query");
            ui.open_at(false, page);
            assert_eq!(ui.page(), page);
            assert!(!ui.in_world);
            if page == Page::Language {
                assert_eq!(ui.language_choice, None);
                assert!(ui.language_search.value.is_empty());
                assert!(ui.center_language);
            }
            assert!(ui.back(&mut s).contains(&Event::Close));
            assert_eq!(s.language, "en_us");
            ui.open(false);
            ui.show(page);
            assert!(!ui.back(&mut s).contains(&Event::Close));
            assert_eq!(ui.page(), Page::Main);
        }
        ui.open_at(false, Page::Language);
        ui.language_choice = Some("de_de".into());
        let events = ui.back(&mut s);
        assert!(events.contains(&Event::Language("de_de".into())));
        assert!(events.contains(&Event::Close));
        assert_eq!(s.language, "de_de");
    }

    #[test]
    fn horizon_settings_hang_off_options_and_reach_every_horizon_page() {
        let s = Settings::default();
        let mut ui = OptionsUi::default();
        let pushes = |ui: &mut OptionsUi| -> Vec<Page> {
            let (widgets, _, _, _) = ui.layout(&s, 427.0, 240.0);
            widgets
                .iter()
                .filter_map(|w| match &w.kind {
                    WKind::Button {
                        act: Act::Push(page),
                        active: true,
                        ..
                    } => Some(*page),
                    _ => None,
                })
                .collect()
        };
        // Vanilla's Options buttons, Horizon's two in the Telemetry and Credits slots, then
        // Credits & Attribution (WinScreen's credits).
        let main = pushes(&mut ui);
        assert_eq!(
            &main[main.len() - 3..],
            [Page::Horizon, Page::Mods, Page::Credits]
        );
        ui.push(Page::Horizon);
        assert_eq!(
            pushes(&mut ui),
            [
                Page::Graphics,
                Page::GraphicsAdvanced,
                Page::ShaderPacks,
                Page::Controller,
                Page::Extras,
                Page::Mods,
                Page::Agent,
            ]
        );
        // No AI hides the AI Agent page.
        let mut no_ai = Settings::default();
        no_ai.extras.no_ai = true;
        let (widgets, _, _, _) = ui.layout(&no_ai, 427.0, 240.0);
        assert!(!widgets.iter().any(|w| matches!(
            &w.kind,
            WKind::Button {
                act: Act::Push(Page::Agent),
                ..
            }
        )));
        // The AI Agent page: the switch, Copy and the status the game sets.
        ui.push(Page::Agent);
        ui.agent_status = vec!["Waiting for an agent.".into()];
        let (widgets, _, _, _) = ui.layout(&s, 427.0, 240.0);
        let acts: Vec<&Act> = widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WKind::Button { act, .. } => Some(act),
                _ => None,
            })
            .collect();
        assert!(acts.contains(&&Act::Extra(crate::extras::Item::AgentControl)));
        assert!(acts.contains(&&Act::CopyAgentDetails));
        // Vanilla's own pages keep the Horizon pages they had.
        ui.stack = vec![Page::Main, Page::Video];
        let video = pushes(&mut ui);
        for page in [Page::Graphics, Page::GraphicsAdvanced] {
            assert!(video.contains(&page), "{page:?}");
        }
        ui.stack = vec![Page::Main, Page::Controls];
        assert!(pushes(&mut ui).contains(&Page::Controller));
    }

    #[test]
    fn custom_buttons_take_a_line_or_a_key_and_empty_ones_go() {
        use crate::settings::CustomAction;
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = OptionsUi {
            stack: vec![Page::Main, Page::CustomButtons],
            ..Default::default()
        };
        let size = (427.0, 480.0);
        let press = |ui: &mut OptionsUi, s: &mut Settings, wanted: Act| {
            let (widgets, ..) = ui.layout(s, size.0, size.1);
            let at = widgets
                .iter()
                .find_map(|w| match &w.kind {
                    WKind::Button { act, .. } if *act == wanted => Some((w.x + 2.0, w.y + 2.0)),
                    _ => None,
                })
                .expect("the button shows");
            ui.click(s, &atlas, at.0, at.1, size, [1281, 960], false, false)
        };
        // Added and left empty, it goes.
        press(&mut ui, &mut s, Act::CustomAdd);
        assert_eq!(ui.page(), Page::CustomButton(1));
        ui.layout(&s, size.0, size.1);
        assert!(ui.back(&mut s).contains(&Event::CustomButtons));
        assert!(s.custom_buttons.is_empty());
        // What the box holds, it sends.
        press(&mut ui, &mut s, Act::CustomAdd);
        ui.layout(&s, size.0, size.1);
        let field = (ui.custom_send.x + 10.0, ui.custom_send.y + 4.0);
        ui.click(
            &mut s,
            &atlas,
            field.0,
            field.1,
            size,
            [1281, 960],
            false,
            false,
        );
        assert!(ui.text_focused() && ui.text_field_at(field.0, field.1));
        ui.type_text(&atlas, "/home");
        ui.back(&mut s);
        assert_eq!(
            s.custom_buttons[0].action,
            CustomAction::Send("/home".into())
        );
        assert_eq!(s.custom_buttons[0].label(), "/home");
        // A key takes its place, and stays without a line.
        let id = s.custom_buttons[0].id;
        press(&mut ui, &mut s, Act::CustomEdit(id));
        assert_eq!(ui.custom_send.value, "/home");
        press(&mut ui, &mut s, Act::CustomKey(id, "key.sprint"));
        ui.back(&mut s);
        assert_eq!(
            s.custom_buttons[0].action,
            CustomAction::Key("key.sprint".into())
        );
        // And it can be taken away.
        assert!(press(&mut ui, &mut s, Act::CustomDelete(id)).contains(&Event::CustomButtons));
        assert!(s.custom_buttons.is_empty());
    }

    #[test]
    fn the_skin_page_takes_a_name_and_cycles_the_skins() {
        use winit::keyboard::KeyCode;
        let atlas = atlas();
        let mut s = Settings::default();
        // The game's own options; on the stack without `push`, which reads the page from beside
        // options.txt.
        let mut ui = OptionsUi {
            skin_page: true,
            ..Default::default()
        };
        ui.stack.push(Page::Skin);
        ui.skin.saved = "Player00042".into();
        ui.skin.name.set_value("Player00042");
        ui.skin.files = vec!["alex.png".into(), "steve.png".into()];
        let size = (427.0, 240.0);
        let click = |ui: &mut OptionsUi, s: &mut Settings, (x, y): (f32, f32)| {
            ui.click(s, &atlas, x, y, size, [1281, 720], false, false)
        };
        let find = |ui: &mut OptionsUi, wanted: Act| {
            let (widgets, ..) = ui.layout(&Settings::default(), size.0, size.1);
            widgets
                .iter()
                .find_map(|w| match &w.kind {
                    WKind::Button { act, active, .. } if *act == wanted => {
                        Some(((w.x + 2.0, w.y + 2.0), *active))
                    }
                    _ => None,
                })
                .unwrap()
        };
        // The default skin's model is the default's.
        assert!(!find(&mut ui, Act::SkinModel).1);
        // At the box's right end: the cursor after the text.
        let field = (ui.skin.name.x + 260.0, ui.skin.name.y + 4.0);
        assert!(click(&mut ui, &mut s, field).is_empty());
        assert!(ui.text_focused() && ui.text_field_at(field.0, field.1));
        let retype = |ui: &mut OptionsUi, s: &mut Settings, text: &str| {
            for _ in 0..16 {
                ui.key(s, &atlas, KeyCode::Backspace, None);
            }
            for c in text.chars() {
                ui.key(s, &atlas, KeyCode::KeyA, Some(&c.to_string()));
            }
            ui.key(s, &atlas, KeyCode::Enter, None)
        };
        // Refused: the old name back, a note.
        retype(&mut ui, &mut s, "no");
        assert!(ui.take_skin_actions().is_empty());
        assert_eq!(ui.skin.name.value, "Player00042");
        assert!(!ui.text_focused());
        assert!(ui.skin.warning.as_deref().unwrap().contains("Player00042"));
        click(&mut ui, &mut s, field);
        retype(&mut ui, &mut s, "Steve_2");
        assert_eq!(ui.take_skin_actions(), [SkinAction::Name("Steve_2".into())]);
        assert_eq!(ui.skin.warning, None);
        // Unchanged, nothing to keep; leaving the box by a press elsewhere keeps a new one.
        click(&mut ui, &mut s, field);
        ui.key(&mut s, &atlas, KeyCode::Enter, None);
        assert!(ui.take_skin_actions().is_empty());
        click(&mut ui, &mut s, field);
        ui.type_text(&atlas, "X");
        click(&mut ui, &mut s, (1.0, 1.0));
        assert_eq!(
            ui.take_skin_actions(),
            [SkinAction::Name("Steve_2X".into())]
        );
        // Default, each skin, Default again; the model once there is a skin.
        let next = find(&mut ui, Act::NextSkin).0;
        let mut choice_after = |ui: &mut OptionsUi, at: (f32, f32)| {
            assert!(click(ui, &mut s, at).contains(&Event::ButtonClick));
            match ui.take_skin_actions().as_slice() {
                [SkinAction::Choice(choice)] => choice.clone(),
                other => panic!("{other:?}"),
            }
        };
        let alex = choice_after(&mut ui, next);
        assert_eq!(alex.file.as_deref(), Some("alex.png"));
        let (model, active) = find(&mut ui, Act::SkinModel);
        assert!(active);
        assert!(choice_after(&mut ui, model).slim);
        assert_eq!(
            choice_after(&mut ui, next).file.as_deref(),
            Some("steve.png")
        );
        assert_eq!(choice_after(&mut ui, next).file, None);
        let (folder, active) = find(&mut ui, Act::OpenSkinFolder);
        assert!(active);
        click(&mut ui, &mut s, folder);
        assert_eq!(ui.take_skin_actions(), [SkinAction::OpenFolder]);
        // The layers and Main Hand follow the skin.
        find(&mut ui, Act::Cycle(Opt::MainHand));
        // The menus before the game open the page too, with only the layers and Main Hand: the
        // name and skin are the game's own options'.
        let mut menu = OptionsUi::default();
        let (main, ..) = menu.layout(&s, size.0, size.1);
        assert!(main.iter().any(|w| matches!(
            &w.kind,
            WKind::Button {
                act: Act::Push(Page::Skin),
                active: true,
                ..
            }
        )));
        menu.stack.push(Page::Skin);
        let (widgets, ..) = menu.layout(&s, size.0, size.1);
        let acts: Vec<&Act> = widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WKind::Button { act, .. } => Some(act),
                _ => None,
            })
            .collect();
        assert!(acts.contains(&&Act::Cycle(Opt::ModelPart(0))));
        assert!(!acts.contains(&&Act::NextSkin) && !acts.contains(&&Act::OpenSkinFolder));
    }
}
