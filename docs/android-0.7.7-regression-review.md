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

- Matched production-profile GPU/CPU comparisons and causal isolation of the slowdown.
- ARMv7 device recheck after the ARM64 GLES fix; other drivers and x86 ABIs.
- Mixed physical/touch toggle inputs, suspend/resume and resource reloads.
- Save/reopen player state, dimensions and server travel after the nightly integration.
- Mod enable/disable/restart, disabled defaults and profile/file-access isolation.

The earlier missing-world report was withdrawn by the owner. No world migration,
relocation or deletion was added.
