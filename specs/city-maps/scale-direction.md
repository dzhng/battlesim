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
measured result, and these proposals are not selected mechanics.

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

## Recommended navigation proposals

| Proposal | Physical rule and player consequence | Work to prove |
|---|---|---|
| Hierarchical routing | Plan the long journey through traversable regions, junctions and crossings, then refine a local route for the mover's footprint. Tanks/trucks may take a sensible route that differs from the old fine-grid winner; infantry still uses gaps it physically fits. Keep fastest/shortest policy intent and measure route quality. Roads must not become the only legal vehicle terrain. | High-level graph/preparation cost, search growth with meaningful geography, bounded local refinement and path quality on roads, forests, competing bridges and disconnected regions. |
| Planning over several ticks | A movement request can be visibly pending while a deterministic amount of planning work runs each tick. While pending, continue a still-valid existing order, or hold when none exists. Commit a complete validated route on a deterministic tick. This intentionally changes order response timing. | Counted work budgets for search, construction and validation; bounded queued jobs, fairness, cancellation/supersession, side-known revision changes and explicit failure. Wall-clock worker races cannot choose the activation tick. Account for authoritative pending state in replay/digests. |
| Local obstruction handling | Keep a valid long-distance corridor while reacting locally to temporary traffic; rebuild broader routes when meaningful known obstacles invalidate that corridor. Units must stop or find another legal passage when the route becomes unsafe. | No indefinite waiting, stale unsafe movement or hidden-state leaks; bounded invalidation and contention. Sharing immutable corridor data between compatible movers must not share mutable orders or assume different footprints fit. |

Picture a truck approaching a blocked bridge, infantry using a narrow side gap and
a tank meeting a wreck it can or cannot push. Check both sides' knowledge, cancel
or replace orders during planning, and walk blockage/destruction to its end state.
A known newly blocked passage stops or invalidates movement; an unseen change is
handled through the existing knowledge/contact rules. An alternative bridge can
be chosen without exact old-parent parity. A wait needs an explicit retry, cancel
or failure exit. These moments, replay and paired battle results are the acceptance
cases when a proposal is selected under tweak-mechanics.

The exact uniform-parent experiment remains an optional preservation investigation.
It must not monopolize the navigation gate now that named alternatives are allowed.
SA2 should compare the simplest promising alternatives on the safe bridge and
physical/knowledge cases before admitting larger extents; G0 records the selected
contract and its compatibility consequences.
