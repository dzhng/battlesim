# 16 — Water surface and ground scars

**Contract:** native draws the browser's water surface and the side's learned scars (craters, scorch, tracks, trample), and leaves grass out of the places the browser does.

**Seam:**
- Port into `terrain/ground.gdshaderinc` and the water draw:
  - `waterSurface` and `waterNormal` from `packages/battle-renderer/src/frame/terrainMaterial.ts`, plus the world pass's water branch (`worldPass.ts`);
  - `groundScars` and `scarredSurface` with the scar texture (`frame/scarTexture.ts`), fed by `capture/learned_ground.gd`.
- Grass reads the same scars.
- The grass count gate's `scar-omission` allowance is deleted.

**Runnable artifact:**
- Plate pairs over tank tracks and craters on market-town.
- Water: neither reel map has any, so use a pair from a capture of a map with water. Run the capture scene against such a map, for example `river`, if its encounter suits the menu reel. Otherwise judge water by a TS-exported vector test of the surface formulas and say so.

**Focused gates:**
- The grass count gate holds at 10% in every cell with no scar allowance.
- `compare-screenshots`, then an unprimed `screenshot-critique`.
- Pass cost reported.

**Visual variable:** scars and water.

**Delegated:** the scar texture's upload path.

## Findings (2026-10-11)

**Contract as built.**
- The manifest's terrain group gains two tables. `scars` is the browser's `ScarParams` record as last written: the grid (`grid`, `pages.xy`) and the biome's scar look (`full`, `crater`, `ejecta`, `scorch`, `tracks`, `trampled`, `relief`, `grass`). Its cache fields (`pages.zw`, `clip`, `cacheBounds`, `defaultWord`) describe the browser's page cache, and native ignores them. `water` is one record per water-surface cell (`corner`, `z`, `size`), the cells `waterSurfaceMesh` lays (`rivers.ts` `waterCells`).
- Native keeps the side's learned ground as the browser's scar bytes per cell in one word table (`terrain/scar_texture.gd`): the browser's normalisation words, a directory word per 16-cell tile, and a 256-word page per marked tile. Only the tiles that changed are rewritten (`learned_ground.gd` `take_changes`). The run replays the ground at every record, before the frame draws.
- `ground.gdshaderinc` ports `groundScars`, `groundScarsSeen`, `scarredSurface`, `scarredNormal`, the exact RGBA8 filter (`scarFilterPosition`, `scarFilterQuanta`), `waterSurface` and `waterNormal`. The terrain shader applies the scars, and the grass build leaves clumps out, lays them over and tints them on them, as `grassPass.ts` does. The grass gate's `scar-omission` allowance and the learned ground's `bare_at` are deleted.
- With the `water` layer, `terrain/water.gdshader` draws the water cells, blended, reading depth and writing none, casting no shadow.

**Count gate, no allowance.** Control `2026-10-11T12-20-57-541Z` (slice 15's, which already carries `scars` and `water`), read through a scratch copy of its manifests without the `buildings`, `scenery` and `instanceCounts` keys, which native's reader didn't admit yet.
- Market-town: all 18 plates, 131 non-empty cells. Every cell is within 10%, and the worst is 0.07% (plate 5384, cell 10: 1,520 against 1,521).
- Falsified: the same run with `scars` left out of the layers stops at 5 blockers, at plates 2349, 6088, 6845 (×2) and 7551, with 14.6–18.3% too many clumps. These are the cells the allowance used to name.
- Paris-corner: no blockers. Its one plate has no grass in any cell.

**Plates.** Scar plates are 2349, 4954, 6845 and 7551: tank tracks through wheat and meadow, and worn tracks on a dirt road. No plate frames a crater close enough to read, so craters are judged only by the shared formulas and the count gate.
- `compare-screenshots`, mean absolute difference on each scar crop against the browser, without and then with scars: road tracks (4954) 6.69 → 0.34; meadow ruts (7551) 1.99 → 0.54; wheat track (2349) 7.65 → 5.58; grass arc (6845) 6.93 → 5.46. What remains on 2349 and 6845 is the browser's dust and smoke, which belong to slice 17.
- Unprimed `screenshot-critique`: on all four pairs native has the same marks in the same place, path and width, and the same flattening. It found no artifacts. Its low-confidence notes: 7551's ruts look slightly redder in the browser, and native's 4954 road is a touch softer.

**Water: judged by vectors only.** Neither reel map has water. The capture scene can't film one: `?scenes=`/`PARITY_SCENES` only filter `fixtures/menu-backdrop.json`, whose maps must be in the `menu` category, and `river` is `test` with no authored shots. Filming it would take new game data or an alternative workload, which nobody asked for.
- `tests/water_vectors_display_test.gd` compares `water_look` and `water_normal` with the browser's own `waterSurface` and `waterNormal`, resolved by TypeGPU and run on WebGPU, with `groundWater`'s depth fed in. It covers 210 points (six positions, five footprints, seven depths, under the fixture's params), and every component is equal. It goes red on a one-digit change to a noise offset.
- The water draw itself is checked only headless (`terrain_layer_test.gd`: the cells read and lie flat at their height). It has never been shown on a plate.
- Approximation: native keeps Godot's own light on water in shadow. The browser multiplies by `water.shadow`, which needs Godot's `light()` and would replace its ambient. This is reported as the budget row `water.shadow`.

**Pass cost.** Release export, market-town whole reel, plugged in, window minimised, 3 runs each, interleaved, against the same control with and without `scars`:

| | avg FPS | p50 ms | p95 ms | p99 ms | CPU p50 ms |
| --- | --- | --- | --- | --- | --- |
| without scars | 111.9 (111.1–113.6) | 8.26 | 14.55 (13.98–14.96) | 15.60 (14.97–16.08) | 1.65 |
| with scars | 108.3 (106.4–108.5) | 8.24 | 15.96 (15.87–16.53) | 16.84 (16.75–17.40) | 1.74 |

- Scars cost about 1.4 ms at p95.
- Traced GPU busy per traced frame, 3 traces each at 16, 32 and 48 s: with scars 6.61 ms (5.71–7.07), without 6.49 (5.21–7.01). That is within the traces' spread, so traces don't resolve it.
- Slice 12's browser bar is p95 23.9, p99 25.4, 71.6 FPS and 10.8 ms GPU busy. Native stays well under it.
- The exports ran from the shared tree while slices 14 and 15 were in progress, so the absolute numbers aren't slice 12's. The pair is interleaved and differs only in the layer.

**Scratch:** `throwaway/slice16/` holds the control copies, native runs, sheets, critique crops, the vector exporter, and `perf/`.
