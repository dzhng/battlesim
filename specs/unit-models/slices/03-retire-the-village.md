# 03 Retire the village

**Unlocks:** no village code, route, map, format or name; the encounter
behaviour it owned lives under its real name.

## Work, in order

1. **Move, don't change.** `village::Defender` and `village::Referee` move to
   `sim::encounter` (they are already role-keyed). `battle.rs` and the
   encounter planner call them there. Digests unchanged: run the menu and
   generated-assault parity checks that exist.
2. **Map-free rules.** `crates/sim/src/fixtures.rs` `game()` stops injecting
   the village map. Tests that need ground load a named test map or build
   their own; tests that relied on village geometry move to `geometry`,
   `movement` or another existing test map, or to a generated map by seed.
   Native tests are the bulk of this slice; work crate by crate, running each
   crate's tests.
3. **Delete** `crates/sim/src/village/` (scenario, scripted blue, scripts,
   trials), `crates/sim/tests/village.rs`, `examples/village_report.rs`, the
   wasm exports `village_scenario` and `Battle::scripted`, the worker
   protocol's `script` field (`web/src/battle/sim/protocol.ts`, `module.ts`),
   `routes/village*.tsx` and their router entries and menu link, the village
   replay format (`replayFile.tsx` `VillageReplayFile`, `/replay/village`;
   rename the IndexedDB key, no migration), `fixtures/game.json`'s village-only
   sections (`spawn`, `variants`, `defender_policy`, `encounter`), `"village"`
   in `MAP_CHARACTERS`, `fixtures/maps/village/`, `scenes/village.mjs`,
   `blender/city/village.py` and `house.py` if nothing else exports through them.
4. **Labs that drew on the village map** (`/lab/fog`, `/lab/projectiles`,
   `/lab/fog-look`, `/lab/ground?village`, `/workbench` sources, the street
   scenario) switch to a generated map by fixed seed or an existing test map.
5. **Benchmark** default becomes `city-contact`; delete `village-contact`
   and its camera; update `web/src/battle/benchmark/README.md`.
6. **Rename live art off the name:** `village_ruin` → `ruin`;
   `assets/source/village/*` → `assets/source/props/`; city set `village` →
   `lab_boxes` (its `authored` catalogue serves only test maps, labelled in
   slice 06); `tank_wreck*`, `jeep_wreck`, `supply_truck_wreck` →
   `wreck_heavy_tracked*`, `wreck_light_wheeled`, `wreck_truck`. Renames change
   art identity, not physics: map hashes unchanged.
7. Docs: root README ("the village remains a developer test arena", menu
   description), `apps/battle-lab/README.md`, `web/README.md`,
   `crates/sim/README.md`, examples README, `fixtures/README.md`. Use
   `write-docs`.

## Verify

Per step, the narrowest tests: sim crate tests after 1–3, web tests for the
protocol and replay changes, the touched lab scenes after 4, `asset bake` and
`check` after 6. `git grep -i village` then lists only the settlement class
in mapgen and presets, and choices.md. Labs that changed look: screenshot each
once, compare against its before shot (compare-screenshots), screenshot-critique
unprimed, last.

## Delegated

Which test map each test moves to; storage key name.
