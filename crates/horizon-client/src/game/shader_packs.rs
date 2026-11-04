//! Shader packs: the one picked on the Shader Packs page, with its profile
//! (`Graphics::shader_choice`; `HORIZON_SHADER_PACK` for development), runs `shaderpacks/<name>`
//! (a folder or a zip beside `options.txt`) over the world; Horizon's own packs go to
//! `game/native_shaders.rs`. The programs of the current dimension's folder are loaded when the
//! dimension or the choice changes (spec 1.2).

use horizon_client::camera::CameraFluid;
use horizon_client::gfx::pack_uniforms::{PackInputs, PlayerStatus};
use horizon_client::graphics::{AUTO_PROFILE, light_shader_pack};
use horizon_client::shader_options::ShaderOptions;
use horizon_client::text::Text;
use horizon_shaders::{BlockIds, Macros, Pack, Properties, dimension_folder, load_post};

use super::{Dim, Game};
use horizon_client::gfx::pack_draw::EntitySpan;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

/// The values the player chose for pack `name`'s options (`shaderpacks/<name>.txt`).
fn chosen_shader_options(name: &str) -> BTreeMap<String, String> {
    let file = Game::shader_packs_dir().join(horizon_shaders::chosen_file(name));
    std::fs::read_to_string(file)
        .map(|text| horizon_shaders::parse_chosen(&text))
        .unwrap_or_default()
}

/// Pack `name`'s options, screens and names in `language`, with the values the player chose. An
/// option's default is what `profile` runs it at, so the page shows what runs.
pub(super) fn read_shader_options(
    name: &str,
    profile: &str,
    language: &str,
    macros: &Macros,
) -> Option<ShaderOptions> {
    let pack = match Pack::open(&Game::shader_packs_dir().join(name)) {
        Ok(pack) => pack,
        Err(e) => {
            log::warn!("shader pack {name}: {e}");
            return None;
        }
    };
    let mut options = horizon_shaders::options(&pack);
    let lang = horizon_shaders::Lang::load(&pack, language);
    let chosen = chosen_shader_options(name);
    let pack = pack.with_chosen(chosen.clone());
    let text = pack.read_text("shaders.properties").unwrap_or_default();
    let properties = Properties::parse(&text, &horizon_shaders::with_options(&pack, macros));
    let screens = horizon_shaders::Screens::parse(&properties, &options);
    let running = picked_values(&properties, &options, name, profile, &BTreeMap::new());
    for option in options.values_mut() {
        if let Some(value) = running.declared(&option.name, &option.default) {
            option.default = value;
        }
    }
    Some(ShaderOptions::new(
        name.to_owned(),
        options,
        screens,
        lang,
        chosen,
    ))
}

/// Saves the values chosen on the Shader Options page; none chosen removes the file.
pub(super) fn write_shader_options(options: &ShaderOptions) {
    let file = Game::shader_packs_dir().join(horizon_shaders::chosen_file(&options.pack));
    let saved = if options.chosen.is_empty() {
        std::fs::remove_file(&file).or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        })
    } else {
        std::fs::write(&file, horizon_shaders::chosen_text(&options.chosen))
    };
    if let Err(e) = saved {
        log::warn!("shader pack {}: {}: {e}", options.pack, file.display());
    }
}

/// The pack that runs over the world ("": none): `HORIZON_SHADER_PACK`, else the pick
/// (`active_pack`).
fn picked_pack(settings: &horizon_client::settings::Settings) -> &str {
    static DEV_PACK: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    let dev = DEV_PACK.get_or_init(|| std::env::var("HORIZON_SHADER_PACK").ok());
    active_pack(
        dev.as_deref().unwrap_or(&settings.horizon.shader_pack),
        &settings.mods,
    )
}

/// Whether the pack picked (`picked_pack`) draws flat where the device can (`pack_draws_flat`):
/// a renderer made now lays the sections out for it before the first world frame loads it
/// (`Renderer::new`).
pub(super) fn picked_pack_draws_flat(settings: &horizon_client::settings::Settings) -> bool {
    let picked = picked_pack(settings);
    !picked.is_empty() && pack_draws_flat(&Game::shader_packs_dir().join(picked))
}

/// Whether the pack at `path` has passes that draw the terrain flat where the device can,
/// reading translucent quads beside their section's solid ones: a `shaders/` pack, taken to
/// draw terrain, or one of Horizon's own with a `[shadow]` pass.
fn pack_draws_flat(path: &std::path::Path) -> bool {
    !horizon_shader_api::package::is_pack(path)
        || horizon_shader_api::Pack::read(path).is_ok_and(|p| p.manifest.shadow.is_some())
}

// A native mod can bundle a pack as `native-<mod-id>`; its Mods toggle controls it too.
fn active_pack<'a>(picked: &'a str, mods: &horizon_client::mod_list::ModList) -> &'a str {
    if picked.strip_prefix("native-").is_some_and(|id| {
        mods.installed
            .iter()
            .any(|m| m.id == id && m.native.is_some() && mods.disabled.contains(id))
    }) {
        ""
    } else {
        picked
    }
}

