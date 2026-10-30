# Fixtures

Authored settings belong here, validated by their reader. [Battle rules](rules.json)
own simulation numbers, [presentation](presentation.json) owns visual settings,
and [audio](audio.json) owns the mix. [Presentation admission](../crates/presentation/src/settings.rs)
and [sound admission](../crates/presentation/src/sound/catalog.rs) refuse unknown keys and invalid values
for native, browser and authoring tools. Browser input settings belong beside
the input code; the lobby owns the default battle seed. [Units](units/), [props](props/) and [biomes](biomes/)
own component catalogs and terrain palettes; [catalog resolution](../crates/contract/src/catalog.rs)
produces the browser's resolved [catalog](catalog.json).

[The rules contract](../crates/contract/src/scenario.rs) refuses unknown sections,
fields and invalid section ranges, including in encounter overrides.
[Simulation admission](../crates/sim/src/admission.rs) checks external rules before
a world or battle is built. Application settings have separate owners and do not
enter resolved battle rules.

[Map presets](map-presets.json) and the generator's [physical template library](prototype-building-templates.json)
feed [mapgen](../crates/mapgen/README.md). Its templates are derived from accepted
[city sets](../packages/scene-assets/README.md#city-buildings), and it is the one
catalogue of buildings: a hand-authored map places the generator's templates too,
selecting the ones it uses, so the library changing elsewhere leaves its identity alone.

[Encounter recipes](encounters.json) describe rosters, objectives and placement
policy without coordinates; [the simulation planner](../crates/sim/src/encounter/)
places them on admitted geometry. Players never field a recipe: the map workbench
and native reports plan one on a generated map to judge it, and tests use them.
[Generated-battle settings](generated-battle.json) own preparation policy. [Camera lab settings](camera-lab.json)
belong to the isolated camera experiment, not a second gameplay camera policy.
[Captures](captures/) say how saved encounters are filmed, camera and scenes only:
each scene a saved battle (an ordinary saved encounter, scripted orders and all, on
a `capture` map) and its reel of camera shots. The
[capture tool](../apps/battle-lab/src/capture/README.md) films any of them and owns
the format; [the menu backdrop](captures/menu.json) is the one the main menu plays.
The rest of this section is the backdrop's: each of its encounters is staged
like a film, not balanced: set-pieces open fire on a schedule, set by how far each
force starts from its enemy, so every cut has guns firing in frame. Cannon fire and
blasts read at film distance; rifle fire barely does. A changed encounter or rule
(movement included) retimes the set-pieces, so recheck each scene's cuts after
either. Each battle is directed, not left to play out: starting health, scheduled
fire policies and scripted attacks set who fires and who dies when, so a change
keeps the cuts it was not asked to change and retimes the battle to them, never
the reverse.

**Directing the reel after a change.** You control the whole scenario; the shots
are the fixed part. When a battle no longer plays a shot as approved, change the
battle, never the shot:

- **A followed unit dies too early:** give it more health through the
  encounter's `overrides` (a scenario-only hero row), or delay or move what
  kills it.
- **A shot's action happens too early, too late or not at all:** move the
  units, retime the scripted orders or fire policies, or pin the stats through
  `overrides`.
- **Something new happens in a shot** (a rule now lets a unit fire): script it
  out, or move the unit out of frame.

Only when the user asks for a new shot do a shot's camera, framing, timing or
subject change. After re-recording the log, render the reel and show the user
the footage to approve.

The backdrop is directed footage, not a battle. It may field any units, including
the [capture set's own](units/capture/) (`capture_*`, in the capture catalog set only) with
whatever stats, hulls, weapons, scripts and positions it needs, and anything in it
may change. What may not change is an approved shot: its camera, framing, timing
and what happens in it (who fires, which round or missile flies, who is hit, who
dies, and when). [The exact-shot test](../crates/sim/tests/menu_reel.rs) holds
each scene's battle to the event log recorded from the approved reel, and no shot
may open on a unit it follows that has already fallen. A battle this chaotic can
only keep its log if its physics stays the same, so a capture unit extends the test
unit whose part it plays, hull and all, and changes its look; one shared random
stream means any physical difference retimes the whole scene. So each scene's
encounter pins, in its `overrides`, every stat of every weapon its cast fires, at
the values the approved reel was filmed with: the game's weapons are retuned all
the time, and none of that retuning reaches the reel. The pins prevent only
stat-driven reshoots: a change to a rule (a mechanic, a rules section, a test
unit) still retimes the battle, and is never held back to keep the reel. It
re-records the reel's log, and the user approves the new footage.

[Sound authoring](#sound-catalog) owns audio provenance and choices independently
of gameplay weapon cycles. Saved maps, component inheritance and paired evidence
have the contracts below.

## Saved maps

A saved map is one folder, `maps/<id>/`, under [the saved-map catalog](maps/), and nothing reads a map any other way. The id is the folder's name: lowercase letters, digits, hyphens and underscores. It names where the map is stored and nothing else: a route's id and a map's content hash are separate things, and several routes may play one map.

- **`map.json`** is the physical map in its saved form (`contract::map::SavedMap`): ground, relief, water, surfaces, forests, props and buildings. A building is stored as what is its own: `owner`, `kind`, `template_id`, `frame` (translation and yaw) and `parts` (each named part's prop id). Its geometry, category and regional family are the template's and are not stored. The map names its own region once, `regional_family`, which every building's template must share (absent only on a map without buildings): the renderer and the asset loader read the region there and nowhere else. Only the resolver reads this file.
- **`SOURCES.json`** pins what the map is and where it came from (`contract::maps::MapSources`): its content identity (an authored map's hash, or a generated map's whole generation identity), its physical catalogue (`catalogue.library`, the file name of the template library in this folder, and `catalogue.template_ids`, the templates of it the map uses: an authored map lists them, so only a change to one of its own templates moves it), and a receipt for each input it was made from. A receipt is `repository` (path, revision, sha256) or `supplied` (label, sha256).
- **`meta.json`** is what a listing shows without loading the map: category, status, label, character, biome, size, feature tags, source, seed, and the names of its encounters and benchmarks. [`web/src/maps/catalogue.ts`](../web/src/maps/catalogue.ts) owns the schema, the validator and `listMaps`. Every field the map itself also states is checked against the map, so a listing cannot drift from what loads.
- **The category says what a saved map is for** (`contract::maps::MapCategory`): `test` (a test's, a lab's or a benchmark's ground) or `capture` (a battlefield the capture tool and menu film); anything else is refused by both readers. No saved map is the game's: player battles are generated and never name one. The capture tool and menu film take only `capture` maps, and labs and native tests only `test` maps (`fixtureMap`, `sim::fixtures::with_map`), so a test needing content another owner has gets its own copy of the map rather than borrowing that owner's. The same holds the other way: to film a test map's ground, copy its folder into a new `capture` map. Test maps stay as their tests need them; capture maps change as often as a film does.

- **`encounters/<name>.json`** is an encounter laid on the map (`contract::scenario::EncounterDefinition`): units, events and scripted orders, and optionally an opponent, a completion rule and `overrides`. A scenario is the resolved map, the rules and one encounter. The endurance battle is not a file: the simulation builds it from the rules and the resolved map (`sim::endurance`).

**A scenario may override any rules entity.** An encounter's `overrides` is shaped like the authored rules: any section of `rules.json` (`weapons`, `sensors`, `air` and the rest) or of a catalog document (`units`, `soldiers`, `parts`, `roles`, `props`). It patches the rows it names field by field, by the merge an `extends` child uses, and a row the rules lack is added for this scenario alone; it may `extends` any row of its section, and the encounter's units may field it. The merged rules are read and checked again exactly as the shipped ones are, so a bad override is refused naming the encounter, the row and the field. The one owner is `EncounterDefinition::on` (`Rules::overridden`): native readers reach it through `sim::maps::scenario`, and the browser through the WebAssembly `encounter_scenario`, so a scenario's rules never differ between them. The shared rules it was laid on are untouched.

Scripted group orders address the formation's surviving own units when their tick arrives. Fallen own actors are removed before ordinary command validation, so one casualty cannot cancel a reserve wave; an all-fallen group has no effect. Unknown or foreign actors still refuse the script, and player commands validate their exact submitted selection. Script execution follows the same rule during live play and replay in [the battle authority](../crates/sim/src/battle.rs).

- **`sites.json`**, on a generated map only, is what the encounter planner reads beside the map (`contract::encounter::EncounterSites`), as the `mapgen` CLI wrote it. Nothing loads it at run time: it is there so the saved encounter can be planned again.

Routes and saved maps have different identities. The [fixture registry](../apps/battle-lab/src/fixtures.json) declares each route's fixed saved-map reference; tools and generated or synthetic worlds have no fixed catalogue source. The developer index offers every active catalogue map through the shared geometry inspector, so saving a new map needs no bespoke route or scene. Player skirmishes generate fresh maps; the saved catalogue serves tests, labs and captures.

**One resolver admits every map** (`contract::maps::resolve`), in Rust, so native and WebAssembly readers agree. It materializes every building from its template at its frame, out of the library the sources name, and answers the resolved map (`contract::map::MapDefinition`) that the simulation, the renderer and every test receive. It checks that the map names the catalogue its selection hashes to, and that the resolved map is the one `SOURCES.json` pins: a map's content hash is the hash of its resolved definition. The library's file name only says where to find it; the catalogue hash is what admits it. A building whose template the catalogue lacks is refused by its index. A map that fails is refused with a code and the document and field at fault. Nothing stands in for it, and there is no other saved form: a `map.json` that carries building geometry is refused. Both catalogue adapters admit what the game's generator may make (`MapAdmission::CATALOGUE`, the limits of `generated-battle.json`). Three thin adapters fetch the documents and call it:

- native tests, examples and tools: `sim::maps::load(id)` and `sim::maps::encounter(id, name)`;
- the browser: `loadMap(id)` and `loadEncounter(id, name)` in [`web/src/maps/browser.ts`](../web/src/maps/browser.ts). Each document is its own served file, so no map is part of a script;
- Node tests, scenes and tools: the same two names in [`web/src/maps/node.ts`](../web/src/maps/node.ts), over the built WebAssembly.

**Adding a map:** make the folder, write `map.json` and `meta.json`, and write `SOURCES.json` with the map's library, the templates it selects and its content hash. The catalogue hash is the contract's hash of those templates, and the resolver's refusal states the content hash it computed, so an authored map's first load tells you the value. A map the compiler makes is saved in this form, with its `SOURCES.json`, by the `mapgen` CLI.

A released saved generated map is a reviewed, fixed battlefield. Its template
hash must continue to resolve; changing physical templates may require generating
and reviewing that map again. [The mapgen CLI](../crates/mapgen/src/main.rs) owns
saving generated maps, and [encounter reporting](../crates/sim/examples/encounter_report.rs)
owns saving a planned encounter. The [catalog tests](../crates/sim/tests/maps.rs)
and [browser tests](../web/tests/mapCatalogue.test.ts) hold each folder to admission
and listing policy. Directory discovery requires no parallel index.

## The unit catalog

Deployment preserves one reversible progress value. Setup and packing may have different durations; the integer progress lattice makes both directions and partial reversals deterministic. Movement waits for packing to finish, and supply waits for setup to finish.

A unit type is **one catalog entry**, addressed by its string id (`"us_m1_abrams_sep_v3_trophy"`, `"test_tank"`). No code lists unit types. Its `name` is what the player reads (`"M1A2 SEP v3 Trophy"`), authored, never derived from the id; so are its family's, its mounts', its weapon rows', its protection's and its planned weapons' labels, each held to [the label rule](../crates/contract/src/labels.rs), and each weapon names its generated icon. Scenarios, spawn rows and commands name types by id; the publication sends the id list (`unitKinds`) and each unit's index into it.

[The catalog parser](../crates/contract/src/catalog.rs) owns component schemas,
validation and inheritance. Families contain unit, soldier, role and upgrade
records; ids are defined once across authored files. Behavior comes from body,
mobility, sensors, mounts and capabilities rather than the type's name.
Each resolved physical unit requires its own positive `concealment`, which
inheritance may supply; values above one represent more conspicuous bodies.
Optional `sensors.spotting` defaults to the neutral one. `sensors.ground_m`,
`sensors.low_air_m` and `sensors.high_air_m` author sight into each band
independently; zero air reach means blind. `capabilities.radar_emitter` authors
a signature independently of those ranges. The [sensing contract](../crates/sim/README.md#sensing) explains
how these values interact with ground and movement.
A unit's `capabilities.carriage.seats` (3, 8 or 16) makes it a carrier; nothing
else does. The rules' `carriage` row sets the boarding timer and reach, and their
`bail_out` rows, one per kind of host (building, ground, air), set how a garrison
or riders come out of a destroyed host: death odds, the health lost and how far
to search for a place to stand.

- **Soldier kinds** are a catalog of their own: `hp`, the `appearance` set a soldier of the kind wears (one picked per soldier), and the `mounts` he carries. A `special` mount passes to the next living soldier when its carrier falls; any other is lost with him. Every soldier carries an unlimited `squad` gun, and every finite weapon is a `special` extra, so no squad runs dry: a sniper whose rifle is empty still has his carbine or rifle ([squad weapons](../specs/done/squad-weapons/README.md)). A single-operator mount may name `operator_appearance` with paired active and carried appearance sets: the equipment worn by its current living carrier while using that weapon or his rifle, including after a handoff. Variant identity stays the same across both sets. The observation publishes the carrier and each soldier’s active weapon only when visible. A squad's slots name soldier kinds, so hundreds of squads reuse a few kinds.
- **A variant is `extends` plus overrides.** `"m1a1": { "extends": "m1", "body": { "hull": { "armor": { "front": 180 } } } }` inherits everything else. An `abstract` entry only exists to be extended. Weapon rows in `rules.json` extend the same way. The merge:
  - objects merge key by key, and a list of objects with ids (mounts) merges by id, a new id appended;
  - a unit's one-key variant component (`body`, `mobility`) written as another variant replaces the parent's: `"mobility": { "wheeled": … }` over a tracked parent is wheeled;
  - a unit's `parts` gather along the chain, the parent's first;
  - anything else is replaced, a list of strings (`roles`, `slots`) whole.
  - What a variant can't do: drop a key or a mount (by id) it inherits. A type that loses a mount extends a common parent instead.
- **Parts are upgrades:** `"parts": ["trophy_aps"]` merges each part's `patch` into the type after inheritance, by the same rules, so a part may add any capability: the Trophy adds active protection, and a radar or a flare pod would add `radar_emitter`, `radar_receiver` or `flares` the same way. A part that needs a capability the simulation doesn't build yet is refused at load, because the type no longer parses. A part names the model nodes that show its hardware (`nodes`), and the type's model must draw them.

Resolution happens once through the contract owner. Invalid inheritance,
references, physical mounts or numbers fail at load with an entry-specific error.
[Catalog tests](../crates/sim/tests/catalog.rs) own the explicit regeneration gate
for `catalog.json`; browser presentation reads that resolved view, including
resolved weapons ([presentation rules](../web/src/battle/catalog/compose.ts) hand
them to panels and the range ruler), rather than reinterpreting authored families.

A battle runs one of three document sets (`sim::fixtures::CatalogSet`): the
game's, the only one committed as `catalog.json` and holding only what roster
cards reach; the test set, which adds the [test units](units/test/) (`test_*`)
that tests, labs, scenes and art checks field; and the capture set, which adds the
test units and the [capture units](units/capture/) (`capture_*`). The test and capture sets are resolved when asked for and
never written, so one generated file stays the editors' and publication's. The
browser's [session factory](../web/src/battle/catalog/) is the one reader of
`catalog.json`, and each battle session owns the set its page runs. After a catalog
change, the [asset CLI](../web/asset.mjs) regenerates and checks model-derived icons.

## Weapon cycles

Engagement distances belong to each weapon row's target layers. The resolved top-level `range_m` is their greatest range; authored rows do not set a second range. Minimum range is a crew's firing restriction measured from the actual muzzle to the aim point, not a delayed arming fuse: already-fired rounds still collide and explode normally. A too-close weapon holds its rounds rather than requesting an advance, while other mounts remain independent. Target selection and each physical shot enforce the same minimum, including sampled contacts and individual soldiers.

A weapon row's `ammo` is total carried rounds, including loaded magazines; `"unlimited"` means unlimited reserves. A finite-ammunition row must supply `round_cost`, directly or by inheritance, so resupply always has an authored price. `magazine` adds a physical gun's capacity and interval between shots, while `reload_s` is the pause to replace its magazine or belt. An optional `burst` groups rapid shots. Pace beats realism here: no weapon takes more than about 5 s to reload (artillery and the long-range SAM up to 15 s), none aims for longer than 3 s, and an aircraft's flares come back in 3 s ([D50](../specs/done/air-war-flight/flight-model-map.md)). The [fire cadence rationale](../specs/done/fire-cadence/README.md) explains their readiness and interruptible idle reload contract.

The nominal vehicle aim height comes from its body, with a fixture-owned fraction allowing fire into both the upper body and hull. Direct-fire scatter stays at or below that aim point. A round that survives to its aim plane falls under stronger downward gravity, chosen at launch to reach even the lowest ground within the authored fall-time limit. Its horizontal flight and its position and velocity at the join stay intact. Both legs use ordinary swept collision: intervening bodies still decide the first impact. A ricochet cancels that fall and keeps its deflected direction under ordinary gravity; its remaining lifetime is capped from the first bounce, without restarting on later bounces. Guided fire keeps its own guidance, and indirect fire keeps its ballistic arc.

Tracer frequency and shape belong to presentation. A round's sampled visibility persists across its observed flight; it does not flicker with each publication. Small-arms traces use a fixed screen-width line, while muzzle flashes, ricochet sparks and impact puffs still show their own published causes. Only a tracer round throws sparks, a hot flash and its light off what it hits, so they answer a round the viewer saw fly; every hit still raises its surface's dust, and a ricochet always sparks. Sparks fly only off metal: a hull, or a prop whose body is `metal`, which the simulation publishes as its own hit kind (`metal_prop`); a round into a wall, a hedge or sandbags raises dust and nothing else. Hiding a tracer never hides its physical impact or changes the shot. A hull grinding a metal body it shoves (its body's `metal`) throws sparks where they meet, `grind_sparks_per_s` a second at the size a ricochet's are; the simulation publishes only the grinds, so a wooden fence, a crate or sandbags throw none.

Impact puffs keep the material of the surface they strike, with per-round size and lifetime overrides inside that surface's style. Heavy rounds can leave larger, longer-lived dirt clouds without extending armour flashes or explosion smoke; the effect owner's lifetime bound includes those overrides. A burst's fireball grows with its blast radius times its round's `blast_scale`; a burst smaller than the blast style's reference also throws fewer, smaller sparks and puffs and lights less ground, so a cannon's small shell flashes and kicks up dirt rather than drawing a shell's explosion. The ground keeps the same proportion: a burst below `ground.mark_reference_blast_m` adds scorch in proportion to its share of that radius and crater depth by the square of it, a dark pock rather than a shell's crater and black scar.

Each physical weapon owns its targeting, ammunition and firing cycle. Identical special weapons stay separate, so their rows can show different rounds and reload progress. Ammo quantities are authored per mount: adding another identical gun adds another authored load. The only shared row is infantry's default gun (`squad` in the soldier's mount), whose copies keep independent firing cycles and have no reload pause. Its readout stays loaded while any surviving carrier is loaded. Suppression's tier widens launch scatter as well as slowing cycle progress; those penalties compose with movement and cover.

A replacement soldier brings a fresh default gun; a transferable special weapon keeps its identity, rounds and reload progress across handoffs. Living original carriers operate their own guns, survivors retain recovered assignments, and free soldiers take up remaining stocked weapons. One soldier operates one special weapon at a time. Additional recovered weapons remain spares: their reload pauses until an operator is available, while movement still interrupts stationary weapons. An empty launcher's operator finishes guiding its last missile before taking up a stocked spare. Guidance belongs to the physical launcher, so an eligible survivor can continue an in-flight missile; loss of support, Stop or lost sight releases it, and so does movement for a row whose `guidance` is `"stationary"`. Movement means the body carrying the weapon changing place: the soldier operating a squad weapon, otherwise the unit, so a gunner standing still keeps aiming and guiding while a teammate moves. Every guided row states its `guidance`: `"stationary"` (a ground crew's wire-guided missile) or `"on_the_move"` (a helicopter's, which keeps guiding as it flies).

A guided row's `top_attack` changes only the path its supported missile flies: it shoots up at `dive_deg` to `loft_m` above the commanded point, holds that height, then dives once that point lies `dive_deg` below its horizon, so geometry, not a rule, puts the hit on the roof. Guidance, release and active protection are unchanged; a released missile flies straight at its fixed point. The dive must fit the row's turn rate: a slow-turning missile overshoots a steep pitch-over, so a top-attack row turns far faster than a direct one ([traces](../specs/done/ground-admission/assets/top-attack/)).

A round's damage to a hull is the share its `penetration` gets through the face it meets: all of it at or past the armour, none short of the weapon row's `partial_from`, and in proportion between, so a round that nearly gets through still shakes the hull. Whether it glances off is only how it flies on; a glancing round still does its share, and each ricochet cuts its penetration for the next face it meets. A weapon engages a hull when any of its damage gets through the weakest face, and prefers the target whose facing armour lets most of it through. Penetration is the one lever for how much a weapon hurts armour: there is no separate blast-through-armour share.

Each target layer owns its launch spread, `scatter_mrad`: a one-axis angular
standard deviation, independent of miss distance and blast radius. Admission
requires every powered, unguided layer's spread at its own range
(`scatter_mrad × range_m × 1e-3`) to meet
`physics.min_spread_at_max_range_m`; guided rounds steer out their initial spread,
and unpowered bombs are exempt.

Every weapon row states the height bands its crew can bring it to bear on, `targets`, each with how it fires there: `"ground"`, and `"low_air"` for guns that swing up at helicopters (rifles, machine guns, autocannons). A band's `range_m` and `scatter_mrad` are explicit in the resolved row, and a row that `extends` another inherits those values unless it authors a layer override. Each band also names the target classes it is fired at there, `classes`: `infantry` and `vehicle` on the ground, `air_rotor` and `air_fixed_wing` in the air, which a unit is by its body and its `mobility.air.flight`, never by its name, and `missile` in any band: an `interceptable` round fired from that band, which the row shoots down at that band's range and spread (a guided row homes on it as on an aircraft). A dedicated anti-tank row lists `vehicle` alone, so it is never fired at soldiers or at an area: area fire is for a row that engages infantry, and never armour-piercing. A ground point or an area is no unit, so no class bars it. A band with no classes, an unknown one, or one never found at that height is refused at load, as is any field a weapon row does not have. A gun fires through what its rounds can break; a row with `requires_clear_launch_path` (the artillery and the long-range SAM) instead holds, showing a blocked trajectory, while the ground, a body that stops rounds or tree crowns stands in its round's first `launch.clearance_m` from the muzzle. The skirmish AI drives a unit of its own that stays held (by any weapon's `BlockedTrajectory`) to other ground, after a wait and over a distance its own tuning in `skirmish-ai.json` sets. Only the launcher's end is judged: a high arc clears a low wall, and a target in a wood or past a hill is fair. A ground gun reaches a helicopter from much closer and with a much wider spread than along the ground, so a helicopter outranges it and is hard to hit, but falls to a few hits. A band never reaches past the row's `range_m`, whose flight the rules admitted; the gun, its magazine and its round are the same at every band. A tank gun, a grenade, a rocket pod or a ground missile reaches the ground only. Flight never reads it: a stray round still strikes a helicopter in its way. A gun fixed in a hull (a vehicle mount that is no turret and rides none) bears where the body faces, so it fires only once the body has turned onto its target: an aircraft turns its airframe to aim while it flies on, idle tracks pivot, and wheels hold fire. A turret the hull carries may state an `arc` (`centre_deg` off the hull's heading, counter-clockwise, and `width_deg`): a door gun's door. It swings only inside it, the way that stays inside, and fires only on what it bears on; when its target lies outside, the body turns the arc's middle onto it, unless a gun with the same weapons already bears on it (a helicopter with a gun in each door turns for neither). A turret that has held no enemy in sight for a while turns back to its hull's front; ground it was ordered onto, or a round it is tracking, keeps it busy without one. `helicopter_crash` is the burst of a downed airframe meeting the ground: no gun fires it, and its structural damage fells the trees beneath.

## Adding a unit type (a tank variant, a vehicle, an infantry type)

Add one entry. A variant is an `extends` and what differs. Code learns nothing about the type, and changes only where a genuinely new behaviour appears, as a new component or capability. Each rule has a guard; if you add a type that the guard doesn't cover, extend the guard.

- **Never branch on a type's id or role for a rule.** If a function asks "is this a tank?" to pick an offset, a size or a speed, that value belongs in the type's components. Scripts and the AI may select units by role; rules never do.
- **Every mount is its own weapon.** It has an `id`, which other fields and models name it by, and a `name`, the label its card shows. It has its own lock, bearing, pivot and muzzle. A mount on a turret pivots with the turret; a mount on its own ring (a roof HMG) turns on its own bearing about its pivot. Every `pivot_m` is in the hull's frame at rest; a mount `on` a turret swings round that turret's pivot as it turns (one level only: a carrier itself sits on the hull). Two mounts never share a muzzle point. A vehicle model declares which of its rigs draws each mount in its `assets/catalog.json` entry (`"mounts": { "cannon": "gun", "HMG": "hmg" }`, by mount id); a mount's id decides nothing.
- **The simulation's numbers are the authority, and art is fitted to them.** Hull extents, eye heights, mount pivots and muzzles, the canopy and the footprint are set in data. Each type's own model is fit-checked against its own resolved numbers (the `fit.*` checks in `packages/scene-assets/src/validate.ts`). A variant with a new model inherits its parent's mount geometry, so the new model must put its turret and gun where the inherited mounts say, or the variant overrides those numbers. When you add a number with a drawn counterpart, add its `fit.*` check in the same change.
- **Presentation anchors to what is drawn, not to the simulation's points.**
  - Effects that belong to a model part (muzzle flashes, exhaust, dust at the tracks) sit on the drawn part's socket, and fall back to the published point only when nothing is drawn.
  - Ground markers are sized from the type's footprint (its hull), so they peek out from under the model.
  - The renderer skill has the mechanics (`.agents/skills/renderer/SKILL.md`).
- **Culling bounds cover every reachable pose:** a traversed turret, a deployed spade, a raised gun. Size them from the articulation's extremes, not the rest pose.
- **A vehicle needs its whole life cycle:** its `mobility`, its weight and push class (what it pushes, and what it's worth as cover, live or wrecked), its `wreck` (a prop row with hit points and a destroyed state, which may chain into a lighter wreck), and its `sound`. Missing one shows up late, as a vehicle that can't be pushed, or a wreck that never burns out.
- **Presentation follows physics, never the id.** A vehicle sounds and articulates as its class (`vehicleClass` in `packages/scene-assets/src/units.ts`: tracked, wheeled, air or high air, its hull's weight class, and `_logistics` for a supply hauler), and a derived weapon row inherits its `appearance` through the ordinary weapon resolver. That appearance names its look in [presentation](presentation.json). A new type of a fielded class needs no presentation edit; a new class needs its engine row in [audio](audio.json). Each ground model authors its own track gauge in the [asset catalog](../assets/catalog.json); fielding refuses one without it.
- **Infantry** share one body frame: the soldier's radius, height, eye and muzzle heights, and the cover rules, are `physics` numbers here, not per soldier kind. Variants within one appearance set share a skeleton; each soldier animates with the clips of his current appearance.

**The guards scale with the catalog.** Every resolved type runs the same generated checks with no test of its own: its components are complete and in range, it sets up, fires each mount and moves (`crates/sim/tests/catalog.rs`), its appearance exists and fits its numbers, and its icons exist. Then add the type to the lab or scene that exercises it: the workbench sheet for the model, and a movement scenario (`crates/sim/tests/movement_scenarios.rs`) for its movement and cover. Run the checks for whatever you changed (see [`AGENTS.md`](../AGENTS.md)).

## The prop catalog

A prop type (a house, a wall, a tree, a wreck, rubble) is **one entry** of a `props` section, in `props/<faction>/<family>.json`, addressed by its string id. No code lists prop types, and no rule asks which one a prop is. A map's props, a forest's trees (`forests.tree` in `rules.json`), a bridge's deck (`deck` on each map bridge) and a vehicle's wreck all name types by id; the world layout and the publication send the id list (`propKinds`) and each prop's index into it.

[The prop schema](../crates/contract/src/catalog.rs) separates body properties,
destroyed-state transitions and appearance bindings. Each physical rule reads its
own column; a type id supplies identity rather than a mechanic. Destruction chains
must terminate. Appearance bindings distinguish ordinary scenery, building parts
and forest trees because their geometry has different owners.

- **A building's end is its row's `destroyed.into`:** `prop` is what each part of a collapsed building becomes, and `building.gutted_prop` what each part of a taller one becomes where it stands (`building.collapse_max_floors` decides which). A side that has seen a building destroyed is published those props in its parts' places, and draws the building's template in the matching state, a ruin or a gutted shell: the published prop's type decides, never its height.
- **Systems-only physical rows** carry `appearance.status: "systems_only"` until their actual scenery is fitted and accepted. They name the intended scenery, never borrow a released asset to pass validation. Removing that status restores the ordinary appearance gate; the systems marker is not an art acceptance.
- **A variant is `extends` plus overrides,** as for units: `wrecks.json` shares one abstract `wreck` frame.
- **Resolution happens once, in the simulation,** with the units: unknown or looping destroyed states, `hp` without `destroyed`, a cleared state on a body that doesn't topple, and a unit's wreck that names no prop type fail at load, naming the entry. The exported WASM catalog resolver accepts supplied documents through the same validation path; shipped-fixture tests therefore do not replace load-time validation. Regenerate `catalog.json` after editing, as for the units above.

## Adding a prop type (an obstacle, a vehicle's wreck, a city's building)

Add one entry. A mechanic comes from the body's columns, so a new obstacle needs no code: dragon's teeth are small heavy bodies that block vehicles, and a squad takes cover behind each one.

- **Never branch on a prop's id.** If code asks "is this a building?", "is this a tree?" or "is this metal?", the answer belongs in a column (`garrison`, `topples`, `metal`) or in the appearance binding (`drawn_by`).
- **A wreck's physics is a shared row; its look is its unit's.** A hull's `wreck` names a physics row extending `wreck` (the vehicle's weight class and cover tier, which a wreck keeps and the load checks, `hp`, and a destroyed state that chains into a lighter wreck or goes). Every wreck also names the unit type it was (`wreck_of`, on the published prop), so it is drawn as that unit's own wreck art. A map that places a wreck names its unit too: a prop drawn by `wreck` without one, or any other prop with one, is refused.
- **Every type needs something that draws it:** a scenery kind in `packages/scene-assets` with a catalog appearance authored to a box the simulation places (the catalog footprint test holds every drawn kind to one), `forest` for a tree, or `building` for a part of a building, whose template the art library dresses. The renderer reads the binding from the world layout; it lists no prop types.
- Then add it to the map or scenario that exercises it, and run the checks for what you changed (see [`AGENTS.md`](../AGENTS.md)).

## Surface speeds

Every mover states its own two top speeds in its `mobility` row, in km/h: `offroad_kmh` on open ground and `road_kmh` on a full road (validated against its off-road speed and the catalog's limits). An aircraft (`"air"`) states one `cruise_kmh` instead, with its turn and climb rates, the `layer` it flies in and its `flight` model, always authored: `rotor` or `fixed_wing`. A pair no flight law flies yet is refused at load. `rules.json`'s `air` section holds each air layer's heights (`low_air`, `high_air`) and each flight model's rules (`rotor`, `fixed_wing`). Everything else that changes a mover's speed is a **speed factor**, a share of its own speed at most 1, named `*speed_factor` wherever it lives:

- **The ground** is `rules.json`'s `movement.speed_factors`, one row per locomotion, keyed by the unit's `mobility` kind (`foot`, `wheeled`, `tracked`), because how a unit moves, not how big it is, decides how terrain slows it. Each row holds `surfaces`, one factor of road speed per surface kind a map may pave (`road`, `country_road`, `dirt_track`, and `paving`, hard ground that is no way through: a yard, a court), never below off-road speed, with 0 for no road at all; `forest` and `swamp`, of off-road speed off any road (where both lie, the slower applies once); `crater`, over a full crater, a shallower one slowing in proportion; and `slope`, speed falling in proportion to the slope to nothing at `zero_speed_deg` but never below `min_speed_factor` (the map's `slope_cutoff_deg` decides what is too steep to climb). A new surface kind is a new field in every row plus its variant in `contract::map::SurfaceKind`.
- **Driving** is a vehicle's own `reverse_speed_factor` and the rules' `movement.drive.turn_speed_factor`, the most a manoeuvre keeps.
- **An afterburner** is a jet's `capabilities.air.afterburner` `speed_factor`, above 1, paid for by its `fuel_factor`.

Suppression is the one exception: its tiers slow infantry by a `move_penalty` taken from full speed, read alongside their reload penalty.

A paved area's authored kind also selects its appearance. A map names a rural road `country_road` and a town street `road`; the renderer does not infer a different kind from the other surfaces present. The biome owns a country road's local appearance through built ground.

Vehicle surface speeds are targets, not instantaneous velocity. The drive settings in
[`rules.json`](rules.json) express acceleration and braking as time from rest to
full road speed and back; each vehicle's own top speed sets the rate. This keeps
road entry gradual without reducing the road advantage. Surface and shove limits
act on the target speed so slowdown does not compound every tick. The follower
brakes on the incoming leg of a planned bend, using its turning radius and yaw rate to
choose a corner speed. A tracked pivot blocked by traffic backs up when it needs room.
Stops, collisions and gear changes discard momentum. Infantry retain their own
pace; their yield horizon follows a vehicle's accepted velocity, including reverse
movement. Once clear of that path, a soldier waits rather than stepping back
into it. Accepted vehicle speed participates in replay digests but adds no
observation or command fields. Focused mechanic labs may pin unrelated timing to
isolate their experiment; playable battles use the shared gameplay settings.

## Rivers

A map's water is its `rivers`: each a line of points with the water's `width_m` and its `depth_m` at the middle there, and one `surface_z` for the whole river. The line is rounded like a road's, and width and depth run evenly between points.

- **Water is one distance:** a point is water when it lies within half the width of the rounded line. The simulation classifies by it, the terrain is carved by it and the water is drawn to it (`contract::river`).
- **The cross-section is a V.** The bed falls from the waterline to `depth_m` at the middle, and the bank climbs away at the same grade (depth over half the width) until it has made up the height the land stands above the water at the edge. So `surface_z` below the land sets how far the bank runs: 0.5 m of freeboard at a grade of 1 in 4 is a 2 m bank.
- **A map is refused at load** when its width cannot be represented by the height grid, when a point is so deep for its width that its bank would pass the map's slope cutoff on the grid, when the surface stands above the land at the water's edge, or when a bridge's deck ends over the water or too near it for a bank at the steepest climbable grade to reach the land by its end.
- **Water is crossed at `bridges`.** Along a bridge's approach the bank is steepened into a ramp, so the deck is stepped onto from the land's own height. Author the deck to end a few metres past the water on each side.
- Water is neither road nor forest whatever is authored over it, and no trunk stands in it or within the forest rule's clearance of it.

[The river map](maps/river/) is the worked example. [The shared river contract](../crates/contract/src/river.rs) owns numeric admission and bridge constraints.

## Swamps

A map's standing shallow water is its `swamps`: each an outline polygon and the `depth_m` its bed lies under the water. The swamp carves its own dip. Inside the outline the ground lowers by up to `depth_m`, easing in over a few height samples from the edge, and the water stands level with the land it was cut from.

- **Membership is one distance:** how far a point lies inside the outline. Every rule reads it, and the terrain is cut to it (`contract::swamp`).
- **Wading is slow.** Off a road, a swamp's water scales a ground mover's speed by its locomotion's `swamp` speed factor (`movement.speed_factors`, under [Surface speeds](#surface-speeds)), however deep. A road crosses a swamp uncut.
- **A map is refused at load** when a swamp is a stroke, too steep at its edge for the map's slope cutoff, too small to carve on the grid, not on level land, or within an edge's width of a river, a building or a bridge and its approach.

[The wetlands map](maps/wetlands/) is the worked example. [The shared swamp contract](../crates/contract/src/swamp.rs) owns its admission.

## Paired records

[Paired records](parity/) hold an input and what the native build made of it, which a Rust test and a web test both read. The simulation and the map generator run natively and as WebAssembly, and the same request must give the same bytes in both, or a shared seed, a saved map or a replay means different things in different places; the page, which reads publications through the Rust reader compiled to WebAssembly, must also read exactly what the Rust encoder wrote. The shared record is how the two test suites agree without one invoking the other. The simulation pairs include firing and physical impacts: a movement-only match cannot catch drift in weapon cycles, flight or damage. State digests remain exact even when the packed Float32 observations agree; the authority uses the same pinned math implementation for combat angles, random spread, sampled aim and seeded soldier placement in native and WebAssembly builds. The publication pairs check construction before the first tick; float32 delivery can hide a different initial authoritative state. [Template fixtures](parity/templates/) also hold the labelled descriptors those tests are given, including the ones that must keep failing.

`BLESS_PARITY=1` on the Rust test that reads a record re-records its native half, for a named behaviour change; the web test then holds Wasm to it.

The [capture files](parity/capture/) are browser runs of the menu reel (`native-parity-capture`), re-recorded whole, never edited: a change to the publication layout means a new run. The paris-corner run is copied as it is, and gzipped for the native counts. The market-town window is seven samples of that battle from a fresh publisher, so its first sample is a full snapshot. The run gives its layout and its first tick's frame record. The samples come from replaying the battle natively (`rules.json` with the capture catalog), and they match the run's own samples except for the fog revision counters.

The generation corpus pins its [physical rules](parity/map-layout/physical-rules.json)
as well as its requests. The record supplies complete battle rules for admission;
only the generator's physical fields enter generation identity. That document is
frozen and hand-edited, never regenerated from current game rules. A schema change needs a deliberate edit
that preserves unrelated frozen values. Changes to shipped unit vision can
change the sight certificate and forest placement; they must not silently
change the inputs behind a frozen output. Current-rule generation and
preparation remain covered separately.

A record belongs here only if both sides read it, and it stores a hash of the output unless a reader needs the bytes. A snapshot of one implementation's own output, read by that implementation alone, is not a paired record: state the rule in a test instead.


## Forest ground

Uncleared forest ground uses the single `forests.rule.concealment` multiplier
for every unit; differences in how well bodies hide belong on the units.
Trunk crowns separately own sight-line attenuation. The required
`sensors.moving_concealment` multiplier applies while a move order is in
progress; it is at least one, so moving cannot improve concealment.

## Weapon looks

Every weapon's `appearance` names one entry in [presentation](presentation.json).
[The look reader](../crates/presentation/src/looks.rs) resolves the battle's rows
through the contract's ordinary inheritance, then supplies sight, kick and sound
from that one look. Firing recordings and sound levels are explicit; other sound
parts may use [the mix's](audio.json) default recording. Unknown looks fail at
fielding, and unused looks fail the shipped-fixture check. Changing which look a
weapon names changes battle identity; tuning that look does not.

## Sound catalog

[The sound catalog](sounds.json) owns source receipts, exact clip crops, reusable
recipes. A stored recording may remain unused;
a reload clip does not create a reload event. [Battle audio](../packages/battle-audio/README.md)
owns preparation and source loudness, while [the sound workbench](../apps/sound-workbench/README.md)
owns audition and reviewed edits. Gameplay ranges, cadence and hearing remain with
their simulation owners.

Its `music` section maps each mood (`menu`, `tension`, `combat`) to tracks, and
each track to the source it was encoded from, the served file's hash and a gain.
Music is streamed, never a clip, so no recipe or assignment names a track. Unit
voices are not in this catalog: no clip here is ever a voice line.

## Voices

[`voices.json`](voices.json) is the only authority on what units say: each role's
script (two takes of every trigger, steady and stressed), each country × role
set's callsign, processing profile and voices, which unit type speaks with which
set (by faction for a shared row), and the retake counts that buy a judged-bad take
again. People write it, and so does the sound workbench's save path; unit rows
carry no voice field. [The voice catalog](../packages/battle-audio/src/voices/voiceCatalog.ts)
validates it, and every playable roster unit must resolve to a set.

[`voices.lock.json`](voices.lock.json) is the generation tool's ledger and has no
other writer: per take, the hashes of what it was made from, of its raw master and
of its served file, its length, transcript, match against the script, provenance,
and the verdict a reviewer gave, bound to the recording it judged; each voice's
design candidates; and each generated bed. Never edit it by hand; the workbench records verdicts through
the tool. [Battle audio](../packages/battle-audio/README.md#generated-voices) owns
generation, review and packs.
