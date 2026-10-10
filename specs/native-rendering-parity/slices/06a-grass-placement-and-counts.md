# 06a — Grass: placement and counts

**Contract:** native grows grass where the browser does. Hashed sample positions are bit-exact for fixed tiles, and rendered blades per region are within about 10% of the browser's count for the same frame.

**Seam:**
- Lift `grassPass.ts`'s constants (`GRASS_JITTER_M`, the lean/sink/wear constants, `BLADE_ROUGHNESS` and the gust/flutter terms) into `packages/battle-renderer/src/terrain/grassTuning.ts`. The generated WGSL stays byte-identical.
- The manifest gains `grass`: rules, kinds, packed shapes (`packGrassShapes`), tuning and per-region counts.
  - Regions are a 4×3 screen grid.
  - The counts come from the browser's own probe (`grassPass.ts:1017`) on the plate frames.
- **Native:** a GLSL build shader includes `terrain/ground.gdshaderinc` and ports the tile/clump sampling and `pcgHash` with 32-bit integer maths.
  - Start with a probe of a compute shader writing the MultiMesh buffer (`multimesh_get_buffer_rd_rid`).
  - If compute-written instances don't render in 4.7.2, fall back to the browser's own approach: a fixed-count MultiMesh that drops non-grown blades in the vertex stage (`grassPass.ts:569-572`). Counts then come from a GPU counter.
  - The grass window is sized in metres per pixel at the record's canvas height (`grassField.ts:288-302`).
- Delete the native target-near grid and the 512 cap.

**Runnable artifact:** a count table per plate frame (browser and native per region) and broad and close plate pairs.

**Visual variable:** coverage, judged by the counts first, then by eye.

**Focused gates:**
- Display-backed (a local rendering device needs a display): hash words and admission decisions for fixed tiles equal vectors exported from TS.
- Counts within about 10% in every region.
- The WGSL byte-identity scratch check for the lift.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** blade look, wind, colour mix (06b) and scars (phase 2).

**Delegated:** which build path, compute or vertex-stage, decided by the probe and recorded.