/// Whether the pack `settings` pick (or `HORIZON_SHADER_PACK`) is one of Horizon's own: only
/// then do resource loads read the resource packs' LabPBR maps (`sprites::set_labpbr_maps`).
pub(super) fn picks_native_pack(settings: &horizon_client::settings::Settings) -> bool {
    let dev = std::env::var("HORIZON_SHADER_PACK").ok();
    let picked = active_pack(
        dev.as_deref().unwrap_or(&settings.horizon.shader_pack),
        &settings.mods,
    );
    !picked.is_empty()
        && horizon_shader_api::package::is_pack(&Game::shader_packs_dir().join(picked))
}

/// The ids a pack gives entities and block entities (4.1, 5.3), for the spans of the entity
/// geometry.
pub(super) struct PackIds {
    entities: HashMap<String, i32>,
    blocks: BlockIds,
    /// `blockEntityId` by block state, filled as states are drawn.
    block_states: RefCell<HashMap<u32, i32>>,
    /// The id of a name the file lacks: 65535, or 0 without the file.
    missing_entity: i32,
    missing_block: i32,
    /// `item.properties` (4.1) and the id of a held item the file lacks, or of an empty hand:
    /// −1, or 0 without the file (5.3).
    items: HashMap<String, i32>,
    missing_item: i32,
    /// `oldHandLight` (3.2): the first held item and light are those of the brighter hand.
    old_hand_light: bool,
    /// `vignette`: `false` hides the game's vignette over the pack's image.
    vignette: bool,
    /// `oldLighting`: `true` keeps the game's face shading in the block colours.
    old_lighting: bool,
    /// `separateAo`: `true` passes AO factor in vertex alpha and keeps pure tint in RGB.
    pub(super) separate_ao: bool,
}

/// A pack the renderer is making on its own thread (`Renderer::start_shader_pack`): its ids,
/// which entity spans use once it draws, and what the log says of it then.
pub(super) struct Making {
    ids: PackIds,
    folder: String,
    started: std::time::Instant,
}

/// What the Auto profile does to a pack on this device. Packs written for a PC spend most of a
/// phone's frame on their pixels, water and shadow map, so it takes one of the pack's own light
/// profiles (Complementary on a Snapdragon 8 Gen 2 at distance 10: 46 ms of GPU on LOW, 28 ms on
/// VERYLOW), limits the shadow and the resolution, and has terrain draw its depth first. Which
/// profile is lightest is read from what the profiles set (`Values::profiles_by_cost`), not from
/// their names, so any pack that has profiles works.
struct AutoProfile {
    /// Place among the pack's profiles, lightest first. A strong phone skips the very lightest
    /// when the pack has three or more (a profile that turns everything off looks like none).
    rank: usize,
    shadow_resolution: f64,
    shadow_distance: f64,
}

impl AutoProfile {
    fn here() -> Self {
        use horizon_client::system_stats::Tier;
        match Tier::here().unwrap_or(Tier::High) {
            Tier::High => Self {
                rank: 1,
                shadow_resolution: 1024.0,
                shadow_distance: 64.0,
            },
            Tier::Mid => Self {
                rank: 0,
                shadow_resolution: 1024.0,
                shadow_distance: 64.0,
            },
            Tier::Low => Self {
                rank: 0,
                shadow_resolution: 512.0,
                shadow_distance: 64.0,
            },
        }
    }
}

