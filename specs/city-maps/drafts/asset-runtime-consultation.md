# Independent asset/runtime draft — consultation only

Claude Opus, high effort, 2026-09-30. This is an independent alternative, not implementation instructions; the disposition lives in decisions.md. Source observations were inspected, not benchmarked.

I've drafted my own graph and compared it with the current spec: the README, 24 slice files and the relevant code. I built nothing, edited nothing and ran nothing in a browser. Every number below is arithmetic or read from the code; none is a measurement.

# City-maps consultation: asset/runtime lifecycle and decision budget

## 1. My independent plan for the whole feature

### Main idea

The **physical template set** and the **art for it** have separate lifecycles and separate content hashes:
- the physical set (the "geometry catalogue") is the list of legal building shapes the simulation sees;
- the art is the module placements that draw those shapes.

Maps, digests and replays pin only the geometry catalogue. The art only has to cover it. So swapping a labelled placeholder for real art, or rebaking ruin art, never changes a map or a battle digest.

### Contracts at the boundaries between components

```text
contract::templates   (new; pure data, no art)
  TemplateGeometryCatalog { hash, templates: [TemplateGeometry {
      id, class: Farmstead|Detached|AttachedShop|Apartment|Highrise|Industry,
      family, floors, floor_heights, parts (local boxes), exposed_edges,
      entrances, joins }] }
  Frozen at the template gate. A new shape is a named geometry change.

contract::map  (hard cutover)
  MapDefinition { …, buildings: [BuildingDefinition { template_id, frame,
      materialized parts, floors, floor_heights, edges, entrances }],
      land_regions, template_catalog_hash }
  The template id is per building; the catalogue hash is per map.
  No art hash appears anywhere in the map.

contract::generation
  GenerationRequest { kind: Open|Mixed|Metro,
      extent: Preset(Small|Medium|Large) | Diagnostic([w,h]),   // Diagnostic: catalogue only
      seed: DecimalU64String, generator_version, preset_revision,
      template_catalog_hash }
  CatalogueSource { request | ArenaComposition { reservation, authored fragment,
      surroundings request }, expected_map_hash }

crates/mapgen          request + catalogue → MapPlan → validate → lower → MapDefinition + diagnostics
                       (one crate, called natively and through wasm)
sim scenario builder   EncounterDefinition, one type for every source:
                       authored (labs, village, first fixed seed) or planned (plan_encounter)
game-wasm, worker      prepare_battle(PrepareBattleRequest) → PreparedBattle
                       { static-world arrays, scenario, identity }
                       One world is built, in the worker. The main thread gets typed
                       exports and never builds a second world.
scene-assets           recipe export (Blender, offline, cached by recipe hash)
                       → library bake (TypeScript, deterministic, inside `asset check`)
                       → TemplateArtLibrary { covers: template_catalog_hash,
                           per template × state (intact | terminal):
                           placements, status: prototype|release }
                       → resolve(template_id, frame, side-known state) → placements
                       → one static-chunk owner
replay file            { PrepareBattleRequest, composite identity, Replay }
                       Loading regenerates the map, checks its hash and digests,
                       and refuses on any mismatch.
```

### Slices

**Spikes (throwaway):**
- **S0 extents:** arithmetic first; run only the storage owners the arithmetic says exceed the ceiling.
- **S1a layout trials (plan only):** presets, the top/bottom fairness metric and open-approach geometry. No sim.
- **S1b active battle cost at full extent.**
- **S2 template export**, **S5 joins**, **S6 native/wasm parity:** S6 runs on a labelled box catalogue.

**Gates:**
- **G0-T (template gate)** reads S2, S5, S6 and S1a. It freezes the shape set, the class × family matrix, the identity split and the presets.
- **G0-X (extent gate)** reads S0, S1b, S3 and S4. It fixes the storage and query architecture and the budgets.

**Templates:**
- T1: `contract::templates` plus a checked-in prototype geometry catalogue with the frozen shapes.
- T2: a generated labelled-massing art source (generated from the geometry, the way grass already is). This is the missing-art contract.

**Map:** C01a descriptor types → C04 lowering → C52 layout → C53 parcels. Every one of these needs only G0-T.

**P1, the first playable:** a generated `mapgen-lab`.
- Diagnostic extent of about 3 km, clearly labelled as a lab and not a player preset. It exists to prove the pipeline, not the size.
- Generated through wasm in the worker.
- Authored encounter, prototype massing, existing ground look.
- Proves generate → compile → sim → render → play on this Mac.

**Extent:** the G0-X architecture slices → C06/C07/C20 as required → P2 = C58, a Small fixed seed with an authored encounter pinned to its map hash.

**Art:** C11 kit export for one family → C13a recipe export → C13b library bake and resolver → C22 chunks. Then C16–C19 per class × family, each switching its cells from prototype to release. C14 terminal states per class.

