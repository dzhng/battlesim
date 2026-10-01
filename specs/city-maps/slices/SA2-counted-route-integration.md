# SA2: counted route jobs in Battle

**Next architecture proposal; production selection pending.** This follows the accepted
long-order road preference and planning delay. [The current hierarchy cut](SA2-hierarchical-road-routing.md)
contains its physical proof and remaining limits. The original navigation oracle remains
comparison evidence; this proposal names different corridor priority, paths and commit ticks.

## Rule presented before production

An ordered leg strictly over 2,000 m straight-line distance at activation infers road
preference; exactly 2,000 m does not qualify. Freeze this choice across replans, and evaluate
queued legs when they activate. Battle integration must own the 2,000 m value in its rule
data/config identity; it is not a prototype-only CLI threshold. Access radius and usefulness
remain measured provisional choices.

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

## Stopping point and next cleanup owner

The user requested a clean, committed stopping point before further feature work. The
[standalone core receipt](../assets/navigation-proof/counted-core/README.md) freezes the
first cut's source, accepted 2 km red/green and complete Native/Wasm records. Production
Battle integration, general fallback and polygon-road connectivity have not begun.

The core counts failed-search queue/path draining, but ordinary request destruction can
still visit every label, queue entry, splice, connector, exclusion and output point in one
call. A Ready request retains that storage. Releasing its last shared geometry lease can
also destroy all snapshot cells or topology incidence vectors. Counting search while
leaving cancellation or output consumption synchronous would move the freeze to another
tick. No new cleanup mechanism is implemented in this proof.

Before Battle integration, the next owner must make retirement resumable under the same
active-work budget. Cancel immediately invalidates the old leg's authority to commit,
then transfers retained state to a retirement cursor. Drain bounded entries/pages per
credit; never use an unbounded clear, collection drop or replacement as one tick atom.
Output copying, follower adoption and output destruction require their own accounted
ownership transfer. Avoid last-job bulk destruction of shared data: a registry can anchor
snapshot/topology leases until a separately admitted retirement cursor disposes them.
Loading cancellation/disposal also contributes to the startup/cancel budget.

Keep at most one retiring payload per unit plus a coalesced newest request while cleanup
is pending. A stream of replacements must not retain every abandoned search. This is a
proposed resource policy, not selected production behavior: prove that it preserves the
latest applied command and fair forward progress before adopting it. Digest every
future-affecting retirement cursor, remaining owned resource, newest intent and scheduler
cursor alongside the active job; exclude wall time and allocator addresses.

The next red/green must count actual cleanup visits/deallocations under tiny credits,
including a large Ready output and cancellation during physical construction. Superseded
work must never commit; repeated cancel/replacement must have bounded peak residency;
replay must agree mid-retirement and across Native/Wasm. Calibrate tree-operation depth,
allocator work and physical-page disposal before calling the combined tick budget green.
The required source/snapshot preparation, side-known invalidation, polygon roads and
general portals remain separate proof owners; no old synchronous A* fallback qualifies.

## Outcome

**Landed 2026-10-01** on `city-maps/sa2-navigation`: the counted planner is in `Battle`, long legs go by road, and planning no longer stalls a tick at either probe size. The ledger of decisions is [choices.md](../choices.md#sa2-navigation-at-full-extent). What follows replaces the proposal above where they differ.

### The seams as built

- **Rules** (`fixtures/village.json`, `navigation`, refused at load when out of range): `work_per_tick` 4,000, `search_cells_base` 20,000, `search_cells_per_m` 200, `road_leg_m` 2,000, `road_access_m` 1,000.
- **Observation:** `MoveState` gains `planning` (a unit holding for its route). The publication layout carries the name list, so the browser decodes it without a change; no panel row shows it yet. A long leg's `policy` reads `fastest` from the tick it starts.
- **Commands:** unchanged. `applied_tick` still means the order was applied, not that a route is ready.
- **`Battle::digest`:** folds the planner's pending state (each request, the work spent on it, its search's progress, the round-robin cursor and any overrun owed). A planner with nothing pending folds nothing. `Battle::from_replay` reproduces every tick's digest, mid-plan included.
- **`RoutePlanner`** (`crates/sim/src/route_planner.rs`), owned by `Battle`, driven by movement: `submit(unit, Request)`, `cancel(unit)`, `pending(unit)`, `advance(&NavigationRules, &RoadNet, [each side's grid and knowledge revision]) -> finished (unit, Request, Plan)`. One request per unit; submitting replaces. `Battle::load()` reports `routes_pending` and `planning_work`.
- **Navigation** (`crates/sim/src/navigation.rs` and `navigation/`): `NavGrid` is what a route is checked on; `RoadNet::build(&WorldGeometry)` is the map's road graph, built once at load and shared by both sides; a `Journey` is one leg's route in progress (road graph, then the grid); `navigation::plan` runs one to its end for tools and tests. `NavGrid::plan` and the exact planner's start-up proofs are gone.

### Measured

`city_report` on the probe maps (`throwaway/city`, and a 10 km variant with a road through the edge midpoint and the centre), on the development machine at load 12 to 23. Instructions are the comparable number.

