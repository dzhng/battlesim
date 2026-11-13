# Armory: choices

## Slice 05

- **`derived` kept as the key.** No existing convention publishes computed
  facts beside authored catalog fields. *Sound.*
- **`Reach` fields: `layer`, `range_m`, `only`, `intercepts`.** `only` is
  absent when the row takes every kind of unit its layer holds. When present,
  it lists the kinds the row does take, in `TargetClass` order, never
  `missile`. Interception is `intercepts`. A layer that lists only `missile`
  publishes `only: []` with `intercepts: true`: it fires at no unit there.
  Alternative: `only: null` serialized, which adds a third state to the
  TypeScript type. *Sound.*
- **`reach` reads through `against` and `engages`, not `targets.classes`.**
  The class match keeps its one owner. *Sound.*
- **The published rows are a contract function, `contract::weapons::published`,
  not code in `sim::fixtures`.** `sim::fixtures` is native-only, and the
  browser needs the same rows (next entry). `Derived` is a typed struct with
  `WeaponDefinition::derived()`. *Sound.*
- **Browser sessions publish `derived` too, through a new wasm export
  `publish_weapons`.** `SessionCatalog.weapons` came from `resolve_rules`, which
  returns the rules that the authority runs again. Resolution refuses
  `derived` as an authored field, so those rows can't carry it. Without this,
  slice 08's `unitSheet(catalog: SessionCatalog)` would read `undefined`, and
  slice 08 is a TypeScript-only lane that can't add a wasm export.
  - `composeCatalog` and `composeScenarioCatalog` now take one
    `CatalogResolver` (`resolve_catalog`, `resolve_rules`, `publish_weapons`),
    not loose functions.
  - The TypeScript types split to match. `WeaponDefinition` is the rules row
    (`ResolvedRules.weapons`). `WeaponRow extends WeaponDefinition` adds
    `derived` (`SessionCatalog.weapons`, `GameView.weapons`).
  - Alternative: `SessionCatalog.weapons` taken from the committed game view's
    rows. That is correct for the three catalog sets, but wrong for uploaded
    scenario bundles, which carry their own rules. *Provisional:* it goes past
    the slice's text (K5 names only the catalog view). The orchestrator should
    confirm it.
- **The editor shows the saved row's `derived`, not the draft's.** The facts
  come from the resolver, so a draft changes them only once saved. The panel
  says so. Alternative: read the preview's catalog after Preview, which is a
  second data path for a read-only panel. *Provisional.*
- **The sound frame takes `SessionCatalog.rules.weapons`, not
  `SessionCatalog.weapons`.** The renderer's sound frame resolves its weapon
  rows as rules, and the resolver refuses `derived`, so a battle's audio
  failed to start (`battleRules.test.ts` caught it). The rules' rows are what it
  reads. Alternative: strip `derived` in `createBattleAudio`, a second shape
  for the same rows. *Sound.*
- **The editor's panel names every altitude layer, "Does not engage" where
  the row has none,** and words the facts for an author ("Fires at what it
  cannot hurt", "vehicle only", "also shoots down incoming rounds"). Class
  names stay the contract's ids, as the editor's own `classes` field shows
  them. Three unprimed critiques drove the layout: a dashed, fixed-width,
  read-only panel apart from the amber "Used by" note. *Sound.*
- **The editor's dev-API client moved to `api.ts`.** `MechanicsEditor.tsx` was
  over the size budget, and the ratchet refuses growth. The client is its own
  concept. *Sound.*
- **The catalog-view tests moved to `crates/sim/tests/catalog/view.rs`**
  (`the_browsers_catalog_view_is_current` and the new generative test), for
  the same ratchet on `catalog.rs`. The `catalog::` filter still selects them.
  *Sound.*
- **An authored `derived` is refused by name.** The slice said such a field
  "would vanish silently". It doesn't: `resolve_weapons` refuses unknown
  fields. The removed-field test now covers `derived`. *Sound.*
- **Shared TypeScript `AltitudeLayer` and `TargetClass` types** in
  `scene-assets/units.ts`. `layerOf`, `mobility.air.layer` and `targets` use
  them in place of three literal unions. *Sound.*
- **W09's amendment is dated 2026-11-12 and credited to the user** (map row
  W09), and `specs/default-weapon-cleanup/README.md` no longer says the
  amendment is pending. *Sound.*
