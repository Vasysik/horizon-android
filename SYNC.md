# Sync with Horizon (temporary, delete after the merge lands)

Upstream commit merged: `31f83499b` (2026-11-02, horizon-rust/horizon#7585). Base `f8988d5a7`.
Android before the merge: `73fb969be`. Wasm removal, files only: `30344a60b` (does not build).

The merge commit `212b96cb7` holds its conflict markers on purpose; each group below was resolved in
a commit of its own. Part of the conflicts came resolved from the older `sync-horizon-3` worktree,
read only and carried forward with 3-way merges (the mod crates were left out).

How to work and tools: `sync4/` in the session notes folder
(`%APPDATA%/Claude/side-session-notes/local_4c49e3d5-8a78-4799-b072-15ba12d5ea8a/`): `BRIEF.md`,
`COMPILE.md`, `FOLLOWUPS.md` (what fell, in detail), `vm.sh MODE` (check, all, android, clippy,
clippy-android, fmt, test, raw CMD). On the VM: `~/sync4`, `~/sync4-target`, `~/sync4-apk.sh`.

## Steps
- [x] A. Mods: `8f18fd9d6`
- [x] B. Render and shaders: `26f8ed428`
- [x] C. Client gameplay: `2315ac2db`
- [x] D. game.rs, title, Cargo, CI: `3bbeb3a20`
- [x] E. Desktop builds (`cda4b7629`); Android builds, hooks in `horizon_client::android`, Dev APK (`7a5fb6925`)
- [x] fmt
- [x] Client opens on the VM (Xvfb, software Vulkan, new world, no mods or pack)
- [x] Tests: 5495 passed, 0 failed, 104 ignored
- [x] Clippy desktop and Android, `-D warnings`
- [ ] Owner tests the APK; merge into android

## What fell
- Wasm mods and their API: horizon-mod-sdk, starling-mod-sdk, starling-mod-cli, mods/official
  (far-view, fps-overlay, keystrokes, minimap, recipes, zoom) and the SDK examples.
- Create: own block states (`states`, `own-states.json`) and so placement orientation and belt
  funnels (`belt_funnel_for`); `fuel` on items; the support and occlusion part of `properties.shape`.
  The local .hmod data has to move to `state_properties` and `components`.
- Create New World's "Mods & Data Packs": upstream's Data Packs screen stays; a new world plays with
  the running mods; the phone's `datapacks` folder choice.
- Stack tooltip grids; Starling's remote item displays (upstream's `text_displays` instead).
- Failed shader experiments `HORIZON_PACK_SKY_LAST`, `HORIZON_PACK_VRS`, `HORIZON_PACK_HALF`.
- Item grids in tooltips (`StackTooltip`), the Config button and summary line on the Mods page.
- Android: `learn_layout` does nothing, the splitscreen pad router is a stub.

## Not checked
Nothing ran on a phone. On the VM the client ran without mods and without a shader pack: Create,
shader packs, touch, custom buttons, the setup download and native mods builds were not run after
the merge.
