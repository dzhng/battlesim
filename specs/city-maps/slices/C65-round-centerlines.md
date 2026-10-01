# C65: round centerlines

**Depends on:** C64. **Kind:** slice.

## Question
Do roads (and later rivers) follow one round curve in the sim, the plot cutter and the shading (Q-G19)?

## Contract it unlocks
- The contract's loader densifies authored strokes through a bounded local curve (passes through control points; stays within the admitted corridor) to points ≤2 m apart, identically in native and wasm. Every consumer sees only densified points; authored data stays sparse.
- **The plot cutter (`terrain/plots.ts:101, :219`) cuts along the control polyline's runs**, which the export carries too, so fields still meet roads edge-on.
- Trunk clearance from roads (`world/forest.rs:150`) follows the round curve.
- **Named village digest change:** the corners at (420,420) and (1150,420) round, moving road cells and nearby trunks. The movement lab's right-angle road moves too; list it.

## API seam
`contract::map` loader, `world/export.rs` (densified plus control points), `terrain/plots.ts`.

## What the human can run or see
The village bend at `bend-25/65`, and a nav PNG of the corner.

## Verification
- Run tweak-mechanics first (a convoy rounding a smooth corner).
- Tests: spacing ≤2 m; passes through control points; native equals wasm; plot count within ±10% and road-side plot edges within 5° of the run.
- `village_report -- --quick --compare main`; the move is recorded in `decisions.md`.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**road-edge band |SD| < 1 m only; road colour and feather stay as today**) against **C62's baseline at `bend-25/65`, and `../assets/reference/ground/manor-path-aerial.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Road colour, shoulder, ruts, grass.


## Delegated to the implementer
The spline family, if centripetal fails the overshoot test. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Other labs with straight roads.

## Feedback that would change this slice
A curve that differs between physics and pixels reopens shared sampling parity before road/river consumers tune it.


## Outcome

**Physical half done; the visual gates wait for the specialist.** `contract::curve::Centerline` rounds each authored corner with a pair of cubic Béziers that meet at the control point. The line still passes through every control (so junctions stay where they were authored), never strays more than half the stroke's width from its authored runs, and takes at most a third of either adjacent run. Roads, forest strokes and (later) rivers all load through it, so the sim, the exports and the drawing share one line.

- Only bends are sampled, at most 2 m apart. A straight stretch stays one segment: see `choices.md`.
- The export carries the rounded strokes and, separately, the road strokes' control runs (`export_surface_runs`); `terrain/surfaces.ts` cuts fields along the runs.
- Native and Wasm produce the same sample bits (`fixtures/parity/ground/curve-strokes.json`).
- Named changes: the village's corners at (420,420) and (1150,420) and the labs' road corners round (the village's two roads go from 5 segments to 101). `village_report --quick`: one of six trials changes digest; outcomes and losses are the same.
- A stroke is round only at its bends. Its two ends are cut square across the first and last control point ("Road ends" in `choices.md`), so the rounded line's first and last sample each end a flat face, not a half-disc.
- Open: the visual verification list above, and the bend's look. Passing through the corner makes the road swing about 2 m to the outside of the turn rather than cut the inside.