/// The pack's option values: the chosen `profile` (`LOW`), then what the player chose on the
/// Shader Options page, which Auto's limits leave alone, then `HORIZON_SHADER_OPTIONS` lists
/// values over it (`shadowDistance=64.0 SHADOW_QUALITY=0 !BLOOM`).
fn picked_values(
    properties: &Properties,
    options: &BTreeMap<String, horizon_shaders::PackOption>,
    name: &str,
    profile: &str,
    chosen: &BTreeMap<String, String>,
) -> horizon_shaders::Values {
    let mut values = horizon_shaders::Values::default();
    if profile == AUTO_PROFILE {
        let auto = AutoProfile::here();
        let ranked = horizon_shaders::Values::profiles_by_cost(properties, options);
        let at = if ranked.len() > 2 { auto.rank } else { 0 };
        match ranked.get(at) {
            Some((found, cost)) => {
                log::info!(
                    "shader pack {name}: Auto picks profile `{found}` (cost {cost:.1}) of {}",
                    ranked
                        .iter()
                        .map(|(n, c)| format!("{n} {c:.1}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                values = horizon_shaders::Values::profile(properties, found).unwrap_or_default();
            }
            None => log::info!("shader pack {name}: Auto finds no profile to start from"),
        }
        for (option, limit) in [
            ("shadowMapResolution", auto.shadow_resolution),
            ("shadowDistance", auto.shadow_distance),
        ] {
            if !chosen.contains_key(option) {
                values.cap(option, limit);
            }
        }
    } else if !profile.is_empty() {
        match horizon_shaders::Values::profile(properties, profile) {
            Some(p) => values = p,
            None => log::warn!("shader pack {name}: no profile `{profile}`"),
        }
    }
    for (option, value) in chosen {
        values.set(option, value);
    }
    if let Ok(list) = std::env::var("HORIZON_SHADER_OPTIONS") {
        values.parse_list(&list);
    }
    if !values.is_empty() {
        log::info!("shader pack {name}: option values {values:?}");
    }
    values
}
impl Game<'_> {
    /// Loads the pack for the current dimension when it changed, and returns this frame's
    /// inputs to the pack's uniforms.
    pub(super) fn shader_pack_inputs(
        &mut self,
        camera: glam::DVec3,
        partial: f32,
        sky: &horizon_client::environment::Sky,
    ) -> PackInputs {
        // A `shaders/` pack lights the faces itself unless `oldLighting=true`: sections mesh
        // without the game's face shading (the pack loaded or dropped below shows next frame).
        let pack_lighting = self.pack_ids.as_ref().is_some_and(|ids| !ids.old_lighting);
        if self.streaming.shader_pack_lighting != pack_lighting {
            self.streaming.shader_pack_lighting = pack_lighting;
            self.all_sections_changed();
        }
        let separate_ao = self.pack_ids.as_ref().is_some_and(|ids| ids.separate_ao);
        if self.streaming.separate_ao != separate_ao {
            self.streaming.separate_ao = separate_ao;
            self.all_sections_changed();
        }
        // A new choice, once the options close (each load takes seconds): what ran for the old
        // one goes, of either kind.
        let choice = self
            .settings
            .horizon
            .shader_choice()
            .filter(|(name, _)| !active_pack(name, &self.settings.mods).is_empty());
        if choice != self.shader_choice && self.screen != super::Screen::Options {
            self.shader_choice = choice;
            self.native_shaders = Default::default();
            self.set_translucent_layers(Vec::new());
            self.pack_ids = None;
            self.pack_making = None;
            let scale = self.world_scale();
            let horizon = &self.settings.horizon;
            let (antialias, taa, upscale) = (horizon.fxaa, horizon.taa, horizon.upscaling.ratio());
            if let Some(renderer) = &mut self.renderer {
                renderer.drop_shader_packs();
                renderer.set_world_post(scale, antialias, taa, upscale);
            }
        }
        let Some((name, profile)) = self.shader_choice.clone() else {
            return PackInputs::default();
        };
        let dimension = match self.dim {
            Dim::Overworld => 0,
            Dim::Nether => -1,
            Dim::End => 1,
        };
        let path = Self::shader_packs_dir().join(&name);
        if horizon_shader_api::package::is_pack(&path) {
            // One of Horizon's own packs (game/native_shaders.rs).
            self.tick_native_shader_pack(&path);
        } else if let Some(renderer) = &mut self.renderer
            && renderer.shader_pack_dimension() != Some(dimension)
        {
            let name = name.as_str();
            let started = std::time::Instant::now();
            self.pack_ids = None;
            self.pack_making = None;
            // A pack is not the game's own code: one that makes the shader compiler (or our
            // translation of it) panic is turned off instead of crashing every world load (#3525).
            let loaded = horizon_client::crash::contain(|| {
                match Pack::open(&path).map(|p| p.with_chosen(chosen_shader_options(name))) {
                    Ok(mut pack) => {
                        let mut macros = Macros::standard();
                        renderer.define_pack_macros(&mut macros);
                        // Its conditionals test the pack's options too.
                        let (mut properties, notes) =
                            horizon_shaders::read_properties(&pack, &mut macros);
                        for note in notes {
                            log::warn!("shader pack {name}: {note}");
                        }
                        for note in &properties.notes {
                            log::warn!(
                                "shader pack {name}: shaders.properties:{}: {}",
                                note.line,
                                note.message
                            );
                        }
                        let chosen = chosen_shader_options(name);
                        let values = picked_values(
                            &properties,
                            &horizon_shaders::options(&pack),
                            name,
                            &profile,
                            &chosen,
                        );
                        if !values.is_empty() {
                            pack.set_values(values);
                            // A profile can switch on what shaders.properties tests (`image.*`,
                            // `program.*.enabled`, features): read it again with the values in,
                            // or the stage files and the properties disagree about the options.
                            macros = Macros::standard();
                            renderer.define_pack_macros(&mut macros);
                            properties = horizon_shaders::read_properties(&pack, &mut macros).0;
                        }
                        let separate_ao = properties
                            .get("separateAo")
                            .is_some_and(|v| v.trim() == "true");
                        renderer.set_separate_ao(separate_ao);
                        let folder = dimension_folder(&pack, dimension);
                        // On a phone whatever the profile: it pays for itself even on a pack's
                        // highest settings, and the player picks a profile for how it looks.
                        renderer.set_pack_prepass(
                            (cfg!(target_os = "android") || profile == AUTO_PROFILE)
                                && !light_shader_pack(name),
                        );
                        let inputs = renderer.pack_inputs();
                        let biomes: Vec<String> = world_biomes!(self)
                            .list
                            .iter()
                            .map(|b| b.name.clone())
                            .collect();
                        let resources = self.options.resources.clone();
                        let text = pack.read_text("block.properties").unwrap_or_default();
                        let ids = BlockIds::parse(&Properties::parse(&text, &macros));
                        for note in &ids.notes {
                            log::warn!("shader pack {name}: block.properties: {note}");
                        }
                        let text = pack.read_text("entity.properties");
                        let entities = BlockIds::parse_kind(
                            &Properties::parse(text.as_deref().unwrap_or_default(), &macros),
                            "entity",
                        );
                        let text = pack.read_text("item.properties");
                        let items = BlockIds::parse_kind(
                            &Properties::parse(text.as_deref().unwrap_or_default(), &macros),
                            "item",
                        );
                        let pack_ids = PackIds {
                            items: items.plain(),
                            missing_item: if text.is_some() { -1 } else { 0 },
                            // OQ-21: on unless the pack turns it off.
                            old_hand_light: properties
                                .get("oldHandLight")
                                .is_none_or(|v| v.trim() != "false"),
                            // On unless the pack turns it off (`vignette=false`).
                            vignette: properties
                                .get("vignette")
                                .is_none_or(|v| v.trim() != "false"),
                            // Off unless the pack turns it on (Iris shades no faces by default).
                            old_lighting: properties
                                .get("oldLighting")
                                .is_some_and(|v| v.trim() == "true"),
                            separate_ao,
                            entities: entities.plain(),
                            missing_entity: if text.is_some() { 65535 } else { 0 },
                            missing_block: if ids.is_empty() { 0 } else { 65535 },
                            blocks: ids.clone(),
                            block_states: RefCell::default(),
                        };
                        let started_ids = std::time::Instant::now();
                        let table = terrain_table(
                            &block_id_table(&self.pack, &ids),
                            &block_emission_table(&self.pack),
                        );
                        log::info!(
                            "shader pack {name}: mc_Entity for {} of {} block sprites in {} ms",
                            table.chunks(3).filter(|p| p[0] != -1).count(),
                            table.len() / 3,
                            started_ids.elapsed().as_millis()
                        );
                        // The programs are translated and built on the renderer's thread for
                        // packs; the world draws without the pack until they are.
                        let program_folder = folder.clone();
                        let pack_name = name.to_owned();
                        renderer.start_shader_pack(
                            dimension,
                            move || {
                                let mut post = load_post(
                                    &pack,
                                    &program_folder,
                                    &properties,
                                    &macros,
                                    &inputs,
                                );
                                // Textures that name one of the game's: from the resource packs
                                // over the game's own resources.
                                let read = |namespace: &str, path: &str| {
                                    std::fs::read(
                                        resources.join("assets").join(namespace).join(path),
                                    )
                                    .ok()
                                };
                                if !post.skipped.is_empty() {
                                    let fell: Vec<&str> =
                                        post.skipped.iter().map(|s| s.name.as_str()).collect();
                                    let uses = |p: &Option<horizon_shaders::PostProgram>| {
                                        p.as_ref().map_or("none", |p| p.name.as_str()).to_owned()
                                    };
                                    log::warn!(
                                        "shader pack {pack_name}: {} programs did not load ({}); \
                                         terrain draws with {}, water with {}, entities with {}, \
                                         a composite pass that fell is skipped",
                                        fell.len(),
                                        fell.join(" "),
                                        uses(&post.terrain),
                                        uses(&post.water),
                                        uses(&post.entities),
                                    );
                                }
                                for note in
                                    horizon_shaders::resolve_resources(&mut post.textures, &read)
                                {
                                    log::warn!("shader pack {pack_name}: {note}");
                                }
                                let names: Vec<&str> = biomes.iter().map(String::as_str).collect();
                                post.custom.set_biomes(&names);
                                post
                            },
                            table,
                        );
                        self.pack_making = Some(Making {
                            ids: pack_ids,
                            folder,
                            started,
                        });
                    }
                    Err(e) => {
                        log::warn!("shader pack {name}: {e}");
                        renderer.set_shader_pack(dimension, None, &[]);
                    }
                }
            });
            if let Err(panic) = loaded {
                self.shader_pack_failed(name, dimension, &panic);
            }
        }
        if let Some(made) = self.renderer.as_mut().and_then(|r| r.poll_shader_pack())
            && let Some(making) = self.pack_making.take()
        {
            match made {
                Ok((programs, notes)) => {
                    for note in notes {
                        log::warn!("shader pack {name}: {note}");
                    }
                    log::info!(
                        "shader pack {name}: folder `{}`, {} programs ({}) in {} ms, made \
                         beside the game",
                        making.folder,
                        programs.len(),
                        programs.join(" "),
                        making.started.elapsed().as_millis()
                    );
                    // None could be made (the notes above say why): the world would stay as it
                    // is with the pack still "on".
                    if programs.is_empty() {
                        self.shader_pack_failed(&name, dimension, "none of its programs loaded");
                    } else {
                        self.pack_ids = Some(making.ids);
                    }
                }
                Err(e) => self.shader_pack_failed(&name, dimension, &e),
            }
        }
        let lerp = |(old, new): (f32, f32)| old + (new - old) * partial;
        let at = camera.floor().as_ivec3();
        let light = self.world.light(at.x, at.y, at.z);
        let biome = self
            .world
            .chunks
            .get(&(at.x >> 4, at.z >> 4))
            .map(|c| c.biome(at.x, at.y, at.z));
        let (held_items, held_light) = self.held_for_pack();
        let status = self.pack_player_status();
        let (end_flash, end_flash_direction) = self.end_flash_for_pack(partial);
        let info = biome.map(|b| world_biomes!(self).get(b));
        PackInputs {
            day_time: self.world.day_time,
            rain: lerp(self.world.weather.rain_level),
            thunder: lerp(self.world.weather.thunder_level),
            eye_in: match self.camera_fluid(camera.to_array()) {
                CameraFluid::None => 0,
                CameraFluid::Water => 1,
                CameraFluid::Lava => 2,
                CameraFluid::PowderSnow => 3,
            },
            blindness: f32::from(u8::from(self.potions.effects.get("blindness").is_some())),
            eye_light: [i32::from(light & 15), i32::from(light >> 4)],
            hide_gui: self.ui.hide_gui,
            brightness: self.settings.gamma,
            biome: biome.map_or(-1, i32::from),
            biome_known: biome.is_some(),
            biome_category: info.map_or(0, |b| horizon_shaders::biome_category(&b.name)),
            biome_precipitation: info.map_or(0, |b| {
                horizon_shaders::biome_precipitation(b.precipitation, b.temperature)
            }),
            temperature: info.map_or(0.0, |b| b.temperature),
            rainfall: info.map_or(0.0, |b| b.downfall),
            held_items,
            held_light,
            right_handed: !self.settings.main_hand_left,
            on_ground: self.player.as_ref().is_some_and(|p| p.ground.on_ground),
            min_y: self.world.shape.min_y,
            height: self.world.shape.height,
            logical_height: self.world.shape.logical_height,
            has_skylight: self.world.shape.sky_light,
            has_ceiling: self.dim == Dim::Nether,
            elytra_flying: self.player.as_ref().is_some_and(|p| p.fall_flying),
            cloud_height: horizon_client::gfx::pack_uniforms::cloud_height(
                sky.cloud_color[3],
                sky.cloud_height,
            ),
            // Vanilla's drift in float, times its 0.030000001F blocks a tick.
            cloud_time: horizon_client::clouds::drift(
                self.world.game_time,
                f64::from(partial),
                self.pack.clouds.0,
            ) as f32
                * 0.030_000_001,
            mod_params: self.native_shader_params_now(partial),
            status,
            end_flash,
            end_flash_direction,
        }
    }

    /// The local player's state for Iris's status uniforms, read as the HUD reads it.
    fn pack_player_status(&mut self) -> PlayerStatus {
        let spectator = self.spectator();
        let survival = !self.creative && !spectator;
        let invisible = self.own_invisible();
        let riding = self.riding.is_some();
        let Some(p) = &mut self.player else {
            return PlayerStatus::default();
        };
        PlayerStatus {
            survival,
            health: p.health.health(),
            max_health: p.attribute("minecraft:max_health").unwrap_or(20.0) as f32,
            food: p.health.food.food as f32,
            air: p.air as f32,
            armor: p
                .attribute("minecraft:armor")
                .or_else(|_| p.attribute("armor"))
                .unwrap_or(0.0) as f32,
            spectator,
            riding,
            feet_in_water: p.fluids.was_touching_water,
            swimming: p.pose == horizon_client::pose::Pose::Swimming,
            burning: p.fire_ticks > 0 && !spectator,
            hurt: p.health.hurt_animation_time > 0,
            invisible,
            sneaking: p.pose == horizon_client::pose::Pose::Crouching,
            sprinting: p.sprinting,
        }
    }

    /// The world's resolution, of the screen's: the player's, and no more than Auto's while it
    /// runs a pack that isn't a light one.
    pub(super) fn world_scale(&self) -> f32 {
        let chosen = self.settings.horizon.resolution_scale as f32 / 100.0;
        match &self.shader_choice {
            Some((name, profile))
                if profile == AUTO_PROFILE
                    && !light_shader_pack(name)
                    && !horizon_shader_api::package::is_pack(
                        &Self::shader_packs_dir().join(name),
                    ) =>
            {
                chosen.min(horizon_client::graphics::auto_shader_scale())
            }
            _ => chosen,
        }
    }

    /// The Shader Options page opened: the picked pack's options, screens and names.
    pub(super) fn open_shader_options(&mut self) {
        let Some((name, profile)) = self.settings.horizon.shader_choice() else {
            return;
        };
        let mut macros = Macros::standard();
        if let Some(renderer) = &self.renderer {
            renderer.define_pack_macros(&mut macros);
        }
        self.settings.shader_options =
            read_shader_options(&name, &profile, &self.settings.language, &macros);
    }

    /// The Shader Options page closed with changes: they are saved, and the pack loads again.
    pub(super) fn save_shader_options(&mut self) {
        let Some(options) = self.settings.shader_options.take() else {
            return;
        };
        write_shader_options(&options);
        self.pack_ids = None;
        self.pack_making = None;
        if let Some(renderer) = &mut self.renderer {
            renderer.drop_shader_packs();
        }
    }

    /// Turns off the pack `name` after loading it panicked: nothing runs in its place, it is no
    /// longer the picked pack (so the next start does not load it again), and the chat says so.
    fn shader_pack_failed(&mut self, name: &str, dimension: i32, error: &str) {
        log::error!("shader pack {name}: turned off: {error}");
        self.pack_ids = None;
        self.pack_making = None;
        if let Some(renderer) = &mut self.renderer {
            // Marks this dimension as loaded, without a pack, so it is not tried again.
            renderer.set_shader_pack(dimension, None, &[]);
        }
        if self.settings.horizon.shader_pack == name {
            self.settings.horizon.shader_pack.clear();
            self.settings.save(&Self::options_path());
        }
        self.system_message(Text::colored(
            format!("Shader pack {name} could not be loaded and was turned off"),
            0xFF5555,
        ));
    }

    /// The device was lost with a pack picked: most likely its frames were too long for the
    /// driver, so the rebuilt renderer starts without it and it stays off at the next start.
    pub(super) fn shader_pack_lost_device(&mut self) {
        if self.settings.horizon.shader_pack.is_empty() {
            return;
        }
        let name = std::mem::take(&mut self.settings.horizon.shader_pack);
        log::error!("shader pack {name}: turned off after the graphics device was lost");
        self.pack_ids = None;
        self.pack_making = None;
        self.settings.save(&Self::options_path());
        self.system_message(Text::colored(
            format!("Shader pack {name} was too heavy for this device and was turned off"),
            0xFF5555,
        ));
    }

    /// Whether the game's vignette draws: the option, unless the loaded pack turns it off.
    pub(super) fn vignette_shown(&self) -> bool {
        self.settings.vignette && self.pack_ids.as_ref().is_none_or(|ids| ids.vignette)
    }

    /// `heldItemId(2)` and `heldBlockLightValue(2)`: main hand, then off hand (5.2, 5.3).
    fn held_for_pack(&mut self) -> ([i32; 2], [i32; 2]) {
        if self.pack_ids.is_none() {
            return ([0; 2], [0; 2]);
        }
        let lum = self.luminance();
        let Some(ids) = &self.pack_ids else {
            return ([0; 2], [0; 2]);
        };
        let hands = [self.selected, super::shield::OFFHAND_SLOT].map(|slot| {
            let item = self.stack_in(slot).map(|s| &*s.item);
            let id = item.and_then(|i| ids.items.get(i).copied());
            (
                id.unwrap_or(ids.missing_item),
                item.map_or(0, |i| i32::from(lum.of(i))),
            )
        });
        let first = if ids.old_hand_light && hands[1].1 > hands[0].1 {
            hands[1]
        } else {
            hands[0]
        };
        ([first.0, hands[1].0], [first.1, hands[1].1])
    }
}

impl Game<'_> {
    /// Starts a span of the entity geometry at vertex `start` for entity type `name`
    /// (`minecraft:zombie`; "" for one the pack cannot name). Nothing without a pack.
    pub(super) fn mark_entity(&self, start: usize, name: &str) {
        self.mark_entity_part(start, name, false);
    }

    /// Starts a span for entity geometry that blends (name tags, shadows).
    pub(super) fn mark_translucent_entity(&self, start: usize, name: &str) {
        self.mark_entity_part(start, name, true);
    }

    fn mark_entity_part(&self, start: usize, name: &str, translucent: bool) {
        let id = match &self.pack_ids {
            Some(ids) => ids
                .entities
                .get(name)
                .copied()
                .unwrap_or(ids.missing_entity),
            None if native_spans() => name_id(name),
            None => return,
        };
        self.push_span(start, "ENTITIES", id);
        if let Some(last) = self.entity_spans.borrow_mut().last_mut() {
            last.translucent = translucent;
        }
    }

    /// Starts a span for the block entity of block state `state`.
    pub(super) fn mark_block_entity(&self, start: usize, state: u32) {
        let Some(ids) = &self.pack_ids else {
            if native_spans() {
                self.push_span(start, "BLOCK_ENTITIES", state as i32);
            }
            return;
        };
        let id = *ids
            .block_states
            .borrow_mut()
            .entry(state)
            .or_insert_with(|| {
                let registry = self.pack.assets.registry;
                registry
                    .state(horizon_world::registry::BlockStateId(state))
                    .and_then(|s| {
                        ids.blocks
                            .id(&s.name, |k| s.properties.get(k).map(String::as_str))
                    })
                    .unwrap_or(ids.missing_block)
            });
        self.push_span(start, "BLOCK_ENTITIES", id);
        let emission = self
            .res
            .facts()
            .get(horizon_world::registry::BlockStateId(state))
            .emission;
        if let Some(last) = self.entity_spans.borrow_mut().last_mut() {
            last.emissive = emission > 0;
        }
    }

    /// Starts the span of the local player's model; `hidden` in first person, where only a
    /// pack's shadow pass draws it.
    pub(super) fn mark_player(&self, start: usize, hidden: bool) {
        self.mark_entity(start, "minecraft:player");
        if (self.pack_ids.is_some() || native_spans())
            && let Some(last) = self.entity_spans.borrow_mut().last_mut()
        {
            last.player = true;
            last.hidden = hidden;
        }
    }

    /// Marks vertices `start..end` (one entity) as hidden from the camera, for an entity
    /// Horizon's culling would drop but a pack's shadow pass still draws (`EntitySpan::hide`).
    pub(super) fn hide_spans_from(&self, start: usize, end: usize) -> bool {
        EntitySpan::hide(&mut self.entity_spans.borrow_mut(), start, end)
    }

    /// Starts a span of geometry that is not an entity (`PARTICLES`, `DESTROY`...).
    pub(super) fn mark_stage(&self, start: usize, stage: &str) {
        if self.pack_ids.is_some() || native_spans() {
            self.push_span(start, stage, 0);
        }
    }

    fn push_span(&self, start: usize, stage: &str, id: i32) {
        let mut spans = self.entity_spans.borrow_mut();
        let span = EntitySpan {
            start: start as u32,
            stage: horizon_shaders::render_stage(stage),
            id,
            ..Default::default()
        };
        // A span that drew nothing gives way to the next.
        match spans.last_mut() {
            Some(last) if last.start == span.start => *last = span,
            _ => spans.push(span),
        }
    }
}

/// Spans for a native mod (`native_render::RenderNeeds::entity_spans`), without a pack.
fn native_spans() -> bool {
    horizon_client::gfx::native_render::render_needs().entity_spans
}

/// A span's id for entity type `name` without a pack: stable, so a mod can match runs from one
/// frame to the next.
fn name_id(name: &str) -> i32 {
    // FNV-1a.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in name.bytes() {
        hash = (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193);
    }
    hash as i32
}

