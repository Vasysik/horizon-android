//! Bedrock's touch inventory on the screens with slots, made of the clicks a mouse makes there (so
//! a server sees vanilla's): a tap picks a stack, framed, its tooltip beside the window; a tap on
//! another slot moves it there (onto the same item as much as fits, onto another item the two
//! swap); a finger held on a stack picks half of it, shown by a bar on the slot; a second quick
//! tap on the picked stack sends it to the other side (Shift-click); a tap outside the window
//! throws it. In creative an item of the tabs taps into a slot as a full stack. With a stack
//! picked (or a finger still on the half it held), a finger drawn across slots shares it out
//! among them, as the mouse's drag does (QUICK_CRAFT). Stacks slide to where they went, and a
//! finger on a slot's frame or just past it still means that slot. A finger dragged across the
//! player turns it round. A finger that moves from a slot (nothing picked) scrolls or drags as the
//! pointer does elsewhere.
use super::super::input_trace::InputOrigin;
use super::super::{Game, Screen, event_loop::EventLoopCtl};
use horizon_client::{
    creative::Creative,
    hud::{HudBatch, rgba},
    items::ItemStack,
};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, Touch, TouchPhase, WindowEvent},
    keyboard::KeyCode,
    window::WindowId,
};

/// How long a finger rests on a stack to pick half of it.
const HOLD: Duration = Duration::from_millis(450);
/// A second tap on the picked stack this soon sends it across.
const DOUBLE_TAP: Duration = Duration::from_millis(350);
/// How long a moved stack takes to slide to its slot.
const SLIDE: Duration = Duration::from_millis(150);
/// How far from a slot's middle a finger still means it: half its 18-pixel cell and 2 beyond.
const REACH: f32 = 11.0;
/// Degrees the player turns for each GUI pixel a finger drags across it.
const TURN: f32 = 2.5;

#[derive(Default)]
pub(super) struct SlotTouch {
    /// The finger on a slot (or outside the window with a stack picked), not yet lifted or moved.
    down: Option<Down>,
    picked: Option<Picked>,
    /// Stacks sliding to the slots they went to.
    slides: Vec<Slide>,
    /// The finger turning the player, and where it was last (window pixels).
    turning: Option<(u64, f64)>,
    /// The finger drawing the pick across slots.
    spread: Option<Spread>,
    /// How far the player was turned, in degrees.
    spin: f32,
}

struct Slide {
    /// What the slot it goes to holds once it lands.
    stack: ItemStack,
    from: usize,
    to: usize,
    at: Instant,
}

/// A finger drawn across slots with a stack picked: the slots it crossed, in order, which share
/// the stack out once it lifts.
struct Spread {
    finger: u64,
    slots: Vec<usize>,
}

struct Down {
    /// Where it landed, to hand it to the pointer if it moves.
    touch: Touch,
    /// None: outside the window.
    slot: Option<usize>,
    at: Instant,
    /// The hold already picked half of the stack.
    held: bool,
}

#[derive(Clone, Copy)]
struct Picked {
    slot: usize,
    half: bool,
    at: Instant,
}

