# 04 — Grass and vegetation field parity

**Contract:** native reproduces the browser grass field's deterministic membership, density, species, rows, patches, LOD, exclusion, and world stability.

**Seam:** `grassField.ts`, `grassPass.ts`, `biome.ts`, `scene-assets/src/grass.ts`, scenery placement, terrain height/material and prop footprints. Native consumes manifest rows/species/seed/clearance and uses catalog bundles; it does not use a fixed target-near grid as the parity algorithm.

**Runnable checkpoint:** fixed-seed market-town meadow/crop/forest cuts with camera movement; clumps do not reshuffle and remain inside admitted regions.

**Focused gates:** same deterministic tile samples, drilled crop rows, meadow scatter, species shares, road/forest/water/prop exclusion, near/far LOD, wind/scars, stable count and resource cost. Report admitted versus rendered counts and every cap.

**Visual variable:** grass/trees/hedges/understorey coverage and continuity. Terrain colour, lighting, sky, models, and effects are out of scope.

**Acceptance evidence:** ground/forest masks, `compare-screenshots`, unprimed `screenshot-critique`, and `preview-shots`.
