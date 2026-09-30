# C69: rivers contract

**Depends on:** C65, C60, SG2. **Kind:** slice.

## Question
Is water a round river that the sim and shading read from one distance, impassable and crossed at bridges (Q-G5)?

## Contract it unlocks
- `MapDefinition.rivers: Vec<River { points: Vec<{ xy, width_m, depth_m }>, surface_z }>`, densified by C65's loader. **`water` is deleted on every map.**
- `HeightField::build` carves a gentle bank from distance, with a ramp at bridge ends.
- The sim classifies water by distance through C03's `SurfaceIndex`, replacing the rect scans at `world/mod.rs:213, 221`.
- Validation rejects a point under 12 m wide, a surface above its bank, or a bank steeper than GG's slope.
- The export carries river segments, C63 gains the water channel, and the water surface becomes a ribbon along the centerline (`worldMesh.ts:136-172` rects go). There's no new look yet.
- Plots cut along river control runs.
- A new catalogued map, `river` (lab), with a meander, a bridge, the 12 m minimum, a 30 m section, a country road, a dirt track and stations.
- **Named lab changes:** the `geometry` and `movement` labs' rects become straight rivers. The village has no water.

## API seam
`contract::map`, `sim::world::{mod.rs, terrain.rs}`, `world/export.rs`, `worldMesh.ts`.

## What the human can run or see
`/lab/river`: a nav PNG and a GIF of a squad and a tank routing over the bridge and never entering the water.

## Verification
- Run tweak-mechanics first.
- Tests: exact edge classification; bank slope under `slope_cutoff_deg` outside the water; the bridge ramp stays traversable; validation refusals; native equals wasm.
- Named lab digests; village digests identical.

## Delegated to the implementer
Bank profile; ramp length; per-river `surface_z`. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village; bridge deck behaviour.

## Feedback that would change this slice
A river/bridge that blocks an intended route reopens the shared physical contract before bank appearance work.
