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
const CONFIG_FILES: &[&str] = &[
    "options.txt",
    "horizon-graphics.txt",
    "horizon-touch.json",
    "horizon-keypad.json",
];
static GAME_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_game_active(active: bool) {
    GAME_ACTIVE.store(active, std::sync::atomic::Ordering::Release);
}
fn world_access(id: &str) -> io::Result<()> {
    if matches!(id.split('/').nth(1), Some("worlds" | "configuration"))
        && GAME_ACTIVE.load(std::sync::atomic::Ordering::Acquire)
    {
        return Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "Return to the launcher before accessing worlds or configuration",
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
    configuration: bool,
}
fn shareable_report(name: &str) -> bool {
    (name.starts_with("horizon-diagnostics-") && name.ends_with(".zip"))
        || (name.starts_with("horizon-controls-") && name.ends_with(".json"))
        || name.ends_with(".jsonl")
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
            Some("configuration") => ("", "Configuration"),
            _ => return Err(invalid()),
        };
        (root.join(folder), label, false)
    };
    let tail: Vec<_> = parts.collect();
    let configuration = label == "Configuration";
    if configuration && !tail.is_empty() && (tail.len() != 1 || !CONFIG_FILES.contains(&tail[0])) {
        return Err(invalid());
    }
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
        configuration,
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
    let flags = if doc.configuration {
        if meta.is_file() { 2 } else { 0 }
    } else if doc.reports {
        0
    } else if meta.is_dir() {
        8
    } else {
        2
    };
    let flags = flags
        | if doc.label.is_none() && !doc.reports && !doc.configuration {
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
        return [
            "mods",
            "packs",
            "shaders",
            "worlds",
            "configuration",
            "reports",
        ]
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
        if doc.configuration && !CONFIG_FILES.contains(&name) {
            continue;
        }
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
            if parent.reports || !parent.path.is_dir() || (parent.configuration && op == "mkdir") {
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
            if doc.label.is_some() || doc.reports || doc.configuration {
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
        assert_eq!(roots.len(), 6);
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
        fs::write(home.join("options.txt"), b"renderDistance:16\n").unwrap();
        fs::write(home.join("account.json"), b"private").unwrap();
        fs::write(home.join("android-launcher.json"), br#"{"profile":0}"#).unwrap();
        let config = children(&home, "p0/configuration").unwrap();
        assert_eq!(config.len(), 1);
        assert_eq!(config[0]["_display_name"], "options.txt");
        assert_eq!(config[0]["flags"], 2);
        assert!(open(&home, "p0/configuration/account.json", "r").is_err());
        assert!(open(&home, "p0/configuration/android-launcher.json", "r").is_err());
        assert!(open(&home, "p0/configuration/../account.json", "r").is_err());
        assert!(call(&home, "mkdir", "p0/configuration", "options.txt").is_err());
        assert!(call(&home, "delete", "p0/configuration/options.txt", "").is_err());
        open(&home, "p0/configuration/options.txt", "wt")
            .unwrap()
            .write_all(b"renderDistance:12\n")
            .unwrap();
        assert_eq!(
            fs::read(home.join("options.txt")).unwrap(),
            b"renderDistance:12\n"
        );
        fs::create_dir_all(home.join("profiles/profile-7")).unwrap();
        fs::write(
            home.join("profiles/profile-7/options.txt"),
            b"renderDistance:8\n",
        )
        .unwrap();
        assert_eq!(children(&home, "p7/configuration").unwrap().len(), 1);
        set_game_active(true);
        let blocked = open(&home, "p0/worlds/Example/level.dat", "r").is_err()
            && call(&home, "delete", "p0/worlds/Example/level.dat", "").is_err()
            && children(&home, "p0/worlds").is_err();
        let config_blocked = open(&home, "p0/configuration/options.txt", "r").is_err()
            && children(&home, "p0/configuration").is_err();
        set_game_active(false);
        assert!(blocked);
        assert!(config_blocked);
        assert!(open(&home, "p0/worlds/Example/level.dat", "r").is_ok());
        fs::remove_dir_all(home).unwrap();
    }
}

// ---- Integrated Mini File Manager Backend (Real Filesystem Operations) ----

pub const PROTECTED_SYSTEM_NAMES: &[&str] = &[
    "android-launcher.json",
    "android-launcher.partial",
    "engine-data.zip",
    ".local",
    "account.json",
    "accounts.json",
    "sessions",
    "installer",
    "wardrobe",
    "profiles",
];

pub const FM_ROOT_DIRS: &[(&str, &str)] = &[
    ("", "Root"),
    ("saves", "Worlds"),
    ("horizon-mods", "Mods"),
    ("resourcepacks", "Packs"),
    ("shaderpacks", "Shaders"),
    ("controls", "Controls"),
    ("screenshots", "Shots"),
    ("performance", "Reports"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FmEntry {
    pub name: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_secs: u64,
    pub kind_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FmFileDetails {
    pub name: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_label: String,
    pub kind_label: String,
    pub preview_lines: Vec<String>,
}

pub fn is_protected_system_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower.starts_with('.')
        || PROTECTED_SYSTEM_NAMES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&lower))
}

pub fn is_protected_root_folder(rel_path: &str) -> bool {
    let clean = rel_path.trim_matches('/');
    clean.is_empty()
        || matches!(
            clean,
            "saves"
                | "horizon-mods"
                | "resourcepacks"
                | "shaderpacks"
                | "controls"
                | "screenshots"
                | "performance"
        )
}

pub fn validate_entry_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty.".into());
    }
    if trimmed == "." || trimmed == ".." || trimmed.contains("..") {
        return Err("Path traversal ('..') is not allowed.".into());
    }
    if trimmed.starts_with('.') {
        return Err("Hidden or system names starting with '.' are not allowed.".into());
    }
    if trimmed.len() > 200 {
        return Err("Name is too long.".into());
    }
    if trimmed.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err("Name cannot contain slashes or special path characters.".into());
    }
    if is_protected_system_name(trimmed) {
        return Err("This name is reserved by the Horizon launcher.".into());
    }
    Ok(())
}

