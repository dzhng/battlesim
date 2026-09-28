# C65: round centerlines

**Depends on:** C64. **Kind:** slice.

## Question
Do roads (and later rivers) follow one round curve in the sim, the plot cutter and the shading (Q-G19)?

## Contract it unlocks
- The contract's loader densifies authored strokes through a centripetal Catmull-Rom spline (no overshoot; passes through control points) to points ≤2 m apart, identically in native and wasm. Every consumer sees only densified points; authored data stays sparse.
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
