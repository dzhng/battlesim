# Fixtures

Authored settings belong here, validated by their reader. [Game rules](game.json)
own battle and presentation numbers. [Units](units/), [props](props/) and [biomes](biomes/)
own component catalogs and terrain palettes; [catalog resolution](../crates/contract/src/catalog.rs)
produces the browser's resolved [catalog](catalog.json).

[Map presets](map-presets.json) and the generator's [physical template library](prototype-building-templates.json)
feed [mapgen](../crates/mapgen/README.md). Their templates are derived from accepted
[city sets](../packages/scene-assets/README.md#city-buildings). The [authored template library](building-templates.json)
stays separate because existing authored maps pin its hash and it must not be
silently regenerated from art.

[Encounter recipes](encounters.json) describe rosters, objectives and placement
policy without coordinates; [the simulation planner](../crates/sim/src/encounter/)
places them on admitted geometry. Players never field a recipe: the map workbench
and native reports plan one on a generated map to judge it, and tests use them.
[Generated-battle settings](generated-battle.json) own preparation policy. [Camera lab settings](camera-lab.json)
belong to the isolated camera experiment, not a second gameplay camera policy.
[The menu backdrop](menu-backdrop.json) lists the scenes the main menu films, in order:
each a saved battle (an ordinary saved encounter, scripted orders and all, on its own
map) and its reel of camera shots;
a tracking shot frames by an offset from one of blue's units. Each encounter is staged
like a film, not balanced: set-pieces open fire on a schedule, set by how far each
force starts from its enemy, so every cut has guns firing in frame. Cannon fire and
blasts read at film distance; rifle fire barely does. A changed encounter or rule
(movement included) retimes the set-pieces, so recheck each scene's cuts after
either. Each battle is directed, not left to play out: starting health, scheduled
fire policies and scripted attacks set who fires and who dies when, so a change
keeps the cuts it was not asked to change and retimes the battle to them, never
the reverse.

[Sound authoring](#sound-catalog) owns audio provenance and choices independently
of gameplay weapon cycles. Saved maps, component inheritance and paired evidence
have the contracts below.

## Saved maps

A saved map is one folder, `maps/<id>/`, under [the saved-map catalog](maps/), and nothing reads a map any other way. The id is the folder's name: lowercase letters, digits, hyphens and underscores. It names where the map is stored and nothing else: a route's id and a map's content hash are separate things, and several routes may play one map.

- **`map.json`** is the physical map in its saved form (`contract::map::SavedMap`): ground, relief, water, surfaces, forests, props and buildings. A building is stored as what is its own: `owner`, `kind`, `template_id`, `frame` (translation and yaw) and `parts` (each named part's prop id). Its geometry, category and regional family are the template's and are not stored. The map names its own region once, `regional_family`, which every building's template must share (absent only on a map without buildings): the renderer and the asset loader read the region there and nowhere else. Only the resolver reads this file.
- **`SOURCES.json`** pins what the map is and where it came from (`contract::maps::MapSources`): its content identity (an authored map's hash, or a generated map's whole generation identity), its physical catalogue (`catalogue.library`, the file name of a template library in this folder, and `catalogue.template_ids`, the templates of it the map uses, `null` for the whole library), and a receipt for each input it was made from. A receipt is `repository` (path, revision, sha256) or `supplied` (label, sha256).
- **`meta.json`** is what a listing shows without loading the map: category, status, label, character, biome, size, feature tags, source, seed, and the names of its encounters and benchmarks. [`web/src/maps/catalogue.ts`](../web/src/maps/catalogue.ts) owns the schema, the validator and `listMaps`. Every field the map itself also states is checked against the map, so a listing cannot drift from what loads.
- **The category says what a saved map is for** (`contract::maps::MapCategory`): `test` (a test's, a lab's or a benchmark's ground) or `menu` (a battlefield the menu backdrop films); anything else is refused by both readers. No saved map is the game's: player battles are generated and never name one. The menu backdrop takes only `menu` maps, and labs and native tests only `test` maps (`fixtureMap`, `sim::fixtures::with_map`), so a test needing content another owner has gets its own copy of the map rather than borrowing that owner's.

- **`encounters/<name>.json`** is an encounter laid on the map (`contract::scenario::EncounterDefinition`): units, events and scripted orders, and optionally an opponent and a completion rule. A scenario is the resolved map, the rules and one encounter. The endurance battle is not a file: the simulation builds it from the rules and the resolved map (`sim::endurance`).

Scripted group orders address the formation's surviving own units when their tick arrives. Fallen own actors are removed before ordinary command validation, so one casualty cannot cancel a reserve wave; an all-fallen group has no effect. Unknown or foreign actors still refuse the script, and player commands validate their exact submitted selection. Script execution follows the same rule during live play and replay in [the battle authority](../crates/sim/src/battle.rs).

- **`sites.json`**, on a generated map only, is what the encounter planner reads beside the map (`contract::encounter::EncounterSites`), as the `mapgen` CLI wrote it. Nothing loads it at run time: it is there so the saved encounter can be planned again.

Routes and saved maps have different identities. The [fixture registry](../apps/battle-lab/src/fixtures.json) declares each route's fixed saved-map reference; tools and generated or synthetic worlds have no fixed catalogue source. The developer index offers every active catalogue map through the shared geometry inspector, so saving a new map needs no bespoke route or scene. Player skirmishes generate fresh maps; the saved catalogue serves tests, labs and the menu backdrop.

**One resolver admits every map** (`contract::maps::resolve`), in Rust, so native and WebAssembly readers agree. It materializes every building from its template at its frame, out of the library the sources name, and answers the resolved map (`contract::map::MapDefinition`) that the simulation, the renderer and every test receive. It checks that the map names the catalogue its selection hashes to, and that the resolved map is the one `SOURCES.json` pins: a map's content hash is the hash of its resolved definition. The library's file name only says where to find it; the catalogue hash is what admits it. A building whose template the catalogue lacks is refused by its index. A map that fails is refused with a code and the document and field at fault. Nothing stands in for it, and there is no other saved form: a `map.json` that carries building geometry is refused. Both catalogue adapters admit what the game's generator may make (`MapAdmission::CATALOGUE`, the limits of `generated-battle.json`). Three thin adapters fetch the documents and call it:

- native tests, examples and tools: `sim::maps::load(id)` and `sim::maps::encounter(id, name)`;
- the browser: `loadMap(id)` and `loadEncounter(id, name)` in [`web/src/maps/browser.ts`](../web/src/maps/browser.ts). Each document is its own served file, so no map is part of a script;
- Node tests, scenes and tools: the same two names in [`web/src/maps/node.ts`](../web/src/maps/node.ts), over the built WebAssembly.

**Adding a map:** make the folder, write `map.json` and `meta.json`, and write `SOURCES.json` with the map's library and content hash. The resolver's refusal states the hash it computed, so an authored map's first load tells you the value. A map the compiler makes is saved in this form, with its `SOURCES.json`, by the `mapgen` CLI.

A released saved generated map is a reviewed, fixed battlefield. Its template
hash must continue to resolve; changing physical templates may require generating
and reviewing that map again. [The mapgen CLI](../crates/mapgen/src/main.rs) owns
saving generated maps, and [encounter reporting](../crates/sim/examples/encounter_report.rs)
owns saving a planned encounter. The [catalog tests](../crates/sim/tests/maps.rs)
and [browser tests](../web/tests/mapCatalogue.test.ts) hold each folder to admission
and listing policy. Directory discovery requires no parallel index.

## The unit catalog

Deployment preserves one reversible progress value. Setup and packing may have different durations; the integer progress lattice makes both directions and partial reversals deterministic. Movement waits for packing to finish, and supply waits for setup to finish.

A unit type is **one catalog entry**, addressed by its string id (`"us_m1_abrams_sep_v3_trophy"`, `"test_tank"`). No code lists unit types. Scenarios, spawn rows and commands name types by id; the publication sends the id list (`unitKinds`) and each unit's index into it.

[The catalog parser](../crates/contract/src/catalog.rs) owns component schemas,
validation and inheritance. Families contain unit, soldier, role and upgrade
records; ids are defined once across authored files. Behavior comes from body,
mobility, sensors, mounts and capabilities rather than the type's name.

- **Soldier kinds** are a catalog of their own: `hp`, the `appearance` set a soldier of the kind wears (one picked per soldier), and the `mounts` he carries. A `special` mount passes to the next living soldier when its carrier falls; any other is lost with him. A single-operator mount may name `operator_appearance` with paired active and carried appearance sets: the equipment worn by its current living carrier while using that weapon or his rifle, including after a handoff. Variant identity stays the same across both sets. The observation publishes the carrier and each soldier’s active weapon only when visible. A squad's slots name soldier kinds, so hundreds of squads reuse a few kinds.
- **A variant is `extends` plus overrides.** `"m1a1": { "extends": "m1", "body": { "hull": { "armor": { "front": 180 } } } }` inherits everything else. An `abstract` entry only exists to be extended. Weapon rows in `game.json` extend the same way. The merge:
  - objects merge key by key, and a list of named objects (mounts) merges by name, a new name appended;
  - a unit's one-key variant component (`body`, `mobility`) written as another variant replaces the parent's: `"mobility": { "wheeled": … }` over a tracked parent is wheeled;
  - a unit's `parts` gather along the chain, the parent's first;
  - anything else is replaced, a list of strings (`roles`, `slots`) whole.
  - What a variant can't do: drop a key or a named mount it inherits. A type that loses a mount extends a common parent instead.
- **Parts are upgrades:** `"parts": ["trophy_aps"]` merges each part's `patch` into the type after inheritance, by the same rules. A part that needs a capability the simulation doesn't build yet is refused at load, because the type no longer parses. A part names the model nodes that show its hardware (`nodes`), and the type's model must draw them.

Resolution happens once through the contract owner. Invalid inheritance,
references, physical mounts or numbers fail at load with an entry-specific error.
[Catalog tests](../crates/sim/tests/catalog.rs) own the explicit regeneration gate
for `catalog.json`; browser presentation reads that resolved view, including
resolved weapons, rather than reinterpreting authored families.

A battle runs one of three document sets (`sim::fixtures::CatalogSet`): the
game's, the only one committed as `catalog.json` and holding only what roster
cards reach; the test set, which adds the [test units](units/test/) (`test_*`)
that tests, labs, scenes and art checks field; and the menu set, which adds the
test units and the menu reel's own units. The test and menu sets are resolved when asked for and
never written, so one generated file stays the editors' and publication's. The
browser's [session factory](../web/src/battle/catalog/) is the one reader of
`catalog.json`, and each battle session owns the set its page runs. After a catalog
change, the [asset CLI](../web/asset.mjs) regenerates and checks model-derived icons.

## Weapon cycles

Engagement distances belong to each weapon row. Minimum range is a crew's firing restriction measured from the actual muzzle to the aim point, not a delayed arming fuse: already-fired rounds still collide and explode normally. A too-close weapon holds its rounds rather than requesting an advance, while other mounts remain independent. Target selection and each physical shot enforce the same minimum, including sampled contacts and individual soldiers.

A weapon row's `ammo` is total carried rounds, including loaded magazines; `"unlimited"` means unlimited reserves. `magazine` adds a physical gun's capacity and interval between shots, while `reload_s` is the pause to replace its magazine or belt. An optional `burst` groups rapid shots. The [fire cadence rationale](../specs/done/fire-cadence/README.md) explains their readiness and interruptible idle reload contract.

The nominal vehicle aim height comes from its body, with a fixture-owned fraction allowing fire into both the upper body and hull. Direct-fire scatter stays at or below that aim point. A round that survives to its aim plane falls under stronger downward gravity, chosen at launch to reach even the lowest ground within the authored fall-time limit. Its horizontal flight and its position and velocity at the join stay intact. Both legs use ordinary swept collision: intervening bodies still decide the first impact. A ricochet cancels that fall and keeps its deflected direction under ordinary gravity; its remaining lifetime is capped from the first bounce, without restarting on later bounces. Guided fire keeps its own guidance, and indirect fire keeps its ballistic arc.

Tracer frequency and shape belong to presentation. A round's sampled visibility persists across its observed flight; it does not flicker with each publication. Small-arms traces use a fixed screen-width line, while muzzle flashes, ricochet sparks and ground impacts still show their own published causes. Hiding a tracer never hides its physical impact or changes the shot.

Impact puffs keep the material of the surface they strike, with per-round size and lifetime overrides inside that surface's style. Heavy rounds can leave larger, longer-lived dirt clouds without extending armour flashes or explosion smoke; the effect owner's lifetime bound includes those overrides.

Each physical weapon owns its targeting, ammunition and firing cycle. Identical special weapons stay separate, so their rows can show different rounds and reload progress. Ammo quantities are authored per mount: adding another identical gun adds another authored load. The only shared row is infantry's default gun (`squad` in the soldier's mount), whose copies keep independent firing cycles and have no reload pause. Its readout stays loaded while any surviving carrier is loaded. Suppression's tier widens launch scatter as well as slowing cycle progress; those penalties compose with movement and cover.

A replacement soldier brings a fresh default gun; a transferable special weapon keeps its identity, rounds and reload progress across handoffs. Living original carriers operate their own guns, survivors retain recovered assignments, and free soldiers take up remaining stocked weapons. One soldier operates one special weapon at a time. Additional recovered weapons remain spares: their reload pauses until an operator is available, while movement still interrupts stationary weapons. An empty launcher's operator finishes guiding its last missile before taking up a stocked spare. Guidance belongs to the physical launcher, so an eligible survivor can continue an in-flight missile; loss of support, movement, Stop or lost sight still releases it.

## Adding a unit type (a tank variant, a vehicle, an infantry type)

Add one entry. A variant is an `extends` and what differs. Code learns nothing about the type, and changes only where a genuinely new behaviour appears, as a new component or capability. Each rule has a guard; if you add a type that the guard doesn't cover, extend the guard.

- **Never branch on a type's id or role for a rule.** If a function asks "is this a tank?" to pick an offset, a size or a speed, that value belongs in the type's components. Scripts and the AI may select units by role; rules never do.
- **Every mount is its own weapon.** It has its own lock, bearing, pivot and muzzle. A mount on a turret pivots with the turret; a mount on its own ring (a roof HMG) turns on its own bearing about its pivot. Two mounts never share a muzzle point. A vehicle model declares which of its rigs draws each mount in its `assets/catalog.json` entry (`"mounts": { "cannon": "gun", "HMG": "hmg" }`); a mount's name decides nothing.
- **The simulation's numbers are the authority, and art is fitted to them.** Hull extents, eye heights, mount pivots and muzzles, the canopy and the footprint are set in data. Each type's own model is fit-checked against its own resolved numbers (the `fit.*` checks in `packages/scene-assets/src/validate.ts`). A variant with a new model inherits its parent's mount geometry, so the new model must put its turret and gun where the inherited mounts say, or the variant overrides those numbers. When you add a number with a drawn counterpart, add its `fit.*` check in the same change.
- **Presentation anchors to what is drawn, not to the simulation's points.**
  - Effects that belong to a model part (muzzle flashes, exhaust, dust at the tracks) sit on the drawn part's socket, and fall back to the published point only when nothing is drawn.
  - Ground markers are sized from the type's footprint (its hull), so they peek out from under the model.
  - The renderer skill has the mechanics (`.agents/skills/renderer/SKILL.md`).
- **Culling bounds cover every reachable pose:** a traversed turret, a deployed spade, a raised gun. Size them from the articulation's extremes, not the rest pose.
- **A vehicle needs its whole life cycle:** its `mobility`, its weight and push class (what it pushes, and what it's worth as cover, live or wrecked), its `wreck` (a prop row with hit points and a destroyed state, which may chain into a lighter wreck), and its `sound`. Missing one shows up late, as a vehicle that can't be pushed, or a wreck that never burns out.
- **Presentation follows physics, never the id.** A vehicle sounds and articulates as its class (`vehicleClass` in `packages/scene-assets/src/units.ts`: tracked or wheeled, its hull's weight class, and `_logistics` for a supply hauler), and a derived weapon row looks and sounds like its nearest styled ancestor. A new type of a fielded class needs no presentation edit; a new class needs its rows in `presentation.audio.vehicles` and `presentation.pose.gauge`.
- **Infantry** share one body frame: the soldier's radius, height, eye and muzzle heights, and the cover rules, are `physics` numbers here, not per soldier kind. Variants within one appearance set share a skeleton; each soldier animates with the clips of his current appearance.

**The guards scale with the catalog.** Every resolved type runs the same generated checks with no test of its own: its components are complete and in range, it sets up, fires each mount and moves (`crates/sim/tests/catalog.rs`), its appearance exists and fits its numbers, and its icons exist. Then add the type to the lab or scene that exercises it: the workbench sheet for the model, and a movement scenario (`crates/sim/tests/movement_scenarios.rs`) for its movement and cover. Run the checks for whatever you changed (see [`AGENTS.md`](../AGENTS.md)).

## The prop catalog

A prop type (a house, a wall, a tree, a wreck, rubble) is **one entry** of a `props` section, in `props/<faction>/<family>.json`, addressed by its string id. No code lists prop types, and no rule asks which one a prop is. A map's props, a forest's trees (`forests.tree` in `game.json`), a bridge's deck (`deck` on each map bridge) and a vehicle's wreck all name types by id; the world layout and the publication send the id list (`propKinds`) and each prop's index into it.

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

- **Never branch on a prop's id.** If code asks "is this a building?" or "is this a tree?", the answer belongs in a column (`garrison`, `topples`) or in the appearance binding (`drawn_by`).
- **A unit type's wreck is its own prop type.** At hundreds of vehicles, give each family's wreck an entry extending `wreck`, with the vehicle's weight class and cover tier (a wreck keeps its vehicle's tier, checked at load), `hp`, and a destroyed state that chains into a lighter wreck or goes.
- **Every type needs something that draws it:** a scenery kind in `packages/scene-assets` with a catalog appearance authored to a box the simulation places (the catalog footprint test holds every drawn kind to one), `forest` for a tree, or `building` for a part of a building, whose template the art library dresses. The renderer reads the binding from the world layout; it lists no prop types.
- Then add it to the map or scenario that exercises it, and run the checks for what you changed (see [`AGENTS.md`](../AGENTS.md)).

## Surface speeds

Every mover states its own two top speeds in its `mobility` row, in km/h: `offroad_kmh` on open ground and `road_kmh` on a full road (validated against its off-road speed and the catalog's limits). `game.json`'s `surfaces` table has one row per surface kind a map may pave (`road`, `country_road`, `dirt_track`, and `paving`, hard ground that is no way through: a yard, a court). A row's `speed_factor` scales each unit type's own road speed on that surface, never below its off-road speed: 1 is a full road, 0 is no road at all. A new surface kind is a new row plus its variant in `contract::map::SurfaceKind`.

A paved area's authored kind also selects its appearance. A map names a rural road `country_road` and a town street `road`; the renderer does not infer a different kind from the other surfaces present. The biome owns a country road's local appearance through built ground.

Vehicle surface speeds are targets, not instantaneous velocity. The drive settings in
[`game.json`](game.json) express acceleration and braking as time from rest to
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

## Paired records

[Paired records](parity/) hold an input and what the native build made of it, which a Rust test and a web test both read. The simulation, the map generator and the encounter planner run natively and as WebAssembly, and the same request must give the same bytes in both, or a shared seed, a saved map or a replay means different things in different places; the TypeScript decoder must also read exactly what the Rust encoder wrote. The shared record is how the two test suites agree without one invoking the other. The simulation pairs include firing and physical impacts: a movement-only match cannot catch drift in weapon cycles, flight or damage. State digests remain exact even when the packed Float32 observations agree; the authority uses the same pinned math implementation for combat angles, random spread, sampled aim and seeded soldier placement in native and WebAssembly builds. The publication pairs check construction before the first tick; float32 delivery can hide a different initial authoritative state. [Template fixtures](parity/templates/) also hold the labelled descriptors those tests are given, including the ones that must keep failing.

`BLESS_PARITY=1` on the Rust test that reads a record re-records its native half, for a named behaviour change; the web test then holds Wasm to it.

The generation corpus pins its [physical rules](parity/map-layout/physical-rules.json) as well as its requests. Changes to shipped unit vision can change the sight certificate and forest placement; they must not silently change the inputs behind a frozen output. Current-rule generation and preparation remain covered separately.

A record belongs here only if both sides read it, and it stores a hash of the output unless a reader needs the bytes. A snapshot of one implementation's own output, read by that implementation alone, is not a paired record: state the rule in a test instead.


## Forest floor cover

The one `forests.rule` controls sparse log and boulder candidate density and
physical size. The forest's catalog references select their body properties.
The world places floor cover after all trunks with independent seeds, so changing
floor density cannot shift any trunk or its published source range. Candidates
that conflict with trees, roads, water, bodies or the forest boundary are omitted;
per-hectare densities are placement ceilings, not guaranteed counts. The floor
adds no concealment. Uncleared forest ground owns the binary concealment bonus; trunk crowns own sight-line attenuation.

## Sound catalog

[The sound catalog](sounds.json) owns source receipts, exact clip crops, reusable
recipes and exact unit/mount assignments. A stored recording may remain unused;
a reload clip does not create a reload event. [Battle audio](../packages/battle-audio/README.md)
owns preparation and source loudness, while [the sound workbench](../apps/sound-workbench/README.md)
owns audition and reviewed edits. Gameplay ranges, cadence and hearing remain with
their simulation owners.
