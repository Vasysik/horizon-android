//! Bedrock's touch buttons, which players know: fetched once on the phone from Mojang's own
//! samples repository (bedrock-samples, at the revision `assets/bedrock-touch.json` pins, each
//! file checked by its git blob SHA-1, Mojang's terms kept beside them) and written as the
//! `starling:touch/<name>` sprites the game draws its controls with (hud.rs: they win over
//! Starling's own, packaging/touch; a resource pack's win over both). The project carries their
//! names and hashes only. Bedrock has no icon for some of Starling's buttons (pause, chat...):
//! those are its plate with the icon taken off and a glyph of Starling's in its place.
use std::path::{Path, PathBuf};
use std::time::Duration;

const MANIFEST: &str = include_str!("../assets/bedrock-touch.json");

/// The sprites' folder in the resources: their own namespace, which a pack's sprites still win
/// over.
fn folder(resources: &Path) -> PathBuf {
    resources.join("assets/starling/textures/gui/sprites/touch")
}

/// Written last, with the revision: the buttons are all there.
fn marker(resources: &Path) -> PathBuf {
    folder(resources).join(".bedrock")
}

/// The plate's grey, where its icon is taken off.
const FACE: [u8; 4] = [128, 128, 128, 255];
/// Its icon's colour, up (black) and pressed (light grey), as Bedrock draws them.
const INK: [[u8; 4]; 2] = [[0, 0, 0, 255], [230, 230, 230, 255]];

/// Starling's glyphs for the buttons Bedrock has none for, drawn as Bedrock's own: black on the
/// plate, light grey pressed.
const GLYPHS: &[(&str, &str)] = &[
    (
        "pause",
        "###..###\n###..###\n###..###\n###..###\n###..###\n###..###\n###..###\n###..###",
    ),
    ("inventory", "##..##..##\n##..##..##"),
    (
        "chat",
        ".##########.\n#..........#\n#.########.#\n#..........#\n#.######...#\n#..........#\n\
         .###.######.\n...#.#......\n...##.......",
    ),
    (
        "camera",
        "...######...\n.##......##.\n#....##....#\n#...####...#\n#....##....#\n.##......##.\n\
         ...######...",
    ),
    (
        "drop",
        "....##....\n....##....\n....##....\n.########.\n..######..\n...####...\n....##....\n\
         ..........\n##########",
    ),
    (
        "offhand",
        "...#........\n..##........\n.##########.\n..##........\n...#........\n........#...\n\
         ........##..\n.##########.\n........##..\n........#...",
    ),
    (
        "command",
        ".......#\n......##\n.....##.\n....##..\n...##...\n..##....\n.##.....\n##......\n#.......",
    ),
    (
        "debug",
        "######.#####.\n######.######\n##.........##\n##.........##\n#####...####.\n\
         #####...####.\n##.........##\n##.........##\n##.....######\n##.....#####.",
    ),
    (
        "pick",
        "......#......\n....##.##....\n..##.....##..\n##.........##\n#.##.....##.#\n\
         #...##.##...#\n#.....#.....#\n#.....#.....#\n#.....#.....#\n##....#....##\n\
         ..##..#..##..\n....##.##....\n......#......",
    ),
    (
        "players",
        "###.#########\n###.#########\n.............\n###.#########\n###.#########\n\
         .............\n###.#########\n###.#########",
    ),
    // A custom button's icons (settings.rs `CUSTOM_ICONS`), and the plate alone for its canvas.
    ("custom_blank", ""),
    (
        "custom_star",
        "......##......\n......##......\n......##......\n......##......\n##############\n##############\n..##########..\n..##########..\n....######....\n....######....\n..####..####..\n..####..####..\n..##......##..\n..##......##..",
    ),
    (
        "custom_heart",
        "..####..####..\n..####..####..\n##############\n##############\n##############\n##############\n..##########..\n..##########..\n....######....\n....######....\n......##......\n......##......",
    ),
    (
        "custom_shield",
        "##############\n##############\n##############\n##############\n##############\n##############\n##############\n##############\n..##########..\n..##########..\n....######....\n....######....\n......##......\n......##......",
    ),
    (
        "custom_pickaxe",
        "..##########....\n..##########....\n##........####..\n##........####..\n..........######\n..........######\n........####..##\n........####..##\n......####....##\n......####....##\n....####......##\n....####......##\n..####..........\n..####..........\n####............\n####............",
    ),
    (
        "custom_potion",
        "....######....\n....######....\n......##......\n......##......\n......##......\n......##......\n....######....\n....######....\n..##########..\n..##########..\n##############\n##############\n##############\n##############\n..##########..\n..##########..",
    ),
    (
        "custom_apple",
        "........##....\n........##....\n......##......\n......##......\n..####..####..\n..####..####..\n##############\n##############\n##############\n##############\n##############\n##############\n..##########..\n..##########..",
    ),
    (
        "custom_home",
        "......####......\n......####......\n....########....\n....########....\n..############..\n..############..\n################\n################\n..####....####..\n..####....####..\n..####....####..\n..####....####..\n..####....####..\n..####....####..",
    ),
    (
        "custom_flag",
        "####..........\n####..........\n##############\n##############\n##############\n##############\n##############\n##############\n####..........\n####..........\n####..........\n####..........\n####..........\n####..........\n####..........\n####..........",
    ),
    (
        "custom_fire",
        "......##......\n......##......\n....####......\n....####......\n....######..##\n....######..##\n..##########..\n..##########..\n##############\n##############\n######..######\n######..######\n####......####\n####......####\n..##########..\n..##########..",
    ),
    (
        "custom_skull",
        "..##########..\n..##########..\n##############\n##############\n##....##....##\n##....##....##\n##....##....##\n##....##....##\n##############\n##############\n..##########..\n..##########..\n..##..##..##..\n..##..##..##..",
    ),
    (
        "custom_crown",
        "##....####....##\n##....####....##\n##....####....##\n##....####....##\n####..####..####\n####..####..####\n################\n################\n################\n################\n################\n################",
    ),
    (
        "custom_plus",
        "....####....\n....####....\n....####....\n....####....\n############\n############\n############\n############\n....####....\n....####....\n....####....\n....####....",
    ),
    (
        "custom_check",
        "............####\n............####\n..........####..\n..........####..\n####....####....\n####....####....\n..########......\n..########......\n....####........\n....####........\n....##..........\n....##..........",
    ),
    (
        "custom_cross",
        "####....####\n####....####\n..########..\n..########..\n....####....\n....####....\n....####....\n....####....\n..########..\n..########..\n####....####\n####....####",
    ),
    (
        "custom_music",
        "....##########\n....##########\n....##########\n....##########\n....##......##\n....##......##\n....##......##\n....##......##\n######..######\n######..######\n######..######\n######..######\n######..######\n######..######",
    ),
];

