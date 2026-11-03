//! Kinetic networks: rotation passed from block to block, as Create's (first written from its
//! documented behaviour, its Ponder scenes and wiki; its code is MIT, and the machines are ported
//! from it, licenses/create-MIT.txt): shafts along their axis,
//! cogwheels meshing with the cogwheels beside them, gearboxes around a corner, and the sources
//! that turn them (motors, cranks). Content mods say which of their blocks are what
//! (`kinetic` in a block's declaration, gathered in the root's kinetic.json).
//!
//! A part's speed is one number, in RPM, solved from the block states alone (and the cranks
//! being turned). A shaft-like part's (shaft, cogwheels) is about the positive direction of its
//! axis; a gearbox's, a motor's and a crank's is about each face outwards. Two parts joined end
//! to end turn their shared shaft alike; meshed cogwheels move alike where they touch: a small
//! cogwheel turns the one beside it the other way, a large one half as fast (the small one
//! twice), two large ones at right angles as fast.
use crate::Result;
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;

pub use horizon_mods::content::{Kinetic, KineticPart, Machine};

/// The fastest a part turns (Create's default speed limit).
pub const MAX_RPM: f32 = 256.0;
/// Degrees a part at 1 RPM turns in a tick (360 degrees a minute, 20 ticks a second).
pub const DEGREES_PER_TICK: f32 = 0.3;

/// A kinetic state: its part and how its state turns it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub kind: KineticPart,
    /// The axis it turns about (0 x, 1 y, 2 z): a shaft's, a gearbox's, a motor's or crank's
    /// facing's.
    pub axis: usize,
    /// A motor's facing; a crank's back (the block it turns). Zero for the others.
    pub face: [i32; 3],
    /// A source's speed: a motor's, a crank's while turned.
    pub rpm: f32,
    pub spins: bool,
    pub half: Option<String>,
    pub extra: Option<String>,
    /// The quarter turns that point `extra` at the state's facing, when it is made pointing
    /// elsewhere than the block's own model (None: the blockstate's turns).
    pub extra_turns: Option<[u8; 3]>,
    /// The models a machine moves down as it works: a press's head, a mixer's head and pole.
    pub head: Option<String>,
    pub pole: Option<String>,
    /// Its state's `powered`: a clutch's break, a gearshift's turn the other way.
    pub powered: bool,
    /// A belt segment with a pulley (its start, end or a shaft along it), which joins the shafts
    /// at its sides; a middle one joins nothing but its belt.
    pub pulley: bool,
    /// More models the game draws for it (a belt's, game/belts.rs).
    pub models: Vec<String>,
    /// What it takes from its network and, a source, gives, per RPM ([`Kinetic::stress`]).
    pub stress: f32,
    pub capacity: f32,
}

