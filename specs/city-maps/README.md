# Procedural battle maps: towns and open plains

Generate seeded battle maps with urban areas and usable surrounding plains on **every catalogued map**, including the village, labs and benchmarks. The player selects Open/Mixed/Metro and a separate Small/Medium/Large size, defined in the [completed unknowns map](procedural-maps.md). Larger maps add settlements at stable building/street scale. Roads connect towns to open approaches useful for 1,800 m weapons. Metro has a dominant central city with smaller surrounding districts; top/bottom town and forest coverage is comparable without mirrored geography. Maps play under the village's rules: Hollywood realism, physical fire and sharp fog of war.

- **Buildings** are sim bodies selected from legal, varied templates in six categories. An offline bake produces shared kit modules and reusable template placements from the NYC/Paris/China sources and dedicated missing families. Each map uses one coherent regional family. Runtime generation selects templates; it never runs Blender or stretches art to invent a footprint.
- **Streets** are the ground: roadbed, sidewalks and curbs.
- **Street furniture** is bodies.

**Terrain scope:** the initial procedural generator uses broadly flat ground; rivers retain their local bed and bank shaping. Hills and ridges are deferred to the [terrain-relief placeholder](../terrain-relief/README.md).

**Done** means friends play a generated encounter spanning town and plain on this Mac at ≥30 FPS, with safe camera movement around buildings. Reviewed fixed-seed artifacts ship first; runtime generation is required before this spec closes. A gallery covers every type × size across multiple seeds. Focused labs/benchmarks keep their arena and gain real surroundings outside it. Buildings stop rounds and sight at their physical heights; squads use the bottom three floor bands. The accepted collapse/gutting rules remain unchanged by category names.

**Read with this README:**
- [`scale-direction.md`](scale-direction.md): current user direction for startup, the architecture extent requirement and proposed performance-driven behavior changes; supersedes older exact-route requirements for named alternatives.
- [`procedural-maps.md`](procedural-maps.md): the completed four-quadrant walk, accepted sizes/categories/presets, landmines, OPEN spike outputs and copyable kickoff prompt. It is authoritative for current map scope.
- [`decisions.md`](decisions.md): retained combat decisions, historical interviews and synthesis rationale; current map scope lives in `procedural-maps.md`.
- [`procedural-buildings.md`](procedural-buildings.md): the unknowns map for the vendored building graphs (Q-A … Q-J, landmines L1–L11).
- [`ground-look.md`](ground-look.md): the unknowns map for open-country ground (roads, rivers, forests, grass, farms) and the map catalogue (Q-G1 … Q-G19, L-G1–L-G11). It owns spikes SG1–SG6, gate GG and slices C60–C87. The ground synthesis (four drafts) is in `decisions.md`, "Ground synthesis".
- [`research.md`](research.md): historical NYC data research and source evaluation; its real-data recommendation is superseded for this spec.
- `slices/`: one file per slice, the contract you implement.
- [`choices.md`](choices.md): the ledger of implementation choices the spec didn't make.
- [`sim-lane.md`](sim-lane.md): the finished lane of simulation rules and costs, with what landed and what it left open.

## Next Agent Prompt

