//! A content mod's assets from Modrinth (Starling; `horizon_mods::fetch`): the jar its manifest
//! names, downloaded on the player's device when the mod is first on, checked by its SHA-512 and
//! unpacked into the cache. Nothing of the jar but the folders the manifest takes is kept, and
//! none of its code runs.
use horizon_mods::{Manifest, fetch};
use sha2::{Digest, Sha512};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// Why a mod's assets could not be fetched at this start, by mod id (the Mods page says so).
static FAILED: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// Why the assets of mod `id` could not be fetched at this start, if they could not.
pub(super) fn failed(id: &str) -> Option<String> {
    FAILED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(m, _)| m == id)
        .map(|(_, e)| e.clone())
}

/// Modrinth asks for an agent naming the program and where to reach its makers.
fn agent_name() -> String {
    format!(
        "JuanLuci/starling/{} (github.com/JuanLuci/starling)",
        version()
    )
}

/// The APK's version on a phone, else the program's.
#[cfg(target_os = "android")]
fn version() -> String {
    crate::android::app_version().map_or_else(|| env!("CARGO_PKG_VERSION").to_owned(), |(v, _)| v)
}
#[cfg(not(target_os = "android"))]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// The assets `manifest` fetches, into `dir` ([`fetch::dir`]), with `progress` hearing the bytes
/// so far and in all. The failure is kept for the Mods page.
pub(super) fn get(
    manifest: &Manifest,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<usize, String> {
    let result = download(manifest, dir, progress);
    let mut failed = FAILED.lock().unwrap_or_else(PoisonError::into_inner);
    failed.retain(|(m, _)| *m != manifest.id);
    if let Err(e) = &result {
        failed.push((manifest.id.clone(), e.clone()));
    }
    result
}

fn download(
    manifest: &Manifest,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<usize, String> {
    let Some(fetch) = manifest.assets.as_ref().map(|a| &a.fetch) else {
        return Ok(0);
    };
    // No limit on the whole download (a slow network), only on each wait.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();
    let name = agent_name();
    let text = agent
        .get(&fetch::version_url(fetch))
        .set("User-Agent", &name)
        .call()
        .map_err(|e| format!("Modrinth: {e}"))?
        .into_string()
        .map_err(|e| format!("Modrinth: {e}"))?;
    let version: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Modrinth: {e}"))?;
    let url = fetch::file_url(&version, fetch)?;
    let mut body = agent
        .get(&url)
        .set("User-Agent", &name)
        .call()
        .map_err(|e| format!("{}: {e}", fetch.file))?
        .into_reader();
    let parent = dir.parent().unwrap_or(dir);
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let jar = dir.with_extension("jar");
    let mut file = std::fs::File::create(&jar).map_err(|e| format!("{}: {e}", jar.display()))?;
    let mut hash = Sha512::new();
    let mut buffer = vec![0; 256 * 1024];
    let mut done = 0u64;
    let fail = |e: String| {
        let _ = std::fs::remove_file(&jar);
        e
    };
    loop {
        let n = body
            .read(&mut buffer)
            .map_err(|e| fail(format!("{}: {e}", fetch.file)))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > fetch.size {
            return Err(fail(format!("{} is bigger than it should be", fetch.file)));
        }
        hash.update(&buffer[..n]);
        file.write_all(&buffer[..n])
            .map_err(|e| fail(format!("{}: {e}", jar.display())))?;
        progress(done, fetch.size);
    }
    drop(file);
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != fetch.sha512 {
        return Err(fail(format!(
            "{} came out damaged (another hash); try again",
            fetch.file
        )));
    }
    let unpacked = fetch::unpack(&jar, manifest, dir);
    let _ = std::fs::remove_file(&jar);
    unpacked
}
