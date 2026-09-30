# C78: forest body models

**Depends on:** C77, C73. **Kind:** slice.

## Question
Do logs and boulders visibly fit their bodies?

## Contract it unlocks
Workbench appearances, fitted to their boxes: the log reuses C73's bark; the boulder comes from a new small rock generator (not `~/dev/game`'s pyramids).

## API seam
`packages/scene-assets/blender/`.

## What the human can run or see
A workbench sheet and `forest-deep-25`.

## Verification
- `asset check`; `fit.*`.
- "Models are good enough": one round of fixes for outright errors.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**body model read only**) against **`../assets/reference/ground/forest-road-rocks.jpg` and C77's box shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Dressing.


## Delegated to the implementer
Form details. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C77's behaviour.

## Feedback that would change this slice
A forest floor body that reads as impassable despite its physical properties reopens model/body fit.
