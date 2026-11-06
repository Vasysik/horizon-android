//! Native resource installation from Mojang HTTPS endpoints. No Java or LAN server.
use crate::Result;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

#[path = "official_install_cache.rs"]
mod cache_input;

const CATALOG: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const LOCK: &str = include_str!("../../../research-lock.json");
static INSTALLATION: Mutex<()> = Mutex::new(());
static DOWNLOAD_CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Android supplies Context.getCacheDir(), keeping temporary downloads out of files/.
pub fn configure_download_cache_dir(directory: PathBuf) {
    let _ = DOWNLOAD_CACHE_DIR.set(directory.join("minecraft-downloads"));
}

fn download_cache(home: &Path) -> PathBuf {
    DOWNLOAD_CACHE_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| home.join(".local/downloads"))
        .join(horizon_core::VANILLA_VERSION)
}

/// Earlier Android packages installed textures only. An audio-capable APK must
/// upgrade those resources before entering the game, not silently stay mute.
pub fn ready(home: &Path) -> bool {
    let root = home
        .join(".local/horizon-client")
        .join(horizon_core::VANILLA_VERSION);
    crate::resource_install::ready(home)
        && root.join("android-downloads-v2.json").is_file()
        && (!cfg!(feature = "audio") || root.join("audio-complete.json").is_file())
}

/// Discard redundant installer inputs only after the prepared installation is complete.
/// An interrupted download/preparation keeps its cache for the next attempt.
pub fn cleanup_download_cache(home: &Path) -> Result<()> {
    if !ready(home) {
        return Ok(());
    }
    let root = fs::canonicalize(home)?;
    let cache = root
        .join(".local/downloads")
        .join(horizon_core::VANILLA_VERSION);
    let metadata = match fs::symlink_metadata(&cache) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    // Verify the final absolute target before recursive removal. Reject aliases
    // of this path, including a redirected .local/downloads ancestor.
    if !metadata.is_dir() || metadata.file_type().is_symlink() || fs::canonicalize(&cache)? != cache
    {
        return Err("Refusing redirected installer cache path".into());
    }
    fs::remove_dir_all(cache)?;
    Ok(())
}

fn cleanup_system_download_cache() -> Result<()> {
    let Some(directory) = DOWNLOAD_CACHE_DIR.get() else {
        return Ok(());
    };
    if !directory.exists() {
        return Ok(());
    }
    let root = fs::canonicalize(directory)?;
    let cache = root.join(horizon_core::VANILLA_VERSION);
    let metadata = match fs::symlink_metadata(&cache) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() || fs::canonicalize(&cache)? != cache
    {
        return Err("Refusing redirected system installer cache".into());
    }
    fs::remove_dir_all(cache)?;
    Ok(())
}

/// Clean both old files/ downloads and the system cache when resources are complete.
pub fn cleanup_completed_downloads(home: &Path) -> Result<()> {
    let _guard = INSTALLATION.lock().unwrap_or_else(|e| e.into_inner());
    cleanup_completed_downloads_inner(home)
}
fn cleanup_completed_downloads_inner(home: &Path) -> Result<()> {
    if !ready(home) {
        return Ok(());
    }
    cleanup_download_cache(home)?;
    cleanup_system_download_cache()
}

fn official_url(text: &str) -> Result<()> {
    let url = url::Url::parse(text)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
        || !matches!(
            url.host_str(),
            Some(
                "piston-meta.mojang.com"
                    | "piston-data.mojang.com"
                    | "resources.download.minecraft.net"
                    | "launcher.mojang.com"
                    | "launchermeta.mojang.com"
            )
        )
    {
        return Err("Download URL is not an official Mojang HTTPS endpoint".into());
    }
    Ok(())
}

fn agent() -> ureq::Agent {
    // Thousands of sound objects share a host; retain pooled TLS connections.
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT
        .get_or_init(|| {
            ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(std::time::Duration::from_secs(30))
                .timeout_read(std::time::Duration::from_secs(120))
                .build()
        })
        .clone()
}

