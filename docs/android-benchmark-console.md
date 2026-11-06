# Android benchmark console

Run from the repository root in an interactive PowerShell terminal:

```powershell
python tools/android_benchmark_console.py --device tablet --session natural-flight-night --renderers compatibility batching opengl --unattended --continue-on-error
```

The console dependencies are installed privately in
`.local/android/benchmark-python`. To install them on a different workstation:
`python -m pip install --target .local/android/benchmark-python rich`.

## Controls and recovery

- **P** requests a pause after the current trial; press again to resume.
- **Q** or **Ctrl+C** stops after the current trial collects its files and restores
  the isolated launcher's configuration. Closing the terminal forcibly is different.
- Repeating the same command resumes pending jobs from `console-jobs.json`.
- `--continue-on-error` records failures and proceeds to other configurations.
  Failed trials are never counted as valid FPS results.
- `--retry-failed` schedules failed trials again after their cause is addressed.
- A second console cannot acquire the same device lock. An existing test app is
  allowed to finish before the console starts another trial.
- `--demo` displays explicitly simulated UI without contacting Android.

Remove `--unattended` to require scene review before measurement. The console
then displays the preview path and waits for a `validation.json` decision in that
trial directory. Q cancels an unapproved review without starting measurement.

## Default natural workloads

The default console plan uses ocean, forest, jungle, desert, snowy plains,
Nether and End spectator routes, at **12, 16 and 24 chunks**. It compares the
original 0.7.3/0.7.7 libraries in isolated wrappers and the local release/ThinLTO
0.7.8 APK. The f42ef42 integration is excluded by default and requires explicit
`--versions` selection for a separate session.

Each measured minute alternates 10-second forward-flight segments and hovering
camera sweeps. Sweeps turn through 360 degrees and vary pitch toward the ground
and sky. Input uses the game's existing held-W and camera controls. `motion.json`
records the start/end state and actual duration. A spectator may cross terrain
during the flight; an air check at the starting camera is not full path validation.

Real overworld biomes are discovered using `/locate biome`; their coordinates
are retained in private `natural-routes.json`. Nether/End use independent camera
positions. The driver checks dimension and starting eye block, and can search a
nearby open starting point. Automatic checks are recorded as such, not as a
human visual review. Synthetic material scenes remain optional, not defaults.

Three builds × seven routes × three distances × three renderers × two passes =
**378 trials**. Compatibility alone is **126 trials**. Build order is reversed
between passes/cases. The three renderer series are separate and never averaged
together. Unsupported OpenGL attempts are retained as failures.

## Controlled settings

| Setting | Value |
|---|---|
| Renderer | Compatibility / Batching / OpenGL, separate series |
| Resolution | 100% |
| Render distance | 12 / 16 / 24 |
| Simulation distance | 12 |
| LOD | Off in the default biome series |
| Frame limit | 120; VSync remains enabled |
| Texture filtering | None |
| AO / entity shadows | Enabled in the common source options |
| Mipmaps / biome blend | 4 / 2 |
| Clouds | Off in the common source options |
| Entity distance scale | 1.0 |
| Time / weather | Daytime / clear |
| ADPF / display request | Off / System |
| Worker limit / cache budget override | 0: no custom override |
| Resource packs / shaders / mods | No packs copied into fresh test profiles |
| Internal recording / GPU timing | Enabled consistently |

The full effective input options are copied to each trial, rather than relying
on a preset name such as Fancy. Effective backend and rendering dimensions are
recorded; a fallback to another API fails the trial. VSync/display limits mean a
high-FPS result can be refresh-limited rather than workload-limited.

## Files per trial

- `run.json`: build/APK SHA-256, scene, renderer, distances, resolution and
  instrumentation flags. Source revision is also in the performance header.
- `controlled-options.txt`, `device-model.txt`, `memory-before.txt`.
- `effective-state.json`: actual renderer, world, player and scene state.
- `scene-state.json`: discovered route, dimension and starting eye-block check.
- Performance JSONL: frame windows and slow frames, CPU/GPU timings, nested
  stages where that historical version supports them, queues, chunks, geometry,
  memory and Android power/thermal information. Missing fields stay missing.
- `events.json`: startup, world entry, measurement boundaries and exit request.
- `motion.json`: movement/camera input, positions, duration and update intervals.
- Start/end, pause and exit PNGs; these are screenshots, not video recordings.
- `thermal-before.txt`, `thermal-after.txt`, `exit-info.txt`, and the driver log.
- `validation.json`: automatic/manual decision and its provenance.

Console FPS/CPU/GPU graphs come from real recorder windows. CPU/GPU are timings
in milliseconds, not utilization percentages. Summary percentiles are medians
of window percentiles, not percentiles reconstructed from every individual frame.

Raw Simpleperf call stacks, Perfetto system traces and GPU frame captures are
separate diagnostic passes. They are not automatically captured for every trial
and must not be implied by the presence of a JSONL report. They add overhead;
compare matching instrumentation modes. Never add nested CPU-stage timings.

Results and progress remain under the selected private fixture/session directory.
Successful disposable profiles are cleaned after collection; source fixtures and
production player data are retained. The console pauses below 2 GiB free space.
