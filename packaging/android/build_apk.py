#!/usr/bin/env python3
"""Build Starling's arm64-v8a Android APK (Horizon on Android): horizon-android as a
NativeActivity library.

The APK holds the game library, the launcher icon, Starling Visuals and what prepares the game
files on the phone (the Java launchers in native/ and the exporters' sources), nothing of
Minecraft: the resources
horizon-prepare makes from the player's own copy are made on the phone, or copied there.

    python3 packaging/android/build_apk.py                 # build the library, then the APK
    python3 packaging/android/build_apk.py --no-build      # package the library built last
    python3 packaging/android/build_apk.py --install       # and adb install it
    python3 packaging/android/build_apk.py --zip           # and a zip of it to send
    python3 packaging/android/build_apk.py --dev           # Starling Dev, beside the release

Needs the Android SDK (ANDROID_HOME, else the default folder) with the build-tools, platform and
NDK pinned below, the aarch64-linux-android Rust target, cargo-ndk and a JDK (javac, keytool,
apksigner). Without HORIZON_ANDROID_KEYSTORE it signs with a debug key kept in target/android.
"""

import argparse
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent

APPLICATION_ID = "io.github.juanluci.starling"
LABEL = "Starling"
# --dev: an app of its own, installed beside the release (which a build signed with another key
# cannot install over), with its own folder (StarlingActivity.sharedFolder: StarlingDev) and its
# own sign-in and game files.
DEV_SUFFIX = ".dev"
VERSION_NAME = "0.1.6"
VERSION_CODE = 8
MIN_SDK = 26  # AAudio (cpal)
TARGET_SDK = 35
PLATFORM = 36
BUILD_TOOLS = "35.0.0"
NDK = "28.2.13676358"
ABI = "arm64-v8a"
TRIPLE = "aarch64-linux-android"
LIBRARY = "horizon_android"
# Every LOAD segment aligned for 16 KB pages, which Android 15 devices may use.
PAGE_FLAGS = "-C link-arg=-Wl,-z,max-page-size=16384"
ICONS = {
    "mipmap-mdpi": "48x48",
    "mipmap-xxhdpi": "128x128",
    "mipmap-xxxhdpi": "256x256",
}
# What horizon-prepare reads from its version folder on the phone (assets/prepare-kit.zip):
# the pins and the exporters' Java sources, which the phone compiles against the player's jars.
PREPARE_KIT = (
    "research-lock.json",
    "crates/horizon-prepare/src/exporters.rs",
    "mods/horizon/src/test/java",
)


def sdk_dir():
    env = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if env:
        return Path(env)
    if sys.platform == "win32":
        return Path(os.environ["LOCALAPPDATA"]) / "Android" / "Sdk"
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Android" / "sdk"
    return Path.home() / "Android" / "Sdk"


def tool(directory, name):
    """An SDK tool, with Windows' .exe or .bat suffix."""
    for suffix in ("", ".exe", ".bat", ".cmd"):
        path = directory / (name + suffix)
        if path.is_file():
            return path
    sys.exit(f"build_apk: {name} not found in {directory}")


def run(*args, **kwargs):
    print("+", " ".join(str(a) for a in args), flush=True)
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def ndk_tool(ndk, name):
    host = {"win32": "windows-x86_64", "darwin": "darwin-x86_64"}.get(sys.platform, "linux-x86_64")
    return tool(ndk / "toolchains" / "llvm" / "prebuilt" / host / "bin", name)


def build_js_engine():
    """Starling's shared JavaScript engine (crates/starling-mod-sdk/js-engine), which the game
    embeds to run JavaScript mods: built with Node when missing or older than its sources.
    Without Node the APK runs component mods only."""
    folder = ROOT / "crates" / "starling-mod-sdk" / "js-engine"
    engine = ROOT / "target" / "js-engine" / "engine.wasm"
    sources = [folder / "engine.js", folder / "build.mjs", folder / "package.json",
               ROOT / "crates" / "starling-mod-sdk" / "wit" / "starling.wit"]
    if engine.is_file() and all(engine.stat().st_mtime >= s.stat().st_mtime for s in sources):
        return
    npm = shutil.which("npm")
    if npm is None:
        print("build_apk: no npm: the APK runs no JavaScript mods", flush=True)
        return
    if not (folder / "node_modules").is_dir():
        run(npm, "install", "--no-audit", "--no-fund", cwd=folder)
    run(npm, "run", "build", cwd=folder)


def build_library(profile, ndk, sdk):
    env = dict(os.environ, ANDROID_NDK_HOME=str(ndk), ANDROID_HOME=str(sdk))
    env["RUSTFLAGS"] = (env.get("RUSTFLAGS", "") + " " + PAGE_FLAGS).strip()
    run(
        "cargo", "ndk", "-t", ABI, "--platform", MIN_SDK,
        "build", "--profile", profile, "-p", "horizon-android",
        cwd=ROOT, env=env,
    )


