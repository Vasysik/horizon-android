//! A mesh's solid quads sorted by the side they face, so that the renderer
//! can leave out the quads turned away from the eye before the GPU shades
//! their vertices.
//!
//! A quad in the plane `x = p` whose corners go counter-clockwise seen from
//! +X faces +X. From an eye at `x < p` it is seen from behind, and at
//! `x = p` edge on; the solid pipeline culls back faces, so it draws nothing
//! from there. A bucket of such quads draws nothing at all while the eye is
//! at or below the lowest of their planes, and likewise for the other five
//! axis directions. The seventh bucket holds every other quad (sloped,
//! rotated, degenerate, or whose two triangles wind opposite ways): it is
//! drawn from everywhere.
//!
//! The buckets lie one after another in the mesh, in an order chosen so that
//! the buckets an eye can see are usually one or two runs of quads: one draw
//! each. Coplanar quads facing one way keep their model order, so an overlay
//! on its base still draws in that order. Solid cutout quads are otherwise
//! ordered from the outside toward the section interior to reduce overdraw.
//!
//! The opaque quads (`OPAQUE_LAYER`) and the cutout ones of each bucket are
//! apart, so that each of Metal's two solid passes draws only its own quads
//! (`Part`). Where the renderer draws them in two passes (`group_parts`), all
//! the opaque buckets come first, then all the cutout ones, so each part is
//! one or two runs; elsewhere each bucket holds its opaque quads, then its
//! cutout ones, so one pass over all of them draws the runs it drew before.
use std::sync::atomic::{AtomicBool, Ordering};

use super::{OPAQUE_LAYER, TerrainVertex};

/// The buckets in mesh order: +X, +Z, other, +Y, -Z, -X, -Y. Seen from
/// above (+Y, the commonest case), each pair of horizontal sides with a
/// common corner is then one run, or two.
const BUCKETS: [Bucket; 7] = [
    Bucket::Side(0, true),
    Bucket::Side(2, true),
    Bucket::Other,
    Bucket::Side(1, true),
    Bucket::Side(2, false),
    Bucket::Side(0, false),
    Bucket::Side(1, false),
];
const OTHER: usize = 2;
/// How far, in blocks, the eye must be behind a bucket's planes before the
/// bucket is left out. The renderer computes both the eye and the section
/// origins in `f32`, with errors far below this.
const MARGIN: f32 = 1.0 / 16.0;

#[derive(Clone, Copy)]
enum Bucket {
    /// Quads in a plane across `axis` facing its positive (`true`) or
    /// negative direction.
    Side(usize, bool),
    Other,
}

/// Whether meshes sorted from now on put every bucket's opaque quads before
/// any cutout ones: set by the renderer when it draws them in two passes.
static GROUPED: AtomicBool = AtomicBool::new(false);

/// Sorts later meshes for two solid passes (`true`) or one.
pub fn group_parts(on: bool) {
    GROUPED.store(on, Ordering::Relaxed);
}

/// The bucket and whether it holds cutout quads of the `n`th part of a
/// mesh, in mesh order.
fn slot(grouped: bool, n: usize) -> (usize, bool) {
    if grouped {
        (n % 7, n >= 7)
    } else {
        (n / 2, n % 2 == 1)
    }
}

/// Where each part of each bucket of a mesh's solid quads ends, and the
/// planes that tell whether a bucket can face the eye.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sides {
    /// The solid quads up to the end of each part of each bucket, in mesh
    /// order (`slot`).
    ends: [u32; 14],
    /// Whether all the opaque parts come first (`group_parts`).
    grouped: bool,
    /// Whether opaque and cutout quads are apart; else either part is every
    /// quad.
    split: bool,
    /// Per bucket facing a positive direction, its lowest plane; facing a
    /// negative one, its highest (unused for the other bucket).
    planes: [f32; 7],
}

