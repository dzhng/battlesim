# Armory: choices

## Slice 07

- **Fade: 0.5 s, in and out.** A dropped loop fades over 0.5 s, and a new one
  ramps in over the same time (its `attack`). Why: a unit switch reads as a
  quick crossfade, well under the menu's 1.2 s bed fade, which is a
  screen-change pace. Alternative: reuse `MENU_FADE_S`, which drags on a list
  sweep. *Provisional*: slice 11 hears it.
- **Id range: open-ended, from 16 up.** The sink keys voices in a map, so the
  range needs no cap; each new show voice takes the next id. The bed (1) and
  the cue (3) sit below. Alternative: a fixed block of N ids, which adds a
  "too many loops" case that no unit has. *Sound.*
- **The bed restart clears by id.** `WebAudioSink.clear` takes an optional
  id filter, and the bed restart clears everything but show voices. Why: the
  restart has to cut the previous bed while it is still fading, and a stopped
  voice has already left the id map, so stopping slot 1 cannot reach it.
  Alternative: a second sink for show loops, which would need its own master,
  limiter and volume wiring. *Sound.*
- **Leaving the menu screen forgets the asked-for loops; mute keeps them.**
  `loading` fades them out and `other` cuts them with the rest of the menu
  mix. Returning to the menu plays nothing until a page asks again. Why: the
  loops belong to the page that asked, and that page is gone. Mute is a
  setting, so unmuting resumes what was asked for. *Sound.*
- **Readiness has one owner.** `SoundBank.prepared(name)` reports whether
  `get` can return a sound. `AppAudio` starts a loop only once its sound is
  prepared, and asks the bank for the missing ones. The bank already joins
  duplicate preparations. Alternative: a ready set in `AppAudio`, which
  duplicates the bank's own state. *Sound.*
- **Show loops ride the `units` bus with no low-pass, no reverb and no
  position.** The spec names the bus. The rest matches the bed: the sound is
  heard at the camera, not placed in a world. *Sound.*
- **Loops are keyed by sound name.** If a call names the same sound twice,
  the last entry wins. *Sound.*

## Slice 09

- **The constructor takes the unit catalog: `new ArmoryAppearances(units, fetcher?)`.**
  The spec's seam was `constructor(fetcher?)`, but a kind's art comes from
  `UnitCatalog.appearancesFor`. The catalog is the session's
  (`useSessionCatalog`), and taking it once keeps `show` to the spec's
  selection. Alternative: pass the worn appearance set into `show`, which
  pushes the kind-to-art mapping onto every caller. *Sound.*
- **`show` resolves `{ appearances, runtime }`, not the bare
  `InstalledAppearances`.** `runtime` is the selection's library as
  `RuntimeFiles` (its `catalog` and `contents`). `fieldedBattle` reads a
  unit's bundle contents through these, and they must come from the library
  that installed the art: the shared `gameRuntime` would answer from the
  wrong library. Alternative: a `runtime` getter for the newest library,
  which races a pending switch. *Sound*; slice 11 consumes it.
- **A fresh `AppearanceLibrary` per call, no narrowing method.** The loader
  is untouched, so it stays the one owner of loading, and an old selection's
  decoded art goes when its library is dropped. A narrowing method would add
  a second way to shrink a generation that only the Armory uses. *Sound.*
- **The memo keeps the newest selection's files.** It holds every file that
  the last selection to resolve read, as bytes, keyed by URL. After each
  answered call it is replaced by that call's files. The yard, the catalog
  and the template library are in every selection, so they download once,
  while the previous unit's bytes leave on the next switch. A failed or
  superseded call never replaces it, and no failed response is kept.
  Alternative: memoize only the yard's own URLs, which means asking the
  catalog which files belong to the yard. That duplicates
  `catalogLoadNames`. *Sound.*
- **A superseded call stops fetching.** Its next fetch rejects with
  AbortError, so a quick list sweep doesn't keep downloading units nobody
  will see. The call itself answers AbortError however its load ended.
  Fetches already in flight are not cancelled, because `Fetch` takes no
  signal. *Sound.*
- **The default fetcher is `countedFetch`,** so the Armory's downloads move
  the loading screen's counter as the battle's do. *Sound.*
