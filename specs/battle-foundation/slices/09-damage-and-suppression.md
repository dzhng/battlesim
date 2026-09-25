# 09 — Consequences of physical fire

**Status:** planned, not implemented. **Dependencies:** 07, 08. **Milestone:** Village checkpoint.

## Contract and question

Do impacts and near misses produce the intended tactical consequences?

User requirements owned or exercised: V03, P09, P10, P13, P14, M06, M07, M08. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::damage::resolve(events, bodies, armor, cover) -> casualties/suppression/destruction`; health and suppression are sim-owned.

## Runnable artifact

/lab/consequences: exposed/forest infantry, near-miss pinning, tank front/side/rear/roof, friendly collateral, accumulating corpses and blocking vehicle wrecks. Persistent remains use the normal world obstacle/render owners.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Penetrating damage fixed after face check; failed penetration stops with zero HP loss; no overpenetration; near misses suppress without damage; suppression slows rather than retreats; lull recovery; blast per soldier and friendly teams; cover probability differs but conditional damage does not; late wreck invalidates known route; corpse stays without blocking navigation.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Impact, suppression and remains legibility**. Review crop/mask: **Exposed/covered pair plus wreck passage**. Explicitly out of scope: Garrison wall abstraction, smoke art and particle density.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/09/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Impact glyphs and sparse particles; probability sampling must be deterministic. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If exposed and covered results differ only through damage reduction, or one lucky seed supports acceptance, reject.
