//! Create's mechanical belt (Starling; ported from Create 6.0.10, MIT, licenses/create-MIT.txt):
//! a belt connector laid between two shafts (BeltConnectorItem) makes belt segments from one to
//! the other; their pulleys turn together (the solver's groups) and the belt carries the items on
//! it (BeltInventory, TransportedItemStack) a block apart, at a 480th of its speed a tick, into
//! what is at its end (a depot, a millstone, a basin, another belt) or off it. Items falling on
//! it go on (BeltBlock.updateEntityAfterFallOn); an empty hand takes them off. A belt's items
//! are its start's block entity; the window moves them as the server does between updates and
//! draws the belt and them (BeltRenderer).
use super::Game;
use super::machine_entities::Kind;
use horizon_client::items::ItemStack;
use horizon_core::random::RandomSource;
use std::collections::HashMap;

/// `maxBeltLength`.
pub(super) const MAX_LENGTH: i32 = 20;
/// `BeltInventory`'s spacing between items.
const SPACING: f32 = 1.0;

/// `BeltSlope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slope {
    Horizontal,
    Upward,
    Downward,
    Vertical,
    Sideways,
}

/// `BeltPart`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Start,
    Middle,
    End,
    Pulley,
}

/// A belt segment's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Segment {
    pub(super) facing: [i32; 3],
    pub(super) slope: Slope,
    pub(super) part: Part,
}

impl Segment {
    pub(super) fn of(
        props: &horizon_world::block_state_properties::BlockProperties,
    ) -> Option<Self> {
        let facing = horizon_mods::content::direction(props.get("facing")?)?;
        let slope = match props.get("slope").map(String::as_str) {
            Some("upward") => Slope::Upward,
            Some("downward") => Slope::Downward,
            Some("vertical") => Slope::Vertical,
            Some("sideways") => Slope::Sideways,
            _ => Slope::Horizontal,
        };
        let part = match props.get("part").map(String::as_str) {
            Some("start") => Part::Start,
            Some("end") => Part::End,
            Some("pulley") => Part::Pulley,
            _ => Part::Middle,
        };
        Some(Segment {
            facing,
            slope,
            part,
        })
    }
}

/// A belt from its start to its end (its controller, BeltBlockEntity, and the segments after).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Chain {
    pub(super) segments: Vec<[i32; 3]>,
    pub(super) facing: [i32; 3],
    pub(super) slope: Slope,
}

impl Chain {
    pub(super) fn start(&self) -> [i32; 3] {
        self.segments[0]
    }
    pub(super) fn len(&self) -> usize {
        self.segments.len()
    }
    fn verticality(&self) -> i32 {
        match self.slope {
            Slope::Upward => 1,
            Slope::Downward => -1,
            _ => 0,
        }
    }
    /// `BeltHelper.getPositionForOffset`: the block `offset` segments along (past either end
    /// too).
    pub(super) fn position_for(&self, offset: i32) -> [i32; 3] {
        let s = self.start();
        if self.slope == Slope::Vertical {
            let up = if self.facing[0] + self.facing[2] > 0 {
                1
            } else {
                -1
            };
            return [s[0], s[1] + offset * up, s[2]];
        }
        let clamped = offset.clamp(0, self.len() as i32 - 1);
        [
            s[0] + offset * self.facing[0],
            s[1] + clamped * self.verticality(),
            s[2] + offset * self.facing[2],
        ]
    }
    /// `BeltHelper.getVectorForOffset`: where an item `offset` along lies (its block's middle).
    pub(super) fn vector_for(&self, offset: f32) -> [f64; 3] {
        let s = self.start().map(|v| f64::from(v) + 0.5);
        let length = self.len() as f32;
        let vertical = if offset < 0.5 {
            0.0
        } else {
            self.verticality() as f32 * (offset.min(length - 0.5) - 0.5)
        };
        let along = f64::from(offset - 0.5);
        [
            s[0] + f64::from(self.facing[0]) * along,
            s[1] + f64::from(vertical),
            s[2] + f64::from(self.facing[2]) * along,
        ]
    }
    /// `getDirectionAwareBeltMovementSpeed`: how far along its items go a tick, its pulleys
    /// turning at `speed` (about their axis's positive way).
    pub(super) fn movement(&self, speed: f32) -> f32 {
        let step = self.facing[0] + self.facing[2];
        let flip = if self.facing[0] != 0 { -1 } else { 1 };
        speed / 480.0 * (step * flip) as f32
    }
    /// `getMovementFacing`: the way the items go (`movement` > 0: towards its end).
    pub(super) fn movement_facing(&self, movement: f32) -> [i32; 3] {
        if movement >= 0.0 {
            self.facing
        } else {
            self.facing.map(|v| -v)
        }
    }
    /// The segment `pos` is, if it is one of this belt's.
    pub(super) fn segment_at(&self, pos: [i32; 3]) -> Option<usize> {
        self.segments.iter().position(|p| *p == pos)
    }
}

