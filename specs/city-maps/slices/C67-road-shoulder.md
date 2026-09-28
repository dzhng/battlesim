# C67: road shoulder

**Depends on:** C66, SG3. **Kind:** slice.

## Question
Does a road fade into the field through a worn shoulder, with grass thinning across it, never an outline or a shadow (Q-G2)?

## Contract it unlocks
- A shoulder of 2–4 m per kind, with a one-sided noise-jittered edge. Its luminance is at or above the grass's and it differs by hue.
- Grass density ramps across it from C63's field, replacing the flat `clear_m.road` margin.
- **The road-keyed branch of `groundVerge` (`terrainMaterial.ts:420`) is deleted**, so the shoulder is the one "beside the road" owner.
- A technique-only manifest entry for Selo Empire's `RoadMeshBuilder` and `TerrainRoadWear`.

## API seam
`terrainMaterial.ts`, `grassPass.ts` density, biome road rows.

## What the human can run or see
`bend-65` with grass off, then on; `track-65`.

## Verification
- Per 0.5 m band, luminance is monotone or flat from core to grass and never dips below the grass band.
- Clump counts per 1 m band are monotone.
- The jitter is world-anchored.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the road-to-field transition band (0 < SD < 6 m) only; critique: "does the road read as an outline?"**) against **`../assets/reference/ground/manor-path-aerial.jpg`, `manor-path-closeup.jpg`, `forest-road-summer.jpg` and C66's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Ruts, grass species, field palette.


## Delegated to the implementer
Widths, noise scales, thinning curve. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C66's core pixels; `fog-look`.
