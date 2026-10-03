# Procedural battle maps: towns and open plains

Generate seeded player battlefields with urban areas and usable surrounding plains. The player selects Open/Mixed/Metro and a separate Small/Medium/Large size, defined in the [completed unknowns map](procedural-maps.md), or plays a saved generated map from the catalogue. Developer village, lab and benchmark arenas retain their original extent under M29. Larger player maps add settlements at stable building/street scale. Roads connect towns to clear approaches at least 1,800 m deep across a 400 m front. Metro has a dominant central city with smaller surrounding districts; top/bottom town and forest coverage is comparable without mirrored geography. Maps play under the village's rules: Hollywood realism, physical fire and sharp fog of war.

- **Buildings** are sim bodies selected from legal, varied templates in six categories. An offline bake produces shared kit modules and reusable template placements from the NYC/Paris/China sources and dedicated missing families. Each map uses one coherent regional family. Runtime generation selects templates; it never runs Blender or stretches art to invent a footprint.
- **Streets** are the ground: roadbed, sidewalks and curbs.
- **Street furniture** is bodies.

**Terrain scope:** the initial procedural generator uses broadly flat ground; rivers retain their local bed and bank shaping. [Hills and ridges are deferred within this spec](deferred-terrain-relief.md), outside the current implementation and completion gates. Flat elevation does not permit empty countryside.

**Done** means friends play a generated encounter spanning town and plain on this Mac at ≥30 FPS, with safe camera movement around buildings. Reviewed fixed-seed artifacts ship first; runtime generation is required before this spec closes. A gallery covers every type × size across multiple seeds. Player battlefields have the wider rendered surroundings required by [scale direction](scale-direction.md); focused developer arenas get no added surroundings. Buildings stop rounds and sight at their physical heights; squads use the bottom three floor bands. The accepted collapse/gutting rules remain unchanged by category names.

**Read with this README:**
- [`scale-direction.md`](scale-direction.md): current user direction for playable closeout, startup and later optimization; the 20 km architecture requirement is deferred.
- [`generation-report.md`](generation-report.md) and its [parameter appendix](generation-parameters.md): detailed generation business rules, numbers, checks and diagnostic meanings requested by the user; refreshed at final identity integration.
- [`procedural-maps.md`](procedural-maps.md): the completed four-quadrant walk, accepted sizes/categories/presets, landmines, OPEN spike outputs and copyable kickoff prompt. It is authoritative for current map scope.
- [`decisions.md`](decisions.md): retained combat decisions, historical interviews and synthesis rationale; current map scope lives in `procedural-maps.md`.
- [`procedural-buildings.md`](procedural-buildings.md): the unknowns map for the vendored building graphs (Q-A … Q-J, landmines L1–L11).
- [`ground-look.md`](ground-look.md): the unknowns map for open-country ground (roads, rivers, forests, grass, farms) and the map catalogue (Q-G1 … Q-G19, L-G1–L-G11). It owns spikes SG1–SG6, gate GG and slices C60–C87. The ground synthesis (four drafts) is in `decisions.md`, "Ground synthesis".
- [`research.md`](research.md): historical NYC data research and source evaluation; its real-data recommendation is superseded for this spec.
- `slices/`: one file per slice, the contract you implement.
- [`choices.md`](choices.md): the ledger of implementation choices the spec didn't make.
- [`sim-lane.md`](sim-lane.md): the finished lane of simulation rules and costs, with what landed and what it left open.

## Next Agent Prompt

