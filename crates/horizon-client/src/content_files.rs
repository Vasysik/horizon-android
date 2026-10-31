//! Local add-on files. Imports remain archives: no untrusted path is extracted.
use std::{
    collections::HashSet,
    io::{self, Read},
    path::{Component, Path},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Mods,
    Packs,
    Shaders,
}
impl Kind {
    pub const ALL: [Self; 3] = [Self::Mods, Self::Packs, Self::Shaders];
    pub fn folder(self) -> &'static str {
        match self {
            Self::Mods => "horizon-mods",
            Self::Packs => "resourcepacks",
            Self::Shaders => "shaderpacks",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Mods => "Mods",
            Self::Packs => "Resource Packs",
            Self::Shaders => "Shaders",
        }
    }
    pub fn accepts(self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        match self {
            Self::Mods => lower.ends_with(".hmod"),
            Self::Packs => lower.ends_with(".zip"),
            Self::Shaders => lower.ends_with(".zip") || lower.ends_with(".hshader"),
        }
    }
}
pub const MAX_INPUT: u64 = 256 << 20;
const MAX_EXPANDED: u64 = 512 << 20;
pub fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 180
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':', '\0'])
        && !name.chars().any(char::is_control)
        && matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
        && Path::new(name).components().count() == 1
}
pub fn validate(
    path: &Path,
    kind: Kind,
    name: &str,
    cancelled: impl Fn() -> bool,
) -> crate::Result<bool> {
    if !plain_name(name) || !kind.accepts(name) {
        return Err(
            "Choose a ZIP pack, .hshader shader pack or sandboxed .hmod matching this folder"
                .into(),
        );
    }
    let file = std::fs::File::open(path)?;
    if kind == Kind::Mods && file.metadata()?.len() > 32 << 20 {
        return Err("Horizon mod exceeds 32 MiB".into());
    }
    if file.metadata()?.len() > MAX_INPUT {
        return Err("Import exceeds 256 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(file)?;
    if zip.len() > 32768 {
        return Err("Archive contains too many files".into());
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    let mut pack = false;
    let mut shader = false;
    let mut native_shader = false;
    let mut manifest = false;
    let mut buffer = [0u8; 65536];
    for i in 0..zip.len() {
        if cancelled() {
            return Err("Import cancelled".into());
        }
        let mut entry = zip.by_index(i)?;
        let raw = entry.name()?.into_owned();
        if raw.is_empty()
            || raw.len() > 1024
            || raw.chars().any(char::is_control)
            || raw.contains(['\\', ':', '\0'])
            || raw.starts_with('/')
            || raw.split('/').any(|p| p == ".." || p == ".")
            || !names.insert(raw.clone())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Archive contains an unsafe or duplicate path".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Archive size overflow")?;
        if total > MAX_EXPANDED || kind == Kind::Mods && total > 64 << 20 {
            return Err("Expanded archive exceeds 512 MiB".into());
        }
        pack |= raw == "pack.mcmeta";
        native_shader |= raw == "horizon-shaders.toml";
        shader |= native_shader
            || raw.starts_with("shaders/")
                && !entry.is_dir()
                && ["vsh", "fsh", "glsl", "csh", "gsh", "tcs", "tes"]
                    .iter()
                    .any(|ext| raw.ends_with(&format!(".{ext}")));
        manifest |= raw == "horizon-mod.toml";
        let expected = entry.size();
        if raw == "pack.mcmeta" && expected > 1 << 20 {
            return Err("Pack metadata is too large".into());
        }
        let mut metadata = Vec::new();
        let mut actual = 0u64;
        while !entry.is_dir() {
            if cancelled() {
                return Err("Import cancelled".into());
            }
            let n = entry.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            actual += n as u64;
            if raw == "pack.mcmeta" {
                metadata.extend_from_slice(&buffer[..n]);
            }
            if actual > expected || actual > MAX_EXPANDED {
                return Err("Archive size mismatch".into());
            }
        }
        if !entry.is_dir() && actual != expected {
            return Err("Incomplete archive entry".into());
        }
        if raw == "pack.mcmeta"
            && !serde_json::from_slice::<serde_json::Value>(&metadata)?
                .get("pack")
                .is_some_and(|v| v.is_object())
        {
            return Err("Resource pack metadata has no pack object".into());
        }
    }
    if !match kind {
        Kind::Packs => pack,
        Kind::Shaders => shader,
        Kind::Mods => manifest,
    } {
        return Err(format!(
            "Archive is not a {} package (required files must be at the ZIP root)",
            kind.label()
        )
        .into());
    }
    if kind == Kind::Mods {
        // Accept both sandboxed code and upstream content-only packages. The
        // shared parser requires mod.wasm unless the manifest grants registries.
        horizon_mods::Package::read(path).map_err(|e| format!("Invalid Horizon mod: {e}"))?;
    }
    Ok(kind == Kind::Shaders && native_shader)
}
pub fn copy_bounded(
    input: &mut impl Read,
    output: &mut impl io::Write,
    mut progress: impl FnMut(u64) -> bool,
) -> io::Result<u64> {
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        if !progress(total) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Import cancelled",
            ));
        }
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_INPUT {
            return Err(io::Error::other("Import exceeds 256 MiB"));
        }
        output.write_all(&buffer[..n])?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn validates_structure_paths_metadata_and_cancellation() {
        let dir = std::env::temp_dir().join(format!(
            "horizon-import-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("stage.part");
        let metadata = br#"{"pack":{"pack_format":1,"description":"test"}}"#;
        std::fs::write(
            &path,
            archive(&[("pack.mcmeta", metadata), ("assets/test.png", b"png")]),
        )
        .unwrap();
        assert_eq!(
            validate(&path, Kind::Packs, "Test.zip", || false).unwrap(),
            false
        );
        assert!(validate(&path, Kind::Packs, "Test.zip", || true).is_err());
        assert!(validate(&path, Kind::Packs, "../Test.zip", || false).is_err());
        for name in ["../outside", "/absolute", "C:/outside", "folder\\outside"] {
            std::fs::write(&path, archive(&[("pack.mcmeta", metadata), (name, b"bad")])).unwrap();
            assert!(
                validate(&path, Kind::Packs, "Test.zip", || false).is_err(),
                "{name}"
            );
        }
        std::fs::write(&path, archive(&[("pack.mcmeta", b"not JSON")])).unwrap();
        assert!(validate(&path, Kind::Packs, "Test.zip", || false).is_err());
        std::fs::write(&path, archive(&[("shaders/test.fsh", b"shader")])).unwrap();
        assert!(validate(&path, Kind::Shaders, "Test.zip", || false).is_ok());
        assert!(validate(&path, Kind::Packs, "Test.zip", || false).is_err());
        std::fs::write(&path, archive(&[("horizon-shaders.toml", b"manifest")])).unwrap();
        assert!(validate(&path, Kind::Shaders, "Test.zip", || false).unwrap());
        std::fs::write(&path, archive(&[("horizon-mod.toml", b"manifest")])).unwrap();
        assert!(validate(&path, Kind::Mods, "Test.hmod", || false).is_err());
        let content = b"id='test_content'\nname='Content'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n";
        std::fs::write(&path, archive(&[("horizon-mod.toml", content)])).unwrap();
        assert!(validate(&path, Kind::Mods, "Content.hmod", || false).is_ok());
        let code = b"id='test_code'\nname='Code'\nversion='1'\napi='0.1'\ncapabilities=['hud']\n";
        std::fs::write(&path, archive(&[("horizon-mod.toml", code)])).unwrap();
        assert!(validate(&path, Kind::Mods, "Code.hmod", || false).is_err());
        std::fs::write(
            &path,
            archive(&[("horizon-mod.toml", code), ("mod.wasm", b"wasm")]),
        )
        .unwrap();
        assert!(validate(&path, Kind::Mods, "Code.hmod", || false).is_ok());
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink(
                "link",
                "../outside",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
        assert!(validate(&path, Kind::Packs, "Test.zip", || false).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn streamed_copy_is_bounded_and_cancellable() {
        let mut out = Vec::new();
        assert_eq!(
            copy_bounded(&mut Cursor::new(b"test"), &mut out, |_| true).unwrap(),
            4
        );
        assert_eq!(out, b"test");
        assert!(copy_bounded(&mut io::repeat(0), &mut io::sink(), |_| true).is_err());
        assert!(copy_bounded(&mut Cursor::new(b"test"), &mut io::sink(), |_| false).is_err());
    }
}