/// `mc_Entity` (x, y) of each block sprite layer, as (id, fluid) pairs: terrain vertices carry
/// their sprite layer, not their block. A layer takes the id of the block named like its sprite
/// (`block/oak_leaves` for `oak_leaves`), else the id most blocks drawing it share (ties: the
/// lowest); -1 when no block drawing it has one. y is 1 for the fluid sprites, -1 otherwise.
pub(super) fn block_id_table(pack: &super::PackResources, ids: &BlockIds) -> Vec<i32> {
    use std::collections::{BTreeMap, BTreeSet};
    let layers = pack
        .render
        .sprite_layers
        .values()
        .max()
        .map_or(0, |l| l + 1) as usize;
    let mut table = vec![-1; layers * 2];
    if ids.is_empty() {
        return table;
    }
    let registry = pack.assets.registry;
    let catalog = &pack.render.model_catalog;
    let sprite = |name: &str| pack.render.sprite_layers.get(name).copied();
    // (layer, id) -> (a block named like the sprite, the blocks).
    let mut votes: BTreeMap<(u32, i32), (bool, BTreeSet<&str>)> = BTreeMap::new();
    for i in 0..registry.len() {
        let state_id = horizon_world::registry::BlockStateId(i as u32);
        let Some(state) = registry.state(state_id) else {
            continue;
        };
        let Some(id) = ids.id(&state.name, |k| state.properties.get(k).map(String::as_str)) else {
            continue;
        };
        let block = state.name.strip_prefix("minecraft:").unwrap_or(&state.name);
        let mut vote = |layer: u32| {
            let named = pack
                .layer_names
                .get(&layer)
                .and_then(|n| n.strip_prefix("block/"))
                == Some(block);
            let v = votes.entry((layer, id)).or_default();
            v.0 |= named;
            v.1.insert(&state.name);
        };
        // Fluids draw from their own sprites.
        let fluid = match block {
            "water" => ["water_still", "water_flow"],
            "lava" => ["lava_still", "lava_flow"],
            _ => ["", ""],
        };
        for f in fluid.iter().filter(|f| !f.is_empty()) {
            if let Some(l) = sprite(&format!("minecraft:block/{f}")) {
                vote(l);
            }
        }
        let origin = horizon_core::position::BlockPos::new(0, 0, 0);
        for (_, model) in catalog.collect(state_id, origin).unwrap_or_default() {
            for face in &model.faces {
                vote(face.layer);
            }
        }
    }
    let mut best: BTreeMap<u32, (bool, usize, i32)> = BTreeMap::new();
    for ((layer, id), (named, blocks)) in votes {
        let candidate = (named, blocks.len(), -id);
        let b = best.entry(layer).or_insert(candidate);
        if candidate > *b {
            *b = candidate;
        }
    }
    for (layer, (_, _, id)) in best {
        if let Some(slot) = table.get_mut(layer as usize * 2) {
            *slot = -id;
        }
    }
    for f in ["water_still", "water_flow", "lava_still", "lava_flow"] {
        if let Some(l) = sprite(&format!("minecraft:block/{f}"))
            && let Some(slot) = table.get_mut(l as usize * 2 + 1)
        {
            *slot = 1;
        }
    }
    table
}

