# 02 — Authoritative terrain and obstacles

**Status:** planned, not implemented. **Dependencies:** 01. **Milestone:** Village checkpoint.

## Contract and question

Do physical queries and the visible world describe the same surfaces?

User requirements owned or exercised: M03, M07, M09. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::world::WorldGeometry::{height_at, raycast, sweep, surface_at, obstacle_revision}` owns ground triangles, prop shapes and bounded map. Renderer receives its mesh/props.

## Runnable artifact

/lab/geometry: hill, slope threshold, forest volume, road, water, indestructible bridge and two obstacle heights. Click/probe terrain and toggle query overlays. Use contracts.md shape/triangulation rules; include a thin-wall target for later flight tests.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Analytic plane/ridge intersection, identical triangle sample and rendered vertex positions, shared slope cutoff, bridge top over water, bounded edge rejection, prop ray occlusion and surface normals. The debug probe consumes exported geometry rather than regenerating it independently.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Terrain and obstacle silhouette**. Review crop/mask: **Ridge outline and bridge/ground contact crops**. Explicitly out of scope: Materials, grass density, environmental effects and fog.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/02/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Acceleration data structure and primitive tessellation that preserves authoritative triangles. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Visible/physical height disagreement rejects the slice; no compensating visual offsets.
