# Accounts, skins and capes

The native Android launcher's **Skins** page uses the shared Rust wardrobe APIs.
Import a 64×64 or legacy 64×32 PNG skin, or a 64×32 / legacy 22×17 PNG cape.
Imports are bounded to 1 MiB, validated before decoding and stored as re-encoded
PNG images. The gallery keeps multiple images. Select an image to apply it locally.
Drag the 3D character to rotate, pinch or use +/− to zoom, and switch Classic/Slim.
The cape gallery includes original local designs; these do not grant official capes.

Local choices affect this client's view of its player. They do not upload textures,
change an account profile or tell other clients to use these images. A separate,
explicit cloud action is required to change a Microsoft profile. **Use current
cloud appearance** removes the corresponding local override.

Microsoft accounts can synchronize their active skin and the official cape list
returned by their authenticated profile, upload a selected skin, and activate a
cape that the account owns. No unowned official cape is offered for activation.
Cloud synchronization and image downloads run off the UI thread. A failed cloud
request leaves the local gallery available.

Ely.by sign-in uses its documented Minecraft authentication protocol, including
the optional six-digit two-factor code. Passwords and codes are not saved. Ely.by
access/client tokens use the existing protected credential store, under a separate
provider key; they are not Microsoft credentials or proof of Java ownership.
The game's session file identifies the provider and selects the matching session
join service. Ely.by accounts work with servers configured for that provider;
they cannot authenticate as Microsoft accounts on Microsoft-only servers.

Ely.by's public skins API exposes current skin/cape downloads, but does not
document a skin-upload API. Cloud skin management therefore opens Ely.by's site.
Its documented cape system does not allow players to equip arbitrary cloud capes;
custom cape choices remain local.

## Frontend and engine boundary

`launcher/src/account.rs` and `account/ely.rs` implement account operations and
credential storage. `skins/{library,cloud,preview}.rs` provide the frontend-neutral
wardrobe, provider operations and textured 3D preview. Android's `wardrobe.rs` is a
view over these APIs. No UI object is passed into the game.

At launch, the frontend writes `horizon-appearance.json` into the explicit game
directory. This is a public, versioned contract; another frontend language can
write the same file:

```json
{"schema":1,"skin":"/absolute/skin.png","cape":"/absolute/cape.png","slim":false,"hide_cape":false}
```

`skin` and `cape` may be null. The game validates and loads these PNGs afresh on
each launch, including repeated games in Android's process. This handoff carries
no access token and does not replace the read-once `--session-file` protocol.
Cloud textures downloaded by the launcher can use the same image handoff.
Legacy desktop offline-skin environment variables remain available as fallbacks.

Local metadata lives in `wardrobe/wardrobe.json`; images use digest-based names
under `wardrobe/images`, cloud cache images under `wardrobe/cloud`. Account
selections are provider-qualified, so an offline name and signed-in UUID cannot
overwrite each other's choices. Account completion, sign-out and offline-name
changes refresh an already-open gallery, including background browser sign-in;
responses for the previous account are discarded. A session worker publishes the
game appearance only after checking the current selection on the UI thread.
Renewal never selects an account or rewrites the account list; busy sign-out
returns a retry message without blocking native UI. Unsupported schemas are refused without replacing
the stored index.

## Verification

`python tools/test_android_appearance.py` tests the real shared account and image
code using mock credentials and services. Android builds use
`python tools/android_build.py --abi x86_64 --check` (or a device ABI).
UI and game checks must use fresh emulator/app data. Mock API tests do not prove
live Microsoft or Ely.by account permissions; real sign-in and cloud mutation
require the account owner and an appropriately configured service/application.

Protocol references: [Ely.by Minecraft authentication](https://docs.ely.by/en/minecraft-auth.html),
[Ely.by skins](https://docs.ely.by/en/skins-system.html).

### Verification on 2026-11-02

- Shared account/wardrobe harness: 65 passed, one existing ignored test. Includes
  Microsoft/Offline regression tests, provider isolation, Ely.by 2FA/refresh HTTP
  fixtures, Microsoft PNG upload/owned-cape HTTP fixtures and game-handoff reset.
- Android API 35 x86_64 VM: 24 engine tests passed (auth, skin processing, handoff).
- ARM64 Android type-check and Clippy with warnings denied passed.
- Android source/release Python tests: 13 passed.
- Fresh emulator, host GPU, 1280x800: imported a skin and cape through Files,
  rejected an invalid-size PNG, selected local designs, rotated/zoomed the model,
  switched Classic/Slim, checked persistence, Ely.by validation and secure-field
  keyboard layout. No real account credentials were used.
- Created a superflat Creative world on the default server thread. Third-person
  views showed the imported skin and cape. Returned to the launcher, changed to
  Classic and a different cape, and reopened the world in the same process: both
  changes reached the game. This is a VM observation, not a phone performance claim.

The VM APK used development dependencies with the native game module compiled at
optimization level 1. A fully unoptimized module overflowed the native activity
thread stack after renderer preparation; the optimized module completed the above
checks. Generated 26.3 tables came from the fork's published 0.7.6 APK; original
resources were downloaded by the installer inside the disposable emulator. A
release APK with freshly prepared tables was not validated here.

Live Microsoft/Ely.by sign-in, real cloud mutations and an Ely.by server connection
still require account-owner testing. Provider-specific skins of other players on
Ely.by servers are not extended by this change; the gallery and local-player
handoff load the signed-in account's current public textures.

The full desktop `pr_ready --quick --base android` is not green: Python discovery
collides on the existing `test_basic_feature_parity` module, and strict workspace
Clippy reports existing findings in `horizon-world`. Android checks above are
separate; these desktop failures must not be represented as passing.

A final APK retest also produced one native fault in the emulator's
`vulkan.ranchu.so` while naming a swapchain texture view. The same APK opened the
gallery normally on retry. This VM driver observation is separate from PNG
validation and does not establish stability on physical Android GPUs.
