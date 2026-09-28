# C28: pavement

**Depends on:** C03, C63. **Kind:** slice.

## Question
Do roadbed and sidewalks cover exactly the right ground and never read as fog or shadow?

## Contract it unlocks
City pavement reads **C63's surface distance field** (the one owner of distance to roads, forests and water; it already replaced the per-fragment loops). This slice adds the city surface kinds (roadbed, sidewalk) and their palette; it bakes nothing of its own.

## API seam
`packages/battle-renderer/src/terrain/`.

## What the human can run or see
A top-down crop of a block, and the village terrain before and after.

## Verification
- Surface and export agreement.
- Texture ≤16 MB.
- Terrain GPU ms.
- Village terrain compared (must be unchanged within tolerance).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**pavement coverage only: "does any road read as fog or shadow?"**) against **an aerial reference crop of the real block (never shipped), and the village's pre-change frame**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Curbs (C29), markings (C30), the city biome (C31), props.


## Delegated to the implementer
Palette; texture resolution. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Terrain height; speed classification.