def build_launchers(ndk, out):
    """The `java` and `javac` horizon-prepare runs on the phone (native/java_launcher.c), with the
    C++ runtime the Java runtime's libraries need: an app runs programs only from its library
    folder, so they go in the APK as libraries."""
    clang = ndk_tool(ndk, f"{TRIPLE}{MIN_SDK}-clang")
    source = HERE / "native" / "java_launcher.c"
    libraries = {}
    for name, flags in (("libstarling_java.so", []), ("libstarling_javac.so", ["-DJAVAC"])):
        target = out / name
        run(
            clang, "-O2", "-fPIE", "-pie", "-Wall", "-Werror", *flags,
            "-Wl,-z,max-page-size=16384", "-o", target, source, "-ldl",
        )
        libraries[name] = target
    sysroot = ndk_tool(ndk, "clang").parent.parent / "sysroot"
    libraries["libc++_shared.so"] = sysroot / "usr" / "lib" / TRIPLE / "libc++_shared.so"
    return libraries


def zip_folder(folder, archive):
    """`folder`'s files in the zip `archive`, all dated alike: the same files make the same zip,
    which the app tells from the next by its bytes."""
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for file in sorted(p for p in folder.rglob("*") if p.is_file()):
            info = zipfile.ZipInfo(file.relative_to(folder).as_posix(), (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, file.read_bytes())


def assets(out):
    """The APK's assets/: prepare-kit.zip (PREPARE_KIT, with the paths horizon-prepare reads),
    lowbudget.zip, the skin the setup draws the menus with (packaging/lowbudget/pack, made by its
    make.py), and StarlingVisuals.zip, the shader pack the app puts in shaderpacks/ (also left in
    `out` for the release page)."""
    folder = out / "assets"
    shutil.rmtree(folder, ignore_errors=True)
    folder.mkdir()
    with zipfile.ZipFile(folder / "prepare-kit.zip", "w", zipfile.ZIP_DEFLATED) as kit:
        for entry in PREPARE_KIT:
            path = ROOT / entry
            files = sorted(p for p in path.rglob("*.java")) if path.is_dir() else [path]
            for file in files:
                kit.write(file, file.relative_to(ROOT).as_posix())
    zip_folder(HERE.parent / "lowbudget" / "pack", folder / "lowbudget.zip")
    zip_folder(HERE.parent / "shaderpacks" / "starling-visuals", folder / "StarlingVisuals.zip")
    shutil.copyfile(folder / "StarlingVisuals.zip", out / "StarlingVisuals.zip")
    return folder


def compile_java(build_tools, android_jar, out):
    """StarlingActivity and LauncherActivity (java/) as classes.dex."""
    classes, dex = out / "classes", out / "dex"
    for folder in (classes, dex):
        shutil.rmtree(folder, ignore_errors=True)
        folder.mkdir()
    sources = sorted((HERE / "java").rglob("*.java"))
    run("javac", "--release", "8", "-nowarn", "-classpath", android_jar, "-d", classes, *sources)
    run(
        tool(build_tools, "d8"), "--release", "--min-api", MIN_SDK, "--lib", android_jar,
        "--output", dex, *sorted(classes.rglob("*.class")),
    )
    return dex / "classes.dex"


def link_resources(aapt2, android_jar, out, assets, dev):
    """The manifest, the launcher icon, the themes and `assets`, compiled and linked into
    `out/base.apk`."""
    manifest = (HERE / "AndroidManifest.xml").read_text(encoding="utf-8")
    application_id = APPLICATION_ID + DEV_SUFFIX if dev else APPLICATION_ID
    label = LABEL + " Dev" if dev else LABEL
    manifest = manifest.replace("@APPLICATION_ID@", application_id).replace("@LABEL@", label)
    (out / "AndroidManifest.xml").write_text(manifest, encoding="utf-8")
    res = out / "res"
    shutil.rmtree(res, ignore_errors=True)
    for folder, size in ICONS.items():
        target = res / folder
        target.mkdir(parents=True)
        shutil.copyfile(
            ROOT / "packaging" / "icons" / "hicolor" / size / "apps" / "horizon.png",
            target / "ic_launcher.png",
        )
    # The window themes (res/: the starting window's icon and background).
    shutil.copytree(HERE / "res", res, dirs_exist_ok=True)
    compiled = out / "compiled"
    shutil.rmtree(compiled, ignore_errors=True)
    compiled.mkdir()
    run(aapt2, "compile", "--dir", res, "-o", compiled)
    # Debuggable (adb's run-as reaches the app's own files) except when signed for a release.
    debug = [] if os.environ.get("HORIZON_ANDROID_KEYSTORE") else ["--debug-mode"]
    run(
        aapt2, "link", "-o", out / "base.apk",
        "--manifest", out / "AndroidManifest.xml",
        "-I", android_jar,
        "--min-sdk-version", MIN_SDK, "--target-sdk-version", TARGET_SDK,
        "--version-code", VERSION_CODE, "--version-name", VERSION_NAME,
        *debug,
        "-A", assets,
        *sorted(compiled.glob("*.flat")),
    )


def add_code(base, libraries, dex, unsigned):
    """`base` plus the activity's dex and the libraries, compressed: Android extracts them at
    install (extractNativeLibs), as it must for the Java launchers to run."""
    shutil.copyfile(base, unsigned)
    with zipfile.ZipFile(unsigned, "a") as apk:
        apk.write(dex, "classes.dex", compress_type=zipfile.ZIP_DEFLATED)
        for name, path in sorted(libraries.items()):
            apk.write(path, f"lib/{ABI}/{name}", compress_type=zipfile.ZIP_DEFLATED)


def sign(build_tools, aligned, apk, out):
    keystore = os.environ.get("HORIZON_ANDROID_KEYSTORE")
    alias = os.environ.get("HORIZON_ANDROID_KEY_ALIAS", "horizon-debug")
    env = dict(os.environ)
    if not keystore:
        keystore = out / "debug.keystore"
        env["HORIZON_ANDROID_STORE_PASSWORD"] = env["HORIZON_ANDROID_KEY_PASSWORD"] = "android"
        if not keystore.exists():
            run(
                "keytool", "-genkeypair", "-keystore", keystore, "-storepass", "android",
                "-keypass", "android", "-alias", alias, "-keyalg", "RSA", "-keysize", "2048",
                "-validity", "10000", "-dname", "CN=Horizon Android debug",
            )
    apksigner = tool(build_tools, "apksigner")
    run(
        apksigner, "sign", "--ks", keystore, "--ks-key-alias", alias,
        "--ks-pass", "env:HORIZON_ANDROID_STORE_PASSWORD",
        "--key-pass", "env:HORIZON_ANDROID_KEY_PASSWORD",
        "--out", apk, aligned, env=env,
    )
    run(apksigner, "verify", apk)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--profile", default="quick",
        help="Cargo profile (default: quick, for iteration; fast or release to measure and ship)",
    )
    parser.add_argument("--no-build", action="store_true", help="package the library built last")
    parser.add_argument("--install", action="store_true", help="adb install the APK")
    parser.add_argument(
        "--zip", action="store_true",
        help="also zip the APK, for chats that take no .apk",
    )
    parser.add_argument(
        "--keep-symbols", action="store_true", help="ship the library unstripped (backtraces)"
    )
    parser.add_argument(
        "--dev", action="store_true",
        help=f"Starling Dev ({APPLICATION_ID}{DEV_SUFFIX}), installed beside the release",
    )
    args = parser.parse_args()

    sdk = sdk_dir()
    ndk = Path(os.environ.get("ANDROID_NDK_HOME") or sdk / "ndk" / NDK)
    build_tools = sdk / "build-tools" / BUILD_TOOLS
    android_jar = sdk / "platforms" / f"android-{PLATFORM}" / "android.jar"
    for path in (ndk, build_tools, android_jar):
        if not path.exists():
            sys.exit(f"build_apk: missing {path}")

    if not args.no_build:
        build_js_engine()
        build_library(args.profile, ndk, sdk)
    profile_dir = "debug" if args.profile == "dev" else args.profile
    built = ROOT / "target" / TRIPLE / profile_dir / f"lib{LIBRARY}.so"
    if not built.is_file():
        sys.exit(f"build_apk: {built} not built")

    out = ROOT / "target" / "android"
    out.mkdir(parents=True, exist_ok=True)
    library = out / f"lib{LIBRARY}.so"
    if args.keep_symbols:
        shutil.copyfile(built, library)
    else:
        run(ndk_tool(ndk, "llvm-strip"), "--strip-unneeded", "-o", library, built)

    link_resources(tool(build_tools, "aapt2"), android_jar, out, assets(out), args.dev)
    dex = compile_java(build_tools, android_jar, out)
    libraries = build_launchers(ndk, out)
    libraries[library.name] = library
    add_code(out / "base.apk", libraries, dex, out / "unsigned.apk")
    aligned = out / "aligned.apk"
    run(tool(build_tools, "zipalign"), "-P", "16", "-f", "4", out / "unsigned.apk", aligned)
    apk = out / f"starling{'-dev' if args.dev else ''}-{VERSION_NAME}-{ABI}.apk"
    sign(build_tools, aligned, apk, out)
    print(apk)
    if args.zip:
        # Sent where an .apk is refused (a chat, a phone away from adb): unzip, then install.
        packed = apk.with_suffix(".apk.zip")
        with zipfile.ZipFile(packed, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
            z.write(apk, apk.name)
        print(packed)
    if args.install:
        adb = tool(sdk / "platform-tools", "adb")
        run(adb, "install", "-r", apk)


if __name__ == "__main__":
    main()
