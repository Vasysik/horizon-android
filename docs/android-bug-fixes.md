# Horizon Android Bug Fixes

## Bug #69: Minecart chest not opening

**Issue:** When right-clicking on a chest minecart, the inventory doesn't open.

**Root cause:** On Android, mouse clicks were being forwarded as touch events even while playing, causing the chest minecart interaction to be intercepted.

**Fix:** Added a menu-screen gate in `android_aux_mouse()` that only forwards mouse clicks as touch events when the player is in a menu screen (Inventory, Creative, Options, Paused). While playing, mouse clicks now pass through to the game world normally.

## Bug #76: Mouse clicks open chat

**Issue:** Left clicks were intercepted by `android_aux_mouse` and forwarded as touch events, triggering `android_show_keyboard()` and opening the chat screen.

**Root cause:** Same as #69 - the `android_aux_mouse` function was too aggressive in forwarding all mouse clicks as touch events.

**Fix:** Same menu-screen gate as #69.

## Files changed
- `crates/horizon-client/src/game/android_input.rs`: Fixed mouse click handling in `android_aux_mouse()`

## Testing
- `cargo check -p horizon-client` passes
- `cargo test -p horizon-client --lib` passes (1166 tests)

> 🤖 Opened by gabgab1209's AI agent (Copilot), not by gabgab1209 personally.
> Co-Authored-By: Copilot <model> <noreply@anthropic.com>
> Generated with Copilot (https://claude.com/claude-code)
