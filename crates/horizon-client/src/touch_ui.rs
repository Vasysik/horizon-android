//! Android menu gestures and auxiliary keys, independent of gameplay fingers.
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    osk::{Keyboard, Output},
};
use horizon_client_input::control_layout::{
    calculate_button_bounds, calculate_joystick_bounds, LayoutEditor, LayoutRuntime, LayoutSummary,
    SelectedWidget, SnapLine, TextAlignment,
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use winit::{event::TouchPhase, keyboard::KeyCode};

#[derive(Default)]
pub struct TouchUi {
    pub expanded: bool,
    pub virtual_mouse: bool,
    mouse_buttons: BTreeMap<u64, bool>,
    relative: Option<RelativeTouch>,
    relative_click: Option<bool>,
    pub keyboard: Option<Keyboard>,
    gesture: Option<(u64, [f32; 2], [f32; 2], bool)>,
    pointer_drag: Option<u64>,
    held_keys: BTreeMap<u64, KeyCode>,
    pub f3_latched: bool,
    pub control_runtime: Option<LayoutRuntime>,
    pub replacement_dialog: Option<ReplacementControlDialog>,
    pub editor: Option<LayoutEditor>,
}

#[derive(Debug, Clone)]
pub struct ReplacementControlDialog {
    pub open: bool,
    pub layouts: Vec<LayoutSummary>,
    pub selected_index: usize,
    pub scroll: f32,
}

impl ReplacementControlDialog {
    pub fn new(layouts: Vec<LayoutSummary>) -> Self {
        let selected_index = layouts.iter().position(|l| l.is_active).unwrap_or(0);
        Self {
            open: true,
            layouts,
            selected_index,
            scroll: 0.0,
        }
    }
}
struct RelativeTouch {
    id: u64,
    origin: [f32; 2],
    last: [f32; 2],
    since: Instant,
    dragged: bool,
    right_sent: bool,
}
pub enum Gesture {
    None,
    Tap,
    Scroll(f32),
}
pub enum Action {
    LatchF3,
    Pick,
    DebugCombo(KeyCode),
    Key(KeyCode),
    Keyboard,
    VirtualMouse,
    MouseButton(bool),
    More,
    OpenControlDialog,
    SelectControlLayout(String),
    CloseControlDialog,
    OpenControlEditor,
    CloseControlEditor,
    SaveControlEditor,
    EditorAddButton,
    EditorAddJoystick,
    EditorDuplicate,
    EditorDelete,
    EditorToggleLeftDrawer,
    EditorToggleRightDrawer,
    EditorTogglePreview,
    EditorToggleFocusMode,
    EditorAddLayer,
    EditorSelectLayer(usize),
    EditorToggleLayerHide(usize),
}

const KEYS: [(&str, KeyCode); 28] = [
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("Tab", KeyCode::Tab),
    ("Swap", KeyCode::KeyF),
    ("Drop", KeyCode::KeyQ),
    ("Adv", KeyCode::KeyL),
    ("Chat", KeyCode::KeyT),
    ("/", KeyCode::Slash),
    ("Enter", KeyCode::Enter),
    ("Esc", KeyCode::Escape),
    ("Shift", KeyCode::ShiftLeft),
    ("Ctrl", KeyCode::ControlLeft),
    ("Inv", KeyCode::KeyE),
    ("Pick", KeyCode::ContextMenu),
    ("F3+B", KeyCode::KeyB),
    ("F3+G", KeyCode::KeyG),
    ("F3+A", KeyCode::KeyA),
    ("F3+T", KeyCode::KeyT),
];

// Shared draw/hit rectangles: utility-style buttons at the lower right.
fn mouse_button_rects(w: f32, h: f32) -> [([f32; 4], bool); 2] {
    [
        ([w - 53.0, h - 51.0, 50.0, 20.0], false),
        ([w - 53.0, h - 27.0, 50.0, 20.0], true),
    ]
}

/// Shared outlined pointer for virtual mouse and controller menus.
pub fn draw_pointer(batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32)) {
    let (x, y) = (cursor.0.floor(), cursor.1.floor());
    const ROWS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 4, 2, 1];
    for (i, &w) in ROWS.iter().enumerate() {
        batch.fill(
            atlas,
            x - 1.0,
            y + i as f32,
            f32::from(w) + 2.0,
            1.0,
            rgba(0, 0, 0, 255),
        );
    }
    batch.fill(atlas, x - 1.0, y + 10.0, 2.0, 1.0, rgba(0, 0, 0, 255));
    for (i, &w) in ROWS.iter().enumerate().skip(1).take(8) {
        batch.fill(
            atlas,
            x,
            y + i as f32,
            f32::from(w) - 1.0,
            1.0,
            rgba(255, 255, 255, 255),
        );
    }
}

