# C81: wild grass

**Depends on:** C80. **Kind:** slice.

## Question
Do wild-ground species (short meadow, tall rough, dry prairie, weedy verge) read as distinct at the default camera?

## Contract it unlocks
Biome growth rows point meadow, pasture, verge and rough ground at C80's presets. The grass kind and growth-row caps (16 and 16) hold.

## API seam
The biome's `grass.growth`, `grassPass.ts`.

## What the human can run or see
`field-65` and `bend-25`.

## Verification
- Grass ms and clump counts; a frame-cost row.
- **Kill if** it's >+0.4 ms over today's 1–1.2 ms.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**a single-species meadow plot**) against **`../assets/reference/ground/grass-a.jpg`, `manor-ground-closeup.jpg` and C62's baseline**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Crops, palette, within-field variation.


## Delegated to the implementer
Preset parameters. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Grass exclusions.

## Feedback that would change this slice
Grass silhouette that appears taller than accepted cover changes species/placement height within the shared cap.
