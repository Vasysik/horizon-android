//! Item stacks on the wire (ItemStack.OPTIONAL_STREAM_CODEC): a count, the
//! item's registry id and a data component patch, plus the hashed form the
//! client sends back in container clicks (HashedStack).
use crate::{
    Result,
    buf::{Codec, Reader, Writer, pack_block_pos, unpack_block_pos},
};
use horizon_world::nbt::Tag;
use std::sync::Arc;

#[doc(hidden)]
pub mod custom;
mod hash;
mod skip;

pub use hash::{Names, NoNames};

/// minecraft:data_component_type registry ids (26.3).
pub mod component {
    pub const CUSTOM_DATA: i32 = 0;
    pub const MAX_STACK_SIZE: i32 = 1;
    pub const MAX_DAMAGE: i32 = 2;
    pub const DAMAGE: i32 = 3;
    pub const UNBREAKABLE: i32 = 4;
    pub const USE_EFFECTS: i32 = 5;
    pub const CUSTOM_NAME: i32 = 6;
    pub const MINIMUM_ATTACK_CHARGE: i32 = 7;
    pub const DAMAGE_TYPE: i32 = 8;
    pub const ITEM_NAME: i32 = 9;
    pub const ITEM_MODEL: i32 = 10;
    pub const LORE: i32 = 11;
    pub const RARITY: i32 = 12;
    pub const ENCHANTMENTS: i32 = 13;
    pub const CAN_PLACE_ON: i32 = 14;
    /// `AdventureModePredicate`: the blocks an adventure player may break.
    pub const CAN_BREAK: i32 = 15;
    pub const ATTRIBUTE_MODIFIERS: i32 = 16;
    pub const CUSTOM_MODEL_DATA: i32 = 17;
    pub const TOOLTIP_DISPLAY: i32 = 18;
    pub const REPAIR_COST: i32 = 19;
    pub const CREATIVE_SLOT_LOCK: i32 = 20;
    pub const ENCHANTMENT_GLINT_OVERRIDE: i32 = 21;
    pub const INTANGIBLE_PROJECTILE: i32 = 22;
    pub const FOOD: i32 = 23;
    pub const CONSUMABLE: i32 = 24;
    pub const USE_REMAINDER: i32 = 25;
    pub const USE_COOLDOWN: i32 = 26;
    pub const DAMAGE_RESISTANT: i32 = 27;
    pub const TOOL: i32 = 28;
    pub const WEAPON: i32 = 29;
    /// `AttackRange`: six floats, in the record's field order.
    pub const ATTACK_RANGE: i32 = 30;
    pub const ENCHANTABLE: i32 = 31;
    pub const EQUIPPABLE: i32 = 32;
    pub const REPAIRABLE: i32 = 33;
    pub const GLIDER: i32 = 34;
    pub const TOOLTIP_STYLE: i32 = 35;
    pub const DEATH_PROTECTION: i32 = 36;
    pub const BLOCKS_ATTACKS: i32 = 37;
    pub const PIERCING_WEAPON: i32 = 38;
    pub const KINETIC_WEAPON: i32 = 39;
    pub const ATTACK_ANIMATION: i32 = 40;
    pub const INTERACT_ANIMATION: i32 = 41;
    pub const ADDITIONAL_TRADE_COST: i32 = 42;
    pub const BLOCK_TRANSFORMER: i32 = 43;
    pub const VILLAGER_FOOD: i32 = 44;
    pub const STORED_ENCHANTMENTS: i32 = 45;
    pub const DYE: i32 = 46;
    pub const DYED_COLOR: i32 = 47;
    pub const MAP_ID: i32 = 48;
    pub const MAP_DECORATIONS: i32 = 49;
    pub const MAP_POST_PROCESSING: i32 = 50;
    pub const CHARGED_PROJECTILES: i32 = 51;
    pub const BUNDLE_CONTENTS: i32 = 52;
    pub const POTION_CONTENTS: i32 = 53;
    pub const POTION_DURATION_SCALE: i32 = 54;
    pub const SUSPICIOUS_STEW_EFFECTS: i32 = 55;
    pub const WRITABLE_BOOK_CONTENT: i32 = 56;
    pub const WRITTEN_BOOK_CONTENT: i32 = 57;
    pub const TRIM: i32 = 58;
    pub const DEBUG_STICK_STATE: i32 = 59;
    pub const ENTITY_DATA: i32 = 60;
    pub const BUCKET_ENTITY_DATA: i32 = 61;
    pub const BLOCK_ENTITY_DATA: i32 = 62;
    pub const INSTRUMENT: i32 = 63;
    pub const PROVIDES_TRIM_MATERIAL: i32 = 64;
    pub const OMINOUS_BOTTLE_AMPLIFIER: i32 = 65;
    pub const JUKEBOX_PLAYABLE: i32 = 66;
    pub const PROVIDES_BANNER_PATTERNS: i32 = 67;
    pub const RECIPES: i32 = 68;
    pub const LODESTONE_TRACKER: i32 = 69;
    pub const FIREWORK_EXPLOSION: i32 = 70;
    pub const FIREWORKS: i32 = 71;
    pub const PROFILE: i32 = 72;
    pub const NOTE_BLOCK_SOUND: i32 = 73;
    pub const BANNER_PATTERNS: i32 = 74;
    pub const BASE_COLOR: i32 = 75;
    pub const POT_DECORATIONS: i32 = 76;
    pub const CONTAINER: i32 = 77;
    pub const BLOCK_STATE: i32 = 78;
    pub const BEES: i32 = 79;
    pub const SULFUR_CUBE_CONTENT: i32 = 80;
    pub const LOCK: i32 = 81;
    pub const CONTAINER_LOOT: i32 = 82;
    pub const BREAK_SOUND: i32 = 83;
    pub const COMPOSTABLE: i32 = 84;
    pub const COOKING_FUEL: i32 = 85;
    pub const BREWING_FUEL: i32 = 86;
    pub const MOB_VISIBILITY: i32 = 87;
    pub const VILLAGER_VARIANT: i32 = 88;
    pub const WOLF_VARIANT: i32 = 89;
    pub const WOLF_SOUND_VARIANT: i32 = 90;
    pub const WOLF_COLLAR: i32 = 91;
    pub const FOX_VARIANT: i32 = 92;
    pub const SALMON_SIZE: i32 = 93;
    pub const PARROT_VARIANT: i32 = 94;
    pub const TROPICAL_FISH_PATTERN: i32 = 95;
    pub const TROPICAL_FISH_BASE_COLOR: i32 = 96;
    pub const TROPICAL_FISH_PATTERN_COLOR: i32 = 97;
    pub const MOOSHROOM_VARIANT: i32 = 98;
    pub const RABBIT_VARIANT: i32 = 99;
    pub const PIG_VARIANT: i32 = 100;
    pub const PIG_SOUND_VARIANT: i32 = 101;
    pub const COW_VARIANT: i32 = 102;
    pub const COW_SOUND_VARIANT: i32 = 103;
    pub const CHICKEN_VARIANT: i32 = 104;
    pub const CHICKEN_SOUND_VARIANT: i32 = 105;
    pub const ZOMBIE_NAUTILUS_VARIANT: i32 = 106;
    pub const FROG_VARIANT: i32 = 107;
    pub const HORSE_VARIANT: i32 = 108;
    pub const PAINTING_VARIANT: i32 = 109;
    pub const LLAMA_VARIANT: i32 = 110;
    pub const AXOLOTL_VARIANT: i32 = 111;
    pub const CAT_VARIANT: i32 = 112;
    pub const CAT_SOUND_VARIANT: i32 = 113;
    pub const CAT_COLLAR: i32 = 114;
    pub const SHEEP_COLOR: i32 = 115;
    pub const SHULKER_COLOR: i32 = 116;
    pub const PROVIDES_POTTERY_PATTERN: i32 = 117;
    pub const SIGN_TEXT_FRONT: i32 = 118;
    pub const SIGN_TEXT_BACK: i32 = 119;
    pub const WAXED: i32 = 120;
    pub const CUSHION_COLOR: i32 = 121;
    /// The number of component types; ids at or past it are refused.
    pub const COUNT: i32 = 122;
}

/// A written book's content (`WrittenBookContent.STREAM_CODEC`). The raw title
/// and pages are what a book shows; the filtered variants (`Filterable.filtered`,
/// which a server sends when chat filtering is on) are kept only so that the
/// component hashes as the server holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WrittenBook {
    pub title: String,
    pub title_filtered: Option<String>,
    pub author: String,
    pub generation: i32,
    /// Each page's text component.
    pub pages: Vec<Tag>,
    /// Each page's filtered variant: empty (none filtered) or one per page.
    pub pages_filtered: Vec<Option<Tag>>,
    pub resolved: bool,
}

