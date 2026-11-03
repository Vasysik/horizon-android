#!/usr/bin/env python3
"""Export reviewable Android fork sources without requiring a local Git checkout.

Does not initialize Git, publish anything or include generated game data/APKs.
Upstream workflows are retained as inactive reference files in the export.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

from check_public_tree import prohibited

ROOT = Path(__file__).resolve().parents[1]
DIRECTORIES = {'.githooks', '.github', 'benchmarks', 'crates', 'docs', 'launcher',
               'mods', 'packaging', 'packs', 'tests', 'third_party', 'tools', 'website', 'xtask'}
FILES = {'.dockerignore', '.gitattributes', '.gitignore', 'AGENTS.md', 'CLAUDE.md',
         'Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'CONTRIBUTING.md', 'CONTRIBUTING_ANDROID.md', 'LICENSE', 'README.md',
         'research-lock.json', 'THIRD_PARTY_NOTICES.md'}
SKIP = {'.git', '.local', 'target', 'build', 'dist', 'logs', 'node_modules',
        '__pycache__', '.gradle', '.venv', '.horizon-cache'}
SECRET = re.compile(rb'(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{60,}|'
                    rb'AKIA[A-Z0-9]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)')
BINARY = {'.png', '.webp', '.jpg', '.jpeg', '.gif', '.ttf', '.otf', '.woff', '.woff2', '.ico'}


def candidates(root):
    def descend(directory):
        for path in sorted(directory.iterdir()):
            if path.is_symlink():
                raise ValueError(f'Symlink requires manual review: {path.relative_to(root)}')
            if path.is_dir():
                if path.name not in SKIP:
                    yield from descend(path)
            elif path.is_file():
                relative = path.relative_to(root).as_posix()
                if not prohibited(relative) and path.suffix.lower() not in {'.pyc', '.pyo'}:
                    yield relative, path
    for name in sorted(FILES):
        path = root / name
        if path.is_symlink():
            raise ValueError(f'Symlink requires manual review: {name}')
        if path.is_file():
            yield name, path
    for name in sorted(DIRECTORIES):
        if (root / name).is_symlink():
            raise ValueError(f'Symlink requires manual review: {name}')
        if (root / name).is_dir():
            yield from descend(root / name)


def export(root, output):
    entries = []
    payload = []
    inactive = []
    for relative, path in candidates(root):
        data = path.read_bytes()
        if len(data) > 10 * 1024 * 1024:
            raise ValueError(f'Unexpected large source file: {relative}')
        if path.suffix.lower() not in BINARY and SECRET.search(data):
            raise ValueError(f'Possible credential; review file before export: {relative}')
        destination = relative
        if relative.startswith('.github/workflows/'):
            destination = '.github/upstream-workflows/' + path.name + '.disabled'
            inactive.append(relative)
        payload.append((destination, data))
        entries.append({'path': destination, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    required = {'LICENSE', 'THIRD_PARTY_NOTICES.md', 'Cargo.toml', 'README.md'}
    if not required.issubset({e['path'] for e in entries}):
        raise ValueError('Missing source/license files')
    workflow = root / 'packaging/android/source-checks.yml'
    if workflow.is_file():
        data = workflow.read_bytes()
        name = '.github/workflows/android-source.yml'
        payload.append((name, data))
        entries.append({'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    provenance = root / 'docs/android-upstream.json'
    revisions = json.loads(provenance.read_text(encoding='utf-8')) if provenance.is_file() else {}
    base = revisions.get('merged_upstream_commit') or revisions.get('base_commit')
    manifest = {'format': 1, 'upstream': 'https://github.com/ludovic111/horizon',
                'upstream_revision': base, 'history': 'Preserved upstream history' if base else 'Source snapshot',
                'inactive_upstream_workflows': inactive, 'files': entries}
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix('.partial')
    with zipfile.ZipFile(temporary, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for name, data in payload:
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 24, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            # Preserve executable scripts/hooks on Unix checkouts.
            executable = name.startswith('.githooks/') or name.endswith('.sh')
            info.external_attr = (0o100755 if executable else 0o100644) << 16
            archive.writestr(info, data)
        archive.writestr('SOURCE-MANIFEST.json', json.dumps(manifest, indent=2) + '\n')
    temporary.replace(output)
    report = {'archive': output.name, 'bytes': output.stat().st_size,
              'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
              'files': len(entries), 'upstream_workflows_disabled': len(inactive),
              'secret_patterns_checked': True, 'history_recovered': base is not None}
    output.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / '.local/android/public/horizon-android-source-0.5.0.zip')
    args = parser.parse_args()
    print(json.dumps(export(ROOT, args.output.resolve()), indent=2))


if __name__ == '__main__':
    main()
