# 05 — Own sensors and shared identification

**Status:** complete 2026-09-25. **Dependencies:** 02, 03, 04. **Milestone:** Village checkpoint.

## Contract and question

Does reconnaissance discover enemies through the agreed geometry and concealment rules?

User requirements owned or exercised: V01, V02, V03, V04, V07. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::sensing::evaluate(world, side) -> SensorEvidence`; `sim::knowledge` retains own-sensor and team identification separately.

## Runnable artifact

/lab/sensors: scout, infantry and tank at forest edge; move an enemy through thin/deep forest and behind a hill/building. Switch explicitly labeled side views; inspect detection distances and own-versus-shared sensor status only in lab.



## Verification and verdict

Range/layer bounds; edge infantry hidden sooner than vehicles; thick forest blocks; thin forest permits farther vehicle detection; hill/building LOS; shared sight extends a tank target but not its own sensor flag; partial squad sight exports only seen soldier poses. Fixed observation fixtures exercise absent aerial layers without implementing aircraft.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Visible versus obstructed ground**. Review crop/mask: **Fixed forest-edge crop and hillside occlusion crop**. Explicitly out of scope: Contact glow, audio and air units.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Spatial indexing, batched query scheduling if preserving the initial tick-level behavior. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Incorrect discovery distances or an unexplainable LOS result requires geometry/sensor correction.

## Verdict — 2026-09-25

Accepted. What shipped:

- **`sim::sensing::evaluate`** runs every tick. Exact rays go from the observer's eye to each target sample: vehicle hull centre and top, or each living soldier at 1 m. Solid geometry blocks the ray. Reach is `range × concealment × exp(−foliage / 120)`, and 120 m of foliage below the canopy blocks outright. Concealment is sampled per soldier, and a soldier outside the forest is exposed. Strength is continuous with depth (infantry `0.8 + d/100`, vehicle `(d − 15)/25`) and interpolates toward the class multiplier. The world gained `forest_path_length` (below-canopy, 3D) and `forest_depth`.
- **`sim::knowledge`.** Team identification with side-scoped `ObservedTargetId` handles that survive the 1.5 s grace and are retired after it. Records carry class, cost, observed position, heading and velocity, and only the soldiers actually seen, reported at their centroid. There is no HP, ammunition or order. Each own unit's `sees` list is its own sensor, kept apart from team sharing.
- **Ground visibility field** (`sim::visibility`). A radial line-of-sight sweep over true terrain, solid prop tops and foliage, on the fixture's 8 m cells. It is recomputed every 6 ticks per side (0.2 s bounded display lag; identification itself is per tick). Dynamic obstacles inside a side's visible ground become known to that side, completing slice 04's learn-by-sight follow-up.
- **Scripts.** Fixture `scripts` are authored, timed orders for either side, identical in live and replay runs.
- **Browser.** A layout-driven group decoder. Shader fog from a bitset storage buffer (16 bits per published float so every value is exact). Enemies are drawn from `identified` only. `/lab/sensors` has per-unit own-sensor lists, the team list with who sees each contact, and a labelled diagnostic side switch.

Tests:

- **10 native:** range bounds for recon, rifle and tank; soldiers a few metres inside the forest edge are concealed while a tank at the same depth is not; a vehicle is seen through 40 m of foliage while 160 m blocks; ridge and building occlusion with clear controls; shared identification without the tank's own-sensor flag; a partly hidden squad exports only its seen soldiers at their centroid; a hidden enemy moving leaves blue's serialized frame identical for 90 ticks; a brief loss of sight keeps the handle; a long one gets a new handle; the fog field is clear in front, fogged behind the ridge and beyond range.
- **5 browser checks:** drawn enemies equal the identified accounting; fog darkens ground behind the ridge by about 27 luminance while seen ground changes by 0; the scripted red tank reaches the ridge's far side, where it is neither listed nor drawn; the side switch is labelled and swaps the whole view.

Visual gate (visible versus obstructed ground):

- **Changed after the first critique.** Fog edges now blend the four nearest cells bilinearly instead of stepping cell by cell. Fogged ground keeps its shading (lighter, desaturated treatment). Unit hulls are lighter so side tints read. A blue squad was added on the hill's open flank, so the near slope shows clear and the far slope fogged; from the original viewpoint the hill sat behind 200 m of deep forest, correctly fogged but a poor demonstration.
- **Dispositions.** The "speck in the fog" is the 3 m wall prop, not a leak. Ground beyond the thin strip is correctly visible (40 m of foliage only shortens range). Darkening the canopy over fogged ground is intended, because the fog is keyed to ground position. The missing last-seen marker is slice 06.

The aerial layers stay absent. The observation carries no air contacts, and slice 18 adds them without changing this ground path.