/// The value of one patched data component. Components Horizon does not model
/// are kept as `Raw` bytes; only an unknown component type makes the stack
/// (and packet) undecodable.
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Int(i32),
    Unit,
    Bool(bool),
    Text(Tag),
    Nbt(Tag),
    Ident(String),
    Lore(Vec<Tag>),
    Enchantments(Vec<(i32, i32)>),
    AttackRange([f32; 6]),
    /// `PotionContents`: the potion's `minecraft:potion` id and the custom
    /// colour; the custom effects and name stay their encoded bytes.
    Potion {
        potion: Option<i32>,
        color: Option<i32>,
        rest: Vec<u8>,
    },
    Profile(Profile),
    CustomModelData {
        floats: Vec<f32>,
        flags: Vec<bool>,
        strings: Vec<String>,
        colors: Vec<i32>,
    },
    TooltipDisplay {
        hide: bool,
        hidden: Vec<i32>,
    },
    /// `WritableBookContent`: the pages (raw text) and their filtered variants
    /// (empty, or one per page).
    WritableBook {
        pages: Vec<String>,
        filtered: Vec<Option<String>>,
    },
    WrittenBook(WrittenBook),
    /// `ChargedProjectiles`: a loaded crossbow's ammunition, as templates.
    Stacks(Vec<Stack>),
    /// `Fireworks`: flight duration and its explosions.
    Fireworks {
        flight: i32,
        explosions: Vec<FireworkExplosion>,
    },
    /// `FireworkExplosion` (a firework star).
    FireworkExplosion(FireworkExplosion),
    /// `LodestoneTracker`: the target's dimension and block, and whether it
    /// follows the lodestone.
    LodestoneTracker {
        target: Option<(String, [i32; 3])>,
        tracked: bool,
    },
    /// `ArmorTrim` naming its material and pattern by registry id (`trim_material`,
    /// `trim_pattern`); a trim with a direct holder stays [`Component::Raw`].
    Trim {
        material: i32,
        pattern: i32,
    },
    /// `SuspiciousStewEffects`: each entry's `minecraft:mob_effect` id and
    /// duration in ticks.
    StewEffects(Vec<(i32, i32)>),
    /// `InstrumentComponent` naming its instrument by `minecraft:instrument`
    /// registry id; a direct instrument stays [`Component::Raw`].
    Instrument(i32),
    /// `BannerPatternLayers`: each layer's `banner_pattern` registry id and
    /// its dye id (`DyeColor.getId`). A layer with a direct pattern has no
    /// registry name: that value stays [`Component::Raw`].
    BannerPatterns(Vec<(i32, i32)>),
    /// `painting/variant` naming its `painting_variant` by registry id; a
    /// direct variant stays [`Component::Raw`].
    PaintingVariant(i32),
    /// A component Horizon reads past without interpreting it: its encoded
    /// bytes, written back as they came.
    Raw(Vec<u8>),
}

