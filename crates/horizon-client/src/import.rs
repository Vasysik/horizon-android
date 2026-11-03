//! Files handed to the game from outside its folder (Android's "Open with", share and file
//! picker): what each one is, and putting it where the game finds it.
//!
//! The activity copies a file into the inbox (`inbox_dir`) as `<name>.part` and renames it to
//! `<name>` once complete. `Inbox` installs one complete file at a time on a worker thread and
//! removes it from the inbox, whatever the outcome.
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

/// Where files to import arrive.
pub fn inbox_dir() -> PathBuf {
    crate::logging::private_home().join("import-inbox")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    ResourcePack,
    /// A pack with `data/` and no `assets/`: for `datapacks/`, chosen when a world is created.
    DataPack,
    ShaderPack,
    World,
    Mod,
    /// A native mod (`horizon-native.json`): for `horizon-native-mods/`, compiled from there.
    NativeMod,
    /// A folder horizon-prepare made (`horizon-resources.json`), zipped.
    Resources,
    /// Bedrock Edition content (.mcpack, .mcaddon, .mcworld, .mctemplate).
    Bedrock,
    /// A PNG: a player skin for `skins/` (`own_skin`).
    Skin,
    Unknown,
}

/// What a file is, and the folder inside the archive its content starts at: "" for the root, or
/// "name/" for its only top folder (an archive made by zipping the folder itself).
pub fn classify(path: &Path) -> (Kind, String) {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        ext.as_str(),
        "mcpack" | "mcaddon" | "mcworld" | "mctemplate"
    ) {
        return (Kind::Bedrock, String::new());
    }
    // A picked image may have lost its extension on the way: its first bytes tell.
    let mut head = [0u8; 8];
    if File::open(path).is_ok_and(|mut f| io::Read::read_exact(&mut f, &mut head).is_ok())
        && crate::own_skin::is_png(&head)
    {
        return (Kind::Skin, String::new());
    }
    let Some(names) = File::open(path)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .map(|a| {
            a.file_names()
                .filter_map(|n| n.ok().map(|n| n.into_owned()))
                .collect::<Vec<_>>()
        })
    else {
        return (Kind::Unknown, String::new());
    };
    let found = |marker: &str| marker_prefix(&names, marker);
    if let Some(prefix) = found("horizon-resources.json") {
        return (Kind::Resources, prefix);
    }
    if names
        .iter()
        .any(|n| n == horizon_mods::index::native::MANIFEST)
    {
        return (Kind::NativeMod, String::new());
    }
    if let Some(prefix) = found("horizon-mod.toml") {
        return (Kind::Mod, prefix);
    }
    if ext == "hmod" {
        return (Kind::Mod, String::new());
    }
    if let Some(prefix) = found("level.dat") {
        return (Kind::World, prefix);
    }
    if let Some(prefix) = found("pack.mcmeta") {
        let holds = |top: &str| {
            let at = format!("{prefix}{top}/");
            names.iter().any(|n| n.starts_with(&at))
        };
        let kind = if holds("data") && !holds("assets") {
            Kind::DataPack
        } else {
            Kind::ResourcePack
        };
        return (kind, prefix);
    }
    // OptiFine and Iris packs: a `shaders/` folder.
    if let Some(prefix) = names.iter().find_map(|n| match n.split_once('/') {
        Some(("shaders", _)) => Some(String::new()),
        Some((top, rest)) if rest.starts_with("shaders/") => Some(format!("{top}/")),
        _ => None,
    }) {
        return (Kind::ShaderPack, prefix);
    }
    (Kind::Unknown, String::new())
}

/// The next complete file of the inbox as a custom button's picture, taken out of it: any size,
/// and no skin. None while none has arrived.
pub fn take_picture() -> Option<Result<(u32, u32, Vec<u8>), String>> {
    picture_in(&inbox_dir())
}

