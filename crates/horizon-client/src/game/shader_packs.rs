//! Shader packs: the one picked on the Shader Packs page, with its profile
//! (`Graphics::shader_choice`; `HORIZON_SHADER_PACK` for development), runs `shaderpacks/<name>`
//! (a folder or a zip beside `options.txt`) over the world; Horizon's own packs go to
//! `game/native_shaders.rs`. The programs of the current dimension's folder are loaded when the
//! dimension or the choice changes (spec 1.2).

use horizon_client::camera::CameraFluid;
use horizon_client::gfx::pack_uniforms::PackInputs;
use horizon_client::graphics::AUTO_PROFILE;
use horizon_shaders::{BlockIds, Macros, Pack, Properties, dimension_folder, load_post};

use super::{Dim, Game};
use horizon_client::gfx::pack_draw::EntitySpan;
use std::cell::RefCell;
use std::collections::HashMap;

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
}

/// A pack the renderer is making on its own thread (`Renderer::start_shader_pack`): its ids,
/// which entity spans use once it draws, and what the log says of it then.
pub(super) struct Making {
    ids: PackIds,
    folder: String,
    started: std::time::Instant,
}

/// What the Auto profile does to a pack on this device. Packs written for a PC spend most of a
/// phone's frame on their pixels, water and shadow map, so it takes the pack's own light profile
/// (Complementary on a Snapdragon 8 Gen 2 at distance 10: 46 ms of GPU on LOW, 28 ms on VERYLOW),
/// limits the shadow and the resolution, and has terrain draw its depth first.
struct AutoProfile {
    /// The profiles to look for, the first the pack has wins (`Values::first_profile`).
    profiles: &'static [&'static str],
    shadow_resolution: f64,
    shadow_distance: f64,
    /// The most of the screen's resolution the world draws at (`light_pack`s aside).
    scale: f32,
}

impl AutoProfile {
    fn here() -> Self {
        use horizon_client::system_stats::Tier;
        match Tier::here().unwrap_or(Tier::High) {
            Tier::High => Self {
                profiles: &["verylow", "low", "potato"],
                shadow_resolution: 1024.0,
                shadow_distance: 64.0,
                scale: 0.5,
            },
            Tier::Mid => Self {
                profiles: &["potato", "verylow", "shadowlesslow", "low"],
                shadow_resolution: 1024.0,
                shadow_distance: 64.0,
                scale: 0.4,
            },
            Tier::Low => Self {
                profiles: &["potato", "verylow", "shadowlesslow", "low"],
                shadow_resolution: 512.0,
                shadow_distance: 64.0,
                scale: 0.3,
            },
        }
    }
}

/// A pack made to be cheap, which Auto leaves at the player's resolution: Starling Visuals and
/// MakeUp Ultra Fast.
fn light_pack(name: &str) -> bool {
    let name = name.replace(['-', '_', ' '], "").to_ascii_lowercase();
    name.starts_with("starlingvisuals") || name.starts_with("makeup")
}

