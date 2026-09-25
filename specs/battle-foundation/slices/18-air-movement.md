# 18 — Helicopters and layered observation

**Status:** planned, not implemented. **Dependencies:** 05, 06, 07, 08, 10, 16. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Does physically lower helicopter flight create the agreed visibility and mobility tradeoff?

User requirements owned or exercised: V01, V05, V06, A01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::movement` aerial flight modes plus sensing Layer::Helicopter; same flight/body world, no parallel air sim.

## Runnable artifact

/lab/low-flight: helicopter normal/low paths around ridge, building and forest, with vehicles/infantry beneath. Prototype starts normal 60 m AGL and low 15 m AGL, speed 40/20 m/s, adjusted only through recorded tuning.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Remain helicopter detection layer; terrain clearance along path; speed/altitude transition; low flight limits own sight/fire; air observers ignore foliage for vehicles but not infantry; hill/building obstruction; midair collision and ground impact; direction and altitude exposed only through legal observation.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Altitude and terrain clearance**. Review crop/mask: **Ridge profile and helicopter ground-reference crop**. Explicitly out of scope: Rotor animation and flight aerodynamics.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/18/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Visual rotor proxy and deterministic local obstacle avoidance; no altitude teleport. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Low flight that provides stealth without restricting own geometry violates the chosen physical behavior.
