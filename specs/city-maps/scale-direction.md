# Startup, engine scale and navigation latitude

**User direction, 2026-09-30.** This is the current policy for startup, the engine's
long-term extent and proposed performance-driven behavior changes. It supersedes
older requirements to preserve every original route winner or immediate planning
tick when evaluating a named behavior alternative. Historical proof results and
their frozen oracles remain unchanged.

## Startup and loading

First battle-map startup must take **less than one minute**. A loading screen is
allowed. Measure the complete interval from starting map preparation to a usable,
interactive battle view, including generation/acquisition, encounter preparation,
Battle construction and required renderer resources. Record cold and warm starts,
asset-cache state and download assumptions separately; worker completion alone is
not the finish line.

Preparation may do substantial work before play. Keep the loading UI responsive,
show honest stage/progress information, allow cancellation and report failures.
Superseded requests cannot start a stale battle. Begin gameplay after the complete
prepared scenario and its first usable view are ready. This allowance does not
relax active simulation, frame, publication or memory budgets. S3 and C55 own the
startup proof and player flow; the existing game-ui/visual gates still apply.

## Engine goal: 100 km

Aim for an engine capable of maps **100 km across**, if feasible. Use 100 × 100 km
as an explicit engineering proof envelope until a different shape is selected.
This is a scaling goal, not a measured capability or an additional release preset;
the accepted Small/Medium/Large extents remain in the map-design brief.

Separate world extent from active units, physical content density and resident
detail. A sparse 100 km world does not prove a dense Metro battle at that extent.
Inventory coordinate/index/codec limits, transport ceilings and worst-case native,
wasm/browser/GPU memory before admitting an arm. Bounds must cover both sides,
temporary overlap, rebuilds, full snapshots and edge-to-edge movement. Raising a
cap or allocating a whole-world fine grid is not a scale architecture.

Investigate chunked records, bounded resident detail and hierarchical route work.
Local physical checks still use believable body/footprint resolution; distant route
planning need not search every fine cell. Measure preparation and active work
separately, in instructions retired as well as latency. Keep unsafe full-size arms
rejected until their failed owner changes and a safe proof passes.

## Behavior latitude and evidence

The user invites proposals for game-mechanic changes that substantially simplify
the engine or unlock performance; exact parity with every old behavior is not a
requirement. Present the physical rule, player consequence, scope and expected
work reduction before choosing a production change. A proposed benefit is not a
measured result. The long-move road rule and planning delay below are accepted;
their implementation architecture and the remaining proposals are not selected.

For a named route/timing change, old raw parents, tie order, rounded costs,
waypoints, command visibility ticks and cross-version battle digests may differ.
Keep the old oracle as comparison evidence, not a compulsory equality gate for
that alternative. Record intentional changes and new scenario/rules/build
identities. Same-build deterministic replay and native/wasm agreement remain
required. Digest-neutral optimizations still require exact parity.

Retain physical collision/footprint checks, side-known geometry and fog boundaries,
explicit failures and one authoritative navigation owner. Resource exhaustion is
not proof of NoRoute. Never reveal hidden destruction through a route, teleport
through an obstacle, or silently rewrite a baseline to pass a changed mechanic.
Combat-rule simplifications may also be proposed; none is selected by this policy.

## Accepted navigation behavior

**Automatic road travel:** when a move destination is **more than 5 km away** and
a usable road is nearby, automatically use roads for the long journey, like the
existing double-right-click fast move. This is normal physical movement using
road speeds, not teleportation or a separate strategic travel mode. Retain manual
fast move for shorter journeys.

Use straight-line start-to-destination distance when a move leg starts planning;
freeze that leg's mode across routine replans rather than switching it as the unit
crosses the threshold. Start with ordered move legs, including queued waypoints;
do not silently turn targeting, pursuit or garrison approach into road travel.
These scope/measurement choices are implementation defaults, not additional user
requirements. The user has not specified the numerical meaning of nearby: select
and document a practical access radius with mover-footprint/reachability evidence.

