//! User-added keys share drawing/hit geometry and retain per-finger ownership.
use super::*;
use layout::Placement;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomButton {
    pub key: String,
    pub layouts: [Placement; 3],
    #[serde(default)]
    pub gestures: super::gestures::Bindings,
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,
}
fn default_text_scale() -> f32 {
    1.0
}
impl CustomButton {
    pub fn new(key: String, index: usize) -> Self {
        Self {
            key,
            gestures: super::gestures::Bindings::default(),
            text: String::new(),
            text_scale: 1.0,
            layouts: [Placement::new(
                0.68 + (index % 3) as f32 * 0.1,
                0.35 + (index / 3 % 3) as f32 * 0.12,
                0.105,
            ); 3],
        }
    }
    pub fn valid_token(token: &str) -> bool {
        token == "key.keyboard.unknown"
            || crate::keybinds::key_code(token).is_some()
            || token
                .strip_prefix("action:")
                .is_some_and(|name| crate::keybinds::MAPPINGS.iter().any(|m| m.0 == name))
    }
    pub fn label(&self) -> String {
        if !self.text.is_empty() {
            return self.text.clone();
        }
        if self.key == "key.keyboard.unknown" {
            if let Some(binding) = self.gestures.single.first() {
                if let Some(action) = binding.token.strip_prefix("action:") {
                    return crate::text::Text::translate(action, &[]).plain();
                }
                return crate::keybinds::display_name(&binding.token).plain();
            }
            return "Button".into();
        }
        if let Some(name) = self.key.strip_prefix("action:") {
            return crate::text::Text::translate(name, &[]).plain();
        }
        crate::keybinds::display_name(&self.key).plain()
    }
}
pub(super) fn choices(raw_keys: bool) -> Vec<(String, String)> {
    if raw_keys {
        crate::keybinds::available_keys()
            .iter()
            .map(|(name, _)| ((*name).into(), crate::keybinds::display_name(name).plain()))
            .collect()
    } else {
        crate::keybinds::MAPPINGS
            .iter()
            .filter(|m| m.1 != "key.keyboard.unknown")
            .map(|(name, _, _)| {
                (
                    format!("action:{name}"),
                    crate::text::Text::translate(name, &[]).plain(),
                )
            })
            .collect()
    }
}
impl TouchControls {
    pub(super) fn inventory_token(control: Control) -> Option<&'static str> {
        match control {
            Control::Sneak => Some("key.keyboard.left.shift"),
            Control::Attack => Some("action:key.attack"),
            Control::Use => Some("action:key.use"),
            _ => None,
        }
    }
    pub fn inventory_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if self.custom_runtime.owns(id) {
            self.custom_runtime.motion(id, point, self.aspect());
            return self.custom_update(id, point, false, ended);
        }
        if started
            && self.custom_at(point).is_none()
            && let Some(token) = self.control_at(point).and_then(Self::inventory_token)
        {
            self.custom_runtime.direct(id, token);
            self.fingers.insert(id, (Role::Ignore, point));
            return true;
        }
        false
    }
    pub(super) fn fit_placement(&self, mut p: Placement) -> Placement {
        p.width = (p.width() * self.settings.button_scale).min(self.aspect());
        p.height = (p.height() * self.settings.button_scale).min(1.0);
        p.size = (p.size * self.settings.button_scale)
            .min(self.aspect())
            .min(1.0);
        let half = [p.width / self.aspect() / 2.0, p.height / 2.0];
        for (i, half) in half.into_iter().enumerate() {
            p.center[i] = p.center[i].clamp(half, 1.0 - half);
        }
        p
    }
    pub(super) fn custom_placement(&self, index: usize) -> Placement {
        let mut p = self.settings.custom_buttons[index].layouts[self.settings.mode.index()];
        if self.settings.left_handed {
            p.center[0] = 1.0 - p.center[0];
        }
        self.fit_placement(p)
    }
    pub fn custom_at(&self, point: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len())
            .rev()
            .find(|&i| layout::contains(self.custom_placement(i).rect(self.aspect()), point))
    }
    pub(super) fn custom_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if self.custom_runtime.owns(id) {
            self.custom_runtime.motion(id, point, self.aspect());
            if ended {
                self.custom_runtime.end(id, Instant::now());
                self.fingers.remove(&id);
            } else if let Some((_, previous)) = self.fingers.get_mut(&id) {
                *previous = point;
            }
            return true;
        }
        if started && let Some(index) = self.custom_at(point) {
            let key = self.settings.custom_buttons[index].key.clone();
            let mut bindings = self.settings.custom_buttons[index].gestures.clone();
            if bindings.single.is_empty() && key != "key.keyboard.unknown" {
                bindings.single.push(super::gestures::Binding {
                    token: key.clone(),
                    toggle: false,
                });
            }
            if bindings.single.is_empty() && bindings.double.is_empty() && bindings.hold.is_empty()
            {
                return false;
            }
            self.custom_runtime
                .start(id, index, bindings, Instant::now());
            self.custom_runtime.motion(id, point, self.aspect());
            self.fingers.insert(id, (Role::Ignore, point));
            self.clicked = true;
            return true;
        }
        false
    }
    pub fn held_custom_tokens(&self) -> impl Iterator<Item = &str> {
        self.custom_runtime.tokens()
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        let mut events = self.custom_runtime.poll(Instant::now());
        events.append(&mut self.custom_events);
        events
    }
    pub fn drop_sneak_toggles(&mut self, physical_binding: &str) {
        self.custom_runtime
            .clear_toggle_tokens(|token| token == "action:key.sneak" || token == physical_binding);
    }
    pub(super) fn clear_custom(&mut self) {
        self.custom_events.clear();
        self.custom_runtime.clear();
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            let p = self.custom_placement(i);
            let [x, y, w, h] = p.rect(self.aspect());
            let rect = [x * b.width, y * b.height, w * b.width, h * b.height];
            let pressed = self.held_custom_tokens().any(|key| {
                key == button.key
                    || button
                        .gestures
                        .single
                        .iter()
                        .chain(&button.gestures.double)
                        .chain(&button.gestures.hold)
                        .any(|b| b.token == key)
            });
            let opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            if opacity <= 0.0 {
                continue;
            }
            text_button_scaled(
                b,
                a,
                rect,
                &button.label(),
                pressed,
                rgba(255, 255, 255, (opacity * 255.0) as u8),
                true,
                button.text_scale,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_modifier_and_mouse_hold_independently_and_cancel_cleanly() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.inventory_mode = true;
        t.settings.mode = Mode::Crosshair;
        for (id, control, token) in [
            (1, Control::Sneak, "key.keyboard.left.shift"),
            (2, Control::Attack, "action:key.attack"),
            (3, Control::Use, "action:key.use"),
        ] {
            assert!(t.visible(control));
            assert!(t.inventory_update(id, t.placement(control).center, true, false));
            assert_eq!(t.key_events(), [(token.into(), true)]);
        }
        t.clear_world_gesture();
        assert_eq!(t.held_custom_tokens().count(), 3);
        assert_eq!(t.poll_state(), PadState::default());
        assert!(t.inventory_update(2, [0.0, 0.0], false, true));
        assert_eq!(t.key_events(), [("action:key.attack".into(), false)]);
        t.clear();
        let released = t.key_events();
        assert!(released.contains(&("key.keyboard.left.shift".into(), false)));
        assert!(released.contains(&("action:key.use".into(), false)));
    }
    #[test]
    fn inventory_overlay_preserves_configured_visibility_and_placements() {
        let mut touch = TouchControls::default();
        for mode in Mode::ALL {
            touch.settings.mode = mode;
            touch.settings.button_visibility[Control::Attack as usize] = 2;
            let before: Vec<_> = Control::ALL
                .into_iter()
                .map(|c| (c, touch.visible(c), touch.placement(c).center))
                .collect();
            touch.inventory_mode = true;
            for (control, visible, center) in before {
                assert_eq!(touch.visible(control), visible);
                assert_eq!(touch.placement(control).center, center);
            }
            touch.inventory_mode = false;
        }
    }
    #[test]
    fn unassigned_button_emits_no_input() {
        let mut touch = TouchControls::default();
        touch.viewport(1920, 1080);
        touch
            .settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.unknown".into(), 0));
        let point = touch.custom_placement(0).center;
        touch.update(1, point, true, false);
        assert!(touch.key_events().is_empty());
    }
    #[test]
    fn custom_keys_hold_through_motion_and_shared_keys_release_only_after_last_finger() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let p = t.custom_placement(0).center;
        t.update(1, p, true, false);
        t.update(2, p, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        t.update(1, [0.99, 0.1], false, false);
        assert!(t.key_events().is_empty());
        t.update(1, p, false, true);
        assert!(t.key_events().is_empty());
        t.update(2, p, false, true);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn cancellation_releases_all_custom_keys_and_scaling_preserves_saved_sizes() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let stored = t.settings.custom_buttons[0].layouts[0].size;
        t.settings.button_scale = 2.0;
        assert_eq!(t.custom_placement(0).size, stored * 2.0);
        t.update(1, t.custom_placement(0).center, true, false);
        t.key_events();
        t.clear();
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
        assert_eq!(t.settings.custom_buttons[0].layouts[0].size, stored);
    }
}
