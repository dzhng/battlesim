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
Decisions the implementing agents made where the spec was silent. Each gives
the choice, why, the alternative, and a verdict (sound / provisional /
needs-user).

## Slice 02

- **The flat map's owner is `MapDefinition::flat(size)` in `contract`;
  `WorldGeometry::flat(size, rules)` builds on it.** Why: both test helpers
  take extra map fields (props, forests, buildings), so what they duplicated
  is the map, not the geometry. The helper in `sim/tests/common` merges its
  extra fields over `MapDefinition::flat`, and `world_geometry.rs`'s copy
  calls that helper. Alternative: `WorldGeometry::flat` alone, leaving the
  helpers' JSON literal as a second owner of the defaults. *Sound.*
- **Only the two named copies moved.** About 30 other test files write the
  same `fog_cell_m: 8, height_grid_m: 4, slope_cutoff_deg: 35` literal inside
  whole scenario JSON (maps with props, roads, encounters). Folding those in
  is a test-wide refactor outside this slice. *Provisional:* a later
  test-cleanup pass can point them at `MapDefinition::flat`.
- **`original_carriers` lives in `weapons/operators.rs`** beside
  `assign_operators`, which moved there out of the over-budget `weapons.rs`
  in its own commit first. It takes each mount's spec (`None` for a gun not
  wanted, such as an empty one) and each member's slot (`None` once fallen),
  and returns member indices. `assign_operators` keeps its
  handoff and spare passes. Alternative: the spec's `(specs, slots)` with the
  ammo filter left in `assign_operators`, which would have needed a second
  filtered copy of the specs. *Sound.*
- **The showground's ground is 400 m square and the squad stands about its
  middle,** reported as offsets from it, so `SquadAtRest` is about (0, 0).
  Soldier ids are `slot + 1` and the arrangement draws as battle unit 0. Why:
  the largest squad spread fits with room to spare; flat ground scatters the
  same anywhere, so the test compares against `squad_spots` on its own flat
  world within 1e-9. *Sound.*
- **`Showground` keeps Rust-typed `admit` and `at_rest` beside the wasm
  `new` and `squad`.** Why: `JsError` cannot be built natively, so a native
  test of a refusal needs the `String` error. *Sound.*

## Slice 03

- **A presentation shows a battle or a showcase, never both.** `set_showcase`
  errs while a battle is loaded and `load_world` with a battle errs while a
  unit is shown. Why: the showcase's effect batch and the battle's are each
  the frame's one `effects()`; holding both would need a merge nobody needs.
  A world of statics alone (no battle) still loads beside a showcase.
  Alternative: draw both and concatenate batches. *Sound.*
- **The shown unit's rows are the presentation's own (canonical), after the
  world's and before the lab's replaceable suffix.** Why: the browser loads
  art for canonical rows (`BrowserScene`'s `sourceNames`), and when those
  rows change it rebuilds its scene, the one rule a battle's reload already
  follows; tooling rows would never have their bundles loaded. The world's
  own rows never move; the lab's shift and are reinstalled with the
  renderer's tables. Alternative (my first cut): rows in the tooling suffix,
  which would need a second art path in the browser. *Sound.*
- **Both wasm calls return the world header's address**
  (`set_showcase(fielded, input) -> u32`, `clear_showcase() -> u32`), not
  `()` as the slice sketched. Why: rows change, and `install_lab_assets`
  already hands the renderer the new header this way. *Sound.*
- **The wasm `set_showcase` reads `{units, effects}`** (what
  `Fielding::resolve` returned, minus its table names) and takes the clip
  tables staged with `add_clip_table`, as `load_world` does. *Sound.*
- **The clearing test is two tests:** clearing leaves the lab's rows and
  model as they were, and a showcase and a battle refuse to share a
  presentation. Why: one asserts rows, the other an exclusion. *Sound.*
- **The idle test allows a watcher to stand still.** The driver makes one
  soldier in `rest.watch_every` a watcher, weapon up, holding; the test asserts
  more than one soldier idles and no two share a pose, without naming which
  soldier watches. *Sound.*
- **`idle_loops_of(setup, kind)` parses the whole `SoundFrame` setup** (one
  `Setup` struct, admitted by one `setup::admit` that `Sound::from_json` also
  uses) and finds the kind's `vehicle_class` in its `unit_catalog`. Why: one
  setup document, one admission; the browser already builds it. Alternative:
  a smaller idle-only setup, a second parser of the same fields. *Sound.*
- **Posing has one owner, `presentation/posing.rs`:** the battle's
  `Loaded::frame` and the showcase draw soldiers and vehicles through
  `UnitArt` (which now carries the frame's view and detail), share a
  `Scratch` of node transforms, and build models with `Model::unit` plus
  struct update. Why: removes the battle's nine-argument model closure
  without adding lint exemptions. The showcase's rotor blur calls the
  battle's own `draw_rotors`. *Sound.*
- **No "rotors turning" feed field.** The slice planned one, but the driver
  already turns and blurs rotors by the type's `airborne` (a property of the
  type, not of where it stands), so a parked rotorcraft's rotors turn with no
  driver change and battle frames are untouched. The showcase draws its unit
  with no aircraft shadow, in the view pass, as a grounded vehicle is drawn.
  Alternative: the planned feed property, which would add a second switch for
  one behaviour. *Sound.*

## Slice 04

- **`ShowcaseStage.show` is async** (`Promise<IdleLoop[]>`), not the sketch's
  synchronous `IdleLoop[]`. Why: a shown kind is fielded from runtime files
  the page fetches. A show another supersedes resolves with none. *Sound.*
- **Showing a unit rebuilds the stage's `BrowserScene`.** The shown unit's
  rows are canonical (slice 03), so the scene's `worldCurrent()` turns false
  and the stage builds a new scene, the one rule a battle reload follows.
  Alternative: an in-place row reinstall, a second admission path.
  *Provisional:* slice 11 should time unit switches; if a rebuild is slow,
  the fix belongs to `BrowserScene` for every caller.
- **`StageOptions` adds `look` (scene settings, detail, overlay glow) and
  `failed`.** Why: `web/src` does not import the lab's `game*` settings from
  `apps/`; the page that mounts the stage owns its look. *Sound.*
- **Fielding has one owner, `web/src/battle/sim/fielding.ts` (`fieldUnits`,
  `RuntimeFiles`)**; the lab's `fieldedBattle` builds on it. *Sound.*
- **`idleLoops(options, kind)` lives in `battle-audio/soundFrame.ts`** beside
  `vehicleLoops`, reading the one `setupJson` the battle's `SoundFrame` reads.
  *Sound.*
- **The lab shows one `ShowcaseStage` per unit, three canvases side by side.**
  Why: a presentation shows one unit; three panels also give each unit its
  own framing for the gate's crops. It drives its own orbit camera (drag,
  wheel) rather than `LabViewport`, which builds its own scene. Fixture
  `showcase` uses the `game` catalog, `loading: false`. *Sound.*
