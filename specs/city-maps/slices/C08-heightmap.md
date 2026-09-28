# C08: heightmap

**Depends on:** C04 (cut candidate). **Kind:** slice.

## Question
Does imported elevation form one physical surface?

## Contract it unlocks
`Relief::Heightmap { origin, cell_m, cols, rows, heights_m }` from the NYC DEM or USGS 3DEP (a SOURCES.json entry), as a binary sample file beside `map.json`. It's consumed by `HeightField::build` (`terrain.rs:33`) with the existing triangle convention. Buildings gain `base_z`.

## API seam
`contract::map`, `sim::world::terrain`.

## What the human can run or see
An elevation probe comparing samples, slope, ray hit and exported triangle height.

## Verification
- Boundary and no-data handling.
- Native and wasm agreement.
- Village terrain and digests unchanged.

## Delegated to the implementer
Resampling; the no-data policy (documented). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village terrain.

## Feedback that would change this slice
**Cut** if the crop's relief is under about 2 m.
