"""Starling's touch buttons in a Bedrock-like look, drawn from scratch: a 24x24 stone-grey tile
(a speckled face, a black outline with cut corners, a 2-pixel bevel lit at the top left) and a
thick white icon outlined in dark grey; pressed, the face darkens, the bevel turns over and the
icon sinks a pixel. The sprites go to crates/horizon-client/assets/touch, which the game carries
as `starling:touch/<name>` (hud.rs `TOUCH_SPRITES`; a resource pack's own win).

    python packaging/touch/make.py                     # the sprites
    python packaging/touch/make.py SCREENSHOT OUTDIR   # and a sheet and a preview, to look at
"""
import random
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates" / "horizon-client" / "assets" / "touch"
T = 24  # a button's tile
STICK = 48  # the stick's base

OUTLINE = (16, 16, 16, 245)
ICON = (255, 255, 255, 255)
ICON_EDGE = (38, 38, 38, 255)


def face(pressed, alpha):
    """The face's grey and its bevel's, up and pressed."""
    if pressed:
        return dict(face=118, light=(78, 92), dark=(176, 158), alpha=alpha)
    return dict(face=150, light=(222, 196), dark=(92, 112), alpha=alpha)


def plate(size, pressed, alpha=226, seed=7):
    """A bevelled stone plate: outline, two rings of bevel, a speckled face."""
    look = face(pressed, alpha)
    rng = random.Random(seed + (1000 if pressed else 0))
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    px = img.load()
    last = size - 1
    for y in range(size):
        for x in range(size):
            # Cut corners: the outline's corner pixels go, and the bevel's step in.
            if (x in (0, last)) and (y in (0, last)):
                continue
            ring = min(x, y, last - x, last - y)
            if ring == 0:
                px[x, y] = OUTLINE
                continue
            if ring in (1, 2) and (x in (1, last - 1)) and (y in (1, last - 1)):
                px[x, y] = OUTLINE
                continue
            top_left = (x + y) < last
            if ring <= 2:
                shade = look["light"][ring - 1] if top_left else look["dark"][ring - 1]
                px[x, y] = (shade, shade, shade, min(255, look["alpha"] + 20))
            else:
                g = look["face"] + rng.choice((-14, -8, -4, 0, 0, 0, 4, 8, 10))
                px[x, y] = (g, g, g, look["alpha"])
    return img


ICONS = {
    "jump": """
        .......##.......
        ......####......
        .....######.....
        ....########....
        ...##########...
        ..############..
        .....######.....
        .....######.....
        .....######.....
        .....######.....
        ................
        ..############..
        ..############..
    """,
    "sneak": """
        ..############..
        ..############..
        ................
        .....######.....
        .....######.....
        .....######.....
        .....######.....
        ..############..
        ...##########...
        ....########....
        .....######.....
        ......####......
        .......##.......
    """,
    "attack": """
        .............###
        ............####
        ...........####.
        ..........####..
        .........####...
        ........####....
        .......####.....
        ..##..####......
        ...######.......
        ....####........
        ...######.......
        ..###..##.......
        .###............
        ###.............
        ##..............
    """,
    "use": """
        ......####......
        ......####......
        ......####......
        ......####......
        ................
        ####...##...####
        ####..####..####
        ####..####..####
        ####...##...####
        ................
        ......####......
        ......####......
        ......####......
        ......####......
    """,
    "inventory": """
        ###..###..###
        ###..###..###
        ###..###..###
    """,
    "chat": """
        .############.
        ##############
        ##..........##
        ##.########.##
        ##..........##
        ##.######...##
        ##..........##
        ##############
        .#####..######
        ....###.......
        ....##........
        ....#.........
    """,
    "pause": """
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
        ..####..####..
    """,
    "drop": """
        .....####.....
        .....####.....
        .....####.....
        .....####.....
        ..##########..
        ...########...
        ....######....
        .....####.....
        ......##......
        ..............
        ##############
        ##############
    """,
    "offhand": """
        ....##..........
        ...###..........
        ..##############
        .###############
        ..##############
        ...###..........
        ....##..........
        ................
        ..........##....
        ..........###...
        ##############..
        ###############.
        ##############..
        ..........###...
        ..........##....
    """,
    "camera": """
        .....######.....
        ...##########...
        .####......####.
        ###....##....###
        ##....####....##
        ###....##....###
        .####......####.
        ...##########...
        .....######.....
    """,
    "command": """
        ..........###
        .........###.
        ........###..
        .......###...
        ......###....
        .....###.....
        ....###......
        ...###.......
        ..###........
        .###.........
        ###..........
    """,
    "debug": """
        ######.#####.
        ######.######
        ##.........##
        ##.........##
        #####...####.
        #####...####.
        ##.........##
        ##.........##
        ##.....######
        ##.....#####.
    """,
    "pick": """
        ......#......
        ....##.##....
        ..##.....##..
        ##.........##
        #.##.....##.#
        #...##.##...#
        #.....#.....#
        #.....#.....#
        #.....#.....#
        ##....#....##
        ..##..#..##..
        ....##.##....
        ......#......
    """,
    "players": """
        ###.#########
        ###.#########
        .............
        ###.#########
        ###.#########
        .............
        ###.#########
        ###.#########
    """,
    "up": """
        .......##.......
        ......####......
        .....######.....
        ....########....
        ...##########...
        ..############..
        .##############.
        ################
    """,
}