impl TouchUi {
    pub fn key_down(&mut self, id: u64, key: KeyCode) -> bool {
        let first = !self.key_held(key);
        self.held_keys.insert(id, key);
        first
    }
    pub fn key_up(&mut self, id: u64) -> Option<KeyCode> {
        let key = self.held_keys.remove(&id)?;
        (!self.key_held(key)).then_some(key)
    }
    pub fn owns_key(&self, id: u64) -> bool {
        self.held_keys.contains_key(&id)
    }
    pub fn key_held(&self, key: KeyCode) -> bool {
        (key == KeyCode::F3 && self.f3_latched) || self.held_keys.values().any(|k| *k == key)
    }
    pub fn release_keys(&mut self) -> Vec<KeyCode> {
        let mut keys: Vec<_> = std::mem::take(&mut self.held_keys).into_values().collect();
        if self.f3_latched {
            keys.push(KeyCode::F3);
        }
        self.f3_latched = false;
        keys.sort();
        keys.dedup();
        keys
    }
    pub fn press_mouse(&mut self, id: u64, right: bool) -> bool {
        if let Some(touch) = &mut self.relative {
            touch.dragged = true;
        }
        let first = !self.mouse_buttons.values().any(|v| *v == right);
        self.mouse_buttons.insert(id, right);
        first
    }
    pub fn release_mouse(&mut self, id: u64) -> Option<(bool, bool)> {
        let right = self.mouse_buttons.remove(&id)?;
        Some((right, !self.mouse_buttons.values().any(|v| *v == right)))
    }
    pub fn mouse_held(&self) -> bool {
        !self.mouse_buttons.is_empty()
    }
    pub fn release_all_mouse(&mut self) -> Vec<bool> {
        let mut buttons: Vec<_> = std::mem::take(&mut self.mouse_buttons)
            .into_values()
            .collect();
        buttons.sort();
        buttons.dedup();
        buttons
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.pointer_drag = None;
        self.relative = None;
        self.relative_click = None;
    }
    pub fn cancel_relative(&mut self) {
        self.relative = None;
        self.relative_click = None;
    }
    /// Relative motion: touching down does not teleport the inventory cursor.
    pub fn relative_touch(
        &mut self,
        id: u64,
        point: [f32; 2],
        phase: TouchPhase,
        cursor: [f32; 2],
        size: [f32; 2],
        sensitivity: f32,
        clicks: bool,
    ) -> [f32; 2] {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return cursor;
        }
        if phase == TouchPhase::Started && self.relative.is_none() {
            self.relative = Some(RelativeTouch {
                id,
                origin: point,
                last: point,
                since: Instant::now(),
                dragged: !clicks,
                right_sent: false,
            });
        }
        let Some(touch) = self.relative.as_mut().filter(|t| t.id == id) else {
            return cursor;
        };
        touch.dragged |= (point[0] - touch.origin[0]).hypot(point[1] - touch.origin[1]) > 3.0;
        let position = [0, 1].map(|i| {
            (cursor[i] + (point[i] - touch.last[i]) * sensitivity)
                .clamp(0.0, (size[i] - 1.0).max(0.0))
        });
        touch.last = point;
        if phase == TouchPhase::Ended {
            if clicks && !touch.dragged && !touch.right_sent {
                self.relative_click = Some(touch.since.elapsed() >= Duration::from_millis(500));
            }
            self.relative = None;
        }
        position
    }
    /// false = LMB, true = RMB. A stationary hold clicks once, even before release.
    pub fn relative_click(&mut self, clicks: bool) -> Option<bool> {
        if !clicks {
            self.relative_click = None;
            return None;
        }
        if let Some(right) = self.relative_click.take() {
            return Some(right);
        }
        let touch = self.relative.as_mut()?;
        if !touch.dragged
            && !touch.right_sent
            && touch.since.elapsed() >= Duration::from_millis(500)
        {
            touch.right_sent = true;
            return Some(true);
        }
        None
    }
    /// Sliders retain mouse dragging; list rows use scroll-or-tap gestures.
    pub fn direct_pointer(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> bool {
        if phase == TouchPhase::Started
            && crate::widgets::last_targets().iter().any(|t| {
                let [x, y, w, h] = t.rect;
                matches!(t.kind, crate::widgets::Kind::Slider { .. })
                    && (x..x + w).contains(&p[0])
                    && (y..y + h).contains(&p[1])
            })
        {
            self.pointer_drag = Some(id);
        }
        if self.pointer_drag == Some(id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pointer_drag = None;
            }
            return true;
        }
        false
    }
    /// A drag scrolls; a tap clicks only on release. Never activate a row while scrolling it.
    pub fn gesture(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> Gesture {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return Gesture::None;
        }
        if phase == TouchPhase::Started {
            if self.gesture.is_none() {
                self.gesture = Some((id, p, p, false));
            }
            return Gesture::None;
        }
        let Some((owner, start, previous, dragged)) = &mut self.gesture else {
            return Gesture::None;
        };
        if *owner != id {
            return Gesture::None;
        }
        *dragged |= (p[0] - start[0]).hypot(p[1] - start[1]) > 12.0;
        let dy = p[1] - previous[1];
        *previous = p;
        if phase == TouchPhase::Ended {
            let tap = !*dragged;
            self.cancel();
            return if tap { Gesture::Tap } else { Gesture::None };
        }
        if *dragged {
            Gesture::Scroll(dy / 30.0)
        } else {
            Gesture::None
        }
    }
    pub fn action(&self, w: f32, h: f32, p: [f32; 2], menu: bool) -> Option<Action> {
        if let Some(editor) = &self.editor {
            // Check top header bar
            if (3.0..25.0).contains(&p[1]) {
                if (4.0..46.0).contains(&p[0]) {
                    return Some(Action::EditorToggleLeftDrawer);
                }
                if (50.0..98.0).contains(&p[0]) {
                    return Some(Action::EditorToggleRightDrawer);
                }
                if (102.0..176.0).contains(&p[0]) {
                    return Some(Action::EditorTogglePreview);
                }
                if (w - 90.0..w - 50.0).contains(&p[0]) {
                    return Some(Action::SaveControlEditor);
                }
                if (w - 46.0..w - 4.0).contains(&p[0]) {
                    return Some(Action::CloseControlEditor);
                }
            }

            // Check Left Drawer
            if editor.left_drawer_open && p[0] <= 160.0 && p[1] >= 26.0 {
                if (48.0..72.0).contains(&p[1]) {
                    return Some(Action::EditorAddButton);
                }
                if (74.0..98.0).contains(&p[1]) {
                    return Some(Action::EditorAddJoystick);
                }
                if (100.0..124.0).contains(&p[1]) {
                    return Some(Action::EditorDuplicate);
                }
                if (126.0..150.0).contains(&p[1]) {
                    return Some(Action::EditorDelete);
                }
                if (152.0..176.0).contains(&p[1]) {
                    return Some(Action::EditorToggleFocusMode);
                }
                if (178.0..202.0).contains(&p[1]) {
                    return Some(Action::SaveControlEditor);
                }
                if (204.0..228.0).contains(&p[1]) {
                    return Some(Action::CloseControlEditor);
                }
                return None; // swallow click inside left drawer
            }

            // Check Right Drawer
            if editor.right_drawer_open && p[0] >= w - 180.0 && p[1] >= 26.0 {
                let dx = w - 180.0;
                if (48.0..72.0).contains(&p[1]) {
                    return Some(Action::EditorAddLayer);
                }
                for (idx, _) in editor.layout.layers.iter().enumerate() {
                    let ly = 78.0 + (idx as f32 * 30.0);
                    if (ly..ly + 26.0).contains(&p[1]) {
                        if (dx + 180.0 - 44.0..dx + 180.0 - 12.0).contains(&p[0]) {
                            return Some(Action::EditorToggleLayerHide(idx));
                        } else {
                            return Some(Action::EditorSelectLayer(idx));
                        }
                    }
                }
                return None; // swallow click inside right drawer
            }

            return None;
        }

        if let Some(dialog) = &self.replacement_dialog {
            if dialog.open {
                let dw: f32 = 320.0_f32.min(w - 24.0);
                let dh: f32 = 240.0_f32.min(h - 24.0);
                let dx = (w - dw) * 0.5;
                let dy = (h - dh) * 0.5;

                // Close button: upper right of dialog card
                if (dx + dw - 36.0..dx + dw - 6.0).contains(&p[0]) && (dy + 6.0..dy + 28.0).contains(&p[1]) {
                    return Some(Action::CloseControlDialog);
                }

                // Edit button: bottom right
                if (dx + dw - 85.0..dx + dw - 10.0).contains(&p[0]) && (dy + dh - 32.0..dy + dh - 8.0).contains(&p[1]) {
                    return Some(Action::OpenControlEditor);
                }

                // Layout list entries
                let list_top = dy + 36.0;
                let list_bottom = dy + dh - 36.0;
                if (dx + 12.0..dx + dw - 12.0).contains(&p[0]) && (list_top..list_bottom).contains(&p[1]) {
                    let item_y = p[1] - list_top + dialog.scroll;
                    let idx = (item_y / 36.0) as usize;
                    if let Some(item) = dialog.layouts.get(idx) {
                        return Some(Action::SelectControlLayout(item.file_stem.clone()));
                    }
                }

                // Absorb any other click inside dialog bounds
                if (dx..dx + dw).contains(&p[0]) && (dy..dy + dh).contains(&p[1]) {
                    return None;
                }
            }
        }

        if menu && self.virtual_mouse {
            for ([x, y, width, height], right) in mouse_button_rects(w, h) {
                if (x..x + width).contains(&p[0]) && (y..y + height).contains(&p[1]) {
                    return Some(Action::MouseButton(right));
                }
            }
        }
        if menu && (3.0..23.0).contains(&p[1]) && (w - 108.0..w - 56.0).contains(&p[0]) {
            return Some(Action::VirtualMouse);
        }
        if (3.0..23.0).contains(&p[1]) {
            if menu && (w - 53.0..w - 3.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::Escape));
            }
            if (3.0..36.0).contains(&p[0]) {
                return Some(Action::More);
            }
            if self.expanded && (39.0..72.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F3));
            }
            if self.expanded && (75.0..108.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F5));
            }
            if self.expanded && (111.0..144.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::KeyT));
            }
            if self.expanded && (147.0..186.0).contains(&p[0]) {
                return Some(Action::Keyboard);
            }
            if self.expanded && (189.0..235.0).contains(&p[0]) {
                return Some(Action::OpenControlDialog);
            }
        }
        if self.expanded && (3.0..203.0).contains(&p[0]) && (27.0..219.0).contains(&p[1]) {
            let i = ((p[1] - 27.0) / 24.0) as usize * 4 + ((p[0] - 3.0) / 50.0) as usize;
            if i == 28 {
                return Some(Action::LatchF3);
            }
            return KEYS.get(i).map(|(_, k)| match i {
                23 => Action::Pick,
                24.. => Action::DebugCombo(*k),
                _ => Action::Key(*k),
            });
        }
        None
    }
    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas, menu: bool) {
        if let Some(editor) = &self.editor {
            draw_layout_editor(b, a, editor);
            return;
        }

        if let Some(runtime) = &self.control_runtime {
            draw_custom_controls(b, a, runtime);
        }

        for (label, x, width) in [
            ("...", 3.0, 33.0),
            ("F3", 39.0, 33.0),
            ("F5", 75.0, 33.0),
            ("Chat", 111.0, 33.0),
            ("Keys", 147.0, 39.0),
            ("Layout", 189.0, 44.0),
        ] {
            if label != "..." && !self.expanded {
                continue;
            }
            button(
                b,
                a,
                if label == "F3" && self.key_held(KeyCode::F3) {
                    "F3*"
                } else {
                    label
                },
                x,
                3.0,
                width,
            );
        }
        if menu {
            button(
                b,
                a,
                if self.virtual_mouse {
                    "Mouse ON"
                } else {
                    "Mouse"
                },
                b.width - 108.0,
                3.0,
                50.0,
            );
        }
        if menu && self.virtual_mouse {
            for ([x, y, width, _], right) in mouse_button_rects(b.width, b.height) {
                button(b, a, if right { "RMB" } else { "LMB" }, x, y, width);
            }
        }
        if menu {
            button(b, a, "Back", b.width - 53.0, 3.0, 50.0);
        }
        if self.expanded {
            button(
                b,
                a,
                if self.f3_latched { "F3 ON" } else { "HoldF3" },
                3.0,
                195.0,
                48.0,
            );
            for (i, (label, _)) in KEYS.iter().enumerate() {
                button(
                    b,
                    a,
                    label,
                    3.0 + (i % 4) as f32 * 50.0,
                    27.0 + (i / 4) as f32 * 24.0,
                    48.0,
                );
            }
        }

        if let Some(dialog) = &self.replacement_dialog {
            if dialog.open {
                draw_replacement_dialog(b, a, dialog);
            }
        }
    }
    pub fn keyboard_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        p: [f32; 2],
    ) -> Option<Output> {
        let ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
        self.keyboard.as_mut()?.touch(
            ui.w,
            ui.h,
            false,
            [
                p[0] * ui.batch.scale / ui.density,
                p[1] * ui.batch.scale / ui.density,
            ],
        )
    }
    pub fn draw_keyboard(&self, b: &mut HudBatch, a: &HudAtlas) {
        if let Some(k) = &self.keyboard {
            let mut ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
            k.draw(&mut ui, false, crate::gamepad::GlyphStyle::Xbox);
            ui.finish();
        }
    }
    pub fn ensure_control_runtime(&mut self, data_dir: &std::path::Path, active_slug: &str) {
        if self.control_runtime.is_none() {
            let storage = horizon_client_input::control_layout::LayoutStorage::new(
                data_dir.join("control_layouts"),
                active_slug,
            );
            let _ = storage.ensure_default_layout();
            let layout = storage.load_layout(active_slug).unwrap_or_else(|_| {
                horizon_client_input::control_layout::LayoutStorage::default_layout()
            });
            self.control_runtime = Some(LayoutRuntime::new(layout));
        }
    }
    pub fn open_editor(&mut self, data_dir: &std::path::Path, slug: &str) {
        let storage = horizon_client_input::control_layout::LayoutStorage::new(
            data_dir.join("control_layouts"),
            slug,
        );
        let _ = storage.ensure_default_layout();
        let layout = storage
            .load_layout(slug)
            .unwrap_or_else(|_| horizon_client_input::control_layout::LayoutStorage::default_layout());
        let mut editor = LayoutEditor::new(layout, slug);
        if let Some(runtime) = &self.control_runtime {
            editor.set_dimensions(
                runtime.screen_width(),
                runtime.screen_height(),
                runtime.display_density(),
            );
        }
        self.editor = Some(editor);
        self.replacement_dialog = None;
    }
    pub fn close_editor(&mut self) {
        self.editor = None;
    }
}