/// `FireworkExplosion.STREAM_CODEC`: shape id, colors, fade colors, trail
/// and twinkle (FireworkExplosion.Shape's BY_ID order).
#[derive(Clone, Debug, PartialEq)]
pub struct FireworkExplosion {
    pub shape: i32,
    pub colors: Vec<i32>,
    pub fade: Vec<i32>,
    pub trail: bool,
    pub twinkle: bool,
}
impl FireworkExplosion {
    fn read(r: &mut Reader) -> Result<Self> {
        let ints = |r: &mut Reader| -> Result<Vec<i32>> {
            let n = r.count(1024)?;
            (0..n).map(|_| r.i32()).collect()
        };
        Ok(Self {
            shape: r.varint()?,
            colors: ints(r)?,
            fade: ints(r)?,
            trail: r.bool()?,
            twinkle: r.bool()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.varint(self.shape);
        w.varint(self.colors.len() as i32);
        self.colors.iter().for_each(|c| w.i32(*c));
        w.varint(self.fade.len() as i32);
        self.fade.iter().for_each(|c| w.i32(*c));
        w.bool(self.trail);
        w.bool(self.twinkle);
    }
}

/// `Filterable.streamCodec`: the raw value, then an optional filtered one.
fn filterable<T>(
    r: &mut Reader,
    value: impl Fn(&mut Reader) -> Result<T>,
) -> Result<(T, Option<T>)> {
    let raw = value(r)?;
    let filtered = if r.bool()? { Some(value(r)?) } else { None };
    Ok((raw, filtered))
}
/// The filtered variants of a list of pages, or none when no page has one.
fn filtered_variants<T>(pages: &mut Vec<(T, Option<T>)>) -> (Vec<T>, Vec<Option<T>>) {
    let any = pages.iter().any(|(_, f)| f.is_some());
    let (raw, filtered): (Vec<_>, Vec<_>) = pages.drain(..).unzip();
    (raw, if any { filtered } else { Vec::new() })
}

/// `ResolvableProfile`: an optional name and UUID and the profile's property
/// list, followed by `PlayerSkin.Patch`'s four optional textures.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: Option<String>,
    pub id: Option<u128>,
    pub properties: Vec<(String, String, Option<String>)>,
    pub skin: SkinPatch,
}
/// `PlayerSkin.Patch`: the optional body, cape and elytra texture ids and the
/// model type (`true` for slim).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinPatch {
    pub body: Option<String>,
    pub cape: Option<String>,
    pub elytra: Option<String>,
    pub slim: Option<bool>,
}
impl SkinPatch {
    fn read(r: &mut Reader) -> Result<Self> {
        let texture = |r: &mut Reader| -> Result<Option<String>> {
            if r.bool()? {
                Ok(Some(r.string(32767)?))
            } else {
                Ok(None)
            }
        };
        let (body, cape, elytra) = (texture(r)?, texture(r)?, texture(r)?);
        let slim = if r.bool()? { Some(r.bool()?) } else { None };
        Ok(Self {
            body,
            cape,
            elytra,
            slim,
        })
    }
    fn write(&self, w: &mut Writer) {
        for texture in [&self.body, &self.cape, &self.elytra] {
            w.bool(texture.is_some());
            if let Some(texture) = texture {
                w.string(texture);
            }
        }
        w.bool(self.slim.is_some());
        if let Some(slim) = self.slim {
            w.bool(slim);
        }
    }
}
impl Component {
    pub(crate) fn read(kind: i32, r: &mut Reader) -> Result<Self> {
        use component::*;
        Ok(match kind {
            MAX_STACK_SIZE
            | MAX_DAMAGE
            | DAMAGE
            | RARITY
            | REPAIR_COST
            | MAP_ID
            | MAP_POST_PROCESSING
            | OMINOUS_BOTTLE_AMPLIFIER => Self::Int(r.varint()?),
            WRITABLE_BOOK_CONTENT => {
                let n = r.count(100)?;
                let mut pages = (0..n)
                    .map(|_| filterable(r, |r| r.string(1024)))
                    .collect::<Result<Vec<_>>>()?;
                let (pages, filtered) = filtered_variants(&mut pages);
                Self::WritableBook { pages, filtered }
            }
            WRITTEN_BOOK_CONTENT => {
                let (title, title_filtered) = filterable(r, |r| r.string(32))?;
                let author = r.string(32767)?;
                let generation = r.varint()?;
                let n = r.count(r.remaining())?;
                let mut pages = (0..n)
                    .map(|_| filterable(r, |r| r.nbt()))
                    .collect::<Result<Vec<_>>>()?;
                let (pages, pages_filtered) = filtered_variants(&mut pages);
                Self::WrittenBook(WrittenBook {
                    title,
                    title_filtered,
                    author,
                    generation,
                    pages,
                    pages_filtered,
                    resolved: r.bool()?,
                })
            }
            CHARGED_PROJECTILES | BUNDLE_CONTENTS => {
                let n = r.count(1024)?;
                Self::Stacks((0..n).map(|_| Template::read(r)).collect::<Result<_>>()?)
            }
            DYED_COLOR => Self::Int(r.i32()?),
            UNBREAKABLE | CREATIVE_SLOT_LOCK => Self::Unit,
            ENCHANTMENT_GLINT_OVERRIDE => Self::Bool(r.bool()?),
            CUSTOM_NAME | ITEM_NAME => Self::Text(r.nbt()?),
            CUSTOM_DATA => Self::Nbt(r.nbt()?),
            ITEM_MODEL => Self::Ident(r.string(32767)?),
            LORE => {
                let n = r.count(256)?;
                Self::Lore((0..n).map(|_| r.nbt()).collect::<Result<_>>()?)
            }
            // SuspiciousStewEffects.Entry.STREAM_CODEC: MobEffect.STREAM_CODEC
            // (the registry id), then the duration, both varints.
            SUSPICIOUS_STEW_EFFECTS => {
                let n = r.count(r.remaining())?;
                Self::StewEffects(
                    (0..n)
                        .map(|_| Ok((r.varint()?, r.varint()?)))
                        .collect::<Result<_>>()?,
                )
            }
            ENCHANTMENTS | STORED_ENCHANTMENTS => {
                let n = r.count(r.remaining())?;
                Self::Enchantments(
                    (0..n)
                        .map(|_| Ok((r.varint()?, r.varint()?)))
                        .collect::<Result<_>>()?,
                )
            }
            POTION_CONTENTS => {
                let potion = if r.bool()? { Some(r.varint()?) } else { None };
                let color = if r.bool()? { Some(r.i32()?) } else { None };
                let start = r.position();
                skip::potion_effects_and_name(r)?;
                // No custom effects and no name: an empty tail.
                let rest = match r.since(start) {
                    [0, 0] => Vec::new(),
                    bytes => bytes.to_vec(),
                };
                Self::Potion {
                    potion,
                    color,
                    rest,
                }
            }
            ATTACK_RANGE => {
                let mut values = [0.0f32; 6];
                for v in &mut values {
                    *v = r.f32()?;
                }
                Self::AttackRange(values)
            }
            PROFILE => {
                fn properties(r: &mut Reader) -> Result<Vec<(String, String, Option<String>)>> {
                    let n = r.count(1024)?;
                    let mut out = Vec::with_capacity(n);
                    for _ in 0..n {
                        let name = r.string(32767)?;
                        let value = r.string(32767)?;
                        let signature = if r.bool()? {
                            Some(r.string(32767)?)
                        } else {
                            None
                        };
                        out.push((name, value, signature));
                    }
                    Ok(out)
                }
                // Either<GameProfile, Partial>: the partial form carries
                // optional fields, the resolved one a UUID, a name and the
                // properties.
                let (name, id, properties) = if r.u8()? == 0 {
                    let name = if r.bool()? { Some(r.string(64)?) } else { None };
                    let id = if r.bool()? { Some(r.uuid()?) } else { None };
                    (name, id, properties(r)?)
                } else {
                    let id = Some(r.uuid()?);
                    let name = Some(r.string(64)?);
                    (name, id, properties(r)?)
                };
                // PlayerSkin.Patch: three texture ids and the model type.
                Self::Profile(Profile {
                    name,
                    id,
                    properties,
                    skin: SkinPatch::read(r)?,
                })
            }
            CUSTOM_MODEL_DATA => {
                let n = r.count(r.remaining())?;
                let floats = (0..n).map(|_| r.f32()).collect::<Result<_>>()?;
                let n = r.count(r.remaining())?;
                let flags = (0..n).map(|_| r.bool()).collect::<Result<_>>()?;
                let n = r.count(r.remaining())?;
                let strings = (0..n).map(|_| r.string(32767)).collect::<Result<_>>()?;
                let n = r.count(r.remaining())?;
                let colors = (0..n).map(|_| r.i32()).collect::<Result<_>>()?;
                Self::CustomModelData {
                    floats,
                    flags,
                    strings,
                    colors,
                }
            }
            TOOLTIP_DISPLAY => {
                let hide = r.bool()?;
                let n = r.count(r.remaining())?;
                Self::TooltipDisplay {
                    hide,
                    hidden: (0..n).map(|_| r.varint()).collect::<Result<_>>()?,
                }
            }
            FIREWORKS => {
                let flight = r.varint()?;
                let n = r.count(256)?;
                Self::Fireworks {
                    flight,
                    explosions: (0..n)
                        .map(|_| FireworkExplosion::read(r))
                        .collect::<Result<_>>()?,
                }
            }
            FIREWORK_EXPLOSION => Self::FireworkExplosion(FireworkExplosion::read(r)?),
            LODESTONE_TRACKER => {
                let target = if r.bool()? {
                    Some((r.string(32767)?, unpack_block_pos(r.i64()?)))
                } else {
                    None
                };
                Self::LodestoneTracker {
                    target,
                    tracked: r.bool()?,
                }
            }
            // ArmorTrim.STREAM_CODEC: two holders, each a reference's id plus
            // one. A direct material or pattern has no registry name: read past.
            TRIM => {
                let start = r.position();
                skip::component(TRIM, r)?;
                let bytes = r.since(start);
                let mut ids = Reader::new(bytes);
                match (ids.varint(), ids.varint()) {
                    (Ok(material @ 1..), Ok(pattern @ 1..)) if ids.remaining() == 0 => Self::Trim {
                        material: material - 1,
                        pattern: pattern - 1,
                    },
                    _ => Self::Raw(bytes.to_vec()),
                }
            }
            // InstrumentComponent.STREAM_CODEC: a holder, a reference's id plus one.
            // A direct instrument has no registry name: read past.
            INSTRUMENT => {
                let start = r.position();
                skip::component(INSTRUMENT, r)?;
                let bytes = r.since(start);
                let mut ids = Reader::new(bytes);
                match ids.varint() {
                    Ok(id @ 1..) if ids.remaining() == 0 => Self::Instrument(id - 1),
                    _ => Self::Raw(bytes.to_vec()),
                }
            }
            // Holder<PaintingVariant>: a reference's id plus one, or 0 and
            // the variant itself (no registry name: read past).
            PAINTING_VARIANT => {
                let start = r.position();
                skip::component(PAINTING_VARIANT, r)?;
                let bytes = r.since(start);
                let mut ids = Reader::new(bytes);
                match ids.varint() {
                    Ok(id @ 1..) if ids.remaining() == 0 => Self::PaintingVariant(id - 1),
                    _ => Self::Raw(bytes.to_vec()),
                }
            }
            // DyeColor.STREAM_CODEC: its id.
            BASE_COLOR => Self::Int(r.varint()?),
            // BannerPatternLayers.STREAM_CODEC: a list of a pattern holder (a
            // reference's id plus one, or 0 and the pattern itself) and a dye.
            BANNER_PATTERNS => {
                let start = r.position();
                skip::component(BANNER_PATTERNS, r)?;
                let bytes = r.since(start);
                match Self::banner_layers(bytes) {
                    Some(layers) => Self::BannerPatterns(layers),
                    None => Self::Raw(bytes.to_vec()),
                }
            }
            other if custom::contains(other) => Self::Nbt(r.nbt()?),
            other => {
                let start = r.position();
                skip::component(other, r)?;
                Self::Raw(r.since(start).to_vec())
            }
        })
    }
    /// The layers of encoded `BannerPatternLayers` whose patterns are all
    /// registry references (`None` when one is direct).
    fn banner_layers(bytes: &[u8]) -> Option<Vec<(i32, i32)>> {
        let mut r = Reader::new(bytes);
        let n = r.count(r.remaining()).ok()?;
        let mut layers = Vec::with_capacity(n);
        for _ in 0..n {
            let pattern = r.varint().ok()?;
            let color = r.varint().ok()?;
            if pattern < 1 {
                return None;
            }
            layers.push((pattern - 1, color));
        }
        (r.remaining() == 0).then_some(layers)
    }
    pub(crate) fn write(&self, kind: i32, w: &mut Writer) {
        match self {
            Self::Raw(bytes) => w.bytes(bytes),
            Self::BannerPatterns(layers) => {
                w.varint(layers.len() as i32);
                for (pattern, color) in layers {
                    w.varint(pattern + 1);
                    w.varint(*color);
                }
            }
            Self::Trim { material, pattern } => {
                w.varint(material + 1);
                w.varint(pattern + 1);
            }
            Self::Instrument(id) => w.varint(id + 1),
            Self::PaintingVariant(id) => w.varint(id + 1),
            Self::Int(v) if kind == component::DYED_COLOR => w.i32(*v),
            Self::Int(v) => w.varint(*v),
            Self::Unit => {}
            Self::Bool(v) => w.bool(*v),
            Self::Text(t) | Self::Nbt(t) => w.nbt(t),
            Self::Ident(s) => w.string(s),
            Self::Lore(lines) => {
                w.varint(lines.len() as i32);
                lines.iter().for_each(|l| w.nbt(l));
            }
            Self::Enchantments(list) | Self::StewEffects(list) => {
                w.varint(list.len() as i32);
                for (e, level) in list {
                    w.varint(*e);
                    w.varint(*level);
                }
            }
            Self::AttackRange(values) => values.iter().for_each(|v| w.f32(*v)),
            Self::Potion {
                potion,
                color,
                rest,
            } => {
                w.bool(potion.is_some());
                if let Some(id) = potion {
                    w.varint(*id);
                }
                w.bool(color.is_some());
                if let Some(color) = color {
                    w.i32(*color);
                }
                // An empty tail is no custom effects and no name.
                if rest.is_empty() {
                    w.varint(0);
                    w.bool(false);
                } else {
                    w.bytes(rest);
                }
            }
            Self::Profile(p) => {
                w.u8(0);
                w.bool(p.name.is_some());
                if let Some(name) = &p.name {
                    w.string(name);
                }
                w.bool(p.id.is_some());
                if let Some(id) = p.id {
                    w.uuid(id);
                }
                w.varint(p.properties.len() as i32);
                for (name, value, signature) in &p.properties {
                    w.string(name);
                    w.string(value);
                    w.bool(signature.is_some());
                    if let Some(signature) = signature {
                        w.string(signature);
                    }
                }
                p.skin.write(w);
            }
            Self::CustomModelData {
                floats,
                flags,
                strings,
                colors,
            } => {
                w.varint(floats.len() as i32);
                floats.iter().for_each(|f| w.f32(*f));
                w.varint(flags.len() as i32);
                flags.iter().for_each(|f| w.bool(*f));
                w.varint(strings.len() as i32);
                strings.iter().for_each(|s| w.string(s));
                w.varint(colors.len() as i32);
                colors.iter().for_each(|c| w.i32(*c));
            }
            Self::TooltipDisplay { hide, hidden } => {
                w.bool(*hide);
                w.varint(hidden.len() as i32);
                hidden.iter().for_each(|h| w.varint(*h));
            }
            Self::Stacks(stacks) => {
                w.varint(stacks.len() as i32);
                stacks.iter().for_each(|s| Template::write(s, w));
            }
            Self::Fireworks { flight, explosions } => {
                w.varint(*flight);
                w.varint(explosions.len() as i32);
                explosions.iter().for_each(|e| e.write(w));
            }
            Self::FireworkExplosion(e) => e.write(w),
            Self::LodestoneTracker { target, tracked } => {
                w.bool(target.is_some());
                if let Some((dimension, pos)) = target {
                    w.string(dimension);
                    w.i64(pack_block_pos(*pos));
                }
                w.bool(*tracked);
            }
            Self::WritableBook { pages, filtered } => {
                w.varint(pages.len() as i32);
                for (i, page) in pages.iter().enumerate() {
                    w.string(page);
                    match filtered.get(i).and_then(Option::as_ref) {
                        Some(f) => {
                            w.bool(true);
                            w.string(f);
                        }
                        None => w.bool(false),
                    }
                }
            }
            Self::WrittenBook(b) => {
                w.string(&b.title);
                match &b.title_filtered {
                    Some(f) => {
                        w.bool(true);
                        w.string(f);
                    }
                    None => w.bool(false),
                }
                w.string(&b.author);
                w.varint(b.generation);
                w.varint(b.pages.len() as i32);
                for (i, page) in b.pages.iter().enumerate() {
                    w.nbt(page);
                    match b.pages_filtered.get(i).and_then(Option::as_ref) {
                        Some(f) => {
                            w.bool(true);
                            w.nbt(f);
                        }
                        None => w.bool(false),
                    }
                }
                w.bool(b.resolved);
            }
        }
    }
}