/// The belts among `segments`: from each start along its facing (up or down a slope's) to its
/// end, at most [`MAX_LENGTH`] long.
pub(super) fn chains(segments: &HashMap<[i32; 3], Segment>) -> Vec<Chain> {
    let mut out = Vec::new();
    let mut starts: Vec<[i32; 3]> = segments
        .iter()
        .filter(|(_, s)| s.part == Part::Start)
        .map(|(p, _)| *p)
        .collect();
    starts.sort_unstable();
    for start in starts {
        let first = segments[&start];
        let mut chain = vec![start];
        let mut at = start;
        let mut done = false;
        for _ in 0..MAX_LENGTH * 2 {
            let next = match first.slope {
                Slope::Vertical => {
                    let up = if first.facing[0] + first.facing[2] > 0 {
                        1
                    } else {
                        -1
                    };
                    [at[0], at[1] + up, at[2]]
                }
                slope => {
                    let dy = match slope {
                        Slope::Upward => 1,
                        Slope::Downward => -1,
                        _ => 0,
                    };
                    [at[0] + first.facing[0], at[1] + dy, at[2] + first.facing[2]]
                }
            };
            let Some(s) = segments.get(&next) else {
                break;
            };
            if s.facing != first.facing || s.slope != first.slope {
                break;
            }
            chain.push(next);
            at = next;
            if s.part == Part::End {
                done = true;
                break;
            }
        }
        if done {
            out.push(Chain {
                segments: chain,
                facing: first.facing,
                slope: first.slope,
            });
        }
    }
    out
}

/// `TransportedItemStack`: an item on a belt, how far along (from its start, in segments) and
/// how far off the middle, its turn, and whether something above holds it (a press).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Transported {
    pub(super) stack: ItemStack,
    pub(super) position: f32,
    pub(super) previous: f32,
    pub(super) side: f32,
    pub(super) angle: i32,
    pub(super) locked: bool,
}

impl Transported {
    pub(super) fn new(stack: ItemStack, position: f32, angle: i32) -> Self {
        Transported {
            stack,
            position,
            previous: position,
            side: 0.0,
            angle,
            locked: false,
        }
    }
}

/// `BeltInventory.Ending`: what is past the belt's end, and how far before it items stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ending {
    Eject,
    Insert,
    Blocked,
}

impl Ending {
    fn margin(self) -> f32 {
        match self {
            Ending::Eject => 0.0,
            Ending::Insert => 0.25,
            Ending::Blocked => 0.45,
        }
    }
}

/// `BeltInventory.tick`'s moving: each item along by `movement` (a tick), no nearer than a block
/// to the one ahead, nor past the end (less the ending's margin). Held items stay. The indices
/// of those that reached the end this tick, for the caller to put somewhere.
pub(super) fn advance(
    items: &mut [Transported],
    length: usize,
    movement: f32,
    ending: Ending,
) -> Vec<usize> {
    advance_with(items, length, movement, ending, &mut |_, _| false)
}