impl Part {
    /// A declared part on a state with these `axis` and `facing` properties; None when the
    /// state has not the one its part needs. A shaft-like part with no `axis` turns about its
    /// facing's (a water wheel).
    pub fn new(kinetic: &Kinetic, axis: Option<&str>, facing: Option<&str>) -> Option<Self> {
        let by_name = |a: &str| match a {
            "x" => Some(0),
            "y" => Some(1),
            "z" => Some(2),
            _ => None,
        };
        let (axis, face) = match kinetic.part {
            KineticPart::Motor
            | KineticPart::Crank
            | KineticPart::Drill
            | KineticPart::Fan
            | KineticPart::Bearing => {
                let facing = direction(facing?)?;
                let face = if kinetic.part == KineticPart::Motor {
                    facing
                } else {
                    facing.map(|v| -v)
                };
                (axis_of(face), face)
            }
            // SawBlock: sideways, a drill's back; up or down, across it (set where it loads, by
            // its `axis_along_first`).
            KineticPart::Saw => {
                let facing = direction(facing?)?;
                if facing[1] == 0 {
                    let face = facing.map(|v| -v);
                    (axis_of(face), face)
                } else {
                    (2, [0; 3])
                }
            }
            // Cogwheels about y whatever their state says.
            KineticPart::Millstone | KineticPart::Mixer => (1, [0; 3]),
            // ArmBlock: about y, its shaft below (above, on a ceiling: set where it loads).
            KineticPart::Arm => (1, [0, -1, 0]),
            // A belt's pulley across it, level (a sideways belt's is set upright where it loads).
            KineticPart::Belt => (
                if axis_of(direction(facing?)?) == 0 {
                    2
                } else {
                    0
                },
                [0; 3],
            ),
            _ => match axis {
                Some(axis) => (by_name(axis)?, [0; 3]),
                None => (axis_of(direction(facing?)?), [0; 3]),
            },
        };
        let extra_turns = match (kinetic.extra_facing.as_deref(), facing) {
            (Some(made), Some(facing)) => Some(turns_from(direction(made)?, direction(facing)?)),
            _ => None,
        };
        Some(Part {
            kind: kinetic.part,
            axis,
            face,
            rpm: kinetic.rpm.clamp(-MAX_RPM, MAX_RPM),
            spins: kinetic.spins,
            half: kinetic.half.clone(),
            extra: kinetic.extra.clone(),
            extra_turns,
            head: kinetic.head.clone(),
            pole: kinetic.pole.clone(),
            powered: false,
            pulley: true,
            models: kinetic.models.clone(),
            stress: kinetic.stress.max(0.0),
            capacity: kinetic.capacity.max(0.0),
        })
    }
    /// A cogwheel's teeth (Create's models: 8 and 16, one along each axis at no turn).
    fn teeth(&self) -> f32 {
        match self.kind {
            KineticPart::LargeCogwheel => 16.0,
            _ => 8.0,
        }
    }
    /// A cogwheel's radius in blocks; 0 for parts that do not mesh.
    pub fn radius(&self) -> f32 {
        match self.kind {
            KineticPart::Cogwheel
            | KineticPart::Millstone
            | KineticPart::Mixer
            | KineticPart::Arm => 0.5,
            KineticPart::LargeCogwheel => 1.0,
            _ => 0.0,
        }
    }
    /// The faces it joins through and how its speed turns there (1: as its own, about its
    /// axis; ±1 by the face's sign: about the face outwards).
    pub fn ends(&self) -> Vec<([i32; 3], f32)> {
        let unit = |axis: usize, sign: i32| {
            let mut v = [0; 3];
            v[axis] = sign;
            v
        };
        match self.kind {
            KineticPart::Shaft
            | KineticPart::Cogwheel
            | KineticPart::LargeCogwheel
            | KineticPart::CrushingWheel
            | KineticPart::Press
            | KineticPart::WaterWheel => {
                vec![(unit(self.axis, 1), 1.0), (unit(self.axis, -1), 1.0)]
            }
            // ClutchBlockEntity: powered, nothing passes; GearshiftBlockEntity: powered, the far
            // side turns the other way.
            KineticPart::Clutch if self.powered => Vec::new(),
            KineticPart::Clutch => vec![(unit(self.axis, 1), 1.0), (unit(self.axis, -1), 1.0)],
            KineticPart::Gearshift => {
                let far = if self.powered { -1.0 } else { 1.0 };
                vec![(unit(self.axis, 1), 1.0), (unit(self.axis, -1), far)]
            }
            KineticPart::Belt if self.pulley => {
                vec![(unit(self.axis, 1), 1.0), (unit(self.axis, -1), 1.0)]
            }
            KineticPart::Belt => Vec::new(),
            // MillstoneBlock.hasShaftTowards: down only; the mixer's none (both mesh).
            KineticPart::Millstone => vec![([0, -1, 0], 1.0)],
            KineticPart::Mixer => Vec::new(),
            KineticPart::Arm => vec![(self.face, 1.0)],
            KineticPart::Gearbox => (0..3)
                .filter(|a| *a != self.axis)
                .flat_map(|a| [(unit(a, 1), 1.0), (unit(a, -1), -1.0)])
                .collect(),
            KineticPart::Motor
            | KineticPart::Crank
            | KineticPart::Drill
            | KineticPart::Fan
            | KineticPart::Bearing => vec![(self.face, self.face[self.axis] as f32)],
            KineticPart::Saw if self.face != [0; 3] => {
                vec![(self.face, self.face[self.axis] as f32)]
            }
            KineticPart::Saw => vec![(unit(self.axis, 1), 1.0), (unit(self.axis, -1), 1.0)],
        }
    }
    /// Whether it is a source: a motor always, a crank while turned.
    pub fn source(&self) -> bool {
        self.kind == KineticPart::Motor
    }
}

pub use horizon_mods::content::{direction, turns_from, turns_to};

fn axis_of(v: [i32; 3]) -> usize {
    v.iter().position(|c| *c != 0).unwrap_or(1)
}

/// Every state's part and machine, from a content root's kinetic.json and machines.json; empty
/// without them.
#[derive(Default)]
pub struct KineticTable {
    parts: Vec<Option<Arc<Part>>>,
    machines: Vec<Option<Machine>>,
    /// The models the game draws for blocks besides their own, by block (block-models.json).
    models: HashMap<String, Vec<String>>,
}

