#!/usr/bin/env python3
"""Run the Android input device tracker without a device or game assets."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    root = ROOT / '.local/android/hotplug-tests'
    root.mkdir(parents=True, exist_ok=True)
    source = ROOT / 'third_party/winit/src/platform_impl/android/input_devices.rs'
    wrapper = root / 'tests.rs'
    wrapper.write_text(f'#[path = "{source.as_posix()}"] mod input_devices;\n', encoding='utf-8')
    binary = root / ('tests.exe' if os.name == 'nt' else 'tests')
    subprocess.run(['rustc', '--edition=2021', '--test', str(wrapper), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    main()
