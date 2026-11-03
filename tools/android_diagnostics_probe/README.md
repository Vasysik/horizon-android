# Android diagnostics probe

Standalone Rust NativeActivity linking the real local writer and Android exit
collector. It uses only its own `files/diagnostic-probe/` directory and links no
game or renderer. Never package this in a player APK.

Package as a distinct debuggable application ID `dev.horizon.diagnosticsprobe`,
with stock `android.app.NativeActivity`, `android.app.lib_name` set to
`horizon_diagnostics_probe`, `hasCode=false`, min SDK 28 and target SDK 35.

After launch, write `abort`, `panic`, `export` or `finish` to
`files/diagnostic-probe/command` using `run-as` for this probe only. The first
two intentionally terminate **the probe process**. Relaunch and inspect its
`performance/android-exits.json`, previous session, panic file and exported ZIP.
Native traces are OS-dependent; binary tombstones remain separate from the
redacted ZIP. `abort` checks recovery when no Rust panic hook runs.

Keep manifests, APKs, signing material, build outputs and captured traces under
ignored `.local/`. This is a diagnostics test, not evidence of game performance.
