# 03 — Terrain relief and surface material parity

**Contract:** native seats every world layer on the same height/relief and preserves browser surface semantics.

**Seam:** saved-map height grid and surface exports through the browser terrain owners (`packages/battle-renderer/src/frame/terrainHeights.ts`, `packages/battle-renderer/src/terrain/terrainSurface.ts`, `packages/battle-renderer/src/terrain/terrainGrid.ts`, `packages/battle-renderer/src/frame/terrainMaterial.ts`, `packages/battle-renderer/src/terrain/biome.ts`); native receives resolved triangles/rows/material parameters through the manifest.

**Runnable checkpoint:** market-town and paris-corner broad/near cuts with relief, roads, walks, curbs, fields, forest floor, water edges, scars, and stable world-anchored material variation.

**Focused gates:** height/relief sampling, polygon outline preservation, road/field/forest membership, hidden-ground rules, and no change to map admission or Rust visibility.

**Visual variable:** lower-frame substrate, relief, road/field edge structure, and material variation. Grass density, sky, models, and effects are out of scope.

**Acceptance evidence:** lower-third ground/road masks, `compare-screenshots`, unprimed `screenshot-critique`, and `preview-shots`.
