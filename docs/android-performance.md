# Android performance reports

In the launcher, open Settings, enable **Performance recording**, then press
Play. Reproduce the slow startup, travel or other problem during normal play.
Return to the launcher and select **Performance report → Open in Files / Reports**.
Copy or share the `.jsonl` file from **Horizon Android → Reports** in system Files.

Recording is off by default. Each run is limited to 30 minutes and 8 MiB; the
three latest internal reports are retained. Opening Reports does not create copies.
Copies exported by the user remain in their chosen destination. Older versions'
copies in Download/HorizonAndroid are left untouched.
Reports are written on a bounded background queue and flushed once per sample;
if Android kills the process, completed samples remain available on next launch.
No general storage permission is needed to browse the document provider.

The report contains:

- One-second frame/CPU/GPU summaries with percentiles, maxima and slow events.
- Named render/server CPU spans: elapsed and thread CPU time, counts and maxima.
- Worker tasks: saved-chunk loading, generation, decoration, lighting, meshing
  and saving; concurrent task totals are reported separately from frame timings.
- Streaming queues, dirty light, pending mesh uploads and cumulative upload bytes.
- Terrain draw calls, resident sections, GPU allocation counters where available,
  resolution, render/simulation distances, presentation mode and effective FPS cap.
- Focus/pause/idle state, underwater state and entity/particle counts.
- Camera yaw/pitch and player velocity for distinguishing a changed view or movement
  from a stationary scene. Player coordinates remain excluded.
- Buffer creation, error-scope resolution and arena growth-copy CPU timings.
- The source commit and tracked-change flag when built with `android_build.py`,
  so reports from different builds with the same version can be distinguished.
- Android power-saving mode and reported thermal status (Android 10+), without
  root or access to device identifiers. A thermal status of 0 means none, 1 light,
  2 moderate, 3 severe, 4 critical, 5 emergency, 6 shutdown; unavailable is null.

These identify expensive stages and correlate spikes with background work. Nested
spans overlap and must not be added together. Wall time exceeding thread CPU time
can mean scheduling or waiting; it does not prove a particular lock is responsible.
GPU timing is asynchronous and can be missing; this is not a per-shader GPU profiler.
Recording itself adds timing overhead, so compare benchmark runs with recording off.

There is no automatic upload. Reports do not include account tokens, nicknames,
chat, server addresses, world names, seeds or player coordinates.

Developer regression check: `python tools/test_android_performance.py`. It runs
the actual recorder/statistics/timing code in a host harness and substitutes only
Android publication. The isolated storage probe tests provider access separately.

## Vulkan terrain validation

