# 16 — Current-host scale and late-battle verdict

**Status:** planned, not implemented. **Dependencies:** 15. **Milestone:** Village checkpoint.

## Contract and question

Can the architecture sustain full target populations and persistent battle history?

User requirements owned or exercised: S04, W11, M06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`apps/battle-lab` load scenarios and harness telemetry consume actual simulation/resource counters; no second benchmark sim.

## Runnable artifact

/lab/endurance: 100 units per side, then dense speculative fire, seeded reinforcement/casualty turnover and accumulated remains. Report actual browser/adapter/config. Village stays the play checkpoint; stress is synthetic, clearly labeled.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

60 simulated minutes with real projectile/knowledge/supply operations and remains retention; repeated-session cleanup; no wall-clock catchup spiral; bounded publication credits and path work; compare frame/tick/memory to validation.md targets. Distinguish accelerated sim soak from real-time render run. Never cap bullets or erase remains to pass.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Late-state readability under load**. Review crop/mask: **Same full camera and dense contact/remains crop at early/late states**. Explicitly out of scope: New art and lowering rules to meet budgets.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Indexing/batching/culling optimizations preserving outcomes; measured budget revisions require documented rationale. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If workload is infeasible, reslice the bottleneck and state the failure; do not silently shrink the agreed eventual battle scale.
