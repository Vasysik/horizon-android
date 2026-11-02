//! Player texture layout normalization, shared by the engine and launcher previews.
/// A cape in a 64x64 skin slot: the 64x32 picture (or the older 22x17,
/// which has the same layout) in its top half. Vanilla draws a cape of any
/// size; a larger one is scaled down to 64x32 here.
pub fn process_cape(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return None;
    }
    let mut img = vec![0u8; 64 * 64 * 4];
    let (fit_w, fit_h) = if w <= 64 && h <= 32 { (w, h) } else { (64, 32) };
    for y in 0..fit_h {
        for x in 0..fit_w {
            let (sx, sy) = (x * w / fit_w, y * h / fit_h);
            let (s, d) = ((sy * w + sx) * 4, (y * 64 + x) * 4);
            img[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    Some(img)
}

/// `SkinTextureDownloader.processLegacySkin` on RGBA pixels: 64x32 skins
/// get the 64x64 layout, and the opaque parts lose their alpha. `None` for
/// any other size (the download fails).
pub fn process_legacy_skin(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if width != 64 || height != 32 && height != 64 {
        return None;
    }
    let legacy = height == 32;
    let mut img = vec![0u8; 64 * 64 * 4];
    let n = rgba.len().min(img.len());
    img[..n].copy_from_slice(&rgba[..n]);
    let at = |x: i32, y: i32| ((y * 64 + x) * 4) as usize;
    if legacy {
        let rects = [
            (4, 16, 16, 32, 4, 4),
            (8, 16, 16, 32, 4, 4),
            (0, 20, 24, 32, 4, 12),
            (4, 20, 16, 32, 4, 12),
            (8, 20, 8, 32, 4, 12),
            (12, 20, 16, 32, 4, 12),
            (44, 16, -8, 32, 4, 4),
            (48, 16, -8, 32, 4, 4),
            (40, 20, 0, 32, 4, 12),
            (44, 20, -8, 32, 4, 12),
            (48, 20, -16, 32, 4, 12),
            (52, 20, -8, 32, 4, 12),
        ];
        for (sx, sy, ox, oy, w, h) in rects {
            for y in 0..h {
                for x in 0..w {
                    let (s, d) = (at(sx + x, sy + y), at(sx + ox + w - 1 - x, sy + oy + y));
                    let p: [u8; 4] = img[s..s + 4].try_into().unwrap();
                    img[d..d + 4].copy_from_slice(&p);
                }
            }
        }
    }
    let no_alpha = |img: &mut Vec<u8>, x0: i32, y0: i32, x1: i32, y1: i32| {
        for x in x0..x1 {
            for y in y0..y1 {
                img[at(x, y) + 3] = 255;
            }
        }
    };
    no_alpha(&mut img, 0, 0, 32, 16);
    if legacy {
        // doNotchTransparencyHack: a fully opaque hat layer is cleared.
        let opaque = (32..64).all(|x| (0..32).all(|y| img[at(x, y) + 3] >= 128));
        if opaque {
            for x in 32..64 {
                for y in 0..32 {
                    img[at(x, y) + 3] = 0;
                }
            }
        }
    }
    no_alpha(&mut img, 0, 16, 64, 32);
    no_alpha(&mut img, 16, 48, 48, 64);
    Some(img)
}
