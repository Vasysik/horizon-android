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

Experimental smaller GPU allocations are now opt-in in launcher Setup. Missing
settings default to Standard, including upgrades from 0.7.6. The choice applies
at the next game start; restart the app to change the launcher's own GPU device.
Both the wgpu MemoryUsage hint and 8 MiB soft page-growth cap follow this choice.

The launch entrypoint receives its argument vector directly. Game startup no
longer reads launcher config, a launcher-owned session getter or a thread-local
argument queue. Prepared sessions remain state of the current Launcher instance;
sign-in/out invalidates them. Exit/Cancel remain available during preparation,
and completing a prepared session does not override an open chooser.

Validation: ARM64 Android check and Clippy passed; five host launch/preference
tests passed. The isolated tablet build completed launcher → game → launcher
and displayed the compact Cancel/Exit confirmation. ARMv7 compiled, but its
terrain rendering remains under investigation.

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
