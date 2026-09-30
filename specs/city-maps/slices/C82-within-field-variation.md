# C82: within field variation

**Depends on:** C81. **Kind:** slice.

## Question
Does a field vary in colour, height and clumping instead of reading as a mown carpet (Q-G14)?

## Contract it unlocks
- **A growth row becomes a weighted mix of appearances**, and a clump picks its species by hash at build (`grassPass.ts:334-349`). This removes the one-appearance-per-row limit.
- World-anchored low-frequency height, density and colour fields.
- Colour varies in **hue and saturation at bounded luminance**, one-sided (no two-sided blobs, per the avoid-list).

## API seam
`grassPass.ts`, the biome growth rows.

## What the human can run or see
`field-65/250`.

## Verification
- Luminance spread inside a plot stays under a bound while hue spread rises.
- Retained roots are stable under zoom.
- A grass-build ms row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**one meadow plot's interior; critique: "could any patch read as cloud shadow?"**) against **`../assets/reference/ground/manor-ground-closeup.jpg`, `grass-b.jpg` and C81's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Crops, palette.


## Delegated to the implementer
Noise scales; amplitudes. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C81's species.

## Feedback that would change this slice
Patchiness that reads as random noise changes within-field variation while retaining field boundaries and height caps.