**Rules:** C01b aggregate → C40–C43.

**Runtime:** R1 `prepare_battle` in the worker (lands with P1) → C59 planner → C55 menu, request, cancel, replay.

**Cutover:** C09 resolver over catalogue sources →
- C56a: the reservation boundary plus geometry-lab;
- C56b: the village (named digest change);
- C56c: the remaining labs;
- C56d: endurance and benchmark (performance rebaseline).

**Closeout:**
- C50 balance.
- C54 gallery: nine type × size cells × 10 seeds; release status required for every class × family the presets can select.
- C51 closeout.

**Fast things the user can look at, in order:**
1. S1a/C52: a top-down gallery of all nine type × size cells with no art and no sim (days).
2. P1: a playable generated town and plain at lab extent.
3. P2: Small at full extent.
4. The runtime menu.

Each art slice is a contact sheet that also shows up in P1/P2 without moving a digest.

---

## 2. Changes the existing spec needs, highest risk first

### 1. The replay promise can't be kept, and it creates an asset-retention duty nobody owns

**Evidence:**
- A `Replay` holds only the scenario/config digests, the seed and the accepted commands (`crates/sim/src/battle.rs:167-173`). Playback re-runs the simulation, and `from_replay` refuses on any digest mismatch (`battle.rs:538-545`). So any sim code change breaks old replays whether or not the compiled map is stored.
- Yet M-S9 (`decisions.md:121`), C55 (line 19), C50 (line 11) and brief L12 promise that preserved replays keep their original compiled scenario and library.
- `asset bake` deletes every runtime hash directory no current bake produces (`web/asset.mjs:221-223`), and `asset check` fails on orphans (`asset.mjs:256-258`). C13's "library files live outside generic bundle cleanup" and C50's "the asset owner retains referenced immutable data" therefore need a second retention mechanism, which no slice owns.

**Fix:** use same-build replay identity, the policy the village has today.
- The replay file is `{PrepareBattleRequest, composite identity, Replay}`.
- Loading regenerates the map and checks its hash and the digests.
- A mismatch refuses with a message naming the identity.
- Remove the retention and "older generator" wording from C13, C50, C55, M-S9 and L12.

Cross-build replays would need a versioned sim and belong in a separate spec. **This is the user's call** (§3).

### 2. The template library hash is part of map identity, so any art change moves every map and digest

**Evidence:**
- The inputs the brief hands the builder put `template_library_hash` into `GenerationInput`.
- C01 puts `{library_hash, template_id}` on every building (line 9).
- C13 pins "geometry/placement hashes together" (line 14).
- `scenario_digest` hashes the whole map JSON (`battle.rs:338-347`).

So a material or LOD tweak changes every catalogued map, every lab digest and every replay.

**Fix:** split the two hashes as in §1.
- `template_catalog_hash` (physical) goes into the request and the map.
- The art library declares which catalogue hash it covers. `asset check` fails if any template lacks placements for each state.
- The run identity records the art hash for diagnostics, but it never enters the map or the digest.

Change the inherited contracts in `procedural-maps.md`, plus S6 (line 12), C01, C13, C55 and the provenance row in the README invariants table.

### 3. Placeholder art has no place outside the spikes, so the first playable waits on all the missing art

**Evidence:**
- Placeholders are allowed only in S2/S6.
- C13 requires "every required category has accepted art" (line 27).
- C22 depends on C13, and C58 depends on C22 (C58 line 3).
- C13 depends on C16–C19 and C10–C12 (README line 46).

So play waits on roughly ten authored sources.

**Fix:** add T2, generated labelled massing, as the art-side missing-input contract, with a per template × state `status: prototype|release` in the art library.
- C54 and C51 refuse any preset-selectable class × family cell still at prototype.
- Each C16–C19 acceptance becomes "these cells switch to release". This doesn't touch digests, because the shapes are frozen at G0-T.
- Removal condition: C51 deletes the prototype generator once every cell is release.

### 4. The first playable is buried behind most of the plan

**Evidence:** C58 depends on C59, C22, C31, C40–C43, C57 and C06/C07 (C58 line 3). C31 needs C83, crops (README line 82), which sits behind C80–C82 and the ground gate GG.

**Fix:**
- Add P1 (a generated lab at diagnostic extent). It needs only C01, C04, C52, C53, C09, T1/T2 and R1.
- Rebase C58 on G0-X plus an **authored** encounter pinned to the map hash.
- Drop C31, C83 and C59 from C58's prerequisites. Ground look isn't a playability question, and C59 is needed first for runtime seeds.
- C59 then has to plan a legal encounter on the C58 map as one of its tests.

### 5. The gate G0 couples two independent lanes

