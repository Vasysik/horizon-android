//! Customizable on-screen touch control layout engine.
//!
//! Provides clean-room data models, storage manager, runtime touch dispatch,
//! and input event translation for mobile touch gameplay.

pub mod editor;
pub mod model;
pub mod runtime;
pub mod storage;

pub use editor::*;
pub use model::*;
pub use runtime::*;
pub use storage::*;

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event::TouchPhase;
    use winit::keyboard::KeyCode;

    #[test]
    fn test_model_json_serialization_roundtrip() {
        let layout = LayoutStorage::default_layout();
        let json = serde_json::to_string(&layout).expect("serialize default layout");
        assert!(json.contains("\"editorVersion\":12"));
        assert!(json.contains("\"info\""));
        assert!(json.contains("\"layers\""));

        let deserialized: ControlLayout = serde_json::from_str(&json).expect("deserialize default layout");
        assert_eq!(deserialized.editor_version, 12);
        assert_eq!(deserialized.layers.len(), layout.layers.len());
        assert_eq!(deserialized.info.name.default, "Default");
    }

    #[test]
    fn test_duplicate_generates_distinct_uuids() {
        let original = LayoutStorage::default_layout();
        let duplicate = original.duplicate("Copy");

        assert_eq!(duplicate.info.name.default, "Copy");
        assert_ne!(original.layers[0].uuid, duplicate.layers[0].uuid);
        assert_ne!(
            original.layers[0].normal_buttons[0].uuid,
            duplicate.layers[0].normal_buttons[0].uuid
        );
        assert_ne!(original.styles[0].uuid, duplicate.styles[0].uuid);
    }

    #[test]
    fn test_storage_lifecycle_in_temp_dir() {
        let temp_dir = std::env::temp_dir().join(format!("horizon_test_layout_{}", generate_uuid()));
        let storage = LayoutStorage::new(&temp_dir, "default");

        let default_name = storage.ensure_default_layout().expect("ensure default");
        assert_eq!(default_name, "default");

        let list = storage.list_layouts("en");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_stem, "default");
        assert!(list[0].is_active);

        // Create new layout
        let new_slug = storage.create_layout("PVP Pro", "Tester", "1.0").expect("create layout");
        assert_eq!(new_slug, "PVP_Pro");
        let list2 = storage.list_layouts("en");
        assert_eq!(list2.len(), 2);

        // Duplicate
        let copy_slug = storage.duplicate_layout("PVP_Pro", "PVP Pro Copy").expect("duplicate");
        assert_eq!(copy_slug, "PVP_Pro_Copy");

        // Delete
        storage.delete_layout("PVP_Pro_Copy").expect("delete layout");
        let list3 = storage.list_layouts("en");
        assert_eq!(list3.len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_runtime_button_press_and_release() {
        let mut layout = ControlLayout::default();
        let mut layer = ControlLayer::new("Main");

        let mut btn = NormalButton::new("JUMP", 5000, 5000);
        btn.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 2000,
            height_percentage: 2000,
            ..Default::default()
        };
        btn.click_events.push(ClickEvent::key("GLFW_KEY_SPACE"));
        layer.normal_buttons.push(btn);
        layout.layers.push(layer);

        let mut runtime = LayoutRuntime::new(layout);
        runtime.set_screen_dimensions(1000.0, 1000.0, 1.0);

        // Button is located at 500..700 x and 500..700 y
        let hit_pos = (550.0, 550.0);
        let miss_pos = (100.0, 100.0);

        // Miss touch
        let evs = runtime.handle_touch(1, TouchPhase::Started, miss_pos);
        assert!(evs.is_empty());

        // Hit touch down
        let evs = runtime.handle_touch(1, TouchPhase::Started, hit_pos);
        assert_eq!(evs, vec![LayoutOutputEvent::KeyDown(KeyCode::Space)]);

        // Touch up
        let evs = runtime.handle_touch(1, TouchPhase::Ended, hit_pos);
        assert_eq!(evs, vec![LayoutOutputEvent::KeyUp(KeyCode::Space)]);
    }

    #[test]
    fn test_runtime_toggle_button() {
        let mut layout = ControlLayout::default();
        let mut layer = ControlLayer::new("Main");

        let mut btn = NormalButton::new("SNEAK", 1000, 1000);
        btn.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 2000,
            height_percentage: 2000,
            ..Default::default()
        };
        btn.is_toggleable = true;
        btn.click_events.push(ClickEvent::key("GLFW_KEY_LEFT_SHIFT"));
        layer.normal_buttons.push(btn);
        layout.layers.push(layer);

        let mut runtime = LayoutRuntime::new(layout);
        runtime.set_screen_dimensions(1000.0, 1000.0, 1.0);

        let hit_pos = (150.0, 150.0);

        // Tap 1 down -> KeyDown
        let evs = runtime.handle_touch(1, TouchPhase::Started, hit_pos);
        assert_eq!(evs, vec![LayoutOutputEvent::KeyDown(KeyCode::ShiftLeft)]);

        // Tap 1 up -> remains down (no KeyUp)
        let evs = runtime.handle_touch(1, TouchPhase::Ended, hit_pos);
        assert!(evs.is_empty());

        // Tap 2 down -> KeyUp (toggled off)
        let evs = runtime.handle_touch(2, TouchPhase::Started, hit_pos);
        assert_eq!(evs, vec![LayoutOutputEvent::KeyUp(KeyCode::ShiftLeft)]);
    }

    #[test]
    fn test_runtime_anti_stuck_release_on_layer_hidden() {
        let mut layout = ControlLayout::default();
        let mut layer = ControlLayer::new("Combat");
        layer.visibility_type = VisibilityType::InGame;

        let mut btn = NormalButton::new("ATTACK", 2000, 2000);
        btn.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 2000,
            height_percentage: 2000,
            ..Default::default()
        };
        btn.click_events.push(ClickEvent::key("GLFW_KEY_F"));
        layer.normal_buttons.push(btn);
        layout.layers.push(layer);

        let mut runtime = LayoutRuntime::new(layout);
        runtime.set_screen_dimensions(1000.0, 1000.0, 1.0);

        // Player presses button in game
        let evs = runtime.handle_touch(1, TouchPhase::Started, (250.0, 250.0));
        assert_eq!(evs, vec![LayoutOutputEvent::KeyDown(KeyCode::KeyF)]);

        // Player opens menu -> InGame layer is hidden -> KeyF must be immediately released!
        let evs = runtime.set_in_menu(true);
        assert_eq!(evs, vec![LayoutOutputEvent::KeyUp(KeyCode::KeyF)]);
    }

    #[test]
    fn test_runtime_multi_touch_independence() {
        let mut layout = ControlLayout::default();
        let mut layer = ControlLayer::new("Main");

        let mut btn_a = NormalButton::new("A", 1000, 1000);
        btn_a.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 1000,
            height_percentage: 1000,
            ..Default::default()
        };
        btn_a.click_events.push(ClickEvent::key("GLFW_KEY_A"));

        let mut btn_b = NormalButton::new("B", 8000, 8000);
        btn_b.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_percentage: 1000,
            height_percentage: 1000,
            ..Default::default()
        };
        btn_b.click_events.push(ClickEvent::key("GLFW_KEY_B"));

        layer.normal_buttons.push(btn_a);
        layer.normal_buttons.push(btn_b);
        layout.layers.push(layer);

        let mut runtime = LayoutRuntime::new(layout);
        runtime.set_screen_dimensions(1000.0, 1000.0, 1.0);

        // Finger 1 taps button A
        let ev1 = runtime.handle_touch(10, TouchPhase::Started, (150.0, 150.0));
        assert_eq!(ev1, vec![LayoutOutputEvent::KeyDown(KeyCode::KeyA)]);

        // Finger 2 taps button B while finger 1 is still down
        let ev2 = runtime.handle_touch(20, TouchPhase::Started, (850.0, 850.0));
        assert_eq!(ev2, vec![LayoutOutputEvent::KeyDown(KeyCode::KeyB)]);

        // Finger 1 releases -> releases A, B stays held
        let ev3 = runtime.handle_touch(10, TouchPhase::Ended, (150.0, 150.0));
        assert_eq!(ev3, vec![LayoutOutputEvent::KeyUp(KeyCode::KeyA)]);

        // Finger 2 releases -> releases B
        let ev4 = runtime.handle_touch(20, TouchPhase::Ended, (850.0, 850.0));
        assert_eq!(ev4, vec![LayoutOutputEvent::KeyUp(KeyCode::KeyB)]);
    }

    #[test]
    fn test_runtime_joystick_direction_and_sprint_lock() {
        let mut layout = ControlLayout::default();
        let mut layer = ControlLayer::new("Move");

        let mut joy = JoystickButton::default();
        joy.position = ButtonPosition::new(0, 0);
        joy.size_type = SizeType::Percentage;
        joy.size_percentage = 4000; // Radius = 200 on 1000x1000 screen
        joy.dead_zone_ratio = 0.2;
        joy.lock_threshold = 0.2;
        joy.can_lock = true;

        layer.joystick_buttons.push(joy);
        layout.layers.push(layer);

        let mut runtime = LayoutRuntime::new(layout);
        runtime.set_screen_dimensions(1000.0, 1000.0, 1.0);

        // Center is at (200, 200).
        // Slight drag inside deadzone (< 40px)
        let evs = runtime.handle_touch(1, TouchPhase::Started, (200.0, 190.0));
        assert!(evs.is_empty());

        // Drag forward (dy = -100px) -> outside deadzone -> emits KeyW
        let evs = runtime.handle_touch(1, TouchPhase::Moved, (200.0, 100.0));
        assert_eq!(evs, vec![LayoutOutputEvent::KeyDown(KeyCode::KeyW)]);

        // Drag far forward beyond lock threshold (> 240px from center, e.g. y = -60) -> locks sprint!
        let evs = runtime.handle_touch(1, TouchPhase::Moved, (200.0, -60.0));
        assert_eq!(evs, vec![LayoutOutputEvent::KeyDown(KeyCode::ControlLeft)]);

        // Release pointer -> since it's locked, it stays active until tapped again or released
        let evs = runtime.handle_touch(1, TouchPhase::Ended, (200.0, -60.0));
        assert!(evs.is_empty());

        // Clear all inputs
        let clear_evs = runtime.release_all_inputs();
        assert!(clear_evs.contains(&LayoutOutputEvent::KeyUp(KeyCode::KeyW)));
        assert!(clear_evs.contains(&LayoutOutputEvent::KeyUp(KeyCode::ControlLeft)));
    }

    #[test]
    fn test_editor_selection_drag_snap_and_resize() {
        let layout = LayoutStorage::default_layout();
        let mut editor = LayoutEditor::new(layout, "default");
        editor.set_dimensions(1000.0, 1000.0, 1.0);

        // Add a button at (5000, 5000)
        let btn_uuid = editor.add_normal_button("TEST");
        assert_eq!(editor.selected, Some(SelectedWidget::Button(btn_uuid.clone())));

        // Select at button center (560px, 550px)
        assert!(editor.select_at((560.0, 550.0)));
        assert_eq!(editor.selected, Some(SelectedWidget::Button(btn_uuid.clone())));

        // Drag move near left edge -> snaps to x = 0
        assert!(editor.start_drag(1, (560.0, 550.0)));
        // Move -520px horizontally (towards left)
        editor.update_drag(1, (40.0, 550.0));
        assert!(editor.active_snap_lines.contains(&SnapLine::Vertical(0.0)));
        editor.end_drag(1);

        // Verify button position snapped to 0
        let layer = &editor.layout.layers[editor.active_layer_index];
        let btn = layer.normal_buttons.iter().find(|b| b.uuid == btn_uuid).unwrap();
        assert_eq!(btn.position.x, 0);

        // Test duplicate
        let dup_uuid = editor.duplicate_selected().expect("duplicate button");
        assert_ne!(dup_uuid, btn_uuid);
        assert_eq!(editor.layout.layers[editor.active_layer_index].normal_buttons.len(), 3);

        // Test delete
        assert!(editor.delete_selected());
        assert_eq!(editor.layout.layers[editor.active_layer_index].normal_buttons.len(), 2);
        assert_eq!(editor.selected, None);
    }
}
