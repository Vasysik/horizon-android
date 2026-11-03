//! Explicit classic shader downloads; native .hshader files keep the registry path.
use crate::browser::{self, Kind, Project, Source, SourceId, Version, installed};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    #[default]
    Horizon,
    Modrinth,
}
impl Origin {
    pub fn source(self) -> SourceId {
        match self {
            Self::Horizon => SourceId::Horizon,
            Self::Modrinth => SourceId::Modrinth,
        }
    }
}

fn version(versions: &[Version]) -> Option<&Version> {
    let usable = |v: &&Version| {
        v.blocked.is_none()
            && v.file()
                .is_some_and(|f| f.name.to_ascii_lowercase().ends_with(".zip"))
    };
    versions
        .iter()
        .filter(usable)
        .find(|v| v.game_versions.iter().any(|g| g == browser::GAME))
        .or_else(|| versions.iter().find(usable))
}

pub fn install(
    source: &dyn Source,
    project: &Project,
    home: &Path,
    progress: &dyn Fn(&str, f32),
) -> Result<usize, String> {
    if project.kind != Kind::Shader
        || project.source != SourceId::Modrinth
        || source.id() != project.source
    {
        return Err("Expected a Modrinth shader pack".into());
    }
    let versions = source.versions(project)?;
    let v = version(&versions).ok_or("No downloadable shader ZIP is available")?;
    if !v.required.is_empty() {
        return Err(
            "This shader requires additional projects which Horizon cannot install automatically"
                .into(),
        );
    }
    let file = v.file().ok_or("Shader file is missing")?;
    if file.size > horizon_client::content_files::MAX_INPUT {
        return Err("Shader ZIP exceeds 256 MiB".into());
    }
    let name = browser::safe_name(&file.name).ok_or("Unsafe shader file name")?;
    let folder = home.join("shaderpacks");
    if fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Shader folder cannot be a symbolic link".into());
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let relative = format!("shaderpacks/{name}");
    let target = home.join(&relative);
    let mut list = installed::Installed::load(home);
    let ours = |e: &installed::Entry| e.source == project.source && e.project == project.id;
    let old = list.entries.iter().find(|e| ours(e)).cloned();
    let replacing = target.exists();
    if let Ok(meta) = fs::symlink_metadata(&target) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("Cannot replace a folder or symbolic link".into());
        }
        if !list.entries.iter().any(|e| ours(e) && e.file == relative)
            || list.entries.iter().any(|e| !ours(e) && e.file == relative)
        {
            return Err("This shader file already exists. Remove it in Installed or rename it in Files first".into());
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = home
        .join("mod-browser/downloads")
        .join(format!("shader-{stamp}"));
    let result = (|| {
        let path = source.fetch(file, &stage, &|f| progress(&project.title, f))?;
        let native = horizon_client::content_files::validate(
            &path,
            horizon_client::content_files::Kind::Shaders,
            name,
            || false,
        )
        .map_err(|e| e.to_string())?;
        if native {
            return Err("This ZIP is a native Horizon shader pack. Use Import file to install it as .hshader".into());
        }
        let (algo, hash) = file.checksum().ok_or("The source supplies no checksum")?;
        if replacing {
            fs::rename(&path, &target).map_err(|e| e.to_string())?;
        } else {
            // Do not replace a file created during the download by an importer/Files.
            // Android's app-data SELinux policy forbids hard links. Reserve the
            // destination exclusively instead; a failed copy removes only ours.
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            let copied = (|| -> std::io::Result<()> {
                std::io::copy(&mut fs::File::open(&path)?, &mut output)?;
                output.sync_all()
            })();
            if let Err(error) = copied {
                drop(output);
                let _ = fs::remove_file(&target);
                return Err(error.to_string());
            }
        }
        list.entries.retain(|e| !ours(e));
        list.entries.push(installed::Entry {
            source: project.source,
            project: project.id.clone(),
            slug: project.slug.clone(),
            title: project.title.clone(),
            kind: Kind::Shader,
            version: v.id.clone(),
            number: v.number.clone(),
            file: relative,
            checksum: format!("{algo}:{hash}"),
            page: project.page.clone(),
            dependency: false,
            verdict: Some("Experimental shader compatibility".into()),
            skipped: vec![],
            namespaces: vec![],
        });
        list.save(home)?;
        if let Some(old) = old.filter(|e| e.file != format!("shaderpacks/{name}"))
            && let Some(previous) = old.file.strip_prefix("shaderpacks/")
            && browser::safe_name(previous).is_some()
            && !list.entries.iter().any(|e| e.file == old.file)
        {
            let path = folder.join(previous);
            if fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            {
                let _ = fs::remove_file(path);
            }
        }
        Ok(1)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use browser::FileRef;
    use sha2::{Digest, Sha512};
    struct Fixture {
        bytes: Vec<u8>,
        versions: Vec<Version>,
    }
    impl Source for Fixture {
        fn id(&self) -> SourceId {
            SourceId::Modrinth
        }
        fn search(&self, _: &browser::Query) -> Result<Vec<Project>, String> {
            unreachable!()
        }
        fn project(&self, _: &str) -> Result<Project, String> {
            unreachable!()
        }
        fn versions(&self, _: &Project) -> Result<Vec<Version>, String> {
            Ok(self.versions.clone())
        }
        fn fetch(
            &self,
            file: &FileRef,
            folder: &Path,
            progress: &dyn Fn(f32),
        ) -> Result<std::path::PathBuf, String> {
            browser::download::save(file, self.bytes.as_slice(), folder, progress)
        }
    }
    fn fixture() -> (Fixture, Project) {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "shaders/final.fsh",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"// Original test fixture; not a shader implementation")
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let file = FileRef {
            url: "https://cdn.modrinth.com/fixture.zip".into(),
            name: "fixture.zip".into(),
            size: bytes.len() as u64,
            sha512: Some(format!("{:x}", Sha512::digest(&bytes))),
            sha256: None,
            sha1: None,
            primary: true,
        };
        let v = Version {
            id: "fixture-v1".into(),
            number: "1".into(),
            published: "2026-10-29".into(),
            game_versions: vec![browser::GAME.into()],
            loaders: vec!["iris".into()],
            files: vec![file],
            required: vec![],
            blocked: None,
        };
        let p = Project {
            source: SourceId::Modrinth,
            id: "fixture".into(),
            slug: "fixture".into(),
            title: "Fixture shader".into(),
            summary: String::new(),
            author: String::new(),
            downloads: 0,
            page: "https://modrinth.com/shader/fixture".into(),
            icon: None,
            license: String::new(),
            kind: Kind::Shader,
            game_versions: v.game_versions.clone(),
            loaders: v.loaders.clone(),
        };
        (
            Fixture {
                bytes,
                versions: vec![v],
            },
            p,
        )
    }
    #[test]
    fn version_prefers_current_game_then_newest_zip_and_skips_blocked_files() {
        let (f, _) = fixture();
        let mut newest = f.versions[0].clone();
        newest.id = "newest".into();
        newest.game_versions = vec!["1.21".into()];
        let mut all = vec![newest, f.versions[0].clone()];
        assert_eq!(version(&all).unwrap().id, "fixture-v1");
        all[1].blocked = Some("withdrawn".into());
        assert_eq!(version(&all).unwrap().id, "newest");
        all[0].files[0].name = "not-a-pack.jar".into();
        assert!(version(&all).is_none());
        assert_eq!(
            serde_json::to_string(&Origin::Modrinth).unwrap(),
            "\"modrinth\""
        );
    }
    #[test]
    fn install_validates_checksum_archive_and_ownership_before_replacing() {
        let (mut source, project) = fixture();
        let home = std::env::temp_dir().join(format!(
            "horizon-shader-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join("shaderpacks")).unwrap();
        let target = home.join("shaderpacks/fixture.zip");
        fs::write(&target, b"player file").unwrap();
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"player file");
        fs::remove_file(&target).unwrap();
        assert_eq!(install(&source, &project, &home, &|_, _| {}).unwrap(), 1);
        let original = fs::read(&target).unwrap();
        assert_eq!(
            installed::Installed::load(&home).entries[0].kind,
            Kind::Shader
        );
        source.bytes = b"not a ZIP".to_vec();
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        source.versions[0].files[0].size = source.bytes.len() as u64;
        source.versions[0].files[0].sha512 = Some(format!("{:x}", Sha512::digest(&source.bytes)));
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        assert_eq!(
            fs::read_dir(home.join("mod-browser/downloads"))
                .unwrap()
                .count(),
            0
        );
        fs::remove_dir_all(home).unwrap();
    }
}
