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

## Finding — full-world browser stress

The frozen `layout-6` physical Metro Large seed-4 map is loaded in the ordinary
browser worker and production view: 10 × 10 km, 13,006 buildings, 22,577 parts,
100 living units per side. Its generated identity is `33bbd0d9…`, catalogue
`5416684f…`. The 60 s early arm advances 1,787 ticks (29.8 Hz); the late arm
advances 1,661 (27.7 Hz). Neither admits the required 30 Hz. Worst sampled frame
p95/p99 are 58.7/91.7 ms early and 67.4/75.1 ms late. Late whole-page memory,
including the worker, is 1,173 MiB; final main-thread heaps are 581/931 MiB.
These measurements include immutable static decoder/feed reuse, before the
fog candidate union. They are neither an instruction comparison nor current
`layout-7` admission.

Reset resource counts differ: 363 buffers before, 362 after; textures remain
41. The buffer-byte difference is 1,491,008. This is a red contract requiring
allocation attribution and controlled rendered history, not a reason to relax
the assertion. Raw telemetry, preparation and captures are preserved under
`throwaway/scale-lane/browser-full-generated-checkpoint/`.

All four full-world states and their crops have visible content (full-frame
entropy 5.26–5.32 bits, edge density 0.73–0.74, no transparency), but an unprimed
review finds bright sight boundaries obscuring dense city blocks and weak unit
readability at strategic zoom. Other saved movement/benchmark captures expose
heavy fog hatching, weak route contrast, unclear water, and benchmark link/
results layout issues. These are renderer/UI lane findings; the scale lane has
not changed appearance or accepted visual fidelity. The final stress check needs
readable unit/contact framing alongside the full-world loading proof.

## Finding — exact fog queries and late corpse sorting

The bounded frozen-Metro late snapshot spends 104.142 M instructions in six
Observation phases. Replaying the existing position comparator over its public
corpse rows, restored to source soldier order, costs about 9.47 M per observation
and reproduces the exact published list. This is a substantial recurring sort;
skipping it would require death, fighting-floor loss and side-knowledge mutation
ownership. A second corpse projection is not the next pass. An unstable coordinate sort
would need the original construction ordinal for exact ties: reinforcement
appends a new high soldier ID inside an existing unit, so soldier IDs cannot
stand in for global unit/member iteration order.

A short native stack sample identifies out-of-line world height and foliage
queries in the unchanged fog traversal. The height field already forces its
triangle reader inline to prevent new callers from increasing sweep cost, but
the world wrapper does not. Measure each query owner's inlining separately,
including whole Fog plus Learning and emitted code size. Keep only a clear
instruction reduction with unchanged complete observations and battle digests;
this introduces no cache and changes no arithmetic.


## Outcome — expose the existing foliage query

Only the foliage owner's existing query is forced inline. Height-only inlining
was measured separately and left unchanged: it saves 3.15% early and 2.05% late
Fog plus Learning work while growing the linked text by 7,580 bytes. Foliage-only
inlining saves more and shrinks text; both candidates keep all arithmetic intact.

Against the fog-union checkpoint on the frozen full Metro input and rules,
six-tick Fog plus Learning falls from 330.527 M to 317.109 M instructions early
and 355.871 M to 345.378 M late: 4.06% and 2.95%, about 2.24 M and 1.75 M fewer
per tick. Every tick digest and complete serialized side-observation hash stays
identical; the final digests remain `a98b13486b947938` and `b1ce437b780fa761`.
The same linked native tracer's text shrinks from 2,562,712 to 2,559,388 bytes.
This sizes one linked consumer, not every target. No retained allocation, cache,
visibility-kernel fork or physical rule is added. The existing public collection
budget remains intact; a scratch tighter bar did not become a new contract.
All sight tests and focused clearing, canopy/bucket-edge depth and side-known
clearing checks pass, as do library clippy and independent read-only review.
These are bounded initial-view measurements; contact peaks and browser admission
remain with the scale lane.

## Outcome — enclosed destination component proof

