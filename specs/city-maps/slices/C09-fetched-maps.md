# C09: fetched maps

**Depends on:** C01. **Kind:** slice.

## Question
Do maps load by id at runtime instead of riding in the JS bundle?

## Contract it unlocks
The scenario references its map by id (`map: "maps/<id>"`), and the browser and the wasm worker fetch `fixtures/maps/<id>/map.json`. The village's map moves out of `village.json` into `fixtures/maps/village/` in the same commit (S-fetch). Static imports of `village.json` for map data (`captions.tsx:8`, `hudTheme.ts:7`, `input/reverseZone.ts:6`) read the map through the loader instead. The `config_digest` header change is named.

## API seam
`contract::scenario`, `web/src/battle/sim/` loader, `crates/sim/src/village/`.

## What the human can run or see
The village plays unchanged; `vite build` size before and after.

## Verification
- Village digest and replay parity (outcomes).
- The named config-identity change recorded.
- JS gzip does not grow; the map is absent from the bundle.

## Delegated to the implementer
Loader caching. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village battles.
