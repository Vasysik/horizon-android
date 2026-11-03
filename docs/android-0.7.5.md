# Horizon Android 0.7.5

The published RC is followed by source changes for the next 0.7.5 candidate.

- Native-mod upstream synchronization and Android compatibility fixes.
- Always-available menu virtual mouse: relative touchpad movement for menu tooltips, adjustable sensitivity, tap for LMB and hold for RMB. Dedicated LMB/RMB buttons appear at the lower right of menus while the virtual mouse is active; this does not enable gameplay controls. Tap clicks LMB and a stationary hold clicks RMB without a separate option.
- Short world taps attack entities without holding the attack button into the next tick. Usable items wait for a long press; ordinary blocks retain tap placement and hold mining. Item classification uses the engine use actions, components, crop classes and sandbox item claims.
- Four existing touch schemes are retained. An independent crosshair-gesture option allows touch taps and holds to target the center in either crosshair movement scheme.
- Gyroscope aiming: opt-in, adjustable sensitivity, display-rotation axis mapping, and sensor shutdown on focus loss (within a bounded 20 ms worker poll). A device without a gyro remains playable. Gyro input applies only to active touch gameplay.
- Preferred display refresh rate: a native `ANativeWindow_setFrameRate` request on Android 11+, with System default restoring the OS choice. It is a preference, not a guaranteed display mode and not the game's FPS cap.
- Optional ADPF performance hints on supported Android 12+ devices. A session tracks the rendering thread's actual CPU-side frame work including submission waits. It uses the explicitly requested display period, or 60 Hz when System default is chosen. Hints close on focus loss. There is no guaranteed performance gain or change to graphics quality.
- Detailed GPU profiling: recording switch plus rolling p50/p95/p99 for preparation/uploads, world, and post/hand/HUD stages, correlated with CPU/streaming and native API diagnostics. Timestamp availability depends on the driver. These are not vendor hardware counters, GPU clock readings, or proof of a slowdown's cause. Profiling has overhead and is off by default.
- Exit reports: Android 11+ historical process exits with OS reasons and available Java/native/ANR traces; a previous-log tail and Rust panic export. Traces are bounded and retained locally. Native crash traces can be binary tombstones. Older firmware or the OS may provide no trace.
- Screen presentation: optional immersive system-bar hiding, camera-cutout avoidance and Native/16:9/4:3/1:1 fitting. Rendering and absolute input share physical content bounds; portrait swaps the ratio. These settings belong to the launcher.
- Launcher recovery: regenerate pending text/corner atlas uploads when Android skips a frame during surface suspension or loss, preventing missing labels after unlocking.

Virtual mouse, gyroscope and crosshair gestures are in Options > Controls > Touch Controls. Display refresh rate, Performance hints (ADPF), GPU profiling and existing performance recording are in launcher Settings. Earlier system preferences are migrated from the touch file without overriding choices already saved by the client. Sensor/performance controls default to off and display frequency defaults to System default.

Reports are accessible through the Android Files app under Horizon Android > Reports. Native exit history appears after the next launch. Review logs before sharing: they can include file paths, server addresses and exception text. No reports are uploaded automatically. Normal Android low-memory process termination is recorded as such; it is not automatically labeled a game crash.

Device support is detected at runtime. Old Android releases retain normal startup without linking newer performance/frequency symbols at load time.

Sandboxed `.hmod` execution is retained in this Android fork alongside the upstream native hooks. Source-only `.hnmod` entries remain visible in the catalog with an unsupported status; compiling and running them on Android is deferred. Native-engine experiments are excluded from this release.

## Release scope and verification

Skin/cape management and native `.hnmod` engine experiments are excluded from this candidate. The existing four schemes, resource-pack imports, Android storage integration and renderer selection remain available.

Source checks and focused input tests cover gesture ownership, cancellation, mouse dragging and settings migration. A complete on-device playthrough of the final source is still required before publication. Compilation checks are not APK startup tests or a performance benchmark.

The screen adapter and viewport changes pass ARM64/ARMv7 source checks and standalone geometry tests. The minimal x86_64 screen probe builds, but this host's emulator exits with Windows error `0xC0000005` before the application can run; native window behavior and final game presentation still need device testing.

Immersive fullscreen defaults to on for missing screen preferences; explicitly
saved Off choices are retained. The status bar stays hidden in both modes. Off
shows only Android navigation, with its occupied area excluded from the picture.
Microsoft device-code polling retains the active code through temporary network
outages, with bounded request retries, backoff and service/error categories.
The configured `HORIZON_MSA_CLIENT_ID` is honored when supplied; the upstream
registration value is not distributed in source and has not been substituted.

Custom physical-key touch buttons now send sandbox `.hmod` key-down/key-up events, matching hardware keyboard dispatch. Entity-placement items (boats, rafts, minecarts, spawn eggs and decorations) retain tap placement; contextual holds include boats and minecarts.
