//! A content mod's fetched assets (Starling; a manifest's `[assets.fetch]`). A port of a Java mod
//! whose code may be shared but whose assets its authors keep (Create: MIT code, All Rights
//! Reserved textures, models and sounds) ships without them. The game downloads the very file the
//! player could get themselves, the mod's jar on Modrinth, checks it by its SHA-512 and unpacks
//! the folders the manifest names into its cache, where the mod's content reads them as its own
//! ([`crate::content::Content::read_with`]). The jar's code never runs; it is deleted once
//! unpacked. The download itself is the game's (`game/mod_assets.rs`), this the rest.
use crate::content::{MAX_FILE, MAX_FILES, MAX_TOTAL, check_path, without_comments};
use crate::manifest::{Fetch, Manifest};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// The cache's folder of unpacked assets, one folder per file.
pub const DIR: &str = "mod-assets";
/// In an unpacked folder: the file's SHA-512, written last.
const DONE: &str = ".horizon-fetched";

/// Where the assets of `fetch` are unpacked in `cache`.
pub fn dir(cache: &Path, fetch: &Fetch) -> PathBuf {
    cache.join(DIR).join(&fetch.sha512[..32])
}

/// Whether `dir` ([`dir`]) holds the assets of `fetch`, unpacked to the end.
pub fn ready(dir: &Path, fetch: &Fetch) -> bool {
    fs::read_to_string(dir.join(DONE)).is_ok_and(|s| s.trim() == fetch.sha512)
}

/// Modrinth's API: the version the file with this SHA-512 belongs to.
pub fn version_url(fetch: &Fetch) -> String {
    format!(
        "https://api.modrinth.com/v2/version_file/{}?algorithm=sha512",
        fetch.sha512
    )
}

/// In Modrinth's answer for [`version_url`], the file's link: the one with its hash, on
/// Modrinth's own host.
pub fn file_url(version: &serde_json::Value, fetch: &Fetch) -> Result<String, String> {
    let files = version["files"]
        .as_array()
        .ok_or("Modrinth sent no files")?;
    let file = files
        .iter()
        .find(|f| f["hashes"]["sha512"].as_str() == Some(fetch.sha512.as_str()))
        .ok_or_else(|| format!("Modrinth has no file {}", fetch.file))?;
    let url = file["url"].as_str().unwrap_or_default();
    if !url.starts_with("https://cdn.modrinth.com/") {
        return Err(format!(
            "Modrinth's link for {} is elsewhere: {url}",
            fetch.file
        ));
    }
    Ok(url.to_owned())
}

/// Unpacks `jar`, the file of `manifest`'s `[assets.fetch]` (checked by its hash already), into
/// `out`: the entries under its `take` folders, as `horizon-port-pack carry` carries them. Paths
/// the game would not read are left out, JSON loses its byte-order mark and comments, and item
/// models of the mod's own namespace get the `items/` definitions 26.3 looks for. Returns how
/// many files it wrote. A folder half written is never taken for one done.
pub fn unpack(jar: &Path, manifest: &Manifest, out: &Path) -> Result<usize, String> {
    let Some(fetch) = manifest.assets.as_ref().map(|a| &a.fetch) else {
        return Err(format!("{} fetches no assets", manifest.id));
    };
    let file = fs::File::open(jar).map_err(|e| format!("{}: {e}", jar.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", fetch.file))?;
    let partial = out.with_extension("partial");
    let _ = fs::remove_dir_all(&partial);
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let path = partial.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    };
    let models = format!("assets/{}/models/item/", manifest.id);
    let mut item_models = Vec::new();
    let mut written = BTreeSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.into_owned();
        if entry.is_dir() || !fetch.take.iter().any(|t| name.starts_with(t.as_str())) {
            continue;
        }
        // Vanilla skips a path it cannot read (upper case: Create's lang/README.md).
        if check_path(&name).is_err() || entry.size() > MAX_FILE {
            continue;
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_FILE)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("{name}: {e}"))?;
        if (name.ends_with(".json") || name.ends_with(".mcmeta")) && !clean_json(&mut bytes) {
            continue;
        }
        total += bytes.len() as u64;
        if total > MAX_TOTAL || written.len() >= MAX_FILES {
            return Err(format!("{} holds more than a mod may", fetch.file));
        }
        if let Some(model) = name
            .strip_prefix(&models)
            .and_then(|m| m.strip_suffix(".json"))
        {
            item_models.push(model.to_owned());
        }
        write(&name, &bytes)?;
        written.insert(name);
    }
    // 1.21.4 moved item models behind `items/<name>.json` definitions.
    for model in item_models {
        let id = &manifest.id;
        let definition = format!("assets/{id}/items/{model}.json");
        if !written.contains(&definition) {
            let v = serde_json::json!({
                "model": {"type": "minecraft:model", "model": format!("{id}:item/{model}")}
            });
            write(
                &definition,
                &serde_json::to_vec_pretty(&v).unwrap_or_default(),
            )?;
            written.insert(definition);
        }
    }
    write(DONE, fetch.sha512.as_bytes())?;
    let _ = fs::remove_dir_all(out);
    fs::rename(&partial, out).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(written.len())
}

