# C85: field texture

**Depends on:** C84. **Kind:** slice.

## Question
Does each field carry ground texture inside it (furrows, clods, stubble) without cloud-shadow blobs?

## Contract it unlocks
Plot-oriented filtered texture in the terrain material, per crop. It's luminance-neutral and one-sided, and fades with pixel footprint like the rows (`terrainMaterial.ts:445-450`).

## API seam
`terrainMaterial.ts`, biome plot rows.

## What the human can run or see
`field-65/250`, grass off, then on at 250 m.

## Verification
- The `fog-look` check again.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**plot interiors with grass off**) against **`../assets/reference/ground/manor-ground-closeup.jpg`, the Broken Arrow farm frames and C84's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Palette, grass.


## Delegated to the implementer
Texture scales; contrast within the bound. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C84's mean colours.
