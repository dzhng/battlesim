# 14 — Concurrent readiness and ammunition UI

**Status:** planned, not implemented. **Dependencies:** 06, 08, 10, 12, 13. **Milestone:** Village checkpoint.

## Contract and question

Can the player understand simultaneous aim/reload/deployment without inspecting debug state?

User requirements owned or exercised: U02, U03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`packages/battle-renderer` world-anchor projections and `web` React selection panel consume WeaponReadiness/ActionReason only.

## Runnable artifact

/lab/readouts: fixed selected multiweapon unit with simultaneous timers, finite/∞ ammo, guidance and deploying supply, plus near/far zoom states. The command bar exposes all village commands and policy.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Every displayed timer equals published state; concentric rings advance independently; no completed timer remains; cannon variants not separate guns; readable ammo counts; guidance icon; no color-only action distinction; blocked-fire explanations; no enemy hidden readiness leak. Compare same camera/tick before and after UI change.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Weapon-state legibility**. Review crop/mask: **Selected unit ring cluster at 2×/4× plus full 1280×800 and 900×600 frames**. Explicitly out of scope: New terrain/art direction and polishing all HUD chrome.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/14/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Exact colors/icons/spacing and zoom presentation within required information. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Confusing overlapping circles or unreadable numbers require layout iteration, not deleting per-weapon state.