impl Sides {
    /// Whether separating opaque/cutout draws preserves this mesh's ordering.
    pub fn can_partition(&self) -> bool {
        self.split
    }
    /// `quads` solid quads in one bucket drawn from everywhere: for meshes
    /// that were not sorted.
    pub fn unsorted(quads: u32) -> Self {
        let mut ends = [0; 14];
        ends[OTHER * 2..].fill(quads);
        Self {
            ends,
            grouped: false,
            split: false,
            planes: [0.0; 7],
        }
    }

    /// The number of solid quads the buckets hold.
    pub fn quads(&self) -> u32 {
        self.ends[13]
    }

    /// Calls `draw(first, count)` for each run of quads an eye at `eye`
    /// (relative to the mesh's origin) can see the front of, nearest the
    /// start first. Empty buckets do not break a run.
    pub fn visible(&self, eye: [f32; 3], draw: impl FnMut(u32, u32)) {
        self.visible_part_from(eye, eye[1], Part::All, draw);
    }

    /// `visible` for every eye from `eye` straight down to the height `low`: the camera and
    /// its water mirror's eye below the plane, which differ only in y. Only the quads of
    /// `part`, but every quad of a mesh whose parts are not split.
    pub fn visible_from(&self, eye: [f32; 3], low: f32, draw: impl FnMut(u32, u32)) {
        self.visible_part_from(eye, low, Part::All, draw);
    }
    pub fn visible_part_from(
        &self,
        eye: [f32; 3],
        low: f32,
        part: Part,
        draw: impl FnMut(u32, u32),
    ) {
        let seen = |i: usize| match BUCKETS[i] {
            Bucket::Side(axis, true) => eye[axis] > self.planes[i] - MARGIN,
            Bucket::Side(1, false) => low < self.planes[i] + MARGIN,
            Bucket::Side(axis, false) => eye[axis] < self.planes[i] + MARGIN,
            Bucket::Other => true,
        };
        self.runs(seen, part, draw);
    }

    /// The runs of `part`'s quads facing every way.
    pub fn every(&self, part: Part, draw: impl FnMut(u32, u32)) {
        self.runs(|_| true, part, draw);
    }

    /// Calls `draw` for each run of `part`'s quads in the buckets `seen` keeps.
    fn runs(&self, seen: impl Fn(usize) -> bool, part: Part, mut draw: impl FnMut(u32, u32)) {
        let mut start = 0;
        let mut run: Option<u32> = None;
        for (n, &end) in self.ends.iter().enumerate() {
            if end == start {
                continue;
            }
            let (bucket, cutout) = slot(self.grouped, n);
            let wanted = match part {
                Part::All => true,
                _ if !self.split => true,
                Part::Opaque => !cutout,
                Part::Cutout => cutout,
            };
            match (wanted && seen(bucket), run) {
                (true, None) => run = Some(start),
                (false, Some(first)) => {
                    draw(first, start - first);
                    run = None;
                }
                _ => {}
            }
            start = end;
        }
        if let Some(first) = run {
            draw(first, start - first);
        }
    }
}

/// The quads `Sides::visible_from` draws: all of them, or those of one of
/// Metal's two solid passes (terrain.wgsl's `opaque_quad`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    All,
    Opaque,
    Cutout,
}

/// The bucket of a quad: the axis plane it lies in and the side its
/// triangles (corners 0, 1, 2 and 0, 2, 3, as the renderer draws them) face.
fn bucket(quad: &[TerrainVertex; 4]) -> usize {
    let p = quad.map(|v| v.pos);
    for axis in 0..3 {
        if p.iter().any(|v| v[axis] != p[0][axis]) {
            continue;
        }
        // The normal's component along `axis`, twice each triangle's area.
        let (u, w) = ((axis + 1) % 3, (axis + 2) % 3);
        let area = |a: usize, b: usize, c: usize| {
            (p[b][u] - p[a][u]) * (p[c][w] - p[a][w]) - (p[b][w] - p[a][w]) * (p[c][u] - p[a][u])
        };
        let (first, second) = (area(0, 1, 2), area(0, 2, 3));
        let positive = first >= 0.0 && second >= 0.0;
        let negative = first <= 0.0 && second <= 0.0;
        if positive == negative {
            // Opposite windings, or no area at all.
            return OTHER;
        }
        return BUCKETS
            .iter()
            .position(|b| matches!(*b, Bucket::Side(a, s) if a == axis && s == positive))
            .expect("every side has a bucket");
    }
    OTHER
}

