//! User-added keys share drawing/hit geometry and retain per-finger ownership.
use super::*;
use layout::Placement;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomButton {
    pub key: String,
    #[serde(default)]
    pub second_key: String,
    #[serde(default)]
    pub third_key: String,
    #[serde(default)]
    pub custom_label: String,
    #[serde(default)]
    pub always_visible: bool,
    pub layouts: [Placement; 3],
}
impl CustomButton {
    pub fn new(key: String, index: usize) -> Self {
        Self {
            key,
            second_key: String::new(),
            third_key: String::new(),
            custom_label: String::new(),
            always_visible: false,
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
            || crate::keybinds::mouse_button(token).is_some()
            || matches!(
                token,
                "mouse.scroll.up" | "mouse.scroll.down" | "mouse.virtual.toggle" | "mouse.look"
            )
            || token
                .strip_prefix("action:")
                .is_some_and(|name| crate::keybinds::MAPPINGS.iter().any(|m| m.0 == name))
    }
    pub fn is_keyboard_binding(token: &str) -> bool {
        !token.is_empty()
            && token != "key.keyboard.unknown"
            && crate::keybinds::key_code(token).is_some()
    }
    pub fn is_mouse_binding(token: &str) -> bool {
        !token.is_empty()
            && token != "key.keyboard.unknown"
            && !Self::is_keyboard_binding(token)
            && Self::valid_token(token)
    }
    /// Sets a single exclusive binding (Keyboard OR Mouse), clearing any secondary/tertiary slots.
    pub fn set_exclusive_binding(&mut self, token: String) {
        self.key = token;
        self.second_key.clear();
        self.third_key.clear();
    }
    pub fn event_slot(&self, slot: usize) -> &str {
        match slot {
            0 => &self.key,
            1 => &self.second_key,
            _ => &self.third_key,
        }
    }
    pub fn set_event_slot(&mut self, slot: usize, token: String) {
        match slot {
            0 => {
                self.key = if token.is_empty() {
                    "key.keyboard.unknown".into()
                } else {
                    token
                };
            }
            1 => {
                if (self.key.is_empty() || self.key == "key.keyboard.unknown")
                    && !token.is_empty()
                    && token != "key.keyboard.unknown"
                {
                    self.key = token;
                } else {
                    self.second_key = token;
                }
            }
            _ => {
                if (self.key.is_empty() || self.key == "key.keyboard.unknown")
                    && !token.is_empty()
                    && token != "key.keyboard.unknown"
                {
                    self.key = token;
                } else {
                    self.third_key = token;
                }
            }
        }
    }
    pub fn active_tokens(&self) -> Vec<String> {
        let mut out = Vec::new();
        for t in [&self.key, &self.second_key, &self.third_key] {
            if !t.is_empty()
                && t != "key.keyboard.unknown"
                && Self::valid_token(t)
                && !out.contains(t)
            {
                out.push(t.clone());
            }
        }
        out
    }
    pub fn label(&self) -> String {
        if !self.custom_label.trim().is_empty() {
            return self.custom_label.trim().to_owned();
        }
        let tokens = self.active_tokens();
        if tokens.is_empty() {
            return "Button".into();
        }
        let parts: Vec<String> = tokens.iter().map(|t| friendly_token_name(t)).collect();
        parts.join("+")
    }
}

pub fn friendly_token_name(token: &str) -> String {
    match token {
        "" | "key.keyboard.unknown" => "Unassigned".into(),
        "key.mouse.left" => "LMB (Attack)".into(),
        "key.mouse.right" => "RMB (Use)".into(),
        "key.mouse.middle" => "MMB (Pick)".into(),
        "key.mouse.4" => "Mouse 4".into(),
        "key.mouse.5" => "Mouse 5".into(),
        "mouse.scroll.up" => "Wheel Up".into(),
        "mouse.scroll.down" => "Wheel Down".into(),
        "mouse.virtual.toggle" => "Virtual Mouse".into(),
        "mouse.look" => "Mouse Look".into(),
        _ => {
            if let Some(name) = token.strip_prefix("action:") {
                let tr = crate::text::Text::translate(name, &[]).plain();
                if tr == name || tr.starts_with("key.") {
                    match name {
                        "key.attack" => "Attack / Mine".into(),
                        "key.use" => "Use / Place".into(),
                        "key.forward" => "Walk Forward".into(),
                        "key.left" => "Strafe Left".into(),
                        "key.back" => "Walk Back".into(),
                        "key.right" => "Strafe Right".into(),
                        "key.jump" => "Jump".into(),
                        "key.sneak" => "Sneak".into(),
                        "key.sprint" => "Sprint".into(),
                        "key.drop" => "Drop Item".into(),
                        "key.inventory" => "Inventory".into(),
                        "key.chat" => "Open Chat".into(),
                        "key.playerlist" => "Player List".into(),
                        "key.pickItem" => "Pick Block".into(),
                        "key.command" => "Command /".into(),
                        "key.toggleGui" => "Hide HUD (F1)".into(),
                        "key.screenshot" => "Screenshot (F2)".into(),
                        "key.togglePerspective" => "Perspective (F5)".into(),
                        "key.swapOffhand" => "Swap Hands (F)".into(),
                        "key.advancements" => "Advancements (L)".into(),
                        "key.debug.overlay" => "Debug Info (F3)".into(),
                        other => other.trim_start_matches("key.").into(),
                    }
                } else {
                    tr
                }
            } else {
                let d = crate::keybinds::display_name(token).plain();
                if d.starts_with("key.keyboard.") {
                    token.trim_start_matches("key.keyboard.").to_uppercase()
                } else {
                    d
                }
            }
        }
    }
}

pub fn choices(raw_keys: bool) -> Vec<(String, String)> {
    if raw_keys {
        crate::keybinds::available_keys()
            .iter()
            .map(|(name, _)| ((*name).into(), friendly_token_name(name)))
            .collect()
    } else {
        crate::keybinds::MAPPINGS
            .iter()
            .filter(|m| m.1 != "key.keyboard.unknown")
            .map(|(name, _, _)| {
                let tok = format!("action:{name}");
                let lbl = friendly_token_name(&tok);
                (tok, lbl)
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
        if self.custom_fingers.contains_key(&id) || (started && self.custom_at(point).is_some()) {
            return self.custom_update(id, point, started, ended);
        }
        if started
            && self.custom_at(point).is_none()
            && let Some(token) = self.control_at(point).and_then(Self::inventory_token)
        {
            let pressed = !self.custom_fingers.values().any(|v| v == token);
            self.custom_fingers.insert(id, token.into());
            self.fingers.insert(id, (Role::Ignore, point));
            if pressed {
                self.custom_events.push((token.into(), true));
            }
            return true;
        }
        false
    }
    /// Handles touch input for `visible_in_menu` GoyControl buttons & auxiliary controls while any
    /// menu (Main Title Menu, Inventory, Container, Pause, Options) is open.
    pub fn menu_update(&mut self, id: u64, point: [f32; 2], started: bool, ended: bool) -> bool {
        let handled = if self.custom_fingers.contains_key(&id)
            || (started && self.custom_at(point).is_some())
        {
            self.custom_update(id, point, started, ended)
        } else {
            self.inventory_update(id, point, started, ended)
        };
        handled
    }
    pub fn fit_placement(&self, mut p: Placement) -> Placement {
        p.size = (p.size * self.settings.button_scale)
            .min(self.aspect())
            .min(1.0);
        let half = [p.size / self.aspect() / 2.0, p.size / 2.0];
        for (i, half) in half.into_iter().enumerate() {
            p.center[i] = p.center[i].clamp(half, 1.0 - half);
        }
        p
    }
    pub fn custom_placement(&self, index: usize) -> Placement {
        let Some(btn) = self.settings.custom_buttons.get(index) else {
            return Placement::new(0.5, 0.5, 0.105);
        };
        let mut p = btn.layouts[self.settings.mode.index()];
        if self.settings.left_handed {
            p.center[0] = 1.0 - p.center[0];
        }
        self.fit_placement(p)
    }
    pub fn custom_visible_for_state(&self, index: usize, in_menu: bool) -> bool {
        if index >= self.settings.custom_buttons.len() {
            return false;
        }
        let p = self.custom_placement(index);
        if p.opacity <= 0.0 {
            return false;
        }
        if self.editor.is_some() {
            return true;
        }
        if in_menu {
            p.visible_in_menu
        } else {
            p.visible_in_game
        }
    }
    pub fn custom_at(&self, point: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len()).rev().find(|&i| {
            self.custom_visible_for_state(i, self.in_menu)
                && layout::contains(self.custom_placement(i).rect(self.aspect()), point)
        })
    }
    pub(super) fn custom_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if let Some(key) = self.custom_fingers.get(&id).cloned() {
            if ended {
                self.custom_fingers.remove(&id);
                let was_passthrough = self
                    .fingers
                    .remove(&id)
                    .is_some_and(|(r, _)| matches!(r, Role::Look { .. }));
                if !self.custom_fingers.values().any(|k| *k == key)
                    && !self.latched_custom.contains(&key)
                {
                    let mut extra_tokens = Vec::new();
                    for btn in &self.settings.custom_buttons {
                        if btn.key == key {
                            for extra in [&btn.second_key, &btn.third_key] {
                                if !extra.is_empty()
                                    && CustomButton::valid_token(extra)
                                    && extra != "key.keyboard.unknown"
                                    && !self.latched_custom.contains(extra)
                                    && !extra_tokens.contains(extra)
                                {
                                    extra_tokens.push(extra.clone());
                                }
                            }
                        }
                    }
                    self.custom_events.push((key, false));
                    for extra in extra_tokens {
                        self.custom_events.push((extra, false));
                    }
                }
                let _ = was_passthrough;
                return true;
            }
            // If this custom button has Pass-through enabled in gameplay, let TouchControls::update
            // also compute camera look delta for this finger!
            if !self.in_menu
                && self
                    .fingers
                    .get(&id)
                    .is_some_and(|(r, _)| matches!(r, Role::Look { .. }))
            {
                return false;
            }
            return true;
        }
        // Swipeable custom buttons activate when a moving finger slides over them.
        if !started && !ended {
            if let Some(index) = self.custom_at(point) {
                let placement = self.custom_placement(index);
                if placement.swipeable
                    && let Some(btn) = self.settings.custom_buttons.get(index)
                {
                    let key = btn.key.clone();
                    let second_key = btn.second_key.clone();
                    let third_key = btn.third_key.clone();
                    if key != "key.keyboard.unknown"
                        && !self.custom_fingers.values().any(|k| *k == key)
                        && !self.latched_custom.contains(&key)
                    {
                        self.custom_fingers.insert(id, key.clone());
                        self.custom_events.push((key, true));
                        for extra in [second_key, third_key] {
                            if !extra.is_empty()
                                && CustomButton::valid_token(&extra)
                                && extra != "key.keyboard.unknown"
                            {
                                self.custom_events.push((extra, true));
                            }
                        }
                        self.clicked = true;
                    }
                }
            }
        }
        if started
            && let Some(index) = self.custom_at(point)
            && let Some(btn) = self.settings.custom_buttons.get(index)
        {
            let active = btn.active_tokens();
            let Some(key) = active.first().cloned() else {
                return false;
            };
            let extras: Vec<String> = active.into_iter().skip(1).collect();
            let placement = self.custom_placement(index);
            self.clicked = true;
            if placement.toggleable && key != "mouse.virtual.toggle" {
                let now_on = !self.latched_custom.remove(&key);
                if now_on {
                    self.latched_custom.insert(key.clone());
                }
                self.custom_events.push((key, now_on));
                for extra in extras {
                    if now_on {
                        self.latched_custom.insert(extra.clone());
                    } else {
                        self.latched_custom.remove(&extra);
                    }
                    self.custom_events.push((extra, now_on));
                }
                if !placement.passthrough || self.in_menu {
                    self.fingers.insert(id, (Role::Ignore, point));
                    return true;
                }
                return false;
            }
            let pressed = !self.custom_fingers.values().any(|k| *k == key)
                && !self.latched_custom.contains(&key);
            self.custom_fingers.insert(id, key.clone());
            if pressed {
                self.custom_events.push((key, true));
                for extra in extras {
                    self.custom_events.push((extra, true));
                }
            }
            if placement.passthrough && !self.in_menu {
                self.fingers.insert(
                    id,
                    (
                        Role::Look {
                            start: point,
                            since: Instant::now(),
                            dragged: true,
                            held_use: false,
                            committed: false,
                        },
                        point,
                    ),
                );
            } else {
                self.fingers.insert(id, (Role::Ignore, point));
            }
            return true;
        }
        false
    }
    pub fn held_custom_tokens(&self) -> impl Iterator<Item = &str> {
        let extra_tokens = self
            .settings
            .custom_buttons
            .iter()
            .filter(|btn| {
                !btn.key.is_empty()
                    && btn.key != "key.keyboard.unknown"
                    && (self.custom_fingers.values().any(|k| *k == btn.key)
                        || self.latched_custom.contains(&btn.key))
            })
            .flat_map(|btn| [btn.second_key.as_str(), btn.third_key.as_str()])
            .filter(|t| {
                !t.is_empty() && *t != "key.keyboard.unknown" && CustomButton::valid_token(t)
            });
        self.custom_fingers
            .values()
            .chain(self.latched_custom.iter())
            .map(String::as_str)
            .chain(extra_tokens)
    }
    pub fn has_pending_key_events(&self) -> bool {
        !self.custom_events.is_empty()
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.custom_events)
    }
    pub(super) fn clear_custom(&mut self) {
        let mut keys: std::collections::BTreeSet<_> = std::mem::take(&mut self.custom_fingers)
            .into_values()
            .collect();
        keys.extend(std::mem::take(&mut self.latched_custom));
        let mut all_tokens = keys.clone();
        for btn in &self.settings.custom_buttons {
            if keys.contains(&btn.key)
                || (!btn.second_key.is_empty() && keys.contains(&btn.second_key))
                || (!btn.third_key.is_empty() && keys.contains(&btn.third_key))
            {
                for extra in [&btn.key, &btn.second_key, &btn.third_key] {
                    if !extra.is_empty()
                        && CustomButton::valid_token(extra)
                        && extra != "key.keyboard.unknown"
                    {
                        all_tokens.insert(extra.clone());
                    }
                }
            }
        }
        self.custom_events
            .extend(all_tokens.into_iter().map(|k| (k, false)));
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        let hud_scale = if self.editor.is_some() {
            1.0
        } else {
            self.hud_opacity_scale.clamp(0.0, 1.0)
        };
        if hud_scale <= 0.001 && self.editor.is_none() {
            return;
        }
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            if self.editor.is_none() && !self.custom_visible_for_state(i, self.in_menu) {
                continue;
            }
            let p = self.custom_placement(i);
            let is_goy =
                self.settings.control_style == layout::ControlStyle::GoyControl || p.use_goy_style;
            if is_goy && self.goy_rendered_by_ui && self.editor.is_none() {
                continue;
            }
            let [nx, ny, nw, nh] = p.rect(self.aspect());
            let rect = [nx * b.width, ny * b.height, nw * b.width, nh * b.height];
            let held = self.custom_fingers.values().any(|key| *key == button.key);
            let toggled = self.latched_custom.contains(&button.key);
            let pressed = held || toggled;
            let base_opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            let opacity = (base_opacity * hud_scale).clamp(0.0, 1.0);
            if opacity <= 0.001 {
                continue;
            }
            if is_goy {
                draw_goy_static_button(b, a, rect, &p, &button.label(), held, toggled, opacity);
            } else {
                text_button(
                    b,
                    a,
                    rect,
                    &button.label(),
                    pressed,
                    rgba(255, 255, 255, (opacity * 255.0).round() as u8),
                    true,
                );
            }
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
        let mut btn = CustomButton::new("key.keyboard.z".into(), 0);
        btn.set_event_slot(1, "key.keyboard.x".into());
        btn.set_event_slot(2, "key.mouse.left".into());
        t.settings.custom_buttons.push(btn);
        let stored = t.settings.custom_buttons[0].layouts[0].size;
        t.settings.button_scale = 2.0;
        assert_eq!(t.custom_placement(0).size, stored * 2.0);
        t.update(1, t.custom_placement(0).center, true, false);
        let down = t.key_events();
        assert_eq!(down.len(), 3);
        t.clear();
        let up = t.key_events();
        assert!(up.contains(&("key.keyboard.z".into(), false)));
        assert!(up.contains(&("key.keyboard.x".into(), false)));
        assert!(up.contains(&("key.mouse.left".into(), false)));
        assert_eq!(t.settings.custom_buttons[0].layouts[0].size, stored);
    }
}
