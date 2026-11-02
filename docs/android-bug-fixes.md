## What and why

Two Android reports look like mouse or touch clicks being eaten:

1. **#69** — right click (Use) on a chest minecart only plays a sound; the container never opens (reporter: touch scheme "Joystick and crosshair", no mouse).
2. **#76** — pressing any mouse button opens the chat bar and cancels the action (reporter: Samsung DeX, external display, "Touch control scheme: None").

An earlier draft gate scoped `android_aux_mouse` to four screens. That carried regression risks named in review and is **withdrawn**: the shared event behavior is restored stock, and this change instead documents the boundary precisely and pins its transitions with tests. Neither issue is claimed fixed here; both need their own traces (below).

## The input trace: when `android_aux_mouse` is even consulted

Event order in `window_event` (crates/horizon-client/src/game.rs:16160), with every precondition. A physical `MouseInput` event passes through the aux handler only when **all** of these hold:

| # | Condition | Code | Refuted uses |
|---|---|---|---|
| 1 | Android build | `#[cfg(target_os = "android")]` | desktop traces |
| 2 | Keypad profile enabled | `self.settings.keypad.enabled` (opt-in) | default profiles |
| 3 | Free pointer: not captured while keyboard/mouse input is on | `!(self.captured && crate::keyboard_mouse_input())` | captured crosshair play |
| 4 | **Left button only** | `button: MouseButton::Left` match | **any RMB report** |
| 5 | Not a synthetic `DeviceId::dummy()` click | `device_id != DeviceId::dummy()` | the pad's own events |
| 6 | Cursor over a touched auxiliary/utility button, or the pad already owns the synthetic pointer | `aux_mouse_phase(pressed, owned, touched)` | clicks anywhere else |

- (4) alone: this handler **cannot** intercept a physical right click, so it cannot explain #69's RMB failure on a chest minecart.
- #69's reporter has no mouse at all (touch Use): their Use press is a touch token on `pad.touch`, a path `android_aux_mouse` never sees.
- For #76, the aux path was live only if the DeX setup had the keypad profile enabled, an uncaptured pointer, and the cursor over an auxiliary area; the reviewer's reading matches the guards.

## What the code change actually does

Stock semantics, re-expressed so the press/release call is under test (`horizon_client::touch_ui::aux_mouse_phase`):

- A press becomes a synthetic touch only over a touched pad button; everything else (attack, use) reaches the world handler.
- A release completes whatever the pad already owns: `TouchUi::owns_mouse` is now included next to `owns_key` and `owns_pointer`, so a held auxiliary LMB/RMB press survives a menu entry, exit, echo-flight or IME layout change instead of sticking until the next cancel-all.
- A release the press never produced is inert.

Coverage in `crates/horizon-client/src/touch_ui.rs::tests` (runs on every target; the module itself is Android-only, cross-platform tests were demanded for this reason):

- `aux_mouse_presses_belong_to_the_pad_only_over_its_buttons` — gameplay clicks pass through free.
- `a_menu_press_survives_closing_the_menu_before_its_release` — press over an action, layout changes, release still ends it, later strays are inert.
- `a_held_auxiliary_mouse_button_releases_after_a_screen_change` — `owns_mouse` holding/releases.
- `a_release_over_a_touched_button_ends_it`.

## Open questions for #69 and #76

- **#69**: candidate suspect is the Use→container path for entity-borne inventories (minecart chest), outside the mouse handler. Next step: a debug build logging `play_mouse`/`use_item` → container open for chest minecarts, and the reporter's hotbar/Use layout.
- **#76**: needs the reporter's keypad-profile state (was `keypad.enabled` on?), HUD visibility, and whether the pointer was captured during DeX play; if the chat opening instead comes from the IME path (`crate::platform::show`), a logged `android_show_keyboard` caller trace separates it from the aux handler.

## Kind

- [x] bug: investigation notes and transition coverage; no behavior beyond owned-release preservation

## Vanilla reference

- none (input plumbing is Horizon-specific)

## Other projects' code

- none
