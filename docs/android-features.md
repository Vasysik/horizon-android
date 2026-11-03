# Android feature support (0.7.2 development)

Support varies by device and build architecture. The table lists implemented
integrations and remaining platform work; device-specific validation is described
in the [Android guide](android.md#check).

| Horizon feature | Android integration / remaining work |
|---|---|
| Controller action mapping | Touch uses the same actions, default Java eight-direction or optional analog movement, and toggle sneak. Hardware controller enumeration/stick input still needs a native Android backend; do not infer it from desktop gilrs support |
| Touch layouts | Four explicit schemes, normal Options > Controls settings, 16–64 px circle, optional 360-degree movement, sprint visibility, editable layouts, custom key buttons, global size and per-button scale/opacity sliders, handedness and direct F3/F5 buttons; passive HUD safe-area margins are in Accessibility |
| Contextual touch | Use action for held items/entities; movement actions serve flight/swimming. Not all Bedrock contextual icons/gestures are implemented |
| Controller vibration settings | Phone `Vibrator` now wired to existing game events and the existing vibration switch; no adaptive triggers or external-pad vibration backend |
| Physical keyboard / mouse | One stock NativeActivity with selectable input mode; normal keyboard, modifiers, cursor/buttons/wheel and relative pointer-capture path in Rust. Physical gameplay and vendor firmware still need testing |
| ABI / TV | ARM64 and x86_64; experimental ARMv7 and x86 without Wasmtime. Android TV launcher entry, D-pad/Enter navigation, editable keypad profile with in-game focus navigation and retained utility buttons, plus portrait orientations; full analog gamepad backend remains unfinished |
| Unicode keyboard | Stock Android text editor and IME through Rust/JNI on the UI thread; system keyboard remains responsible for its own clipboard suggestions |
| Horizon clipboard actions | Newly connected to Android ClipboardManager for explicit text copy/paste; no background clipboard polling or file-URI reads |
| Sound and subtitles | AAudio mixer and downloaded sound assets; existing category volumes, subtitles and accessibility settings remain available |
| Graphics / streaming | Client-resolution presets and numeric input scale the complete frame, including menus and HUD, from 25–100%, subject to minimum menu dimensions. Existing game graphics controls remain available; launcher selects real Vulkan draw paths |
| Sandboxed Horizon mods | 64-bit builds: Wasmtime `.hmod` runtime; 32-bit omits this runtime; existing capabilities and errors appear in the game mod menu. Official Horizon Mods catalog and direct HTTPS import; no Java-mod search or Java/Mixin execution |
| Resource packs | Checksum-verified resource-pack search/install; modpack browsing, importing, selection and automatic launch hooks are removed on Android |
| APK distribution | Dedicated `--public` build: 32 generated tables, original resources downloaded from official sources on the phone, no automatic agent socket; no player PC preparation |
| Single player | Default integrated server and optional in-frame path. Editor pauses unpublished single player through the engine pause decision |
| Servers | Existing protocol 777 client. Manual matching-version server address supported; Android device-code sign-in and session handoff implemented; temporary Switch dev ID with Microsoft Live device-code flow, browser Custom Tab, code copy and persistent pending login; initial code request accepted, full account login unverified |
| Play with Friends | Shared core exists, but relay/account/clipboard/background behavior needs an Android end-to-end pass before advertising parity |
| Saves and backups | Engine save logic works; Android Storage Access Framework import/export and share UI remain to implement. Uninstalling the app deletes internal data |
| Lifecycle | Touch/key cancellation, vibration stop and surface release/recreation. Leaving the game returns to the native launcher; background process termination still needs broader save/recovery testing |
| Android file access | System-picker import for add-ons; Horizon Android in system Files exposes mods, resource packs, shaders, worlds and reports. Physical game files stay internal. F2 gallery export uses MediaStore; Android 8.1–9 requests storage access for that explicit action. World folders can be copied in Files while the game is closed; dedicated ZIP-world import/export remains separate |
| Physical device tuning | Thermal/frame-pacing and GPU-driver comparisons need measurements on the target phone; an emulator is not evidence of phone performance |
| Desktop updates / telemetry services | Desktop deployment/auto-update infrastructure is not an Android updater; fork exports keep original workflows inactive |

Priorities after this change: world import/export, Minecraft Services approval and production account validation,
hardware gamepads, then measured lifecycle and thermal behavior. Preserve
the current engine performance baseline while adding these platform bridges.
