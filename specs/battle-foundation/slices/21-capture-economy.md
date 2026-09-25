# 21 — Persistent objectives and reinforcements

**Status:** planned, not implemented. **Dependencies:** 04, 06, 08, 17, 20. **Milestone:** Deferred continuation — implement only after village checkpoint and explicit continuation scope.

## Contract and question

Do objectives drive fighting without amplifying purchasing power?

User requirements owned or exercised: S06, S07, S08, S09, S10. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::match` owns objective control/income/purchase admission; every spawned unit uses normal scenario/unit creation contracts.

## Runnable artifact

/lab/match: three objectives, equal periodic income, persistent ownership, hidden contest clue, edge reinforcement entry and jet purchase. Prototype full match starts 1500 points, 100 points/min per side, 20 s capture, and 1800 victory points; majority earns 1 point/s. These are explicit provisional continuation values, not village victory rules.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Infantry/armed ground capture; passengers/supply/air do not; hidden eligibility contests without exact disclosure; progress pauses when contested and resumes for same captor, changes reset on opposing attempt; contested objective contributes no score to owner count; strict majority of all objectives required; ownership persists; equal income independent of territory; budget debit once; legal free edge entry or visible queue; no offmap bypass. Victory threshold first reached ends scoring; simultaneous tie is draw.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Objective status and purchasing clarity**. Review crop/mask: **Objective contested state and purchase/arrival panel**. Explicitly out of scope: Deck building, factions and campaign.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/21/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Initial roster prices/budgets and entry queue UI; preserve equal-income policy. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If territorial lead compounds income or objective UI reveals exact hidden units, fix the contract.
