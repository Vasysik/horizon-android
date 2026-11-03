//! Frontend-independent appearance handoff, schema 1. Read each game launch.
use std::{
    path::Path,
    sync::{Arc, RwLock},
};
#[derive(Default)]
pub struct Appearance {
    pub skin: Option<(Vec<u8>, bool)>,
    pub cape: Option<Vec<u8>>,
    pub hide_cape: bool,
}
static CURRENT: RwLock<Option<Arc<Appearance>>> = RwLock::new(None);
pub fn current() -> Option<Arc<Appearance>> {
    CURRENT.read().ok()?.clone()
}
pub fn configure(home: &Path) -> Result<(), String> {
    *CURRENT
        .write()
        .map_err(|_| "Appearance state unavailable")? = None;
    use std::io::Read;
    let bytes = match std::fs::File::open(home.join("horizon-appearance.json")).and_then(|f| {
        let mut bytes = Vec::new();
        f.take(65537).read_to_end(&mut bytes)?;
        Ok(bytes)
    }) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Could not read the appearance handoff".into()),
    };
    if bytes.len() > 65536 {
        return Err("Appearance handoff is too large".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid appearance JSON")?;
    if value["schema"] != 1 {
        return Err("Unsupported appearance schema".into());
    }
    let read = |key: &str| -> Result<Option<(u32, u32, Vec<u8>)>, String> {
        let Some(path) = value[key].as_str() else {
            return Ok(None);
        };
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err("Appearance PNG paths must be absolute".into());
        }
        use std::io::Read;
        let mut data = Vec::new();
        std::fs::File::open(path)
            .and_then(|f| f.take(1024 * 1024 + 1).read_to_end(&mut data))
            .map_err(|_| "Could not read appearance PNG")?;
        if data.len() > 1024 * 1024 {
            return Err("Appearance PNG exceeds 1 MiB".into());
        }
        let reader = png::Decoder::new(std::io::Cursor::new(&data))
            .read_info()
            .map_err(|_| "Invalid appearance PNG")?;
        let (w, h) = (reader.info().width, reader.info().height);
        let valid = if key == "skin" {
            w == 64 && (h == 32 || h == 64)
        } else {
            (w == 64 && h == 32) || (w == 22 && h == 17)
        };
        if !valid {
            return Err("Invalid appearance PNG dimensions".into());
        }
        crate::hud::decode_bytes(&data)
            .map(Some)
            .map_err(|_| "Could not decode appearance PNG".into())
    };
    let skin = read("skin")?
        .and_then(|(w, h, rgba)| super::process_legacy_skin(w, h, &rgba))
        .map(|rgba| (rgba, value["slim"].as_bool().unwrap_or(false)));
    let cape = read("cape")?.and_then(|(w, h, rgba)| super::process_cape(w, h, &rgba));
    *CURRENT
        .write()
        .map_err(|_| "Appearance state unavailable")? = Some(Arc::new(Appearance {
        skin,
        cape,
        hide_cape: value["hide_cape"].as_bool().unwrap_or(false),
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_loads_pixels_and_clears_state_between_games_and_on_errors() {
        let root = std::env::temp_dir().join(format!(
            "horizon-appearance-contract-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("skin.png");
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 64, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&vec![127; 64 * 64 * 4])
                .unwrap();
        }
        std::fs::write(&image, bytes).unwrap();
        let handoff = root.join("horizon-appearance.json");
        std::fs::write(
            &handoff,
            serde_json::json!({"schema":1,"skin":image,"slim":true,"hide_cape":true}).to_string(),
        )
        .unwrap();
        configure(&root).unwrap();
        let appearance = current().unwrap();
        assert_eq!(appearance.skin.as_ref().unwrap().0.len(), 64 * 64 * 4);
        assert!(appearance.skin.as_ref().unwrap().1 && appearance.hide_cape);
        std::fs::remove_file(&handoff).unwrap();
        configure(&root).unwrap();
        assert!(current().is_none());
        for bad in [
            serde_json::json!({"schema":2}),
            serde_json::json!({"schema":1,"skin":"relative.png"}),
        ] {
            std::fs::write(&handoff, bad.to_string()).unwrap();
            assert!(configure(&root).is_err());
            assert!(current().is_none());
        }
        std::fs::write(&handoff, vec![b' '; 65537]).unwrap();
        assert!(configure(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