fn draw_layout_editor(b: &mut HudBatch, a: &HudAtlas, editor: &LayoutEditor) {
    let (w, h) = (b.width, b.height);
    let sw = editor.screen_width;
    let sh = editor.screen_height;
    let density = editor.display_density;

    // 1. Canvas background & grid lines
    b.fill(a, 0.0, 0.0, w, h, rgba(14, 16, 22, 255));
    let mut gx = 0.0;
    while gx < w {
        b.fill(a, gx, 0.0, 1.0, h, rgba(25, 29, 38, 120));
        gx += 50.0;
    }
    let mut gy = 0.0;
    while gy < h {
        b.fill(a, 0.0, gy, w, 1.0, rgba(25, 29, 38, 120));
        gy += 50.0;
    }

    // Center crosshair (5000, 5000)
    b.fill(a, w * 0.5, 0.0, 1.0, h, rgba(40, 50, 70, 160));
    b.fill(a, 0.0, h * 0.5, w, 1.0, rgba(40, 50, 70, 160));

    // 2. Snap guidelines
    for snap in &editor.active_snap_lines {
        match snap {
            SnapLine::Horizontal(y) => {
                b.fill(a, 0.0, *y, w, 1.5, rgba(0, 240, 255, 240));
            }
            SnapLine::Vertical(x) => {
                b.fill(a, *x, 0.0, 1.5, h, rgba(0, 240, 255, 240));
            }
        }
    }

    // 3. Render layout elements across layers
    for (layer_idx, layer) in editor.layout.layers.iter().enumerate() {
        if editor.layer_focus_mode && layer_idx != editor.active_layer_index {
            continue;
        }
        let alpha_mult = if layer.hide { 0.4 } else { 1.0 };

        // Normal buttons
        for btn in &layer.normal_buttons {
            let (bx, by, bw, bh) =
                calculate_button_bounds(&btn.position, &btn.button_size, sw, sh, density);
            let is_selected = editor.selected == Some(SelectedWidget::Button(btn.uuid.clone()));

            let bg = if is_selected {
                rgba(50, 85, 130, (220.0 * alpha_mult) as u8)
            } else {
                rgba(24, 28, 38, (180.0 * alpha_mult) as u8)
            };
            b.fill(a, bx, by, bw, bh, bg);

            // Border
            let border_color = if is_selected {
                rgba(0, 220, 255, 255)
            } else {
                rgba(70, 85, 110, (180.0 * alpha_mult) as u8)
            };
            b.fill(a, bx, by, bw, 1.0, border_color);
            b.fill(a, bx, by + bh - 1.0, bw, 1.0, border_color);
            b.fill(a, bx, by, 1.0, bh, border_color);
            b.fill(a, bx + bw - 1.0, by, 1.0, bh, border_color);

            let label = btn.text.resolve("en");
            let tx = bx + 6.0;
            let ty = by + (bh - 10.0) * 0.5;
            b.text(a, label, tx, ty, rgba(240, 240, 240, 255), true);

            // Resize handles if selected
            if is_selected {
                b.fill(a, bx - 6.0, by - 6.0, 12.0, 12.0, rgba(0, 240, 255, 255));
                b.fill(
                    a,
                    bx + bw - 6.0,
                    by + bh - 6.0,
                    12.0,
                    12.0,
                    rgba(0, 240, 255, 255),
                );
            }
        }

        // Joysticks
        for joy in &layer.joystick_buttons {
            let (jx, jy, jr) = calculate_joystick_bounds(joy, sw, sh, density);
            let is_selected = editor.selected == Some(SelectedWidget::Joystick(joy.uuid.clone()));

            let border_color = if is_selected {
                rgba(0, 220, 255, 255)
            } else {
                rgba(80, 100, 130, (160.0 * alpha_mult) as u8)
            };
            b.fill(
                a,
                jx - jr,
                jy - jr,
                jr * 2.0,
                jr * 2.0,
                rgba(22, 27, 38, (120.0 * alpha_mult) as u8),
            );
            b.fill(a, jx - jr, jy - jr, jr * 2.0, 1.0, border_color);
            b.fill(a, jx - jr, jy + jr - 1.0, jr * 2.0, 1.0, border_color);
            b.fill(a, jx - jr, jy - jr, 1.0, jr * 2.0, border_color);
            b.fill(a, jx + jr - 1.0, jy - jr, 1.0, jr * 2.0, border_color);

            // Center thumb
            let thumb_r = jr * 0.35;
            b.fill(
                a,
                jx - thumb_r,
                jy - thumb_r,
                thumb_r * 2.0,
                thumb_r * 2.0,
                rgba(80, 120, 180, 200),
            );
            b.text(
                a,
                "JOYSTICK",
                jx - 20.0,
                jy - 5.0,
                rgba(220, 230, 245, 255),
                false,
            );
        }
    }

    // 4. Top Header Bar
    b.fill(a, 0.0, 0.0, w, 26.0, rgba(20, 24, 32, 240));
    b.fill(a, 0.0, 25.0, w, 1.0, rgba(60, 75, 100, 200));

    button(b, a, "Menu", 4.0, 3.0, 42.0);
    button(b, a, "Layers", 50.0, 3.0, 48.0);
    let preview_label = if editor.preview_mode {
        "Preview: ON"
    } else {
        "Preview: OFF"
    };
    button(b, a, preview_label, 102.0, 3.0, 74.0);

    let title = if editor.is_dirty {
        format!("Editor: {}*", editor.layout.info.name.resolve("en"))
    } else {
        format!("Editor: {}", editor.layout.info.name.resolve("en"))
    };
    b.text(a, &title, 185.0, 7.0, rgba(255, 255, 255, 255), true);

    button(b, a, "Save", w - 90.0, 3.0, 40.0);
    button(b, a, "Exit", w - 46.0, 3.0, 42.0);

    // 5. Left Drawer (Editor Menu)
    if editor.left_drawer_open {
        let dw = 160.0;
        b.fill(a, 0.0, 26.0, dw, h - 26.0, rgba(22, 26, 36, 250));
        b.fill(a, dw - 1.0, 26.0, 1.0, h - 26.0, rgba(70, 85, 115, 220));

        b.text(a, "EDITOR MENU", 10.0, 32.0, rgba(0, 220, 255, 255), true);
        button(b, a, "+ Add Button", 10.0, 48.0, 140.0);
        button(b, a, "+ Add Joystick", 10.0, 74.0, 140.0);
        button(b, a, "Duplicate Sel", 10.0, 100.0, 140.0);
        button(b, a, "Delete Sel", 10.0, 126.0, 140.0);
        let focus_label = if editor.layer_focus_mode {
            "Focus: ON"
        } else {
            "Focus: OFF"
        };
        button(b, a, focus_label, 10.0, 152.0, 140.0);
        button(b, a, "Save Layout", 10.0, 178.0, 140.0);
        button(b, a, "Exit Editor", 10.0, 204.0, 140.0);
    }

    // 6. Right Drawer (Layers Menu)
    if editor.right_drawer_open {
        let dw = 180.0;
        let dx = w - dw;
        b.fill(a, dx, 26.0, dw, h - 26.0, rgba(22, 26, 36, 250));
        b.fill(a, dx, 26.0, 1.0, h - 26.0, rgba(70, 85, 115, 220));

        b.text(a, "LAYERS MENU", dx + 10.0, 32.0, rgba(0, 220, 255, 255), true);
        button(b, a, "+ Add Layer", dx + 10.0, 48.0, 160.0);

        for (idx, layer) in editor.layout.layers.iter().enumerate() {
            let ly = 78.0 + (idx as f32 * 30.0);
            if ly + 28.0 > h {
                break;
            }
            let is_active = idx == editor.active_layer_index;
            let bg = if is_active {
                rgba(40, 70, 110, 200)
            } else {
                rgba(30, 34, 46, 160)
            };
            b.fill(a, dx + 8.0, ly, dw - 16.0, 26.0, bg);

            let layer_name = format!("{}. {}", idx + 1, layer.name);
            b.text(
                a,
                &layer_name,
                dx + 12.0,
                ly + 6.0,
                rgba(240, 240, 240, 255),
                is_active,
            );

            let eye = if layer.hide { "[-]" } else { "[V]" };
            button(b, a, eye, dx + dw - 44.0, ly + 2.0, 32.0);
        }
    }
}