/// `at_midBlock.w` of each block sprite layer: the light the block drawing it gives (Iris's
/// `BLOCK_EMISSION_ATTRIBUTE`; 0 to 15, as Iris 1.11.7 passes it). Terrain vertices carry their
/// sprite layer, not their block, so as for `mc_Entity` a layer takes the light of the block
/// named like its sprite, else the light most block states drawing it give (ties: the lowest).
pub(super) fn block_emission_table(pack: &super::PackResources) -> Vec<i32> {
    use std::collections::BTreeMap;
    let layers = pack
        .render
        .sprite_layers
        .values()
        .max()
        .map_or(0, |l| l + 1) as usize;
    let registry = pack.assets.registry;
    let catalog = &pack.render.model_catalog;
    let sprite = |name: &str| pack.render.sprite_layers.get(name).copied();
    // (layer, light) -> (a block named like the sprite draws it, how many states do).
    let mut votes: BTreeMap<(u32, u8), (bool, usize)> = BTreeMap::new();
    for i in 0..registry.len() {
        let state_id = horizon_world::registry::BlockStateId(i as u32);
        let Some(state) = registry.state(state_id) else {
            continue;
        };
        let light = pack
            .context
            .tables
            .facts(state_id)
            .map_or(0, |f| f.emission);
        let block = state.name.strip_prefix("minecraft:").unwrap_or(&state.name);
        let mut vote = |layer: u32| {
            let named = pack
                .layer_names
                .get(&layer)
                .and_then(|n| n.strip_prefix("block/"))
                == Some(block);
            let v = votes.entry((layer, light)).or_default();
            v.0 |= named;
            v.1 += 1;
        };
        let fluid = match block {
            "water" => ["water_still", "water_flow"],
            "lava" => ["lava_still", "lava_flow"],
            _ => ["", ""],
        };
        for f in fluid.iter().filter(|f| !f.is_empty()) {
            if let Some(l) = sprite(&format!("minecraft:block/{f}")) {
                vote(l);
            }
        }
        let origin = horizon_core::position::BlockPos::new(0, 0, 0);
        for (_, model) in catalog.collect(state_id, origin).unwrap_or_default() {
            for face in &model.faces {
                vote(face.layer);
            }
        }
    }
    layer_lights(&votes, layers)
}