/// A JSON file as the game reads it: without a byte-order mark, and without comments where it
/// has them (Gson's lenient reader skips them). False when it does not parse even so.
fn clean_json(bytes: &mut Vec<u8>) -> bool {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        bytes.drain(..3);
    }
    if serde_json::from_slice::<serde::de::IgnoredAny>(bytes).is_ok() {
        return true;
    }
    match serde_json::from_slice::<serde_json::Value>(&without_comments(bytes)) {
        Ok(v) => {
            *bytes = serde_json::to_vec_pretty(&v).unwrap_or_default();
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn manifest(sha512: &str) -> Manifest {
        Manifest::parse(&format!(
            r#"id = "gem"
name = "Gem"
version = "1"
api = "0.2"
capabilities = ["registries"]
[assets.fetch]
modrinth = "gem"
version = "1.0+mc1.21.1"
file = "gem-1.0.jar"
size = 1000
sha512 = "{sha512}"
take = ["assets/gem/"]
"#
        ))
        .unwrap()
    }

    #[test]
    fn manifests_name_one_file_and_their_own_folders() {
        let sha = "a".repeat(128);
        let m = manifest(&sha);
        let fetch = &m.assets.as_ref().unwrap().fetch;
        assert_eq!(fetch.take, ["assets/gem/"]);
        assert!(version_url(fetch).ends_with(&format!("{sha}?algorithm=sha512")));
        let text = |sha: &str, take: &str| {
            format!(
                "id = \"gem\"\nname = \"Gem\"\nversion = \"1\"\napi = \"0.2\"\n\
                 capabilities = [\"registries\"]\n[assets.fetch]\nmodrinth = \"gem\"\n\
                 version = \"1\"\nfile = \"gem.jar\"\nsize = 10\nsha512 = \"{sha}\"\n\
                 take = [{take}]\n"
            )
        };
        // Another mod's folder, vanilla's, or a hash that is not one: refused.
        for (sha, take) in [
            (sha.as_str(), r#""assets/other/""#),
            (sha.as_str(), r#""assets/minecraft/""#),
            (sha.as_str(), r#""data/gem/""#),
            ("abc", r#""assets/gem/""#),
        ] {
            assert!(Manifest::parse(&text(sha, take)).is_err(), "{take} {sha}");
        }
        assert!(Manifest::parse(&text(&sha, r#""assets/gem/""#)).is_ok());
    }

    #[test]
    fn modrinths_link_is_the_file_with_the_hash() {
        let sha = "b".repeat(128);
        let m = manifest(&sha);
        let fetch = &m.assets.as_ref().unwrap().fetch;
        let answer = serde_json::json!({"files": [
            {"hashes": {"sha512": "c".repeat(128)}, "url": "https://cdn.modrinth.com/other.jar"},
            {"hashes": {"sha512": sha}, "url": "https://cdn.modrinth.com/data/x/gem-1.0.jar"},
        ]});
        assert_eq!(
            file_url(&answer, fetch).unwrap(),
            "https://cdn.modrinth.com/data/x/gem-1.0.jar"
        );
        let elsewhere = serde_json::json!({"files": [
            {"hashes": {"sha512": sha}, "url": "https://example.com/gem-1.0.jar"},
        ]});
        assert!(file_url(&elsewhere, fetch).is_err());
    }

    #[test]
    fn unpacks_the_folders_taken_as_the_carry_does() {
        let tmp = std::env::temp_dir().join(format!("horizon-fetch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let jar = tmp.join("gem-1.0.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("com/gem/Gem.class", "code"),
            ("assets/gem/textures/item/gem.png", "png"),
            (
                "assets/gem/models/item/gem.json",
                "\u{feff}{\"parent\": \"item/generated\"}",
            ),
            (
                "assets/gem/models/item/ruby.json",
                "{// a comment\n\"parent\": \"x\"}",
            ),
            ("assets/gem/items/ruby.json", "{\"model\": {}}"),
            ("assets/gem/lang/README.md", "upper case"),
            ("assets/minecraft/atlases/blocks.json", "{}"),
            ("data/gem/recipe/gem.json", "{}"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let m = manifest(&"d".repeat(128));
        let fetch = &m.assets.as_ref().unwrap().fetch;
        let out = dir(&tmp, fetch);
        assert!(!ready(&out, fetch));
        // png, two models, ruby's own definition and the one made for gem.
        assert_eq!(unpack(&jar, &m, &out).unwrap(), 5);
        assert!(ready(&out, fetch));
        let read = |name: &str| fs::read_to_string(out.join(name)).ok();
        assert_eq!(
            read("assets/gem/textures/item/gem.png").as_deref(),
            Some("png")
        );
        assert!(
            read("assets/gem/models/item/gem.json")
                .unwrap()
                .starts_with('{')
        );
        assert!(
            !read("assets/gem/models/item/ruby.json")
                .unwrap()
                .contains("//")
        );
        assert!(
            read("assets/gem/items/gem.json")
                .unwrap()
                .contains("gem:item/gem")
        );
        assert_eq!(
            read("assets/gem/items/ruby.json").as_deref(),
            Some("{\"model\": {}}")
        );
        for left in [
            "com/gem/Gem.class",
            "assets/gem/lang/README.md",
            "assets/minecraft/atlases/blocks.json",
            "data/gem/recipe/gem.json",
        ] {
            assert!(read(left).is_none(), "{left}");
        }
        let _ = fs::remove_dir_all(&tmp);
    }
}
