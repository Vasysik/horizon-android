# Horizon on Android

Horizon Android is a native Rust/wgpu client for Minecraft Java **26.3**.
It downloads the required resources on the device and supports single player
and Java multiplayer. It does not run arbitrary Minecraft JAR versions.

## Install and play

1. Install the APK matching the device architecture from [Releases](https://github.com/Vasysik/horizon-android/releases).
2. Open Horizon and choose **Download Minecraft 26.3**.
3. Wait for resource verification and installation, then choose **Play**.

The first installation downloads approximately 500 MiB, including sounds.
Downloads use official Mojang HTTPS endpoints and pinned hashes. Verified cache
entries are reused after an interrupted download. Subsequent launches can use
the installed resources offline. No PC preparation or Java runtime is required
on the phone. Use a licensed Minecraft account for authenticated servers.

Worlds, profiles, resource caches and account data use app-private storage.
Updating with the same signing certificate preserves them; uninstalling the app
or clearing its storage removes them.

| APK architecture | Minimum Android | Native `.hmod` runtime |
| --- | --- | --- |
| `arm64-v8a` | 9 / API 28 | Included |
| `x86_64` | 9 / API 28 | Included |
| `armeabi-v7a` | 8.1 / API 27 | Not included; experimental legacy build |
| `x86` | 8.1 / API 27 | Not included; experimental legacy build |

A matching CPU architecture alone does not establish graphics-driver or memory
compatibility. Legacy GPU testing remains in progress. When a shared atlas cannot
fit the adapter, Android reduces static atlas sprites while retaining font pixels,
dynamic slots and the separate terrain texture arrays.

## Launcher

- **Play:** install resources and launch a world or server.
- **Mods:** import native Horizon packages using a direct HTTPS `.hmod` URL
  on 64-bit builds. New packages are disabled until enabled in Installed.
- **Packs:** find and install resource packs with checksum verification.
- **Installed:** enable, disable or remove installed additions.
- **Settings:** rendering mode, input mode, profile, server and orientation.
  Profiles keep separate saves and settings. Offline player names are edited
  alongside Microsoft sign-in in **Accounts**.

Java mod search and modpack import are not available on Android. Fabric, Forge,
NeoForge and Mixin code require native ports; installing their JARs does not make
them executable by this engine. See [Android feature support](android-features.md).

The installer displays the current stage: metadata checks, resource download,
unpacking, world-data preparation, touch textures and finalization. Progress shows
MiB or file counts where available. Retry retains previously verified downloads.

### Accounts

Sign-in opens a browser Custom Tab. The device code is copied to the clipboard
and passed to Microsoft's verification page; a Copy button is also available.
Closing the account panel keeps a pending request active; **Cancel sign-in**
cancels it. Account status distinguishes signed in, signing in and offline use.

Refresh tokens are encrypted with an app-owned Android Keystore AES-GCM key and
kept in private storage. Clearing app data requires signing in again. The current
application registration is a temporary development configuration, not a
project-owned Microsoft/Mojang approval. Production registration remains pending.
Server and anticheat compatibility varies; see [account integration](microsoft-account.md).

## Touch controls

Four schemes combine joystick or D-pad movement with tap or crosshair aiming:

| Movement | Aiming |
| --- | --- |
| Joystick | Crosshair |
| Joystick | Tap to interact |
| D-pad | Crosshair |
| D-pad | Tap to interact |

In tap modes, a short mob tap attacks. A stationary hold commits mining or an
available item/entity interaction. A camera swipe does not immediately consume
held food. After a hold commits, the camera can move while the action continues;
releasing the finger ends it. Changing a mining target starts its own progress.
The mining indicator grows an inner circle towards the fixed outer rim.

Crosshair modes keep Use/Place visible. The optional contextual Interact button
is off by default in tap modes; when enabled it appears for a usable target and
stays visible while pressed. Default joystick movement follows eight Java input
directions; optional 360-degree movement is available in Touch Controls.

Open **Options > Controls > Touch Controls** to select a scheme, adjust
sensitivity, global button size, circle size, F3 text scale, handedness and
Sprint-button visibility. Global button size affects both drawing and hit areas;
individual saved sizes remain editable.
**Auto Sprint** is beside **Auto-Jump** in Controls. Hiding Sprint leaves normal
double-forward sprinting available. The attack indicator uses actual weapon
recovery and can sit left or right of the hotbar.

**Customize layout** edits button position, size and opacity for each scheme.
**Add buttons...** restores built-in controls or adds a **custom key**. Select its
Action to choose a keyboard key, or press a physical key while the picker is open.
For example, a Z button holds Z until its finger is released, using the same
key bindings as a keyboard. Position, size and opacity are editable per scheme.
Up to 32 custom buttons are supported, including duplicate keys. Removing a
custom button deletes that binding; removing a built-in hides it. Done saves;
Cancel/Android Back restores the previous layout. Multi-key macros are not implemented.

The top row includes F3, chat, pause, Options and F5 perspective. The auxiliary
`...` drawer retains F1â€“F12 and other Java keys. Multiple fingers or **Hold F3**
allow combinations. Menus support taps, list swipes and slider/inventory dragging.
Android Back closes the keyboard first, then the current screen.

### HUD margins and orientation

**Options > Accessibility** has horizontal and vertical HUD margins. These inset
passive information such as F3, scoreboard, effects and subtitles. They do not
resize the world, hotbar, inventory, touch controls or their input coordinates.

Launcher settings offer landscape, portrait, reversed orientations and sensor
rotation. Orientation and HUD margins are independent.

### Keyboard and mouse

In launcher Settings, **Automatic** selects keyboard/mouse input when Android reports
both a QWERTY keyboard and a mouse. **Keyboard & mouse** selects it explicitly;
**Touch & Android keyboard** restores touch input. The preference is applied at Play.

One stock `android.app.NativeActivity` runs the Rust client for all input modes.
The system keyboard uses Android's stock text editor and input-method service
through Rust/JNI on the UI thread. There are no project Java/Kotlin classes and
no second copy of the engine in the APK.

Mouse cursor, buttons, wheel and modifiers use the normal desktop input path.
Pointer capture provides relative camera movement and releases in menus or on
focus loss. Touch buttons hide on hardware input in mouse mode and return on a
touch. The system keyboard is available in either mode.

### Physical keypad and Android TV

Open **Options > Controls > Key Binds** and change **Input profile** from
Keyboard to **D-pad / keypad**. The profile is optional and off by default;
its rows support rebinding and Reset just like keyboard bindings. It is saved
per game profile in `horizon-keypad.json`. Existing launcher presets migrate once.

| Default key | Action |
| --- | --- |
| D-pad arrows / numpad 8, 2, 4, 6 | Walk |
| Numpad 7, 9, 1, 3 | Diagonal movement |
| Number row 2 / 8 / 4 / 6 | Look up / down / left / right |
| Number row 1 / 3 | Attack / use |
| 5, numpad 5, Enter or D-pad centre | Jump |
| 0 / numpad 0 | Sneak |
| Number row 7 | Inventory |
| Number row 9 | Next hotbar slot |

All eighteen actions are editable. Opposing directions cancel and diagonals
are normalized to normal movement speed. Number-row keys and numpad keys can
have different actions. Menus retain arrows and Enter. Android TV has a Leanback
launcher entry; TV support is experimental and analog gamepad axes are not connected yet.

### Screenshots

F2 keeps the original PNG in the profile and publishes a copy to
**Pictures/HorizonAndroid** through Android MediaStore. Android 10+ needs no
storage permission. Android 8.1–9 asks for storage permission when first saving;
if access is denied, the profile copy remains available. Gallery publication runs
outside the render thread and reports success or failure in game chat.

## Rendering, sound and vibration

The launcher exposes actual engine paths: **Vulkan / GPU batching**,
**Vulkan / compatibility** and experimental **OpenGL ES**. Compatibility mode
uses direct terrain draws for drivers with rendering issues. GLES needs ES 3.0+
and the texture/shader limits required by the engine; 32-bit builds default to it.
These modes use the system driver, without bundled ANGLE or Turnip replacements.

Resolution, chunks, frame limits, graphics quality and worker budgets remain game
settings. The launcher does not overwrite them. Its own renderer uses Auto
(Vulkan then GLES) independently, so a failing game API does not lock out settings.

AAudio plays the downloaded official sounds and respects game/category volumes.
The controller Vibration setting controls the phone vibrator for supported game
events, including block destruction. External-controller rumble and adaptive
triggers need a separate backend. Leaving the game returns to the launcher.

## Build (developers only)

Install Rust 1.95 (pinned in `rust-toolchain.toml`), Python 3.11+, Android SDK platform 35, build tools, NDK, a JDK and
Gradle 8.14. Android Studio's bundled JDK is supported. Prepare the pinned engine
tables privately, then build the required ABI:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
python tools/android_prepare.py
python tools/android_build.py --abi arm64-v8a --public
python tools/android_build.py --abi armeabi-v7a --public
python tools/android_build.py --abi x86 --public
python tools/android_build.py --abi x86_64 --public
```

Windows discovers `%LOCALAPPDATA%/Android/Sdk` and Android Studio's JDK. Otherwise
set `ANDROID_HOME`/`JAVA_HOME` or pass `--sdk`/`--java-home`. `--gradle` selects a
Gradle executable. Public APKs are written to
`.local/android/apk/<abi>/horizon-android-public.apk`; signing material stays in
`.local/android/`. Keep the signing key to allow updates over an existing install.

Each APK contains one `libhorizon_android.so`, using the NativeActivity winit
backend and the Rust system-keyboard bridge. `--rust-only` builds it without shared
APK staging; `CARGO_TARGET_DIR` can separate parallel compilation caches. Run Gradle
packaging sequentially. Public builds default to `release`: optimization level 3,
ThinLTO and one codegen unit. Local iteration defaults to optimized `fast` without
LTO. `--profile dev` is for debugging and is rejected for public/candidate APKs.

Public builds include only allowlisted generated engine tables. Original game
resources and control textures download on the device. Never commit prepared
resources, game assets, APKs, accounts or keys. See [publication](android-publication.md).

### Editing button textures

Original touch textures are downloaded at a pinned Mojang sample revision.
`tools/android_bedrock_controls.py` prepares a private development copy; its PNGs
are not source-controlled. Chat/F3/F5 use regions of `gui.png`; other states use
`ui/` and `controls/`. A resource pack can override them under
`assets/minecraft/textures/gui/bedrock/<relative path>`.

## Check

```sh
python tools/android_build.py --abi x86_64 --clippy
python -m unittest discover -s tests -p 'test_android*.py'
python tools/check_public_tree.py --history
cargo test -p horizon-client --lib --no-default-features touch
```

Use isolated AVDs and fresh game/launcher/XDG directories for runtime tests, as
required by [AGENTS.md](../AGENTS.md). Developer builds expose the optional
`tools/android_ctl.py` harness; public builds disable automatic agent control and
ignore `android-launch.json` overrides.

The API 35 x86_64 input probe verified keyboard press/release, Shift, touch,
cursor movement, both wheel axes and pointer-capture state. Five Rust tests cover
button edges, relative deltas, cancellation and repeated input snapshots. See
[the probe](../tools/android_input_probe/README.md) for reproduction and limits.
Actual USB/Bluetooth gameplay, vendor GPU compatibility and thermal performance
still need physical-device testing; emulator frame rates are not phone results.

Reviewed upstream revisions are recorded in [android-upstream.json](android-upstream.json).
