# Android performance benchmark

For the interactive PowerShell console and current natural-biome defaults, see
[Android benchmark console](android-benchmark-console.md). The matrix runner below
also exposes older synthetic diagnostic workloads; they are not the console defaults.

Run `python tools/android_benchmark.py --fixtures <private-fixture-directory>`.
The default matrix runs Poco and the tablet concurrently, with two repetitions
and alternating version order. Each device runs one application at a time.
`--smoke --repeats 1` first checks the water fixture on both devices.
`--summarize` regenerates results without contacting a device.

To use only the tablet and keep an independent session:

```powershell
python tools/android_benchmark.py --fixtures .local/android/regression-073-077 --devices tablet --repeats 2 --session tablet-detailed-20261104
```

The current default matrix contains 19 workloads × four versions × two repetitions: 152 trials
on one device, run sequentially. It takes hours, not minutes. `--versions` selects
builds; `--cases` selects workload IDs listed by `--help`, including distance and
LOD distance. A failed launch stops the device queue for inspection.

The current device driver targets the isolated package
`dev.horizon.android.regression076`. Its disposable profiles contain copies of
prepared test worlds. APKs, Minecraft resources, raw device reports and worlds
belong in the ignored fixture directory. Production player profiles are not
benchmark inputs.

## Inputs

The fixture directory contains the original 0.7.3 and 0.7.7 native libraries in
isolated APK wrappers, plus locally built 0.7.8 and 0.7.8-f42ef42 release/ThinLTO
APKs. The wrappers support `run-as` and isolate data. APK SHA-256 hashes are saved
with the matrix and each run. These wrappers differ from public APKs. This
benchmark does not measure normal installation or Microsoft login.

The device driver currently specifies serials, test profile IDs, source world
names and UI coordinates for the two prepared devices. Inspect those inputs
before running on another device or launcher layout. Unlock devices before
starting. A failed launch stops that device's matrix for inspection.

## Workloads

- Natural forest, fixed camera, 16 chunks, LOD disabled.
- The same natural forest at 24 chunks with LOD disabled.
- Nether and End, generated from the copied test world.
- Water plane, leaf canopy, sand surface and a dense jungle-material fixture.
  These are synthetic material workloads, not natural biome benchmarks.
- No entities, 64 static zombies, 63 frozen husks, and 63 active husks.
- Cold and warmed traversal at 24 chunks.
- Continuous spectator flight through a new route and the same prewarmed route,
  holding the existing W action for 60 seconds. These differ from teleport workloads.
- A stationary 360-degree camera sweep over 60 seconds, targeting 20 yaw updates
  per second through the authenticated game-control channel.
- Far terrain at 16 chunks with LOD distances 64 and 128.

Simulation distance is 12, resolution scale 100%, Vulkan Compatibility,
Texture Filtering None, and the frame limit is 120. Commands set daytime and
clear weather where applicable. A datapack executes long fixture setup commands
to avoid truncated Android keyboard input.

## Output and interpretation

Each run retains performance JSONL, configuration and APK identity, before/after
thermal reports, screenshots, measurement events and Android exit history.
`benchmark-matrix/results.json` summarizes window FPS, frame-time distributions,
CPU/GPU timings, stage costs and resident/drawn geometry. Stage timings can be
nested and must not be added together.

Reports reject insufficient world samples, mismatched render distance, severe
thermal throttling, missing measurement boundaries, and unreviewed scenes.
After inspecting a trial's start/end screenshots and observed scene state, write
`validation.json` alongside its `run.json` with `{"valid": true}` or
`{"valid": false, "reason": "..."}`. A successful process exit alone does not
validate rendering or fixture commands. Results from a broken renderer remain
recorded but cannot be counted as performance wins.
A missing setup command can leave a visually plausible but incorrect scene.
New trials mark start/end boundaries in the recorder's `t_ms` clock. Only world
windows inside those boundaries enter the steady result. Legacy trailing-window
results are explicitly rejected for comparison. Results are scoped to the session;
old captures are not silently mixed in. Traversal windows include command-entry
time, so they are not pure stationary GPU measurements.

The report distinguishes total frames / total recorded frame time (`mean_fps`)
from median window FPS (`fps`). Percentile summaries are medians of window
percentiles, not global frame percentiles. Keep raw traces for individual stalls.

Successful disposable profiles are removed from the isolated app after their
reports have been collected. Failed trials are retained for investigation.
The original fixture and production app data are never removed. The driver waits
up to five minutes for reported thermal status to fall below moderate before
starting a trial; raw before/after thermal reports are retained. This does not
prove identical GPU/CPU clocks across trials.

Compare rendering size, drawn sections, geometry bytes, entities and thermal
state before comparing FPS. LOD sampling may finish at different rates across
versions. Equal configured LOD distance does not guarantee equal finished geometry.
Internal performance recording and GPU timestamps add overhead and are enabled
consistently. These results need a separate recording-off validation before a
release-wide performance claim.

## Remaining coverage

This is a workload matrix, not yet a complete release qualification suite. Natural
desert/jungle worlds must be prepared and validated separately from the synthetic
material scenes. Dedicated menu/blur, downward/sky camera, long-session thermal,
and recording-off scenarios are still needed. Source-level native CPU profiles
and GPU frame captures are separate diagnostic passes; do not mix their overhead
into the unprofiled comparison without marking it.
