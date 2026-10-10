# 05b — Terrain: field character

**Contract:** native's ground material has the browser's field character: furrows, broken rows, wheelings, mottle, grain, patchiness and wear, at broad and close cuts.

**Seam:**
- Already built (`8b101db5`): `terrainTuning.ts`, with WGSL byte-identical apart from one comment. Before that, the plan was to lift `terrainMaterial.ts`'s tuning constants (the `MOTTLE_*`, `GRAIN_*`, `BANK_*` and `DISTANT_FADE_M` families and the other literals spliced into WGSL) into one exported table, `packages/battle-renderer/src/terrain/terrainTuning.ts`.
  - The browser shader and the manifest both read it.
  - The values are interpolated into WGSL as constants, so the generated shader text must stay byte-identical.
- Already built (`948dc7fd`): the manifest's `tuning.terrain` is the table.
- Native `terrain.gdshader` ports the formulas and reads the tuning as uniforms.

**Runnable artifact:** a close and a broad plate pair over crop fields and meadow.

**Visual variable:** furrows, patches and wear: does it read as the same kind of field?

**Focused gates:**
- A scratch check that the generated terrain WGSL is byte-identical before and after the lift. It's run once and not kept as a snapshot test.
- A TS test checks that the manifest's tuning equals the table.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** grass, scars and models.

**Delegated:** further splitting per surface family if one formula block resists matching.
