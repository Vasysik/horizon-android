//! The images players give their own touch buttons, drawn in the options or imported: one PNG a
//! button in `custom-buttons/`, each with a slot kept for it in the atlas.
use std::{fs, io, path::PathBuf};

/// A button's image is this many pixels a side, as the game's own buttons.
pub const SIZE: u32 = 24;
/// How many buttons can have one. A button's id is its slot.
pub const SLOTS: u32 = 32;

/// The slot of the image button `id` wears while pressed.
pub fn pressed_slot(id: u32) -> u32 {
    id + SLOTS
}

/// A slot's sprite in the atlas.
pub fn sprite(id: u32) -> String {
    format!("starling:gui/sprites/touch/own_{id}")
}

fn path(id: u32) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("custom-buttons")
        .join(format!("{id}.png"))
}

/// A button's image, from its file.
pub fn load(id: u32) -> Option<Vec<u8>> {
    let (w, h, pixels) = crate::hud::decode_bytes(&fs::read(path(id)).ok()?).ok()?;
    Some(fit(w, h, &pixels))
}

/// Keeps a button's image in its file.
pub fn save(id: u32, rgba: &[u8]) -> io::Result<()> {
    let path = path(id);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut encoder = png::Encoder::new(fs::File::create(path)?, SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer.write_image_data(rgba).map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)
}

/// Forgets a button's image.
pub fn remove(id: u32) {
    let _ = fs::remove_file(path(id));
}

/// Forgets both of a button's images.
pub fn remove_all(id: u32) {
    remove(id);
    remove(pressed_slot(id));
}

/// An image of any size made a button's: its middle square, each pixel the average of what falls
/// in it, by how opaque that is (a clear pixel's colour shows through nothing).
pub fn fit(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let side = width.min(height).max(1);
    let (left, up) = (
        (width - side.min(width)) / 2,
        (height - side.min(height)) / 2,
    );
    let size = SIZE as usize;
    let mut out = vec![0; size * size * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let span = |v: u32| {
                (
                    v * side / SIZE,
                    ((v + 1) * side / SIZE).max(v * side / SIZE + 1),
                )
            };
            let (x0, x1) = span(x);
            let (y0, y1) = span(y);
            let mut sum = [0u32; 4];
            let mut count = 0;
            for sy in y0..y1.min(side) {
                for sx in x0..x1.min(side) {
                    let at = (((up + sy) * width + left + sx) * 4) as usize;
                    let Some(p) = rgba.get(at..at + 4) else {
                        continue;
                    };
                    let alpha = u32::from(p[3]);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]) * alpha;
                    }
                    sum[3] += alpha;
                    count += 1;
                }
            }
            let at = (y as usize * size + x as usize) * 4;
            for c in 0..3 {
                out[at + c] = sum[c].checked_div(sum[3]).unwrap_or(0) as u8;
            }
            out[at + 3] = sum[3].checked_div(count).unwrap_or(0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_fits_a_button_from_its_middle() {
        // Red with a clear left half, wider than tall: the middle square is what counts.
        let (w, h) = (96u32, 48u32);
        let mut rgba = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                let on = x >= w / 2;
                rgba.extend([255, 0, 0, if on { 255 } else { 0 }]);
            }
        }
        let out = fit(w, h, &rgba);
        assert_eq!(out.len(), (SIZE * SIZE * 4) as usize);
        let at = |x: u32, y: u32| ((y * SIZE + x) * 4) as usize;
        assert_eq!(out[at(2, 12) + 3], 0);
        assert_eq!(&out[at(20, 12)..at(20, 12) + 4], [255, 0, 0, 255]);
        // Smaller than the button: each pixel fills its share.
        let out = fit(2, 2, &[9, 9, 9, 255].repeat(4));
        assert_eq!(&out[at(23, 23)..at(23, 23) + 4], [9, 9, 9, 255]);
    }
}
