# C68: forest dressing

**Depends on:** C22, C65. **Kind:** slice.

## Question
Is the forest floor dense with ferns, bushes, saplings, small rocks and litter, drawn cheaply (Q-G12)?

## Contract it unlocks
- Presentation-only workbench models (Blender scripts) for ferns, bushes, small rocks, litter clumps and saplings, scattered inside forests with a seeded rule.
- They draw through **C22's static chunks** (L-G5) and fade out by projected size.
- They're hidden where fog hides ground, and are never bodies.

## API seam
`packages/scene-assets/blender/` (dressing kit), `scenery/placement.ts`, C22's chunks.

## What the human can run or see
The village woods at 25 and 65 m.

## Verification
- Frame-cost row within budget.
- `asset check`.
- Critique.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**forest floor between trunks only**) against **`../assets/reference/ground/forest-road-rocks.jpg` and `forest-road-summer.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Trees, grass outside forests.


## Delegated to the implementer
Kit contents within the list; densities. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No sim effect.
