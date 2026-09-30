# SA2: counted route jobs in Battle

**Next architecture proposal; production selection pending.** This follows the accepted
long-order road preference and planning delay. [The current hierarchy cut](SA2-hierarchical-road-routing.md)
contains its physical proof and remaining limits. The original navigation oracle remains
comparison evidence; this proposal names different corridor priority, paths and commit ticks.

## Rule presented before production

A crew receiving a long move chooses a useful accessible road journey, rather than
searching every field cell before moving. The coarse graph ranks estimated journey time
including approaches; actual passage and movement still use existing physical footprints,
sub-cells, push classes, ground/material speeds, steering and collision. No teleport or
second speed multiplier is introduced. Foot movement uses its own mobility. Pursuit,
garrison and other automatic goals do not acquire automatic road preference.

For the first integration, a unit **holds translation while its activated leg is planning**,
including a routine replan. This is the simpler allowed hold behavior; it does not preserve
an old route while a different destination is pending. Existing packing, engagement and
weapon logic continue. A complete route commits before movement at a deterministic tick,
only for the still-current leg and compatible pose/knowledge. Queued legs wait for activation;
replacements and Stop cancel the old job. A failed corridor tries other candidates or the
general planner. Exhausting this tick's work means Pending, never NoRoute.

Road priority ranks coarse estimates, not the old fine-grid optimum. A selected path's
exact physical cost is recorded separately. A blocked directed graph edge is excluded for
this captured side snapshot and search advances to another candidate; it is not globally
closed for every mover or the other side. Quality/time versus the frozen planner must be
reported before this ranking becomes production behavior. No new detour tolerance is selected.

A truck that cannot cross a collapsed deck waits/plans another crossing; a tank may pay
its ordinary shove cost where a lighter vehicle cannot proceed. A hidden collapse must
not change the other side's captured belief. A new command supersedes old work immediately
on its applied tick. A road detour may improve arrival without increasing any vehicle's
physical cruise cap. Suppression/traffic/combat can delay arrival, so the representative
uncontested midpoint result is not an all-edge or every-vehicle promise.

## One owner and concrete seams

Battle owns one RoutePlanner containing one job per activated unit leg. Unit owns its
orders and frozen activated-leg intent; the planner owns mutable planning progress.
SideGeometry owns navigation beliefs. Prepared public terrain/road topology is immutable
and shared; it never includes secret side-specific bodies. A renderer chooses no route
or completion tick.

Proposed internal seam (names are provisional, contract is the point):

```rust
struct RouteRequest {
    unit: UnitId,
    leg: LegId,
    side: Side,
    snapshot: Arc<NavSnapshot>,
    start: Pose,
    goal: V2,
    mobility: Mobility,
    preference: ActivatedPreference,
    avoidance: Vec<Obb2>,
}
struct PreparedRoads { /* immutable indexed junctions, directed arcs, source identity */ }
struct NavSnapshot { id: NavSnapshotId /* immutable existing NavGrid cell/body semantics */ }
enum RouteProgress {
    Pending,
    Ready { leg: LegId, snapshot: NavSnapshotId, route: Vec<V2> },
    CorridorDeclined,
    Blocked(BlockedReason), // only a completed destination feasibility proof
    ResourceFailure,        // distinct from destination infeasibility
}
impl RoutePlanner {
    fn new(roads: Arc<PreparedRoads>) -> Self;
    fn submit(&mut self, request: RouteRequest);
    fn cancel(&mut self, unit: UnitId);
    fn advance(&mut self, budget: WorkCredits) -> Vec<(UnitId, RouteProgress)>;
    fn digest(&self, digest: &mut Digest);
}
```

Snapshots consume the existing side-known NavGrid semantics, not WorldGeometry's current
truth bodies. Their immutable content identity covers every physical input read by a job.
Only the current snapshot and ones referenced by active jobs survive; completed/canceled
jobs release theirs. Their creation/rebuild and temporary overlap are separate admitted
owners: making advance counted cannot hide a synchronous grid rebuild before it.
Road topology consumes the common prepared map's authoritative movement/playable bounds
and physical road primitives. Polygon road connectivity is an explicit missing source
contract; stroke-only support cannot silently qualify all generated maps or rendered margins.

## Counted stages and finite progress

PreparedRoads builds the indexed public junction topology once during loading, not once
per mover. Preparation exposes stage/cursors/cancellation and records actual candidates,
vertices/arcs, memory and elapsed startup cost. Overlapping diagonal road bounds remain a
real candidate-density risk; an index alone does not make them linear.

