# 02: GPU fog kill gate

## Contract

The per-eye GPU fog can take a dynamic list of smoke circles. Two things must
hold:
- **Parity:** the WGSL smoke step agrees with `contract::smoke::piece_depth`.
- **Cost:** a cloud changing step rebuilds only the eyes within reach, inside
  the existing budget.

**Verdict:** **keep**, or **kill and reslice**. On kill, stop, rewrite this
spec from slice 02, and tell the user. There is no degraded fallback: no grid
smoke and no drift (D1, D5, D8).

## Seam

- **WGSL:** add `fogSmokeStep(a, b, sightZ, groundZ) -> f32` in
  `crates/renderer-core/src/shaders/modules/fog.wgsl`. It reads a storage buffer
  of `SmokeCircle { x, y, r, height, depth_per_m }` and is called from the
  terrain march in `passes_fog.wgsl` (~259) next to the foliage read.
- **Parity probe:** add a `probeFogSmoke` compute entry, modelled on
  `probeFogShapes`. A test in `crates/renderer-core/tests/probes.rs` runs a
  table of lines and clouds through both the WGSL and
  `contract::smoke::piece_depth`, and requires agreement within f32 tolerance.
  Add `contract` as a dev-dependency of `renderer-core`.
- **Dirty eyes:** `FogSight::set_smoke(&[(id, phase, SmokeCircle)])` in
  `crates/presentation/src/fog/` diffs the list against the previous one. Eyes
  whose reach touches a changed circle are queued under
  `rebuild_eyes_per_frame` (48), following the precedent of `set_occluders`
  (`sight.rs:1008`).
  - **Trap:** an eye whose built map is dropped (`built = None`) rebuilds
  immediately and skips the budget (`sight.rs:1033-1038`). Queue those eyes; do
  not drop their maps.
  - Put new code in a new module: `fog/sight.rs` is over the size budget.
- **Data path:** confirm that the publication's eyes already reach the fog
  inside the presentation crate (`presentation/side_fog.rs`, `Fog.from`), so
  clouds can take the same path without touching
  `apps/battle-lab/src/useBattleSession.ts:397` or
  `native/client/src/capture.rs:578`. Record the answer in `choices.md`
  (the row marked "to confirm in slice 02").

## What you can see

A frame-cost table in `throwaway/smoke-screen/gate/`, measured with 100 eyes and
0, 4 and 12 clouds, through bloom and fade. It records:
- `FogStats.rebuilt` for each frame;
- the worst and median GPU fog time;
- a comparison with the 1.55 ms worst case at 100 eyes in
  `specs/done/battle-look/frame-cost.md`.

## Gate

The gate passes when all of these hold:
- the parity test is green;
- no frame rebuilds more than 48 eyes;
- eyes outside every changed cloud's reach rebuild 0 times;
- frames stay above the 30 FPS floor in `frame-cost.md:3`.

The 1.55 ms figure is a tripwire, not a target ("numbers are not
requirements"). Write the verdict and the table into `choices.md`.

## Delegated

You may choose the buffer layout and its binding slot, the probe's table, and
how circles are packed. A probe harness that isn't the parity test may be
scratch code.

## Must stay green

- The renderer-core and presentation fog tests.
- The fog-sight vectors, unchanged, because no clouds exist in them.

## Feedback that would change this slice

If the GPU time is over budget, consider a coarser cloud step before
abandoning circles. Any approach that changes D1 or D5 needs the user's
approval.
