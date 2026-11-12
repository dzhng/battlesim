# 16: integration milestone (coordinator)

The procedural plan had this slice re-export every family once through the
shared slices 08–15. The Argon pilot already did that for the tracked
families whose frames changed (P14), so what is left is the wheeled side of
slice 11's bevel floor, the art-load limit and a green, pushed milestone.
The family lanes (17–29) don't wait for it: they touch only their own
families, and merge after it.

## Contract

- Every receipt is fresh against HEAD, tracked and wheeled.
- `CATALOG_LOAD_MAX_BYTES` has room for the rollout's denser bodies.
- `bun run check` is green and the milestone is pushed to `staging`, then
  promoted.

## Steps

1. **Receipt freshness probe** (in `throwaway/`): every receipt under
   `assets/source/roster/` whose recorded source hashes no longer match HEAD.
   Expect the wheeled scripts that call `bolted_panel` or `armour_tiles`
   (C19) and the T-90M (P14 left it at its committed GLB).
2. **Re-export** each stale family, live and `--wreck`, with no new lint
   issues.
3. **Art load.** The pilot left every unit's art at 636.7 of 640 MiB. Raise
   `CATALOG_LOAD_MAX_BYTES` (`packages/scene-assets/src/schema.ts`) to the
   measured load plus the rollout's expected growth, as Q3 allows, and record
   the value in `decisions.md`.
4. **Bake and check:** `asset bake`, `asset icons`, `asset check`, then
   `bun run check`.
5. **Push the milestone** to `staging`, then `bun run promote:main`.
