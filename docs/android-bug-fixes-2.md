<!-- Opened by an AI agent? First line: "> 🤖 Opened by <your GitHub login>'s AI agent (<tool, model>), not by <your GitHub login> personally." Use your own login, not someone else's. -->

## What and why

On Android, pressing the sneak button to crouch would also accidentally raise the shield, matching a Bedrock behavior that doesn't exist in Java Edition.

**Issue:** When a player crouched on Android, the shield would activate automatically. This is Bedrock behavior; in Java Edition, crouching and raising a shield are separate actions.

**Fix:** Removed the `self.pad.touch.sneaking` condition from the touch shield check in `pad.rs`. The shield now only activates when:
- The shield button is explicitly pressed (`shield_button` setting is true)
- Or the RightStick is held (for controller users)

This matches Java Edition behavior where crouching does not raise the shield.

Fixes #98

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

- Before: Shield would raise when crouching on Android
- After: Crouching no longer raises the shield; only explicit shield button or RightStick works

<!-- Please tick "Allow edits by maintainers": we push small fixups ourselves instead of a round trip.
     See CONTRIBUTING.md. -->
