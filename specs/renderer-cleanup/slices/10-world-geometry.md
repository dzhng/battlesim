# 10 · World geometry: one Rust constructor

**Contract.** Both shells build terrain, skirt, backdrop and water through one
Rust constructor. The browser stops assembling them in TypeScript.

## Today

- Native: `WorldGeometry::from_triangles` (`crates/renderer-core/src/runtime/world.rs:26-40`).
- Browser: `web/src/battle/render/sceneGeometry.ts` and
  `packages/battle-renderer/src/worldMesh.ts:230-252` call the same wasm mesh
  helpers, then `set_world_geometry` (`crates/renderer-browser/src/world.rs:23-45`).
- Browser-only inputs: the traversal view's surface tint and blocked flag plus
  prop boxes (`worldMesh.ts:194-212`), the labs' prebuilt terrain meshes
  (`terrainSurface.ts:109-136`, `benchWorld.ts:103-107`, `routes/frame.tsx:75`),
  and the geometry lab's backdrop override (`scene.ts:53`).

## Seam

- One Rust constructor takes the triangles plus an optional tint, extra faces
  and a water switch (map unknown 8), exported through renderer-browser.
- Labs with a prebuilt mesh keep a direct entry point, and the backdrop override
  stays.
- The slot is renamed `skirt_and_props` in Rust (`runtime/world.rs:10`,
  `submitted.rs`, `ground.rs` stats) and TypeScript (`WorldLayers.props`,
  `sceneStats.ts`).

## Verify

- Probe first: hash the arrays the browser passes to `set_world_geometry`
  against the Rust constructor's output for the same inputs. If they differ
  today, switching changes browser pixels: stop and record why.
- All three reels byte-identical; browser scenes `geometry`, `ground`, `river`,
  `water`, `frame`; vitest (`worldMesh`).

**Delegated:** the constructor's signature details.
**Stays green:** the traversal overlay, the labs' surrounds.
