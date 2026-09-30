# C24: cutout

**Depends on:** C21, C22. **Kind:** slice.

## Question
Does alpha-cutout keep its coverage through colour, depth and shadow?

## Contract it unlocks
Cutout coverage through the model colour, depth-prepass and shadow paths (`MATERIAL_ROWS` grows, `modelLayer.ts:129-132`).

## API seam
`packages/battle-renderer/src/models/modelLayer.ts`.

## What the human can run or see
A grille and sign fixture.

## Verification
- Matching colour and shadow silhouettes; mip behaviour; MSAA edges; fog-mask behaviour.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**cutout edges only**) against **the same fixture in Blender (S2 driver)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors.


## Delegated to the implementer
Cutoff values as material data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Opaque wear unchanged; no leaf-card street trees.

## Feedback that would change this slice
Cutouts that alias or vanish change coverage thresholds/mips; glass and interiors stay separate.
