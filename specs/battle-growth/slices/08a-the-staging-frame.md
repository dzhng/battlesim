# 08a The staging frame

**Question:** what still makes one frame long when a type arrives, and does
it still scale with the catalog?

**Depends on:** 08. Added 2026-11-15, when slice 08's gate measured the
freeze the counts no longer explain.

## What happens today

With slices 01–08 merged (card reuse, slice 05, not yet), a growth in
`battle-growth` decodes 2 bundles, sends 8 MB, packs 8.6 MB of vertices,
writes 3 texture layers and does no scenery work. One frame at
`battle staged` is still 735 ms at the scene's profile, and 0.7–1.6 s in the
reference battle at high graphics. Every count `AdmissionWork` has is already
the new art's, so the time is in work nothing counts.

Read from the code, not timed:

- [`Models::from_world`](../../../crates/renderer-core/src/models/admission.rs)
  walks every appearance row. For each corpse row it skins the finest mesh
  again to find its rest bounds (`RestPose::new` over `posed_mesh`), outside
  the geometry cache; for each other row it scans the mesh for bounds. The
  first measurement put "rest poses and other loop work" at about 0.5 s.
- The materials table is rebuilt for every content, and the row tables for
  every row.
- The card bake submits all 189 cards (until slice 05).
- Growing a texture array copies the live layers on the GPU (40 MB here).
- Presentation stages a whole catalog: rows, strings, the battle's unit
  tables. The page builds the wanted list, asks what is missing, and reads
  the world's rows back.

## Confirm first

Time the staging frame's parts in the reference battle at high graphics and
in `battle-growth`, with temporary timers, three growths each: the page's
work before the call, presentation's `stage_catalog`, each part of
`Models::from_world` (row walk, rest poses, materials, texture store), scenery
and grass reuse, the card bake's submission, and the page's work after. The
parts must add up to the frame. Record the table in the ledger. Timers are
scratch.

## Contract

The staging frame's work follows the new art. What a row needs that depends
only on its content and pose is derived once per scene and shared, as its
geometry is.

## Seam

- Whatever the timing finds that scales with the catalog and depends only on
  content gets the store's key and lifetime (the geometry key of slice 03).
  No second cache: extend the one owner.
- `AdmissionWork` counts the kind of work that was hiding (for example rows
  whose rest pose was derived), so the proof is a count.
- What is left and is honestly per generation (small tables) is named in the
  ledger with its measured cost.

**Fixed:** no shader or vertex-layout change; the swap stays atomic; nothing
is spread over frames or moved to a worker here (that is the reslice trigger
in the README, and it is decided after this slice's numbers).

## Verification

Write first; test 1 is red today.

1. In `crates/renderer-core/tests/asset_generations/`: commit {x, y}, then
   stage {a, x, y}: the new count equals `a`'s rows only.
2. The committed frame is pixel-equal to a clean build, and cards baked from
   a shared rest pose equal a fresh bake's.
3. `battle-growth`'s gate asserts the new count against the new art it
   observed, beside the others.

Then the measurements again, before and after, in the ledger. The gate's
long-frame tripwire is not loosened to pass: if the frame is still over it,
say what the remaining time is.

## What you can run or see

The long frame at `battle staged` in the diagnostics report shrinks to what
the new art costs.

## Delegated

The count's name, and where the shared facts live in the store.

## Stays green

Everything in `asset_generations`, `tests/impostors.rs`, `model_geometry`,
the scenes `battle-growth`, `stand-in`, `armory` and `workbench`.

## Feedback that would change this slice

If the time is not in catalog-proportional work at all (one large fixed cost,
or the GPU copy), record where it is and stop: the fix is then a different
slice, or the reslice trigger.
