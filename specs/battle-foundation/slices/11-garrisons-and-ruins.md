# 11 — Buildings as abstract fighting positions

**Status:** planned, not implemented. **Dependencies:** 02, 04, 08, 09. **Milestone:** Village checkpoint.

## Contract and question

Can occupants gain probability-based protection without inconsistent projectile walls?

User requirements owned or exercised: P12, P13, M07, L08, L09, L10, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::garrison::{enter, exit, allocate_slots, collapse}` owns occupancy; world owns facade slots/shell and ruin geometry.

## Runnable artifact

/lab/garrison: two friendly squads share a building by capacity; entry/exit, outward firing, direct hits, wall misses, enemy behind building, collapse and escape. Use contracts.md target-independent perimeter hit-region representation.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Whole-squad capacity atomicity; no duplicate occupant bodies; outgoing own-wall clearance; all-target collision semantics; cover applied once; shots beyond hit shell; structural-only damage; collapse survivor/corpse accounting and heavy suppression; no legal exit means casualty, not teleport; permanent impassable lower ruin blocks according to shape.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Garrison position and collapse readability**. Review crop/mask: **Building perimeter hit regions and lower ruin silhouette**. Explicitly out of scope: Window meshes, interiors and destruction animation.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/11/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Test outgoing shots toward all four facades and corners, target-facing slot capacity waits, observer rays from occupied slots, and another building blocking normally.

## Decision budget

Delegated: Slot distribution with safe capacity; placeholder collapse swap. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If the proxy looks like unprotected exposed infantry or feels wall-transparent, reslice this representation before polished assets.