/// Each of `layers` sprite layers' light from `votes`, (layer, light) -> (a block named like the
/// sprite draws it, how many states do): the named block's, else the most states', ties the
/// lowest; 0 for a layer no block draws.
fn layer_lights(
    votes: &std::collections::BTreeMap<(u32, u8), (bool, usize)>,
    layers: usize,
) -> Vec<i32> {
    let mut best: std::collections::BTreeMap<u32, (bool, usize, i32)> = Default::default();
    for (&(layer, light), &(named, states)) in votes {
        let candidate = (named, states, -i32::from(light));
        let b = best.entry(layer).or_insert(candidate);
        if candidate > *b {
            *b = candidate;
        }
    }
    let mut table = vec![0; layers];
    for (layer, (_, _, light)) in best {
        if let Some(slot) = table.get_mut(layer as usize) {
            *slot = -light;
        }
    }
    table
}

/// The table a `shaders/` pack's terrain reads per sprite layer: `mc_Entity`'s (id, fluid) pair
/// from `ids` (`block_id_table`), then `at_midBlock.w` from `emission`.
fn terrain_table(ids: &[i32], emission: &[i32]) -> Vec<i32> {
    ids.chunks(2)
        .enumerate()
        .flat_map(|(layer, pair)| [pair[0], pair[1], emission.get(layer).copied().unwrap_or(0)])
        .collect()
}

