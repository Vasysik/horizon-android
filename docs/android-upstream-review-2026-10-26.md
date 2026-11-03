# Android upstream review, 26 October 2026

Integrated 69 commits across 25 stable merge batches, from 29c09b4b through
5ba4793d. This is a reviewed subset of main, not a merge of every fetched change.

- Android helper-thread scheduling now uses Linux/Android nice levels.
- Requested GUI scale survives temporary small framebuffers and rotation.
- Global level sounds, anvil sound duplication, heart blinking, leashes,
  crop interactions, statue poses and item-frame name display are corrected.
- Optional native mod command/render capabilities retain explicit manifest
  permissions and bounded output; no installed mods means no mod render work.
- Pregeneration is opt-in; ordinary streaming retains its normal area.
- Preparation checks output identity and reuses verified files, with stronger
  hashes for indexed replacements. Prepared-output version remains 1.
- New parity CI and deployment workflow updates remain under
  `.github/upstream-workflows/*.disabled` in this Android fork.

Manual resolutions preserved Android HUD information margins, touch preference
synchronization, game/launcher renderer separation, player-name lookup and the
Android workspace crate. Both remote sound lookup and player-name lookup remain.
Android's previous two-core worker headroom is retained.

Later shader-pack and server-mod changes are deferred to a separate review.
Android builds, targeted tests and public payload checks gate publication.
