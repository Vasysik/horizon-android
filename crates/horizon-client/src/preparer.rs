//! The game files prepared on the device, where no computer does it (Android): once the sign-in
//! has fetched the official files (accounts.rs), the Java runtime (starling_account::java) and
//! the preparer's kit the app carries (`assets/prepare-kit.zip`, build_apk.py), then
//! horizon-prepare over those files with the app's Java launchers. What it makes becomes the
//! game's resources folder; the runtime goes once it is done.
use starling_account::java::JRE25_ANDROID_ARM64 as RUNTIME;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Where the preparing is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stage {
    #[default]
    Idle,
    /// Fetching the Java runtime: bytes so far, and in all.
    Java(u64, u64),
    /// Unpacking it.
    Unpacking,
    /// horizon-prepare at work: its last line.
    Preparing(String),
    /// The resources are in place.
    Done,
    Failed(String),
}

fn shared() -> MutexGuard<'static, Stage> {
    static STAGE: OnceLock<Mutex<Stage>> = OnceLock::new();
    STAGE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// Where the preparing is now.
pub fn stage() -> Stage {
    shared().clone()
}

/// Back to idle once what it made was taken (the game loads it).
pub fn clear() {
    let mut stage = shared();
    if *stage == Stage::Done {
        *stage = Stage::Idle;
    }
}

/// Whether it is under way.
pub fn running() -> bool {
    matches!(
        stage(),
        Stage::Java(..) | Stage::Unpacking | Stage::Preparing(_)
    )
}

/// The runtime, the kit and horizon-prepare's work files: the app's own folder.
fn home() -> PathBuf {
    crate::logging::private_home().join("preparer")
}

/// Prepare the fetched files into `resources` (the folder the game reads), on a thread of its
/// own; `stage` follows it.
pub fn start(resources: PathBuf) {
    {
        let mut stage = shared();
        if matches!(
            *stage,
            Stage::Java(..) | Stage::Unpacking | Stage::Preparing(_)
        ) {
            return;
        }
        *stage = Stage::Java(0, RUNTIME.size);
    }
    let spawned = std::thread::Builder::new()
        .name("preparer".into())
        .spawn(move || {
            let started = std::time::Instant::now();
            let result = run(&resources);
            *shared() = match result {
                Ok(()) => {
                    log::info!(
                        "Horizon: the game files were prepared in {} s",
                        started.elapsed().as_secs()
                    );
                    Stage::Done
                }
                Err(e) => {
                    log::warn!("Horizon: the game files were not prepared: {e}");
                    Stage::Failed(e)
                }
            };
        });
    if let Err(e) = spawned {
        *shared() = Stage::Failed(e.to_string());
    }
}

fn run(resources: &Path) -> Result<(), String> {
    let server_jar = crate::accounts::server_jar()
        .ok_or("The game files are not all there: download them again.")?;
    let library = crate::platform::library_dir().ok_or("Starling cannot find its own files.")?;
    let home = home();
    let jre = home.join("jre25");
    starling_account::java::install(
        &RUNTIME,
        &jre,
        &|done, total| {
            *shared() = if done >= total {
                Stage::Unpacking
            } else {
                Stage::Java(done, total)
            };
        },
        &|| false,
    )?;
    let kit = home.join("kit");
    unpack_kit(&kit)?;
    let temp = home.join("tmp");
    std::fs::create_dir_all(&temp).map_err(|e| format!("{}: {e}", temp.display()))?;
    let env = |path: &Path| path.display().to_string();
    horizon_prepare::java::supply(horizon_prepare::java::Supplied {
        java: library.join("libstarling_java.so"),
        javac: library.join("libstarling_javac.so"),
        env: vec![
            ("STARLING_JAVA_HOME".into(), env(&jre)),
            // The runtime's libraries need one another by name (libnio.so libnet.so): Android's
            // linker finds them on this path only. Not lib/server, libjvm.so's (loaded by its
            // path): with that on it the launcher runs the runtime's bin/java again, which an
            // app may not.
            (
                "LD_LIBRARY_PATH".into(),
                format!("{}:{}", env(&jre.join("lib")), env(&library)),
            ),
            ("JAVA_HOME".into(), env(&jre)),
            ("TMPDIR".into(), env(&temp)),
            (
                "JAVA_TOOL_OPTIONS".into(),
                format!("-Djava.io.tmpdir={}", env(&temp)),
            ),
        ],
    });
    *shared() = Stage::Preparing("Starting Java".into());
    let made = horizon_prepare::prepare(
        &horizon_prepare::Options {
            root: kit.clone(),
            import: true,
            minecraft_dir: Some(crate::accounts::files_root()),
            server_jar: Some(server_jar),
            java_home: None,
        },
        &|line| {
            log::info!("Horizon: preparing: {line}");
            *shared() = Stage::Preparing(line.to_owned());
        },
    )?;
    // The sounds: `prepare` leaves them out (the desktop launcher copies them after it,
    // backend.rs). Without them the game plays, silent.
    if let Err(e) = copy_sounds(&kit) {
        log::warn!("Horizon: no sounds (the game will be silent): {e}");
    }
    // The prepared folder becomes the resources; the ones there did not load.
    if resources.exists() {
        std::fs::remove_dir_all(resources).map_err(|e| format!("{}: {e}", resources.display()))?;
    }
    if let Some(parent) = resources.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::rename(&made, resources).map_err(|e| format!("{}: {e}", resources.display()))?;
    // Bedrock's touch buttons, from Mojang too, so the first game already has them.
    crate::bedrock_touch::fetch_logged(resources);
    // Everything the game reads is in the resources now: the runtime and the work files go.
    for done in [&jre, &kit.join(".local"), &temp] {
        let _ = std::fs::remove_dir_all(done);
    }
    Ok(())
}

