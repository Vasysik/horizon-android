#!/usr/bin/env python3
"""Download the pinned vanilla inputs and prepare private Android resources on a PC.

Mojang's exporters require a desktop JDK. Game inputs stay outside Git;
local development APKs currently embed generated engine tables.
All downloads are checked against Mojang's hashes and the repository's pins.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request
import hashlib
import zipfile
import tempfile

import lab

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
RUNTIMES = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json"


def fetch_json(url):
    with urllib.request.urlopen(url, timeout=120) as response:
        return json.load(response)


def relative(base, value):
    path = (base / value).resolve()
    if not path.is_relative_to(base.resolve()):
        raise ValueError(f"Path escapes download directory: {value}")
    return path


def parallel_download(files):
    def one(item):
        url, path, checksum = item
        for attempt in range(4):
            try:
                lab.download(url, path, checksum)
                return
            except (OSError, ValueError):
                if attempt == 3:
                    raise
                time.sleep(1 + attempt)
    with ThreadPoolExecutor(max_workers=8) as pool:
        for _ in pool.map(one, files):
            pass


def download_game(version, directory):
    pins = json.loads((ROOT / 'research-lock.json').read_text())['minecraft']
    if version != pins['version']:
        raise ValueError(f"This engine implements {pins['version']}; {version} is unsupported")
    entry = next((v for v in fetch_json(MANIFEST)['versions'] if v['id'] == version), None)
    if entry is None:
        raise ValueError(f"Mojang does not list Minecraft {version}")
    # Mojang metadata may be republished: lab.download checks the catalog SHA-1,
    # and client/server jar hashes are checked against the engine pins below.
    versions = directory / 'versions' / version
    manifest_path = versions / f'{version}.json'
    lab.download(entry['url'], manifest_path, entry['sha1'])
    manifest = json.loads(manifest_path.read_text())
    files = []
    for side in ['client', 'server']:
        artifact = manifest['downloads'][side]
        if artifact['sha1'] != pins[f'{side}_sha1']:
            raise ValueError(f'{side} jar differs from the engine pin')
        files.append((artifact['url'], versions / f'{version}{"-server" if side == "server" else ""}.jar', artifact['sha1']))
    for artifact in lab.client_library_artifacts(manifest):
        files.append((artifact['url'], relative(directory / 'libraries', artifact['path']), artifact['sha1']))
    index = manifest['assetIndex']
    index_path = relative(directory / 'assets/indexes', f"{index['id']}.json")
    lab.download(index['url'], index_path, index['sha1'])
    objects = json.loads(index_path.read_text())['objects']
    for value in objects.values():
        checksum = value['hash']
        name = f'{checksum[:2]}/{checksum}'
        files.append((f'https://resources.download.minecraft.net/{name}', directory / 'assets/objects' / name, checksum))
    print(f'Downloading Minecraft {version}: {len(files)} verified files', flush=True)
    parallel_download(files)
    return manifest, versions / f'{version}-server.jar'


def download_jdk(manifest, directory):
    import platform
    key = {'Windows': 'windows-x64', 'Linux': 'linux', 'Darwin': 'mac-os-arm64' if platform.machine() == 'arm64' else 'mac-os'}[platform.system()]
    component = manifest['javaVersion']['component']
    entry = fetch_json(RUNTIMES)[key][component][0]['manifest']
    metadata = directory / 'runtime.json'
    lab.download(entry['url'], metadata, entry['sha1'])
    files = json.loads(metadata.read_text())['files']
    home = directory / component
    downloads = []
    for name, info in files.items():
        path = relative(home, name)
        if info['type'] == 'file':
            artifact = info['downloads']['raw']
            downloads.append((artifact['url'], path, artifact['sha1']))
    print(f'Downloading Mojang JDK: {component} ({len(downloads)} files)', flush=True)
    parallel_download(downloads)
    for name, info in files.items():
        path = relative(home, name)
        if info['type'] == 'file' and info.get('executable'):
            path.chmod(path.stat().st_mode | 0o111)
        elif info['type'] == 'link' and not path.exists():
            relative(home, str(Path(name).parent / info['target']))
            path.parent.mkdir(parents=True, exist_ok=True)
            path.symlink_to(info['target'])
    return home


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', default=lab.pinned_version())
    parser.add_argument('--java-home', type=Path)
    parser.add_argument('--bundle-only', action='store_true', help='Package already prepared resources without downloading or exporting again')
    args = parser.parse_args()
    private = ROOT / '.local/android'
    if args.bundle_only:
        package_resources(args.version, private)
        return
    minecraft = private / 'minecraft'
    manifest, server = download_game(args.version, minecraft)
    java_home = args.java_home or download_jdk(manifest, private / 'jdk')
    environment = dict(os.environ, JAVA_HOME=str(java_home.resolve()))
    isolated = Path(tempfile.mkdtemp(prefix='prepare-', dir=private))
    for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
        environment[key] = str(isolated / key.lower())
    environment.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1')
    subprocess.run(['cargo', 'run', '-p', 'horizon-prepare', '--', '--root', str(ROOT), '--prepare',
                    '--minecraft-dir', str(minecraft), '--server-jar', str(server)],
                   cwd=ROOT, env=environment, check=True)
    package_resources(args.version, private)


def package_resources(version, private):
    if version != lab.pinned_version():
        raise ValueError('Only the engine-pinned Minecraft version is supported')
    resources = ROOT / '.local/horizon-client' / version
    if not (resources / 'horizon-resources.json').is_file():
        raise FileNotFoundError('Prepare the resources before packaging them')
    serve = private / 'serve'
    serve.mkdir(parents=True, exist_ok=True)
    bundle = serve / f'{version}.zip'
    temporary = bundle.with_suffix('.partial')
    with zipfile.ZipFile(temporary, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
        for path in sorted(resources.rglob('*')):
            if path.is_file():
                archive.write(path, path.relative_to(resources).as_posix())
    temporary.replace(bundle)
    with bundle.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    config = {'version': version, 'url': f'http://127.0.0.1:8123/{bundle.name}', 'sha256': digest, 'size': bundle.stat().st_size}
    (serve / 'bootstrap.json').write_text(json.dumps(config, indent=2))
    print(f'Private resource bundle: {bundle}\nStart: python -m http.server 8123 --bind 127.0.0.1 --directory "{serve}"', flush=True)


if __name__ == '__main__':
    main()
