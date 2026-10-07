# 04 Session-owned unit catalog

**Unlocks:** a battle draws, reads and loads exactly the units its own catalog
holds, so test units (slice 05) and menu units (slice 08) can exist without
ever entering the game's catalog, and with no temporary path for either.

## What is wrong

The browser has one unit catalog for every page: `fixtures/catalog.json`,
imported at module level by `packages/scene-assets/src/shippedUnits.ts`
(`UNITS`) and folded into `apps/battle-lab/src/scenarios.ts` `GAME_RULES`. 11
non-test files import the file directly and 76 read `UNITS`/`GAME_RULES`
(`useBattleSession.ts`, `useUnitControl.ts`, `panelRows.ts`, `readouts.tsx`,
`pointerPaint.ts`, `battleOverlay.ts`, `purchasePicker.tsx`, …). A lab whose
wasm runs other units has presenters that don't know them:
`AppearanceCatalog.resolve` returns null (`appearanceCatalog.ts:87`) and the
unit is silently not drawn; `panelRows.ts:165` drops supply stock the same way.

## Contract

- **One resolver, three document sets.** The game: roster, profiles, roles,
  props. Tests: those plus `fixtures/units/test/` (slice 05). The menu: those
  plus `fixtures/units/menu/` (slice 08). Each is resolved by the same code:
  the WebAssembly `resolve_catalog` export (`crates/game-wasm/src/lib.rs:663`)
  in the browser, `sim::fixtures` natively. Only the game's set is committed,
  as `fixtures/catalog.json`; the others are resolved at run time and never
  written, so the mechanics editor, fixture publication and hot reload stay
  single-file.
- **The battle session owns its catalog.** A prepared battle carries its
  resolved catalog view; the session builds one `UnitCatalog` from it. UI
  reads it through a context; renderer, audio and overlay constructors take it
  as an argument (as `useBattleSession.ts` already passes `UNITS` today).
  `shippedUnits.ts` and every module-level `@fixtures/catalog.json` import go,
  except the game session's factory.
- **A page loads the appearances its session's catalog binds**, not every
  appearance in `assets/catalog.json`: a game page never fetches test or menu
  art (test art is 265 MB of today's bundles).
- **Readers that judge art take the test set**, since test-unit art must stay
  valid: the asset CLI's fit authority and icons (`web/asset.mjs:92,119,687`),
  the model workbench (`routes/workbench.tsx:52`), `web/scenes/_units.mjs`,
  the mechanics editor's model fit (`apps/mechanics-editor/modelFit.ts`).

## Work

1. **Tests first:** (a) a lab session whose scenario holds a unit only the
   test set has draws it (`AppearanceCatalog.resolve` non-null); (b) a game
   session given a scenario with a unit outside its catalog refuses it by name
   instead of drawing nothing; (c) only the session factory imports
   `fixtures/catalog.json` (a module-graph check).
2. Session catalog through the app, owner by owner, each step green. Until
   slice 05 lands, the test set equals the game's plus `fixtures/units/generic`.
3. Appearance loading scoped to the session catalog's bindings.

## Verify

The three tests; `battle-lab`, `battle-renderer`, `battle-audio` unit tests;
one game route scene and one lab scene. No digest moves (presentation only).

## Delegated

Context and constructor shapes; how the factory caches resolved sets.
