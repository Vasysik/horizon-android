#!/usr/bin/env python3
"""Exercise shared account, wardrobe, cloud parsing and 3D preview code on the host.

No real account, credential store, game resources, game or launcher is opened.
Only the unrelated backend paths and proxy settings are replaced by fixtures.
"""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    harness = ROOT / '.local/android/appearance-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    manifest = '''[package]
name = "horizon-appearance-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
png = "0.18"
sha2 = "0.10"
getrandom = "0.3"
ureq = { version = "2", features = ["json"] }
url = "2"
image = { version = "0.25", default-features = false, features = ["png"] }
[target.'cfg(windows)'.dependencies]
keyring = { version = "3", features = ["windows-native"] }
[target.'cfg(any(target_os = "linux", target_os = "freebsd"))'.dependencies]
oo7 = { version = "0.6", default-features = false, features = ["async-std", "native_crypto"] }
futures-lite = "2"
[target.'cfg(target_os = "macos")'.dependencies]
keyring = { version = "3", features = ["apple-native"] }
security-framework = "3"
'''
    manifest = manifest.replace('[dependencies]', '[dependencies]\nhorizon-core = { path = "' + (ROOT / 'crates/horizon-core').as_posix() + '" }')
    (harness / 'Cargo.toml').write_text(manifest, encoding='utf-8')
    source = '''#![allow(dead_code)]
mod backend {
    pub const VERSION: &str = "appearance-test";
    pub fn data_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("horizon-appearance-harness")
    }
}
mod proxy {
    pub fn agent(builder: ureq::AgentBuilder, _: &str) -> ureq::AgentBuilder { builder }
}
mod java_launch {
    #[derive(Debug)]
    pub struct Player { pub name: String, pub uuid: String, pub token: String, pub xuid: String }
}
'''
    for name in ('account', 'skins'):
        source += f'#[path = "{(ROOT / "launcher/src" / (name + ".rs")).as_posix()}"] pub mod {name};\n'
    source += "mod hud { pub fn decode_bytes(bytes:&[u8])->Result<(u32,u32,Vec<u8>),String> { let image=image::load_from_memory(bytes).map_err(|e|e.to_string())?.to_rgba8(); Ok((image.width(),image.height(),image.into_raw())) } }\n"
    source += 'mod engine { pub use horizon_core::player_texture::{process_cape,process_legacy_skin}; #[path="' + (ROOT / 'crates/horizon-client-render/src/skins/appearance.rs').as_posix() + '"] pub mod appearance; }\n'
    (harness / 'src/lib.rs').write_text(source, encoding='utf-8')
    environment = dict(os.environ)
    environment['HORIZON_LAUNCHER_NO_SELF_UPDATE'] = '1'
    subprocess.run(['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml')],
                   cwd=ROOT, env=environment, check=True)


if __name__ == '__main__':
    main()