Active jobs persist cursors for endpoint-index traversal/projection, coarse frontier pops
and arc relaxations, selected-corridor physical refinement, blocked-edge exclusion and
commit checks. A stable unit-ID round robin spends a total budget and per-job quantum.
Every stage either consumes bounded credited work, finishes or changes its next cursor.
Do not sort an arbitrary candidate vector, copy a complete graph, scan all bodies or build
a whole clearance tile inside one credit. The existing resumable refiner is the physical
stage; graph operations and allocation/capacity growth still need their own falsification.

A declined arc is marked once per captured snapshot/mobility/avoidance context; retries
cannot rediscover the same unchanged invalid corridor forever. Revision invalidation
discards affected validity/cost labels before publication. Unrelated known changes should
not repeatedly restart a long job: first prove a conservative dependency/change-span test,
or retain only public topology work and revalidate affected physical work against the new
snapshot. The initial conservative all-revision restart is useful only as a small correctness
tracer, not a forward-progress acceptance for an active battle.

A final pose change repairs the local connector through counted physical work; it cannot
splice unchecked current position onto a stale route. Unit death, mobility change, new
traffic avoidance, a different leg or changed known geometry invalidates incompatible work.
Actual movement still handles live traffic and discovers true obstacles through its existing
contact owner. Planning does not learn hidden bodies just to avoid them.

## Battle, observation and replay contract

Today Battle accepts a command for the next tick, applies it before movement, then
movement::plan_if_needed can synchronously plan every requesting unit. Unit digest covers
orders/routes but no route job. The integration replaces that planning owner for the named
eligible ordered-leg scope; it must not call the old synchronous planner after refinement
as the external transit tracer still does.

On the applied tick, create/activate or cancel jobs before movement; advance counted work,
validate Ready results and commit before the existing follower. Freeze automatic preference
once per activated queued leg. Failed candidates continue planning; completed destination
infeasibility may use the existing RouteBlocked state. Propose explicit public MoveState::Planning
for accepted pending motion, rather than overloading traffic Waiting. This is an observation
schema/client contract change and needs its own consumer/UI review before implementation.
The command envelope/ack can stay unchanged: applied_tick means order applied, not route ready.

Digest every future-affecting job input and cursor, including scheduler cursor, captured
snapshot identity, partial physical sums/transform state, frontier labels/parents, excluded
arcs and frozen intent. Exclude wall time and memory addresses. Whole-graph/full-scratch digest
scans per tick are another unbounded-work owner: the small proof may use a canonical reference
fold, then an incremental semantic-state digest must be proved against it before scale
admission. Replay keeps recorded command applied ticks and derives planning completion from
counted deterministic work; same-build Native/Wasm must agree on complete tick traces.

## Red owners and next tests

1. **Immutable topology/query/search:** two concurrent legs share one prepared topology,
   but have independent side/mover labels. A one-credit poll cannot perform full endpoint
   discovery or a complete search. Connected/disconnected junctions and the corrected
   destination direction remain observable route contracts. Preflight actual geography counts.
2. **Battle pending authority:** accept through Battle, observe Planning, move only after
   deterministic completion, cancel/supersede on applied ticks, and activate queued legs in
   order. Replay every tick including mid-job digests. Test a known change for one side,
   a hidden change for the other, and a moved start before commit. No test-only route injector.
3. **General-region fallback:** prove a finite resumable region/portal search with bounded
   local physical refinements. Uniform interiors can be analytic; detailed regions preserve
   real crossings. A coarse blocked cell cannot falsely prove destination infeasibility.
   Bent rivers, alternative bridges, forest/infantry gaps and disconnected roads must exercise
   fallback. The present straight separator tracer is insufficient; a whole-world old A*
   fallback would restore the measured memory/work pathology and is not this architecture.
4. **Measured combined admission:** count topology loading plus snapshots/rebuild overlap,
   all concurrent jobs and per-tick digest work. Then the actual integrated light-vehicle
   midpoint arrival and complete Native/Wasm replay at release extents, followed by admitted
   20 km content/active-unit loads plus bounded rendered surroundings. Preserve the separate
   physical, playable and rendered extents in every receipt.

No full-size arm or production numeric rule is selected by this proposal. The first
implementation cut is the standalone immutable topology/counting core; the Battle state
seam follows its small proof, and general fallback remains a named admission blocker.
