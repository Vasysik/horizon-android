//! Horizon's own graphics settings, beyond vanilla's options: kept in
//! `horizon-graphics.txt` next to `options.txt` so that file stays exactly
//! vanilla's. Every default reproduces vanilla's rendering; only choices the
//! player makes here depart from it.
use std::{fs, path::Path};

pub const FILE: &str = "horizon-graphics.txt";
/// Written beside it before HDR output is switched on and removed once the first HDR frame is
/// presented. Found at the next start, it means HDR output took the game down, so HDR starts off.
pub const HDR_MARKER: &str = "horizon-hdr-pending";

/// Simple-level presets. `Custom` leaves every option as the player set it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Preset {
    Performance,
    Balanced,
    Quality,
    Ultra,
    #[default]
    Custom,
}
impl Preset {
    pub const ALL: [Preset; 5] = [
        Preset::Performance,
        Preset::Balanced,
        Preset::Quality,
        Preset::Ultra,
        Preset::Custom,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Preset::Performance => "performance",
            Preset::Balanced => "balanced",
            Preset::Quality => "quality",
            Preset::Ultra => "ultra",
            Preset::Custom => "custom",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Preset::Performance => "Performance",
            Preset::Balanced => "Balanced",
            Preset::Quality => "Quality",
            Preset::Ultra => "Ultra",
            Preset::Custom => "Custom",
        }
    }
    fn parse(s: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == s)
            .unwrap_or_default()
    }
}

/// How the swapchain presents frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresentMode {
    /// Vanilla's choice: Fifo with VSync on, else the first of Immediate,
    /// Mailbox and Fifo the surface supports. The other modes apply only
    /// with VSync off: VSync on always presents with Fifo.
    #[default]
    Auto,
    Fifo,
    Mailbox,
    Immediate,
}
impl PresentMode {
    pub const ALL: [PresentMode; 4] = [
        PresentMode::Auto,
        PresentMode::Fifo,
        PresentMode::Mailbox,
        PresentMode::Immediate,
    ];
    pub fn name(self) -> &'static str {
        match self {
            PresentMode::Auto => "auto",
            PresentMode::Fifo => "fifo",
            PresentMode::Mailbox => "mailbox",
            PresentMode::Immediate => "immediate",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PresentMode::Auto => "Auto (VSync)",
            PresentMode::Fifo => "FIFO",
            PresentMode::Mailbox => "Mailbox",
            PresentMode::Immediate => "Immediate",
        }
    }
    fn parse(s: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == s)
            .unwrap_or_default()
    }
}

/// The graphics API the renderer runs on. Only Windows has a choice: the
/// other systems always use their one backend (Vulkan on Linux, Metal on
/// macOS). Applies at the next start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GraphicsApi {
    /// Vulkan, then DirectX 12 (Windows), then OpenGL when neither has a surface, adapter or device.
    #[default]
    Auto,
    Vulkan,
    Dx12,
    /// OpenGL, for GPUs with no Vulkan or DirectX 12 driver (Windows and Linux).
    OpenGl,
}
impl GraphicsApi {
    pub const ALL: [GraphicsApi; 4] = [
        GraphicsApi::Auto,
        GraphicsApi::Vulkan,
        GraphicsApi::Dx12,
        GraphicsApi::OpenGl,
    ];
    pub fn name(self) -> &'static str {
        match self {
            GraphicsApi::Auto => "auto",
            GraphicsApi::Vulkan => "vulkan",
            GraphicsApi::Dx12 => "dx12",
            GraphicsApi::OpenGl => "opengl",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            GraphicsApi::Auto => "Auto",
            GraphicsApi::Vulkan => "Vulkan",
            GraphicsApi::Dx12 => "DirectX 12",
            GraphicsApi::OpenGl => "OpenGL",
        }
    }
    /// The API a name stands for, ignoring case; `None` for anything else.
    pub fn from_name(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|api| api.name().eq_ignore_ascii_case(s.trim()))
    }
    /// `HORIZON_GRAPHICS_API` (auto, vulkan, dx12 or opengl) if set, else the setting.
    pub fn with_env(self) -> Self {
        std::env::var("HORIZON_GRAPHICS_API")
            .ok()
            .and_then(|v| Self::from_name(&v))
            .unwrap_or(self)
    }
}

