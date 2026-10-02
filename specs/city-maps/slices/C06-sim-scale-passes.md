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
cell across an obstacle. Bounded nearest-fit selection remains, but a returned
route still needs a certified first link; an outside-map start is enclosed.

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

### Follow-up — squad representative starts

The movement owner already starts a squad corridor at its closest living
soldier, because a legal squad's centroid can lie inside a body. The conservative
grid can nevertheless reject that physically clear soldier. A narrow-wall Battle
arrives on the pre-correction build but becomes blocked after connector
certification; another existing soldier is grid-standing. Planning now selects
the nearest living member accepted by its side-known grid before falling back
to the original nearest member. It grants no escape through a blocked mask and
uses no hidden world geometry. The distance used for road preference is unchanged.
Non-standing goals still project to a legal, finitely timed endpoint.

The exact failing Battle now arrives with all soldier bodies clear at every
tick and identical same-build replay digests. The pre-correction comparison
used the same frozen rules, not the integrating branch's new art catalogue.
All 23 navigation, 15 route-planning and 15 movement tests and library clippy
pass. This admission correction changes affected squad starts and digests; it
adds no state, storage, public API or relaxed segment rule. Independent review
found no remaining issue.

## Outcome — full generated browser stress fixture

The existing endurance route has a bounded generated arm: the current Metro
Large seed-4 map, under game admission, with the simulation's `city-arena-1`
contact workload. The full world remains loaded around the central arena. Map
resolution and stress placement run in the existing preparation worker; its
optional lab selector leaves ordinary preparation and assault planning intact.
The Wasm response carries canonical scenario bytes and a small authoritative
start/roster report; preparation does not parse the full scenario again merely
to recover metadata. Changing the fixture or leaving it cancels that worker.
The saved endurance
battle remains the route and scene default.

The prepared report carries the admitted map identity, physical counts, extent
and living rosters. The scene selects this arm and checks its early/late world
identity while retaining its soak and reset contracts. Usage lives with the
route and scene. Narrow red/green tracers cover the authoritative stress branch,
malformed selector refusal and cancellation. Rebuilt Wasm verifies full-map
retention and the early/late roster and remains. Normal preparation checks stay
green. A Node/Wasm preparation on the current physical catalogue admits the
10 × 10 km Metro Large seed-4 map with 100 living units per side. Its Node
process peaks at about 789 MiB RSS with 354 MB of Wasm memory; this excludes
browser transfer, the battle authority and rendering.

This is tooling readiness, not browser admission. The real browser worker,
rendered picture, startup overlap, reset allocations and throughput still need
the integrating agent's coordinated GPU run. No rendering or game rule changed.
## Finding — fog candidate collection

Bounded public sweeps on the frozen full Metro input separate world candidates
from traversal. With current rules and release optimization, paired-side steady
candidate gathering costs about 104 M instructions early and 132 M late;
traversal costs about 200 M. Ray direction and sight-range math is only 18 M of
that traversal, so a trig cache is not the first fix. These are static initial
views, not a claim about every contact tick. Empty remembered-index queries
provide only a lower bound on that separate owner.

Every eye sorts its world candidates, although the Fog caller then canonicalizes
all eyes' IDs before learning. A scratch batch of the same bucket entries has
exactly the same final ID set. Including its more expensive final merge, early
collection falls from roughly 142 M to 99 M instructions across both sides.
Raw candidate volume grows by 2.3–2.8 times, so the production pass must disclose
transient memory and prove whole Fog plus Learning work, not merely move sorting
across the profile boundary. Preserve ordinary sorted query consumers and all
body knowledge; add no retained query cache or full-extent fine storage.


## Outcome — collect fog candidates once

Fog now appends the existing world and remembered bucket entries for every eye,
then canonicalizes each union before its first body-knowledge read. Ordinary
spatial queries retain sorted, unique results. The visibility kernel receives
no candidate vector, and its exact field computation is unchanged. This adds
no retained state, query cache or full-extent allocation. In the early frozen
Metro collection probe, the largest world-ID vector capacity rises from
0.762 MB to 2.884 MB; sides are collected separately. This is temporary vector
capacity, not a whole-process peak-memory measurement.

The isolated public dense-town regression was red at 82.460 M instructions for
Fog plus Learning and passes at 47.630 M. The separate-observer test proves both
visible bodies are learned, an unseen body stays unknown, the other side learns
nothing, public fresh sweeps agree bit for bit, and replay agrees every tick.
Omitting later observers' candidates makes that test fail on the missing body.
Remembered replacement/removal and aggregate revelation checks also pass.

A release six-tick Battle comparison on the same frozen full Metro geometry and
rules preserves every tick digest and complete serialized side-observation hash
for both initial early and late snapshots. Combined Fog plus Learning falls
from 371.236 M to 330.527 M instructions early, and 418.619 M to 355.871 M late:
about 6.8 M and 10.5 M fewer per tick respectively. Final digests are
`a98b13486b947938` and `b1ce437b780fa761`. These bounded initial views do not
certify dense-contact ticks or close the city-scale budget. All sight tests,
the remembered-body owner tests and library clippy pass; integrated contact,
whole-system memory and browser admission remain with the scale lane.
Independent read-only review found no correctness or shape issue.
