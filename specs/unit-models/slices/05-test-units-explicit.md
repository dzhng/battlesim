# 05 Test units are explicit

**Unlocks:** the fake units tests run on are named as fakes everywhere, and
the game's catalog holds none.

## Rule

Tests run on test units or fake units unless a test states why it needs the
roster (the roster's own resolution rules, a roster model's fit, the menu
reel). A roster add, remove or edit must not break a test. This is already the
practice on main (`847c813c`: `sim::fixtures::stand_in_game`,
`stand_in_documents`; `0281e694`: `with_units_at_limits`). This slice makes it
visible in names and enforced for the game's catalog. "Tests" covers Rust and
web tests, browser scenes, parity fixtures, encounters on `test` maps (slice
06), and the developer tools that must not move with the roster (the
benchmark, the endurance lab, the map workbench).

**Name:** "test unit", never "stand-in", which already names the placeholder
box a prop without art is drawn as (`presentation.stand_ins`, `asset stand-in`,
`standInKit.ts`, `propAppearance.ts` `standsIn`).

## Work

1. **Test first:** the committed `fixtures/catalog.json` has no concrete unit
   or soldier kind that no roster card reaches, and no `test_` or `menu_` id.
   Red today (the generic units ship; slice 02 makes the 13 roster
   intermediates abstract).
2. Move `fixtures/units/generic/` to `fixtures/units/test/` with a header
   saying these are test units, never game content. Ids: `test_tank`,
   `test_jeep`, `test_supply`, `test_rifle`, `test_recon`, `test_at`; soldier
   kinds `test_rifleman`, …; the limit fakes `limit_tracked`/`limit_wheeled`
   become `test_limit_tracked`/`test_limit_wheeled`. `stand_in_game`,
   `stand_in_documents` become `test_game`, `test_documents`. The game set
   (slice 04) excludes the folder.
3. Rename every use: Rust tests and examples, web tests, browser scenes,
   test-map encounters, parity fixtures, mechanics editor tests, endurance
   (`crates/sim/src/endurance.rs` army and `by_id("rifle")`),
   `map_analysis.rs` `INFANTRY`, the map workbench
   (`apps/map-workbench/server.ts`, `crates/mapgen/examples/map_workbench_report.rs`),
   `crates/mapgen/tests/layout_cli.rs`, `fixtures/encounters.json`, and the
   words in `crates/sim/README.md`'s checks section and the `write-tests`
   skill. Digests move only because type ids sort differently; regenerate and
   name it in choices.md.
4. **Test-unit art is labelled, not remodelled.** Appearances `tank`, `jeep`,
   `supply_truck` and the generic soldier sets (`rifle*`, `at*`, `recon*`)
   become `test_*`; sources move to `assets/source/test/`; the assets readme
   says they draw test units only. Their wrecks become the test units' own
   wrecks under slice 11's contract. `vehicle_crew.py` no longer reads
   `assets/source/infantry/rifle.glb` as roster crew (slice 09 gives crew a
   per-faction source).
5. The mechanics editor lists test units apart from roster units, labelled.

## Verify

The test; Rust and web suites that use test units (crate by crate); `asset
bake` and `check`. `git grep` for the bare old ids in quotes finds only the
weapon id `rifle` and the roles `at` and `recon`.

## Delegated

Nothing beyond the mechanical renames.
