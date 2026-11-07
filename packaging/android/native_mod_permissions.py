#!/usr/bin/env python3
"""What of Android a native mod needs that the game asks the player for, read from the mod's source.

A native mod is written for Horizon on a computer, where nothing asks before a program opens the
microphone; on Android the app must hold the permission, and ask the player for it. The mod is
never changed for that (it is plug and play): native_build.sh writes what this finds in the
build's build.json, each mod with its own, and StarlingActivity asks for it, saying which mod
wants it, once the build is running (NativeMods.permissions).

    native_mod_permissions.py FOLDER...    # each mod folder, and what it needs

A mod needs a permission when its sources call the APIs that reach what it guards: for the
microphone, capturing sound through cpal, AAudio (and the ndk crate, Oboe), OpenSL ES or
Android's AudioRecord and MediaRecorder.
"""

import re
import sys
from pathlib import Path

# Each permission, by the name build.json gives it, and the signs of a mod using what it guards.
PERMISSIONS = {
    "microphone": re.compile(
        r"build_input_stream|default_input_device|input_devices\s*\(|supported_input_configs"
        r"|AAUDIO_DIRECTION_INPUT|AudioDirection::Input|Direction::Input"
        r"|SL_IID_RECORD|SLRecordItf|CreateAudioRecorder"
        r"|android[/.]media[/.]AudioRecord|android[/.]media[/.]MediaRecorder"
    ),
}

# The files read: Rust, C and C++ (a mod's own, and what it carries), Java and Kotlin.
SOURCES = {".rs", ".c", ".h", ".cc", ".cpp", ".hpp", ".java", ".kt"}


def needs(folder: Path) -> list[str]:
    """The permissions the mod in `folder` needs, sorted."""
    found = set()
    for path in sorted(folder.rglob("*")):
        if path.suffix.lower() not in SOURCES or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for name, sign in PERMISSIONS.items():
            if name not in found and sign.search(text):
                found.add(name)
        if len(found) == len(PERMISSIONS):
            break
    return sorted(found)


def main() -> int:
    for folder in sys.argv[1:]:
        print(f"{folder}: {' '.join(needs(Path(folder))) or '-'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
