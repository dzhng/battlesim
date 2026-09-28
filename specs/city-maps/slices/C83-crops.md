# C83: crops

**Depends on:** C82. **Kind:** slice.

## Question
Is farmland a patchwork of distinct crops, every one ≤0.9 m, with no false buff (Q-G10, Q-G11)?

## Contract it unlocks
Presets for wheat, barley, rapeseed, hay and stubble, pasture, and ploughed or fallow (no growth). Each plot kind picks its crop in the biome, within the 15 plot growth rows. No maize; visual only.

## API seam
The biome's `plots` and `grass`.

## What the human can run or see
`field-65` and a 25 m station per crop.

## Verification
- The C80 height validator passes.
- A grass ms row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**each crop's plot, at uniform tint; critique: "does any crop look tall enough to hide a standing soldier?"**) against **battle-look's `brokenarrow/gameplay-trailer-08.jpg` and C82's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Palette, ground texture.


## Delegated to the implementer
Crop weights. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No sim effect.
