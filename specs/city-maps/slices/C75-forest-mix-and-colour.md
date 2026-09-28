# C75: forest mix and colour

**Depends on:** C72, C74. **Kind:** slice.

## Question
Does a wood read varied only in shape and colour: mostly one family per stand, plus the odd birch and snag?

## Contract it unlocks
The biome's `trees.species` holds weights and tints over all kinds. Snags are ≤5% and stay out of a forest's outer ring and out of strips. Placement stays one tree per sim trunk (`placement.ts:262-265`).

## API seam
`fixtures/biomes/summer.json` (`trees`), `scenery/placement.ts`.

## What the human can run or see
`forest-edge-65` and `patchwork-1100`.

## Verification
- The placement test still gives one tree per trunk.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the forest canopy seen from above; critique: "does any darker crown read as cloud shadow?"**) against **battle-look's WARNO forest frames, `../assets/reference/ground/forest-road-summer.jpg` and C74's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Density read, floor.


## Delegated to the implementer
Weights and tints. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule.
