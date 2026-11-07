# Native mods on Starling

Horizon's native mods ([native-mods.md](native-mods.md)) are horizon-mixins bundles: Rust that hooks the game's own
functions and is compiled into the game. On a computer the launcher downloads Rust and builds the game with them. A
phone cannot build the game (Rust and the NDK are gigabytes, and the build takes hours on a phone), so Starling's build
service does it and the phone swaps in the result:

1. **The mod.** The player opens an `.hnmod` with Starling (`import.rs`), which puts it in `horizon-native-mods/`.
2. **Compile.** Its row on the Mods page has a Compile button (`game/native_builds.rs`). After a warning, the game sends
   the folder's mods to the service, and the player can fish or mine while it waits (`game/setup/fishing.rs`,
   `game/build_mining.rs`).
3. **The download.** The game's library comes back, is checked against its SHA-256, and is chosen for the next start
   (`files/native/current`). A dropped download goes on from where it stopped.
4. **The swap.** At the next start `StarlingActivity` loads the chosen library in place of the APK's own
   (`NativeMods.java`), so Java's native methods bind to it. NativeActivity still loads the APK's library: its entry
   point is `starling_on_create` (the manifest's `android.app.func_name`, `android_lib.rs`), which finds the chosen
   build already loaded (`STARLING_GAME_LIBRARY`) and starts the game there. Activity and game share one library.

**Safety nets.** A build is only loaded by the version it was made for (after an update, Compile again). A build that
closes twice before its first frame is set aside: the next start plays the APK's own game and the card says why. "Play
without native mods" stops using one. Nothing is installed: the APK stays the release.

## The build (native_build.sh)

1. horizon-mixins at Horizon's launcher's pinned commit (`MIXINS_REV`), built once into the cache.
2. The source of the commit (git archive, without links), and copies of the bundles, their pins set to that commit.
3. **One identity for the game module.** While horizon-mixins plans, horizon-android's library is an empty file (it
   refuses a module destination reached under two names, `E_TARGET_AMBIGUOUS`), and `lib.rs` loses the conditional
   attribute that keeps it from looking at the modules under that file. The bundles' targets stay Horizon's
   (`horizon_client::game::Game<'_>::use_item`), so Horizon's own native mods apply unchanged.
4. `horizon-mixins apply` into a stage, then into a build folder kept between builds: files whose bytes changed get
   the time of now, the others keep theirs, so Cargo recompiles only what the mods touched.
5. `build_apk.py --profile fast`, as a release: the stripped library, an APK unless `NATIVE_LIBRARY_ONLY` is set, and
   `build.json`.

## Permissions

A native mod is written for Horizon on a computer, where nothing asks before a program opens the microphone. On Android
the app holds the permission and asks the player, and the mod stays as its maker wrote it: `native_build.sh` reads each
mod's source with `native_mod_permissions.py` and writes what it needs in `build.json` (the mod's `permissions`, and
all of them at the top). Today that is `microphone`: capturing sound through cpal (`build_input_stream`,
`default_input_device`...), AAudio, Oboe, OpenSL ES or Android's `AudioRecord`. Once a build with such a mod shows its
first frame, Starling says which mod wants what, then hands it to Android's own question (StarlingActivity
`askModPermissions`); Not now is remembered for that build. The manifest declares `RECORD_AUDIO` for this, and nothing
asks for it without such a mod.

## The service

`packaging/android/mod_build_server.py` builds one job at a time and keeps the last results: the same mods for the same
version get the first library at once. `versions.json` beside it says which commit each version of the game is.

A mod's code runs while it compiles (build scripts, macros), so `mod_build_sandbox.sh` runs each build with no network,
none of the machine's files or keys, and no way to gain privileges, over a clean build cache: what a build writes stays
in an overlay thrown away after it, so a mod can't reach a later player's build.

`mod_build_gateway.py`, behind HTTPS on a small machine that stays on, is what the game talks to. The machine that builds
sleeps until a request wakes it (about 35 seconds) and goes back to sleep once it has been idle for 15 minutes. It limits
how many builds an address, and everyone together, can ask for in a day.

Each new release needs `versions.json` updated and the clean cache rebuilt on its commit, or its first builds are slow.

## Security

- **No secret in the APK.** It only talks to the gateway, over HTTPS, and needs no account.
- **A native mod has the game's full access on the phone.** The warning says so; only the player can decide to trust it.
- **The library is checked** against the SHA-256 the service gives for it.