#[cfg(test)]
mod emission_tests {
    use super::{layer_lights, terrain_table};

    /// A sprite's light: the block named like it gives its own (a torch, 14, over a wall
    /// torch's); a shared one takes what most states drawing it give (a furnace's side: as
    /// many lit states, 13, as unlit, 0: the lowest), and a layer no block draws stays 0.
    #[test]
    fn a_sprite_takes_the_light_of_its_block() {
        let votes = std::collections::BTreeMap::from([
            ((0, 14), (true, 1)),
            ((0, 0), (false, 5)),
            ((1, 13), (false, 4)),
            ((1, 0), (false, 4)),
            ((2, 15), (false, 4)),
            ((2, 0), (false, 5)),
            ((3, 7), (false, 2)),
        ]);
        assert_eq!(layer_lights(&votes, 5), [14, 0, 0, 7, 0]);
    }

    /// The table `shaders/` packs' terrain reads: three values a layer, `mc_Entity`'s pair
    /// then the light.
    #[test]
    fn the_terrain_table_holds_ids_then_light() {
        assert_eq!(
            terrain_table(&[10, -1, 20, 1], &[14, 0]),
            [10, -1, 14, 20, 1, 0]
        );
    }
}

#[cfg(test)]
mod native_pack_tests {
    use super::{active_pack, pack_draws_flat};
    use horizon_client::mod_list::{Installed, ModList, NativePage};

