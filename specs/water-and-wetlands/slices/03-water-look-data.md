# 03: Water look data and variant picks

**Lane:** render. **Depends on:** 02.
**Visual variable:** none. Both river looks start equal to today's look.
**Status:** done (2026-10-28).

## Contract

- **Biome:** `water.rivers` is a list of looks, each `{name, weight,
  palette, opacity, shallows_m, streak}`. Summer has two, temperate (weight
  3) and murky lowland (1), both today's look. A biome holds at most
  `WATER_LOOKS_MAX` (8) looks across every kind of water, and each kind
  needs one look with weight.
- **The pick:** `presentation::static_world::ground::water::look(seed,
  river, looks)` gives a river's look index, seeded from the biome's seed and
  the river's index alone, as tree stands are seeded per cell
  (`crate::seed::{stream, weighted}`). A look of no weight is never picked.
- **The export:** the river stride is 12 floats. The last field, `river`,
  is the stretch's index in `MapDefinition.rivers`. Readers find fields by
  name (`riverFields`).
- **Water cells:** `Ground::water_cells` returns `WaterCell`s of five floats:
  corner, surface height, side and look. A junction cell takes the first
  river's. `mesh::water` carries the look in its vertices' red.
- **The GPU:** `TerrainParams` ends with `shoreTuning` (the bank's bands and
  ground, from `presentation.terrain.shore`) and `waterLooks`, eight
  `WaterLook` rows (`frame/waterLook.ts`). Each row is a biome look over
  the shared surface (`presentation.terrain.water`). The water's fragment
  and its surface material (`frame/waterSurface.ts`) read its cell's row.
  No water or shore number is spliced into shader text any more. The
  browser refuses a table whose length differs from `TerrainParams`.
- **Live tuning:** on the dev server, an edit to `fixtures/biomes/summer.json`
  reaches a battle's ground in place (`useGameBiome` in `gameBiome.ts`),
  with no shader rebuilt. `?water_look=<row>` pins one look on every river.
  An edit to `game.json` still reloads the page. A battle's world layers
  have their own hook (`useWorldLayers.ts`), which takes the live biome.

## What changed from the plan

- The shore's tuning is one shared record. The terrain draws the bank
  without knowing which river it borders.
- The pick takes the river's index, not a feature; slice 13 calls it with
  the swamps' looks.
- The reload rides the dev server's hot update of the biome's file. The map
  workbench's protocol serves its own server, not a battle page.

## Verification (done)

1. The seam tests in `crates/presentation/tests/water_looks.rs` pass, and
   each was seen to fail with its rule broken:
   - a river's look depends only on the seed and the river's index: the same
     on a map with its other features reordered and one added (a swamp
     joins this test when slice 10 merges);
   - a look of no weight is never picked.
2. `cargo test -p presentation` passes. The ground-mesh vectors were
   re-recorded for the cells' look column (named in `choices.md`). One
   unrelated failure was there before this slice: the paris-corner poses
   drift, fixed in its own commit.
3. The curve-strokes parity oracle was blessed with the river index column
   alone.
4. The shader export and the `crates/pipelines` tests pass.
5. The `water` and `river` scenes pass, native's draw check included, with
   the same water-against-bank readings on both clients as before. The
   `river` stills against slice 02's differ only in the FPS counter's
   corner.
6. `?water_look=` and the live reload were not driven in a page. The scenes
   ran without either, and with no console error from the hot-update hook.

## Native

Free: native uploads Rust's terrain params and water mesh as they are.

## Must stay green

Battle digests and replays. All shots, exactly.
