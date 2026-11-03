#!/usr/bin/env python3
"""Freeze the pinned, SIL OFL Noto Sans SC variable font at weight 400.

Download google/fonts at 2fe1e8c6b137fb13041a7cfca6fdcd8a910ea4dc, ofl/notosanssc/NotoSansSC[wght].ttf,
and pass the local file. The full Unicode coverage is kept for player names
and imported filenames, not just the translated UI strings.
Requires fontTools 4.68.0; the font and its license are recorded in
THIRD_PARTY_NOTICES.md.
"""
import argparse
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE_SHA256 = 'a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    args = parser.parse_args()
    if hashlib.sha256(args.source.read_bytes()).hexdigest() != SOURCE_SHA256:
        parser.error('Source font does not match the pinned SHA-256')
    from fontTools.ttLib import TTFont
    from fontTools.varLib.instancer import instantiateVariableFont
    font = instantiateVariableFont(TTFont(args.source), {'wght': 400}, inplace=True)
    font.recalcTimestamp = False
    output = ROOT / 'crates/horizon-theme/fonts/NotoSansSC-Regular.ttf'
    font.save(output)
    print(f'{output}: {output.stat().st_size} bytes')


if __name__ == '__main__':
    main()
