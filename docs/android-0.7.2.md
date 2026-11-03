# Android 0.7.2

Changes after 0.7.1:

- Reset riding state when the server replaces the local player, including proxy
  lobby transfers in the same dimension.
- Initialize menu audio and the FPS/frame graph before full game startup, and
  preserve them when handing over to the game.
- Browse the official Horizon Mods catalog, retaining direct HTTPS `.hmod`
  imports. Native mod execution requires a 64-bit APK.
- Show per-project download progress, installed state and retry actions in mod
  and resource-pack search results; provide launcher button press feedback.
- Use the upstream Horizon application icon.
- Add a launcher icon, text-field carets, offline player-name status, client
  version and a GitHub link. Catalog tabs remain Mods/Packs, with Horizon Mods
  and Resource Packs section headings.
- Move Add buttons below the editor toolbar and draw editable controls above
  the toolbar. Improve launcher content insets and footer spacing.
- Reset the Android inactivity timer when world loading completes.
- Add opt-in detailed performance reports with frame/server/worker timings,
  slow-frame events, streaming queues and renderer statistics, exported locally
  to Downloads. See [recording instructions](android-performance.md).
- Integrate reviewed upstream history through `b6c936f8`; see the
  [upstream review](android-upstream-review-0.7.2.md).

The public package uses versionName `0.7.2` and versionCode `23`. Build with
`python tools/android_build.py --abi arm64-v8a --public`; substitute
`armeabi-v7a`, `x86` or `x86_64` for other architectures. Keep the existing signing
key so users can update without uninstalling.

This remains experimental. Initial world-loading frame-rate drops are under
investigation; no FPS gain is claimed. Newly built APKs need device validation,
including sound and server transitions. Java Fabric/Forge/NeoForge mods are not
supported.
