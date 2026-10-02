# C06: sim scale passes

**Depends on:** C05 (conditional on S1 and C05). **Kind:** slice.

## Question
Do the sim's static-occluder and nav costs fit the budget at city scale?

## Contract it unlocks
Only the fixes S1 or C05 flag, one commit each, likely:
- (a) `OcclusionGrid::refresh` rebuilds only the dirty cells of the props that changed;
- (b) nav rebuilds only the cells under the changed bodies (`movement/mod.rs:208-213`).

**Closes without code if the numbers are under budget.**

This pass handles local active-work costs after G0's full-extent storage/export architecture is proven. Dirty rebuilds cannot fix dense permanent allocations. Any required terrain/navigation/foliage representation change gets its own prerequisite slice at G0; do not hide it inside this pass.

## API seam
`sim::visibility`, `sim::navigation`, `sim::movement`.

## What the human can run or see
`city_report` rows before and after.

## Verification
- Property tests: incremental equals a full rebuild, byte for byte.
- Village and city digests unchanged.
- Instructions retired; `village_report --compare main` as a cross-check (0 battles if digests match).

## Delegated to the implementer
Dirty-region bookkeeping. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
All digests.

## Feedback that would change this slice
A measured rebuild bottleneck selects a focused incremental owner; changed outcomes invalidate this performance-only pass.

## Outcome — termination pass; scale still open

A soldier's rejoin search sampled farther along a corridor until the candidate
left local reach. The sampler clamps at the endpoint, so a blocked endpoint
inside local reach could be tested forever. The search now stops after exhausting
the remaining corridor; the existing fallback can then run. The red/green
regression is bounded by its predicate and proves search exhaustion without
hanging the harness. Successful candidate order is unchanged.

Fine routes now ask the existing boolean walkability query; bridges, water,
steep ground and bounds agree with the full surface query. This simplification
produced no measured speedup and does not substantiate a fine-routing bottleneck.
Matched city early/late digests and [system costs](../scale-lane.md#c06-termination-checkpoint)
remain the scale lane's evidence. C06 is still open against the tick budget.

## Outcome — exact per-tick bounds; cover still red

Hull traffic now visits only living, standing squads in original unit order.
Its current footprint bounds refresh after every relevant movement, so later
vehicles see the same geometry. Sensing derives target bounds once while its
unit slice is immutable. The regression measures fallen-squad amplification in
an isolated process: remains may add gather work, but more vehicles must not
multiply that work. Restoring the old traffic scan makes the regression fail.

Matched city early/late digests are unchanged; measured instruction gains and
the next cover-planning owner live in the scale lane's
[checkpoint](../scale-lane.md#c06-live-traffic-and-sight-bounds-checkpoint).
No mechanic or scheduling delay was introduced. C06 remains open.

## Outcome — impossible engagement searches

The cover probe found 146 resolutions in the first two seconds against targets
beyond weapon, holding-area and lean reach. Their failed step-out searches alone
retired 18.7 G instructions. A conservative necessary range condition now avoids
that proven empty search. Ordinary near-target candidates and boundary cases keep
the original exact search. The negative public-battle cost test and positive
edge-lean test both have red/green proofs; existing cover/replay tests stay green.
Matched city costs and unchanged digests live in the
[checkpoint](../scale-lane.md#c06-impossible-engagement-checkpoint).
