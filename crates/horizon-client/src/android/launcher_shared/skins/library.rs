//! Frontend-independent, versioned local wardrobe. Selection is explicitly local:
//! this module never calls an account service or changes a remote profile.
use super::{Model, read_cape, read_file, read_skin};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Skin,
    Cape,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub kind: Kind,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    pub skin: Option<String>,
    pub cape: Option<String>,
    pub model: Model,
    /// Hide even the account's equipped cape, locally only.
    #[serde(default)]
    pub hide_cape: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub schema: u32,
    pub entries: Vec<Entry>,
    pub selections: std::collections::BTreeMap<String, Selection>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            schema: 1,
            entries: vec![],
            selections: Default::default(),
        }
    }
}
impl Library {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("wardrobe.json");
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(_) => return Err("Could not read the local wardrobe.".into()),
        };
        if data.len() > 2 * 1024 * 1024 {
            return Err("The wardrobe index is too large.".into());
        }
        let mut value: Self =
            serde_json::from_slice(&data).map_err(|_| "The wardrobe index is damaged.")?;
        if value.schema != 1 {
            return Err("This wardrobe needs a newer launcher.".into());
        }
        if value.entries.iter().any(|e| !valid_id(&e.id)) {
            return Err("The wardrobe contains an invalid image identifier.".into());
        }
        let mut removed = Vec::new();
        value.entries.retain(|entry| {
            let titles = [
                "Violet stripes",
                "Tidal weave",
                "Copper trail",
                "Midnight grid",
            ];
            let index = titles.iter().position(|title| *title == entry.title);
            if let Some(index) = index.filter(|_| entry.kind == Kind::Cape) {
                let mut png = std::io::Cursor::new(Vec::new());
                if cape_design(index)
                    .write_to(&mut png, image::ImageFormat::Png)
                    .is_ok()
                    && entry.id == format!("{:x}", Sha256::digest(png.get_ref()))
                {
                    removed.push(entry.id.clone());
                    return false;
                }
            }
            true
        });
        for selection in value.selections.values_mut() {
            if selection
                .cape
                .as_ref()
                .is_some_and(|id| removed.contains(id))
            {
                selection.cape = None;
            }
        }
        Ok(value)
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        std::fs::create_dir_all(root).map_err(|_| "Could not create the wardrobe folder.")?;
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|_| "Could not encode the wardrobe.")?;
        std::fs::write(root.join("wardrobe.json.tmp"), bytes)
            .map_err(|_| "Could not save the wardrobe.")?;
        std::fs::rename(root.join("wardrobe.json.tmp"), root.join("wardrobe.json"))
            .map_err(|_| "Could not replace the wardrobe index.".into())
    }
    pub fn image(root: &Path, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) {
            return Err("Invalid wardrobe image identifier.".into());
        }
        Ok(root.join("images").join(format!("{id}.png")))
    }
    pub fn import(
        &mut self,
        root: &Path,
        title: &str,
        kind: Kind,
        bytes: &[u8],
    ) -> Result<String, String> {
        let image = match kind {
            Kind::Skin => read_skin(bytes)?,
            Kind::Cape => read_cape(bytes)?,
        };
        // Decode and re-encode so local metadata, trailing data and file names never persist.
        let mut output = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut output, image::ImageFormat::Png)
            .map_err(|_| "Could not encode PNG.")?;
        let png = output.into_inner();
        let id = format!("{:x}", Sha256::digest(&png));
        let path = Self::image(root, &id)?;
        std::fs::create_dir_all(path.parent().unwrap())
            .map_err(|_| "Could not create image storage.")?;
        std::fs::write(path, png).map_err(|_| "Could not keep this image.")?;
        if !self.entries.iter().any(|e| e.id == id && e.kind == kind) {
            self.entries.push(Entry {
                id: id.clone(),
                title: title.chars().filter(|c| !c.is_control()).take(48).collect(),
                kind,
            });
        }
        self.save(root)?;
        Ok(id)
    }
    pub fn import_file(&mut self, root: &Path, path: &Path, kind: Kind) -> Result<String, String> {
        self.import(
            root,
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Imported PNG"),
            kind,
            &read_file(path)?,
        )
    }
    pub fn select(
        &mut self,
        root: &Path,
        account_key: &str,
        id: &str,
        kind: Kind,
    ) -> Result<(), String> {
        if !self.entries.iter().any(|e| e.id == id && e.kind == kind) {
            return Err("Choose an image from your gallery.".into());
        }
        let selection = self.selections.entry(account_key.into()).or_default();
        match kind {
            Kind::Skin => selection.skin = Some(id.into()),
            Kind::Cape => {
                selection.cape = Some(id.into());
                selection.hide_cape = false;
            }
        }
        self.save(root)
    }
    /// The core reads this public JSON contract from the game directory on each
    /// launch. Absolute PNG paths; no credentials, launcher widgets or env changes.
    pub fn prepare_game(
        &self,
        root: &Path,
        account_key: &str,
        game_home: &Path,
        cloud: Option<(&Path, bool, Option<&Path>)>,
    ) -> Result<(), String> {
        let selection = self
            .selections
            .get(account_key)
            .cloned()
            .unwrap_or_default();
        let local = |id: &Option<String>| id.as_ref().map(|id| Self::image(root, id)).transpose();
        let skin = local(&selection.skin)?.or_else(|| cloud.map(|(p, _, _)| p.to_owned()));
        let cape = if selection.hide_cape {
            None
        } else {
            local(&selection.cape)?.or_else(|| cloud.and_then(|(_, _, p)| p.map(Path::to_owned)))
        };
        let slim = if selection.skin.is_some() {
            selection.model == Model::Slim
        } else {
            cloud.is_some_and(|(_, s, _)| s)
        };
        std::fs::create_dir_all(game_home).map_err(|_| "Could not create the game folder.")?;
        let descriptor = serde_json::json!({"schema":1,"skin":skin,"cape":cape,"slim":slim,"hide_cape":selection.hide_cape});
        std::fs::write(
            game_home.join("horizon-appearance.json"),
            descriptor.to_string(),
        )
        .map_err(|_| "Could not prepare the game's appearance.".into())
    }
}
fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Original procedural designs, not copies of official entitlement capes.
pub fn cape_design(index: usize) -> image::RgbaImage {
    let colors = [
        [93, 63, 214, 255],
        [28, 137, 134, 255],
        [186, 79, 58, 255],
        [54, 88, 153, 255],
    ];
    image::RgbaImage::from_fn(64, 32, |x, y| {
        let mut color = colors[index % colors.len()];
        if (x + y * (index as u32 % 3)) % 8 < 2 {
            color = [224, 222, 239, 255];
        }
        image::Rgba(color)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gallery_round_trip_is_scoped_and_cannot_escape_its_image_store() {
        let root = std::env::temp_dir().join(format!("horizon-wardrobe-{}", std::process::id()));
        let mut library = Library::default();
        let mut png = std::io::Cursor::new(vec![]);
        cape_design(0)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let id = library
            .import(&root, "Cape", Kind::Cape, png.get_ref())
            .unwrap();
        library
            .select(&root, "offline:Player", &id, Kind::Cape)
            .unwrap();
        let loaded = Library::load(&root).unwrap();
        assert!(loaded.selections.contains_key("offline:Player"));
        assert!(!loaded.selections.contains_key("msa:Player"));
        assert!(Library::image(&root, "../private").is_err());
        assert!(
            library
                .select(&root, "ely:Player", "missing", Kind::Skin)
                .is_err()
        );
        library
            .prepare_game(&root, "offline:Player", &root.join("game"), None)
            .unwrap();
        assert!(root.join("game/horizon-appearance.json").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }
}