**Status (2026-10-02):** the former lanes are integrated on `main`, but the parent spec remains unfinished. Their Status sections are evidence inventories, not whole-spec acceptance: [buildings](buildings-lane.md), [ground](ground-lane.md), [sim](sim-lane.md), [startup](startup-lane.md), and the [scale reference](../done/city-maps-scale/evidence.md). The player can start generated battles or saved Market Town; developer arenas remain small under M29. Current selectable extents are 6/8/10 km. [The latest user direction](scale-direction.md#current-closeout-direction) defers 20 km architecture and further optimization; closeout prioritizes playable release maps. Historical scale proofs do not establish the current layout or rendered surroundings.

**Current pickup: finish playable closeout on the frozen generator.** `layout-13` / `layout-presets-12` integrate continuous coverage, the initial 1,500 m rendered margin and refreshed Market Town. Coverage uses actual circular ground observers, native fog sampling, intervening positions and clipped edges; source/receipt review and Native/Wasm cases pass. The 90-request gallery admits 89 maps across all nine type/size cells and retains Metro Small seed 10’s bounded `ground_sight` refusal. The [generation report](generation-report.md) owns the frozen logic/numbers; [C54](slices/C54-generation-gate.md) still needs its final integrated visual/access/playability verdict. Keep the user-selected 50% median openness floor, strict main approaches and cheap fairness policy.

**Integrated evidence ledger:**

| Contract | Current scope and owner |
| --- | --- |
| Listings, roads, yards and kerbside placement | [C61](slices/C61-map-catalogue-listings.md), [C64](slices/C64-road-kinds.md#outcome), [C31](slices/C31-city-biome.md), [C46](slices/C46-street-placement.md#outcome--kerbside-placement-2026-10-02) and [C53](slices/C53-parcels-and-buildings.md) are integrated. |
| Actual movement | Combined arrival and 901 Native/Wasm recovery digests pass on the identified refreshed map; [move validity](../done/move-validity/README.md) owns the mechanism. Later geometry needs its own arrival proof. |
| Rendered surroundings | Common map-owned playable, rounded physical and rendered extents are integrated. The existing scenery owners honor positive margins; zero-margin authored readouts are pixel-identical. Player preset rollout is integrated; current complete release admission remains open. |
| Physical countryside coverage | The shared forest owner supplies actual trunks/foliage to the continuous certificate. Twenty focused country behavior checks and canonical Native/Wasm generation/encounter cases pass; rejected maps retain their diagnostics. [The generator README](../../crates/mapgen/README.md) owns construction and refusal. |
| Focused integration and appearance | Frozen landscape and tour pictures have scoped acceptance. Source openings now match physical fighting bays, without changing catalogue identity. Inherited close grass, distant damage simplification and card crowding remain documented limits; these do not reopen numeric tuning. |

**Current tuning policy:** finish generation business rules with the current reasonable parameters and collect playable evidence. Do not tune every threshold or eliminate legitimate bounded refusals. The user will tune every business-rule number personally in a later map workbench with live results, like the unit editor; that editor is outside city-maps. The [generation report](generation-report.md) documents the logic, parameters and checks.

**Frozen implementation checks:** the full Rust test run passes; after repairing stale browser-test assumptions, all 960 browser tests pass. Four source-opening tests and the integrated asset check also pass (all eighty runtime files match). These are functional/source checks, separate from frame admission. The complete browser run and normal-encounter frame probe are in progress; the latest synthetic stress verdict remains failed. Final code review and consolidated choices are integrated; browser findings may still require a focused correction.

**Next order:**
1. [C55](slices/C55-runtime-generation.md)'s captured compiled-scenario replay and asynchronous saved-file admission are integrated. [C05](slices/C05-measuring-tools.md#corrected-stress-workload-v4)'s corrected preset v4 / tour v3 completes the five-minute window with correct camera motion, but records 25.9 average FPS and eight own firing units in the final minute. Both unchanged checks fail. Preserve that verdict and avoid further countryside tuning or deferred architecture work. The earlier phase-picture review establishes city framing with combat-model readability limits retained separately.
2. Finish [C50](slices/C50-durability-balance.md)'s integrated verdict with current coefficients. The fifty-trial village report is assembled with the corrected ten-flank cohort: forty retained trials, nine matching retested digests and the sole controller refusal removed. This is one full run plus a focused replacement cohort. The generated Mixed Small seed 1 assault demonstrates kilometre-scale infantry travel and combat over 2,700 simulated seconds; eight of nine attackers are placed, with one tank destination refused. A narrow follow-up reproduces that refusal and demonstrates the same tank driving to its deployment road, so no trapped-spawn defect is established. Preserve partial placement and the defeated assault, rather than claiming universal arrival or capture. Saved-map individual/group arrival already passes. Keep current approach geometry and weapon ranges.
3. Complete [C54](slices/C54-generation-gate.md)'s integrated verdict using the retained ninety-request matrix, Native/Wasm parity, physical/access checks and accepted frozen landscape review. Preserve bounded generation refusals and distinguish partial group placement from whole-command refusal. Reopen an owner only for a concrete correctness or playability failure; countryside parameter refinement waits for the later workbench.
4. Reconcile release loading, resources, publication and frame evidence at G0/GG using the [integration triage](buildings-lane.md#current-integration-triage). Under-30-second player startup and the 30 FPS floor remain requirements. Record missed inherited targets separately, including v4 GPU worst-window p95 of 40.56 ms against 25 ms; give each a named measured disposition without claiming it passed or starting deferred architecture work.
5. Once implementation is frozen, run `check` and `verify` once, fix actual failures, perform whole-spec review, consolidate `choices.md` against final code, and close the spec.

**Open-country contract:** [M24/M25](procedural-maps.md#closed-decisions) applies at every playable location on every generated map. Keep most countryside open and preserve the main settlement's 1,800 m approaches. Construction must account for intervening locations and failed placements; finite samples and two obstructed starts cannot admit the global rule. The [pickup review](assets/open-country-review/README.md) separates sampled sight, actual published fog and remaining landscape judgments against the [user's references](assets/reference/broken-arrow/SOURCES.md#open-country-reference-target).

**Working contract:** implement the whole spec with [implement-spec](../../.agents/skills/implement-spec/SKILL.md); delegate independent work to separate worktrees and share installed dependencies, never build outputs. Green focused passes may be merged and pushed to `main`. Use the GPU lock for every scene/render, hydrate only needed LFS assets, and keep scratch in ignored `throwaway/`. Narrow checks prove each pass; the full gate waits for whole-spec closeout. Update this prompt, the owning Outcome and the choices ledger at each checkpoint. Maintenance must leave one current pickup and no stale completed claims.

Read [M04 and the closed decisions](procedural-maps.md#closed-decisions), [scale/transit direction](scale-direction.md), [S0](spikes/S0.md) before new full-size allocations, and [the systems handoff](systems-handoff.md) before accepting appearance. Programmatic art and measured owner/resource constraints remain binding. A fourth town-look pass is justified by a failing composition review in play; historical diagram findings remain open until that decision is resolved.

**Evidence:** [S0 baseline](assets/scale-baseline/README.md), [SA1](spikes/SA1.md), [SA4](spikes/SA4.md), [navigation](assets/navigation-proof/README.md), [ground transport](spikes/SA3-transport.md), [ground learning](assets/ground-learning-work/README.md), [polygon surfaces](assets/surface-contract/README.md), [physical compiler](assets/map-compiler/README.md). Original rejected arms and numeric discrepancies retain their separate identities. Empty boot, uniform sampling and distant pending routes cannot prove active combat or arrival.

### TODO

A slice marked "physical" has its systems half done; its look is in the visual pass below ([what each left open](systems-handoff.md)).

**Done**
- [x] Scale: [S0](spikes/S0.md) · SA1 terrain/export · SA4 fog delivery · SA3 sparse ground (storage) · [S7 composition](spikes/S7.md) · [S1 first pass](spikes/S1.md) · [SA2 route planning and grid updates](slices/SA2-counted-route-integration.md)
- [x] Map and sim: C00 physical templates · C01 buildings aggregate · C02 fog cell · C03 surfaces · C72 one forest rule · C64 road kinds (physical) · C65 round centerlines (physical) · C63 surface index · [C69 rivers](slices/C69-rivers-contract.md) (physical)
- [x] Generation: C04 compiler (buildings and shared physical roads, rivers, bridges, forests and props) · [C52 layout generator](slices/C52-procedural-generator.md) with rivers, bridges and the M22/M23 town model · [C53 parcels and buildings](slices/C53-parcels-and-buildings.md) over the release physical template catalogue
- [x] Sim rules ([lane record](sim-lane.md)): C40 seats → C41 facade eyes · C42 low-rise lifecycle → C43 tall gutted · SA6 movement gaps · SA5 sight cost · C44 street bodies · C77 forest bodies · C80 effective-height validator · C86 tree lines (all physical)
- [x] Delivery: [C09 map resolution](slices/C09-fetched-maps.md) · [C60 catalogue data](slices/C60-map-catalogue-data.md) with compact saved maps · [C59 encounter planner](slices/C59-encounter-planner.md) · [C55 runtime generation](slices/C55-runtime-generation.md) from the menu · [C58 saved generated map](slices/C58-offline-encounter.md) (physical)
- [x] Roads: square ends, sound joints, bends at sharp forks, width changes only at a crossing
- [x] Town look, three passes: cities with arms on secondary roads, large towns on a second road, main settlements off the middle, streets square to every road, four-arm junctions at most, parks ([M27](procedural-maps.md#closed-decisions), [C52](slices/C52-procedural-generator.md#outcome), [C53](slices/C53-parcels-and-buildings.md#outcome))
- [x] [C46 street furniture](slices/C46-street-placement.md) in generated towns (as stand-in boxes) · open country furnished, with sampled individual-eye interruptions ([M24, M25](procedural-maps.md#closed-decisions))
- [x] [Startup lane](startup-lane.md): C33 one prepared world · C20 fog at scale · the camera lab through the catalogue · a native-against-Wasm pair with combat. Deploy-to-playable is 5 to 6 s on the largest map against a budget of 30 s (the owner's, 2026-10-01), so startup time is closed on that identified reference, with current-layout scale and surroundings still open
- [x] [Scale lane](../done/city-maps-scale/README.md): exact cost, delivery and resource contracts admitted on the identified full reference; [decisions and measured scope](../done/city-maps-scale/evidence.md)
- [x] Lab: [C57 camera clearance](slices/C57-camera-clearance.md) · the generated-map lab route

**In flight:** C54 evidence and remaining integration gates. Global countryside coverage and [one-report contact clarity](../battle-foundation/slices/06-contacts-and-audio.md) are integrated. C81 reduced blade facing and yellow dry patches are retained; the rejected finest-grain C85 prototype does not authorize further aesthetic tuning before play.

**Next, systems**
- [x] Kerbside parking: lane margin restored to 0.9 m after physical traversal, access and combined arrival proofs ([C46](slices/C46-street-placement.md#outcome--kerbside-placement-2026-10-02)); rendered-art/frame-cost acceptance remains C54's
- [x] Long-move admission and refreshed-map combined arrival: vehicle, squad and mixed moves physically arrive, individual stalled members recover, and intermediate shoves are rehearsed ([move validity](../done/move-validity/README.md)). The reversible isolated rehearsal carries long empty stretches between physical departure, interaction and arrival checkpoints; planner and physical interactions still cost work
- [x] Global countryside construction: continuous actual-observer coverage and bounded refusal are integrated; the frozen opening-fog and landscape review is accepted within its captured scope in C54
- [ ] Town look, what is left: stair-step outlines, hard seams between building kinds, streets meeting a main road 20 to 45 m apart on either side, dead ends where a street was refused; do it if towns still read as diagrams in play
- [ ] Scale at full extent: browser startup and memory, rendered surroundings, C20 fog at scale (C06/C07 are admitted on the [scale reference](../done/city-maps-scale/evidence.md); current-layout rendering integration and C22/C23 remain parent/buildings work)
- [ ] [C54 integrated seed gate](slices/C54-generation-gate.md), after the lanes
- [ ] C05 measuring tools: the distinct city-contact benchmark is integrated; complete its sustained contact, timed cost and tour evidence. C10/C13/C32/C21 are implemented by the buildings lane; their remaining integrated fit, appearance and download-budget judgments belong to C54/G0
- [ ] Completion: C50 durability balance → C51 playable generated encounter (requires C54 and C87)

**Deferred optimization:** the [20 km architecture envelope](scale-direction.md#architecture-envelope-20--20-km) is outside current completion gates. Retain existing measured evidence and allocation safety; do not start new maximum-extent experiments to finish this release.

**The visual pass**
- [x] **[Buildings lane](buildings-lane.md):** C10 sources → C11 kit → C12 materials → C13 placement bake → C32 template library · C22 placement chunks → C23 far tier · C16–C19 categories · C37 village houses · C21 → C24 cutout → C25 glass → C15 → C26 interiors · C14 → C27 ruin and gutted art. Its status lives in that file. **Done; open look questions are in its Status.**
- [x] **[Ground lane](ground-lane.md):** C62 rig · C66 → C67 → C68 country roads (with the looks of C64 and C65) · C28 → C29 · C30 town streets · C70 → C71 banks · C73–C76 trees · C78 → C79 forest bodies · C86 drawn tree lines · C81–C85 grass and fields · C45 street models · C31 → C87 composition. Its status lives in that file. **Done; open look questions are in its Status.**
- [x] Map catalogue listings and generic inspection: C61
- [ ] Gates G0 and GG closed from the lanes' final verdicts

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
| Yard and agricultural ground composition | C31’s renderer plot owner, using exported buildings and physical roads | A second urban-region export, colour-derived gameplay rules or `MapPlan` reaching the renderer |
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
  - narrow runners for each pass, and `bun run check` / `bun run verify` once at whole-spec closeout.
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
- **Hills and ridges,** [deferred within city-maps](deferred-terrain-relief.md), with no current implementation gate; local riverbeds/banks remain in scope.
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
