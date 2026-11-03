//! Create's funnels where this game simulates the world (Starling; ported from Create 6.0.10,
//! MIT, licenses/create-MIT.txt): FunnelBlock and FunnelBlockEntity. A funnel faces away from
//! the inventory behind it (a container or a machine). Collecting, it puts the items that come
//! in at its open side into that inventory; extracting, it takes them out (one at a time, a
//! brass one up to a stack) every 8 ticks and drops them in front of it while nothing lies
//! there. A redstone signal beside it pauses it.
use super::Game;
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use horizon_core::position::BlockPos;

/// `defaultExtractionTimer`: ticks between a funnel's extractions.
const EXTRACTION_TIMER: i32 = 8;
/// `ItemEntity.setDefaultPickUpDelay`.
const PICKUP_DELAY: u32 = 10;

/// What a funnel's state says.
struct Funnel {
    /// The way it faces: away from the inventory behind it.
    facing: [i32; 3],
    extracting: bool,
    powered: bool,
    /// A brass funnel's filter (none here: everything passes) lets it take up to a stack.
    brass: bool,
    /// A belt funnel's `shape` (BeltFunnelBlock.Shape): retracted, extended, pushing, pulling.
    belt: Option<String>,
}

fn neg(d: [i32; 3]) -> [i32; 3] {
    d.map(|v| -v)
}

fn direction_of(d: [i32; 3]) -> Direction {
    match d {
        [0, -1, 0] => Direction::Down,
        [0, 1, 0] => Direction::Up,
        [0, 0, -1] => Direction::North,
        [0, 0, 1] => Direction::South,
        [-1, 0, 0] => Direction::West,
        _ => Direction::East,
    }
}

