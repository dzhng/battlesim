# 05a Cards bake together

**Question:** does a stage's card bake stop waiting on each card in turn?

**Depends on:** 00. Split out of [05](05-card-reuse.md) on 2026-11-15 because
it needs none of the reuse chain and removes the longest wait on its own.

## What happens today

[`bake_cards`](../../../crates/renderer-core/src/models/impostors.rs) submits
one card, awaits its three GPU reads one after another
([`BakeRequest::read`](../../../crates/renderer-core/src/impostor/bake.rs)),
then submits the next. In the reference battle a growth bakes 189 cards and
commits 62–69 s after staging; the same cards take 1.4 s in the small
skirmish. The [ledger](../decisions.md) infers that each read waits about
0.1 s behind the battle's own frames (567 reads in turn is about a minute).

## Confirm first

That inference: no timer isolated the bake. Time the bake phase in the
reference battle with [the probe](../assets/growth-probe.mjs) and a temporary
timer, and count the awaited reads. If the minute is somewhere else, stop and
record where; do not build this slice on a wrong cause.

## Contract

- A stage's cards are submitted together and their reads are awaited together,
  in batches bounded by the memory their targets and readbacks hold. The
  number of waits a bake makes follows the number of batches, not the number
  of cards.
- The atlases are the same bytes as baking each card alone.
- One authored-materials upload per stage, not one per card.

## Seam

- `bake_cards` renders a batch of cards, then awaits that batch's reads
  together.
- The batch bound is in bytes of render target and readback, not a card
  count: one runtime card is about 3 MB across its three targets, so a full
  rebuild's cards cannot all be held at once. A new type's cards fit in one
  batch.
- `AdmissionWork` gains the count of read waits a bake made, so the proof
  counts work and not time.

**Fixed:** no cancellation of an in-flight bake (see the ledger). The swap
stays atomic: a dropped or stale bake leaves the live cards drawing the same.

## Verification

Write first; tests 1 and 2 are red today.

1. In `crates/renderer-core/tests/asset_generations.rs`: a stage that bakes
   several cards reports one read wait per batch. A bound set by the test
   forces more than one batch, and the count follows it.
2. Baking several cards in one batch produces the same atlases as baking each
   alone (`Atlas.hash`, and the pixels).
3. The materials table is uploaded once for the stage: buffer bytes uploaded
   do not grow with the number of cards baked.

Then the probe on the reference battle: record staging-to-commit before and
after in the ledger. It is evidence, not a gate.

## What you can run or see

In the reference battle, new art appears seconds after `battle staged`, not a
minute.

## Delegated

The batch bound's value (a tripwire, logged when it splits a bake), and how
the reads are awaited together.

## Stays green

`tests/impostors.rs` (the bake keeps coverage, normals and tint; texture
channel diagnostics do not change the bake), the swap test in
`asset_generations.rs`.

## Feedback that would change this slice

If the reads still resolve one frame apart when awaited together, record the
measured wait and stop: the remaining fix is slice 05's reuse, which makes the
batch small.
