# Units: roster-only content and reference-built models

This feature has two halves with one goal. The units a player sees are the
official roster, and they look like the real vehicles and soldiers.

- **Roster-only content.** "Village" was the first test arena, and the
  pre-roster generic units (`tank`, `jeep`, `supply`, `rifle`, `recon`, `at`)
  were its army. Both reached the game: the shipped catalog carried them, a
  `/battle` link without factions fielded them, and the village's opponent ran
  every encounter. Now the game catalog holds only what roster cards reach.
  The generic units survive only as named test units (`test_*`), every fixed
  map says whether it is a test or a menu map, and the menu reel films the
  same battle in roster looks.
- **Reference-built models.** Every roster vehicle (40), roster infantry kit
  (17) and disabled card (90, including the four prototypes and light tanks
  the user added) is built from committed reference photographs in one chosen
  look. Each has its own wreck, detail tiers that really reduce, and materials
  that read as what they are (black tyres, dark glass, the real nation's paint).
  All of it fits within loose budgets set on the target machine.

The decisions, including the user's own and the calls still open for them,
are in [choices](choices.md).

## Why it is shaped this way

**The roster changes all the time, so tests must not stand on it.** Tests,
browser scenes, labs, the benchmark and the endurance lab run on test units.
A roster edit therefore moves none of their verdicts or digests. The test
units live in their own folder and a catalog set resolved at run time, never
in the committed game catalog. A flag on one catalog was rejected because a
single missed check would ship fakes.

**The battle session owns its catalog.** One resolver produces three
document sets: game, test, and menu. Each page and tool names its set, and
nothing imports a catalog at module level. That one rule stops a lab's test
units from silently not drawing, stops a game page from fetching test art, and
lets the menu use units no one can buy.

**Presentation is derived from physics, not keyed by unit id.** Sound, gauge,
effects and art budgets read a vehicle class composed from mobility, weight
class and the logistics role. Weapon rows inherit along `extends`. Before
this, every roster vehicle played the default engine and every derived roster
weapon played the rifle, because the tables were keyed by generic ids. A new
roster unit now gets the right presentation with no table edit.

**Frames are physics, and art is drawn to them.** A unit's frame (hull
extents, eye, mount pivots and muzzles) decides what it can see, what hits it
and where it fires from. Art never moves a frame. The only frame changed here
is the HMMWV's, which had been wearing the JLTV's. Where a frame disagrees with
the photos, the art bends (a lower hull, a compressed cab) and the
disagreement goes to the user. Several remain open.

**The menu reel is a film.** The user's rule is that every approved shot stays
exactly the same, while anything else may change. The battle draws all scatter
from one shared random stream, so a single changed hit retimes everything
after it. The menu units therefore keep the test units' physics and wear only
roster looks, and a native test pins the reel's event log line for line.

**Every vehicle has its own wreck.** The user wants to be able to tell which
unit died by looking at the wreck. A wreck prop names the unit type it was, so
the renderer draws that unit's wreck and cook-offs throw a turret only when
that wreck has one. Matching by nearest footprint is gone.

**Numbers are tripwires, not targets.** Every budget, limit, ratio and
allowance was picked by an agent from measurement, with room to spare. When
one is exceeded and nothing visibly suffers, raise it. The real bars are the
user's: the look, black tyres, recognisable units and wrecks, detail that
holds up close, tiers that switch with zoom, and a battle that runs well on
the Mac mini.

## Principles and invariants

- **What content is.** Game content is the roster on generated maps. Test
  content is `test_*` units and `category: "test"` maps. Menu content is
  `menu_*` units and `category: "menu"` maps. The game catalog holds no
  concrete unit that an enabled card doesn't reach, and a test enforces it.
  Outside specs and design history, only the generator's settlement class
  says "village".