fn picture_in(dir: &Path) -> Option<Result<(u32, u32, Vec<u8>), String>> {
    let entry = fs::read_dir(dir).ok()?.flatten().find(|e| {
        !e.file_name().to_string_lossy().ends_with(".part")
            && e.file_type().is_ok_and(|t| t.is_file())
    })?;
    let name = entry.file_name().to_string_lossy().into_owned();
    let decoded = fs::read(entry.path())
        .map_err(|e| format!("could not read {name}: {e}"))
        .and_then(|bytes| {
            crate::hud::decode_bytes(&bytes).map_err(|_| format!("{name} is not a PNG"))
        });
    let _ = fs::remove_file(entry.path());
    Some(decoded)
}

/// `marker` at the archive's root (""), or one folder down ("folder/").
fn marker_prefix(names: &[String], marker: &str) -> Option<String> {
    if names.iter().any(|n| n == marker) {
        return Some(String::new());
    }
    names.iter().find_map(|n| {
        let (top, rest) = n.split_once('/')?;
        (rest == marker).then(|| format!("{top}/"))
    })
}

/// Where imports go: the game folder, and the prepared resources' folder.
#[derive(Clone, Debug)]
pub struct Places {
    pub game: PathBuf,
    pub resources: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub kind: Kind,
    /// What the player reads: what was added, and where to turn it on.
    pub message: String,
    /// The file made, for what the game puts to use at once (a skin is chosen).
    pub path: Option<PathBuf>,
}

/// Put the file `name` (at `path`) where the game finds it.
pub fn install(path: &Path, name: &str, places: &Places) -> Result<Installed, String> {
    let (kind, prefix) = classify(path);
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_owned());
    // An archive of one folder is named after the folder.
    let title = if prefix.is_empty() {
        safe_name(&stem)
    } else {
        safe_name(prefix.trim_end_matches('/'))
    };
    let fail = |e: io::Error| format!("could not import {name}: {e}");
    let mut made = None;
    let message = match kind {
        Kind::ResourcePack | Kind::DataPack | Kind::ShaderPack => {
            let (folder, what) = match kind {
                Kind::ResourcePack => ("resourcepacks", "Resource pack"),
                Kind::DataPack => ("datapacks", "Data pack"),
                _ => ("shaderpacks", "Shader pack"),
            };
            let dir = places.game.join(folder);
            fs::create_dir_all(&dir).map_err(fail)?;
            if prefix.is_empty() {
                // Packs are read from their zip.
                let to = unique(&dir, &title, ".zip");
                copy_file(path, &to).map_err(fail)?;
            } else {
                extract_into(path, &prefix, &unique(&dir, &title, ""), &[]).map_err(fail)?;
            }
            let turn_on = match kind {
                Kind::ResourcePack => "turn it on in Options > Resource Packs",
                Kind::DataPack => "pick it in Create New World > More > Data Packs",
                _ => "pick it in Video Settings > Shader Pack",
            };
            format!("{what} \"{title}\" added: {turn_on}")
        }
        Kind::World => {
            let dir = places.game.join("saves");
            fs::create_dir_all(&dir).map_err(fail)?;
            extract_into(path, &prefix, &unique(&dir, &title, ""), &["session.lock"])
                .map_err(fail)?;
            format!("World \"{title}\" added to Singleplayer")
        }
        Kind::Mod => {
            let dir = places.game.join("horizon-mods");
            fs::create_dir_all(&dir).map_err(fail)?;
            if prefix.is_empty() {
                copy_file(path, &unique(&dir, &title, ".hmod")).map_err(fail)?;
            } else {
                extract_into(path, &prefix, &unique(&dir, &title, ""), &[]).map_err(fail)?;
            }
            format!(
                "Mod \"{title}\" added: turn it on in Options > Mods, or, if it adds blocks or items, pick it for a new world in Create New World > More > Mods & Data Packs"
            )
        }
        Kind::NativeMod => {
            let bundle = horizon_mods::index::native::Bundle::read(path)
                .map_err(|e| format!("{name} can't be added: {e}"))?;
            let id = bundle.manifest.id;
            let dir = places.game.join("horizon-native-mods");
            fs::create_dir_all(&dir).map_err(fail)?;
            copy_file(path, &dir.join(format!("{id}.hnmod"))).map_err(fail)?;
            format!("Native mod \"{id}\" added: compile it in Options > Mods")
        }
        Kind::Resources => {
            install_resources(path, &prefix, &places.resources)?;
            "Minecraft resources imported".to_owned()
        }
        Kind::Skin => {
            // The file as it came: the game lays out a legacy 64x32 one when it reads it.
            let bytes = fs::read(path).map_err(fail)?;
            crate::own_skin::decode(&bytes).map_err(|e| format!("{name} is not a skin: {e}"))?;
            let dir = places.game.join(crate::own_skin::DIR);
            fs::create_dir_all(&dir).map_err(fail)?;
            let to = unique(&dir, &title, ".png");
            copy_file(path, &to).map_err(fail)?;
            made = Some(to);
            format!("Skin \"{title}\" added and put on")
        }
        Kind::Bedrock => {
            return Err(format!(
                "{name} is Bedrock Edition content: this game plays Java Edition packs, worlds and mods"
            ));
        }
        Kind::Unknown => {
            return Err(format!(
                "the game does not know what {name} is: it takes resource packs, shader packs, \
                 worlds and mods as .zip or .starling, and skins as .png"
            ));
        }
    };
    Ok(Installed {
        kind,
        message,
        path: made,
    })
}

