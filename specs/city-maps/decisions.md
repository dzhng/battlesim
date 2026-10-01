# city-maps decisions

The [completed procedural-map walk](procedural-maps.md), dated 2026-09-30, is authoritative for current map scope, sizes, presets, categories, runtime templates, fairness and fixture surroundings. Retained combat decisions below remain active. Historical real-data/map-bake rows are explicitly superseded; the README and slice files own execution.

## Procedural map pivot (2026-09-29, reconciled with the completed walk)

| # | Decision | Why |
|---|---|---|
| P1 | Seeded procedural generation is the **primary and required** source of playable battle maps. NYC reproduction and its importer leave this spec. | The user wants control over map layout rather than geographic fidelity. Supersedes Q1, Q2, Q5's importer rationale, S-excerpt, S-import's crate name and S-order. |
| P2 | **Every catalogued map has real urban areas and usable surrounding plains**, including existing village/lab/benchmark maps. C56 preserves focused arenas and adds surroundings outside them. | Current scope and user rationale are M01/M15 in procedural-maps.md; the earlier fixture exemption is superseded. |
| P3 | One Rust mapgen/compiler serves offline fixed-seed artifacts and required runtime generation. Lots select legal pre-baked templates from one regional family; identity pins canonical request, generator/preset/physical-catalogue versions and map hash; appearance identity is separate. | M02/M08/M14 supersede offline-only and arbitrary per-part bake assumptions. C04/C13/C55 own the seams. |
| P4 | NYC/Paris/China are reusable art sources on invented geography; dedicated missing building families complete the accepted category set. | M07/M08 and C16–C19 keep regions coherent without claiming that apartment graphs supply every silhouette. |
| P5 | Generator, runtime delivery, all-map surroundings and camera clearance are required before completion; fixed-seed play is an earlier checkpoint. | M02/M13/M15 supersede the generator-last/cut-first order. The current ladder lives in README.md. |

