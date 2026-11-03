# Android 0.7.6 test checklist

Use an APK built from the current `android` branch. Record its source commit, ABI, device, renderer and scale. Keep an existing profile to check migration; use a separate disposable profile for destructive tests.

1. **Existing settings:** launch after updating from 0.7.5. Custom positions, bindings, button sizes and visibility must remain intact. Swap hands is initially disabled.
2. **Swap hands:** enable its button in Touch Controls, move it in the editor, exchange two different held items, then disable it. Check singleplayer and a server.
3. **Shield:** with a shield in the off hand and food or a usable item in the main hand, test both Sneak and Button modes. Hold/release, rotate the camera, target a chest, open inventory, pause and resume. The shield must release correctly and the other item must not be used. Check a shield in the main hand too.
4. **Aiming:** in Joystick & tap and D-pad & tap, equip bow, crossbow, trident, snowball, egg, potion, wind charge and fishing rod. Check the center crosshair, aiming and hold/release. Switch back to a block: the automatic crosshair must disappear.
5. **Surface actions:** tap with flint and steel, fire charge, bone meal, dye, honeycomb, a music disc, water/fish buckets, boat and spawn egg. Food and projectile use still require a hold. Check ordinary placement/mining and holding on a boat to mount it.
6. **Sensitivity:** drag equal distances horizontally and vertically in landscape and portrait. Tune each axis independently and restart to verify persistence. Test gyro separately; these two sliders calibrate finger movement.
7. **Tutorial:** in a disposable new profile, walk/jump and turn with touch (or a gamepad). Both first hints should complete. Leave/re-enter and create another world: a completed stage should not restart, but an unfinished stage can return.
8. **D-pad:** immediately after Play, press Down without touching the display or waiting for full resource loading. Check highlight and Enter/center-button activation, then options, world and server lists. Text-field arrow editing must still work.
9. **Diagnostics:** record once with Performance recording only, GPU profiling only, and both together. Return to the launcher and export through Files / Horizon Android / Reports. With both enabled, there must be one combined session containing CPU/frame context and supported GPU stage data. Check a second Play too. Empty GPU stage data must correspond to unsupported encoder timestamps or no completed samples, not a missing report.
10. **Installation #30:** on a disposable installation, download and prepare resources without a PC. Restart/retry preparation and verify it finishes instead of returning to Download with a missing-file error. Preserve logs if the affected Samsung device still fails.
11. **Upstream regression smoke test:** Compatibility and Batching, world loading, translucent water/glass, dawn/dusk, resource-pack reload, `.hmod` Zoom and minimap, keyboard input, multiplayer join and reconnect, pause/Home/lock recovery.

These checks do not require deleting real worlds or resetting the primary app's storage.

12. **Crash diagnostics:** performance recording can remain off. After a real failure,
    reopen Horizon, wait a few seconds and use Settings > Diagnostic report > Export
    ZIP. Check previous/current session phase, Android exit collection status and
    available panic/native summaries. A return to the launcher must preserve its
    handled error, including when a non-default profile is active. Do not force a
    crash in a valued world; intentional failures belong only in the standalone
    diagnostics probe. Review/redact the ZIP before posting it. Older Android or
    firmware may expose no OS trace; the report must say so explicitly.

13. **Player persistence:** in a disposable new world obtain several items, gain experience and walk away from spawn. Save and Quit, reopen twice and restart the app. Inventory, position, XP and block edits must persist. Keep backups before testing existing worlds.
14. **Periodic stalls/audio:** leave both launcher and game menu idle for a minute, then play with performance recording off and on. Confirm the former roughly one-second freezes are absent. Check sound and reconnect headphones; an actual output change may briefly reopen audio.
15. **Mali composition:** at 100% and 85%, compare sun, fire and lantern colors in SDR; check shader packs and screenshots. The alpha-only compositor fix must not darken/change RGB.
16. **Graphics recovery:** first verify ordinary OpenGL ES starts launcher and game. An interrupted Vulkan startup should select GLES at next app launch, preserving profiles. Test intentional interruption only in a disposable test installation; later crashes may have other causes.
17. **Swap/aim:** enable Swap hands, check it does not overlap Sprint, and confirm custom layouts survive migration. Fireworks must not add an automatic crosshair; bows and thrown projectiles still do.
