# 19 — Radar support and self-guided AA

**Status:** planned, not implemented. **Dependencies:** 10, 12, 18. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Do radar placement and guidance type provide different counterplay?

User requirements owned or exercised: V01, V06, P07, P08, A03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::sensing` radar evidence and `sim::flight` guidance variants; weapon owner selects support/seeker modes.

## Runnable artifact

/lab/air-defense: radar AA in clearing/forest, a scripted high-layer flight target, and self-guided infantry AA. Radar range/LOS respects buildings, hills and forest; inside forest has zero radar detection. One truth/observation/guidance engine.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Radar supports only with own contact and stationary launcher; shared detection insufficient; loss freezes last point forever. Infantry seeker launch needs own acquisition, then crew free; seeker LOS loss freezes last target point with no reacquisition as a spec continuation default. Midair last-point arrival detonates configured warhead or expires, never searches hidden truth. Ground/air visibility fields remain distinct.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Radar blocked/support versus seeker state**. Review crop/mask: **Clearing/forest pair and missile guidance indicators**. Explicitly out of scope: Radar UI spectacle and missile smoke art.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Initial ranges/damage/turn limits in continuation config preserving 2–3-hit intent. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If forest radar or autonomous seekers read team truth incorrectly, reject at observation seam.