/// Unpack prepared resources beside the current ones, check they are for this Horizon, then
/// swap them in: a failed import leaves the old ones as they were.
fn install_resources(path: &Path, prefix: &str, resources: &Path) -> Result<(), String> {
    let staged = sibling(resources, ".new");
    let _ = fs::remove_dir_all(&staged);
    extract_into(path, prefix, &staged, &[])
        .map_err(|e| format!("could not unpack the resources: {e}"))?;
    let manifest: serde_json::Value = fs::read(staged.join("horizon-resources.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let wanted = i64::from(horizon_world::DATA_VERSION);
    if manifest["data_version"].as_i64() != Some(wanted) {
        let _ = fs::remove_dir_all(&staged);
        return Err(format!(
            "these resources are for another Minecraft version (data version {}, Horizon needs {wanted})",
            manifest["data_version"]
        ));
    }
    let old = sibling(resources, ".old");
    let _ = fs::remove_dir_all(&old);
    if resources.exists() {
        fs::rename(resources, &old).map_err(|e| format!("could not replace the resources: {e}"))?;
    }
    if let Err(e) = fs::rename(&staged, resources) {
        // Put the old ones back.
        let _ = fs::rename(&old, resources);
        return Err(format!("could not replace the resources: {e}"));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

/// `dir/name.suffix` beside `path`'s folder name: `resources` -> `resources.new`.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// A file or folder name made of `name` that has no path separators or reserved characters.
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    if cleaned.is_empty() {
        "Imported".to_owned()
    } else {
        cleaned
    }
}

/// `dir/name<ext>`, or `dir/name (2)<ext>` and so on when that is taken.
fn unique(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{name}{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{name} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("an unused name")
}

/// Copy through a `.part` file, so a failed copy leaves nothing the game would read.
fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
    let part = sibling(to, ".part");
    fs::copy(from, &part)?;
    fs::rename(&part, to)
}

/// Unpack the archive's entries under `prefix` into the new folder `to` (through `to.part`),
/// leaving out entries named in `skip` and any whose path would leave the folder.
fn extract_into(path: &Path, prefix: &str, to: &Path, skip: &[&str]) -> io::Result<()> {
    let part = sibling(to, ".part");
    let _ = fs::remove_dir_all(&part);
    fs::create_dir_all(&part)?;
    let mut archive = zip::ZipArchive::new(File::open(path)?).map_err(io::Error::other)?;
    let prefix = Path::new(prefix.trim_end_matches('/'));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(io::Error::other)?;
        // enclosed_name: no absolute paths and no `..`.
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let Ok(rel) = name.strip_prefix(prefix) else {
            continue;
        };
        if rel.as_os_str().is_empty()
            || rel
                .file_name()
                .is_some_and(|f| skip.iter().any(|s| f == *s))
        {
            continue;
        }
        let out = part.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(&out)?)?;
    }
    fs::rename(&part, to)
}

/// What the inbox is doing, for a note on screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The activity is still copying the file in.
    Receiving {
        name: String,
        bytes: u64,
    },
    Installing {
        name: String,
    },
    Done(Installed),
    Failed {
        name: String,
        error: String,
    },
}

