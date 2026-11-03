# Contributing to Horizon Android

This is an unofficial Android fork of Horizon. Start with small changes and keep
the upstream attribution and history intact. Read [CONTRIBUTING.md](CONTRIBUTING.md)
and [AGENTS.md](AGENTS.md) for the engine's rules.

Useful contributions include touch input, Android lifecycle/save recovery,
accessibility, native hardware controllers, resource import/export and profiling
on real phones. For rendering or performance reports, include the phone, Android
version, renderer, resolution scale, render distance and APK/source version.
Emulator FPS does not establish phone performance.

[Report an Android bug](https://github.com/Vasysik/horizon-android/issues/new?template=android-bug.yml)
or send a focused pull request against `android`. Reproduction steps and a short
video often help more than a large log dump. Do not post account/session files,
tokens, private server details, signing keys or complete app data.

See [Android build instructions](docs/android.md#build-developers-only) for the SDK/NDK setup.
Game inputs must be prepared from your own Minecraft installation and stay in
`.local/`. Build products and generated game data never enter Git.

```sh
python tools/android_build.py --abi arm64-v8a --clippy
python tools/android_build.py --abi arm64-v8a
python -m unittest discover -s tests -p test_android_release_audit.py
python tools/check_public_tree.py
```

Touch preferences and the HUD editor live in `crates/horizon-client/src/touch*`;
standard options widgets in `options_ui.rs`; the Android entry point and launcher
in `android_main.rs` and `android/`; game bindings in `game/pad.rs` and
`game/android_input.rs`. Add focused regression tests for input/state changes.

For upstream fixes, review and merge `upstream/main` rather than copying files.
Keep deployment workflows inactive and update `docs/android-upstream.json`.
Never merge experimental branches automatically.

`--release-candidate` produces a non-debuggable local APK, remaps personal build
paths and checks known credential patterns and packaged data paths. This checks
privacy, not permission to redistribute bundled Minecraft resources. Use
`--public` for distribution: it filters the generated table bundle, downloads
original world data/textures from Mojang on the phone, and disables automatic
agent control. See [publication notes](docs/android-publication.md).
