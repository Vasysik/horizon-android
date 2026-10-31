//! Identity of the ordered block-state registry used by chunk packets, and of
//! the item and mob effect registries once content mods add to them (item
//! stacks and effect packets carry their network ids).
use horizon_protocol::registry::Builtin;
use serde::{Deserialize, Serialize, de::IgnoredAny};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fmt, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

pub const CHANNEL: &str = "horizon:block_registry";
pub const MARKER: &str = "horizon-content-blocks";
pub const MISMATCH: &str =
    "Content registry mismatch: install the same content mods as the server.";

#[derive(Debug)]
pub enum IdentityError {
    Read(io::Error),
    Json(serde_json::Error),
    Registries(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "cannot read facts.json: {error}"),
            Self::Json(error) => write!(f, "invalid facts.json: {error}"),
            Self::Registries(error) => write!(f, "invalid registries.json: {error}"),
        }
    }
}

impl std::error::Error for IdentityError {}

#[derive(Deserialize)]
struct Facts {
    states: Vec<StateRow>,
}

#[derive(Deserialize)]
struct StateRow(IgnoredAny, IgnoredAny, IgnoredAny, IgnoredAny, State);

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct State {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, String>>,
}

/// Hash the network order and each state's name and properties, not its gameplay facts.
pub fn identity(root: &Path) -> Result<[u8; 32], IdentityError> {
    // Keep each resource root stable and avoid reparsing its large facts file for every join.
    static IDENTITIES: OnceLock<Mutex<HashMap<PathBuf, [u8; 32]>>> = OnceLock::new();
    let mut identities = IDENTITIES.get_or_init(Default::default).lock().unwrap();
    if let Some(hash) = identities.get(root) {
        return Ok(*hash);
    }
    let bytes = std::fs::read(root.join("facts.json")).map_err(IdentityError::Read)?;
    let mut hash = hash_facts(&bytes)?;
    // A root without mod entries keeps its block-only identity.
    if let Ok(registries) = std::fs::read(root.join("registries.json")) {
        let report = parse_registries(&registries)?;
        for registry in REGISTRIES {
            let entries = mod_entries(&report, registry);
            if !entries.is_empty() {
                hash = hash_entries(hash, registry, &entries);
            }
        }
    }
    identities.insert(root.to_owned(), hash);
    Ok(hash)
}

fn hash_facts(bytes: &[u8]) -> Result<[u8; 32], IdentityError> {
    let facts: Facts = serde_json::from_slice(bytes).map_err(IdentityError::Json)?;
    let mut hash = Sha256::new();
    for row in facts.states {
        let encoded = serde_json::to_vec(&row.4).map_err(IdentityError::Json)?;
        hash.update((encoded.len() as u64).to_le_bytes());
        hash.update(encoded);
    }
    Ok(hash.finalize().into())
}

/// The built-in registries whose network ids content mods extend besides blocks.
const REGISTRIES: [&str; 2] = ["minecraft:item", "minecraft:mob_effect"];

fn parse_registries(bytes: &[u8]) -> Result<Builtin, IdentityError> {
    let text = std::str::from_utf8(bytes).map_err(|e| IdentityError::Registries(e.to_string()))?;
    Builtin::parse(text).map_err(|e| IdentityError::Registries(e.to_string()))
}

/// The content mods' entries of `registry` in `registries.json`, by network id.
fn mod_entries(report: &Builtin, registry: &str) -> Vec<(u64, String)> {
    // Names come indexed by network id, so they are already in order.
    report
        .get(registry)
        .into_iter()
        .flat_map(|ids| ids.names.iter().enumerate())
        .filter(|(_, name)| !name.is_empty() && !name.starts_with("minecraft:"))
        .map(|(id, name)| (id as u64, name.clone()))
        .collect()
}

