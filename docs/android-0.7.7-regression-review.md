# Android 0.7.7 regression review

This review compares v0.7.6 with the Android candidate based on published nightly
8c02c0b85. It is not a release sign-off or a guarantee of zero regressions.

## Review scope

The first-parent changes were reviewed by subsystem: blur resource reuse; launch
request/session handoff and exit dialog; installer cache-eviction recovery;
unchanged GPU allocation policy; derived behaviour-cache invalidation; block-editor
IME; GLES varying packing; startup input; opt-in worker/cache budgets; contributor
crouch cancellation and connection logs; bundled opt-in mods; Worlds access;
nightly integration; overlay lookup and sprint modifier reconciliation.
The nightly brings 1,846 commits including merge history. This subsystem review
must not be represented as individual behavioral verification of all those commits.

## ARM64 OpenGL linker failure

On an Adreno 710 tablet the exact generated terrain vertex/fragment pair compiled
and linked in an ordinary EGL context, but failed to link with robust access.
Constant-index isolation narrowed it to the emissive-layer uniform-array search.
GLES now expands its 32 entries, preserving first-match ordering and the no-match
sentinel. Robust access remains enabled. Vulkan shader source is unchanged.

The ordinary optimized ARM64 APK passed launcher creation, game entry and world
rendering on that device. GPU image tests compare every emissive-table slot with
the original shader, in addition to the existing fog/fade/HDR cases. This does
not establish compatibility with every GLES driver. GLES TAA remains unavailable
where GLSL cannot load depth textures; the renderer reports that limitation.

## Early GLES Create World request

The Android GLES deferred-start gate originally considered world/server entry
requests only. Upstream's new preparing screen needs full game resources before
it can open Create World settings. Without treating that request as a handoff,
the early menu continued presenting Preparing indefinitely. The gate now also
considers the title's preparation request, preserving the presented-frame boundary
and the existing asynchronous world-creation loader. Cold-start ARM64 tablet validation passed: launch GLES, enter Singleplayer,
choose Create New World, reach the editable creation settings without first
entering a world. The latest complete APK has not been rechecked on F-03L;
its minimal GLES 3.0 probe compiled and linked the expanded shader successfully.

## GPU performance remains an open release gate

In copies of the same old test world, at the same camera, 12 chunks and 100%
resolution on Adreno 710, Vulkan Compatibility GPU-time medians were 14.23 ms
for the release 0.7.6 native library and 16.23/16.39 ms for two candidate runs.
Profiling was enabled in both. Idle-capped samples were excluded; no thermal
warning or power-saving mode was reported. The candidate used optimized fast,
the baseline ThinLTO: these are not matched production-profile CPU benchmarks.

The frame-stage profiler, frame scaler and opaque-output modules have no diff
from v0.7.6. The Vulkan terrain WGSL matches the selected upstream nightly.
Upstream changed None filtering from a nearest sampler/raw lookup to a linear
sampler plus shader pixel reconstruction, and added emissive/colored-light work.
These are candidates for isolation, not proof that all Android changes are innocent.
No visual-quality rollback has been applied to mask the regression.

New upstream main was fetched through fe1294845. Its colored-light flattening
matches our GLES lowering. The opaque-pass optimization was subsequently limited
to Metal upstream; blindly applying its original double-draw version on Android
would be inappropriate. No wholesale synchronization was performed in this pass.

## Remaining checks

### Follow-up: include the 0.7.4 baseline

The owner reports that the first slowdown predates 0.7.5. A comparison against
0.7.6 alone therefore cannot exclude the original regression. Exact public native
libraries of 0.7.4 (`53a1235f0917`), 0.7.5 (`74657bb648a4`) and the 0.7.7 candidate
(`9285d1a249ed`) were exercised in an isolated tablet host on copies of the same
heavy world, seed `-4240118709374351666`: position `-24.5 102 -47.5`, yaw/pitch
zero, 16 render chunks, 12 simulation chunks, 100% 2560×1600, Vulkan Compatibility,
no shader pack or LOD, clear daylight and frozen ticks. All had 8,625 resident
sections and empty streaming queues. Recording and GPU profiling were enabled.

