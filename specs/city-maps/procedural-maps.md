# Procedural battle maps: completed unknowns map

**Walk completed with the user, 2026-09-30.** This page is the authoritative map-design brief. It supersedes the map scope, dimensions, generation timing and per-map building bake in earlier interview artifacts. The [README](README.md) owns current implementation status and the ladder. [Current scale direction](scale-direction.md) owns the later startup requirement, architecture extent requirement and latitude to propose performance-driven mechanics; it does not replace the selected map presets below.

## 1. Known knowns: settled territory

- The simulation is the authority on movement, sight, damage and garrisons; presentation reads observations and public static geometry. The [root README](../../README.md) establishes this boundary.
- The current [map contract](../../crates/contract/src/map.rs) has bounds, sampled terrain, roads, water, forests and prop boxes. Building aggregates, region polygons, template references and generation requests are planned contracts, not existing APIs.
- Terrain, navigation, foliage and occlusion currently allocate over the full map. See [height construction and mesh export](../../crates/sim/src/world/terrain.rs), [navigation](../../crates/sim/src/navigation.rs), [foliage](../../crates/sim/src/world/forest.rs) and [visibility](../../crates/sim/src/visibility.rs). Flat ground alone does not remove their allocations.
- The [camera controller](../../packages/renderer-core/src/cameraController.ts) changes orbit intent and target ground height; [camera projection](../../packages/renderer-core/src/camera3d.ts) derives the eye. There is currently no building-clearance resolver.
- Appearance sources, fit validation, content hashes and loading belong to [scene-assets](../../packages/scene-assets/README.md). Geometry-node art must remain shared module instances, as established by the [building-art walk](procedural-buildings.md); flattened meshes per placed building are unaffordable.
- The existing [house source](../../packages/scene-assets/blender/house.py) is a courtyard farmstead. The three vendored sources provide urban apartment archetypes. A detached home, a true tower and industrial buildings require dedicated sources; changing their scale does not supply the missing silhouettes.
- The [Canvas of Kings clip](https://x.com/MightofMe/status/2104919371605041659) shows bounded district plots filling with streets/buildings and settlements linked across open land. It is a composition reference, not evidence of its subdivision algorithm. Reference images never ship.

## 2. Known unknowns: decisions and their owners

### Closed decisions

| ID | Decision | Why | Closed by |
|---|---|---|---|
| M01 | Seeded procedural maps are the primary, required map source. Every catalogued map has real urban ground and usable surrounding plains, including the village, labs, benchmarks and catalogued test maps. | Town and open-country combat belong together; decorative scenery cannot satisfy the rule. | User |
| M02 | Deliver reviewed, fixed-seed artifacts offline first, then deliver runtime generation in this same spec. Runtime uses the same Rust generator through wasm. | Get a playable checkpoint early while retaining arbitrary runtime seeds as a required outcome. | User |
| M03 | The player's composition controls are **Open / Mixed / Metro** and a separate **Small / Medium / Large** size. There is no coverage slider. Seed is a reproducibility input; polygon editing is a developer workbench surface. | Simple choices describe the kind of battle. | User; developer surface scoped by agent |
| M04 | Playable area: Small = **6 × 6 km**, Medium = **8 × 8 km**, Large = **10 × 10 km**. The [rendered surroundings](scale-direction.md#playable-area-and-rendered-surroundings) extend beyond these bounds. These supersede the earlier selected playable extents; building/street dimensions and weapon ranges retain their metre scale. | The user reduced playable map sizes to improve travel pacing with Broken Arrow-like vehicle speeds; [the transit target](scale-direction.md#vehicle-transit-target) governs movement tuning. | User |
| M05 | Building, street and ordinary town dimensions stay believable and similar across sizes. Larger maps add settlements rather than enlarging houses and streets. | A size preset changes the battlefield, not the scale of its contents. | User |
| M06 | Metro has one dominant central city with smaller surrounding towns/districts and outskirts. Open uses small settlements on predominantly open country; Mixed sits between them. Metro's urban share is set by M18. | The types must have visibly different composition. Exact shares remain OPEN below. | User |
| M07 | Use the six building categories in the table below. Open forbids seven-storey and taller buildings; highrises belong to Metro. | A rural town cannot acquire a tower through an unlucky seed. | User; category boundaries proposed by agent and accepted |
| M08 | Each map has one coherent regional family/palette. NYC, Paris and China remain art sources on invented geography; buildings do not choose unrelated regional styles independently. | Variety should look like a place, rather than an assortment of demos. | User |
| M09 | Roads vary by seed, with accessible settlement streets and [the accepted long-move road coverage/connectivity rule](scale-direction.md#accepted-navigation-behavior). | Maps should not all share one highway arrangement; long journeys should usually have usable road access. | User |
| M10 | Initial generated terrain is flat apart from local riverbeds and banks. Hills and ridges are deferred to the [terrain-relief placeholder](../terrain-relief/README.md). | Establish composition and scale before relief complicates movement and sight. | User |
| M11 | Seeds vary sparse woodland, larger forests, and river/no-river layouts while retaining useful plains. Sparse woodland means less forest coverage, not a new tree-density rule. | Natural cover provides variety without contradicting the existing one-forest-rule decision. | User; distinction supplied by territory |
| M12 | Generated battle maps have broadly comparable top/bottom town area and forest coverage, demonstrated by forest area or count. Their shapes and exact placements need not mirror. | Approximate composition fairness with varied geography. | User, reacting to the accepted sketch |
| M13 | Camera resolution first tries a nearby clear pose by lifting/sliding, then uses smooth pushback. Zoom, pan, orbit and scripted placement must keep the eye outside buildings. | Dense streets and towers must remain navigable with the camera. | User |
| M14 | Runtime selects **pre-baked, varied building templates**, each assembled from shared instanced modules. No runtime Blender or geometry-node evaluator port. | Runtime seeds must work without a per-map art job. | User |
| M15 | Focused labs and benchmarks retain their test/measurement arena and receive real town/plain surroundings outside it. Their bounds remain purpose-appropriate; primary generated battles use player size presets. Surroundings remain normal sim geometry and draw cost. | Preserve focused checks while satisfying the all-map rule. | User, final A choice; diagnostic bounds scoped by agent |
| M16 | Retain hard schema cutovers; no compatibility shims or cutover scaffolding. Name intentional map/config/digest changes when surroundings or rules change. | One contract and one loader are easier to verify. | User, earlier interview |
| M17 | Building categories do not change the accepted combat rules: bottom three garrison bands, one integrity owner, collapse at ≤6 floors and standing gutting above 6. | The 9+ highrise category is an art/layout distinction, not a new destruction threshold. | Earlier user decisions retained |
| M18 | Metro's urban share is whatever still leaves an 1,800 m open approach at every size, Small included: S7 measured that 40–58% does not on Small, so the preset is tuned down until it does. "Up to roughly 80% urban" (M06) is withdrawn; Metro stays the type with one dominant city. | One approach criterion for every type and size, with no per-size exception. Also bounds Metro's building count, the main scale risk. | User, after S7 |
| M19 | Every type and size offers open approaches of at least 1,800 m across a front of at least 400 m, to settlements in both halves. | Weapons reach 1,800 m; a map without such ground is a different game. | User, after S7 |
| M20 | Light wheeled vehicles do 110 km/h on roads; no vehicle exceeds 130 km/h. | S7: edge-to-centre on Large takes 201–205 s at 97 km/h and 179–183 s at 110, against the three-minute target. Realistic, with everyone driving flat out. | User |
| M21 | Mixed has one clearly larger town among its settlements, and road patterns vary by seed beyond a fixed central crossroads. | S7: Mixed Large had no dominant town and every map shared the same four-arm crossroads. | User, after S7 |

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

**Changes the plan:** C56 proves the reservation contract on one arena; C34/C35/C36 cut over village/labs/benchmarks with declared protected bounds and behavior probes. Inventory every shipped/catalogued world, including synthetic benchmark worlds. Name geometry/config/digest changes; rebaseline whole-map costs only after arena behavior is proved. Low-level geometry probes remain probes of the geometry API; the composition gate belongs at the catalogued-map boundary.

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
  → one Rust mapgen implementation (CLI / wasm)
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
- Find every map/fixture/benchmark producer before declaring the all-map cutover complete; C56 owns the inventory and arena bounds.
- Confirm graph inputs, legal dimensions, floor heights, pre-realize instance taps, class coverage and attribution under the pinned Blender version; S2 owns the matrix.
- Freeze accepted prototype inputs, dependency versions, canonical outputs and evidence by immutable hash; G0 maps each proven behavior to its production owner and parity check.
- Choose and record numerical OPEN outputs at their named gates; do not let provisional values become defaults by omission.

## Copyable next implementation prompt

> Implement the first scale spike S0 from `specs/city-maps/README.md`. Treat `procedural-maps.md` as the authoritative brief. Use the current Small/Medium/Large extents in M04 and the startup/transit policy in `scale-direction.md`, and budget dense allocations before running them. Measure safe current-path full-extent startup and opposite-edge activity where the allocation model permits; record predicted ceiling failures without allocating them. G0 may immediately reslice failed owners into focused representation proofs before dependent spikes resume. This is a throwaway spike: no production lane merges before G0. Preserve the accepted types, six building categories, runtime template path, approximate top/bottom fairness and real urban/plain surroundings on all catalogued maps. Use the repo's narrow verification runners and update the handoff with the measured verdict and required architecture reslicing.
