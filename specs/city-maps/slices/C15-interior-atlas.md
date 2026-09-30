# C15: interior atlas

**Depends on:** G0. **Kind:** slice.

## Question
Is there a project-owned, dim, daylight-only interior atlas in the repo's 2×5 layout?

## Contract it unlocks
A numpy or Blender recipe renders rooms and shopfronts (ground floor) with no lamps, recorded as `project-owned` (Q-E).

## API seam
`packages/scene-assets` texture recipe.

## What the human can run or see
The atlas sheet.

## Verification
- `asset check`; provenance.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the atlas as a whole: "does any room read as lamp-lit or glowing?"**) against **the repo's atlases for **layout only**; never copy content**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** How it looks behind glass (C26).


## Delegated to the implementer
The room and shop content within 'dim, daylight, generic'. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No photos; no lamps.

## Feedback that would change this slice
An interior atlas that reads as repetition or a real brand changes its project-owned source, not facade geometry.
