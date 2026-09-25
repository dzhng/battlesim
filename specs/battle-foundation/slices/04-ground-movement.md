# 04 — Routing and group intent

**Status:** complete 2026-09-25. **Dependencies:** 02, 03. **Milestone:** Village checkpoint.

## Contract and question

Can a player position a mixed group predictably through roads and obstacles?

User requirements owned or exercised: W16, W17, M01, M02, M03, M04, M05, M08, M09, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::navigation::plan(known_geometry, footprint, route_policy, goal) -> Path|Blocked`; `sim::orders` owns queues/group destinations; physical movement consumes true collisions.

## Runnable artifact

/lab/movement: shortest/fastest side-by-side routes, friendly traffic, group destination offsets, late obstruction injection through scenario events, route-blocked state. Deliver selection, ordinary move, double-right fast move, Shift waypoints, Stop and camera controls now; later command types extend this owner.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Crafted fastest route differs from shortest; speed multipliers and shared slope cutoff; water/bridge; no off-map detour or diagonal corner cut; vehicle footprint clearance; moving crowd makes progress or explains blockage; relevant obstacle change replans; unreachable retains destination; no every-frame repeated searches; queue/double-click semantics.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Route and destination readability**. Review crop/mask: **Road junction plus selected group destinations**. Explicitly out of scope: Combat/contacts and detailed animations.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Double-right-click must upgrade its own move token after ticks have applied the first click, both active and Shift-queued; unrelated waypoints survive.

## Decision budget

Delegated: Local avoidance method under deterministic forward-progress checks. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Unexpected detours, rigid soldier jams or formation mixing require fixture changes before combat.

## Verdict — 2026-09-25

Accepted. What shipped:

- **`sim::navigation`.** A 2 m planning grid classifies each cell with the world's surface rule, plus the movement-blocking props the side *knows*. A chamfer clearance field lets a footprint of any width ask whether it fits. A* over octile moves never cuts a blocked corner; shortest minimises distance and fastest minimises per-class travel time (road beats forest; slope multiplier `max(0.35, 1 − slope/50)`). String-pulling runs over turn points and never raises the plan's cost. `plan_avoiding` handles traffic detours.
- **`sim::movement`.** Plans happen only on an order, on a relevant change to side knowledge, or after 2 s without progress. Route-blocked units keep their destination and retry only when the side's knowledge changes or a new order arrives. Units follow routes at surface speed; vehicles turn at 45°/s and turn in place beyond 60°.
  - **Traffic.** Vehicles wait for friendly vehicles and squads and name the blocker. After 2 s, the higher-id vehicle plans around the lower-id one it waits for, which clears head-on deadlocks. Squads step around vehicles and softly push apart from other squads (flexible spacing).
  - **Knowledge.** Obstacles added after setup (`ScenarioEvent`) become known to a side only when one of its units comes within 2 m. True geometry always decides actual movement.
- **Orders** (`Battle::apply`). A group move keeps each unit's offset from the group centre (compressed to 40 m, snapped to standing room within 16 m). Shift queues; a plain order replaces the queue; Stop clears it. `UpgradeMove { gesture }` changes only orders from that gesture, active or queued, even after they applied. A finished gesture is an acknowledged no-op.
- **Squads.** Infantry squads carry member soldiers in two staggered ranks 2.5 m apart. Their poses are published for the renderer and for slice 05's sight samples.
- **Observation.** Adds move state, policy, blocker, remaining route, queued destinations and members. These travel as layout-described variable sections.
- **Browser.**
  - `useUnitControl` is the one command path: click, Shift-click and box selection; right-click move; double right-click upgrade via `MoveGestures` (350 ms / 6 px); Shift queue; S stop; the ack log.
  - `TickInterpolator` blends own units between completed ticks. This resolves slice 03's open follow-up.
  - `orderOverlay` drapes routes on the walkable surface and draws destination and queued rings plus blocked and waiting marks.
  - The viewport gained left-drag box selection, arrow-key and screen-edge panning, and per-frame instances.

Tests:

- **Native (16).** Navigation (8): fastest takes a road detour where shortest goes straight, and infantry doesn't bother; speeds by surface and slope; one cutoff for everyone, routed round; water crossed only at the bridge; on-map with no corner cutting; a 5 m gap fits infantry but not a tank; an enclosed goal is blocked; only known props shape a plan. Movement (8): tank routes round walls; a group keeps its arrangement and each unit its own speed; an injected wall is learned on contact and routed around while the far side never plans; an unreachable plateau top stays blocked with one search in 300 ticks; double-click upgrades only its own gesture after it applied; head-on tanks pass without overlapping; a squad skirts a parked tank; idle units never search.
- **TypeScript.** Gesture recogniser (3) and interpolation (2).
- **Browser (11 checks).** Fastest route more than 1.5× longer than shortest yet arrives first (tick 2168 against 2530); a unit meets the tick-150 wall, replans and still arrives; the cliff top reports "route blocked" and keeps its order; squads route through the gap; drag box selects; real double right-click upgrades the same gesture without adding a waypoint; Shift right-click queues; S stops and clears.

Visual gate (route and destination readability). The first critique found:

- the fast route ran through the other tank's destination ring (demo goals were 20 m apart);
- the pale "shortest" colour had low contrast on grass;
- the start area sat under the panel;
- routes and rings were too thin at lab zoom.

All were fixed: goals separated, a warm yellow for shortest, 1.6 m ribbons and 4 m rings, reframed camera. Authentic items stay: the long wall reads edge-on as a thin pole, soldiers are specks at true scale, and routes through forest are real (M02 slows, not blocks). The final critique found the yellow selection highlight, the yellow shortest route and the amber waiting mark too alike, so shortest is now magenta. It also found route ownership ambiguous where two routes start near each other. On-map unit and destination labels are recorded for slice 14's world-anchored readouts rather than added here.

Resolution consequence recorded in choices.md: at 2 m planning cells a gap needs about 4 m to admit infantry and about 6 m to admit a tank.
