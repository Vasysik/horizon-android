# Upstream integration, 28 October 2026

The 0.7.1 integration reviews 68 main-branch commits after `8155baa6`, through
`1feb28c0`. The base is the published Android 0.7.0 (`23658ad4`), including its
installer fix. No experimental branch is merged separately.

- Input moves into `horizon-client-input`, with the old client paths re-exported.
  Android clipboard callbacks move with the module; desktop-only arboard remains
  excluded on Android. Keypad, touch and NativeActivity input retain their bridges.
- Mob sound loops, mob state synchronization, fishing interactions, campfire
  extinguishing and held bundle use follow the upstream changes.
- Account integration keeps Android Keystore and the legacy endpoint support.
  Upstream account unit tests use memory stores rather than the player's keychain.
  Desktop launcher ownership screens do not replace the Android launcher UI.
- Upstream triage and publication workflows remain inactive. Fork-specific public
  documentation and the reviewed Android installer fix are retained.

Android compilation, targeted tests and device checks are recorded separately;
this review does not establish performance or device compatibility.
