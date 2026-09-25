# 10 — Supported AT ambush

**Status:** planned, not implemented. **Dependencies:** 05, 06, 07, 08, 09. **Milestone:** Village checkpoint.

## Contract and question

Can launcher visibility and prompt movement change an AT engagement without hidden homing?

User requirements owned or exercised: P05, P06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::flight::GuidanceSupport { launcher, target_observation, last_point }`; own sensor feed from sensing, support slots from weapon state.

## Runnable artifact

/lab/ambush: two tanks, AT team, hill escape, shared scout-only spotting, launch/Stop/move and last-point replay. Use ordinary and prepared crossfire variants of the same weapon data.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Own-ID launch and sustain; shared ID insufficient; movement/Stop/death/LOS releases support immediately; missile never reacquires; frozen point remains fixed despite hidden motion; stationary target at that point takes normal damage; same trajectory owner; bounded turn/lifetime; crew free despite missile still flying. Compare prompt/delayed retreat on fixed seeds.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Guidance and escape legibility**. Review crop/mask: **Missile/launcher-target corridor with observed point markers in lab only**. Explicitly out of scope: Radar/seeker AA, jet flight and cinematics.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/10/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Guidance steering numeric solver within configured turn limits. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Retreat always fails/succeeds regardless of geometry indicates tuning or state error; retain N03 tradeoff.
