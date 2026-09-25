# 15 — Replayable combined-arms encounter

**Status:** planned, not implemented. **Dependencies:** 04, 06, 09, 10, 11, 13, 14. **Milestone:** Village checkpoint.

## Contract and question

Does the combined encounter reward scouting, suppression, flanking and rotation?

User requirements owned or exercised: N01, N04, N05, N06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`ScenarioDefinition` + `OpponentPolicy(ObservationFrame) -> commands` compose existing owners; no new combat logic in scenario/UI.

## Runnable artifact

/battle/village and /replay/village with authored setup, defensive bot, reset/pause/seed/replay controls and fixture-specific hold condition. Use encounter.md scripts and acceptance targets; all tactical play uses real commands and same observation boundary.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

All ten-seed tactical comparisons and service roundtrip from encounter.md; same-build replay digest; bot hidden-state metamorphic test; reset resource cleanup; full UI command path; no scenario damage cheats. Archive result tables and full image set, including failures.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Composed tactical readability**. Review crop/mask: **Whole village frame plus scout/ambush/supply crops**. Explicitly out of scope: Full match economy, aircraft and finished art.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Ordinary disables Red spawn index 4; crossfire enables it. Verify the own-optics ambush trigger and that replay does not rerun the defender.

## Decision budget

Delegated: Authored patrol/defense micro within own observation; map offsets/tuning with recorded paired evidence. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If village is not replayably interesting, revise scenario/config or split the failing mechanic; do not declare full-game completion.
