# C66: forest look

**Depends on:** C65. **Kind:** slice.

## Question
Does a forest read as dense but see-through-able from the battle camera (Q-G16)?

## Contract it unlocks
- A species mix per biome (weights; mostly one family per stand, with birch and dead snags sprinkled) and colour variation per tree.
- The canopy mostly closes at 65 m, but units and floor show through gaps.
- Keep the dapple.
- No near-black shade (the avoid-list).

## API seam
`scenery/placement.ts`, `fixtures/biomes/summer.json` (`trees`).

## What the human can run or see
The village woods at 65 and 250 m, with and without units inside.

## Verification
- Critique: "can you see the squad under the trees?" and "does the forest read as one dense wood?"
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the forest mass and its edge only**) against **the Broken Arrow forest masses in `../assets/reference/ground/ours-vs-refs-board.jpg` (row 4), with a looser target (Q-G16)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Floor dressing, grass.


## Delegated to the implementer
Mix weights; tint ranges. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule (C64).