fn draw_custom_controls(b: &mut HudBatch, a: &HudAtlas, runtime: &LayoutRuntime) {
    let layout = runtime.layout();
    let sw = runtime.screen_width();
    let sh = runtime.screen_height();
    let density = runtime.display_density();

    for layer in &layout.layers {
        if !runtime.is_layer_visible(layer) {
            continue;
        }

        // Draw normal buttons
        for btn in &layer.normal_buttons {
            let (bx, by, bw, bh) = calculate_button_bounds(&btn.position, &btn.button_size, sw, sh, density);
            let is_pressed = runtime
                .state()
                .buttons_state
                .get(&btn.uuid)
                .map(|s| s.is_pressed)
                .unwrap_or(false);

            // Button body
            let bg = if is_pressed {
                rgba(60, 80, 110, 210)
            } else {
                rgba(22, 26, 35, 160)
            };
            b.fill(a, bx, by, bw, bh, bg);

            // Border outline
            let border = if is_pressed {
                rgba(120, 160, 220, 240)
            } else {
                rgba(70, 80, 100, 180)
            };
            b.fill(a, bx, by, bw, 1.0, border);
            b.fill(a, bx, by + bh - 1.0, bw, 1.0, border);
            b.fill(a, bx, by, 1.0, bh, border);
            b.fill(a, bx + bw - 1.0, by, 1.0, bh, border);

            // Button label
            let text = btn.text.resolve("en");
            let text_w = text.len() as f32 * 6.0;
            let tx = match btn.text_alignment {
                TextAlignment::Left => bx + 6.0,
                TextAlignment::Center => bx + (bw - text_w).max(0.0) * 0.5,
                TextAlignment::Right => bx + bw - text_w - 6.0,
            };
            let ty = by + (bh - 10.0) * 0.5;
            b.text(a, text, tx, ty, rgba(245, 245, 245, 255), true);
        }

        // Draw joysticks
        for joy in &layer.joystick_buttons {
            let (jx, jy, jr) = calculate_joystick_bounds(joy, sw, sh, density);
            let state = runtime.state().joysticks_state.get(&joy.uuid);
            let (ox, oy) = state.map(|s| s.normalized_offset).unwrap_or((0.0, 0.0));
            let is_locked = state.map(|s| s.is_locked).unwrap_or(false);

            // Outer ring base
            b.fill(a, jx - jr, jy - jr, jr * 2.0, jr * 2.0, rgba(20, 25, 36, 120));
            b.fill(a, jx - jr, jy - jr, jr * 2.0, 1.0, rgba(80, 95, 120, 160));
            b.fill(a, jx - jr, jy + jr - 1.0, jr * 2.0, 1.0, rgba(80, 95, 120, 160));
            b.fill(a, jx - jr, jy - jr, 1.0, jr * 2.0, rgba(80, 95, 120, 160));
            b.fill(a, jx + jr - 1.0, jy - jr, 1.0, jr * 2.0, rgba(80, 95, 120, 160));

            // Thumb handle
            let handle_r = jr * 0.4;
            let hx = jx + ox * jr * 0.6;
            let hy = jy + oy * jr * 0.6;
            let handle_color = if is_locked {
                rgba(50, 180, 120, 220)
            } else {
                rgba(80, 110, 160, 200)
            };
            b.fill(a, hx - handle_r, hy - handle_r, handle_r * 2.0, handle_r * 2.0, handle_color);

            if is_locked {
                b.text(a, "LOCK", jx - 12.0, jy - jr - 14.0, rgba(80, 240, 150, 255), true);
            }
        }
    }
}

