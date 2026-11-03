#!/usr/bin/env python3
"""Prepare original Bedrock control textures privately; never vendor them in Git.

Source is Mojang/bedrock-samples at the exact revision below. Its LICENSE.md
retains Mojang's EULA terms; these assets are not covered by Horizon's MIT license.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REVISION = '46ba6ea985fb5a92d79a9419198f10dda14c199d'
REPOSITORY = 'https://github.com/Mojang/bedrock-samples.git'
UI = ['attack', 'interact', 'jump', 'sneak', 'sprint']
UI_FILES = [name + suffix + '.png' for name in UI for suffix in ['', '_pressed']]
UI_FILES += ['joystick_frame.png', 'joystick_knob.png', 'pause_icon.png',
             'wysiwyg_reset.png', 'text_label_box.png', 'slider_background.png',
             'slider_button_default.png', 'button_borderless_light.png', 'more-dots.png', 'gear.png']


def main():
    work = ROOT / '.local/android/bedrock-reference/samples'
    if not (work / '.git').is_dir():
        subprocess.run(['git', 'clone', '--filter=blob:none', '--no-checkout', REPOSITORY, str(work)], check=True)
    git = ['git', '-c', f'safe.directory={work.as_posix()}', '-C', str(work)]
    subprocess.run([*git, 'fetch', '--depth', '1', 'origin', REVISION], check=True)
    subprocess.run([*git, 'sparse-checkout', 'set', 'resource_pack/ui',
                    'resource_pack/textures/gui', 'resource_pack/textures/ui'], check=True)
    subprocess.run([*git, 'checkout', '--detach', REVISION], check=True)
    resources = work / 'resource_pack/textures'
    files = list((resources / 'gui/controls').glob('*.png'))
    files += [resources / 'ui' / name for name in UI_FILES]
    files += [resources / 'gui/touchgui.png', resources / 'gui/touchgui2.png', resources / 'gui/gui.png']
    records = []
    output = ROOT / '.local/android/bedrock-controls.zip'
    with zipfile.ZipFile(output.with_suffix('.partial'), 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(files):
            relative = path.relative_to(resources).as_posix()
            name = relative.replace('gui/controls/', 'controls/').replace('gui/', '')
            data = path.read_bytes()
            expected = subprocess.check_output([*git, 'rev-parse', f'{REVISION}:resource_pack/textures/{relative}'], text=True).strip()
            actual = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            if actual != expected:
                raise ValueError(f'Upstream file differs from pinned Git object: {relative}')
            archive.writestr(name, data)
            records.append({'path': name, 'source': relative, 'git_blob': expected,
                            'sha256': hashlib.sha256(data).hexdigest()})
        archive.writestr('LICENSE.md', (work / 'LICENSE.md').read_bytes())
        archive.writestr('provenance.json', json.dumps({'repository': REPOSITORY,
                          'revision': REVISION, 'files': records}, indent=2))
    output.with_suffix('.partial').replace(output)
    print(f'Prepared {len(records)} original Bedrock textures in {output}')


if __name__ == '__main__':
    main()
