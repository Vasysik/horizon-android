//! Delay a direct slot touch until it is a tap, a drag, or a timed Drop.
use super::*;
use horizon_client::touch::drop_gesture::{DropGesture, Outcome};
use winit::event::{Touch, TouchPhase};

pub(super) struct ItemTouch {
    start: Touch,
    slot: usize,
    screen: Screen,
    container: Option<(i32, i32, i32)>,
    gesture: DropGesture,
    point: [f32; 2],
    viewport: [u32; 2],
}
impl ItemTouch {
    pub(super) fn id(&self) -> u64 {
        self.start.id
    }
}
impl Game<'_> {
    fn android_drop_valid(&self, touch: &ItemTouch) -> bool {
        self.focused
            && self.screen == touch.screen
            && self.container == touch.container
            && self.carried.is_none()
            && self.window.as_ref().is_some_and(|window| {
                let size = window.inner_size();
                [size.width, size.height] == touch.viewport
            })
            && (self.screen != Screen::Playing || self.selected == touch.slot)
    }
    fn android_drop_one(&mut self, slot: usize) {
        if self.screen == Screen::Playing {
            self.drop_held(false);
        } else {
            self.menu_click(Some(slot), 0, horizon_client::menu::Input::Throw);
            if let Some(container) = self.container {
                self.container_changed(container);
            }
        }
    }
    fn android_replay_item_press(
        &mut self,
        events: &dyn event_loop::EventLoopCtl,
        id: WindowId,
        start: &Touch,
    ) {
        self.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: start.location,
            },
        );
        if let Err(error) = self.android_menu_mouse(events, id, false, true) {
            log::warn!("Touch slot press: {error}");
        }
    }
    pub(super) fn android_item_touch(
        &mut self,
        touch: &Touch,
        events: &dyn event_loop::EventLoopCtl,
        id: WindowId,
    ) -> bool {
        if touch.phase == TouchPhase::Started {
            self.pad.item_cancelled.remove(&touch.id);
        } else if self.pad.item_cancelled.contains(&touch.id) {
            if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pad.item_cancelled.remove(&touch.id);
            }
            return true;
        }
        let Some(batch) = self.gui_scale() else {
            return false;
        };
        let point = [
            touch.location.x as f32 / batch.scale,
            touch.location.y as f32 / batch.scale,
        ];
        if let Some(mut pending) = self.pad.item_touch.take() {
            if pending.start.id != touch.id {
                self.pad.item_touch = Some(pending);
                return false;
            }
            pending.point = point;
            let outcome = pending.gesture.update(
                Instant::now(),
                point,
                touch.phase == TouchPhase::Ended,
                touch.phase == TouchPhase::Cancelled || !self.android_drop_valid(&pending),
            );
            self.pad.touch.drop_indicator = None;
            match outcome {
                Outcome::Wait => {
                    self.pad.item_touch = Some(pending);
                }
                Outcome::Drop => {
                    self.android_drop_one(pending.slot);
                    if touch.phase != TouchPhase::Ended {
                        self.pad.item_touch = Some(pending);
                    }
                }
                Outcome::Tap | Outcome::Drag if pending.screen != Screen::Playing => {
                    self.android_replay_item_press(events, id, &pending.start);
                    // The current move/release continues through the original drag handler.
                    return false;
                }
                Outcome::Cancel
                    if !matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) =>
                {
                    self.pad.item_cancelled.insert(touch.id);
                }
                _ => {}
            }
            return true;
        }
        if touch.phase != TouchPhase::Started
            || self.pad.touch_ui.virtual_mouse
            || self.carried.is_some()
            || !crate::touch_visible()
        {
            return false;
        }
        let normalized = [point[0] / batch.width, point[1] / batch.height];
        if matches!(self.screen, Screen::Inventory | Screen::Creative) && self.has_shift_down() {
            return false;
        }
        if self.pad.touch.control_at_point(normalized)
            && (!self.pad_in_menu() || self.android_inventory_controls())
        {
            return false;
        }
        let slot = match self.screen {
            Screen::Playing if self.player.is_some() => {
                let Some(slot) =
                    horizon_client::touch::hotbar_slot(batch.width, batch.height, point)
                else {
                    return false;
                };
                self.selected = slot;
                slot
            }
            Screen::Inventory | Screen::Creative => {
                let Some(slot) = self.slot_at(&batch, point[0], point[1]) else {
                    return false;
                };
                slot
            }
            _ => return false,
        };
        self.pad.item_touch = Some(ItemTouch {
            start: *touch,
            slot,
            screen: self.screen,
            container: self.container,
            gesture: DropGesture::new(Instant::now(), point),
            point,
            viewport: [
                (batch.width * batch.scale) as u32,
                (batch.height * batch.scale) as u32,
            ],
        });
        true
    }
    pub(super) fn android_poll_item_touch(&mut self) {
        self.pad.touch.drop_indicator = None;
        let Some(mut pending) = self.pad.item_touch.take() else {
            return;
        };
        if !self.android_drop_valid(&pending) {
            self.pad.item_cancelled.insert(pending.start.id);
            return;
        }
        let now = Instant::now();
        if pending.gesture.update(now, pending.point, false, false) == Outcome::Drop {
            self.android_drop_one(pending.slot);
        }
        if let Some(batch) = self.gui_scale() {
            self.pad.touch.drop_indicator = Some((
                [
                    pending.point[0] / batch.width,
                    pending.point[1] / batch.height,
                ],
                pending.gesture.progress(now),
            ));
        }
        self.pad.item_touch = Some(pending);
    }
}
