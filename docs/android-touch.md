# Android touch controls and inventory

Touch controls continue to feed the existing controller, keyboard, mouse and
container input paths. Inventory drops use `Input::Throw` (button 0), and hotbar
drops use the existing single-item Drop action. The integrated server and remote
servers retain authority over inventory changes.

## Button editor

Open Touch Controls, then Edit layout. Buttons have independent Width, Height,
Opacity and position. Movement keeps its original Size/radius control.
`Customize actions / text...` converts a built-in button to a custom button at
the same saved positions; it hides the original button, which can be restored
through Add buttons. The joystick and Touch settings shortcut keep their own
specialized behavior.

Custom buttons support a label (up to 32 characters), text scale, and up to eight
bindings for each of Single, Double and Hold. Tap a binding row to switch between
momentary and Toggle, or X to remove it. Double and Hold thresholds are adjustable
in milliseconds. Game actions use the player's existing action paths; raw keys
still use the player's key bindings.

- **Single: immediate +** is the default. Single starts on touch down with no
  double-tap delay. Double adds its action on the second press; Hold adds its
  action after the hold threshold. An already executed Single is never undone.
- **Single: deferred** waits for a completed tap and the double-tap window. Double
  suppresses Single. A recognized Hold or a drag suppresses the pending Single.
- Momentary actions stop when their owning finger ends. Toggle actions stay
  active until toggled off or the controls are cancelled. Multiple buttons and
  bindings holding the same action do not release each other.
- Cancellation, focus loss, rotation and screen changes clear touch ownership,
  pending gestures and toggles. Physical keyboard holds remain independent.
- Custom sneak toggles become momentary in water, flight and while mounted, using
  the same context as the existing #75 crouch correction. A physically held key
  remains held.

Done saves `horizon-touch.json`; Cancel restores the previous settings. Copy layout
exports JSON to the system clipboard, and Paste layout imports and validates it
without saving until Done. Invalid input keeps the current layout. Android Files
also exposes the saved `horizon-touch.json` through the existing document provider.
This format imports Horizon layouts, not other launchers' formats.

Old square buttons keep their `size` when `width`/`height` are absent. Old custom
buttons keep their `key` as an immediate momentary Single. Existing 10-, 12- and
14-control layouts remain supported. New gesture and geometry fields are optional.

## Camera and joystick

**Look while holding mine** is enabled by default. The attack/mine button's finger
can turn the camera while retaining attack ownership, including custom buttons
bound to the Attack action. Crossing another button never presses it. Disabling
the option keeps the earlier stationary-button behavior.

**Joystick follows finger** is disabled by default and is separate from Dynamic
joystick. Dynamic chooses its starting center on touch down. Following moves the
center during an existing touch only after the finger exceeds the working radius,
within the configured movement area. The movement vector remains clamped to the
normal unit circle. Release restores the fixed home or removes the dynamic frame.
The D-pad does not follow.

## Single-item touch drops

Direct touches on the hotbar or an inventory/container slot recognize:

- Tap before 300 ms: the existing select/click operation.
- Release at or after 300 ms: drop one item.
- Hold until 750 ms: drop one item, then repeat every 125 ms while held.

The inner and outer progress rings show the single-item and repeat thresholds.
Release after repeating adds no extra drop. Moving more than 10 GUI pixels before
repeating hands the gesture to the original drag handler; moving after repeating
cancels it. Android touch cancellation also cancels without a release drop.
Changing the selected hotbar slot, closing/changing the container, taking a carried
stack or losing focus stops repetition. Slow frames do not produce catch-up bursts.

Carried-stack gestures, virtual-mouse operations and Shift-clicks keep their
original handlers. Ctrl+Q remains a separate whole-stack operation; a touch drop
never derives a whole-stack drop from a modifier. Creative slots use the existing
creative menu operations rather than mutating item counts separately.

## Interaction scope

Physical and on-screen Shift/Control contribute independently to inventory
modifiers. Both mouse buttons retain the existing Java container operation rules.
Closing a screen clears touch modifiers without releasing a physical key.

Disabling the dedicated Shield button now disables that extra control. It no
longer binds Sneak to Use implicitly. Shield use remains available through Use or
an explicitly configured button. This removes a confirmed Android mapping that
could raise a shield while trying to sneak-place on an interactive block.

The Java 26.3 bytecode reference is the client JAR pinned in `research-lock.json`:
`Minecraft.handleKeybinds` suppresses attack/use clicks during item use;
`ServerPlayerGameMode.useItemOn` bypasses block interaction for secondary use with
something in either hand; `AbstractArrow.onHitEntity` reverses a refused arrow hit
with a 0.2 velocity scale, not a guaranteed damaging return shot. Bundle primary
and secondary clicks remain the existing `BundleItem`/menu behavior.

These changes do not claim to fix ANR or movement stalls in #43, or every vague
bundle/interaction symptom in #65. The existing #75 correction for #71 is retained.
