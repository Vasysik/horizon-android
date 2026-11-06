use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorDragKind {
    Move {
        offset: [f32; 2],
    },
    Resize {
        mask: u8,
        start_pos: [f32; 2],
        start_size: f32,
        start_wr: f32,
        start_hr: f32,
    },
    FloatingPod {
        offset: [f32; 2],
        moved: bool,
    },
    KeyboardScroll {
        last_x: f32,
        moved: bool,
    },
    PropsSlider {
        tab: usize,
        index: usize,
        rect: [f32; 4],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerStep {
    None,
    /// Step 1: Ask user whether to bind a Keyboard Key or Mouse/Game Action for event slot 0..=2.
    AskInputType {
        event_slot: usize,
    },
    /// Step 2a: Full Horizontally Scrollable 104-Key PC Keyboard Dialog.
    Keyboard {
        event_slot: usize,
    },
    /// Step 2b: Mouse Buttons, Wheel, Look & Game Actions Dialog.
    MouseAndActions {
        event_slot: usize,
    },
}

pub struct Editor {
    pub selected: Option<Control>,
    pub option: Option<usize>,
    pub apply_all: bool,
    pub original: Settings,
    drag: Option<(u64, Drag)>,
    pub error: Option<String>,
    pub palette: bool,
    pub selected_custom: Option<usize>,
    pub key_picker: bool,
    pub key_page: usize,
    pub raw_keys: bool,
    /// True when the Main Floating Control Manager Dialog (MCPE vs GoyControl, Save, Default Always) is open.
    pub menu_open: bool,
    /// True when the Component Property Inspector Dialog (Style, Hint Text, Stroke, Colors, Shadow, Events, Joystick) is open.
    pub props_open: bool,
    /// Active tab inside the Component Property Inspector (0 = Style & Colors, 1 = Stroke & Shape, 2 = Events & Behavior).
    pub props_tab: usize,
    /// Active event picker dialog state.
    pub picker_step: PickerStep,
    /// Horizontal scroll offset (in dp) for the full PC keyboard picker dialog.
    pub kb_scroll_x: f32,
    /// Whether the floating pod is expanded to show Undo / Redo / Properties / Add / Save pills.
    pub pod_expanded: bool,
    /// Smooth 0..1 animation progress for the floating pod & dialogs.
    pub pod_anim: f32,
    /// Show hidden built-in controls as translucent ghosts in the editor.
    pub show_hidden: bool,
    /// Active canvas drag or corner/edge resize operation.
    pub active_drag: Option<(u64, EditorDragKind)>,
    /// Undo history stack (up to 40 snapshots).
    pub undo_stack: Vec<Settings>,
    /// Redo history stack.
    pub redo_stack: Vec<Settings>,
    /// Status banner message inside editor.
    pub status_msg: Option<String>,
    /// Custom profile name input inside the Save / Control Manager dialog.
    pub save_name_input: String,
    /// Custom profile description input inside the Save / Control Manager dialog.
    pub save_desc_input: String,
    /// Whether the custom profile name field is actively focused.
    pub editing_save_name: bool,
    /// Whether the custom profile description field is actively focused.
    pub editing_save_desc: bool,
    /// True when the Editor Save Dialog (Option A: Replace existing preset, Option B: Create new preset as...) is open.
    pub save_dialog_open: bool,
    /// A naming field was tapped: the game opens the system keyboard.
    pub keyboard_request: bool,
    /// True when Option B (Create a new preset as...) is selected in the Save Dialog.
    pub save_create_new_mode: bool,
    /// True when the Back confirmation dialog (`[ Save ]` / `[ Back ]`) is open.
    pub back_confirm_open: bool,
}

enum Drag {
    Control([f32; 2]),
    Slider(usize),
}

impl TouchControls {
    pub fn open_editor(&mut self) {
        self.clear();
        let is_default = self.settings.is_default_preset();
        self.editor = Some(Editor {
            selected: Some(Control::Move),
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
            menu_open: false,
            props_open: false,
            props_tab: 0,
            picker_step: PickerStep::None,
            kb_scroll_x: 0.0,
            pod_expanded: true,
            pod_anim: 1.0,
            show_hidden: true,
            active_drag: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            status_msg: None,
            save_name_input: if is_default {
                String::new()
            } else {
                self.settings.preset_name.clone()
            },
            save_desc_input: if is_default {
                String::new()
            } else {
                self.settings.preset_description.clone()
            },
            editing_save_name: false,
            editing_save_desc: false,
            save_dialog_open: false,
            keyboard_request: false,
            save_create_new_mode: is_default,
            back_confirm_open: false,
        });
    }
    pub fn cancel_editor_gesture(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.drag = None;
            editor.active_drag = None;
        }
    }
    pub fn cancel_editor(&mut self) {
        if let Some(e) = self.editor.take() {
            self.settings = e.original;
        }
        self.clear();
    }
    pub fn has_unsaved_changes(&self) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|e| self.settings != e.original)
    }
    /// Handle Android Back / Escape inside the Control Editor.
    /// Closes any open sub-dialog first; if at the root canvas and unsaved
    /// changes exist, opens the `[ Save ]` `[ Back ]` confirmation dialog;
    /// otherwise closes the editor and returns `true`.
    pub fn request_editor_back(&mut self) -> bool {
        let Some(e) = self.editor.as_mut() else {
            return true;
        };
        if e.picker_step != PickerStep::None {
            e.picker_step = PickerStep::None;
            return false;
        }
        if e.props_open {
            e.props_open = false;
            return false;
        }
        if e.menu_open {
            e.menu_open = false;
            return false;
        }
        if e.save_dialog_open {
            e.save_dialog_open = false;
            e.editing_save_name = false;
            e.editing_save_desc = false;
            return false;
        }
        if e.back_confirm_open {
            e.back_confirm_open = false;
            return false;
        }
        if self.settings != e.original {
            e.back_confirm_open = true;
            e.pod_anim = 0.0;
            return false;
        }
        self.editor = None;
        self.clear();
        true
    }
    /// The editor owns every finger: editing cannot move, mine or open menus.
    /// Returns true when Done/Cancel closed the editor.
    pub fn edit_touch(&mut self, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        let Some(mut e) = self.editor.take() else {
            return false;
        };
        if e.selected_custom
            .is_some_and(|idx| idx >= self.settings.custom_buttons.len())
        {
            e.selected_custom = None;
            e.drag = None;
        }
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
                let def = Settings::default_for_style(self.settings.control_style);
                self.settings.layouts[self.settings.mode.index()] =
                    def.layouts[self.settings.mode.index()];
                if self.settings.control_style == ControlStyle::GoyControl {
                    self.settings.custom_buttons = def.custom_buttons;
                }
                if e.selected_custom
                    .is_some_and(|idx| idx >= self.settings.custom_buttons.len())
                {
                    e.selected_custom = None;
                    e.drag = None;
                }
            } else if layout::contains([0.02, 0.13, 0.335, 0.075], p) {
                e.palette = !e.palette;
                e.selected = None;
                e.selected_custom = None;
                e.key_picker = false;
                e.drag = None;
            } else if e.palette {
                for (index, control) in Control::ALL.into_iter().enumerate() {
                    if layout::contains(palette_rect(index), p) {
                        let show = !self.visible(control);
                        self.settings.button_visibility[index] = if show { 1 } else { 2 };
                    }
                }
            } else if layout::contains([0.365, 0.04, 0.14, 0.075], p) {
                self.settings.snap_to_grid = !self.settings.snap_to_grid;
            } else if layout::contains([0.515, 0.04, 0.22, 0.075], p) {
                let presets =
                    Settings::list_presets(std::path::Path::new("."), self.settings.control_style);
                if !presets.is_empty() {
                    let idx = presets
                        .iter()
                        .position(|m| m.id == self.settings.preset_id)
                        .unwrap_or(0);
                    let next = &presets[(idx + 1) % presets.len()].id;
                    self.settings.select_preset(
                        std::path::Path::new("."),
                        self.settings.control_style,
                        next,
                    );
                    if e.selected_custom
                        .is_some_and(|idx| idx >= self.settings.custom_buttons.len())
                    {
                        e.selected_custom = None;
                        e.drag = None;
                    }
                }
            } else if layout::contains([0.32, 0.46, 0.115, 0.06], p) && e.selected.is_some() {
                let slot = self.settings.slot(e.selected.unwrap());
                self.settings.layouts[self.settings.mode.index()][slot].passthrough ^= true;
            } else if layout::contains([0.442, 0.46, 0.115, 0.06], p) && e.selected.is_some() {
                let slot = self.settings.slot(e.selected.unwrap());
                self.settings.layouts[self.settings.mode.index()][slot].toggleable ^= true;
            } else if layout::contains([0.565, 0.46, 0.115, 0.06], p) && e.selected.is_some() {
                let slot = self.settings.slot(e.selected.unwrap());
                self.settings.layouts[self.settings.mode.index()][slot].swipeable ^= true;
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
                && e.option.is_some()
            {
                e.drag = Some((id, Drag::Slider(e.option.unwrap())));
            } else if layout::contains([0.32, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(1);
            } else if layout::contains([0.51, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(0);
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
                    if self.settings.snap_to_grid {
                        center = center.map(|v| (v * 40.0).round() / 40.0);
                    }
                    let size = (place.size * self.settings.button_scale)
                        .min(aspect)
                        .min(1.0);
                    place.center = [
                        center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                        center[1].clamp(size / 2.0, 1.0 - size / 2.0),
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
                                place.size = 0.045 + value * 0.405;
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
        for (rect, label, on) in [
            ([0.02, 0.04, 0.105, 0.075], "Done".to_owned(), false),
            ([0.135, 0.04, 0.105, 0.075], "Cancel".to_owned(), false),
            ([0.25, 0.04, 0.105, 0.075], "Reset".to_owned(), false),
            (
                [0.365, 0.04, 0.14, 0.075],
                format!(
                    "Grid: {}",
                    if self.settings.snap_to_grid {
                        "On"
                    } else {
                        "Off"
                    }
                ),
                self.settings.snap_to_grid,
            ),
            (
                [0.515, 0.04, 0.22, 0.075],
                format!("Profile: {}", self.settings.active_profile),
                false,
            ),
            (
                [0.02, 0.13, 0.335, 0.075],
                "Add buttons...".to_owned(),
                false,
            ),
        ] {
            plate(b, a, rect, &label, on);
        }
    }

    pub(super) fn draw_editor(&self, b: &mut HudBatch, a: &HudAtlas) {
        let e = self.editor.as_ref().unwrap();
        let (w, h) = (b.width, b.height);
        let text = rgba(245, 245, 245, 255);
        if let Some(selected) = e.selected {
            plate(b, a, [0.32, 0.88, 0.36, 0.07], "Remove button", false);
            let p = self.placement(selected);
            plate(b, a, [0.32, 0.46, 0.115, 0.06], "PassThru", p.passthrough);
            plate(b, a, [0.442, 0.46, 0.115, 0.06], "Toggle", p.toggleable);
            plate(b, a, [0.565, 0.46, 0.115, 0.06], "Swipe", p.swipeable);
            plate(
                b,
                a,
                [0.32, 0.79, 0.17, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(b, a, [0.51, 0.79, 0.17, 0.07], "Size", e.option == Some(0));
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
                let value = if option == 0 {
                    (p.size - 0.045) / 0.405
                } else {
                    p.opacity
                };
                let percent = if option == 0 {
                    p.size * 100.0
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
        if e.error.is_some() {
            b.text(
                a,
                "Save failed. Retry Done.",
                w * 0.33,
                h * 0.88,
                rgba(255, 120, 120, 255),
                true,
            );
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
                            let is_mcpe = self.settings.control_style == ControlStyle::Mcpe;
                            if let Some(btn) = self.settings.custom_buttons.get_mut(index) {
                                if is_mcpe {
                                    btn.set_exclusive_binding(token);
                                } else {
                                    btn.key = token;
                                }
                            }
                            e.key_picker = false;
                            break;
                        }
                    }
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
        if started && let Some(idx) = e.selected_custom {
            let mode_idx = self.settings.mode.index();
            if layout::contains([0.32, 0.46, 0.115, 0.06], p) {
                if let Some(btn) = self.settings.custom_buttons.get_mut(idx) {
                    btn.layouts[mode_idx].passthrough ^= true;
                }
                return true;
            }
            if layout::contains([0.442, 0.46, 0.115, 0.06], p) {
                if let Some(btn) = self.settings.custom_buttons.get_mut(idx) {
                    btn.layouts[mode_idx].toggleable ^= true;
                }
                return true;
            }
            if layout::contains([0.565, 0.46, 0.115, 0.06], p) {
                if let Some(btn) = self.settings.custom_buttons.get_mut(idx) {
                    btn.layouts[mode_idx].swipeable ^= true;
                }
                return true;
            }
            if layout::contains([0.32, 0.54, 0.36, 0.07], p) {
                e.key_picker = true;
                e.key_page = 0;
                e.drag = None;
                return true;
            }
            if layout::contains([0.32, 0.88, 0.36, 0.07], p) {
                if idx < self.settings.custom_buttons.len() {
                    self.settings.custom_buttons.remove(idx);
                }
                e.selected_custom = None;
                e.drag = None;
                return true;
            }
            if layout::contains([0.32, 0.79, 0.17, 0.07], p) {
                e.option = Some(1);
                return true;
            }
            if layout::contains([0.51, 0.79, 0.17, 0.07], p) {
                e.option = Some(0);
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
        if index >= self.settings.custom_buttons.len() {
            e.selected_custom = None;
            e.drag = None;
            return false;
        }
        if let Some((owner, drag)) = &e.drag
            && *owner == id
        {
            let aspect = self.aspect();
            let scale = self.settings.button_scale;
            let mirror = self.settings.left_handed;
            let mode_idx = self.settings.mode.index();
            if let Some(btn) = self.settings.custom_buttons.get_mut(index) {
                let place = &mut btn.layouts[mode_idx];
                match drag {
                    Drag::Control(offset) => {
                        let mut center = [p[0] - offset[0], p[1] - offset[1]];
                        if mirror {
                            center[0] = 1.0 - center[0];
                        }
                        let size = (place.size * scale).min(aspect).min(1.0);
                        place.center = [
                            center[0].clamp(size / aspect / 2.0, 1.0 - size / aspect / 2.0),
                            center[1].clamp(size / 2.0, 1.0 - size / 2.0),
                        ];
                    }
                    Drag::Slider(option) => {
                        let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                        if *option == 0 {
                            place.size = 0.045 + value * 0.405;
                        } else {
                            place.opacity = value;
                        }
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }
        true
    }
    pub fn char_to_key_token(ch: char) -> Option<&'static str> {
        match ch.to_ascii_lowercase() {
            'a' => Some("key.keyboard.a"),
            'b' => Some("key.keyboard.b"),
            'c' => Some("key.keyboard.c"),
            'd' => Some("key.keyboard.d"),
            'e' => Some("key.keyboard.e"),
            'f' => Some("key.keyboard.f"),
            'g' => Some("key.keyboard.g"),
            'h' => Some("key.keyboard.h"),
            'i' => Some("key.keyboard.i"),
            'j' => Some("key.keyboard.j"),
            'k' => Some("key.keyboard.k"),
            'l' => Some("key.keyboard.l"),
            'm' => Some("key.keyboard.m"),
            'n' => Some("key.keyboard.n"),
            'o' => Some("key.keyboard.o"),
            'p' => Some("key.keyboard.p"),
            'q' => Some("key.keyboard.q"),
            'r' => Some("key.keyboard.r"),
            's' => Some("key.keyboard.s"),
            't' => Some("key.keyboard.t"),
            'u' => Some("key.keyboard.u"),
            'v' => Some("key.keyboard.v"),
            'w' => Some("key.keyboard.w"),
            'x' => Some("key.keyboard.x"),
            'y' => Some("key.keyboard.y"),
            'z' => Some("key.keyboard.z"),
            '0' | ')' => Some("key.keyboard.0"),
            '1' | '!' => Some("key.keyboard.1"),
            '2' | '@' => Some("key.keyboard.2"),
            '3' | '#' => Some("key.keyboard.3"),
            '4' | '$' => Some("key.keyboard.4"),
            '5' | '%' => Some("key.keyboard.5"),
            '6' | '^' => Some("key.keyboard.6"),
            '7' | '&' => Some("key.keyboard.7"),
            '8' | '*' => Some("key.keyboard.8"),
            '9' | '(' => Some("key.keyboard.9"),
            ' ' => Some("key.keyboard.space"),
            '/' | '?' => Some("key.keyboard.slash"),
            '.' | '>' => Some("key.keyboard.period"),
            ',' | '<' => Some("key.keyboard.comma"),
            ';' | ':' => Some("key.keyboard.semicolon"),
            '\'' | '"' => Some("key.keyboard.apostrophe"),
            '[' | '{' => Some("key.keyboard.left.bracket"),
            ']' | '}' => Some("key.keyboard.right.bracket"),
            '-' | '_' => Some("key.keyboard.minus"),
            '=' | '+' => Some("key.keyboard.equal"),
            '\\' | '|' => Some("key.keyboard.backslash"),
            '`' | '~' => Some("key.keyboard.grave.accent"),
            _ => None,
        }
    }
    pub fn capture_custom_key_code(&mut self, code: winit::keyboard::KeyCode) -> bool {
        let Some(name) = crate::keybinds::key_name(code) else {
            return false;
        };
        self.capture_custom_key(name)
    }
    pub fn capture_custom_text(&mut self, text: &str) -> bool {
        let Some(ch) = text.chars().find(|c| !c.is_control()) else {
            return false;
        };
        let Some(token) = Self::char_to_key_token(ch) else {
            return false;
        };
        self.capture_custom_key(token)
    }
    pub fn capture_custom_key(&mut self, name: &str) -> bool {
        if crate::keybinds::key_code(name).is_none() {
            return false;
        }
        let is_mcpe = self.settings.control_style == ControlStyle::Mcpe;
        if let Some(e) = &mut self.editor
            && (e.key_picker || e.picker_step != PickerStep::None)
            && let Some(i) = e.selected_custom
            && let Some(btn) = self.settings.custom_buttons.get_mut(i)
        {
            let slot = match e.picker_step {
                PickerStep::AskInputType { event_slot }
                | PickerStep::Keyboard { event_slot }
                | PickerStep::MouseAndActions { event_slot } => event_slot,
                _ => 0,
            };
            let lbl = super::custom::friendly_token_name(name);
            if is_mcpe {
                btn.set_exclusive_binding(name.into());
                btn.custom_label = lbl.clone();
            } else {
                btn.set_event_slot(slot, name.into());
                if slot == 0 && (btn.custom_label.is_empty() || btn.custom_label == "BTN") {
                    btn.custom_label = lbl.clone();
                }
            }
            e.key_picker = false;
            e.picker_step = PickerStep::None;
            e.status_msg = Some(format!("Captured key '{lbl}'."));
            let _ = self.settings.save();
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
            && let Some(btn) = self.settings.custom_buttons.get(i)
            && !e.palette
            && !e.key_picker
        {
            let p = btn.layouts[self.settings.mode.index()];
            plate(b, a, [0.32, 0.46, 0.115, 0.06], "PassThru", p.passthrough);
            plate(b, a, [0.442, 0.46, 0.115, 0.06], "Toggle", p.toggleable);
            plate(b, a, [0.565, 0.46, 0.115, 0.06], "Swipe", p.swipeable);
            plate(
                b,
                a,
                [0.32, 0.54, 0.36, 0.07],
                &format!("Action: {}", btn.label()),
                false,
            );
            plate(
                b,
                a,
                [0.32, 0.79, 0.17, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(b, a, [0.51, 0.79, 0.17, 0.07], "Size", e.option == Some(0));
            plate(b, a, [0.32, 0.88, 0.36, 0.07], "Remove button", false);
            if let Some(option) = e.option {
                let value = if option == 0 {
                    (p.size - 0.045) / 0.405
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
                let percent = if option == 0 {
                    p.size * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!(
                        "{} {:.0}%",
                        if option == 0 { "Size" } else { "Opacity" },
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
fn key_rect(i: usize) -> [f32; 4] {
    [0.15, 0.25 + i as f32 * 0.072, 0.7, 0.06]
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[test]
    fn interrupted_drag_preserves_editor_changes_and_does_not_accept_picker() {
        let mut tc = TouchControls::default();
        tc.open_editor();
        tc.settings.left_handed = !tc.settings.left_handed;
        let expected = tc.settings.clone();
        let ed = tc.editor.as_mut().unwrap();
        ed.picker_step = PickerStep::Keyboard { event_slot: 0 };
        ed.active_drag = Some((
            7,
            EditorDragKind::KeyboardScroll {
                last_x: 0.0,
                moved: false,
            },
        ));
        tc.clear();
        assert_eq!(tc.settings, expected);
        let ed = tc.editor.as_ref().unwrap();
        assert!(ed.active_drag.is_none());
        assert_eq!(ed.picker_step, PickerStep::Keyboard { event_slot: 0 });
        tc.cancel_editor();
        assert_ne!(tc.settings.left_handed, expected.left_handed);
    }
}