impl KineticTable {
    pub fn load(root: &Path, registry: &BlockRegistry) -> Result<Self> {
        let read = |name: &str| std::fs::read(root.join(name)).ok();
        let mut out = Self::default();
        if let Some(bytes) = read(crate::mod_content::KINETIC) {
            let declared: HashMap<String, Kinetic> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("{}: {e}", crate::mod_content::KINETIC))?;
            out.parts = vec![None; registry.len()];
            for (i, slot) in out.parts.iter_mut().enumerate() {
                let Some(state) = registry.state(BlockStateId(i as u32)) else {
                    continue;
                };
                let Some(kinetic) = declared.get(&state.name) else {
                    continue;
                };
                let prop = |k: &str| state.properties.get(k).map(String::as_str);
                *slot = Part::new(kinetic, prop("axis"), prop("facing")).map(|mut part| {
                    part.powered = prop("powered") == Some("true");
                    if part.kind == KineticPart::Belt {
                        part.pulley = prop("part") != Some("middle");
                        if prop("slope") == Some("sideways") {
                            part.axis = 1;
                        }
                    }
                    if part.kind == KineticPart::Arm && prop("ceiling") == Some("true") {
                        part.face = [0, 1, 0];
                    }
                    // DirectionalAxisKineticBlock.getRotationAxis, facing up or down.
                    if part.kind == KineticPart::Saw && part.face == [0; 3] {
                        part.axis = if prop("axis_along_first") == Some("true") {
                            0
                        } else {
                            2
                        };
                    }
                    Arc::new(part)
                });
            }
        }
        if let Some(bytes) = read(crate::mod_content::BLOCK_MODELS) {
            out.models = serde_json::from_slice(&bytes)
                .map_err(|e| format!("{}: {e}", crate::mod_content::BLOCK_MODELS))?;
        }
        if let Some(bytes) = read(crate::mod_content::MACHINES) {
            let declared: HashMap<String, Machine> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("{}: {e}", crate::mod_content::MACHINES))?;
            out.machines = (0..registry.len())
                .map(|i| {
                    let state = registry.state(BlockStateId(i as u32))?;
                    declared.get(&state.name).copied()
                })
                .collect();
        }
        Ok(out)
    }
    pub fn is_empty(&self) -> bool {
        self.parts.iter().all(Option::is_none) && self.machines.iter().all(Option::is_none)
    }
    /// The models the game draws for `block` besides its own, in its declared order.
    pub fn block_models(&self, block: &str) -> &[String] {
        self.models.get(block).map_or(&[], Vec::as_slice)
    }
    pub fn get(&self, state: BlockStateId) -> Option<&Arc<Part>> {
        self.parts.get(state.0 as usize)?.as_ref()
    }
    /// The machine a state is, when it is one that is no kinetic part (a depot, a basin).
    pub fn machine(&self, state: BlockStateId) -> Option<Machine> {
        *self.machines.get(state.0 as usize)?
    }
    /// Whether a state keeps a block entity of its machine's (its items, its work): a declared
    /// machine, or a millstone, a press, a mixer or a saw.
    pub fn keeps_entity(&self, state: BlockStateId) -> bool {
        self.machine(state).is_some()
            || self.get(state).is_some_and(|p| {
                matches!(
                    p.kind,
                    KineticPart::Millstone
                        | KineticPart::Press
                        | KineticPart::Mixer
                        | KineticPart::Saw
                        | KineticPart::Bearing
                        | KineticPart::Arm
                )
            })
    }
    /// The states that keep a block entity ([`KineticTable::keeps_entity`]).
    pub fn entity_states(&self) -> impl Iterator<Item = BlockStateId> + '_ {
        (0..self.parts.len().max(self.machines.len()) as u32)
            .map(BlockStateId)
            .filter(|s| self.keeps_entity(*s))
    }
}

/// `WaterWheelBlockEntity.determineAndApplyFlowScore` and `getGeneratedSpeed` for a water wheel
/// at `pos` turning about `axis`: each fluid beside it in its plane flowing round it one way or
/// the other (more than half along its rim) counts for that way, and the wheel turns 8 RPM
/// (about its axis's positive way) the way most of them push, or not at all. `flow` is the
/// fluid's flow at a block (`FluidState.getFlow`).
pub fn water_wheel_speed(axis: usize, pos: [i32; 3], flow: impl Fn([i32; 3]) -> [f64; 3]) -> f32 {
    let mut score = 0i32;
    for d in [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, 1],
        [0, 0, -1],
    ] {
        if d[axis] != 0 {
            continue;
        }
        let at = [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]];
        let mut f = flow(at);
        f[axis] = 0.0;
        let length = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
        if length == 0.0 {
            continue;
        }
        let f = f.map(|v| v / length);
        // VecHelper.rotate(normal, 90, axis): the rim's positive way at that side.
        let n = d.map(f64::from);
        let motion = match axis {
            0 => [n[0], -n[2], n[1]],
            1 => [n[2], n[1], -n[0]],
            _ => [-n[1], n[0], n[2]],
        };
        let dot = f[0] * motion[0] + f[1] * motion[1] + f[2] * motion[2];
        if dot.abs() > 0.5 {
            score += dot.signum() as i32;
        }
    }
    score.clamp(-1, 1) as f32 * 8.0
}

/// The water wheels' speeds among `parts`, by [`water_wheel_speed`]: sources for [`solve`].
pub fn water_wheels(
    parts: &HashMap<[i32; 3], Arc<Part>>,
    flow: impl Fn([i32; 3]) -> [f64; 3],
) -> HashMap<[i32; 3], f32> {
    parts
        .iter()
        .filter(|(_, p)| p.kind == KineticPart::WaterWheel)
        .map(|(pos, p)| (*pos, water_wheel_speed(p.axis, *pos, &flow)))
        .filter(|(_, s)| *s != 0.0)
        .collect()
}

