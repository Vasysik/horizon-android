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

/// Tries in a row that brought no byte before the download gives up (the next start goes on).
const QUIET_TRIES: u32 = 12;

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

/// One try at the file, from byte `have` on (a `Range` request; a server that sends the whole file
/// again is followed from its start). The bytes reach `part` as they come, so a drop loses nothing
/// already written.
fn fetch_part(
    agent: &ureq::Agent,
    name: &str,
    fetch: &horizon_mods::manifest::Fetch,
    url: &mut Option<String>,
    part: &Path,
    have: u64,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    if url.is_none() {
        let text = agent
            .get(&fetch::version_url(fetch))
            .set("User-Agent", name)
            .call()
            .map_err(|e| format!("Modrinth: {e}"))?
            .into_string()
            .map_err(|e| format!("Modrinth: {e}"))?;
        let version: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("Modrinth: {e}"))?;
        *url = Some(fetch::file_url(&version, fetch)?);
    }
    let mut request = agent
        .get(url.as_deref().unwrap_or_default())
        .set("User-Agent", name);
    if have > 0 {
        request = request.set("Range", &format!("bytes={have}-"));
    }
    let response = request.call().map_err(|e| format!("{}: {e}", fetch.file))?;
    let resumed = response.status() == 206;
    let mut file = if resumed {
        std::fs::OpenOptions::new().append(true).open(part)
    } else {
        std::fs::File::create(part)
    }
    .map_err(|e| format!("{}: {e}", part.display()))?;
    let mut done = if resumed { have } else { 0 };
    let mut body = response.into_reader();
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let n = body
            .read(&mut buffer)
            .map_err(|e| format!("{}: {e}", fetch.file))?;
        if n == 0 {
            return Ok(());
        }
        done += n as u64;
        if done > fetch.size {
            let _ = std::fs::remove_file(part);
            return Err(format!("{} is bigger than it should be", fetch.file));
        }
        file.write_all(&buffer[..n])
            .map_err(|e| format!("{}: {e}", part.display()))?;
        progress(done, fetch.size);
    }
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
    let parent = dir.parent().unwrap_or(dir);
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let jar = dir.with_extension("jar");
    // What is there stays: a dropped connection, or a game that was closed, goes on from the byte
    // it reached. Only a long stretch with no byte at all gives up.
    let part = dir.with_extension("part");
    let size_of = |p: &Path| std::fs::metadata(p).map_or(0, |m| m.len());
    let mut url = None;
    let mut quiet = 0u32;
    loop {
        let have = size_of(&part);
        if have > fetch.size {
            let _ = std::fs::remove_file(&part);
            continue;
        }
        if have == fetch.size {
            break;
        }
        progress(have, fetch.size);
        let tried = fetch_part(&agent, &name, fetch, &mut url, &part, have, progress);
        let now = size_of(&part);
        quiet = if now > have { 0 } else { quiet + 1 };
        if let Err(e) = tried {
            log::warn!(
                "Horizon: {} stopped at {now} of {} bytes ({e}); trying again",
                fetch.file,
                fetch.size
            );
            if quiet >= QUIET_TRIES {
                return Err(format!(
                    "{e} (stopped at {} of {} MB; the next start goes on from there)",
                    now / 1_000_000,
                    fetch.size / 1_000_000
                ));
            }
            std::thread::sleep(Duration::from_secs(u64::from(quiet.min(5)) * 3 + 2));
        } else if now == have && quiet >= QUIET_TRIES {
            return Err(format!(
                "{}: the server sent nothing more at {now} bytes",
                fetch.file
            ));
        }
    }
    let mut hash = Sha512::new();
    let mut file = std::fs::File::open(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("{}: {e}", part.display()))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    drop(file);
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != fetch.sha512 {
        let _ = std::fs::remove_file(&part);
        return Err(format!(
            "{} came out damaged (another hash); try again",
            fetch.file
        ));
    }
    std::fs::rename(&part, &jar).map_err(|e| format!("{}: {e}", jar.display()))?;
    let unpacked = fetch::unpack(&jar, manifest, dir);
    let _ = std::fs::remove_file(&jar);
    unpacked
}
