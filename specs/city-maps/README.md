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

## Next Agent Prompt

**Status (2026-09-30):** systems (non-3D) implementation is under way on main's line; art and visual gates wait for the specialist ([systems handoff](systems-handoff.md)). In production: sparse terrain, navigation and ground storage; incremental fog delivery; physical templates and aggregate buildings; per-map fog; shared surfaces; the building/identity compiler core; the saved-map resolver core; **C72, the one forest rule** over the shared ground shape; and **C64 road kinds** (the per-kind speed table; its painted look waits). Raw evidence left the spec: each `assets/` note keeps its conclusions, test oracles live in `fixtures/parity/`, and the raw files are at tag `city-maps-evidence-2026-09-30`.

**Current pickup:** **navigation at full extent ([SA2](slices/SA2-counted-route-integration.md)) is the blocker.** [S1](spikes/S1.md) measured a 6 km map with 63,000 trees: world and battle build and steady ticks fit, but one 900 m route plan costs 0.3–3 s and edge-to-edge plans do not finish. `city_report` (a sim example) is the measuring tool. In flight on its own branch: SA2 (`city-maps/sa2-navigation`). Landed: the [C63 surface index](slices/C63-surface-distance-field.md) and the [C52 layout generator](slices/C52-procedural-generator.md) (`mapgen generate`, `generate-map`, `inspect`; presets in `fixtures/map-presets.json`), whose maps already load and tick in the sim. Next: [C53 parcels and buildings](slices/C53-parcels-and-buildings.md) (blocks and streets inside the districts), [C69 rivers](slices/C69-rivers-contract.md), then a generated Small map playable in the lab. Also done on this line: C72, C64, C65 (physical halves), C04's ground pass, and unit speeds in km/h. Still pending: C09/C60 saved catalogue data and the cutover of existing maps onto the resolver; building-count scale proof for Metro (S7: tens of thousands of buildings); browser startup, memory and rendered-surroundings admission at full extent.

