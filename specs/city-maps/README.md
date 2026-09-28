# City maps: real city districts as battle maps

Fight on real city ground, starting with a piece of New York. The map comes from real building footprints and roof heights, plus roadbed and sidewalk outlines. It is played under the village's rules: Hollywood realism, physical fire and sharp fog of war.

- **Buildings** are sim bodies with real footprints and heights. Their art is baked from three vendored MIT geometry-node building graphs (NYC, Paris, China) into **placement lists** of shared, instanced kit modules.
- **Streets** are the ground: roadbed, sidewalks and curbs.
- **Street furniture** is bodies.

**Done** means friends play one city encounter on this Mac at ≥30 FPS at the default camera. Buildings stop rounds and sight by their real heights. Squads fight from the bottom three floors. Low-rise buildings collapse to ruins and towers burn out standing. From above, the map is recognisably the real place.

**Read with this README:**
- [`decisions.md`](decisions.md): the interview (Q1–Q11, Compat) and the synthesis calls, each with its why and the alternative that lost.
- [`procedural-buildings.md`](procedural-buildings.md): the unknowns map for the vendored building graphs (Q-A … Q-J, landmines L1–L11).
- [`ground-look.md`](ground-look.md): the unknowns map for open-country ground (roads, rivers, forests, grass, farms) and the map catalogue (Q-G1 … Q-G19, L-G1–L-G11). It owns spikes SG1–SG6, gate GG and slices C60–C87. The ground synthesis (four drafts) is in `decisions.md`, "Ground synthesis".
- [`research.md`](research.md): data sources, the Spiderbench licence verdict and the original code map.
- `slices/`: one file per slice, the contract you implement.
- [`choices.md`](choices.md): the ledger of implementation choices the spec didn't make.

## Next Agent Prompt

**Status (2026-09-28):** planned; no slice started. Battle-look is closed (`specs/done/battle-look/`).

You are implementing `city-maps` in `/Users/david/dev/battlegame`. Use [implement-spec](../../.agents/skills/implement-spec/SKILL.md).

1. Read this README, `decisions.md` and `procedural-buildings.md`. Every decision in them is a given; don't reopen it. Read AGENTS.md's worktree recipe before making any worktree. Pull only the LFS files your slice needs.
2. **Start Phase 0.** The five spikes S1–S5 are throwaway and independent, so run them in parallel worktrees. Each writes `spikes/S<n>.md` with its numbers and a verdict against the kill thresholds in its slice file. **Nothing in the city lanes merges before G0.** The ground lane's six spikes SG1–SG6 run alongside and are read at **GG**. The forest and field sub-lanes gate on GG, not G0; SG1's tree budget also feeds G0.
3. **At G0,** decide on the evidence (see [G0](slices/G0-verdicts.md)). If a kill threshold fired, reslice before any Phase 1 slice starts. Then rewrite this prompt to point at the first Phase 1 slice of each lane.
4. Build lane by lane (the graph below). Shared contract edits (C01 → C02 → C03 → C04) merge serially.
5. **Before ending your pass,** update this section: status, the next pickup point per lane, blockers, and the TODO checklist.

**Blockers:** none.

**Warnings:**
- The sim does not limit gun elevation (S-pitch), so a tank can hit a floor-3 window point-blank. That's a known wrong moment and out of scope; don't fix it here.
- No fog cell finer than 8 m ships on a playable map before C07 (S-fogpub).

