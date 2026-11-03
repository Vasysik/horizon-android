# Android 0.7.2 upstream integration

Reviewed 69 stable `upstream/main` commits after `79549027`, through
`b6c936f82b08798057161bdfa577e3aac53fb91e`.

- Performance: avoid loading loot tables on a remote client's first tick,
  calculate container minecart signals only when a detector rail reads them,
  and release unused sprite pixel-buffer capacity.
- Gameplay/UI: remote eating sounds and subtitles, creative off-hand and search
  fixes, book click actions, resource-pack prompt focus, beacon distance,
  armadillo reactions, Riptide countdown and advancement triggers.
- Rendering/input: own and slim skins, blindness/portal/nausea effects,
  high-contrast outlines, extended keyboard bindings and vanilla mouse options.
- Maintenance: TLS verification dependencies, Cyrillic font glyphs, upstream
  license attribution and harness/documentation changes.

The HUD merge conflict preserves Android safe-area margins around saving/toast
overlays and upstream's suppression of book/sign/dialog layers while a server
resource-pack prompt is open. Android runtime bridges and launcher remain in
place. No upstream deployment workflow was enabled.

Android release-profile Clippy and source checks cover compilation/integration.
Upstream's full gameplay parity suite and complete release APK device testing are
separate; this integration does not establish an FPS improvement on a phone.