**Evidence:** G0 depends on S0–S6 plus SG1 (G0 line 3). Production map/sim work is gated on G0 (README line 29), which leaves the template/generator lane waiting on full-extent scale work. S1 also mixes plan-only layout trials with active battle cost ("S1/S3/S4 measurements" in O04; "S1 infantry/vehicle crossing" in O03).

**Fix:**
- Split into G0-T (S2, S5, S6, S1a) and G0-X (S0, S1b, S3, S4, SG1).
- Split S1 into S1a (plan-only composition, the fairness metric, approach geometry) and S1b (battle cost at extent).
- C01a, C04, C52, C53, C11 and C13 wait only on G0-T.

### 6. The main thread builds a second full world beside the worker's

**Evidence:**
- `apps/battle-lab/src/useStaticWorld.ts:28` constructs `WorldView`, which calls `WorldGeometry::new` (`crates/game-wasm/src/lib.rs:43-48`), on the main thread.
- The worker builds a `Battle` from the same scenario (`lib.rs:525-528`).
- At 18 km a 4 m height field is about 20.25 million cells, allocated twice, plus the map JSON crossing threads. Runtime generation in the worker would add a third copy of the map.
- S0 mentions "worker/browser copies" only in general terms.

**Fix:**
- Name this owner in S0's inventory.
- Make R1's contract `prepare_battle` → the worker's single world, with static exports (terrain/props/roads) sent to the main thread as transferred typed arrays.
- Remove main-thread `WorldView` construction in the same cutover.
- The tests that construct `WorldView` directly (`web/tests/fog.test.ts:46`, `scenery.test.ts:42`, `terrainSurface.test.ts:37`) move to the new export path.

### 7. Catalogue maps stored as compiled JSON create a second lifecycle and a split between offline and runtime

**Evidence:**
- C04 has the CLI write `fixtures/maps/<id>/{map.json,SOURCES.json}` (line 13).
- C09 moves every map there (line 9).
- C56 adds generated surroundings to 11 labs, the village and endurance (the lab sizes are in `fixtures/*-lab.json:2`).
- Every generator change regenerates and commits every compiled surrounding. Offline maps load from JSON while runtime maps generate — two paths, one of them unexercised until C55.

**Fix, as the default for S6 to confirm:**
- The catalogue stores a `source.json`: a `CatalogueSource` with `expected_map_hash`.
- One resolver generates through mapgen, natively and in wasm, for catalogue and runtime sources alike.
- A hash-pin test fails on drift and forces a named rebaseline.
- Add a gitignored, content-addressed compiled cache only if S6 shows catalogue generation exceeding the startup budget.
- C55 then shrinks to the menu, request, cancellation and replay.

### 8. C13 fuses four owners, and Blender would end up inside `check`

**Evidence:**
- C13 covers the Blender run, the library codec, validation and the resolver (C13 lines 9-12), and demands coverage/variety.
- `asset check` re-bakes everything in memory (`asset.mjs:229-258`). `bun run check` doesn't call `asset check` directly; it reaches the bake through the web tests (the `package.json` `check` script).
- Blender today is a separate script stage (`blender/build_sources.sh`, as the scene-assets README describes).

**Fix:**
- C13a: recipe → Blender → module GLBs and placement rows, cached by recipe hash, with project-owned manifest entries. Never run in `check`.
- C13b: the TypeScript library bake, the coverage check against `template_catalog_hash`, and the pure resolver.
- Move the class × family coverage requirement to C54.

### 9. The village house draws through its own path, with no removal slice

**Evidence:**
- The `building` prop draws with `"drawn_by": "building"` (`fixtures/props/generic/structures.json:6`).
- C22 introduces `drawn_by: "placements"` and must keep "house GLBs untouched" (C22 lines 11, 39).
- C01 gives the house a descriptor. No slice deletes the per-building GLB path, so two building appearance owners would survive to closeout.

**Fix:** C56b turns the village houses into library templates, with `house.py` as the farmstead source feeding C13a/C16. In the same commit it deletes `drawn_by: building`, its bundles and its loader branch. State this as a short-lived seam in the README.

### 10. C01 has two questions and two owners

**Evidence:** C01 covers contract descriptor types plus the sim aggregate, export, decoder and village cutover (C01 lines 9-16).

**Fix:** C01a is the contract descriptor types only (needs only G0-T). C01b is the sim aggregate cutover, with village parity.

### 11. C56 cuts everything over at once

**Evidence:** C56 bundles:
- the reservation boundary;
- the village's named digest change;
- about 11 labs' focused checks;
- the endurance/benchmark performance rebaseline;
- per-lab reservation bounds, left to the implementer (C56 lines 9-15, 32).

