# SA2: road journeys with physical local refinement

**Kind:** named behavior architecture proof; production architecture unselected.
The current [scale direction](../scale-direction.md) owns accepted long-move road
preference and planning delay. Frozen exact-navigation oracles remain comparison evidence.
The current architecture envelope is 20 × 20 km playable plus bounded rendered surroundings;
release presets are 6/8/10 km. The earlier 100 km proof inputs remain historical receipts,
not current map admission.

## Physical rule and moments

A sufficiently long ordered move leg uses an accessible useful road corridor for the
journey, joined by legal local approaches. The mover keeps its ordinary speeds, footprint
and push class. A truck takes the bridge instead of searching every open field cell;
a soldier still fits a gap a tank cannot. A road through water without a deck is unusable.
A wreck the tank can push adds the ordinary shove cost; a heavier wreck closes passage.
A known blockage forces stop/refinement or a different crossing. Unseen destruction leaves
the other side's old belief intact until its existing learning/contact rules change it.
These apply to both sides and all vehicle stopping ranks; roads grant no new passage ability.

Short ordered legs retain manual fast move. A trial configurable threshold uses straight-line distance at activated leg start;
5,000 m is an experimental value, not a user requirement. A cheap coarse journey-time
comparison should include road access, destination connectors and detours. It must not
run another whole-world fine search simply to select a mode. Retain the preference across
routine replans. Queued legs evaluate when activated. Pursuit, targeting and garrison
approach are excluded from automatic preference. A replaced leg supersedes its planning
job. No repeated wait is terminal: jobs advance, are canceled, finish, or report a named
failure; local traffic retains the existing stall/avoidance recovery while its corridor
remains safe.

## Candidate ownership and seam

One navigation owner takes side-known revision, ordered leg identity, endpoints, footprint,
push class and route policy. It produces Pending or a complete validated route/physical
blocked verdict; proof decline and resource failure are distinct from NoRoute. No renderer
or wall-clock worker chooses its activation tick.

The candidate has three layers, each independently falsifiable:

- **Road graph:** deterministic authored stroke segments split at junctions/intersections
  and real crossing transitions. Endpoint access can project onto a segment, rather than
  requiring a nearby authored vertex. Edges represent physical passage, not decorative
  connectivity. Road polygons from the surface owner need an explicit connectivity source;
  this prototype does not infer every polygon junction from a bounding rectangle.
- **Local refinement:** existing footprint, sub-cell crossing, material and shove rules
  validate start/end connectors and the selected road path. The graph may choose different
  raw parents, priority and waypoints than the old fine-grid winner. Fine search is local
  to meaningful obstructions; it cannot secretly become a whole-world fallback.
- **General regions:** certified uniform interiors become macro corridors, with detailed
  boundary/passage refinement where terrain or known bodies differ. A separator/opening
  tracer establishes only its narrow straight-passage family. Bent rivers, winding gaps,
  forests/slopes and arbitrary disconnected geography remain separate red domains.

Public authored road topology may be shared immutably; mutable side beliefs, blockage
labels, pending jobs and orders remain separate. Clearance is keyed by mover class/push/
footprint and known revision; one vehicle's traversable edge is not evidence for another.
Rebuild, old/new overlap and graph/path residency count against memory admission.

## Numeric choices to test, not selected rules

Existing representative vehicle speeds already give a 2× road/off-road ratio; do not
apply an automatic second multiplier. Foot movement has its own speed data and must
be measured separately. Absolute arrival time and changed battle outcomes need evidence.

Use a provisional 256 m road-access search envelope in the small prototype, including
starts near the middle of long segments. Measure reachability and route quality while
varying access distance; generator access coverage is a separate corpus. Nearby geometry
alone is insufficient if its component cannot serve the destination. Compare corridor
travel time against a legal off-road alternative when available; record distance/time
stretch against the old oracle. A usefulness/detour threshold needs physical and route
quality evidence before a production numeric rule is chosen. A failed connector continues
bounded general-region planning instead of rejecting the goal.