/// The inbox, installed from one file at a time on a worker.
pub struct Inbox {
    dir: PathBuf,
    places: Places,
    running: Option<(String, JoinHandle<Result<Installed, String>>)>,
}

impl Inbox {
    pub fn new(dir: PathBuf, places: Places) -> Self {
        Self {
            dir,
            places,
            running: None,
        }
    }

    /// Look at the inbox (a few times a second at most): finish or start an import, or say what
    /// is arriving. None when nothing is happening.
    pub fn poll(&mut self) -> Option<Event> {
        if let Some((name, handle)) = &self.running {
            if !handle.is_finished() {
                return Some(Event::Installing { name: name.clone() });
            }
            let (name, handle) = self.running.take()?;
            return Some(
                match handle
                    .join()
                    .unwrap_or_else(|_| Err(format!("the import of {name} stopped")))
                {
                    Ok(installed) => Event::Done(installed),
                    Err(error) => Event::Failed { name, error },
                },
            );
        }
        let mut receiving = None;
        let mut entries: Vec<_> = fs::read_dir(&self.dir).ok()?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(arriving) = name.strip_suffix(".part") {
                let bytes = entry.metadata().map_or(0, |m| m.len());
                receiving = Some(Event::Receiving {
                    name: arriving.to_owned(),
                    bytes,
                });
                continue;
            }
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let (path, places, shown) = (entry.path(), self.places.clone(), name.clone());
            let handle = std::thread::Builder::new()
                .name("horizon-import".into())
                .spawn(move || {
                    let result = install(&path, &shown, &places);
                    let _ = fs::remove_file(&path);
                    result
                })
                .ok()?;
            self.running = Some((name.clone(), handle));
            return Some(Event::Installing { name });
        }
        receiving
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(path: &Path, files: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, data) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("horizon-import-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn archives_are_told_apart_by_what_they_hold() {
        let dir = scratch("classify");
        let cases: &[(&str, &[(&str, &[u8])], Kind, &str)] = &[
            (
                "pack.zip",
                &[("pack.mcmeta", b"{}"), ("assets/a.png", b"")],
                Kind::ResourcePack,
                "",
            ),
            (
                "nested.zip",
                &[("Faithful/pack.mcmeta", b"{}")],
                Kind::ResourcePack,
                "Faithful/",
            ),
            (
                "datapack.zip",
                &[
                    ("pack.mcmeta", b"{}"),
                    ("data/show/function/build.mcfunction", b""),
                ],
                Kind::DataPack,
                "",
            ),
            (
                "both.zip",
                &[
                    ("Both/pack.mcmeta", b"{}"),
                    ("Both/data/x/function/f.mcfunction", b""),
                    ("Both/assets/x/lang/en_us.json", b"{}"),
                ],
                Kind::ResourcePack,
                "Both/",
            ),
            (
                "world.zip",
                &[
                    ("My World/level.dat", b""),
                    ("My World/region/r.0.0.mca", b""),
                ],
                Kind::World,
                "My World/",
            ),
            (
                "bsl.zip",
                &[("shaders/composite.fsh", b"")],
                Kind::ShaderPack,
                "",
            ),
            (
                "deep.zip",
                &[("BSL/shaders/final.fsh", b"")],
                Kind::ShaderPack,
                "BSL/",
            ),
            (
                "res.zip",
                &[("26.3/horizon-resources.json", b"{}")],
                Kind::Resources,
                "26.3/",
            ),
            (
                "m.zip",
                &[("horizon-mod.toml", b""), ("mod.wasm", b"")],
                Kind::Mod,
                "",
            ),
            ("notes.zip", &[("readme.txt", b"")], Kind::Unknown, ""),
        ];
        for (name, files, kind, prefix) in cases {
            let path = dir.join(name);
            zip_of(&path, files);
            assert_eq!(classify(&path), (*kind, prefix.to_string()), "{name}");
        }
        fs::write(dir.join("addon.mcaddon"), b"").unwrap();
        assert_eq!(classify(&dir.join("addon.mcaddon")).0, Kind::Bedrock);
        fs::write(dir.join("plain.txt"), b"hello").unwrap();
        assert_eq!(classify(&dir.join("plain.txt")).0, Kind::Unknown);
    }

