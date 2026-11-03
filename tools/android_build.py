#!/usr/bin/env python3
"""Build an optimized Rust APK with Android's stock NativeActivity and Rust system-IME adapter.

Requires Android SDK, NDK, Rust Android target, and a JDK (for APK signing).
Generated APKs and the throwaway signing key stay in .local/android.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import zipfile
import json
import hashlib

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {'x86_64': 'x86_64-linux-android', 'arm64-v8a': 'aarch64-linux-android',
           'armeabi-v7a': 'armv7-linux-androideabi', 'x86': 'i686-linux-android'}


def latest(directory):
    return max(directory.iterdir(), key=lambda p: tuple(int(v) for v in p.name.split('.')))


def run(command, **kwargs):
    print(' '.join(map(str, command)), flush=True)
    subprocess.run(list(map(str, command)), check=True, **kwargs)


def save_crash_symbols(library, stripped, abi, objcopy, revision):
    """Keep matching symbols locally; never enlarge or attach them to the APK."""
    digest = hashlib.sha256(stripped.read_bytes()).hexdigest()
    notes = subprocess.check_output([str(objcopy.with_name('llvm-readelf' + objcopy.suffix)), '--notes', str(stripped)], text=True)
    build_id = notes.split('Build ID:', 1)[1].split()[0] if 'Build ID:' in notes else None
    key = build_id or digest
    output = ROOT / '.local/android/native-symbols' / abi
    output.mkdir(parents=True, exist_ok=True)
    symbols = output / (key + '.debug')
    if not symbols.is_file():
        run([objcopy, '--only-keep-debug', library, symbols])
    (output / (key + '.json')).write_text(json.dumps({
        'schema': 1, 'abi': abi, 'source_commit': revision, 'build_id': build_id,
        'packaged_library_sha256': digest, 'symbols': symbols.name,
    }, indent=2) + '\n', encoding='utf-8')
    print(f'Private crash symbols: {symbols}', flush=True)


def generated_engine_file(name):
    path = Path(name)
    if len(path.parts) == 1:
        return name in {
            'attributes.json', 'block-behaviours.json', 'cached-collision.nbt',
            'command-registries.json', 'commands.json', 'creative-inventory.json',
            'damage-catalog.json', 'entity-models.json', 'entity-types.json',
            'facts.json', 'gameplay-facts.json', 'horizon-resources.json', 'items.json',
            'landing-definitions.json', 'mob-data.json', 'model-states.json',
            'movement-properties.nbt', 'occlusion.nbt', 'player-float-ids.json',
            'registries.json', 'render-definitions.json'}
    return (name in {'biome-parameters/nether.json', 'biome-parameters/overworld.json',
                    'worldgen/biome_parameters/nether.json', 'worldgen/biome_parameters/overworld.json'}
            or (path.parts[:2] == ('worldgen', 'blocks') and path.name in {
                'block-properties.nbt', 'cached-collision.nbt', 'cached-support.nbt',
                'survival-rules.json', 'survival-extra-definitions.json', 'placement-definitions.json',
                'structure-blocks.json'}
                and len(path.parts) == 3)
            or name == 'worldgen/block_flags.json')


def engine_data(public=False):
    version = json.loads((ROOT / 'research-lock.json').read_text())['minecraft']['version']
    resources = ROOT / '.local/horizon-client' / version
    if not (resources / 'horizon-resources.json').is_file():
        raise FileNotFoundError('Developer build needs prepared engine tables; run tools/android_prepare.py once')
    if public and not (resources / 'worldgen/blocks/structure-blocks.json').is_file():
        raise FileNotFoundError('Public build needs generated structure transforms; run tools/android_prepare.py again')
    files = [p for p in resources.iterdir() if p.is_file() and p.suffix in ('.json', '.nbt') and p.name != 'asset-index.json']
    for name in ['worldgen', 'biome-parameters']:
        files.extend(p for p in (resources / name).rglob('*') if p.is_file())
    files.sort()
    if public:
        files = [p for p in files if generated_engine_file(p.relative_to(resources).as_posix())]
    fingerprint = hashlib.sha256(''.join(f'{p.relative_to(resources)}:{p.stat().st_size}:{p.stat().st_mtime_ns}\n' for p in files).encode()).hexdigest()
    output = ROOT / '.local/android' / ('engine-data-generated.zip' if public else 'engine-data.zip')
    stamp = output.with_suffix('.fingerprint')
    if not output.exists() or not stamp.exists() or stamp.read_text() != fingerprint:
        output.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(output.with_suffix('.partial'), 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
            for file in files:
                archive.write(file, file.relative_to(resources).as_posix())
        output.with_suffix('.partial').replace(output)
        stamp.write_text(fingerprint)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--abi', choices=TARGETS, default='arm64-v8a')
    parser.add_argument('--sdk', type=Path, default=os.environ.get('ANDROID_HOME') or Path(os.environ.get('LOCALAPPDATA', str(Path.home()))) / 'Android/Sdk')
    parser.add_argument('--java-home', type=Path, default=os.environ.get('JAVA_HOME') or 'C:/Program Files/Android/Android Studio/jbr')
    parser.add_argument('--skip-rust', action='store_true')
    parser.add_argument('--rust-only', action='store_true', help='Build the Rust host without touching shared APK staging')
    parser.add_argument('--release-candidate', action='store_true', help='Build a non-debuggable candidate and audit privacy; this does not authorize redistribution of bundled game resources')
    parser.add_argument('--public', action='store_true', help='Distribution APK: generated tables only; original resources download from Mojang')
    parser.add_argument('--profile', choices=['release', 'fast', 'dev'], help='Public/candidate builds default to release (ThinLTO); local builds default to optimized fast')
    parser.add_argument('--gradle', type=Path, help='Gradle 8.14 executable (otherwise discover the cached distribution)')
    parser.add_argument('--check', action='store_true', help='Only type-check the Android target')
    parser.add_argument('--clippy', action='store_true', help='Run Android Clippy with warnings denied')
    parser.add_argument('--bootstrap', type=Path, help=argparse.SUPPRESS)  # Old scripts: no longer used.
    args = parser.parse_args()
    if args.profile is None:
        args.profile = 'release' if args.public or args.release_candidate else 'fast'
    if args.public:
        args.release_candidate = True
        if args.skip_rust:
            parser.error('--public cannot reuse a native library from a different build mode')
    if args.release_candidate and args.profile == 'dev':
        parser.error('--release-candidate requires an optimized release or fast profile')
    suffix = '.exe' if os.name == 'nt' else ''
    tools = latest(args.sdk / 'build-tools')
    ndk = latest(args.sdk / 'ndk') / 'toolchains/llvm/prebuilt' / ('windows-x86_64' if os.name == 'nt' else 'linux-x86_64')
    target = TARGETS[args.abi]
    # NDK spells the ARMv7 compiler triple differently from Rust. Android 8.1
    # is the legacy build's floor; the modern builds keep their existing floor.
    compiler_target = 'armv7a-linux-androideabi' if args.abi == 'armeabi-v7a' else target
    legacy = args.abi in ('armeabi-v7a', 'x86')
    min_sdk = 27 if legacy else 28
    features = ['--no-default-features', '--features', 'audio'] if legacy else []
    manifests = [(['-p', 'horizon-android'], features)]
    environment = dict(os.environ)
    environment['PATH'] = os.pathsep.join([str(ndk / 'bin'), str(args.java_home / 'bin'), environment['PATH']])
    environment['JAVA_HOME'] = str(args.java_home)
    target_dir = Path(environment.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
    environment['CARGO_TARGET_DIR'] = str(target_dir)
    triple = target.replace('-', '_')
    clang = ndk / 'bin' / (compiler_target + str(min_sdk) + '-clang' + ('.cmd' if os.name == 'nt' else ''))
    environment[f'CC_{triple}'] = str(clang)
    environment[f'CXX_{triple}'] = str(ndk / 'bin' / (compiler_target + str(min_sdk) + '-clang++' + ('.cmd' if os.name == 'nt' else '')))
    environment[f'AR_{triple}'] = str(ndk / 'bin' / ('llvm-ar' + suffix))
    environment[f'CARGO_TARGET_{triple.upper()}_LINKER'] = str(clang)
    environment['CARGO_PROFILE_DEV_DEBUG'] = '0'
    environment['HORIZON_ANDROID_PUBLIC'] = '1' if args.public else '0'
    try:
        environment['HORIZON_ANDROID_BUILD_COMMIT'] = subprocess.check_output(
            ['git', 'rev-parse', '--short=12', 'HEAD'], cwd=ROOT,
            stderr=subprocess.DEVNULL, text=True).strip()
        environment['HORIZON_ANDROID_BUILD_DIRTY'] = str(bool(subprocess.check_output(
            ['git', 'status', '--porcelain', '--untracked-files=no'], cwd=ROOT,
            stderr=subprocess.DEVNULL, text=True).strip())).lower()
    except (OSError, subprocess.CalledProcessError):
        environment['HORIZON_ANDROID_BUILD_COMMIT'] = 'source-export'
        environment['HORIZON_ANDROID_BUILD_DIRTY'] = 'unknown'
    if args.release_candidate:
        flags = environment.get('CARGO_ENCODED_RUSTFLAGS', '').split('\x1f') if environment.get('CARGO_ENCODED_RUSTFLAGS') else environment.get('RUSTFLAGS', '').split()
        for original, replacement in [(Path.home(), '/build'), (ROOT, '/horizon')]:
            for prefix in dict.fromkeys([str(original), original.as_posix()]):
                flags.append(f'--remap-path-prefix={prefix}={replacement}')
        environment['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join(flags)
    if args.check or args.clippy:
        for selection, host_features in manifests:
            command = ['cargo', 'clippy' if args.clippy else 'check', *selection, '--target', target, *host_features]
            if args.clippy:
                command += ['--no-deps', '--', '-D', 'warnings']
            run(command, cwd=ROOT, env=environment)
        return
    if not args.skip_rust:
        for selection, host_features in manifests:
            run(['cargo', 'build', *selection, '--target', target, '--profile', args.profile, *host_features], cwd=ROOT, env=environment)
    if args.rust_only:
        return
    output = ROOT / '.local/android/apk' / args.abi
    output.mkdir(parents=True, exist_ok=True)
    staging = ROOT / '.local/android/gradle-jni' / args.abi
    staging.mkdir(parents=True, exist_ok=True)
    (staging / 'libhorizon_android_native.so').unlink(missing_ok=True)
    for name in ('libhorizon_android.so',):
        library = target_dir / target / ('debug' if args.profile == 'dev' else args.profile) / name
        if not library.is_file():
            raise FileNotFoundError(library)
        shutil.copyfile(library, staging / name)
        run([ndk / 'bin' / ('llvm-strip' + suffix), '--strip-unneeded', staging / name])
        if args.profile == 'release':
            save_crash_symbols(library, staging / name, args.abi,
                               ndk / 'bin' / ('llvm-objcopy' + suffix),
                               environment.get('HORIZON_ANDROID_BUILD_COMMIT', 'unknown'))
    assets = ROOT / '.local/android/gradle-assets'
    assets.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / 'LICENSE', assets / 'LICENSE.txt')
    shutil.copyfile(ROOT / 'THIRD_PARTY_NOTICES.md', assets / 'THIRD_PARTY_NOTICES.md')
    shutil.copyfile(engine_data(args.public), assets / 'engine-data.zip')
    controls = ROOT / '.local/android/bedrock-controls.zip'
    if not args.public and not controls.is_file():
        import sys
        run([sys.executable, ROOT / 'tools/android_bedrock_controls.py'], cwd=ROOT)
    if args.public:
        (assets / 'bedrock-controls.zip').unlink(missing_ok=True)
    else:
        shutil.copyfile(controls, assets / 'bedrock-controls.zip')
    key = ROOT / '.local/android/debug.keystore'
    if not key.exists():
        run([args.java_home / 'bin' / ('keytool' + suffix), '-genkeypair', '-keystore', key, '-storepass', 'android', '-keypass', 'android', '-alias', 'androiddebugkey', '-keyalg', 'RSA', '-validity', '3650', '-dname', 'CN=Horizon Android Development'])
    gradle = args.gradle
    if gradle is None:
        candidates = list((Path.home() / '.gradle/wrapper/dists/gradle-8.14-all').glob('*/gradle-8.14/bin/gradle' + ('.bat' if os.name == 'nt' else '')))
        if not candidates:
            raise FileNotFoundError('Install Gradle 8.14 or pass --gradle PATH')
        gradle = candidates[0]
    environment['ANDROID_HOME'] = str(args.sdk)
    variant = 'release' if args.release_candidate else 'debug'
    run([gradle, '--no-daemon', '-p', ROOT / 'packaging/android', '-Pabi=' + args.abi, '-PminSdk=' + str(min_sdk), 'assemble' + variant.title()], cwd=ROOT, env=environment)
    apk = output / ('horizon-android-public.apk' if args.public else 'horizon-android-release.apk' if args.release_candidate else 'horizon-android.apk')
    shutil.copyfile(ROOT / f'.local/android/gradle-build/outputs/apk/{variant}/HorizonAndroid-{variant}.apk', apk)
    # Invoke the signing JAR directly so Windows does not need a batch-file shell.
    signer = [args.java_home / 'bin' / ('java' + suffix), '-jar', tools / 'lib/apksigner.jar']
    run([*signer, 'verify', '--verbose', apk])
    if args.release_candidate:
        import sys
        run([sys.executable, ROOT / 'tools/check_android_release.py', apk, '--aapt2', tools / ('aapt2' + suffix), *(['--public'] if args.public else [])], cwd=ROOT)
    print(f'APK: {apk}', flush=True)


if __name__ == '__main__':
    main()
