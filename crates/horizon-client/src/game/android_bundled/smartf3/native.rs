// Copyright (c) 2026 d3grando
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: 184b92bab7851684bbe6b840c5b8cacc10fa05f2. Android static-bundle adaptation.
// SmartF3's adapter to the game (Horizon 142388e1c136e622652ec7505af15d8ad195e041,
// see horizon-mixin.toml). Injected as `crate::game::smartf3_native`: reads
// what the debug screen shows from the game into a `Snapshot`, lets the
// game-independent `crate::game::smartf3` (runtime.rs) arrange it, and draws
// the result where and as the vanilla screen is drawn. Only the window's game
// draws, so nothing here runs on a server thread.
use super::smartf3::{self as sf, Config, Effect, Loaded, ModuleId, Snapshot};
use super::{Game, Screen};
use horizon_client::{
    blocks::Fluid,
    debug_overlay,
    hud::HudBatch,
    hud_layout::{self, GuiDraw, PoseOp},
    save::VERSION_NAME,
    text::Text,
};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// The settings as last read, and the module the settings page shows.
struct Shared {
    /// `config.json` as last read; None until something needs it.
    config: Option<Loaded>,
    selected: ModuleId,
}
static SHARED: Mutex<Shared> = Mutex::new(Shared {
    config: None,
    selected: ModuleId::Fps,
});
/// Never held while the game runs.
fn shared<T>(run: impl FnOnce(&mut Shared) -> T) -> T {
    let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
    run(&mut shared)
}

/// `config.json` in the mod's settings folder; missing is the defaults.
fn read_config(game: &Game<'_>) -> Loaded {
    match game.native_setting(sf::ID, sf::CONFIG_FILE) {
        Some(text) => Config::from_json(&text),
        None => Ok(Config::default()),
    }
}
/// Puts `loaded` in effect.
fn apply(loaded: Loaded) {
    if let Err(error) = &loaded {
        eprintln!(
            "SmartF3: {}: {error}; F3 shows the vanilla screen until it is fixed",
            sf::CONFIG_FILE
        );
    }
    shared(|s| s.config = Some(loaded));
}
/// The configuration as read (once, the first time something needs it).
fn loaded(game: &Game<'_>) -> Loaded {
    if let Some(loaded) = shared(|s| s.config.clone()) {
        return loaded;
    }
    let read = read_config(game);
    apply(read.clone());
    read
}
/// The configuration in effect: a broken file leaves the vanilla screen.
fn config(game: &Game<'_>) -> Config {
    loaded(game).unwrap_or_else(|_| Config {
        enabled: false,
        ..Config::default()
    })
}
/// The window's game, and SmartF3 not turned off in Options > Mods (then
/// every hook but the row and its page passes through untouched).
fn on(game: &Game<'_>) -> bool {
    game.native_window() && game.native_on(sf::ID)
}

/// The game's resident memory, read at most every 2 seconds on a thread of
/// its own (on Windows it asks `tasklist`, which takes a moment) while the
/// System module shows; the last reading meanwhile.
struct Memory {
    bytes: Option<u64>,
    asked: Option<Instant>,
    busy: bool,
}
static MEMORY: Mutex<Memory> = Mutex::new(Memory {
    bytes: None,
    asked: None,
    busy: false,
});
fn memory() -> Option<u64> {
    let mut m = MEMORY.lock().unwrap_or_else(PoisonError::into_inner);
    let due = m
        .asked
        .is_none_or(|at| at.elapsed() >= Duration::from_secs(2));
    if due && !m.busy {
        m.busy = true;
        m.asked = Some(Instant::now());
        let spawned = std::thread::Builder::new()
            .name("SmartF3 memory".into())
            .spawn(|| {
                let bytes = resident_bytes();
                let mut m = MEMORY.lock().unwrap_or_else(PoisonError::into_inner);
                m.bytes = bytes.or(m.bytes);
                m.busy = false;
            });
        if spawned.is_err() {
            m.busy = false;
        }
    }
    m.bytes
}
#[cfg(windows)]
fn resident_bytes() -> Option<u64> {
    use std::os::windows::process::CommandExt;
    /// No console window flashes up for it.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let filter = format!("PID eq {}", std::process::id());
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    sf::parse_tasklist(&String::from_utf8_lossy(&output.stdout))
}
#[cfg(not(windows))]
fn resident_bytes() -> Option<u64> {
    horizon_bench::rss_bytes(std::process::id())
}

