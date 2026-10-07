# Units: roster-only content and reference-built models

Two halves of one goal: the units a player sees are the official roster, and
they look like the real vehicles and soldiers.

- **Part A, roster-only content (slices 01–06).** "Village" was the first test
  arena and the pre-roster generic units (`tank`, `jeep`, `supply`, `rifle`,
  `recon`, `at`) its army. Both still reach the game. Part A converts every
  such place to roster units or deletes it, keeps the generic units only as
  explicitly named test stand-ins, and labels every fixed map as test or menu
  content.
- **Part B, reference-built models (slices 07–13, 15).** Every roster unit is
  rebuilt from committed reference photographs, to a stated detail bar, with
  materials that read as what they are: black rubber, dark glass, worn steel,
  each nation's own paint.

## Next Agent Prompt

**Status, 2026-10-06:** planned, nothing implemented. The earlier pass of this
spec (every disabled card has a source GLB, see [History](#history)) is finished.

**Pick up at [slice 01](slices/01-presentation-by-property.md):** every roster
vehicle plays the default engine sound today, a bug players hear. Part A runs
01 → 06 in order (each shrinks the next one's renames). Part B is independent
of Part A and may run in parallel from [slice 07](slices/07-pipeline.md);
08 (materials) and 09 (references) can run together after 07, and 10 (pilot)
needs both. 14 closes out the playable game; 15 follows.

**Staffing** (user, 2026-10-06): one coordinator owns shared code (the
parts library, materials and roles, validator, catalogs, icons, bindings,
merges) and runs Part A, slices 07–08 and the pilot (10) itself. Everything
per family fans out to parallel workers, each in its own worktree on one
family at a time: references (09) as soon as 07 lands, rebuilt families
(11–13) once the pilot sets the bar, disabled cards (15). Start with six
workers and add more while the machine keeps up (Blender exports and bakes
are CPU-bound; GPU browser scenes run one at a time, so workers screenshot
through the workbench queue, not in parallel). A worker owns only its family's
script, export directory and references, never a shared helper; a needed
helper change goes to the coordinator. Workers share the main checkout's
`web/node_modules` and, since art work changes no Rust, the coordinator's
built WebAssembly for `bake`/`check`; anyone who changes Rust builds their own.

Warnings:

- **Tests use stand-ins.** A roster add, remove or edit must not break a test.
  Tests run on stand-in or fake units (`sim::fixtures::stand_in_game`, on main
  since `847c813c`) unless a test says why it needs the roster. Don't move a
  test onto roster units while converting game content.
- Re-exporting changes GLBs, not physics. Never move a mount, pivot, muzzle,
  hull extent or eye height to make art fit, and never widen a catalog
  tolerance. A frame that disagrees with the photos goes in
  [choices](choices.md) and is left alone.
- Changing a saved encounter's unit ids changes its digests. Regenerate the
  affected parity fixtures and name each in choices.md. A digest that moves in
  a slice meant to move no outcome (02, 03's move, 05's renames beyond id
  order) is a bug.
- The menu backdrop is directed shot by shot (`fixtures/README.md`): new
  armies retime it. Keep the liked shots; retime with unit health and fire
  windows, not by re-cutting.
- Source GLBs and photos are Git LFS. In a worktree, pull only what you touch
  (`git lfs pull --include="assets/source/roster/<family>/**"`).
- Other sessions merge and delete checkouts mid-run; commit often.

TODO:

- [ ] [01 Presentation by property](slices/01-presentation-by-property.md): vehicle sound, gauge and shot sounds keyed by unit properties, not generic ids.
- [ ] [02 Roster stands alone](slices/02-roster-stands-alone.md): no roster soldier inherits a generic soldier kind.
- [ ] [03 Retire the village](slices/03-retire-the-village.md): Defender/Referee move to `sim::encounter`; village scenario, routes, map, wasm exports, replay format and benchmark preset deleted; village-named art renamed.
- [ ] [04 Game content on the roster](slices/04-game-content-on-roster.md): `/battle` without factions refused; menu backdrop films the roster.
- [ ] [05 Stand-ins are explicit](slices/05-stand-ins-explicit.md): `stand_in_*` ids, out of the shipped catalog, art labelled as test art.
- [ ] [06 Fixed maps are labelled](slices/06-fixed-maps-labelled.md): every saved map is `test` or `menu`.
- [ ] [07 Pipeline](slices/07-pipeline.md): exporters read the fixture catalog; shared detail parts; references on `asset sheet`; baseline counts.
- [ ] [08 Materials](slices/08-materials.md): material roles, black tyres, dark glass, nation paint on every current model; validator guard.
- [ ] [09 References](slices/09-references.md): photos for every family committed with provenance.
- [ ] [10 Pilot](slices/10-pilot.md): Abrams SEPv3 and Stryker M1126 to the detail bar; triangle budget fixed.
- [ ] [11 Tracked](slices/11-tracked.md): every runtime tank and tracked IFV/recon family.
- [ ] [12 Wheeled](slices/12-wheeled.md): wheeled armour, light vehicles and trucks.
- [ ] [13 Infantry](slices/13-infantry.md): uniforms, load-bearing kit and team weapons.
- [ ] [14 Closeout](slices/14-closeout.md): full check and verify, menu reel, frame cost, docs.
- [ ] [15 Disabled cards](slices/15-disabled-cards.md): the 86 deferred cards' models and icons, still unplayable.

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
  `grenadier`, `scout`, `at_rifleman`), so a roster squad changes when a stand-in does.
- Village-named live art: `village_ruin` draws every collapsed building;
  `assets/source/village/` holds walls, crates, bridge decks, fences, sandbags
  and dragon's teeth; the `village` city set is the labs' box buildings.
- Every roster vehicle dies into `tank_wreck`, `jeep_wreck` or
  `supply_truck_wreck`: live art with unit names.
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
- **Test content** is named as such: stand-in units `stand_in_*` in
  `fixtures/units/stand-in/`, with art labelled as stand-in art; fixed maps
  with `meta.json` `category: "test"`. Tests, browser scenes, labs that serve
  them, the benchmark and the endurance lab use stand-ins, so roster changes
  move none of them.
- **Menu content** is fixed maps with `category: "menu"`, armies from the roster.
- No identifier, route, file, type or doc says "village" except the
  generator's settlement class (see [choices](choices.md)).
- Presentation (sound, gauge, shot sounds) is keyed by unit properties.
- Every roster unit meets the [detail bar](#the-detail-bar), built from its
  committed references, within the class triangle budget.

## Scope of the models

| Group | Families (appearances) | Slice |
|---|---|---|
| Tracked | Abrams (3), Leopard 2 (3), Challenger 2, Leclerc, KF51, T-72B3, T-80BVM, T-90M, Type 99A, Bradley (2), CV90 (2), Puma, Ajax, BMP-2M/BMP-3 | 10, 11 |
| Wheeled armour | Stryker (4), LAV (2), Boxer (2), VBCI, BTR-82A, ZBL-08, ACV-P | 10, 12 |
| Light and trucks | HMMWV, Tigr-M, Fennek, VBL; HEMTT, MAN HX, Ural-4320 | 12 |
| Infantry | 17 roster kits | 13 |
| Disabled cards | 86 cards in `fixtures/units/model-manifest.json`: 26 aircraft, 24 support, 19 rotorcraft, 12 ground, 5 infantry | 15 |

Stand-in art is labelled, not remodelled. Not in scope: new wreck art (every
roster vehicle still dies into one of three class wrecks, renamed in slice 03,
so a rebuilt T-90 leaves the old generic tank's hull; per-family wrecks are a
later plan, and the cook-off path must keep working with those wreck states);
animation clips; new mechanics; admitting any disabled card.

## Contracts

Nothing physical changes: no roster gameplay number, hull extent, mount,
muzzle or tolerance.

- **Battle request (04).** `/battle` requires `faction`; without it the address
  is refused by name. The recipe-planned player battle and its `recipe` and
  `encounter` parameters are deleted.
- **Catalogs (05).** The shipped catalog is roster, profiles, roles and props.
  Stand-ins (`stand_in_*`, `fixtures/units/stand-in/`) load only through
  `stand_in_documents()` and a test catalog for labs and scenes.
- **Presentation class (01).** Ground profiles carry `presentation.vehicle`
  (`heavy_tracked`, `light_tracked`, `heavy_wheeled`, `light_wheeled`,
  `truck`); sound and gauge rows are keyed by it, shot overrides by weapon id.
- **Encounter opponent (03).** `sim::encounter::{Defender, Referee}`, moved
  unchanged from `village`.
- **Saved map category (06).** `meta.json` `category` is `test` or `menu`.
- **Exporter input (07).** Exporters read each variant's frame from the
  resolved fixture catalog by type id. Node vocabulary and articulation limits
  are as in the [manifest contract](../done/unit-roster/manifests/README.md#engine-and-mount-contract).
- **Reference library (09).** `assets/references/<family>/` photos and
  `references.json` (`file`, `variant`, `view`, `page`, `author`, `licence`,
  `sha256`, `note`, `source`: photo, drawing or generated; plus `gaps`). Generated views come from the latest gpt-image via the duet CLI and are labelled as such. Review inputs only: no runtime, bake or test reads them.
- **Material roles (08).** glTF material `extras.role` (`rubber`, `glass`,
  `paint`, `steel`, `track`, `fabric`, `skin`, `marking`); the validator holds
  rubber and glass dark. Side tint stays on the ORM alpha mask, paint only.
- **Detail parts (07).** `blender/vehicle_parts.py` owns reusable pieces; no
  family redefines a wheel.
- **Tiers and budget (10).** Four mesh tiers; detail removed tier by tier; a
  per-tier triangle budget per vehicle class, enforced by validation.

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
  integrated GPU). Slice 10's budget is set so a busy battle stays within the
  frame budget there.
- **One look per variant for now.** Two of the same vehicle look identical.
  Per-unit variation (stowage, dirt, numbers) is a later feature; build no
  machinery for it now.

## The detail bar

A finished vehicle has, where its photos show them:

- **Running gear:** tracked: road wheels with rubber tyres on steel discs and
  hubs, toothed sprocket, idler, return rollers, track with shoes and guide
  horns at tier 0; wheeled: black treaded tyres, rims at true size, hubs (CTIS
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

Infantry's bar is in [slice 13](slices/13-infantry.md).

## Verification

Narrowest check first; the full `check` and `verify` run once, in slice 14.
Content slices: their own tests and the suites they rename, crate by crate.
Model slices, per family:

1. `asset validate`, then `bake` and `check` (icons are derived: re-derive,
   never hand-edit).
2. `asset sheet <appearance> --references` (production renderer: contact,
   surface, textures, stats, impostor, references).
3. An in-renderer shot at battle distance (where the impostor takes over) and
   close up, from `/workbench?bundle=<appearance>`.
4. [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md):
   after against before, and against references.
5. [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md),
   unprimed, last.
6. [preview-shots](../../.agents/skills/preview-shots/SKILL.md) for the user;
   non-blocking (about five minutes, then decide on the evidence, record it,
   close the shots, continue).

No battle runs for art-only changes.

## One owner per concept

- What is game content: the roster and generated maps. What is test content:
  `stand_in_*` units and `test` maps. Nothing else is either.
- Vehicle presentation class: the ground profile. Not tables keyed by unit id.
- Encounter opponent and referee: `sim::encounter`.
- What a fixed map is for: its `meta.json` category.
- Frames: the resolved fixture catalog. Not archived manifests, not scripts.
- Reusable geometry: `vehicle_parts.py`. Materials and roles: `parts.py`
  helpers and `textures.py` recipes. References: `assets/references/`.
  Review sheets: `asset sheet`.

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
