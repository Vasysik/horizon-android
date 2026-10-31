# Horizon Android 0.7.6

This update targets touch controls, initial menu navigation and diagnostic recording.

- Optional **Swap hands button**, off by default. Its factory position is beside Inventory in landscape and portrait, mirrored for left-handed controls. Customized placements are preserved. Swap, Shield and custom text buttons share the same default frame and visible held feedback; custom artwork remains supported.
- **Raise shield with: Sneak / Button**. The separate button appears when a blocking item is available. Shield input selects the blocking hand without consuming the main-hand item or interacting with a targeted block.
- Tap-control schemes show a crosshair for bows, crossbows, tridents and throwable items. Their use gestures aim through the center; other items retain touch targeting.
- **Horizontal sensitivity** and **Vertical sensitivity** calibrate camera input independently. Equal physical drag distances now produce equal default turns on both axes, including portrait screens.
- Flint and steel, fire charges, bone meal, buckets and other surface actions use short taps. Food, charged weapons and throwable projectiles retain hold-to-use behavior. Ordinary building blocks keep tap-to-place / hold-to-mine.
- Touch, gyro and controller camera movement count toward the look-around tutorial. Completed tutorial stages remain saved across worlds; an unfinished stage may appear again.
- Hardware D-pad navigation works in the initial title menu before the full game loads. Focus acquisition retries after widgets become available.
- GPU profiling and performance recording share one report session. Both launcher controls use the same button style. Reports include the requested profiling mode and whether encoder timestamps are supported; unsupported hardware cannot provide these stage timings.
- Installation cleanup waits for active resource preparation to finish; the final asset index is saved from verified memory rather than reopened from Android’s evictable cache. This prevents a concurrent startup cleanup from deleting files needed by a reinstall (#30). The original device report does not include a log, so a clean installation on that affected device still needs verification.
- Stable upstream history merged through `c0b65f1d2` (571 additional incoming commits after RC1, including merge history): rendering, chunk work, resource packs, gameplay, network and mod API changes. Android input, presentation bounds, upload budgets and sandbox `.hmod` support are retained.

Native `.hnmod` compilation on Android remains unsupported. Java Fabric, Forge and NeoForge mods are not supported.

The #34 Mali-G52 startup report identified a 1024-layer hardware limit with 1291
sprites. Renderer construction now selects a supported per-array size and
regroups the already prepared mip bytes across existing arrays. Shader packs use
the same actual split. Texture resolution, mipmaps and colors are retained;
verification on the originally affected phone is still required.


## Release fixes

- Periodic CPAL device discovery runs on a worker, at most once per second even in the launcher. Normal frame ticks only poll its result; reopening an actually changed audio output still occurs when needed.
- Diagnostic heartbeat/logging skip a busy state lock; redaction, memory collection and writes run on the worker. Logger flush no longer waits for disk during rendering/startup. Reports exposes ZIP exports and performance JSONL, not private session logs or raw OS traces.
- Installer finalization calls cleanup while retaining its existing transaction lock, avoiding recursive mutex acquisition (PR #39). Metadata requests retry transient failures; completed files remain reusable.
- Integrated-server saves write the owner's state again. A dedicated-server compatibility flag must not suppress singleplayer inventory, position and experience persistence.
- Android prefers opaque surface composition; where unavailable, the final output makes alpha opaque without changing RGB or internal shader/HDR data (#37). Original-phone verification is still required.
- OpenGL ES selection also applies to the launcher. An interrupted first Vulkan frame triggers OpenGL ES recovery on the next launch, with an explanatory status (#38). This does not identify or repair every vendor-driver crash.
- GLES sprite selection preserves the newly added texture-filtering functions; assembled shader validation and DirectX translation pass. The storage probe follows the relocated file-lock module.
- Fireworks no longer enable the automatic projectile crosshair. Swap hands moves away from Sprint; only the original factory placement is migrated.

## Follow-up upstream integration

The additional 571 incoming commits include SRV handshake corrections, player saving,
movement/attack ordering, safer archive reading, item/model fixes and server-side
resource-allocation improvements. Android keeps sandboxed code `.hmod` support and
now validates upstream content-only `.hmod` packages with the shared package parser.
Native source `.hnmod` compilation remains unsupported.

The new boot-renderer interface and ZIP 9 API were adapted explicitly for Android.
Audio-device discovery remains off-thread; upstream device fingerprints reduce
unnecessary queries. Upstream deployment workflows remain disabled.

## Crash diagnostics

- Always-on bounded local diagnostic log, independent of performance recording.
- Current/previous process markers with build/ABI, last startup/game phase,
  a five-second memory heartbeat and handled game errors. An unclean marker is
  evidence of interruption, not proof of a crash: Android may kill a process.
- Android 11+ exit collection runs on a worker. Reports distinguish native/Java
  crashes, ANR and low-memory kills, retain OS memory samples and explicitly report
  unavailable/failed collection. Trace availability depends on Android.
- Settings > Diagnostic report exports a local ZIP and opens Reports. It includes
  bounded log tails, exit history, available panic reports and readable summaries
  of the crashed native thread (module/PC/Build ID). Raw tombstone memory/register
  dumps, accounts and worlds are not included. Redaction is best effort: review
  before sharing. Nothing is uploaded automatically.
- Release packaging keeps matching private native symbols under
  `.local/android/native-symbols/`, outside APKs and release attachments. Final
  Android libraries carry an ELF Build ID for matching native crash frames.

See [diagnostic collection and limitations](android-diagnostics.md).

## Validation

Local checks passed: 2 execution-role/player save tests, 45 touch tests (including layout migration and per-axis calibration), 8 installer tests, 5 crash-diagnostic tests, 9 performance/timing tests, 27 content tests (one fixture-dependent test skipped), 18 audio tests, 19 GPU tests, 26 renderer/shader tests and a standalone storage-probe build, 13 Android tooling tests, ARM64 Clippy and ARMv7 cargo check. Public-tree and whitespace checks passed.

The change is a release candidate until the device checklist below passes. Local source tests do not establish performance or compatibility on every phone.

See [the test checklist](android-0.7.6-testing.md).

## Post-RC1 allocator candidate

RC1 phone reports contain frame-thread `arena_buffer_create` spikes of 799.5 ms,
571.0 ms and 364.4 ms, nested inside geometry upload. Android now uses wgpu's
`MemoryUsage` allocation policy and an 8 MiB soft ceiling for normal terrain-page
growth. Large single meshes retain the hardware limit. Shader features, texture
quality and geometry are unchanged; desktop allocation policy is unchanged.

On an Adreno 710 tablet in Vulkan Compatibility at 2560×1600, Fancy and 12 chunks,
sequential runs of an isolated test world gave these results:

| Build | Maximum terrain-buffer creation | Reserved GPU memory after loading |
| --- | --- | --- |
| 0.7.5 | 578 ms | 1,024 MiB |
| 0.7.6 RC1 | 608 ms; 318 ms in a later return run | 1,024 MiB |
| Allocator candidate | 44 ms; 153 ms in a later run | approximately 548 MiB |

The scene was capped at 60 FPS; run medians were approximately 58.5–60 FPS.
These are individual sequential runs with audio disabled, not a phone benchmark
or proof of higher uncapped FPS. Mobs and time were not frozen in these loading
runs. Phone verification and older-driver compatibility remain required.

Short separate uncapped sky-view runs with recording off, recording on, and CPU/GPU
recording together did not reproduce the long allocation stalls. The combined
report contained all three GPU stages. This does not establish zero diagnostic
overhead; recording and GPU profiling remain opt-in.

Also fixed a stale Vulkan startup marker when a renderer reuses a device. Its
completion is now scheduled after that renderer's first submission even when
first-device startup logging has already finished. Two consecutive tablet launches
retained Vulkan Compatibility and cleared the marker.

Validation: the existing 20 arena/render GPU tests, a GPU timestamp test with
`MemoryUsage`, the startup-marker ordering test, an optimized ARM64 build and
ARM64 Clippy passed. The Fujitsu device was offline during the allocator follow-up; its legacy
GLES investigation is recorded below.

## Legacy Adreno GLES follow-up

Adreno 304 with driver V@269 crashes during terrain-program linking when RGSS
indexes a local four-element offset array. The GLES variant now spells out the
same four samples in the same accumulation order. Filtering modes, sample
positions and mip blending are preserved; Vulkan WGSL is unchanged.

An isolated native EGL probe reproduced SIGSEGV with the original shader and
linked the rewritten version successfully with one and eight sprite arrays.
The GLES source/entrypoint validation test also passes. An optimized ARMv7
private build on the F-03L now enters the title screen and an isolated world;
physical D-pad input activates a menu item and moves the player.

F-03L compatibility is still incomplete: terrain appears as thin or fragmented
geometry. Disabling sky, fog and cave culling, or explicitly rebasing section
instance bindings, did not resolve it. The unhelpful binding experiment was
reverted. These tests establish the shader-link crash fix, not correct world
rendering or improved FPS. The main player application and its data were not used.

## Legacy GPU release limitation

Older OpenGL ES GPUs may display transparent or fragmented terrain. This is
confirmed on the Fujitsu F-03L (Adreno 304, driver V@269), including the 0.7.5
baseline. The shader-link crash fix allows startup but does not fix terrain
rendering. ARMv7 availability does not imply that every legacy GPU is supported.
The normalized-vertex and simplified-fragment experiments were not retained.

On Android, Video Settings > Graphics API is now read-only and displays the
backend selected in launcher Setup. Desktop backend selection is unchanged.

For this candidate the upstream base stays at c0b65f1d2. The later 646 commits
through f2b9a33b4 are pending Android validation rather than included in the release.
