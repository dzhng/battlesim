# 17 — Transport lifecycle

**Status:** moved on 2026-10-10 to the [transport placeholder](../../transport/README.md), which covers trucks and helicopters together; that plan carries the checks below. Previously: planned, not implemented. **Dependencies:** 04, 08, 09, 11, 12, 16. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Can squads ride, unload and survive destruction without duplicate bodies or commands?

User requirements owned or exercised: L06, L07, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::transport::load/unload/destroy` owns passenger membership; orders, occupancy and casualty owners remain shared.

## Runnable artifact

/lab/transports: timed stationary load/unload, queued arrival and dismount, mixed survivors from destruction. Continuation after the village checkpoint; no embark support is falsely claimed earlier.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Capacity and whole-squad entry; stop/proximity gates; aboard no fire/sensors/capture or independent world collider; transport destroyed samples each passenger; survivors outside heavily suppressed; blocked unload rejects visibly; no duplicate IDs/corpses; transport default gun rules remain.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Passenger state and dismount positions**. Review crop/mask: **Transport/passenger panel and wreck survivor crop**. Explicitly out of scope: Interior meshes and boarding animations.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Prototype capacity, load time and survival probability in one continuation config; default 12 soldiers, 2 s, 50%. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Unclear occupancy or overlapping survivors changes slots/UI, not selected survival model.