/// The fetched files' sounds into `kit`'s prepared folder (horizon-prepare's `sounds`).
fn copy_sounds(kit: &Path) -> Result<(), String> {
    horizon_prepare::sounds(kit, Some(&crate::accounts::files_root()), &|line| {
        log::info!("Horizon: preparing: {line}");
        *shared() = Stage::Preparing(line.to_owned());
    })
    .map(drop)
}

/// Starling before 0.1.6 prepared the game without its sounds: once the fetched files are
/// there, they are added to the prepared `resources`, before the game reads its sounds.
pub fn complete_sounds(resources: &Path) {
    if !crate::platform::registered()
        || !resources.is_dir()
        || resources.join("assets/minecraft/sounds.json").is_file()
        || crate::accounts::files() != crate::accounts::Files::Ready
    {
        return;
    }
    match add_sounds(resources) {
        Ok(()) => log::info!(
            "Horizon: the game's sounds were added to {}",
            resources.display()
        ),
        Err(e) => log::warn!("Horizon: the game's sounds were not added: {e}"),
    }
}

/// horizon-prepare copies into the kit's prepared folder: a link to `resources` stands in for
/// it while it does.
#[cfg(unix)]
fn add_sounds(resources: &Path) -> Result<(), String> {
    let kit = home().join("kit");
    unpack_kit(&kit)?;
    let version = horizon_prepare::artifacts::Lock::read(&kit)?.version;
    let target = kit.join(".local/horizon-client").join(&version);
    // A link left by a stop midway, or the work of a preparation cut short.
    let _ = std::fs::remove_file(&target);
    let _ = std::fs::remove_dir_all(&target);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::os::unix::fs::symlink(resources, &target)
        .map_err(|e| format!("{}: {e}", target.display()))?;
    let copied = horizon_prepare::sounds(&kit, Some(&crate::accounts::files_root()), &|line| {
        log::info!("Horizon: adding the sounds: {line}")
    });
    let _ = std::fs::remove_file(&target);
    copied.map(drop)
}

#[cfg(not(unix))]
fn add_sounds(_: &Path) -> Result<(), String> {
    Err("only a phone's setup prepares on the device".into())
}

/// The low-budget skin the setup draws the game's menus with while their own resources are not
/// there (packaging/lowbudget, the APK's `assets/lowbudget.zip`): a resource folder, unpacked
/// into the app's own folder once for each skin the app carries.
pub fn skin() -> Option<PathBuf> {
    let bytes = crate::platform::asset("lowbudget.zip")?;
    let stamp = format!("{}-{:08x}", bytes.len(), crc32(&bytes));
    let dir = home().join("lowbudget");
    let marker = dir.join(".starling-skin");
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(&stamp) {
        return Some(dir);
    }
    let _ = std::fs::remove_dir_all(&dir);
    match unpack(&bytes, &dir)
        .and_then(|()| std::fs::write(&marker, &stamp).map_err(|e| e.to_string()))
    {
        Ok(()) => Some(dir),
        Err(e) => {
            log::warn!("Horizon: the setup's skin did not unpack: {e}");
            None
        }
    }
}

/// Starling Visuals, the shader pack the app carries (packaging/shaderpacks/starling-visuals, the
/// APK's `assets/StarlingVisuals.zip`), put in `shaderpacks/` of the game folder: again only when
/// the app carries another one than it put there last, so a pack the player deleted stays deleted
/// until an update brings a new one.
pub fn put_shader_pack(game_folder: &Path) {
    let Some(bytes) = crate::platform::asset("StarlingVisuals.zip") else {
        return;
    };
    let stamp = format!("{}-{:08x}", bytes.len(), crc32(&bytes));
    let marker = crate::logging::private_home().join(".starling-visuals");
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(&stamp) {
        return;
    }
    let dir = game_folder.join("shaderpacks");
    match std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join("StarlingVisuals.zip"), &bytes))
        .and_then(|()| std::fs::write(&marker, &stamp))
    {
        Ok(()) => log::info!("Horizon: Starling Visuals put in {}", dir.display()),
        Err(e) => log::warn!(
            "Horizon: Starling Visuals could not be put in {}: {e}",
            dir.display()
        ),
    }
}

/// CRC-32 (IEEE), to tell one skin from the next.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The kit the app carries, over `dir` (its sources replaced, horizon-prepare's work kept).
fn unpack_kit(dir: &Path) -> Result<(), String> {
    let bytes = crate::platform::asset("prepare-kit.zip")
        .ok_or("Starling's preparer is missing from the app: install it again.")?;
    unpack(&bytes, dir)
}

/// A zip the app carries over `dir`, its entries kept inside it.
fn unpack(bytes: &[u8], dir: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            return Err(format!("The app's archive names {}", entry.name()));
        };
        let path = dir.join(name);
        if entry.is_dir() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut file =
            std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}
