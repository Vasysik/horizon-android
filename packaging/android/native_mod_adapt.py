#!/usr/bin/env python3
"""Native mods made before Horizon split its client into crates, pointed at where things went.

A mod names what it hooks by crate and module, and is built by Horizon at the commit it pins. Starling
builds every mod at its own commit (native_build.sh), and at that commit `horizon_client` holds only the
game module and a glob of `horizon_client_lib`, which horizon-mixins can't look through. The manifests
are pointed at where things went, and the mods' calls to what changed its shape since are brought up to
date (CALLS, CHANGED). What is left is the compiler's to judge.

    native_mod_adapt.py SOURCE MODS    SOURCE is a checkout, MODS holds <bundle>/horizon-mixin.toml
"""
import pathlib
import re
import sys

ROOTS = {
    "horizon_client_lib": "crates/horizon-client/src/client_lib.rs",
    "horizon_client_render": "crates/horizon-client-render/src/lib.rs",
    "horizon_client_core": "crates/horizon-client-core/src/lib.rs",
    "horizon_client_input": "crates/horizon-client-input/src/lib.rs",
}
FOLDERS = ("crates/horizon-client-render/src/", "crates/horizon-client-core/src/", "crates/horizon-client-input/src/")
OLD_FOLDER = "crates/horizon-client/src/"
OLD_ROOT = OLD_FOLDER + "lib.rs"
# What Horizon renamed since, and takes the same arguments.
RENAMED = {"horizon_client::game::titles::Game<'_>::draw_titles": "horizon_client::game::titles::Game<'_>::draw_title"}
MODULE = re.compile(r"^\s*(?:pub(?:\(crate\))? )?mod (\w+)\s*(?:;|\{)", re.M)
# Calls with the arguments they took (a call, how many, what the new ones are).
CALLS = (
    (".block_loot_at(", 3, "None"),
    (".show_suggestions(", 1, "false"),
    ("ui::slider(", 9, "false"),
    ("super::build_renderer(", 7, "false, false"),
)
# What became something else. Some of it gives a little up: a LAN guest as the F3 target, `/dh` in the
# chat's completion, a camera's own eye height, the bounds of a dimension lower than the overworld.
CHANGED = (
    (r"GuestUse::Entity\(Hand::Main\)", "GuestUse::Entity(Hand::Main, _)"),
    (r"Slots::Ender => Vec::new\(\),", "_ => Vec::new(),"),
    (r"\b[\w.]*\.dim\.min_y\(\)", "(-64)"),
    (r"\b[\w.]*\.dim\.max_y\(\)", "319"),
    (r"fn treefall_destroy_progress\(&self", "fn treefall_destroy_progress(&mut self"),
    (r"game\.camera_eye_height\(\)\.unwrap_or\(STANDING_EYE\)", "STANDING_EYE"),
    (r"portal::set_entities\(vertices\)", "portal::set_entities(vertices.0)"),
    (
        r"let Some\(tree\) = remote\.commands\.as_ref\(\)\.and_then\(with_commands\) else \{ return \};"
        r"\s*remote\.commands = Some\(tree\);\s*game\.merge_remote_tree\(\);",
        "let _ = remote;",
    ),
    (
        r"(oversized_in_gui: resolved\.oversized_in_gui,\s*animated,)",
        r"\1 cit_glint: None, hdr_tier: 0, hdr_light: 0,",
    ),
    (r"extra: &\[\(&Part, \[f32; 4\], u32, u32\)\]", "extra: &[(&Part, [f32; 4], u32, u32, bool)]"),
    (r"crate::game::Game::tooltip_box\(batch, atlas, ", "game.tooltip_box(batch, None, "),
    (r"= self\.guest_hit\(\) \{", "= None::<(u32, _)> {"),
    (r"game\.native_is_multiplayer\(\)", "game.server_address().is_some()"),
    (r"game\.native_server_address\(\)", "game.server_address().map(|a| a.to_string())"),
    (r"game\.native_world_folder\(\)", "game.native_world_dir().map(|d| d.display().to_string())"),
    (
        r"(lava_flow: from\.lava_flow,)(\s*\}\)\);)",
        r"\1 animated_layers: from.animated_layers.clone(), full_faces: from.full_faces.clone(),"
        r" solid: from.solid.clone(),\2",
    ),
    (
        r"d\.custom_name = Some\(bc::ITEM_NAME\.to_owned\(\)\);",
        "d.custom_name = Some(horizon_world::nbt::Tag::String(bc::ITEM_NAME.to_owned().into()));",
    ),
)


def with_arguments(text, call, count, new):
    """`text` with `new` after the arguments of each `call` that has `count` of them."""
    at = 0
    while (start := text.find(call, at)) >= 0:
        first = start + len(call)
        depth, commas, i = 1, 0, first
        while depth and i < len(text):
            depth += (text[i] in "([{") - (text[i] in ")]}")
            commas += text[i] == "," and depth == 1
            i += 1
        inner = text[first : i - 1].rstrip()
        trailing = inner.endswith(",")
        if commas + (not trailing) == count:
            kept = inner[:-1] if trailing else inner
            text = f"{text[:first]}{kept}, {new}{',' if trailing else ''}{text[first + len(inner) :]}"
        at = first
    return text


def code(text):
    for call, count, new in CALLS:
        text = with_arguments(text, call, count, new)
    for old, new in CHANGED:
        text = re.sub(old, new, text)
    return text


def owners(source):
    """Each top-level module's crate, the first root that declares it."""
    found = {}
    for crate, root in ROOTS.items():
        for module in MODULE.findall((source / root).read_text(encoding="utf-8")):
            found.setdefault(module, crate)
    return found


def adapt(text, source, modules):
    def target(match):
        path = match[2]
        for old, new in RENAMED.items():
            path = path.replace(old, new)
        path = re.sub(
            r"\bhorizon_client::(\w+)",
            lambda m: f"{modules[m[1]]}::{m[1]}" if m[1] in modules else m[0],
            path,
        )
        return f'{match[1]}{path}"'

    def file(match):
        path = match[2]
        if path == OLD_ROOT:
            path = ROOTS["horizon_client_lib"]
        elif path.startswith(OLD_FOLDER) and not (source / path).exists():
            moved = (folder + path[len(OLD_FOLDER) :] for folder in FOLDERS)
            path = next((p for p in moved if (source / p).exists()), path)
        return f'{match[1]}{path}"'

    # A mob's extra parts gained a flag; a binding spells the type as its helper does (CHANGED).
    text = re.sub(r"(\[f32;\s*4\],\s*u32,\s*u32)\)\]", r"\1, bool)]", text)
    text = re.sub(r'^(\s*target\s*=\s*")([^"]*)"', target, text, flags=re.M)
    return re.sub(r'^(\s*file\s*=\s*")([^"]*)"', file, text, flags=re.M)


def main():
    source, mods = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    modules = owners(source)
    for manifest in mods.glob("*/horizon-mixin.toml"):
        old = manifest.read_text(encoding="utf-8")
        new = adapt(old, source, modules)
        if new != old:
            manifest.write_text(new, encoding="utf-8")
            print(f"native_mod_adapt: {manifest.parent.name} pointed at where things went")
    for source_file in mods.glob("*/**/*.rs"):
        old = source_file.read_text(encoding="utf-8", errors="surrogateescape")
        new = code(old)
        if new != old:
            source_file.write_text(new, encoding="utf-8", errors="surrogateescape")
            print(f"native_mod_adapt: {source_file.parent.name}/{source_file.name} brought up to date")


if __name__ == "__main__":
    main()
