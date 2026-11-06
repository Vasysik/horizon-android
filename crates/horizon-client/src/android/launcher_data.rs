//! What the launcher's pages list and change, on plain files: instances (named game folders over
//! the existing profiles), play time, published builds and the words pages use for sizes and
//! dates. Worlds, servers, screenshots and data packs are the desktop launcher's own modules.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const FILE: &str = "android-instances.json";
/// What an instance keeps, as the desktop's copy and export do: never sign-in sessions, logs or
/// the launcher's own files, which share the first instance's folder.
const COPIED_FILES: [&str; 5] = [
    "options.txt",
    "servers.dat",
    "horizon.txt",
    "horizon-graphics.txt",
    "horizon-keypad.json",
];
const COPIED_FOLDERS: [&str; 6] = [
    "saves",
    "resourcepacks",
    "shaderpacks",
    "horizon-mods",
    "config",
    "screenshots",
];

/// Removes a file or a folder. Android has no trash the app can reach.
pub fn remove(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| format!("Could not remove {}: {e}", path.display()))
}

/// The two helpers `launcher/src/datapacks.rs` takes from the desktop's `packs.rs`, whose other
/// parts need crates the app doesn't have.
pub mod packs {
    use std::{fs, io::Read, path::Path};

    /// A file at the root of a pack folder or zip, at most 4 MB.
    pub(crate) fn entry(path: &Path, name: &str) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        if path.is_dir() {
            fs::File::open(path.join(name))
                .ok()?
                .take(4 << 20)
                .read_to_end(&mut out)
                .ok()?;
        } else {
            let mut zip = zip::ZipArchive::new(fs::File::open(path).ok()?).ok()?;
            let entry = zip.by_name(name).ok()?;
            entry.take(4 << 20).read_to_end(&mut out).ok()?;
        }
        Some(out)
    }

    /// A text component as plain text: its strings, without `§` formatting codes.
    pub(crate) fn plain(component: &serde_json::Value) -> String {
        fn walk(v: &serde_json::Value, out: &mut String) {
            match v {
                serde_json::Value::String(s) => out.push_str(s),
                serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, out)),
                serde_json::Value::Object(o) => {
                    if let Some(t) = o.get("text").or_else(|| o.get("translate")) {
                        walk(t, out);
                    }
                    if let Some(extra) = o.get("extra") {
                        walk(extra, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = String::new();
        walk(component, &mut out);
        let mut plain = String::new();
        let mut chars = out.chars();
        while let Some(c) = chars.next() {
            if c == '§' {
                chars.next();
            } else {
                plain.push(c);
            }
        }
        plain
    }
}

/// An instance's own name, colour and record. The folder is `profile_home` of its id.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    pub name: String,
    pub hue: usize,
    pub played_secs: u64,
    pub last_played: u64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Instances {
    pub list: BTreeMap<u32, Meta>,
}

/// The game folder of instance `id`: the first keeps the app's own folder, as it always has.
pub fn instance_home(home: &Path, id: u32) -> PathBuf {
    if id == 0 {
        home.into()
    } else {
        home.join("profiles").join(format!("profile-{id}"))
    }
}

impl Instances {
    /// The list, with the first instance and any older profile folder in it.
    pub fn load(home: &Path) -> Instances {
        let mut me: Instances = fs::read(home.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        me.list.entry(0).or_insert_with(|| Meta {
            name: "Horizon".into(),
            ..Meta::default()
        });
        for entry in fs::read_dir(home.join("profiles"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let id = entry
                .file_name()
                .to_str()
                .and_then(|n| n.strip_prefix("profile-")?.parse::<u32>().ok());
            if let Some(id) = id.filter(|_| entry.path().is_dir()) {
                me.list.entry(id).or_insert_with(|| Meta {
                    name: format!("Profile {id}"),
                    hue: id as usize % 7,
                    ..Meta::default()
                });
            }
        }
        me.list
            .retain(|id, _| *id == 0 || instance_home(home, *id).is_dir());
        me
    }

    pub fn save(&self, home: &Path) -> Result<(), String> {
        let part = home.join("android-instances.partial");
        fs::write(
            &part,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(part, home.join(FILE)).map_err(|e| e.to_string())
    }

    /// A new, empty instance, or a copy of `from` without its sessions and logs.
    pub fn create(
        &mut self,
        home: &Path,
        name: &str,
        hue: usize,
        from: Option<u32>,
    ) -> Result<u32, String> {
        let name = clean_name(name)?;
        let id = self.list.keys().max().map_or(1, |m| m + 1);
        let folder = instance_home(home, id);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        if let Some(from) = from {
            let source = instance_home(home, from);
            let copied = (|| -> std::io::Result<()> {
                for file in COPIED_FILES {
                    if source.join(file).is_file() {
                        fs::copy(source.join(file), folder.join(file))?;
                    }
                }
                for dir in COPIED_FOLDERS {
                    copy_tree(&source.join(dir), &folder.join(dir))?;
                }
                Ok(())
            })();
            if let Err(error) = copied {
                let _ = fs::remove_dir_all(&folder);
                return Err(format!("Could not copy the instance: {error}"));
            }
        }
        self.list.insert(
            id,
            Meta {
                name,
                hue,
                ..Meta::default()
            },
        );
        self.save(home)?;
        Ok(id)
    }

    pub fn rename(&mut self, home: &Path, id: u32, name: &str, hue: usize) -> Result<(), String> {
        let name = clean_name(name)?;
        let meta = self.list.get_mut(&id).ok_or("That instance is gone")?;
        meta.name = name;
        meta.hue = hue;
        self.save(home)
    }

    /// Deletes an instance's folder. The first one holds the app's own files too, so it stays.
    pub fn delete(&mut self, home: &Path, id: u32) -> Result<(), String> {
        if id == 0 {
            return Err("The first instance stays: it shares the app's own folder.".into());
        }
        remove(&instance_home(home, id))?;
        self.list.remove(&id);
        self.save(home)
    }

    /// Adds a finished run's time.
    pub fn record_play(&mut self, home: &Path, id: u32, played: Duration) -> Result<(), String> {
        let meta = self.list.entry(id).or_default();
        meta.played_secs += played.as_secs();
        meta.last_played = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.save(home)
    }
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("An instance needs a name.".into());
    }
    Ok(name.chars().take(40).collect())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        // A running game's lock is no part of the copy, and links could lead outside.
        if kind.is_symlink() || entry.file_name() == "session.lock" {
            continue;
        }
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// "Played 4 h 20 min", as the desktop's Instance row says it.
pub fn played(secs: u64) -> String {
    match secs {
        0 => "Never played".into(),
        s if s < 60 => "Played under a minute".into(),
        s if s < 3600 => format!("Played {} min", s / 60),
        s if s % 3600 < 60 => format!("Played {} h", s / 3600),
        s => format!("Played {} h {} min", s / 3600, s % 3600 / 60),
    }
}

/// "today", "2 days ago"…
pub fn age(when: SystemTime) -> String {
    let days = SystemTime::now()
        .duration_since(when)
        .map_or(0, |d| d.as_secs() / 86_400);
    match days {
        0 => "today".into(),
        1 => "yesterday".into(),
        d if d < 60 => format!("{d} days ago"),
        d => format!("{} months ago", d / 30),
    }
}

pub fn size(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    if bytes >= 1_000 * MB {
        format!("{:.1} GB", bytes as f64 / (1_000 * MB) as f64)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else {
        format!("{} KB", (bytes / 1_000).max(1))
    }
}

/// The game's arguments, one line where quotes group a word with spaces (the desktop's
/// `game_args`).
pub fn split_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let (mut quote, mut started) = (None, false);
    for c in line.chars() {
        match (quote, c) {
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (Some(q), _) if q == c => quote = None,
            (None, c) if c.is_whitespace() => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(word);
    }
    out
}

/// The last lines the app logged, for Home's Console. Kept in memory: a report holds the rest.
static LOG: std::sync::Mutex<std::collections::VecDeque<String>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
const LOG_LINES: usize = 400;

pub fn remember_log(level: log::Level, target: &str, message: &str) {
    if let Ok(mut log) = LOG.try_lock() {
        if log.len() == LOG_LINES {
            log.pop_front();
        }
        let line: String = message.chars().take(240).collect();
        log.push_back(format!("[{level}] {target}: {line}"));
    }
}
pub fn log_lines() -> Vec<String> {
    LOG.lock()
        .map(|log| log.iter().cloned().collect())
        .unwrap_or_default()
}

/// A published build: its release on GitHub.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub published_at: String,
    pub body: Option<String>,
    pub html_url: String,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

/// This app's releases, the Android fork's: the launcher's own builds come from there.
pub const RELEASES: &str =
    "https://api.github.com/repos/Vasysik/horizon-android/releases?per_page=12";
const RELEASES_CACHE: &str = "android-releases.json";

impl Release {
    /// The APK of `abi`, else the release's page.
    pub fn download(&self, abi: &str) -> &str {
        self.asset(abi)
            .map_or(&self.html_url, |a| &a.browser_download_url)
    }
    pub fn asset(&self, abi: &str) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|a| a.name.ends_with(".apk") && a.name.contains(abi))
    }
    pub fn date(&self) -> &str {
        self.published_at.get(..10).unwrap_or(&self.published_at)
    }
    /// Its Markdown as text: headings, list marks and emphasis taken out.
    pub fn notes(&self) -> String {
        plain_markdown(self.body.as_deref().unwrap_or(""))
    }
}

/// Markdown read aloud: what a line says, without its marks.
pub fn plain_markdown(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("<!--") || trimmed == "---" {
            continue;
        }
        let bullet = trimmed.starts_with("- ") || trimmed.starts_with("* ");
        let body = trimmed
            .trim_start_matches('#')
            .trim_start_matches("- ")
            .trim_start_matches("* ")
            .trim();
        let mut text = String::new();
        let mut rest = body;
        // [label](address) keeps its label.
        while let Some(open) = rest.find('[') {
            let Some(close) = rest[open..].find("](") else {
                break;
            };
            let Some(end) = rest[open + close..].find(')') else {
                break;
            };
            text.push_str(&rest[..open]);
            text.push_str(&rest[open + 1..open + close]);
            rest = &rest[open + close + end + 1..];
        }
        text.push_str(rest);
        let text = text.replace("**", "").replace('`', "").replace("__", "");
        if text.is_empty() {
            if !out.ends_with("\n\n") && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        if bullet {
            out.push_str("• ");
        }
        out.push_str(&text);
        out.push('\n');
    }
    out.trim().to_owned()
}

/// Reads the releases list: the network, else what was read last time.
pub fn fetch_releases(home: &Path) -> Result<Vec<Release>, String> {
    let cached = || -> Option<Vec<Release>> {
        serde_json::from_slice(&fs::read(home.join(RELEASES_CACHE)).ok()?).ok()
    };
    let read = || -> Result<Vec<u8>, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        let agent = ureq::AgentBuilder::new()
            .https_only(true)
            .timeout_connect(Duration::from_secs(15))
            .timeout_read(Duration::from_secs(30));
        #[cfg(target_os = "android")]
        let agent = crate::proxy::agent(agent, RELEASES);
        agent
            .build()
            .get(RELEASES)
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", "horizon-android-launcher")
            .call()
            .map_err(|e| e.to_string())?
            .into_reader()
            .take(8 << 20)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    };
    match read().and_then(|bytes| {
        let list: Vec<Release> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let _ = fs::write(home.join(RELEASES_CACHE), &bytes);
        Ok(list)
    }) {
        Ok(list) => Ok(list),
        Err(error) => cached().ok_or(error),
    }
}

/// Decoded pictures for the pages, read off the UI thread.
pub struct Pictures {
    ask: mpsc::Sender<(u64, PathBuf, u32)>,
    done: mpsc::Receiver<(u64, Option<crate::launcher_glass::Pixels>)>,
    known: HashMap<u64, Option<crate::launcher_glass::Pixels>>,
    pending: std::collections::HashSet<u64>,
}
impl Default for Pictures {
    fn default() -> Self {
        let (ask, jobs) = mpsc::channel::<(u64, PathBuf, u32)>();
        let (send, done) = mpsc::channel();
        std::thread::spawn(move || {
            for (key, path, max) in jobs {
                let image = fs::read(&path)
                    .ok()
                    .and_then(|bytes| crate::launcher_glass::decode_png(&bytes, max))
                    .map(std::sync::Arc::new);
                if send.send((key, image)).is_err() {
                    break;
                }
            }
        });
        Self {
            ask,
            done,
            known: HashMap::new(),
            pending: Default::default(),
        }
    }
}
impl Pictures {
    /// The picture at `path` no longer than `max`, once it's read: the key names the file's
    /// bytes, so a changed file is read again.
    pub fn get(&mut self, path: &Path, max: u32) -> Option<(u64, crate::launcher_glass::Pixels)> {
        let key = key_of(path, max);
        if let Some(found) = self.known.get(&key) {
            return found.clone().map(|p| (key, p));
        }
        if self.pending.insert(key) {
            let _ = self.ask.send((key, path.to_owned(), max));
        }
        None
    }
    /// Takes what was read; true if anything new came.
    pub fn poll(&mut self) -> bool {
        let mut any = false;
        while let Ok((key, image)) = self.done.try_recv() {
            self.pending.remove(&key);
            self.known.insert(key, image);
            any = true;
        }
        // A long session's thumbnails shouldn't pile up.
        if self.known.len() > 96 {
            self.known.clear();
        }
        any
    }
}

fn key_of(path: &Path, max: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    max.hash(&mut h);
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .hash(&mut h);
    h.finish() >> 24
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-data-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn instances_copy_only_what_an_instance_keeps() {
        let home = temp("copy");
        fs::create_dir_all(home.join("saves/World")).unwrap();
        fs::write(home.join("saves/World/level.dat"), b"x").unwrap();
        fs::write(home.join("saves/World/session.lock"), b"x").unwrap();
        fs::write(home.join("options.txt"), b"fov:1").unwrap();
        fs::create_dir_all(home.join("sessions")).unwrap();
        fs::write(home.join("sessions/secret.json"), b"token").unwrap();
        fs::write(home.join("android-launcher.json"), b"{}").unwrap();
        let mut list = Instances::load(&home);
        assert_eq!(list.list[&0].name, "Horizon");
        let id = list.create(&home, " Copy ", 2, Some(0)).unwrap();
        let folder = instance_home(&home, id);
        assert!(folder.join("saves/World/level.dat").is_file());
        assert!(!folder.join("saves/World/session.lock").exists());
        assert!(folder.join("options.txt").is_file());
        assert!(!folder.join("sessions").exists());
        assert!(!folder.join("android-launcher.json").exists());
        let list = Instances::load(&home);
        assert_eq!(list.list[&id].name, "Copy");
        assert_eq!(list.list[&id].hue, 2);
    }

    #[test]
    fn an_instance_can_be_renamed_timed_and_deleted_but_not_the_first() {
        let home = temp("edit");
        let mut list = Instances::load(&home);
        let id = list.create(&home, "A", 0, None).unwrap();
        list.rename(&home, id, "B", 3).unwrap();
        list.record_play(&home, id, Duration::from_secs(4 * 3600 + 20 * 60))
            .unwrap();
        let mut list = Instances::load(&home);
        assert_eq!(played(list.list[&id].played_secs), "Played 4 h 20 min");
        assert!(list.list[&id].last_played > 0);
        assert!(list.create(&home, "   ", 0, None).is_err());
        assert!(list.delete(&home, 0).is_err());
        list.delete(&home, id).unwrap();
        assert!(!instance_home(&home, id).exists());
        assert!(!Instances::load(&home).list.contains_key(&id));
    }

    #[test]
    fn game_arguments_keep_quoted_words() {
        assert_eq!(
            split_args("--server-thread  --world 'My World' --name \"Two Words\""),
            [
                "--server-thread",
                "--world",
                "My World",
                "--name",
                "Two Words"
            ]
        );
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn older_profiles_are_listed_as_instances() {
        let home = temp("old");
        fs::create_dir_all(home.join("profiles/profile-4")).unwrap();
        let list = Instances::load(&home);
        assert_eq!(list.list[&4].name, "Profile 4");
    }

    #[test]
    fn release_notes_read_as_text() {
        assert_eq!(
            plain_markdown(
                "## What's new\n\n- **Fix** `this`\n- see [the docs](https://x.y/z) now\n```\ncode\n```"
            ),
            "What's new\n\n• Fix this\n• see the docs now\ncode"
        );
    }

    #[test]
    fn a_release_offers_this_abi_s_apk() {
        let release = Release {
            html_url: "https://page".into(),
            assets: vec![Asset {
                name: "Horizon-Android-0.7.7-arm64-v8a.apk".into(),
                size: 1,
                browser_download_url: "https://apk".into(),
            }],
            ..Release::default()
        };
        assert_eq!(release.download("arm64-v8a"), "https://apk");
        assert_eq!(release.download("x86_64"), "https://page");
    }

    #[test]
    fn worlds_servers_and_shots_work_on_a_phone_s_folder() {
        use crate::{datapacks, screenshots, server_list, worlds};
        let home = temp("worlds");
        // A world with a level.dat the desktop module reads.
        let world = home.join("saves/Island");
        fs::create_dir_all(&world).unwrap();
        fs::write(world.join("region.bin"), [0u8; 1000]).unwrap();
        let list = worlds::list(&home);
        assert_eq!(list.len(), 0, "a folder without level.dat is no world");
        fs::write(world.join("level.dat"), []).unwrap();
        let list = worlds::list(&home);
        assert_eq!(list.len(), 1);
        let backup = worlds::backup(&home, &list[0]).unwrap();
        assert!(backup.is_file());
        assert!(backup.starts_with(home.join("backups")));
        let copy = worlds::copy(&list[0]).unwrap();
        assert!(copy.path.join("region.bin").is_file());
        worlds::delete(&copy).unwrap();
        assert!(!copy.path.exists());

        fs::create_dir_all(world.join("datapacks")).unwrap();
        fs::write(world.join("datapacks/pack.zip"), []).unwrap();
        let packs = datapacks::list(&world);
        assert!(packs.len() <= 1);

        let mut servers = server_list::List::load(&home);
        servers.add("Friends", "play.example.net").unwrap();
        servers.save(&home).unwrap();
        let mut servers = server_list::List::load(&home);
        assert_eq!(servers.servers()[0].ip, "play.example.net");
        servers.remove(0).unwrap();
        servers.save(&home).unwrap();
        assert!(server_list::List::load(&home).servers().is_empty());

        fs::create_dir_all(home.join("screenshots")).unwrap();
        fs::write(home.join("screenshots/a.png"), [1, 2]).unwrap();
        let shots = screenshots::list(&home);
        assert_eq!(shots.len(), 1);
        screenshots::delete(&shots[0]).unwrap();
        assert!(screenshots::list(&home).is_empty());
    }
}