- **Nothing defaults silently.** A battle request without both factions, with
  an unknown field, or with a link parameter outside its list is refused by
  name. A scenario naming a unit its session's set lacks throws instead of
  drawing nothing. A saved replay from before the contract change is refused,
  with no compatibility layer.
- **Frames don't move to fit art, and catalog tolerances don't widen.**
  Dressing (`dressing_*`) has its own allowance so it can't read as cover.
  Each wreck's footprint tolerance is fixed when the wreck is made, and debris
  stays inside it.
- **Every tier draws fewer triangles than the one before**, by a recorded
  ratio. A unit mesh that doesn't name its tier is refused, never copied. New
  art rules are errors from the start, with no warning tier and no list of
  expected failures.
- **Rolling parts turn.** Any ground-touching part whose material rolls
  (rubber, track) sits under a `wheel_*` or `track_*` node. Bodies that only
  rest (skids, legs, a belly) need no node.
- **Rubber and glass draw dark.** They are measured the way the shader draws
  them, averaged over area.
- **Exports are repeatable.** Exporting the same code twice gives
  byte-identical GLBs, live and wreck. A changed byte means the code changed.
- **One owner per piece.** Reusable geometry has one owner, materials and
  roles another, and each family has one script with a `build` and a `wreck`.
  A derived family imports its parent. No family redefines a wheel or track.
- **A model never admits a unit.** Disabled cards have models, wrecks and
  icons, and stay unavailable. Only the resolved catalog decides availability.
  Each disabled card has its own model file and its own reference library.
- **References are review inputs only.** They are licensable, committed, and
  read by no runtime, bake or test. A generated view is labelled everywhere
  and never outranks a real photo.
- **Icons are derived from models.** Re-derive them, never hand-edit.

## Where it lives

Content and catalogs:

- Catalog sets: `crates/sim/src/fixtures.rs` (`CatalogSet`, `own_roots`,
  `test_game`), `web/src/battle/catalog/` (`sets.ts`, the only importer of
  `fixtures/catalog.json`; `compose.ts` `SET_FOLDERS`; `node.ts` for Node
  tools; `context.tsx` `useSessionCatalog`). The pins are
  `crates/sim/tests/catalog.rs` (`the_game_catalog_holds_only_what_roster_cards_reach`,
  `the_test_and_menu_sets_extend_the_games`) and `web/tests/sessionCatalog.test.ts`.
- Units: `fixtures/units/roster/` (roster, with abstract base soldiers in
  `shared.json`), `fixtures/units/test/` (test units, README header),
  `fixtures/units/menu/units.json` (menu units: test physics, roster looks).
- Request contract: `crates/contract/src/preparation.rs`
  `PrepareBattleRequest`. Map categories: `crates/contract/src/maps.rs`
  `MapCategory`, `web/src/maps/catalogue.ts` `categoryMap`.
- Encounter opponent: `crates/sim/src/encounter/` (`Defender`, `Referee`).
- The street test map (the former village ground) and its encounters:
  `fixtures/maps/street/`.
- Menu reel pin: `crates/sim/tests/menu_reel.rs`, with recordings under
  `crates/sim/tests/fixtures/menu-reel/`.
- Presentation class: `packages/scene-assets/src/units.ts` `vehicleClass`.
  Row inheritance: `packages/renderer-core/src/kindTable.ts` `inheritRows`.
  Sounds: `fixtures/game.json` `presentation.audio.vehicles` and
  `fixtures/sounds.json`.

Art pipeline and rules:

- Exporters: `packages/scene-assets/blender/vehicle_export.py` (`run`,
  `run_disabled`, `FRAME_TOLERANCE`), `catalog_frames.py` (frames from the
  resolved catalog), `vehicle_parts.py`, `aircraft_parts.py`,
  `vehicle_crew.py` (`CREW_SOLDIER`), `wreckage.py` (`burn`, `heat`,
  `export_wreck`), `mesh_lods.py`, and the family scripts in `blender/roster/`.
  Infantry: `roster/infantry_equipment.py` (`ARMIES`, `KITS`, `LENGTHS`),
  `infantry_kit.py`, `weapons.py`, `textures.py` (`SCHEMES`, uniform prints).
