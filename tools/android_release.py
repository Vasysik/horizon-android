#!/usr/bin/env python3
"""Collect public release APKs, optionally compiling Rust targets concurrently.

Rust-only builds use independent caches; packaging is always serial because the
Gradle project/assets/output are shared. Public packaging reruns Cargo (normally
a no-op), retaining android_build.py's public-mode and provenance checks.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CACHES = {'arm64-v8a': 'release-arm64', 'armeabi-v7a': 'release-armv7',
          'x86': 'release-x86', 'x86_64': 'release-x86_64'}


def build_targets(abis, parallel, invoke):
    if parallel > 1:
        # No shared staging, Gradle or release-output mutations in this phase.
        with ThreadPoolExecutor(max_workers=parallel) as pool:
            list(pool.map(lambda abi: invoke(abi, True), abis))
    for abi in abis:
        invoke(abi, False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--abi', choices=CACHES, nargs='+', default=list(CACHES))
    parser.add_argument('--parallel', type=int, choices=(1, 2), default=1,
                        help='Concurrent Rust targets; 1 by default, 2 needs additional RAM')
    parser.add_argument('--jobs', type=int,
                        help='Cargo jobs per target; defaults to CPU threads / parallel targets')
    args = parser.parse_args()
    if args.jobs is not None and args.jobs < 1:
        parser.error('--jobs must be positive')
    abis = list(dict.fromkeys(args.abi))
    parallel = min(args.parallel, len(abis))
    jobs = args.jobs or max(1, (os.cpu_count() or 2) // parallel)
    version = re.search(r"versionName '([^']+)'", (ROOT / 'packaging/android/build.gradle').read_text()).group(1)
    output = ROOT / '.local/android/releases' / version
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT)

    def check_source():
        if subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT) != revision:
            raise RuntimeError('Source commit changed during the build; rebuild from one fixed commit')
        subprocess.run(['git', 'diff', '--quiet', 'HEAD', '--'], cwd=ROOT, check=True)

    check_source()
    stamp = output / 'BUILD_COMMIT.txt'
    selected = {f'Horizon-Android-{version}-{abi}.apk' for abi in abis}
    retained = [p for p in output.glob(f'Horizon-Android-{version}-*.apk') if p.name not in selected]
    if retained and (not stamp.is_file() or stamp.read_bytes() != revision):
        raise SystemExit('Other APKs in this release directory belong to an unverified commit. '
                         'Build all ABIs together or move the previous attachments first.')
    output.mkdir(parents=True, exist_ok=True)
    print(f'Release {version}: {parallel} Rust target(s), {jobs} Cargo jobs each. '
          'ThinLTO/release optimization settings are unchanged.', flush=True)

    def invoke(abi, rust_only):
        check_source()
        env = dict(os.environ)
        env['CARGO_TARGET_DIR'] = str(ROOT / '.local/android' / CACHES[abi])
        env['CARGO_BUILD_JOBS'] = str(jobs)
        command = [sys.executable, str(ROOT / 'tools/android_build.py'), '--abi', abi,
                   '--public', '--profile', 'release']
        if rust_only:
            command.append('--rust-only')
        print(f'[{abi}] {"Rust compilation" if rust_only else "Build verification and APK packaging"}', flush=True)
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        check_source()
        if not rust_only:
            shutil.copy2(ROOT / '.local/android/apk' / abi / 'horizon-android-public.apk',
                         output / f'Horizon-Android-{version}-{abi}.apk')

    build_targets(abis, parallel, invoke)
    check_source()
    shutil.copy2(ROOT / 'LICENSE', output / 'LICENSE.txt')
    shutil.copy2(ROOT / 'THIRD_PARTY_NOTICES.md', output / 'THIRD_PARTY_NOTICES.md')
    sums = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n'
                   for p in sorted(output.glob(f'Horizon-Android-{version}-*.apk')))
    (output / 'SHA256SUMS.txt').write_text(sums, encoding='utf-8')
    stamp.write_bytes(revision)
    print(f'Release attachments: {output}')


if __name__ == '__main__':
    main()
