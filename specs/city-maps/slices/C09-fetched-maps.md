# C09: fetched maps

**Depends on:** C01; SG6's dry run informs it. **Kind:** slice.

## Question
Do maps load by id at runtime instead of riding in the JS bundle?

## Contract it unlocks
The scenario references its map by id (`map: "maps/<id>"`), and the browser and the wasm worker fetch `fixtures/maps/<id>/map.json`. **Every map moves in the same commit** (S-fetch): the village's map out of `village.json`, the 11 `fixtures/*-lab.json` files, and the endurance map inline at `crates/sim/src/endurance.rs:73-75`. There is one location and one loader from here on; C60 adds only metadata. Rust tests load maps through `maps::load(id)`. Static imports of `village.json` for map data (`captions.tsx:8`, `hudTheme.ts:7`, `input/reverseZone.ts:6`) read the map through the loader instead. The `config_digest` header change is named.

## API seam
`contract::scenario`, `web/src/battle/sim/` loader, `crates/sim/src/village/`.

## What the human can run or see
The village plays unchanged; `vite build` size before and after.

## Verification
- Village and lab digest and replay parity (outcomes).
- The named config-identity change recorded.
- JS gzip does not grow; no map is in the bundle.
- Every scene id unchanged; no alias or compat import.

## Delegated to the implementer
Loader caching. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village battles.