## Deterministic delayed planning proof

Prototype a counted scheduler with at most one active job per moving unit/activated leg.
Stable round-robin ordering gives every admitted job progress; supersession replaces its
job and does not accumulate historical requests. Jobs capture side-known revision and
explicit stage/cursors: graph query, connector refinement, passage validation and commit.
A known change invalidates affected work before publication. Continue an existing route
only while it is safe under current side beliefs; otherwise hold. Commit only a complete
validated route on a deterministic tick. Pending stage, frozen leg preference, job identity,
progress and cancellation enter replay/digest authority.

A counted unit must include its hidden work: heap/table operations, cell/clearance tile
construction, primitive-query candidates and refinement samples. A node count alone does
not bound a node whose collision query scans every body. Preparation can use the loading
allowance; active work must fit the independent tick budget. Round-robin proof must cover
many simultaneous requests, revision churn, canceled head jobs, supersession and completion,
not only one route. No wall-time deadline, unlimited queue or budget-triggered NoRoute is
selected by this design.

## First concrete proof and admission

Keep actual sources/build identities and paired old/new route quality, even where routes
intentionally differ. Start with independent small graph/workload tests, then native/wasm
bit parity and same-build replay. Challenge below/at/above each trial mode threshold; connected and
crossing junctions; mid-segment access; disconnected/dead-end roads; inaccessible nearby
roads; no-deck river; competing bridges; pushable/impassable wrecks; infantry-only gaps;
both sides' hidden changes; queued/replaced legs and deterministic scheduler fairness.

The preceding measured 4 km fine-grid bridge is red at the unchanged active work budget.
It supplies the scale pathology receipt, not a universal route-equality requirement for
this named alternative. Prototype functionality at 256 m does not admit a 20 km fine-grid
construction. Preflight graph/content/local-detail counts, temporary overlap, native/wasm
indices and output size before larger arms. Serialize admitted measurements with Root.
A successful sparse road arm is not city-density qualification. Failures retain their
actual domain and motivate the next smallest cut; full SA2/G0 remains open.

## Executed first tracer

[The first road/corridor receipt](../assets/navigation-proof/road-corridor-first/README.md)
contains junction/overlap/directed-cost red/green tests, small physical refinement and
core-only native/wasm records. Its counted parallel-road arm confirms irrelevant quadratic preparation; node deduplication
also scans existing graph vertices. Its original pairwise preparation and unscheduled fine refinement
are the rejected owners addressed by the indexed refinement cut below;
the first receipt remains unchanged and selects no production routing.


## Indexed refinement cut

[The indexed/refinement receipt](../assets/navigation-proof/indexed-refinement/README.md)
banks a deterministic junction/access BVH and resumable existing physical evaluator.
Parallel-road preparation no longer tests disjoint pairs; overlapping content remains
unbounded. Counted cancellation, supersession and side-revision invalidation replay, with
37 complete Native/Wasm physical/scheduler records identical. One approved proof-only Jeep
midpoint transit arrives in 166.3 simulated seconds including planning, under the 110 km/h
road cap. This selects no production tuning for other units or geography.

The next integration cut must own immutable prepared topology, counted graph/search stages,
side-known revision and traffic/mobility/start-pose invalidation, and authoritative Battle
pending state/digest. Existing-route continuation and moving-start commit require physical
validation. Polygon topology and bounded general-region fallback remain red. Do not turn
prototype PhysicalDecline or budget exhaustion into a terminal destination NoRoute.


The [directed destination correction](../assets/navigation-proof/directed-goal-connector/README.md)
resolves Root's independent connector-direction finding. A destination is admitted by
road-to-goal travel, not its reverse; frozen first records remain unchanged. Other reviewed
index/refinement/scheduler owners are clean within their declared proof scope.