/// [`advance`], `stop` asked of each item before it moves where it would (`BeltInventory.tick`'s
/// funnel check): true holds it where `stop` leaves it.
pub(super) fn advance_with(
    items: &mut [Transported],
    length: usize,
    movement: f32,
    ending: Ending,
    stop: &mut dyn FnMut(&mut Transported, f32) -> bool,
) -> Vec<usize> {
    // The first entry furthest along the way they go.
    if movement > 0.0 {
        items.sort_by(|a, b| b.position.total_cmp(&a.position));
    } else {
        items.sort_by(|a, b| a.position.total_cmp(&b.position));
    }
    let positive = movement > 0.0;
    let mut reached = Vec::new();
    let mut ahead: Option<f32> = None;
    for (i, item) in items.iter_mut().enumerate() {
        item.previous = item.position;
        let current = item.position;
        if item.locked || movement == 0.0 {
            ahead = Some(current);
            continue;
        }
        let mut step = movement;
        let mut blocked = false;
        if let Some(front) = ahead {
            let diff = front - current;
            if diff.abs() <= SPACING {
                blocked = true;
            }
            step = if positive {
                step.min(diff - SPACING)
            } else {
                step.max(diff + SPACING)
            };
        }
        let mut to_end = if positive {
            length as f32 - current
        } else {
            -current
        };
        if to_end.abs() < step.abs() + 1.0 {
            to_end += if positive {
                -ending.margin()
            } else {
                ending.margin()
            };
        }
        let limited = if positive {
            step.min(to_end)
        } else {
            step.max(to_end)
        };
        if !blocked && stop(item, current + limited) {
            ahead = Some(item.position);
            continue;
        }
        if !blocked {
            item.position += limited;
            // Back to the middle as it goes.
            let back = (-item.side).clamp(-item.side.abs(), item.side.abs());
            item.side += (back * limited.abs() * 6.0).clamp(-item.side.abs(), item.side.abs());
            if limited != step && (limited - to_end).abs() < 1e-6 {
                reached.push(i);
            }
        }
        ahead = Some(item.position);
    }
    reached
}

/// The belt items of a belt's start, as its tag keeps them (`Belt`).
pub(super) fn read_items(tag: &horizon_world::nbt::Tag) -> Vec<Transported> {
    use horizon_world::nbt::Tag;
    let Some(Tag::List(list)) = tag.get("Belt") else {
        return Vec::new();
    };
    let float = |t: &Tag, k: &str| match t.get(k) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 0.0,
    };
    list.iter()
        .filter_map(|t| {
            let stack = horizon_client::save::item_from_nbt(t.get("Item")?)?;
            let angle = match t.get("Angle") {
                Some(Tag::Int(a)) => *a,
                _ => 0,
            };
            let mut item = Transported::new(stack, float(t, "Pos"), angle);
            item.side = float(t, "Side");
            item.locked = matches!(t.get("Locked"), Some(Tag::Byte(1)));
            Some(item)
        })
        .collect()
}

/// `read_items`'s inverse.
pub(super) fn items_tag(items: &[Transported]) -> horizon_world::nbt::Tag {
    use horizon_world::nbt::Tag;
    Tag::List(
        items
            .iter()
            .map(|i| {
                Tag::compound([
                    ("Item", horizon_client::save::item_to_nbt(&i.stack)),
                    ("Pos", Tag::Float(i.position.to_bits())),
                    ("Side", Tag::Float(i.side.to_bits())),
                    ("Angle", Tag::Int(i.angle)),
                    ("Locked", Tag::Byte(i8::from(i.locked))),
                ])
            })
            .collect(),
    )
}

/// Where the game keeps its belts: found with its machines.
#[derive(Default)]
pub(super) struct Belts {
    /// The belts at the last look.
    pub(super) chains: Vec<Chain>,
    /// Each belt's items, by its start, between their tags' updates.
    pub(super) items: HashMap<[i32; 3], Vec<Transported>>,
    /// The shaft a belt connector was used on first (`BELT_FIRST_SHAFT`).
    pub(super) first: Option<[i32; 3]>,
}

