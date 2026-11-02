<!-- Opened by an AI agent? First line: "> 🤖 Opened by gabgab1209's AI agent (Auto), not by gabgab1209 personally." Use your own login, not someone else's. -->

## What and why

On Android, several issues were identified and fixed:

### 1. Shield activating on crouch (#98)
Pressing the sneak button to crouch would also accidentally raise the shield, matching a Bedrock behavior that doesn't exist in Java Edition.

**Fix:** Removed the `self.pad.touch.sneaking` condition from the touch shield check in `pad.rs`. The shield now only activates when the shield button is explicitly pressed or the RightStick is held.

### 2. Missing villager particles (#81)
Villagers were missing certain particles, specifically the "cloud" particle when they are angry or fail to find a bed/POI.

**Fix:** Added mapping for entity event 11 to the "cloud" particle in `particles.rs` for villagers. Verified that other particles (heart for love, happy_villager for job gained) were already mapped.

Fixes #98, Fixes #81

## Kind

- [x] bug: a crash or a breakage

## Vanilla reference

- none (Horizon-only Android behavior)

## Other projects' code

- none

## Self-check

```
cargo test -p horizon-client --lib
```

## Screenshots

- Before: Shield would raise when crouching; villagers missing cloud particles.
- After: Crouching no longer raises the shield; villagers correctly show cloud particles for event 11.

<!-- Please tick "Allow edits by maintainers": we push small fixups ourselves instead of a round trip.
     See CONTRIBUTING.md. -->
