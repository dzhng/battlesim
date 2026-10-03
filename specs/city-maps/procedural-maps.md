# Procedural battle maps: completed unknowns map

**Walk completed with the user, 2026-09-30.** This page is the authoritative map-design brief. It supersedes the map scope, dimensions, generation timing and per-map building bake in earlier interview artifacts. The [README](README.md) owns current implementation status and the ladder. [Current scale direction](scale-direction.md) owns the later startup requirement, architecture extent requirement and latitude to propose performance-driven mechanics; it does not replace the selected map presets below.

## 1. Known knowns: settled territory

- The simulation is the authority on movement, sight, damage and garrisons; presentation reads observations and public static geometry. The [root README](../../README.md) establishes this boundary.
- The [map contract](../../crates/contract/src/map.rs) owns physical ground, buildings, regions and template references; the generator and resolver materialize that geometry once through their shared contracts. Runtime and saved maps consume the same physical definition.
- Terrain, navigation, foliage and occlusion must be measured at the admitted extent. See [height construction and mesh export](../../crates/sim/src/world/terrain.rs), [navigation](../../crates/sim/src/navigation.rs), [foliage](../../crates/sim/src/world/forest.rs) and [visibility](../../crates/sim/src/visibility.rs). Flat ground alone does not remove allocation cost; the README and scale evidence own current admission status.
- The [camera controller](../../packages/renderer-core/src/cameraController.ts) changes orbit intent and target ground height; [camera projection](../../packages/renderer-core/src/camera3d.ts) derives the eye. [C57](slices/C57-camera-clearance.md) owns clearance through public physical queries.
- Appearance sources, fit validation, content hashes and loading belong to [scene-assets](../../packages/scene-assets/README.md). Geometry-node art must remain shared module instances, as established by the [building-art walk](procedural-buildings.md); flattened meshes per placed building are unaffordable.
- Building categories need dedicated sources and silhouettes; stretching an apartment does not supply a detached home, tower or industrial building. The [buildings lane](buildings-lane.md) records the implemented source and fit coverage.
- The [Canvas of Kings clip](https://x.com/MightofMe/status/2104919371605041659) shows bounded district plots filling with streets/buildings and settlements linked across open land. It is a composition reference, not evidence of its subdivision algorithm. Reference images never ship.

## 2. Known unknowns: decisions and their owners

### Closed decisions

| ID | Decision | Why | Closed by |
|---|---|---|---|
| M01 | Seeded procedural maps are the primary, required player map source, with real urban ground and usable surrounding plains. The original all-catalogue requirement, including developer arenas, is superseded by M29. | Town and open-country combat belong together; decorative scenery cannot satisfy the player-map rule. | User; developer scope superseded by M29 |
| M02 | Deliver reviewed, fixed-seed artifacts offline first, then deliver runtime generation in this same spec. Runtime uses the same Rust generator through wasm. | Get a playable checkpoint early while retaining arbitrary runtime seeds as a required outcome. | User |
| M03 | The player's composition controls are **Open / Mixed / Metro** and a separate **Small / Medium / Large** size. There is no coverage slider. Seed is a reproducibility input; polygon editing is a developer workbench surface. | Simple choices describe the kind of battle. | User; developer surface scoped by agent |
| M04 | Playable area: Small = **6 × 6 km**, Medium = **8 × 8 km**, Large = **10 × 10 km**. The [rendered surroundings](scale-direction.md#playable-area-and-rendered-surroundings) extend beyond these bounds. These supersede the earlier selected playable extents; building/street dimensions and weapon ranges retain their metre scale. | The user reduced playable map sizes to improve travel pacing with Broken Arrow-like vehicle speeds; [the transit target](scale-direction.md#vehicle-transit-target) governs movement tuning. | User |
| M05 | Building, street and ordinary town dimensions stay believable and similar across sizes. Larger maps add settlements rather than enlarging houses and streets. | A size preset changes the battlefield, not the scale of its contents. | User |
| M06 | Metro has one dominant central city with smaller surrounding towns/districts and outskirts. Open uses small settlements on predominantly open country; Mixed sits between them. Metro's urban share is set by M18. | The types must have visibly different composition. Exact shares remain OPEN below. | User |
| M07 | Use the six building categories in the table below. Open forbids seven-storey and taller buildings; highrises belong to Metro. | A rural town cannot acquire a tower through an unlucky seed. | User; category boundaries proposed by agent and accepted |
| M08 | Each map has one coherent regional family/palette. NYC, Paris and China remain art sources on invented geography; buildings do not choose unrelated regional styles independently. | Variety should look like a place, rather than an assortment of demos. | User |
| M09 | Roads vary by seed, with accessible settlement streets and [the accepted long-move road coverage/connectivity rule](scale-direction.md#accepted-navigation-behavior). | Maps should not all share one highway arrangement; long journeys should usually have usable road access. | User |
| M10 | Initial generated terrain is flat apart from local riverbeds and banks. [Hills and ridges are deferred within city-maps](deferred-terrain-relief.md), outside its current implementation and completion gates. | Establish composition and scale before relief complicates movement and sight. | User; reaffirmed 2026-10-02 |
| M11 | Seeds vary sparse woodland, larger forests, and river/no-river layouts while retaining useful plains. Sparse woodland means less forest coverage, not a new tree-density rule. | Natural cover provides variety without contradicting the existing one-forest-rule decision. | User; distinction supplied by territory |
| M12 | Generated battle maps have broadly comparable top/bottom town area and forest coverage, demonstrated by forest area or count. Their shapes and exact placements need not mirror. | Approximate composition fairness with varied geography. | User, reacting to the accepted sketch |
| M13 | Camera resolution first tries a nearby clear pose by lifting/sliding, then uses smooth pushback. Zoom, pan, orbit and scripted placement must keep the eye outside buildings. | Dense streets and towers must remain navigable with the camera. | User |
| M14 | Runtime selects **pre-baked, varied building templates**, each assembled from shared instanced modules. No runtime Blender or geometry-node evaluator port. | Runtime seeds must work without a per-map art job. | User |
| M15 | Historical requirement for added lab/benchmark surroundings: superseded by M29. Focused developer arenas retain their original extent and measurement frames; player maps retain their wider rendered-surroundings requirement. | Preserve focused checks while keeping the player's battlefield scope explicit. | User; superseded by M29 |
| M16 | Retain hard schema cutovers; no compatibility shims or cutover scaffolding. Name intentional map/config/digest changes when surroundings or rules change. | One contract and one loader are easier to verify. | User, earlier interview |
| M17 | Building categories do not change the accepted combat rules: bottom three garrison bands, one integrity owner, collapse at ≤6 floors and standing gutting above 6. | The 9+ highrise category is an art/layout distinction, not a new destruction threshold. | Earlier user decisions retained |
| M18 | Metro's urban share is whatever still leaves an 1,800 m open approach at every size, Small included: S7 measured that 40–58% does not on Small, so the preset is tuned down until it does. "Up to roughly 80% urban" (M06) is withdrawn; Metro stays the type with one dominant city. | One approach criterion for every type and size, with no per-size exception. Also bounds Metro's building count, the main scale risk. | User, after S7 |
| M19 | Every type and size offers open approaches of at least 1,800 m across a front of at least 400 m, to settlements in both halves. | Weapons reach 1,800 m; a map without such ground is a different game. | User, after S7 |
| M20 | Light wheeled vehicles do 110 km/h on roads; no vehicle exceeds 130 km/h. | S7: edge-to-centre on Large takes 201–205 s at 97 km/h and 179–183 s at 110, against the three-minute target. Realistic, with everyone driving flat out. | User |
| M21 | Mixed has one clearly larger town among its settlements, and road patterns vary by seed beyond a fixed central crossroads. | S7: Mixed Large had no dominant town and every map shared the same four-arm crossroads. | User, after S7 |
| M22 | The two sides start at the top and the bottom of the map. Every map has a road connection from the bottom edge to the top edge; it may be several roads joined through the middle. A road across the middle from side to side is optional: some maps have one, not every map. | Each side needs a road toward the other and toward the centre. Requiring all four edges to reach the centre forced the same crossroads on every large map. | User, after seeing the layouts |
| M23 | A town is not a dartboard. Districts are irregular blocks laid out along the town's roads, and the town's outline follows them; no round outline cut into pie slices that meet at one point, no wedge of forest sliced into the town. See the [Broken Arrow references](assets/reference/broken-arrow/SOURCES.md). | The ring-and-sector model reads as a diagram, not a place. | User, on a crop of a generated main town |
| M24 | Every playable location on every generated map has interesting physical surroundings, including intervening positions, edges and unbuilt town areas. Country remains lightly furnished: sparse rural homes, short broken tree lines, copses, single trees and low field cover. Preserve the widest 1,800 m × 400 m sight-clear approach in each half; place obstructions beside it. The halves hold about the same of each feature. | The user clarified that the problem applies throughout the map, not only at deployment. Broken Arrow's countryside retains open fields with landmarks distributed between settlements. | User; reaffirmed 2026-10-02; construction choices in [choices](choices.md#open-country) |
| M25 | Ground-level circular sight is interrupted at every playable location, while most bearings remain open at a typical country position. Account for actual circular observers' ranges and eye heights, and demonstrate coverage between samples. Final sight uses the simulation's body and foliage queries; actual published fog is reviewed separately. A finite sample grid or opening view alone cannot prove the global rule. The current shortest circular ground observer is the jeep at 450 m; the old standardized 600 m rifle report is insufficient. | "There should be no point on the map where it's just pure flat with a circle fog of war, every point on the map must be interesting." Narrow directional sight is reported honestly but is not promised an obstruction at every heading: its lobe may fit wholly inside a required clear firing lane. | User's global clarification; circular-versus-directional interpretation recorded in [choices](choices.md#global-country-coverage-scope) |
| M26 | A road is a strip that ends square, keeps one width and one kind from junction to junction, and changes either only under a road that crosses it. Two alike roads meeting end to end are one road round a bend; a branch that comes in at a slant turns to meet the road square. Where a road stops, it stops at the map edge, at a junction, or at the last lot it serves. The simulation and the picture read the same shape. | The owner sent back a road ending in a half-circle and then a sharp fork with a flat heel; an unprimed review then ranked the step where a wide road narrowed at a corner as the worst thing left. Counted flaws fell from 362 to 6 per roughly 24,000 road ends. | User (the two pictures); measured |
| M27 | Streets run from a junction to a junction, or end at the last lots by the town's edge; a suburb's streets have cross streets, so they bound blocks. A city has two to four secondary roads out of its main roads and grows along them in arms with fields between; a large town always has a second country road at an angle to the first. A secondary road is part of the block structure: it leaves its road at a junction of its own, square or beside another road, never at a slant, and the blocks either side are cut along it. A junction has four arms at most; a street meets a road within 20° of square, bends to meet it square, or stops a block short. The top and bottom roads meet at one junction. A town, a large town and a city have a park or two near the centre and an unbuilt block here and there. The core stays near the middle of a Metro city (M23). | Unprimed reviews: a city read as a disc round one crossroads, a large town on one road as a slab, suburb streets as wood grain, and link streets left stubs, double bends and near-parallel pairs; then, of the slanting secondary roads, five-arm junctions, shallow forks, hooked street ends and streets bending in unison, and no open ground inside a town. Each wart is counted over a seed sweep and held at its new count by a test. | Measured; the look is still open |
| M28 | Generated towns are furnished by rule with the bodies the simulation already has: parked cars in runs, lamps, street trees on avenues, bins and benches, skips and pallets in industrial yards, the odd construction site. Nothing stands on a carriageway, within 8 m of a junction corner, in front of a door, on a bridge or in an open approach, and every street stays drivable by the widest vehicle. Roadblocks and wrecks are not placed: a roadblock is the defender's to prepare, so it belongs to the encounter planner. | A street fight needs something to crouch behind and a town should read as lived in. The earlier verge compromise is resolved by [C46's kerbside checkpoint](slices/C46-street-placement.md#outcome--kerbside-placement-2026-10-02), with actual widest-hull traversal and preserved parked cover. | Designed here; physical kerbside checkpoint integrated 2026-10-02; appearance acceptance remains C54 |
| M29 | The player's maps are generated ones and saved generated ones. A saved map is a catalogue folder holding each building as its template and frame; `market-town` is the first, listed in the menu. The village, the labs and the benchmark fields are developer test arenas: they stay the small maps they are and get no surroundings, and the village sits behind the menu's developer link. | "I'm not playing village... that's just a test map that's used for all the labs." A generated Small map saved with materialized buildings was 9 to 15 MB; as templates and frames it is 0.7 MB. This supersedes the earlier decision that every catalogued map gains surroundings (C56, C34–C36: cut). | User |
| M30 | The menu asks for a map type and a size and nothing else. Each visit draws a seed silently; it rides in the battle's address, which is how a battle is shared and replayed, and is shown nowhere. | "Delete the seed. Nobody knows what the seed means." | User |
| M31 | Startup under 30 seconds from Deploy to a playable battle is good enough, and room to spare against a budget is spent on a simpler design. One world is built in the preparation worker for the encounter planner and the battle; the page builds its own plain world from the same map for picking, height and camera clearance, with the simulation's own query code. | Deploy to playable is 5 to 6 s on the largest map. A second, page-side query implementation that had to be kept equal to the first cost about 1,000 lines for no measured startup gain and was removed. | User |
| M32 | Finish playable 6/8/10 km maps before further optimization. Defer the 20 km architecture requirement outside current completion gates; fairness gets cheap corrections for low-hanging or obviously wrong cases. | The user prioritizes a playable spec closeout; [current scale direction](scale-direction.md#current-closeout-direction) owns the detailed scope. | User, 2026-10-02 |

### Building categories

| Category | Floors | Composition role |
|---|---|---|
| Farmstead | 1–2 | Rural courts and farm buildings |
| Detached home | 1–2 | Small-town and suburban housing |
| Attached home / small storefront | 2–3 | Town centres and street frontage |
| Urban apartment / mixed use | 4–8 | Denser districts; Open never selects a 7–8-floor variant |
| Highrise | 9+ | Metro's city core |
| Warehouse / light industry | Template-defined | Industrial estates and settlement edges; height follows the selected physical template |

The generator's type weights and industrial proportions are measured decisions, not implied by this category list. Building class and regional family are generator/template metadata; gameplay follows physical geometry and the accepted rules.

### OPEN: measured decisions, with an unblocker

| ID | Still open | What unblocks it | Owner |
|---|---|---|---|
| O01 | Town counts, district sizes, building counts/density, urban/plain/forest shares and category weights for each type × size | S7 plan-only full-extent layout trials and S1's throwaway physical demonstrations; G0 writes versioned preset data. The old universal 10% urban / 25% plain minimums are discarded. | G0 |
| O02 | A metric/tolerance for comparable top/bottom town area and forest coverage (area or count) | S7 metrics on multiple accepted, non-mirrored layouts; G0 freezes the metric and tolerance before C52. | G0 |
| O03 | Useful open approaches at 1,800 m weapon scale, including route width and firing opportunities | S1 infantry/vehicle crossing and ranged-engagement demonstrations for each type; G0 defines per-type criteria rather than inheriting the old 200 m corridor. | G0 |
| O04 | Terrain, navigation, foliage, fog, export and publication storage/query architecture; memory, cold-load and generation budgets | S0 allocation model and focused failed-owner representation proofs at all three extents; S1/S3/S4 measurements. Reslice needed owners before production if dense storage fails. | G0 |
| O05 | Template dimensions, variety count, regional coverage, legal attachment/party-wall capabilities and resident kit budgets | S2 export/fit experiments, S5 joins and S6 finite-template generation parity. Missing categories use labelled prototype geometry until their art slices pass. | G0 |
| O06 | Camera clearance margin, nearby-pose limits and recovery smoothing | C57 scripted trajectories against public/side-known geometry. Bounded reversible tuning is delegated there; the no-penetration invariant is fixed. | C57 |
| O07 | Numerical release budgets beyond the existing 30 FPS floor, including initial/full resubscription bytes and strategic overview cost | S0–S4/S7 and G0; steady-state delivery and full snapshots are measured separately. | G0 |

These are explicit spike outputs. Evidence can change the technical architecture and provisional densities; it cannot silently change the fixed dimensions, the type's character, required runtime generation or all-map scope.

## Open-country reference checkpoint (2026-10-02)

The user supplied two more [Broken Arrow countryside views](assets/reference/broken-arrow/SOURCES.md#open-country-reference-target). They reaffirm M24/M25 throughout every generated map, not just deployment: even a field needs nearby tree lines, small woods, occasional homes or building groups, and loose physical objects. Keep open stretches useful. Judge variety and spacing between settlements at tactical and overview scales as well as the opening view.

The existing owner is `mapgen::open_country` ([implementation principles](../../crates/mapgen/README.md#open-country-open_country)), followed by the simulation's sight report and C86's tree-line drawing. C54 requires a continuous coverage argument and explicit treatment of failed placements, alongside native physical queries and pictures. An object count or a non-circular sample alone does not prove the landscape feels interesting. [Hills and ridges remain deferred within this spec](deferred-terrain-relief.md).

## 3. Unknown knowns: taste and tacit context extracted

- **Mixed reads as a mosaic of districts** ([Broken Arrow references](assets/reference/broken-arrow/SOURCES.md)). A town is single-use districts side by side (garden suburb, apartment rows, a tower, an industrial compound with paved yards), loosely strung along a main road, with fields and woods pushing in between them and right up to the last houses. Objectives sit on districts. A compact blob of blended blocks is the wrong picture.
- **Range changes composition.** The user's 1,800 m reminder rejected an attractive but physically cramped map. Review open approaches at tactical range as well as from an overview.
- **Town size stays familiar.** Larger maps get more places to fight; houses and streets retain their dimensions.
- **Fairness is approximate.** The user liked the sketches' top/bottom balance and specifically rejected a requirement for exact symmetry. Review area/count metrics alongside routes; geometry need not mirror.
- **Metro has a centre.** A dominant city surrounded by smaller districts gives a different battle from scattered equal towns.
- **Keep the first terrain simple.** Flat ground was an explicit simplification after discussing hills. Rivers retain local shaping; relief is a future walk.
- **Review in the browser on this Mac.** The useful checkpoint is an infantry/vehicle encounter across the plain/town transition at ≥30 FPS, eventually played with friends. A top-down diagram alone cannot close playability.
- **The accepted [composition sketch](visualizations/map-character.html) is schematic.** It illustrates hypothetical Medium maps, not generated output. Its drawn town counts, percentages and forest outlines are not hidden preset constants.

## 4. Unknown unknowns: landmines and their disposition

The systematic sweep covered the map contract, terrain/nav/fog/foliage/export/publication paths, camera integration, building sources and focused fixtures, plus the affected generation, bake and spike plans. Later drafting extended the original 11-file check into foliage, terrain exports, publication and viewport integration.

### L01 — Full extent costs memory even when empty — OPEN technical gate

**Evidence:** [HeightField::build and mesh](../../crates/sim/src/world/terrain.rs), [NavGrid::build and search scratch](../../crates/sim/src/navigation.rs), [ForestState::new](../../crates/sim/src/world/forest.rs), [OcclusionGrid](../../crates/sim/src/visibility.rs). At 18 km, today's 4 m height sampling is about 20.25 million cells; 2 m navigation is 81 million cells per grid. Search scratch, clearance and per-side storage add to that. These are arithmetic/code findings, not measured timings.

**Why it bites:** building-count benchmarks miss empty-land allocations; smaller camera windows cannot stand in for a full battlefield.

**Changes the plan:** S0 precedes expensive runs, includes opposite-edge activity and budgets allocations before risking an OOM. G0 must reslice terrain/nav/export owners when required. No fallback shrinks Small/Medium/Large.

### L02 — Terrain export duplicates whole meshes — OPEN technical gate

**Evidence:** [export_terrain_positions, indices and triangle surfaces](../../crates/sim/src/world/export.rs) each request the full mesh.

**Why it bites:** a flat 18 km map can fail during world export/upload before the first battle tick.

**Changes the plan:** S0/S3 measure native/wasm/browser peak copies and first usable frame; a bounded terrain/export seam must be specified if required.

### L03 — Even 8 m fog breaks the old delivery assumption — OPEN technical gate

**Evidence:** [publication's FOG_BITS_PER_FLOAT](../../crates/sim/src/publication.rs) packs 16 cells into each f32. An 18 km map at 8 m has 5,062,500 cells, approximately 1.266 MB per full fog payload before other records.

**Why it bites:** the earlier 19.8 KB/tick target is already exceeded at 8 m; finer fog is not the only trigger for C07.

**Changes the plan:** S1 measures steady-state and snapshot delivery separately; C07 is required whenever the measured budget fails, at any cell size. G0 specifies the layout/decoder cutover.

### L04 — Per-map Blender cannot serve arbitrary runtime seeds — DECIDED

**Evidence:** the earlier C13/Q-D called for graph evaluation per generated footprint part; [scene-assets](../../packages/scene-assets/README.md) already owns validated, hashed art.

**Why it bites:** a new runtime seed could create geometry with no matching baked art.

**Changes the plan:** C13 exports reusable physical/template sources before C53; C32 packs their appearance library. Lots select legal template envelopes; art never stretches to invent new floors/footprints. CLI and wasm consume the same physical geometry manifest/hash; compatible appearance has a separate identity. Module instances remain shared.

### L05 — Existing apartment sources do not supply all six categories — DECIDED scope; OPEN sizing

**Evidence:** the [building-art inventory](procedural-buildings.md) and the courtyard [house source](../../packages/scene-assets/blender/house.py).

**Why it bites:** renamed apartment blocks would not look like suburban homes, towers or industrial sheds.

**Changes the plan:** dedicated farmstead/home/tower/industrial silhouette slices incrementally feed C13/C32; S2 records class × regional-family coverage and missing sources. Prototypes may use labelled boxes; release cannot claim category coverage from those placeholders.

### L06 — Camera target height is not eye clearance — DECIDED

**Evidence:** [CameraController.step/place](../../packages/renderer-core/src/cameraController.ts), [eyePosition](../../packages/renderer-core/src/camera3d.ts), [LabViewport steering](../../apps/battle-lab/src/LabViewport.tsx). C24's alpha cutout concerns grilles/signs, not camera collision.

**Why it bites:** zoom, orbit, pan or initial placement can put the eye inside a building, and an unsafe interpolated recovery can pass through a wall.

**Changes the plan:** C57 owns one pure clearance resolver used by input and scripted placement, including swept motion and safe recovery. It reads public geometry and side-known damage, never hidden live destruction.

### L07 — Focused fixtures can lose their purpose — DECIDED

**Evidence:** [navigation's crafted 400 × 200 m arena](../../crates/sim/tests/navigation.rs) demonstrates why focused geometry must stay controlled; shipped worlds are found through the fixture/catalogue and benchmark builders.

**Why it bites:** surrounding urban bodies may change rays, routes, IDs, digests and frame cost even outside a screenshot crop.

**Changes the plan:** M29 cuts C56/C34–C36: village, labs and crafted benchmark fields retain their original arena bounds and get no surrounding content. Production generated and saved-generated maps still need explicit playable/world/render bounds and wider rendered surroundings under [scale direction](scale-direction.md#playable-area-and-rendered-surroundings). A full generated benchmark measures that production world; being a benchmark does not shrink its selected map. Name intentional geometry/config/digest changes and prove arena behavior remains controlled. Low-level geometry probes remain probes of the geometry API; production composition is judged through the common map preparation boundary.

### L08 — Coverage fairness is not tactical parity — SHARP EDGE

**Evidence:** the user's sketch reaction closed approximate area/count balance; movement and sight still follow the [sim geometry](../../crates/sim/src/world/mod.rs).

**Why it bites:** equal forest/town area can leave one side with different routes, bridges or firing positions.

**Changes the plan:** C54 reports coverage and route/range metrics separately. No automatic claim of balanced win rates follows from coverage symmetry.

### L09 — Rivers and templates constrain connectivity — SHARP EDGE

**Evidence:** [bridge-only navigation behavior](../../crates/sim/tests/navigation.rs), C69's river contract, and S5's template joins.

**Why it bites:** a connected road graph can still yield disconnected movement when a bridge, entrance or template attachment is invalid.

**Changes the plan:** validate compiled physical geometry with infantry and vehicle footprints. Bound retries and report the failing seed/config; never substitute a seed or draw a bridge the sim cannot cross.

### L10 — JavaScript can round a u64 seed — DECIDED boundary

**Evidence:** the existing [battle wasm constructor](../../crates/game-wasm/src/lib.rs) takes an f64 seed. JavaScript's Number cannot represent every u64 exactly.

**Why it bites:** routing an arbitrary map seed through that numeric path could break native/wasm map identity.

**Changes the plan:** S6/C55 use a canonical decimal string at the generation JSON/JS boundary. Map-generation seed and battle RNG input are distinct recorded inputs; this generator seam does not silently alter the existing battle RNG bridge.

### L11 — A valid map does not place an encounter — DECIDED owner

**Evidence:** the [runtime preparation](slices/C55-runtime-generation.md) and [fixed-seed encounter](slices/C58-offline-encounter.md) need deployments, objectives and garrisons tied to the actual compiled geometry.

**Why it bites:** a saved village's coordinates/building references cannot serve a newly generated layout.

**Changes the plan:** C59 extends the existing sim scenario builder with one deterministic map-aware recipe planner for runtime and prepared saved outputs. C58's first fixed-seed encounter may be authored in the existing format before that planner lands. It uses authoritative physical/seat queries, bounds candidate attempts, rejects impossible placement and stores the exact prepared encounter/rules in replay inputs.

### L12 — Ruin-ratio tuning can invalidate baked fit — SHARP EDGE with named gate

**Evidence:** [C14 terminal templates](slices/C14-damage-placements.md) fit physical ruin bounds; [C50 balance](slices/C50-durability-balance.md) can tune the ratio after the first bake.

**Why it bites:** changing physical ruin height without new fitted art breaks the immutable template contract.

**Changes the plan:** C50 reruns C14's bake/fit gate only when the ratio changes, publishes a new appearance identity and updates rule/config/replay expectations for the changed rule. Intact physical maps/catalogue do not change from the art rebake. Replay is same-build with stored compiled scenario/rules and matching build/digests; cross-build compatibility and archival art retention are outside scope. C54/C51 consume the final fitted candidate and C50's single full balance report.

## Contracts the builder inherits

```text
MapConfig { type: Open|Mixed|Metro, size: Small|Medium|Large }
GenerationInput { config, seed: DecimalU64String, generator_version, preset_revision, template_catalog_hash }
TemplateGeometryCatalog { hash, templates: geometry + bays + class + regional family + legal joins }
  → one Rust mapgen implementation (CLI / wasm), in this order:
      layout (roads, settlements, woods, rivers) → road joints and ends (M26)
      → parcels, streets and buildings (M23, M27) → open country (M24, M25)
      → street furniture (M28)
  → MapPlan → validate/lower → MapDefinition + provenance + diagnostics

scene-assets offline:
  graphs / authored family scripts → offline source export (C13)
  → deterministic asset packing/fit (C32) → TemplateArtLibrary { art_hash, covers: template_catalog_hash }

battle loading:
  catalogue source OR generated request → one resolver → compiled MapDefinition
  → authored fixed-seed encounter OR shared map-aware recipe planner → prepared scenario
  → same sim, publication, appearance resolution and static chunk owner

camera:
  desired orbit pose + public/side-known obstacles → clear, safely recovered pose
```

Template geometry types belong to `contract`; their art codec belongs to `scene-assets`. The compiler materializes authoritative building dimensions once. The sim reads compiled building geometry, not generator state or an art catalogue. The renderer resolves immutable template references and cannot invent physical dimensions. C00 owns physical descriptor identity; C32 owns compatible appearance identity. Art hashes never enter simulation JSON/digests. C33 prepares one simulation world and transfers public static/query data to the main thread. Runtime same-build replay stores compiled scenario/rules and checks engine build/scenario/config identity. Current validated art may cover that physical catalogue; pixel-exact historical art and cross-build replay are outside this spec.

## Confirm before coding

- Inventory dense permanent/temporary allocations and wasm/browser transfer copies before attempting full-size builds; S0 owns the safe experiment limits.
- Every saved map is a catalogue folder read through the one resolver (C09, C60: done). The test arenas get no surroundings (M29).
- Confirm graph inputs, legal dimensions, floor heights, pre-realize instance taps, class coverage and attribution under the pinned Blender version; S2 owns the matrix.
- Freeze accepted prototype inputs, dependency versions, canonical outputs and evidence by immutable hash; G0 maps each proven behavior to its production owner and parity check.
- Choose and record numerical OPEN outputs at their named gates; do not let provisional values become defaults by omission.

## What to do next

The live handoff is the spec README's [Next Agent Prompt](README.md#next-agent-prompt); the lanes each carry their own status. This brief records the decisions; it is not a work list.
