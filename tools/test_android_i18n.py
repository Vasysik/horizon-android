#!/usr/bin/env python3
"""Host tests for offline Android localization, fonts and text fitting."""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    harness = ROOT / '.local/android/i18n-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    (harness / 'Cargo.toml').write_text('''[package]
name = "android-i18n-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ab_glyph = "0.2.32"
horizon-theme = { path = "../../../crates/horizon-theme" }
''', encoding='utf-8')
    (harness / 'src/lib.rs').write_text('''#![allow(dead_code)]
#[path = "../../../../crates/horizon-client/src/android/i18n.rs"] mod i18n;
#[path = "../../../../crates/horizon-client/src/ui_fonts.rs"] mod ui_fonts;
#[path = "../../../../crates/horizon-client/src/ui_text.rs"] mod ui_text;
''', encoding='utf-8')
    command = ['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml')]
    if args.offline:
        command.append('--offline')
    subprocess.run(command, cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