# What a custom button can wear (settings.rs `CUSTOM_ICONS`): `custom_<name>` and its `_pressed`.
# Drawn on a coarse grid and doubled: strokes two pixels wide, as the F3 button's letters.
CUSTOM_ICONS = {
    "star": """
        ...#...
        ...#...
        #######
        .#####.
        ..###..
        .##.##.
        .#...#.
    """,
    "heart": """
        .##.##.
        #######
        #######
        .#####.
        ..###..
        ...#...
    """,
    "shield": """
        #######
        #######
        #######
        #######
        .#####.
        ..###..
        ...#...
    """,
    "pickaxe": """
        .#####..
        #....##.
        .....###
        ....##.#
        ...##..#
        ..##...#
        .##.....
        ##......
    """,
    "potion": """
        ..###..
        ...#...
        ...#...
        ..###..
        .#####.
        #######
        #######
        .#####.
    """,
    "apple": """
        ....#..
        ...#...
        .##.##.
        #######
        #######
        #######
        .#####.
    """,
    "home": """
        ...##...
        ..####..
        .######.
        ########
        .##..##.
        .##..##.
        .##..##.
    """,
    "flag": """
        ##.....
        #######
        #######
        #######
        ##.....
        ##.....
        ##.....
        ##.....
    """,
    "fire": """
        ...#...
        ..##...
        ..###.#
        .#####.
        #######
        ###.###
        ##...##
        .#####.
    """,
    "skull": """
        .#####.
        #######
        #..#..#
        #..#..#
        #######
        .#####.
        .#.#.#.
    """,
    "crown": """
        #..##..#
        #..##..#
        ##.##.##
        ########
        ########
        ########
    """,
    "plus": """
        ..##..
        ..##..
        ######
        ######
        ..##..
        ..##..
    """,
    "check": """
        ......##
        .....##.
        ##..##..
        .####...
        ..##....
        ..#.....
    """,
    "cross": """
        ##..##
        .####.
        ..##..
        ..##..
        .####.
        ##..##
    """,
    "music": """
        ..#####
        ..#####
        ..#...#
        ..#...#
        ###.###
        ###.###
        ###.###
    """,
}


def doubled(art):
    rows = [r.strip() for r in art.strip("\n").splitlines() if r.strip()]
    return "\n".join("".join(c * 2 for c in row) + "\n" + "".join(c * 2 for c in row) for row in rows)


ICONS.update({f"custom_{k}": doubled(v) for k, v in CUSTOM_ICONS.items()})


def bitmap(name):
    rows = [r.strip() for r in ICONS[name].strip("\n").splitlines() if r.strip()]
    width = max(len(r) for r in rows)
    return [r.ljust(width, ".") for r in rows]


def draw_icon(img, rows, ox, oy):
    """`rows` at (ox, oy): white, with a dark edge around (and between) its pixels."""
    px = img.load()
    on = {(ox + x, oy + y) for y, row in enumerate(rows) for x, c in enumerate(row) if c == "#"}
    for (x, y) in on:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                p = (x + dx, y + dy)
                if p not in on and 0 <= p[0] < img.width and 0 <= p[1] < img.height:
                    px[p] = ICON_EDGE
    for p in on:
        px[p] = ICON


