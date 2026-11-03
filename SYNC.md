# Sync with Horizon (temporary, delete after the merge lands)

Upstream commit merged: `31f83499b` (2026-11-02, horizon-rust/horizon#7585). Base `f8988d5a7`.
Android before the merge: `73fb969be`. Wasm removal, files only: `30344a60b` (does not build).

The merge commit holds its conflict markers on purpose; each group below is resolved in a commit of
its own. Part of the conflicts came resolved from the older `sync-horizon-3` worktree, read only and
carried forward with 3-way merges (files where that left fewer hunks; the mod crates were left out).

How to resolve and tools: `sync4/BRIEF.md` in the session notes folder
(`%APPDATA%/Claude/side-session-notes/local_4c49e3d5-8a78-4799-b072-15ba12d5ea8a/`).

## Groups
- [ ] A. Mods: horizon-mods, docs, port-pack, mod_content, game/mods.rs, native_recipes (upstream wins)
- [ ] B. Render and shaders: horizon-client-render, horizon-shaders, client-core
- [ ] C. Client gameplay: game/* except game.rs and mods.rs, mobs, net, control, examples, tests
- [ ] D. game.rs, title, lib.rs, Cargo.toml files, CI, notices, tools
- [ ] E. Cargo.lock from upstream's, Android crate on upstream's lib split, build_apk.py without the JS engine
- [ ] F. VM: cargo check, clippy desktop and Android, fmt, tests, client, Dev APK
- [ ] G. Push; list what fell

## What fell (for the final report)
- Wasm mods and their API: horizon-mod-sdk, starling-mod-sdk, starling-mod-cli, mods/official
  (far-view, fps-overlay, keystrokes, minimap, recipes, zoom) and the SDK examples.
