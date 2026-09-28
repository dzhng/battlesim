# C63: surface distance field

**Depends on:** C03, SG5. **Kind:** slice.

## Question
Do terrain and grass read every ground rule (road, forest, river) from one exact-enough distance structure, with the per-fragment loops gone?

## Contract it unlocks
- **Owner:** `terrain/surfaceField.ts`, built from C03's export with SG5's chosen structure (a signed-distance bake or a segment-bucket index). It uses **the same distance function as the sim**.
- It is a signed distance, **never a class or coverage mask**.
- `groundSite` and `groundWater` sample it, and the loops at `terrainMaterial.ts:236-243, 371-380, 385-394` are deleted. Grass reads it through `groundSite`.
- **C28 is reordered after this slice** and reads this field (its pavement bake becomes a consumer, not a second owner).

## API seam
`packages/battle-renderer/src/terrain/surfaceField.ts`, `terrainMaterial.ts`, `grassPass.ts`.

## What the human can run or see
The village, pixel-equivalent before and after.

## Verification
- Village terrain and grass frames compared before and after with compare-screenshots (must be unchanged within tolerance).
- Export-to-field agreement ≤2 cm at 10k random points, including at joins.
- Bytes; a frame-cost row.
- Digests untouched.

## Delegated to the implementer
Resolution and packing within SG5's verdict. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The village look; the `fog-look` and ground scenes.