- **The page loads unit art with no scenery** (`scenery: null`); the probe's
  aircraft load asks only for the aircraft's art. *Sound.*
- **Grounded is judged from each drawn model's placement plus its far-pose
  bounds** (`farPoseBounds`), not the idle clip's frame. *Provisional.*
- **Parked aircraft: gear is the `wheel_*`/`track_*` subtree,** the
  validator's own rule; a skid or belly rests on the node that carries it.
  *Sound.*
- **Reference photo:** `assets/references/ah_64_apache/ah_64d_apache-parked_rotors_turning.jpg`,
  a public-domain US Army hot-refuel photo (AH-64D, rotors turning). *Sound.*
- **The lab's sound setup takes the battle's weapon rules**
  (`catalog.rules.weapons`), not the published rows (`catalog.weapons`).
  Why: after slice 05 merged, the published rows carry `derived`, which the
  sound admission refuses; the lab came up with no stage at all. The battle
  (`soundFeed.ts`) already passes the rules, as `SoundFrameOptions.weapons`
  documents. *Sound.*
- **A hull's opening view stands back 6.5 times its larger half extent**
  (was 5). Why: a gun reaches past the hull box, and the self-propelled gun's
  muzzle ran off its panel. *Sound.*
- **Parked aircraft report (2026-11-13, 44 appearances):** no store or
  non-gear part lies on the ground. Four skid helicopters (AH-1Z, AH-6M,
  MH-6M, UH-1Y) rest on their `hull` node, because their skids are modelled
  in it, which is the rule this slice set for skids. *Sound.*

## Slice 10

- **The yard names prop kinds, and the art load gets scenery appearance names
  from the runtime catalog** (`yardScenery(doc, props, catalog)`). It keeps
  every scenery appearance whose kind draws a yard prop, through
  `propsDrawnBy`. Why: a prop kind and its art's name differ
  (`crate` → `crate_stack`, `tooth` → `dragon_tooth`), and only the art
  catalog knows the names. Alternative: author appearance names in
  `yard.json`, a second copy of the catalog's mapping. *Sound.*
- **The lab reads `/catalog.json` once before its art load** to get those
  names. *Provisional:* slice 11 can read the catalog through
  `ArmoryAppearances`' own fetch, so `library.load` reuses the bytes.
- **`yardModels(doc, props, installed)` takes the prop catalog** (the sketch
  had `(doc, installed)`), because mapping a kind to its scenery is the prop
  catalog's job. Each prop is drawn by the first installed static appearance
  of no region whose kind draws it, at its authored size. Catalog props carry
  no size, and the art's footprint is the box it is authored to. A modular
  prop (sandbags, barriers, fence) is one module per entry. *Sound.*
- **`Spot` is `SpotPlace` plus its `id`. `CameraPose` is an orbit (target,
  distance, pitch, yaw); the lens is the game camera's.** *Sound.*
