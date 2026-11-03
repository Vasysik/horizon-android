#!/usr/bin/env python3
"""Host tests for the Android launch contract and persistent device preferences."""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    harness = ROOT / '.local/android/launch-contract-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    (harness / 'Cargo.toml').write_text("""[package]
name = "android-launch-contract-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
""", encoding='utf-8')
    (harness / 'src/lib.rs').write_text("""#![allow(dead_code)]
#[path = "../../../../crates/horizon-client/src/android/game_launch.rs"] mod game_launch;
#[path = "../../../../crates/horizon-client/src/android/device_preferences.rs"] mod device_preferences;
""", encoding='utf-8')
    command = ['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml')]
    if args.offline:
        command.append('--offline')
    subprocess.run(command, cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