/// A component patch as `DelimitedSlot` writes it (both counts, then each
/// value behind its length), kept from a stack that was read so one nobody
/// edited goes back byte for byte. It is no part of a stack's equality.
#[derive(Clone, Debug, Default)]
pub struct WirePatch(pub Option<Arc<[u8]>>);
impl WirePatch {
    /// Whether a patch in this form sets component `kind`.
    pub fn adds(patch: &[u8], kind: i32) -> bool {
        let mut r = Reader::new(patch);
        let Ok(positive) = r.count(65536) else {
            return false;
        };
        if r.count(65536).is_err() {
            return false;
        }
        (0..positive).any(|_| {
            let Ok(k) = r.varint() else {
                return false;
            };
            let Ok(len) = r.count(r.remaining()) else {
                return false;
            };
            r.bytes(len).is_ok() && k == kind
        })
    }
}
impl WirePatch {
    /// The components and removals a patch in this form holds, read as
    /// [`DelimitedSlot`] does.
    pub fn decode(patch: &[u8]) -> Result<(Vec<(i32, Component)>, Vec<i32>)> {
        let mut r = Reader::new(patch);
        let positive = r.count(65536)?;
        let negative = r.count(65536)?;
        let mut added = Vec::with_capacity(positive.min(64));
        for _ in 0..positive {
            let kind = r.varint()?;
            let len = r.count(r.remaining())?;
            added.push((
                kind,
                Component::read(kind, &mut Reader::new(r.bytes(len)?))?,
            ));
        }
        let removed = (0..negative).map(|_| r.varint()).collect::<Result<_>>()?;
        Ok((added, removed))
    }
}
impl PartialEq for WirePatch {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// A non-empty item stack.
#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub item: i32,
    pub count: i32,
    pub added: Vec<(i32, Component)>,
    pub removed: Vec<i32>,
    /// The patch as it was read; [`DelimitedSlot`] writes it in place of
    /// `added` and `removed` when set. Only for a stack nobody has edited.
    pub wire: WirePatch,
}
impl Stack {
    pub fn new(item: i32, count: i32) -> Self {
        Self {
            item,
            count,
            added: Vec::new(),
            removed: Vec::new(),
            wire: WirePatch::default(),
        }
    }
    pub fn component(&self, kind: i32) -> Option<&Component> {
        self.added.iter().find(|(k, _)| *k == kind).map(|(_, c)| c)
    }
    pub fn damage(&self) -> i32 {
        match self.component(component::DAMAGE) {
            Some(Component::Int(d)) => *d,
            _ => 0,
        }
    }
    /// The stack's `use_cooldown`: its seconds and cooldown group (None: the item's id).
    pub fn use_cooldown(&self) -> Option<(f32, Option<String>)> {
        let Some(Component::Raw(bytes)) = self.component(component::USE_COOLDOWN) else {
            return None;
        };
        let mut r = Reader::new(bytes);
        let seconds = r.f32().ok()?;
        let group = if r.bool().ok()? {
            Some(r.string(32767).ok()?)
        } else {
            None
        };
        Some((seconds, group))
    }
    /// The stack's own `equippable` (a server's custom armour), read from its bytes.
    pub fn equippable(&self) -> Option<Equippable> {
        let Some(Component::Raw(bytes)) = self.component(component::EQUIPPABLE) else {
            return None;
        };
        let mut r = Reader::new(bytes);
        let slot = r.varint().ok()?;
        // equipSound: a registry id plus one, or 0 and the sound itself.
        if r.varint().ok()? == 0 {
            r.string(32767).ok()?;
            if r.bool().ok()? {
                r.f32().ok()?;
            }
        }
        let mut ident = || match r.bool().ok()? {
            true => Some(Some(r.string(32767).ok()?)),
            false => Some(None),
        };
        let asset = ident()?;
        let overlay = ident()?;
        Some(Equippable {
            slot,
            asset,
            overlay,
        })
    }
}
/// What a stack's `equippable` says of where and how it is worn: the slot by `EquipmentSlot`'s
/// id (0 main hand, 1 feet, 2 legs, 3 chest, 4 head, 5 off hand, 6 body, 7 saddle), the
/// equipment asset it draws (`nexo:ruby` is `assets/nexo/equipment/ruby.json`) and the camera
/// overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct Equippable {
    pub slot: i32,
    pub asset: Option<String>,
    pub overlay: Option<String>,
}
/// ItemStack.OPTIONAL_STREAM_CODEC: `None` is the empty stack (count 0).
pub struct Slot;
impl Codec<Option<Stack>> for Slot {
    fn read(r: &mut Reader) -> Result<Option<Stack>> {
        read_stack(r, false)
    }
    fn write(v: &Option<Stack>, w: &mut Writer) {
        let Some(s) = v else {
            w.varint(0);
            return;
        };
        w.varint(s.count);
        w.varint(s.item);
        w.varint(s.added.len() as i32);
        w.varint(s.removed.len() as i32);
        for (kind, c) in &s.added {
            w.varint(*kind);
            c.write(*kind, w);
        }
        for kind in &s.removed {
            w.varint(*kind);
        }
    }
}

/// How deep stacks may nest in stacks (a bundle in a shulker box in a bundle...): deeper, the
/// packet is refused before a hostile server's nesting can overflow the reading thread's stack.
const MAX_NESTING: u32 = 64;
thread_local! {
    static NESTING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}
/// One level of stacks inside a stack, left when dropped.
struct Nested;
impl Nested {
    fn enter() -> Result<Self> {
        NESTING.with(|n| {
            if n.get() >= MAX_NESTING {
                return Err(crate::Error::new(format!(
                    "item stacks nested over {MAX_NESTING} deep"
                )));
            }
            n.set(n.get() + 1);
            Ok(Nested)
        })
    }
}
impl Drop for Nested {
    fn drop(&mut self) {
        NESTING.with(|n| n.set(n.get().saturating_sub(1)));
    }
}

/// A stack of either optional codec: with `delimited` each component value
/// has a length in front (DataComponentPatch.DELIMITED_STREAM_CODEC). The
/// patch is also kept, in the delimited form, for [`Stack::wire`].
fn read_stack(r: &mut Reader, delimited: bool) -> Result<Option<Stack>> {
    let count = r.varint()?;
    if count <= 0 {
        return Ok(None);
    }
    let item = r.varint()?;
    let positive = r.count(65536)?;
    let negative = r.count(65536)?;
    let mut added = Vec::with_capacity(positive.min(64));
    let mut patch = Writer::new();
    patch.varint(positive as i32);
    patch.varint(negative as i32);
    for _ in 0..positive {
        let kind = r.varint()?;
        let (component, value) = if delimited {
            let len = r.count(r.remaining())?;
            let value = r.bytes(len)?;
            let mut component_reader = Reader::new(value);
            let component = Component::read(kind, &mut component_reader)?;
            if custom::contains(kind) && component_reader.remaining() != 0 {
                return Err(crate::Error::new("trailing custom component data"));
            }
            (component, value)
        } else {
            let start = r.position();
            (Component::read(kind, r)?, r.since(start))
        };
        patch.varint(kind);
        patch.byte_array(value);
        added.push((kind, component));
    }
    let removed: Vec<i32> = (0..negative).map(|_| r.varint()).collect::<Result<_>>()?;
    removed.iter().for_each(|kind| patch.varint(*kind));
    Ok(Some(Stack {
        item,
        count,
        added,
        removed,
        wire: WirePatch((positive + negative > 0).then(|| patch.data.into())),
    }))
}