/// The parts `p` joins (by end or mesh) and the factor from its speed to theirs.
fn links(parts: &HashMap<[i32; 3], Arc<Part>>, p: [i32; 3], part: &Part) -> Vec<([i32; 3], f32)> {
    let at = |d: [i32; 3]| [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
    let mut out = Vec::new();
    for (face, k) in part.ends() {
        let q = at(face);
        let Some(other) = parts.get(&q) else {
            continue;
        };
        let back = face.map(|v| -v);
        if let Some((_, k2)) = other.ends().into_iter().find(|(f, _)| *f == back) {
            // The shaft between them turns alike from both sides.
            out.push((q, k * k2));
        }
    }
    let r = part.radius();
    if r == 0.0 {
        return out;
    }
    let a = part.axis;
    let (b, c) = ((a + 1) % 3, (a + 2) % 3);
    let offset = |pairs: [(usize, i32); 2]| {
        let mut d = [0; 3];
        for (axis, v) in pairs {
            d[axis] = v;
        }
        d
    };
    // In its plane: beside it (small ones mesh) and diagonally (small with large).
    for (u, v) in [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ] {
        let q = at(offset([(b, u), (c, v)]));
        let Some(other) = parts.get(&q) else {
            continue;
        };
        if other.axis != a || other.radius() == 0.0 {
            continue;
        }
        let diagonal = u != 0 && v != 0;
        let meshes = if diagonal {
            r != other.radius()
        } else {
            r == 0.5 && other.radius() == 0.5
        };
        if meshes {
            // Alike where they touch: the other way, slower by the radii.
            out.push((q, -r / other.radius()));
        }
    }
    // Large ones at right angles, on two faces of one block: d is one step along each axis.
    if r == 1.0 {
        for other_axis in [b, c] {
            for (alpha, beta) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let q = at(offset([(a, alpha), (other_axis, beta)]));
                let Some(other) = parts.get(&q) else {
                    continue;
                };
                if other.axis == other_axis && other.radius() == 1.0 {
                    out.push((q, (alpha * beta) as f32));
                }
            }
        }
    }
    out
}

/// Every part's speed, from the sources: the motors, and `turned` (cranks being turned, at
/// their own speed, outwards). Each network takes its fastest source's speed; a slower one
/// turning it the same way is overpowered. A network its sources (or a loop of gears) would
/// turn two ways at once stands still, and so does one taking more stress than its sources
/// give (Create's KineticNetwork: each part's stress times its speed, against each source's
/// capacity times its own). Parts no source reaches are not listed.
pub fn solve(
    parts: &HashMap<[i32; 3], Arc<Part>>,
    turned: &HashMap<[i32; 3], f32>,
) -> HashMap<[i32; 3], f32> {
    solve_linked(parts, turned, &[])
}

/// [`solve`], with `groups` of parts that turn alike wherever they are (each belt's segments:
/// its belt joins its pulleys).
pub fn solve_linked(
    parts: &HashMap<[i32; 3], Arc<Part>>,
    turned: &HashMap<[i32; 3], f32>,
    groups: &[Vec<[i32; 3]>],
) -> HashMap<[i32; 3], f32> {
    let mut group_of: HashMap<[i32; 3], usize> = HashMap::new();
    for (i, g) in groups.iter().enumerate() {
        for p in g {
            group_of.insert(*p, i);
        }
    }
    let mut sources: Vec<([i32; 3], f32)> = parts
        .iter()
        .filter(|(_, part)| part.source())
        .map(|(p, part)| (*p, part.rpm))
        .chain(
            turned
                .iter()
                .filter(|(p, _)| parts.contains_key(*p))
                .map(|(p, v)| (*p, *v)),
        )
        .filter(|(_, v)| *v != 0.0)
        .collect();
    // The fastest first; ties in position order, so the outcome does not hang on map order.
    sources.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
    let mut speed: HashMap<[i32; 3], (f32, usize)> = HashMap::new();
    let mut stuck: Vec<bool> = Vec::new();
    // Each network's stress capacity: its sources' (calculateCapacity).
    let mut capacity: Vec<f32> = Vec::new();
    // Parts held at the limit: their gears no longer match, which jams nothing.
    let mut capped: std::collections::HashSet<[i32; 3]> = std::collections::HashSet::new();
    for (start, value) in sources {
        let gives = parts[&start].capacity * value.abs();
        if let Some((s, net)) = speed.get(&start) {
            // Reached from a faster source: the other way jams the network.
            if s * value < 0.0 {
                stuck[*net] = true;
            } else {
                capacity[*net] += gives;
            }
            continue;
        }
        let net = stuck.len();
        stuck.push(false);
        capacity.push(gives);
        speed.insert(start, (value.clamp(-MAX_RPM, MAX_RPM), net));
        let mut queue = VecDeque::from([start]);
        while let Some(p) = queue.pop_front() {
            let (s, _) = speed[&p];
            let mut joined = links(parts, p, &parts[&p]);
            if let Some(g) = group_of.get(&p) {
                joined.extend(
                    groups[*g]
                        .iter()
                        .filter(|q| **q != p && parts.contains_key(*q))
                        .map(|q| (*q, 1.0)),
                );
            }
            for (q, factor) in joined {
                let raw = s * factor;
                let want = raw.clamp(-MAX_RPM, MAX_RPM);
                match speed.get(&q) {
                    None => {
                        if want != raw {
                            capped.insert(q);
                        }
                        speed.insert(q, (want, net));
                        queue.push_back(q);
                    }
                    Some((had, _))
                        if (had - want).abs() > 1e-3
                            && !capped.contains(&p)
                            && !capped.contains(&q) =>
                    {
                        stuck[net] = true
                    }
                    Some(_) => {}
                }
            }
        }
    }
    // calculateStress: what every part takes at its speed; more than the capacity overstresses.
    let mut stress = vec![0.0f32; stuck.len()];
    for (p, (s, net)) in &speed {
        stress[*net] += parts[p].stress * s.abs();
    }
    for (net, jammed) in stuck.iter_mut().enumerate() {
        *jammed |= stress[net] > capacity[net];
    }
    speed
        .into_iter()
        .map(|(p, (s, net))| (p, if stuck[net] { 0.0 } else { s }))
        .collect()
}