Android Vulkan separates proven opaque terrain from alpha-cutout terrain within
the same render pass. The opaque shader does not discard fragments. This lets
mobile GPUs reject more hidden pixels before shading them; see the
[Khronos tile-rendering guidance](https://docs.vulkan.org/guide/latest/tile_based_rendering_best_practices.html).
Resolution, textures, fog, lighting and render distance are unchanged.

Meshes with order-sensitive overlapping surfaces retain the ordinary shader and
ordering. Fading sections use the cutout path. Shader packs, visible distant
terrain and GPU occlusion culling retain their existing render path. OpenGL ES
retains its previous mesh order and shaders.

On a 2405CPCFBG tablet with Adreno 710, Vulkan Compatibility at 2560×1600 and
16 chunks was measured in a stationary, frozen test world with recording,
extended GPU profiling and ADPF disabled. Alternating runs used the same camera
and resident geometry, without pending chunk work:

| Lighting | Ordinary terrain | Split terrain | GPU time reduction |
| --- | ---: | ---: | ---: |
| Night, three runs per path | 18.79 ms | 13.63 ms | 27% |
| Day, two runs per path | 19.79 ms | 14.77 ms | 25% |

These results describe this fixture and device, not a general FPS guarantee.
VSync limited the faster night runs to 60 FPS. The authenticated developer
control command `terrain-passes single|split` switches the pipeline for A/B
measurement; it does not change graphics settings or rebuild meshes. Status and
performance reports expose `terrain_pass_draws` as opaque/cutout/order-sensitive
draw counts, or null when the ordinary path is active.

The terrain shader regression tests compare the output of both paths over alpha
thresholds, fog, section fading and emission tiers. Geometry tests preserve the
ordering of overlapping cutout/opaque faces. This does not replace testing on
other GPUs, shader packs and resource packs.

## Vulkan sky ordering

Without a shader pack, Android Vulkan can draw the vanilla Overworld sky after
solid terrain, before entities and translucent surfaces, within the same render
pass. The sky reads depth at the far plane without writing it. This avoids shading
pixels hidden by terrain; see the [skybox ordering discussion from NVIDIA](https://developer.nvidia.com/gpugems/gpugems/part-v-performance-and-practicalities/chapter-28-graphics-pipeline-performance).
The sky fragment shaders, time, colours, fog and celestial rotation are unchanged.

Native and classic shader packs retain their existing ordering. The title screen,
End sky, water mirror and GPU occlusion path also retain their original path.
OpenGL ES is unchanged. Failure to create any of the additional pipelines retains
the original sky order. The authenticated developer command `sky-order early|late`
allows comparison without changing the world's settings or network protocol.

On the same Adreno 710 tablet at 2560×1600 and 16 chunks, with split terrain active:

| Test | Early sky | Late sky |
| --- | ---: | ---: |
| Stationary GPU time, mean of three runs | 14.73 ms | 13.59 ms |
| GPU time while turning, two runs | 16.47–16.58 ms | 13.43–13.49 ms |
| Frame p99 while turning | 21.17–21.40 ms | 18.45–18.91 ms |

The turning test used running world time near sunrise and the same eight-second
touch swipe from the same initial camera direction. It is a separate workload
from the stationary comparison; its percentages must not be multiplied with the
earlier terrain results to claim a combined speedup.

Paired daytime, sunset, night and sunrise captures had identical RGB pixels outside
the changing diagnostic counters. Golden Hour was enabled and disabled, and its
captures also matched between the two sky-order requests. This verifies that pack
on this device, not every shader pack or GPU. Capture tools and captures are not
included in the APK or repository.

## Vulkan Batching: indirect ranges and the vanilla threshold

The opaque/cutout split initially passed each material group's first draw as the
base of its indirect buffer. Those groups actually share the complete frame
buffer. Direct draws ignored that parameter, hiding the bug in Compatibility;
indirect draws could replay the wrong commands and omit visible surfaces when
culling changed paths. Both material subranges now retain buffer base zero.

Android's selected Batching mode also now uses the existing flat-index path for
ordinary terrain above its 512-section threshold. Previously the Android
threshold was defined but the vanilla backend policy still disabled it, leaving
the slower per-section indirect calls. Native shader materials retain their
previous path; classic terrain/shadow programs retain their explicit flat inputs.
Neither this selection nor the offset fix changes the default Compatibility mode.

On the Adreno 710 tablet at 2560×1600, 16 chunks, seed
`-4240118709374351666`, position `-24.5 102 -47.5`, with world time frozen and
resident geometry, a same-build developer A/B comparison with culling active gave:

| Terrain commands | GPU mean | Mean FPS |
| --- | ---: | ---: |
| Per-section indirect | 46.05 ms | 20.09 |
| Flattened index batches | 22.30 ms | 38.37 |

Four camera directions were compared with direct draws after the offset fix;
the large missing regions were gone. This is a device/scene-specific comparison,
not a performance guarantee or proof that every reported Batching glitch has the
same cause. Final automatic-path validation is recorded separately from forced
developer A/B runs.

The resulting optimized test APK's automatic Batching path measured 38.58 FPS
and 22.29 ms mean GPU time at that same spawn, without forcing the flat-index
switch. A ten-second camera flight through the area measured 31.90 FPS and
27.00 ms mean GPU time; movement and newly exposed geometry remain more costly.
ARM64 Clippy, ARMv7 compile checks and four flat-index planning/GPU tests passed.
Golden Hour was enabled in-world and disabled again on this APK; this is not a
validation of every native or classic shader pack.
