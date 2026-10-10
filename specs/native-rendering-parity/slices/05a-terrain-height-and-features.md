# 05a — Terrain: height and features

**Contract:** native seats every layer on the browser's height function, and draws roads, plots, walks, forest floor and surfaces in the same places.

**Seam:**
- Already built (`5d6898ad`): the manifest's `terrain` holds hashed artifacts `{name, sha256, words, layout}`, with files `terrain-<name>.u32` beside the manifest. Layout kinds are `records` (field word offsets and types), `words` (with constants) and `height-pages`. They hold the packed words the browser already uploads, produced by the same packers: the height grid (`packTerrainHeights`), plot nodes, plots, surface records, surface index and `TerrainParams` (`terrainMaterial.ts:235-264`).
  - This avoids porting `plots.ts`, `surfaceField.ts` and `biome.ts`.
- **Native:**
  - `core/data_texture.gd` provides `words_texture(words) -> Texture2DRD`, an `R32_UINT` RD texture. It is the single owner of table uploads. Godot spatial shaders have no storage buffers, and `Image` has no u32 format.
  - `terrain/height_field.gd` provides `height_at(x, y)`, using the same triangle rule as `terrainGrid.ts`.
  - `terrain/ground.gdshaderinc` holds the ground-class formulas. Slice 06a includes it.
  - `terrain/terrain_layer.gd` builds the ground mesh.
- The flat ground plane, the centimetre layer offsets and the `max(0, z)` unit clamp are deleted (`reel.gd:291-404, 1168`).
- **Class view:** both clients draw a ground-class view of the same frames. The browser uses its existing class view (`battleFrame.ts:345-348`, decoded by `groundClasses.ts`); native adds a class branch to its terrain shader.

**Runnable artifact:** lit plate pairs plus class-view plate pairs from the same frames.

**Visual variable:** features in the same places.

**Focused gates:**
- `height_at` matches sample vectors exported from TS, including a non-flat test grid. Neither reel map has relief.
- Measured: class agreement over the ground pixels both clients drew is above a threshold the implementer sets and records, with an edge mask. This also proves seating and projection.
- Run `compare-screenshots` on the lit pairs, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** field character (05b), grass, scars (phase 2, as omission rows) and models.

**Delegated:** the agreement threshold and edge-mask width (recorded), and how the terrain mesh is tiled.

**Scaffolding:** the class-view capture and the native class branch are removed at the phase 1 closeout, unless phase 2 adopts them.