fn draw_replacement_dialog(b: &mut HudBatch, a: &HudAtlas, dialog: &ReplacementControlDialog) {
    // Dim background overlay
    b.fill(a, 0.0, 0.0, b.width, b.height, rgba(0, 0, 0, 150));

    let dw: f32 = 320.0_f32.min(b.width - 24.0);
    let dh: f32 = 240.0_f32.min(b.height - 24.0);
    let dx = (b.width - dw) * 0.5;
    let dy = (b.height - dh) * 0.5;

    // Dialog card
    b.fill(a, dx, dy, dw, dh, rgba(24, 27, 36, 250));
    b.fill(a, dx, dy, dw, 1.0, rgba(70, 85, 115, 255));
    b.fill(a, dx, dy + dh - 1.0, dw, 1.0, rgba(70, 85, 115, 255));
    b.fill(a, dx, dy, 1.0, dh, rgba(70, 85, 115, 255));
    b.fill(a, dx + dw - 1.0, dy, 1.0, dh, rgba(70, 85, 115, 255));

    // Title
    b.text(a, "Select Control Layout", dx + 12.0, dy + 10.0, rgba(255, 255, 255, 255), true);

    // Close button [X]
    button(b, a, "X", dx + dw - 36.0, dy + 6.0, 30.0);

    // Layout rows
    let list_top = dy + 36.0;
    let list_bottom = dy + dh - 36.0;
    for (i, item) in dialog.layouts.iter().enumerate() {
        let iy = list_top + (i as f32 * 36.0) - dialog.scroll;
        if iy < list_top - 30.0 || iy > list_bottom {
            continue;
        }

        let bg = if item.is_active {
            rgba(40, 75, 120, 220)
        } else {
            rgba(32, 36, 48, 180)
        };
        b.fill(a, dx + 12.0, iy, dw - 24.0, 32.0, bg);

        let label = if item.is_active {
            format!("{} (Active)", item.display_name)
        } else {
            item.display_name.clone()
        };
        b.text(a, &label, dx + 20.0, iy + 6.0, rgba(240, 240, 240, 255), true);
        let sub = format!("v{} by {}", item.version, item.author);
        b.text(a, &sub, dx + 20.0, iy + 18.0, rgba(160, 170, 190, 255), false);
    }

    // Bottom action button: Open Editor
    button(b, a, "Edit Layout", dx + dw - 95.0, dy + dh - 32.0, 85.0);
}