/// The identity so far extended with the mods' entries of one registry.
fn hash_entries(previous: [u8; 32], registry: &str, entries: &[(u64, String)]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(previous);
    hash.update((registry.len() as u64).to_le_bytes());
    hash.update(registry);
    for (id, name) in entries {
        hash.update(id.to_le_bytes());
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name);
    }
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{hash_entries, hash_facts, identity, mod_entries, parse_registries};

    #[test]
    fn mod_items_and_effects_join_the_identity() {
        const EFFECT: &str = "minecraft:mob_effect";
        const ITEM: &str = "minecraft:item";
        let entries = |extra: &str, registry: &str| {
            let report = format!(
                r#"{{"minecraft:mob_effect":{{"entries":{{"minecraft:speed":{{"protocol_id":0}}{extra}}}}},
                "minecraft:item":{{"entries":{{"minecraft:air":{{"protocol_id":0}},"ruby:ruby":{{"protocol_id":1}}}}}}}}"#
            );
            mod_entries(&parse_registries(report.as_bytes()).unwrap(), registry)
        };
        assert!(entries("", EFFECT).is_empty());
        let ruby = entries("", ITEM);
        assert_eq!(ruby, [(1, "ruby:ruby".to_owned())]);
        let glow = entries(r#","ruby:glow":{"protocol_id":40}"#, EFFECT);
        assert_eq!(glow, [(40, "ruby:glow".to_owned())]);
        let zap = entries(r#","ruby:zap":{"protocol_id":40}"#, EFFECT);
        let blocks = [7; 32];
        assert_ne!(
            hash_entries(blocks, EFFECT, &glow),
            hash_entries(blocks, EFFECT, &zap)
        );
        assert_ne!(hash_entries(blocks, EFFECT, &glow), blocks);
        assert_ne!(hash_entries(blocks, ITEM, &ruby), blocks);
        // the same id and name in another registry is other content.
        assert_ne!(
            hash_entries(blocks, ITEM, &glow),
            hash_entries(blocks, EFFECT, &glow)
        );
    }

    #[test]
    fn a_root_without_mod_entries_keeps_its_block_identity() {
        let root = std::env::temp_dir().join(format!(
            "horizon-content-identity-vanilla-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let facts = br#"{"states":[["minecraft:stone",false,false,"Block{minecraft:stone}",{"id":"minecraft:stone"}]]}"#;
        std::fs::write(root.join("facts.json"), facts).unwrap();
        std::fs::write(
            root.join("registries.json"),
            br#"{"minecraft:item":{"entries":{"minecraft:air":{"protocol_id":0}}},
            "minecraft:mob_effect":{"entries":{"minecraft:speed":{"protocol_id":0}}}}"#,
        )
        .unwrap();
        assert_eq!(identity(&root).unwrap(), hash_facts(facts).unwrap());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn identity_tracks_order_and_properties_not_gameplay_data() {
        let facts = |a: &str, b: &str| {
            format!(r#"{{"states":[["x",false,false,"x",{a}],["y",false,false,"y",{b}]]}}"#)
        };
        let a = r#"{"id":"demo:a","properties":{"facing":"north"}}"#;
        let b = r#"{"id":"demo:b"}"#;
        let first = hash_facts(facts(a, b).as_bytes()).unwrap();
        assert_eq!(first, hash_facts(facts(a, b).as_bytes()).unwrap());
        assert_ne!(first, hash_facts(facts(b, a).as_bytes()).unwrap());
        assert_ne!(first, hash_facts(facts(a, a).as_bytes()).unwrap());
        assert_ne!(
            first,
            hash_facts(facts(&a.replace("north", "south"), b).as_bytes()).unwrap()
        );
        assert_ne!(
            first,
            hash_facts(format!(r#"{{"states":[["x",false,false,"x",{a}]]}}"#).as_bytes()).unwrap()
        );
        assert!(hash_facts(br#"{"states":[["x",false,false,"x","minecraft:stone"]]}"#).is_err());
        assert!(hash_facts(br#"{"states":[[]]}"#).is_err());
        assert!(hash_facts(br#"{"states":[["x",false,false,"x",{"id":"x"},0]]}"#).is_err());
    }
}
