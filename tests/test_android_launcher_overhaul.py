import math
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def strip_rust_comments_and_literals(source: str) -> str:
    """Single-pass lexer that strips Rust line comments, block comments, char literals,
    raw string literals, and standard string literals (including '\\\n' line continuations)."""
    token_re = re.compile(
        r"//[^\n]*|"
        r"/\*[\s\S]*?\*/|"
        r"(?<![a-zA-Z0-9_'])b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|[\s\S])|[^'\\\n])'|"
        r'(?<![a-zA-Z0-9_])b?r(#*)"[\s\S]*?"\1|'
        r'b?"(?:\\[\s\S]|[^"\\])*"'
    )
    return token_re.sub(" ", source)


class LauncherOverhaulTests(unittest.TestCase):
    def test_rust_delimiter_and_syntax_sanity_across_modified_files(self) -> None:
        modified_files = [
            "crates/horizon-client-render/src/touch/layout.rs",
            "crates/horizon-client-render/src/touch.rs",
            "crates/horizon-client-render/src/options_ui.rs",
            "crates/horizon-client/src/touch_ui.rs",
            "crates/horizon-client/src/ui_theme.rs",
            "crates/horizon-client/src/game/android_input.rs",
            "crates/horizon-client/src/game/startup_menu.rs",
            "crates/horizon-client/src/game/options.rs",
            "crates/horizon-client/src/game/pad.rs",
            "crates/horizon-client/src/game/pad_menu.rs",
            "crates/horizon-client/src/game.rs",
            "crates/horizon-client/src/android/catalog.rs",
            "crates/horizon-client/src/android/wardrobe.rs",
            "crates/horizon-client/src/android/launcher_pages.rs",
            "crates/horizon-client/src/android/launcher.rs",
            "crates/horizon-client/src/android/chooser.rs",
            "crates/horizon-client/src/android/login.rs",
            "crates/horizon-client/src/android/native_ime.rs",
            "crates/horizon-client/src/android/display.rs",
            "crates/horizon-client-render/src/touch/editor.rs",
            "crates/horizon-client-render/src/gfx.rs",
            "crates/horizon-client-render/src/audio.rs",
            "crates/horizon-client-render/src/mesher.rs",
            "crates/horizon-client-render/src/mesher/sides.rs",
            "crates/horizon-client-render/src/world.rs",
            "crates/horizon-world/src/level/mod.rs",
            "crates/horizon-world/src/level/update.rs",
            "crates/horizon-world/src/level/tick.rs",
            "crates/horizon-client-render/src/touch/custom.rs",
            "crates/horizon-client-render/src/menu_blur.rs",
            "crates/horizon-client-render/src/ui.rs",
            "crates/horizon-client/src/android/document_store.rs",
            "crates/horizon-client/src/android/documents.rs",
            "crates/horizon-client/src/android/platform.rs",
            "crates/horizon-client/src/game/perspective.rs",
            "crates/horizon-client/src/game/recipe_book.rs",
            "crates/horizon-client/src/game/screens.rs",
            "crates/horizon-client/src/item_render.rs",
            "crates/horizon-client/src/item_render/gui.rs",
            "third_party/winit/src/platform_impl/android/keycodes.rs",
            "third_party/winit/src/platform_impl/android/mod.rs",
            "third_party/winit/src/platform_impl/android/mouse_state.rs",
            "launcher/src/skins/preview.rs",
            "crates/horizon-client-render/src/gfx/pack_post.rs",
            "crates/horizon-client-render/src/gfx/pack_atlas.rs",
            "crates/horizon-client-render/src/gfx/pack_uniforms.rs",
            "crates/horizon-client-render/src/gfx/arena.rs",
            "crates/horizon-client-render/src/graphics.rs",
            "crates/horizon-client/src/game/shader_packs.rs",
            "crates/horizon-shaders/src/custom.rs",
            "crates/horizon-shaders/src/translate/scan.rs",
            "crates/horizon-shaders/src/translate/rewrite.rs",
            "crates/horizon-client-render/src/gfx/pack_sky.rs",
            "crates/horizon-client-render/src/gfx/pack_shadow.rs",
            "crates/horizon-client-render/src/gfx/pack_images.rs",
            "crates/horizon-client-render/src/gfx/flat.rs",
            "crates/horizon-client/src/game/fov.rs",
        ]
        for rel in modified_files:
            path = ROOT / rel
            self.assertTrue(path.is_file(), f"Missing file: {rel}")
            text = path.read_text(encoding="utf-8")
            cleaned = strip_rust_comments_and_literals(text)
            self.assertEqual(
                cleaned.count("{"),
                cleaned.count("}"),
                f"Unbalanced braces in {rel}",
            )
            self.assertEqual(
                cleaned.count("("),
                cleaned.count(")"),
                f"Unbalanced parens in {rel}",
            )
            self.assertEqual(
                cleaned.count("["),
                cleaned.count("]"),
                f"Unbalanced brackets in {rel}",
            )

    def test_virtual_mouse_windows11_pointer_and_authoritative_toggle(self) -> None:
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(encoding="utf-8")
        touch = (ROOT / "crates/horizon-client-render/src/touch.rs").read_text(encoding="utf-8")
        options_ui = (ROOT / "crates/horizon-client-render/src/options_ui.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")

        # 1. Faithful Windows 11 standard arrow pointer (aero_arrow) with exact (0,0) tip hotspot
        self.assertIn("pub fn draw_pointer(", touch_ui)
        self.assertIn("[pt(0.0, 0.0), pt(4.1, 11.4), pt(0.0, 14.8), pt(0.0, 0.0)]", touch_ui)
        self.assertIn("pub fn sync_viewport_cursor(", touch_ui)
        self.assertIn("pub fn compute_virtual_mouse_visibility(", touch_ui)

        # 2. Authoritative persisted virtual_mouse_enabled in touch::Settings
        self.assertIn("pub virtual_mouse_enabled: bool,", layout)
        self.assertIn("pub fn mcpe_controls_visible(", touch)

        # 3. Duplicate Virtual Mouse settings removed from Controls settings (both MCPE & GoyControl)
        page_touch_block = options_ui.split("Page::Touch => {")[1].split("Page::Controller =>")[0]
        self.assertNotIn("Opt::TouchMouseSensitivity", page_touch_block)
        controls_cat_block = launcher_pages.split("SettingsCategory::Controls => {")[1].split(
            "SettingsCategory::Experimental => {"
        )[0]
        self.assertNotIn("Virtual Mouse Pointer Speed", controls_cat_block)

    def test_floating_menu_settings_wired_and_scrollable(self) -> None:
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        android_input = (
            ROOT / "crates/horizon-client/src/game/android_input.rs"
        ).read_text(encoding="utf-8")
        startup_menu = (
            ROOT / "crates/horizon-client/src/game/startup_menu.rs"
        ).read_text(encoding="utf-8")
        pad = (ROOT / "crates/horizon-client/src/game/pad.rs").read_text(encoding="utf-8")

        # Scrollable Floating Menu content pane & Virtual Mouse speed control
        self.assertIn("pub hub_scroll: f32,", touch_ui)
        self.assertIn("pub hub_max_scroll: Cell<f32>,", touch_ui)
        self.assertIn("SetVirtualMouseSpeed(f32),", touch_ui)
        self.assertIn("InGameMenuAction::StepVirtualMouseSpeed", touch_ui)
        self.assertIn("InGameMenuAction::SetVirtualMouseSpeed", android_input)
        self.assertIn("InGameMenuAction::SetVirtualMouseSpeed", startup_menu)

        # Haptic Feedback wired to pad.rs and android_touch_haptic
        self.assertIn("!self.pad.touch.settings.vibrate", pad)
        self.assertIn("fn android_touch_haptic(", android_input)

        # Auto-Jump wired to self.settings.auto_jump and player.auto_jump_enabled
        self.assertIn("player.auto_jump_enabled = next;", android_input)
        self.assertIn("self.settings.auto_jump = next;", startup_menu)

        # Toggle Sprint wired to both sprint_toggle and toggle_sprint
        self.assertIn("self.pad.touch.settings.sprint_toggle = next;", android_input)
        self.assertIn("self.menu.controls.settings.sprint_toggle = next;", startup_menu)

        # Split Touch wired to toggle_split_touch()
        self.assertIn("self.pad.touch.settings.toggle_split_touch();", android_input)
        self.assertIn("self.menu.controls.settings.toggle_split_touch();", startup_menu)

    def test_skin_3d_projection_and_drag_direction_math(self) -> None:
        preview = (ROOT / "launcher/src/skins/preview.rs").read_text(encoding="utf-8")
        wardrobe = (ROOT / "crates/horizon-client/src/android/wardrobe.rs").read_text(encoding="utf-8")
        self.assertIn("pub fn drag(&mut self, dx: f32, dy: f32)", preview)
        self.assertIn("self.rotate(-dx * 0.7, -dy * 0.5);", preview)
        self.assertIn("self.camera.drag(point[0] - old[0], point[1] - old[1]);", wardrobe)

        # Verify the exact projection math from Raster::project across front (yaw=0) and back (yaw=180)
        def project(pt, yaw_deg, pitch_deg, zoom=1.0, width=160.0, height=200.0):
            x, y, z = pt
            s, c = math.sin(math.radians(yaw_deg)), math.cos(math.radians(yaw_deg))
            x, z = x * c + z * s, -x * s + z * c
            s, c = math.sin(math.radians(pitch_deg)), math.cos(math.radians(pitch_deg))
            y, z = (y - 16.0) * c - z * s, (y - 16.0) * s + z * c
            scale = height / 39.0 * zoom
            perspective = 100.0 / (100.0 + z)
            return [width * 0.5 + x * scale * perspective, height * 0.5 - y * scale * perspective, z]

        # Front chest point [0, 18, -2] at yaw=0, pitch=0
        p0 = project([0.0, 18.0, -2.0], 0.0, 0.0)
        p_drag_right = project([0.0, 18.0, -2.0], -20.0 * 0.7, 0.0)
        p_drag_down = project([0.0, 18.0, -2.0], 0.0, -20.0 * 0.5)
        self.assertGreater(p_drag_right[0], p0[0])
        self.assertGreater(p_drag_down[1], p0[1])

        # Back cape point [0, 18, 3] at yaw=180, pitch=0
        b0 = project([0.0, 18.0, 3.0], 180.0, 0.0)
        b_drag_right = project([0.0, 18.0, 3.0], 180.0 - 20.0 * 0.7, 0.0)
        b_drag_down = project([0.0, 18.0, 3.0], 180.0, -20.0 * 0.5)
        self.assertGreater(b_drag_right[0], b0[0])
        self.assertGreater(b_drag_down[1], b0[1])

    def test_theme_customization_and_controls_animation_exclusion(self) -> None:
        ui_theme = (ROOT / "crates/horizon-client/src/ui_theme.rs").read_text(encoding="utf-8")
        catalog = (ROOT / "crates/horizon-client/src/android/catalog.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")

        self.assertIn("pub struct LauncherThemeConfig", ui_theme)
        self.assertIn("pub enum ThemePreset", ui_theme)
        self.assertIn("pub enum GradientDirection", ui_theme)
        self.assertIn("pub fn draw_theme_background(", ui_theme)
        self.assertIn("pub theme: horizon_client::ui_theme::LauncherThemeConfig", catalog)
        self.assertIn("SettingsCategory::Theme =>", launcher_pages)
        self.assertIn("Action::ResetTheme", launcher_pages)

        # Ensure draw_hud_editor in launcher_pages.rs renders directly without page/modal animation wrapper
        editor_branch = launcher_pages.split("if let Some(tc) = &mut self.control_editor {")[
            1
        ].split("return Ok(());")[0]
        self.assertIn("draw_hud_editor", editor_branch)
        self.assertNotIn("self.anim", editor_branch)



    def test_stability_6s_freeze_burning_village_and_leaves_fps_optimizations(self) -> None:
        gfx = (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8")
        audio = (ROOT / "crates/horizon-client-render/src/audio.rs").read_text(encoding="utf-8")
        mesher = (ROOT / "crates/horizon-client-render/src/mesher.rs").read_text(encoding="utf-8")
        sides = (ROOT / "crates/horizon-client-render/src/mesher/sides.rs").read_text(encoding="utf-8")
        terrain_wgsl = (ROOT / "crates/horizon-client-render/src/terrain.wgsl").read_text(
            encoding="utf-8"
        )
        tick = (ROOT / "crates/horizon-world/src/level/tick.rs").read_text(encoding="utf-8")
        world = (ROOT / "crates/horizon-client-render/src/world.rs").read_text(encoding="utf-8")
        game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
        native_ime = (ROOT / "crates/horizon-client/src/android/native_ime.rs").read_text(encoding="utf-8")

        # 1. In-place Loading::set_ui texture update & surface_extent tracking
        self.assertIn("pub fn set_loading_ui(", gfx)
        self.assertIn("surface_extent: std::sync::atomic::AtomicU64,", gfx)
        self.assertIn("pub fn with_registry(", audio)

        # 2. Invisible fire age updates do not mark 8 chunk sections dirty (#88)
        self.assertIn("pub fn set_with_visibility(", world)
        self.assertIn("260 | super::flags::UPDATE_KNOWN_SHAPE", tick)

        # 3. Smart/hollow leaf culling & early fragment discard (#156)
        self.assertIn('static GROUPED: AtomicBool = AtomicBool::new(cfg!(all(target_os = "android", not(test))));', sides)
        self.assertLess(
            terrain_wgsl.index("if texel.a * in.color.a < 0.5 && (in.layer & 0x8000u) == 0u { discard; }"),
            terrain_wgsl.index("var color = texel * in.color;"),
        )

        # 4. Immersive fullscreen constant decor layout & safe exit_app
        self.assertIn("immersive: AtomicBool::new(false),", native_ime)
        self.assertRegex(native_ime, r'call_method\(\s*&decor,\s*"getWidth"')
        self.assertIn("crate::game::exit_app(event_loop);", game)

    def test_custom_input_dialogue_typing_back_confirm_launching_and_slider(self) -> None:
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        editor = (ROOT / "crates/horizon-client-render/src/touch/editor.rs").read_text(encoding="utf-8")
        android_input = (
            ROOT / "crates/horizon-client/src/game/android_input.rs"
        ).read_text(encoding="utf-8")
        startup_menu = (
            ROOT / "crates/horizon-client/src/game/startup_menu.rs"
        ).read_text(encoding="utf-8")
        launcher = (ROOT / "crates/horizon-client/src/android/launcher.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")

        # Send Custom Input modal & action
        self.assertIn("SendCustomInput(String),", touch_ui)
        self.assertIn("fn draw_input_picker_modal(", touch_ui)
        self.assertIn("InGameMenuAction::SendCustomInput(token)", android_input)
        self.assertIn("InGameMenuAction::SendCustomInput(token)", startup_menu)

        # Unified dialogue typing across Launcher, StartupMenu, and In-Game
        self.assertIn("pub fn apply_editor_or_preset_output(", touch_ui)
        self.assertIn("apply_editor_or_preset_output(", android_input)
        self.assertIn("apply_editor_or_preset_output(", startup_menu)
        self.assertIn("apply_editor_or_preset_output(", launcher)

        # Control Editor back confirmation when unsaved changes exist
        self.assertIn("pub back_confirm_open: bool,", editor)
        self.assertIn("pub fn has_unsaved_changes(&self) -> bool", editor)
        self.assertIn("pub fn request_editor_back(&mut self) -> bool", editor)
        self.assertIn("Unsaved Layout Changes", touch_ui)

        # Main Launcher "Launching..." state & Virtual Mouse Speed slider
        self.assertIn("pub(super) launching: bool,", launcher)
        self.assertIn('"Launching..."', launcher_pages)
        self.assertIn("pub mouse_speed_dragging: bool,", touch_ui)


    def test_next_development_pass_controls_kbm_file_manager_dpi_intro_and_ui_perf(self) -> None:
        touch = (ROOT / "crates/horizon-client-render/src/touch.rs").read_text(encoding="utf-8")
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(encoding="utf-8")
        custom = (ROOT / "crates/horizon-client-render/src/touch/custom.rs").read_text(encoding="utf-8")
        editor = (ROOT / "crates/horizon-client-render/src/touch/editor.rs").read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        ui_theme = (ROOT / "crates/horizon-client/src/ui_theme.rs").read_text(encoding="utf-8")
        doc_store = (ROOT / "crates/horizon-client/src/android/document_store.rs").read_text(encoding="utf-8")
        documents = (ROOT / "crates/horizon-client/src/android/documents.rs").read_text(encoding="utf-8")
        catalog = (ROOT / "crates/horizon-client/src/android/catalog.rs").read_text(encoding="utf-8")
        launcher = (ROOT / "crates/horizon-client/src/android/launcher.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")
        mouse_state = (
            ROOT / "third_party/winit/src/platform_impl/android/mouse_state.rs"
        ).read_text(encoding="utf-8")
        keycodes = (
            ROOT / "third_party/winit/src/platform_impl/android/keycodes.rs"
        ).read_text(encoding="utf-8")
        native_ime = (ROOT / "crates/horizon-client/src/android/native_ime.rs").read_text(encoding="utf-8")
        game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
        android_input = (
            ROOT / "crates/horizon-client/src/game/android_input.rs"
        ).read_text(encoding="utf-8")
        startup_menu = (
            ROOT / "crates/horizon-client/src/game/startup_menu.rs"
        ).read_text(encoding="utf-8")
        menu_blur = (ROOT / "crates/horizon-client-render/src/menu_blur.rs").read_text(encoding="utf-8")
        perspective = (
            ROOT / "crates/horizon-client/src/game/perspective.rs"
        ).read_text(encoding="utf-8")
        item_gui = (ROOT / "crates/horizon-client/src/item_render/gui.rs").read_text(encoding="utf-8")
        item_render = (ROOT / "crates/horizon-client/src/item_render.rs").read_text(encoding="utf-8")
        ui_render = (ROOT / "crates/horizon-client-render/src/ui.rs").read_text(encoding="utf-8")
        screens = (ROOT / "crates/horizon-client/src/game/screens.rs").read_text(encoding="utf-8")

        # 1. MCPE Floating Menu bubble appearance (Horizon H icon in MCPE, MenuEquals in GoyControl)
        self.assertIn("if settings.control_style == ControlStyle::Mcpe {", touch_ui)
        self.assertIn("Icon::HorizonLogo", touch_ui)
        self.assertIn("Icon::MenuEquals", touch_ui)

        # 2. Dynamic joystick touch-origin behaviour
        self.assertIn("pub fn move_anchor(&self) -> Option<[f32; 2]>", touch)
        self.assertIn("move_area: JoystickMoveArea::FollowFinger", layout)
        self.assertIn("dynamic_joystick: true,", layout)
        self.assertIn("let Some(anchor) = tc.move_anchor()", touch_ui)

        # 3. Control-type switching safety & custom button bounds safety
        self.assertIn("tc.settings = ed.original.clone();", touch_ui)
        self.assertIn("fn clamp_editor_custom_selection(", touch_ui)
        self.assertIn("self.android_release_touch_keys(true);", android_input)

        # 4. External KBM support, pointer capture & stability
        self.assertIn("pub rel_x: f64,", mouse_state)
        self.assertIn("matches!(sample.action, 2 | 7)", mouse_state)
        self.assertIn("let ending = sample.action == 3;", mouse_state)
        self.assertIn("KEY_CHARACTER_MAPS", keycodes)
        self.assertIn("os_mouse_captured: false,", game)
        self.assertIn('"setDefaultFocusHighlightEnabled"', native_ime)

        # 5. Smooth auto-hide touch controls on external KBM
        self.assertIn("pub hud_opacity_scale: f32,", touch)
        self.assertIn("pub fn update_visibility(&mut self, touch_active: bool, dt: f32)", touch)
        self.assertIn("self.pad.touch.update_visibility(", game)
        self.assertIn(".update_visibility(crate::touch_visible(), 1.0 / 60.0);", startup_menu)

        # 6. Mouse cursor rendering & clipping (never over Control Editor, no (0,0) teleport)
        self.assertIn("if controls.editor.is_some() {\n            return false;\n        }", touch_ui)
        self.assertNotIn("(cursor.0 == 0.0 && cursor.1 == 0.0)", touch_ui)
        self.assertIn("let _ = crate::platform::capture_mouse(false);", launcher)

        # 7. Installed Tab redesign & Integrated Mini File Manager
        self.assertIn("pub enum InstalledSubTab", catalog)
        self.assertIn("pub fn fm_paste_here(", catalog)
        self.assertIn("pub fn fm_paste(", doc_store)
        self.assertIn("pub fn fm_confirm_delete(&mut self, home: &Path)", catalog)
        self.assertIn("pub fn fm_submit_rename(&mut self, home: &Path)", catalog)
        self.assertIn("pub fn fm_submit_new_folder(&mut self, home: &Path)", catalog)
        self.assertIn("pub fn fm_paste(", doc_store)
        self.assertIn("pub fn begin_into_dir(", documents)
        self.assertIn("pub fn export_file(", documents)
        self.assertIn("fn draw_fm_modal(", launcher_pages)

        # 8 & 9. DPI scaling & EditText / single-line hint redesign
        self.assertIn("(pw / 880.0).min(ph / 440.0)", ui_theme)
        self.assertIn("pub fn hint_line(", ui_theme)
        self.assertIn("ui.hint_line(", touch_ui)

        # 10. Animated Horizon intro screen
        self.assertIn("pub fn draw_horizon_intro(", ui_theme)
        self.assertIn("pub fn draw_animated_horizon_mark(", ui_theme)
        self.assertIn("pub intro_done: bool,", launcher_pages)

        # 11 & 12. MCPE Control Editor Add Control & single-binding exclusivity
        self.assertIn("pub fn set_exclusive_binding(&mut self, token: String)", custom)
        self.assertIn("pub fn is_keyboard_binding(token: &str) -> bool", custom)
        self.assertIn("pub fn is_mouse_binding(token: &str) -> bool", custom)
        self.assertIn("set_exclusive_binding", editor)

        # 13. Rust Edition in-game UI & menu performance
        self.assertIn("pub fn blit_cached(", menu_blur)
        self.assertIn("pub menu_blur_cache: bool,", (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8"))
        self.assertIn("struct GuiCachedItem", item_gui)
        self.assertIn("static CACHE: std::cell::RefCell<Vec<(crate::items::ItemStack, StackView)>>", item_render)
        self.assertIn("src[2] <= 2.0 || src[3] <= 2.0", ui_render)
        self.assertRegex(
            screens,
            r"fill_gradient\(\s*atlas,\s*\(0\.0, 0\.0, w, h\),\s*"
            r"rgba\(80, 0, 0, 96\),\s*rgba\(128, 48, 48, 160\),\s*\)",
        )

        # 14. Vulkan Batch render chunk stabilization & mobile performance
        gfx_code = (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8")
        cull_code = (ROOT / "crates/horizon-client-render/src/gfx/cull.rs").read_text(encoding="utf-8")
        flat_code = (ROOT / "crates/horizon-client-render/src/gfx/flat.rs").read_text(encoding="utf-8")
        budget_code = (ROOT / "crates/horizon-client-render/src/gfx/upload_budget.rs").read_text(encoding="utf-8")
        self.assertIn("let multi_draw = !cfg!(target_os = \"android\")", gfx_code)
        self.assertIn("if cfg!(target_os = \"android\") {", cull_code)
        self.assertIn("std::env::var(\"HORIZON_FLAT_DRAWS\").as_deref() == Ok(\"1\")", flat_code)
        self.assertIn("crate::android_runtime::gpu_batching()", budget_code)



    def test_anr_fix_z_order_keybind_dispatch_and_controls_overhaul(self) -> None:
        test_content = (ROOT / "tools/test_android_content.py").read_text(encoding="utf-8")
        gfx = (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8")
        game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        android_input = (
            ROOT / "crates/horizon-client/src/game/android_input.rs"
        ).read_text(encoding="utf-8")
        startup_menu = (
            ROOT / "crates/horizon-client/src/game/startup_menu.rs"
        ).read_text(encoding="utf-8")
        pad = (ROOT / "crates/horizon-client/src/game/pad.rs").read_text(encoding="utf-8")
        custom = (ROOT / "crates/horizon-client-render/src/touch/custom.rs").read_text(encoding="utf-8")
        editor = (ROOT / "crates/horizon-client-render/src/touch/editor.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")
        options_ui = (ROOT / "crates/horizon-client-render/src/options_ui.rs").read_text(encoding="utf-8")
        ui_theme = (ROOT / "crates/horizon-client/src/ui_theme.rs").read_text(encoding="utf-8")

        # 1. tools/test_android_content.py supplies mod android_runtime stub
        self.assertIn("mod android_runtime {", test_content)
        self.assertIn("pub fn gpu_batching() -> bool", test_content)

        # 2. In-place set_world_assets on Renderer eliminates concurrent VkQueue ANR
        self.assertIn("pub fn matches_sprite_layout(&self, layers: u32) -> bool", gfx)
        self.assertIn("pub fn set_world_assets(", gfx)
        self.assertIn("renderer.set_world_assets(", game)

        # 3. Mouse cursor z-order strictly below touch controls & floating menu, hidden when floating_menu is open
        self.assertLess(
            game.index("horizon_client::touch_ui::draw_pointer("),
            game.index("self.pad.touch.draw("),
        )
        self.assertLess(
            startup_menu.index("horizon_client::touch_ui::draw_pointer("),
            startup_menu.index("horizon_client::touch_ui::draw_goy_hud("),
        )
        self.assertIn("if self.floating_menu", touch_ui)

        # 4. GoyControls / PRI / SEC / SendCustomInput key & mouse dispatch + key capture
        self.assertIn("fn android_custom_token_edge(", android_input)
        self.assertIn("fn android_custom_token_tap(", android_input)
        self.assertIn("pub(super) late_custom_releases: Vec<String>,", pad)
        self.assertIn("pub fn capture_custom_input_key(", touch_ui)
        self.assertIn("pub fn capture_custom_key_code(", editor)
        self.assertIn("pub fn held_custom_tokens(&self)", custom)

        # 5. Settings > Controls & Page::Touch & UI performance overhaul
        self.assertIn("Action::CycleJoystickZone", launcher_pages)
        self.assertIn("Action::ToggleTouchVibrate", launcher_pages)
        self.assertIn('Row::Header("Movement & Look Sensitivity")', options_ui)
        self.assertIn("glyph_scratch: Vec<(f32, f32, Rect)>,", ui_theme)


    def test_mali_g52_shaders_startup_freeze_arena_pooling_and_m3_controls_overhaul(self) -> None:
        pack_post = (
            ROOT / "crates/horizon-client-render/src/gfx/pack_post.rs"
        ).read_text(encoding="utf-8")
        pack_atlas = (
            ROOT / "crates/horizon-client-render/src/gfx/pack_atlas.rs"
        ).read_text(encoding="utf-8")
        pack_uniforms = (
            ROOT / "crates/horizon-client-render/src/gfx/pack_uniforms.rs"
        ).read_text(encoding="utf-8")
        arena = (ROOT / "crates/horizon-client-render/src/gfx/arena.rs").read_text(encoding="utf-8")
        graphics = (ROOT / "crates/horizon-client-render/src/graphics.rs").read_text(encoding="utf-8")
        gfx = (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8")
        shader_packs = (
            ROOT / "crates/horizon-client/src/game/shader_packs.rs"
        ).read_text(encoding="utf-8")
        game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
        startup_menu = (
            ROOT / "crates/horizon-client/src/game/startup_menu.rs"
        ).read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")

        # 1. Shader pack compatibility on max_bind_groups=4 mobile GPUs & non-flat mc_Entity
        self.assertIn("fn remap_bind_group(", pack_post)
        self.assertIn("vertex_writable_storage: bool,", pack_post)
        self.assertIn("pub fn has_any_pass(&self) -> bool", pack_post)
        self.assertIn("pub fn terrain_inputs_with_ids(", pack_atlas)
        self.assertIn('"endFlashIntensity"', pack_uniforms)
        self.assertIn("pub fn has_active_shader_pack(&self) -> bool", gfx)
        self.assertIn("PENDING_SHADER_JOB", shader_packs)

        # 2. Startup / MC Title Menu freeze elimination on Android
        self.assertIn('let rest_loader = if menu && !cfg!(target_os = "android") {', game)
        self.assertIn('let android_boot_loading = cfg!(target_os = "android")', startup_menu)
        self.assertIn("self.arena.prewarm(&self.device);", gfx)

        # 3. Mali-G52 MC2 32 MiB 64-bit arena pages, spare page recycling & big.LITTLE worker cap
        self.assertIn("spare: Vec<(wgpu::Buffer, u64, bool)>,", arena)
        self.assertIn("pub fn prewarm(&mut self, device: &wgpu::Device)", arena)
        self.assertIn("cores.saturating_sub(2).clamp(1, 4)", graphics)

        # 4. Material Design 3 Expressive overhaul of Settings > Controls & Property Inspector
        self.assertIn("SettingControl::SectionHeader(", launcher_pages)
        self.assertIn("SettingControl::Slider {", launcher_pages)
        self.assertIn("fn draw_m3_stepper_cell(", touch_ui)
        self.assertIn("fn draw_m3_swatch_chip(", touch_ui)


    def test_studio_props_recommended_resolution_shaders_and_animations_revamp(self) -> None:
        custom_rs = (ROOT / "crates/horizon-shaders/src/custom.rs").read_text(encoding="utf-8")
        scan_rs = (ROOT / "crates/horizon-shaders/src/translate/scan.rs").read_text(encoding="utf-8")
        rewrite_rs = (ROOT / "crates/horizon-shaders/src/translate/rewrite.rs").read_text(encoding="utf-8")
        pack_sky = (ROOT / "crates/horizon-client-render/src/gfx/pack_sky.rs").read_text(encoding="utf-8")
        pack_post = (ROOT / "crates/horizon-client-render/src/gfx/pack_post.rs").read_text(encoding="utf-8")
        pack_uniforms = (
            ROOT / "crates/horizon-client-render/src/gfx/pack_uniforms.rs"
        ).read_text(encoding="utf-8")
        shader_packs = (ROOT / "crates/horizon-client/src/game/shader_packs.rs").read_text(encoding="utf-8")
        menu_blur = (ROOT / "crates/horizon-client-render/src/menu_blur.rs").read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(encoding="utf-8")
        launcher_pages = (
            ROOT / "crates/horizon-client/src/android/launcher_pages.rs"
        ).read_text(encoding="utf-8")
        ui_theme = (ROOT / "crates/horizon-client/src/ui_theme.rs").read_text(encoding="utf-8")

        # 1. MakeUp & E-Lite gl_FogFragCoord + trailing semicolon shader fixes
        self.assertIn("gl_FogFragCoord", scan_rs)
        self.assertIn("hz_FogFragCoord", rewrite_rs)
        self.assertIn('self.tokens.last().is_some_and(|t| t == ";")', custom_rs)

        # 2. BSL subdivided sky dome, unified far plane & Mali-G52 4-bind-group / 4-color-attachment folding
        self.assertIn("let rings = 6i32;", pack_sky)
        self.assertIn("let slices = 24i32;", pack_sky)
        self.assertIn("frame.depth_far.max(frame.detail_distance)", pack_uniforms)
        self.assertIn("fn trim_fragment_outputs(", pack_post)
        self.assertIn("160 + b", pack_post)
        self.assertIn("shader_pack_build_context()", shader_packs)

        # 3. Zero-lag menu backdrop cache & Virtual Mouse OFF by default
        self.assertIn("if radius < 0.5 {", menu_blur)
        self.assertIn("virtual_mouse_enabled: false,", layout)

        # 4. Studio ButtonProperty Inspector (90511.png) + Icon badge + Recommended tab + Real-Time Resolution
        self.assertIn("pub icon_id: u8,", layout)
        self.assertIn("pub resolution_scale_percent: u32,", layout)
        self.assertIn("pub fn control_icon_from_id(", touch_ui)
        self.assertIn("fn draw_preview_brackets(", touch_ui)
        self.assertIn("(Icon::HorizonLogo, \"Recommended\")", touch_ui)
        self.assertIn("SetResolutionScale(u32),", touch_ui)
        self.assertIn("ToggleGameAnimations,", touch_ui)

        # 5. Disable Launcher / In-Game Animations & Diagnostics Summary Modal
        self.assertIn("pub fn set_launcher_animations_disabled(", ui_theme)
        self.assertIn("pub fn set_game_animations_disabled(", ui_theme)
        self.assertIn("Action::ToggleLauncherAnimations", launcher_pages)
        self.assertIn("Action::ToggleGameAnimations", launcher_pages)
        self.assertIn("fn draw_diagnostics_summary_modal(", launcher_pages)



    def test_mali_mrt_shader_ground_complementary_passthrough_and_m3_sliders(self) -> None:
        adapter = (ROOT / "third_party/wgpu-hal/src/vulkan/adapter.rs").read_text(encoding="utf-8")
        flat = (ROOT / "crates/horizon-client-render/src/gfx/flat.rs").read_text(encoding="utf-8")
        mesher = (ROOT / "crates/horizon-client-render/src/mesher.rs").read_text(encoding="utf-8")
        pack_atlas = (ROOT / "crates/horizon-client-render/src/gfx/pack_atlas.rs").read_text(encoding="utf-8")
        pack_post = (ROOT / "crates/horizon-client-render/src/gfx/pack_post.rs").read_text(encoding="utf-8")
        gfx = (ROOT / "crates/horizon-client-render/src/gfx.rs").read_text(encoding="utf-8")
        game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(encoding="utf-8")
        touch = (ROOT / "crates/horizon-client-render/src/touch.rs").read_text(encoding="utf-8")
        editor = (ROOT / "crates/horizon-client-render/src/touch/editor.rs").read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        launcher_pages = (ROOT / "crates/horizon-client/src/android/launcher_pages.rs").read_text(encoding="utf-8")

        # 1. ARM Mali (0x13b5) MRT limit fix & !flat terrain normal/tangent decoding
        self.assertIn("0x13b5", adapter)
        self.assertIn("0x5143", adapter)
        self.assertIn('std::env::var("HORIZON_FLAT_DRAWS").as_deref() == Ok("1")', flat)
        self.assertIn("((q.facing as u32) & 0x7) << 28", mesher)
        self.assertIn(
            "uint hz_face = min((uint(round(hz_v_color.a * 255.0)) >> 4u) & 7u, 5u);",
            pack_atlas,
        )
        self.assertIn("module.types.insert(", pack_post)
        self.assertIn("pub use pack_post::PackPost;", gfx)

        # 2. Complementary Reimagined loads while in-world menu is open (if title, not if skip_world)
        self.assertIn("let pack_inputs = if title {", game)

        # 3. Default MCPE Hit button (Control::Attack) passthrough & continuous camera drag
        self.assertIn("passthrough_fingers: BTreeSet<u64>,", touch)
        self.assertIn("row[Control::Attack as usize].passthrough = true;", layout)

        # 4. Real-time continuous slider dragging in ButtonProperty & Video Settings
        self.assertIn("PropsSlider {", editor)
        self.assertIn("fn apply_props_slider(", touch_ui)
        self.assertIn("fn draw_m3_selector_cell(", touch_ui)
        self.assertIn("Action::RenderScaleDelta(", launcher_pages)


    def test_final_polish_vertex_shaders_mods_2d_goy_skins_and_sliders(self):
        manifest = (ROOT / "crates/horizon-shader-api/src/manifest.rs").read_text(encoding="utf-8")
        stages = (ROOT / "crates/horizon-shader-api/src/stages.rs").read_text(encoding="utf-8")
        pack_shadow = (ROOT / "crates/horizon-client-render/src/gfx/pack_shadow.rs").read_text(encoding="utf-8")
        pack_uniforms = (ROOT / "crates/horizon-client-render/src/gfx/pack_uniforms.rs").read_text(encoding="utf-8")
        perspective = (ROOT / "crates/horizon-client/src/game/perspective.rs").read_text(encoding="utf-8")
        shader_packs = (ROOT / "crates/horizon-client/src/game/shader_packs.rs").read_text(encoding="utf-8")
        rewrite = (ROOT / "crates/horizon-shaders/src/translate/rewrite.rs").read_text(encoding="utf-8")
        catalog = (ROOT / "crates/horizon-client/src/android/catalog.rs").read_text(encoding="utf-8")
        wardrobe = (ROOT / "crates/horizon-client/src/android/wardrobe.rs").read_text(encoding="utf-8")
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(encoding="utf-8")
        touch_ui = (ROOT / "crates/horizon-client/src/touch_ui.rs").read_text(encoding="utf-8")
        launcher_pages = (ROOT / "crates/horizon-client/src/android/launcher_pages.rs").read_text(encoding="utf-8")

        self.assertIn("pub vertex: Option<Stage>,", manifest)
        self.assertIn("StageKind::Vertex", stages)
        self.assertIn(".min(4096)", pack_shadow)
        self.assertIn('"lightningBoltPosition"', pack_uniforms)
        self.assertIn('"normalMatrix"', pack_uniforms)
        self.assertIn("self.pack_shadow_casters.0", perspective)
        self.assertIn("pub fn pending_shader_pack_status()", shader_packs)
        self.assertIn("hz_dot", rewrite)
        self.assertIn("float hz_dot(vec3 a, vec3 b) { return dot(a, b); }", rewrite)
        inputs_rs = (ROOT / "crates/horizon-shader-api/src/inputs.rs").read_text(encoding="utf-8")
        self.assertIn("colored_light:", inputs_rs)
        self.assertIn("displaced:", inputs_rs)
        self.assertIn("pub const SURFACE_DATA: Struct", inputs_rs)
        self.assertIn("crate::touch::draw_mcpe_custom_button(", touch_ui)
        self.assertIn("{ if (true) discard; }", rewrite)
        self.assertIn("builtin:", catalog)
        self.assertIn("query.limit = 100;", catalog)
        self.assertNotIn("Action::WardrobeZoom(false)", wardrobe)
        self.assertIn("fn default_shadow_size() -> f32 {\n    0.0\n}", layout)
        self.assertIn("let sens_card_y = body_y + 148.0;", touch_ui)
        self.assertIn("pub gyro_sensitivity_dragging: bool,", touch_ui)
        self.assertIn("pub button_scale_dragging: bool,", touch_ui)
        self.assertIn("ACTIVE_SLIDER_DRAG", launcher_pages)


if __name__ == "__main__":

    unittest.main()
