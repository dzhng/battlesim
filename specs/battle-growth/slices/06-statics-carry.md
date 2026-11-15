# 06 Statics carry (conditional)

**Question:** does growth that changes no ground, scenery or grass still do
scenery and grass work?

**Depends on:** 00.

## Run this slice only if

Slice 00 measured more than about one frame of scenery and grass work at
growth in the reference battle. If it measured less, record the number in the
[ledger](../decisions.md), mark this slice dropped in the README's checklist,
and move on: the margin is spent on keeping the admission simple.

## What happens today

Every stage call re-unrolls the finest scenery meshes and re-runs whole-map
scenery placement
([`stage_assets`](../../../crates/renderer-core/src/runtime/assets.rs),
[static_art.rs](../../../crates/presentation/src/presentation/catalog/static_art.rs)),
then builds a new `Scenery` and `Grass`. It cost 50 ms on the small
`battle-growth` map.

## Contract

An admission whose ground, scenery kinds, grass kinds, their content and the
grass density are unchanged does no scenery placement and uploads no scenery
or grass.

## Seam

- The live scenery tiers and grass are carried into the new generation by
  shared handle. Commit replaces only the owners that changed.
- Placement is carried too. The appearance rows it names are resolved again,
  because rows renumber.

## Verification

Write first; test 1 is red today.

1. [`crates/presentation/tests/catalog_statics.rs`](../../../crates/presentation/tests/catalog_statics.rs):
   adding a unit appearance leaves every placement identical and counts no
   placement run.
2. `asset_generations.rs`: scenery and grass bytes uploaded are zero, and the
   committed frame is pixel-equal to a clean build.
3. Changing a scenery kind's content does place and upload again.

## What you can run or see

`battle-growth` and the reference battle report no scenery work at growth.

## Delegated

How unchanged identity is detected.

## Stays green

`tests/grass.rs`, `tests/grass_assets.rs`, felled trees and stumps (their rows
are placement-owned), the visual suite.

## Feedback that would change this slice

A measured cost under a frame drops the slice (see the rule at the top).
