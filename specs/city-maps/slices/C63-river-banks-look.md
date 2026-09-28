# C63: river banks look

**Depends on:** C62, C28. **Kind:** slice.

## Question
Do rivers meet the ground with a soft bank and no hard edge?

## Contract it unlocks
- The renderer shades **bed → wet bank → mud → grass** from C62's exported distance field.
- The water surface follows the centerline, with its existing ripple shading.
- Grass thins toward the bank.
- **Mud and wet bank stay at or above the grass's luminance, differing by hue** (L-G3).

## API seam
`frame/terrainMaterial.ts`, `worldMesh.ts` (water surface), `frame/grassPass.ts` density.

## What the human can run or see
`/lab/river` at 25, 65 and 250 m.

## Verification
- Terrain and water GPU ms.
- Critique: "could any dark region read as shadow, or shadow as fog?", "does the shore read as a hard edge?" and "does any bend or bank read as blocky or stepped?"

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the water edge plus 8 m of bank either side**) against **`../assets/reference/ground/river-108.jpg`, `river-112.jpg` (sunken-channel read) and `manor-ground-closeup.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Reeds and shore stones (later dressing), grass species.


## Delegated to the implementer
Band widths; colours. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Sim geometry.
