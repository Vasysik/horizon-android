//! Clean-room layout editor state machine and geometry engine.
//!
//! Provides selection, dragging, multi-point resizing, magnetic snapping,
//! layer management, and element mutation in 0..10000 coordinate space.

use super::model::*;
use super::runtime::{calculate_button_bounds, calculate_joystick_bounds};
use super::storage::LayoutStorage;

/// Selected widget type in the editor canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedWidget {
    Button(String),
    Joystick(String),
    TextBox(String),
}

/// Dragging / resizing handle on a selected widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    BottomRight,
}

/// Active drag or resize gesture state.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorDrag {
    Move {
        pointer_id: u64,
        start_pixel: (f32, f32),
        orig_position: ButtonPosition,
    },
    Resize {
        pointer_id: u64,
        handle: ResizeHandle,
        start_pixel: (f32, f32),
        orig_position: ButtonPosition,
        orig_width_pct: i32,
        orig_height_pct: i32,
    },
}

/// Visual snapping guideline rendered on the editor canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapLine {
    Horizontal(f32),
    Vertical(f32),
}

/// Full-screen layout editor state machine.
#[derive(Debug, Clone)]
pub struct LayoutEditor {
    pub layout: ControlLayout,
    pub original_slug: String,
    pub active_layer_index: usize,
    pub selected: Option<SelectedWidget>,
    pub drag: Option<EditorDrag>,
    pub active_snap_lines: Vec<SnapLine>,
    pub left_drawer_open: bool,
    pub right_drawer_open: bool,
    pub property_dialog_open: bool,
    pub preview_mode: bool,
    pub preview_in_menu: bool,
    pub layer_focus_mode: bool,
    pub screen_width: f32,
    pub screen_height: f32,
    pub display_density: f32,
    pub is_dirty: bool,
}

impl LayoutEditor {
    /// Creates a new editor instance with the specified layout.
    pub fn new(layout: ControlLayout, slug: impl Into<String>) -> Self {
        Self {
            layout,
            original_slug: slug.into(),
            active_layer_index: 0,
            selected: None,
            drag: None,
            active_snap_lines: Vec::new(),
            left_drawer_open: false,
            right_drawer_open: false,
            property_dialog_open: false,
            preview_mode: false,
            preview_in_menu: false,
            layer_focus_mode: false,
            screen_width: 1280.0,
            screen_height: 720.0,
            display_density: 1.0,
            is_dirty: false,
        }
    }

    pub fn set_dimensions(&mut self, width: f32, height: f32, density: f32) {
        self.screen_width = width.max(100.0);
        self.screen_height = height.max(100.0);
        self.display_density = density.max(0.1);
    }

    /// Selects an element at the given pixel position, or returns false if tapped empty space.
    pub fn select_at(&mut self, pos: (f32, f32)) -> bool {
        let sw = self.screen_width;
        let sh = self.screen_height;
        let density = self.display_density;

        // Check layers in reverse order (topmost layer first)
        for (layer_idx, layer) in self.layout.layers.iter().enumerate().rev() {
            if self.layer_focus_mode && layer_idx != self.active_layer_index {
                continue;
            }

            // 1. Joysticks
            for joy in &layer.joystick_buttons {
                let (jx, jy, jr) = calculate_joystick_bounds(joy, sw, sh, density);
                let dx = pos.0 - jx;
                let dy = pos.1 - jy;
                if dx * dx + dy * dy <= jr * jr {
                    self.active_layer_index = layer_idx;
                    self.selected = Some(SelectedWidget::Joystick(joy.uuid.clone()));
                    return true;
                }
            }

            // 2. Normal Buttons
            for btn in &layer.normal_buttons {
                let (bx, by, bw, bh) = calculate_button_bounds(&btn.position, &btn.button_size, sw, sh, density);
                if pos.0 >= bx && pos.0 <= bx + bw && pos.1 >= by && pos.1 <= by + bh {
                    self.active_layer_index = layer_idx;
                    self.selected = Some(SelectedWidget::Button(btn.uuid.clone()));
                    return true;
                }
            }
        }

        self.selected = None;
        false
    }