impl Game<'_> {
    /// The screens whose slots the fingers work as Bedrock's.
    fn slot_touch_screen(&self) -> bool {
        matches!(self.screen, Screen::Inventory | Screen::Creative)
            && self.title.is_none()
            && self.carried.is_none()
            && self.has_touch()
    }

    fn touch_gui_batch(&self) -> HudBatch {
        HudBatch::new(self.touch_size())
    }

    /// The slot under a window position, and whether it is outside the window.
    fn slot_touch_at(&mut self, p: PhysicalPosition<f64>) -> (Option<usize>, bool) {
        let batch = self.touch_gui_batch();
        let gui = (p.x as f32 / batch.scale, p.y as f32 / batch.scale);
        if self.touch.slots.spread.is_none() {
            self.bedrock_side(gui.0, batch.width);
        }
        let slot = self
            .slot_at(&batch, gui.0, gui.1)
            .or_else(|| self.touch_slot_near(&batch, gui));
        let cursor = std::mem::replace(&mut self.cursor, gui);
        let outside = slot.is_none() && self.clicked_outside(&batch);
        self.cursor = cursor;
        (slot, outside)
    }

    /// Whether a window position is on the player shown in the inventory (its box in the survival
    /// inventory, in the creative screen's inventory tab).
    fn touch_on_player(&self, p: PhysicalPosition<f64>) -> bool {
        let batch = self.touch_gui_batch();
        let (x, y) = (p.x as f32 / batch.scale, p.y as f32 / batch.scale);
        let (ox, oy) = self.inventory_origin(&batch);
        let [x0, y0, x1, y1] = if self.creative_inventory_open() {
            [73.0, 6.0, 105.0, 49.0]
        } else if self.screen == Screen::Inventory
            && self.container.is_none()
            && self.station.is_none()
            && self.craft_size != 3
            && !self.mount_active()
        {
            [26.0, 8.0, 75.0, 78.0]
        } else {
            return false;
        };
        (ox + x0..ox + x1).contains(&x) && (oy + y0..oy + y1).contains(&y)
    }

    /// How far a finger turned the inventory's player, in degrees, on a touch screen (where the
    /// player does not follow a cursor).
    pub(in crate::game) fn player_spin(&self) -> Option<f32> {
        self.has_touch().then_some(self.touch.slots.spin)
    }

    /// The slot whose cell holds `gui` (its frame, a little past it), the nearest if two do.
    fn touch_slot_near(&self, batch: &HudBatch, gui: (f32, f32)) -> Option<usize> {
        let (ox, oy) = self.inventory_origin(batch);
        self.slots()
            .into_iter()
            .map(|i| {
                let (sx, sy) = self.slot_position(i);
                (i, ox + sx + 8.0 - gui.0, oy + sy + 8.0 - gui.1)
            })
            .filter(|&(_, dx, dy)| dx.abs() <= REACH && dy.abs() <= REACH)
            .min_by(|a, b| (a.1 * a.1 + a.2 * a.2).total_cmp(&(b.1 * b.1 + b.2 * b.2)))
            .map(|(i, ..)| i)
    }

    fn touch_has_stack(&mut self, slot: usize) -> bool {
        self.touch_stack(slot).is_some()
    }

    /// The stack a slot shows: a creative tab's display slots show the tab's list.
    fn touch_stack(&mut self, slot: usize) -> Option<ItemStack> {
        self.bedrock_for_slot(slot);
        if !self.touch_display_slot(slot) {
            return self.slot_view(slot);
        }
        let first = self.creative_screen.first_row(&self.pack.names);
        let items = self.creative_screen.items(&self.pack.names);
        items.get(first * 9 + slot - Creative::DISPLAY).cloned()
    }

    /// A finger on a screen with slots: true when it is the touch inventory's (the pointer then
    /// leaves it alone).
    pub(in crate::game) fn slot_touch_event(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        t: &Touch,
    ) -> bool {
        if !self.slot_touch_screen() {
            self.touch.slots = SlotTouch::default();
            return false;
        }
        if let Some((finger, last)) = self.touch.slots.turning
            && finger == t.id
        {
            let scale = f64::from(self.touch_gui_batch().scale);
            self.touch.slots.spin += ((t.location.x - last) / scale) as f32 * TURN;
            self.touch.slots.turning = match t.phase {
                TouchPhase::Ended | TouchPhase::Cancelled => None,
                _ => Some((finger, t.location.x)),
            };
            return true;
        }
        match t.phase {
            TouchPhase::Started => {
                if self.touch.slots.down.is_some()
                    || self.touch.slots.turning.is_some()
                    || self.touch.slots.spread.is_some()
                {
                    return true;
                }
                let batch = self.touch_gui_batch();
                self.bedrock_side(t.location.x as f32 / batch.scale, batch.width);
                if self.touch_on_player(t.location) {
                    self.touch.slots.turning = Some((t.id, t.location.x));
                    return true;
                }
                let (slot, outside) = self.slot_touch_at(t.location);
                let picked = self.touch.slots.picked.is_some();
                if slot.is_none() && !(outside && picked) {
                    // A button, a tab, the recipe book: the pointer's, and the pick is dropped.
                    if picked {
                        self.touch_unpick(event_loop, id, t.device_id);
                    }
                    return false;
                }
                self.touch.slots.down = Some(Down {
                    touch: *t,
                    slot,
                    at: Instant::now(),
                    held: false,
                });
                true
            }
            TouchPhase::Moved => {
                if let Some(spread) = &self.touch.slots.spread {
                    if spread.finger == t.id {
                        self.touch_spread_over(t.location);
                    }
                    return true;
                }
                let Some(down) = &self.touch.slots.down else {
                    return false;
                };
                if down.touch.id != t.id {
                    return true;
                }
                let layout = self.touch_layout();
                let slop = horizon_client::touch::slop(layout.width, layout.height);
                let (dx, dy) = (
                    t.location.x - down.touch.location.x,
                    t.location.y - down.touch.location.y,
                );
                if (dx * dx + dy * dy).sqrt() < f64::from(slop) {
                    return true;
                }
                // From a slot with a stack picked (or the half the finger held): drawn across
                // slots, which share it out. A tab's items still scroll under a finger.
                if self.touch.slots.picked.is_some()
                    && let Some(slot) = down.slot
                    && !self.touch_display_slot(slot)
                {
                    let finger = down.touch.id;
                    self.touch.slots.down = None;
                    self.touch.slots.spread = Some(Spread {
                        finger,
                        slots: Vec::new(),
                    });
                    self.touch_spread_add(slot);
                    self.touch_spread_over(t.location);
                    return true;
                }
                if down.held {
                    return true;
                }
                // It moves: the pointer's from where it landed (a list scrolls, a scrollbar
                // drags).
                let start = self.touch.slots.down.take().expect("down").touch;
                let gui_scale = self.touch_gui_batch().scale;
                self.touch
                    .pointer
                    .set_gui_scale(gui_scale, self.settings.mouse_wheel_sensitivity);
                let now = Instant::now();
                let mut events = self.touch.pointer.event(&start, slop, now);
                events.extend(self.touch.pointer.event(t, slop, now));
                for mouse in events {
                    self.handle_window_event(event_loop, id, mouse, InputOrigin::User);
                }
                true
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(spread) = self.touch.slots.spread.take_if(|s| s.finger == t.id) {
                    if t.phase == TouchPhase::Ended {
                        self.touch_spread_end(
                            event_loop,
                            id,
                            t.device_id,
                            spread.slots,
                            t.location,
                        );
                    }
                    return true;
                }
                let Some(down) = self.touch.slots.down.take_if(|d| d.touch.id == t.id) else {
                    return self.touch.slots.down.is_some();
                };
                if t.phase == TouchPhase::Ended && !down.held {
                    self.touch_slot_tap(event_loop, id, t.device_id, down.slot, t.location);
                }
                true
            }
        }
    }

    /// Once a frame: a finger resting on a stack picks half of it.
    pub(in crate::game) fn slot_touch_frame(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
    ) {
        if !self.slot_touch_screen() {
            self.touch.slots = SlotTouch::default();
            return;
        }
        let Some(down) = &self.touch.slots.down else {
            return;
        };
        let (Some(slot), false, device) = (down.slot, down.held, down.touch.device_id) else {
            return;
        };
        if down.at.elapsed() < HOLD || self.touch_display_slot(slot) || !self.touch_has_stack(slot)
        {
            return;
        }
        if let Some(down) = &mut self.touch.slots.down {
            down.held = true;
        }
        self.touch_pick(event_loop, id, device, slot, true);
        self.touch_vibrate(super::SPLIT_BUZZ);
    }

    /// A tap at `at`: on a slot (`Some`) or outside the window.
    fn touch_slot_tap(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        slot: Option<usize>,
        at: PhysicalPosition<f64>,
    ) {
        let picked = self.touch.slots.picked;
        match (picked, slot) {
            (None, Some(slot)) => {
                if self.touch_has_stack(slot) {
                    self.touch_pick(event_loop, id, device, slot, false);
                }
            }
            (None, None) => {}
            (Some(p), None) => {
                // Out of the window: thrown, as a mouse carrying it would.
                self.touch_grab(event_loop, id, device, p);
                self.touch_click_at(event_loop, id, device, at, MouseButton::Left, false);
                self.touch_unpick(event_loop, id, device);
            }
            (Some(p), Some(slot)) if slot == p.slot => {
                if !p.half && p.at.elapsed() < DOUBLE_TAP && !self.touch_display_slot(slot) {
                    let before = self.touch_contents();
                    self.touch_slot_click(event_loop, id, device, slot, MouseButton::Left, true);
                    self.touch_slide(&before, p.slot);
                }
                self.touch_unpick(event_loop, id, device);
            }
            (Some(p), Some(slot)) => {
                let before = self.touch_contents();
                self.touch_grab(event_loop, id, device, p);
                self.touch_slot_click(event_loop, id, device, slot, MouseButton::Left, false);
                // What the hand still holds goes back where the pick was: a swapped stack, what did
                // not fit (onto a creative tab, which takes it away).
                if self.carried.is_some() {
                    self.touch_slot_click(event_loop, id, device, p.slot, MouseButton::Left, false);
                }
                self.touch_slide(&before, p.slot);
                self.touch_unpick(event_loop, id, device);
            }
        }
    }

    /// The slot under a finger drawing the pick across slots joins them.
    fn touch_spread_over(&mut self, at: PhysicalPosition<f64>) {
        if let (Some(slot), _) = self.slot_touch_at(at) {
            self.touch_spread_add(slot);
        }
    }

    /// Slot `slot` joins the slots the pick is shared out among: not the pick's own, a tab's
    /// display slot or a crafting result (200).
    fn touch_spread_add(&mut self, slot: usize) {
        let Some(p) = self.touch.slots.picked else {
            return;
        };
        if slot == p.slot || slot == 200 || self.touch_display_slot(slot) {
            return;
        }
        if let Some(spread) = &mut self.touch.slots.spread
            && !spread.slots.contains(&slot)
        {
            spread.slots.push(slot);
        }
    }

    /// The finger drawn across `slots` lifted at `at`: the pick shared out among them, as the
    /// mouse's drag does it (pressed on the first, moved over the rest, let go), what the hand
    /// keeps going back where the pick was. Across one slot it is a tap there.
    fn touch_spread_end(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        slots: Vec<usize>,
        at: PhysicalPosition<f64>,
    ) {
        let Some(p) = self.touch.slots.picked else {
            return;
        };
        match slots[..] {
            [] => {}
            [slot] => self.touch_slot_tap(event_loop, id, device, Some(slot), at),
            _ => {
                let before = self.touch_contents();
                self.touch_grab(event_loop, id, device, p);
                for (i, &slot) in slots.iter().enumerate() {
                    let position = self.touch_slot_center(slot);
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::CursorMoved {
                            device_id: device,
                            position,
                        },
                        InputOrigin::User,
                    );
                    if i == 0 {
                        self.touch_mouse_button(event_loop, id, device, ElementState::Pressed);
                    }
                }
                self.touch_mouse_button(event_loop, id, device, ElementState::Released);
                if self.carried.is_some() {
                    if self.touch_display_slot(p.slot) {
                        // A tab's item: what is left goes, as the creative screen's cursor does.
                        self.carried = None;
                    } else {
                        self.touch_slot_click(
                            event_loop,
                            id,
                            device,
                            p.slot,
                            MouseButton::Left,
                            false,
                        );
                    }
                }
                self.touch_slide(&before, p.slot);
                self.touch_unpick(event_loop, id, device);
            }
        }
    }

    fn touch_mouse_button(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        state: ElementState,
    ) {
        self.handle_window_event(
            event_loop,
            id,
            WindowEvent::MouseInput {
                device_id: device,
                state,
                button: MouseButton::Left,
            },
            InputOrigin::User,
        );
    }

    /// What every slot holds now (a creative tab's display slots aside: they never change).
    fn touch_contents(&mut self) -> Vec<(usize, Option<ItemStack>)> {
        // Bedrock Inventory: its inventory's window has them all, whichever is worked with.
        self.bedrock_for_slot(0);
        let slots: Vec<usize> = self
            .slots()
            .into_iter()
            .filter(|&s| !self.touch_display_slot(s))
            .collect();
        slots.into_iter().map(|s| (s, self.slot_view(s))).collect()
    }

    /// Each slot that gained a stack since `before` gets it sliding in from the slot that lost
    /// it (else from `from`, the picked slot: a creative tab's).
    fn touch_slide(&mut self, before: &[(usize, Option<ItemStack>)], from: usize) {
        let same = |a: &ItemStack, b: &ItemStack| a.item == b.item && a.extra == b.extra;
        let (mut lost, mut gained) = (Vec::new(), Vec::new());
        for (slot, old) in before {
            let new = self.slot_view(*slot);
            match (old, new) {
                (old, new) if *old == new => {}
                (Some(o), Some(n)) if same(o, &n) => {
                    if n.count > o.count {
                        gained.push((*slot, n));
                    } else {
                        lost.push((*slot, o.clone()));
                    }
                }
                (old, new) => {
                    lost.extend(old.clone().map(|o| (*slot, o)));
                    gained.extend(new.map(|n| (*slot, n)));
                }
            }
        }
        let at = Instant::now();
        for (to, stack) in gained {
            let source = lost.iter().find(|(_, s)| same(s, &stack)).map(|(s, _)| *s);
            self.touch.slots.slides.push(Slide {
                stack,
                from: source.unwrap_or(from),
                to,
                at,
            });
        }
    }

    /// Takes the picked stack into the hand: all of it, half of it (the right button), or a full
    /// stack of a creative tab's item (Shift-click on it).
    fn touch_grab(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        p: Picked,
    ) {
        if self.touch_display_slot(p.slot) {
            self.touch_slot_click(event_loop, id, device, p.slot, MouseButton::Left, true);
        } else {
            let button = if p.half {
                MouseButton::Right
            } else {
                MouseButton::Left
            };
            self.touch_slot_click(event_loop, id, device, p.slot, button, false);
        }
    }

    /// Picks a stack (drawn by `draw_slot_pick`).
    fn touch_pick(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        slot: usize,
        half: bool,
    ) {
        self.touch.slots.picked = Some(Picked {
            slot,
            half,
            at: Instant::now(),
        });
        self.touch_unhover(event_loop, id, device);
    }

    /// Lets go of the pick.
    fn touch_unpick(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, device: DeviceId) {
        self.touch.slots.picked = None;
        self.touch_unhover(event_loop, id, device);
    }

    /// The cursor out of the window: no slot shows hovered, no tooltip follows the finger.
    fn touch_unhover(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, device: DeviceId) {
        let away = PhysicalPosition::new(-1000.0, -1000.0);
        self.handle_window_event(
            event_loop,
            id,
            WindowEvent::CursorMoved {
                device_id: device,
                position: away,
            },
            InputOrigin::User,
        );
    }

    /// The middle of a slot, in window pixels.
    fn touch_slot_center(&mut self, slot: usize) -> PhysicalPosition<f64> {
        self.bedrock_for_slot(slot);
        let batch = self.touch_gui_batch();
        let (ox, oy) = self.inventory_origin(&batch);
        let (sx, sy) = self.slot_position(slot);
        PhysicalPosition::new(
            f64::from((ox + sx + 8.0) * batch.scale),
            f64::from((oy + sy + 8.0) * batch.scale),
        )
    }

    /// Where `slot` is on the screen (GUI pixels), in whichever window shows it.
    fn touch_slot_gui(&mut self, slot: usize, batch: &HudBatch) -> (f32, f32) {
        self.bedrock_for_slot(slot);
        let (ox, oy) = self.inventory_origin(batch);
        let (sx, sy) = self.slot_position(slot);
        (ox + sx, oy + sy)
    }

    fn touch_display_slot(&self, slot: usize) -> bool {
        self.screen == Screen::Creative
            && (Creative::DISPLAY..Creative::DISPLAY + 54).contains(&slot)
    }

    fn touch_slot_click(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        slot: usize,
        button: MouseButton,
        shift: bool,
    ) {
        let at = self.touch_slot_center(slot);
        self.touch_click_at(event_loop, id, device, at, button, shift);
    }

    /// A mouse click at a window position, Shift held for it when asked.
    fn touch_click_at(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        device: DeviceId,
        at: PhysicalPosition<f64>,
        button: MouseButton,
        shift: bool,
    ) {
        let held = shift.then(|| {
            horizon_client_input::modifiers::hold(
                &mut self.modifiers,
                &mut self.physical_keys,
                KeyCode::ShiftLeft,
            )
        });
        self.handle_window_event(
            event_loop,
            id,
            WindowEvent::CursorMoved {
                device_id: device,
                position: at,
            },
            InputOrigin::User,
        );
        for state in [ElementState::Pressed, ElementState::Released] {
            self.handle_window_event(
                event_loop,
                id,
                WindowEvent::MouseInput {
                    device_id: device,
                    state,
                    button,
                },
                InputOrigin::User,
            );
        }
        if let Some(held) = held {
            horizon_client_input::modifiers::restore(
                &mut self.modifiers,
                &mut self.physical_keys,
                held,
            );
        }
    }

    /// The touch inventory over the screen: stacks sliding to their slots, and the pick.
    pub(in crate::game) fn draw_slot_touch(&mut self, batch: &mut HudBatch) {
        if !self.slot_touch_screen() {
            return;
        }
        self.draw_slides(batch);
        self.draw_slot_pick(batch);
        self.draw_spread(batch);
    }

    /// The slots a finger drew the pick across, framed, each with what it would get: the stack
    /// shared out evenly (AbstractContainerMenu.getQuickCraftPlaceCount).
    fn draw_spread(&mut self, batch: &mut HudBatch) {
        let (Some(p), Some(spread)) = (self.touch.slots.picked, &self.touch.slots.spread) else {
            return;
        };
        let slots = spread.slots.clone();
        let Some(stack) = self.touch_stack(p.slot) else {
            return;
        };
        let max = self
            .res
            .items()
            .get(&stack.item)
            .map_or(64, |i| i.max_stack)
            .max(1);
        let count = if self.touch_display_slot(p.slot) {
            max
        } else if p.half {
            stack.count.div_ceil(2)
        } else {
            stack.count
        };
        let share = count / slots.len().max(1) as u32;
        let white = rgba(255, 255, 255, 255);
        for slot in slots {
            let there = self.slot_view(slot);
            if there
                .as_ref()
                .is_some_and(|s| s.item != stack.item || s.extra != stack.extra)
            {
                continue;
            }
            let (x, y) = self.touch_slot_gui(slot, batch);
            let atlas = &self.gui.atlas;
            batch.fill(atlas, x - 1.0, y - 1.0, 18.0, 1.0, white);
            batch.fill(atlas, x - 1.0, y + 16.0, 18.0, 1.0, white);
            batch.fill(atlas, x - 1.0, y, 1.0, 16.0, white);
            batch.fill(atlas, x + 16.0, y, 1.0, 16.0, white);
            if share > 0 {
                let mut shown = stack.clone();
                shown.count = (there.map_or(0, |s| s.count) + share).min(max);
                self.draw_slot_item(batch, None, usize::MAX, &Some(shown), x, y);
            }
        }
    }

    /// A moved stack slides from where it was (eased out), its slot showing it once it lands.
    fn draw_slides(&mut self, batch: &mut HudBatch) {
        let slides = std::mem::take(&mut self.touch.slots.slides);
        let mut kept = Vec::new();
        for slide in slides {
            let t = slide.at.elapsed().as_secs_f32() / SLIDE.as_secs_f32();
            if t >= 1.0 {
                continue;
            }
            let eased = 1.0 - (1.0 - t).powi(3);
            let (fx, fy) = self.touch_slot_gui(slide.from, batch);
            let (tx, ty) = self.touch_slot_gui(slide.to, batch);
            // The slot's own drawing waits under its background colour.
            let slot_grey = rgba(139, 139, 139, 255);
            batch.fill(&self.gui.atlas, tx, ty, 16.0, 16.0, slot_grey);
            let (x, y) = (fx + (tx - fx) * eased, fy + (ty - fy) * eased);
            self.draw_slot_item(batch, None, usize::MAX, &Some(slide.stack.clone()), x, y);
            kept.push(slide);
        }
        self.touch.slots.slides = kept;
    }

    /// The pick as Bedrock shows it: the slot framed, a bar over it for half of the stack, and its
    /// tooltip beside the window at the slot's height, near the pick and over no slot (under the
    /// window where the screen is too narrow for that).
    fn draw_slot_pick(&mut self, batch: &mut HudBatch) {
        let Some(p) = self.touch.slots.picked else {
            return;
        };
        let Some(stack) = self.touch_stack(p.slot) else {
            return;
        };
        let (x, y) = self.touch_slot_gui(p.slot, batch);
        let white = rgba(255, 255, 255, 255);
        let atlas = &self.gui.atlas;
        batch.fill(atlas, x - 1.0, y - 1.0, 18.0, 1.0, white);
        batch.fill(atlas, x - 1.0, y + 16.0, 18.0, 1.0, white);
        batch.fill(atlas, x - 1.0, y, 1.0, 16.0, white);
        batch.fill(atlas, x + 16.0, y, 1.0, 16.0, white);
        batch.fill(atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 60));
        if p.half {
            batch.fill(atlas, x + 1.0, y + 1.0, 14.0, 2.0, rgba(60, 60, 60, 255));
            batch.fill(atlas, x + 1.0, y + 1.0, 7.0, 2.0, rgba(85, 255, 85, 255));
        }
        let window = self.window_image(batch);
        // Bedrock Inventory's two windows count as one: no slot of either under the tooltip.
        let (left, right) = if self.screen == Screen::Creative {
            Self::creative_bounds(batch)
        } else {
            (window.left as f32, (window.left + window.width) as f32)
        };
        // Clear of the window's frame and of the tooltip's own, on the roomier side; lines too
        // long for it wrap (a phone's GUI is narrow).
        const GAP: f32 = 8.0;
        let (room_right, room_left) = (batch.width - right - GAP - 4.0, left - GAP - 4.0);
        let room = room_right.max(room_left);
        let lines = self.wrap_tooltip(self.stack_tooltip(&stack), room);
        let width = lines
            .iter()
            .map(|(t, _)| self.gui.atlas.font.width_text(t))
            .fold(0.0, f32::max);
        let (tx, ty) = if width <= room_right {
            (right + GAP, y + 4.0)
        } else if width <= room_left {
            (left - GAP - width, y + 4.0)
        } else {
            let bottom = (window.top + window.height) as f32;
            ((x + 8.0 - width / 2.0).max(4.0), bottom + GAP)
        };
        // draw_tooltip puts the box 12 right of and 12 above the point it is given.
        self.draw_tooltip_lines(batch, &lines, tx - 12.0, ty + 12.0);
    }

    /// Tooltip lines wrapped to `width` GUI pixels (left whole when that is too narrow to read).
    fn wrap_tooltip(
        &self,
        lines: Vec<(horizon_client::text::Text, u32)>,
        width: f32,
    ) -> Vec<(horizon_client::text::Text, u32)> {
        if width < 60.0 {
            return lines;
        }
        let font = &self.gui.atlas.font;
        lines
            .into_iter()
            .flat_map(|(text, colour)| {
                font.split(&text, width)
                    .into_iter()
                    .map(move |part| (part, colour))
            })
            .collect()
    }
}
