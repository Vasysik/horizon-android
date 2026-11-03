//! Allowlisted document tree, independent of Android and the running game.
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

pub const ROOT: &str = "horizon";
const DIRECTORY: &str = "vnd.android.document/directory";
const MAX_CHILDREN: usize = 32768;
static GAME_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_game_active(active: bool) {
    GAME_ACTIVE.store(active, std::sync::atomic::Ordering::Release);
}
fn world_access(id: &str) -> io::Result<()> {
    if id.split('/').nth(1) == Some("worlds")
        && GAME_ACTIVE.load(std::sync::atomic::Ordering::Acquire)
    {
        return Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "Return to the launcher before copying or editing worlds",
        ));
    }
    Ok(())
}

fn invalid() -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, "Invalid Horizon document")
}
fn component(name: &str) -> io::Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.len() > 240
        || name.chars().any(|c| c.is_control() || "/\\:".contains(c))
    {
        return Err(invalid());
    }
    Ok(())
}

pub fn folder_id(profile: u32, folder: &str) -> String {
    if folder == "reports" {
        "reports".into()
    } else {
        format!("p{profile}/{folder}")
    }
}
pub fn profile(home: &Path) -> u32 {
    fs::read(home.join("android-launcher.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["profile"].as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}
struct Document {
    path: PathBuf,
    label: Option<&'static str>,
    reports: bool,
}
fn shareable_report(name: &str) -> bool {
    (name.starts_with("horizon-diagnostics-") && name.ends_with(".zip")) || name.ends_with(".jsonl")
}
fn resolve(home: &Path, id: &str) -> io::Result<Document> {
    let mut parts = id.split('/');
    let first = parts.next().ok_or_else(invalid)?;
    let (base, label, reports) = if first == "reports" {
        (home.join("performance"), "Reports", true)
    } else {
        let number = first.strip_prefix('p').ok_or_else(invalid)?;
        let profile: u32 = number.parse().map_err(|_| invalid())?;
        // IDs are canonical and remain tied to their profile after switching.
        if format!("p{profile}") != first {
            return Err(invalid());
        }
        let root = if profile == 0 {
            home.to_owned()
        } else {
            home.join("profiles").join(format!("profile-{profile}"))
        };
        let (folder, label) = match parts.next() {
            Some("mods") => ("horizon-mods", "Mods"),
            Some("packs") => ("resourcepacks", "Resource Packs"),
            Some("shaders") => ("shaderpacks", "Shader Packs"),
            Some("worlds") => ("saves", "Worlds"),
            _ => return Err(invalid()),
        };
        (root.join(folder), label, false)
    };
    let tail: Vec<_> = parts.collect();
    for name in &tail {
        component(name)?;
    }
    // Internal session/error logs and raw OS traces are private. Sharing goes
    // through the bounded, redacted ZIP exporter; retain standalone performance
    // reports for existing workflows, including previously granted document IDs.
    if reports && !tail.is_empty() && (tail.len() != 1 || !shareable_report(tail[0])) {
        return Err(invalid());
    }
    let mut path = base.clone();
    for name in &tail {
        path.push(name);
    }
    // Reject every symlink, including a replaced profile or allowlisted root.
    let relative = path.strip_prefix(home).map_err(|_| invalid())?;
    let mut checked = home.to_owned();
    for part in relative.components() {
        checked.push(part);
        match fs::symlink_metadata(&checked) {
            Ok(m) if m.file_type().is_symlink() => return Err(invalid()),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    if tail.is_empty() {
        fs::create_dir_all(&base)?;
    }
    Ok(Document {
        path,
        label: tail.is_empty().then_some(label),
        reports,
    })
}
fn row(home: &Path, id: &str) -> io::Result<Value> {
    if id == ROOT {
        return Ok(
            json!({"document_id": ROOT, "_display_name": "Horizon Android",
            "mime_type": DIRECTORY, "flags": 0}),
        );
    }
    let doc = resolve(home, id)?;
    let meta = fs::symlink_metadata(&doc.path)?;
    if !meta.is_dir() && !meta.is_file() {
        return Err(invalid());
    }
    let name = doc.label.map(str::to_owned).unwrap_or_else(|| {
        doc.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    let mime = if meta.is_dir() {
        DIRECTORY
    } else {
        match doc.path.extension().and_then(|s| s.to_str()) {
            Some("jsonl") => "application/x-ndjson",
            Some("zip" | "hmod" | "hshader") => "application/zip",
            Some("png") => "image/png",
            Some("json") => "application/json",
            Some("txt" | "wgsl" | "glsl" | "fsh" | "vsh" | "toml") => "text/plain",
            _ => "application/octet-stream",
        }
    };
    // Android DocumentsContract.Document flags. Fixed roots cannot be removed.
    let flags = if doc.reports {
        0
    } else if meta.is_dir() {
        8
    } else {
        2
    };
    let flags = flags
        | if doc.label.is_none() && !doc.reports {
            4 | if meta.is_file() { 64 } else { 0 }
        } else {
            0
        };
    Ok(
        json!({"document_id": id, "_display_name": name, "mime_type": mime,
        "flags": flags, "_size": meta.len(), "last_modified": meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|t| t.as_millis().min(i64::MAX as u128) as i64).unwrap_or(0)}),
    )
}
pub fn children(home: &Path, id: &str) -> io::Result<Vec<Value>> {
    if id == ROOT {
        return ["mods", "packs", "shaders", "worlds", "reports"]
            .iter()
            .map(|folder| row(home, &folder_id(profile(home), folder)))
            .collect();
    }
    world_access(id)?;
    let doc = resolve(home, id)?;
    let mut result = Vec::new();
    for (index, entry) in fs::read_dir(doc.path)?.enumerate() {
        if index == MAX_CHILDREN {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Too many files in one folder to list safely",
            ));
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if component(name).is_err() || entry.file_type()?.is_symlink() {
            continue;
        }
        let child = format!("{id}/{name}");
        if let Ok(value) = row(home, &child) {
            result.push(value);
        }
    }
    result.sort_by(|a, b| {
        a["_display_name"]
            .as_str()
            .cmp(&b["_display_name"].as_str())
    });
    Ok(result)
}
pub fn is_child(home: &Path, parent: &str, child: &str) -> bool {
    child != ROOT
        && child != parent
        && resolve(home, child).is_ok()
        && (parent == ROOT
            || (resolve(home, parent).is_ok()
                && child
                    .strip_prefix(parent)
                    .is_some_and(|rest| rest.starts_with('/'))))
}
pub fn open(home: &Path, id: &str, mode: &str) -> io::Result<File> {
    world_access(id)?;
    let doc = resolve(home, id)?;
    if !fs::metadata(&doc.path)?.is_file() || (doc.reports && mode != "r") {
        return Err(invalid());
    }
    let mut options = OpenOptions::new();
    match mode {
        "r" => {
            options.read(true);
        }
        "w" | "wt" => {
            options.write(true).truncate(true);
        }
        "wa" => {
            options.append(true);
        }
        "rw" => {
            options.read(true).write(true);
        }
        "rwt" => {
            options.read(true).write(true).truncate(true);
        }
        _ => return Err(invalid()),
    }
    options.open(doc.path)
}
pub fn call(home: &Path, op: &str, id: &str, arg: &str) -> io::Result<Value> {
    if !matches!(op, "document" | "path" | "child") {
        world_access(id)?;
    }
    match op {
        "roots" => Ok(json!([{"root_id": ROOT, "document_id": ROOT,
            "title": "Horizon Android", "flags": 18, "mime_types": "*/*"}])),
        "document" => Ok(json!([row(home, id)?])),
        "children" => Ok(json!(children(home, id)?)),
        "child" => Ok(json!(is_child(home, id, arg))),
        "path" => {
            row(home, id)?;
            let mut path = vec![ROOT.to_owned()];
            if id != ROOT {
                let mut parts = id.split('/');
                let first = parts.next().ok_or_else(invalid)?;
                let mut current = if first == "reports" {
                    first.into()
                } else {
                    format!("{first}/{}", parts.next().ok_or_else(invalid)?)
                };
                path.push(current.clone());
                for name in parts {
                    current.push('/');
                    current.push_str(name);
                    path.push(current.clone());
                }
            }
            if !arg.is_empty() {
                let offset = path.iter().position(|p| p == arg).ok_or_else(invalid)?;
                path.drain(..offset);
            }
            Ok(json!(path))
        }
        "create" | "mkdir" => {
            component(arg)?;
            let parent = resolve(home, id)?;
            if parent.reports || !parent.path.is_dir() {
                return Err(invalid());
            }
            // create_new never overwrites a name. Let DocumentsUI request another name.
            let child = format!("{id}/{arg}");
            let target = resolve(home, &child)?;
            if op == "mkdir" {
                fs::create_dir(target.path)?;
            } else {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target.path)?;
            }
            Ok(json!(child))
        }
        "rename" | "delete" => {
            let doc = resolve(home, id)?;
            if doc.label.is_some() || doc.reports {
                return Err(invalid());
            }
            if op == "delete" {
                // Nonempty directories must be emptied by the client explicitly.
                if doc.path.is_dir() {
                    fs::remove_dir(doc.path)?;
                } else {
                    fs::remove_file(doc.path)?;
                }
                Ok(Value::Null)
            } else {
                if !doc.path.is_file() {
                    return Err(invalid());
                }
                component(arg)?;
                let (parent, _) = id.rsplit_once('/').ok_or_else(invalid)?;
                let child = format!("{parent}/{arg}");
                let target = resolve(home, &child)?;
                if target.path.try_exists()? {
                    return Err(io::Error::from(ErrorKind::AlreadyExists));
                }
                rename_new(&doc.path, &target.path)?;
                Ok(json!(child))
            }
        }
        _ => Err(invalid()),
    }
}

fn rename_new(source: &Path, target: &Path) -> io::Result<()> {
    #[cfg(target_os = "android")]
    {
        // Android SELinux disallows hard links. renameat2 preserves no-overwrite
        // semantics even if the installer creates the target concurrently.
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            source,
            rustix::fs::CWD,
            target,
            rustix::fs::RenameFlags::NOREPLACE,
        )?;
    }
    #[cfg(not(target_os = "android"))]
    {
        fs::hard_link(source, target)?;
        if let Err(error) = fs::remove_file(source) {
            let _ = fs::remove_file(target);
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundaries_profiles_and_non_overwriting_operations() {
        let home = std::env::temp_dir().join(format!(
            "horizon-documents-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&home).unwrap();
        let roots = children(&home, ROOT).unwrap();
        assert_eq!(roots.len(), 5);
        assert_eq!(roots[0]["document_id"], "p0/mods");
        for id in [
            "../tokens",
            "p0/mods/../account.json",
            "p0/mods/a\\b",
            "p0/accounts",
            "p00/mods",
            "p0/mods/.secret",
        ] {
            assert!(resolve(&home, id).is_err(), "{id}");
        }
        call(&home, "create", "p0/mods", "test.hmod").unwrap();
        assert!(call(&home, "create", "p0/mods", "test.hmod").is_err());
        fs::write(home.join("horizon-mods/test.hmod"), b"original").unwrap();
        call(&home, "create", "p0/mods", "other.hmod").unwrap();
        assert!(call(&home, "rename", "p0/mods/other.hmod", "test.hmod").is_err());
        assert_eq!(
            fs::read(home.join("horizon-mods/test.hmod")).unwrap(),
            b"original"
        );
        assert!(call(&home, "delete", "p0/mods", "").is_err());
        assert!(call(&home, "create", "reports", "bad.txt").is_err());
        fs::write(home.join("performance/report.jsonl"), b"{}").unwrap();
        fs::write(home.join("performance/current-session.json"), b"{}").unwrap();
        fs::write(home.join("performance/exit-1.trace"), b"private trace").unwrap();
        assert!(open(&home, "reports/exit-1.trace", "r").is_err());
        assert!(open(&home, "reports/current-session.json", "r").is_err());
        assert_eq!(children(&home, "reports").unwrap().len(), 1);
        assert!(open(&home, "reports/report.jsonl", "r").is_ok());
        assert!(open(&home, "reports/report.jsonl", "w").is_err());
        assert!(!is_child(&home, "p0/mods", "p0/packs/pack.zip"));
        assert!(is_child(&home, ROOT, "p0/mods/test.hmod"));
        assert_eq!(
            call(&home, "path", "p0/mods/test.hmod", "").unwrap(),
            json!(["horizon", "p0/mods", "p0/mods/test.hmod"])
        );
        assert!(call(&home, "path", "p0/mods/test.hmod", "p0/packs").is_err());
        fs::write(home.join("android-launcher.json"), br#"{"profile":7}"#).unwrap();
        assert_eq!(children(&home, ROOT).unwrap()[0]["document_id"], "p7/mods");
        assert!(open(&home, "p0/mods/test.hmod", "r").is_ok());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(home.join("performance"), home.join("horizon-mods/link"))
                .unwrap();
            assert!(resolve(&home, "p0/mods/link/report.jsonl").is_err());
            assert!(!is_child(&home, ROOT, "p0/mods/link/report.jsonl"));
        }
        fs::remove_dir_all(home).unwrap();
    }
}

#[cfg(test)]
mod world_tests {
    use super::*;
    #[test]
    fn worlds_roundtrip_and_busy_guard() {
        let home =
            std::env::temp_dir().join(format!("horizon-world-documents-{}", std::process::id()));
        fs::create_dir_all(&home).unwrap();
        let id = call(&home, "mkdir", "p0/worlds", "Example").unwrap();
        assert_eq!(id, "p0/worlds/Example");
        call(&home, "create", id.as_str().unwrap(), "level.dat").unwrap();
        use std::io::{Read, Write};
        open(&home, "p0/worlds/Example/level.dat", "wt")
            .unwrap()
            .write_all(b"fixture")
            .unwrap();
        let mut data = Vec::new();
        open(&home, "p0/worlds/Example/level.dat", "r")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"fixture");
        set_game_active(true);
        let blocked = open(&home, "p0/worlds/Example/level.dat", "r").is_err()
            && call(&home, "delete", "p0/worlds/Example/level.dat", "").is_err()
            && children(&home, "p0/worlds").is_err();
        set_game_active(false);
        assert!(blocked);
        assert!(open(&home, "p0/worlds/Example/level.dat", "r").is_ok());
        fs::remove_dir_all(home).unwrap();
    }
}