    /// Begins a pointer drag or resize interaction on the selected element.
    pub fn start_drag(&mut self, pointer_id: u64, pos: (f32, f32)) -> bool {
        let sw = self.screen_width;
        let sh = self.screen_height;
        let density = self.display_density;

        let Some(selected) = &self.selected else {
            return false;
        };

        match selected {
            SelectedWidget::Button(uuid) => {
                let layer = match self.layout.layers.get(self.active_layer_index) {
                    Some(l) => l,
                    None => return false,
                };
                let btn = match layer.normal_buttons.iter().find(|b| &b.uuid == uuid) {
                    Some(b) => b,
                    None => return false,
                };

                let (bx, by, bw, bh) = calculate_button_bounds(&btn.position, &btn.button_size, sw, sh, density);
                let handle_radius = 20.0;

                // Check Top-Left handle
                if (pos.0 - bx).hypot(pos.1 - by) <= handle_radius {
                    self.drag = Some(EditorDrag::Resize {
                        pointer_id,
                        handle: ResizeHandle::TopLeft,
                        start_pixel: pos,
                        orig_position: btn.position,
                        orig_width_pct: btn.button_size.width_percentage,
                        orig_height_pct: btn.button_size.height_percentage,
                    });
                    return true;
                }

                // Check Bottom-Right handle
                if (pos.0 - (bx + bw)).hypot(pos.1 - (by + bh)) <= handle_radius {
                    self.drag = Some(EditorDrag::Resize {
                        pointer_id,
                        handle: ResizeHandle::BottomRight,
                        start_pixel: pos,
                        orig_position: btn.position,
                        orig_width_pct: btn.button_size.width_percentage,
                        orig_height_pct: btn.button_size.height_percentage,
                    });
                    return true;
                }

                // Otherwise, move entire button
                if pos.0 >= bx && pos.0 <= bx + bw && pos.1 >= by && pos.1 <= by + bh {
                    self.drag = Some(EditorDrag::Move {
                        pointer_id,
                        start_pixel: pos,
                        orig_position: btn.position,
                    });
                    return true;
                }
            }
            SelectedWidget::Joystick(uuid) => {
                let layer = match self.layout.layers.get(self.active_layer_index) {
                    Some(l) => l,
                    None => return false,
                };
                let joy = match layer.joystick_buttons.iter().find(|j| &j.uuid == uuid) {
                    Some(j) => j,
                    None => return false,
                };

                let (jx, jy, jr) = calculate_joystick_bounds(joy, sw, sh, density);
                if (pos.0 - jx).hypot(pos.1 - jy) <= jr {
                    self.drag = Some(EditorDrag::Move {
                        pointer_id,
                        start_pixel: pos,
                        orig_position: joy.position,
                    });
                    return true;
                }
            }
            SelectedWidget::TextBox(_) => {}
        }

        false
    }