    #[test]
    fn shaders_packs_and_packs_with_a_shadow_pass_draw_flat() {
        let root = std::env::temp_dir().join(format!("horizon-flat-packs-{}", std::process::id()));
        // A folder without a manifest is a `shaders/` pack; with one, a pack of Horizon's own.
        let pack = |name: &str, manifest: Option<&str>| {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            if let Some(extra) = manifest {
                std::fs::write(dir.join("material.wgsl"), "").unwrap();
                let text = format!(
                    "schema = 1\nid = \"{name}\"\nname = \"{name}\"\nversion = \"1\"\napi = \"{}\"\n\
                     [material]\nfile = \"material.wgsl\"\n{extra}",
                    horizon_shader_api::API
                );
                std::fs::write(dir.join(horizon_shader_api::MANIFEST), text).unwrap();
            }
            dir
        };
        assert!(pack_draws_flat(&pack("shaders_pack", None)));
        assert!(!pack_draws_flat(&pack("unshadowed", Some(""))));
        assert!(pack_draws_flat(&pack("shadowed", Some("[shadow]\n"))));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn native_shader_follows_its_mod_without_affecting_other_packs() {
        let mut mods = ModList::default();
        mods.installed.push(Installed {
            id: "horizon_better_visuals".into(),
            title: "Better Visuals".into(),
            description: String::new(),
            problem: None,
            dev: false,
            native: Some(NativePage::default()),
        });
        let owned = "native-horizon_better_visuals";
        assert_eq!(active_pack(owned, &mods), owned);
        mods.disabled.insert("horizon_better_visuals".into());
        assert_eq!(active_pack(owned, &mods), "");
        assert_eq!(active_pack("golden_hour", &mods), "golden_hour");
        mods.installed[0].native = None;
        assert_eq!(active_pack(owned, &mods), owned);
    }
}
