# Android native mods: implementation plan

## Scope

Bring Horizon's source-only native mod bundles (`.hnmod`) to HorizDroid without replacing the Android renderer or weakening the existing sandboxed `.hmod` path.

This document describes a proposed implementation. It is not evidence that Android native-mod support already works.

## Important format distinction

A Horizon `.hnmod` is a source bundle for `horizon-mixins`. It contains a native manifest, a `horizon-mixin.toml`, and Rust source files. The mixin tool applies those changes to a specific Horizon source revision before Cargo compiles a new game client.

It is **not** a precompiled Android shared library that can simply be loaded with `dlopen`, and it is not interchangeable with a `.hmod` content package.

The desktop launcher currently performs this sequence:

1. Read and validate the `.hnmod` bundle and its pinned Horizon / horizon-mixins revisions.
2. Obtain the matching Horizon source.
3. Run `horizon-mixins apply` into a separate staged workspace.
4. Compile a separate game client from that modified source.

The Android launcher currently recognises source-only `.hnmod` catalogue entries but refuses installation because it cannot build a modified client on-device.

## Recommended Android approach: build-time integration

The first supported route should compile trusted native bundles into a custom Android APK in GitHub Actions.

A native mod is therefore included in the APK at build time. This is **not** dynamic loading of arbitrary `.hnmod` files by an already-installed APK.

### Proposed pipeline

1. Accept a selected bundle from a controlled repository path (initially `native-mods/<id>.hnmod`).
2. Validate the archive and native manifest using the existing `horizon-mods` native bundle parser.
3. Enforce exact source revisions and the existing native-mod trust/compatibility checks.
4. Fetch the pinned Horizon source and pinned `horizon-mixins` revision.
5. Apply the bundle to a temporary source workspace with `horizon-mixins apply`.
6. Build `horizon-android` for `aarch64-linux-android` using the existing Android SDK/NDK toolchain and Android packaging scripts.
7. Run the existing source/public-tree checks and APK audit; upload an experimental APK artifact with its source commit, mod ID, pinned revisions, ABI, and SHA-256.
8. Test on physical Android devices before calling the bundle compatible.

The initial proof of concept should target **ARM64 only**. Other ABIs can be considered after the Android build and native dependencies are verified.

### Renderer and content-mod invariants

- Keep the existing Vulkan path.
- Keep the existing OpenGL ES path for older GPUs.
- Do not replace the renderer or silently change the user's selected graphics backend.
- Keep the existing sandboxed `.hmod` runtime and its enable/disable behaviour.
- A native mod must not be able to disable the existing `.hmod` path merely by being present.

### Security requirements

Native mods are unsandboxed Rust code and can run with the app's process permissions. A build pipeline must not compile arbitrary bundles from untrusted pull requests while exposing signing secrets.

- Never expose signing keys, release credentials, or repository secrets to untrusted PR code.
- Pin Horizon and horizon-mixins to immutable commit revisions.
- Validate archive paths, file count, expanded size, manifest identity, and source layout before applying.
- Build untrusted contributions without secrets and keep artifacts clearly marked experimental.
- Do not silently install or execute a native bundle from a download URL.
- Keep provenance metadata and checksums with every APK artifact.

## Milestones

### M0 — Architecture audit

- [x] Confirm the Android crate and Android APK build workflow.
- [x] Confirm that Android currently exposes Vulkan and OpenGL ES rendering paths.
- [x] Confirm that Android currently imports sandboxed `.hmod` packages.
- [x] Confirm that Android currently labels `.hnmod` as unsupported.
- [x] Confirm that `.hnmod` uses source transformation and a separate Cargo build.
- [ ] Verify the exact Android cross-compilation requirements of `horizon-mixins` output.

### M1 — CI proof of concept

- [ ] Add a safe, explicit input for a trusted `.hnmod` bundle.
- [ ] Apply one test native mod to a temporary source workspace.
- [ ] Build the modified ARM64 Android native library and APK.
- [ ] Confirm that the APK retains Vulkan, OpenGL ES, and `.hmod` support.
- [ ] Publish an experimental artifact with reproducible provenance.

### M2 — Android user workflow

- [ ] Add a clear native-mod entry in the Android Mods UI.
- [ ] Explain that a selected `.hnmod` requires a custom APK build.
- [ ] Add a trust and compatibility warning before requesting a build.
- [ ] Provide a safe route to obtain the matching build artifact.
- [ ] Keep ordinary `.hmod` installation independent of native builds.

### M3 — Device validation

- [ ] Test ARM64 installation and launch.
- [ ] Test Vulkan on a compatible device.
- [ ] Test OpenGL ES on an older-GPU device.
- [ ] Test a native mod's actual hooks in-game.
- [ ] Test existing `.hmod` packages with and without the native mod.
- [ ] Record crashes, startup time, memory use, and frame-time changes.

## Current status

Only the architecture audit is complete. No Android `.hnmod` build pipeline or runtime loader is implemented yet. Do not describe the feature as supported until the proof-of-concept APK has been built and tested.
