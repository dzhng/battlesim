# C79: forest dressing

**Depends on:** C76, C78, SG1. **Kind:** slice.

## Question
Is the forest floor full of ferns, bushes, saplings, small rocks and litter within budget, without false cover?

## Contract it unlocks
- A third population in `scenery/placement.ts`, **drawn by the existing scenery layer's chunks** (`frame/sceneryLayer.ts`, `scenery/lod.ts`), not C22.
- Seeded per forest, kept off trunks, bodies, roads and cleared lanes. Everything ≤0.9 m; rocks ≤0.5 m.
- Far chunks leave it out, and it fades by pixel size within GG's radius. Presentation-only.

## API seam
`scenery/placement.ts`, `sceneryLayer.ts`, the biome's `forest_floor.dressing`.

## What the human can run or see
`forest-deep-25` and `forest-edge-65`, crowns off, then on.

## Verification
- Instance count and bytes; a frame-cost row; benchmark worst window.
- A test that nothing is placed outside forests.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the forest floor below 2 m; critique: "does any bush look like it hides more than the forest does?" and the shadow question**) against **`../assets/reference/ground/forest-road-summer.jpg`, `forest-road-rocks.jpg` and C78's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Canopy.


## Delegated to the implementer
Mix and densities (density is the cut knob). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C76's frame budget; the forest-floor fog.

## Feedback that would change this slice
Dressing that spills outside real forests changes distribution; a performance failure follows the stated density/cut order.
