//! User-added keys share drawing/hit geometry and retain per-finger ownership.
use super::*;
use layout::Placement;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomButton {
    pub key: String,
    #[serde(default)]
    pub keys: Vec<String>,
    /// An empty string intentionally leaves the button unlabelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub layouts: [Placement; 3],
}
impl CustomButton {
    pub fn new(key: String, index: usize) -> Self {
        Self {
            key,
            keys: Vec::new(),
            label: None,
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
    pub fn tokens(&self) -> Vec<String> {
        let mut tokens: Vec<String> = self
            .keys
            .iter()
            .filter(|key| key.as_str() != "key.keyboard.unknown" && Self::valid_token(key.as_str()))
            .cloned()
            .collect();
        if tokens.is_empty() && self.key != "key.keyboard.unknown" && Self::valid_token(&self.key) {
            tokens.push(self.key.clone());
        }
        tokens
    }
    pub fn valid(&self) -> bool {
        self.key == "key.keyboard.unknown"
            || Self::valid_token(&self.key)
            || self.keys.iter().any(|key| Self::valid_token(key.as_str()))
    }
    pub fn sanitize(&mut self) {
        self.keys.retain(|key| {
            key.as_str() != "key.keyboard.unknown" && Self::valid_token(key.as_str())
        });
        self.keys.dedup();
        if self.keys.len() > 8 {
            self.keys.truncate(8);
        }
        if !Self::valid_token(&self.key) {
            self.key = "key.keyboard.unknown".into();
        }
        if self.key == "key.keyboard.unknown" && !self.keys.is_empty() {
            self.key = self.keys[0].clone();
        }
        if let Some(label) = &mut self.label {
            if label.chars().count() > 32 {
                *label = label.chars().take(32).collect();
            }
        }
    }
    pub fn label(&self) -> String {
        if let Some(label) = &self.label {
            return label.clone();
        }
        let tokens = self.tokens();
        if tokens.is_empty() {
            return "Button".into();
        }
        tokens
            .iter()
            .map(|key| {
                if let Some(name) = key.strip_prefix("action:") {
                    crate::text::Text::translate(name, &[]).plain()
                } else {
                    crate::keybinds::display_name(key).plain()
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
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
        if self.custom_fingers.contains_key(&id) {
            return self.custom_update(id, point, false, ended);
        }
        if started
            && self.custom_at(point).is_none()
            && let Some(token) = self.control_at(point).and_then(Self::inventory_token)
        {
            let pressed = !self.custom_fingers.values().flatten().any(|v| v == token);
            self.custom_fingers.insert(id, vec![token.into()]);
            self.fingers.insert(id, (Role::Ignore, point));
            if pressed {
                self.custom_events.push((token.into(), true));
            }
            return true;
        }
        false
    }
    pub(super) fn fit_placement(&self, mut p: Placement) -> Placement {
        p.size = (p.size * self.settings.button_scale)
            .min(self.aspect())
            .min(1.0);
        let half = [p.size / self.aspect() / 2.0, p.size / 2.0];
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
        if let Some(keys) = self.custom_fingers.get(&id).cloned() {
            if ended {
                self.custom_fingers.remove(&id);
                self.fingers.remove(&id);
                let remaining: std::collections::BTreeSet<_> =
                    self.custom_fingers.values().flatten().cloned().collect();
                for key in keys {
                    if !remaining.contains(&key) {
                        self.custom_events.push((key, false));
                    }
                }
            }
            return true;
        }
        if started && let Some(index) = self.custom_at(point) {
            let keys = self.settings.custom_buttons[index].tokens();
            if keys.is_empty() {
                return false;
            }
            let pressed: std::collections::BTreeSet<_> =
                self.custom_fingers.values().flatten().cloned().collect();
            self.custom_fingers.insert(id, keys.clone());
            self.fingers.insert(id, (Role::Ignore, point));
            for key in keys {
                if !pressed.contains(&key) {
                    self.custom_events.push((key, true));
                }
            }
            self.clicked = true;
            return true;
        }
        false
    }
    pub fn held_custom_tokens(&self) -> impl Iterator<Item = &str> {
        self.custom_fingers.values().flatten().map(String::as_str)
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.custom_events)
    }
    pub(super) fn clear_custom(&mut self) {
        let keys: std::collections::BTreeSet<_> = std::mem::take(&mut self.custom_fingers)
            .into_values()
            .flatten()
            .collect();
        self.custom_events
            .extend(keys.into_iter().map(|k| (k, false)));
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        let held: std::collections::BTreeSet<&str> = self
            .custom_fingers
            .values()
            .flatten()
            .map(String::as_str)
            .collect();
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            let p = self.custom_placement(i);
            let size = p.size * b.height;
            let rect = [
                p.center[0] * b.width - size / 2.0,
                p.center[1] * b.height - size / 2.0,
                size,
                size,
            ];
            let pressed = button
                .tokens()
                .iter()
                .any(|key| held.contains(key.as_str()));
            let opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            if opacity <= 0.0 {
                continue;
            }
            text_button(
                b,
                a,
                rect,
                &button.label(),
                pressed,
                rgba(255, 255, 255, (opacity * 255.0) as u8),
                true,
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