### TODO
- [ ] Phase 0: S1 sim scale · S2 graph export · S3 frame · S4 renderer fog · S5 seams → G0
- [ ] Phase G: SG1 tree and dressing cost · SG2 river on grid · SG3 road wear read · SG4 palette vs shadow floor · SG5 distance field · SG6 catalogue dry run → GG
- [ ] Map/sim: C01 buildings aggregate → C02 per-map fog cell → C03 surfaces → C04 importer · C09 fetched maps → C05 measuring tools → C06 sim scale passes* · C07 publication at scale* · C08 heightmap (cut candidate)
- [ ] Assets: C10 provenance → C11 kit modules → C12 baked materials · C13 placement bake → C14 damage placements · C15 interior atlas
- [ ] Renderer: C20 fog at scale · C21 material transport · C22 placement chunks → C23 far tier · C24 cutout → C25 glass → C26 interiors · C27 ruin and gutted art · C28 pavement (after C63) → C29 curbs · C30 markings · C31 city biome
- [ ] Rules: C40 floor-band seats → C41 facade eyes · C42 low-rise lifecycle → C43 tall buildings gutted
- [ ] Streets: C44 street bodies → C45 street models · C46 street placement
- [ ] Ground, catalogue: C60 catalogue data → C61 listings → C62 evidence rig
- [ ] Ground, roads and rivers: C63 surface distance field → C64 road kinds → C65 round centerlines → C66 road core → C67 shoulder → C68 ruts · C69 rivers contract → C70 bank bands → C71 bank roundness
- [ ] Ground, forests: C72 one forest rule · C73 tree skeleton → C74 species → C75 mix and colour → C76 canopy closure · C77 forest bodies → C78 body models → C79 dressing
- [ ] Ground, fields: C80 grass presets → C81 wild grass → C82 within-field variation → C83 crops → C84 field palette → C85 field texture · C86 tree lines · C87 ground composition gate
- [ ] Completion: C50 durability balance → C51 playable city encounter · C52 procedural layout generator (last; first to cut)

`*` = conditional. It closes without code if its spike or measuring-tool numbers are under budget.

## Slice graph

```
Phase 0 (throwaway, parallel)      S1 sim scale ─┐
                                   S2 graph export ─┬─ S5 seams (uses S2's driver)
                                   S3 frame (uses S2's counts) ─┤
                                   S4 renderer fog ────────────┴─► G0 verdicts ─► reslice if a kill fired

Map/sim     C01 aggregate ─► C02 fog cell ─► C03 surfaces ─► C04 importer ─┬► C05 tools ─► C06* · C07*
            C01 ─► C09 fetched maps ─────────────────────────────────────┘  C04 ─► C08 (cut candidate)
Assets      C10 provenance ─► C11 kit modules ─► C12 baked materials
                                             └─► C13 placement bake (needs C04) ─► C14 damage placements
            C15 interior atlas (any time after G0)
Renderer    C20 fog at scale (needs C01)
            C21 material transport ─┐
            C22 placement chunks (needs C13) ─► C23 far tier
                                    └─► C24 cutout ─► C25 glass ─► C26 interiors (needs C15)
            C27 ruin and gutted art (needs C14, C42, C43)
            C28 pavement (needs C03, C63) ─► C29 curbs · C30 markings · C31 city biome
Rules       C40 seats (needs C04) ─► C41 eyes        C42 low-rise lifecycle ─► C43 tall gutted
Streets     C44 bodies ─► C45 models (needs C11) · C46 placement (needs C04)
Phase G     SG1 · SG2 · SG3 · SG4 · SG5 · SG6 (after C09) ─► GG
Ground      C09 ─► C60 catalogue data ─► C61 listings ─► C62 evidence rig
            C03 + SG5 ─► C63 surface distance field ─► C64 road kinds ─► C65 round centerlines
                 ─► C66 road core ─► C67 shoulder (SG3) ─► C68 ruts
            C65 + C60 + SG2 ─► C69 rivers contract ─► C70 bank bands ─► C71 bank roundness
            GG ─► C72 one forest rule ─► C77 forest bodies ─► C78 body models
            GG ─► C73 tree skeleton ─► C74 species ─► C75 mix and colour (needs C72) ─► C76 canopy closure
                 ─► C79 dressing (needs C78, SG1)
            GG ─► C80 grass presets ─► C81 wild grass ─► C82 variation ─► C83 crops ─► C84 palette (SG4) ─► C85 texture
            C63 + C65 + C72 + C60 ─► C86 tree lines
            every retained ground slice ─► C87 ground composition gate
Completion  C50 balance (needs C40–C43) ─► C51 encounter (required city slices + C87) · C52 generator (last)
```

