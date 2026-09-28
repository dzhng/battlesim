# C62: rivers contract

**Depends on:** C03. **Kind:** slice.

## Question
Can rivers meander, with carved banks, as one distance field the sim and the renderer share (Q-G5)?

## Contract it unlocks
- `MapDefinition.rivers: Vec<River { points: Vec<{ xy, width_m, depth_m }> }>` **replaces `water` rects on every map.**
- The sim carves a sloped bank into the height grid, and `SurfaceKind::Water` comes from distance to the centerline. Water stays impassable and is crossed at bridges only.
- Map validation: **width ≥ 12 m** (3 cells on the 4 m grid); a bridge's approach gets a ramp under the slope cutoff.
- `geometry-lab` and `movement-lab` migrate their rects to rivers (**named** lab outcome change, L-G8).
- A new `/lab/river` map (catalogued as `lab`, C60) with a meander, a bridge and a bend.
- **Round, never blocky (Q-G19):** centerlines are splines densified to ≤2 m points by the contract's loader, so classification and shading follow one round curve. Carves stay gentle so the 4 m flat-shaded facets don't read.

## API seam
`contract::map`, `sim::world` (terrain carve, surface classification), `world/export.rs`.

## What the human can run or see
`/lab/river`: a nav PNG and a `movement_shots` GIF of a column crossing the bridge.

## Verification
- Native tests: bank slope; classification at the edge; the bridge ramp stays traversable; validation rejects a river under 12 m.
- Village digests unchanged (no water).
- Lab changes named.

## Delegated to the implementer
Bank profile shape; carve sampling. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Bridges; village.

## Feedback that would change this slice
Narrow creeks wait for a finer height grid (a later spec).