    #[test]
    fn imports_land_where_the_game_reads_them_without_overwriting() {
        let dir = scratch("install");
        let places = Places {
            game: dir.join("game"),
            resources: dir.join("private/resources"),
        };
        let pack = dir.join("Faithful 32x.zip");
        zip_of(&pack, &[("pack.mcmeta", b"{}")]);
        let first = install(&pack, "Faithful 32x.zip", &places).unwrap();
        assert_eq!(first.kind, Kind::ResourcePack);
        install(&pack, "Faithful 32x.zip", &places).unwrap();
        assert!(places.game.join("resourcepacks/Faithful 32x.zip").is_file());
        assert!(
            places
                .game
                .join("resourcepacks/Faithful 32x (2).zip")
                .is_file()
        );

        let world = dir.join("w.zip");
        zip_of(
            &world,
            &[
                ("Castle/level.dat", b"L"),
                ("Castle/session.lock", b""),
                ("Castle/region/r.0.0.mca", b"R"),
            ],
        );
        install(&world, "w.zip", &places).unwrap();
        let castle = places.game.join("saves/Castle");
        assert_eq!(fs::read(castle.join("level.dat")).unwrap(), b"L");
        assert_eq!(fs::read(castle.join("region/r.0.0.mca")).unwrap(), b"R");
        assert!(!castle.join("session.lock").exists());

        let evil = dir.join("evil.zip");
        zip_of(&evil, &[("level.dat", b""), ("../escaped.txt", b"x")]);
        install(&evil, "evil.zip", &places).unwrap();
        assert!(!places.game.join("saves/escaped.txt").exists());
        assert!(!places.game.join("escaped.txt").exists());

        assert!(install(&dir.join("addon.mcaddon"), "addon.mcaddon", &places).is_err());
    }