fn verify_no_symlinks(home: &Path, target: &Path) -> Result<(), String> {
    let rel = target
        .strip_prefix(home)
        .map_err(|_| "Path escapes the Horizon home directory.".to_owned())?;
    let mut cur = home.to_path_buf();
    for part in rel.components() {
        use std::path::Component;
        match part {
            Component::Normal(seg) => {
                let seg_str = seg.to_string_lossy();
                if seg_str == ".local" {
                    return Err("Access to internal engine storage (.local) is restricted.".into());
                }
                cur.push(seg);
            }
            Component::CurDir => continue,
            _ => return Err("Path traversal outside profile directory is forbidden.".into()),
        }
        match fs::symlink_metadata(&cur) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("Symbolic links are not permitted in Horizon directories.".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub fn resolve_fm_path(home: &Path, profile_root: &Path, rel: &str) -> Result<PathBuf, String> {
    verify_no_symlinks(home, profile_root)?;
    let clean = rel.trim().trim_matches('/');
    if clean.is_empty() {
        return Ok(profile_root.to_path_buf());
    }
    let mut parts = clean.split('/');
    let first = parts.next().unwrap_or("");
    validate_entry_name(first)?;
    let mut path = if first == "performance" {
        home.join("performance")
    } else {
        profile_root.join(first)
    };
    for seg in parts {
        validate_entry_name(seg)?;
        path.push(seg);
    }
    verify_no_symlinks(home, &path)?;
    Ok(path)
}

pub fn ensure_fm_dirs(home: &Path, profile_root: &Path) {
    let _ = fs::create_dir_all(profile_root);
    for (sub, _) in FM_ROOT_DIRS {
        if sub.is_empty() {
            continue;
        }
        let target = if *sub == "performance" {
            home.join("performance")
        } else {
            profile_root.join(sub)
        };
        let _ = fs::create_dir_all(target);
    }
}

pub fn classify_entry_kind(path: &Path, is_dir: bool) -> String {
    if is_dir {
        return "Folder".into();
    }
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("hmod") => "Horizon Mod (.hmod)".into(),
        Some("hnmod") => "Native Mod (.hnmod)".into(),
        Some("jar") => "Java Mod (.jar)".into(),
        Some("hshader") => "Horizon Shader (.hshader)".into(),
        Some("zip") => "Archive / Pack (.zip)".into(),
        Some("png") => "PNG Image (.png)".into(),
        Some("json") => "JSON Config (.json)".into(),
        Some("jsonl") => "Telemetry Log (.jsonl)".into(),
        Some("toml") => "TOML Config (.toml)".into(),
        Some("txt" | "cfg" | "properties") => "Text Config (.txt)".into(),
        Some("dat" | "mca" | "mcr") => "World Data".into(),
        Some(ext) => format!("{} File (.{ext})", ext.to_ascii_uppercase()),
        None => "File".into(),
    }
}

pub fn format_timestamp_utc(secs: u64) -> String {
    if secs == 0 {
        return "Unknown".into();
    }
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{m:02}-{d:02} {hh:02}:{mm:02} UTC")
}

pub fn list_fm_dir(
    home: &Path,
    profile_root: &Path,
    rel_dir: &str,
) -> Result<Vec<FmEntry>, String> {
    ensure_fm_dirs(home, profile_root);
    let clean_rel = rel_dir.trim().trim_matches('/');
    let dir_path = resolve_fm_path(home, profile_root, clean_rel)?;
    let meta = fs::symlink_metadata(&dir_path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("Target path is not a valid directory.".into());
    }
    let mut entries = Vec::new();
    let mut seen_names = std::collections::BTreeSet::new();
    for (idx, item) in fs::read_dir(&dir_path)
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if idx >= MAX_CHILDREN {
            break;
        }
        let Ok(item) = item else { continue };
        let file_name = item.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        if validate_entry_name(name_str).is_err() {
            continue;
        }
        let Ok(smeta) = fs::symlink_metadata(item.path()) else {
            continue;
        };
        if smeta.file_type().is_symlink() {
            continue;
        }
        let is_dir = smeta.is_dir();
        if !is_dir && !smeta.is_file() {
            continue;
        }
        let child_rel = if clean_rel.is_empty() {
            name_str.to_owned()
        } else {
            format!("{clean_rel}/{name_str}")
        };
        let modified_secs = smeta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let size_bytes = if is_dir {
            fs::read_dir(item.path())
                .map(|rd| rd.count() as u64)
                .unwrap_or(0)
        } else {
            smeta.len()
        };
        seen_names.insert(name_str.to_owned());
        entries.push(FmEntry {
            name: name_str.to_owned(),
            rel_path: child_rel,
            is_dir,
            size_bytes,
            modified_secs,
            kind_label: classify_entry_kind(&item.path(), is_dir),
        });
    }
    if clean_rel.is_empty() && !seen_names.contains("performance") {
        let perf_dir = home.join("performance");
        if let Ok(smeta) = fs::symlink_metadata(&perf_dir)
            && smeta.is_dir()
            && !smeta.file_type().is_symlink()
        {
            let count = fs::read_dir(&perf_dir)
                .map(|r| r.count() as u64)
                .unwrap_or(0);
            let modified_secs = smeta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push(FmEntry {
                name: "performance".into(),
                rel_path: "performance".into(),
                is_dir: true,
                size_bytes: count,
                modified_secs,
                kind_label: "Folder".into(),
            });
        }
    }
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });
    Ok(entries)
}

