# Android screen probe

A standalone Rust NativeActivity probe sharing the production system-bar/cutout
adapter and the patched winit coordinate transform. It links no game and uses no
player data. SAF registration is stubbed; it does not test document callbacks.

Build this manifest for an installed Android Rust target with the NDK linker.
Package `libhorizon_screen_probe.so` using stock `android.app.NativeActivity`,
`android.app.lib_name=horizon_screen_probe`, `android:hasCode=false`, a distinct
application ID (`dev.horizon.screenprobe`), target SDK 35 and minimum SDK 28.
Keep APKs and build outputs in `.local/`.

Every eight seconds it advances through native, 16:9, 4:3 and square fitting;
phases 1-7 enable immersive bars, 4-7 avoid cutouts, and 6-7 use 50% logical input
resolution. A software buffer shows a blue content area, white border and black
unused space. `HorizonScreenProbe` log messages report phase, rectangle, logical
size and touch coordinates. Test rotation, an emulated cutout and taps at the
content center and in black bars. Center touches must map to the logical center;
new touches in bars must not reach the app.

This checks native window APIs and input transforms. It does not exercise wgpu,
HDR, shader packs, game screenshots or real-device camera cutout firmware.
