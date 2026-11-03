# Android launcher languages

In **Setup / Settings → Launcher language / Language**, choose **Follow system**,
**English**, or **简体中文**. The preference is saved in `android-launcher.json`
(`language`: `system`, `en`, or `zh_cn`). Existing installations default to
following the system, and unknown future values fall back to that preference.

The first configured Android language determines the system choice: Simplified
Chinese (`zh-CN`, `zh-SG`, `zh-Hans`) uses Chinese; other languages use English.
Explicit Traditional Chinese (`zh-Hant`, `zh-TW`, `zh-HK`, `zh-MO`) uses English
until a Traditional Chinese catalog is available. A manual choice overrides the
system. Bringing the launcher back into focus refreshes the system preference.

Changes take effect immediately, including existing launcher status messages.
The in-game language remains a separate per-profile option in **Options →
Language**. Player names, server addresses, filenames, third-party descriptions
and unknown backend diagnostics retain their original content.

The native Android launcher uses `android/i18n.rs`, following the desktop
launcher's English-source-to-translation JSON convention without depending on
the desktop GPUI workspace. Its catalog is compiled from
`crates/horizon-client/assets/android/lang/zh_cn.json`; missing translations fall
back to the English source. Named placeholders are substituted in one pass, so
user content containing braces is not interpreted a second time. `Message`
keeps background-operation text and arguments until display, allowing a message
to change language without restarting its operation.

Both the catalog and Noto Sans SC fallback font are compiled in. No game asset
download is needed to read Chinese on first launch. Text measurement, drawing,
truncation and wrapping use the same selected font. The static Chinese font adds
about 10.6 MB before APK compression; it retains all 30,890 mapped characters,
including characters needed for player names and filenames. Its license and
reproduction instructions are in `THIRD_PARTY_NOTICES.md`.

## Validation

```sh
python3 tools/test_android_i18n.py
cargo test -p horizon-client --lib --no-default-features ui_theme::
cargo clippy -p horizon-client --lib --no-default-features --no-deps -- -D warnings
python3 -m unittest discover -s tests -p test_android_release_audit.py
python3 tools/check_public_tree.py
```

Android compile checks require an SDK/NDK installation:

```sh
python3 tools/android_build.py --abi arm64-v8a --check
python3 tools/android_build.py --abi arm64-v8a --clippy
```

On a phone, verify Chinese on the initial install screen before downloading game
assets, switch all three choices, restart, change the system language while
following it, and check portrait/landscape settings and account dialogs. Confirm
that switching the launcher language preserves the game's language and saved
profiles. Host tests cover locale resolution, JSON storage, fallback,
placeholders, live status translation, glyph coverage and CPU text layout; they
do not substitute for Android JNI and lifecycle testing on a device.
