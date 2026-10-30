# 09 · Scenery tier mesh: one Rust owner

**Contract.** Both shells place scenery from tier meshes Rust builds from the
bundles. The TypeScript copy goes.

## Today

- Rust `tier_mesh`, `STATE`, `FOLIAGE` (`crates/renderer-core/src/scenery.rs:30-64`)
  and TypeScript `tierMesh`, `SCENERY_STATE`, `FOLIAGE_MATERIAL`
  (`packages/battle-renderer/src/scenery/appearance.ts`) produce identical meshes.
- Native already places from Rust (`native/client/src/capture.rs:312-319`).
  Asset replacement already rebuilds scenery from Rust tiers
  (`runtime/assets.rs:48`). Only the browser's **initial** placement uses the
  TypeScript copy (`web/src/battle/sim/presentation.ts:530-542`).
- Placement needs the mesh for tree height and radius, felling, and stumps
  (`ground/scenery.rs:538-550`, `:666`, `:717`, `:764`).

## Seam

- Scenery kinds join source selection as grass does
  (`crates/renderer-browser/src/appearance.rs:88-95`), so scenery bundles load
  before placement creates appearance rows (map unknown 7).
- One Rust function places scenery from bundles, called by native's capture and
  by renderer-browser after its bundles are staged.
- Delete `tierMesh`, `SCENERY_STATE`, `FOLIAGE_MATERIAL` and the TypeScript
  placement call. `sceneryAppearances` (used at `worldMesh.ts:270-271`) is a
  catalog lookup and stays; check it no longer needs `SCENERY_STATE`.

## Verify

- Probe first: log the frame `place_scenery` lands on in `street` and `ground`,
  before and after; placement moves later, after bundles decode.
- Menu reel byte-identical (it has trees); browser scenes `street`, `ground`,
  `unit-roster`; vitest.

**Delegated:** how renderer-browser reaches the presentation mutably; the
function's name and home.
**Stays green:** tree scale, felling, stumps, asset replacement atomicity.
