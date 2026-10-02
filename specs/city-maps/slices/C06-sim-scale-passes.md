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
On the frozen [initial contact workload](C05-measuring-tools.md#generated-contact-baseline),
the pass retires 369.9 / 499.4 G step instructions early/late versus 369.5 /
498.2 G before; movement averages 80.343 / 115.399 M instructions per tick,
effectively unchanged. Digests remain `32ba0e3c667a728b` / `1afb9cf97c2120f3`.
Movement, bounded rejoin, geometry parity and authority/replay checks pass.
This is termination proof, not a measured speedup or current timing admission.

## Outcome — exact per-tick bounds; cover still red

Hull traffic now visits only living, standing squads in original unit order.
Its current footprint bounds refresh after every relevant movement, so later
vehicles see the same geometry. Sensing derives target bounds once while its
unit slice is immutable. The regression measures fallen-squad amplification in
an isolated process: remains may add gather work, but more vehicles must not
multiply that work. Restoring the old traffic scan makes the regression fail.

The same frozen 60 s contact controls retire 356.5 / 457.5 G step instructions
early/late, versus 369.5 / 498.4 G before this pass. Movement drops to
77.863 / 96.912 M instructions per tick and sight to 23.941 / 25.088 M.
Both exact digests above remain unchanged. Fine diagnostic brackets are removed
after identifying cover planning; the production report retains its system
contract. No mechanic or scheduling delay was introduced. Loaded clocks are
not timing admission.

## Outcome — impossible engagement searches

The cover probe found 146 resolutions in the first two seconds against targets
beyond weapon, holding-area and lean reach. Their failed step-out searches alone
retired 18.7 G instructions. A conservative necessary range condition now avoids
that proven empty search. Ordinary near-target candidates and boundary cases keep
the original exact search. The negative public-battle cost test and positive
edge-lean test both have red/green proofs; existing cover/replay tests stay green.
The old public battle raises peak movement cost from 405,912 to 85,232,244
instructions for a distant visible enemy; the range rejection removes that
amplification. Dropping the lean allowance fails the positive boundary test.

| Matched frozen 60 s contact cost | Before | After |
|---|---:|---:|
| Early step, G instructions | 356.5 | 278.9 |
| Late step, G instructions | 457.5 | 348.7 |
| Early movement, mean M/tick | 77.863 | 34.203 |
| Late movement, mean M/tick | 96.912 | 36.557 |

Digests remain `32ba0e3c667a728b` / `1afb9cf97c2120f3`; rounds remain 149 / 124.
Cover and authority/replay checks pass. These instruction gains do not establish
current native or browser timing admission.

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
advances 1,661 (27.7 Hz). Neither met the original 30 Hz target; both meet the
later provisional floor on these frozen inputs. Worst sampled frame
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

## Outcome — shared final-connector component

After a failed final road connector, navigation searches outward from the same
resolved destination with the ordinary symmetric mover graph. Only an exhausted
frontier is retained. Its membership rejects later final connectors using their
actual constructed source and the original 4 m source admission; it also answers
the direct fallback. It never rejects a whole road journey merely because the
original start is outside that strict component: a sampled-clear road may enter
it across a corner. A missing canonical goal endpoint remains the refusal shared
by every mandatory final connector. Search limits, start refusal and terrain
shortcuts leave the proof inconclusive and preserve ordinary alternatives.

Road access admission uses a conservative envelope before constructing physical
approaches. Every final centre slides along its goal arc, then takes one lane
offset. The whole arc's bounds expanded by that offset, source-snap reach and
point-coalescing tolerance must be disjoint from the exhausted component's
bounds before an access is omitted. Overlap retains the ordinary reader and
exact source test. Excluded arcs cannot set the fallback's nearest-access radius;
otherwise they could hide a valid farther approach. Bounds and a first-visit
membership fingerprint are updated
in the already-counted rare search; pending and retained state enter the digest,
without a visited-map scan on each tick. Healthy journeys perform no extra search.

The initial twelve-exit thin-courtyard probe fails at 198,554 expansions without
shared connector reachability. With only a late membership check, the 128-exit
public regression still spends 304,034 counted work rebuilding approaches and graphs. Early access
admission satisfies one map search plus flat access passes, keeping the same
20,000-cell and 300,000-work allowances. Sampled corner crossings and a distant
legal exit after `SearchLimit` remain successful. The small actual Battle clears
its planning job, holds behind the walls and reproduces every replay tick.

Frozen layout-6/catalogue-541668 inputs remain historical controls. The previous
relaxed whole-journey proof resolved Mixed Small seed 3 and Metro Medium seed 9,
but Mixed Small seed 9 was still planning after 120 seconds. Its relaxed probe
correctly reached an exterior connector; repeated graph and approach work also
survived late strict membership alone. The final public Journey now proves that
case blocked in 51,293 counted work, including a five-cell goal component.
The dependency-matched native Battle rerun plays all three exact frozen scenarios
and commands for 120 seconds. Each affected unit remains alive and blocked with
no pending work at both 30 and 120 seconds. Seed 9 records 82 planning ticks rather
than all 3,600, and 390,885 total force planning work rather than 14,400,000. Its
final digest is `bbde4d96aef7eeb3`; same-build replay is covered by the small Battle
regression. Current-generator and native/Wasm admission remain integration checks;
this scoped correction does not close C06 or the navigation budget.

The rare component has fresh sparse scratch, while ordinary searches reuse their
existing bank. In that seed-9 query the retained component uses one 20,480-byte
tile beside 67 ordinary tiles (1,372,160 bytes): 1,392,640 bytes of tile payload,
plus containers. A large inconclusive probe can briefly coexist with the normal
bank; both retain the existing search bound. No world-sized grid, limit tuning,
physical exemption or snapshot-memory admission is introduced. Affected planning
ticks and digests change intentionally; same-build replay remains exact.

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

The current-main restart remounts the lab and temporarily removes its probe.
The reset wait now treats that interval as pending. The original current-layout-7
run throws while reading the missing probe; the corrected two-second functional
run completes all three resets with identical counts and bytes. This repairs
the harness wait, with no simulation, rendering or timeout change.

## Outcome — reject footprint rotation reuse as the next cost pass

The current native compiler already hoists `footprint_seen`'s sine/cosine pair
outside the inner sample loop, but recomputes it for each footprint row. Reusing
the existing `Rotation` once per footprint preserves all sample arithmetic;
the matched current layout-7 Metro Large seed-4 control confirms every six-tick
battle digest and both complete observation hashes unchanged, early and late.
Whole Fog plus Learning falls from 312.077 to 309.597 million instructions early
and 341.756 to 339.329 million late: 0.795% and 0.710%, about 0.41 million per tick.

The scratch comparison first rejects unchanged old-versus-old code, then rejects
both candidate arms against the declared 2% gain hypothesis. This is a bounded
initial-view experiment, not a whole-battle or browser timing claim. The source
change is discarded; existing public cost requirements stay unchanged. Remaining
Fog plus Learning work remains the next attribution target, with current full
report means of 54.941 and 59.969 million instructions per tick early and late.

## Outcome — changed edges during infantry refinement

Incremental infantry refinement can outlive the knowledge revision on which its
coarse path was admitted. A new body can close a diagonal's shared edge before
refinement reads it. Missing edges now reject that candidate through the existing
bounded route-failure path; the planner's revision check owns the fresh search.
No stale crossing is emitted, and unchanged geometry keeps the same path.

The public Battle regression adds a body during planning. It reproduces the
first-edge panic on the original source, then proves eventual publication of a
route fitting the actual changed world, no pending job, and every serialized
replay tick digest. Its deliberately tiny planning allowance stretches the
revision transition; the watchdog is a completion check, not latency admission.
A private phase probe confirms finite smoothing progress. The preserved current
catalogue late admission crash selects this fix, but the full late run and scale
budgets remain integration gates.

## Outcome — current-main movement admission changes the measured load

The combined build `c1fb0f09…` includes main's physical move certification and
world ownership changes. Native and Wasm generation receipts match: Metro Large
seed 4 retains map `7ea9ba8a…`, catalogue `6b0a5e8b…`, and 12,887 buildings,
22,442 parts, 228,030 bays and 46,813 ground points. Rules are now `b7e67721…`.
The earlier complete reports use different rules and remain historical controls.

A bounded unmodified 150-tick early arm spends 1,179.833 million instructions per
tick on average in Orders, with a maximum of 113,197.618 million at a scripted
tick. Both scripted ticks are included; this is not ordinary per-tick cost.
All 16 supply moves have no new goal at tick 1. At tick 150, blue's first 74-unit
fighter wave yields eight new goals and four active routes; red's yields none.
Tick 149 has no movement goals on either side, maximum displacement of 9.23 m
and 8.65 m, and one launched round. Tick 150 ends with 200 living units, one
round, and digest `5b586cfb01b49e34`. These public observations do not expose
individual hidden preparation verdicts. Some short requested journeys also fail,
so the validation allowance's travel bound alone is not a sufficient diagnosis.
The city recipe shifts living starts toward contact by 850 m; conclusions from
the unshifted rear-edge recipe were discarded.

Fresh Open Medium seeds 5 and 8 and Metro Small seed 10 retain their exact map
identities, but all three formerly accepted nine-unit moves are now refused with
`no_valid_destination` and zero placed destinations. The ten historical pending
infantry stay living and idle through 120 s, with null goals and zero pending
work. Their final digests are `b5720017b15f56de`, `520de95a7c07cef3` and
`80285fa00696bac4`. This proves refusal, not resolution of their earlier planning
delays. Initial command-acceptance cost is outside those tick counters.

Fresh full contact and browser admission wait for this movement contract to be
resolved. No validation-budget increase, script shortening, destination bypass
or performance parity claim is made. Raw receipts and traces stay in ignored
`throwaway/scale-lane/` and the publication worktree's corresponding directory.

A paired scratch-only nested profile attributes that cost without changing the
rules, destinations, scripts or validation allowance. Its distinct diagnostic
build `3c8934a0…` produces the same terminal digest, every recorded goal/progress
row, and byte-identical complete blue/red observations as the unmodified
`c1fb0f09…` control. The instrumentation is removed after the run.

| Scripted tick | Certifier calls / attempts | Virtual movement ticks | Nested total instructions G | Physical advance G | Navigation G | Snapshot and attempt setup G |
|---|---:|---:|---:|---:|---:|---:|
| 1 | 16 / 16 | 24,633 | 62.851 | 44.433 | 8.109 | 5.558 |
| 150 | 20 / 24 | 20,386 | 111.598 | 90.488 | 8.806 | 7.885 |

All supply slots and all 148 first-wave slots have placement points. Every
supply attempt exhausts the roster-step allowance. The first-wave attempts end
with 16 roster-budget exhaustions, seven planning-work exhaustions and one
completed cohort; eight units are placed. These traced causes explain the
absent goals without equating all failures to distance or formation placement.
The allowance is per command, so one first wave can spend 20 × 250,000 abstract
work units. Each unit charges living-unit steps and planner work, while physical
advance includes collision, cover and soldier movement. Physical advance owns
70.7% and 81.1% of the two nested totals. Coarse counter reads, about six per
virtual step, are included; these are attribution measurements, not a measured
speedup. No demonstrated small allocation cleanup resolves that amplification.

The unchanged control also runs through 900 ticks to test whether the five-second
probe ended before weapon setup. At ticks 150/300/600/900 it has 1/2/3/3 launched
rounds, 200/200/199/199 living units, no soldier corpses and 0/0/1/1 wrecks. Blue
ends with six movement goals and routes, maximum displacement 150.13 m; red has
no movement goals throughout and moves at most 9.36 m. The final digest is
`d167f5f4d7bc46ce`. Orders spend 176.662 billion instructions across the arm;
ordinary phases average 101.437 million per tick. Complete final observations
and the matching input identities are retained with the 900-tick receipts.
Some combat occurs, but the intended broadly moving contact load is not
established. Faster execution with identical refusals cannot repair that workload
gap. Current admission needs a movement-admission resolution or an explicitly
accepted representative workload; a quiet five-minute run cannot close it.

## Outcome — idle squads need no local steering geometry

A moving vehicle makes the threat list nonempty for every squad, even far from
its path. A squad with no active corridor, no living soldier needing motion to
his holding post and no vehicle dodge cannot produce steering. It now skips
local obstacle gathering and the no-steer member loop. Holding-post eligibility
is shared with the ordinary steering reader; nearby dodges, cover routes and
corridor motion keep the same owner. Velocity reset, garrison handling and the
old empty-threat return remain in their original order. The nonempty-threat
shortcut retains the ordinary no-corridor centroid settlement.

The paired native control uses source `b5c63565`, the frozen current physical
Metro Large seed-4 scenario with benchmark fronts at 1,050 m, and 900 ticks.
Whole Orders falls from 61.047 to 45.356 G instructions (25.704%). All stepping,
including Orders, falls from 206.305 to 190.810 G; construction costs 16.137 /
16.185 G. Combined stepping plus construction falls from 222.442 to 206.995 G
(6.944%), so work was not merely moved outside Orders. Every tick digest and both
complete side observation hashes match; both arms launch 8,835 rounds. The
scratch comparator first rejects unchanged old-versus-old against a declared
5% whole-Orders improvement, then accepts this candidate.

A public battle keeps a far idle squad's exact positions, zero velocities and
living-member centroid while a nearby squad yields before physical hull contact.
Settled holding posts isolate this from cover motion; omitting the dodge check
fails the predictive-yield assertion. Restored code passes all 18 movement tests,
focused clippy and independent review.
The proof concerns one coherent steering cost owner, not movement admission:
scripted move destinations remain refused on this paired input. Full current
late delivery, real-time throughput and incoming movement-contract findings
remain separate integration work. No cache, retained state, physical rule or
validation allowance changes.


## Outcome — omit unread cover exposure rows

Movement's cover field keeps its unit-indexed outer vector, live hulls and
blockers. Its sole soldier-position reader first requires a side's enemy track.
The two knowledge references remain immutable during that field's lifetime, so
an ID tracked by neither side has an empty inner vector. Tracked units keep every
member in original order, including dead members and grace-retained sightings;
current visibility and living status are not substitutes for this read contract.
No retained cache, rule, placement solver or admission allowance changes.

The frozen late Metro Large seed-4 comparison uses the named 1050 m front staging,
100 living units per side and the original 20,000 fallen soldiers / 2,000 wrecks.
Both arms load the same serialized scenario, effective identity `df34e7db…`,
map-with-remains `6c6022f6…` and rules `b7e67721…`. Its factory receipt before JSON
reload is `41126341…`; those identities are not claimed interchangeable. The
unmodified engine is `c1fb0f09…`; the measured field candidate is `75aec037…`.
Final comment cleanup changes no executable rule.

| Counted work over 900 ticks | Control G instructions | Candidate G instructions | Gain |
|---|---:|---:|---:|
| Whole Orders | 123.605 | 106.684 | 13.69% |
| Whole steps | 400.288 | 383.345 | 4.23% |
| Construction plus whole steps | 418.423 | 401.481 | 4.05% |

The declared 5% whole-Orders gate passes; the whole-step gains remain below 5%.
Ordinary Movement means fall from 75.519 to 74.825 million instructions, including
field preparation. The construction and tick counters include report counter
reads; input parsing, complete-observation hashing and output are outside them.
The scope is this 30-second native arm, not browser or full scale admission.

Every tick's digest and complete blue/red observation hashes match, and both
full final observation byte strings match. Both arms end at digest
`d52cf36268e41eb6`, 19,579 launched rounds, 73 blue and 68 red distinct units with
observed shot counters, 20,301 corpses and 2,011 wrecks. This is active combat,
while move refusals and the remaining order spikes still need their own decision.
The avoided born-fallen rows account for 20,000 two-coordinate values (320,000
payload bytes) and 2,500 inner allocations per gather; this is a structural
allocation count, not a measured process peak. The outer slots remain allocated.

The old-versus-itself cost assertion is red at zero gain. Existing public cover,
last-seen, hidden-enemy and same-build replay proofs pass. Omitting tracked rows
as well falsifies the public reach/cover test: a soldier stays tucked when the
enemy is reachable; restoring the tracked rows restores green. These owners
already cover the behavior, so no private field-layout test or duplicate model
was added. Raw commands, identities, per-tick proofs and counters stay in ignored
publication-worktree `throwaway/scale-lane/field-*` artifacts.

## Outcome — portable seeded placement

The first differing authoritative field is one soldier's Y coordinate, one bit
apart, followed by its squad centroid. All world-body poses and cold float32
publication words agree. The mismatch reproduces with one rifle squad on an
empty map. Tracing its unchanged draw sequence isolates platform sine before
scaling and mean-centering: the pinned software sine yields the Wasm position.
Squad draws and nearest-free ring samples now use that existing pinned library,
with no change to random draws, placement policy or physical rules. Native
last-bit positions intentionally change; this is a named portability repair,
not a CPU speedup. No temporary diagnostic export ships.

The existing shared publication-record tests include initial authoritative
state, with no compatibility branch. The new one-squad record fails on the
unmodified native/Wasm pair before float32 packing. With pinned math, initial
state and all six subsequent publication/digest rows agree; existing record rows
are unchanged and gain initial digest checks. Focused public placement and
formation/replay checks pass. Full generated-map parity remains the next gate;
no long performance run is claimed from this small regression.