    /// Updates active drag/resize movement with magnetic snapping to screen edges and widget guidelines.
    pub fn update_drag(&mut self, pointer_id: u64, pos: (f32, f32)) {
        let sw = self.screen_width;
        let sh = self.screen_height;
        self.active_snap_lines.clear();

        let drag = match &self.drag {
            Some(d) => d.clone(),
            None => return,
        };

        match drag {
            EditorDrag::Move {
                pointer_id: pid,
                start_pixel,
                orig_position,
            } => {
                if pid != pointer_id {
                    return;
                }

                let dx_px = pos.0 - start_pixel.0;
                let dy_px = pos.1 - start_pixel.1;

                let dx_norm = ((dx_px / sw) * 10000.0).round() as i32;
                let dy_norm = ((dy_px / sh) * 10000.0).round() as i32;

                let mut new_x = (orig_position.x + dx_norm).clamp(0, 10000);
                let mut new_y = (orig_position.y + dy_norm).clamp(0, 10000);

                // Magnetic snap to edges: 0%, 50%, 100% (within 100 coordinate threshold = 1%)
                const SNAP_THRESHOLD: i32 = 120;
                if new_x.abs() < SNAP_THRESHOLD {
                    new_x = 0;
                    self.active_snap_lines.push(SnapLine::Vertical(0.0));
                } else if (new_x - 5000).abs() < SNAP_THRESHOLD {
                    new_x = 5000;
                    self.active_snap_lines.push(SnapLine::Vertical(sw * 0.5));
                }

                if new_y.abs() < SNAP_THRESHOLD {
                    new_y = 0;
                    self.active_snap_lines.push(SnapLine::Horizontal(0.0));
                } else if (new_y - 5000).abs() < SNAP_THRESHOLD {
                    new_y = 5000;
                    self.active_snap_lines.push(SnapLine::Horizontal(sh * 0.5));
                }

                // Apply new position
                if let Some(selected) = &self.selected {
                    let layer = match self.layout.layers.get_mut(self.active_layer_index) {
                        Some(l) => l,
                        None => return,
                    };
                    match selected {
                        SelectedWidget::Button(uuid) => {
                            if let Some(btn) = layer.normal_buttons.iter_mut().find(|b| &b.uuid == uuid) {
                                btn.position = ButtonPosition::new(new_x, new_y);
                                self.is_dirty = true;
                            }
                        }
                        SelectedWidget::Joystick(uuid) => {
                            if let Some(joy) = layer.joystick_buttons.iter_mut().find(|j| &j.uuid == uuid) {
                                joy.position = ButtonPosition::new(new_x, new_y);
                                self.is_dirty = true;
                            }
                        }
                        SelectedWidget::TextBox(_) => {}
                    }
                }
            }
            EditorDrag::Resize {
                pointer_id: pid,
                handle,
                start_pixel,
                orig_position,
                orig_width_pct,
                orig_height_pct,
            } => {
                if pid != pointer_id {
                    return;
                }

                let dx_px = pos.0 - start_pixel.0;
                let dy_px = pos.1 - start_pixel.1;

                let dx_norm = ((dx_px / sw) * 10000.0).round() as i32;
                let dy_norm = ((dy_px / sh) * 10000.0).round() as i32;

                if let Some(SelectedWidget::Button(uuid)) = &self.selected {
                    let layer = match self.layout.layers.get_mut(self.active_layer_index) {
                        Some(l) => l,
                        None => return,
                    };
                    if let Some(btn) = layer.normal_buttons.iter_mut().find(|b| &b.uuid == uuid) {
                        match handle {
                            ResizeHandle::BottomRight => {
                                let new_w = (orig_width_pct + dx_norm).clamp(300, 10000);
                                let new_h = (orig_height_pct + dy_norm).clamp(300, 10000);
                                btn.button_size.width_percentage = new_w;
                                btn.button_size.height_percentage = new_h;
                                self.is_dirty = true;
                            }
                            ResizeHandle::TopLeft => {
                                let new_w = (orig_width_pct - dx_norm).clamp(300, 10000);
                                let new_h = (orig_height_pct - dy_norm).clamp(300, 10000);
                                let new_x = (orig_position.x + dx_norm).clamp(0, 10000);
                                let new_y = (orig_position.y + dy_norm).clamp(0, 10000);
                                btn.position = ButtonPosition::new(new_x, new_y);
                                btn.button_size.width_percentage = new_w;
                                btn.button_size.height_percentage = new_h;
                                self.is_dirty = true;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Finishes the active drag gesture.
    pub fn end_drag(&mut self, pointer_id: u64) {
        if let Some(drag) = &self.drag {
            let pid = match drag {
                EditorDrag::Move { pointer_id, .. } => *pointer_id,
                EditorDrag::Resize { pointer_id, .. } => *pointer_id,
            };
            if pid == pointer_id {
                self.drag = None;
                self.active_snap_lines.clear();
            }
        }
    }

    /// Adds a new normal button to the currently active layer.
    pub fn add_normal_button(&mut self, text: &str) -> String {
        let uuid = generate_uuid();
        let mut btn = NormalButton::new(text, 5000, 5000);
        btn.uuid = uuid.clone();
        btn.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 1200,
            height_percentage: 1000,
            ..Default::default()
        };

        if self.layout.layers.is_empty() {
            self.layout.layers.push(ControlLayer::new("Main"));
        }
        let layer_idx = self.active_layer_index.min(self.layout.layers.len() - 1);
        self.layout.layers[layer_idx].normal_buttons.push(btn);
        self.selected = Some(SelectedWidget::Button(uuid.clone()));
        self.is_dirty = true;
        uuid
    }

    /// Adds a new movement joystick to the currently active layer.
    pub fn add_joystick(&mut self) -> String {
        let uuid = generate_uuid();
        let mut joy = JoystickButton::default();
        joy.uuid = uuid.clone();
        joy.position = ButtonPosition::new(1000, 6000);

        if self.layout.layers.is_empty() {
            self.layout.layers.push(ControlLayer::new("Main"));
        }
        let layer_idx = self.active_layer_index.min(self.layout.layers.len() - 1);
        self.layout.layers[layer_idx].joystick_buttons.push(joy);
        self.selected = Some(SelectedWidget::Joystick(uuid.clone()));
        self.is_dirty = true;
        uuid
    }

    /// Duplicates the currently selected widget with a slight offset (+200 coords).
    pub fn duplicate_selected(&mut self) -> Option<String> {
        let selected = self.selected.clone()?;
        let layer = self.layout.layers.get_mut(self.active_layer_index)?;

        match selected {
            SelectedWidget::Button(uuid) => {
                let btn = layer.normal_buttons.iter().find(|b| b.uuid == uuid)?.clone();
                let mut dup = btn;
                dup.uuid = generate_uuid();
                dup.position.x = (dup.position.x + 200).min(9800);
                dup.position.y = (dup.position.y + 200).min(9800);
                let new_uuid = dup.uuid.clone();
                layer.normal_buttons.push(dup);
                self.selected = Some(SelectedWidget::Button(new_uuid.clone()));
                self.is_dirty = true;
                Some(new_uuid)
            }
            SelectedWidget::Joystick(uuid) => {
                let joy = layer.joystick_buttons.iter().find(|j| j.uuid == uuid)?.clone();
                let mut dup = joy;
                dup.uuid = generate_uuid();
                dup.position.x = (dup.position.x + 200).min(9800);
                dup.position.y = (dup.position.y + 200).min(9800);
                let new_uuid = dup.uuid.clone();
                layer.joystick_buttons.push(dup);
                self.selected = Some(SelectedWidget::Joystick(new_uuid.clone()));
                self.is_dirty = true;
                Some(new_uuid)
            }
            SelectedWidget::TextBox(_) => None,
        }
    }

    /// Deletes the currently selected widget from the active layer.
    pub fn delete_selected(&mut self) -> bool {
        let selected = match &self.selected {
            Some(s) => s.clone(),
            None => return false,
        };
        let layer = match self.layout.layers.get_mut(self.active_layer_index) {
            Some(l) => l,
            None => return false,
        };

        let removed = match selected {
            SelectedWidget::Button(uuid) => {
                let before = layer.normal_buttons.len();
                layer.normal_buttons.retain(|b| b.uuid != uuid);
                layer.normal_buttons.len() < before
            }
            SelectedWidget::Joystick(uuid) => {
                let before = layer.joystick_buttons.len();
                layer.joystick_buttons.retain(|j| j.uuid != uuid);
                layer.joystick_buttons.len() < before
            }
            SelectedWidget::TextBox(uuid) => {
                let before = layer.text_boxes.len();
                layer.text_boxes.retain(|t| t.uuid != uuid);
                layer.text_boxes.len() < before
            }
        };

        if removed {
            self.selected = None;
            self.is_dirty = true;
        }
        removed
    }

    /// Adds a new layer to the layout.
    pub fn add_layer(&mut self, name: impl Into<String>) {
        let layer = ControlLayer::new(name);
        self.layout.layers.push(layer);
        self.active_layer_index = self.layout.layers.len() - 1;
        self.is_dirty = true;
    }

    /// Toggles the visibility/hide flag of a layer.
    pub fn toggle_layer_visibility(&mut self, layer_index: usize) {
        if let Some(layer) = self.layout.layers.get_mut(layer_index) {
            layer.hide = !layer.hide;
            self.is_dirty = true;
        }
    }

    /// Saves the current layout to disk via LayoutStorage.
    pub fn save(&mut self, storage: &LayoutStorage) -> Result<(), String> {
        storage.save_layout(&self.original_slug, &self.layout)?;
        self.is_dirty = false;
        Ok(())
    }
}