/// What the frame limiter does in the last half millisecond before a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pacing {
    /// Sleep the whole wait: no CPU spent spinning, a little less precise.
    #[default]
    Efficient,
    /// Vanilla's `FramerateLimiter`: sleep, then spin until the deadline.
    Precise,
}

/// The performance overlay drawn over the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Overlay {
    #[default]
    Off,
    /// Frames per second and frame time.
    Fps,
    /// The numbers and a graph of recent frame times.
    Graph,
    /// Everything that tells why a frame is slow: frame times with the 1% low, memory, chunks,
    /// meshing, draw calls, temperatures and throttling (stats_overlay.rs).
    Stats,
}

/// How finely far terrain is drawn (`FarDetail` in the LOD options).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FarDetail {
    /// Each tile level starts at eight times its size from the camera.
    #[default]
    Full,
    /// Coarser levels start at half that distance: about a quarter of the
    /// far quads, for GPUs that far terrain slows down. Changes the picture.
    Reduced,
}
impl FarDetail {
    pub fn name(self) -> &'static str {
        match self {
            FarDetail::Full => "full",
            FarDetail::Reduced => "reduced",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FarDetail::Full => "Full",
            FarDetail::Reduced => "Reduced",
        }
    }
    fn parse(s: &str) -> Self {
        if s == "reduced" {
            FarDetail::Reduced
        } else {
            FarDetail::Full
        }
    }
}

