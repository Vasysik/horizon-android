//! Where Starling's releases come out, and what of a release is Starling's: for the update prompt
//! (game/update_offer.rs) and the native mod builds (game/native_builds.rs), on Android.
//!
//! Starling and Horizon Android are one project now, in the organisation's repository
//! (starling-client/starling); Starling's own (JuanLuci/starling) is where its releases came out
//! before, and still do until the merged one. Both are asked, the organisation's first, so an
//! install follows the releases wherever they come out. The organisation's repository also holds
//! Horizon Android's releases (its APKs are named after it) and preview builds of Starling
//! (`starling-preview-...`): only an APK named `starling-<version>-...` is an update.

/// The repositories, as GitHub's API names them, the organisation's first.
pub const REPOSITORIES: [&str; 2] = ["starling-client/starling", "JuanLuci/starling"];

/// A repository's latest release, as GitHub's API serves it.
pub fn latest_url(repository: &str) -> String {
    format!("https://api.github.com/repos/{repository}/releases/latest")
}

/// A repository's release of native mod builds (native-mods.yml publishes to it).
pub fn native_builds_url(repository: &str) -> String {
    format!("https://api.github.com/repos/{repository}/releases/tags/native-builds")
}

/// Whether version `latest` (a tag: `v0.1.2`) comes after `current` (`0.1.1`), number by number.
pub fn newer(latest: &str, current: &str) -> bool {
    let numbers = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    numbers(latest) > numbers(current)
}

/// A Starling release's APK.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    /// Without its `v`: `0.8.0`.
    pub version: String,
    /// The APK's file name, its download and its size.
    pub name: String,
    pub url: String,
    pub size: u64,
    /// The SHA-256 GitHub gives for it (hex), checked after the download.
    pub sha256: Option<String>,
}

/// The release `latest` (a `releases/latest` answer) as a Starling update, when it is newer than
/// `current` and has Starling's own APK for its version.
pub fn update_in(latest: &serde_json::Value, current: &str) -> Option<Update> {
    let tag = latest["tag_name"].as_str()?;
    if !newer(tag, current) {
        return None;
    }
    let version = tag.trim_start_matches('v');
    let prefix = format!("starling-{version}-");
    let apk = latest["assets"].as_array()?.iter().find(|a| {
        a["name"]
            .as_str()
            .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".apk"))
    })?;
    Some(Update {
        version: version.to_owned(),
        name: apk["name"].as_str()?.to_owned(),
        url: apk["browser_download_url"]
            .as_str()
            .filter(|u| u.starts_with("https://"))?
            .to_owned(),
        size: apk["size"].as_u64().unwrap_or(0),
        sha256: apk["digest"]
            .as_str()
            .and_then(|d| d.strip_prefix("sha256:"))
            .map(str::to_owned),
    })
}

/// The newest of the updates the repositories offer; the first of two of one version.
pub fn newest(updates: impl IntoIterator<Item = Update>) -> Option<Update> {
    updates
        .into_iter()
        .fold(None, |best: Option<Update>, u| match best {
            Some(b) if !newer(&u.version, &b.version) => Some(b),
            _ => Some(u),
        })
}

/// A build of version's game with native mods, in a native-builds release.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeBuild {
    /// `v0.1.4__wrench`: its assets' names, and its folder on the phone.
    pub name: String,
    pub mods: Vec<String>,
    /// Its JSON's and its library's downloads.
    pub json: String,
    pub library: String,
}

/// A name of the release's assets: lowercase letters, digits, `.`, `_` and `-`.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

/// The builds made for `version` in `release` (a native-builds release as the API serves it).
pub fn native_builds_in(release: &serde_json::Value, version: &str) -> Vec<NativeBuild> {
    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let url = |name: &str| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(name))
            .and_then(|a| a["browser_download_url"].as_str())
            .filter(|u| u.starts_with("https://"))
            .map(str::to_owned)
    };
    let prefix = format!("v{version}__");
    assets
        .iter()
        .filter_map(|a| a["name"].as_str())
        .filter_map(|n| n.strip_suffix(".json"))
        .filter(|n| n.starts_with(&prefix) && plain(n))
        .filter_map(|name| {
            Some(NativeBuild {
                name: name.to_owned(),
                mods: name[prefix.len()..]
                    .split("__")
                    .map(str::to_owned)
                    .collect(),
                json: url(&format!("{name}.json"))?,
                library: url(&format!("{name}.so"))?,
            })
        })
        .collect()
}

