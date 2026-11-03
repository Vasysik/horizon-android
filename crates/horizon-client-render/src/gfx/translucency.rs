//! Vanilla 26.3 MeshData.SortState / VertexSorting.byDistance.
//! Keep vertex order intact; only the quad indices change as the camera moves.

/// MeshData.decodeQuadCentroids uses the opposite corners, not all four vertices.
pub fn quad_center(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (a[i] + b[i]) / 2.0)
}

#[derive(Default)]
pub struct Sorter {
    order: Vec<u64>,
    scratch: Vec<u64>,
    indices: Vec<u32>,
}
impl Sorter {
    pub fn sort(&mut self, centers: &[[f32; 3]], camera: [f32; 3]) -> &[u32] {
        self.order.clear();
        self.order.extend(centers.iter().enumerate().map(|(i, c)| {
            let [x, y, z] = std::array::from_fn(|j| camera[j] - c[j]);
            // JOML associates x*x + (y*y + z*z), which matters for rounded ties.
            sort_key(x * x + (y * y + z * z), i as u32)
        }));
        // Stable distance ties without per-sort allocation after warm-up.
        if self.order.len() < 2048 {
            self.order.sort_unstable();
        } else if !self.order.is_sorted() {
            if self.order.windows(2).all(|p| p[0] > p[1]) {
                self.order.reverse();
            } else {
                radix_distance_sort(&mut self.order, &mut self.scratch);
            }
        }
        self.indices.clear();
        for &key in &self.order {
            let v = (key as u32) * 4;
            self.indices.extend([v, v + 1, v + 2, v + 2, v + 3, v]);
        }
        &self.indices
    }
}
// Four stable byte passes over the distance word. Input indices already
// ascend, so ties retain mesh order without sorting the index word.
// Reuse scratch storage across sections and camera updates.
fn radix_distance_sort(order: &mut Vec<u64>, scratch: &mut Vec<u64>) {
    scratch.resize(order.len(), 0);
    for shift in [32, 40, 48, 56] {
        let mut counts = [0usize; 256];
        for &key in order.iter() {
            counts[((key >> shift) & 255) as usize] += 1;
        }
        if counts.contains(&order.len()) {
            continue;
        }
        let mut at = 0;
        for n in &mut counts {
            let size = *n;
            *n = at;
            at += size;
        }
        for &key in order.iter() {
            let bucket = ((key >> shift) & 255) as usize;
            scratch[counts[bucket]] = key;
            counts[bucket] += 1;
        }
        std::mem::swap(order, scratch);
    }
}
// Monotonic encoding of f32::total_cmp, including signed zero and NaNs.
// Complement for descending distance; the low word preserves mesh order.
fn sort_key(distance: f32, index: u32) -> u64 {
    let bits = distance.to_bits();
    let ascending = if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits ^ 0x8000_0000
    };
    (u64::from(!ascending) << 32) | u64::from(index)
}

pub struct SortState {
    centers: Vec<[f32; 3]>,
    last: Option<([i32; 3], [i32; 3])>,
}

impl SortState {
    pub fn new(centers: Vec<[f32; 3]>) -> Self {
        Self {
            centers,
            last: None,
        }
    }

    pub fn is_sorted(&self) -> bool {
        self.last.is_some()
    }

