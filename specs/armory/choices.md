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