The visible [Canvas of Kings clip](https://x.com/MightofMe/status/2104919371605041659) shows a polygonal district plot with handles, a grid and buildings filling the plot, then multiple settlements linked by curved roads through open land. That is an interaction and composition reference, not evidence of a particular subdivision algorithm. C52/C53 reproduce the outcome through our own geometry and C54 judges map playability. Reference imagery never ships as game art.

### Superseded historical rows

Q1/Q2 real NYC and its 1.6 km crop, Q5's importer rationale, Q6's dataset fields for generated maps, S-excerpt, S-floors' HEIGHT_ROOF formula, the per-map Q7/Q-D bake and optional S-order generator are superseded. Current dimensions, runtime library and all-map scope live in procedural-maps.md. Physical floor heights remain authoritative building data, the compiler remains Rust/contract-typed, and one resolver serves saved/runtime maps. Q3/Q4 combat rules, Q9 body-backed props, Q11 generic signs and compatible ground decisions remain active.

**Historical interview rows:** real-data rows below are retained as rationale, not executable instructions.

| # | Decision | Why |
|---|---|---|
| Q1 | **Both, real first.** The first map is real NYC data (footprints and roof heights from NYC Open Data). A **procedural layout generator** is a later slice in this spec. It emits the *same intermediate* the importer does (footprint polygons, heights and floors, archetype, roadbed and sidewalk polygons), so box decomposition, `MapDefinition`, bake and sim can't tell the two apart. | Real realism first, with invented maps later and no fork in the pipeline. |
| Q2 | A **mixed low/mid-rise district** (3–8-storey walk-ups and lofts), **1.6 × 1.6 km** like the village. C01 picks the crop from the Lower East Side, Greenpoint or Long Island City. | Known nav and fog costs. The NYC archetype fits most buildings, and garrisons stay meaningful. |
| Q3 | **Garrisons fight from floor bands.** Bands = min(floors, 3), and each band's slots have their own eye and muzzle height. **Capacity = one soldier per ~3 m window bay per band on outer facades only** (none on walls where footprint parts meet), with a **hard gameplay cap of 32**. **One eye per facade, at the highest band held on that facade** (eye count stays ≤4; on compound footprints, facades are the 4 directional groups of S-eyes). **Floor heights are authoritative building data**, materialized from legal template descriptors; art and slots fit those same floors. Garrison shelter keeps today's flat `buildings.cover_strength`. Ruled out: floors above 3, rooftops, walkable storeys. | War-film test: an MG on floor 3 fires over the 2-storey shop opposite, and a 50 m block holds more than a shed. The cover-tier sub-rule was dropped once Q4 made gutted buildings ungarrisonable (no garrisoned building changes tier). |
| Q4 | **Height classes.** A building of **≤6 floors collapses** at 0 hp to a ruin whose **height scales with the building** (25% of height, from 2 to 6 m). A building of **>6 floors becomes gutted**: it keeps its height, still blocks and occludes, drops to medium cover tier, bakes as the burnt tier, **is not garrisonable**, and can't be destroyed further. Occupants of a gutted building take the collapse path (survival roll, escape suppressed). **hp scales with footprint area × banded floors** (today's flat 400 is wrong). The coefficients (25%, hp per m²) are left to the balance pass. Gun elevation: answered by S-pitch below (the sim doesn't limit it; out of scope). | A tower doesn't fold like a house. The user chose "not garrisonable" to keep it simple for now; ruin fighting can be a later rule. |
| Q5 | **Deferred, and the firewall stays.** No ODbL/OSM data in this spec. The importer's input seam is source-agnostic (the same seam as Q1), so a later city spec adds an OSM reader and accepts ODbL then. | Other cities are future work. |
| Q6 | **`fixtures/maps/<id>/SOURCES.json` per map, test-checked.** It records the dataset URL, version and date, licence, attribution, modifications and the sha256 of the pinned download, checked against the data allow-list (public domain, NYC Open Data terms). Raw downloads stay out of git. Generated maps record `project-owned` plus the generator seed. | It mirrors how `reuse-manifest.json` guards art. |
| Q7 | **Superseded by M14:** reusable templates are baked offline into shared module placements; generated maps select/transform them without per-map Blender. | C13/C53 own the current contract; L1's shared-instancing lesson remains active. |
| Q8 | S1/S4/G0 decide per-map fog and publication at the fixed extents. The original village stays 8 m at the schema-only cutover; C56 names later geometry/identity changes. | Full 8 m publication also needs measurement, as recorded in the completed map. |
| Q9 | **Every street prop is a body** (L5: their lamp, bench, bollard, bins, hydrant, utility box, scooter and planter; our cars, wrecks, Jersey barriers, bus shelters, street trees and scaffolds). **C44 runs tweak-mechanics per kind** and writes each row by the war-film test (for example: a car stops rounds and a tank shoves it; a bollard stops cars, not tanks; a glass shelter blocks movement but not rounds or sight). Property rows only, no named cases. | Props-are-bodies (first-principles physics). |
| Q10 | Moot. Spiderbench is superseded by the MIT ProceduralBuildingsThreeJS source. | |
| Q11 | **Generic only.** No landmark look-alikes and no real brand, logo, livery or plate. Signs come from a **project-owned list of invented generic names** fed into the graphs' sign-text inputs at bake. | Trademark risk. |
| Compat | **Neither.** Hard cutovers: C02 cuts the village fixture over in the same commit, with no compat shims or cutover scaffolding. Village digests change only where a slice names it as a decision; otherwise digest and replay parity is the gate. | The user's default. |

## Scope notes from the interview

- **Future, out of scope:** additional regional styles and ODbL/real-world maps. The current invented-map and required six-category scope is defined in procedural-maps.md; missing home/tower/industrial families are in this build.
- **Interiors:** interior *mapping* is in as a visual (every floor, unlit, our own atlas). Room-by-room clearing and walkable interiors stay out.

## Synthesis decisions (2026-09-28)

Four drafts (fewest slices, risk first, performance, and seam quality from Codex) were merged into the ladder in `README.md`. First, what the user decided:

| # | Decision | Why |
|---|---|---|
| S-eyes | **A compound footprint's outer walls fall into 4 directional facade groups** (N/E/S/W relative to the building frame). Each group's eye sits at a **real occupied seat on the highest held band**, never averaged into a wall, so there are ≤4 eyes per garrison. | Keeps the eye perf bound and never puts an eye inside a wall. User. |
| S-pitch | **The sim does not enforce gun elevation limits** (`weapons.rs:540, :1278` ignore `PITCH_LIMITS`, which is presentation-only, `scene-assets/src/articulation.ts:49`). **Out of scope: a later spec.** A tank beside a building can hit a floor-3 window point-blank: a known wrong moment, recorded, not fixed here. | User. |
| S-excerpt | **A small clipped excerpt of NYC Open Data (a few blocks of footprints and surfaces) is committed as importer test data**, attributed in its SOURCES.json. The full-crop download stays out of git. | A golden test must run in CI. This refines Q6. User. |
| S-floors | **Floors are derived from height only:** `floors = max(1, 1 + round((HEIGHT_ROOF − 4.2) / 3.2))`, with ground floor 4.2 m and upper floors `(HEIGHT_ROOF − 4.2) / (floors − 1)`. **No MapPLUTO.** | One dataset. The user accepted that tall-ceiling lofts and low-ceiling tenements come out wrong. User. |

Then the calls made in synthesis. The alternative is recorded for each.

| # | Call | Alternative that lost, and why |
|---|---|---|
| S-agg | **Buildings are a first-class aggregate:** `MapDefinition.buildings`, where each building has its parts, height, floors, floor heights, archetype and seed. At world load, parts become props **before** all other props, following the forest precedent (`world/mod.rs:134-160`). The village's 3 houses become one-part buildings and keep PropIds 0–2. **One integrity owner per building.** | *A `PropDefinition.building` id* (drafts A and D): building facts get duplicated per part or pushed into a side table. *One PropId owning several parts* (Codex): the cleanest ownership, but every box consumer (rays, nav, exports, knowledge) cuts over at once. |
| S-spikes | **S0–S7 are throwaway spikes read at G0**, with full-extent allocation/startup first, plus separate plan composition and active battle cost, template export, frame, fog, joins and native/wasm template parity. | Occupied-building counts and art alone miss empty-land allocation and runtime identity risks. |
| S-import | **Superseded crate name:** one `crates/mapgen` library/CLI depends on contract; C04 owns lowering and C55 exposes the same implementation through wasm. | Source-independent ownership survives; a real-data importer does not. |
| S-surf | **Surfaces are exact shapes (`Polygon` or `Stroke`) behind a bucket index**, replacing `roads` on every map. The village's polylines become `Stroke`s with today's exact distance rule, so the village digest doesn't move **at C03**. (Rounding those roads into splines later moves it, named in C65.) | *Lowering the village's roads to polygons* (performance draft and Codex): it moves the village's road-edge answers. |
| S-chunks | **Placements draw through one static-chunk owner, promoted from the scenery layer's existing chunk path** (`frame/sceneryLayer.ts`, `scenery/lod.ts`; revised in the ground synthesis). Corpse chunks (`modelDetail.ts:66-110`) move onto it in C22. There is no second model layer. | *Generalising the corpse chunks* (the first synthesis): it would have left the scenery layer as a second owner. *A separate placement layer:* two model owners. |
| S-fetch | **Saved maps are fetched by id, never imported into the JS bundle.** C55 runtime sources resolve through the same owner without a persistent catalogue folder. The village's map moves out of `village.json` in the same cutover. | A multi-MB city map in the bundle. |
| S-fogpub | **C07 is required whenever full-extent delivery fails its measured budget, at any fog cell size.** G0 specifies steady-state and snapshot/resubscription budgets separately. | Even 8 m full fog is large at the selected extents; the old finer-than-8m trigger was insufficient. |
| S-order | **Superseded:** generation/runtime/all-map gates cannot be cut. C58 provides fixed-seed play before C55 runtime integration and C54/C51 closeout. | A required primary generator cannot remain the optional last slice. |

## Ground lane

The open-country ground decisions (roads, rivers, forests, grass, farms, map catalogue) are in [`ground-look.md`](ground-look.md), Q-G1 … Q-G19. They are givens for SG1–SG6, GG and C60–C87.

## Ground synthesis (2026-09-28)

Four drafts of the ground lane were merged into SG1–SG6, GG and C60–C87: fewest slices (9), risk first (19), one visual variable per slice (29), and seam quality from Codex (31). The walk's decisions are in `ground-look.md`. These are the calls made in synthesis and refactor-clean, each with the alternative that lost.

**All four drafts agreed on these; each corrects the first cut:**
- **Grass is one indirect draw per tier over every kind** (`grassPass.ts:21`). The real limits are 8-blade padding, the 16 kinds and 16 growth rows, and one appearance per growth row. The last is what blocks within-field variation, so C82 turns a growth row into a weighted mix. L-G7's "cap species at 4" is dropped.
- **Forest dressing rides the scenery layer's existing chunks, not C22.**
- **Round curves move the village digest** (the corners at (420,420) and (1150,420) move road cells and nearby trunks). **The plot cutter reads control runs**, not 2 m segments.
- **The one forest rule is a named change on 5 fixtures** (village, endurance, sensors lab, geometry lab, `benchWorld.ts`).
- **Saplings are dressing only, and fallen trunks exist only as log bodies.** A sapling outside a sim forest, or drawn fallen trunks that stop nothing, would lie about sight or cover.
- **Distances are signed, never a class mask.**
- **Tree lines are `Stroke`-shaped forests.** Plots cut along them, crowns clamp to the canopy radius, and the village gets none by default.
- **The catalogue lists maps; routes stay in `apps/battle-lab/src/fixtures.json`**, since there are 25 routes over 12 maps.

**Calls made where the drafts split:**

| # | Call | Alternative that lost, and why |
|---|---|---|
| G-S1 | About 28 slices, with visual splits only where the verdicts really are separate. | *9 slices* (fewest): battle-look's dead-end list records a fewest-slices ladder whose merged slices had decision budgets too large for one pass. *29–31* (visual lens and Codex): some splits judged variables that only read together (a shoulder with the grass thinning across it). |
| G-S2 | Six throwaway spikes and gate **GG**. The forest and field sub-lanes gate on GG, not G0. SG1's tree budget feeds G0. | *Wait for G0* (Codex): it would idle lanes that touch no city spike. |
| G-S3 | **SG5 decides the distance structure**: a signed-distance bake, or an exact segment-bucket index. | *Commit to a bake now* (risk-first and visual drafts), or *to an exact index* (Codex): the error at joins and where width changes needs measuring first. |
| G-S4 | **Forest bodies are generated by the one forest rule's seed, after the trunks**, so no trunk moves. | *Hand-authored in `map.props`* (Codex): per-map work on every future map, and a second placement owner. |
| G-S5 | **The catalogue schema is owned in TypeScript** (`web/src/maps/catalogue.ts`; only JS reads `meta.json`), with **no committed generated index**. | *A Rust schema with a generated TS view* (risk-first and visual drafts): the sim never reads `meta.json`. *A generated index:* it brings back the merge conflicts Q-G9 was chosen to avoid. |
| G-S6 | **One `surfaces.<kind>.speed_factor` table**, with country road at 1.0. | *No speed per kind* (visual draft): Q-G4 kept speed per kind in the sim. *A rural-only table* (first cut): a second owner beside C03's kinds. |
| G-S7 | **C62 evidence rig** (frozen stations, shader-exact class masks), **C87 composition gate**, **the effective-height validator** (C80) and **a +3 ms ground-lane budget** are added. | From the visual, Codex and fewest-slices drafts respectively; no draft opposed them. |

**Refactor-clean:**

| # | Change | Why |
|---|---|---|
| G-R1 | **One static-chunk owner promoted from the scenery layer's chunk path** (C22); corpses move onto it. | Scenery chunks and corpse chunks were already two owners, and C22 would have made a third. |
| G-R2 | **C63's surface distance field is the one owner; C28 consumes it** and is reordered after it. | C28 and C63 would otherwise have been two bakes. |
| G-R3 | **C09 moves every map (village, 11 labs, endurance) into `fixtures/maps/<id>/map.json` at once; C60 adds only metadata.** | Otherwise two map locations and two loaders would live between C09 and C60. |
| G-R4 | **Street trees use the one tree generator** (C45 depends on C74). | Otherwise a second tree technique. |
| G-R5 | **C51 depends on required city slices plus C87**, not on optional ground slices. | Optional ground work mustn't hold the encounter. |

## Plan synthesis (2026-09-30)

Three independent read-only drafts used fewest-slices, risk-first and seam-quality lenses. All caught the full-extent/shrink, per-map Blender, fixture-exemption, publication and camera gaps. The canonical ladder takes their shared ownership conclusions and uses these cuts:

| ID | Plan call | Alternative and why |
|---|---|---|
| M-S1 | S0 isolates allocation/startup feasibility before S1 active battle cost. | A merged sim spike could spend time tuning fights before discovering the map cannot boot. Numeric architectures/budgets remain G0 outputs. |
| M-S2 | C00 defines physical descriptors; C13 exports reusable sources before C53; C32 owns deterministic art fit/codec/resolution. Selection/lowering belongs to mapgen. | Per-map Blender cannot supply arbitrary runtime seeds; a TypeScript generator or art-owned simulation types would duplicate owners. |
| M-S3 | C16–C19 give missing farmstead/home/tower/industry silhouettes separate acceptance surfaces. | Folding every missing silhouette into C13 would hide several visual variables inside a codec/bake slice. Attached/storefront/apartment coverage is tested through legal source variants at S2/C13. |
| M-S4 | C58 delivers fixed-seed play, then C55 adds runtime acquisition to the same battle path; C51 is final closeout. | Two delivery phases inside an undifferentiated final encounter slice would obscure the required early playable checkpoint. |
| M-S5 | C69/C72 establish physical river/forest contracts before C04; later ground visuals consume them. | Letting the compiler emit an as-yet undefined or soon-to-change ground schema would create avoidable second interpretations. |
| M-S6 | C54 is the integrated compiled-map/gallery gate after runtime and fixture cutover; construction invariants belong to C04/C52/C53. | Duplicating production validation or introducing an early/late gate with the same name would blur ownership. |
| M-S7 | C56 uses arena reservations through the same compiler; diagnostic bounds remain purpose-appropriate. | Rebuilding each focused fixture as a full generated battle contradicts the user's A choice. Decorative outside-sim cities would not satisfy physical surroundings. |
| M-S8 | C57 extends the camera intent owner with one pure clearance resolver over public/side-known geometry. | A second rig or hidden live-state collision source would split projection/knowledge ownership. |
| M-S9 | Same-build replays store exact compiled scenario/rules and engine build/digests; generation/encounter seeds use canonical strings at JS boundaries. | Seed-only regeneration depends on future generator/library versions, and Number/f64 cannot carry arbitrary u64 seeds losslessly. Battle RNG input remains separately recorded. |

A final consistency review caught three seams, now closed: SG6 inspects today's producers before C09, C58 uses a validated authored fixed-seed encounter; C59 owns map-aware recipe planning before runtime delivery, and C50 ratio tuning reruns C14 fit/bake with updated rule/config and appearance identities, keeping intact physical maps stable. C51 consumes the final candidate's balance evidence rather than duplicating the full report.

The [completed map](procedural-maps.md) owns all user givens and measured OPEN items. The [README](README.md) owns dependencies and the next pickup point.

## Write-spec synthesis: independent asset/runtime draft

The fourth draft used Claude Opus at high effort with an asset/runtime-lifecycle bias, independently of the three Codex drafts. Its [full consultation](drafts/asset-runtime-consultation.md) is planning evidence, not the live ladder. Claims were checked against map hashing, same-build replay, asset cleanup and `useStaticWorld.ts`; none are performance measurements.

| Finding / alternative | Canonical disposition and rationale | Owner |
|---|---|---|
| Physical and art hashes were fused | Split physical catalogue identity from appearance; art-only changes cannot move a scenario digest. Facade bay geometry is part of the shared physical contract. | C00, C13, C32 |
| Historical replay/asset retention promise exceeded current engine | Keep same-build compiled-scenario playback with engine-build and digest checks. No cross-build sim compatibility, pixel-exact old art or archival retention machinery. This removes an agent-added promise; no earlier user requirement requested it. | C55, C50 |
| Missing categories blocked all useful play | Incremental library coverage; labelled prototype rows support developer checkpoints through the same codec. C58 uses accepted art for its selected templates; C54/C51 require complete selectable coverage. G0 settles regional matrix. | C13, C32, C16–C19, C54 |
| Blender/source export, deterministic bake and resolver were fused | Separate offline source export from deterministic asset packing/fit/resolution. Asset check never runs Blender. | C13, C32 |
| First playable waited for final ground and recipe planner | First fixed-seed encounter is full Small size, authored in the existing encounter format and physically validated. Current ground look may be used; final ground composition remains release work. Runtime uses the subsequent recipe planner. | C58, C59 |
| Proposed ~3 km generated playable lab | Dropped: a full 12 km checkpoint preserves selected dimensions without adding another delivery target. Diagnostic maps keep only their accepted purpose-specific scope. | C58 |
| G0-T/G0-X split proposed | Keep one production gate; allow early failed-prerequisite reslicing. S0 inventories current allocations, focused followup proofs investigate failed owners, S7 handles plan composition and S1 active battle cost. Blocked dependents do not prevent a failure verdict. | S0, S7, S1, G0 |
| Main-thread WorldView duplicated worker world | Explicit inventory, worker preparation reuse and public export/query cutover. Camera/picking use public plus side-known geometry, never hidden live destruction. | S0, C33 |
| Regenerate all catalogue maps from source recipes | Dropped: reviewed compiled offline artifacts are an accepted outcome. Saved/generated acquisition converges on the same MapDefinition and scenario path; no new cache/source lifecycle is justified by measurement yet. | C04, C09 |
| Village house rendering had no removal condition | Name short-lived house appearance seam; cut over every existing consumer before deleting the branch. Village surroundings remain a separate geometry change. | C37, C34, C22 |
| Descriptor and aggregate contracts fused | Separate shared physical schema from simulation aggregate cutover. | C00, C01 |
| All focused maps cut over in one pass | Separate reservation proof, village, remaining labs and benchmarks. Explicit protected-behavior probes justify each envelope; a universal sensor-range margin is rejected because it cannot prove isolation. | C56, C34–C36 |
| Ruin rebake rewrote intact maps | Change terminal rules/config and appearance identity; rerun fit. Art rebake leaves intact catalogue/map identity stable. | C14, C50 |
| Planner duplicated full integrated seed matrix | Planner verifies C58, one seed per cell and edge/failure cases; multiple-seed gallery belongs to C54. | C59, C54 |
| Battle f64 seed range implicit | Map/encounter seeds use canonical u64 strings; existing battle RNG adapter accepts only safe JS integers. Avoid an unrelated battle RNG interface rewrite. | C55 |

No new game mechanic or requirement is delegated by this consultation. Numeric preset/resource/family decisions remain measured G0 outputs. Source quality, release coverage and full dimensions remain gates.

## Scale failure reslicing

Three independent risk-first, fewest-cut and seam-quality drafts agree on four failed owners: exact terrain/export, side-known navigation, truth/learned ground and fog delivery. The canonical [SA1–SA4 contracts](README.md#next-agent-prompt) inherit frozen S0 evidence. None assumes that sparse storage alone bounds search or that fixing JS arrays fixes GPU texture limits. Physical/source outputs and observation lifetimes must survive the consumer cutover; every failed arm remains explicit. Systems unlocks are separate from complete visual acceptance under the user's deferred-art instruction.

## Individual simulation resource verdicts

| Decision | Evidence-driven contract | Evidence | Scope |
|---|---|---|---|
| G0-SIM-SIGHT | Exact sensing/fog consumers use sparse direct page lookup, existing spatial indexes and a complete redundant-ray union proof. | Frozen paired source, maps, canonical outputs and failed arms are pinned by the [SA5 manifest](assets/sim-sight-cost/README.md). | Unlock SA5's production owner with exact battle parity. The attributed endurance bracket proves sensing/fog work only; complete active-world, whole-simulation timing and visual admission remain open. |
| G0-SIM-STRUCTURES | Resolved floor-band seats and occupied facade eyes; footprint/floor-band integrity; atomic low-rise ruin/tall gutted transition and side-owned corpse support | [Structure systems packet](assets/sim-structures/README.md), focused regressions and [integrated gates](assets/sim-lane-closeout/README.md) | Physical owners C40–43 unlocked. Legacy missing-fact policy and provisional coefficient remain disclosed; MG presentation, source windows and terminal art remain open. |
| G0-SIM-MOVEMENT | Connected terrain proof, resumable route revalidation, shared bridge approaches and contextual traffic recovery | [Movement packet](assets/sa6-movement/README.md), manifest SHA-256 `3c04fa1015a946305aea1b868f773eb3d9571787a415a54a99c765afe84c443f`, plus final merged clocks in [closeout](assets/sim-lane-closeout/README.md) | SA6 physical owner unlocked with named mechanic identities and observed route-cost tradeoff. Dense-unit cost is not completion or whole-world admission. |
| G0-SIM-BODIES | Ordinary street catalog rows and optional seeded route-preserving forest-floor admission | [C44](slices/C44-street-bodies.md), [C77 paired evidence](assets/sim-forest-bodies/README.md), [integrated gates](assets/sim-lane-closeout/README.md) | Physical owners unlocked. Street placement/models remain C45/C46; forest default stays zero, active timing and C78 drawing remain open. |
| G0-SIM-GROUND | All-LOD effective grass height at most 0.9 m; existing physical tree-line semantics | [C80](slices/C80-grass-presets.md), [C86](slices/C86-tree-lines.md), [integrated gates](assets/sim-lane-closeout/README.md) | Validator/tree-line simulation scope unlocked; wider presets, grass look and new art are unaccepted. |
