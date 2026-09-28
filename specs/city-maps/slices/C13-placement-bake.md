# C13: placement bake

**Depends on:** C11, C04 (building data + exposed edges). **Kind:** slice.

## Question
Can each footprint part's building be baked into cached, deterministic placement files?

## Contract it unlocks
- `asset city-bake <map-id>`: headless Blender evaluates each part's archetype graph with the building's floors, floor heights, seed, exposed edges (S5) and street outputs off.
- Cached by input hash: graph, patches, Blender version, parameters, textures and fit data.
- It writes content-addressed **per-tile placement files** in G0's encoding to `assets/runtime/maps/<id>/` (LFS), **outside** `bakeAll`'s cleanup (L2). Rows are (building, part, state, module, transform or run).
- Placements are presentation-only and never enter `map.json`.

## API seam
`web/asset.mjs city-bake`, `packages/scene-assets/src/placements.ts` (schema, codec, validate: every module id resolves).

## What the human can run or see
The bake log with minutes, bytes and cache hits, and one real block baked.

## Verification
- Two-run byte identity.
- A warm re-run is all cache hits.
- Missing-module and bounds checks.
- Per-map placement bytes within G0's budget.

## Delegated to the implementer
Cache location; tile file layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No per-building GLB or catalog row (L1); no runtime Blender.