fn button(b: &mut HudBatch, a: &HudAtlas, label: &str, x: f32, y: f32, w: f32) {
    if !crate::touch::theme_button(b, a, "button", [x, y, w, 20.0], false, u32::MAX, true) {
        b.fill(a, x, y, w, 20.0, rgba(16, 18, 24, 180));
    }
    b.text(a, label, x + 4.0, y + 6.0, rgba(245, 245, 245, 255), true);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_mouse_drag_does_not_generate_an_extra_tap_or_hold_click() {
        let mut ui = TouchUi::default();
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Started,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        ui.press_mouse(2, false);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_millis(700);
        assert_eq!(ui.relative_click(false), None);
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Ended,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn mouse_toggle_is_available_without_a_setting_and_clicks_stay_on_right() {
        let mut ui = TouchUi::default();
        assert!(matches!(
            ui.action(427.0, 267.0, [350.0, 10.0], true),
            Some(Action::VirtualMouse)
        ));
        assert!(ui.action(427.0, 267.0, [350.0, 10.0], false).is_none());
        ui.virtual_mouse = true;
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 226.0], true),
            Some(Action::MouseButton(false))
        ));
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 250.0], true),
            Some(Action::MouseButton(true))
        ));
        assert!(ui.action(427.0, 267.0, [213.0, 250.0], true).is_none());
    }
    #[test]
    fn menu_mouse_buttons_share_ownership_and_release_outside_their_bounds() {
        let mut ui = TouchUi {
            virtual_mouse: true,
            ..Default::default()
        };
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 226.0], true),
            Some(Action::MouseButton(false))
        ));
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 250.0], true),
            Some(Action::MouseButton(true))
        ));
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], false).is_none());
        assert!(ui.press_mouse(1, false));
        assert!(!ui.press_mouse(2, false));
        assert!(ui.press_mouse(3, true));
        assert_eq!(ui.release_mouse(1), Some((false, false)));
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.release_all_mouse(), vec![true]);
        assert_eq!(ui.release_mouse(3), None);
        ui.virtual_mouse = false;
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], true).is_none());
    }
    #[test]
    fn virtual_mouse_moves_relatively_and_dragging_never_clicks() {
        let mut ui = TouchUi::default();
        let cursor = [100.0, 80.0];
        let size = [400.0, 240.0];
        assert_eq!(
            ui.relative_touch(
                1,
                [300.0, 200.0],
                TouchPhase::Started,
                cursor,
                size,
                2.0,
                true
            ),
            cursor
        );
        let p = ui.relative_touch(
            1,
            [310.0, 195.0],
            TouchPhase::Moved,
            cursor,
            size,
            2.0,
            true,
        );
        assert_eq!(p, [120.0, 70.0]);
        ui.relative_touch(1, [310.0, 195.0], TouchPhase::Ended, p, size, 2.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn virtual_mouse_tap_hold_cancel_and_click_preference() {
        let mut ui = TouchUi::default();
        let p = [100.0, 80.0];
        let size = [400.0, 240.0];
        ui.relative_touch(1, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative_touch(1, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), Some(false));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_secs(1);
        assert_eq!(ui.relative_click(true), Some(true));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(3, p, TouchPhase::Started, p, size, 1.0, false);
        ui.relative_touch(3, p, TouchPhase::Ended, p, size, 1.0, false);
        assert_eq!(ui.relative_click(false), None);
        ui.relative_touch(4, p, TouchPhase::Started, p, size, 1.0, true);
        ui.cancel_relative();
        ui.relative_touch(4, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn simultaneous_f3_f4_and_f6_release_independently() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(10, KeyCode::F3));
        assert!(ui.key_down(11, KeyCode::F4));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(11), Some(KeyCode::F4));
        assert!(ui.key_down(12, KeyCode::F6));
        assert_eq!(ui.key_up(12), Some(KeyCode::F6));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(10), Some(KeyCode::F3));
    }
    #[test]
    fn shared_key_and_latch_do_not_release_early() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(1, KeyCode::F3));
        assert!(!ui.key_down(2, KeyCode::F3));
        assert_eq!(ui.key_up(1), None);
        ui.f3_latched = true;
        assert_eq!(ui.key_up(2), None);
        assert_eq!(ui.release_keys(), vec![KeyCode::F3]);
        assert!(!ui.key_held(KeyCode::F3));
    }
    #[test]
    fn scrolling_never_clicks() {
        let mut u = TouchUi::default();
        assert!(matches!(
            u.gesture(1, [20.0, 200.0], TouchPhase::Started),
            Gesture::None
        ));
        assert!(
            matches!(u.gesture(1,[20.0,140.0],TouchPhase::Moved),Gesture::Scroll(v) if v == -2.0)
        );
        assert!(matches!(
            u.gesture(1, [20.0, 140.0], TouchPhase::Ended),
            Gesture::None
        ));
    }
    #[test]
    fn tap_release_cancel_and_second_finger() {
        let mut u = TouchUi::default();
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        assert!(matches!(
            u.gesture(2, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
        assert!(matches!(
            u.gesture(1, [21.0, 21.0], TouchPhase::Ended),
            Gesture::Tap
        ));
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        u.gesture(1, [20.0, 20.0], TouchPhase::Cancelled);
        assert!(matches!(
            u.gesture(1, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
    }
}
