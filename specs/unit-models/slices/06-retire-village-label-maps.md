# 06 Retire the village, label every fixed map

**Unlocks:** no village code, route, map, format or name remains (only the
generator's `village` settlement class, which is real map content), and every
hard-coded map says what it is for.

Runs after 05: the test-unit rename has already touched the same call sites,
so this slice only moves tests off the village map and deletes.

## Contract

- **Map-free rules.** `crates/sim/src/fixtures.rs` `game()` (`:127-133`)
  stops injecting the village map; `test_game()` inherits that. A test that
  needs ground calls `fixtures::with_map("<test map>")`.
- **Saved map category** is a typed `MapCategory { test, menu }` in the
  `contract` crate, written in `meta.json` and refusing anything else.
  `web/src/maps/catalogue.ts` and `crates/sim/tests/maps.rs` validate it. The
  menu backdrop accepts only `menu` maps; labs and tests only `test` maps or
  generated ones; player battles never read the saved catalogue.
  - `menu`: market-town, paris-corner (`status: draft`: the menu check must not
    also require `released`).
  - `test`: ambush, camera-lab, consequences, deployment, endurance, garrison,
    geometry, ground, movement, readouts, river, sensors, supply, weapons, and
    **market-town-test**, a copy of market-town holding its `assault`
    encounter (used as test content by `crates/sim/tests/move_admission.rs:359`,
    `web/tests/prepareBattle.test.ts:338-354`, `web/scenes/geometry.mjs`,
    `downloadBudget.test.ts:41`, and the map workbench report's fallback,
    `map_workbench_report.rs:180-186`). market-town keeps only `menu`.
  - Today's values are `lab` (14), `benchmark` (endurance) and `playable`
    (market-town); only `listMaps` filters and `web/tests/mapCatalogue.test.ts`
    read them, and no tool writes them.

## Work

1. **Tests first:** listing refuses a map without a valid category; the menu
   refuses a `test` map; a native rules fixture carries no map.
2. **Move tests off the village map** (the bulk of the slice):
   `common::game()`/`common::rules()` have ~148 uses in 33 files; direct map
   reads in `sight.rs:177`, `flight_load.rs:36`, `examples/ground_resources.rs:59`.
   Tests that relied on village geometry move to `geometry`, `movement` or
   another test map, or a generated map by seed. Crate by crate, each crate's
   tests green.
3. **Delete** `crates/sim/src/village/` (scenario, scripted blue, scripts,
   trials, now without Defender/Referee, slice 03), `crates/sim/tests/village.rs`,
   `examples/village_report.rs`, the wasm exports `village_scenario` and
   `Battle::scripted`, the worker protocol's `script` field
   (`web/src/battle/sim/protocol.ts`, `module.ts`), `routes/village*.tsx` and
   their router entries and menu link, the village replay format
   (`replayFile.tsx` `VillageReplayFile`, `/replay/village`; rename the
   IndexedDB key `village-last-replay`, dropping the last saved replay, no
   migration), `fixtures/game.json`'s village-only sections (`spawn`,
   `variants`, `defender_policy`, `encounter`), `"village"` in
   `MAP_CHARACTERS`, `fixtures/maps/village/`, `web/scenes/village.mjs`,
   `web/scenes/_units.mjs` `villageMap`, the benchmark's `villageScenario`
   path, and `blender/city/village.py`/`house.py` if nothing else exports
   through them. The app-journey scene's village run (re-pinned in `5531eb3c`)
   moves to a generated battle.
4. **Labs on the village map** (`/lab/fog`, `/lab/projectiles`,
   `/lab/fog-look`, `/lab/ground?village`, `/workbench` sources, the street
   scenario) move to a generated map by fixed seed or a test map.
5. **Benchmark** default becomes `city-contact`; delete `village-contact` and
   its camera; update `web/src/battle/benchmark/README.md`.
6. **Live art off the name:** `village_ruin` → `ruin`;
   `assets/source/village/*` → `assets/source/props/`; city set `village` →
   `lab_boxes` (its `authored` catalogue serves only `test` maps). Art names
   change, not physics or content hashes: map and replay hashes unchanged.
7. **Relabel every `meta.json`**, create market-town-test.
8. **Docs** (`write-docs`): root README (drop "the village remains a developer
   test arena"; say game battles are generated, game units are the roster,
   test units and `test` maps are tests' own, menu units and `menu` maps the
   menu's own, and the developer menu's entries are tools, kept visible in
   production builds), `apps/battle-lab/README.md`, `web/README.md`,
   `crates/sim/README.md`, examples README, `fixtures/README.md` "Saved maps".

## Verify

Per step, the narrowest tests: sim crate tests after 2–3, web tests for the
protocol and replay changes, the touched lab scenes after 4, `asset bake` and
`check` after 6, the map catalogue tests after 7. `git grep -i village` lists
only the settlement class in mapgen and presets, and the specs. Labs that
changed look: screenshot each once, compare-screenshots against its before
shot, screenshot-critique unprimed, last.

## Delegated

Which test map each moved test uses; the storage key name.
