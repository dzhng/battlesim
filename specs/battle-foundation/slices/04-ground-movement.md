# 04 — Routing and group intent

**Status:** planned, not implemented. **Dependencies:** 02, 03. **Milestone:** Village checkpoint.

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

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/04/` with machine/config metadata.
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
