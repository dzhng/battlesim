# 22 — Full combined-arms scale and pacing

**Status:** planned, not implemented. **Dependencies:** 16, 17, 18, 19, 20, 21. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Does the complete roster sustain the intended 4 km, 60–100-unit-per-side, 45–60-minute battle?

User requirements owned or exercised: S04, S05, P03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

Scenario/config composition over existing owners; add dedicated indirect-fire weapon definitions using the already-proven high-arc flight capability.

## Runnable artifact

/battle/combined-arms: 4 km map with redundant crossings, infantry/recon/tanks/transports/supplies/helicopters/jets/radar AA and deployed mortar/artillery. This is a continuation milestone, not authorized implementation during spec writing or a claim the village contains every feature.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Full-length fixed-seed and human play evidence; objective outcomes/income/stock/pacing; air-ground counters; high arc versus direct arc; sparse and dense late-state performance; finite specialist ammo and recovery; persist all remains; replay and side-filtered AI. Target match length is an empirical tuning distribution, not forced delays or guaranteed timer termination.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Whole battlefield information hierarchy**. Review crop/mask: **Full map, frontline and rear-service crops**. Explicitly out of scope: Finished visual assets, campaign, deck building and networking.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Map layout and provisional unit tuning with archived comparison evidence; the user owns changed tactical rules. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Pacing failure triggers scenario/balance reslicing; network architecture remains separately specified.
