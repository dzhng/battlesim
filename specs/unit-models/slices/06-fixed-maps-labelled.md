# 06 Fixed maps are labelled

**Unlocks:** every hard-coded map says what it is for, and the game can't
pick one up as content.

## Contract

Saved map `meta.json` `category` is `test` or `menu` (today's categories
fold into these). `web/src/maps/catalogue.ts` refuses anything else. The menu
backdrop accepts only `menu` maps; labs and tests only `test` maps (or
generated ones). Player battles never read the saved catalogue, as today.

`menu`: market-town, paris-corner. `test`: everything else (ambush,
camera-lab, consequences, deployment, endurance, garrison, geometry, ground,
movement, readouts, river, sensors, supply, weapons), plus **market-town-test**
(decision 11): a copy of market-town holding its `assault` encounter, which
four callers use as test content (`crates/sim/tests/move_admission.rs:359`,
`web/tests/prepareBattle.test.ts:338-354`, `web/scenes/geometry.mjs`,
`downloadBudget.test.ts:41`) and the map workbench report falls back to
(`map_workbench_report.rs:180-186`). market-town keeps only `menu`. The
`lab_boxes` authored catalogue serves only `test` maps.

Today's categories are `lab` (13), `benchmark` (endurance) and `playable`
(market-town); only `listMaps` filters and `web/tests/mapCatalogue.test.ts`
read them, and no tool writes them. `paris-corner` is `status: draft`: the menu
check must not also require `released`, or it refuses a map the reel uses.

## Work

1. Test first: listing refuses an unlabelled map and a menu backdrop scene
   naming a `test` map.
2. Relabel every `meta.json`; update the validator and its test.
3. Docs: `fixtures/README.md` "Saved maps" and the root README say saved maps
   are test or menu fixtures, game battles are generated, game units are the
   roster, test units are tests' own, and menu units are the menu's own (use `write-docs`).
4. The developer menu stays visible in production builds (user, 2026-10-06:
   the audience is technical). Its entries are tools, not game content; say so
   in the root README's menu description.

## Verify

The two tests; the map catalogue tests in `crates/sim/tests/maps.rs` and
`web/tests/mapCatalogue.test.ts`.