/// The stress the network `start` is in takes and the capacity its sources give, in stress units
/// (Create's KineticNetwork, as [`solve_linked`] weighs it), from the speeds it solved.
pub fn network_stress(
    parts: &HashMap<[i32; 3], Arc<Part>>,
    turned: &HashMap<[i32; 3], f32>,
    groups: &[Vec<[i32; 3]>],
    speeds: &HashMap<[i32; 3], f32>,
    start: [i32; 3],
) -> Option<(f32, f32)> {
    parts.get(&start)?;
    let mut seen = std::collections::HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let (mut stress, mut capacity) = (0.0f32, 0.0f32);
    while let Some(p) = queue.pop_front() {
        let part = &parts[&p];
        let speed = speeds.get(&p).copied().unwrap_or(0.0);
        stress += part.stress * speed.abs();
        let own = if part.source() {
            part.rpm
        } else {
            turned.get(&p).copied().unwrap_or(0.0)
        };
        capacity += part.capacity * own.abs();
        let mut joined: Vec<[i32; 3]> = links(parts, p, part).into_iter().map(|(q, _)| q).collect();
        if let Some(g) = groups.iter().find(|g| g.contains(&p)) {
            joined.extend(g.iter().copied().filter(|q| parts.contains_key(q)));
        }
        for q in joined {
            if seen.insert(q) {
                queue.push_back(q);
            }
        }
    }
    Some((stress, capacity))
}

/// Every part's turn when the world's clock began, in degrees, so meshed cogwheels' teeth go into
/// each other's gaps as they turn: a small one beside another half a tooth (22.5 degrees) from it,
/// a large one diagonal to a small one half of its own tooth (11.25). Parts on one axle keep one
/// turn; each separate set of parts starts at 0 from its least position.
pub fn phases(parts: &HashMap<[i32; 3], Arc<Part>>) -> HashMap<[i32; 3], f32> {
    phases_from(parts, &HashMap::new())
}

/// [`phases`], each separate set of parts starting from one of `known` (a part's turn now) when
/// it holds one: parts already turning keep their turn, and a part joining them meshes with them.
pub fn phases_from(
    parts: &HashMap<[i32; 3], Arc<Part>>,
    known: &HashMap<[i32; 3], f32>,
) -> HashMap<[i32; 3], f32> {
    let mut starts: Vec<[i32; 3]> = parts.keys().copied().collect();
    starts.sort_by_key(|p| (!known.contains_key(p), *p));
    let mut phase = HashMap::new();
    for start in starts {
        if phase.contains_key(&start) {
            continue;
        }
        phase.insert(start, known.get(&start).copied().unwrap_or(0.0));
        let mut queue = VecDeque::from([start]);
        while let Some(p) = queue.pop_front() {
            let (a, c) = (&parts[&p], phase[&p]);
            for (q, _) in links(parts, p, a) {
                if phase.contains_key(&q) {
                    continue;
                }
                let b = &parts[&q];
                let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
                let meshed =
                    a.radius() > 0.0 && b.radius() > 0.0 && b.axis == a.axis && d[a.axis] == 0;
                phase.insert(q, if meshed { mesh_phase(a, b, d, c) } else { c });
                queue.push_back(q);
            }
        }
    }
    phase
}

/// The turn of cogwheel `b`, `d` from `a` in `a`'s plane, that meshes with `a` at turn `a_turn`:
/// with angles in the plane about the axis (from its next axis to the one after, as the turn
/// goes), a tooth of `a` pointing at `b` faces a gap of `b`. Where they touch, both move alike
/// as they turn (speeds the other way by their radii, as their teeth's counts), so it holds.
fn mesh_phase(a: &Part, b: &Part, d: [i32; 3], a_turn: f32) -> f32 {
    let (u, v) = ((a.axis + 1) % 3, (a.axis + 2) % 3);
    let toward = (d[v] as f32).atan2(d[u] as f32).to_degrees();
    let (pa, pb) = (360.0 / a.teeth(), 360.0 / b.teeth());
    (toward + 180.0 - pb * (0.5 - (toward - a_turn) / pa)).rem_euclid(pb)
}

