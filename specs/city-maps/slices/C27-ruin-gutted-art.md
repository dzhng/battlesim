# C27: ruin gutted art

**Depends on:** C14, C42, C43. **Kind:** slice.

## Question
Do ruin and gutted buildings read as the same building, switching when the side learns of it?

## Contract it unlocks
C14's placements are drawn on the published collapse or gutting, via the knowledge path (`KnownProp.replaces`). The far tier has matching ruin and gutted variants.

## API seam
`propAppearance.ts` state switch, C22's chunks.

## What the human can run or see
`/city` frames of a collapse and a gutting, plus the replay picture.

## Verification
- No visual collapse of a still-standing collider.
- Knowledge isolation: the other side sees the old state until it learns.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**terminal silhouette and burnt read only**) against **C22's intact shot of the same buildings**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Street props, fog.


## Delegated to the implementer
Burnt treatment. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Sim owns timing and dimensions.

## Feedback that would change this slice
Terminal appearance that hides a standing gutted body or suggests intact cover reopens terminal art/fit.
