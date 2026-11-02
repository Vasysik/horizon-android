<!-- Opened by an AI agent? First line: "> 🤖 Opened by <your GitHub login>'s AI agent (<tool, model>), not by <your GitHub login> personally." Use your own login, not someone else's. -->

## What and why

On Android, mouse clicks were being forwarded as touch events even while playing, causing two issues:

1. **#69 - Minecart chest not opening**: Right-clicks on chest minecarts were intercepted instead of opening the inventory
2. **#76 - Mouse clicks open chat**: Left-clicks triggered chat input instead of attacking/interacting

The `android_aux_mouse()` function was forwarding ALL mouse clicks as touch events, regardless of whether the player was in a menu or playing. This caused gameplay interactions to be blocked.

The fix adds a menu-screen gate: mouse clicks are only forwarded as touch events when the player is in a menu screen (Inventory, Creative, Options, Paused). While playing, mouse clicks pass through to the game world normally.

Fixes #69, Fixes #76

## Kind

- [x] bug: a crash or a breakage

## Vanilla reference

- none

## Other projects' code

- none

## Self-check

```
python3 tools/pr_ready.py
```

## Screenshots

- Before: Mouse clicks on chest minecarts opened chat instead of inventory
- After: Mouse clicks correctly interact with entities while playing

<!-- Please tick "Allow edits by maintainers": we push small fixups ourselves instead of a round trip.
     See CONTRIBUTING.md. -->