| Run | Units hold for a route | Planning work | Busiest tick's planning | Slowest tick, ticks over 33 ms | Arrivals |
|---|---|---|---|---|---|
| 6 km, twelve 900 m fast moves, 120 s | 0.13–0.27 s | 30,739 units | 4,019 (allowance 4,000) | 5 ms, none | jeeps 100.7 s, tanks 80–116 s; one jeep still threading a wood |
| 6 km, twelve edge-to-edge (5.5 km), 500 s | 0.7–1.5 s (one jeep in a wood 8.3 s over several plans) | 1,733,501 | 4,074 | 9 ms, none | tanks 468 s; one jeep 378 s, one within 10 m at 499 s |
| 10 km, twelve edge-to-edge (9.2 km), 800 s | 1.0–2.2 s (one jeep in a wood 35 s over several plans) | 5,476,652 | 4,060 | 23 ms, none | two jeeps 594 s and 632 s; tanks still on the road |
| 10 km, one jeep, edge midpoint to centre by road | 0.07 s (2 ticks) | 6,763 | 4,076 | 5 ms, none | **165.4 s** |
| 10 km lattice probe, same order, start 830 m from any road | 0.10 s | 9,758 | 4,056 | 43 ms, one (no planning in it) | 369.1 s |
| 6 km, one jeep, edge midpoint to centre (3 km, 750 m from roads) | 0.07 s | 5,002 | 4,010 | 2 ms, none | 278.8 s |

- **No tick's planning exceeds the allowance by more than one step** (576 units): the busiest tick in any run did 4,076. A full planning tick costs about 25 to 35 million instructions (8 to 15 thousand a unit of work).
- **No tick over 33 ms traces to planning.** Tick p50 is 0.04 to 0.31 ms and p95 0.7 to 4.8 ms across the runs. On the same machine at load 35 to 90 the same runs showed hundreds of ticks over 33 ms; the slowest carried 60 to 100 million instructions and no planning work (the fog sweep and soldiers' own routes among 63,000 to 180,000 trees, which S1 already listed).
- **Load:** building a side's planning grid costs 10.5 G instructions on the 6 km map and about 31 G on the 10 km one. Both are now built with the battle (6 km battle build 22.3 G, 1.5 s, 267 MB resident; 10 km 65 to 67 G, 4.5 to 5 s, about 550 MB), not in the first order's tick.
- **Route quality** against the frozen exact planner (`navigation_quality`, 12,064 cases): shortest routes 0.41% longer on average, worst 7.8%; fastest routes 0.50% slower on average, 15 of 4,757 over 10%, worst 17%. Every route fits the dense grid; no leg has a route in one planner and none in the other. 1,670 goals within a cell of a body or the map edge are moved to the middle of their cell.
- **Village, quick report** (named digest changes): flank 3/3 captured at 431, 430 and 568 s, 225 lost (before: 2/3, 175 lost); ambush 0/3 captured, 60 lost, no tank lost (before: 0/3, 200 lost, one tank). 5,250 G instructions against 5,148 G.

### Tests that pinned the old planner

- `navigation::an_unbroken_water_strip_proves_no_route_without_exploring_a_map_half` asserted the separating-line proof (zero cells searched). The proof is deleted; the test is now `an_unbroken_water_strip_leaves_no_route`: no route, and no more than the near bank searched.
- `movement_scenarios` `t3-tanks-meet-head-on` put the two tanks on a road to make them meet. On a road they now keep right and pass, so the scenario runs in the open, where the wait and the detour it checks still happen.
- `drive::each_road_kind_carries_its_own_speed` measured cruise in the last 50 m of its strip, where a vehicle now moves back to the middle of the road to leave it; the strip is 200 m longer.
- The dense-planner oracle `navigation_equivalence` asserted identical routes. It is now `navigation_quality` and measures the difference.
- `web/scenes/movement.mjs` checked a blocked order five ticks after giving it; it now waits for planning to finish.
- `web/scenes/_battleLook.mjs`'s walk into the west wood (a live village battle, already retuned twice for earlier rule changes) keeps 40 m south of the wood instead of 110 m: in the battle as it now plays the squad was pinned at the corner.

### What is still open

1. **A side's planning grid is rebuilt whole when its knowledge changes** (a tank fells or shoves a tree, a wreck appears, a body is destroyed): 10.5 G instructions on the 6 km map, 31 G on the 10 km one, in one tick. None of the probe runs triggered it; a real battle will. The grid samples the terrain again every time though only bodies changed; the fix is to keep the terrain layer and re-lay only the bodies near the change (cleared forest ground needs a change feed first). **This is the next blocker for a full-size battle.**
2. **Vehicles among trees.** The grid judges a vehicle as a disc on 2 m cells; a trunk at a cell corner blocks no cell, and a hull is longer than it is wide. A jeep sent into or through a wood can stop against a trunk and plan the same route again every two seconds (one jeep did in each 12-unit run, and a jeep sent to a point among trees stops a metre short of it). The off-centre rule fixed the cases the tests met, not the general one.
3. **Unreachable goals cost the whole search limit** before the unit reports blocked (about 5 s for a 2.8 km leg across an unbridged river).
4. **Two columns head-on on a forest track** have nowhere to pull off; in the open they pass or drive round each other.
5. **Roads authored as polygons** are not in the road graph.
6. **The client:** `planning` has no panel row. The order marks now flash again when a route turns out blocked (the order's own flash is over by then); that wants a look in the UI pass.
7. **Native/Wasm digest agreement at full extent** was not run. The planner reads no hash-map order and no wall time, and the web suite and the movement and village scenes pass on the Wasm build.