/// The pack's option values: the chosen `profile` (`LOW`), then `HORIZON_SHADER_OPTIONS` lists
/// values over it (`shadowDistance=64.0 SHADOW_QUALITY=0 !BLOOM`).
fn picked_values(properties: &Properties, name: &str, profile: &str) -> horizon_shaders::Values {
    let mut values = horizon_shaders::Values::default();
    if profile == AUTO_PROFILE {
        let auto = AutoProfile::here();
        match horizon_shaders::Values::first_profile(properties, auto.profiles) {
            Some(found) => {
                log::info!("shader pack {name}: Auto picks profile `{found}`");
                values = horizon_shaders::Values::profile(properties, &found).unwrap_or_default();
            }
            None => log::info!("shader pack {name}: Auto finds no light profile"),
        }
        values.cap("shadowMapResolution", auto.shadow_resolution);
        values.cap("shadowDistance", auto.shadow_distance);
    } else if !profile.is_empty() {
        match horizon_shaders::Values::profile(properties, profile) {
            Some(p) => values = p,
            None => log::warn!("shader pack {name}: no profile `{profile}`"),
        }
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
    pub(super) fn shader_pack_inputs(&mut self, camera: glam::DVec3, partial: f32) -> PackInputs {
        // A new choice, once the options close (each load takes seconds): what ran for the old
        // one goes, of either kind.
        let choice = self.settings.horizon.shader_choice();
        if choice != self.shader_choice && self.screen != super::Screen::Options {
            self.shader_choice = choice;
            self.native_shaders = Default::default();
            self.pack_ids = None;
            self.pack_making = None;
            let scale = self.world_scale();
            let antialias = self.settings.horizon.fxaa;
            if let Some(renderer) = &mut self.renderer {
                renderer.drop_shader_packs();
                renderer.set_world_post(scale, antialias);
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
            match Pack::open(&path) {
                Ok(mut pack) => {
                    let macros = Macros::standard();
                    let text = pack.read_text("shaders.properties").unwrap_or_default();
                    let properties = Properties::parse(&text, &macros);
                    let values = picked_values(&properties, name, &profile);
                    if !values.is_empty() {
                        pack.set_values(values);
                    }
                    for note in &properties.notes {
                        log::warn!(
                            "shader pack {name}: shaders.properties:{}: {}",
                            note.line,
                            note.message
                        );
                    }
                    let folder = dimension_folder(&pack, dimension);
                    renderer.set_pack_prepass(profile == AUTO_PROFILE && !light_pack(name));
                    let inputs = renderer.pack_inputs();
                    let biomes: Vec<String> = self
                        .res
                        .biomes()
                        .list
                        .iter()
                        .map(|b| b.name.clone())
                        .collect();
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
                        entities: entities.plain(),
                        missing_entity: if text.is_some() { 65535 } else { 0 },
                        missing_block: if ids.is_empty() { 0 } else { 65535 },
                        blocks: ids.clone(),
                        block_states: RefCell::default(),
                    };
                    let started_ids = std::time::Instant::now();
                    let table = block_id_table(&self.pack, &ids);
                    log::info!(
                        "shader pack {name}: mc_Entity for {} of {} block sprites in {} ms",
                        table.chunks(2).filter(|p| p[0] != -1).count(),
                        table.len() / 2,
                        started_ids.elapsed().as_millis()
                    );
                    // The programs are translated and built on the renderer's thread for packs;
                    // the world draws without the pack until they are.
                    let program_folder = folder.clone();
                    renderer.start_shader_pack(
                        dimension,
                        move || {
                            let mut post =
                                load_post(&pack, &program_folder, &properties, &macros, &inputs);
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
                    self.pack_ids = Some(making.ids);
                }
                Err(e) => log::warn!("shader pack {name}: {e}"),
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
        let info = biome.map(|b| self.res.biomes().get(b));
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
            biome_category: info.map_or(0, |b| horizon_shaders::biome_category(&b.name)),
            biome_precipitation: info.map_or(0, |b| {
                horizon_shaders::biome_precipitation(b.precipitation, b.temperature)
            }),
            temperature: info.map_or(0.0, |b| b.temperature),
            rainfall: info.map_or(0.0, |b| b.downfall),
            held_items,
            held_light,
            player_flags: self.player.as_ref().map_or([false; 5], |p| {
                [
                    p.fire_ticks > 0,
                    p.health.hurt.cooldown > 0,
                    self.potions.effects.get("invisibility").is_some(),
                    p.shift_key_down,
                    p.sprinting,
                ]
            }),
        }
    }

    /// The world's resolution, of the screen's: the player's, and no more than Auto's while it
    /// runs a pack that isn't a light one.
    pub(super) fn world_scale(&self) -> f32 {
        let chosen = self.settings.horizon.resolution_scale as f32 / 100.0;
        match &self.shader_choice {
            Some((name, profile))
                if profile == AUTO_PROFILE
                    && !light_pack(name)
                    && !horizon_shader_api::package::is_pack(
                        &Self::shader_packs_dir().join(name),
                    ) =>
            {
                chosen.min(AutoProfile::here().scale)
            }
            _ => chosen,
        }
    }

    /// `heldItemId(2)` and `heldBlockLightValue(2)`: main hand, then off hand (5.2, 5.3).
    fn held_for_pack(&mut self) -> ([i32; 2], [i32; 2]) {
        if self.pack_ids.is_none() {
            return ([0; 2], [0; 2]);
        }
        if self.luminance.is_none() {
            self.luminance = Some(horizon_client::dynamic_light::Luminance::new(
                self.res.facts(),
                self.pack.assets.registry,
            ));
        }
        let (Some(ids), Some(lum)) = (&self.pack_ids, &self.luminance) else {
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
        if let Some(ids) = &self.pack_ids {
            let id = ids.entities.get(name).copied();
            self.push_span(start, "ENTITIES", id.unwrap_or(ids.missing_entity));
            if let Some(last) = self.entity_spans.borrow_mut().last_mut() {
                last.translucent = translucent;
            }
        }
    }

    /// Starts a span for the block entity of block state `state`.
    pub(super) fn mark_block_entity(&self, start: usize, state: u32) {
        let Some(ids) = &self.pack_ids else { return };
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
        if self.pack_ids.is_some()
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
        if self.pack_ids.is_some() {
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

/// `mc_Entity` (x, y) of each block sprite layer, as (id, fluid) pairs: terrain vertices carry
/// their sprite layer, not their block. A layer takes the id of the block named like its sprite
/// (`block/oak_leaves` for `oak_leaves`), else the id most blocks drawing it share (ties: the
/// lowest); -1 when no block drawing it has one. y is 1 for the fluid sprites, -1 otherwise.
fn block_id_table(pack: &super::PackResources, ids: &BlockIds) -> Vec<i32> {
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
