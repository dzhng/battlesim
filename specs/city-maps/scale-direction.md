# Startup, engine scale and navigation latitude

**User direction, 2026-09-30.** This is the current policy for startup, the engine's
long-term extent and proposed performance-driven behavior changes. It supersedes
older requirements to preserve every original route winner or immediate planning
tick when evaluating a named behavior alternative. Historical proof results and
their frozen oracles remain unchanged.

## Release presets and transit target

The current playable sizes are [M04 in the map-design brief](procedural-maps.md#closed-decisions).
This user-authorized size revision supersedes older fixed-extent instructions in
slices and spike verdicts. Preserve the original sizes/hashes of measured evidence;
update generator presets, bounds-dependent consumers and new admission arms to M04
through a named preset/map identity change. This changes playable battlefield extent, not
building/street scale or weapon ranges. The architecture envelope below is a
separate capability requirement from selectable release presets.

## Playable area and rendered surroundings

The selected size is the **playable area**, not the outer edge of the rendered
world. Render surrounding terrain/scenery beyond the playable perimeter so the
battlefield sits inside a larger landscape. The margin's exact extent and detail
are measured design choices; no numerical margin has been selected.

Keep playable bounds and outer rendered/world bounds explicit in the common map
preparation contract; implementation chooses the schema once with the compiler,
simulation and renderer consumers. Use playable bounds for size labels, encounter
placement and gameplay-boundary policy. Use the wider bounds for scenery coverage,
overview/camera framing and resource accounting. Do not derive the playable boundary
from terrain mesh dimensions or enlarge deployment space because scenery extends
beyond it. The physical role of surrounding geometry must be explicit: visual-only
scenery must not silently become a movement blocker or combat cover, and scenery
with a physical role must use the shared geometry contract.

Travel targets measure from the **playable edge** to the playable centre. Startup,
memory, resident detail, uploads and frame measurements include the rendered
surroundings. Record playable, physical-world and rendered extents separately when
they differ, including the bounded margin at the architecture's maximum playable
extent. A playable-only allocation/frame pass cannot prove the complete map fits.

## Vehicle transit target

The user wants a vehicle on Large to reach the centre from an edge **within three
minutes**. For the midpoint of an edge this is 5 km: at least **27.78 m/s = 100
km/h average** along a straight unobstructed journey. At the accepted light-vehicle
road cap, that journey takes about 2 minutes 44 seconds at uninterrupted cruising
speed, leaving about 16 seconds for planning, turns and slower approaches. At that
cap, 5.5 km consumes all three minutes before stationary delays. A corner-to-centre
journey is about 7.07 km and requires 141.42 km/h average, beyond the cap. These are
arithmetic limits, not observed travel times or an all-edge-position guarantee.

Use an uncontested road-connected edge-to-centre scenario as the initial tuning
proof, starting with a light vehicle; name the vehicle, start location, actual route length and first/last local
connectors. Include planning delay, turning and other travel losses in elapsed
order-to-arrival time. Representative vehicle scope and edge midpoint versus other
edge positions must be explicit; this is not a guarantee through hostile blockages.
Report heavier vehicle transit separately rather than giving every vehicle the
light vehicle's speed. Tune absolute mobility and road-network directness to the arrival target, rather
than treating the provisional road/off-road ratio as sufficient. Do not silently
scale simulation time or multiply all units' combat movement by the same factor.

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

## Architecture envelope: 20 × 20 km

The architecture must support a playable area up to **20 × 20 km**, plus its
bounded rendered surroundings. This replaces the earlier 100 km aspiration;
larger playable extents are outside the required design envelope. Inventory the
complete world/render margin rather than treating the playable size as its limit.
Prefer the simplest architecture that proves this support, rather than adding
complexity for a hypothetical larger world. This is a capability requirement,
not an already measured pass or an additional selectable release preset; the
accepted Small/Medium/Large extents remain in the map-design brief.

Separate world extent from active units, physical content density and resident
detail. A sparse 20 km world does not prove a dense Metro battle at that extent.
Inventory coordinate/index/codec limits, transport ceilings and worst-case native,
wasm/browser/GPU memory before admitting an arm. Bounds must cover both sides,
temporary overlap, rebuilds, full snapshots and edge-to-edge movement. Raising a
cap or allocating a whole-world fine grid is not a scale architecture.

After safe admission, prove the envelope with representative active routes,
content density and resident/view transitions, not only empty construction. G0
must name supported unit/content loads and resource budgets alongside the extent.
Retain historical measurements at their original sizes; new 20 km evidence needs
its own inputs and results.

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

**Automatic road travel:** for an ordered move leg **over 2 km straight-line distance**
with usable nearby roads, automatically choose road travel rather than a slow direct
cross-country journey, like the existing double-right-click fast move. The user selected
this cutoff on 2026-09-30; exactly 2 km does not cross the strict threshold. It replaces
the earlier 5 km prototype test value. Access thresholds, usefulness/detour admission and
movement-speed tuning remain implementation choices to measure.
This is normal physical movement using road speeds, not teleportation or a separate
strategic travel mode. Retain manual fast move for shorter journeys.

Use straight-line start-to-destination distance when a move leg starts planning;
freeze that leg's mode across routine replans rather than switching it as the unit
crosses the threshold. Keep the selected distance trigger and evaluate the road corridor's estimated arrival-time benefit, including access/exit detours;
use a cheap coarse estimate rather than another full fine-grid search to decide.
Start with ordered move legs, including queued waypoints;
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

**Movement pace:** use roughly **2–3× faster road travel than ordinary cross-country
movement** as an initial tuning direction, not a compulsory multiplier for every
unit or a verified universal ratio in the reference games. Tune absolute speeds
and the road advantage together against representative journey times and readable
combat. Distinguish cruising speed from complete journey time: acceleration, turns,
traffic and road detours reduce the end-to-end gain. The trigger chooses a route;
it does not grant a second speed bonus on top of the road mobility data.

**Vehicle speeds:** light wheeled vehicles do **110 km/h** on roads (30.56 m/s), and
no mover exceeds **130 km/h** (M20; the catalog refuses a faster type at load).
Use Broken Arrow's unit-specific road/off-road speeds as tuning references, with
heavier vehicles slower as appropriate; do not turn the light cap into every
vehicle's default. The cap takes precedence over the earlier provisional ratio
and any illustrative higher speeds used to explain the old, larger map sizes.
The Tigr's documented 110/55 km/h is a reference light profile. Higher reference
speeds or engine upgrades do not override this game's cap. No new runtime profile
or source-unit parity is selected by this documentation update.

Tune through the existing mobility/surface owners, with one effective speed shared
by route-time estimates and movement. Vehicles retain footprint, steering, reverse
and body/push constraints. Treat foot movement and transport as separate physical
cases; do not automatically give infantry a vehicle cruising multiplier. Name
intentional speed/config/digest changes and measure paired battle outcomes.

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

For the accepted road rule, include just below/at/above the selected tunable trigger, accessible versus
inaccessible nearby roads, disconnected roads, competing bridges, road blockages,
local start/end connectors, queued/replaced moves and both sides' hidden-change
cases. A truck takes the main road for the long haul; infantry and wider vehicles
still obey their own access/clearance. A wreck blocks or slows passage according
to the existing body/push rules; road preference grants no new ability to pass it.

## Reference movement evidence

These references inform tuning; they do not set this game's constants.

- Broken Arrow's [official manual](https://ftp.matrixgames.com/pub/BrokenArrow/BrokenArrowManualEBOOK.pdf)
  describes Fast Move as selecting a fast route using roads. Its developer's
  September 2026 [Russian Guard announcement](https://steamcommunity.com/app/1604270/allnews/?l=english)
  lists the Tigr at 110/55 km/h road/off-road (140/70 with its engine upgrade),
  Vystrel at 90/45 and Typhoon-K at 110/55: **2× for these vehicles**, not proof
  of a universal multiplier. The Shchuka's listed 95/45 is about 2.11×.
- A current universal WARNO road/off-road multiplier was not verified from primary
  documentation. Do not adopt older modding formulas, card values or world-scale
  compression claims as measured current movement ratios.
- This engine already has separate road/off-road vehicle speeds and a foot road
  multiplier in the [mobility contract](../../crates/contract/src/catalog.rs).
  Check actual movement and total journey time when tuning; policy selection alone
  does not create a new road-speed boost.

The exact uniform-parent experiment remains an optional preservation investigation.
It must not monopolize the navigation gate now that named alternatives are allowed.
SA2 should compare the simplest promising alternatives on the safe bridge and
physical/knowledge cases before admitting larger extents; G0 records the selected
contract and its compatibility consequences.
