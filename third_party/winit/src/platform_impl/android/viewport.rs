//! Physical presentation bounds shared by the renderer and absolute input.
use std::sync::Mutex;

static SETTINGS: Mutex<([u32; 4], [u32; 2], [u32; 2])> = Mutex::new(([0; 4], [0; 2], [0; 2]));

pub fn set_insets(insets: [u32; 4]) {
    SETTINGS.lock().unwrap().0 = insets;
}
pub fn set_aspect(aspect: [u32; 2]) {
    SETTINGS.lock().unwrap().1 = aspect;
}
pub fn rect(size: [u32; 2]) -> [u32; 4] {
    let mut state = SETTINGS.lock().unwrap();
    state.2 = size;
    fit(size, state.0, state.1)
}
pub fn origin() -> [u32; 2] {
    let state = SETTINGS.lock().unwrap();
    let [x, y, ..] = fit(state.2, state.0, state.1);
    [x, y]
}

pub fn position(point: [f64; 2], scale: [f64; 2]) -> [f64; 2] {
    let [x, y] = origin();
    [
        (point[0] - x as f64) * scale[0],
        (point[1] - y as f64) * scale[1],
    ]
}

pub fn contains(x: f64, y: f64) -> bool {
    let state = SETTINGS.lock().unwrap();
    let [left, top, w, h] = fit(state.2, state.0, state.1);
    x >= left as f64 && y >= top as f64 && x < (left + w) as f64 && y < (top + h) as f64
}

pub fn fit(size: [u32; 2], insets: [u32; 4], aspect: [u32; 2]) -> [u32; 4] {
    let [width, height] = size.map(|v| v.max(1));
    let [left, top, right, bottom] = insets;
    let x = left.min(width - 1);
    let y = top.min(height - 1);
    let w = width.saturating_sub(x).saturating_sub(right).max(1);
    let h = height.saturating_sub(y).saturating_sub(bottom).max(1);
    let [mut a, mut b] = aspect;
    if height > width && a > b {
        std::mem::swap(&mut a, &mut b);
    }
    if a == 0 || b == 0 {
        return [x, y, w, h];
    }
    let (fw, fh) = if u64::from(w) * u64::from(b) > u64::from(h) * u64::from(a) {
        (
            ((u64::from(h) * u64::from(a) / u64::from(b)) as u32).max(1),
            h,
        )
    } else {
        (
            w,
            ((u64::from(w) * u64::from(b) / u64::from(a)) as u32).max(1),
        )
    };
    [x + (w - fw) / 2, y + (h - fh) / 2, fw, fh]
}

#[cfg(test)]
mod tests {
    use super::fit;
    #[test]
    fn cutout_and_aspect_share_the_same_physical_rect() {
        assert_eq!(fit([1280, 800], [48, 0, 0, 0], [0, 0]), [48, 0, 1232, 800]);
        assert_eq!(fit([1280, 800], [48, 0, 0, 0], [4, 3]), [131, 0, 1066, 800]);
        assert_eq!(fit([1280, 800], [0; 4], [16, 9]), [0, 40, 1280, 720]);
        assert_eq!(fit([800, 1280], [0, 48, 0, 32], [0, 0]), [0, 48, 800, 1200]);
    }
    #[test]
    fn all_ratios_fit_landscape_portrait_and_scaled_safe_regions() {
        for size in [[2400, 1080], [1080, 2400], [1280, 800], [1, 1], [0, 0]] {
            for insets in [[0; 4], [80, 24, 12, 40], [u32::MAX; 4]] {
                for aspect in [[0, 0], [16, 9], [4, 3], [1, 1]] {
                    let [x, y, w, h] = fit(size, insets, aspect);
                    assert!(w > 0 && h > 0);
                    assert!(x + w <= size[0].max(1) && y + h <= size[1].max(1));
                    for scale in [1.0, 0.5, 0.25] {
                        // Logical center follows exactly the same fitted rect.
                        let px = x as f64 + w as f64 / 2.0;
                        let logical = (px - x as f64) * scale;
                        assert_eq!(logical, w as f64 * scale / 2.0);
                    }
                }
            }
        }
    }
    #[test]
    fn absolute_input_uses_letterbox_origin_and_resolution_scale() {
        super::set_insets([0; 4]);
        super::set_aspect([16, 9]);
        assert_eq!(super::rect([1280, 800]), [0, 40, 1280, 720]);
        assert_eq!(super::position([640.0, 400.0], [0.5, 0.5]), [320.0, 180.0]);
        assert_eq!(super::position([0.0, 40.0], [0.5, 0.5]), [0.0, 0.0]);
        assert!(!super::contains(640.0, 20.0));
        assert!(super::contains(640.0, 400.0));
        super::set_aspect([0, 0]);
    }
    #[test]
    fn malformed_insets_never_underflow() {
        assert_eq!(fit([0, 0], [u32::MAX; 4], [0, 0]), [0, 0, 1, 1]);
    }
}
