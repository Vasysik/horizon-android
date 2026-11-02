//! Layout file storage, discovery, import/export, and defaults.

use super::model::*;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSummary {
    pub file_stem: String,
    pub display_name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub is_active: bool,
}

pub struct LayoutStorage {
    directory: PathBuf,
    active_layout: String,
}

impl LayoutStorage {
    pub fn new(directory: impl Into<PathBuf>, active_layout: impl Into<String>) -> Self {
        let dir = directory.into();
        let _ = fs::create_dir_all(&dir);
        Self {
            directory: dir,
            active_layout: active_layout.into(),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn active_layout(&self) -> &str {
        &self.active_layout
    }

    pub fn set_active_layout(&mut self, name: impl Into<String>) {
        self.active_layout = name.into();
    }

    /// Lists all valid `.json` layouts found in the storage directory.
    pub fn list_layouts(&self, lang_tag: &str) -> Vec<LayoutSummary> {
        let mut result = Vec::new();
        let entries = match fs::read_dir(&self.directory) {
            Ok(e) => e,
            Err(_) => return result,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(layout) = serde_json::from_str::<ControlLayout>(&content) {
                            let is_active = stem == self.active_layout;
                            result.push(LayoutSummary {
                                file_stem: stem.to_string(),
                                display_name: layout.info.name.resolve(lang_tag).to_string(),
                                author: layout.info.author.resolve(lang_tag).to_string(),
                                version: layout.info.version_name.clone(),
                                description: layout.info.description.resolve(lang_tag).to_string(),
                                is_active,
                            });
                        }
                    }
                }
            }
        }
        result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        result
    }

    /// Loads a layout by file stem or file name.
    pub fn load_layout(&self, name: &str) -> Result<ControlLayout, String> {
        let clean_name = name.strip_suffix(".json").unwrap_or(name);
        let file_path = self.directory.join(format!("{}.json", clean_name));
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read layout {}: {}", file_path.display(), e))?;
        serde_json::from_str::<ControlLayout>(&content)
            .map_err(|e| format!("Failed to parse layout {}: {}", file_path.display(), e))
    }

    /// Saves a layout to disk.
    pub fn save_layout(&self, name: &str, layout: &ControlLayout) -> Result<(), String> {
        let clean_name = name.strip_suffix(".json").unwrap_or(name);
        let file_path = self.directory.join(format!("{}.json", clean_name));
        let json = serde_json::to_string_pretty(layout)
            .map_err(|e| format!("Failed to serialize layout: {}", e))?;
        fs::write(&file_path, json)
            .map_err(|e| format!("Failed to write layout {}: {}", file_path.display(), e))
    }

    /// Creates a new layout file with the given name, author, and version.
    pub fn create_layout(&self, name: &str, author: &str, version: &str) -> Result<String, String> {
        let slug = sanitize_filename(name);
        let mut layout = Self::default_layout();
        layout.info.name = TranslatableText::single(name);
        layout.info.author = TranslatableText::single(author);
        layout.info.version_name = version.to_string();
        self.save_layout(&slug, &layout)?;
        Ok(slug)
    }

    /// Duplicates an existing layout with a new name and regenerated UUIDs.
    pub fn duplicate_layout(&self, source_name: &str, new_name: &str) -> Result<String, String> {
        let source = self.load_layout(source_name)?;
        let duplicated = source.duplicate(new_name);
        let slug = sanitize_filename(new_name);
        self.save_layout(&slug, &duplicated)?;
        Ok(slug)
    }

    /// Deletes a layout file by name.
    pub fn delete_layout(&self, name: &str) -> Result<(), String> {
        let clean_name = name.strip_suffix(".json").unwrap_or(name);
        let file_path = self.directory.join(format!("{}.json", clean_name));
        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to delete layout: {}", e))?;
        }
        Ok(())
    }

    /// Imports a layout from a JSON string.
    pub fn import_layout(&self, json_content: &str, fallback_name: &str) -> Result<String, String> {
        let layout: ControlLayout = serde_json::from_str(json_content)
            .map_err(|e| format!("Invalid control layout JSON: {}", e))?;
        let name = if !layout.info.name.default.is_empty() {
            &layout.info.name.default
        } else {
            fallback_name
        };
        let slug = sanitize_filename(name);
        self.save_layout(&slug, &layout)?;
        Ok(slug)
    }

    /// Exports a layout as formatted JSON string.
    pub fn export_layout(&self, name: &str) -> Result<String, String> {
        let layout = self.load_layout(name)?;
        serde_json::to_string_pretty(&layout)
            .map_err(|e| format!("Failed to export layout JSON: {}", e))
    }

    /// Ensures at least one default layout exists in the directory.
    pub fn ensure_default_layout(&self) -> Result<String, String> {
        let default_name = "default";
        let path = self.directory.join("default.json");
        if !path.exists() {
            let layout = Self::default_layout();
            self.save_layout(default_name, &layout)?;
        }
        Ok(default_name.to_string())
    }

    /// Returns a full-featured default layout conforming to the 5-layer standard specification.
    pub fn default_layout() -> ControlLayout {
        let mut layout = ControlLayout {
            editor_version: CURRENT_EDITOR_VERSION,
            info: LayoutInfo {
                name: TranslatableText {
                    default: "Default".to_string(),
                    match_queue: vec![
                        LocaleMatch {
                            language_tag: "id".to_string(),
                            value: "Bawaan".to_string(),
                        },
                        LocaleMatch {
                            language_tag: "zh".to_string(),
                            value: "默认".to_string(),
                        },
                    ],
                },
                author: TranslatableText::single("Horizon"),
                description: TranslatableText::single("Default mobile touch layout with movement joystick and essential keys."),
                version_code: 1,
                version_name: "1.0".to_string(),
            },
            layers: Vec::new(),
            styles: Vec::new(),
            joystick_styles: Vec::new(),
        };

        // Shared default button style
        let button_style_id = generate_uuid();
        layout.styles.push(ButtonStyle {
            name: "Standard".to_string(),
            uuid: button_style_id.clone(),
            animate_swap: false,
            common_style: true,
            light_style: ButtonStyleConfig {
                alpha: 0.8,
                pressed_alpha: 1.0,
                background_color: 0x80222222,
                pressed_background_color: 0xB3444444,
                content_color: 0xFFFFFFFF,
                pressed_content_color: 0xFFFFFFFF,
                font_size: Some(13),
                border_width: 1,
                border_color: 0x40FFFFFF,
                border_radius: ButtonShape {
                    top_start: 20.0,
                    top_end: 20.0,
                    bottom_end: 20.0,
                    bottom_start: 20.0,
                },
            },
            dark_style: ButtonStyleConfig::default(),
        });

        // Layer 0: GUI / System (Always visible)
        let mut layer_gui = ControlLayer::new("GUI");
        let mut btn_gui = NormalButton::new("GUI", 9300, 100);
        btn_gui.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_dp: 50.0,
            height_dp: 35.0,
            width_percentage: 1000,
            height_percentage: 700,
            width_reference: SizeReference::ScreenHeight,
            height_reference: SizeReference::ScreenHeight,
        };
        btn_gui.button_style = Some(button_style_id.clone());
        btn_gui.click_events.push(ClickEvent::launcher("LAUNCHER_EVENT_SWITCH_MENU"));
        layer_gui.normal_buttons.push(btn_gui);

        // Layer 1: Tab / Auxiliary tools (Always visible)
        let mut layer_tab = ControlLayer::new("Tab");
        let aux_buttons = [
            ("F3", 100, 100, "GLFW_KEY_F3"),
            ("F5", 1000, 100, "GLFW_KEY_F5"),
            ("TAB", 1900, 100, "GLFW_KEY_TAB"),
            ("CHAT", 2800, 100, "GLFW_KEY_T"),
            ("KEYBOARD", 3700, 100, "LAUNCHER_EVENT_SWITCH_IME"),
        ];
        for (label, x, y, key) in aux_buttons {
            let mut btn = NormalButton::new(label, x, y);
            btn.button_size = ButtonSize {
                size_type: SizeType::Percentage,
                width_dp: 45.0,
                height_dp: 35.0,
                width_percentage: 800,
                height_percentage: 650,
                width_reference: SizeReference::ScreenHeight,
                height_reference: SizeReference::ScreenHeight,
            };
            btn.button_style = Some(button_style_id.clone());
            if key.starts_with("LAUNCHER_EVENT_") {
                btn.click_events.push(ClickEvent::launcher(key));
            } else {
                btn.click_events.push(ClickEvent::key(key));
            }
            layer_tab.normal_buttons.push(btn);
        }

        // Layer 2: Action Controls (In Game)
        let mut layer_actions = ControlLayer::new("Actions");
        layer_actions.visibility_type = VisibilityType::InGame;

        let mut btn_jump = NormalButton::new("JUMP", 8800, 7500);
        btn_jump.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_dp: 60.0,
            height_dp: 60.0,
            width_percentage: 1200,
            height_percentage: 1200,
            width_reference: SizeReference::ScreenHeight,
            height_reference: SizeReference::ScreenHeight,
        };
        btn_jump.button_style = Some(button_style_id.clone());
        btn_jump.click_events.push(ClickEvent::key("GLFW_KEY_SPACE"));
        layer_actions.normal_buttons.push(btn_jump);

        let mut btn_sneak = NormalButton::new("SNEAK", 8800, 8900);
        btn_sneak.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_dp: 50.0,
            height_dp: 50.0,
            width_percentage: 1000,
            height_percentage: 1000,
            width_reference: SizeReference::ScreenHeight,
            height_reference: SizeReference::ScreenHeight,
        };
        btn_sneak.button_style = Some(button_style_id.clone());
        btn_sneak.is_toggleable = true;
        btn_sneak.click_events.push(ClickEvent::key("GLFW_KEY_LEFT_SHIFT"));
        layer_actions.normal_buttons.push(btn_sneak);

        // Layer 3: Inventory / Secondary (Always visible)
        let mut layer_inv = ControlLayer::new("Inventory");
        let mut btn_inv = NormalButton::new("INV", 8800, 100);
        btn_inv.button_size = ButtonSize {
            size_type: SizeType::Percentage,
            width_dp: 50.0,
            height_dp: 40.0,
            width_percentage: 900,
            height_percentage: 800,
            width_reference: SizeReference::ScreenHeight,
            height_reference: SizeReference::ScreenHeight,
        };
        btn_inv.button_style = Some(button_style_id.clone());
        btn_inv.click_events.push(ClickEvent::key("GLFW_KEY_E"));
        layer_inv.normal_buttons.push(btn_inv);

        // Layer 4: Movement Joystick (In Game)
        let mut layer_move = ControlLayer::new("Movement");
        layer_move.visibility_type = VisibilityType::InGame;

        let mut joystick = JoystickButton::default();
        joystick.position = ButtonPosition::new(1200, 7800);
        joystick.size_percentage = 2600;
        joystick.can_lock = true;
        layer_move.joystick_buttons.push(joystick);

        layout.layers.push(layer_gui);
        layout.layers.push(layer_tab);
        layout.layers.push(layer_actions);
        layout.layers.push(layer_inv);
        layout.layers.push(layer_move);

        layout
    }
}

fn sanitize_filename(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if clean.is_empty() {
        "layout".to_string()
    } else {
        clean
    }
}

// ponytail: single-folder layout storage is enough for now. Add cloud/sync when requested.
