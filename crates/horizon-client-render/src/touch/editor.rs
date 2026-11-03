use super::*;

pub struct Editor {
    pub selected: Option<Control>,
    option: Option<usize>,
    apply_all: bool,
    original: Settings,
    drag: Option<(u64, Drag)>,
    error: Option<String>,
    palette: bool,
    selected_custom: Option<usize>,
    key_picker: bool,
    key_page: usize,
    raw_keys: bool,
    advanced: bool,
    gesture: usize,
    label_editing: bool,
    label_requested: bool,
}

enum Drag {
    Control([f32; 2]),
    Slider(usize),
}

impl TouchControls {
    pub fn take_label_request(&mut self) -> Option<String> {
        let e = self.editor.as_mut()?;
        if !std::mem::take(&mut e.label_requested) {
            return None;
        }
        Some(
            self.settings.custom_buttons[e.selected_custom?]
                .text
                .clone(),
        )
    }
    pub fn editing_label(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| e.label_editing)
    }
    pub fn edit_label(&mut self, text: Option<&str>, backspace: bool, done: bool) -> bool {
        let Some(e) = self.editor.as_mut().filter(|e| e.label_editing) else {
            return false;
        };
        if let Some(index) = e.selected_custom {
            let value = &mut self.settings.custom_buttons[index].text;
            if backspace {
                value.pop();
            }
            if let Some(text) = text {
                value.extend(text.chars().filter(|c| !c.is_control()));
                *value = value.chars().take(32).collect();
            }
        }
        if done {
            e.label_editing = false;
        }
        true
    }
    pub fn open_editor(&mut self) {
        self.clear();
        self.editor = Some(Editor {
            selected: None,
            option: None,
            apply_all: false,
            original: self.settings.clone(),
            drag: None,
            error: None,
            palette: false,
            selected_custom: None,
            key_picker: false,
            key_page: 0,
            raw_keys: false,
            advanced: false,
            gesture: 0,
            label_editing: false,
            label_requested: false,
        });
    }
    pub fn cancel_editor(&mut self) {
        if let Some(e) = self.editor.take() {
            self.settings = e.original;
        }
        self.clear();
    }
    /// The editor owns every finger: editing cannot move, mine or open menus.
    /// Returns true when Done/Cancel closed the editor.
    pub fn edit_touch(&mut self, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        let Some(mut e) = self.editor.take() else {
            return false;
        };
        self.clicked |= started;
        if self.edit_custom(&mut e, id, p, started, ended) {
            self.editor = Some(e);
            return false;
        }
        if started {
            if layout::contains([0.02, 0.04, 0.105, 0.075], p) {
                match self.settings.save() {
                    Ok(()) => {
                        self.clear();
                        return true;
                    }
                    Err(error) => e.error = Some(error.to_string()),
                }
            } else if layout::contains([0.135, 0.04, 0.105, 0.075], p) {
                self.settings = e.original;
                self.clear();
                return true;
            } else if layout::contains([0.25, 0.04, 0.105, 0.075], p) {
                self.settings.layouts[self.settings.mode.index()] =
                    Settings::default().layouts[self.settings.mode.index()];
                self.settings.button_visibility = [0; 14];
                for (i, button) in self.settings.custom_buttons.iter_mut().enumerate() {
                    button.layouts[self.settings.mode.index()] =
                        CustomButton::new(button.key.clone(), i).layouts
                            [self.settings.mode.index()];
                }
            } else if layout::contains([0.02, 0.13, 0.335, 0.075], p) {
                e.palette = !e.palette;
                e.selected = None;
                e.selected_custom = None;
                e.key_picker = false;
                e.drag = None;
            } else if layout::contains([0.69, 0.04, 0.14, 0.075], p) {
                match serde_json::to_string_pretty(&self.settings) {
                    Ok(json) => crate::clipboard::clipboard_set(&json),
                    Err(error) => e.error = Some(error.to_string()),
                }
            } else if layout::contains([0.84, 0.04, 0.14, 0.075], p) {
                let imported = crate::clipboard::clipboard_get()
                    .filter(|text| text.len() <= 1024 * 1024)
                    .and_then(|text| serde_json::from_str::<Settings>(&text).ok());
                if let Some(mut settings) = imported {
                    settings.sanitize();
                    self.settings = settings;
                    e.selected = None;
                    e.selected_custom = None;
                    e.drag = None;
                    e.error = None;
                    e.label_editing = false;
                    e.label_requested = false;
                    e.advanced = false;
                    e.key_picker = false;
                } else {
                    e.error = Some("Invalid layout in clipboard".into());
                }
            } else if e.palette {
                for (index, control) in Control::ALL.into_iter().enumerate() {
                    if layout::contains(palette_rect(index), p) {
                        let show = !self.visible(control);
                        self.settings.button_visibility[index] = if show { 1 } else { 2 };
                    }
                }
            } else if layout::contains([0.32, 0.54, 0.36, 0.07], p)
                && e.selected
                    .is_some_and(|c| !matches!(c, Control::Move | Control::Settings))
                && self.settings.custom_buttons.len() < 32
            {
                let control = e.selected.take().unwrap();
                let token = match control {
                    Control::Sprint => "action:key.sprint",
                    Control::Jump => "action:key.jump",
                    Control::Sneak => "action:key.sneak",
                    Control::Attack => "action:key.attack",
                    Control::Use | Control::Shield => "action:key.use",
                    Control::Chat => "action:key.chat",
                    Control::Inventory => "action:key.inventory",
                    Control::Offhand => "action:key.swapOffhand",
                    Control::Perspective => "action:key.togglePerspective",
                    Control::Debug => "key.keyboard.f3",
                    Control::Pause => "key.keyboard.escape",
                    _ => "key.keyboard.unknown",
                };
                let index = self.settings.custom_buttons.len();
                let mut button = CustomButton::new(token.into(), index);
                button.layouts = std::array::from_fn(|i| {
                    let slot = if i == Mode::Dpad.index() && self.settings.swap_jump_sneak {
                        match control {
                            Control::Jump => Control::Sneak as usize,
                            Control::Sneak => Control::Jump as usize,
                            _ => control as usize,
                        }
                    } else {
                        control as usize
                    };
                    self.settings.layouts[i][slot]
                });
                self.settings.custom_buttons.push(button);
                self.settings.button_visibility[control as usize] = 2;
                e.selected_custom = Some(index);
                e.drag = None;
                e.option = None;
            } else if layout::contains([0.32, 0.88, 0.36, 0.07], p) && e.selected.is_some() {
                self.settings.button_visibility[e.selected.unwrap() as usize] = 2;
                e.selected = None;
                e.drag = None;
            } else if layout::contains([0.32, 0.61, 0.36, 0.06], p)
                && e.selected.is_some()
                && e.option.is_some()
            {
                e.apply_all = !e.apply_all;
            } else if layout::contains([0.32, 0.68, 0.36, 0.07], p)
                && e.selected.is_some()
                && let Some(option) = e.option
            {
                e.drag = Some((id, Drag::Slider(option)));
            } else if layout::contains([0.32, 0.79, 0.11, 0.07], p) && e.selected.is_some() {
                e.option = Some(1);
            } else if layout::contains([0.445, 0.79, 0.11, 0.07], p) && e.selected.is_some() {
                e.option = Some(0);
            } else if layout::contains([0.57, 0.79, 0.11, 0.07], p) && e.selected.is_some() {
                e.option = Some(if e.selected == Some(Control::Move) {
                    0
                } else {
                    2
                });
            } else if let Some(c) = Control::ALL.into_iter().rev().find(|c| {
                self.visible(*c) && layout::contains(self.placement(*c).rect(self.aspect()), p)
            }) {
                e.selected = Some(c);
                e.option = None;
                let center = self.placement(c).center;
                e.drag = Some((id, Drag::Control([p[0] - center[0], p[1] - center[1]])));
            } else {
                e.selected = None;
                e.option = None;
            }
        }
        if let Some((owner, drag)) = &e.drag
            && *owner == id
            && let Some(selected) = e.selected
        {
            let aspect = self.aspect();
            let slot = self.settings.slot(selected);
            let place = &mut self.settings.layouts[self.settings.mode.index()][slot];
            match drag {
                Drag::Control(offset) => {
                    let mut center = [p[0] - offset[0], p[1] - offset[1]];
                    if self.settings.left_handed {
                        center[0] = 1.0 - center[0];
                    }
                    // Match the displayed size on narrow portrait surfaces.
                    // A stored button may be wider than the screen; its raw
                    // size would otherwise produce inverted clamp bounds.
                    let width = (place.width() * self.settings.button_scale).min(aspect);
                    let height = (place.height() * self.settings.button_scale).min(1.0);
                    place.center = [
                        center[0].clamp(width / aspect / 2.0, 1.0 - width / aspect / 2.0),
                        center[1].clamp(height / 2.0, 1.0 - height / 2.0),
                    ];
                }
                Drag::Slider(index) => {
                    let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                    for (i, place) in self.settings.layouts[self.settings.mode.index()]
                        .iter_mut()
                        .enumerate()
                    {
                        if i == slot || e.apply_all {
                            if *index == 0 {
                                if selected == Control::Move {
                                    place.size = 0.045 + value * 0.405;
                                } else {
                                    place.width = 0.045 + value * 0.405;
                                }
                            } else if *index == 2 {
                                place.height = 0.045 + value * 0.405;
                            } else {
                                place.opacity = value;
                            }
                        }
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }

        self.editor = Some(e);
        false
    }
    pub(super) fn draw_editor_toolbar(&self, b: &mut HudBatch, a: &HudAtlas) {
        for (rect, label) in [
            ([0.02, 0.04, 0.105, 0.075], "Done"),
            ([0.135, 0.04, 0.105, 0.075], "Cancel"),
            ([0.25, 0.04, 0.105, 0.075], "Reset"),
            ([0.02, 0.13, 0.335, 0.075], "Add buttons..."),
            ([0.69, 0.04, 0.14, 0.075], "Copy layout"),
            ([0.84, 0.04, 0.14, 0.075], "Paste layout"),
        ] {
            plate(b, a, rect, label, false);
        }
    }

    pub(super) fn draw_editor(&self, b: &mut HudBatch, a: &HudAtlas) {
        let e = self.editor.as_ref().unwrap();
        let (w, h) = (b.width, b.height);
        let text = rgba(245, 245, 245, 255);
        if let Some(selected) = e.selected {
            if !matches!(selected, Control::Move | Control::Settings) {
                plate(
                    b,
                    a,
                    [0.32, 0.54, 0.36, 0.07],
                    "Customize actions / text...",
                    false,
                );
            }
            plate(b, a, [0.32, 0.88, 0.36, 0.07], "Remove button", false);
            let p = self.placement(selected);
            plate(
                b,
                a,
                [0.32, 0.79, 0.11, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(
                b,
                a,
                [0.445, 0.79, 0.11, 0.07],
                if selected == Control::Move {
                    "Size"
                } else {
                    "Width"
                },
                e.option == Some(0),
            );
            plate(
                b,
                a,
                [0.57, 0.79, 0.11, 0.07],
                if selected == Control::Move {
                    "Size"
                } else {
                    "Height"
                },
                e.option == Some(2),
            );
            if let Some(option) = e.option {
                b.fill(
                    a,
                    w * 0.31,
                    h * 0.59,
                    w * 0.38,
                    h * 0.18,
                    rgba(15, 15, 15, 235),
                );
                plate(
                    b,
                    a,
                    [0.32, 0.61, 0.36, 0.06],
                    if e.apply_all {
                        "Apply to all: On"
                    } else {
                        "Apply to all: Off"
                    },
                    e.apply_all,
                );
                let dimension = if option == 2 { p.height() } else { p.width() };
                let value = if option != 1 {
                    (dimension - 0.045) / 0.405
                } else {
                    p.opacity
                };
                let percent = if option != 1 {
                    dimension * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!("{} {:.0}%", selected.name(), percent),
                    w * 0.325,
                    h * 0.68,
                    text,
                    true,
                );
                nine(
                    b,
                    a,
                    "ui/slider_background",
                    1.0,
                    [w * 0.335, h * 0.72, w * 0.33, h * 0.025],
                    rgba(255, 255, 255, 255),
                );
                nine(
                    b,
                    a,
                    "ui/slider_button_default",
                    2.0,
                    [w * (0.335 + value * 0.33) - 3.0, h * 0.707, 6.0, h * 0.05],
                    rgba(255, 255, 255, 255),
                );
            }
        }
        if e.palette {
            b.fill(
                a,
                w * 0.2,
                h * 0.14,
                w * 0.6,
                h * 0.72,
                rgba(15, 15, 15, 245),
            );
            for (index, control) in Control::ALL.into_iter().enumerate() {
                let shown = self.visible(control);
                plate(
                    b,
                    a,
                    palette_rect(index),
                    &format!("{} {}", if shown { "[-]" } else { "[+]" }, control.name()),
                    shown,
                );
            }
        }
        self.draw_custom_editor(b, a, e);
        if let Some(error) = &e.error {
            b.text(a, error, w * 0.33, h * 0.88, rgba(255, 120, 120, 255), true);
        }
    }
}
fn palette_rect(index: usize) -> [f32; 4] {
    [
        0.22 + (index % 2) as f32 * 0.29,
        0.17 + (index / 2) as f32 * 0.11,
        0.27,
        0.09,
    ]
}
fn plate(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], label: &str, on: bool) {
    let (bw, bh) = (b.width, b.height);
    nine(
        b,
        a,
        "ui/button_borderless_light",
        1.0,
        [x * bw, y * bh, w * bw, h * bh],
        if on {
            rgba(100, 210, 90, 255)
        } else {
            rgba(255, 255, 255, 255)
        },
    );
    if let Some((first, second)) = label.split_once(" & ") {
        b.text(
            a,
            &format!("{first} &"),
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
        b.text(
            a,
            second,
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0 + 10.0,
            rgba(35, 35, 35, 255),
            false,
        );
    } else {
        let mut display = label.to_owned();
        while HudBatch::text_width(a, &display) > w * bw - 6.0 && !display.is_empty() {
            display.pop();
        }
        b.text(
            a,
            &display,
            x * bw + 3.0,
            y * bh + (h * bh - 8.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
    }
}

impl TouchControls {
    fn edit_custom(
        &mut self,
        e: &mut Editor,
        id: u64,
        p: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if p[1] < 0.13 && started {
            return false;
        }
        if e.key_picker {
            if started {
                if layout::contains([0.15, 0.16, 0.33, 0.06], p) {
                    e.raw_keys = false;
                    e.key_page = 0;
                } else if layout::contains([0.52, 0.16, 0.33, 0.06], p) {
                    e.raw_keys = true;
                    e.key_page = 0;
                } else if layout::contains([0.2, 0.86, 0.18, 0.07], p) {
                    e.key_page = e.key_page.saturating_sub(1);
                } else if layout::contains([0.41, 0.86, 0.18, 0.07], p) {
                    e.key_picker = false;
                } else if layout::contains([0.62, 0.86, 0.18, 0.07], p) {
                    e.key_page = (e.key_page + 1)
                        .min(super::custom::choices(e.raw_keys).len().saturating_sub(1) / 8);
                } else {
                    for (i, (token, _)) in super::custom::choices(e.raw_keys)
                        .into_iter()
                        .skip(e.key_page * 8)
                        .take(8)
                        .enumerate()
                    {
                        if layout::contains(key_rect(i), p)
                            && let Some(index) = e.selected_custom
                        {
                            self.settings.custom_buttons[index].key = token;
                            if e.advanced {
                                let button = &mut self.settings.custom_buttons[index];
                                let token = std::mem::replace(
                                    &mut button.key,
                                    "key.keyboard.unknown".into(),
                                );
                                let bindings = gesture_bindings(button, e.gesture);
                                if bindings.len() < 8 && !bindings.iter().any(|b| b.token == token)
                                {
                                    bindings.push(super::gestures::Binding {
                                        token,
                                        toggle: false,
                                    });
                                }
                            } else {
                                self.settings.custom_buttons[index].gestures.single.clear();
                            }
                            e.key_picker = false;
                            break;
                        }
                    }
                }
            }
            return true;
        }
        if e.advanced {
            if started && let Some(index) = e.selected_custom {
                let button = &mut self.settings.custom_buttons[index];
                if layout::contains([0.15, 0.16, 0.7, 0.06], p) {
                    e.advanced = false;
                }
                for gesture in 0..3 {
                    if layout::contains([0.15 + gesture as f32 * 0.24, 0.25, 0.22, 0.06], p) {
                        e.gesture = gesture;
                    }
                }
                let bindings = gesture_bindings(button, e.gesture);
                let mut remove = None;
                for (i, binding) in bindings.iter_mut().enumerate() {
                    let y = 0.33 + i as f32 * 0.052;
                    if layout::contains([0.15, y, 0.6, 0.045], p) {
                        binding.toggle ^= true;
                    }
                    if layout::contains([0.78, y, 0.07, 0.045], p) {
                        remove = Some(i);
                    }
                }
                if let Some(i) = remove {
                    bindings.remove(i);
                }
                if layout::contains([0.15, 0.77, 0.33, 0.06], p) {
                    e.key_picker = true;
                    e.key_page = 0;
                }
                if layout::contains([0.52, 0.77, 0.33, 0.06], p) {
                    button.gestures.immediate_single ^= true;
                }
                if layout::contains([0.15, 0.85, 0.33, 0.06], p) {
                    button.gestures.double_ms = if button.gestures.double_ms >= 700 {
                        100
                    } else {
                        button.gestures.double_ms + 50
                    };
                }
                if layout::contains([0.52, 0.85, 0.33, 0.06], p) {
                    button.gestures.hold_ms = if button.gestures.hold_ms >= 2000 {
                        150
                    } else {
                        button.gestures.hold_ms + 50
                    };
                }
            }
            return true;
        }
        if e.palette && started && layout::contains([0.22, 0.84, 0.56, 0.07], p) {
            if self.settings.custom_buttons.len() < 32 {
                let index = self.settings.custom_buttons.len();
                self.settings
                    .custom_buttons
                    .push(CustomButton::new("key.keyboard.unknown".into(), index));
                e.selected = None;
                e.selected_custom = Some(index);
                e.key_picker = true;
                e.raw_keys = false;
                e.key_page = 0;
                e.palette = false;
                e.option = None;
                e.drag = None;
            }
            return true;
        }
        if e.palette {
            return false;
        }
        if started && e.selected_custom.is_some() {
            if layout::contains([0.32, 0.27, 0.36, 0.07], p) {
                e.option = Some(3);
                return true;
            }
            if layout::contains([0.32, 0.36, 0.36, 0.07], p) {
                e.label_editing = true;
                e.label_requested = true;
                return true;
            }
            if layout::contains([0.32, 0.45, 0.36, 0.07], p) {
                let button = &mut self.settings.custom_buttons[e.selected_custom.unwrap()];
                if button.gestures.single.is_empty() && button.key != "key.keyboard.unknown" {
                    button.gestures.single.push(super::gestures::Binding {
                        token: button.key.clone(),
                        toggle: false,
                    });
                }
                button.key = "key.keyboard.unknown".into();
                e.advanced = true;
                e.drag = None;
                return true;
            }
            if layout::contains([0.32, 0.54, 0.36, 0.07], p) {
                e.key_picker = true;
                e.key_page = 0;
                e.drag = None;
                return true;
            }
            if layout::contains([0.32, 0.88, 0.36, 0.07], p) {
                self.settings
                    .custom_buttons
                    .remove(e.selected_custom.unwrap());
                e.selected_custom = None;
                e.drag = None;
                return true;
            }
            if layout::contains([0.32, 0.79, 0.11, 0.07], p) {
                e.option = Some(1);
                return true;
            }
            if layout::contains([0.445, 0.79, 0.11, 0.07], p) {
                e.option = Some(0);
                return true;
            }
            if layout::contains([0.57, 0.79, 0.11, 0.07], p) {
                e.option = Some(2);
                return true;
            }
            if layout::contains([0.32, 0.68, 0.36, 0.07], p)
                && let Some(option) = e.option
            {
                e.drag = Some((id, Drag::Slider(option)));
            } else {
                e.selected_custom = None;
                e.drag = None;
            }
        }
        if started
            && e.drag.is_none()
            && let Some(index) = self.custom_at(p)
        {
            let center = self.custom_placement(index).center;
            e.selected = None;
            e.selected_custom = Some(index);
            e.option = None;
            e.apply_all = false;
            e.drag = Some((id, Drag::Control([p[0] - center[0], p[1] - center[1]])));
        }
        let Some(index) = e.selected_custom else {
            return false;
        };
        if let Some((owner, drag)) = &e.drag
            && *owner == id
        {
            let aspect = self.aspect();
            let scale = self.settings.button_scale;
            let mirror = self.settings.left_handed;
            let place =
                &mut self.settings.custom_buttons[index].layouts[self.settings.mode.index()];
            match drag {
                Drag::Control(offset) => {
                    let mut center = [p[0] - offset[0], p[1] - offset[1]];
                    if mirror {
                        center[0] = 1.0 - center[0];
                    }
                    let width = (place.width() * scale).min(aspect);
                    let height = (place.height() * scale).min(1.0);
                    place.center = [
                        center[0].clamp(width / aspect / 2.0, 1.0 - width / aspect / 2.0),
                        center[1].clamp(height / 2.0, 1.0 - height / 2.0),
                    ];
                }
                Drag::Slider(option) => {
                    let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                    if *option == 0 {
                        place.width = 0.045 + value * 0.405;
                    } else if *option == 2 {
                        place.height = 0.045 + value * 0.405;
                    } else if *option == 3 {
                        self.settings.custom_buttons[index].text_scale = 0.5 + value * 2.5;
                    } else {
                        place.opacity = value;
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }
        true
    }
    pub fn capture_custom_key(&mut self, name: &str) -> bool {
        if crate::keybinds::key_code(name).is_none() {
            return false;
        }
        if let Some(e) = &mut self.editor
            && e.key_picker
            && let Some(i) = e.selected_custom
        {
            let button = &mut self.settings.custom_buttons[i];
            if e.advanced {
                let bindings = gesture_bindings(button, e.gesture);
                if bindings.len() < 8 && !bindings.iter().any(|b| b.token == name) {
                    bindings.push(super::gestures::Binding {
                        token: name.into(),
                        toggle: false,
                    });
                }
            } else {
                button.key = name.into();
                button.gestures.single.clear();
            }
            e.key_picker = false;
            true
        } else {
            false
        }
    }
    fn draw_custom_editor(&self, b: &mut HudBatch, a: &HudAtlas, e: &Editor) {
        let (w, h) = (b.width, b.height);
        if e.palette {
            plate(b, a, [0.22, 0.84, 0.56, 0.07], "Create button...", false);
        }
        if let Some(i) = e.selected_custom
            && !e.palette
            && !e.key_picker
            && !e.advanced
        {
            let p = self.settings.custom_buttons[i].layouts[self.settings.mode.index()];
            plate(
                b,
                a,
                [0.32, 0.27, 0.36, 0.07],
                "Text size",
                e.option == Some(3),
            );
            plate(
                b,
                a,
                [0.32, 0.36, 0.36, 0.07],
                "Edit text...",
                e.label_editing,
            );
            plate(
                b,
                a,
                [0.32, 0.45, 0.36, 0.07],
                "Gestures / multi-bind...",
                false,
            );
            plate(
                b,
                a,
                [0.32, 0.54, 0.36, 0.07],
                &format!("Action: {}", self.settings.custom_buttons[i].label()),
                false,
            );
            plate(
                b,
                a,
                [0.32, 0.79, 0.11, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(
                b,
                a,
                [0.445, 0.79, 0.11, 0.07],
                "Width",
                e.option == Some(0),
            );
            plate(
                b,
                a,
                [0.57, 0.79, 0.11, 0.07],
                "Height",
                e.option == Some(2),
            );
            plate(b, a, [0.32, 0.88, 0.36, 0.07], "Remove button", false);
            if let Some(option) = e.option {
                let dimension = if option == 2 { p.height() } else { p.width() };
                let value = if option == 3 {
                    (self.settings.custom_buttons[i].text_scale - 0.5) / 2.5
                } else if option != 1 {
                    (dimension - 0.045) / 0.405
                } else {
                    p.opacity
                };
                b.fill(
                    a,
                    w * 0.3,
                    h * 0.66,
                    w * 0.4,
                    h * 0.11,
                    rgba(15, 15, 15, 235),
                );
                let percent = if option == 3 {
                    self.settings.custom_buttons[i].text_scale * 100.0
                } else if option != 1 {
                    dimension * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!(
                        "{} {:.0}%",
                        match option {
                            0 => "Width",
                            2 => "Height",
                            3 => "Text",
                            _ => "Opacity",
                        },
                        percent
                    ),
                    w * 0.325,
                    h * 0.68,
                    rgba(245, 245, 245, 255),
                    true,
                );
                nine(
                    b,
                    a,
                    "ui/slider_background",
                    1.0,
                    [w * 0.335, h * 0.72, w * 0.33, h * 0.025],
                    rgba(255, 255, 255, 255),
                );
                nine(
                    b,
                    a,
                    "ui/slider_button_default",
                    2.0,
                    [w * (0.335 + value * 0.33) - 3.0, h * 0.707, 6.0, h * 0.05],
                    rgba(255, 255, 255, 255),
                );
            }
        }
        if e.advanced
            && !e.key_picker
            && let Some(index) = e.selected_custom
        {
            let button = &self.settings.custom_buttons[index];
            b.fill(
                a,
                w * 0.12,
                h * 0.14,
                w * 0.76,
                h * 0.81,
                rgba(15, 15, 15, 250),
            );
            plate(b, a, [0.15, 0.16, 0.7, 0.06], "Back to button", false);
            for (gesture, label) in ["Single", "Double", "Hold"].into_iter().enumerate() {
                plate(
                    b,
                    a,
                    [0.15 + gesture as f32 * 0.24, 0.25, 0.22, 0.06],
                    label,
                    e.gesture == gesture,
                );
            }
            let bindings = match e.gesture {
                1 => &button.gestures.double,
                2 => &button.gestures.hold,
                _ => &button.gestures.single,
            };
            for (i, binding) in bindings.iter().enumerate() {
                let y = 0.33 + i as f32 * 0.052;
                plate(
                    b,
                    a,
                    [0.15, y, 0.6, 0.045],
                    &format!(
                        "{}: {}",
                        if binding.toggle { "Toggle" } else { "Hold" },
                        super::custom::token_label(&binding.token)
                    ),
                    binding.toggle,
                );
                plate(b, a, [0.78, y, 0.07, 0.045], "X", false);
            }
            plate(b, a, [0.15, 0.77, 0.33, 0.06], "Add action...", false);
            plate(
                b,
                a,
                [0.52, 0.77, 0.33, 0.06],
                if button.gestures.immediate_single {
                    "Single: immediate +"
                } else {
                    "Single: deferred"
                },
                false,
            );
            plate(
                b,
                a,
                [0.15, 0.85, 0.33, 0.06],
                &format!("Double: {} ms +", button.gestures.double_ms),
                false,
            );
            plate(
                b,
                a,
                [0.52, 0.85, 0.33, 0.06],
                &format!("Hold: {} ms +", button.gestures.hold_ms),
                false,
            );
        }
        if e.key_picker {
            b.fill(
                a,
                w * 0.12,
                h * 0.14,
                w * 0.76,
                h * 0.81,
                rgba(15, 15, 15, 250),
            );
            plate(b, a, [0.15, 0.16, 0.33, 0.06], "Game action", !e.raw_keys);
            plate(b, a, [0.52, 0.16, 0.33, 0.06], "Keyboard key", e.raw_keys);
            for (i, (_, label)) in super::custom::choices(e.raw_keys)
                .iter()
                .skip(e.key_page * 8)
                .take(8)
                .enumerate()
            {
                plate(b, a, key_rect(i), label, false);
            }
            plate(b, a, [0.2, 0.86, 0.18, 0.07], "Previous", false);
            plate(b, a, [0.41, 0.86, 0.18, 0.07], "Back", false);
            plate(b, a, [0.62, 0.86, 0.18, 0.07], "Next", false);
        }
    }
}
fn gesture_bindings(
    button: &mut CustomButton,
    gesture: usize,
) -> &mut Vec<super::gestures::Binding> {
    match gesture {
        1 => &mut button.gestures.double,
        2 => &mut button.gestures.hold,
        _ => &mut button.gestures.single,
    }
}
fn key_rect(i: usize) -> [f32; 4] {
    [0.15, 0.25 + i as f32 * 0.072, 0.7, 0.06]
}