- **`ground.half_m` is how far the ground reaches** (250 m, as far as the
  bench's). **`plainWorld(tint?, reach?)` is the one plain flat world**, used
  by the facade lab, the showcase and the yard. `benchGround` lost its
  `marked` flag, because `plainGround` now draws the unmarked ground.
  *Sound.*
- **`admitYard` checks the document's shape and that every body names a
  spot**, as `fixtures/README.md` asks of authored data. The layout rules
  (the props' kinds, the clear discs, the fit) are the tests'. *Sound.*
- **`ShowcaseStage` takes `models`**: static models standing round the unit,
  drawn from the stage's appearances. Slice 11 moves `appearances` to
  `show`, and the yard's art is in every selection, so the models still
  resolve. *Sound.*
- **In yard mode the lab still loads art through the shared game library**
  (`useGameAppearances`), not `ArmoryAppearances`; slice 11 moves it. Its
  default picks add the first fixed-wing type, so the jet is checked too.
  *Provisional.*
- **Layout.**
  - Three spots on one ground: the motor pool at the origin, the apron 48 m
    east, the tents 34 m west.
  - The clear radii are 8, 15 and 9 m, sized by the largest body each takes:
    6.4 m (S-300), 13.8 m (Su-34), and about 6 m for a squad's scatter.
  - Each spot's dressing stands on a ring behind and beside its camera's
    line of sight, kept inside the middle three fifths of the frame, which
    the Armory's panels leave visible.
  - The ground is a dusty khaki.

  *Provisional:* the human judges the composition.
- **References are real photos only.** WARNO and Broken Arrow armory screens
  are not stored, because they are not freely licensed; the chosen mock
  (`assets/mock/a-tank.png`) stands in for the genre's framing. *Sound.*
- **The scene's dressing check covers each spot's own props** (within its
  clear disc plus 10 m). The other spots' props lie outside the camera's view
  and are culled. *Sound.*

## Slice 08

- **Resliced into two commits,** as the slice allows: the sheet model with its
  tests, then the page with its visuals. *Sound.*
- **A family's group is its first typed variant's (`armoryRoster`).** A family
  with no typed variant takes its category's first group, as `unitGroup`
  places a planned card. *Why:* one tile per family must sit under one tab;
  the per-card rule (slice 06) would split a future family that mixes a
  planned variant with a typed one in a split category. The picker keeps the
  per-card rule; no shipped family differs between the two. *Alternative:*
  group per card and show such a family under two tabs. *Sound.*
- **The page API slice 11 mounts into is `ArmoryPage({ cards, stage })`.**
  `stage(card, faction)` renders the centre for the open card under the open
  faction (L11); absent, the backdrop shows through. `cards` is the game's
  roster on the menu (`ArmoryVisit`, under `SessionCatalogScope set="game"`)
  and a fixed set in the UI gallery. *Why:* the page stays free of 3D
  imports, and the gallery pins it without a stage. *Alternative:* slice 11
  edits the page to mount its stage directly. *Provisional:* slice 11 may
  prefer to pass the whole selection.
- **`useArmorySelection(roster)` takes the roster** (the spec's seam took
  nothing), since the fallback and the group of a card are roster facts. A
  faction tab keeps the open card where that faction fields it, else opens
  its group's first tile (or its first non-empty group's). A card opened by
  address opens under its first faction. *Sound.*
- **Bars are magnitudes:** every figure's bar is its value over the largest
  of that figure in the same body's roster, times included (a slow reload has
  the long bar). *Why:* one rule, and the number beside it says what the bar
  means; "the best fills its bar" holds where more is better. *Alternative:*
  invert time bars so a full bar always means better, which makes a 5 s bar
  shorter than a 1 s one. *Provisional:* the user judges the look.
- **Which figures get bars:** road and off-road speed (cruise for a flier),
  sight per band, visibility, and each ammunition kind's damage,
  penetration, aim and reload. Hull HP and armour are plain numbers; reach
  is per band beneath each kind, so a range bar would say it twice. *Sound.*
- **`concealment` reads as "Visibility ×n".** The catalog's figure multiplies
  how far the unit is seen (infantry 0.5, a helicopter 2.8), so "Concealment
  2.8" would read backwards. *Alternative:* the mock's "Concealment".
  *Provisional.*
- **Stat names and units are one table, `SHEET_STATS` in `unitSheet.ts`.**
  `SheetStat.key` is its key; the page reads the label from it. *Sound.*
