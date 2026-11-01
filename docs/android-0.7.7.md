# Horizon Android 0.7.7 development

This version is in development; no device-wide performance improvement is established.

- Reuse menu blur bind groups until the target size changes, instead of recreating six per frame. Filtering, resolution and pass order are unchanged.
- Skip the identity blur chain when the radius is zero.
- Adreno 304 investigation: removing terrain fade discard alone did not fix transparent terrain. Removing alpha discard as an additional private probe did not restore correct rendering either. Neither shader experiment is retained.
- The 0.7.6 owner report shows substantially higher GPU time in blurred menus. These are observational, mixed-transition samples, not a controlled benchmark.
- The Mali title-screen ANR from RC1 resolves to Renderer::draw_frame -> Queue::submit -> Mali GLES. It remains unconfirmed on the current build.
- Upstream tamed-pet owner protection (PR #4184, b80768cd4) is already included in Android 0.7.6. The other symptom from Android #31, following the owner's attack target, still needs an end-to-end test.

Turnip cannot supply Vulkan support for Adreno 304; Mesa excludes Adreno 5xx and earlier. Optional custom-driver loading for supported hardware remains a separate investigation.

## Launcher/game boundary

The native host still owns the Activity, event loop and the one Android window.
The frontend prepares an explicit `android/game_launch.rs::LaunchRequest` with
absolute resource/profile directories, identity/session-file reference, optional
server, renderer mode and development-only control arguments. `start(request)`
consumes a snapshot rather than rereading launcher config inside game startup.
Game construction failure restores the previous working directory; normal return
uses the existing launcher recovery path. Session tokens are not command arguments.
The current engine still depends on the process working directory; this is an
initial boundary, not an independently embeddable game or a Java frontend.

Back dismisses editing/dialogs first, then asks before exiting the launcher.
The confirmation defaults keyboard focus to Cancel.

GPU allocation policy remains the same as 0.7.6: the Android wgpu MemoryUsage
hint and 8 MiB soft page-growth cap remain active. No extra memory-policy toggle
is exposed; the briefly tested Standard/Small switch was removed before release.

The launch entrypoint receives its argument vector directly. Game startup no
longer reads launcher config, a launcher-owned session getter or a thread-local
argument queue. Prepared sessions remain state of the current Launcher instance;
sign-in/out invalidates them. Exit/Cancel remain available during preparation,
and completing a prepared session does not override an open chooser.

Validation: ARM64 Android check and Clippy passed; host launch/preference
tests passed. The isolated tablet build completed launcher → game → launcher
and displayed the compact Cancel/Exit confirmation. ARMv7 terrain validation
is described below.

Tablet menu measurements still show roughly 31 ms GPU time with radius 5 blur
versus 12 ms at radius 0. Bind-group reuse alone does not resolve this GPU cost.
These short diagnostic runs used different native build profiles and are not
a release-to-release FPS benchmark.

## Installer cache recovery

Installer inputs remain in Android's disposable cache. Preparation now reopens
the index, client JAR and each asset through bounded missing-file recovery,
redownloading an evicted input with its existing size/SHA-1 verification.
An open source handle remains alive during extraction/copy. Download retries
recreate missing cache directories, and failures identify the affected input.
This does not change worlds, accounts, the installation destination or cache
cleanup after success. Persistent storage exhaustion still needs free space.

Four cache-recovery regression tests and the Android content harness passed
(31 passed, one fixture-dependent test ignored), as did ARM64 check and Clippy.
The report for Android #55 records RC1 source `2a6dd4d80861`; the same vulnerable
cache lifetime assumptions remained in the final 0.7.6 installer. Its generic
error alone does not establish which particular input was evicted.

An isolated Adreno 710 tablet completed the official 5,148-file installation
with deliberate removal of one verified sound object after download, during
extraction. The missing object was restored, the installed file matched its
official SHA-1, the launcher offered Play and successful cleanup removed the
download cache. This verifies that eviction case, not every possible ENOENT.

## Block behaviour cache

Adapted the cache portion of Android PR #58: the resolved-behaviour cache has
a new schema and fingerprints the exact block-class variant order. This avoids
reusing serialized class ordinals after an engine update on Android, where
`current_exe` identifies the system's app_process rather than the game library.
Crate version 0.1.0 is not treated as the Android application version. This
change rebuilds derived tables; it does not rewrite world saves.

## Native text input in block editors

The shared Structure/Jigsaw/Test block editor now participates in IME focus,
initial text synchronization and committed text/key routing. Command-block OSK
input is dispatched before inventory searches. Existing per-field character
validation and confirmation rules remain in the editors.

On the isolated Fujitsu build, Android input reached the command field,
structure name and jigsaw pool; Backspace and Done worked, and the edited
jigsaw pool remained after closing and reopening. ARM64 check/Clippy and the
optimized ARMv7 build passed.

## Legacy GLES terrain

The GLES terrain shader packs scalars with matching interpolation qualifiers
into five varying locations instead of nine. On the Fujitsu F-03L's Adreno 304,
the compiled source now renders the previously transparent terrain: stone,
sand, foliage and the jigsaw block were visible in the isolated test world.
This is a driver workaround, not a general FPS improvement.

Vertex buffers, texture sampling, lighting, fog, fade and HDR emission markers
retain their existing calculations. Vulkan receives the original shader source;
sky and outline shader bodies remain unchanged. GPU readback tests compare
original and packed shaders byte for byte for RGB, fog, fade and emission tiers.
All 23 isolated GPU tests passed on the host GPU; ARM64 Clippy and the optimized
ARMv7 build passed. Other GLES devices still require testing.

## Optional CPU and reusable-cache budgets

Launch settings offer a chunk-worker ceiling (1–64), disabled by default.
It caps the engine's chosen generation/meshing pool; it does not increase a
smaller pool or cap render, audio, server and all other process threads.
Changes apply on the next Play. Existing automatic selection is unchanged
when no additional ceiling is selected.

The reusable RAM-cache setting defaults to the existing budgets: 16 MiB
for empty mesh buffers per streaming pool and 64 MiB for decoded sounds per
audio cache. Custom totals split 20%/80% between these categories. Eviction
releases reusable buffers and least-recently-used sound entries; playing
channels retain their own sample references. Active terrain, textures, worlds
and sound playback are excluded. This is not a total RAM or VRAM cap.
Smaller caches retain the same output quality but may require more CPU work.
Unused GPU arena pages already return to the allocator after upload batches;
no artificial live-VRAM cap is added.

## Early-menu D-pad navigation

Standalone startup frames publish their widget targets and combine enabled
labelled buttons with explicit fields/list targets. Previously early focus
could miss ordinary title buttons until Game took over.
Host tests cover replacement of standalone focus targets and default/custom
worker and cache policies. Early Android controller D-pad/confirm events are also consumed during loading,
instead of being ignored until Game takes over. Physical-controller startup
stalls in Android #57 still need separate device verification.

On GLES, world-only renderer creation remains on the event thread, but now
waits for a world/server request and a presented loading frame. The title menu
no longer freezes automatically to create unused world pipelines. This moves
existing GPU initialization to explicit world loading; it does not make shader
compilation itself faster. Vulkan retains background renderer preparation.

## GPU timestamp readback failure

A failed mapped-range read no longer unmaps a destroyed readback buffer.
Mapping/readback failures disable that timer, preventing reuse of its dead
slot on later frames. A GPU regression test completes a real async mapping,
destroys its buffer, collects the result and attempts a subsequent frame.
All 24 isolated GPU tests passed. This addresses the validation-panic path
reported on Adreno 610; it does not establish why the driver/device lost the
buffer, nor prove every crash in Android #57 has the same cause.

On the isolated F-03L build, D-pad Down followed by Enter opened the title's
Credits screen before world-renderer creation. Selecting the test world then
showed a loading overlay while GLES initialized. These checks do not cover an
Android TV controller's analog-stick navigation or vibration.
