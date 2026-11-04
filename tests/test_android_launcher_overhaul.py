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
            "launcher/src/skins/preview.rs",
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
        layout = (ROOT / "crates/horizon-client-render/src/touch/layout.rs").read_text(
            encoding="utf-8"
        )
        touch = (ROOT / "crates/horizon-client-render/src/touch.rs").read_text(encoding="utf-8")
        options_ui = (ROOT / "crates/horizon-client-render/src/options_ui.rs").read_text(
            encoding="utf-8"
        )
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
        wardrobe = (ROOT / "crates/horizon-client/src/android/wardrobe.rs").read_text(
            encoding="utf-8"
        )
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
        catalog = (ROOT / "crates/horizon-client/src/android/catalog.rs").read_text(
            encoding="utf-8"
        )
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


if __name__ == "__main__":
    unittest.main()
