# C45: street models

**Depends on:** C44, C11. **Kind:** slice.

## Question
Does each prop's model fit its body?

## Contract it unlocks
Appearances:
- theirs, from C11's standalone street kit;
- ours: cars, wrecks, Jersey barrier, bus shelter and scaffold, generic with no brands or plates;
- our existing solid-crown trees.

Each has LODs and its terminal state.

## API seam
`packages/scene-assets` appearances.

## What the human can run or see
A contact sheet per kind.

## Verification
- `asset check`; provenance; fit validation.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**body/model fit only**) against **the kind's body box overlay**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Placement density.


## Delegated to the implementer
Model details within 'generic'. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C44's body values.