    /// Large tiles cannot use the section viewpoint shortcut: the camera may
    /// cross a quad's plane while remaining on the same side of the first 16³.
    pub fn update_lod<'a>(
        &mut self,
        camera: [f64; 3],
        origin: [i32; 3],
        sorter: &'a mut Sorter,
    ) -> Option<&'a [u32]> {
        let block = camera.map(|v| v.floor() as i32);
        if self.last.is_some_and(|(last, _)| last == block) {
            return None;
        }
        self.last = Some((block, [0; 3]));
        Some(sorter.sort(
            &self.centers,
            std::array::from_fn(|i| (camera[i] - f64::from(origin[i])) as f32),
        ))
    }

    /// TranslucencyPointOfView and ResortTransparencyTask: diagonal views keep
    /// their ordering until the point of view changes; aligned views also
    /// resort on crossing a block boundary. Sub-block movement needs no upload.
    pub fn update<'a>(
        &mut self,
        camera: [f64; 3],
        origin: [i32; 3],
        sorter: &'a mut Sorter,
    ) -> Option<&'a [u32]> {
        let block = camera.map(|v| v.floor() as i32);
        let pov = std::array::from_fn(|i| ((block[i] >> 4) - (origin[i] >> 4)).clamp(-1, 1));
        if let Some((last_block, last_pov)) = self.last
            && last_pov == pov
            && (last_block == block || !pov.contains(&0))
        {
            return None;
        }
        self.last = Some((block, pov));
        Some(sorter.sort(
            &self.centers,
            std::array::from_fn(|i| (camera[i] - f64::from(origin[i])) as f32),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_distances_keep_mesh_order_and_triangle_winding() {
        let mut sorter = Sorter::default();
        assert_eq!(
            sorter.sort(
                &[[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
                [0.0; 3]
            ),
            &[8, 9, 10, 10, 11, 8, 0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4]
        );
        assert!(sorter.sort(&[], [0.0; 3]).is_empty());
    }

    #[test]
    fn in_place_sort_matches_stable_reference_for_ties_and_non_finite_values() {
        let mut centers = vec![[0.0; 3]; 8192];
        let mut rng = 12345u32;
        for c in &mut centers {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            *c = [(rng % 17) as f32 - 8.0, 0.0, 0.0];
        }
        centers.extend([
            [f32::INFINITY, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0],
            [-0.0, 0.0, 0.0],
        ]);
        let mut sorter = Sorter::default();
        for count in [0, 1, 17, 2047, 2048, 2049, 8195] {
            let centers = &centers[..count];
            for camera in [[0.0; 3], [3.5, -1.0, 2.0], [f32::INFINITY, 0.0, 0.0]] {
                let mut reference: Vec<_> = centers
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let [x, y, z] = std::array::from_fn(|j| camera[j] - c[j]);
                        (i as u32, x * x + (y * y + z * z))
                    })
                    .collect();
                reference.sort_by(|a, b| b.1.total_cmp(&a.1));
                let expected: Vec<_> = reference
                    .into_iter()
                    .flat_map(|(i, _)| {
                        let v = i * 4;
                        [v, v + 1, v + 2, v + 2, v + 3, v]
                    })
                    .collect();
                assert_eq!(sorter.sort(centers, camera), expected);
            }
        }
    }
    #[test]
    fn large_ordered_runs_and_identical_distances_keep_mesh_order() {
        let mut sorter = Sorter::default();
        let centers: Vec<_> = (0..8192).map(|i| [i as f32, 0.0, 0.0]).collect();
        assert_eq!(sorter.sort(&centers, [0.0; 3])[0], 8191 * 4);
        assert_eq!(sorter.sort(&centers, [8192.0, 0.0, 0.0])[0], 0);
        let same = vec![[1.0, 0.0, 0.0]; 8192];
        let indices = sorter.sort(&same, [0.0; 3]);
        for (i, quad) in indices.chunks_exact(6).enumerate() {
            assert_eq!(quad[0], i as u32 * 4);
        }
    }
    #[test]
    fn joml_distance_rounding_keeps_vanilla_ties_stable() {
        let centers = [
            [
                f32::from_bits(0x40e8_eea8),
                f32::from_bits(0x40e6_f5d4),
                f32::from_bits(0x40d0_9846),
            ],
            [
                f32::from_bits(0x410e_22f3),
                f32::from_bits(0x4123_1fa0),
                f32::from_bits(0x4059_a478),
            ],
        ];
        let mut sorter = Sorter::default();
        assert_eq!(
            sorter.sort(&centers, [16.0; 3]),
            [0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4]
        );
    }

    #[test]
    fn movement_reverses_faces_without_moving_vertices() {
        let centers = [[0.5, 0.5, 0.0], [0.5, 0.5, 1.0]];
        let mut sorter = Sorter::default();
        assert_eq!(sorter.sort(&centers, [0.5, 0.5, -2.0])[0], 4);
        assert_eq!(sorter.sort(&centers, [0.5, 0.5, 3.0])[0], 0);
    }

    #[test]
    fn cached_viewpoint_handles_negative_and_large_coordinates() {
        let origin = [-30_000_000, -64, 30_000_000];
        let mut state = SortState::new(vec![[0.5; 3]]);
        let mut sorter = Sorter::default();
        let camera = [-29_999_999.5, -63.5, 30_000_000.5];
        assert!(state.update(camera, origin, &mut sorter).is_some());
        assert!(state.update(camera, origin, &mut sorter).is_none());
        assert!(
            state
                .update([-29_999_999.1, -63.1, 30_000_000.9], origin, &mut sorter)
                .is_none()
        );
        assert!(
            state
                .update([-29_999_998.9, -63.1, 30_000_000.9], origin, &mut sorter)
                .is_some()
        );
    }
    #[test]
    fn large_tiles_resort_inside_the_same_section_viewpoint() {
        let mut state = SortState::new(vec![[32.0, 0.0, 0.0], [96.0, 0.0, 0.0]]);
        let mut sorter = Sorter::default();
        assert_eq!(
            state
                .update_lod([48.0, 20.0, 20.0], [0; 3], &mut sorter)
                .unwrap()[0],
            4
        );
        assert!(
            state
                .update_lod([48.5, 20.0, 20.0], [0; 3], &mut sorter)
                .is_none()
        );
        // Both views have positive x/y/z signs relative to the first section,
        // so the section-sized shortcut would incorrectly keep the old order.
        assert_eq!(
            state
                .update_lod([80.0, 20.0, 20.0], [0; 3], &mut sorter)
                .unwrap()[0],
            0
        );
    }
}
