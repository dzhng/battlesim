# C70: river bank bands

**Depends on:** C69. **Kind:** slice.

## Question
Does the river meet the ground through soft bed, wet-bank, mud and grass bands, with no hard edge and no dark shore?

## Contract it unlocks
- Bands by signed distance with a one-sided noisy edge (Selo Empire's shore distance-field technique, as a technique entry).
- The waterline meets the carved bank at the exact half-width line, with no quad overhang.
- Wet bank and mud stay **at or above grass luminance**, differing by hue.
- Grass thins across the mud band.

## API seam
`terrainMaterial.ts` (`groundShore`, `groundWater`), `worldPass` water.

## What the human can run or see
`/lab/river` at `bend-25/65`.

## Verification
- Luminance per band ≥ grass.
- The `shadow_floor` scene check.
- No depth-fight flicker.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the water-to-grass band (0 < SD < 8 m) only; critique leads with the shadow question**) against **`../assets/reference/ground/river-108.jpg`, `river-112.jpg`, `manor-ground-closeup.jpg` and C69's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Bank facet roundness.


## Delegated to the implementer
Band widths, palettes, noise. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C69's geometry and agreement.
