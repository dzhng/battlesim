# city-maps decisions

The write-spec interview with the user, 2026-09-28. Each entry is a given for the plan. The procedural-buildings decisions (Q-A … Q-J, L1–L11) live in [`procedural-buildings.md`](procedural-buildings.md) and are givens too. The old README question list (Q1–Q11) is answered here.

| # | Decision | Why |
|---|---|---|
| Q1 | **Both, real first.** The first map is real NYC data (footprints and roof heights from NYC Open Data). A **procedural layout generator** is a later slice in this spec. It emits the *same intermediate* the importer does (footprint polygons, heights and floors, archetype, roadbed and sidewalk polygons), so box decomposition, `MapDefinition`, bake and sim can't tell the two apart. | Real realism first, with invented maps later and no fork in the pipeline. |
| Q2 | A **mixed low/mid-rise district** (3–8-storey walk-ups and lofts), **1.6 × 1.6 km** like the village. C01 picks the crop from the Lower East Side, Greenpoint or Long Island City. | Known nav and fog costs. The NYC archetype fits most buildings, and garrisons stay meaningful. |
| Q3 | **Garrisons fight from floor bands.** Bands = min(floors, 3), and each band's slots have their own eye and muzzle height. **Capacity = one soldier per ~3 m window bay per band on outer facades only** (none on walls where footprint parts meet), with a **hard gameplay cap of 32**. **One eye per facade, at the highest band held on that facade** (eye count stays ≤4; on compound footprints, facades are the 4 directional groups of S-eyes). **Floor heights are sim data** (roof height ÷ floors, falling back to 4.2 m ground plus 3.2 m upper floors), and the bake passes them to Blender so windows line up with slots. Garrison shelter keeps today's flat `buildings.cover_strength`. Ruled out: floors above 3, rooftops, walkable storeys. | War-film test: an MG on floor 3 fires over the 2-storey shop opposite, and a 50 m block holds more than a shed. The cover-tier sub-rule was dropped once Q4 made gutted buildings ungarrisonable (no garrisoned building changes tier). |
| Q4 | **Height classes.** A building of **≤6 floors collapses** at 0 hp to a ruin whose **height scales with the building** (25% of height, from 2 to 6 m). A building of **>6 floors becomes gutted**: it keeps its height, still blocks and occludes, drops to medium cover tier, bakes as the burnt tier, **is not garrisonable**, and can't be destroyed further. Occupants of a gutted building take the collapse path (survival roll, escape suppressed). **hp scales with footprint area × banded floors** (today's flat 400 is wrong). The coefficients (25%, hp per m²) are left to the balance pass. Gun elevation: answered by S-pitch below (the sim doesn't limit it; out of scope). | A tower doesn't fold like a house. The user chose "not garrisonable" to keep it simple for now; ruin fighting can be a later rule. |
| Q5 | **Deferred, and the firewall stays.** No ODbL/OSM data in this spec. The importer's input seam is source-agnostic (the same seam as Q1), so a later city spec adds an OSM reader and accepts ODbL then. | Other cities are future work. |
| Q6 | **`fixtures/maps/<id>/SOURCES.json` per map, test-checked.** It records the dataset URL, version and date, licence, attribution, modifications and the sha256 of the pinned download, checked against the data allow-list (public domain, NYC Open Data terms). Raw downloads stay out of git. Generated maps record `project-owned` plus the generator seed. | It mirrors how `reuse-manifest.json` guards art. |
| Q7 | Superseded by the map: Q-D + L1. Blender evaluates the archetype graph per footprint part and exports **placement lists** of shared, instanced kit modules. | See `procedural-buildings.md`. |
| Q8 | **C00 decides the fog cell** by measuring 2, 4 and 8 m on the city crop. The value becomes a **per-map `fog_cell_m`**; the village keeps 8 m and its digests don't move. | The cost and correctness trade needs numbers. |
| Q9 | **Every street prop is a body** (L5: their lamp, bench, bollard, bins, hydrant, utility box, scooter and planter; our cars, wrecks, Jersey barriers, bus shelters, street trees and scaffolds). **C08 runs tweak-mechanics per kind** and writes each row by the war-film test (for example: a car stops rounds and a tank shoves it; a bollard stops cars, not tanks; a glass shelter blocks movement but not rounds or sight). Property rows only, no named cases. | Props-are-bodies (first-principles physics). |
| Q10 | Moot. Spiderbench is superseded by the MIT ProceduralBuildingsThreeJS source. | |
| Q11 | **Generic only.** No landmark look-alikes (a real footprint always gets archetype art) and no real brand, logo, livery or plate. Signs come from a **project-owned list of invented generic names** fed into the graphs' sign-text inputs at bake. | Trademark risk. |
| Compat | **Neither.** Hard cutovers: C02 migrates the village fixture in the same commit, with no compat shims or migration scaffolding. Village digests change only where a slice names it as a decision; otherwise digest and replay parity is the gate. | The user's default. |

## Scope notes from the interview

- **Future, out of scope:** new city styles (Eastern European panel blocks and tenements and so on), other cities' maps, ODbL. China's embedded scripts are the template for scripted archetypes. The kit still vendors all three styles (NYC, Paris, China), and only NYC gets a map here.
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
| S-spikes | **C00 is five throwaway spikes with kill thresholds**, S1 to S5 (sim scale, graph export, frame, renderer fog, seams), read at one gate, G0. | *One spike in boxes* (draft A): boxes can't falsify the art plan, and after L1 art is the dominant frame cost. |
| S-import | **The importer is a Rust crate, `crates/city-import`, depending only on `contract`.** Its output is typed against `MapDefinition` and written by the serializer the sim reads. | *A TypeScript package* (risk-first draft and Codex): it would re-declare the map types outside their owner. |
| S-surf | **Surfaces are exact shapes (`Polygon` or `Stroke`) behind a bucket index**, replacing `roads` on every map. The village's polylines become `Stroke`s with today's exact distance rule, so the village digest doesn't move. | *Lowering the village's roads to polygons* (performance draft and Codex): it moves the village's road-edge answers. |
| S-chunks | **Placements draw through one static-chunk owner, generalised from the corpse chunks** (`modelDetail.ts:66-110`). Corpses move onto it in the same slice. There is no second model layer. | *A separate placement layer:* two model owners. |
| S-fetch | **Maps are fetched by id, never imported into the JS bundle.** The village's map moves out of `village.json` in the same cutover. | A multi-MB city map in the bundle. |
| S-fogpub | **No fog cell finer than 8 m ships on a playable map before the publication slice (C07) lands.** The fog field is published every tick even though the sweep runs every 6th, so 4 m is 4× the bytes and 2 m is 16×. | |
| S-order | **The Q1 procedural generator is the last slice, and the first to cut.** A balance slice (hp coefficient, ruin ratio) sits before the encounter. | *Moving the generator to its own spec* (draft A): the user put it in this spec (Q1). |

## Ground lane

The open-country ground decisions (roads, rivers, forests, grass, farms, map catalogue) are in [`ground-look.md`](ground-look.md), Q-G1 … Q-G19. They are givens for C60–C72.
