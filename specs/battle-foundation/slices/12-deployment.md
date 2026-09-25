# 12 — Reversible deployment progress

**Status:** planned, not implemented. **Dependencies:** 03, 04, 08. **Milestone:** Village checkpoint.

## Contract and question

Does setup reverse cleanly under move and cancellation without losing time or trapping a unit?

User requirements owned or exercised: W15, L01, L02. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::deployment::advance(current, desired, duration) -> readiness + movement_gate`; unit-owned, used by later service and weapons.

## Runnable artifact

/lab/deployment: supply proxy with deploy/pack/Stop and queued movement; inspect one progress value and desired end state. State controls pose interpolation, not vice versa.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Equal forward/back duration; 40%-deployed unit needs 40% duration to pack; 50%-packed reversal needs half duration to deploy; no translation until packed; no service before fully deployed; Stop clears move and returns desired deployed; repeated reversals do not reset or create stock/actions.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Deployment progress direction**. Review crop/mask: **Selected supply proxy and progress indicator**. Explicitly out of scope: Authored animation and actual supplies.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/12/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Primitive folded/unfolded pose; no new state flags duplicating progress. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Confusing cancellation feedback requires a presentation fix unless the user explicitly changes reversal semantics.