You are implementing `city-maps`. Use [implement-spec](../../.agents/skills/implement-spec/SKILL.md), use the current extents in [M04](procedural-maps.md#closed-decisions) and [startup/transit policy](scale-direction.md), and read the [S0 verdict](spikes/S0.md) before allocating full-size arms. Earlier extent measurements remain evidence at their original sizes, not the current preset definitions.

1. Complete SA3 native all-touched/churn/transport admission and mixed/fragmented GPU/pixel proofs. Evaluate SA2 navigation alternatives under [the current scale direction](scale-direction.md), including its accepted automatic road-travel and planning-delay behavior; the exact uniform-parent proof is optional, not the required next architecture. Continue C01 aggregates and indexed surface contracts; serialize resource benchmarks. Bridge search remains a failed-work contract even when sparse storage fits. Full-size arms rejected by S0 stay rejected until their owner changes.
2. Freeze each representation's code/config/inputs/results and parity oracle. Record individual physical/resource unlocks at G0; complete art G0/GG is still open. The systems scope does not allow art to masquerade as accepted.
3. Continue the nonvisual wavefront in systems-handoff.md: physical descriptors/aggregates, shared surfaces/fog, one map acquisition/public preparation, compiler/layout/parcels, seats/lifecycle, reservations, encounter/runtime/replay, transport and the full physical matrix. Finish every nonvisual contract, leaving precise visual seams for the specialist.
4. Before ending each committed pass, update this handoff, its owning slice/evidence and choices.md. Keep one next pickup and clear unresolved gates.

**Evidence:** [S0 baseline](assets/scale-baseline/README.md), [SA1](spikes/SA1.md), [SA4](spikes/SA4.md), [navigation](assets/navigation-proof/README.md), [ground transport](spikes/SA3-transport.md), [ground learning](assets/ground-learning-work/README.md), [polygon surfaces](assets/surface-contract/README.md), [physical compiler](assets/map-compiler/README.md). Original rejected arms and numeric discrepancies retain their separate identities. Empty boot or uniform sampling alone is not active-battle proof.

**Known retained rule:** the sim does not limit gun elevation; do not fix it in this spec.

### TODO

A slice marked "physical" has its systems half done; its look waits for the visual pass ([systems handoff](systems-handoff.md)).

**Done**
- [x] Scale: [S0](spikes/S0.md) · SA1 terrain/export · SA4 fog delivery · SA3 sparse ground (storage) · [S7 composition](spikes/S7.md) · [S1 first pass](spikes/S1.md)
- [x] Map and sim: C00 physical templates · C01 buildings aggregate · C02 fog cell · C03 surfaces · C72 one forest rule · C64 road kinds (physical) · C65 round centerlines (physical) · C63 surface index
- [x] Generation: C04 compiler (buildings, roads, forests) · [C52 layout generator](slices/C52-procedural-generator.md) · [C53 parcels and buildings](slices/C53-parcels-and-buildings.md) over a labelled prototype template catalogue

**In flight (one branch each)**
- [ ] [SA2 navigation at full extent](slices/SA2-counted-route-integration.md): `city-maps/sa2-navigation`
- [ ] [C69 rivers contract](slices/C69-rivers-contract.md): `city-maps/c69-rivers`

**Next, systems**
- [ ] Rivers and bridges in the layout generator (after C69 and C53); land regions in the compiler
- [ ] A generated Small map playable in the lab: C09/C60 saved catalogue data and the cutover of existing maps onto the resolver → [C33 public preparation](slices/C33-battle-preparation.md) → [C58 fixed-seed encounter](slices/C58-offline-encounter.md) → [C59 encounter planner](slices/C59-encounter-planner.md) → [C55 runtime generation](slices/C55-runtime-generation.md)
- [ ] Scale at full extent with buildings: S1 on generated towns (tens of thousands of buildings), browser startup and memory, rendered surroundings, C06 scale passes* · C07 publication* · C20 fog at scale · C22 placement chunks → C23 far tier
- [ ] [C57 camera clearance](slices/C57-camera-clearance.md)
- [ ] Rules: C40 floor-band seats → C41 facade eyes · C42 low-rise lifecycle → C43 tall buildings gutted · C44 street bodies · C46 street placement · C77 forest bodies · C80 effective-height validator · C86 tree lines
- [ ] Existing maps: [C56 reservations](slices/C56-fixture-surroundings.md) → C34 village · C35 labs · C36 benchmarks → [C54 integrated seed gate](slices/C54-generation-gate.md)
- [ ] C05 measuring tools (`city_report` is its first piece) · C10 provenance · C13/C32 template source and library schema · C21 material transport
- [ ] Completion: C50 durability balance → C51 playable generated encounter (requires C54 and C87)

**Waiting for the visual pass**
- [ ] Gates G0 and GG; spikes S2–S6 and SG1–SG6 where they judge art
- [ ] Assets: C11/C12 · C14 · C15 · C16 farmstead · C17 detached home · C18 tower · C19 industry · C37 shared houses
- [ ] Renderer look: C24 cutout → C25 glass → C26 interiors · C27 ruin and gutted art · C28 pavement → C29 curbs · C30 markings · C31 urban/plain composition · C45 street models
- [ ] Ground look: C66 road core → C67 shoulder → C68 ruts · C70 bank bands → C71 bank roundness · C73–C76 trees · C78 body models → C79 dressing · C81–C85 fields · C87 ground composition gate · C61 listings · C62 evidence rig
- [ ] The looks of C64 (per-kind road rows) and C65 (the bend)

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
Cutover       C37 shared house appearance; C56 reservation proof ─► C34 village · C35 labs · C36 benchmarks
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
| Urban/plain mix and approximate coverage fairness | C52 composition; C54 checks compiled maps; C56 preserves focused arenas while adding surroundings | A colour-only plain, decorative-only towns, mirrored shapes as the fairness oracle |
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
| Provenance | `reuse-manifest.json` (art); `SOURCES.json` for saved maps and equivalent runtime/replay identity (generator/preset/physical-catalogue versions, canonical config, lossless seed, map hash); appearance identity recorded separately | Seed-only replay identity; art hashes inside simulation identity |
| Simulation world and public queries | Worker preparation/Battle reuse (C33); main thread receives public exports and a bounded query index | Main-thread WorldView building a second terrain/nav world; hidden live destruction in camera/picking |
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
| Sim | Step p95 within the target S1 proposes (kill: no known local fix brings it ≤16 ms); no tick over 33 ms | `city_report` (instructions retired) |
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
- **Replacing focused lab/benchmark arenas with full generated battles.** C56 adds real surroundings outside declared arenas; it preserves their test stimuli and names affected whole-map identities and baselines.
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

**Never cut:** fixed selected dimensions, C52–C56 generation/runtime/all-map coverage, C57 clearance, compound buildings, garrison bands, fog correctness, body-backed street props, provenance, reusable instanced templates and the 30 FPS floor. Missing category coverage is required; detail within its silhouette/material budgets can be tuned.

**Never cut, ground:** the map catalogue (C60, C61), the surface distance field (C63), round curves (C65, C71), the rivers cutover (C69), the one forest rule (C72), and the no-false-buff checks (crops ≤0.9 m, dressing only inside forests, sim-real tree lines).