| Version | Mean FPS | Mean GPU frame time |
| --- | ---: | ---: |
| 0.7.4 | 46.09 | 19.38 ms |
| 0.7.5 | 42.34 | 21.00 ms |
| Candidate 0.7.7 | 39.61 | 21.72 ms |

Each row averages ten eligible one-second report samples after geometry settled.
These short stationary tablet observations establish the need to investigate the
earlier interval; they do not reproduce or explain the full reported Poco loss,
nor certify movement, multiplayer, UI or thermally sustained performance.

Between 0.7.4 and 0.7.5, upstream added the LOD fade/dither path to the ordinary
terrain shader (`ada4e6567`, `94f48bbf6`, `963b7d8bd`). The later texture-sampling
rewrite (`b56f94b2b`) cannot explain the first interval by itself. Both require
separate causal comparisons rather than an indiscriminate quality rollback.

A narrow cutout entrypoint skips the LOD dither calculation only when the CPU
proves every origin has a nonnegative fade encoding. It retains alpha cutout,
visibility, lighting, HDR and fog. Native/classic packs, visible LODs, GPU
culling, fading origins and unsupported pipelines retain their existing paths.
Pixel comparisons cover alpha-threshold edges, fog and HDR; unsupported pipeline
creation falls back to the original shader. An initial isolated nightly-based
ABAB measured GPU 23.263/23.271 ms with the original path and 22.587/22.598 ms
with the specialization. That approximately 3% result is not a fix for the full
regression and is not a production-build comparison against 0.7.4.
On the existing Android baseline, an optimized fast-build ABAB subsequently
measured 21.770/22.101 ms with the original cutout and 20.988/21.247 ms with the
specialization. All 31 tests in the Android GPU harness and ARM64 Clippy passed.
Flat-index frames retain their existing shader; this result is for Compatibility.

### Fully visible terrain specialization

A second guarded Vulkan path specializes the existing reconstructed sampler's
non-RGSS branch and omits section-appearance calculations after every instance
has reached full visibility. It preserves the sampler, gradients, mip selection,
alpha cutout, lighting, emissive masks, HDR and distance fog. It does not restore
the older raw-nearest sampler or reduce quality settings. The CPU requires the
bare ordinary section UV-step encoding for every instance. Partial visibility,
LOD encodings, RGSS, shader packs and GPU-culling paths retain the general shader;
pipeline-creation failure also falls back. Flat-index Batching is unchanged.

An optimized fast-build ABAB in the same isolated 16-chunk tablet scene measured:

| Path | Mean FPS | Mean GPU frame time |
| --- | ---: | ---: |
| Existing static cutout | 39.51 | 21.576 ms |
| Fully visible specialization | 42.02 | 20.154 ms |
| Existing path, repeat | 39.53 | 21.567 ms |
| Specialization, repeat | 41.95 | 20.168 ms |

Each run lasted ten seconds. This is a causal comparison within one process,
not a production release comparison against 0.7.4 or a Poco result. Camera
rotation and a teleport triggering chunk work completed without a validation
failure. With RGSS selected, toggling the specialization gave 33.282/33.355 ms
GPU time, consistent with retaining the original path. Anisotropic filtering
also rendered successfully. All 32 Android GPU tests passed, including comparison
of fog, alpha thresholds, colored light and all emissive-mask slots. ARM64 fast
compilation and Clippy passed. These gains remain partial; the full reported
performance regression is not resolved.

The specialized non-RGSS sampler also replaces a square-root followed by
division with an inverse-square-root multiplication, retaining the same
denominator floor and all texture coordinates/gradients. The generic sampler
and shader-pack paths are unchanged. A textured GPU comparison was added; all
33 GPU tests passed. Separate optimized-fast APK comparisons in order A/B/B/A
measured 20.038/19.841/19.814/20.030 ms GPU time (43.71/44.11/44.16/43.56 FPS),
with the same 8,265 resident sections in each run. This additional approximately
1% gain is small and remains a short tablet observation, not a Poco claim.

### Looking down and the display's idle refresh policy

