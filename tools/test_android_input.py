#!/usr/bin/env python3
"""Test Android controller decoding and input policy without an APK or player data."""
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[1]
def main():
    harness = ROOT / '.local/android/input-074-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    (harness / 'Cargo.toml').write_text('''[package]
name = "android-input-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
horizon-client-input = { path = "../../../crates/horizon-client-input" }
horizon-client-core = { path = "../../../crates/horizon-client-core" }
horizon-narrator = { path = "../../../crates/horizon-narrator" }
winit = "0.30.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
''', encoding='utf-8')
    (harness / 'src/lib.rs').write_text('''#![allow(dead_code, unused_imports)]
pub use horizon_client_input::{gamepad, keybinds, clipboard};
pub use horizon_client_core::text;
#[path = "../../../../crates/horizon-client-render/src/android_gamepad.rs"] mod android_gamepad;
#[path = "../../../../crates/horizon-client-render/src/android_runtime.rs"] mod android_runtime;
#[path = "../../../../crates/horizon-client-render/src/keypad.rs"] mod keypad;
#[path = "../../../../crates/horizon-client-render/src/widgets.rs"] mod widgets;
mod graphics { pub enum GraphicsApi {OpenGl, Vulkan} }
''', encoding='utf-8')
    subprocess.run(['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml'), '--', '--test-threads=1'], cwd=ROOT, check=True)
if __name__ == '__main__':
    main()
