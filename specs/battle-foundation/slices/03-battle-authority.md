# 03 — Commands, observation transport and replay

**Status:** planned, not implemented. **Dependencies:** 01, 02. **Milestone:** Village checkpoint.

## Contract and question

Can one authority process commands and publish coherent player state independently of drawing?

User requirements owned or exercised: S01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`Battle::accept/step/observe`, WASM filtered exports, `web/src/battle/sim` worker/client and `Replay` from architecture.md.

## Runnable artifact

/lab/authority: own-unit movement stub driven by real tick commands, ordered ack log, pause/step/reset, replay comparison and consumer-credit control. Enemy fixture data remains absent until sensing produces permitted observations. Use the actual contract early; no all-truth presentation placeholder.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Same-build same-seed command replay tick digests; direct versus worker parity; no ticking behind loading cover; ordered burst/ack; invalid side/target rejection; withheld credit pauses instead of queuing/dropping observations; no use-after-transfer; scripted advance resolves on consumption; cancellation/disposal/failure terminate work.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Authority status honesty**. Review crop/mask: **Tick/ack/status panel and one moving proxy**. Explicitly out of scope: Final HUD design, combat and fog.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/03/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Replay includes both sides; disable controller/input generation while replaying and prove no double execution of bot commands.

## Decision budget

Delegated: Internal packing layout with Rust-exported descriptor; scheduler helper names. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If render stalls alter simulation outcomes or UI reads hidden truth, fix the boundary before continuing.
