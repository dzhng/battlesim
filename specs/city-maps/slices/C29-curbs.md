# C29: curbs

**Depends on:** C28. **Kind:** slice.

## Question
Do curbs give sidewalks their edge without cracks?

## Contract it unlocks
Presentation curb geometry along sidewalk boundaries; no navigation step mechanic.

## API seam
`packages/battle-renderer/src/terrain/` or the static chunks.

## What the human can run or see
A grazing street-edge crop.

## Verification
- Joins, intersections, slopes, no cracks.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**curb relief only**) against **C28's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Markings.


## Delegated to the implementer
Curb profile as data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Movement on authoritative terrain.