def tile(name, pressed, turn=None):
    img = plate(T, pressed, seed=sum(map(ord, name or "plain")))
    if name:
        rows = bitmap(name)
        if turn:
            # The D-pad's arrows: the same icon turned.
            square = Image.new("1", (16, 16))
            for y, row in enumerate(rows):
                for x, c in enumerate(row):
                    if c == "#":
                        square.putpixel((x, y + (16 - len(rows)) // 2), 1)
            square = square.transpose(turn)
            rows = ["".join("#" if square.getpixel((x, y)) else "." for x in range(16))
                    for y in range(16)]
        h, w = len(rows), len(rows[0])
        ox, oy = (T - w) // 2, (T - h) // 2 + (1 if pressed else 0)
        draw_icon(img, rows, ox, oy)
    return img


def stick_base():
    """The stick's base: a large plate, its face see-through so the world shows."""
    return plate(STICK, False, alpha=96, seed=3)


def build():
    OUT.mkdir(parents=True, exist_ok=True)
    sprites = {}
    for name in ["jump", "sneak", "attack", "use", "inventory", "chat", "pause", "drop", "offhand",
                 "camera", "command", "debug", "pick", "players"]:
        sprites[name] = tile(name, False)
        sprites[name + "_pressed"] = tile(name, True)
    for name in CUSTOM_ICONS:
        sprites[f"custom_{name}"] = tile(f"custom_{name}", False)
        sprites[f"custom_{name}_pressed"] = tile(f"custom_{name}", True)
    turns = {"up": None, "down": Image.Transpose.ROTATE_180,
             "left": Image.Transpose.ROTATE_90, "right": Image.Transpose.ROTATE_270}
    for d, turn in turns.items():
        sprites["dpad_" + d] = tile("up", False, turn) if turn else tile("up", False)
        sprites["dpad_" + d + "_pressed"] = tile("up", True, turn) if turn else tile("up", True)
    sprites["dpad_middle"] = tile(None, False)
    sprites["custom_blank"] = tile(None, False)
    sprites["stick"] = stick_base()
    sprites["stick_knob"] = tile(None, False)
    for name, img in sprites.items():
        img.save(OUT / f"{name}.png")
    return sprites


def preview(sprites, background, out):
    """The sprites where Starling's default layout puts its buttons, over a screenshot of it
    (3200 x 1440): each covers the old round button it replaces."""
    bg = Image.open(background).convert("RGBA")
    k = bg.width / 2000  # the places below are measured on the screenshot shown at 2000 wide

    def put(img, cx, cy, side):
        big = img.resize((round(side * k),) * 2, Image.Resampling.NEAREST)
        bg.alpha_composite(big, (round(cx * k - big.width / 2), round(cy * k - big.height / 2)))

    put(sprites["attack_pressed"], 1620, 548, 170)
    put(sprites["use"], 1840, 481, 130)
    put(sprites["jump"], 1810, 682, 170)
    put(sprites["sneak"], 1610, 772, 130)
    put(sprites["offhand"], 578, 858, 90)
    put(sprites["drop"], 1420, 858, 90)
    put(sprites["camera"], 860, 77, 100)
    put(sprites["pause"], 1000, 77, 100)
    put(sprites["stick"], 220, 705, 300)
    put(sprites["stick_knob"], 250, 680, 130)
    bg.convert("RGB").save(out, quality=92)


def sheet(sprites, out):
    names = list(sprites)
    cols = 8
    cell = T * 5 + 12
    rows = (len(names) + cols - 1) // cols
    img = Image.new("RGBA", (cols * cell, rows * cell), (70, 120, 60, 255))
    for i, name in enumerate(names):
        sp = sprites[name]
        side = T * 5 if sp.width == T else T * 5
        big = sp.resize((side, side), Image.Resampling.NEAREST)
        img.alpha_composite(big, ((i % cols) * cell + 6, (i // cols) * cell + 6))
    img.convert("RGB").save(out)


if __name__ == "__main__":
    sprites = build()
    if len(sys.argv) > 2:
        look = Path(sys.argv[2])
        look.mkdir(parents=True, exist_ok=True)
        sheet(sprites, look / "sheet.png")
        preview(sprites, sys.argv[1], look / "preview.jpg")
    print("ok", len(sprites))