/// ItemStack.OPTIONAL_UNTRUSTED_STREAM_CODEC (`SetCreativeModeSlot`): as [`Slot`],
/// but every component value goes behind its byte length
/// (DataComponentPatch.DELIMITED_STREAM_CODEC), so a server skips what it can't read.
pub struct DelimitedSlot;
impl Codec<Option<Stack>> for DelimitedSlot {
    fn read(r: &mut Reader) -> Result<Option<Stack>> {
        read_stack(r, true)
    }
    fn write(v: &Option<Stack>, w: &mut Writer) {
        let Some(s) = v else {
            w.varint(0);
            return;
        };
        w.varint(s.count);
        w.varint(s.item);
        if let Some(patch) = &s.wire.0 {
            w.bytes(patch);
            return;
        }
        w.varint(s.added.len() as i32);
        w.varint(s.removed.len() as i32);
        for (kind, c) in &s.added {
            w.varint(*kind);
            let mut value = Writer::new();
            c.write(*kind, &mut value);
            w.byte_array(&value.data);
        }
        for kind in &s.removed {
            w.varint(*kind);
        }
    }
}

/// ItemStackTemplate.STREAM_CODEC: the item, the count, then the patch.
pub struct Template;
impl Codec<Stack> for Template {
    fn read(r: &mut Reader) -> Result<Stack> {
        // Every stack inside a stack is a template: the one place nesting recurses.
        let _nested = Nested::enter()?;
        let item = r.varint()?;
        let count = r.varint()?;
        let positive = r.count(65536)?;
        let negative = r.count(65536)?;
        let mut added = Vec::with_capacity(positive.min(64));
        for _ in 0..positive {
            let kind = r.varint()?;
            added.push((kind, Component::read(kind, r)?));
        }
        let removed = (0..negative).map(|_| r.varint()).collect::<Result<_>>()?;
        Ok(Stack {
            item,
            count,
            added,
            removed,
            wire: WirePatch::default(),
        })
    }
    fn write(s: &Stack, w: &mut Writer) {
        w.varint(s.item);
        w.varint(s.count);
        w.varint(s.added.len() as i32);
        w.varint(s.removed.len() as i32);
        for (kind, c) in &s.added {
            w.varint(*kind);
            c.write(*kind, w);
        }
        for kind in &s.removed {
            w.varint(*kind);
        }
    }
}