    #[test]
    fn a_picture_comes_out_of_the_inbox_at_any_size() {
        let dir = scratch("picture");
        assert!(picture_in(&dir).is_none());
        fs::write(dir.join("big.png.part"), b"arriving").unwrap();
        assert!(picture_in(&dir).is_none());
        fs::remove_file(dir.join("big.png.part")).unwrap();
        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, 24, 16);
            encoder.set_color(png::ColorType::Rgba);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255; 24 * 16 * 4]).unwrap();
        }
        fs::write(dir.join("0.png"), &png_bytes).unwrap();
        let (w, h, pixels) = picture_in(&dir).unwrap().unwrap();
        assert_eq!((w, h, pixels.len()), (24, 16, 24 * 16 * 4));
        assert!(picture_in(&dir).is_none(), "taken out of the inbox");
        fs::write(dir.join("photo.jpg"), b"not a png").unwrap();
        assert_eq!(
            picture_in(&dir).unwrap().unwrap_err(),
            "photo.jpg is not a PNG"
        );
        assert!(picture_in(&dir).is_none(), "taken out of the inbox");
    }

    #[test]
    fn a_png_skin_goes_to_the_skins_folder() {
        let dir = scratch("skin");
        let places = Places {
            game: dir.join("game"),
            resources: dir.join("resources"),
        };
        let image = |width: u32, height: u32| {
            let mut out = Vec::new();
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(png::ColorType::Rgba);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![255; (width * height * 4) as usize])
                .unwrap();
            writer.finish().unwrap();
            out
        };
        // The picker's copy may have no extension.
        let legacy = dir.join("old skin");
        fs::write(&legacy, image(64, 32)).unwrap();
        assert_eq!(classify(&legacy).0, Kind::Skin);
        let done = install(&legacy, "old skin", &places).unwrap();
        assert_eq!(done.kind, Kind::Skin);
        let to = places.game.join("skins/old skin.png");
        assert_eq!(done.path.as_deref(), Some(to.as_path()));
        assert_eq!(fs::read(&to).unwrap(), fs::read(&legacy).unwrap());
        let photo = dir.join("photo.png");
        fs::write(&photo, image(16, 16)).unwrap();
        let error = install(&photo, "photo.png", &places).unwrap_err();
        assert!(error.contains("not a skin"), "{error}");
        assert!(!places.game.join("skins/photo.png").exists());
    }

    #[test]
    fn resources_of_another_version_leave_the_old_ones() {
        let dir = scratch("resources");
        let places = Places {
            game: dir.join("game"),
            resources: dir.join("resources"),
        };
        fs::create_dir_all(&places.resources).unwrap();
        fs::write(places.resources.join("old.txt"), b"old").unwrap();
        let wrong = dir.join("wrong.zip");
        zip_of(
            &wrong,
            &[("horizon-resources.json", br#"{"data_version": 1}"#)],
        );
        assert!(install(&wrong, "wrong.zip", &places).is_err());
        assert!(places.resources.join("old.txt").is_file());
        let right = dir.join("right.zip");
        let manifest = format!(r#"{{"data_version": {}}}"#, horizon_world::DATA_VERSION);
        zip_of(
            &right,
            &[
                ("26.3/horizon-resources.json", manifest.as_bytes()),
                ("26.3/facts.json", b"[]"),
            ],
        );
        let done = install(&right, "right.zip", &places).unwrap();
        assert_eq!(done.kind, Kind::Resources);
        assert!(places.resources.join("facts.json").is_file());
        assert!(!places.resources.join("old.txt").exists());
    }

    #[test]
    fn the_inbox_waits_for_complete_files_and_empties_itself() {
        let dir = scratch("inbox");
        let inbox_dir = dir.join("inbox");
        fs::create_dir_all(&inbox_dir).unwrap();
        let places = Places {
            game: dir.join("game"),
            resources: dir.join("resources"),
        };
        let mut inbox = Inbox::new(inbox_dir.clone(), places.clone());
        assert_eq!(inbox.poll(), None);
        fs::write(inbox_dir.join("pack.zip.part"), b"half").unwrap();
        assert!(matches!(
            inbox.poll(),
            Some(Event::Receiving { bytes: 4, .. })
        ));
        fs::remove_file(inbox_dir.join("pack.zip.part")).unwrap();
        zip_of(&inbox_dir.join("pack.zip"), &[("pack.mcmeta", b"{}")]);
        assert!(matches!(inbox.poll(), Some(Event::Installing { .. })));
        let done = loop {
            match inbox.poll() {
                Some(Event::Installing { .. }) => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                other => break other,
            }
        };
        assert!(matches!(
            done,
            Some(Event::Done(Installed {
                kind: Kind::ResourcePack,
                ..
            }))
        ));
        assert!(!inbox_dir.join("pack.zip").exists());
        assert!(places.game.join("resourcepacks/pack.zip").is_file());
    }
}