A failed final road connector can now prove the whole destination inaccessible,
rather than recertifying it from every nearby road. Navigation searches backward
from the same effective destination cell, using the weakest existing mover fit,
push and avoidance checks. Its certificate-only graph deliberately admits more
links than a real route: sampled roads may cross cell corners and their next
search may snap from an unchecked endpoint. A fixed local stencil covers both;
it emits no route and never weakens the ordinary validator. Exhausting that
entire relaxed component
and finding the original start's resolved cell outside it proves `NoRoute`.
The original goal having no admitted endpoint is the same terminal refusal every
mandatory final connector already makes. Other start refusals, terrain shortcuts
and `SearchLimit` do not prove component exhaustion.
Reaching the attempted exit or failing to finish the proof retains the ordinary
alternate-road and direct-search behavior. The proof runs once, only after a
final connector fails; healthy road journeys incur no added search.

The frozen layout-6, physical-catalogue-541668 Mixed Small seed-3 failure confirms
the cause: its reverse destination query exhausts 36 cells, while forward
connectors repeatedly search tens of thousands. This is historical input evidence,
not admission of the incoming layout-7 generator. The complete corrected journey
finishes with proven obstruction at 52,687 counted work, after the first failed
connector. A small public twelve-exit
courtyard regression fails at 192,136 expansions without the correction and now
fits one search of its 20,000-cell map plus local proof work. A low-limit reverse
query remains `SearchLimit`, and the complete journey still takes a distant legal
road opening. Treating that limit as component exhaustion falsifies the positive
test. A sampled diagonal crossing falsifies using the ordinary no-corner graph
as a global road proof; the relaxed certificate retains its legal road route.
An actual Battle holds while planning, finishes blocked within the same
derived allowance, clears its pending job and matches every serialized-replay tick.

This intentionally changes affected planning completion ticks and may replace a
later inconclusive verdict with an earlier proven obstruction. Its pending stage
and the flag preventing repeated proofs enter the digest only when used. The
existing sparse search scratch is reused; no world-sized storage, limit tuning or
physical exemption is added. Large connected destination components and other
repeated connector failures retain their existing costs. Current generated-case
follow-up, native/Wasm agreement and full browser admission remain integration
work; this scoped correction does not close C06 or the whole navigation budget.

### Integrated frozen-case rerun

On current native build `0666de00…`, the original frozen scenarios and commands
run for 120 simulated seconds without changing their map or rules identities.
Mixed Small seed 3 first leaves planning at tick 31; Metro Medium seed 9 at tick
29. Both finish alive with explicit blocked routes, no pending job or planning
work. Their battle digests are `c7296593532e935c` and `bbf7d86052ff3f05`.
This proves termination, not reachable encounter placement.

Mixed Small seed 9 remains planning for all 3,600 ticks, using 14.4 M counted
planning work and ending at `e0901a622c524a92`. Its relaxed reverse graph reaches
the failed connector outside a thin enclosure, so the global proof correctly
declines. The next seam is the actual final off-road connector: an exhausted
strict destination component may reject that connector's exact resolved start,
while sampled road runs retain their existing links. Resource exhaustion must
remain inconclusive. Fresh layout-7 pipeline and timing admission stay open.

## Outcome — controlled reset resource baseline

The full-world reset failure compared a late battle with a fresh opening.
Factory-level allocation traces identify the extra late buffer as the translucent
overlay's replaceable mesh (`overlayPass` → `MeshSlot.set`). An empty opening
does not materialize that lazy buffer; replacing the late mesh destroys it.
In the attributed run, its 1,422,960 bytes disappear on reset and corpse cards
shrink from 320 to 16 bytes. Three fresh resets have identical counts and bytes.
This is a valid resource-lifetime transition, not a missing disposal.

The endurance scene now compares all three resets at paused, presented tick 90,
after the live arm warms lazy resources. It checks exact buffer/texture counts
and bytes and records them in telemetry. The bounded generated browser proof
passes with 362 buffers / 41 textures and 294,501,276 / 263,868,125 bytes on
every reset. A separate scratch wrapper deliberately retains a real 16-byte
GPU buffer on each explicit draw; the check fails with 366 → 367 → 368 buffers
and precisely 16 added bytes per reset. The ordinary source has no leak hook.
This corrects an uncontrolled comparison and adds byte accounting; stress
durations, rendering and the timing target remain unchanged. The two-second
arms prove the reset contract only. Full timing admission remains open.
