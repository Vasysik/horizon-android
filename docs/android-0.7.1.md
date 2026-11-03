# Android 0.7.1 source changes

Changes relative to the published Android 0.7.0:

- Retry Android keyboard activation after the native text editor receives window
  focus, in both the launcher and game. The isolated Rust input probe was checked
  on Poco; the complete APK still needs device validation.
- Add a persistent client-resolution setting with presets and numeric input.
  The renderer scales the complete frame, including menus and HUD. At 100%,
  rendering uses the existing direct presentation path.
- Preserve GUI, font and touch pixels when fitting the mobile atlas. Reuse gaps
  with skyline packing; reduce panorama faces separately on the GLES path.
- Create an empty custom touch button, then select its game action or keyboard
  key from a paged list.
- Add optional resource-pack hooks for touch button artwork. Existing artwork
  remains the default.
- Accept reissued official version metadata when both pinned game JAR hashes
  match; retain catalogue and downloaded-content verification.
- Merge 68 reviewed upstream commits through `1feb28c0`, including input-crate
  extraction, mob sound/state and gameplay fixes. See the
  [integration review](android-upstream-review-2026-10-28.md).

ARM64 release-profile Clippy, targeted Rust checks and the ten Android Python
tests passed during development. These are source checks; no performance
improvement or complete APK device-validation result is claimed for this update.

Build the public ARM64 APK with `python tools/android_build.py --abi arm64-v8a --public`.
The package uses versionName `0.7.1` and versionCode `22`. Keep the signing key
used for 0.7.0 so the resulting APK can update an existing installation.
