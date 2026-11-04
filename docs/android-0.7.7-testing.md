# Android 0.7.7 candidate: changes and verification

This is a development candidate, not a declaration that every Android device is supported.
The upstream integration targets the published 2026-11-01 nightly
`8c02c0b8534e11b7bb8be14238b140a8534280a0`, not the moving main branch.
Device observations recorded before that integration must be repeated on the resulting APK.

## Changes from 0.7.6

1. Installer recovery: re-opened asset index, client JAR and objects recover from cache eviction with bounded downloads and the original size/hash checks. Errors identify the input. Disposable cache and successful-install cleanup are retained.
2. Block-behaviour cache: schema and exact class-variant order participate in its fingerprint, avoiding stale serialized ordinals after Android engine updates.
3. GLES terrain: compatible flat integers and interpolated scalar varyings are packed into vectors. No vertex-format, texture-quality, fog, lighting or fade downgrade. The nightly height and emissive-mask fields are included. GLES 3.0 also uses a flat colored-light uniform array with the same byte layout, avoiding unsupported nested arrays, and conditional HDR texture selection for legacy drivers rejecting unsigned switch labels. Vulkan uses the unmodified upstream WGSL.
4. GPU readback: a destroyed or failed timestamp buffer is retired rather than unmapped or reused. This addresses a specific validation panic, not every driver crash.
5. Startup D-pad: standalone menu frames publish navigation targets; Android controller directions and confirmation can navigate before full world-renderer initialization.
6. GLES startup: world GPU pipelines wait for an explicit world/server request and a presented loading frame, rather than freezing an idle title screen to build unused pipelines.
7. Native IME: command, structure, jigsaw and test-block editor fields participate in focus, committed text, backspace and confirmation routing.
8. Launcher/game boundary: explicit launch request contains profile/resources, identity/session-file reference, server and renderer selection. The engine does not reread frontend configuration to construct that request. Tokens are not command arguments.
9. Launcher Back: dismiss editing first, then a compact Cancel/Exit confirmation. Cancel receives initial keyboard focus.
10. Chunk worker ceiling: configurable; zero preserves the engine default, with no extra Android ceiling. It does not restrict all process threads.
11. Reusable RAM caches: configurable mesh/sound cache budget; zero preserves the current engine defaults. Live world data, playing audio and active textures are not evicted by this setting. It is not a process-RAM or VRAM hard limit. Smaller caches can increase CPU work.
12. Menu blur: reuse bind groups and skip the identity radius-zero chain. Filtering, output resolution and pass order are unchanged. Slower unrolled/compute shader experiments were rejected; GPU blur cost remains unresolved.
13. Touch crouch (PR #75): cancel the touch/controller toggle on entering water, flight or a mount; preserve held-key and vanilla-toggle handling.
14. Connection diagnostics (PR #79): always record connection failures and departures without enabling per-frame network logging. Reports may contain player/server details.
15. Fixed opt-in Android mods: Zoom, Minimap, Recipes and SmartF3 compiled into the APK, with local Enable/Disable in the Mods catalog and settings under Options > Mods. Defaults are off; choices persist per profile and runtime state resets between launches. Original MIT licenses and pinned source provenance are retained.
16. Existing sandboxed `.hmod` execution remains available on compatible 64-bit builds. Arbitrary `.hnmod` compilation/loading is not added. Existing Horizon far terrain remains; Distant Horizons is not bundled.
17. World transfer: selected-profile saves exposed as Worlds in Android Files. Complete folders can be copied for import/backup. Finish copying before Play; new world-file access is refused while playing. This does not add a ZIP-world wizard.
18. Launcher Settings > Game files opens the provider root; World files opens saves directly. Private accounts, keys and internal traces are excluded.
19. Upstream synchronization: 1,846 commits, including merge history, to the selected nightly. Areas include authoritative item use/prediction, server connection lifecycle, shared block-state storage, native hooks/content APIs, keyboard focus, shader options/LabPBR, graphics memory handling and rendering. Upstream deployment workflows remain disabled in the Android fork. Individual upstream commits are not all independently device-tested.
20. Android integration preserves launcher-selected rendering, system DNS, native Activity/IME/SAF, lifecycle recovery, memory allocation policy, Android GPU compatibility mode and release optimization profiles.

## Files and profiles

Normal installed package: `dev.horizon.android`.
The physical internal directory is normally `/data/user/0/dev.horizon.android/files`.
Profile 0 uses this directory; additional profiles use `profiles/profile-<id>` beneath it.
The game keeps `saves`, `horizon-mods`, `resourcepacks`, `shaderpacks` and settings in that profile.
Prepared engine resources and cache are application-managed; do not move them by hand.

Use **Settings > Game files** or **Android Files > Horizon Android** for supported document access.
The provider shows the selected profile's Mods, Resource Packs, Shader Packs and Worlds,
plus Reports. It deliberately does not expose every private file in the physical directory.
Screenshots remain under Pictures. Switching profiles must not mix world or mod choices.

## Issue disposition for this candidate

A code change or a merged PR is not equivalent to a closed and verified issue.
No new GitHub issue is automatically closed by this checklist.

| Issue | Candidate change | Verification boundary |
| --- | --- | --- |
| #55, duplicate #30 | Bounded installer cache-eviction recovery | Host recovery tests and a full isolated tablet installation with deliberate object eviction passed before nightly; repeat final APK installation |
| #52 | Block-editor native IME routing | Command/structure/jigsaw typing, backspace and Done observed on F-03L before nightly; repeat final APK |
| #71 (already closed), PR #75 | Crouch-toggle transitions | Input unit tests; physical water/flight/riding checks remain in checklist |
| #72 (open), #84 (already closed) | Worlds exposed through Files | Non-overwrite/path/symlink tests and real F-03L folder listing; cross-client transfer still requires validation |
| #57 | Early navigation, deferred GLES world initialization and failed GPU-readback retirement | Specific paths tested; not a blanket fix for all low-end crashes, TV controller stalls or FPS drops |
| #63 (already closed) | Cache/worker choices and derived-cache invalidation may be relevant | No general FPS-regression resolution established |
| #31 (already closed) | Pet-owner protection already existed in 0.7.6 | Not a new 0.7.7 fix; following an attack target needs gameplay verification |
| #74 (already closed), PR #79 | Improved disconnect logging | Diagnostic improvement; does not itself fix connection handling |
| #82 | New import report reviewed | Not reproduced or declared fixed; exact mod format/log required |
| #83 | Startup crash and differing-build comment reviewed | No crash report or precise failing-build identity yet; unresolved |
| #76 | DeX mouse buttons opening chat | Needs DeX/external-display reproduction; unresolved |
| #78, #80, #81 | Missing eyes, mobs through glass, villager particles | Reviewed; no claim of an Android fix |
| #77, #69, #70 | Combat/use, chest minecart, IME overlap | Need targeted gameplay/device verification; not declared fixed |
| #10, #22, #24, #33, #53, #54 | Renderer-specific glitches/crashes | No blanket closure from the Adreno 304 workaround or newer nightly |

## Complete manual regression checklist

Use a backup or a disposable profile. Record APK/commit, device, renderer, resolution,
render/simulation distance, enabled mods/shaders and whether diagnostics are on.

### Install, launcher and lifecycle

- Install over 0.7.6 without clearing app data; verify accounts, options, worlds and touch layouts survive.
- In a fresh profile download Minecraft, cancel/resume, retry a failed network request, and launch after completion. Verify no stuck finalization or unexplained OS error 2.
- Launch while offline after preparation. Retry catalogs when network returns.
- Verify title/launcher text, button press feedback, caret, keyboard search and scrolling.
- Test Back while editing, with a chooser, on the home page; Cancel keeps app open, Exit exits.
- Navigate title/menu immediately with D-pad Down/Up/Enter before waiting for resource completion; repeat with a controller if available.
- Idle at title, open settings, enter a world, leave to title, return to launcher and Play again.
- Home/resume, screen-off/resume, notification shade, rotate, resize/aspect and IME open/close; check no crash, permanent slowdown or missing HUD.

### Input and touch

- All four touch schemes, crosshair and touch-target modes; gesture setting on/off.
- Tap entity attack versus hold-use; ordinary block placement/mining; boats/eggs, flint-and-steel, fire charge and bone meal.
- Food, bow/crossbow/trident, shield, fishing rod and fireworks/gliding; camera movement while holding an action.
- Crouch toggle on ground, enter water, fly, mount, dismount; ordinary physical Shift and vanilla toggle remain independent.
- Custom buttons, held combinations, F3, F5, swap hand, shield, sprint and layout editing; no clipped/overlapping defaults.
- Virtual mouse hover/tooltips, relative movement, tap/hold clicks and dedicated LMB/RMB; inventory controls on/off.
- Physical keyboard/mouse and controller when available; Auto/forced input mode. DeX needs its own hardware test.
- Chat, server address, sign, book, anvil, creative search, recipe-book search, command block, jigsaw, structure/test blocks: type, erase, move focus, Done and Back.

### Files, packs and mods

- Game files opens Horizon Android root, World files opens the selected profile's saves.
- Copy a complete disposable world out, import under a different name, open in a compatible client; verify inventory, location, entities and block edits.
- Existing folder name does not silently overwrite a world; cancel copying; finish transfers before Play.
- Accounts/credentials are not visible in provider; traversal/symlink access is rejected.
- Import valid resource ZIP, shader ZIP/.hshader and supported .hmod; Cancel picker; reject malformed/wrong-format packages with a useful result.
- Install/remove from catalog and Installed; verify status, progress, replacement confirmation and profile isolation.
- Enable each bundled mod separately: held-Z zoom, minimap, Recipes recipe/usage panel, SmartF3. Test all together, disable, restart and switch profiles.
- Reload resource packs in a world and title; check loading feedback, no ANR loop, fonts/texture changes and sound restoration.
- Test native .hshader and a previously working classic ZIP shader; enable/disable/reload without stale textures.

### Rendering, worlds and performance

- Vulkan Compatibility and Batching separately; GLES where supported. In-game API selector follows launcher and cannot contradict it.
- In Batching, stay still for at least 30 seconds, turn through every direction,
  walk across chunk boundaries and alternate open/occluded views. Foliage, ground
  and cutout surfaces must remain present as GPU culling probes or changes paths.
  Repeat with shader packs and at distances below/above the batching threshold.
- Legacy Adreno: opaque terrain, glass, water, leaves, cutouts, torches/emissive blocks, chunk edges, fog and moving camera; no transparent blocks or stripes.
- Compare dawn/sunset/sky, HDR emission and texture filtering; no quality downgrade accepted.
- Test spawn stationary for 60 seconds, walk, water/coast, vegetation, mountains, inventory and blurred options. Compare like-for-like 0.7.6 with diagnostics off.
- Default worker/cache settings versus explicit limits; no forced reduction of render distance or texture quality. Smaller cache may trade RAM for CPU.
- Create/save/reopen worlds; inventory/location preserved; change dimension and join/leave servers.
- Multiplayer pickup, melee/ranged hits, item use, teleport/riding reset; record exact disconnect reason for failures.
- Particle effects, mob textures/eyes and mobs behind glass at distance remain specific open regressions to check.
- Performance recording plus GPU profiling: readable reports, bounded recording, export available through Files; inspect privacy before sharing.

## Release gates

For the optional dynamic joystick, test both joystick schemes with the option
off and on. With it on, touch inside and outside the configured width/height,
drag and release, then repeat with the left-handed layout. The frame should stay
at the initial touch, movement should stop on release, and a second finger should
still turn the camera. Other buttons retain priority. Hide the movement control
and confirm the area no longer starts movement. D-pad behavior must stay unchanged.

ARM64 compilation alone is insufficient. Require clean merge markers, formatting, lockfile,
public-tree/license audit, Android checks, targeted host/GPU tests and a final device smoke test.
Check ARMv7 separately; report untested x86/x86_64 instead of inferring success.
Do not present emulator/probe FPS as phone FPS or mark unresolved issue families as fixed.

## Verification recorded during nightly integration

- ARM64 Android Clippy passed; optimized fast ARM64 and ARMv7 builds completed.
- Fixed mod runtime/mod-list tests: 27 passed.
- Content/storage/import tests: 36 passed, one resource-fixture-dependent test ignored.
- Android input/navigation tests: 26 passed.
- Launch contract/preferences: 4 passed.
- GPU arena/upload/readback and actual terrain/sunrise rendering: 29 passed, including the exact white-overlay lookup and pixel equality for the original and packed terrain shaders with a colored-light case.
- Isolated account/skin/session tests: 55 passed, one ignored; no real accounts used.
- Public-tree audit passed before final staging.
- Tablet Vulkan Compatibility: entered an existing disposable world, rendered terrain,
  opened inventory, returned from Home to a paused game, quit back to the launcher,
  and opened the five-folder Android Files root through Game files.
- F-03L GLES: the remaining startup driver exit was isolated to the dynamic
  white-overlay palette index in the cloud vertex program. Constant-index lookup
  preserves all sixteen exact values. The standalone failing GLSL pair links
  after that change. A normal source-built diagnostic APK (no binary patches or
  exit hook) reached launcher, title, world-renderer readiness and joined its
  integrated server. The normal source-built APK rendered opaque terrain, trees
  and water in an open landscape view on the device.
  World startup and runtime are very slow on this device; performance regression
  remains unresolved and this is not a four-ABI release sign-off.
- x86/x86_64 and a final ThinLTO release APK have not been built in this pass.
- Sprint regression: in the isolated tablet world, controller/touch action held Ctrl across
  simulation frames, ground forward velocity was 1.300000 times the walk velocity,
  and release removed Ctrl. Physical-key/touch combinations still need manual testing.
- No matched release-to-release performance benchmark was completed in this pass.

## Candidate regressions reported after initial ARM64 testing

The owner reported a nonfunctional Sprint button and worse GPU performance on
commit da0b893b9. The candidate must not be released until these are resolved.
The initial missing-world report was withdrawn after the owner checked again;
no world relocation or migration is needed. The two supplied reports both identify that commit and Vulkan
Compatibility at 100% resolution. They are different devices/scenes, not a
matched 0.7.6/0.7.7 benchmark. CPU time alone does not account for the GPU-limited
samples with no outstanding chunk jobs. Do not silently relocate/delete worlds
or infer data loss from an empty list; establish the old and current paths first.
