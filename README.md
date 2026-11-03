# HorizDroid

**HorizDroid** is ZYVIX's Android-focused fork of [Horizon Android](https://github.com/Vasysik/horizon-android).

The goal is to extend Horizon Android with native Rust mod support while keeping its Android experience and graphics compatibility intact.

> HorizDroid is an independent, unofficial community fork. It is not affiliated with or endorsed by the Horizon developers, Mojang, or Microsoft.

## Project goals

- **Native mod support:** investigate and implement Android-compatible loading for compiled `.hnmod` native mods.
- **Keep content mods:** preserve existing `.hmod` support.
- **Graphics compatibility:** preserve Vulkan and OpenGL ES paths, including OpenGL ES support for older GPUs and devices.
- **Android-first:** retain touch controls, Android lifecycle handling, and support for the devices Horizon Android targets.
- **ARM64 builds:** make reproducible Android builds through GitHub Actions where practical.

## Status

HorizDroid is at the fork and development-planning stage.

**Native `.hnmod` loading is a development goal, not an implemented or verified feature yet.** A working implementation will require changes to the Android client/runtime and a compatible native-mod build and loading pipeline. File extensions alone are not interchangeable: `.hmod` content packages and `.hnmod` native mods have different formats and execution models.

Vulkan and OpenGL ES compatibility must be checked against the actual renderer implementation and tested on real Android devices. No performance gains or device compatibility are claimed until measured.

## Planned development

1. Map the Android client's renderer, mod discovery, and loading architecture.
2. Identify the upstream native-mod interface and its Android build requirements.
3. Design a safe Android-compatible `.hnmod` loading pipeline.
4. Keep existing `.hmod` behaviour working.
5. Build debug APKs with GitHub Actions and test on multiple GPU/backend combinations.
6. Develop and test native mods such as **TITANIUM** separately.

## Building and testing

Build instructions will be documented after the fork's Android toolchain and CI workflow have been reviewed and validated. Do not treat an untested workflow or artifact as a working release.

When reporting bugs, include the device model, Android version, GPU, selected graphics backend, build/commit, and reproducible steps. Do not share account credentials, signing keys, or private tokens.

## Upstream and licensing

HorizDroid is based on [Vasysik's Horizon Android](https://github.com/Vasysik/horizon-android) and retains the upstream repository history. Original copyright notices, licenses, and third-party notices must remain with their respective files and components. Check the applicable licenses before redistributing modified builds or third-party assets.

- [Upstream Horizon Android](https://github.com/Vasysik/horizon-android)
- [Horizon Android mod documentation](https://github.com/Vasysik/horizon-android/blob/android/docs/mods.md)
- [TITANIUM performance-mod project](https://github.com/ITS1NEXO/TITANIUM-Horizon-Rust)

---

**HorizDroid — Android freedom, native modding.**

Developed by **ZYVIX**.
