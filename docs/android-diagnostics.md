# Android crash diagnostics

## Send a useful report without a PC

On an APK containing the 0.7.6 diagnostic changes:

1. Reproduce the problem. If Android closes the app, open Horizon again and wait
   a few seconds so its background system-exit collector can finish.
2. Open launcher Settings > **Diagnostic report** > **Export ZIP**.
3. The system Files app opens **Horizon Android / Reports**. Copy/share the newest
   `horizon-diagnostics-*.zip`. Review it before attaching it to an issue.
4. Include the exact APK version/source commit, device, renderer, resolution,
   and what you did immediately before the failure. State whether you returned
   to the Horizon launcher or the Android home screen.

Performance recording is not required. If it was enabled, its retained report
can also contribute a bounded tail to the ZIP. Nothing is uploaded automatically.

The **Diagnostics** button shows the last handled error and whether the previous
process lacked a clean finish marker. A process killed by Android or force-stopped
by the user also lacks that marker; it is not automatically labelled a crash.

## Contents and limits

- `diagnostic.log`, rotated/previous logs: local Android/Rust log messages from
  launch onward, up to 512 KiB per log. Redaction, process-memory reads and disk writes use a bounded worker queue;
  overload drops messages rather than blocking rendering, with a dropped count
  in the next session snapshot.
- `current-session.json`, `previous-session.json`: build/ABI/PID, startup/game
  phase, handled error and a five-second process-memory heartbeat. Context is
  built only when a snapshot is due. These are last observations, not a profiler
  and not a diagnosis of why a thread stopped.
- `android-exits.json`: Android 11+ OS exit reasons, status, process importance,
  PSS/RSS samples in KiB, device model/API and trace availability. Zero memory
  values can mean no OS sample. Collection errors and unsupported Android
  versions are explicit; one inaccessible trace does not discard other entries.
- `last-rust-panic.txt` and available Horizon crash reports: bounded panic/error
  details. These cannot intercept every signal or an Android low-memory kill.
- `exit-*.trace-summary.txt`: selected fields from available native protobuf
  tombstones: signal, crashed thread, up to 64 frames, relative PCs, module names
  and Build IDs. Unknown/malformed/truncated traces may have no usable frames.

Raw system traces remain in private app storage, bounded to 1 MiB each / eight files.
The Files provider exposes exported diagnostic ZIPs and standalone performance
JSONL files only; session snapshots, raw logs and OS traces cannot be opened through it.
They may contain private memory/path information. The ZIP excludes their raw
memory/register dumps and instead includes a readable crashed-thread summary
when decoding succeeds. ANR/plain-text traces get best-effort text redaction.
Export **one ZIP** when reporting a problem; individual internal log files are not
needed. JSONL files remain available for advanced performance analysis.
Accounts, credentials, worlds, screenshots and launcher configuration are not
exported. Redaction of arbitrary log text is **best effort**; inspect before
sharing. ZIPs retain at most 4 MiB of text payload, 256 KiB per ordinary input,
32 selected files and the three latest exported archives. The manifest records
included/omitted files and source timestamps/truncation.

## Native symbols for maintainers

Release packaging preserves private `.debug` ELF files and matching metadata in
`.local/android/native-symbols/<abi>/`, outside APKs and release attachments.
Android libraries carry an ELF Build ID. Keep these symbols for published APKs;
rebuilding a later revision does not recreate symbols for an earlier crash.

Match the report's ABI and Build ID before passing its relative PCs to the NDK's
`llvm-symbolizer --obj=<matching-file.debug> <relative-pc>`. Optimized release
builds may have limited source-line information. Vendor GPU/Android libraries
need their own symbols; our engine's symbols cannot resolve them.

## If the launcher never opens

Crashes before the native library/entry point loads cannot be logged by Rust.
On Android 9/10 the OS exit-history API is unavailable. Some firmware also does
not retain traces. In these cases collect an ADB log while reproducing, or ask
for a device-specific test. With the correct device selected:

```powershell
adb -s SERIAL logcat -v threadtime > horizon-system-log.txt
# Reproduce; stop collection with Ctrl+C.
adb -s SERIAL shell dumpsys activity exit-info dev.horizon.android
```

A full system log may include other apps' information. Review/redact it before
sharing; do not post a full bugreport automatically. Avoid clearing the system
log first, since it can erase the failure being investigated.

## Verification

The standalone Rust probe in `tools/android_diagnostics_probe` was tested on
Android API 36 / ARM64: a native abort without a panic hook was recovered as
`native_crash`, status 6, with an OS tombstone; a separate Rust panic also saved
its panic file. After relaunch, the real exporter produced a ZIP containing
readable native frames/Build IDs and no binary memory dump. The probe was removed
and the installed game/player data were not used. This verifies collection,
not the causes of issues #22, #24, #33 or #34 and not every vendor's trace support.

Host tests: `python tools/test_android_diagnostics.py`. Optional private fixture:
set `HORIZON_NATIVE_TOMBSTONE_FIXTURE` to a saved Android protobuf trace to test
its readable summary. Fixtures and device captures stay under ignored `.local/`.
