# 14 · Tests and scenes assert behaviour

**Contract.** No test or scene copies an implementation, pins catalog data, or
mocks internals, and the GPU allocation observer has one owner. Use
`write-tests`.

## Each fix

- `web/tests/proxyModels.test.ts:42-52` repeats the gamma formula from
  `web/src/battle/render/proxyModels.ts:29`. Assert the colour that comes out
  instead.
- `web/scenes/_rulerTour.mjs:40-43` repeats the first-mount rule; `:182-184`
  pins a tank with exactly two ranges in order. Assert the ruler's marks against
  the unit's weapons as the ruler groups them, whatever the tank carries.
- `web/scenes/_modelGhost.mjs:44-48` pins `basketball_court` and imports about
  12 internals. Pick its subjects by property (a static model, a skinned one, an
  articulated one).
- `web/tests/modelWorkbenchSource.test.tsx` mocks six modules. Drive the
  workbench through its seam and fake only the boundary.
- `_appJourney.mjs:29-74` and `_terrainQuery.mjs:4-48` carry near-identical
  buffer/texture observers, and re-wrap `requestDevice`, which `_leaks.mjs:13-27`
  already wraps. Move the observer into `_leaks.mjs` and update its header (`:5`
  says buffers and textures belong elsewhere today).

## Verify

- Each replacement fails once against a deliberately broken behaviour, then
  passes.
- Vitest; scenes `street` (tours `ruler`, `appflow`), `unit-roster`,
  `workbench`, and the scenes that use `_terrainQuery` (`_forests`,
  `_groundFilter`, `_surfaces`, `_surfaceField` callers).

**Delegated:** the observer's API; which boundary each test fakes.
**Stays green:** leak detection and every race the workbench test covers.