/// The repositories' builds as one list: a set of mods once (the first repository's), the fewest
/// mods first.
pub fn merge_native_builds(lists: impl IntoIterator<Item = Vec<NativeBuild>>) -> Vec<NativeBuild> {
    let mut builds: Vec<NativeBuild> = Vec::new();
    for build in lists.into_iter().flatten() {
        if !builds.iter().any(|b| b.name == build.name) {
            builds.push(build);
        }
    }
    builds.sort_by(|a, b| (a.mods.len(), &a.name).cmp(&(b.mods.len(), &b.name)));
    builds
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn asset(name: &str) -> serde_json::Value {
        json!({
            "name": name,
            "browser_download_url": format!("https://github.com/x/releases/download/t/{name}"),
            "size": 10,
            "digest": "sha256:ab",
        })
    }

    #[test]
    fn versions_compare_number_by_number() {
        assert!(newer("v0.8.0", "0.2.2"));
        assert!(newer("v0.2.10", "0.2.9"));
        assert!(!newer("v0.2.2", "0.2.2"));
        assert!(!newer("v0.1.9", "0.2.0"));
    }

    #[test]
    fn only_starlings_own_apk_is_an_update() {
        // Horizon Android's last release, the organisation's latest today: not Starling's.
        let horizon =
            json!({"tag_name": "v0.7.8", "assets": [asset("Horizon-Android-0.7.8-arm64-v8a.apk")]});
        assert_eq!(update_in(&horizon, "0.2.2"), None);
        // A preview build is another app, signed with another key.
        let preview = json!({"tag_name": "v0.8.0", "assets": [asset("starling-preview-0.8.0-arm64-v8a.apk")]});
        assert_eq!(update_in(&preview, "0.2.2"), None);
        let release = json!({"tag_name": "v0.8.0", "assets": [
            asset("starling-preview-0.8.0-arm64-v8a.apk"),
            asset("StarlingVisuals.zip"),
            asset("starling-0.8.0-arm64-v8a.apk"),
        ]});
        let update = update_in(&release, "0.2.2").unwrap();
        assert_eq!(update.version, "0.8.0");
        assert_eq!(update.name, "starling-0.8.0-arm64-v8a.apk");
        assert_eq!(update.sha256.as_deref(), Some("ab"));
        assert_eq!(update_in(&release, "0.8.0"), None, "not newer");
    }

    #[test]
    fn the_newest_repository_wins_and_a_tie_keeps_the_first() {
        let update = |version: &str, url: &str| Update {
            version: version.into(),
            name: format!("starling-{version}-arm64-v8a.apk"),
            url: url.into(),
            size: 1,
            sha256: None,
        };
        let org = update("0.8.0", "org");
        let own = update("0.2.3", "own");
        assert_eq!(newest([own.clone(), org.clone()]), Some(org.clone()));
        assert_eq!(
            newest([org.clone(), update("0.8.0", "own")]).unwrap().url,
            "org"
        );
        assert_eq!(newest([]), None);
    }

    #[test]
    fn native_builds_merge_by_name_the_first_repository_first() {
        let release = |base: &str| {
            json!({"assets": [
                asset("v0.2.2__wrench.json"), asset("v0.2.2__wrench.so"),
                asset("v0.2.2__wrench__carry.json"), asset("v0.2.2__wrench__carry.so"),
                asset("v0.2.1__old.json"), asset("v0.2.1__old.so"),
                asset(&format!("v0.2.2__{base}.json")),
            ]})
        };
        let org = native_builds_in(&release("lonely"), "0.2.2");
        assert_eq!(org.len(), 2, "a build needs its library, and this version");
        let own = native_builds_in(&release("other"), "0.2.2");
        let merged = merge_native_builds([org, own]);
        assert_eq!(
            merged.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
            ["v0.2.2__wrench", "v0.2.2__wrench__carry"]
        );
        assert_eq!(merged[1].mods, ["wrench", "carry"]);
    }
}