- **The yard is the scenery appearance set, the loader's own `scenery`
  filter.** Slice 10 maps prop kinds to appearances (`propsDrawnBy`) before
  calling. *Sound.*
Decisions made during implementation where the spec was silent or where the
code argued for a different seam. Each gives the choice, why, the alternative,
and a verdict.

## Slice 01

- **`MENU_PAGES` rows are plain data (`id`, `path`, `title`, `tile`). Each
  page's contents are in `menu/MenuVisit.tsx` `CONTENTS: Record<MenuPageId,
  ComponentType>`, and they are eager, not `lazy`.**
  - Why:
    - `menuHref` reads the Skirmish path from the rows. A component in the
      row would make an import cycle: `battleLinks` → `pages` →
      `SkirmishPage` → `battleLinks`.
    - The browser scenes (`.mjs` under node) import the same paths.
    - A `lazy` page leaves the plate blank the first time it opens, because
      React throttles a Suspense reveal by about 300 ms.
    - The four pages were already in the entry bundle.
    - The `Record` is exhaustive, so a new row without contents fails to
      type-check.
  - Alternative: the slice's `page: LazyExoticComponent` in the row.
  - Verdict: provisional. Slice 08 adds `armory` to `CONTENTS`, as a `lazy`
    import under a `Suspense` if its bundle earns it.
- **`menuPageOf` lives in `router.tsx`, beside `resolvePage`, not in
  `pages.ts`.**
  - Why: its `null` (a lab or battle route) needs the lab route table
    (`ROUTES`, the dev editors, `/labs`). Putting it in `pages.ts` would need
    an import cycle or moving the lab table.
  - Alternative: `pages.ts` as the slice says.
  - Verdict: sound.
- **`left` is derived in the shell from the page changing.** When the address
  goes from a page to home, `left` is that page. Home's tile takes focus with
  `autoFocus`.
  - Why: Back, Escape and the browser's Back all focus the tile left, with no
    flag set at each call site.
  - Alternative: set `left` inside `back()` only.
  - Verdict: sound.
- **`fromMenu` is router location state, so it survives a refresh** (it lives
  in `history.state`). After a refresh, Back still steps back to the home entry
  it came from.
  - Alternative: drop it on refresh, which the slice allowed.
  - Verdict: sound.
- **`PageVisit` keeps keying on `location.key`.**
  - Why: slice 08's raw `history.replaceState` is invisible to the router, so
    a unit pick never remounts the page.
  - Verdict: sound.
- **`detected` means the graphics tier is settled:** detection wasn't needed,
  or it has finished. It is the same condition that gates the runtime
  prefetch.
  - Verdict: sound.
- **The shell's `LoadingTasks` still wraps only the backdrop.**
  - Why: no page reports loading yet.
  - Slice 11 must move the page inside it before `ArmoryStage` can report
    `useLabLoading("renderer", …)` to the shell.
  - Verdict: provisional.
- **`MainMenu.tsx` became `menu/Home.tsx`** (`git mv`), and the page modules
  sit beside it: `SkirmishPage`, `SettingsPage` and `DeveloperPage`. The
  Tutorial page's contents are the existing `Tutorial`, which the pause menu
  also opens.
  - Verdict: sound.
- **The developer list lost its disabled-entry branch** (`href: null`). No
  entry used it.
  - Verdict: sound.
- **`menuHref(null)` is `/`.** A replay's Cancel lands on home, because a
  replay names no map to set up.
  - Verdict: sound.
- **The `lobby` scene opens `/skirmish` directly**, without clicking the tile.
  The `ui` scene, the journey, `generated` and the jsdom tests still click it.
  - Verdict: sound.
- **`ui.mjs` gains one check per page loop:** the tile opens the page at its
  own address.
  - Verdict: sound.
- **`unit-roster.mjs` and `_appJourney.mjs` are unchanged.** The slice listed
  them for rewriting, but neither pins a menu address.
  - The journey opens Skirmish by its tile.
  - Its history steps land on home (the pause menu's Main menu link) or on the
    battle, never on a page that Back would leave.
  - Verdict: sound.
## Slice 06

- **A group's roles live in the table, with `null` for "the rest of its
  category".** `unitGroup` takes the first of the card's category's groups
  that claims one of its type's roles, else that category's first group. So
  IFV·APC is MBT's catch-all, a role no sup group claims lands in ARTY, and a
  label, order or split change is a table edit. *Why:* one rule reads the
  table, with no per-group code. *Alternative:* a switch per category.
  *Verdict:* sound.
- **`unitGroup` takes `Pick<UnitType, "roles"> | null`,** not a whole
  `UnitType`, so tests fabricate a role without a type. *Why:* roles are all
  it reads. *Alternative:* the spec's `UnitType | null`, which any caller still
  satisfies. *Verdict:* sound.
- **The group is per card, not per family.** No shipped family mixes a planned
  (typeless) variant with a typed one in a split category (veh, sup), so no
  family shows under two tabs today. A future family that did would show its
  planned variant under the category's first group. *Why:* the slice's seam is
  per card, and family-level machinery has no consumer yet. *Alternative:* the
  family's group from its first typed variant. *Verdict:* provisional; slice
  08 groups tiles by family and should decide.
- **"Search still spans every group" is a picker test,** not a
  `purchaseFilter` test. `filterCards` never sees groups, so the pure test
  would be vacuous; the picker test proves a search from the MBT tab finds
  another group of the same category and another category. *Verdict:* sound.
- **The picker widens from 720 to 840 px and its tabs sit 6 px apart (were
  12).** The nine tabs measured about 579 px plus about 163 px for the search
  and its gap, against the old 694 px content box. Now there are about 58 px
  to spare at 1280 px. The diamonds and the tab typography are unchanged, so
  the tabs keep every menu choice's look. *Alternative:* drop the diamonds or
  shrink the type. *Verdict:* sound.
- **The tutorial's buy still now crops the tab row in** (`.hud-purchase-filter`
  joins the stack and cards in its fit), so the still shows the tabs the
  prose names. The old still predated the tabs moving under the cards and
  still showed three of them above. *Verdict:* sound.
- **The UI gallery's picker cards** gained an APC (veh, one variant, dearer
  than the short wallet) and moved the Supply Truck to sup. The Jeep's HMG
  variant is now the test Gun Jeep type, so the Jeep family stays in one
  group. *Why:* the scene's hover and click checks need two families on one
  tab, now IFV·APC, and the unsold-variant checks moved to the MBT tab.
  *Verdict:* sound.
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
- **`useBattleSession.ts` imports game-wasm as a namespace (`gameWasm`)** and
  hands it to `composeScenarioCatalog` as the resolver. The file is over the
  size budget; naming three exports grew it past the ratchet. *Sound.*
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

## Slice 13

- **The preset JSON is read through a private flat row, `HomesteadRow`, then
  converted (`serde(from)`) into `Steads { per_road_km, attempts, steading }`.**
  serde can't combine `deny_unknown_fields` with `flatten`, and presets must
  keep refusing unknown keys. *Sound.*
- **`spread_m` belongs to `Steading`, not to the homes.** It spaces a group's
  yards along their road, which depots need too. *Sound.*
- **`Pieces` has only `Homes` for now.** `Bodies` lands in slice 17 with its
  first reader and its tests. An unconstructed variant is dead code (clippy
  `-D warnings`), and a placement arm with no caller would be untested.
  Slice 17 adds the variant and its arm in `stead`. *Sound.*
- **`furnish_yard` takes the `Yard` and the `Homes` it furnishes from,** not
  the preset block. *Sound.*
- **The steading owner is its own module, `open_country/steads.rs`.**
  `open_country/mod.rs` was over the file-size budget, and the ratchet refuses
  its growth. Its homes section (`stead`, `yard_fits`, `furnish_yard`) moved
  there unchanged, in a commit of its own, before the refactor. Slice 17's
  depots extend this module. *Sound.*
- **Each group's eligible templates are `Homes::choices`,** not inline in
  `furnish`. Which templates a group may build is the homes' rule. Bodies
  will have no templates. *Sound.*
- **`tests/open_country.rs` lost its homes and refusal tests to
  `tests/open_country/{homes,refusals}.rs`,** submodules of the same test
  binary, for the same ratchet. *Sound.*