// Metadata requests need the same transient-network recovery as asset files.
// Retry on the installer worker, retaining completed files for the next attempt.
fn metadata_bytes(url: &str) -> Result<Vec<u8>> {
    official_url(url)?;
    for attempt in 0..3 {
        let result = (|| -> Result<Vec<u8>> {
            let mut bytes = Vec::new();
            agent()
                .get(url)
                .call()?
                .into_reader()
                .take(16 * 1024 * 1024)
                .read_to_end(&mut bytes)?;
            Ok(bytes)
        })();
        if result.is_ok() || attempt == 2 {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(500 * (attempt + 1)));
    }
    unreachable!()
}
fn json(url: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&metadata_bytes(url)?)?)
}

fn hash(mut file: impl Read) -> Result<String> {
    let mut digest = Sha1::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

struct Download {
    url: String,
    sha1: String,
    size: u64,
    path: PathBuf,
}

impl Download {
    fn from(value: &Value, path: PathBuf) -> Result<Self> {
        let file = Self {
            url: value["url"].as_str().ok_or("Missing download URL")?.into(),
            sha1: value["sha1"]
                .as_str()
                .ok_or("Missing download hash")?
                .into(),
            size: value["size"].as_u64().ok_or("Missing download size")?,
            path,
        };
        official_url(&file.url)?;
        if file.sha1.len() != 40
            || !file.sha1.bytes().all(|c| c.is_ascii_hexdigit())
            || file.size > 1024 * 1024 * 1024
        {
            return Err("Invalid Mojang download metadata".into());
        }
        Ok(file)
    }
    fn get(&self) -> Result<()> {
        // Keep the opened inode while checking it: Android may unlink cache
        // entries between metadata, checksum validation and preparation.
        match fs::File::open(&self.path) {
            Ok(file) if file.metadata()?.len() == self.size && hash(&file)? == self.sha1 => {
                return Ok(());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not read cached Minecraft file {}: {error}",
                    self.path.display()
                )
                .into());
            }
        }
        let partial = self.path.with_extension("partial");
        for attempt in 0..3 {
            let result = (|| -> Result<()> {
                // Recreate on every retry, not only before the first attempt.
                fs::create_dir_all(self.path.parent().ok_or("Missing download directory")?)?;
                let mut input = agent()
                    .get(&self.url)
                    .call()?
                    .into_reader()
                    .take(self.size + 1);
                let mut output = fs::File::create(&partial)?;
                let count = std::io::copy(&mut input, &mut output)?;
                output.flush()?;
                drop(output);
                if count != self.size || hash(fs::File::open(&partial)?)? != self.sha1 {
                    return Err("Mojang file checksum or size mismatch".into());
                }
                fs::rename(&partial, &self.path)?;
                Ok(())
            })();
            if result.is_ok() {
                return result;
            }
            let _ = fs::remove_file(&partial);
            if attempt == 2 {
                return result.map_err(|error| {
                    format!(
                        "Could not download Minecraft file {}: {error}",
                        self.path.display()
                    )
                    .into()
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(500 * (attempt + 1)));
        }
        unreachable!()
    }

    fn open(&self) -> Result<fs::File> {
        cache_input::open(
            &self.path,
            |file| Ok(file.metadata()?.len() == self.size && hash(file)? == self.sha1),
            || self.get(),
        )
    }
}

fn relative(name: &str) -> Result<&Path> {
    let path = Path::new(name);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || name.contains('\\')
    {
        return Err("Unsafe asset path".into());
    }
    Ok(path)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstallStage {
    #[default]
    Metadata,
    Download,
    Extract,
    WorldData,
    Controls,
    Finalize,
}
impl InstallStage {
    pub const ALL: [Self; 6] = [
        Self::Metadata,
        Self::Download,
        Self::Extract,
        Self::WorldData,
        Self::Controls,
        Self::Finalize,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Metadata => "Checking Minecraft version",
            Self::Download => "Downloading game resources and sound",
            Self::Extract => "Unpacking verified files",
            Self::WorldData => "Preparing worlds and registries",
            Self::Controls => "Downloading touch controls",
            Self::Finalize => "Finishing installation",
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct InstallProgress {
    pub stage: InstallStage,
    pub done: u64,
    pub total: u64,
}

pub fn install(home: &Path, progress: impl Fn(u64, u64) + Sync) -> Result<()> {
    install_with_progress(home, |p| {
        if p.stage == InstallStage::Download {
            progress(p.done, p.total)
        }
    })
}
/// Metadata may be reissued, but the engine's game version and JARs must match.
fn validate_manifest(bytes: &[u8], checksum: &str, version: &str, pins: &Value) -> Result<Value> {
    if format!("{:x}", Sha1::digest(bytes)) != checksum {
        return Err("Version metadata checksum mismatch".into());
    }
    let manifest: Value = serde_json::from_slice(bytes)?;
    if manifest["id"].as_str() != Some(version)
        || ["client", "server"].iter().any(|side| {
            let pinned = pins[format!("{side}_sha1")].as_str();
            pinned.is_none() || manifest["downloads"][side]["sha1"].as_str() != pinned
        })
    {
        return Err("Minecraft game files differ from this engine's pinned version".into());
    }
    Ok(manifest)
}

pub fn install_with_progress(home: &Path, progress: impl Fn(InstallProgress) + Sync) -> Result<()> {
    let _guard = INSTALLATION.lock().unwrap_or_else(|e| e.into_inner());
    let report = |stage, done, total| progress(InstallProgress { stage, done, total });
    report(InstallStage::Metadata, 0, 0);
    let version = horizon_core::VANILLA_VERSION;
    let cache = download_cache(home);
    let legacy = home.join(".local/downloads").join(version);
    if cache != legacy && legacy.is_dir() && !cache.exists() {
        let root = fs::canonicalize(home)?;
        if fs::canonicalize(&legacy)? != root.join(".local/downloads").join(version) {
            return Err("Refusing redirected legacy installer cache".into());
        }
        fs::create_dir_all(cache.parent().ok_or("Missing cache parent")?)?;
        fs::rename(&legacy, &cache)?;
    }
    fs::create_dir_all(&cache)?;
    let pins: Value = serde_json::from_str(LOCK)?;
    let pins = &pins["minecraft"];
    let catalog = json(CATALOG)?;
    let entry = catalog["versions"]
        .as_array()
        .and_then(|vs| vs.iter().find(|v| v["id"] == version))
        .ok_or("Pinned Minecraft version is not listed by Mojang")?;
    let checksum = entry["sha1"]
        .as_str()
        .ok_or("Missing version metadata hash")?;
    // Mojang may reissue metadata while Minecraft 26.3's game jars are unchanged.
    // The downloaded JSON is still verified against the catalogue's SHA-1,
    // and the actual client/server jar hashes remain engine-pinned.
    let manifest_url = entry["url"]
        .as_str()
        .ok_or("Missing version metadata URL")?;
    official_url(manifest_url)?;
    let bytes = metadata_bytes(manifest_url)?;
    let manifest = validate_manifest(&bytes, checksum, version, pins)?;
    let index = Download::from(&manifest["assetIndex"], cache.join("asset-index.json"))?;
    index.get()?;
    let index: Value = serde_json::from_reader(index.open()?)?;
    let objects = index["objects"]
        .as_object()
        .ok_or("Missing asset objects")?;
    let client = Download::from(&manifest["downloads"]["client"], cache.join("client.jar"))?;
    let mut files = vec![client];
    let mut assets = Vec::new();
    let mut seen = HashSet::new();
    for (name, object) in objects {
        if !cfg!(feature = "audio")
            && (name.starts_with("minecraft/sounds/") || name == "minecraft/sounds.json")
        {
            continue;
        }
        relative(name)?;
        let checksum = object["hash"].as_str().ok_or("Missing asset hash")?;
        if checksum.len() != 40 || !checksum.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid asset hash".into());
        }
        let path = cache.join("objects").join(checksum);
        assets.push((name, path.clone()));
        if seen.insert(checksum) {
            files.push(Download::from(&serde_json::json!({"url":format!("https://resources.download.minecraft.net/{}/{checksum}", &checksum[..2]), "sha1":checksum, "size":object["size"]}), path)?);
        }
    }
    let total: u64 = files.iter().map(|f| f.size).sum();
    log::info!(
        "Minecraft {version}: downloading {} files directly from Mojang ({} MiB)",
        files.len(),
        total / 1048576
    );
    report(InstallStage::Download, 0, total);
    let next = AtomicUsize::new(0);
    let done = AtomicU64::new(0);
    let error = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                loop {
                    if error.lock().unwrap().is_some() {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = files.get(i) else { break };
                    if let Err(e) = file.get() {
                        *error.lock().unwrap() = Some(e.to_string());
                        break;
                    }
                    report(
                        InstallStage::Download,
                        done.fetch_add(file.size, Ordering::Relaxed) + file.size,
                        total,
                    );
                }
            });
        }
    });
    if let Some(error) = error.into_inner().unwrap() {
        return Err(error.into());
    }
    let versions = home.join(".local/horizon-client");
    fs::create_dir_all(&versions)?;
    let stage = versions.join("official.partial");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir(&stage)?;
    let result = (|| -> Result<()> {
        report(InstallStage::Extract, 0, 0);
        let mut zip = zip::ZipArchive::new(files[0].open()?).map_err(|error| {
            format!("Could not unpack verified Minecraft client archive: {error}")
        })?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name()?;
            if !(name.starts_with("assets/") || name.starts_with("data/")) || entry.is_dir() {
                continue;
            }
            let name = entry.enclosed_name().ok_or("Unsafe client resource path")?;
            if entry.is_symlink() {
                return Err("Symlink in client resources".into());
            }
            let path = stage.join(name);
            fs::create_dir_all(path.parent().ok_or("Missing resource parent")?)?;
            std::io::copy(&mut entry, &mut fs::File::create(path)?)?;
        }
        let downloads: std::collections::HashMap<_, _> = files
            .iter()
            .map(|file| (file.path.as_path(), file))
            .collect();
        for (name, from) in assets {
            let path = stage.join("assets").join(relative(name)?);
            fs::create_dir_all(path.parent().ok_or("Missing asset parent")?)?;
            let input = downloads
                .get(from.as_path())
                .ok_or("Missing asset download metadata")?;
            // Hold the verified source open throughout the copy, even if its
            // disposable cache directory is reclaimed in the meantime.
            std::io::copy(&mut input.open()?, &mut fs::File::create(&path)?)
                .map_err(|error| format!("Could not prepare Minecraft asset {name}: {error}"))?;
        }
        crate::resource_install::extract(&home.join("engine-data.zip"), &stage)
            .map_err(|error| format!("Could not unpack APK engine tables: {error}"))?;
        report(InstallStage::WorldData, 0, 0);
        crate::android_resources::rebuild(&stage)?;
        report(InstallStage::Controls, 0, 0);
        crate::android_resources::controls_with_progress(&stage, |done, total| {
            report(InstallStage::Controls, done, total)
        })?;
        report(InstallStage::Finalize, 0, 0);
        fs::write(stage.join("android-downloads-v2.json"), b"{\"schema\":2}")?;
        // Android can evict cache files under storage pressure. The verified
        // index was already decoded above; finalization must not reopen it.
        fs::write(stage.join("asset-index.json"), serde_json::to_vec(&index)?)
            .map_err(|error| format!("Could not save prepared asset index: {error}"))?;
        if !stage.join("horizon-resources.json").is_file() {
            return Err("APK has no engine data for this version".into());
        }
        if cfg!(feature = "audio") {
            fs::write(stage.join("audio-complete.json"), b"{}")?;
        }
        let destination = versions.join(version);
        let backup = versions.join(format!("{version}.previous"));
        if backup.exists() && !destination.exists() {
            fs::rename(&backup, &destination)?;
        }
        if backup.exists() {
            fs::remove_dir_all(&backup)?;
        }
        if destination.exists() {
            fs::rename(&destination, &backup)?;
        }
        if let Err(error) = fs::rename(&stage, &destination) {
            if backup.exists() {
                let _ = fs::rename(&backup, &destination);
            }
            return Err(format!("Could not activate prepared Minecraft resources: {error}").into());
        }
        if backup.exists() {
            let _ = fs::remove_dir_all(&backup);
        }
        Ok(())
    })();
    if stage.exists() {
        let _ = fs::remove_dir_all(stage);
    }
    // `install_with_progress` holds INSTALLATION for the entire transaction,
    // so use the inner helper here rather than reacquiring the non-reentrant
    // mutex through `cleanup_completed_downloads`.
    if result.is_ok()
        && let Err(error) = cleanup_completed_downloads_inner(home)
    {
        // The installation guard is still held here; do not lock it again.
        // Resources were committed successfully; failed housekeeping must not
        // turn a working installation into a reported installation failure.
        log::warn!("Could not clean completed Minecraft downloads: {error}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CacheHome(PathBuf);
    impl CacheHome {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for CacheHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn cache_home() -> CacheHome {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = fs::canonicalize(std::env::temp_dir()).unwrap();
        let path = base.join(format!(
            "horizon-installer-cache-test-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        let home = CacheHome(path);
        let cache = home
            .path()
            .join(".local/downloads")
            .join(horizon_core::VANILLA_VERSION);
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("client.jar"), b"download input").unwrap();
        fs::create_dir_all(home.path().join("saves")).unwrap();
        fs::write(home.path().join("saves/keep"), b"player world").unwrap();
        home
    }

    fn mark_cache_install_ready(home: &Path) {
        let root = home
            .join(".local/horizon-client")
            .join(horizon_core::VANILLA_VERSION);
        fs::create_dir_all(&root).unwrap();
        for marker in [
            "horizon-resources.json",
            "android-downloads-v2.json",
            "audio-complete.json",
        ] {
            fs::write(root.join(marker), b"{}").unwrap();
        }
        fs::write(root.join("prepared-data"), b"runtime data").unwrap();
    }

    #[test]
    fn cleanup_waits_until_installation_releases_its_inputs() {
        let home = cache_home();
        mark_cache_install_ready(home.path());
        let guard = INSTALLATION.lock().unwrap();
        let path = home.path().to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = cleanup_completed_downloads(&path).map_err(|e| e.to_string());
            tx.send(result).unwrap();
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(30))
                .is_err()
        );
        assert!(
            home.path()
                .join(".local/downloads")
                .join(horizon_core::VANILLA_VERSION)
                .join("client.jar")
                .is_file()
        );
        drop(guard);
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        assert!(home.path().join("saves/keep").is_file());
    }

    #[test]
    fn completed_install_discards_only_its_download_cache() {
        let home = cache_home();
        mark_cache_install_ready(home.path());
        cleanup_download_cache(home.path()).unwrap();
        assert!(
            !home
                .path()
                .join(".local/downloads")
                .join(horizon_core::VANILLA_VERSION)
                .exists()
        );
        assert!(home.path().join("saves/keep").is_file());
        assert!(
            home.path()
                .join(".local/horizon-client")
                .join(horizon_core::VANILLA_VERSION)
                .join("prepared-data")
                .is_file()
        );
        cleanup_download_cache(home.path()).unwrap();
    }

    #[test]
    fn interrupted_install_keeps_inputs_for_retry() {
        let home = cache_home();
        cleanup_download_cache(home.path()).unwrap();
        assert!(
            home.path()
                .join(".local/downloads")
                .join(horizon_core::VANILLA_VERSION)
                .join("client.jar")
                .is_file()
        );
    }

    #[test]
    fn android_cache_location_is_disposable_after_preparation() {
        let home = cache_home();
        let android_cache = home.path().join("android-cache");
        configure_download_cache_dir(android_cache.clone());
        let cache = download_cache(home.path());
        assert_eq!(
            cache,
            android_cache
                .join("minecraft-downloads")
                .join(horizon_core::VANILLA_VERSION)
        );
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("client.jar"), b"download").unwrap();
        cleanup_completed_downloads(home.path()).unwrap();
        assert!(cache.is_dir());
        mark_cache_install_ready(home.path());
        cleanup_completed_downloads(home.path()).unwrap();
        assert!(!cache.exists());
        assert!(
            !home
                .path()
                .join(".local/downloads")
                .join(horizon_core::VANILLA_VERSION)
                .exists()
        );
        assert!(home.path().join("saves/keep").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn redirected_cache_cannot_remove_a_different_directory() {
        let home = cache_home();
        mark_cache_install_ready(home.path());
        let cache = home
            .path()
            .join(".local/downloads")
            .join(horizon_core::VANILLA_VERSION);
        fs::remove_dir_all(&cache).unwrap();
        std::os::unix::fs::symlink(home.path().join("saves"), &cache).unwrap();
        assert!(cleanup_download_cache(home.path()).is_err());
        assert!(home.path().join("saves/keep").is_file());
    }
    fn manifest_fixture() -> (Value, Value) {
        let pins = serde_json::json!({
            "manifest_sha1": "old-metadata-hash",
            "client_sha1": "client-jar-hash",
            "server_sha1": "server-jar-hash"
        });
        let manifest = serde_json::json!({
            "id": "test-version",
            "downloads": {
                "client": {"sha1": "client-jar-hash"},
                "server": {"sha1": "server-jar-hash"}
            },
            "releaseTime": "new-metadata"
        });
        (manifest, pins)
    }
    fn check_manifest(manifest: &Value, pins: &Value) -> Result<Value> {
        let bytes = serde_json::to_vec(manifest)?;
        let checksum = format!("{:x}", Sha1::digest(&bytes));
        validate_manifest(&bytes, &checksum, "test-version", pins)
    }
    #[test]
    fn republished_metadata_keeps_the_pinned_game_usable() {
        let (manifest, pins) = manifest_fixture();
        assert!(check_manifest(&manifest, &pins).is_ok());
        let mut tampered = serde_json::to_vec(&manifest).unwrap();
        let checksum = format!("{:x}", Sha1::digest(&tampered));
        tampered.push(b' ');
        assert!(validate_manifest(&tampered, &checksum, "test-version", &pins).is_err());
    }
    #[test]
    fn metadata_cannot_replace_either_jar_or_the_game_version() {
        let (manifest, pins) = manifest_fixture();
        for side in ["client", "server"] {
            let mut changed = manifest.clone();
            changed["downloads"][side]["sha1"] = Value::String("other-jar".into());
            assert!(check_manifest(&changed, &pins).is_err());
            let mut missing = pins.clone();
            missing
                .as_object_mut()
                .unwrap()
                .remove(&format!("{side}_sha1"));
            changed["downloads"][side]
                .as_object_mut()
                .unwrap()
                .remove("sha1");
            assert!(check_manifest(&changed, &missing).is_err());
        }
        let mut changed = manifest;
        changed["id"] = Value::String("other-version".into());
        assert!(check_manifest(&changed, &pins).is_err());
    }
    #[test]
    fn only_mojang_https_is_accepted() {
        assert!(official_url("https://piston-data.mojang.com/objects/game.jar").is_ok());
        for url in [
            "http://piston-data.mojang.com/game.jar",
            "https://piston-data.mojang.com.evil.test/game.jar",
            "https://piston-data.mojang.com@evil.test/game.jar",
            "http://127.0.0.1:8123/game.zip",
        ] {
            assert!(official_url(url).is_err());
        }
    }
    #[test]
    fn asset_paths_cannot_escape_installation() {
        assert!(relative("minecraft/lang/en_us.json").is_ok());
        for name in [
            "../escape",
            "/absolute",
            "minecraft/../escape",
            "minecraft\\..\\escape",
        ] {
            assert!(relative(name).is_err());
        }
    }
}