The intended flow is local access to a road, travel through connected road junctions
and crossings, then local travel to the destination. A road is usable only if its
access and passage fit the mover under that side's known geometry. A road across
an impassable river or a disconnected/dead-end segment does not satisfy the check
merely by being close. If a suitable road corridor is unavailable, retain a legal
off-road alternative or report the actual obstruction through bounded planning.
The implementation must define corridor usefulness/detour limits; avoid absurd
road detours or treating the inability to reach a road as NoRoute to the goal.

**Road generation:** generated maps should have plenty of connected, accessible
roads so the nearby-usable-road condition is true for nearly all ordinary long
moves. Layout trials must measure access coverage and connectivity across map
types/seeds and verify real crossings, not just count decorative road length.
Exact coverage and proximity numbers remain measured generator outputs.

**Planning delay:** the user accepts delayed route planning during play. Implement
bounded deterministic planning with explicit pending/completion/cancel/failure
semantics; the battle continues while planning is pending. The table below gives
the recommended current-order behavior and the work/replay proof still required.
Acceptance of a delay does not select an unlimited queue or an indefinite wait.

The current Fastest policy still searches the fine grid. Merely choosing that
policy automatically implements the preference but does not establish a work
bound. Evaluate a road-junction graph and local connectors as the long-route
architecture, with general-region routing for non-road journeys.

## Recommended navigation proposals

| Proposal | Physical rule and player consequence | Work to prove |
|---|---|---|
| Hierarchical routing | Plan the long journey through traversable regions, junctions and crossings, then refine a local route for the mover's footprint. Tanks/trucks may take a sensible route that differs from the old fine-grid winner; infantry still uses gaps it physically fits. Keep fastest/shortest policy intent and measure route quality. Roads must not become the only legal vehicle terrain. | High-level graph/preparation cost, search growth with meaningful geography, bounded local refinement and path quality on roads, forests, competing bridges and disconnected regions. |
| Planning over several ticks (delay accepted; implementation to prove) | A movement request can be visibly pending while a deterministic amount of planning work runs each tick. While pending, continue a still-valid existing order, or hold when none exists. Commit a complete validated route on a deterministic tick. This intentionally changes order response timing. | Counted work budgets for search, construction and validation; bounded queued jobs, fairness, cancellation/supersession, side-known revision changes and explicit failure. Wall-clock worker races cannot choose the activation tick. Account for authoritative pending state in replay/digests. |
| Local obstruction handling | Keep a valid long-distance corridor while reacting locally to temporary traffic; rebuild broader routes when meaningful known obstacles invalidate that corridor. Units must stop or find another legal passage when the route becomes unsafe. | No indefinite waiting, stale unsafe movement or hidden-state leaks; bounded invalidation and contention. Sharing immutable corridor data between compatible movers must not share mutable orders or assume different footprints fit. |

Picture a truck approaching a blocked bridge, infantry using a narrow side gap and
a tank meeting a wreck it can or cannot push. Check both sides' knowledge, cancel
or replace orders during planning, and walk blockage/destruction to its end state.
A known newly blocked passage stops or invalidates movement; an unseen change is
handled through the existing knowledge/contact rules. An alternative bridge can
be chosen without exact old-parent parity. A wait needs an explicit retry, cancel
or failure exit. These moments, replay and paired battle results are the acceptance
cases when a proposal is selected under tweak-mechanics.

For the accepted road rule, include just below/at/above 5 km, accessible versus
inaccessible nearby roads, disconnected roads, competing bridges, road blockages,
local start/end connectors, queued/replaced moves and both sides' hidden-change
cases. A truck takes the main road for the long haul; infantry and wider vehicles
still obey their own access/clearance. A wreck blocks or slows passage according
to the existing body/push rules; road preference grants no new ability to pass it.

The exact uniform-parent experiment remains an optional preservation investigation.
It must not monopolize the navigation gate now that named alternatives are allowed.
SA2 should compare the simplest promising alternatives on the safe bridge and
physical/knowledge cases before admitting larger extents; G0 records the selected
contract and its compatibility consequences.
