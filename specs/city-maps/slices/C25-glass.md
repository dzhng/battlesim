# C25: glass

**Depends on:** C24. **Kind:** slice.

## Question
Does glass composite correctly and affordably?

## Contract it unlocks
Alpha-blended model surfaces in the existing frame owner, with an explicit ordering and depth policy. No refraction and no emission.

## API seam
`packages/battle-renderer/src/models/`, the frame's pass order.

## What the human can run or see
An overlapping-windows fixture, and the China enclosed balcony at 30 m.

## Verification
- Opaque occlusion, both sides of glass, fog boundaries.
- Overdraw and GPU ms within G0's budget.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**transparency composition inside window frames only**) against **a Blender render of the China enclosed-balcony module at 30 m (S2 driver, our atlas). The user's own reference shot of that balcony was shared only in chat; if they drop it into `specs/city-maps/assets/reference/`, add it as a second target**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** What's behind the glass (C26).


## Delegated to the implementer
Sort granularity (bounded by S3). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No new transparency framework beyond this.
