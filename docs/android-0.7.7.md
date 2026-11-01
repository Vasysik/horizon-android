# Horizon Android 0.7.7

## Changes

- The launcher is redesigned in a Material 3 Expressive style. Wide screens get a
  navigation rail; phones held upright get a bottom navigation bar. Pages use
  rounded cards, the Play page has a hero card and a side panel with the account,
  the version and a large launch button. Settings are a list of rows with leading
  icons, Material switches and chevrons; choosers use radio rows; the account
  dialog sits on a scrim. Icons are the design system's pixel icons
  (`horizon_theme::icons`), drawn by `android/launcher_icons.rs`.
- Every launcher action, target and controller/keyboard focus path is unchanged.

## Validation

- `tools/android_build.py --abi arm64-v8a --check --public` and `--clippy` pass.
- The icon rasterizer was checked on the host against every bundled icon.
- Not yet checked on a phone: layout at every screen size and density.
