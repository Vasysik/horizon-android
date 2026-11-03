<div align="center">
  <img src="horizdroid-logo.jpg" alt="HorizDroid logo" width="220">
</div>

# HorizDroid

**HorizDroid** is ZYVIX's Android-focused fork of [Horizon Android](https://github.com/Vasysik/horizon-android).

The goal is to bring Horizon's source-based native Rust modding workflow to Android through reproducible APK builds, while keeping the Android experience and graphics compatibility intact.

> HorizDroid is an independent, unofficial community fork. It is not affiliated with or endorsed by the Horizon developers, Mojang, or Microsoft.

## Project goals

- **Native mods:** build trusted source-only `.hnmod` bundles into custom Android clients through GitHub Actions.
- **Keep content mods:** preserve the existing sandboxed `.hmod` runtime and installation flow.
- **Graphics compatibility:** preserve Vulkan and OpenGL ES paths, including OpenGL ES support for older GPUs and devices.
- **Android-first:** retain touch controls, Android lifecycle handling, and support for the devices Horizon Android targets.
- **Reproducible ARM64 builds:** start with ARM64 APKs and expand only after other ABIs are verified.

## Important: how native mods work

A Horizon `.hnmod` is a Rust source/mixin bundle, not a precompiled library that an installed APK can load directly. It must be applied to a matching Horizon source revision and compiled into a separate game client.

Therefore, the first Android implementation is planned as **build-time integration**: GitHub Actions will apply a selected, trusted `.hnmod` and build a custom APK. This is not arbitrary on-device Rust compilation or dynamic loading of `.hnmod` files.

The existing Android app currently supports sandboxed `.hmod` packages. Android `.hnmod` support is not implemented yet.

## Status

HorizDroid is in active architecture and implementation planning. The initial source audit is documented in [Android native-mod implementation plan](docs/android-native-mods.md).

Vulkan and OpenGL ES paths exist in the upstream Android architecture, but every custom APK must retain and test both paths. No performance gains or device compatibility are claimed until measured on real devices.

## Planned development

1. Validate the Android cross-compilation requirements of the existing native-mod toolchain.
2. Build one trusted test `.hnmod` into an ARM64 Android APK through GitHub Actions.
3. Confirm that Vulkan, OpenGL ES, and existing `.hmod` support remain intact.
4. Add a safe Android workflow for requesting and installing matching custom builds.
5. Develop and test native mods such as **TITANIUM** separately.

## Building and testing

Build instructions and the staged implementation plan are in [docs/android-native-mods.md](docs/android-native-mods.md). Existing Android build instructions are inherited from upstream and will be updated as HorizDroid's verified build workflow evolves.

When reporting bugs, include the device model, Android version, GPU, selected graphics backend, build/commit, and reproducible steps. Do not share account credentials, signing keys, or private tokens.

## Upstream and licensing

HorizDroid is based on [Vasysik's Horizon Android](https://github.com/Vasysik/horizon-android) and retains the upstream repository history. Original copyright notices, licenses, and third-party notices must remain with their respective files and components. Check the applicable licenses before redistributing modified builds or third-party assets.

- [Upstream Horizon Android](https://github.com/Vasysik/horizon-android)
- [Horizon Android mod documentation](https://github.com/Vasysik/horizon-android/blob/android/docs/mods.md)
- [Horizon native-mod documentation](https://github.com/Vasysik/horizon-android/blob/android/docs/native-mods.md)
- [TITANIUM performance-mod project](https://github.com/ITS1NEXO/TITANIUM-Horizon-Rust)

---

**HorizDroid — Android freedom, native modding.**

Developed by **ZYVIX**.