**Status (2026-10-02, handed to a new session on another machine):** everything built is on `main`; one session works the spec now, and every lane is finished. A player starts a battle on a generated map from the main menu (Map and Size; the seed is drawn silently and rides in the address) or on the saved `market-town`, with replay. A generated town is real buildings from the template library on streets, kerbs and fields, with street furniture, woods and a river ([buildings lane](buildings-lane.md), [ground lane](ground-lane.md), [scale lane](../done/city-maps-scale/README.md), [sim rules](sim-lane.md) and [startup](startup-lane.md): each file's Status says what it left open). The full gate (`bun run check`, `bun run verify`) has **not** run since the lanes merged; it runs once, at closeout.

**The generator as it stands** (`layout-11`, presets `layout-presets-10`): a road runs from the bottom edge to the top through the main junction on every map, and a side-to-side road on about half ([M22](procedural-maps.md#closed-decisions)); a settlement is blocks cut along its roads and grown outward unevenly, one use to a block ([M23](procedural-maps.md#closed-decisions)). A city has arms along secondary roads, each a plain T off its main road with rows of blocks along both sides; a street meets a road square or stops short, a junction has four arms at most, and towns have one to three parks and the odd unbuilt block ([M27](procedural-maps.md#closed-decisions)). 900 of 900 swept maps generate, and the largest Metro Large map holds 49,459 authored bodies of the 60,000 allowed. The seam and measurements are in the [C52](slices/C52-procedural-generator.md#outcome) and [C53](slices/C53-parcels-and-buildings.md#outcome) outcomes. Open on the look, by an unprimed review: a town still reads as a diagram (stair-step outlines, hard seams where the building kind changes, a large town as one slanted slab), streets either side of a main road meet it about 30 m apart, a refused street leaves a dead end a block short of its road, and a park is a wood on a block.

**Open-country reference review (2026-10-02):** the furnishing pass is present and the previously empty Mixed Small seed now has zero unbroken circles among its sampled open-ground positions and starting-column units. This is sampled physical evidence, not final opening-fog/landscape acceptance. The [pickup review](assets/open-country-review/README.md) records scope; [the new user references](assets/reference/broken-arrow/SOURCES.md#open-country-reference-target) and C54 require actual combined starting fog, a tactical field view and an overview.

**Current pickup, in order:**

1. **Two unfinished branches, pushed, each one WIP commit that was never compiled clean or tested as a whole** (their agents were cut off mid-work). Read the diff, finish or restart, and merge only when its own tests pass.
   - `city-maps/long-moves`: move admission (`crates/sim/src/movement/certify.rs`) drives only the two ends of each leg and carries the unit along the planner's route between them (`carry`, one new rules row `navigation.move_rehearsal_m`), so the cost of admitting a move does not grow with distance. Decided without the owner's answer, and reversible. Today a cross-map order on a generated map runs out of the rehearsal's allowance and is refused or part-placed. Lines marked `// PROBE` and `crates/sim/examples/long_moves_probe.rs` are scratch to delete. Wanted proofs: cross-map moves for a vehicle, a squad and a mixed group are placed and then arrive in a real battle; no-route and blocked-arrival destinations are still refused; a 5 km admission costs a small multiple of a 500 m one; digests for already-admitted moves unchanged (`village_report -- --quick --compare main`).
   - `city-maps/kerbside`: a road journey (`crates/sim/src/navigation/journey.rs`) treats its lane as the free width of the road and moves over past a body beside it, so parked cars can stand at the kerb (`street_props.lane_margin_m` from 3.7 toward 0.9; [C46 outcome](slices/C46-street-placement.md#outcome)). `crates/mapgen/tests/zz_probe.rs` is scratch to delete. When the furniture moves, bump the generator and preset revisions, re-bless parity (7 layouts succeed and 6 refuse; 3 encounters placed and 3 refused) and re-save Market Town (`fixtures/README.md`).
2. **One web test is red on main:** `tests/terrainSurface.test.ts`, "a country road is drawn as a street between a town's blocks, and as itself out in the plain". Cause found, not fixed: on Mixed Medium seed 2 a lone farmstead stands 27 m from a country road at (4144, 5899), the field plots on both sides of the road fall within `field_rules.yard_m` of it and become yard ground, and the biome's `town` rule then draws that stretch of road as a street. The test is right. The fix belongs in how yard ground is chosen (`packages/battle-renderer/src/terrain/plots.ts`, `pickKind`): a building's yard should not reach across a road, without turning a town's unbuilt blocks and parks into crop fields.
3. **Open items the lanes left for this one** (their Status sections have the detail): some parcels place buildings that overlap; a town fetches more than the 50 MiB kit budget; street trees stand a metre from facades; the village's roads should be named `country_road`; `movement`'s gap check and `village`'s scripted fight were reported failing; two `village` panel-layout checks; the format check fails on `web/src/lab.css` and `web/tests/terrainSurface.test.ts`.
4. **Closeout:** [C54 seed gate](slices/C54-generation-gate.md), C50 durability balance, C51, C61 listings, gates G0 and GG from the lanes' verdicts; then the full gate once, the whole-spec review, one consolidated `choices.md`, and close-spec.

A fourth town-look pass (stair-step outlines, seams between building kinds, streets meeting a main road 20 to 45 m apart) is worth doing only if the owner still finds towns diagram-like in play.

**Where to work:** start from `main`, which holds everything finished. Do each piece on a short branch off `main` and push it to `main` when its own checks pass. The two WIP branches are the only other branches that matter; rebase or merge `main` into one before finishing it. No other branch on the remote is needed.

**Working rules that held all through:** narrow checks are the proof for a commit, merge and push to `main`, and green passes are pushed as they land; every scene and render goes through the GPU lock (the root README's Checks section); pull the Git LFS runtime assets before running scenes after a merge.

Open from the planner: the attacker's squads start on foot 3 to 5 km from the objective (there is no transport); historically pending generated infantry journeys still require a complete arrival proof ([scale limits](../done/city-maps-scale/README.md#limits-that-remain-with-the-parent)); no battle has yet shown a fight across an 1,800 m approach. Open elsewhere: the camera lab still compiles its own map; the village opening is silent since the range tuning (the sides start beyond gun range).

You are implementing `city-maps`. Use [implement-spec](../../.agents/skills/implement-spec/SKILL.md), use the current extents in [M04](procedural-maps.md#closed-decisions) and [startup/transit policy](scale-direction.md), and read the [S0 verdict](spikes/S0.md) before allocating full-size arms. Earlier extent measurements remain evidence at their original sizes, not the current preset definitions.

1. Work the pickup above in order, delegating independent lanes to worktrees. There is one lane now: the sim rules lane has closed, and its ownership table no longer applies.
2. One owner builds the whole spec, look included. [The systems handoff](systems-handoff.md) lists, lane by lane, what each systems slice left for its look. Prototype art never stands in for accepted art. Art stays programmatic (Blender scripts and code in the repo).
5. Test in proportion: every lane and every merge runs only the narrow checks for what changed. The full gate (`check` and `verify`) runs once, when this spec's implementation is finished. Everything before that, including finishing a lane, is proved by the tests, scenes, measurements and pictures that change can move. No frozen-record tests; state a rule in a small behaviour test.
3. Full-size arms rejected by [S0](spikes/S0.md) stay rejected until their owner changes. Measure cost in instructions retired, on generated maps, with `city_report` ([S1](spikes/S1.md)).
4. Before ending each committed pass, update this handoff, the owning slice's Outcome and `choices.md`. Keep one next pickup.

**Evidence:** [S0 baseline](assets/scale-baseline/README.md), [SA1](spikes/SA1.md), [SA4](spikes/SA4.md), [navigation](assets/navigation-proof/README.md), [ground transport](spikes/SA3-transport.md), [ground learning](assets/ground-learning-work/README.md), [polygon surfaces](assets/surface-contract/README.md), [physical compiler](assets/map-compiler/README.md). Original rejected arms and numeric discrepancies retain their separate identities. Empty boot or uniform sampling alone is not active-battle proof.

**Known retained rule:** the sim does not limit gun elevation; do not fix it in this spec.

### TODO

A slice marked "physical" has its systems half done; its look is in the visual pass below ([what each left open](systems-handoff.md)).

**Done**
- [x] Scale: [S0](spikes/S0.md) · SA1 terrain/export · SA4 fog delivery · SA3 sparse ground (storage) · [S7 composition](spikes/S7.md) · [S1 first pass](spikes/S1.md) · [SA2 route planning and grid updates](slices/SA2-counted-route-integration.md)
- [x] Map and sim: C00 physical templates · C01 buildings aggregate · C02 fog cell · C03 surfaces · C72 one forest rule · C64 road kinds (physical) · C65 round centerlines (physical) · C63 surface index · [C69 rivers](slices/C69-rivers-contract.md) (physical)
- [x] Generation: C04 compiler (buildings, roads, forests, land regions) · [C52 layout generator](slices/C52-procedural-generator.md) with rivers, bridges and the M22/M23 town model · [C53 parcels and buildings](slices/C53-parcels-and-buildings.md) over a labelled prototype template catalogue
- [x] Sim rules ([lane record](sim-lane.md)): C40 seats → C41 facade eyes · C42 low-rise lifecycle → C43 tall gutted · SA6 movement gaps · SA5 sight cost · C44 street bodies · C77 forest bodies · C80 effective-height validator · C86 tree lines (all physical)
- [x] Delivery: [C09 map resolution](slices/C09-fetched-maps.md) · [C60 catalogue data](slices/C60-map-catalogue-data.md) with compact saved maps · [C59 encounter planner](slices/C59-encounter-planner.md) · [C55 runtime generation](slices/C55-runtime-generation.md) from the menu · [C58 saved generated map](slices/C58-offline-encounter.md) (physical)
- [x] Roads: square ends, sound joints, bends at sharp forks, width changes only at a crossing
- [x] Town look, three passes: cities with arms on secondary roads, large towns on a second road, main settlements off the middle, streets square to every road, four-arm junctions at most, parks ([M27](procedural-maps.md#closed-decisions), [C52](slices/C52-procedural-generator.md#outcome), [C53](slices/C53-parcels-and-buildings.md#outcome))
- [x] [C46 street furniture](slices/C46-street-placement.md) in generated towns (as stand-in boxes) · open country furnished so no sight circle is unbroken ([M24, M25](procedural-maps.md#closed-decisions))
- [x] [Startup lane](startup-lane.md): C33 one prepared world · C20 fog at scale · the camera lab through the catalogue · a native-against-Wasm pair with combat. Deploy-to-playable is 5 to 6 s on the largest map against a budget of 30 s (the owner's, 2026-10-01), so startup time is closed
- [x] [Scale lane](../done/city-maps-scale/README.md): exact cost, delivery and resource contracts admitted on the identified full reference; [decisions and measured scope](../done/city-maps-scale/evidence.md)
- [x] Lab: [C57 camera clearance](slices/C57-camera-clearance.md) · the generated-map lab route

**In flight:** the two WIP branches in the pickup above.

**Next, systems**
- [ ] Kerbside parking: road journeys refuse a lane with a body beside it, so street furniture stands on the verge and terraced streets have no cover ([C46 outcome](slices/C46-street-placement.md#outcome)); when navigation tolerates it, `street_props.lane_margin_m` goes back to about 0.9
- [ ] Long moves under move certification: a rehearsal's allowance runs out on a cross-map order, so long group moves on generated maps are refused or part-placed ([move validity](../done/move-validity/README.md)). Decided without the owner's answer, and reversible: the planner's route proves the journey and the rehearsal demonstrates only the arrival, so admission cost does not grow with distance
- [ ] Town look, what is left: stair-step outlines, hard seams between building kinds, streets meeting a main road 20 to 45 m apart on either side, dead ends where a street was refused; do it if towns still read as diagrams in play
- [ ] Scale at full extent: browser startup and memory, rendered surroundings, C20 fog at scale (C06/C07 are admitted on the [scale reference](../done/city-maps-scale/evidence.md); current-layout rendering integration and C22/C23 remain parent/buildings work)
- [ ] [C54 integrated seed gate](slices/C54-generation-gate.md), after the lanes
- [ ] C05 measuring tools (`city_report` is its first piece) · C10 third-party sources · C13/C32 template source and library schema · C21 material transport
- [ ] Completion: C50 durability balance → C51 playable generated encounter (requires C54 and C87)

**The visual pass**
- [x] **[Buildings lane](buildings-lane.md):** C10 sources → C11 kit → C12 materials → C13 placement bake → C32 template library · C22 placement chunks → C23 far tier · C16–C19 categories · C37 village houses · C21 → C24 cutout → C25 glass → C15 → C26 interiors · C14 → C27 ruin and gutted art. Its status lives in that file. **Done; open look questions are in its Status.**
- [x] **[Ground lane](ground-lane.md):** C62 rig · C66 → C67 → C68 country roads (with the looks of C64 and C65) · C28 → C29 · C30 town streets · C70 → C71 banks · C73–C76 trees · C78 → C79 forest bodies · C86 drawn tree lines · C81–C85 grass and fields · C45 street models · C31 → C87 composition. Its status lives in that file. **Done; open look questions are in its Status.**
- [ ] Map lane, after both: C61 listings; gates G0 and GG closed from the lanes' verdicts

`*` = conditional on measured full-extent results. It closes without code if its spike or measuring-tool numbers are under budget; resolution alone cannot exempt C07.

## Slice graph

```
Phase 0       S0 allocation inventory ─► S7 plan layouts ─► S1 active cost
              S2 export ─► S5 joins · S6 parity; S3 frame · S4 fog · SG1 cost
              failures ─► early G0 reslicing ─► focused representation proofs
              all passing evidence ─► G0 ─► production lanes

Geometry      C00 physical descriptors ─► C01 aggregates ─► C02/C03
              structural ground contracts ─► C04 compiler ─► C52 layout ─► C53 parcels
              C13 physical catalogue ───────────────────────────────────────► C53
Assets        C10 ─► C11/C12 ─► C13 source export ─► C32 library/resolver ─► C14 damage
              C16–C19 source recipes incrementally complete class/family coverage
Renderer      C32 ─► C22 shared chunks ─► C23 far tier
              C20 fog · C21 material transport ─► C24–C27 facade/terminal art
              C28–C31 street/urban composition; C60–C87 ground/catalogue lane
Rules         C40 seats ─► C41 eyes; C42 collapse ─► C43 gutted
Delivery      C09 resolver ─► C33 public preparation
              C53 + selected art/rules/scale + C57 ─► C58 full Small authored encounter
              C58 ─► C59 recipe planner ─► C55 runtime
Cutover       C37 shared house appearance (surroundings for the old test maps: cut)
Closeout      C50 balance + complete appearance/ground/runtime/cutover ─► C54 ─► C51
```

**Critical paths:** scale inventory → focused failure proofs → complete G0; physical descriptors/compiler/layout/parcels; source export/library/selected art → full Small fixed-seed play → recipe planner/runtime. Complete class/family art and final ground composition gate release rather than first useful play. Slice headers own exact prerequisites.

## Pipeline

```
offline asset sources → explicit Blender source export → deterministic asset library packing/fit
                                              ▼ geometry manifest (contract-owned schema)
type + size + seed + pinned versions/hash → crates/mapgen (same CLI / wasm library)
      → layout + roads + forest/optional river + usable plains → legal template parcels
      → MapPlan → validate/lower → MapDefinition + provenance + diagnostics
      → fixed-seed catalogue files OR runtime worker result
      → one map resolver → authored encounter OR C59 recipe → C33 prepared scenario/public exports → sim/observation
      → renderer resolves templates through one static-chunk owner, including far/damage tiers
```

## Invariants: one owner per concept

These are how the finished code should read, as if designed today, not bolted on. A slice that breaks one is wrong even if its tests pass.

| Concept | Single owner | Never |
|---|---|---|
| A placed building (parts, height, floors, floor heights, immutable template reference) | `MapDefinition.buildings` (C01), materialized from a legal template by C04 | Building facts copied onto parts; art inventing dimensions |
| Template geometry and eligibility | C00 schema in `contract`; C13 physical catalogue export, C32 fit validation | Rust mapgen depending on the scene-assets implementation; stretched footprints/floors |
| Type, size and composition tuning | Versioned preset definitions in `mapgen` (C52), fixed extents from `procedural-maps.md` | A coverage slider or client-owned generation rules |
| Building integrity, collapse and gutting | `sim::structures`, keyed by the building's owner prop | Damage stored on parts |
| Facade bays versus seats/capacity/eyes | C00 physical bay geometry; `sim::garrison` owns band/cap/admission/eyes (C40/C41) | Independent facade phases in art and simulation |
| Floor heights | Building data (C01), written by C04 from C53's plan | The bake or renderer recomputing them |
| "Is this ground a road" | `world` surface index over `MapDefinition.surfaces` (C03) | The renderer re-deriving the rule; terrain reads the exported surface |
| Urban and plain regions | `MapDefinition.land_regions` (C04), generated from plot/field polygons | C31 guessing the region from colour or reading `MapPlan` directly |
| Distance to roads, forests and water in the renderer | `terrain/surfaceField.ts` (C63), built from the export with the sim's distance function; C28 pavement and every ground slice read it | A second bake; per-fragment loops over shapes; a class or coverage mask |
| Surface-kind speeds | One `surfaces.<kind>.speed_factor` table (C64) | A rural-only table beside C03's kinds |
| Fog cell size | `MapDefinition.fog_cell_m` (C02) | `sensors.fog_cell_m` (deleted) |
| Map generation and lowering | `MapPlan` and one compiler in `crates/mapgen` (C04, C52, C53) | Sim or renderer reading generator state; a second map format |
| Urban/plain mix and approximate coverage fairness | C52 composition; C54 checks compiled maps | A colour-only plain, decorative-only towns, mirrored shapes as the fairness oracle |
| Building appearance | C32 template appearance library over C13 offline exports, resolved identically for saved/runtime maps | Per-building GLBs, per-map Blender dependency, catalog rows per building, runtime Blender, a TS graph evaluator |
| Safe camera pose | Existing camera intent owner plus one pure clearance resolver (C57), wired to public/side-known geometry | Separate rigs; hidden live destruction used as a camera obstacle |
| Static instanced drawing (trees, hedgerows, forest dressing, kit modules, far-tier tiles, corpses) | **One static-chunk owner, promoted from the scenery layer's existing chunk path** (`frame/sceneryLayer.ts`, `scenery/lod.ts`) in C22; corpse chunks move onto it; C23 and C79 only build instances for it | A second chunk path or model layer; per-instance CPU work per frame |
| Material coverage (opaque, cutout, blended, interior) | `scene-assets` `Material` (C21) | Alpha channels overloaded (albedo alpha is wear; ORM alpha is tint mask) |
| Street prop behaviour | Catalog body rows (C44) | A rule keyed on a kind's name |
| Map acquisition and battle initialization | One resolver (C09): saved catalogue map or generated request → compiled `MapDefinition`; C55 adds the runtime source | A second loader/battle path, generator state in the sim, maps bundled in JS |
| Encounter placement | Existing scenario builder accepts authored fixed-seed encounters (C58) and one recipe planner result (C59) | Village coordinates reused on arbitrary seeds; another navigation/battle implementation |
| Map catalogue (category, status, labels) | `fixtures/maps/<id>/meta.json` via `web/src/maps/catalogue.ts` (C60, C61) | A hand-kept list of maps; a committed generated index. Routes stay in `apps/battle-lab/src/fixtures.json`, each naming its map |
| Water | `MapDefinition.rivers` centerlines (C69), read through C03's index and C63's field | Water rects; a second shoreline rule in the renderer |
| Curved roads and rivers | Splines densified to ≤2 m points by the contract's loader (C65); the plot cutter reads the control runs | Grid-cell shading, or a renderer-only smoothing the sim doesn't share |
| Forest density, canopy and floor bodies | One `forests.rule` row (C72, C77) | Per-species or per-forest sizes; hand-placed forest bodies |
| Trees (forest, street, hedgerow) | One tree generator in `trees.py` (C73, C74) | A second tree technique for street trees |
| Grass and crop height | The effective-height validator (C80), including every multiplier | A cap on source assets only |
| Provenance | Third-party art's source and licence recorded beside the asset; `SOURCES.json` for saved maps and equivalent runtime/replay identity (generator/preset/physical-catalogue versions, canonical config, lossless seed, map hash); appearance identity recorded separately | Seed-only replay identity; art hashes inside simulation identity |
| Simulation world and map queries | The preparation worker builds one world for the encounter planner and the battle (C33); the page builds its own plain world from the same map for picking, height and camera clearance, with the simulation's own query code | A second query implementation that must be kept equal to the first; a world rebuilt between planning and the battle |
| Replay compatibility | Same-build compiled scenario/rules plus engine build and digest checks (C55) | Cross-build compatibility or historical art retention inferred from immutable map hashes |

**Short-lived seams:** the village house appearance path survives only until C37 moves every existing house consumer to the template library; C37 deletes its bundles/loader/render branch in the same cutover. C13's labelled massing source serves developer checkpoints only and is removed at C54 when selectable release coverage is complete. No other compatibility seam is planned.

## Standing gates (every slice inherits them)

- **G, general:**
  - red/green tests with [write-tests](../../.agents/skills/write-tests/SKILL.md);
  - replay and digest parity unless the slice names the change (village changes are named in `decisions.md`);
  - presentation reads only the observation plus public static geometry;
  - hard cutovers with no compat shims (Compat);
  - a `frame-cost.md` row for anything that touches the frame;
  - narrow runners while iterating, and `bun run check` / `bun run verify` once at pass closeout.
- **V, visual:**
  - freeze the camera, light, seed and every variable except the slice's one;
  - judge its crop or mask with [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) against the named target;
  - run an **unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) last, before accepting any shot**, including "could any dark region read as shadow, or shadow as fog?";
  - open the evidence with [preview-shots](../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint: allow about 5 minutes; if the user is silent, decide on the evidence, record it in the slice's section of `choices.md`, close the shots and proceed.
- **P, performance:**
  - matched fixtures and commands; sim cost in instructions retired;
  - performance-only changes keep digests identical;
  - benchmarks run serially on this Mac, never alongside Blender jobs.
- **R, rules:**
  - [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md) first;
  - a test per ruled-out moment and a movement-scenario GIF;
  - named digest changes;
  - `village_report -- --quick --compare main`;
  - balance tuning waits for C50.

## Budgets (full-extent spikes ratify or amend them at G0)

The 30 FPS floor, fixed sizes and [startup requirement](scale-direction.md#startup-and-loading) are requirements. Other numbers below are inherited targets, not proof that the enlarged maps fit. S0–S4 must specify peak sim/wasm/browser/GPU memory and measure cold startup/runtime generation, as well as active-battle cost. G0 reslices architectural failures before production.

| Axis | Budget | Tool |
|---|---|---|
| Startup | [Complete first usable battle view budget, with loading-screen allowance](scale-direction.md#startup-and-loading) | S3, C55, worker/browser load probe |
| Vehicle transit | [Large-map edge-to-centre travel target](scale-direction.md#vehicle-transit-target) | Timed representative road-connected movement scenario |
| Frame | ≥30 FPS average over the 300 s `city-contact` benchmark at the default camera, 1920×1080; worst-window GPU p95 ≤25 ms; static city ≤15 ms GPU at any camera | `/benchmark?preset=city-contact`, `frame-cost.md` |
| Ground lane | At most +3 ms GPU p50 on the village benchmark for the whole lane (trees ≤1.5, dressing ≤1, grass ≤0.5); GG ratifies | village benchmark, C87 |
| GPU memory | City adds ≤400 MB buffers and ≤200 MB textures over the village row | benchmark columns |
| Publication | Provisional steady-state target ≤19.8 KB/tick; G0 separately ratifies full snapshot/resubscription bytes and delivery time at every selected extent | `city_report`, worker delivery probe |
| Sim | Measured early/late throughput meets the [closed scale contract](../done/city-maps-scale/README.md#contracts-that-must-survive); retain step distributions and instruction counts | `city_report` (instructions retired), full-map browser stress |
| Download | JS gzip within ±5%; provisional saved-map ≤25 MB and shared kit ≤50 MB; G0 adds reusable template library and runtime startup/download budgets | `vite build`, library output, worker load probe |
| Village | Digests unchanged unless named; `endurance_report` instructions ±1% on digest-neutral slices | digest and replay tests, `endurance_report` |

## Firewalls (out of scope)

- **Real-world map reproduction or real-data import.** NYC data research in `research.md` is historical, outside this build.
- **Additional regional city styles** (Eastern European and others; Q-B′). The six required building categories and missing family sources are in scope within the chosen regional families.
- **Any ODbL or OSM data** (Q5).
- **Spiderbench bytes; Google 3D Tiles or photogrammetry** (research.md).
- **The repo's two photo atlases** (`apartmentinterios.png`, `businesses.png`). Interiors use our own atlas (Q-E).
- **Emissive light or lamp glow.** Interiors are unlit.
- **Landmark look-alikes; real brands, logos, liveries or plates** (Q11).
- **Walkable interiors, room clearing, rooftops, floors above 3, underground** (Q3).
- **Gun elevation limits in the sim** (S-pitch; a later spec).
- **Civilians, traffic, night, weather, seasons.**
- **Surroundings for the existing maps (C56, C34–C36): cut.** The village, the labs and the benchmark fields are developer test arenas and stay the small maps they are; the player's maps are generated ones and saved generated ones.
- **Initial hills and ridges,** deferred to `../terrain-relief/`; local riverbeds/banks remain in scope.
- **Player polygon editing and arbitrary building generation at runtime.** Developer overlays may inspect plans; runtime chooses the pre-baked library.
- **Raising `TEXTURE_MAX_PX`,** or a kit texture inflating the shared texture array (L8).
- **Touching `../game`;** a bare `git lfs pull` in a worktree.

## Cut order if it runs long

1. C30 markings, then C29 curbs.
2. C26 interiors down to LOD0 only (the O-1 fallback).
3. C45/C46 down to cars, wrecks, Jersey barriers and street trees.
4. C27's burnt tier (gutted buildings drawn with the standing art).
5. Ground: C68 ruts, then C86 tree lines, then C79 dressing density (then all of C79), then C77/C78 forest bodies, then C85 field texture, then C74's species count.

**Never cut:** fixed selected dimensions, C52–C55 generation and runtime, C57 clearance, compound buildings, garrison bands, fog correctness, body-backed street props, reusable instanced templates and the 30 FPS floor. Missing category coverage is required; detail within its silhouette/material budgets can be tuned.

**Never cut, ground:** the map catalogue (C60, C61), the surface distance field (C63), round curves (C65, C71), the rivers cutover (C69), the one forest rule (C72), and the no-false-buff checks (crops ≤0.9 m, dressing only inside forests, sim-real tree lines).