static REDUCED_SUGGESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Whether the settings suggest Reduced far detail: set when the renderer
/// starts, on the GPUs `lod_capped` names (far terrain slows them most).
pub fn suggest_reduced_far_detail(on: bool) {
    REDUCED_SUGGESTED.store(on, std::sync::atomic::Ordering::Relaxed);
}
pub fn reduced_far_detail_suggested() -> bool {
    REDUCED_SUGGESTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// The farthest LOD Distance, in chunks, on integrated GPUs other than
/// Apple's: they share the CPU's memory bandwidth, and past it far terrain
/// costs them more frame time than it is worth.
pub const INTEGRATED_LOD_DISTANCE: u32 = 128;
/// Whether `INTEGRATED_LOD_DISTANCE` applies to this adapter. Apple's GPUs
/// are integrated too, with memory and bandwidth enough for every distance.
pub fn lod_capped(info: &wgpu::AdapterInfo) -> bool {
    info.device_type == wgpu::DeviceType::IntegratedGpu && !info.name.starts_with("Apple")
}
/// A LOD distance as far terrain uses it on this adapter.
pub fn lod_distance_capped(distance: u32, info: &wgpu::AdapterInfo) -> u32 {
    let distance = distance.min(1024);
    if lod_capped(info) {
        distance.min(INTEGRATED_LOD_DISTANCE)
    } else {
        distance
    }
}
impl Graphics {
    /// `lod_distance` as far terrain uses it on this adapter.
    pub fn lod_distance_on(&self, info: &wgpu::AdapterInfo) -> u32 {
        lod_distance_capped(self.lod_distance, info)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Graphics {
    pub preset: Preset,
    /// Distant surface terrain range, in chunks; 0 disables LODs.
    pub lod_distance: u32,
    /// How finely far terrain is drawn; Full unless the player picks Reduced.
    pub far_detail: FarDetail,
    /// Skip sections hidden behind closed ones (vanilla's occlusion graph).
    pub cave_culling: bool,
    /// Skip drawing entities hidden behind opaque terrain.
    pub entity_culling: bool,
    /// Meshed sections uploaded per frame; 0 uploads all that are ready.
    pub chunk_updates: u32,
    /// Section mesh bytes uploaded per frame, in KiB; 0 is unlimited.
    pub upload_budget_kib: u32,
    pub sky: bool,
    pub fog: bool,
    pub weather: bool,
    pub animated_textures: bool,
    pub present_mode: PresentMode,
    pub pacing: Pacing,
    /// Chunk worker threads; 0 uses every core but one. Applies at the next start.
    pub workers: u32,
    pub overlay: Overlay,
    /// World resolution as a percentage of the window's (50..=200).
    pub resolution_scale: u32,
    /// FXAA over the world.
    pub fxaa: bool,
    /// HDR output (HDR10, or scRGB), where the display path offers it.
    pub hdr: bool,
    /// HDR paper white (where SDR white sits), in nits.
    pub hdr_paper_white: u32,
    /// HDR peak brightness the world's highlights reach, in nits.
    pub hdr_peak: u32,
    /// Battery mode: at most 60 frames per second in a world and 30 in menus
    /// or behind other windows, and half the worker threads. Off by default;
    /// it changes when frames are drawn, never what they show or the ticks.
    pub battery: bool,
    /// The graphics API (Windows only; see `GraphicsApi`).
    pub api: GraphicsApi,
    /// The shader pack picked in `shaderpacks/` (`shader_pack_list`); empty: none. Its key is
    /// not `shaderPack`, which versions before the clean-room rewrite saved and which stays
    /// ignored.
    pub shader_pack: String,
    /// The picked `shaders/` pack's profile (`profile.<name>`): empty for
    /// `HORIZON_SHADER_PROFILE`'s, `default` for the pack's own defaults.
    pub shader_profile: String,
}
/// The profile that fits the pack to the device (`game/shader_packs.rs`): a phone's until the
/// player picks another.
pub const AUTO_PROFILE: &str = "auto";

/// The most of the screen's resolution Auto draws the world at on this device.
pub fn auto_shader_scale() -> f32 {
    use crate::system_stats::Tier;
    static SCALE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *SCALE.get_or_init(|| match Tier::here().unwrap_or(Tier::High) {
        Tier::High => 0.5,
        Tier::Mid => 0.4,
        Tier::Low => 0.3,
    })
}

/// A pack made to be cheap, which keeps the player's resolution and draws no depth first:
/// Starling Visuals and MakeUp Ultra Fast.
pub fn light_shader_pack(name: &str) -> bool {
    let name = name.replace(['-', '_', ' '], "").to_ascii_lowercase();
    name.starts_with("starlingvisuals") || name.starts_with("makeup")
}

impl Graphics {
    /// The shader pack and profile chosen: `HORIZON_SHADER_PACK`'s, for development, else the
    /// Shader Packs page's.
    pub fn shader_choice(&self) -> Option<(String, String)> {
        let env = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());
        let pack = env("HORIZON_SHADER_PACK")
            .or_else(|| Some(self.shader_pack.clone()).filter(|p| !p.is_empty()))?;
        let profile = match self.shader_profile.as_str() {
            "" => env("HORIZON_SHADER_PROFILE").unwrap_or_else(|| {
                if cfg!(target_os = "android") {
                    AUTO_PROFILE.to_owned()
                } else {
                    String::new()
                }
            }),
            "default" => String::new(),
            profile => profile.to_owned(),
        };
        Some((pack, profile))
    }
}
impl Default for Graphics {
    fn default() -> Self {
        Self {
            preset: Preset::Custom,
            lod_distance: 0,
            far_detail: FarDetail::Full,
            cave_culling: true,
            entity_culling: false,
            chunk_updates: 0,
            upload_budget_kib: 0,
            sky: true,
            fog: true,
            weather: true,
            animated_textures: true,
            present_mode: PresentMode::Auto,
            pacing: Pacing::Efficient,
            workers: 0,
            overlay: Overlay::Off,
            resolution_scale: 100,
            fxaa: false,
            hdr: false,
            hdr_paper_white: 200,
            hdr_peak: 1000,
            battery: false,
            api: GraphicsApi::Auto,
            shader_pack: String::new(),
            shader_profile: String::new(),
        }
    }
}
fn bool_text(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}
impl Graphics {
    pub fn load(dir: &Path) -> Self {
        let mut g: Self = fs::read_to_string(dir.join(FILE))
            .map(|t| Self::parse(&t))
            .unwrap_or_default();
        if fs::remove_file(dir.join(HDR_MARKER)).is_ok() && g.hdr {
            g.hdr = false;
            g.save(dir);
            log::warn!(
                "the last start ended before HDR output showed a frame; HDR is off now (graphics option hdr, in {FILE})"
            );
        }
        g
    }
    pub fn parse(text: &str) -> Self {
        let mut g = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let flag = |d: bool| match value {
                "true" => true,
                "false" => false,
                _ => d,
            };
            let number = |d: u32| value.parse().unwrap_or(d);
            match key.trim() {
                "preset" => g.preset = Preset::parse(value),
                "lodDistance" => g.lod_distance = number(g.lod_distance).min(1024),
                "farDetail" => g.far_detail = FarDetail::parse(value),
                "caveCulling" => g.cave_culling = flag(g.cave_culling),
                "entityCulling" => g.entity_culling = flag(g.entity_culling),
                "chunkUpdates" => g.chunk_updates = number(g.chunk_updates).min(512),
                "uploadBudgetKiB" => g.upload_budget_kib = number(g.upload_budget_kib).min(65536),
                "sky" => g.sky = flag(g.sky),
                "fog" => g.fog = flag(g.fog),
                "weather" => g.weather = flag(g.weather),
                "animatedTextures" => g.animated_textures = flag(g.animated_textures),
                "presentMode" => g.present_mode = PresentMode::parse(value),
                "framePacing" => {
                    g.pacing = if value == "precise" {
                        Pacing::Precise
                    } else {
                        Pacing::Efficient
                    }
                }
                "workers" => g.workers = number(g.workers).min(64),
                "resolutionScale" => g.resolution_scale = number(g.resolution_scale).clamp(25, 200),
                "fxaa" => g.fxaa = flag(g.fxaa),
                "hdr" => g.hdr = flag(g.hdr),
                "hdrPaperWhite" => g.hdr_paper_white = number(g.hdr_paper_white).clamp(80, 480),
                "hdrPeak" => g.hdr_peak = number(g.hdr_peak).clamp(200, 2000),
                "batteryMode" => g.battery = flag(g.battery),
                "graphicsApi" => g.api = GraphicsApi::from_name(value).unwrap_or_default(),
                "pickedShaderPack" => g.shader_pack = value.to_owned(),
                // Starling's pick before it took this page's key (`off`: none).
                "shaders" if g.shader_pack.is_empty() && value != "off" => {
                    g.shader_pack = value.to_owned()
                }
                "shadersProfile" => g.shader_profile = value.to_owned(),
                "overlay" => {
                    g.overlay = match value {
                        "fps" => Overlay::Fps,
                        "graph" => Overlay::Graph,
                        "stats" => Overlay::Stats,
                        _ => Overlay::Off,
                    }
                }
                _ => {}
            }
        }
        g
    }
    pub fn serialize(&self) -> String {
        format!(
            "preset:{}\nlodDistance:{}\nfarDetail:{}\ncaveCulling:{}\nentityCulling:{}\nchunkUpdates:{}\nuploadBudgetKiB:{}\nsky:{}\nfog:{}\nweather:{}\nanimatedTextures:{}\npresentMode:{}\nframePacing:{}\nworkers:{}\noverlay:{}\nresolutionScale:{}\nfxaa:{}\nhdr:{}\nhdrPaperWhite:{}\nhdrPeak:{}\nbatteryMode:{}\ngraphicsApi:{}\npickedShaderPack:{}\nshadersProfile:{}\n",
            self.preset.name(),
            self.lod_distance,
            self.far_detail.name(),
            bool_text(self.cave_culling),
            bool_text(self.entity_culling),
            self.chunk_updates,
            self.upload_budget_kib,
            bool_text(self.sky),
            bool_text(self.fog),
            bool_text(self.weather),
            bool_text(self.animated_textures),
            self.present_mode.name(),
            match self.pacing {
                Pacing::Efficient => "efficient",
                Pacing::Precise => "precise",
            },
            self.workers,
            match self.overlay {
                Overlay::Off => "off",
                Overlay::Fps => "fps",
                Overlay::Graph => "graph",
                Overlay::Stats => "stats",
            },
            self.resolution_scale,
            bool_text(self.fxaa),
            bool_text(self.hdr),
            self.hdr_paper_white,
            self.hdr_peak,
            bool_text(self.battery),
            self.api.name(),
            self.shader_pack,
            self.shader_profile,
        )
    }
    pub fn save(&self, dir: &Path) {
        let _ = fs::write(dir.join(FILE), self.serialize());
    }
    /// Worker threads for chunk generation and meshing.
    pub fn worker_threads(&self) -> usize {
        if self.workers > 0 {
            return self.workers as usize;
        }
        // Vanilla's count (Util.maxAllowedExecutorThreads: every core but
        // one). The pool never takes every core: a pool that fills them all
        // starves the render and server threads, and the frame rate collapses
        // while the workers saturate the CPU and its memory bandwidth. One
        // core left is enough with the workers behind those two threads
        // (horizon_threads: niced on Linux; on macOS and Windows the server
        // and render threads are raised above four cores, where vanilla
        // raises its server thread). Leaving two idled a core of a 4-thread
        // laptop while a world loaded: a teleport there took ~15% longer.
        // A phone leaves two, as Starling was measured with: half its cores are small ones, and
        // a hot phone slows every core down.
        let spare = if cfg!(target_os = "android") { 2 } else { 1 };
        let auto = std::thread::available_parallelism()
            .map_or(2, |n| n.get())
            .saturating_sub(spare)
            .max(1);
        if self.battery { auto.div_ceil(2) } else { auto }
    }
    /// The frame cap battery mode adds: 60 in a world, 30 in menus without a
    /// world or while another window has focus.
    pub fn battery_limit(&self, limit: i32, in_world: bool, focused: bool) -> i32 {
        if !self.battery {
            limit
        } else if in_world && focused {
            limit.min(60)
        } else {
            limit.min(30)
        }
    }
}

/// A preset's values for vanilla's options and Horizon's own.
pub fn apply_preset(preset: Preset, s: &mut crate::settings::Settings) {
    let (rd, sim, clouds, particles, shadows, blend, mipmaps, entity, leaves, weather_r) =
        match preset {
            Preset::Performance => (8, 6, "false", 2, false, 0, 0, 0.5, false, 3),
            Preset::Balanced => (12, 8, "fast", 1, true, 1, 2, 0.75, true, 5),
            Preset::Quality => (16, 12, "true", 0, true, 2, 4, 1.0, true, 10),
            Preset::Ultra => (24, 12, "true", 0, true, 3, 4, 1.5, true, 10),
            Preset::Custom => return,
        };
    s.render_distance = rd;
    s.simulation_distance = sim;
    s.clouds = clouds.into();
    s.particles = particles;
    s.entity_shadows = shadows;
    s.biome_blend = blend;
    s.mipmap_levels = mipmaps;
    s.entity_distance = entity;
    s.cutout_leaves = leaves;
    s.weather_radius = weather_r;
    s.graphics_preset = "custom".into();
    let g = &mut s.horizon;
    g.preset = preset;
    g.cave_culling = true;
    g.animated_textures = true;
    g.chunk_updates = 0;
    g.upload_budget_kib = 0;
    g.sky = true;
    g.fog = true;
    g.weather = true;
}

/// A phone's first start: settings for its tier instead of the desktop's (measured on a
/// Snapdragon 8 Gen 2 at 3200x1440; slower tiers scaled down from it).
///
/// The frame cap is 60: a steady 60 runs cooler than 70 to 90 that dips, and the display
/// follows the cap. Mobs think on the render thread, so the simulation distance stays short:
/// at 12 the ticks of a few hundred mobs made every third frame late. Clouds cost the GPU
/// 1.7 ms of a frame at any distance, fancy leaves are overdraw on tiled GPUs, and animated
/// textures are mixed on the CPU each frame.
pub fn apply_mobile_defaults(tier: crate::system_stats::Tier, s: &mut crate::settings::Settings) {
    use crate::system_stats::Tier;
    let high = tier == Tier::High;
    s.max_fps = 60;
    s.render_distance = match tier {
        Tier::High => 10,
        Tier::Mid => 8,
        Tier::Low => 6,
    };
    s.simulation_distance = if high { 8 } else { 6 };
    s.clouds = if high { "fast" } else { "false" }.into();
    s.particles = if high { 1 } else { 2 };
    s.entity_shadows = high;
    s.biome_blend = if high { 1 } else { 0 };
    s.mipmap_levels = if high { 4 } else { 2 };
    s.entity_distance = match tier {
        Tier::High => 1.0,
        Tier::Mid => 0.75,
        Tier::Low => 0.5,
    };
    s.cutout_leaves = high;
    s.weather_radius = if high { 5 } else { 3 };
    // Bedrock's touch default: the stick walks up a block without the jump button.
    s.auto_jump = true;
    s.graphics_preset = "custom".into();
    let g = &mut s.horizon;
    g.preset = Preset::Custom;
    g.resolution_scale = match tier {
        Tier::High => 100,
        Tier::Mid => 85,
        Tier::Low => 70,
    };
    g.animated_textures = tier != Tier::Low;
    // A phone's GPU is an integrated one, which far terrain slows most.
    g.far_detail = if high {
        FarDetail::Full
    } else {
        FarDetail::Reduced
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones_start_lighter_by_tier() {
        use crate::system_stats::Tier;
        let mut low = crate::settings::Settings::default();
        apply_mobile_defaults(Tier::Low, &mut low);
        let mut high = crate::settings::Settings::default();
        apply_mobile_defaults(Tier::High, &mut high);
        assert_eq!((low.max_fps, high.max_fps), (60, 60));
        assert!(low.render_distance < high.render_distance);
        assert_eq!(
            (low.clouds.as_str(), high.clouds.as_str()),
            ("false", "fast")
        );
        assert!(low.horizon.resolution_scale < high.horizon.resolution_scale);
        assert!(!low.horizon.animated_textures && high.horizon.animated_textures);
        // Auto-Jump on, as on Bedrock's touch screens.
        assert!(low.auto_jump && high.auto_jump);
        // What the options screens read back.
        let file = high.to_file();
        let back = crate::settings::Settings::from_file(file);
        assert_eq!(back.render_distance, 10);
        assert!(back.auto_jump);
    }

    #[test]
    fn round_trips() {
        let g = Graphics {
            preset: Preset::Balanced,
            lod_distance: 256,
            far_detail: FarDetail::Reduced,
            cave_culling: false,
            entity_culling: true,
            chunk_updates: 12,
            upload_budget_kib: 4096,
            sky: false,
            fog: false,
            weather: false,
            animated_textures: false,
            present_mode: PresentMode::Mailbox,
            pacing: Pacing::Precise,
            workers: 6,
            overlay: Overlay::Graph,
            resolution_scale: 75,
            fxaa: true,
            hdr: true,
            hdr_paper_white: 240,
            hdr_peak: 1400,
            battery: true,
            api: GraphicsApi::Dx12,
            shader_pack: "Golden Hour".into(),
            shader_profile: "LOW".into(),
        };
        assert_eq!(Graphics::parse(&g.serialize()), g);
        assert_eq!(Graphics::parse(""), Graphics::default());
        // The old menus' Simple / Advanced choice is ignored, and so is the
        // shader pack older versions saved.
        assert_eq!(Graphics::parse("level:advanced\n"), Graphics::default());
        assert_eq!(
            Graphics::parse("shaderPack:Pack.zip\n"),
            Graphics::default()
        );
        assert_eq!(
            Graphics::parse("workers:abc\nsky:maybe\ncullLeaves:true\nentityCulling:maybe\n"),
            Graphics::default()
        );
    }

    #[test]
    fn integrated_gpus_other_than_apple_cap_the_lod_distance() {
        let adapter = |name: &str, device_type| wgpu::AdapterInfo {
            name: name.into(),
            vendor: 0,
            device: 0,
            device_type,
            device_pci_bus_id: String::new(),
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
            subgroup_min_size: 0,
            subgroup_max_size: 0,
            transient_saves_memory: None,
            limit_bucket: None,
        };
        let hd620 = adapter(
            "Intel(R) HD Graphics 620 (KBL GT2)",
            wgpu::DeviceType::IntegratedGpu,
        );
        let m4 = adapter("Apple M4 Pro", wgpu::DeviceType::IntegratedGpu);
        let discrete = adapter("NVIDIA GeForce RTX 4070", wgpu::DeviceType::DiscreteGpu);
        for (distance, capped) in [(0, 0), (64, 64), (128, 128), (256, 128), (1024, 128)] {
            let g = Graphics {
                lod_distance: distance,
                ..Graphics::default()
            };
            assert_eq!(g.lod_distance_on(&hd620), capped);
            assert_eq!(g.lod_distance_on(&m4), distance);
            assert_eq!(g.lod_distance_on(&discrete), distance);
        }
        assert!(
            crate::graphics_ui::rows(crate::graphics_ui::Level::Simple)
                .iter()
                .any(|row| matches!(row, crate::graphics_ui::Row::Setting(s)
                    if s.text.contains(&format!("{INTEGRATED_LOD_DISTANCE} chunks")))),
            "the LOD Distance description names the cap"
        );
    }

    #[test]
    fn lod_distance_is_opt_in_and_bounded() {
        assert_eq!(Graphics::default().lod_distance, 0);
        for distance in [0, 64, 128, 256, 512, 1024] {
            let g = Graphics::parse(&format!("lodDistance:{distance}\n"));
            assert_eq!(g.lod_distance, distance);
            assert_eq!(Graphics::parse(&g.serialize()), g);
        }
        assert_eq!(Graphics::parse("lodDistance:10000\n").lod_distance, 1024);
        assert_eq!(Graphics::parse("lodDistance:-1\n").lod_distance, 0);
        assert_eq!(Graphics::parse("lodDistance:invalid\n").lod_distance, 0);
    }

    #[test]
    fn far_detail_is_full_unless_reduced_is_picked() {
        assert_eq!(Graphics::default().far_detail, FarDetail::Full);
        assert_eq!(
            Graphics::parse("farDetail:reduced\n").far_detail,
            FarDetail::Reduced
        );
        for text in ["farDetail:full\n", "farDetail:other\n", ""] {
            assert_eq!(Graphics::parse(text).far_detail, FarDetail::Full);
        }
        // No preset changes it.
        for preset in Preset::ALL {
            let mut s = crate::settings::Settings::default();
            apply_preset(preset, &mut s);
            assert_eq!(s.horizon.far_detail, FarDetail::Full, "{preset:?}");
        }
    }

    #[test]
    fn every_preset_keeps_the_animated_textures_on() {
        // The Performance preset used to turn them off, which froze the fire,
        // the water, the lava and the portals (#2064).
        assert!(Graphics::default().animated_textures);
        for preset in Preset::ALL {
            let mut s = crate::settings::Settings::default();
            apply_preset(preset, &mut s);
            assert!(
                s.horizon.animated_textures,
                "{preset:?} turned the animated textures off"
            );
        }
    }

    #[test]
    fn automatic_workers_leave_a_core_as_vanilla_does() {
        let g = Graphics::default();
        assert_eq!(g.workers, 0);
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        assert_eq!(g.worker_threads(), cores.saturating_sub(1).max(1));
        // A manual count is the player's own choice, kept as asked.
        let manual = Graphics { workers: 3, ..g };
        assert_eq!(manual.worker_threads(), 3);
    }

    #[test]
    fn battery_mode_only_lowers_the_frame_cap() {
        let mut g = Graphics::default();
        assert_eq!(g.battery_limit(260, true, true), 260);
        g.battery = true;
        assert_eq!(g.battery_limit(260, true, true), 60);
        assert_eq!(g.battery_limit(30, true, true), 30);
        assert_eq!(g.battery_limit(260, false, true), 30);
        assert_eq!(g.battery_limit(260, true, false), 30);
        assert_eq!(g.battery_limit(10, true, false), 10);
    }
}
