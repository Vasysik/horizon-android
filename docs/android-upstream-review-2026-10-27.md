# Upstream integration, 27 October 2026

The integration covers 849 commits from `5ba4793d` through `840d11a3` on upstream
main, including the 51 commits found by the latest fetch. It includes 701 changed
files. No experimental branch was merged separately.

Reviewed integration areas:

- Rendering: extended shader-pack stages, HDR metadata and resource formats.
  HDR and shader packs retain opt-in defaults. Android compatibility draws,
  supported-feature intersection, GLES and game/launcher renderer separation remain.
- Performance: remote chunk work off the render thread, world-data prefetch and
  parallel loading, centre-change requeue, collision edge scans, cached behavior
  hashes, hotbar/uniform/pose reuse and subtitle/debug text measurement.
- Gameplay/networking: collision and sprint prediction, inventory bundles,
  cooldowns, server dialogs, entity fluids, mobs, projectiles and world behavior.
- Mods: Wasmtime 48 requires Rust 1.95; API-level checks and client/server
  capabilities are integrated. The 32-bit builds still omit Wasmtime.
- Shared launcher backend: proxy-aware HTTP agents coexist with Android's Live
  device-code endpoint/cookies, secure storage and app-specific data paths.
- Android resolutions preserve touch settings when editing vanilla options,
  custom F3 scale tests, passive HUD margins, keyboard/mouse hosts and touch HUD.
  New creative search uses the upstream EditBox value for Android IME.

Prepared-resource output version remains 1. Upstream deployment workflows remain
inactive. Compilation and targeted checks validate integration; device performance
and newly added upstream features require their own runtime validation.

## Follow-up through fde2dfff

Reviewed and integrated 38 further main-branch commits. The command parser and
shared data/text types move into `horizon-command` and `horizon-client-core`;
old `horizon_client` import paths remain re-exports. Android's Cargo member,
NativeActivity workspace exclusion and patched winit are retained. Desktop
launcher instances and quick-play arguments remain separate from Android's
existing profile UI. Aquatic mob rules and parity-tool changes are included.
Upstream deployment workflow edits stay under the inactive workflow directory.

Android integration checks caught and fixed the fork scoreboard UUID helper
reference after its move into client-core. Both ARMv7 Activity hosts type-check
with the new crate split.

## 0.7.0 cutoff: 8155baa6

Integrated 655 additional commits after `94290b01` (307 changed files). The
upstream `horizon-items` extraction keeps the existing client re-exports, while
`Opt` now lives in `settings.rs`; Android touch and F3 options follow that move.
Renderer integration includes separate item/armour glint textures and vanilla
HUD visibility fixes. The Android reticle policy, passive HUD margins,
capability-based GLES base-vertex fallback and single NativeActivity lifecycle
remain in place.

Vanilla fixes include attack-speed effects, sprint input, player poses, chat
interaction, entity behaviour and block rendering. A Clippy boolean simplification
in the incoming enchantment code preserves its condition. Incoming desktop and
nightly publication workflows are stored inactive alongside the other upstream
workflows; the Android source check remains the only active workflow.

This commit is the upstream cutoff for 0.7.0. Later upstream changes belong to a
subsequent release rather than restarting the current validation cycle.