- Validator: `packages/scene-assets/src/validate.ts` (`articulatedFindings`,
  `typeFindings`, `footprintFindings`), `unitArt.ts` (`UNIT_ART` budgets,
  `tierFindings`, dressing allowances), `material.ts` (rubber and glass),
  `schema.ts` (`MATERIAL_ROLES`, `RUBBER_MAX_LUMINANCE`,
  `CATALOG_LOAD_MAX_BYTES`, `MAP_DOWNLOAD_MAX_BYTES`,
  `TEXTURE_ARRAY_LAYERS_FLOOR`, `catalogLoadNames`), `references.ts` (the
  reference contract, `allowedLicence`), `icons.ts` (`cardIcon`).
- Transport: `packages/scene-assets/src/codec.ts` (the `BGAT` travelling
  form), `gzip.ts` `readBundle`, and `loader.ts` `AppearanceLibrary`
  (`withUnits`). Device admission: `renderer-core` `requestGpuDevice()`.
- Wrecks: `PropDefinition.wreck_of` and `KnownProp.wreck_of` in the sim,
  published as `wreckOf`. Rendered through
  `packages/battle-renderer/src/models/propAppearance.ts`. Cook-offs:
  `apps/battle-lab/src/cookOffs.ts`.
- Faction looks for soldiers: `assets/catalog.json` appearance `factions`,
  validated as `structure.faction_look`.
- Disabled cards: `fixtures/units/model-manifest.json` (`model_statuses`,
  entries), gated by `crates/sim/tests/catalog.rs`. Each card's stated frame
  is its family script's `DIMENSIONS`/`MOUNTS`, and its sources are recorded
  in its library's `gaps`.
- References: `assets/references/<family>/` with `references.json`
  (described in [assets README](../../../assets/README.md)).

## Where the build diverged from the plan

- **Menu units kept the test hulls.** The plan said they would take the
  roster hulls. That broke one menu battle at tick 219, so the art overhangs
  its box instead.
- **Weapon presentation inherits client-side.** The plan had the resolver
  publish each weapon's `base`. Instead, `kindTable.inheritRows` walks
  `extends`, so weapon presentation has one owner and no new published field.
- **Tyres were already black on average.** The plan's diagnosis was dust
  film. Measuring showed the grey read came from the painted rim covering
  most of the wheel face, so the fix was geometry, guarded by an area-weighted
  luminance rule.
- **"Factions need skirmish geography" became a preparation rule.** It was
  meant to stay a contract check, but stress scenes on standard geography need
  the request too.
- **The village ground survived as the `street` test map.** Its ground
  anchored most browser checks. Its fights became data encounters, and a
  `rear` encounter replaced the deleted scripted blue for the ground check.
- **Weapons are joined into the soldier's skin, not separate nodes.** Crew
  hide a soldier's weapon by cutting every piece skinned wholly to `hand_r`.
- **Soldier looks are per faction on one appearance**, not per card. All three
  factions' rifle squads share one roster card.
- **Every disabled card's frame is stated in its family script.** At first
  six cards read the archived roster manifests. At review they were copied
  into their scripts, so no tool reads a closed spec.
- **The interim wrecks are gone.** The plan's interim wrecks (the live hull
  burnt) were a seam. Every vehicle now has a modelled wreck built from its own
  geometry, and `wreckage.burn()` is only its last step.
- **The running-gear rule became physical.** It asked for `wheel_*` or
  `skid_*` nodes, and now asks that rolling materials turn. That let
  belly-landing drones pass without a named exception.
- **The scope grew by four cards.** The user added the M1E3, M10 Booker,
  CV90120 and Centauro II, so every faction has prototype armour and a light
  tank.
