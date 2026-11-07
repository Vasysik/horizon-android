# Oxide Android 0.7.7

This update rebuilds the launcher around a bottom-docked control bar and adds
MP4 live wallpapers.

## Launcher layout

- **Navigation moved to the bottom edge.** Play, Mods, Packs, Shaders, Files and
  Setup are one bevelled tab strip along the bottom, with the active page lit
  grass-green like a chosen inventory slot. The bar replaces the top tab row.
- **Play sits at the bottom left**, in a dock above the navigation bar, where a
  thumb reaches it first. On every other page it returns to Play; on Play it
  starts the game or continues an installation.
- **A live opacity slider sits at the right of the dock.** Dragging it repaints
  only the launcher's own widget layers, from 30% to 100%, and the value is
  saved. The wallpaper keeps full brightness, so the control shows its own
  effect. The Settings row of the same name still steps the value for anyone
  using a D-pad or keyboard. On a screen too narrow for both controls, Play
  keeps the dock and the slider is hidden rather than overlapping it.
- Page content, list viewports and the status line now end above the dock, so
  nothing scrolls underneath the controls.

## Live wallpaper: GIF, APNG and MP4

- The picker now offers **video clips as well as animated images**. `Action::Wallpaper`
  accepts MP4 (`.mp4`, `.m4v`, `.mov`), GIF and APNG.
- **MP4 is decoded by Android's own media stack**, so no video codec is bundled
  and the APK does not grow by one. The file is staged in app storage (a
  `content://` URI has no filesystem path) and sampled at a bounded rate and
  size: at most 48 frames, a 30 s clip and a 640 px long edge, so a long or
  high-resolution video cannot exhaust memory on a phone.
- **The format comes from the file's bytes, not its name.** Android content
  providers return whatever display name they like; a GIF renamed `.mp4` still
  imports, and a file whose name says nothing is sniffed from its header.
- APNG support comes from the same animated-image path as GIF.
- Only one wallpaper is kept: selecting an MP4 removes the saved GIF and the
  reverse, so the launcher never loads a stale file.

## Limits

- MP4 support depends on the device's media stack having a decoder for the
  clip's codec (H.264 is universal; HEVC and AV1 are not). A device without one
  reports the failure in the status line instead of showing a black launcher.
- MP4 audio is ignored: a live wallpaper is silent.
- Frame limits are unchanged for animations: 48 frames, 16 MiB per GIF.
