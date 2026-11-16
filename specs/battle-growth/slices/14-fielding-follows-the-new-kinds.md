# 14 Fielding's result follows the new kinds

**Question:** can a growth stop rebuilding and re-reading the whole fielded
battle's description?

**Depends on:** 12. Added 2026-11-15 from slice 12's confirm-first
measurement. Not started; it is the floor under every arrival, not the hitch
slice 12 removes.

## What happens today

Each growth, [`Fielding::resolve`](../../../crates/presentation/src/fielded.rs)
assembles the units and effects sections for every fielded kind again, and its
[binding](../../../crates/presentation-wasm/src/fielded.rs) serialises them:
2.6 million characters in the reference battle. The page parses that, composes
the world input around it, serialises it again and hands it back to the same
WASM module, which parses it. With every new content's facts already cached
and nothing supplied, the resolve alone took 40, 37 and 35 ms in three
trials (optimised WASM, development shell); the typed world load after it
is about 10 ms more. A one-vehicle arrival's whole frame is about 52 ms in a
production build, so this is most of what is left of it.

## Confirm first

- What the page reads out of the units and effects sections itself, apart
  from passing them back. If nothing, the sections need not leave Rust.
- The same timings in a production build.

## Contract

A growth's fielding work follows the kinds it adds. What a fielded battle
already described is not assembled, serialised or parsed again.

## Seam

Decided after the first confirm-first answer. The two candidates: the
sections stay in Rust and the page holds a handle (fielding and presentation
are one WASM module already), or the resolve returns only what the new kinds
add and both sides merge it.

**Fixed:** no switch or setting; one fielding owner per battle scope, as now.

## Verification

Write first: a second resolve that adds one kind returns, or does, work for
that kind alone, counted the way `AdmissionWork` counts the rest; the fielded
battle it yields equals a cold resolve of both kinds (the existing test's
oracle).

## Outcome

Built with the first candidate, and more: the sections stay in Rust behind a
handle, and assembly is incremental too, because confirm-first found it grew
with every fielded kind. The final fielding step is under 1 ms. See the
ledger, "Slice 14 as merged".

## Feedback that would change this slice

The page turns out to read most of the sections. Then the delta is the only
candidate, and its merge has to be shown equal to a cold resolve.