pub fn fm_mkdir(
    home: &Path,
    profile_root: &Path,
    parent_rel: &str,
    folder_name: &str,
) -> Result<String, String> {
    validate_entry_name(folder_name)?;
    let clean_parent = parent_rel.trim().trim_matches('/');
    let new_rel = if clean_parent.is_empty() {
        folder_name.trim().to_owned()
    } else {
        format!("{clean_parent}/{}", folder_name.trim())
    };
    let target = resolve_fm_path(home, profile_root, &new_rel)?;
    if target.exists() {
        return Err(format!(
            "'{}' already exists in this folder.",
            folder_name.trim()
        ));
    }
    fs::create_dir(&target).map_err(|e| e.to_string())?;
    Ok(new_rel)
}

pub fn fm_rename(
    home: &Path,
    profile_root: &Path,
    target_rel: &str,
    new_name: &str,
) -> Result<String, String> {
    let clean_target = target_rel.trim().trim_matches('/');
    if is_protected_root_folder(clean_target) {
        return Err("Built-in profile root folders cannot be renamed.".into());
    }
    validate_entry_name(new_name)?;
    let src_path = resolve_fm_path(home, profile_root, clean_target)?;
    if !src_path.exists() {
        return Err("The selected item no longer exists.".into());
    }
    let parent_rel = clean_target.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    let dst_rel = if parent_rel.is_empty() {
        new_name.trim().to_owned()
    } else {
        format!("{parent_rel}/{}", new_name.trim())
    };
    let dst_path = resolve_fm_path(home, profile_root, &dst_rel)?;
    if dst_path == src_path {
        return Ok(dst_rel);
    }
    if dst_path.exists() {
        return Err(format!(
            "An item named '{}' already exists.",
            new_name.trim()
        ));
    }
    fs::rename(&src_path, &dst_path).map_err(|e| e.to_string())?;
    Ok(dst_rel)
}

fn verify_tree_no_symlinks(dir: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("Refusing to operate on symbolic links.".into());
    }
    if meta.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let sm = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if sm.file_type().is_symlink() {
                return Err("Directory contains a symbolic link.".into());
            }
            if sm.is_dir() {
                verify_tree_no_symlinks(&path)?;
            }
        }
    }
    Ok(())
}

