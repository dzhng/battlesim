# C70: crops

**Depends on:** C69. **Kind:** slice.

## Question
Do farm plots grow distinct crops, all ≤0.9 m (Q-G10, Q-G11)?

## Contract it unlocks
- Crop kinds: wheat, barley, rapeseed (yellow), hay/stubble, pasture, ploughed/fallow soil (no grass). **Every crop ≤0.9 m** (today's 0.95 m wheat is trimmed); no maize.
- The biome maps each plot kind to a crop and palette.
- Visual only.

## API seam
The biome `plots` and `grass` sections, `grassPass.ts`.

## What the human can run or see
A farm patchwork at 65 and 250 m.

## Verification
- Grass ms.
- Heights ≤0.9 m checked by a test.
- Critique: "does any crop look like it hides a soldier?"

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**one plot of each crop**) against **battle-look's Broken Arrow frames (`specs/done/battle-look/assets/reference/brokenarrow/gameplay-trailer-08.jpg`)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Field palette and texture (C71).


## Delegated to the implementer
Crop colour ramps. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No sim effect.