impl Game<'_> {
    fn funnel(&self, pos: [i32; 3]) -> Option<Funnel> {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let s = self.pack.assets.registry.state(state)?;
        let facing = horizon_client::kinetics::direction(s.properties.get("facing")?)?;
        let flag = |k: &str| s.properties.get(k).map(String::as_str) == Some("true");
        Some(Funnel {
            facing,
            extracting: flag("extracting"),
            powered: flag("powered"),
            brass: s.name.contains("brass"),
            belt: s
                .name
                .ends_with("_belt_funnel")
                .then(|| s.properties.get("shape").cloned().unwrap_or_default()),
        })
    }

    /// `FunnelBlockEntity.determineCurrentMode` for a belt funnel: whether it puts items onto
    /// the belt below (else it takes them off it): pushing, or (retracted or extended) the belt
    /// moving the way it faces.
    fn belt_funnel_pushes(&self, pos: [i32; 3], f: &Funnel) -> bool {
        match f.belt.as_deref() {
            Some("pushing") => true,
            Some("pulling") | None => false,
            Some(_) => {
                let below = [pos[0], pos[1] - 1, pos[2]];
                self.belt_at(below).is_some_and(|(chain, _)| {
                    let speed = self
                        .machines
                        .speeds
                        .get(&chain.start())
                        .copied()
                        .unwrap_or(0.0);
                    let movement = chain.movement(speed);
                    movement != 0.0 && chain.movement_facing(movement) == f.facing
                })
            }
        }
    }

    /// `FunnelBlock.tick` and `FunnelBlockEntity.tick` for the funnel at `pos`: its `powered`
    /// follows the signal beside it (which also resets its cooldown), then it collects what
    /// comes in or extracts.
    pub(super) fn tick_funnel(&mut self, pos: [i32; 3]) {
        let Some(f) = self.funnel(pos) else {
            return;
        };
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let signal = self.with_level(|b, level, ticks| b.has_neighbor_signal(level, ticks, at));
        if signal != f.powered {
            self.set_property(pos, "powered", if signal { "true" } else { "false" });
        }
        if signal {
            self.machines.funnels.remove(&pos);
            return;
        }
        // A belt funnel taking from the belt does it as the belt moves (belt_funnel_check).
        if f.belt.is_some() {
            if self.belt_funnel_pushes(pos, &f) {
                self.belt_funnel_push(pos, &f);
            }
            return;
        }
        if f.extracting {
            self.funnel_extract(pos, &f);
        } else {
            self.funnel_collect(pos, &f);
        }
    }

    /// The block at `pos` with `key` set to `value` (when its block has that value).
    fn set_property(&mut self, pos: [i32; 3], key: &str, value: &str) {
        let registry = self.pack.assets.registry;
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let Some(mut s) = registry.state(state).cloned() else {
            return;
        };
        s.properties.insert(key.into(), value.into());
        let Some(new) = registry.id(&s) else {
            return;
        };
        if new != state {
            let at = BlockPos::new(pos[0], pos[1], pos[2]);
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, new, 2));
        }
    }

    /// `FunnelBlock.tryInsert`: `stack` into the inventory behind the funnel. What did not go
    /// in (all of it when nothing there holds items).
    fn funnel_insert(&mut self, pos: [i32; 3], f: &Funnel, stack: ItemStack) -> Option<ItemStack> {
        let behind = [
            pos[0] - f.facing[0],
            pos[1] - f.facing[1],
            pos[2] - f.facing[2],
        ];
        let rest = self
            .inventory_insert(behind, direction_of(f.facing), stack.clone())
            .unwrap_or(Some(stack.clone()));
        if rest.as_ref().is_none_or(|r| r.count != stack.count) {
            self.funnel_flap(pos, f);
        }
        rest
    }

    /// `FunnelBlock.tryInsert` into the funnel at `pos` (an arm's): what did not go in.
    pub(super) fn funnel_insert_at(
        &mut self,
        pos: [i32; 3],
        stack: ItemStack,
    ) -> Option<ItemStack> {
        match self.funnel(pos) {
            Some(f) if !f.powered => self.funnel_insert(pos, &f, stack),
            _ => Some(stack),
        }
    }

    /// A horizontal funnel's flap (`FunnelFlapPacket`): its sound (`AllSoundEvents.FUNNEL_FLAP`).
    fn funnel_flap(&mut self, pos: [i32; 3], f: &Funnel) {
        if f.facing[1] == 0 {
            self.machine_sound(pos, "create:funnel_flap", 0.125, 1.5);
        }
    }

    /// `FunnelBlock.entityInside`: each item lying in the funnel past its opening (a quarter
    /// in from its middle, the way it faces) goes in as far as the inventory takes it.
    fn funnel_collect(&mut self, pos: [i32; 3], f: &Funnel) {
        let center = pos.map(|v| f64::from(v) + 0.5);
        let open = [0, 1, 2].map(|i| center[i] - f64::from(f.facing[i]) * 0.25);
        let axis = f.facing.iter().position(|v| *v != 0).unwrap_or(1);
        let inside: Vec<usize> = self
            .dropped
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                let p = [d.position.x, d.position.y, d.position.z];
                let cell = (0..3).all(|i| p[i].floor() as i32 == pos[i]);
                let past = (p[axis] - open[axis]) * f64::from(f.facing[axis]) >= 0.0;
                cell && past
            })
            .map(|(i, _)| i)
            .collect();
        for i in inside.into_iter().rev() {
            let stack = self.dropped[i].stack.clone();
            match self.funnel_insert(pos, f, stack.clone()) {
                None => {
                    self.dropped.remove(i);
                }
                Some(rest) if rest.count < stack.count => {
                    let d = self.dropped.remove(i);
                    let at = [d.position.x, d.position.y, d.position.z];
                    self.spawn_item(rest, at, [0.0; 3], 0);
                }
                Some(_) => {}
            }
        }
    }

    /// `FunnelBlockEntity.activateExtractor`: every 8 ticks, while no item lies where it drops
    /// them (its block, and the one below unless it faces up), what the inventory behind it
    /// gives, dropped just out of its front (up a little, facing up).
    fn funnel_extract(&mut self, pos: [i32; 3], f: &Funnel) {
        let cooldown = self.machines.funnels.entry(pos).or_insert(0);
        if *cooldown > 0 {
            *cooldown -= 1;
            return;
        }
        let below = if f.facing == [0, 1, 0] { 0 } else { 1 };
        let blocked = self.dropped.iter().any(|d| {
            let p = [d.position.x, d.position.y, d.position.z].map(|v| v.floor() as i32);
            p[0] == pos[0] && p[2] == pos[2] && (pos[1] - below..=pos[1]).contains(&p[1])
        });
        if blocked {
            return;
        }
        let behind = [
            pos[0] - f.facing[0],
            pos[1] - f.facing[1],
            pos[2] - f.facing[2],
        ];
        let amount = if f.brass { 64 } else { 1 };
        let Some(Some(stack)) = self.inventory_extract(behind, direction_of(f.facing), amount)
        else {
            return;
        };
        self.funnel_flap(pos, f);
        let vertical = f.facing[0] == 0 && f.facing[2] == 0;
        let up = f.facing == [0, 1, 0];
        let reach = if vertical {
            if up { 0.15 } else { 0.5 }
        } else {
            0.25
        };
        let mut at = [0, 1, 2].map(|i| f64::from(pos[i]) + 0.5 + f64::from(f.facing[i]) * reach);
        if !vertical {
            at[1] -= 0.45;
        }
        let motion = if up { [0.0, 4.0 / 16.0, 0.0] } else { [0.0; 3] };
        self.spawn_item(stack, at, motion, PICKUP_DELAY);
        self.machines.funnels.insert(pos, EXTRACTION_TIMER);
    }

    /// `FunnelBlockEntity.activateExtractingBeltFunnel`: every 8 ticks, what the inventory
    /// behind gives onto the belt (or depot) below, the way the funnel faces, when it takes it.
    fn belt_funnel_push(&mut self, pos: [i32; 3], f: &Funnel) {
        let cooldown = self.machines.funnels.entry(pos).or_insert(0);
        if *cooldown > 0 {
            *cooldown -= 1;
            return;
        }
        let below = [pos[0], pos[1] - 1, pos[2]];
        if !self.takes_from_belt(below) {
            return;
        }
        let behind = [
            pos[0] - f.facing[0],
            pos[1] - f.facing[1],
            pos[2] - f.facing[2],
        ];
        let face = direction_of(f.facing);
        let amount = if f.brass { 64 } else { 1 };
        let Some(Some(stack)) = self.inventory_extract(behind, face, amount) else {
            return;
        };
        match self.belt_insert_into(below, f.facing, stack.clone()) {
            // Nowhere for it: back where it came from.
            Some(rest) if rest.count == stack.count => {
                let _ = self.inventory_insert(behind, face, rest);
                return;
            }
            Some(rest) => {
                let _ = self.inventory_insert(behind, face, rest);
            }
            None => {}
        }
        self.funnel_flap(pos, f);
        self.machines.funnels.insert(pos, EXTRACTION_TIMER);
    }

    /// `BeltFunnelInteractionHandler.checkForFunnels` for `item` on `chain` about to move to
    /// `next` at `movement`: a belt funnel above a segment it crosses that takes from the belt
    /// takes what its inventory does; one facing against the belt holds the item at its entry
    /// meanwhile. True holds the item where it is now.
    pub(super) fn belt_funnel_check(
        &mut self,
        chain: &super::belts::Chain,
        item: &mut super::belts::Transported,
        next: f32,
        movement: f32,
    ) -> bool {
        let positive = movement > 0.0;
        let len = chain.len() as i32;
        let step = if positive { 1 } else { -1 };
        let movement_facing = chain.movement_facing(movement);
        let mut segment = (item.position.floor() as i32).clamp(0, len - 1);
        while (0..len).contains(&segment)
            && if positive {
                segment as f32 <= next
            } else {
                (segment + 1) as f32 >= next
            }
        {
            let below = chain.position_for(segment);
            let at = [below[0], below[1] + 1, below[2]];
            segment += step;
            let Some(f) = self.funnel(at) else {
                continue;
            };
            let Some(shape) = f.belt.as_deref() else {
                continue;
            };
            let blocking = f.facing == neg(movement_facing);
            if f.facing == movement_facing || shape == "pushing" {
                continue;
            }
            let mut entry = (segment - step) as f32 + 0.5;
            if shape == "extended" {
                entry += 0.499 * if positive { -1.0 } else { 1.0 };
            }
            let crossed = if positive { next > entry } else { next < entry };
            if !crossed {
                return false;
            }
            if blocking {
                item.position = entry;
            }
            if f.powered {
                if blocking {
                    return true;
                }
                continue;
            }
            let stack = item.stack.clone();
            match self.funnel_insert(at, &f, stack.clone()) {
                Some(rest) if rest == stack => {}
                Some(rest) => item.stack = rest,
                None => item.stack.count = 0,
            }
            if blocking {
                return true;
            }
        }
        false
    }

    /// `FunnelBlock.useItemOn` on its open face: what the player holds into the inventory
    /// behind it, as much as it takes. True when it took the click.
    pub(super) fn use_funnel(&mut self, pos: [i32; 3], face: [i32; 3]) -> bool {
        let Some(f) = self.funnel(pos) else {
            return false;
        };
        if face != f.facing || f.extracting || f.powered {
            return false;
        }
        let Some(held) = self.held().cloned() else {
            return false;
        };
        let rest = self.funnel_insert(pos, &f, held);
        *self.held_mut() = rest;
        true
    }

    /// Whether a use on the funnel at `pos` from `face` is the funnel's ([`Game::use_funnel`]).
    pub(super) fn funnel_takes(
        &self,
        pos: [i32; 3],
        face: [i32; 3],
        held: Option<&ItemStack>,
    ) -> bool {
        held.is_some()
            && self
                .funnel(pos)
                .is_some_and(|f| face == f.facing && !f.extracting && !f.powered)
    }
}
