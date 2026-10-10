# 07 — Authored models, buildings, props, and material roles

**Contract:** every camera-visible authored subject has the same identity, transform, pivot, LOD tier, material roles, texture channels, alpha semantics, and pose source in both clients.

**Seam:** `scene-assets` loader/schema/template/appearance catalogs plus browser model/building/prop placement owners; native consumes explicit template/module/appearance ids from the manifest. Remove the family/token shell heuristics (`reel.gd:959-1042`) and silent capsule/box fallbacks. Building LOD tiers size to canvas pixels (`buildingTier.ts`), so the manifest records the tier the browser chose for each capture frame and native never re-derives it. Phase 2 placeholder; plan it with a fresh discovery walk.

**Runnable checkpoint:** close tank/infantry/building/prop cuts with authored imports, articulated poses where published, and no duplicate shell-plus-box presentation.

**Focused gates:** catalog completeness, LFS pointer rejection, fit/pivot/yaw/z-axis validation, material-role preservation, animation/pose interpolation, visible-row coverage, and explicit fallback/omission diagnostics.

**Visual variable:** silhouette, identity, texture/material role, articulation, and building/prop composition. Lighting and effects are out of scope.

**Acceptance evidence:** close subject masks, `compare-screenshots`, unprimed `screenshot-critique`, and `preview-shots`.
