# Upstream renderer review — 2026-11-02

The Android merged baseline remains `8c02c0b8534e11b7bb8be14238b140a8534280a0`.
This review does not merge a new engine baseline or claim Android device validation
of upstream changes. It examines rendering and related performance changes, not
every commit in the ranges below.

## Candidate checkpoints

| Checkpoint | Commits beyond baseline, including merge history | Evidence and scope |
| --- | ---: | --- |
| `c9d474dd9116e84c243f8992d75c11314df56f06` | 482 | Published rolling nightly on November 1; `verify`, `launcher`, `parity-main` completed successfully when inspected |
| `549d6b9cd53e4ea0c9eb388f4e99fad8175deda1` | 3,680 | Fetched main on November 2; checks still in progress when inspected; not a completed release checkpoint |

The immutable source tag for the first checkpoint is
`nightly-source/2026-11-01-c9d474d`. The rolling `nightly` tag and dated nightly tags
can move; existing local tags were not overwritten. The newer desktop launcher
release tags are not engine stability certifications.

A non-mutating merge preview of the nightly found 12 conflicting files, including
the renderer, arena, WGSL, game loop, remote connection handling, IME, mods and
account code. Its aggregate change from the baseline is 372 files, about 30,000
insertions. This is a separate integration task with Android regression checks,
not a small performance patch.

## Rendering findings

- [`c9614815f`](https://github.com/horizon-rust/horizon/commit/c9614815f)
  introduces a submission thread and can draw the first-person hand in the world
  pass. Follow-up [`fea41f867`](https://github.com/horizon-rust/horizon/commit/fea41f867)
  limits automatic threaded submission to Linux after severe Metal regressions.
  Preserve that platform gate; Android requires its own comparison. A rendering
  thread's existence alone is not evidence of higher mobile FPS.
- [`27a5ccdbc`](https://github.com/horizon-rust/horizon/commit/27a5ccdbc)
  adds an optional separate arena for translucent geometry, off by default.
  The initial implementation excludes ordinary flat-index frames and can grow
  pages toward 2 GiB. Do not transplant those allocations as Android defaults.
  Later [`18ac513e8`](https://github.com/horizon-rust/horizon/commit/18ac513e8)
  bounds bindings to used ranges, avoiding GPU initialization of unused tails;
  [`b91226cac`](https://github.com/horizon-rust/horizon/commit/b91226cac)
  adds shrinking after terrain settles. Neither follow-up is included in the
  published candidate above.
- [`b0f9532aa`](https://github.com/horizon-rust/horizon/commit/b0f9532aa)
  subsequently supports translucent arenas in flat-index frames. It changes
  section IDs, arena moves, index addressing and draw binding together. Taking
  only its draw-call reduction would omit necessary ownership changes. Test water,
  glass, arena growth/shrink, resource reloads and native/classic shader packs.
- [`873ff80cd`](https://github.com/horizon-rust/horizon/commit/873ff80cd)
  defers buffer release off the render thread. It targets frame spikes on page
  retirement rather than steady-state fragment cost. Android GLES context use,
  bind-group ownership and submission lifetime require review before adaptation.
- [`45129f311`](https://github.com/horizon-rust/horizon/commit/45129f311)
  releases unused blur/outline targets. Our blur caches bind groups, so simply
  clearing textures would retain their views through those groups. Any adaptation
  must release dependent references and measure menu re-entry allocation spikes.
- [`2b874aea1`](https://github.com/horizon-rust/horizon/commit/2b874aea1)
  halves blur dimensions only on software adapters. Hardware GPUs keep their
  previous path. It is not a demonstrated Adreno blur optimization.

## Other relevant hot paths

`1c223ab04` avoids repeated colored-light item gathering without a native pack
requesting colors. Our baseline predates that exact dynamic-light path; it is not
a standalone clean patch. `7a9915273` makes the 4 MiB native image slot optional,
but Android's bundled/sandbox mod requirements must be preserved.

`ec99e376a` guards native camera construction when no native mods read it; the
subsequent `bc04fc95f` fixes a compile error in that change. These illustrate why
an arbitrary intermediate commit must not be called stable. `d1ebc0182` skips
block-entity scans when no relevant mod block entities exist, depending on newer
block-entity bookkeeping.

## Decision

Keep the existing Android baseline for the current tested candidate. Use
`c9d474d` as the bounded candidate for a future full synchronization, with all
12 conflicts resolved explicitly and device tests repeated. Before moving further,
review the complete translucent-arena follow-up series rather than enabling its
first experimental version. No inspected upstream commit establishes a fix for
the remaining Adreno 810 steady-state slowdown.
