# 05 Test units are explicit

**Unlocks:** the fake units tests run on are named as fakes everywhere, and
the game can't draw, load or buy one.

## Rule

Tests run on test units or fake units unless a test states why it needs the
roster (the roster's own resolution rules, a roster model's fit, the menu
reel). A roster add, remove or edit must not break a test. This is already the
practice on main (`847c813c`: `sim::fixtures::stand_in_game`,
`stand_in_documents`); this slice makes it visible in names and enforced for
the game's catalog. "Tests" covers Rust and web tests, browser scenes, parity
fixtures, and the encounters on `test` maps (slice 06).

**Name** (decision 10): "test unit", never "stand-in", which already means the
placeholder box a prop without art is drawn as (`presentation.stand_ins`,
`asset stand-in`, `standInKit.ts`, `propAppearance.ts` `standsIn`).

## Contract

- **One committed catalog, the game's.** `fixtures/catalog.json` is roster,
  profiles, roles and props. No concrete unit or soldier kind in it is
  unreachable from a roster card: the 13 roster intermediates that ship today
  (`roster_assault_rifle`, `roster_*_gunner`, `roster_marksman_rifle_1..6`,
  `roster_heavy_sniper_1/2`) become `abstract` in slice 02.
- **The test catalog is resolved at run time**, never committed: the roster
  documents plus `fixtures/units/test/`, through the existing WebAssembly
  `resolve_catalog` export in the browser and `sim::fixtures::test_documents()`
  natively. Nothing edits or publishes it, so the mechanics editor, fixture
  publication and hot reload stay single-file.
- **The battle session owns its unit catalog.** Today about 25 modules import
  `fixtures/catalog.json` at module level (`packages/scene-assets/src/shippedUnits.ts`
  `UNITS`, read by `useBattleSession.ts`, `useUnitControl.ts`, `panelRows.ts`,
  `readouts.tsx`, `pointerPaint.ts`, `battleOverlay.ts`, `purchasePicker.tsx`, …;
  `scenarios.ts` `GAME_RULES`). They take the catalog from the session that
  runs the battle instead, so a lab running test units draws them and a game
  page can't. A session's catalog is an input: game routes pass the game's,
  labs and scenes that use test units pass the test catalog.
- **Readers that judge art read the test catalog**, since test-unit art must
  stay valid: the asset CLI's fit authority and icons (`web/asset.mjs`), the
  model workbench, `web/scenes/_units.mjs`, the mechanics editor's model fit.

## Work

1. **Tests first:** (a) the game catalog has no concrete type no card reaches
   (red today); (b) a game route mounted with a test unit in its scenario
   refuses it by name instead of drawing nothing (`AppearanceCatalog.resolve`
   returns null silently today).
2. Session-owned catalog: replace module-level `UNITS`/`GAME_RULES` reads with
   the session's catalog, owner by owner; each step keeps its tests green.
3. Move `fixtures/units/generic/` to `fixtures/units/test/` with a header
   saying these are test units, never game content. Ids: `test_tank`,
   `test_jeep`, `test_supply`, `test_rifle`, `test_recon`, `test_at`; soldier
   kinds `test_rifleman`, … Rename `stand_in_game`/`stand_in_documents` to
   `test_game`/`test_documents`.
4. Rename every use: Rust tests and examples, web tests, browser scenes,
   test-map encounters, parity fixtures, mechanics editor tests, the menu's
   unit list if slice 04 kept any, endurance (`crates/sim/src/endurance.rs`
   army and `by_id("rifle")`) and `map_analysis.rs` `INFANTRY` (both read
   through the test catalog by the tools that call them), map workbench and
   `crates/mapgen/examples/map_workbench_report.rs`, `crates/mapgen/tests/layout_cli.rs`.
   Digests move only because type ids sort differently; regenerate and name
   it in choices.md.
5. **Test-unit art is labelled, not remodelled.** Appearances `tank`, `jeep`,
   `supply_truck` and the generic soldier sets (`rifle*`, `at*`, `recon*`)
   become `test_*`, sources move to `assets/source/test/`, and the assets
   readme says they draw test units only. Their old wrecks (`tank_wreck*`,
   `jeep_wreck`, `supply_truck_wreck`) become the test units' own wrecks under
   slice 09's per-unit wreck contract.
6. The mechanics editor lists test units apart from roster units, labelled.

## Verify

The two tests; the Rust and web suites that use test units (crate by crate);
`asset bake` and `check`. `git grep` for the bare old ids in quotes finds only
the weapon id `rifle` and the roles `at` and `recon`.
