# C30: markings

**Depends on:** C28. **Kind:** slice.

## Question
Are crossings and lane lines legible at battle distance and never mistaken for tactical marks?

## Contract it unlocks
Deterministic, generic marking placements on roadbed.

## API seam
`packages/battle-renderer/src/terrain/`.

## What the human can run or see
An intersection crop at 65 m.

## Verification
- No sidewalk overlap; no interference with order or tactical ground marks.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**marking legibility only**) against **C28's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Everything else.


## Delegated to the implementer
Generic layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Pavement and curbs fixed.
