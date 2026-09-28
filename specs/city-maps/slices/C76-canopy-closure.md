# C76: canopy closure

**Depends on:** C75. **Kind:** slice.

## Question
Does a forest read dense but let you see units and the floor beneath (Q-G16)?

## Contract it unlocks
Crown scale is tuned so the canopy mostly closes. **The crown radius never exceeds the sim's `canopy_radius_m`.**

## API seam
`summer.json` (`trees.forest`), `placement.ts`.

## What the human can run or see
The default camera and 120 m, with one test squad inside.

## Verification
- The visible floor share from a models on/off pair.
- The test squad's torsos visible above a threshold (the `torsos` helper in `_battleLook.mjs`).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**forest interior at 65 m; critique: "can you see the soldiers under the trees?"**) against **battle-look's Broken Arrow forest frames (target: looser than theirs) and C75's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Floor dressing, bodies.


## Delegated to the implementer
Crown scale within the radius. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule.
