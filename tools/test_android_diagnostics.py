#!/usr/bin/env python3
"""Run the real bounded diagnostics writer/exporter without Android or player data."""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    harness = ROOT / '.local/android/diagnostics-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text(encoding='utf-8'))
    version = next(p['version'] for p in lock['package']
                   if p['name'] == 'zip' and p['version'].startswith('9.'))
    (harness / 'Cargo.toml').write_text(f'''[package]
name = "android-diagnostics-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde_json = "1"
log = "0.4"
regex = "1"
zip = {{ version = "={version}", default-features = false, features = ["deflate"] }}
''', encoding='utf-8')
    (harness / 'src/lib.rs').write_text('''#![allow(dead_code)]
mod backend { pub const VERSION: &str = "diagnostics-test"; }
#[path = "../../../../crates/horizon-client/src/android/diagnostics.rs"] mod diagnostics;
''', encoding='utf-8')
    subprocess.run(['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml'),
                    '--', '--test-threads=1'], cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
