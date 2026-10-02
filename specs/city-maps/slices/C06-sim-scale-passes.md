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

## Measured finding — before local fog invalidation

Any prop mutation invalidated every subsequently visited occlusion tile, even
when the changed footprint was distant. Each such tile rebuild allocated, sorted
and deduplicated its nearby prop candidates. The cache limited permanent raster
work to visited tiles, but the global revision amplified a local change into
all active eyes' tiles. Native contact fog averages 51–56 M instructions/tick;
the stack probe also samples candidate sorting. A public sweep cost comparison
and fresh-raster parity are the acceptance seams for local invalidation.

## Outcome — local fog invalidation checkpoint

World's existing footprint buckets now retain monotonic mutation stamps. Fog
tiles check their local maximum only after the global obstacle revision changes;
unchanged candidate buckets prove their cached solid heights remain exact. Moves
touch old and new footprints, and removal leaves a stamp in empty buckets. The
raster's cell centres, body ordering and height arithmetic are unchanged.

The old public-sweep cost regression retired 43.6 M instructions for eight
distant-only changes versus 9.48 M for unchanged sweeps. Local tracking passes
the bound of less than twice its steady sweep cost, with identical visibility
bits. Fresh complete sweeps agree through authored revision zero, local additions,
moves, deletions, height replacements, overlapping bodies and partial edge tiles.
Removing deletion stamping makes that parity check fail; restoring it passes.
Focused sight, village replay and library clippy pass on the engagement-guard base.

Added permanent metadata is 6.25 MB at 20 × 20 km with 8 m fog cells: 3.125 MB
for world bucket stamps and 3.125 MB more for tile revisions. Side-known indexes
allocate no stamps, and mutations add no history or scratch buffer. The existing
50 MB fine solid-height raster is unchanged; this pass does not claim to solve
G0 storage. Matched native city digests/costs and browser admission remain the
integrating scale lane's next verification; C06 remains open.

## Finding — infantry town route timing

The coarse infantry search accepts connected free half-metre gaps, but its
reconstructed corner link can cut a blocked sub-cell. Smoothing retains a raw
adjacent link even when its sampled cost is infinite. A public route-time test
fails on one small building corner, before any generated map or large search.
The selected physical correction bends the route through those existing free
sub-cells. It changes infantry waypoints and counted planning progress; this is
a named route correction, not a digest-neutral performance pass. Vehicles keep
their existing clearance and reconstruction. Local connection work must remain
bounded and incremental, with no whole-world fine grid.

## Outcome — infantry corner connectors

Infantry reconstruction now joins the admitted free half-metre gaps inside each
two-metre cell before smoothing. One incremental step runs at most two cell-local
connector searches; each queue has at most sixteen entries. Endpoint fallback
keeps the existing bounded reach. Every emitted link and every
longer smoothing shortcut passes the existing sampled segment reader. Infinity
still means a rejected segment, and walls and disconnected terrain stay blocked.
Smoothing addresses infantry points through an implicit range, so entering that
phase performs no eager scan or turn-index allocation over the expanded route.
Legal infantry endpoints start in their containing cells rather than a nearby
cell across an obstacle. Existing bounded fallback for an inside-map squad
centroid remains; an outside-map start is explicitly enclosed.

The old public timing regression failed at a building corner. Six corner
placements/rotations now return legal, finite routes. A one-man passage admits
infantry while rejecting a tank; closing it or removing a river crossing rejects
the route. Boundary and zero-distance semantics pass. A small Battle follows the
corrected corner route under a one-unit planning allowance: no soldier body
enters the wall, the squad arrives, and replay agrees at every tick and in its
record. All 22 navigation, 15 route-planning and 14 movement tests and library
clippy pass on the 4efec25b rules baseline.

This is a named physical route correction: infantry waypoints, route cost,
planning completion ticks and affected battle digests can change. Vehicles keep
their reconstruction. There is no permanent full-extent allocation: the new
stage retains only its coarse route and emitted points; fixed local queues live
on the stack. The sampled NavGrid guarantee is distinct from movement's exact
body checks. The focused Battle proves that concrete corner, not all continuous
geometry. Generated town timings, matched native city cost and visual admission
remain with the integrating scale lane; C06 stays open.