- **`SheetMount` carries `icon`** (its first kind's weapon icon id), and a
  planned card's planned weapons are mounts with no kinds, named in caps as
  `equipmentName` names real ones. *Sound.*
- **`equipmentName`'s `rules` parameter narrowed to `Pick<PanelRules,
  "weapons">`** so the session catalog passes as it is. Its body is unchanged.
  *Sound.*
- **Capabilities are short phrases** ("Carries 8 men", "Deploys in 10 s",
  "Supply 500", "Trophy: 4 charges", "Flares ×6", "Radar", "Finds radars
  within N m", "Afterburner"), written by `unitSheet`. *Provisional:* wording.
- **The tile is the picker's family card (`hud-army-card hud-purchase-family`
  and `UnitFace`), wider, with the family's name above its price.** A family
  with no typed variant is the picker's `unavailable` card, greyed and
  dashed, its price plain. *Why:* one concept, one look; the Armory browses by
  name where the picker browses by silhouette. *Sound.*
- **Prices stay in the credits gold** on typed cards and plain on planned
  ones, as the picker colours what can and can't be bought. *Provisional.*
- **The tabs are the picker's (`hud-menu-choice` with `role="tab"`)**, and
  the selected-tab rule in `hud.css` now names `.armory-tabs` beside
  `.hud-purchase-groups`. Empty groups and factions are disabled, not hidden,
  so the nine tabs never move. Variant chips are the menu's variant choice
  (`hud-menu-choice`, `role="radio"`, a diamond filled when chosen). *Sound.*
- **The fires-regardless mark is a new generated HUD icon,
  `fires_regardless`** (a gunsight ring and ticks round a round), with U7's
  sentence as a focusable `ro-cmd-tooltip`, the command bar's tooltip look.
  On a mount with one kind, the mark and the rounds stand on the mount's
  head; with several, on each kind's. *Sound.*
- **A typed card that isn't sold reads "Not yet on sale"; a planned card
  "Planned"**, with "Not yet in battle: these are its planned weapons." The
  card's `disabled_reason` is developer text and is never shown (U3).
  *Provisional:* wording.
- **The page's plate is the whole screen and clear.** The list is a panel in
  the plate's left column with the page head on top of it (a subgrid), the
  sheet a panel in the right column, the stage between. Columns scale:
  `clamp(300px, 22vw, 400px)` and `clamp(360px, 26vw, 480px)`. *Why:* the
  head stays the shell's (`PageHead`), and a fixed-position panel can't
  escape the plate's `backdrop-filter` containing block. *Sound.*
- **`PageHead` is exported from `MenuVisit.tsx`** so the UI gallery draws the
  Armory under the same head as the menu. `PageView` keeps focus and Escape.
  *Sound.*
- **The gallery's planned card borrows the first planned card's id** for its
  silhouette; its name and planned weapons are fabricated. Every other
  gallery card is a test unit. *Why:* a silhouette is keyed by card id and no
  test unit is planned. *Alternative:* a fabricated id with no silhouette.
  *Provisional:* a new first planned card changes the picture.
- **`ArmoryVisit` is named in `uiCoverage`'s exemptions:** it draws the
  game's roster, which grows; `ArmoryPage` and `ArmorySheet` are pinned by
  the gallery's `armory` (1280 × 800, a squad and a planned tile) and
  `armory-1920` (a hull with two variants). *Sound.*
- **The `armory` scene drives the real menu without pictures:** tile,
  picks, refresh, unknown unit, Escape. *Sound.*
- **The Armory tile stands third, after Tutorial and before Settings,** so
  Settings stays the small last tile. *Sound.*
- **Key art: Duet CLI through `bunx @duetso/agent model`,** the same model
  (`openai/gpt-image-2.5-flare`) at 1536 × 512. The first prompt's wide shot
  stood out of the set (the unprimed critique picked it as the new tile by its
  small subjects), so a second prompt asked for a tight, low three-quarter
  view with the vehicles filling the banner's height; of its two candidates,
  the one with the whole tank in frame was chosen. Processed as the others
  (`ffmpeg -vf scale=1120:-2 -q:v 3`, which reproduces `skirmish.jpg` to
  within a byte). *Sound.*
- **The card's description shows only when it adds to the head.** 119 of the
  game's 152 cards carry their variant's name as their description, which the
  head already shows. *Alternative:* the type's description, which matches the
  card's for every shipped card. *Sound.*
- **After the unprimed critique:** group tabs sit in even columns with their
  count of tiles (the mock's "REC 7"), faction tabs are boxed and larger, so
  the two levels read apart; a tile says "PLANNED" or "N VAR" in its top
  corner, with its art on one line across tiles; a kind's rounds carry a dim
  "RDS"; reach chips are caps, as the sheet's other labels; and a kind's bars
  start and end on the sheet's one bar column. Its notes on per-figure bar
  scales are the given (U9); its note on bar direction is the magnitudes
  choice above, left to the user. *Sound.*

## Slice 11

- **`ShowcaseStage.show(unit, seed, art)` takes a `StageArt`
  (`{ appearances, runtime }`), and `create` is synchronous and takes no art.**
  `ArmoryAppearances.show` returns that same type; slice 09's `ArmoryArt` is
  gone, so there is one type for one concept. The stage builds no scene
  until its first show. *Why:* `create` had no art to draw before the first
  pick, so building a ground-only scene there only to rebuild it at once
  would be a wasted admission. *Alternative:* keep `ArmoryArt` as a second
  name for the same shape. *Sound.*
- **`StageOptions.models` is a function of each show's appearances**
  (`(installed) => readonly LabModel[]`). *Why:* the yard's props are
  resolved against installed art (`yardModels`), and the art now arrives per
  show. *Alternative:* load the first selection's art before `create`, which
  ties the stage's construction to a pick. *Sound.*
- **A superseded rebuild is dropped by its show's number**, not by whether a
  scene is already installed. The old guard could keep an older scene and
  drop the newer one when two shows' rebuilds finished out of order. *Sound.*
- **The showcase lab is on `ArmoryAppearances`, one per panel, in both
  modes.** `useGameAppearances` and `gameRuntime` left the lab, and the yard's
  scenery names come from `ArmoryAppearances.catalog()`, so the art load
  reuses the catalog bytes. The aircraft probe (`parkedAircraft`) keeps the
  shared library, because it loads every aircraft at once. *Sound.*
- **One owner for what both pages build a stage from:** `armory/stageInputs.ts`
  (`STAGE_LOOK`, `useStageFielding`, `useShowground`, `SQUAD_SEED`), and
  `armory/stageProbes.ts` (drawn models, grounding, world and screen boxes)
  for both pages' scenes. *Sound.*
- **`plainWorld` moved to `apps/battle-lab/src/plainWorld.ts`,** with no
  top-level await. It never needed the presentation module (`bareTerrain` is
  plain data), and the Armory is in the menu's bundle, so importing
  `benchWorld` would have made the menu's module graph wait on the
  presentation load and pulled the bench's posing code into Armory code.
  *Sound.*
- **The yard's ground reaches 2 km (`yard.json` `ground.half_m`), and a plain
  ground's cells scale so the mesh stays a hundred cells a side.** At 250 m
  the ground's edge read as a band under the horizon. At 2 km it meets the
  sky in the haze, and the shadows are unchanged on the screenshots. *Why
  here:* the slice was asked to settle the band with the backdrop.
  *Provisional:* slice 16's military ground may set its own reach.
- **The stage draws the whole screen behind the glass panels**, with the
  unit framed in the centre cell (`armoryCamera.ts` `framedOrbit` slides the
  target so the orbit's centre falls in that cell). *Why:* the mock's layout
  A has the yard filling the screen behind the panels, and the first
  unprimed critique read a canvas confined to the cell as "two scenes stuck
  side by side": a midday yard beside the amber backdrop. The backdrop shows
  only where no stage is drawn (a planned card first opened, a failure).
  *Alternative:* a framed viewport in the cell. *Provisional:* the human
  judges the composition.
- **A switch holds the last frame drawn and blurs the stage while it is
  under way.** Each show rebuilds the scene, and a freed renderer leaves the
  canvas clear, so the amber backdrop flashed through on every pick. A 2D
  canvas behind the live one holds the last frame. A `data-switching` mark
  on the cell blurs, dims and slightly scales both canvases until the next
  unit stands, so the pause reads as a change, not as the unit just left.
  *Alternative:* an in-place row swap in `BrowserScene` (slice 04's open
  note), a renderer change for every caller. *Provisional.*
- **A planned card asks for nothing:** the stage shows the yard alone from
  the art already on show, or nothing if no unit was shown yet. *Sound.*
- **Failures throw into `ArmoryStageBoundary`,** a React error boundary that
  draws the error in the cell and clears itself on the next pick, which
  builds the stage again. Asynchronous failures (art, fielding, scene
  admission) are thrown from render into the same boundary. One surface, and
  the shell's cover lifts on a failure. Copy: "This unit can't be shown",
  the error, then "Its sheet is still here. Pick another unit, or reload to
  try again." *Provisional:* wording.
- **`MenuShell`'s `LoadingTasks` wraps the plate too**, so the stage's
  `renderer` task holds the first cover until it draws. No other menu page
  reports a task. *Sound.*
- **`AppAudio.stats()` reports `showVoices`,** the show loops sounding, for
  the scene's "voices on /armory, none after Back" check. *Sound.*
- **The scene's probe is `window.__armory`** (`armoryProbe.ts`): the unit on
  show, its spot, its ground gaps and screen bounds, the files its art holds,
  the switches, and the voices. The voices and switches outlive the stage, so
  the count can be read after Back. It follows the battle's `window.__lab`,
  which also lives on player pages. *Sound.*
- **The load line (S5) is what `ArmoryAppearances` holds for a selection**
  (`held()`: the files the newest selection read, with the yard and the
  catalog), not resource-timing bytes. Resource timing also counts the
  menu's background prefetch of every runtime file, which showed 200–300 MB
  per switch that the Armory never asked for. *Sound.*
- **The app journey's Armory round trip compares with the menu alone:** the
  allocations a visit leaves alive must be no more than the backdrop leaves
  over as long a stretch on its own, because the backdrop's battle keeps
  allocating behind the stage. *Sound.*
- **The `armory` and `armory-1920` UI baselines are not re-recorded.** The
  gallery pins the page without a stage (slice 08), and the live stage
  animates, so it can't be a pixel baseline. Every `ui` baseline still
  matches on this tree. The stage's pictures are the `armory` scene's
  evidence. *Sound.*
- **The unit tests drive the real menu under `AppShell`** (`visitShell` in
  `tests/support/router.tsx`), because the stage needs the app's audio. The
  3D stage, its art and the squad scatter are stand-ins there; the scene
  judges the drawn page. *Sound.*

## Slice 12

- **`camera_armory` is a section of `fixtures/presentation.json`, admitted in
  Rust** (`ArmoryCamera` in `crates/presentation/src/settings/ui.rs`).
  Validation checks:
  - the lens;
  - a pitch range that rises within (−π/2, π/2);
  - framing with `0 < far < open <= near <= 1`;
  - positive speeds;
  - the same clearance checks as the battle camera, now one shared
    function (`clearance`).

  *Why:* the document refuses unknown fields, and the slice names it as
  the config's home. This lane therefore changes Rust and built its own
  wasm. *Sound.*
- **`CameraPresentation` carries `pitch_limits`, and `CameraController`
  reads them from config.** The battle's config sets them to
  `PITCH_LIMITS`, its unchanged values, in `gameCamera.ts`; every other
  battle rig spreads that config. *Why:* one controller with each camera's
  own range, as the slice asks, and no second clamp path. *Alternative:* add
  `pitch_limits` to the battle's fixture `camera` too, which would move a
  battle constant into data without being asked. *Sound.*
- **The seam is a class and a hook, not one hook returning a camera.**
  - `ArmoryCamera` holds the orbit, framing and clearance state, and gives
    the camera for a frame (`presented(view, cell, width, height, dt)`).
  - `useArmoryCamera(canvasRef, camera)` wires left-drag and the wheel on
    the canvas alone.

  *Why:* the stage draws from its own frame loop, outside React, and a
  unit's bounds arrive when its show resolves, not as a prop.
  *Alternative:* the slice's `useArmoryCamera(canvas, bounds, spot):
  PresentedCamera`, which would re-render React on every frame. *Sound.*
- **Framing is fitted per unit and per view angle.**
  - At a given orbit angle, each framing is the distance at which the
    unit's rest-pose box spans that fraction of the centre cell across and
    up: every corner, from its own depth along the view.
  - The wheel reaches from `near` (1.0: the box fills the cell, nothing
    clipped) to `far` (0.4); a unit opens at `open` (0.85).
  - Turning keeps the zoom, and turns that would clip the box push the eye
    back.

  *Why:* a fixed spot camera is sized for the largest body. A bounding
  sphere, the first try, framed a helicopter loosely, because its rotor
  sweep's diagonal sized it. *Provisional:* the rotor sweep still makes a
  helicopter's box square round its disc, so its nearest zoom is less
  tight than a tank's.
- **A unit's bounds come from data, not from a drawn frame.**
  - A hull: its model at rest, from the art, as it stands on its spot.
  - A squad: the scatter the simulation gave, each soldier a soldier tall
    (`rules.physics.soldier_height_m`), half a metre round his feet.

  *Why:* the presentation culls by the camera, so a frame drawn from the
  previous spot's camera may miss the new unit. *Sound.*
- **The camera opens at the spot's yaw and pitch (`yard.json`), aimed at
  the unit's centre**; the spot's own target and distance remain the yard's
  view for a planned card. *Sound.*
- **The eye keeps clear through renderer-core's clearance** (`resolve`)
  over the unit's box and each yard prop's authored box, on flat ground at
  height 0. The lowest pitch, −0.15 rad, is held above the ground by the
  clearance, so at the nearest zoom the unit stands against the sky with the
  horizon under it. *Sound.*
- **Speeds and limits (data, provisional):** a drag turns 2.5 rad per
  viewport height, as the battle's middle-drag does. Zoom speed 0.0012 per
  wheel pixel. Pitch from −0.15 to 1.1 rad: the unprimed critique read the
  first ceiling, 1.25, as a flat map view. Near plane 0.2 m. Clearance
  margin 0.3 m. *Provisional:* the feel is the user's to tune.
- **No inertia.** The clearance spring already eases the eye. *Provisional.*
- **The camera tour drives real input** (mouse drag and wheel on the stage)
  through 8 yaws × 3 pitches × the nearest and farthest zoom, for one unit
  on each spot. It checks the eye against the unit's bounds, every prop box
  and the ground, from the probe's camera readout. *Sound.*
## Slice 14

- **The rows' bodies** (`fixtures/props/military/{works,camp}.json`), from
  `tweak-mechanics`. Every row is ordinary (no garrison, fixed `hp`) and
  `systems_only` with a stand-in tint until slice 15's art. *Provisional:*
  balance is the closeout's.
  - `hangar`: immovable, blocks everyone, stops rounds, occludes, heavy cover
    (like `garage_row` and buildings), metal; `hp` 800 at `armor` 0.5, so a
    tank's HE takes about sixteen direct hits and an artillery barrage brings
    it down, while machine guns barely mark it. It falls into `hangar_ruin`
    1.5 m high.
  - `hangar_ruin`: a twisted steel pile. It still **blocks infantry and
    vehicles** and gives **light** cover from behind, stops no rounds and hides
    nothing; terminal. Why blocking: the soldiers sheltering at the hangar's
    wall keep their cover where they stand, and a viewer would not walk a
    squad or drive a jeep across a collapsed hangar's girders. Alternative: a
    ground body like `rubble` that covers who stands in it; it lost because
    every soldier at the wall would have to re-cover into the wreck.
  - `revetment`: an earth berm. Immovable, blocks, stops rounds, occludes,
    heavy cover, and **no `hp`**: earthworks soak shelling, like dragon's
    teeth. Alternative: `hp` into rubble, rejected as a berm vanishing under
    tank fire.
  - `hesco`: earth-filled baskets, chest-high. Heavy (no tank shoves it),
    blocks, stops rounds, heavy cover, does not occlude (like sandbags, a
    fighting position soldiers fire over); `hp` 300 into `rubble` 0.4 m, so
    HE bursts a basket open. Modular, like sandbags.
  - `tent`: light canvas. Blocks movers, so a vehicle flattens it (`topples`);
    occludes (no one sees through canvas), stops no rounds, gives no cover;
    `hp` 30, removed when shot apart.
  - `flagpole`: light, topples, blocks movers, metal, no cover, no stop, no
    occlusion; `hp` 20, removed.
  - `supply_net`: per L3, it blocks no one, stops nothing, hides nothing and
    conceals nothing (`conceals` 0). It has `hp` 30, removed, so a burst on the
    stack it covers tears it away rather than leaving a net over nothing.
- **The tests carry their own `test_*` rows** (`crates/sim/tests/shelter.rs`
  pushes a props document onto the test catalog, as `with_units_at_limits`
  does for hulls). No test row ships in `fixtures/`, and no test reads a
  military row. *Sound.*
- **`a_body_with_no_cover_and_no_stop_never_shields` compares whole
  firefights:** the blue soldiers' health, tick by tick, is the same with the
  screen in front of them as without it. `bodies.rs`'s street-lamp test pins
  the same columns on a geometry query; this one proves it through the
  battle. *Sound.*
- **The visual gate ran through the scenario builder's play mode** with a
  package written by a throwaway script
  (`throwaway/armory/14/hangar-shot.mjs`), the map placing the rows, blue
  squads watching and HE bursts on the hangar. No lab or test map was added
  for a stand-in check that slice 15's art replaces. *Sound.*
- **The parity moves.** Only `fixtures/parity/exact-numbers.json` moved: its
  record embeds the catalog's documents, which gained the two military files.
  Its three state digests are unchanged, and the browser's paired test passes.
  The publication-reader parity, the `ConfigMismatch` tests and the replay
  tests pass with no bless, so the state digest is unchanged (S3). *Sound.*

## Slice 17

- **`tweak-mechanics`, in short.** A depot by a country road is a strongpoint
  for whoever reaches it first: a hangar's walls and the berms give heavy cover
  and hide what lies behind them. Three things keep it fair and sane. The
  halves stay even, so each side has as many. Its yards keep off kept approach
  corridors, as homes do (`yard_fits`), so it never closes the main approach.
  And its yards keep off the carriageway, so it never blocks the road. Nothing
  here is a new rule: the rows' bodies (slice 14) decide every effect. *Sound.*
- **A depot piece is a body of `street_props.bodies`,** the presets' one table
  of placed boxes, by kind. The depot rows name only kinds (and a count, and a
  net), never extents. The homes' and field cover's own `half_extents_m`
  copies predate this and are left alone. Alternative: extents in the prop
  catalog, which has no box. *Sound.*
- **A piece is `count` bodies side by side under an optional `net`.** The
  supply piece is two or three pallet stacks under a `supply_net`, so a net
  only ever covers supplies (L3) because only that piece names one. Admission
  refuses a row wider, deeper or taller than its net. Alternative: a net placed
  by a rule naming supply kinds, which would read a prop's id. *Sound.*
- **The groups** (delegated): a hangar, a revetment and supplies (weight 2);
  two tents, a flagpole and supplies (weight 2); HESCO, a tent, a revetment
  and supplies (weight 1). The hangar group is weighted up because its big
  yard fails more often; on 39 skirmish depots the three stood 10, 16 and 13
  times. Yards are 3 m deep front and back and 2 m at the sides, at most
  0.5 m apart, so the slabs join into one pad. Stations are 1500 m apart, and
  40% of depots stand at the end of a 90–150 m lane, long enough for a
  hangar group's row of yards. *Provisional:* the human judges them.
- **The hangar body is 24 × 16 × 8 m** (Quonset size, in
  `street_props.bodies`). A 30 × 20 m one rarely found a straight enough
  stretch of road for its yard. *Provisional:* slice 15's art is fitted to
  this box.
- **A depot's yards are paved** (`SurfaceKind::Paving`, the map's hard ground
  for yards and courts, no way through). The first battle-camera critique read
  the bare bodies as boxes dropped in a crop field; on a shared slab they read
  as one compound. Movers cross paving at its own speed factor. *Sound.*
- **A depot piece that finds no room tries again further along** (up to four
  tries per piece, moving on by `parcels.lot_step_m × 4`), where a home is
  tried once. Without it, no hangar group ever stood whole. Homes are
  unchanged. *Sound.*
- **`per_road_km` 0.3, tuned on the maps players get** (delegated, a
  tripwire). The lobby's maps are the skirmish profile (1.8–3.6 km), whose
  country roads are short: at 0.08 only 1 of 36 skirmish maps held a depot.
  At 0.3, on seeds 1–3 of each type, Small maps hold 0–1, Medium 1, Large 1–2
  and XL 0–2. The larger standard-profile maps, which only the developer
  tools, tests and benchmark use, hold 1–9; `apart_m` 1500 keeps them spread.
  *Needs-user:* "about one per battle, two on some large maps" is the result;
  "too many" or "too few" is this one preset value.
- **A depot is one compound.** A home's yard keeps `clear.yard_m` from every
  other yard, its own group's included. A depot's pieces' yards may touch each
  other, and keep that distance only from everything else. `Ask.own_yard`
  became `own_from`: the yards from that index on are the asker's own. Homes
  pass the current yard count, so they are unchanged. Without it, almost no
  depot found room for all its pieces. *Sound.*
- **A depot stands on one side of its road.** Homes in a group of more than
  two, or on a lane, pick a side each, and the first battle-camera shot showed
  a depot copying that: a hangar on one side and its berm and supplies on the
  other, which reads as things dropped along a road, not a compound. A depot's
  pieces all take the group's side. *Sound.*
- **A depot stands whole or is left out:** if any piece finds no room, its
  lots, yards, bodies and tally are rolled back. A home group keeps as many
  homes as fit. "3–4 props per depot" is the depot's identity. *Sound.*
- **Depots are placed after homesteads, on their own stream
  `country/depots`.** Homes are unchanged, and maps where no depot stands are
  byte-identical (proved below). Their halves are tallied by the middle of the
  depot's first lot, which `measure` reads back from the lot ids
  (`country/depot-<n>/lot-<k>`); `fairness.depots` is 1. *Sound.*
- **Stations are kept apart per kind,** so a depot's `apart_m` is measured
  from other depots, not from homesteads. Their yards still keep apart. *Sound.*
- **The parity proof.** A control run (this code, depots off, the previous
  presets and frozen rules) reproduced all 17 frozen records byte for byte.
  Then each request was re-pinned by hand to `layout-presets-21` (the
  stale-revision record to `-20`) and generated again. Every record kept its
  exit code. The successful records moved only by depots and what the open
  country's later passes do round them: lots, props, forests, lanes, one
  kept approach (metro XL) and the identity hashes. Every successful record
  now holds one to seven depots. The refusals are byte-identical but for the
  stale record's message, which names the revisions. Then `BLESS_PARITY=1`.
  *Sound.*
- **`open_country/mod.rs` split.** `measure` moved to
  `open_country/measure.rs`, and the loose bodies (`body`, the field cover)
  to `open_country/bodies.rs`, each in a commit of its own, to keep `mod.rs`
  under its ratchet. `presets.rs`
  passes itself to `open_country.errors`, which shrinks it, so the depot rows
  can be checked against `street_props.bodies`. *Sound.*
- **Map tools.** The plan picture draws a depot's yards in fuchsia, a hue
  no other layer uses, rings each depot on a whole-map view as it rings a
  bridge (a depot is a few pixels there), and adds a legend entry. Each depot is a workbench feature (`depot-<n>`, rule group
  `open_country.depots`) with its own crop. The field help maps each key of a
  flat steading row (homesteads and depots) to the type that documents it;
  the homesteads' help had pointed at the removed `Homesteads` type. *Sound.*
- **The lobby's map picture does not show depots.** It draws the compiled
  map's buildings, roads, woods and water, and no props, so a depot (all
  props) is absent, as field cover is. The developer plan picture shows them.
  Drawing big bodies in the lobby would be a player-facing look of its own,
  and it would move every lobby picture with a court or garage row in it.
  *Needs-user:* whether a player should see depots on the lobby map.
- **The distant-impostors receipt is not re-recorded.** Its digest
  (`specs/distant-impostors/baseline.json`) is the benchmark's Metro XL
  seed 4 stress battle at tick 90, recorded in the browser on 2026-10-21. A
  native run of the same recipe (generate the map, `city_scenario` on the test
  catalog, `Battle::new` with seed 4) gives `d04d85cb8dc310c4` before depots,
  not the receipt's `60ede87ba60aab0b`. So either the native path differs from
  the browser's or the receipt was already stale. With depots it gives
  `93e60b342dbfed4a`. An unproven digest is worse than a stale one.
  *Needs-user / closeout:* re-record it with the browser benchmark at slice
  18's full run.

## Slice 15

- **One concept-named script, `military.py`, for the seven bodies** (the
  courts precedent: one exporter per prop family, a function per kind
  returning its box). Its shared helpers (`grid`, `bar`, `tube`, `body`) stay
  in it; nothing in `parts.py` changed. *Sound.*
- **Each military prop lints itself (`--lint`)** with `vehicle_lint`'s floating
  and ground checks, and its overlap check between the model's bodies (the
  root's children: canvas and rigging, flag and pole, net and poles). The
  lint caught real defects on the way (pegs and scraps floating, the net
  cutting its spreaders, door rails off their leaves). *Why:* the slice asks
  for "not floating, grounded, no intersections", and the vehicle lint
  already owns those checks. *Alternative:* a prop lint of its own, a second
  owner. *Sound.*
- **15a and 15b land in one commit.** `catalogFootprints.test.ts` holds a
  destroyed body's art to its remains' art: once the hangar draws, its ruin
  must too. Each keeps its own verdict (`decisions.md`). *Sound.*
- **15a, the hangar: an arch hangar with fine (75 mm) corrugation in the
  texture, not the geometry.** The vault runs its own UVs so the `roof_sheet`
  corrugations go over it from curb to curb; seam bands every 4 m, skylight
  sheets down each side, end walls recessed 0.5 m under the vault's edge,
  braced sliding leaves on the front with a wicket door and a plain "3", a
  door and two windows at the back. *Why:* two critiques read geometric ribs
  (0.75 m, then 0.5 m) as "a ribbed barrel", and the Quonset references show
  fine corrugation that reads as one shell. Olive paint, not galvanised, so
  it stands off a khaki yard. *Provisional:* the human judges it.
- **15b, the ruin: what survives says which hangar it was.** The vault's
  feet still stand on their cracked curbs, torn off at uneven heights and
  peeled in or out; bent rib stubs rise from them; the rest lies crushed into
  creased folds of the same olive sheet, torn open over a burnt bed with
  broken concrete; the back wall is a jagged stub; the front's leaves lie
  thrown over the wreck under the fallen track beam. Local scorch, not soot
  everywhere: a first pass's low dust film turned the whole ruin tan.
  *Provisional.*
- **The ruin's kind takes a rubble allowance (`hangar_ruin` in
  `scenery.ts`, `rubble_m` 1.5):** sheets that slid off over the curbs lie
  flat round its plan, as a wreck's thrown plate does (`wreck`, 1 m). They
  block nothing, so the body's box is unchanged. *Sound.*
- **`fixtures/catalog.json` was not re-blessed through Rust.** Building the
  sim test crate costs several GiB, and the disk stood under 8 GiB free. The
  bless only drops each drawn row's `"status": "systems_only"` from its
  appearance (keys sort, so the line and its comma go). I made that edit and
  proved it with the simulation's own resolver through the base wasm:
  `throwaway/armory/15/verify-catalog.mjs` checks that the file's merged
  document carries the prop files' appearances and that resolving it gives
  the file's props exactly. *Closeout:* the full run's
  `catalog::view::the_browsers_catalog_view_is_current` is the byte check.
- **15c, the revetment: a steel-bin revetment, not a bare earth berm.**
  Galvanised corrugated faces 3 m apart in a frame of posts a metre apart,
  a heavy sill and a top rail, tied across, the ends framed the same way,
  earth to just under the rim, the bay number stencilled on the +X end.
  *Why:* bare earth cannot stand 3.2 m on a 3 m foot (a first, bermed
  version read as "a loaf"), and every revetment reference is this kind of
  earth-filled steel bin. The row's body (immovable, no `hp`, earthworks
  soaking shelling) still describes it. *Alternative:* sloped earth, which
  needs a box twice as wide (slice 14's row). *Provisional:* a critique
  still reads a container in it at a distance; the yard puts it beside
  vehicles in slice 16.
- **15d, HESCO: one six-cell module, the battle's repeat unit** (the row is
  `modular`, like sandbags). Each cell's liner pillows out between creased
  joints with a stack of coil turns in each crease; the rim dips between
  the joints, the liner folds in over it, a divider crosses at each joint,
  and the earth lies below the rim, heaped differently per cell. The ends
  are half joints and end faces 5 mm inside the box, so two modules meet
  1 cm apart, never in one plane. A new recipe, `hesco_liner` (geotextile
  under a 75 mm weld mesh), in `textures_military.py`, which `military_parts`
  imports: `textures.py` is over its size budget and may not grow. One albedo
  and two surface texture layers.
  *Provisional:* the critique still finds the module boxy and its coils
  faint at a distance.
- **15e, the tent: a ridge-roofed wall tent** in olive cloth (the `nylon`
  recipe, tinted; the shared `canvas` recipe's folds read as bark at this
  size, and it belongs to the vehicles' tarps), slack between its eave poles,
  a scalloped valance, guys out to pegs inside its box, pole pins through the
  ridge, and the front door open: the dark inside between flaps folded back
  and tied. The walls stand in from the box (1.72 m of its 2.5) so the guys
  fan out. *Provisional:* the reference camp's tents are square with pyramid
  roofs; a ridge tent is as military and fits the 6 x 5 m body better.
- **15e, the flagpole: the flag stands past the pole's box, by the catalog's
  footprint tolerance (`tolerances.footprint_m` 1.6 on its appearance).** The
  body is the pole (16 cm); the flag flies 1.5 m out at 7 m, over everyone's
  heads, so it blocks and hides nothing, as a wreck's thrown plate doesn't.
  The flag is one cloth coloured by height (olive, a sand band): generic, no
  nation. Halyard runs, cleat, snap hooks, ball truck, sleeve and a small
  pad. *Alternative:* a wider body in `street_props.bodies`, which would move
  generated maps for a cloth. *Sound.*
- **No idle cloth motion.** A static scenery bundle carries no motion, and
  waving flags or canvas would be a new renderer feature, battle-wide. The
  flag is caught mid-ripple instead. *Provisional:* for the user, if a still
  flag reads wrong.