/// HashedStack: what the client believes a slot holds. Component values are
/// sent as hashes ([`Hashed::create`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Hashed {
    pub item: i32,
    pub count: i32,
    pub added: Vec<(i32, i32)>,
    pub removed: Vec<i32>,
}
impl Hashed {
    /// The stack hashed without registry names: only components that hold no
    /// registry id get a hash (see [`Hashed::create`]).
    pub fn of(stack: &Stack) -> Self {
        Self::create(stack, &NoNames)
    }
}
pub struct HashedSlot;
impl Codec<Option<Hashed>> for HashedSlot {
    fn read(r: &mut Reader) -> Result<Option<Hashed>> {
        if !r.bool()? {
            return Ok(None);
        }
        let item = r.varint()?;
        let count = r.varint()?;
        let n = r.count(256)?;
        let added = (0..n)
            .map(|_| Ok((r.varint()?, r.i32()?)))
            .collect::<Result<_>>()?;
        let n = r.count(256)?;
        let removed = (0..n).map(|_| r.varint()).collect::<Result<_>>()?;
        Ok(Some(Hashed {
            item,
            count,
            added,
            removed,
        }))
    }
    fn write(v: &Option<Hashed>, w: &mut Writer) {
        w.bool(v.is_some());
        if let Some(h) = v {
            w.varint(h.item);
            w.varint(h.count);
            w.varint(h.added.len() as i32);
            for (k, hash) in &h.added {
                w.varint(*k);
                w.i32(*hash);
            }
            w.varint(h.removed.len() as i32);
            for k in &h.removed {
                w.varint(*k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::literal;

    fn round<T: PartialEq + std::fmt::Debug, C: Codec<T>>(v: T) {
        let mut w = Writer::new();
        C::write(&v, &mut w);
        let mut r = Reader::new(&w.data);
        assert_eq!(C::read(&mut r).unwrap(), v);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn a_wire_patch_tells_what_it_adds() {
        // Two added components (damage 3, then can_break's bytes as read)
        // and one removed.
        let mut w = Writer::new();
        w.varint(2);
        w.varint(1);
        w.varint(component::DAMAGE);
        w.byte_array(&[3]);
        w.varint(component::CAN_BREAK);
        w.byte_array(&[1, 0, 0, 0, 0, 0]);
        w.varint(component::ENCHANTMENTS);
        assert!(WirePatch::adds(&w.data, component::DAMAGE));
        assert!(WirePatch::adds(&w.data, component::CAN_BREAK));
        assert!(!WirePatch::adds(&w.data, component::ENCHANTMENTS));
        assert!(!WirePatch::adds(&[], component::CAN_BREAK));
    }

    #[test]
    fn potion_contents_round_trip() {
        let mut arrow = Stack::new(7, 3);
        arrow.added.push((
            component::POTION_CONTENTS,
            Component::Potion {
                potion: Some(31),
                color: None,
                rest: Vec::new(),
            },
        ));
        round::<Option<Stack>, Slot>(Some(arrow));
        // A custom colour, one custom effect (speed II for 600 ticks, shown,
        // no hidden effect) and a custom name survive as they came.
        let mut w = Writer::new();
        w.bool(false);
        w.bool(true);
        w.i32(0x00FF_8800);
        w.varint(1);
        for v in [1, 1, 600] {
            w.varint(v);
        }
        for b in [false, true, true, false] {
            w.bool(b);
        }
        w.bool(true);
        w.string("Tea");
        let mut r = Reader::new(&w.data);
        let read = Component::read(component::POTION_CONTENTS, &mut r).unwrap();
        assert_eq!(r.remaining(), 0);
        let Component::Potion {
            potion,
            color,
            rest,
        } = &read
        else {
            panic!("{read:?}");
        };
        assert_eq!((*potion, *color), (None, Some(0x00FF_8800)));
        assert!(!rest.is_empty());
        let mut again = Writer::new();
        read.write(component::POTION_CONTENTS, &mut again);
        assert_eq!(again.data, w.data);
    }

    #[test]
    fn stacks_round_trip() {
        round::<Option<Stack>, Slot>(None);
        round::<Option<Stack>, Slot>(Some(Stack::new(42, 64)));
        let mut sword = Stack::new(900, 1);
        sword.added.push((component::DAMAGE, Component::Int(17)));
        sword.added.push((
            component::CUSTOM_NAME,
            Component::Text(literal("Excalibur")),
        ));
        sword.added.push((
            component::ENCHANTMENTS,
            Component::Enchantments(vec![(5, 3), (12, 1)]),
        ));
        sword
            .added
            .push((component::DYED_COLOR, Component::Int(-6265536)));
        sword.added.push((
            component::LORE,
            Component::Lore(vec![literal("a"), literal("b")]),
        ));
        sword.removed.push(component::REPAIR_COST);
        let mut book = Stack::new(1100, 1);
        book.added.push((
            component::WRITTEN_BOOK_CONTENT,
            Component::WrittenBook(WrittenBook {
                title: "Notes".into(),
                author: "Guest".into(),
                generation: 1,
                pages: vec![literal("one"), literal("two")],
                pages_filtered: vec![None, Some(literal("2"))],
                title_filtered: Some("N".into()),
                resolved: true,
            }),
        ));
        book.added
            .push((component::MAP_POST_PROCESSING, Component::Int(1)));
        round::<Option<Stack>, Slot>(Some(book));
        // OminousBottleAmplifier.STREAM_CODEC: a VarInt.
        let mut bottle = Stack::new(1200, 1);
        bottle
            .added
            .push((component::OMINOUS_BOTTLE_AMPLIFIER, Component::Int(3)));
        let mut w = Writer::new();
        Slot::write(&Some(bottle.clone()), &mut w);
        assert_eq!(w.data, [1, 0xb0, 0x09, 1, 0, 65, 3]);
        round::<Option<Stack>, Slot>(Some(bottle));
        let mut quill = Stack::new(1099, 1);
        quill.added.push((
            component::WRITABLE_BOOK_CONTENT,
            Component::WritableBook {
                pages: vec!["a".into(), String::new()],
                filtered: vec![None, Some("b".into())],
            },
        ));
        round::<Option<Stack>, Slot>(Some(quill));
        assert_eq!(sword.damage(), 17);
        round::<Option<Stack>, Slot>(Some(sword.clone()));
        round::<Option<Hashed>, HashedSlot>(Some(Hashed::of(&sword)));
        round::<Option<Hashed>, HashedSlot>(None);
    }

    #[test]
    fn fireworks_components_round_trip() {
        let mut rocket = Stack::new(42, 1);
        rocket.added.push((
            component::FIREWORKS,
            Component::Fireworks {
                flight: 3,
                explosions: vec![
                    FireworkExplosion {
                        shape: 4,
                        colors: vec![0xff0000, 0x00ff00],
                        fade: vec![0x0000ff],
                        trail: true,
                        twinkle: false,
                    },
                    FireworkExplosion {
                        shape: 0,
                        colors: vec![],
                        fade: vec![],
                        trail: false,
                        twinkle: true,
                    },
                ],
            },
        ));
        rocket.added.push((
            component::FIREWORK_EXPLOSION,
            Component::FireworkExplosion(FireworkExplosion {
                shape: 2,
                colors: vec![7],
                fade: vec![],
                trail: false,
                twinkle: false,
            }),
        ));
        round::<Option<Stack>, Slot>(Some(rocket));
    }

    #[test]
    fn charged_projectiles_are_item_stack_templates() {
        // A crossbow (id 7) loaded with one arrow (id 9): ChargedProjectiles
        // is a list of ItemStackTemplate, the item before the count.
        let bytes = [1, 7, 1, 0, 51, 1, 9, 1, 0, 0];
        let stack = Slot::read(&mut Reader::new(&bytes)).unwrap().unwrap();
        assert_eq!(
            stack.component(component::CHARGED_PROJECTILES),
            Some(&Component::Stacks(vec![Stack::new(9, 1)]))
        );
        let mut w = Writer::new();
        Slot::write(&Some(stack), &mut w);
        assert_eq!(w.data, bytes);
    }

    #[test]
    fn bundle_contents_are_item_stack_templates() {
        // A bundle (id 7) holding 32 of item 9: BundleContents is a list of
        // ItemStackTemplate, as ChargedProjectiles is.
        let bytes = [1, 7, 1, 0, 52, 1, 9, 32, 0, 0];
        let stack = Slot::read(&mut Reader::new(&bytes)).unwrap().unwrap();
        assert_eq!(
            stack.component(component::BUNDLE_CONTENTS),
            Some(&Component::Stacks(vec![Stack::new(9, 32)]))
        );
        let mut w = Writer::new();
        Slot::write(&Some(stack), &mut w);
        assert_eq!(w.data, bytes);
    }

    #[test]
    fn filtered_book_pages_are_read_and_kept() {
        // A writable book with one page "hi" and its filtered "**".
        let bytes = [1, 7, 1, 0, 56, 1, 2, b'h', b'i', 1, 2, b'*', b'*'];
        let stack = Slot::read(&mut Reader::new(&bytes)).unwrap().unwrap();
        assert_eq!(
            stack.component(component::WRITABLE_BOOK_CONTENT),
            Some(&Component::WritableBook {
                pages: vec!["hi".into()],
                filtered: vec![Some("**".into())],
            })
        );
    }

    #[test]
    fn empty_stack_is_a_zero_count() {
        let mut w = Writer::new();
        Slot::write(&None, &mut w);
        assert_eq!(w.data, [0]);
        let mut w = Writer::new();
        Slot::write(&Some(Stack::new(1, 2)), &mut w);
        assert_eq!(w.data, [2, 1, 0, 0]);
    }

    #[test]
    fn unknown_components_are_errors() {
        // 122 is past DataComponents' last id (cushion_color, 121).
        let bytes = [1, 5, 1, 0, 122];
        assert!(Slot::read(&mut Reader::new(&bytes)).is_err());
    }

    #[test]
    fn attack_range_and_profiles_round_trip() {
        let mut sword = Stack::new(900, 1);
        sword.added.push((
            component::ATTACK_RANGE,
            Component::AttackRange([0.0, 3.0, 0.0, 4.0, 0.1, 1.0]),
        ));
        sword.added.push((
            component::PROFILE,
            Component::Profile(Profile {
                name: Some("Player53470".into()),
                id: Some(0x7a5d_6a1a_30d7_4187_88c5_efe9_1201_8922),
                properties: vec![("textures".into(), "dGV4dHVyZXM=".into(), None)],
                skin: SkinPatch {
                    body: Some("minecraft:entity/player/wide/steve".into()),
                    cape: None,
                    elytra: None,
                    slim: Some(true),
                },
            }),
        ));
        round::<Option<Stack>, Slot>(Some(sword));
    }

    #[test]
    fn a_partial_profile_decodes_from_the_wire() {
        // count 1, item 1, one added component of kind 72 (profile): a partial
        // profile with an empty name, a UUID, one unsigned texture property
        // and an empty skin patch.
        let uuid = [
            0x7a, 0x5d, 0x6a, 0x1a, 0x30, 0xd7, 0x41, 0x87, 0x88, 0xc5, 0xef, 0xe9, 0x12, 0x01,
            0x89, 0x22,
        ];
        let mut bytes = vec![1, 1, 1, 0, 72, 0, 1, 0, 1];
        bytes.extend_from_slice(&uuid);
        bytes.extend_from_slice(&[1, 8]);
        bytes.extend_from_slice(b"textures");
        bytes.extend_from_slice(&[1, b'x', 0, 0, 0, 0, 0]);
        let mut r = Reader::new(&bytes);
        let stack = Slot::read(&mut r).unwrap().unwrap();
        assert_eq!(r.remaining(), 0);
        let Some(Component::Profile(profile)) = stack.component(component::PROFILE) else {
            panic!("the profile component did not decode");
        };
        assert_eq!(profile.name.as_deref(), Some(""));
        assert_eq!(profile.id, Some(0x7a5d_6a1a_30d7_4187_88c5_efe9_1201_8922));
        assert_eq!(
            profile.properties,
            [("textures".to_owned(), "x".to_owned(), None)]
        );
    }

    #[test]
    fn components_read_past_keep_their_bytes() {
        // Vanilla 26.3's encoding (tools/test_item_wire_parity.py) of a chestplate's
        // armour trim (decoded: both holders are references), a banner's patterns
        // (one direct), a pot's sherds and a stew's effects: one server's
        // inventory, recipe book and advancements had them.
        for (hex, kind) in [
            ("01bd0801003a0502", 58),
            ("01890b01004a02200e000b6d696e6563726166743a7801790b", 74),
            ("01880301004c01f60801000001be0c0100000001d30c010000", 76),
            ("01a408010037020f6412a001", 55),
            // A goat horn's instrument by reference (InstrumentComponent's holder).
            ("01bd0801003f02", 63),
        ] {
            let bytes: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let mut r = Reader::new(&bytes);
            let stack = Slot::read(&mut r).unwrap();
            assert_eq!(r.remaining(), 0, "component {kind}");
            let expected = |c: &Component| match kind {
                58 => {
                    *c == Component::Trim {
                        material: 4,
                        pattern: 1,
                    }
                }
                // night_vision for 100 ticks, poison for the default 160.
                55 => *c == Component::StewEffects(vec![(15, 100), (18, 160)]),
                63 => *c == Component::Instrument(1),
                _ => matches!(c, Component::Raw(_)),
            };
            assert!(
                stack
                    .as_ref()
                    .and_then(|s| s.component(kind))
                    .is_some_and(expected)
            );
            let mut w = Writer::new();
            Slot::write(&stack, &mut w);
            assert_eq!(w.data, bytes, "component {kind}");
        }
    }

    /// A stack with one `kind` component, hand-encoded after its vanilla codec
    /// by `value`, then a plain stack.
    fn with_component(kind: i32, value: impl FnOnce(&mut Writer)) -> (Vec<u8>, Vec<u8>) {
        let mut v = Writer::new();
        value(&mut v);
        let mut w = Writer::new();
        for x in [1, 900, 1, 0, kind] {
            w.varint(x);
        }
        w.bytes(&v.data);
        (v.data, w.data)
    }
    /// The component must come back as exactly its bytes, re-encode
    /// unchanged, end where the codec ends (the next stack still decodes) and
    /// need every one of its bytes.
    fn raw_round_trip(kind: i32, value: impl FnOnce(&mut Writer)) {
        let (bytes, mut data) = with_component(kind, value);
        let end = data.len();
        let mut w = Writer::new();
        Slot::write(&Some(Stack::new(7, 3)), &mut w);
        data.extend_from_slice(&w.data);

        let mut r = Reader::new(&data);
        let stack = Slot::read(&mut r).unwrap().unwrap();
        assert_eq!(stack.added, [(kind, Component::Raw(bytes))]);
        same_bytes_after(stack, r, &data, end);
    }
    /// As [`raw_round_trip`], for a component the codec reads into its own type: it re-encodes
    /// to the same bytes.
    fn typed_round_trip(kind: i32, value: impl FnOnce(&mut Writer)) {
        let (_, mut data) = with_component(kind, value);
        let end = data.len();
        let mut w = Writer::new();
        Slot::write(&Some(Stack::new(7, 3)), &mut w);
        data.extend_from_slice(&w.data);

        let mut r = Reader::new(&data);
        let stack = Slot::read(&mut r).unwrap().unwrap();
        assert!(!matches!(stack.added[..], [(_, Component::Raw(_))]));
        same_bytes_after(stack, r, &data, end);
    }
    fn same_bytes_after(stack: Stack, mut r: Reader<'_>, data: &[u8], end: usize) {
        assert_eq!(Slot::read(&mut r).unwrap(), Some(Stack::new(7, 3)));
        assert_eq!(r.remaining(), 0);
        let mut back = Writer::new();
        Slot::write(&Some(stack), &mut back);
        assert_eq!(back.data, data[..end]);
        assert!(Slot::read(&mut Reader::new(&data[..end - 1])).is_err());
    }
    fn refused(kind: i32, value: impl FnOnce(&mut Writer)) {
        let (_, data) = with_component(kind, value);
        assert!(Slot::read(&mut Reader::new(&data)).is_err(), "{kind}");
    }

    fn text(w: &mut Writer, s: &str) {
        w.nbt(&literal(s));
    }
    /// SoundEvent.STREAM_CODEC by registry id.
    fn sound_id(w: &mut Writer, id: i32) {
        w.varint(id + 1);
    }
    /// SoundEvent.STREAM_CODEC inline, with a fixed range.
    fn sound_inline(w: &mut Writer, location: &str) {
        w.varint(0);
        w.string(location);
        w.bool(true);
        w.f32(16.0);
    }
    /// MobEffectInstance.STREAM_CODEC hiding a weaker instance.
    fn effect(w: &mut Writer, id: i32) {
        w.varint(id);
        for details in [(1, 600, true), (0, 1200, false)] {
            w.varint(details.0);
            w.varint(details.1);
            w.bool(false);
            w.bool(true);
            w.bool(true);
            w.bool(details.2);
        }
    }
    /// ItemStackTemplate.STREAM_CODEC with an empty patch.
    fn template(w: &mut Writer, item: i32, count: i32) {
        for x in [item, count, 0, 0] {
            w.varint(x);
        }
    }

    #[test]
    fn food_and_simple_values() {
        raw_round_trip(component::FOOD, |w| {
            w.varint(4);
            w.f32(2.4);
            w.bool(false);
        });
        raw_round_trip(component::USE_EFFECTS, |w| {
            w.bool(true);
            w.bool(false);
            w.f32(0.2);
        });
        raw_round_trip(component::WEAPON, |w| {
            w.varint(1);
            w.f32(5.0);
        });
        raw_round_trip(component::MINIMUM_ATTACK_CHARGE, |w| w.f32(0.9));
        // A shield's base colour, read as its dye id (upstream's Component::Int).
        typed_round_trip(component::BASE_COLOR, |w| w.varint(11));
        // A painting variant by reference (upstream's Component::PaintingVariant).
        typed_round_trip(component::PAINTING_VARIANT, |w| w.varint(4));
        raw_round_trip(component::TOOLTIP_STYLE, |w| w.string("minecraft:fancy"));
        raw_round_trip(component::ATTACK_ANIMATION, |w| {
            w.varint(2);
            w.varint(10);
        });
        raw_round_trip(component::USE_COOLDOWN, |w| {
            w.f32(1.5);
            w.bool(true);
            w.string("minecraft:ender_pearl");
        });
        raw_round_trip(component::COOKING_FUEL, |w| {
            w.bool(true);
            w.i32(200);
            w.bool(false);
            w.string("minecraft:furnace_speed");
        });
        // Read into upstream's Component::StewEffects.
        typed_round_trip(component::SUSPICIOUS_STEW_EFFECTS, |w| {
            w.varint(1);
            w.varint(15);
            w.varint(160);
        });
    }

    #[test]
    fn profiles_read_their_whole_skin_patch() {
        // A full GameProfile with a signed textures property, then a
        // PlayerSkin.Patch: a body texture, no cape, an elytra texture (one
        // Identifier each) and the slim model (a bool).
        let (_, mut data) = with_component(component::PROFILE, |w| {
            w.bool(true);
            w.uuid(0x069a_79f4_44e9_4726_a5be_fca9_0e38_aaf5);
            w.string("Notch");
            w.varint(1);
            w.string("textures");
            w.string("eyJ0ZXh0dXJlcyI6e319");
            w.bool(true);
            w.string("c2lnbmF0dXJl");
            w.bool(true);
            w.string("minecraft:entity/player/wide/steve");
            w.bool(false);
            w.bool(true);
            w.string("minecraft:entity/equipment/wings/elytra");
            w.bool(true);
            w.bool(true);
        });
        let mut w = Writer::new();
        Slot::write(&Some(Stack::new(7, 3)), &mut w);
        data.extend_from_slice(&w.data);
        let mut r = Reader::new(&data);
        let stack = Slot::read(&mut r).unwrap().unwrap();
        assert_eq!(
            stack.component(component::PROFILE),
            Some(&Component::Profile(Profile {
                name: Some("Notch".into()),
                id: Some(0x069a_79f4_44e9_4726_a5be_fca9_0e38_aaf5),
                properties: vec![(
                    "textures".into(),
                    "eyJ0ZXh0dXJlcyI6e319".into(),
                    Some("c2lnbmF0dXJl".into()),
                )],
                skin: SkinPatch {
                    body: Some("minecraft:entity/player/wide/steve".into()),
                    cape: None,
                    elytra: Some("minecraft:entity/equipment/wings/elytra".into()),
                    slim: Some(true),
                },
            }))
        );
        assert_eq!(Slot::read(&mut r).unwrap(), Some(Stack::new(7, 3)));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn a_stack_names_its_own_equipment_asset() {
        let mut w = Writer::new();
        w.varint(4);
        sound_inline(&mut w, "minecraft:item.armor.equip_generic");
        w.bool(true);
        w.string("nexo:ruby");
        w.bool(false);
        let mut stack = Stack::new(7, 1);
        stack
            .added
            .push((component::EQUIPPABLE, Component::Raw(w.data)));
        assert_eq!(
            stack.equippable(),
            Some(Equippable {
                slot: 4,
                asset: Some("nexo:ruby".into()),
                overlay: None,
            })
        );
        assert_eq!(Stack::new(7, 1).equippable(), None);
    }

    #[test]
    fn equippable_and_consumable() {
        raw_round_trip(component::EQUIPPABLE, |w| {
            w.varint(5);
            sound_id(w, 120);
            w.bool(true);
            w.string("minecraft:diamond");
            w.bool(false);
            w.bool(true);
            w.varint(0);
            w.string("minecraft:can_wear_horse_armor");
            for b in [true, true, true, false, false] {
                w.bool(b);
            }
            sound_inline(w, "minecraft:item.shears.snip");
        });
        raw_round_trip(component::CONSUMABLE, |w| {
            w.f32(1.6);
            w.varint(1);
            sound_id(w, 300);
            w.bool(true);
            w.varint(5);
            // apply_effects
            w.varint(0);
            w.varint(1);
            effect(w, 3);
            w.f32(0.5);
            // remove_effects: two ids
            w.varint(1);
            w.varint(3);
            w.varint(4);
            w.varint(9);
            // clear_all_effects
            w.varint(2);
            // teleport_randomly
            w.varint(3);
            w.f32(16.0);
            w.bool(true);
            // play_sound
            w.varint(4);
            sound_inline(w, "minecraft:entity.generic.burp");
        });
        raw_round_trip(component::DEATH_PROTECTION, |w| {
            w.varint(1);
            w.varint(2);
        });
        // An unknown consume effect type.
        refused(component::DEATH_PROTECTION, |w| {
            w.varint(1);
            w.varint(5);
        });
    }

    #[test]
    fn tools_weapons_and_attributes() {
        raw_round_trip(component::TOOL, |w| {
            w.varint(2);
            // A block tag that denies drops.
            w.varint(0);
            w.string("minecraft:incorrect_for_wooden_tool");
            w.bool(false);
            w.bool(true);
            w.bool(false);
            // Two block ids, mined at 8 with drops.
            w.varint(3);
            w.varint(10);
            w.varint(20);
            w.bool(true);
            w.f32(8.0);
            w.bool(true);
            w.bool(true);
            w.f32(1.0);
            w.varint(1);
            w.bool(true);
        });
        raw_round_trip(component::ATTRIBUTE_MODIFIERS, |w| {
            w.varint(3);
            for (attribute, id, amount, display) in [
                (3, "minecraft:base_attack_damage", 5.0, 0),
                (5, "minecraft:base_attack_speed", -2.4, 1),
                (1, "example:armor", 0.5, 2),
            ] {
                w.varint(attribute);
                w.string(id);
                w.f64(amount);
                w.varint(0);
                w.varint(1);
                w.varint(display);
            }
            text(w, "+50% armor");
        });
        raw_round_trip(component::BLOCKS_ATTACKS, |w| {
            w.f32(0.25);
            w.f32(1.0);
            w.varint(1);
            w.f32(90.0);
            w.bool(true);
            w.varint(0);
            w.string("minecraft:is_projectile");
            w.f32(0.0);
            w.f32(1.0);
            [3.0, 1.0, 1.0].into_iter().for_each(|f| w.f32(f));
            w.bool(true);
            w.varint(0);
            w.string("minecraft:bypasses_shield");
            w.bool(true);
            sound_id(w, 1);
            w.bool(false);
        });
        raw_round_trip(component::KINETIC_WEAPON, |w| {
            w.varint(10);
            w.varint(5);
            w.bool(true);
            w.varint(100);
            w.f32(1.0);
            w.f32(0.5);
            w.bool(false);
            w.bool(true);
            w.varint(200);
            w.f32(2.0);
            w.f32(0.0);
            w.f32(0.38);
            w.f32(1.2);
            w.bool(true);
            sound_inline(w, "minecraft:item.spear.use");
            w.bool(false);
        });
        raw_round_trip(component::PIERCING_WEAPON, |w| {
            w.bool(true);
            w.bool(false);
            w.bool(false);
            w.bool(true);
            sound_id(w, 4);
        });
    }

    #[test]
    fn nesting_past_the_limit_is_refused() {
        // Charged projectiles, each holding the next: a crossbow in a crossbow, 65 deep.
        fn crossbows(w: &mut Writer, depth: u32) {
            w.varint(1); // item
            w.varint(1); // count
            if depth == 0 {
                w.varint(0);
                w.varint(0);
                return;
            }
            w.varint(1);
            w.varint(0);
            w.varint(component::CHARGED_PROJECTILES);
            w.varint(1);
            crossbows(w, depth - 1);
        }
        for (depth, ok) in [(MAX_NESTING - 1, true), (MAX_NESTING, false)] {
            let mut w = Writer::new();
            crossbows(&mut w, depth);
            let read = Template::read(&mut Reader::new(&w.data));
            assert_eq!(read.is_ok(), ok, "depth {depth}: {read:?}");
        }
        // A refusal leaves nothing behind for the next packet.
        NESTING.with(|n| assert_eq!(n.get(), 0));
    }

    #[test]
    fn bundles_and_containers_hold_nested_stacks() {
        typed_round_trip(component::BUNDLE_CONTENTS, |w| {
            w.varint(2);
            template(w, 42, 3);
            // A damaged, named stack holding its own bundle, minus a component.
            for x in [43, 1, 3, 1, component::DAMAGE, 5, component::CUSTOM_NAME] {
                w.varint(x);
            }
            text(w, "inner");
            w.varint(component::BUNDLE_CONTENTS);
            w.varint(1);
            template(w, 1, 1);
            w.varint(component::REPAIR_COST);
        });
        raw_round_trip(component::CONTAINER, |w| {
            w.varint(3);
            w.bool(true);
            template(w, 1, 64);
            w.bool(false);
            w.bool(true);
            template(w, 2, 1);
        });
        raw_round_trip(component::POT_DECORATIONS, |w| {
            w.bool(true);
            template(w, 5, 1);
            w.bool(false);
            w.bool(false);
            w.bool(true);
            template(w, 6, 1);
        });
        raw_round_trip(component::USE_REMAINDER, |w| template(w, 9, 1));
    }

    #[test]
    fn direct_trims_banners_and_songs() {
        // A trim with a direct holder keeps its bytes.
        raw_round_trip(component::TRIM, |w| {
            w.varint(3);
            w.varint(0);
            w.string("minecraft:sentry");
            text(w, "Sentry Armor Trim");
            w.bool(false);
        });
        raw_round_trip(component::TRIM, |w| {
            w.varint(0);
            w.string("minecraft:amethyst");
            text(w, "Amethyst Material");
            w.varint(1);
        });
        raw_round_trip(component::BANNER_PATTERNS, |w| {
            w.varint(2);
            w.varint(6);
            w.varint(14);
            w.varint(0);
            w.string("minecraft:creeper");
            w.string("block.minecraft.banner.creeper");
            w.varint(0);
        });
        raw_round_trip(component::PROVIDES_BANNER_PATTERNS, |w| {
            w.varint(0);
            w.string("minecraft:pattern_item/globe");
        });
        raw_round_trip(component::INSTRUMENT, |w| {
            w.varint(0);
            sound_inline(w, "minecraft:item.goat_horn.sound.0");
            w.f32(7.0);
            w.f32(256.0);
            w.varint(0);
            text(w, "Ponder");
        });
        raw_round_trip(component::JUKEBOX_PLAYABLE, |w| w.varint(4));
    }

    #[test]
    fn adventure_mode_predicates() {
        raw_round_trip(component::CAN_BREAK, |w| {
            w.varint(2);
            // Blocks, state properties, NBT and component matchers.
            w.bool(true);
            w.varint(2);
            w.varint(1);
            w.bool(true);
            w.varint(2);
            w.string("facing");
            w.bool(true);
            w.string("north");
            w.string("age");
            w.bool(false);
            w.bool(true);
            w.string("1");
            w.bool(false);
            w.bool(true);
            w.nbt(&Tag::compound([("Lock", literal("key"))]));
            w.varint(2);
            w.varint(component::DAMAGE);
            w.varint(0);
            w.varint(component::CUSTOM_NAME);
            text(w, "named");
            w.varint(2);
            w.bool(true);
            w.varint(3);
            w.nbt(&Tag::compound([("min", Tag::Int(1))]));
            w.bool(false);
            w.varint(component::ENCHANTMENTS);
            w.nbt(&Tag::Compound(Default::default()));
            // A block tag alone.
            w.bool(true);
            w.varint(0);
            w.string("minecraft:logs");
            w.bool(false);
            w.bool(false);
            w.varint(0);
            w.varint(0);
        });
        raw_round_trip(component::CAN_PLACE_ON, |w| w.varint(0));
    }

    #[test]
    fn block_and_entity_data() {
        let tag = Tag::compound([("id", literal("minecraft:pig")), ("Age", Tag::Int(3))]);
        raw_round_trip(component::ENTITY_DATA, |w| {
            w.varint(12);
            w.nbt(&tag);
        });
        raw_round_trip(component::BLOCK_ENTITY_DATA, |w| {
            w.varint(7);
            w.nbt(&tag);
        });
        raw_round_trip(component::BUCKET_ENTITY_DATA, |w| w.nbt(&tag));
        raw_round_trip(component::BEES, |w| {
            w.varint(1);
            w.varint(9);
            w.nbt(&tag);
            w.varint(100);
            w.varint(600);
        });
        raw_round_trip(component::BLOCK_STATE, |w| {
            w.varint(2);
            for s in ["facing", "east", "lit", "true"] {
                w.string(s);
            }
        });
        raw_round_trip(component::SIGN_TEXT_FRONT, |w| {
            ["a", "b", "c", "d"].into_iter().for_each(|l| text(w, l));
            w.bool(true);
            ["a", "*", "c", "d"].into_iter().for_each(|l| text(w, l));
            w.varint(15);
            w.bool(true);
        });
        typed_round_trip(component::LODESTONE_TRACKER, |w| {
            w.bool(true);
            w.string("minecraft:overworld");
            w.i64(crate::buf::pack_block_pos([10, 64, -5]));
            w.bool(true);
        });
        // Persistent-only types go as their codec's NBT (fromCodecWithRegistries).
        raw_round_trip(component::LOCK, |w| w.nbt(&tag));
        raw_round_trip(component::CONTAINER_LOOT, |w| {
            w.nbt(&Tag::compound([(
                "loot_table",
                literal("minecraft:chests/x"),
            )]));
        });
    }

    #[test]
    fn painting_variants_decode_by_reference() {
        // Holder<PaintingVariant>: a reference (its id plus one) is typed; a
        // direct variant (0, width, height, asset id, no title, no author)
        // is read past. Both encode back to the same bytes.
        let asset = b"minecraft:x";
        let mut direct = vec![1u8, 7, 1, 0, 109, 0, 2, 1, asset.len() as u8];
        direct.extend_from_slice(asset);
        direct.extend_from_slice(&[0, 0]);
        for (bytes, expected) in [
            (
                vec![1u8, 7, 1, 0, 109, 5],
                Some(Component::PaintingVariant(4)),
            ),
            (direct, None),
        ] {
            let mut r = Reader::new(&bytes);
            let stack = Slot::read(&mut r).unwrap();
            assert_eq!(r.remaining(), 0);
            let c = stack
                .as_ref()
                .and_then(|s| s.component(component::PAINTING_VARIANT))
                .unwrap();
            match &expected {
                Some(e) => assert_eq!(c, e),
                None => assert!(matches!(c, Component::Raw(_))),
            }
            let mut w = Writer::new();
            Slot::write(&stack, &mut w);
            assert_eq!(w.data, bytes);
        }
    }
}