pub fn fm_delete(
    home: &Path,
    profile_root: &Path,
    target_rels: &[String],
) -> Result<usize, String> {
    if target_rels.is_empty() {
        return Err("No items selected to delete.".into());
    }
    let mut deleted = 0usize;
    for rel in target_rels {
        let clean = rel.trim().trim_matches('/');
        if is_protected_root_folder(clean) {
            return Err(format!("Cannot delete protected folder '{clean}'."));
        }
        let path = resolve_fm_path(home, profile_root, clean)?;
        if let Some(fname) = path.file_name().and_then(|s| s.to_str())
            && is_protected_system_name(fname)
        {
            return Err(format!("Cannot delete protected launcher file '{fname}'."));
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        if meta.file_type().is_symlink() {
            return Err("Refusing to delete a symbolic link.".into());
        }
        if meta.is_dir() {
            verify_tree_no_symlinks(&path)?;
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        deleted += 1;
    }
    Ok(deleted)
}

pub fn unique_destination_path(dest_dir: &Path, original_name: &str) -> Result<PathBuf, String> {
    let direct = dest_dir.join(original_name);
    if !direct.exists() {
        return Ok(direct);
    }
    let p = Path::new(original_name);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_name);
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    for i in 1..=10_000 {
        let candidate_name = if ext.is_empty() {
            format!("{stem} ({i})")
        } else {
            format!("{stem} ({i}).{ext}")
        };
        let candidate = dest_dir.join(candidate_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Too many duplicate filenames in destination folder.".into())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    verify_tree_no_symlinks(src)?;
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let child_src = entry.path();
        let child_name = entry.file_name();
        let child_dst = dst.join(child_name);
        let meta = fs::symlink_metadata(&child_src).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("Refusing to copy symbolic link.".into());
        }
        if meta.is_dir() {
            copy_dir_recursive(&child_src, &child_dst)?;
        } else if meta.is_file() {
            fs::copy(&child_src, &child_dst).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn fm_paste(
    home: &Path,
    profile_root: &Path,
    sources: &[PathBuf],
    is_cut: bool,
    dest_rel_dir: &str,
) -> Result<usize, String> {
    if sources.is_empty() {
        return Err("Clipboard is empty.".into());
    }
    let dest_dir = resolve_fm_path(home, profile_root, dest_rel_dir)?;
    let dest_meta = fs::symlink_metadata(&dest_dir).map_err(|e| e.to_string())?;
    if !dest_meta.is_dir() || dest_meta.file_type().is_symlink() {
        return Err("Destination is not a valid folder.".into());
    }
    let mut count = 0usize;
    for src in sources {
        verify_no_symlinks(home, src)?;
        let src_meta = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
        if src_meta.file_type().is_symlink() {
            return Err("Refusing to copy or move symbolic links.".into());
        }
        let Some(fname) = src.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        validate_entry_name(fname)?;
        if src_meta.is_dir() && (dest_dir == *src || dest_dir.starts_with(src)) {
            return Err(format!(
                "Cannot {} folder '{fname}' into itself or its own subfolder.",
                if is_cut { "move" } else { "copy" }
            ));
        }
        if is_cut && src.parent() == Some(dest_dir.as_path()) {
            count += 1;
            continue;
        }
        let target = unique_destination_path(&dest_dir, fname)?;
        if is_cut {
            if fs::rename(src, &target).is_err() {
                if src_meta.is_dir() {
                    copy_dir_recursive(src, &target)?;
                    fs::remove_dir_all(src).map_err(|e| e.to_string())?;
                } else {
                    fs::copy(src, &target).map_err(|e| e.to_string())?;
                    fs::remove_file(src).map_err(|e| e.to_string())?;
                }
            }
        } else if src_meta.is_dir() {
            copy_dir_recursive(src, &target)?;
        } else {
            fs::copy(src, &target).map_err(|e| e.to_string())?;
        }
        count += 1;
    }
    Ok(count)
}

pub fn fm_details(
    home: &Path,
    profile_root: &Path,
    target_rel: &str,
) -> Result<FmFileDetails, String> {
    let clean = target_rel.trim().trim_matches('/');
    let path = resolve_fm_path(home, profile_root, clean)?;
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("Refusing to inspect symbolic link.".into());
    }
    let is_dir = meta.is_dir();
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(if clean.is_empty() {
            "Profile Root"
        } else {
            clean
        })
        .to_owned();
    let modified_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let modified_label = format_timestamp_utc(modified_secs);
    let kind_label = classify_entry_kind(&path, is_dir);
    let mut size_bytes = meta.len();
    let mut preview_lines = Vec::new();
    if is_dir {
        let mut files_count = 0u64;
        let mut dirs_count = 0u64;
        let mut total_bytes = 0u64;
        if let Ok(rd) = fs::read_dir(&path) {
            for entry in rd.flatten() {
                if let Ok(m) = entry.metadata() {
                    if m.is_dir() {
                        dirs_count += 1;
                    } else if m.is_file() {
                        files_count += 1;
                        total_bytes = total_bytes.saturating_add(m.len());
                    }
                }
            }
        }
        size_bytes = total_bytes;
        preview_lines.push(format!(
            "Contains {dirs_count} folder(s) and {files_count} file(s)."
        ));
    } else {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        if matches!(
            ext.as_deref(),
            Some(
                "txt"
                    | "toml"
                    | "json"
                    | "jsonl"
                    | "cfg"
                    | "properties"
                    | "wgsl"
                    | "glsl"
                    | "fsh"
                    | "vsh"
                    | "md"
                    | "log"
            )
        ) {
            if let Ok(file) = File::open(&path) {
                use std::io::Read;
                let mut buf = Vec::new();
                if file.take(4096).read_to_end(&mut buf).is_ok() {
                    let text = String::from_utf8_lossy(&buf);
                    for line in text.lines().take(16) {
                        preview_lines.push(line.chars().take(120).collect());
                    }
                    if preview_lines.is_empty() {
                        preview_lines.push("(Empty text file)".into());
                    }
                }
            }
        } else {
            preview_lines.push(
                "Binary file (text preview is enabled for .txt, .toml, and .json files).".into(),
            );
        }
    }
    Ok(FmFileDetails {
        name,
        rel_path: if clean.is_empty() {
            "/".into()
        } else {
            clean.to_owned()
        },
        is_dir,
        size_bytes,
        modified_label,
        kind_label,
        preview_lines,
    })
}

#[cfg(test)]
mod fm_tests {
    use super::*;

    #[test]
    fn file_manager_operations_and_security_guards() {
        let home = std::env::temp_dir().join(format!(
            "horizon-fm-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&home).unwrap();
        ensure_fm_dirs(&home, &home);

        // Protected system files must be rejected
        assert!(validate_entry_name("..").is_err());
        assert!(validate_entry_name("a/b").is_err());
        assert!(validate_entry_name("a\\b").is_err());
        assert!(validate_entry_name(".local").is_err());
        assert!(validate_entry_name("android-launcher.json").is_err());
        assert!(validate_entry_name("engine-data.zip").is_err());

        // Create folder & rename
        let created = fm_mkdir(&home, &home, "saves", "TestWorld").unwrap();
        assert_eq!(created, "saves/TestWorld");
        let renamed = fm_rename(&home, &home, "saves/TestWorld", "RenamedWorld").unwrap();
        assert_eq!(renamed, "saves/RenamedWorld");

        // Write a config/text file and verify details preview
        fs::write(
            home.join("saves/RenamedWorld/notes.txt"),
            "line1
line2
",
        )
        .unwrap();
        let details = fm_details(&home, &home, "saves/RenamedWorld/notes.txt").unwrap();
        assert_eq!(details.size_bytes, 12);
        assert!(details.preview_lines.iter().any(|l| l == "line1"));

        // Copy & Paste with duplicate filename handling (`notes (1).txt`)
        let src_file = home.join("saves/RenamedWorld/notes.txt");
        fm_paste(&home, &home, &[src_file], false, "saves/RenamedWorld").unwrap();
        assert!(home.join("saves/RenamedWorld/notes (1).txt").exists());

        // Prevent copying/moving a folder into itself or its own descendant
        let world_dir = home.join("saves/RenamedWorld");
        let sub_rel = fm_mkdir(&home, &home, "saves/RenamedWorld", "region").unwrap();
        assert!(fm_paste(&home, &home, &[world_dir], false, &sub_rel).is_err());

        // Delete file and folder
        let del_count = fm_delete(
            &home,
            &home,
            &["saves/RenamedWorld/notes (1).txt".to_owned()],
        )
        .unwrap();
        assert_eq!(del_count, 1);
        assert!(!home.join("saves/RenamedWorld/notes (1).txt").exists());

        // Root folders cannot be deleted
        assert!(fm_delete(&home, &home, &["saves".to_owned()]).is_err());

        fs::remove_dir_all(home).unwrap();
    }
}