/// Reorders `solid`'s quads by bucket, opaque ones first in each. Solid
/// cutout quads are also ordered front to back within each axis-facing
/// bucket; stable sorting preserves the mesher's order for coplanar overlays.
pub fn sort(solid: &mut Vec<TerrainVertex>) -> Sides {
    if !GROUPED.load(Ordering::Relaxed) || !partition_safe(solid) {
        return sort_legacy(solid);
    }
    sort_parts(solid, true)
}

// Coplanar overlays need model order. A later opaque quad must not be moved
// before an overlapping cutout. Nonplanar meshes and expensive proofs
// conservatively retain their original ordering and single shader.
fn partition_safe(solid: &[TerrainVertex]) -> bool {
    let quads = solid.as_chunks::<4>().0;
    let opaque = |q: &[TerrainVertex; 4]| q[0].layer & OPAQUE_LAYER != 0;
    if quads.iter().all(opaque) || quads.iter().all(|q| !opaque(q)) {
        return true;
    }
    let mut cutouts = std::collections::BTreeMap::<(usize, u32), Vec<[f32; 4]>>::new();
    let mut sloped_cutouts: Vec<[glam::Vec3; 4]> = Vec::new();
    let mut comparisons = 0;
    for q in quads {
        let b = bucket(q);
        let Bucket::Side(axis, _) = BUCKETS[b] else {
            let points = q.map(|v| glam::Vec3::from(v.pos));
            let normal = (points[1] - points[0]).cross(points[2] - points[0]);
            let tolerance = normal.length() * 1e-5;
            if tolerance <= 1e-12 || normal.dot(points[3] - points[0]).abs() > tolerance {
                return false;
            }
            if !opaque(q) {
                sloped_cutouts.push(points);
            } else {
                for previous in &sloped_cutouts {
                    comparisons += 1;
                    // A non-axis plane cannot overlap an axis plane over an area.
                    // Of two sloped quads, only coplanar overlapping bounds need
                    // the original order. Line intersections are resolved by depth.
                    if comparisons > 4096 {
                        return false;
                    }
                    if previous
                        .iter()
                        .all(|p| normal.dot(*p - points[0]).abs() <= tolerance)
                    {
                        let min_a = previous.iter().copied().reduce(glam::Vec3::min).unwrap();
                        let max_a = previous.iter().copied().reduce(glam::Vec3::max).unwrap();
                        let min_b = points.iter().copied().reduce(glam::Vec3::min).unwrap();
                        let max_b = points.iter().copied().reduce(glam::Vec3::max).unwrap();
                        if min_a.cmple(max_b).all() && min_b.cmple(max_a).all() {
                            return false;
                        }
                    }
                }
            }
            continue;
        };
        let p = q[0].pos[axis];
        let key = (b, if p == 0.0 { 0 } else { p.to_bits() });
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        let lo = |axis: usize| q.iter().map(|v| v.pos[axis]).fold(f32::INFINITY, f32::min);
        let hi = |axis: usize| {
            q.iter()
                .map(|v| v.pos[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        let rect = [lo(u), lo(v), hi(u), hi(v)];
        if !opaque(q) {
            cutouts.entry(key).or_default().push(rect);
        } else if let Some(previous) = cutouts.get(&key) {
            for a in previous {
                comparisons += 1;
                if comparisons > 4096
                    || (a[0] < rect[2] && rect[0] < a[2] && a[1] < rect[3] && rect[1] < a[3])
                {
                    return false;
                }
            }
        }
    }
    true
}

/// `sort`, with all the opaque parts first when `grouped`.
fn sort_parts(solid: &mut Vec<TerrainVertex>, grouped: bool) -> Sides {
    let (quads, rest) = solid.as_chunks::<4>();
    debug_assert!(rest.is_empty(), "solid vertices come in quads");
    let cutout = |q: &[TerrainVertex; 4]| q[0].layer & OPAQUE_LAYER == 0;
    // Each quad's part, by its place in mesh order (`slot`).
    let position = |b: usize, cutout: bool| match grouped {
        true => b + 7 * usize::from(cutout),
        false => b * 2 + usize::from(cutout),
    };
    let slots: Vec<u8> = quads
        .iter()
        .map(|q| position(bucket(q), cutout(q)) as u8)
        .collect();
    let has_cutout = quads.iter().any(cutout);
    let mut counts = [0u32; 7];
    let mut slot_counts = [0u32; 14];
    let mut planes = [0f32; 7];
    for (quad, &part) in quads.iter().zip(&slots) {
        slot_counts[usize::from(part)] += 1;
        let b = slot(grouped, usize::from(part)).0;
        if let Bucket::Side(axis, positive) = BUCKETS[b] {
            let p = quad[0].pos[axis];
            planes[b] = match (counts[b], positive) {
                (0, _) => p,
                (_, true) => planes[b].min(p),
                (_, false) => planes[b].max(p),
            };
        }
        counts[b] += 1;
    }
    let mut slot_ends = [0u32; 14];
    let mut total = 0;
    for (end, count) in slot_ends.iter_mut().zip(slot_counts) {
        total += count;
        *end = total;
    }
    if has_cutout || counts.iter().filter(|&&c| c > 0).count() > 1 {
        let mut next: [u32; 14] = std::array::from_fn(|s| slot_ends[s] - slot_counts[s]);
        let mut sorted = vec![quads[0]; quads.len()];
        for (quad, &part) in quads.iter().zip(&slots) {
            let at = &mut next[usize::from(part)];
            sorted[*at as usize] = *quad;
            *at += 1;
        }
        if has_cutout {
            let mut start = 0;
            for (s, &end) in slot_ends.iter().enumerate() {
                let end = end as usize;
                let slice = &mut sorted[start..end];
                if let Bucket::Side(axis, positive) = BUCKETS[slot(grouped, s).0] {
                    slice.sort_by(|a, b| {
                        let a = a[0].pos[axis];
                        let b = b[0].pos[axis];
                        if positive {
                            b.total_cmp(&a)
                        } else {
                            a.total_cmp(&b)
                        }
                    });
                }
                start = end;
            }
        }
        *solid = sorted.into_flattened();
    }
    Sides {
        ends: slot_ends,
        grouped,
        split: true,
        planes,
    }
}

fn sort_legacy(solid: &mut Vec<TerrainVertex>) -> Sides {
    let (quads, rest) = solid.as_chunks::<4>();
    debug_assert!(rest.is_empty(), "solid vertices come in quads");
    let buckets: Vec<u8> = quads.iter().map(|q| bucket(q) as u8).collect();
    let has_cutout = quads.iter().any(|q| q[0].layer & OPAQUE_LAYER == 0);
    let mut counts = [0u32; 7];
    let mut planes = [0f32; 7];
    for (quad, &b) in quads.iter().zip(&buckets) {
        let b = usize::from(b);
        if let Bucket::Side(axis, positive) = BUCKETS[b] {
            let p = quad[0].pos[axis];
            planes[b] = match (counts[b], positive) {
                (0, _) => p,
                (_, true) => planes[b].min(p),
                (_, false) => planes[b].max(p),
            };
        }
        counts[b] += 1;
    }
    let mut ends = [0u32; 7];
    let mut total = 0;
    for (end, count) in ends.iter_mut().zip(counts) {
        total += count;
        *end = total;
    }
    if has_cutout || counts.iter().filter(|&&c| c > 0).count() > 1 {
        let mut next: [u32; 7] = std::array::from_fn(|b| ends[b] - counts[b]);
        let mut sorted = vec![quads[0]; quads.len()];
        for (quad, &b) in quads.iter().zip(&buckets) {
            let slot = &mut next[usize::from(b)];
            sorted[*slot as usize] = *quad;
            *slot += 1;
        }
        if has_cutout {
            let mut start = 0;
            for (i, bucket) in BUCKETS.iter().enumerate() {
                let end = ends[i] as usize;
                let slice = &mut sorted[start..end];
                if let Bucket::Side(axis, positive) = *bucket {
                    slice.sort_by(|a, b| {
                        let a = a[0].pos[axis];
                        let b = b[0].pos[axis];
                        if positive {
                            b.total_cmp(&a)
                        } else {
                            a.total_cmp(&b)
                        }
                    });
                }
                start = end;
            }
        }
        *solid = sorted.into_flattened();
    }
    Sides {
        ends: std::array::from_fn(|i| ends[i / 2]),
        grouped: false,
        split: false,
        planes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coplanar_overlay_order_requires_the_original_path_only_when_it_changes() {
        let face = |layer| {
            quad(
                [
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [1.0, 1.0, 1.0],
                    [1.0, 0.0, 1.0],
                ],
                layer,
            )
        };
        let opaque_then_cutout = [face(OPAQUE_LAYER | 1), face(2)].concat();
        assert!(partition_safe(&opaque_then_cutout));
        let mut cutout_then_opaque = [face(2), face(OPAQUE_LAYER | 1)].concat();
        assert!(!partition_safe(&cutout_then_opaque));
        let sides = sort_legacy(&mut cutout_then_opaque);
        assert!(!sides.can_partition());
        assert_eq!(cutout_then_opaque[0].layer, 2);
        assert_eq!(cutout_then_opaque[4].layer, OPAQUE_LAYER | 1);

        let mut separate = face(OPAQUE_LAYER | 1);
        for vertex in &mut separate {
            vertex.pos[1] += 2.0;
        }
        assert!(partition_safe(&[face(2), separate].concat()));
        let diagonal = quad(
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
            3,
        );
        assert!(partition_safe(&[diagonal, face(OPAQUE_LAYER | 1)].concat()));
        let mut diagonal_opaque = diagonal;
        for vertex in &mut diagonal_opaque {
            vertex.layer = OPAQUE_LAYER | 4;
        }
        assert!(!partition_safe(&[diagonal, diagonal_opaque].concat()));
    }

    fn quad(corners: [[f32; 3]; 4], layer: u16) -> [TerrainVertex; 4] {
        corners.map(|p| TerrainVertex::new(p, [0.0; 2], layer, 0, 0))
    }

    /// A unit block's six faces at `origin`, wound counter-clockwise seen
    /// from outside, as block models are.
    fn cube(origin: [f32; 3], layer: u16) -> Vec<TerrainVertex> {
        let [x, y, z] = origin;
        let (a, b, c) = (x + 1.0, y + 1.0, z + 1.0);
        [
            [[a, y, z], [a, b, z], [a, b, c], [a, y, c]], // +X
            [[x, y, z], [x, y, c], [x, b, c], [x, b, z]], // -X
            [[x, b, z], [x, b, c], [a, b, c], [a, b, z]], // +Y
            [[x, y, z], [a, y, z], [a, y, c], [x, y, c]], // -Y
            [[x, y, c], [a, y, c], [a, b, c], [x, b, c]], // +Z
            [[x, y, z], [x, b, z], [a, b, z], [a, y, z]], // -Z
        ]
        .into_iter()
        .flat_map(|q| quad(q, layer))
        .collect()
    }

    /// Where each bucket ends, in a mesh sorted for one pass.
    fn bucket_ends(sides: &Sides) -> [u32; 7] {
        assert!(!sides.grouped);
        std::array::from_fn(|b| sides.ends[b * 2 + 1])
    }

    fn runs(sides: &Sides, eye: [f32; 3]) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        sides.visible(eye, |first, count| out.push((first, count)));
        out
    }

    /// The quads of `solid` whose triangles face `eye`, by their layer.
    fn front_facing(solid: &[TerrainVertex], eye: [f32; 3]) -> Vec<u16> {
        let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        solid
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|q| {
                [[0, 1, 2], [0, 2, 3]].iter().any(|t| {
                    let (e1, e2) = (sub(q[t[1]].pos, q[t[0]].pos), sub(q[t[2]].pos, q[t[0]].pos));
                    let n = [
                        e1[1] * e2[2] - e1[2] * e2[1],
                        e1[2] * e2[0] - e1[0] * e2[2],
                        e1[0] * e2[1] - e1[1] * e2[0],
                    ];
                    let to_eye = sub(eye, q[t[0]].pos);
                    n[0] * to_eye[0] + n[1] * to_eye[1] + n[2] * to_eye[2] > 0.0
                })
            })
            .map(|q| q[0].layer)
            .collect()
    }

    #[test]
    fn cube_faces_go_to_their_buckets_in_order() {
        let mut solid = cube([3.0, 4.0, 5.0], 0);
        for (i, v) in solid.iter_mut().enumerate() {
            v.layer = (i / 4) as u16;
        }
        let sides = sort(&mut solid);
        assert_eq!(bucket_ends(&sides), [1, 2, 2, 3, 4, 5, 6]);
        let layers: Vec<u16> = solid.iter().step_by(4).map(|v| v.layer).collect();
        // +X, +Z, +Y, -Z, -X, -Y.
        assert_eq!(layers, [0, 4, 2, 5, 1, 3]);
        assert_eq!(sides.planes[0], 4.0); // +X at x = 4
        assert_eq!(sides.planes[5], 3.0); // -X at x = 3
        assert_eq!(sides.planes[3], 5.0); // +Y at y = 5
    }

    #[test]
    fn the_drawn_runs_hold_every_quad_that_faces_the_eye() {
        // Blocks across a section, with a sloped quad and a bowtie.
        let mut solid = Vec::new();
        let mut layer = 0;
        for origin in [
            [0.0, 0.0, 0.0],
            [7.0, 3.0, 12.0],
            [15.0, 15.0, 15.0],
            [2.0, 9.0, 4.0],
        ] {
            for q in cube(origin, 0).chunks(4) {
                solid.extend(q.iter().map(|v| TerrainVertex { layer, ..*v }));
                layer += 1;
            }
        }
        let slope = [
            [1.0, 1.0, 1.0],
            [2.0, 1.0, 1.0],
            [2.0, 2.0, 2.0],
            [1.0, 2.0, 2.0],
        ];
        let bowtie = [
            [5.0, 1.0, 1.0],
            [5.0, 2.0, 2.0],
            [5.0, 1.0, 2.0],
            [5.0, 2.0, 1.0],
        ];
        for corners in [slope, bowtie] {
            solid.extend(quad(corners, layer));
            layer += 1;
        }
        let sides = sort(&mut solid);
        assert_eq!(sides.quads(), u32::from(layer));
        let eyes = [-20.0f32, -0.5, 0.0, 3.5, 8.0, 15.99, 16.0, 30.0];
        for &x in &eyes {
            for &y in &eyes {
                for &z in &eyes {
                    let eye = [x, y, z];
                    let mut drawn = Vec::new();
                    for (first, count) in runs(&sides, eye) {
                        drawn.extend(
                            solid[first as usize * 4..(first + count) as usize * 4]
                                .iter()
                                .step_by(4)
                                .map(|v| v.layer),
                        );
                    }
                    for layer in front_facing(&solid, eye) {
                        assert!(drawn.contains(&layer), "{eye:?} misses quad {layer}");
                    }
                }
            }
        }
    }

    #[test]
    fn runs_merge_across_hidden_and_empty_buckets() {
        let mut solid = cube([0.0; 3], 0);
        let sides = sort(&mut solid);
        // From above, +X and +Z: +X, +Z, (no other), +Y is one run.
        assert_eq!(runs(&sides, [8.0, 8.0, 8.0]), [(0, 3)]);
        // From below, -X and -Z: -Z, -X, -Y.
        assert_eq!(runs(&sides, [-8.0, -8.0, -8.0]), [(3, 3)]);
        // From above, -X and +Z: +Z, +Y, then -X.
        assert_eq!(runs(&sides, [-8.0, 8.0, 8.0]), [(1, 2), (4, 1)]);
        // Inside the block every face is seen from behind.
        assert!(runs(&sides, [0.5; 3]).is_empty());
        // Level with its top, the eye may see it: the margin keeps it.
        assert_eq!(runs(&sides, [0.5, 1.0, 0.5]), [(2, 1)]);
    }

    #[test]
    fn an_eye_and_its_mirror_below_see_the_faces_of_both() {
        let mut solid = cube([0.0; 3], 0);
        let sides = sort(&mut solid);
        let both = |eye: [f32; 3], low: f32| {
            let mut out = Vec::new();
            sides.visible_part_from(eye, low, Part::All, |first, count| out.push((first, count)));
            out
        };
        // Above the block: +X, +Z and +Y; with the mirror's eye below it, -Y too.
        assert_eq!(runs(&sides, [8.0, 8.0, 8.0]), [(0, 3)]);
        assert_eq!(both([8.0, 8.0, 8.0], -8.0), [(0, 3), (5, 1)]);
        // A mirror above the block's bottom adds nothing.
        assert_eq!(both([8.0, 8.0, 8.0], 0.5), [(0, 3)]);
        // The sides depend on x and z alone.
        assert_eq!(both([-8.0, 8.0, -8.0], -8.0), [(2, 4)]);
    }

    /// Opaque and cutout quads of each bucket split: each pass's runs hold
    /// exactly its own quads facing the eye, and all of them together the
    /// same runs as one pass.
    #[test]
    fn each_part_draws_its_own_quads_facing_the_eye() {
        // Two blocks: one opaque, one cutout, then an opaque one again, so
        // every bucket holds both kinds out of order.
        let mut solid = Vec::new();
        let mut id = 0u16;
        for (origin, opaque) in [
            ([0.0, 0.0, 0.0], true),
            ([4.0, 2.0, 6.0], false),
            ([9.0, 5.0, 3.0], true),
        ] {
            for q in cube(origin, 0).chunks(4) {
                let layer = id | if opaque { OPAQUE_LAYER } else { 0 };
                solid.extend(q.iter().map(|v| TerrainVertex { layer, ..*v }));
                id += 1;
            }
        }
        let unsorted = solid;
        for grouped in [false, true] {
            let mut solid = unsorted.clone();
            let sides = sort_parts(&mut solid, grouped);
            let layers: Vec<u16> = solid
                .as_chunks::<4>()
                .0
                .iter()
                .map(|q| q[0].layer)
                .collect();
            for eye in [[20.0, 30.0, -10.0], [-5.0, 1.0, 4.0], [5.0, 3.0, 7.0]] {
                let quads = |part| {
                    let mut out = Vec::new();
                    sides.visible_part_from(eye, eye[1], part, |first, count| {
                        out.extend(
                            layers[first as usize..(first + count) as usize]
                                .iter()
                                .copied(),
                        )
                    });
                    out.sort_unstable();
                    out
                };
                let mut facing = front_facing(&solid, eye);
                facing.sort_unstable();
                let opaque = quads(Part::Opaque);
                let cutout = quads(Part::Cutout);
                assert!(opaque.iter().all(|l| l & OPAQUE_LAYER != 0), "{eye:?}");
                assert!(cutout.iter().all(|l| l & OPAQUE_LAYER == 0), "{eye:?}");
                for l in &facing {
                    let mine = if l & OPAQUE_LAYER != 0 {
                        &opaque
                    } else {
                        &cutout
                    };
                    assert!(
                        mine.contains(l),
                        "{eye:?}: quad {l} facing the eye left out"
                    );
                }
                let mut both = [opaque, cutout].concat();
                both.sort_unstable();
                assert_eq!(both, quads(Part::All), "{eye:?}");
            }
            // Every way: each part once, whole.
            let mut every = Vec::new();
            sides.every(Part::Opaque, |first, count| every.push((first, count)));
            sides.every(Part::Cutout, |first, count| every.push((first, count)));
            let total: u32 = every.iter().map(|r| r.1).sum();
            assert_eq!(total, sides.quads());

            // Two passes: from above a corner, each part is one or two runs.
            if grouped {
                let mut count = 0;
                sides.visible_part_from([20.0, 30.0, -10.0], 30.0, Part::Opaque, |_, _| count += 1);
                assert_eq!(count, 2);
            }
        }
    }

    /// A mesh that was not sorted has no parts: each pass draws all of it
    /// (the shader leaves out the other pass's quads).
    #[test]
    fn unsorted_meshes_draw_everything_in_each_part() {
        let sides = Sides::unsorted(9);
        for part in [Part::Opaque, Part::Cutout] {
            let mut out = Vec::new();
            sides.every(part, |first, count| out.push((first, count)));
            assert_eq!(out, [(0, 9)]);
        }
    }

    #[test]
    fn unsorted_meshes_draw_everything() {
        let sides = Sides::unsorted(9);
        assert_eq!(sides.quads(), 9);
        assert_eq!(runs(&sides, [-100.0; 3]), [(0, 9)]);
        assert!(runs(&Sides::unsorted(0), [0.0; 3]).is_empty());
    }

    #[test]
    fn one_bucket_keeps_the_mesh_as_it_is() {
        let mut solid: Vec<_> = (0..3)
            .flat_map(|i| {
                quad(
                    [
                        [1.0, 0.0, 0.0],
                        [1.0, 1.0, 0.0],
                        [1.0, 1.0, 1.0],
                        [1.0, 0.0, 1.0],
                    ],
                    i,
                )
            })
            .collect();
        let before = solid.clone();
        let sides = sort(&mut solid);
        assert_eq!(bucket_ends(&sides), [3; 7]);
        assert_eq!(
            solid.iter().map(|v| v.layer).collect::<Vec<_>>(),
            before.iter().map(|v| v.layer).collect::<Vec<_>>()
        );
    }

    #[test]
    fn front_to_back_sort_orders_positive_faces_and_keeps_ties_stable() {
        let mut solid: Vec<_> = [(1.0, 10), (3.0, 30), (3.0, 31)]
            .into_iter()
            .flat_map(|(x, layer)| {
                quad(
                    [[x, 0.0, 0.0], [x, 1.0, 0.0], [x, 1.0, 1.0], [x, 0.0, 1.0]],
                    layer,
                )
            })
            .collect();
        sort(&mut solid);
        let layers: Vec<u16> = solid
            .as_chunks::<4>()
            .0
            .iter()
            .map(|q| q[0].layer)
            .collect();
        assert_eq!(layers, [30, 31, 10]);
    }

    #[test]
    fn front_to_back_sort_orders_negative_faces_toward_the_eye() {
        let mut solid: Vec<_> = [(3.0, 30), (1.0, 10), (2.0, 20)]
            .into_iter()
            .flat_map(|(x, layer)| {
                quad(
                    [[x, 0.0, 0.0], [x, 0.0, 1.0], [x, 1.0, 1.0], [x, 1.0, 0.0]],
                    layer,
                )
            })
            .collect();
        sort(&mut solid);
        let layers: Vec<u16> = solid
            .as_chunks::<4>()
            .0
            .iter()
            .map(|q| q[0].layer)
            .collect();
        assert_eq!(layers, [10, 20, 30]);
    }
}