**Critical path:** S2 → G0 → C10 → C11 → C13 → C22 → C24 → C25 → C26 → C51.

## Pipeline

```
NYC Open Data (footprints + HEIGHT_ROOF, roadbed/sidewalk polygons), pinned; sha256 in SOURCES.json
      ▼
crates/city-import   read → CityPlan (the source-agnostic intermediate; C52's generator emits it too)
                     → decompose footprints into ≤N parts → floors from height (S-floors)
                     → fixtures/maps/<id>/{map.json, SOURCES.json}   (fetched by id, never bundled)
      ▼                                        ▼
sim: MapDefinition.buildings → parts as props;   asset city-bake <map>: headless Blender 5.2.1 runs the
one integrity, one garrison, one ruin per        archetype graph per part (floor heights, exposed edges,
building; surfaces; per-map fog cell             street outputs off) → per-tile placement files (LFS)
      ▼                                        ▼
observation ──────────────────────────► renderer: placement chunks of instanced kit modules, far tier,
                                        glass/cutout/interiors, ruin and gutted tiers, pavement
```

## Invariants: one owner per concept

These are how the finished code should read, as if designed today, not bolted on. A slice that breaks one is wrong even if its tests pass.

| Concept | Single owner | Never |
|---|---|---|
| A building (parts, height, floors, floor heights, archetype, seed) | `MapDefinition.buildings` (C01) | Building facts copied onto parts or into a side table |
| Building integrity, collapse and gutting | `sim::structures`, keyed by the building's owner prop | Damage stored on parts |
| Seats, capacity, eyes | `sim::garrison` (C40, C41) | The renderer or the bake inventing seat positions |
| Floor heights | Building data (C01), written by the importer | The bake or the renderer recomputing them |
| "Is this ground a road" | `world` surface index over `MapDefinition.surfaces` (C03) | The renderer re-deriving the rule; terrain reads the exported surface |
| Distance to roads, forests and water in the renderer | `terrain/surfaceField.ts` (C63), built from the export with the sim's distance function; C28 pavement and every ground slice read it | A second bake; per-fragment loops over shapes; a class or coverage mask |
| Surface-kind speeds | One `surfaces.<kind>.speed_factor` table (C64) | A rural-only table beside C03's kinds |
| Fog cell size | `MapDefinition.fog_cell_m` (C02) | `sensors.fog_cell_m` (deleted) |
| Map data, source-agnostic | `CityPlan` in `crates/city-import` (C04) | A consumer branching on "imported vs generated" |
| Building appearance | Placement files from `asset city-bake` (C13) | Per-building GLBs, catalog rows per building, runtime Blender, a TS graph evaluator |
| Static instanced drawing (trees, hedgerows, forest dressing, kit modules, far-tier tiles, corpses) | **One static-chunk owner, promoted from the scenery layer's existing chunk path** (`frame/sceneryLayer.ts`, `scenery/lod.ts`) in C22; corpse chunks move onto it; C23 and C79 only build instances for it | A second chunk path or model layer; per-instance CPU work per frame |
| Material coverage (opaque, cutout, blended, interior) | `scene-assets` `Material` (C21) | Alpha channels overloaded (albedo alpha is wear; ORM alpha is tint mask) |
| Street prop behaviour | Catalog body rows (C44) | A rule keyed on a kind's name |
| Map location and loading | `fixtures/maps/<id>/map.json` behind one fetch-by-id loader (C09, all maps at once) | A second map location, alias or compat import |
| Map catalogue (category, status, labels) | `fixtures/maps/<id>/meta.json` via `web/src/maps/catalogue.ts` (C60, C61) | A hand-kept list of maps; a committed generated index. Routes stay in `apps/battle-lab/src/fixtures.json`, each naming its map |
| Water | `MapDefinition.rivers` centerlines (C69), read through C03's index and C63's field | Water rects; a second shoreline rule in the renderer |
| Curved roads and rivers | Splines densified to ≤2 m points by the contract's loader (C65); the plot cutter reads the control runs | Grid-cell shading, or a renderer-only smoothing the sim doesn't share |
| Forest density, canopy and floor bodies | One `forests.rule` row (C72, C77) | Per-species or per-forest sizes; hand-placed forest bodies |
| Trees (forest, street, hedgerow) | One tree generator in `trees.py` (C73, C74) | A second tree technique for street trees |
| Grass and crop height | The effective-height validator (C80), including every multiplier | A cap on source assets only |
| Provenance | `reuse-manifest.json` (art); `SOURCES.json` per map (data) | Mixed allow-lists |