/// What the marker holds: the revision and the glyphs drawn, so a glyph added later is drawn on
/// the next start.
fn stamp_key(revision: &str) -> String {
    let names: Vec<&str> = GLYPHS.iter().map(|(name, _)| *name).collect();
    format!("{revision} {}", names.join(","))
}

/// git's name for `bytes`: the SHA-1 of `blob <length>\0` and them.
fn blob_sha1(bytes: &[u8]) -> String {
    use sha1::Digest;
    let mut hash = sha1::Sha1::new();
    hash.update(format!("blob {}\0", bytes.len()).as_bytes());
    hash.update(bytes);
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// An RGBA image as a PNG.
fn png(w: u32, h: u32, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(pixels).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

/// `plate` (Bedrock's 22x22 sprint button) with its icon taken off: the icon's pixels inside the
/// face become the face's grey.
fn blank(plate: &mut [u8], w: u32, pressed: bool) {
    let ink = INK[usize::from(pressed)];
    for y in 2..20u32 {
        for x in 3..19u32 {
            let i = ((y * w + x) * 4) as usize;
            if plate.get(i..i + 4) == Some(&ink[..]) {
                plate[i..i + 4].copy_from_slice(&FACE);
            }
        }
    }
}

/// `glyph` (rows of `#`) on a blank plate, centred where Bedrock's icons are (a pixel lower when
/// pressed, as the whole button is).
fn stamp(plate: &[u8], w: u32, glyph: &str, pressed: bool) -> Vec<u8> {
    let mut out = plate.to_vec();
    let rows: Vec<&str> = glyph.lines().map(str::trim).collect();
    let gw = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
    let gh = rows.len() as i32;
    let (ox, oy) = (11 - (gw + 1) / 2, 10 - (gh + 1) / 2 + i32::from(pressed));
    let ink = INK[usize::from(pressed)];
    for (dy, row) in rows.iter().enumerate() {
        for (dx, c) in row.bytes().enumerate() {
            let (x, y) = (ox + dx as i32, oy + dy as i32);
            if c == b'#' && (0..w as i32).contains(&x) && (0..w as i32).contains(&y) {
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                out[i..i + 4].copy_from_slice(&ink);
            }
        }
    }
    out
}

/// Whether this revision's buttons, and every glyph, are all there.
pub fn present(resources: &Path) -> bool {
    let revision = serde_json::from_str::<serde_json::Value>(MANIFEST)
        .ok()
        .and_then(|m| m["revision"].as_str().map(stamp_key));
    revision.is_some_and(|r| std::fs::read_to_string(marker(resources)).ok() == Some(r))
}

/// Fetches and writes them into `resources` (a prepared resources folder) unless they are there;
/// the number of sprites written.
pub fn fetch(resources: &Path) -> Result<usize, String> {
    if present(resources) {
        return Ok(0);
    }
    if !resources.join("assets/minecraft").is_dir() {
        return Err(format!("{} holds no resources", resources.display()));
    }
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).map_err(|e| e.to_string())?;
    let (repository, revision) = (
        manifest["repository"].as_str().unwrap_or_default(),
        manifest["revision"].as_str().unwrap_or_default(),
    );
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();
    let get = |entry: &serde_json::Value| -> Result<Vec<u8>, String> {
        let source = entry["source"].as_str().unwrap_or_default();
        let url = format!("https://raw.githubusercontent.com/{repository}/{revision}/{source}");
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut agent
                .get(&url)
                .call()
                .map_err(|e| format!("{source}: {e}"))?
                .into_reader(),
            &mut bytes,
        )
        .map_err(|e| format!("{source}: {e}"))?;
        let sha1 = entry["sha1"].as_str().unwrap_or_default();
        if blob_sha1(&bytes) != sha1 {
            return Err(format!("{source}: not the file the manifest names"));
        }
        Ok(bytes)
    };
    let dir = folder(resources);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let write = |name: &str, bytes: &[u8]| {
        let path = dir.join(format!("{name}.png"));
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    };
    let mut written = 0;
    let mut plates = [None, None];
    for entry in manifest["files"].as_array().into_iter().flatten() {
        let name = entry["name"].as_str().unwrap_or_default();
        let bytes = get(entry)?;
        match name {
            "plate" | "plate_pressed" => {
                let (w, h, mut pixels) =
                    crate::hud::decode_bytes(&bytes).map_err(|e| format!("{name}: {e}"))?;
                let pressed = name == "plate_pressed";
                blank(&mut pixels, w, pressed);
                plates[usize::from(pressed)] = Some((w, h, pixels));
            }
            _ => {
                write(name, &bytes)?;
                written += 1;
            }
        }
    }
    let [Some(up), Some(down)] = plates else {
        return Err("the manifest names no plate".into());
    };
    for (name, glyph) in GLYPHS {
        for (suffix, (w, h, plate), pressed) in [("", &up, false), ("_pressed", &down, true)] {
            write(
                &format!("{name}{suffix}"),
                &png(*w, *h, &stamp(plate, *w, glyph, pressed))?,
            )?;
            written += 1;
        }
    }
    // The terms the textures come with, beside them (out of the sprites' folder).
    let license = &manifest["license"];
    if license.is_object() {
        let text = get(license)?;
        let path = resources.join("assets/starling/bedrock-touch-LICENSE.md");
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    std::fs::write(marker(resources), stamp_key(revision)).map_err(|e| e.to_string())?;
    Ok(written)
}

/// [`fetch`], its outcome in the log (no network: Starling's own buttons, and again next start).
pub fn fetch_logged(resources: &Path) {
    match fetch(resources) {
        Ok(0) => {}
        Ok(n) => log::info!("Horizon: Bedrock's touch buttons fetched from Mojang ({n} sprites)"),
        Err(e) => log::info!("Horizon: Bedrock's touch buttons not fetched: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blob_is_named_as_git_names_it() {
        // `git hash-object` of "hello\n".
        assert_eq!(
            blob_sha1(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn the_manifest_names_every_sprite_starling_draws() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let mut names: Vec<String> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap().to_owned())
            .filter(|n| !n.starts_with("plate"))
            .collect();
        for (glyph, _) in GLYPHS {
            names.push((*glyph).to_owned());
            names.push(format!("{glyph}_pressed"));
        }
        for name in crate::hud::touch_sprite_names() {
            if name != "dpad_middle" {
                assert!(names.iter().any(|n| n == name), "{name}");
            }
        }
    }

    /// The real thing, from Mojang (the network): `cargo test -p horizon-client --lib
    /// bedrock_touch -- --ignored`.
    #[test]
    #[ignore = "fetches from github.com"]
    fn the_buttons_come_from_mojang_checked() {
        let root = std::env::temp_dir().join(format!("horizon-bedrock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets/minecraft")).unwrap();
        let written = fetch(&root).unwrap();
        assert_eq!(written, 18 + 2 * GLYPHS.len());
        assert!(present(&root));
        for name in crate::hud::touch_sprite_names().filter(|n| *n != "dpad_middle") {
            let path = folder(&root).join(format!("{name}.png"));
            let (w, h, _) = crate::hud::decode(&path).unwrap();
            assert!(w >= 22 && h >= 22, "{name}: {w}x{h}");
        }
        assert_eq!(fetch(&root).unwrap(), 0, "once is enough");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_glyph_goes_where_the_icon_was() {
        // A 22x22 plate: grey, its icon a black pixel in the middle.
        let w = 22;
        let mut plate = vec![128; (w * w * 4) as usize];
        let centre = ((10 * w + 11) * 4) as usize;
        plate[centre..centre + 4].copy_from_slice(&[0, 0, 0, 255]);
        blank(&mut plate, w, false);
        assert_eq!(&plate[centre..centre + 4], &FACE);
        let out = stamp(&plate, w, "##\n##", false);
        let black = out.chunks(4).filter(|p| *p == INK[0]).count();
        assert_eq!(black, 4);
        let pressed = stamp(&plate, w, "##\n##", true);
        let light = pressed.chunks(4).filter(|p| *p == INK[1]).count();
        assert_eq!(light, 4);
    }
}
