# 06 — Player observations and uncertain evidence

**Status:** planned, not implemented. **Dependencies:** 03, 05. **Milestone:** Village checkpoint.

## Contract and question

Can the player react to uncertain evidence without learning hidden truth?

User requirements owned or exercised: V02, V08, V09, V10, V11, V13. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::knowledge::observe(side) -> ObservationFrame` plus event-to-SoundCue projection; presentation consumes only this frame.

## Runnable artifact

/lab/contacts: firing behind a hill, last-seen fade, hidden movement, periodic firing, and audible approach. Add actual ground fog, uncertain red areas and synthesized categorized sound from quantized cues. Camera motion alone changes neither evidence nor audibility.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Metamorphic hidden-identity/health/order/motion mutations leave identical side payload until permitted evidence. A shot at empty ground reveals an area. Repeated shots do not produce independent position samples; stale marker never follows hidden movement. Clip tracers/muzzle effects; filter changed props/picking/HUD/AI and audio. Contact expiry and reidentification retire the right side-local record.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Identified/contact/unknown distinction**. Review crop/mask: **Same forest-edge crop as 05; contact and fade endpoints; audio cue transcript**. Explicitly out of scope: Artistic fog, photoreal atmosphere and final sound assets.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Glow color/shape and synthesized timbre within information/readability constraints. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

A player can pinpoint hidden motion or confuse contact with a real unit: reject and fix before targeting.