impl Game<'_> {
    /// `Game::hud`'s HEAD, while F3 is open and SmartF3 draws it: the
    /// game's own `hud` with SmartF3's screen where `draw_debug_overlay`
    /// draws the vanilla one (after the screen and the toasts, before
    /// Horizon's frame graph and the controller cursor). None runs the
    /// game's `hud` (F3 closed, SmartF3 off in Options > Mods, set to
    /// vanilla F3, or its file broken).
    pub(in crate::game) fn smartf3_hud(&mut self, size: [u32; 2]) -> Option<HudBatch> {
        if self.code_of_conduct.is_some() || !self.debug || !on(self) || !config(self).enabled {
            return None;
        }
        let mut batch = if (self.server_packs.prompt.is_some() || self.dialog_open())
            && self.player.is_none()
        {
            HudBatch::new(size)
        } else {
            self.hud_layers(size)
        };
        self.draw_dialog(&mut batch);
        self.draw_pack_prompt(&mut batch);
        self.draw_link_prompt(&mut batch);
        self.smartf3_draw(&mut batch, size);
        self.frame_graph
            .draw(self.settings.horizon.overlay, &mut batch, &self.gui.atlas);
        self.draw_pad_cursor(&mut batch);
        Some(batch)
    }

    /// SmartF3's screen in place of `draw_debug_overlay`'s: on the same
    /// conditions (F3 open, not hidden by F1 unless a screen is open), at
    /// the same scale, with the same charts.
    fn smartf3_draw(&self, batch: &mut HudBatch, size: [u32; 2]) {
        let config = config(self);
        let screen_open = self.title.is_some() || self.screen != Screen::Playing;
        if !self.debug || (self.ui.hide_gui && !screen_open) {
            return;
        }
        // The vanilla screen's scale and size (`draw_debug_overlay`).
        let framebuffer = size.map(|v| v as i32);
        let (scale, [width, height]) = debug_overlay::touch_scale(
            framebuffer,
            batch.scale as i32,
            self.settings.file.int("debugGuiScale"),
            self.settings.force_unicode,
            [batch.width as i32, batch.height as i32],
        );
        let snapshot = self.smartf3_snapshot(&config, framebuffer);
        let atlas = &self.gui.atlas;
        let columns = sf::columns(&config, &snapshot);
        let placed = sf::layout(&columns, width, |t| atlas.font.width(t) as i32);
        let mut draws = vec![GuiDraw::Push(vec![PoseOp::Scale(scale, scale)])];
        if let Some(argb) = config.background.argb() {
            for p in &placed {
                draws.push(hud_layout::fill(
                    p.x - 1,
                    p.y - 1,
                    p.x + p.width + 1,
                    p.y + sf::LINE_HEIGHT - 1,
                    argb,
                ));
            }
        }
        for p in placed {
            let value = Text::colored(p.line.value, p.line.value_rgb);
            let text = if p.line.label.is_empty() {
                value
            } else {
                Text::colored(p.line.label, p.line.label_rgb).append(value)
            };
            draws.push(GuiDraw::Text {
                text,
                x: p.x,
                y: p.y,
                color: sf::TEXT_ARGB,
                shadow: config.shadow,
            });
        }
        draws.push(GuiDraw::Pop);
        // The charts (F3+1, F3+2, the lightmap), as the vanilla screen draws them.
        let charts = self.debug_charts.draw(
            &atlas.font,
            width,
            height,
            self.settings.max_fps,
            1000.0 / self.clock.rate(),
            !self.has_singleplayer_server(),
        );
        draws.push(GuiDraw::Push(vec![PoseOp::Scale(scale, scale)]));
        draws.extend(charts);
        if self.debug_charts.lightmap
            && let Some(r) = &self.renderer
        {
            draws.extend(horizon_client::debug_chart::lightmap(
                &r.lightmap_pixels,
                width,
                height,
            ));
        }
        draws.push(GuiDraw::Pop);
        hud_layout::paint(batch, atlas, &draws);
    }

    /// What the shown modules need, read as the vanilla screen reads it.
    fn smartf3_snapshot(&self, config: &Config, framebuffer: [i32; 2]) -> Snapshot {
        let wants = |m: ModuleId| config.wants(m);
        let mut s = Snapshot {
            // `LocalPlayer.isReducedDebugInfo`: the server's rule or the option.
            reduced: self.settings.reduced_debug_info || self.ui.rules.bool("reduced_debug_info"),
            version: format!("{VERSION_NAME} ({VERSION_NAME}/horizon)"),
            dimension: self.dim.id().to_owned(),
            forced_chunks: self.streaming.forced.len(),
            ..Snapshot::default()
        };
        if wants(ModuleId::Fps) {
            s.fps = Some(self.smartf3_fps());
        }
        if wants(ModuleId::Server) {
            s.server = self.smartf3_server();
        }
        if wants(ModuleId::System) {
            s.system = self.smartf3_system(framebuffer);
        }
        if wants(ModuleId::Rendering) {
            s.rendering = self.smartf3_rendering();
        }
        if wants(ModuleId::Charts) {
            s.charts = self.debug_chart_lines().to_vec();
        }
        let Some(p) = self.player.as_ref() else {
            return s;
        };
        let pos = [p.position.x, p.position.y, p.position.z];
        s.player = Some(sf::Player {
            pos,
            yaw: self.yaw,
            pitch: self.pitch,
        });
        s.day_time = Some(self.world.day_time);
        let feet = pos.map(|v| v.floor() as i32);
        if wants(ModuleId::Biome) {
            s.biome = self
                .world
                .chunks
                .get(&(feet[0] >> 4, feet[2] >> 4))
                .and_then(|c| {
                    let biome = c.biome(feet[0], feet[1], feet[2]) as usize;
                    self.res.biomes().list.get(biome).map(|b| b.name.clone())
                });
        }
        if wants(ModuleId::Light) && self.world.loaded(feet[0], feet[2]) {
            let light = self.world.light(feet[0], feet[1], feet[2]);
            s.light = Some([light >> 4, light & 15]);
        }
        if s.reduced {
            return s;
        }
        // F3 looks 20 blocks ahead (`Entity.pick(20.0, ...)`), further
        // than the hand reaches.
        if let Some(eye) = self.eye_position() {
            let dir = super::view_vector(self.yaw, self.pitch);
            if wants(ModuleId::TargetBlock) {
                s.target_block = self
                    .clip_ray(eye, dir, 20.0)
                    .map(|hit| self.smartf3_block(hit.pos));
            }
            if wants(ModuleId::TargetFluid) {
                s.target_fluid = self.smartf3_fluid(eye, dir, 20.0);
            }
        }
        if wants(ModuleId::TargetEntity) {
            s.target_entity = self.smartf3_entity();
        }
        s
    }

    /// `DebugEntryFps`, as `draw_debug_overlay` reads it.
    fn smartf3_fps(&self) -> sf::Fps {
        let limit = self.framerate_limit();
        let present = self
            .renderer
            .as_ref()
            .map_or("", |r| match r.config.present_mode {
                wgpu::PresentMode::Immediate => " (immediate)",
                wgpu::PresentMode::Mailbox => " (mailbox)",
                wgpu::PresentMode::Fifo => " (fifo)",
                wgpu::PresentMode::FifoRelaxed => " (fifo relaxed)",
                _ => "",
            });
        let refresh = self
            .window
            .as_ref()
            .and_then(|w| w.current_monitor())
            .and_then(|m| m.refresh_rate_millihertz())
            .map_or("0".into(), |mhz| {
                debug_overlay::refresh_rate_label(f64::from(mhz) / 1000.0)
            });
        sf::Fps {
            fps: self.fps.round() as i32,
            limit: (limit != 260).then_some(limit),
            present: present.to_owned(),
            refresh,
        }
    }

    /// `DebugEntryTps`, as `draw_debug_overlay` reads it: in a world only.
    fn smartf3_server(&self) -> Option<sf::Server> {
        self.player.as_ref()?;
        let [tx, rx] = self.debug_charts.packet_rates.map(|n| n.round() as i64);
        let (target, status) = if self.clock.sprinting() {
            ("-".to_owned(), " (sprinting)")
        } else if self.clock.stepping() {
            (
                format!("{:.1}", 1000.0 / self.clock.rate()),
                " (frozen - stepping)",
            )
        } else if self.clock.frozen() {
            (format!("{:.1}", 1000.0 / self.clock.rate()), " (frozen)")
        } else {
            (format!("{:.1}", 1000.0 / self.clock.rate()), "")
        };
        let (brand, tick_ms) = if self.remote.is_none() || self.integrated.is_some() {
            let tick_ms = self
                .integrated
                .as_ref()
                .map_or(self.debug_charts.smoothed_tick_ms, |server| {
                    server.smoothed_tick_ms()
                });
            (None, Some(tick_ms))
        } else {
            (Some(self.remote.as_ref()?.brand.clone()), None)
        };
        Some(sf::Server {
            brand,
            tick_ms,
            target,
            status: status.to_owned(),
            tx,
            rx,
        })
    }

    /// `system_specs` and the memory, as `draw_debug_overlay` reads them.
    fn smartf3_system(&self, framebuffer: [i32; 2]) -> sf::System {
        let mut system = sf::System {
            memory: memory(),
            ..sf::System::default()
        };
        let Some(r) = self.renderer.as_ref() else {
            return system;
        };
        static CPU: OnceLock<String> = OnceLock::new();
        system.cpu = Some(CPU.get_or_init(horizon_client::crash::cpu).clone());
        let info = &r.info;
        system.display = Some(format!(
            "{}x{} ({})",
            framebuffer[0],
            framebuffer[1],
            // Metal adapters report no PCI vendor.
            if info.vendor == 0 && info.backend == wgpu::Backend::Metal {
                "Apple".into()
            } else {
                debug_overlay::vendor_name(info.vendor)
            }
        ));
        if let Some(w) = self.window.as_ref() {
            let density = w.scale_factor();
            let logical = w.inner_size().to_logical::<f64>(density);
            system.window = Some(format!(
                "{}x{} ({density:.2}x pixel density)",
                logical.width.round() as i32,
                logical.height.round() as i32
            ));
        }
        let kind = match info.device_type {
            wgpu::DeviceType::IntegratedGpu => " (iGPU)",
            wgpu::DeviceType::DiscreteGpu => " (dGPU)",
            wgpu::DeviceType::VirtualGpu => " (vGPU)",
            wgpu::DeviceType::Cpu => " (software)",
            wgpu::DeviceType::Other => "",
        };
        system.gpu = Some(format!("{}{kind}", info.name));
        let driver = format!("{} {}", info.driver, info.driver_info);
        system.driver = Some(
            format!(
                "{:?} {}",
                info.backend,
                driver.trim().lines().next().unwrap_or("")
            )
            .trim_end()
            .to_owned(),
        );
        system
    }

    /// `simple_performance_impactors` and the shader pack's passes, as
    /// `draw_debug_overlay` writes them.
    fn smartf3_rendering(&self) -> Vec<String> {
        let s = &self.settings;
        let mut lines = vec![
            format!(
                "{}{}B: {}",
                if s.improved_transparency {
                    "improved-transparency "
                } else {
                    ""
                },
                match s.clouds.as_str() {
                    "false" => "",
                    "fast" => "fast-clouds ",
                    _ => "fancy-clouds ",
                },
                s.biome_blend
            ),
            "Filtering: None".to_owned(),
            format!(
                "Terrain Rendering: {}",
                if self.renderer.as_ref().is_some_and(|r| r.multi_draw) {
                    "multidrawindirect"
                } else {
                    "naive"
                }
            ),
        ];
        if let Some(r) = self.renderer.as_ref() {
            lines.extend(debug_overlay::shader_pass_lines(&r.shader_pass_times()));
        }
        lines
    }

    /// The block at `pos`: its id and its state's properties.
    fn smartf3_block(&self, pos: [i32; 3]) -> sf::TargetBlock {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let name = self.res.facts().get(state).name.to_string();
        let properties: Vec<(String, String)> = self
            .pack
            .assets
            .registry
            .state(state)
            .map(|s| {
                s.properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        sf::TargetBlock {
            pos,
            name,
            properties,
        }
    }

    /// The first fluid (source or flowing, waterlogged blocks too) along the
    /// view within `reach` blocks, before any other block.
    fn smartf3_fluid(&self, eye: [f64; 3], dir: [f64; 3], reach: f64) -> Option<sf::TargetFluid> {
        let facts = self.res.facts();
        let mut last: Option<[i32; 3]> = None;
        let mut t = 0.0;
        while t <= reach {
            let b = [0usize, 1, 2].map(|a| (eye[a] + dir[a] * t).floor() as i32);
            if last != Some(b) {
                last = Some(b);
                let state = self.world.block(b[0], b[1], b[2]);
                let f = facts.get(state);
                if f.fluid != Fluid::None {
                    return Some(sf::TargetFluid {
                        pos: b,
                        lava: f.fluid == Fluid::Lava,
                        amount: f.fluid_level,
                        falling: f.fluid_falling,
                        source: f.fluid_level == 8 && !f.fluid_falling,
                    });
                }
                if !facts.is_air(state) {
                    return None;
                }
            }
            t += 0.05;
        }
        None
    }

    /// The type of the entity under the crosshair before any block, as the
    /// game's own hit tests find it (`Minecraft.crosshairPickEntity`).
    fn smartf3_entity(&self) -> Option<String> {
        let mut best: Option<(f64, String)> = None;
        let mut offer = |distance: f64, kind: String| {
            if best.as_ref().is_none_or(|b| distance < b.0) {
                best = Some((distance, kind));
            }
        };
        if let Some((i, d)) = self.mob_hit() {
            offer(d, self.mobs[i].kind.id.clone());
        }
        if let Some((_, d, _)) = self.armor_stand_hit() {
            offer(d, "minecraft:armor_stand".to_owned());
        }
        if let Some((_, d)) = self.painting_hit() {
            offer(d, "minecraft:painting".to_owned());
        }
        if let Some((i, d)) = self.frame_hit() {
            offer(d, self.item_frames[i].kind().to_owned());
        }
        if let Some((i, d)) = self.boat_hit() {
            offer(d, self.boats[i].entity_type());
        }
        if let Some((i, d)) = self.cart_hit() {
            offer(d, self.carts[i].kind.id().to_owned());
        }
        if let Some((_, d)) = self.dragon_hit() {
            offer(d, "minecraft:ender_dragon".to_owned());
        }
        if let Some((_, d)) = self.guest_hit() {
            offer(d, "minecraft:player".to_owned());
        }
        let (distance, kind) = best?;
        self.before_block(distance).then_some(kind)
    }

    /// `native_command` (a hook point): `/smartf3 [status|on|off|toggle|
    /// compact|reload]`, typed in the window's chat; true when it was ours.
    pub(super) fn smartf3_command(&mut self, command: &str) -> bool {
        if !on(self) {
            return false;
        }
        let Some(parsed) = sf::parse_command(command) else {
            return false;
        };
        let message = match parsed {
            Err(usage) => usage,
            Ok(sf::Command::Reload) => {
                let read = read_config(self);
                let message = match &read {
                    Ok(config) => format!("{} read again. {}", sf::CONFIG_FILE, sf::status(config)),
                    Err(error) => format!(
                        "{}: {error}. F3 shows the vanilla screen until it is fixed.",
                        sf::CONFIG_FILE
                    ),
                };
                apply(read);
                message
            }
            Ok(command) => match loaded(self) {
                Err(error) => format!(
                    "{}: {error}. Fix it, or reset it in Options > Mods > {} > Settings.",
                    sf::CONFIG_FILE,
                    sf::NAME
                ),
                Ok(config) => {
                    let (next, answer) = sf::run_command(&config, command);
                    if let Some(next) = next {
                        self.native_set_setting(sf::ID, sf::CONFIG_FILE, &next.to_json());
                        apply(Ok(next));
                    }
                    answer
                }
            },
        };
        self.native_chat(&message);
        true
    }

    /// `native_settings` (a hook point): the settings page's buttons.
    pub(super) fn smartf3_settings(&self) -> Vec<String> {
        let selected = shared(|s| s.selected);
        sf::labels(&loaded(self), selected)
    }

    /// `native_settings_click` (a hook point): change the setting, write
    /// `config.json` and put it in effect at once (or read the file again,
    /// or show another module's buttons).
    pub(super) fn smartf3_settings_click(&mut self, button: usize) {
        let selected = shared(|s| s.selected);
        match sf::click(&loaded(self), selected, button) {
            Some(Effect::Save(config)) => {
                self.native_set_setting(sf::ID, sf::CONFIG_FILE, &config.to_json());
                apply(Ok(config));
            }
            Some(Effect::Reload) => apply(read_config(self)),
            Some(Effect::Select(id)) => shared(|s| s.selected = id),
            None => {}
        }
    }
}

/// Reset profile-scoped state when Android returns from its launcher.
pub(super) fn reset() {
    shared(|state| {
        state.config = None;
        state.selected = ModuleId::Fps;
    });
}