/// The axis (0 x, 1 y, 2 z) and speed about its positive direction a model turning at face `f`
/// of a part whose speed there is `s` outwards turns at.
pub fn at_face(f: [i32; 3], s: f32) -> (usize, f32) {
    let axis = axis_of(f);
    (axis, s * f[axis] as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(kind: KineticPart, axis: Option<&str>, facing: Option<&str>, rpm: f32) -> Arc<Part> {
        let kinetic = Kinetic {
            part: kind,
            rpm,
            spins: true,
            half: None,
            extra: None,
            extra_facing: None,
            head: None,
            pole: None,
            models: Vec::new(),
            stress: 0.0,
            capacity: 0.0,
        };
        Arc::new(Part::new(&kinetic, axis, facing).unwrap())
    }

    #[test]
    fn water_flowing_past_one_side_of_a_wheel_turns_it() {
        // A wheel about z at the origin; water under it flowing east pushes its rim one way.
        let under_east = |p: [i32; 3]| {
            if p == [0, -1, 0] {
                [1.0, 0.0, 0.0]
            } else {
                [0.0; 3]
            }
        };
        assert_eq!(water_wheel_speed(2, [0; 3], under_east), 8.0);
        // Over it, the other way.
        let over_east = |p: [i32; 3]| {
            if p == [0, 1, 0] {
                [1.0, 0.0, 0.0]
            } else {
                [0.0; 3]
            }
        };
        assert_eq!(water_wheel_speed(2, [0; 3], over_east), -8.0);
        // Both: they cancel. Flowing along its axis: nothing.
        let both = |p: [i32; 3]| if p[1] != 0 { [1.0, 0.0, 0.0] } else { [0.0; 3] };
        assert_eq!(water_wheel_speed(2, [0; 3], both), 0.0);
        let along = |p: [i32; 3]| {
            if p == [0, -1, 0] {
                [0.0, 0.0, 1.0]
            } else {
                [0.0; 3]
            }
        };
        assert_eq!(water_wheel_speed(2, [0; 3], along), 0.0);
    }

    #[test]
    fn turns_point_a_model_from_the_way_it_is_made() {
        // From south, as the shaft ends are made.
        let faces = [
            ([0, 0, 1], [0, 0, 0]),
            ([-1, 0, 0], [0, 1, 0]),
            ([0, 0, -1], [0, 2, 0]),
            ([1, 0, 0], [0, 3, 0]),
            ([0, 1, 0], [1, 0, 0]),
            ([0, -1, 0], [3, 0, 0]),
        ];
        for (face, turns) in faces {
            assert_eq!(turns_to(face), turns);
        }
        // Create's crank handle is made pointing north, its block up: a crank facing up turns
        // the handle up, one facing east only about y.
        let kinetic = Kinetic {
            part: KineticPart::Crank,
            rpm: 32.0,
            spins: true,
            half: None,
            extra: Some("create:block/hand_crank/handle".into()),
            extra_facing: Some("north".into()),
            head: None,
            pole: None,
            models: Vec::new(),
            stress: 0.0,
            capacity: 8.0,
        };
        let up = Part::new(&kinetic, None, Some("up")).unwrap();
        assert_eq!(up.extra_turns, Some([3, 0, 0]));
        let east = Part::new(&kinetic, None, Some("east")).unwrap();
        assert_eq!(east.extra_turns, Some([0, 1, 0]));
        // Without it, the blockstate's turns.
        assert_eq!(shaft("x").extra_turns, None);
    }
    fn shaft(axis: &str) -> Arc<Part> {
        part(KineticPart::Shaft, Some(axis), None, 0.0)
    }
    fn cog(axis: &str) -> Arc<Part> {
        part(KineticPart::Cogwheel, Some(axis), None, 0.0)
    }
    fn large(axis: &str) -> Arc<Part> {
        part(KineticPart::LargeCogwheel, Some(axis), None, 0.0)
    }
    fn motor(facing: &str, rpm: f32) -> Arc<Part> {
        part(KineticPart::Motor, None, Some(facing), rpm)
    }

    #[test]
    fn shafts_relay_rotation_in_a_straight_line() {
        // A motor facing east turns the shafts east of it.
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], motor("east", 16.0));
        for x in 1..=3 {
            parts.insert([x, 0, 0], shaft("x"));
        }
        // One beside the line, on another axis, is not joined.
        parts.insert([1, 1, 0], shaft("y"));
        let speeds = solve(&parts, &HashMap::new());
        for x in 1..=3 {
            assert_eq!(speeds[&[x, 0, 0]], 16.0);
        }
        assert!(!speeds.contains_key(&[1, 1, 0]));
        // Facing west, its shafts turn the other way about their axis (alike outwards).
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], motor("west", 16.0));
        parts.insert([-1, 0, 0], shaft("x"));
        assert_eq!(solve(&parts, &HashMap::new())[&[-1, 0, 0]], -16.0);
        // A shaft-like part with only a facing (a water wheel) is an axle along it.
        let wheel = part(KineticPart::Shaft, None, Some("west"), 0.0);
        assert_eq!(wheel.axis, 0);
        parts.insert([-2, 0, 0], wheel);
        parts.insert([-3, 0, 0], shaft("x"));
        assert_eq!(solve(&parts, &HashMap::new())[&[-3, 0, 0]], -16.0);
    }

    #[test]
    fn a_network_taking_more_stress_than_its_sources_give_stands_still() {
        // A motor giving 2 per RPM at 16 (32) turns a drill taking 4 per RPM (64): too much.
        let with = |p: Arc<Part>, stress: f32, capacity: f32| {
            let mut p = (*p).clone();
            p.stress = stress;
            p.capacity = capacity;
            Arc::new(p)
        };
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], with(motor("east", 16.0), 0.0, 2.0));
        parts.insert(
            [1, 0, 0],
            with(part(KineticPart::Drill, None, Some("east"), 0.0), 4.0, 0.0),
        );
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]].abs(), 0.0);
        // Twice the capacity is enough (as much as it takes).
        parts.insert([0, 0, 0], with(motor("east", 16.0), 0.0, 4.0));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]].abs(), 16.0);
        // Two sources turning a line the same way give both their capacities.
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], with(motor("east", 16.0), 0.0, 2.0));
        parts.insert([1, 0, 0], with(shaft("x"), 4.0, 0.0));
        parts.insert([2, 0, 0], shaft("x"));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]], 0.0);
        parts.insert([3, 0, 0], with(motor("west", -16.0), 0.0, 2.0));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]], 16.0);
    }

    #[test]
    fn a_drill_turns_through_its_back_and_crushing_wheels_as_shafts() {
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], motor("east", 32.0));
        parts.insert([1, 0, 0], shaft("x"));
        // Facing east, its back (west) on the shaft: it turns.
        parts.insert([2, 0, 0], part(KineticPart::Drill, None, Some("east"), 0.0));
        // In front of the drill: its head joins nothing.
        parts.insert(
            [3, 0, 0],
            part(KineticPart::CrushingWheel, Some("x"), None, 0.0),
        );
        // Above the motor, a crushing wheel along y joins nothing either (the motor turns east).
        parts.insert(
            [0, 1, 0],
            part(KineticPart::CrushingWheel, Some("y"), None, 0.0),
        );
        let speeds = solve(&parts, &HashMap::new());
        assert_eq!(speeds[&[1, 0, 0]], 32.0);
        assert!(speeds[&[2, 0, 0]].abs() == 32.0);
        assert!(!speeds.contains_key(&[3, 0, 0]));
        assert!(!speeds.contains_key(&[0, 1, 0]));
        // A crushing wheel is an axle: a motor facing up turns one along y over it.
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], motor("up", 16.0));
        parts.insert(
            [0, 1, 0],
            part(KineticPart::CrushingWheel, Some("y"), None, 0.0),
        );
        assert_eq!(solve(&parts, &HashMap::new())[&[0, 1, 0]], 16.0);
    }

    #[test]
    fn neighbouring_cogwheels_turn_opposite_ways_and_large_ones_change_speed() {
        let mut parts = HashMap::new();
        parts.insert([0, -1, 0], motor("up", 32.0));
        parts.insert([0, 0, 0], cog("y"));
        parts.insert([1, 0, 0], cog("y"));
        // Diagonal from the first: a large one, half as fast.
        parts.insert([-1, 0, 1], large("y"));
        let speeds = solve(&parts, &HashMap::new());
        assert_eq!(speeds[&[0, 0, 0]], 32.0);
        assert_eq!(speeds[&[1, 0, 0]], -32.0);
        assert_eq!(speeds[&[-1, 0, 1]], -16.0);
        // Large to small doubles.
        let mut parts = HashMap::new();
        parts.insert([0, -1, 0], motor("up", 8.0));
        parts.insert([0, 0, 0], large("y"));
        parts.insert([1, 0, 1], cog("y"));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 1]], -16.0);
    }

    #[test]
    fn large_cogwheels_join_at_right_angles() {
        // Two large cogwheels on two faces of the block at the origin: one above it (axis y),
        // one east of it (axis x).
        let mut parts = HashMap::new();
        parts.insert([0, 2, 0], motor("down", 16.0));
        parts.insert([0, 1, 0], large("y"));
        parts.insert([1, 0, 0], large("x"));
        let speeds = solve(&parts, &HashMap::new());
        // As fast; the way their meeting edge moves alike (one step along -y, then +x).
        assert_eq!(speeds[&[0, 1, 0]], -16.0);
        assert_eq!(speeds[&[1, 0, 0]].abs(), 16.0);
        // Facing the same way along the edge both ways round gives one answer: the solve
        // from either end agrees.
        let mut back = HashMap::new();
        back.insert([2, 0, 0], motor("west", 16.0));
        back.insert([1, 0, 0], large("x"));
        back.insert([0, 1, 0], large("y"));
        let s = solve(&back, &HashMap::new());
        assert_eq!(
            s[&[0, 1, 0]] / s[&[1, 0, 0]],
            speeds[&[0, 1, 0]] / speeds[&[1, 0, 0]]
        );
    }

    #[test]
    fn a_gearbox_reverses_straight_through_and_mirrors_round_corners() {
        let mut parts = HashMap::new();
        parts.insert([-2, 0, 0], motor("east", 16.0));
        parts.insert([-1, 0, 0], shaft("x"));
        parts.insert([0, 0, 0], part(KineticPart::Gearbox, Some("y"), None, 0.0));
        parts.insert([1, 0, 0], shaft("x"));
        parts.insert([0, 0, 1], shaft("z"));
        parts.insert([0, 0, -1], shaft("z"));
        let s = solve(&parts, &HashMap::new());
        assert_eq!(s[&[-1, 0, 0]], 16.0);
        // Straight through: reversed.
        assert_eq!(s[&[1, 0, 0]], -16.0);
        // Round the corners, the two sides mirror each other.
        assert_eq!(s[&[0, 0, 1]], -s[&[0, 0, -1]]);
    }

    #[test]
    fn sources_turning_a_network_two_ways_jam_it_and_the_faster_one_leads() {
        // Two motors facing each other turn the shafts between them two ways: they stand still.
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], motor("east", 16.0));
        parts.insert([1, 0, 0], shaft("x"));
        parts.insert([2, 0, 0], motor("west", 16.0));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]], 0.0);
        // The same way (the west motor turned back), the faster one's speed.
        parts.insert([2, 0, 0], motor("west", -32.0));
        assert_eq!(solve(&parts, &HashMap::new())[&[1, 0, 0]], 32.0);
        // A ring of four small cogwheels turns each neighbour the other way: it holds.
        let mut ring = HashMap::new();
        ring.insert([0, -1, 0], motor("up", 16.0));
        for p in [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]] {
            ring.insert(p, cog("y"));
        }
        let s = solve(&ring, &HashMap::new());
        assert_eq!(s[&[1, 0, 1]], 16.0);
        assert_eq!(s[&[1, 0, 0]], -16.0);
    }

    #[test]
    fn a_crank_turns_its_back_while_turned_and_speeds_stay_under_the_limit() {
        let crank = part(KineticPart::Crank, None, Some("up"), 32.0);
        assert_eq!(crank.face, [0, -1, 0]);
        let mut parts = HashMap::new();
        parts.insert([0, 1, 0], crank);
        parts.insert([0, 0, 0], shaft("y"));
        assert!(solve(&parts, &HashMap::new()).is_empty());
        let turned = HashMap::from([([0, 1, 0], -32.0)]);
        assert_eq!(solve(&parts, &turned)[&[0, 0, 0]], 32.0);
        // Doubled past the limit: held at it.
        let mut fast = HashMap::new();
        fast.insert([0, -1, 0], motor("up", 200.0));
        fast.insert([0, 0, 0], large("y"));
        fast.insert([1, 0, 1], cog("y"));
        assert_eq!(solve(&fast, &HashMap::new())[&[1, 0, 1]], -MAX_RPM);
    }

    #[test]
    fn meshed_teeth_go_into_the_gaps() {
        let mut parts = HashMap::new();
        parts.insert([0, 0, 0], cog("y"));
        parts.insert([1, 0, 0], cog("y"));
        parts.insert([0, 1, 0], cog("y"));
        parts.insert([-1, 0, 1], large("y"));
        let p = phases(&parts);
        // Beside it: half a tooth off; on the same axle: as it is.
        assert!(((p[&[1, 0, 0]] - p[&[0, 0, 0]]).rem_euclid(45.0) - 22.5).abs() < 1e-3);
        assert_eq!(p[&[0, 1, 0]], p[&[0, 0, 0]]);
        // And the teeth stay in the gaps as they turn (each at its speed: the neighbour the other
        // way, the large one half as fast): a tooth of one pointing at the other means a gap of
        // the other pointing back, at any time.
        let meets = |a: [i32; 3], b: [i32; 3], speed_b: f32, t: f32| {
            let (pa, pb) = (&parts[&a], &parts[&b]);
            let (ta, tb) = (p[&a] + 16.0 * t, p[&b] + speed_b * t);
            // In the plane about y: z, then x.
            let toward = ((b[0] - a[0]) as f32)
                .atan2((b[2] - a[2]) as f32)
                .to_degrees();
            let teeth =
                (toward - ta) / (360.0 / pa.teeth()) + (toward + 180.0 - tb) / (360.0 / pb.teeth());
            (teeth.rem_euclid(1.0) - 0.5).abs() < 1e-3
        };
        for t in [0.0f32, 0.7, 3.3, 51.0] {
            assert!(meets([0, 0, 0], [1, 0, 0], -16.0, t), "beside, at {t}");
            assert!(meets([0, 0, 0], [-1, 0, 1], -8.0, t), "diagonal, at {t}");
        }
        // From turns already there: the one that has one keeps it, and the others mesh with it
        // as they did, a whole tooth's turns apart at most.
        let from = phases_from(&parts, &HashMap::from([([1, 0, 0], p[&[1, 0, 0]] + 45.0)]));
        assert_eq!(from[&[1, 0, 0]], p[&[1, 0, 0]] + 45.0);
        let whole_teeth = |a: f32, b: f32, tooth: f32| {
            let r = (a - b).rem_euclid(tooth);
            r.min(tooth - r) < 1e-3
        };
        assert!(whole_teeth(from[&[0, 0, 0]], p[&[0, 0, 0]], 45.0));
        assert!(whole_teeth(from[&[-1, 0, 1]], p[&[-1, 0, 1]], 22.5));
    }

    #[test]
    fn models_made_pointing_south_turn_to_each_face() {
        assert_eq!(turns_to([0, 0, 1]), [0, 0, 0]);
        assert_eq!(turns_to([0, 0, -1]), [0, 2, 0]);
        assert_eq!(turns_to([0, 1, 0]), [1, 0, 0]);
        assert_eq!(at_face([0, 0, -1], 16.0), (2, -16.0));
        assert_eq!(at_face([1, 0, 0], 16.0), (0, 16.0));
    }
}