- **No full balance run.** The only roster physics change is the HMMWV frame,
  which was sampled when it changed.

## Dead ends

- **A wreck per size class**, then renaming the three generic wrecks by size.
  The user overrode both: each vehicle has its own.
- **Roster hulls for menu units**, and **re-staging the menu battle** with
  real roster stats. Both fail on a shared random stream, because one changed
  hit retimes the whole reel, and no tuning of health, fire windows or
  positions brings it back.
- **Scaling roster art down to the test hulls.** It distorts the look.
- **A `presentation.vehicle` catalog field.** It would change the engine id,
  and derived roster weapons already missed every weapon-keyed table.
- **Committing a second (test) catalog file.** Seventy-six files read the
  module-level catalog, so labs' test units would silently not draw.
- **A WebAssembly export of the resolver's weapon rows.** No set changes
  weapons.
- **A library per catalog set, or a page reload per set.** Both fetch the
  same scenery and textures twice.
- **Recording a second hash for a bundle's travelling form.** One identity
  rule, re-hashed on read, costs the same as before.
- **A vertex-mean luminance for tyres.** Small tread boxes outnumber the tyre
  face in vertices but not in area.
- **Widening wreck tolerances for debris.** Tried twice and reverted. Debris
  was pulled in instead.
- **Extending `armor.py`'s exporter for the rebuilt families.** Its rig and
  palette were Abrams- and Leopard-specific, so one shared `vehicle_export.run`
  replaced it.
- **A vehicle impostor.** At the widest view tier 3 costs nothing
  measurable, so vehicles have none.
- **`mathutils.noise.noise_vector` in wreck heat.** Its offsets differ
  between Blender processes, which made wrecks unrepeatable.
- **Per-faction roster cards for soldier looks.** That would move the roster
  and digests.
- **Marking the catalog set's promise fulfilled so tests don't suspend.**
  That is product machinery for a test-environment property.
- **Moving the street checks onto another map.** That would re-stage every
  coordinate-anchored scene.

## Visual provenance

- [Look target](assets/look-target.png): a generated image of direction D,
  "stylised strategy-game readability". The user chose it on 2026-10-06 from
  the four [look directions](assets/look-directions.png): clean factory,
  field-worn realism, war-film clutter, and stylised. It sets bold bevels,
  simplified large shapes, chunky small parts, painted edge highlights and
  moderate weathering, in the manner of WARNO and Broken Arrow. The rebuilt
  models were judged against it. They land plainer than the target, and the
  edge-highlight strength differs by lane, which is still open in choices.
- [Before sheets](assets/before/README.md): Blender renders of the 43 runtime
  vehicles as they were on 2026-10-06, before any rebuild. These are the
  baseline each family's after-sheet was compared against, not a gate. They
  show the starting point: one-prism hulls, box turrets, cyan optic cubes,
  grey-reading tyres and one olive paint.
- **Reference photographs**: `assets/references/<family>/`. These are the
  requirement each model was built from, with source, author and licence per
  image, and what each model had to guess in `gaps`. Generated views
  (Challenger 3 turret, M1E3, CV90120) are labelled as such.
- [Unknowns map](visualizations/unknowns-map.html): the planning map of what
  was known and unknown at the start. It is history. Its "village" wording
  describes the state the work began from.

Where the result fell short of the references, it is recorded in choices. The
most visible shortfall left is frames that force a squat or forward-placed body;
the disabled cards' detail and the M10's proportions were raised by
[the follow-up](../unit-models-followup/README.md), which also added wreck
debris and authored labels.

## Related

- [Unit roster](../unit-roster/README.md): the roster, factions and skirmish
  loop these models dress, and the archived model-production rationale
  ([model-production](../unit-roster/model-production.md)).
- Vehicle deaths share one cook-off presentation
  (`apps/battle-lab/src/cookOffs.ts`,
  `packages/battle-renderer/src/effects/cookOff.ts`).
