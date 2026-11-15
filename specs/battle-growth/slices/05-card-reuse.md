# 05 Card reuse

**Question:** can a new impostor card join without re-baking the old ones?

**Depends on:** 04.

## What happens today

[`bake_cards`](../../../crates/renderer-core/src/models/impostors.rs) bakes one
atlas for every soldier and corpse row, one at a time, each waiting on three
GPU reads, and uploads the whole materials table again for each. Rows that
share art are baked again. The cards live in one array-texture pair per
generation, one layer per baked row, destroyed outright when the generation
drops.

## Contract

A card is baked once per scene for each distinct content, pose and bake
specification.

## Seam

- **Card key:** the tier-0 geometry key from slice 03 plus the bake
  specification. Rows with the same key share a layer.
- **The card arrays** are shared textures with spare layers, under slice 04's
  rules. The per-generation frames table and the row-to-layer table are
  rebuilt; they are small.
- One authored-materials buffer per stage, not one per row.
- **New cards bake together.** All of a stage's new cards are rendered in one
  submission and their reads are awaited together, not one card after another.
  Slice 00 measured why: in the reference battle each read waits about 0.1 s
  behind the battle's own frames, so eighteen new cards read in turn would
  still take about six seconds.
- `Atlas.hash` stays the cross-platform test oracle. It is not the key: it
  includes the appearance's name
  ([impostor/mod.rs](../../../crates/renderer-core/src/impostor/mod.rs)).

## Verification

Write first; test 1 is red today.

1. Commit {x, y}, then stage and prepare {a, x, y}: cards baked equal `a`'s
   new keys only.
2. A reused card's pixels equal a fresh bake's.
3. Two rows with the same key share one layer.
4. A pending bake that is dropped leaves the live cards drawing the same.
5. Several new cards in one stage produce the same atlases as baking each
   alone.

## What you can run or see

In the reference battle, the wait from `battle staged` to `art committed`
shrinks to the new type's cards.

## Delegated

The arrays' growth step.

**Fixed:** no cancellation of an in-flight bake (see the
[ledger](../decisions.md)).

## Stays green

`tests/impostors.rs` (the bake keeps coverage, normals and tint; texture
channel diagnostics do not change the bake).

## Feedback that would change this slice

If new cards baked together still leave a visible wait in the reference
battle, record the measured wait; the stand-in covers it, and the preload's
depth (slice 09) is decided with that number.
