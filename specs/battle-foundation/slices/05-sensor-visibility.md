# 05 — Own sensors and shared identification

**Status:** planned, not implemented. **Dependencies:** 02, 03, 04. **Milestone:** Village checkpoint.

## Contract and question

Does reconnaissance discover enemies through the agreed geometry and concealment rules?

User requirements owned or exercised: V01, V02, V03, V04, V07. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::sensing::evaluate(world, side) -> SensorEvidence`; `sim::knowledge` retains own-sensor and team identification separately.

## Runnable artifact

/lab/sensors: scout, infantry and tank at forest edge; move an enemy through thin/deep forest and behind a hill/building. Switch explicitly labeled side views; inspect detection distances and own-versus-shared sensor status only in lab.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Range/layer bounds; edge infantry hidden sooner than vehicles; thick forest blocks; thin forest permits farther vehicle detection; hill/building LOS; shared sight extends a tank target but not its own sensor flag; partial squad sight exports only seen soldier poses. Fixed observation fixtures exercise absent aerial layers without implementing aircraft.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Visible versus obstructed ground**. Review crop/mask: **Fixed forest-edge crop and hillside occlusion crop**. Explicitly out of scope: Contact glow, audio and air units.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/05/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Spatial indexing, batched query scheduling if preserving the initial tick-level behavior. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Incorrect discovery distances or an unexplainable LOS result requires geometry/sensor correction.
