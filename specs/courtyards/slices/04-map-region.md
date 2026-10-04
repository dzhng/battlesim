# 04 The map names its region; paving takes the region's finish

**Unlocks:** one owner of the map's region for the renderer and the loader; regional ground.

**Seam (contract change):** `contract::map::MapDefinition.regional_family: Option<String>` (absent only on a map without buildings), set by mapgen from `parcels::family()`; map validation refuses a building whose `regional_family` differs. The renderer's world layout carries it. The biome's `paving` row gains `families: { china, new_york, paris }` overrides, resolved by one function in `terrain/biome.ts`; `roadLooks` reads the map's family.

**Verify:** contract tests (mismatch refused, round trip); saved maps in `fixtures/maps/` re-saved with the field; parity re-blessed if map bytes move. Shots per region at `town-120-low` and `court-a-30-low`; screenshot-critique; compare-screenshots against slice 03's shots.

**Delegated:** palettes and slab sizes per region, within China concrete, New York concrete slabs, Paris pale stone.
