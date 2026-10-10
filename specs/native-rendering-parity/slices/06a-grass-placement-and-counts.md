# 06a — Grass: placement and counts

**Contract:** native grows grass where the browser does. Hashed sample positions are bit-exact for fixed tiles, and clumps per screen cell are within about 10% of the browser's count for the same frame.

**Seam:**
- Lift `grassPass.ts`'s constants (`GRASS_JITTER_M`, the lean/sink/wear constants, `BLADE_ROUGHNESS` and the gust/flutter terms) into `packages/battle-renderer/src/terrain/grassTuning.ts`. The generated WGSL stays byte-identical.
- Already built (`b726a9ca`, `948dc7fd`): the manifest's `grass` holds `{tables, constants, kinds}`. The tables are `params`, `props`, `shapes`, `indicesNear` and `indicesFar`, as hashed `grass-<name>.u32` artifacts, exactly as the pass uploads them. `tuning.grass` is the tuning table.
  - Native recomputes `params.planes[0..3]`, `eye` (w = 2·tan(fovY/2)/canvas height), `window.xy` and `grid.xy` from each frame record. The rest of `params` is fixed for the map.
  - Already built (`5d6898ad`): `grassCounts: [{plate, cells:[12]}]`, a 4×3 screen grid, row-major from the top-left. Each cell counts the clumps whose root projects onto the screen, occlusion ignored, read from the browser's probe in the same frame as the plate.
  - The grass tuning table already exists (`8b101db5`).
- **Native:** a GLSL build shader includes `terrain/ground.gdshaderinc` and ports the tile/clump sampling and `pcgHash` with 32-bit integer maths.
  - Start with a probe of a compute shader writing the MultiMesh buffer (`multimesh_get_buffer_rd_rid`).
  - If compute-written instances don't render in 4.7.2, fall back to the browser's own approach: a fixed-count MultiMesh that drops non-grown blades in the vertex stage (`grassPass.ts:569-572`). Counts then come from a GPU counter.
  - The grass window is sized in metres per pixel at the record's canvas height (`grassField.ts:288-302`).
- Delete the native target-near grid and the 512 cap.

**Runnable artifact:** a count table per plate frame (browser and native per region) and broad and close plate pairs.

**Visual variable:** coverage, judged by the counts first, then by eye.

**Focused gates:**
- Display-backed (a local rendering device needs a display): hash words and admission decisions for fixed tiles equal vectors exported from TS.
- Clump counts within about 10% in every cell, counted the same way: root on screen, occlusion ignored.
- The WGSL byte-identity scratch check for the lift.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** blade look, wind, colour mix (06b) and scars (phase 2).

**Delegated:** which build path, compute or vertex-stage, decided by the probe and recorded.