At the same camera position looking down at pitch 89 degrees, the current
optimized test rendered 22 sections rather than 880 and reached the tablet's
60 FPS display limit (9.584 ms average GPU time). Thus this tablet run does not
reproduce the owner's report that looking down no longer increases frame rate.
The exact 0.7.4 public library also reached 60 FPS looking down, with its game
limit still recorded as 120. Android's display service and SurfaceFlinger both
reported an active 60 Hz mode. After a touch, repeated display queries reported
120 Hz at 0.12/1.27/2.38 seconds, then 60 Hz at 3.73 seconds and thereafter,
without changing system display settings. The observed initial high FPS followed
by 60 therefore occurs on the old version too and coincides with the firmware's
idle refresh switch. This does not explain the sub-60 FPS terrain regression or
prove the same cause on Poco; compare actual GPU/CPU timings as well as FPS.
With the current test app's existing preferred-display setting temporarily set
to 120 Hz, Android remained at 120 Hz after idle. A verified pitch-89-degree
measurement then reached 118.57 FPS with 5.501 ms average GPU time and 22 drawn
sections. The setting was restored after testing. This is a display-policy
observation on this tablet, not a promise that every firmware honors requests.

### Poco verification and worker/cache isolation

On the owner's connected Adreno 810 phone, the combined static-cutout and
settled-terrain paths were toggled A/B/A/B within one optimized-fast process,
at 16 chunks and 100% 2772×1280. The same 8,266 sections were resident throughout:
28.48/31.69/28.54/31.90 FPS and 34.104/30.517/34.028/30.348 ms average GPU time.
This reproduces a roughly 12% FPS gain from the new paths on that phone, but
does not establish parity with 0.7.4. Looking down reduced GPU time to 6.952 ms;
the short sample averaged 69.68 FPS with intermittent longer CPU/frame intervals,
so GPU relief alone did not produce a sustained 120 FPS result there.

The optional worker/cache controls postdate 0.7.5; their zero settings preserve
the engine defaults. The worker-selection formula is the same in 0.7.4 and
0.7.5, and the GPU arena file is identical between those releases. Their upload
code changed, but the old matching scene's GPU stage samples show preparation
at approximately 0.081/0.069 ms and world rendering at 17.962/19.763 ms for
0.7.4/0.7.5 respectively. This points to world rendering in that scene rather
than proving all buffer policies harmless in every workload.

Within the current tablet test, the settled stationary scene measured default
44.14 FPS / 19.597 ms GPU, one chunk worker 44.13 / 19.534, a 32 MiB reusable
cache budget 44.07 / 19.615, and defaults again 43.84 / 19.729. Streaming queues
were empty. These changes did not recover the stationary GPU loss; they may
still affect generation, memory pressure and longer movement tests. Test-only
configuration was restored afterwards.

### Exact 0.7.4 library on Poco

The owner's phone subsequently ran the exact 0.7.4 public native library in the
isolated wrapper on a copy of the test profile. At the matching position and
zero yaw/pitch, 16 chunks, 2772×1280, no packs and 8,266 resident sections,
eight pre-AFK-cap one-second samples averaged 32.98 FPS and 30.194 ms GPU time.
The current optimized-fast build with recording/profiling also enabled measured
31.73/31.72 FPS and 30.334/30.333 ms GPU time in two ten-second runs. This scene
therefore does not reproduce a historical 60-versus-30 FPS loss, and its GPU
times are now close. These are not matched production-profile CPU comparisons:
0.7.4 used the public release library, whereas the current test used fast.
Other scenes and longer phone sessions remain unverified.

An intermediate current-build downward test hit the game's own AFK limit of
30 FPS after a minute without physical input. Control commands do not reset
that timer. It must not be cited as proof of a recording-induced 30 FPS cap.
Repeating with the test-only inactivity setting set to `minimized` reached
59.69 FPS / 7.889 ms GPU time looking down. Ordinary Android touch events do
reset the idle timer in the game event handler. Existing records still show
higher CPU timings with detailed recording, but require separate isolation.

- Matched production-profile GPU/CPU comparisons and causal isolation of the slowdown.
- ARMv7 device recheck after the ARM64 GLES fix; other drivers and x86 ABIs.
- Mixed physical/touch toggle inputs, suspend/resume and resource reloads.
- Save/reopen player state, dimensions and server travel after the nightly integration.
- Mod enable/disable/restart, disabled defaults and profile/file-access isolation.

The earlier missing-world report was withdrawn by the owner. No world migration,
relocation or deletion was added.
