# Units: roster-only content and reference-built models

Two halves of one goal: the units a player sees are the official roster, and
they look like the real vehicles and soldiers.

- **Part A, roster-only content (slices 01–08).** "Village" was the first test
  arena and the pre-roster generic units (`tank`, `jeep`, `supply`, `rifle`,
  `recon`, `at`) its army. Both still reach the game. Part A converts every
  such place to roster units or deletes it, keeps the generic units only as
  explicitly named test units, labels every fixed map as test or menu
  content, and puts the menu reel in roster looks with every shot unchanged.
- **Part B, reference-built models (slices 09–18).** Every roster unit and
  every disabled card is rebuilt from committed reference photographs in the
  [chosen look](#the-look), with its own wreck, real detail tiers, and
  materials that read as what they are, within budgets measured on the target
  machine.
- **Slice 19** closes out once, at the end.

## Next Agent Prompt

**Status, 2026-10-07:** in progress. Done and merged: 01, 02, 03, 09, 10 (catalog load now ~130 MiB, was ~1.2 GB). In flight in parallel worktrees (`um/slice-NN` branches): 04 (steps 1–2; step 3, session-scoped loading, now unblocked by 10), 12, 13. The earlier pass of this
spec (every disabled card has a source GLB, see [History](#history)) is
finished. The plan was walked with the user quadrant by quadrant
([the map](visualizations/unknowns-map.html)), then redrafted by four
independently biased drafters and synthesized (see [choices](choices.md)).

**Pick up at [slice 01](slices/01-presentation-by-property.md):** every roster
vehicle and weapon plays default sounds today, a bug players hear.

Order:

```
Part A:  01 ─┐
         02 ─┼─ any order; no outcome moves (02's digest move is named)
         03 ─┘
         04 session catalog ─► 05 test units ─► 06 village retired, maps labelled
                           ├─► 07 request contract
                           └─► 08 menu reel (after 05: menu units extend test units)
Part B:  09 pipeline ─► 10 transport ──────────┐
                     ─► 11 wrecks (after 04) ───┤
                     ─► 12 art rules (after 01) ┼─► 14 pilot
                     ─► 13 materials ───────────┘
         14 ─► lanes in parallel: 15 tracked · 16 wheeled · 17 infantry · 18 disabled
         all ─► 19 closeout
```

Part B may start at 09 while Part A runs; only slices 11 and 12 wait on Part A
(11 needs 04's session catalog to draw `wreck_of`; 12 needs 01's vehicle
class for budgets).

**Staffing** (user, 2026-10-06): one coordinator owns shared code (parts
library, crew module, materials and roles, validator, catalogs, transport,
wreck and tier contracts, budgets, icons, bindings, merges) and runs Part A,
slices 09–13 and the pilot (14) itself. After the pilot, workers fan out in
lanes, each worker in its own worktree on one family at a time, collecting
that family's references, modelling it and its wreck: tracked (15), wheeled,
light and trucks (16), infantry (17), and disabled aircraft, rotorcraft, and
support and ground (18). Start with six workers and add more while the
machine keeps up (Blender exports are CPU-bound; GPU browser scenes run one at
a time). A worker owns only its family's script, export directory, references
and receipt, never a shared helper. `bake` re-reads every source and rewrites
the one `assets/runtime/catalog.json`, so **workers export and validate only**
(`asset validate`, a `/workbench` drop for previews); the coordinator merges
lanes in batches, pulls sources, bakes, checks and commits runtime output.
Workers share the main checkout's `web/node_modules` and, since art work
changes no Rust, the coordinator's built WebAssembly; set `BLENDER` to the
pinned 5.2.1.

Warnings:

- **No number in this spec is a requirement** (user, 2026-10-06). Every
  number here (byte limits, triangle budgets, tier ratios, dressing
  allowances, the 1600 px reference size, worker counts, and the map and kit
  download limits in `schema.ts`, raised from an earlier session's 50 MiB to
  256 MiB for that reason) was chosen by an agent, not the user. Budgets
  and limits are tripwires against gross regressions, set loosely from what
  was measured with plenty of room, never targets to optimise toward. The
  only real bars are the user's: the look, black tyres, recognisable units
  and wrecks, detail that holds up close, levels of detail that switch with
  zoom, and a battle that runs well on the Mac mini. When something exceeds a
  number and nothing visibly suffers, raise the number and note it in
  choices.md; don't spend effort squeezing under it.
- **Disk.** The machine ran out of disk on 2026-10-06 (every shell command
  failed). Pull LFS sources only for the families you touch and drop them
  (back to pointers) when done; keep scratch small.
- **Tests use test units.** A roster add, remove or edit must not break a
  test. Tests run on test or fake units (`sim::fixtures::stand_in_game` on
  main since `847c813c`, `test_game` after slice 05) unless a test says why it
  needs the roster. Don't move a test onto roster units.
- **The menu reel is a film.** It may use any units with any stats, and
  anything in it may change, but every approved shot must stay exactly the
  same ([slice 08](slices/08-menu-reel.md)).
- Re-exporting changes GLBs, not physics. Never move a mount, pivot, muzzle,
  hull extent or eye height to make art fit, and never widen a catalog
  tolerance; dressing has its own allowance (slice 12). The one decided frame
  change is the HMMWV's (slice 14). Any other frame that disagrees with the
  photos goes in [choices](choices.md) and is left alone.
- Changed unit ids, the wreck field and the request contract move digests and
  the engine id; saved replays are refused, accepted with no compatibility.
  Name every regenerated parity fixture in choices.md. A digest that moves in
  a slice meant to move no outcome (01, 03, 04, 10, 12) is a bug.
- Source GLBs and photos are Git LFS. In a worktree, pull only what you touch
  (`git lfs pull --include="assets/source/roster/<family>/**"`).
- Other sessions merge and delete checkouts mid-run; commit often.

TODO:

- [x] [01 Presentation by property](slices/01-presentation-by-property.md): vehicle class from physics, weapon base key; roster vehicles and weapons get their sounds and effects.
- [x] [02 Roster stands alone](slices/02-roster-stands-alone.md): abstract roster base soldiers; no roster soldier inherits a generic kind.
- [x] [03 Encounter owner](slices/03-encounter-owner.md): Defender/Referee move to `sim::encounter`, unchanged.
- [ ] [04 Session catalog](slices/04-session-catalog.md): one resolver, three document sets (game, test, menu); the battle session owns its catalog and loads only its bindings.
- [ ] [05 Test units](slices/05-test-units-explicit.md): `test_*` units, out of the game catalog, test art labelled.
- [ ] [06 Village retired, maps labelled](slices/06-retire-village-label-maps.md): map-free rules, village deleted, live art renamed, every map `test` or `menu`, market-town-test.
- [ ] [07 Request contract](slices/07-request-contract.md): factions required, unknown parameters refused, recipe fields gone.
- [ ] [08 Menu reel](slices/08-menu-reel.md): `menu_*` units in roster looks; exact-shot test against the recorded reel.
- [x] [09 Pipeline](slices/09-pipeline.md): exporters read the fixture catalog; shared detail parts; crew module fixed; references schema and sheet; baselines.
- [x] [10 Shared textures and download](slices/10-shared-textures-and-download.md): textures as shared files, gzip bundles, catalog-load limit, real texture-layer limit.
- [ ] [11 Wrecks](slices/11-wrecks.md): `wreck_of` on the published prop; every vehicle names its own wreck; interim wrecks.
- [ ] [12 Art rules](slices/12-art-rules.md): strict tiers, dressing allowance, unit budget rules (numbers from 14).
- [ ] [13 Materials](slices/13-materials.md): material roles, black tyres, dark glass, real-nation paint on every current model.
- [ ] [14 Pilot](slices/14-pilot.md): Abrams, Stryker, HMMWV (real frame), rifle squad; class budgets measured and fixed.
- [ ] [15 Tracked lane](slices/15-tracked.md)
- [ ] [16 Wheeled, light and trucks lane](slices/16-wheeled.md)
- [ ] [17 Infantry lane](slices/17-infantry.md)
- [ ] [18 Disabled cards lanes](slices/18-disabled-cards.md): models, wrecks and icons, still unplayable.
- [ ] [19 Closeout](slices/19-closeout.md): full check and verify once, balance report, close-spec.

Update this section before you end a pass.

## What is wrong today (2026-10-06)

### Content

- **Roster vehicles sound and move like nothing in particular.** Engine,
  track and turret loops (`fixtures/game.json` presentation sound `vehicles`,
  picked in `packages/battle-audio/src/soundFrame.ts`), the pose gauge
  (`presentation.pose.gauge`, `packages/battle-renderer/src/models/poseDriver.ts`
  `halfTrack`) and per-unit shot sounds (`fixtures/sounds.json` `units`,
  `packages/battle-audio/src/catalog.ts`) are keyed by `tank`, `jeep`,
  `supply`. Every roster vehicle falls back to the default row.
- **The menu backdrop films generic armies** (`market-town/menu`,
  `paris-corner/corner`).
- **A `/battle` address without a faction** plays the `assault` recipe
  (`fixtures/encounters.json`) with generic units. Ordinary Play always sends
  factions; only typed, shared or lab links reach it.
- **The shipped catalog carries the generic units**: `crates/sim/src/fixtures.rs`
  `catalog_documents()` reads all of `fixtures/units/`. Purchases are safe (the
  simulation admits only roster cards of the side's faction, `battle.rs`
  `preview_purchase`; the AI filters the same way).
- **The village's Defender and Referee run every encounter battle** (`battle.rs`
  builds `village::Defender`/`village::Referee` for any scenario with an
  opponent, the menu backdrop included): real behaviour under a test name.
- The village map, scenario, scripted blue, trials, routes (`/battle/village`,
  `/watch`, `/lean`, `/replay/village`), wasm exports (`village_scenario`,
  `Battle::scripted`), a `script` field in the worker protocol, the village
  replay format, `MAP_CHARACTERS` `"village"`, the default benchmark
  (`village-contact`), and `fixtures::game()`, which injects the village map
  into ~40 native test files' rules.
- Roster soldier kinds inherit stats from generic soldier kinds (`rifleman`,
  `grenadier`, `scout`, `at_rifleman`), so a roster squad changes when a test unit does.
- Village-named live art: `village_ruin` draws every collapsed building;
  `assets/source/village/` holds walls, crates, bridge decks, fences, sandbags
  and dragon's teeth; the `village` city set is the labs' box buildings.
- **Wrecks are picked by nearest size** (`propAppearance.ts:175-187`) among
  three generic wrecks: 31 roster vehicles, wheeled ones included, die into
  the generic tank's wreck and throw a turret.
- **Every page downloads about 1.3 GB.** All unit and most scenery bundles
  load with the catalog, uncompressed, each embedding its own textures (765 MB
  of copies of 18 MB of distinct textures); nothing limits the total.
- **The HMMWV is the JLTV** (same model file, JLTV frame).
- **Roster weapons get default sounds and effects**: presentation tables are
  keyed by base weapon ids, and roster units fire derived weapons.
- Counts: about 53 Rust test files, 60 web test files and 22 browser scenes
  name a generic id; 11 Rust, 28 web test and 27 scene files mention village;
  all 21 saved encounters use generic units.

### Models

[Before sheets](assets/before/README.md) render the 43 runtime vehicles from
their source GLBs:

- **Slabs.** Most roster vehicles are one extruded hull prism, a box turret and
  flat side panels; running gear is plain cylinders; no hatches, periscopes,
  lights, tow points, tools, stowage, grilles, smoke dischargers, antennas or
  weld lines. Leopard 2, Challenger 2, Leclerc, T-72/80/90 and Abrams carry some
  detail; BMP, BTR, ZBL-08, Boxer, VBCI, Stryker, LAV, ACV, CV90, Puma, Ajax,
  Tigr, Fennek and VBL almost none.
- **Cyan cubes.** Optics are `flat_paint` boxes in saturated cyan, so every
  vehicle wears glowing blue blocks. Real sight glass is near black in a housing.
- **Tyres are not black.** The rubber recipe is near black (`textures.py`
  `rubber`, ~0.024 linear), but the exporters' `textured('rubber', …, dirt=.3)`
  film it with `DUST` (0.15) up to `rise`; a tyre is wholly below `rise`, so it
  comes out two to three times brighter, about the dark-olive body's tone. A
  painted rim covering 55–70% of the wheel face finishes the job. Tracked road
  wheels are drawn wholly in rubber.
- **One paint.** Nearly every family is the same dark olive.
- **Terse sources.** Family scripts are dense one-line Python with `wheel()`
  and track belts redefined per family.
- **Exporters can't run.** They read frames from `specs/unit-roster/manifests/`,
  archived to `specs/done/unit-roster/manifests/`.
- Infantry: shared Quaternius rig and `roster/infantry_equipment.py`; uniform
  pattern, plate carriers, helmets and team weapons are plain shapes.

## End state

- **Game content** is the roster (`fixtures/units/roster/`, profiles, roles)
  on generated maps. The shipped catalog holds no concrete unit no roster card
  reaches, and a test holds it to that. Every place a generic unit or the
  village reached the game is converted or deleted; nothing is defaulted
  silently (a battle request without both factions is refused by name).
- **Test content** is named as such: test units `test_*` in
  `fixtures/units/test/`, with art labelled as test art, in a test catalog
  resolved at run time; fixed maps with `meta.json` `category: "test"`. Tests,
  browser scenes, labs that serve them, the benchmark and the endurance lab
  use test units, so roster changes move none of them.
- **Menu content** is fixed maps with `category: "menu"` and menu-only units
  (`menu_*`) wearing roster looks with whatever stats keep every approved shot
  exactly the same.
- No identifier, route, file, type or doc says "village" except the
  generator's settlement class (see [choices](choices.md)).
- Presentation (sound, gauge, shots, effects) is keyed by physical class and
  base weapon, never by unit id.
- Every roster unit and disabled card meets the [detail bar](#the-detail-bar)
  in [the look](#the-look), built from its committed references, with its own
  wreck and tiers that really reduce, within its class budgets; the page load
  stays within its byte limit.

## Scope of the models

| Group | Families (appearances) | Slice |
|---|---|---|
| Tracked | Abrams (3), Leopard 2 (3), Challenger 2, Leclerc, KF51, T-72B3, T-80BVM, T-90M, Type 99A, Bradley (2), CV90 (2), Puma, Ajax, BMP-2M/BMP-3 | 14 (Abrams), 15 |
| Wheeled armour | Stryker (4), LAV (2), Boxer (2), VBCI, BTR-82A, ZBL-08, ACV-P | 14 (Stryker), 16 |
| Light and trucks | HMMWV, Tigr-M, Fennek, VBL; HEMTT, MAN HX, Ural-4320 | 14 (HMMWV), 16 |
| Infantry | 17 roster kits | 14 (rifle squad), 17 |
| Disabled cards | 86 cards in `fixtures/units/model-manifest.json`: 26 aircraft, 24 support, 19 rotorcraft, 12 ground, 5 infantry | 18 |

Every vehicle in the table gets its own wreck (slice 11). Test-unit art is
labelled, not remodelled; the menu's units wear roster art. Not in scope:
animation clips; new mechanics; per-unit variation; admitting any disabled card.

## Contracts

No roster gameplay number, mount, muzzle or tolerance changes. The physical
changes are named: the HMMWV's frame (14) and the wreck prop's `wreck_of` (11).

- **Battle request (07).** `/battle` requires `faction`; missing it or an
  unknown parameter is refused by name. `recipe_id` and `encounter_seed` leave
  `PrepareBattleRequest`.
- **Catalogs (04, 05, 08).** One resolver, three document sets. Only the
  game's is committed (`fixtures/catalog.json`: roster, profiles, roles,
  props); tests' (`+ fixtures/units/test/`, `test_*`) and the menu's
  (`+ fixtures/units/menu/`, `menu_*`) resolve at run time. The battle
  session owns its catalog and loads only the appearances it binds; nothing
  imports a catalog at module level.
- **Presentation keys (01).** Vehicle class derived from mobility × weight
  class × logistics role (`units.ts` `vehicleClass`); weapons keyed by the
  `base` the resolved view publishes. No new catalog field.
- **Transport (10).** Textures are shared content-addressed files; unit and
  scenery bundles travel gzipped; catalog-load bytes have a tested limit; the
  device requests the adapter's texture-layer limit.
- **Wrecks (11).** The published wreck prop carries `wreck_of`; a vehicle
  appearance names its own wreck appearance.
- **Art rules (12).** Each tier draws strictly fewer triangles by a recorded
  ratio; `dressing_*` nodes have their own allowance outside the hull,
  replacing the `hull_top_m` overrides.
- **Encounter opponent (03).** `sim::encounter::{Defender, Referee}`, moved
  unchanged from `village`.
- **Saved map category (06).** `meta.json` `category` is `test` or `menu`.
- **Exporter input (09).** Exporters read each variant's frame from the
  resolved fixture catalog by type id. Node vocabulary and articulation limits
  are as in the [manifest contract](../done/unit-roster/manifests/README.md#engine-and-mount-contract).
- **Reference library (09).** `assets/references/<family>/` photos and
  `references.json` (`file`, `variant`, `view`, `page`, `author`, `licence`,
  `sha256`, `note`, `source`: photo, drawing or generated; plus `gaps`). Generated views come from the latest gpt-image via the duet CLI and are labelled as such. Review inputs only: no runtime, bake or test reads them.
- **Material roles (13).** glTF material `extras.role` (`rubber`, `glass`,
  `paint`, `steel`, `track`, `fabric`, `skin`, `marking`); the validator holds
  rubber and glass dark. Side tint stays on the ORM alpha mask, paint only.
- **Detail parts and crew (09).** `blender/vehicle_parts.py` owns reusable
  pieces; no family redefines a wheel. `vehicle_crew.py` owns visible crew.
- **Budgets (12, 14).** Per vehicle class and for soldiers: triangles per
  tier, tier reduction ratio, bundle bytes, texture layers. Rules in 12,
  numbers measured in 14, enforced by validation.

## References

Every family's lane collects its own references before modelling; the pilot's
families come first (slice 14); slice 09 owns the schema and its check.

1. Find photos of the exact variant (SEPv3 is not SEPv2; BMP-3 is not BMP-2M).
   Coverage per variant: `three_quarter_front`, `side`, `three_quarter_rear`,
   plus `front`, `rear`, `top` and `detail` (running gear, turret roof, sights,
   stowage) where they exist. For infantry: uniform, helmet, carrier, and each
   team weapon deployed and carried.
2. **Licence.** Only images we may redistribute: public domain (US DoD and
   DVIDS photos, most US Army/USMC releases), CC0, CC BY, CC BY-SA. Wikimedia
   Commons file pages state this; record the file page, not a thumbnail URL.
   No press, manufacturer-brochure or game images unless their licence says
   so. **Where photos are scarce** (user chose, 2026-10-06; likely J-20,
   Type 15, KF51, Tornado-S, parts of the Chinese and Russian air and air
   defence lists): use whatever licensable photos exist, even few or small,
   and licensable line drawings or three-views (Commons has many).
2b. **Generated views fill the gaps.** For a view no licensable image covers,
   generate one with the latest gpt-image model through the duet CLI,
   conditioned on the family's real references:
   `duet model -m openai/gpt-image-<latest> --image <real reference> --size 1536x1024 -o <file> "<variant>, <view>, …"`
   (check `duet model --help` and the gateway catalog for the current model id;
   `DUET_API_KEY` must be set). A generated image is labelled everywhere: file
   name suffix `-generated`, `references.json` entry with
   `"source": "generated"`, the model id, the full prompt, the input
   reference files and the seed; licence is ours. It shows a view's layout
   and proportions; it is never evidence of a detail a real photo
   contradicts or doesn't show, and the review sheet marks it as generated.
   Record what each family's model had to guess in its `gaps` and its notes.
3. **Store.** `assets/references/<family>/<variant>-<view>[-n].jpg`, resized
   so the long edge is at most 1600 px (smaller originals stay as they are),
   re-encoded JPEG at a quality that keeps panel lines legible (LFS). Beside them `references.json`:
   `[{file, variant, view, source (photo | drawing | generated), page, author, licence, sha256, note}]` (generated entries add model, prompt, inputs, seed), and a
   `gaps` list. Add a line to [assets README](../../../assets/README.md)'s
   third-party table pointing at `assets/references/` and stating these are
   review inputs, never shipped.
4. Record in each family's file what the photos settle that the old manifest
   got wrong (wheel counts, hatch layout, sight positions). A disagreement with
   a physical frame goes to choices.md for the user; it does not move the frame.

## The look

**Stylised strategy-game readability** (user chose direction D, 2026-10-06),
[look target](assets/look-target.png) (generated; the four candidates are in
[look directions](assets/look-directions.png)): the real vehicle's layout from
its references, drawn with bold bevels, simplified large shapes, chunky
rather than fiddly small parts, gentle painted edge highlights (the material
contract's wear chips on convex edges, in a lighter tone of the scheme) and
moderate weathering. In the manner of WARNO and Broken Arrow. Exaggeration
lives in bevels, edge treatment and the size of small parts, never in the
hull or turret envelope: physical frames don't move.

- **Players zoom in to admire units**, so tier 0 carries real detail and the
  close-up must hold up; tiers 1–3 keep the silhouette and colour that read
  at battle distance.
- **Target machine: the Mac mini this work runs on** (Apple silicon, its
  integrated GPU). Slice 14's budgets are loose tripwires set so a busy battle
  runs well there.
- **One look per variant for now.** Two of the same vehicle look identical.
  Per-unit variation (stowage, dirt, numbers) is a later feature; build no
  machinery for it now.

## The detail bar

A finished vehicle has, where its photos show them:

- **Running gear:** tracked: road wheels with rubber tyres on steel discs and
  hubs, toothed sprocket, idler, return rollers, track whose links read through
  texture and normal map (track UVs scroll, so links can't be modelled); wheeled: black treaded tyres, rims at true size, hubs (CTIS
  where fitted), suspension arms, mudflaps.
- **Hull:** real plate breaks and bevels, weld lines, hinged hatches, driver's
  periscopes, guarded lights, tow hooks and shackles, tow cables, pioneer tools,
  fenders, engine grilles, exhausts.
- **Turret or mount:** mantlet, sights as housings with dark glass, independent
  viewers and RWS as their real shapes, smoke dischargers, bustle racks with
  stowage, antennas, ERA or applique as tiles.
- **Stowage:** jerrycans, crates, tarps and nets where the reference shows
  them, inside the physical envelope.
- **Paint:** the nation's scheme, chipped edges, dust low down, soot at exhausts,
  stencilled markings, black rubber, dark glass, bare steel where worn.

Infantry's bar is in [slice 17](slices/17-infantry.md).

## Verification

Narrowest check first; the full `check` and `verify` run once, in slice 19.
Content slices: their own tests and the suites they rename, crate by crate.
Model slices, per family:

1. `asset validate`, then `bake` and `check` (icons are derived: re-derive,
   never hand-edit).
2. `asset sheet <appearance> --references` (production renderer: contact,
   surface, textures, stats, impostor, references).
3. In-renderer shots at each zoom where the tier changes (tier 3 is what the
   widest view draws: vehicles have no impostor), and close up, from
   `/workbench?bundle=<appearance>`.
4. [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md):
   after against before, and against references.
5. [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md),
   unprimed, last.
6. [preview-shots](../../.agents/skills/preview-shots/SKILL.md) for the user;
   non-blocking (about five minutes, then decide on the evidence, record it,
   close the shots, continue).

No battle runs for art-only changes.

## One owner per concept

- What is game content: the roster and generated maps. Test content: `test_*`
  units and `test` maps. Menu content: `menu_*` units and `menu` maps.
- Vehicle presentation class: derived from physics. Not tables keyed by unit id.
- A unit's wreck: its own appearance's declared wreck. Not nearest footprint.
- Catalog for a battle: its session. Not a module-level import.
- Encounter opponent and referee: `sim::encounter`.
- What a fixed map is for: its `meta.json` category.
- Frames: the resolved fixture catalog. Not archived manifests, not scripts.
- Reusable geometry: `vehicle_parts.py`. Materials and roles: `parts.py`
  helpers and `textures.py` recipes. References: `assets/references/`.
  Review sheets: `asset sheet`.

**Transitional seams**, each with its removal point:

- Until slice 05, the test document set is the game's plus
  `fixtures/units/generic`; slice 05 replaces it with `fixtures/units/test/`.
- Interim wrecks (a vehicle's current hull in burnt materials, slice 11) are
  replaced family by family in slices 14–18; slice 19 fails if any remain.
- The list of current units failing slice 12's new art rules is expected and
  shrinks as families are rebuilt; slice 19 requires it empty. The new rules
  are never silenced to keep a bake green.
- Slice 13 re-exports today's one-liner families with the new materials; their
  geometry is replaced in slices 14–18, and the legacy family helpers
  (`armor.py`, `light_armor.py`, `europe_carriers.py`, `eastern_armor.py`,
  `remaining_ground.py`, …) are deleted as they empty, gone by slice 19.

No compatibility layer: no id aliases, no village replay reader, no storage
migration. When something is replaced, the old one is deleted: a rebuilt
family's one-liner branch, and the legacy family helpers (`armor.py`,
`light_armor.py`, `europe_carriers.py`, `eastern_armor.py`,
`remaining_ground.py`, …) once empty.

## History

The first pass (2026-10) gave every one of the 86 disabled roster cards a
source GLB under `assets/source/roster/disabled/`, generated by
`roster/disabled_placeholders.py` and registered in
`fixtures/units/model-manifest.json`. Those cards stay disabled: a model never
admits a unit, and only the resolved catalog decides availability. Vehicle
deaths use the shared cook-off presentation (`apps/battle-lab/src/cookOffs.ts`,
`packages/battle-renderer/src/effects/cookOff.ts`). The archived roster
model-production rationale is
[`specs/done/unit-roster/model-production.md`](../done/unit-roster/model-production.md).
