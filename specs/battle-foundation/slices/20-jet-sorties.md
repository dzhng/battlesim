# 20 — Off-map strike lifecycle

**Status:** planned, not implemented. **Dependencies:** 07, 09, 18, 19. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Can a purchased jet execute a delayed dangerous strike and return only if it survives?

User requirements owned or exercised: A02. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::sortie::{request, enter, strike, exit, service}` owns jet availability, consuming ordinary flight/weapons/damage.

## Runnable artifact

/lab/sorties: fixed map-edge entry, route to a chosen ground point, bomb release, exit and service cooldown; repeat with radar AA. No teleporting damage. Prototype one pass per order and off-map rearm state.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Travel delay physical; high-air detection only capable sensors; bomb gravity/earliest impacts; friendly blast; destruction cancels availability permanently; surviving exit returns after configured service; no hidden retargeting to moving enemies; leaving bounds follows sortie corridor exception, not ground navigation loophole.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Strike arrival and availability**. Review crop/mask: **Jet/bomb corridor sequence plus availability panel**. Explicitly out of scope: Dogfight AI, airfields and polished jets.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Initial altitude/speed/service tuning; start 1000 m AGL, 200 m/s and 60 s rearm, then record evidence. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

AA must meaningfully counter otherwise strong strikes; tune metrics rather than scripted intercept immunity.
