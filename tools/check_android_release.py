#!/usr/bin/env python3
"""Audit a release-candidate APK's payload and require a non-debuggable Android manifest.

This is a bounded credential/data check, not a guarantee against every secret
format. Reports contain counts and paths, never suspected credential values.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import zipfile

ASSETS = {'assets/engine-data.zip', 'assets/bedrock-controls.zip'}
# AndroidX build-time optimization profiles, not player/account profiles.
BUILD_ASSETS = {'assets/dexopt/baseline.prof', 'assets/dexopt/baseline.profm'}
LICENSE_ASSETS = {'assets/LICENSE.txt', 'assets/THIRD_PARTY_NOTICES.md'}
PRIVATE_PARTS = {'accounts', 'accounts.json', 'session.json', 'session-file.json',
                 'control.json', 'servers.dat', 'level.dat', 'options.txt',
                 'android-launch.json', 'android-launcher.json', 'saves', 'profiles',
                 'logs', 'crash-reports', 'credentials', 'key.properties'}
PRIVATE_SUFFIXES = {'.keystore', '.jks', '.p12', '.pfx', '.log', '.hprof'}
PATTERNS = {
    'private_key': re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'),
    'github_token': re.compile(rb'gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{60,}'),
    'aws_key': re.compile(rb'AKIA[A-Z0-9]{16}'),
    'jwt': re.compile(rb'eyJ[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}'),
    'saved_token': re.compile(rb'"(?:access_token|refresh_token|minecraft_token)"\s*:\s*"[A-Za-z0-9._~+/-]{24,}"'),
    'personal_build_path': re.compile(rb'[A-Za-z]:[\\/]+Users[\\/]+[^\\/\x00\r\n ]+[\\/]'),
}


def inspect_payload(path, public=False):
    findings = []
    totals = {'entries': 0, 'uncompressed_bytes': 0}

    def walk(archive, prefix='', depth=0):
        if depth > 3:
            raise ValueError('Unexpected archive nesting')
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError('Duplicate archive entries')
        for entry in archive.infolist():
            name = entry.filename
            full = prefix + name
            parts = PurePosixPath(name).parts
            if '\\' in name or name.startswith('/') or '..' in parts or re.match(r'^[A-Za-z]:', name):
                raise ValueError('Unsafe archive path')
            if entry.flag_bits & 1 or (entry.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError('Encrypted or symbolic-link archive entry')
            if entry.is_dir():
                continue
            totals['entries'] += 1
            totals['uncompressed_bytes'] += entry.file_size
            if entry.file_size > 256 * 1024 * 1024 or totals['uncompressed_bytes'] > 1024 * 1024 * 1024:
                raise ValueError('APK exceeds audit decompression limits')
            if any(part.lower() in PRIVATE_PARTS for part in parts) or PurePosixPath(name).suffix.lower() in PRIVATE_SUFFIXES:
                findings.append({'path': full, 'kind': 'private_file'})
            if depth == 0 and name.startswith('assets/') and name not in ASSETS | BUILD_ASSETS | LICENSE_ASSETS:
                findings.append({'path': full, 'kind': 'unexpected_asset'})
            data = archive.read(entry)
            for kind, pattern in PATTERNS.items():
                if pattern.search(data):
                    findings.append({'path': full, 'kind': kind})
            if name.lower().endswith('.zip'):
                with zipfile.ZipFile(io.BytesIO(data)) as nested:
                    if public and name == 'assets/engine-data.zip':
                        from android_build import generated_engine_file
                        for item in nested.namelist():
                            if not generated_engine_file(item):
                                findings.append({'path': full + '!/' + item, 'kind': 'original_game_resource'})
                    walk(nested, full + '!/', depth + 1)

    with zipfile.ZipFile(path) as archive:
        missing = ({'assets/engine-data.zip'} if public else ASSETS).difference(archive.namelist())
        if missing:
            raise ValueError('Required resource archives are absent')
        if public and 'assets/bedrock-controls.zip' in archive.namelist():
            raise ValueError('Public APK must download original control textures, not bundle them')
        walk(archive)
    return {**totals, 'findings': findings}


def audit(path, aapt2, public=False):
    report = inspect_payload(path, public)
    badging = subprocess.check_output([str(aapt2), 'dump', 'badging', str(path)], text=True)
    if "package: name='dev.horizon.android'" not in badging:
        raise ValueError('Unexpected Android package identity')
    report['debuggable'] = 'application-debuggable' in badging
    if report['debuggable']:
        report['findings'].append({'path': 'AndroidManifest.xml', 'kind': 'debuggable'})
    report['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    report['bytes'] = path.stat().st_size
    report['passed'] = not report['findings']
    report['payload_mode'] = 'generated-tables-and-official-downloads' if public else 'private-development'
    report['bundled_game_resources'] = ['assets/engine-data.zip (generated tables)'] if public else sorted(ASSETS)
    report['distribution_clearance'] = 'Technical payload audit; no legal certification.'
    report['scope'] = 'Archive paths, packaged assets, known credential patterns, personal compiler paths and manifest debuggability; no exhaustive secret-format guarantee.'
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('apk', type=Path)
    parser.add_argument('--aapt2', required=True, type=Path)
    parser.add_argument('--report', type=Path)
    parser.add_argument('--public', action='store_true')
    args = parser.parse_args()
    report = audit(args.apk, args.aapt2, args.public)
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