impl Game<'_> {
    /// The belt segments among the machines at `positions` (their states read now).
    pub(super) fn belt_segments(&self, positions: &[[i32; 3]]) -> HashMap<[i32; 3], Segment> {
        let registry = self.pack.assets.registry;
        positions
            .iter()
            .filter_map(|p| {
                let state = registry.state(self.world.block(p[0], p[1], p[2]))?;
                Some((*p, Segment::of(&state.properties)?))
            })
            .collect()
    }

    /// The belt `pos` is a segment of, and which.
    pub(super) fn belt_at(&self, pos: [i32; 3]) -> Option<(&Chain, usize)> {
        self.machines
            .belts
            .chains
            .iter()
            .find_map(|c| Some((c, c.segment_at(pos)?)))
    }

    /// Once a tick where the world is simulated: every belt's items along, those at its end into
    /// what is there or off it, items lying on it onto it (BeltBlockEntity.tick).
    pub(super) fn tick_belts(&mut self) {
        let chains = self.machines.belts.chains.clone();
        for chain in chains {
            let start = chain.start();
            let speed = self.machines.speeds.get(&start).copied().unwrap_or(0.0);
            let mut items = match self.machines.belts.items.remove(&start) {
                Some(items) => items,
                None => self.read_belt(start),
            };
            let mut changed = self.catch_on_belt(&chain, &mut items);
            let movement = chain.movement(speed);
            if movement != 0.0 && !items.is_empty() {
                let ending = self.belt_ending(&chain, movement);
                let count =
                    |items: &[Transported]| items.iter().map(|i| i.stack.count).sum::<u32>();
                let before = count(&items);
                // Belt funnels above take what they can as items pass (funnels.rs).
                let reached = advance_with(
                    &mut items,
                    chain.len(),
                    movement,
                    ending,
                    &mut |item, next| self.belt_funnel_check(&chain, item, next, movement),
                );
                changed |= count(&items) != before;
                let mut gone = Vec::new();
                for i in reached {
                    if items[i].stack.count == 0 {
                        continue;
                    }
                    let facing = chain.movement_facing(movement);
                    let past = chain.position_for(if movement > 0.0 {
                        chain.len() as i32
                    } else {
                        -1
                    });
                    match ending {
                        Ending::Insert => {
                            let rest = self.belt_insert_into(past, facing, items[i].stack.clone());
                            match rest {
                                None => gone.push(i),
                                Some(rest) if rest.count != items[i].stack.count => {
                                    items[i].stack = rest;
                                    changed = true;
                                }
                                Some(_) => {}
                            }
                        }
                        Ending::Eject => {
                            self.belt_eject(&chain, &items[i], movement, facing);
                            gone.push(i);
                        }
                        Ending::Blocked => {}
                    }
                }
                gone.sort_unstable();
                for i in gone.into_iter().rev() {
                    items.remove(i);
                    changed = true;
                }
                // What a funnel took all of.
                items.retain(|i| i.stack.count > 0);
            }
            if changed {
                self.write_belt(start, &items);
            }
            self.machines.belts.items.insert(start, items);
        }
    }

    /// A belt's items as its start's tag keeps them.
    pub(super) fn read_belt(&self, start: [i32; 3]) -> Vec<Transported> {
        super::spill::saved_entity(&self.world, start)
            .map(|t| read_items(&t))
            .unwrap_or_default()
    }

    /// Keeps a belt's items as its start's block entity (and tells the players near).
    pub(super) fn write_belt(&mut self, start: [i32; 3], items: &[Transported]) {
        let (Some(Kind::Belt), Some(mut e)) = (
            self.machine_kind(start),
            self.machine_entity(start).map(|(_, e)| e),
        ) else {
            return;
        };
        e.belt = items.to_vec();
        e.since = self.world.game_time;
        self.set_machine_entity(start, &e);
    }

    /// `resolveEnding`: past the end, something that takes items from a belt, a solid face, or
    /// nothing.
    fn belt_ending(&self, chain: &Chain, movement: f32) -> Ending {
        let past = chain.position_for(if movement > 0.0 {
            chain.len() as i32
        } else {
            -1
        });
        if self.takes_from_belt(past) {
            return Ending::Insert;
        }
        let facts = self
            .res
            .facts()
            .get(self.world.block(past[0], past[1], past[2]));
        if facts.full_collision {
            Ending::Blocked
        } else {
            Ending::Eject
        }
    }

    /// Whether what is at `at` takes items from a belt (`DirectBeltInputBehaviour`): a depot, a
    /// millstone, a basin, a belt, a saw facing up, crushing wheels.
    pub(super) fn takes_from_belt(&self, at: [i32; 3]) -> bool {
        match self.machine_kind(at) {
            Some(Kind::Depot | Kind::Millstone | Kind::Basin | Kind::Belt) => true,
            Some(Kind::Saw) => {
                let state = self.world.block(at[0], at[1], at[2]);
                self.pack
                    .assets
                    .registry
                    .state(state)
                    .is_some_and(|s| s.properties.get("facing").map(String::as_str) == Some("up"))
            }
            _ => self.crusher_at(at),
        }
    }

    /// `DirectBeltInputBehaviour.handleInsertion` of what is at `at`, from a belt going `facing`:
    /// what did not go in.
    pub(super) fn belt_insert_into(
        &mut self,
        at: [i32; 3],
        facing: [i32; 3],
        stack: ItemStack,
    ) -> Option<ItemStack> {
        if self.crusher_at(at) {
            return if self.controller_insert(at, &stack) {
                None
            } else {
                Some(stack)
            };
        }
        match self.machine_kind(at) {
            Some(Kind::Belt) => self.belt_insert_from_side(at, facing, stack),
            _ => self
                .machine_insert(at, stack.clone())
                .unwrap_or(Some(stack)),
        }
    }

    /// An item onto the belt segment at `at` from a belt behind it going `facing` (the next
    /// belt's start or side): `BeltInventory.canInsertAtFromSide` and `insert`.
    pub(super) fn belt_insert_from_side(
        &mut self,
        at: [i32; 3],
        facing: [i32; 3],
        stack: ItemStack,
    ) -> Option<ItemStack> {
        let Some((chain, segment)) = self.belt_at(at).map(|(c, s)| (c.clone(), s)) else {
            return Some(stack);
        };
        let start = chain.start();
        let mut items = match self.machines.belts.items.remove(&start) {
            Some(items) => items,
            None => self.read_belt(start),
        };
        // Its first half from behind it, the middle from a side.
        let position = if facing == chain.facing {
            segment as f32
        } else if facing == chain.facing.map(|v| -v) {
            segment as f32 + 1.0
        } else {
            segment as f32 + 0.5
        };
        let free = items
            .iter()
            .all(|i| (i.position - position).abs() >= SPACING);
        let rest = if free {
            let angle = (self.world.random.next_float() * 360.0) as i32;
            items.push(Transported::new(stack, position, angle));
            self.write_belt(start, &items);
            None
        } else {
            Some(stack)
        };
        self.machines.belts.items.insert(start, items);
        rest
    }

    /// `BeltInventory.eject`: off the end, moving the way the belt goes and up a little.
    fn belt_eject(&mut self, chain: &Chain, item: &Transported, movement: f32, facing: [i32; 3]) {
        let at = chain.vector_for(item.position);
        let speed = f64::from(movement.abs().max(1.0 / 8.0));
        let motion = [
            f64::from(facing[0]) * speed,
            1.0 / 8.0,
            f64::from(facing[2]) * speed,
        ];
        let out = [at[0], at[1] + 6.0 / 16.0, at[2]];
        self.spawn_item(item.stack.clone(), out, motion, 10);
    }

    /// `BeltBlock.updateEntityAfterFallOn`: an item lying on a segment goes onto the belt there,
    /// when there is room. True when one did.
    fn catch_on_belt(&mut self, chain: &Chain, items: &mut Vec<Transported>) -> bool {
        let mut caught = false;
        for (segment, pos) in chain.segments.iter().enumerate() {
            let top = f64::from(pos[1]) + 15.0 / 16.0;
            let found = self.dropped.iter().position(|d| {
                d.position.x.floor() as i32 == pos[0]
                    && d.position.z.floor() as i32 == pos[2]
                    && (d.position.y - top).abs() < 0.25
                    && d.on_ground
            });
            let Some(i) = found else {
                continue;
            };
            let position = segment as f32 + 0.5;
            if items
                .iter()
                .any(|t| (t.position - position).abs() < SPACING)
            {
                continue;
            }
            let d = self.dropped.remove(i);
            // The way it lies off the middle, across the belt.
            let across = if chain.facing[0] != 0 {
                d.position.z - (f64::from(pos[2]) + 0.5)
            } else {
                d.position.x - (f64::from(pos[0]) + 0.5)
            };
            let mut item = Transported::new(d.stack, position, (d.id % 360) as i32);
            item.side = across as f32;
            items.push(item);
            caught = true;
        }
        caught
    }

    /// `BeltConnectorItem.useOn`: a belt connector on a shaft keeps it as the first, and on a
    /// second shaft lays a belt from one to the other (sneaking forgets the first). True when it
    /// took the click.
    pub(super) fn use_belt_connector(&mut self, pos: [i32; 3], sneaking: bool) -> bool {
        if self
            .held()
            .is_none_or(|h| &*h.item != "create:belt_connector")
        {
            return false;
        }
        if sneaking {
            self.machines.belts.first = None;
            return true;
        }
        if self.shaft_axis(pos).is_none() {
            return false;
        }
        match self.machines.belts.first.take() {
            None => self.machines.belts.first = Some(pos),
            Some(first) if first == pos => {}
            Some(first) => {
                if self.can_connect_belt(first, pos) {
                    self.create_belts(first, pos);
                    if !self.creative {
                        self.consume_held();
                    }
                }
            }
        }
        true
    }

    /// A shaft's axis (0 x, 1 y, 2 z) at `pos` (`ShaftBlock.isShaft`).
    fn shaft_axis(&self, pos: [i32; 3]) -> Option<usize> {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        if &*self.res.facts().get(state).name != "create:shaft" {
            return None;
        }
        let props = &self.pack.assets.registry.state(state)?.properties;
        match props.get("axis").map(String::as_str) {
            Some("x") => Some(0),
            Some("y") => Some(1),
            _ => Some(2),
        }
    }

    /// `BeltConnectorItem.canConnect`: two shafts of one axis, less than a belt's length apart,
    /// in a line or a diagonal across their axis, with nothing but air or shafts between.
    fn can_connect_belt(&self, first: [i32; 3], second: [i32; 3]) -> bool {
        let (Some(axis), Some(other)) = (self.shaft_axis(first), self.shaft_axis(second)) else {
            return false;
        };
        let d: [i32; 3] = std::array::from_fn(|i| second[i] - first[i]);
        let distance = f64::from(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if distance >= f64::from(MAX_LENGTH) || axis != other || d[axis] != 0 {
            return false;
        }
        let a = d.map(i32::abs);
        let sames = i32::from(a[0] == a[1]) + i32::from(a[1] == a[2]) + i32::from(a[2] == a[0]);
        if sames != 1 || (axis == 1 && d[0] != 0 && d[2] != 0) {
            return false;
        }
        let step = d.map(i32::signum);
        let mut at = [first[0] + step[0], first[1] + step[1], first[2] + step[2]];
        for _ in 0..1000 {
            if at == second {
                return true;
            }
            let shaft = self.shaft_axis(at) == Some(axis);
            let state = self.world.block(at[0], at[1], at[2]);
            if !shaft && !self.res.facts().get(state).replaceable {
                return false;
            }
            at = [at[0] + step[0], at[1] + step[1], at[2] + step[2]];
        }
        false
    }

    /// `BeltConnectorItem.createBelts`: the belt's segments from `start` to `end`, a start, an end,
    /// pulleys where shafts were and middles between; flat across upright shafts (sideways).
    fn create_belts(&mut self, start: [i32; 3], end: [i32; 3]) {
        let d: [i32; 3] = std::array::from_fn(|i| end[i] - start[i]);
        // getSlopeBetween.
        let mut slope = if d[1] != 0 {
            if d[0] != 0 || d[2] != 0 {
                if d[1] > 0 { "upward" } else { "downward" }
            } else {
                "vertical"
            }
        } else {
            "horizontal"
        };
        // getFacingFromTo; a vertical belt faces across its shafts.
        let along_z = start[0] == end[0];
        let positive = if d[0] == 0 && d[2] == 0 {
            d[1] > 0
        } else if along_z {
            d[2] > 0
        } else {
            d[0] > 0
        };
        let axis_x = if d[0] == d[2] {
            self.shaft_axis(start) != Some(0)
        } else {
            !along_z
        };
        let facing: [i32; 3] = match (axis_x, positive) {
            (true, true) => [1, 0, 0],
            (true, false) => [-1, 0, 0],
            (false, true) => [0, 0, 1],
            (false, false) => [0, 0, -1],
        };
        // getBeltChainBetween.
        let mut positions = vec![start];
        let mut at = start;
        for _ in 0..1000 {
            at = if slope == "vertical" {
                [at[0], at[1] + if positive { 1 } else { -1 }, at[2]]
            } else {
                let dy = match slope {
                    "upward" => 1,
                    "downward" => -1,
                    _ => 0,
                };
                [at[0] + facing[0], at[1] + dy, at[2] + facing[2]]
            };
            if at == end {
                break;
            }
            positions.push(at);
        }
        positions.push(end);
        let registry = self.pack.assets.registry;
        let Some(default) = registry.default_state("create:belt") else {
            return;
        };
        let Some(base) = registry.state(default).cloned() else {
            return;
        };
        let facing_name = match facing {
            [1, 0, 0] => "east",
            [-1, 0, 0] => "west",
            [0, 0, 1] => "south",
            _ => "north",
        };
        let mut placed = Vec::new();
        for pos in &positions {
            let shaft = self.shaft_axis(*pos);
            let mut part = if *pos == start {
                "start"
            } else if *pos == end {
                "end"
            } else {
                "middle"
            };
            if part == "middle" && shaft.is_some() {
                part = "pulley";
            }
            if shaft == Some(1) {
                slope = "sideways";
            }
            let mut s = base.clone();
            s.properties.insert("facing".into(), facing_name.into());
            s.properties.insert("slope".into(), slope.into());
            s.properties.insert("part".into(), part.into());
            let Some(state) = registry.id(&s) else {
                return;
            };
            placed.push((*pos, state));
        }
        let at = start.map(|v| f64::from(v) + 0.5);
        self.audio
            .play("minecraft:block.wool.place", Some(at), 0.5, 1.0);
        for (pos, state) in placed {
            let p = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
            self.with_level(|b, level, ticks| b.set_block(level, ticks, p, state, 3));
        }
        let _ = self.process_level_events();
    }

    /// `BeltBlock.useItemOn` with an empty hand: the items on that segment to the player.
    pub(super) fn use_belt(&mut self, pos: [i32; 3]) -> bool {
        if self.held().is_some() {
            return false;
        }
        let Some((chain, segment)) = self.belt_at(pos).map(|(c, s)| (c.clone(), s)) else {
            return false;
        };
        let start = chain.start();
        let mut items = match self.machines.belts.items.remove(&start) {
            Some(items) => items,
            None => self.read_belt(start),
        };
        let (taken, kept): (Vec<_>, Vec<_>) = items
            .drain(..)
            .partition(|i| i.position.floor() as usize == segment);
        let any = !taken.is_empty();
        for item in taken {
            self.give_or_drop(item.stack);
        }
        if any {
            self.write_belt(start, &kept);
        }
        self.machines.belts.items.insert(start, kept);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(len: usize) -> Chain {
        Chain {
            segments: (0..len as i32).map(|z| [0, 0, -z]).collect(),
            facing: [0, 0, -1],
            slope: Slope::Horizontal,
        }
    }

    #[test]
    fn items_go_a_block_apart_and_stop_at_a_blocked_end() {
        let c = chain(3);
        let mut items = vec![
            Transported::new(ItemStack::new("x:a", 1), 0.5, 0),
            Transported::new(ItemStack::new("x:b", 1), 1.0, 0),
        ];
        for _ in 0..200 {
            advance(&mut items, c.len(), 0.1, Ending::Blocked);
        }
        items.sort_by(|a, b| a.position.total_cmp(&b.position));
        // The first stops short of the end by the margin; the next a block behind it.
        assert!((items[1].position - (3.0 - 0.45)).abs() < 1e-4);
        assert!((items[0].position - (3.0 - 0.45 - 1.0)).abs() < 1e-4);
        // At an open end it reaches it.
        let mut one = vec![Transported::new(ItemStack::new("x:a", 1), 2.5, 0)];
        let ticks = (0..10)
            .position(|_| !advance(&mut one, c.len(), 0.1, Ending::Eject).is_empty())
            .expect("reaches the end");
        // Half a block at a tenth a tick: there on the sixth (the game then takes it off).
        assert_eq!(ticks, 5);
        assert!((one[0].position - 3.0).abs() < 1e-4);
    }

    #[test]
    fn a_belts_offsets_run_along_its_facing() {
        let c = chain(4);
        assert_eq!(c.position_for(4), [0, 0, -4]);
        assert_eq!(c.position_for(-1), [0, 0, 1]);
        // North: the pulleys about x; turning positive moves the top south (items back).
        assert!(c.movement(48.0) < 0.0);
        let mut found = HashMap::new();
        for (i, p) in c.segments.iter().enumerate() {
            let part = match i {
                0 => Part::Start,
                3 => Part::End,
                _ => Part::Middle,
            };
            found.insert(
                *p,
                Segment {
                    facing: [0, 0, -1],
                    slope: Slope::Horizontal,
                    part,
                },
            );
        }
        assert_eq!(chains(&found), vec![c]);
    }
}
