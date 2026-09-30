# C71: river bank roundness

**Depends on:** C70. **Kind:** slice.

## Question
Do banks read round, with the 4 m flat-shaded facets hidden (Q-G19)?

## Contract it unlocks
On bank pixels the shading normal comes from the river's analytic bank profile, not the facet (SG2's fallback 2, if GG took it). Normals only; the sim's triangles are untouched. The tilt never exceeds 40° (the avoid-list).

## API seam
`terrainMaterial.ts` bank shading.

## What the human can run or see
`bend-25`, and a low-pitch shot along the meander.

## Verification
- The maximum facet luma step is under GG's bound.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the bank band only; critique: "does any river or bank read as blocky or stepped?"**) against **C70's shot (facets visible)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Colour bands, water.


## Delegated to the implementer
Blend width. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C70's colour masks.

## Feedback that would change this slice
A visibly angular shoreline changes bank geometry sampling; water colour and shading remain separate owners.
