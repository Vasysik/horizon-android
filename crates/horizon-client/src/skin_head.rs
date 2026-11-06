//! Real account heads: the 8x8 skin faces taken from the player's own skin.
//!
//! The launcher draws the account head before the game has loaded a skin, so
//! the name goes to Mojang's public profile APIs on a background thread and the
//! decoded faces are kept for the session. Offline (cracked) names have no
//! Mojang profile and fall back to the drawn Steve head.

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// The three 8x8 faces of a head, one `0xRRGGBB` pixel each, plus the second
/// skin layer over the face (`0` where the skin has no hat).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HeadSkin {
    pub front: [[u32; 8]; 8],
    pub top: [[u32; 8]; 8],
    pub side: [[u32; 8]; 8],
    pub hat: [[u32; 8]; 8],
}

static CACHE: OnceLock<Mutex<HashMap<String, Option<HeadSkin>>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Option<HeadSkin>>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Put an already decoded head in the session cache, so the launcher can draw
/// the account's real skin on its first frame instead of waiting for a lookup.
pub fn remember(player: &str, skin: HeadSkin) {
    let name = player.trim();
    if name.is_empty() || name.len() > 16 || name.contains(char::is_whitespace) {
        return;
    }
    if let Ok(mut map) = cache().lock() {
        map.insert(name.to_owned(), Some(skin));
    }
}

/// The real head for `player`: `None` while the request is in flight and for
/// names with no Mojang profile (offline mode).
pub fn head(player: &str) -> Option<HeadSkin> {
    let name = player.trim();
    if name.is_empty() || name.len() > 16 || name.contains(char::is_whitespace) {
        return None;
    }
    let mut map = cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = map.get(name) {
        return *found;
    }
    // The game already prepared the selected account's skin (wardrobe, cloud
    // or local file) into this launch's appearance handoff. That is the skin
    // the game will show, so it wins over a public-profile lookup.
    if let Some(skin) = prepared_skin() {
        map.insert(name.to_owned(), Some(skin));
        return Some(skin);
    }
    // One attempt per name and session: a miss is cached as a miss.
    map.insert(name.to_owned(), None);
    let wanted = name.to_owned();
    std::thread::spawn(move || {
        let found = fetch(&wanted);
        if let Ok(mut map) = cache().lock() {
            map.insert(wanted, found);
        }
    });
    None
}

/// The skin this launch will actually use, already normalized to 64x64 RGBA.
fn prepared_skin() -> Option<HeadSkin> {
    let appearance = crate::skins::appearance::current()?;
    let (rgba, _slim) = appearance.skin.as_ref()?;
    from_rgba(64, 64, rgba)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout(Duration::from_secs(15))
        .build()
}

fn fetch(name: &str) -> Option<HeadSkin> {
    let agent = agent();
    let profile: serde_json::Value = serde_json::from_str(
        &agent
            .get(&format!(
                "https://api.mojang.com/users/profiles/minecraft/{name}"
            ))
            .call()
            .ok()?
            .into_string()
            .ok()?,
    )
    .ok()?;
    let id = profile.get("id")?.as_str()?;
    let session: serde_json::Value = serde_json::from_str(
        &agent
            .get(&format!(
                "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
            ))
            .call()
            .ok()?
            .into_string()
            .ok()?,
    )
    .ok()?;
    let encoded = session
        .get("properties")?
        .as_array()?
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("textures"))?
        .get("value")?
        .as_str()?;
    let payload: serde_json::Value =
        serde_json::from_slice(&crate::skins::base64_decode(encoded)?).ok()?;
    let url = payload.get("textures")?.get("SKIN")?.get("url")?.as_str()?;
    if !url.starts_with("https://textures.minecraft.net/")
        && !url.starts_with("http://textures.minecraft.net/")
    {
        return None;
    }
    let mut png = Vec::new();
    agent
        .get(url)
        .call()
        .ok()?
        .into_reader()
        .read_to_end(&mut png)
        .ok()?;
    let (w, h, rgba) = crate::hud::decode_bytes(&png).ok()?;
    from_rgba(w, h, &rgba)
}

fn pixel(w: u32, h: u32, rgba: &[u8], x: u32, y: u32) -> Option<(u32, u8)> {
    if x >= w || y >= h {
        return None;
    }
    let i = ((y * w + x) * 4) as usize;
    let rgb = (u32::from(rgba[i]) << 16) | (u32::from(rgba[i + 1]) << 8) | u32::from(rgba[i + 2]);
    Some((rgb, rgba[i + 3]))
}

/// Decode a skin PNG into its head faces. Used by the wardrobe, whose selected
/// image is already on disk and needs no profile lookup.
pub fn from_png(bytes: &[u8]) -> Option<HeadSkin> {
    let (w, h, rgba) = crate::hud::decode_bytes(bytes).ok()?;
    from_rgba(w, h, &rgba)
}

pub fn from_rgba(w: u32, h: u32, rgba: &[u8]) -> Option<HeadSkin> {
    // 64x32 is the legacy layout, which has no hat layer; modern skins are
    // 64x64. Anything smaller is not a Minecraft skin.
    if w < 64 || h < 32 || rgba.len() < (w as usize * h as usize * 4) {
        return None;
    }
    let mut skin = HeadSkin {
        front: [[0; 8]; 8],
        top: [[0; 8]; 8],
        side: [[0; 8]; 8],
        hat: [[0; 8]; 8],
    };
    for row in 0..8u32 {
        for col in 0..8u32 {
            skin.front[row as usize][col as usize] = pixel(w, h, rgba, 8 + col, 8 + row)?.0;
            skin.top[row as usize][col as usize] = pixel(w, h, rgba, 8 + col, row)?.0;
            skin.side[row as usize][col as usize] = pixel(w, h, rgba, col, 8 + row)?.0;
            if h >= 64 {
                let (rgb, alpha) = pixel(w, h, rgba, 40 + col, 8 + row)?;
                if alpha > 0 {
                    skin.hat[row as usize][col as usize] = (u32::from(alpha) << 24) | rgb;
                }
            }
        }
    }
    Some(skin)
}