**Short-lived seams:** none planned. If a slice needs scaffolding, name it in the slice file with its removal condition and the slice that removes it.

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

## Budgets (S1 and S3 ratify or amend them at G0)

| Axis | Budget | Tool |
|---|---|---|
| Frame | ≥30 FPS average over the 300 s `city-contact` benchmark at the default camera, 1920×1080; worst-window GPU p95 ≤25 ms; static city ≤15 ms GPU at any camera | `/benchmark?preset=city-contact`, `frame-cost.md` |
| Ground lane | At most +3 ms GPU p50 on the village benchmark for the whole lane (trees ≤1.5, dressing ≤1, grass ≤0.5); GG ratifies | village benchmark, C87 |
| GPU memory | City adds ≤400 MB buffers and ≤200 MB textures over the village row | benchmark columns |
| Publication | City p95 ≤ the village's max today (19.8 KB/tick) | `city_report` |
| Sim | Step p95 within the target S1 proposes (kill: no known local fix brings it ≤16 ms); no tick over 33 ms | `city_report` (instructions retired) |
| Download | JS gzip within ±5%; per map, `map.json.gz` + placements ≤25 MB; kit bundles ≤50 MB | `vite build`, bake output |
| Village | Digests unchanged unless named; `endurance_report` instructions ±1% on digest-neutral slices | digest and replay tests, `endurance_report` |

## Firewalls (out of scope)

- **Other cities' maps and new city styles** (Eastern European and others; Q-B′). Paris and China are vendored and export-proven, but only NYC gets a map.
- **Any ODbL or OSM data** (Q5).
- **Spiderbench bytes; Google 3D Tiles or photogrammetry** (research.md).
- **The repo's two photo atlases** (`apartmentinterios.png`, `businesses.png`). Interiors use our own atlas (Q-E).
- **Emissive light or lamp glow.** Interiors are unlit.
- **Landmark look-alikes; real brands, logos, liveries or plates** (Q11).
- **Walkable interiors, room clearing, rooftops, floors above 3, underground** (Q3).
- **Gun elevation limits in the sim** (S-pitch; a later spec).
- **Civilians, traffic, night, weather, seasons.**
- **Village map changes beyond shared-schema cutovers, the named changes (C40, C42, C43; C65 rounded road corners; C72 one forest rule; C77 forest bodies; C86 tree lines only if the user opts in) and visual ground changes** (the ground lane, Q-G1).
- **Raising `TEXTURE_MAX_PX`,** or a kit texture inflating the shared texture array (L8).
- **Touching `../game`;** a bare `git lfs pull` in a worktree.

## Cut order if it runs long

1. C52 generator.
2. C08 heightmap (if the crop is flat).
3. C30 markings, then C29 curbs.
4. C26 interiors down to LOD0 only (the O-1 fallback).
5. C45/C46 down to cars, wrecks, Jersey barriers and street trees.
6. C27's burnt tier (gutted buildings drawn with the standing art).
7. Ground: C68 ruts, then C86 tree lines, then C79 dressing density (then all of C79), then C77/C78 forest bodies, then C85 field texture, then C74's species count.

**Never cut:** compound buildings, garrison bands, fog correctness, body-backed street props, provenance, instanced placement storage, the 30 FPS floor.

**Never cut, ground:** the map catalogue (C60, C61), the surface distance field (C63), round curves (C65, C71), the rivers cutover (C69), the one forest rule (C72), and the no-false-buff checks (crops ≤0.9 m, dressing only inside forests, sim-real tree lines).
