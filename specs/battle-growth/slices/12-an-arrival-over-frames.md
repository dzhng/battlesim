# 12 An arrival over frames

**Question:** can one unit type's art be staged without a frame the player
notices?

**Depends on:** 09. Added 2026-11-15 from the README's reslice trigger, when
slice 09 found the largest arrival is a single type.

## Where it stands

Its first confirm-first question stopped it once, as written: 57.2 MB of the
RPG team's 85.7 MB was copies posed for bodies that did not exist yet.
[Slice 13](13-a-body-draws-its-types-mesh.md) removed those copies. The team
now packs 28.6 MB and its longest frame over three runs is 222, 194 and
256 ms (400, 276 and 237 ms before): better, and still a hitch. This slice is
the step left. Its first question is answered; the second (does each cost
divide by content) is still to take, on the tree with slice 13 in it. The
numbers below are from before slice 13.

## What happens today

Every count is already the new art's alone, and the frame is still long
because one type can be a lot of art. The reference battle's RPG team is nine
bundles (three soldiers, each active, carrying and on guard), 85.7 MB of
packed vertices and 27 cards. Its staging is two long frames: 150 ms of
fielding and transport, then 285–357 ms of admission. One profiled run of the
second (ms): fielding 82, the nine supplies with their decode 43, catalog and
GPU preparation 171, staging 14, the rest 46. A two-bundle vehicle is a 52 ms
frame. Infantry teams are common, so most battles meet several.

All of it runs in one go because nothing yields: the page inflates the type's
files, resolves fielding from all of them, supplies every bundle to the draft,
and [`stage_assets`](../../../crates/renderer-core/src/runtime/assets.rs)
packs and uploads every missing geometry before it returns.

## Confirm first

- What the 85.7 MB is: per bundle, how much is the living mesh's tiers and
  how much is a copy posed for a corpse or a far rest pose. If most of it is
  posed copies for bodies that do not exist yet, record that and stop before
  building: packing a body's geometry when the first one lies may be the
  smaller fix, and it is a different slice.
- That fielding, supply and preparation each divide by content: time one
  bundle of each. A cost that does not divide (one table rebuilt whole) is
  named with its time.

## Contract

- Staging a generation does about one content's work in a frame, never a
  type's or a catalog's. The staging takes more frames; no frame is long.
- The swap stays atomic. Nothing of a staging generation is drawn before its
  commit, the stand-in covers the type meanwhile, and a staging that is
  dropped or superseded part-way leaves live art as it was.
- A generation staged over frames is the same generation: its admission work
  adds up to the same counts, and its committed frame is pixel-equal to one
  staged at once.

## Seam

- **Warm the store, then admit.** The art store already lets an admission
  find geometry by key. A pending draft packs and uploads one missing key per
  step and holds what it packed; the admission that follows finds every key
  and only rebuilds its tables. `Models::from_world` stays the one admission
  path: no second, incremental copy of it.
- **The shell steps it.** The viewport already owns latest-request scheduling
  with one candidate at a time; it runs one step per presented frame.
- **Supplies** go to the draft one content per frame the same way.
- **Fielding** absorbs one content's facts per step into the `Facts` it keeps;
  the resolve that follows reads none.
- A step that would take a type's time by itself (one very large mesh) is
  recorded with its cost; it is not split further here.

**Fixed:** no worker and no thread; no shader or vertex-layout change; no
switch, flag or setting; one candidate at a time; the 700 ms tripwire in
`battle-growth` is not loosened, and no new number is pinned as a gate.

## Verification

Write first; tests 1 and 2 are red today.

1. In `crates/renderer-core/tests/asset_generations/`: commit {x, y}, warm
   {a} step by step, then stage {a, x, y}. Each step packs one key's bytes;
   the staging after the last step packs none; the totals equal staging at
   once.
2. The committed frame is pixel-equal to a clean runtime built with
   {a, x, y}. Dropping the draft after some steps leaves the live picture and
   releases what it held.
3. In the page's tests: a growth of several contents supplies and steps them
   over separate frames and commits once; superseding it mid-way commits only
   the newer set.

Then the reference battle with [the probe](../assets/growth-probe.mjs), three
runs before and after: for each arrival the longest frame from learning to
commit, and the time from learning to commit. Both go in the ledger. The
second will grow; say by how much.

## What you can run or see

In the reference battle the RPG team's block stands a moment longer, and the
battle does not hitch when the team appears.

## Delegated

The step's size (one key, or a byte budget a frame), the names, and where the
held geometry lives until admission.

## Stays green

Everything in `asset_generations`, `tests/impostors.rs`, `model_geometry`,
the page's admission tests, and the scenes `battle-growth`, `stand-in`,
`armory` and `workbench`.

## Feedback that would change this slice

- The first confirm-first answer (posed copies dominate).
- After stepping, a frame that is still long because of work that does not
  divide: record what it is and stop.
- A wait behind the stand-in long enough to notice. Then the trade is wrong
  and the work has to shrink, not spread.
