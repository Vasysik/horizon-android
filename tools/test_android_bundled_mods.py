#!/usr/bin/env python3
"""Test the fixed MIT Android mod runtimes without launching a game."""
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[1]

def main():
    work = ROOT / '.local/android/bundled-mod-tests'
    (work / 'src').mkdir(parents=True, exist_ok=True)
    (work / 'Cargo.toml').write_text("""[package]
name = "android-bundled-mod-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde_json = "1"
""", encoding='utf-8')
    lines = ['#![allow(dead_code)]']
    for mod in ['zoom', 'minimap', 'recipes', 'smartf3']:
        source = ROOT / 'crates/horizon-client/src/game/android_bundled' / mod / 'runtime.rs'
        lines.append(f'#[path = "{source.as_posix()}"] mod {mod};')
    source = ROOT / 'crates/horizon-client-render/src/mod_list.rs'
    lines.append(f'#[path = "{source.as_posix()}"] mod mod_list;')
    (work / 'src/lib.rs').write_text('\n'.join(lines), encoding='utf-8')
    subprocess.run(['cargo', 'test', '--manifest-path', str(work / 'Cargo.toml'), '--offline', '--', '--test-threads=1'], cwd=ROOT, check=True)

if __name__ == '__main__':
    main()
