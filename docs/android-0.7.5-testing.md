# Android 0.7.5 candidate testing

Test an APK built from the current `android` branch, not the earlier RC1 or the
obsolete `0.7.5-final` APK. Record the source commit, ABI, phone, Android version,
renderer, resolution scale and render distance with each report. Back up valued
worlds first. Do not reset an existing profile just to test settings migration.

## 1. Startup and recovery

- Open the launcher; check text, scrolling, text cursors and Android keyboard.
- Open the game, remain in the title menu for a minute, then enter a world.
- Open chat twice, type text and dismiss the keyboard with Back.
- Use Home and return; lock/unlock and return. Repeat in a multiplayer session.
- Expected: no crash, missing launcher labels, stuck keys or persistent mouse
  presses. Android may terminate a background process; distinguish that from a
  crash using the exit report after restarting.

## 2. Virtual mouse

In an inventory or server container, activate the always-visible Mouse button at
the top right. Its sensitivity is in Options > Controls > Touch Controls.

- Drag a finger: the cursor moves relative to its previous position, like a
  touchpad, rather than jumping beneath the finger.
- Hover an item without clicking: its tooltip should appear.
- Tap once for LMB; hold without dragging for RMB. No extra tap/hold option is
  required. Adjust Virtual mouse sensitivity and compare the same gesture.
- Use the lower-right LMB/RMB buttons. Hold LMB with one finger and move the cursor
  with another. Release outside the button: it must still release.
- While carrying/dragging an item, disable Mouse, press Back, or background the
  application. Reopen: there must be no stuck button or unintended extra click.
- Mouse must not enable the movement/attack touch overlay. Check with Controls
  in inventory both disabled and enabled.

## 3. World touch gestures

Repeat in the four existing control schemes. In a crosshair scheme also enable
Touch gestures with crosshair: targeting should use the center, not the finger.

- Ordinary building block: tap places; holding mines. Aim away from a block and
  release: no later unintended placement/mining should occur.
- Usable item: tap a creature attacks; a short tap away from creatures must not
  immediately use the item or begin block mining. Hold to use the item.
- Cover food, potions, bow/crossbow, fishing rod, wind charge, firework rocket,
  armor from the hand, seeds, bone meal, buckets and spawn eggs where applicable.
  Item-specific game rules still apply (hunger, ammunition, valid target, etc.).
- Test fireworks while gliding, both through a touch hold and the Use button.
- While holding food or mining, rotate the camera and move. Lift the finger,
  open a menu, or background the app: the action must end correctly.
- Tap a moving creature with a block behind it: the tap must not turn into a
  held attack that mines the exposed block on the next tick.

## 4. Gyroscope

In Touch Controls enable Gyroscope, initially at low sensitivity.

- In active touch gameplay, tilt horizontally and vertically; check direction.
- Increase sensitivity and repeat. Turn it off: tilting must stop moving the view.
- Repeat after rotating the display to each orientation allowed by the client.
- Enter menus, use another input mode and background the app: gyro must not keep
  turning the view. Return to touch gameplay and check recovery.
- A device without a gyro must remain usable. No sensor support is implied by
  the presence of the setting alone.

## 5. Display refresh rate

In launcher Settings choose Display refresh rate, then start the game.

- Test System default and one rate supported by the device (e.g. 60/120 Hz).
- If available, use Android's display-refresh overlay to observe the display
  mode. Do not confuse this with the game's FPS counter.
- Return to System default and restart. Check persistence after closing/reopening.
- The OS may ignore a rate because of hardware, battery saver or thermal policy.
  This setting requests a display rate; it does not uncap or cap game FPS.

## 6. Performance hints and GPU profiling

Both are in launcher Settings and default off.

- Enable Performance hints and play, then background/restore. Unsupported
  devices must continue normally. ADPF gives CPU scheduling hints, not fixed
  clocks or guaranteed higher FPS.
- Enable GPU profiling with performance recording. Play through world entry,
  stationary observation, movement, water and a complex scene. Export the report.
- Check for stage timings and p50/p95/p99 where GPU timestamps are available.
  Missing unsupported timings must not cause a crash. Profiling itself has cost.
- For performance comparisons use the same world, position, view, renderer,
  scale, chunks, graphics, power mode and similar device temperature. Compare
  profiling off against profiling off. Record the first minute after world entry
  as well as a settled scene; one instantaneous FPS value is insufficient.

## 7. Mods, packs and reports

- On a compatible 64-bit build install and enable a known sandbox `.hmod`, then
  verify its behavior in-game, not just its Installed entry. Restart and repeat.
- `.hnmod` catalog entries must show an unsupported status, not claim successful
  Android execution. Java Modrinth mods remain incompatible with this client.
- Import a resource pack and shader pack, cancel a picker, and test replacement
  confirmation. Enable/disable the packs and check recovery after resource reload.
- Open Android Files > Horizon Android > Reports. After a genuine failure,
  restart the client and inspect the exit reason and any available trace/log tail.
  Do not deliberately crash an important world. OS low-memory termination is not
  necessarily a software crash; some firmware supplies no detailed trace.
- Review paths, server addresses and exception text before sharing reports.

## 8. Settings and scope

- Renderer selection belongs to launcher Settings; no editable Graphics API row
  should appear inside the Android game.
- System preferences belong to the launcher; gyro/mouse options belong to Touch
  Controls. Existing values should survive restarting and legacy migration.
- Launcher Settings: toggle Immersive fullscreen. Navigation bars should hide; the status bar stays hidden with either choice;
  an edge swipe temporarily reveals system bars. Recheck after Home, screen lock,
  keyboard use and opening the system file picker.
- Toggle Avoid camera cutout with a real or emulated cutout. The full picture
  should fit in the safe region. Without a cutout this need not visibly change.
- Select Native, 16:9, 4:3 and 1:1 aspect ratios, then rotate the device and try
  100% and 50% client resolution. Portrait uses the corresponding portrait ratio.
  The picture must fit without stretching; unused space is black. Touch every
  corner button, drag inventory items, use the virtual mouse and a physical mouse.
  Starting a touch in a black bar must not activate a game control. Check screenshots
  and HDR/shader output too: emulator window checks do not validate game rendering.
- New Skins & Capes is not included in this candidate.

For a failure provide reproduction steps, expected/actual behavior, build
commit and a new report from that run. For crashes also attach the exit report
after restarting. A performance recording alone may not contain a crash stack.