**Fix:**
- Split into C56a–d.
- Replace the implementer's discretion with a rule: reservation = the hull of authored bodies, spawns, script points and camera stations, plus a margin no smaller than the lab's longest sensor or weapon range. When that doesn't fit, grow the bounds north and east without moving anything.
- C56a proves the rule on geometry-lab.

### 12. The ruin-ratio rebake is over-engineered

**Evidence:**
- Ruin height is a rule (`structures.json:5`; the C42 formula), not map geometry.
- scene-assets already holds art to the sim's ruin height (its README, "Fit to a prop's box"; `catalogFootprints.test.ts`).
- C50 (line 9) and L12 prescribe republishing the library and refreshing maps and replays.

**Fix:** with the hash split, a ratio change fails `asset check`. Rerun C13a's terminal states and rebake. The art hash moves; maps don't. Delete the refresh-maps/replays steps.

### 13. C59 carries the whole matrix

**Evidence:** C59 requires "all nine type × size combinations" (line 30), duplicating C54.

**Fix:** C59 checks the C58 map plus one seed per cell. The matrix stays in C54.

### 14. Seed ranges

**Evidence:** the `Battle` constructor takes `seed: f64` (`lib.rs:525`), and the village reads `Number(...)` (`village.tsx:39`).

**Fix:** state that `PrepareBattleRequest.battle_seed` is either restricted to at most 2^53 or also carried as a decimal string. Don't leave it implicit.

### Audit of the next high-risk slices

| Slice | Finding | Change |
|---|---|---|
| S0 | One question, but up to five alternative prototypes, which can sprawl | Run arithmetic first; execute only the owners over the ceiling; one alternative each |
| S1 | Two questions | Split into S1a/S1b (change 5) |
| S2 | Also reproduces China's scripts "informationally", which is scope creep | Drop it or move it to a note |
| S6 | Pins the library hash | Pin the catalogue hash instead |
| G0 | Two lanes | Split (change 5) |
| C01, C13, C56 | More than one owner | Split (changes 8, 10, 11) |
| C58, C59, C55 | Contain what are really gates | Tighten (changes 4, 13, 7) |
| C22 / village house | Orphaned transitional path | Name its removal (change 9) |

**Missing reproductions or parity checks:**
- P1 needs a native-versus-worker parity row: same request, same map hash, same scenario digest.
- C13b needs a check that a prototype-to-release switch leaves the digest unchanged.

---

## 3. Decisions

### Only the user can make these

1. **Replay policy:** same-build replays (recommended; matches today) or cross-build replays (needs a versioned sim, so a separate spec). This contradicts accepted brief line L12.
2. **Family coverage for release:** how many of NYC, Paris and China must be complete at closeout. Recommendation: one family fully complete for the first friend playtest, and all three for C51. Farmstead and industry shared across families with palette tints; homes and towers made per family.
3. **Placeholder massing in playtests:** may P1/P2 friend playtests use labelled massing? Recommended yes; C54/C51 refuse it at release.
4. **A diagnostic-extent generated lab as the first playable:** this is not a player preset. It needs explicit approval, because "no silent shrinking" could otherwise be read as forbidding it.
5. **Authored encounter first:** may the first fixed-seed encounter be authored before C59 exists?

### The spikes can settle these

- Shape set and variety counts (S2, S5).
- Generation time, and whether catalogue maps need a compiled cache (S6).
- Storage and query architecture and budgets (S0, S1b, S3, S4).
- Presets, the fairness metric and the approach criteria (S1a).
- Fog delivery (S4).
- Camera margins (C57).
- The reservation margin, proved on geometry-lab (C56a).

---

## What I covered

- **Code:** the map contract (`contract/src/map.rs`); `ScenarioDefinition`, `BuildingRules` and `GarrisonRules` in `contract/src/scenario.rs`; the building integrity store (`sim/src/structures.rs`); the replay format and digests (`sim/src/battle.rs` around lines 167-173 and 338-545); `sim/src/digest.rs`; the grid cell sizes in navigation, visibility and terrain; fog packing in `publication.rs`; the wasm boundary (`game-wasm/src/lib.rs`); `sim/src/endurance.rs`; `web/asset.mjs` bake/check; the scene-assets README and schema; `useStaticWorld.ts`; the village replay flow; the lab fixture registry and lab sizes; `structures.json`.
- **Spec:** the README; `procedural-maps.md`; slices S0, S2, S5, S6, G0, C01, C04, C09, C11, C13, C14, C17, C22, C27, C40, C42, C50–C56 and C58–C60; and `decisions.md` rows G-R1–G-R5 and M-S1–M-S9, read after I drafted my graph.
- **Not read:** C57, C16/C18/C19, the renderer and ground slices C20–C31 and C60–C87, S1/S3/S4, the SG spikes, `choices.md`, and the benchmark scenario internals.

