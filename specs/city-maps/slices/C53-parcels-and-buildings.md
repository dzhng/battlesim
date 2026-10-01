# C53: template-aware parcels and buildings

**Depends on:** C52, C04, C13's reusable geometry catalogue. **Kind:** required slice.

## Question
Can varied legal building templates fill settlement plots while keeping streets, entrances and battle routes open?

## Contract it unlocks
`fill_districts(MapPlan, GenerationRequest, TemplateGeometryCatalog) -> Result<MapPlan, Vec<Diagnostic>>` cuts blocks/parcels and selects legal immutable templates using G0's type weights and one regional family per map. The six categories and floor exclusions are defined in the completed map. Open never selects seven-storey+ buildings; highrises are Metro-only. Class labels do not change Q3/Q4 combat rules.

A template's physical footprint, parts, floor heights, entrances and join capabilities constrain parcel fill. Fit parcels to legal envelopes or choose another eligible template; never stretch houses, floors or streets to fill a lot. Translate/rotate at stable metre scale. Courts/yards are intentional parcel results. Entrances face reachable streets and compounds/terraces use only S5/C13's baked compatible edge variants. Unsupported fits return bounded diagnostics.

C04 materializes authoritative building facts from the selection. C13 is a reusable input built independently of maps; there is no per-generated-part Blender job. Map/request identity pins the physical catalogue. C32 separately pins compatible appearance; replacing materials/LOD never changes this map.

## API seam
`mapgen::parcels` → C04 MapPlan rows → C01 MapDefinition.buildings. C13 supplies C00 geometry descriptors; C32 and C22 resolve shared module placements. The renderer never infers lots from textures.

## What the human can run or see
Workbench layers for parcels, category/floors, selected template ID, entrances and rejected fits. One generated block renders from the reusable library without rerunning Blender.

## Verification
- Canonical seed/request/physical-catalogue identity and stable building IDs; complete native/wasm parity against frozen S6 inputs.
- Descriptor bounds/floors match compiled geometry; no overlap with roads, rivers, reserved plains or other buildings beyond G0's named tolerances.
- Every entrance reaches a street by infantry; vehicles can cross town into plain.
- All type × size cases use one compatible regional family, satisfy exclusions and stay within full-extent budgets.
- Visual variable: parcel fit and building distribution only. Compare overlays and block shots with accepted layout and template evidence using compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Parcel subdivision and eligible within-preset template variation. Template stretching, ad hoc art fallback and new weights are not delegated.

## Must stay green
One authoritative building row, matching template art and accessible streets/plains.

## Feedback that would change this slice
Insufficient variety or poor street frontage expands the template library/preset evidence rather than inventing per-map art.

## Outcome

`mapgen::parcels::fill_districts` is implemented, and `generate` and `generate-map` (CLI and Wasm) return a plan and a map with towns in them ([crate guide](../../../crates/mapgen/README.md)). The catalogue it is proved with is [`fixtures/prototype-building-templates.json`](../../../fixtures/prototype-building-templates.json): 29 labelled placeholder boxes of the regional family `prototype`, which certify no real dimension or appearance. Each district kind's streets and setbacks are rows of [`fixtures/map-presets.json`](../../../fixtures/map-presets.json), revision `layout-presets-6`. The decisions the spec left open are in the choices ledger: [parcels and buildings](../choices.md#c53-parcels-and-buildings), and the [town model](../choices.md#town-model-and-edge-roads) for what changed when districts became blocks.

**What a generated map holds.** The layout's roads and the avenues between a settlement's blocks; inside each district, streets joined to them (surface kind `road`, 7 m, or dirt lanes in a hamlet); paved aprons before apartment blocks and sheds; and one building per placed template, each on a parcel cut to that template. A plan also records the parcels (`lots`).

**How a district is built.** A district is one block of its settlement, bounded by roads, avenues and the settlement's edge ([C52](C52-procedural-generator.md#outcome)). Its streets are a grid along the edge that runs longest with the settlement's main street, so neighbouring districts' long streets run the same way. Parcels front every carriageway in or along the district: its own streets, and the road or avenue on its edge.

**Scale, over 100 seeds for each type and size** (900 maps, none refused):

| | Buildings | Parts (props) | Bay positions | Street km | Street strokes | Ground points | Built ground | Generate and compile, instructions (median, most) | `map.json` |
|---|---|---|---|---|---|---|---|---|---|
| Open Small | 530–1,094 | 926–1,815 | 8,938–16,805 | 8–19 | 91–221 | 2,212–9,290 | 2.3–4.7% | 0.45 G, 0.76 G | 1.7–3.3 MiB |
| Open Medium | 815–1,655 | 1,293–2,530 | 13,165–24,865 | 12–27 | 131–311 | 3,413–12,406 | 2.3–4.3% | 0.67 G, 1.02 G | 2.4–4.6 MiB |
| Open Large | 1,211–2,009 | 1,826–3,023 | 19,382–30,777 | 17–33 | 234–418 | 5,104–16,560 | 2.5–3.7% | 1.01 G, 1.50 G | 3.4–5.5 MiB |
| Mixed Small | 2,642–4,915 | 4,773–8,856 | 45,413–81,505 | 53–100 | 395–743 | 5,316–15,831 | 10.4–17.8% | 1.76 G, 2.51 G | 8.7–15.7 MiB |
| Mixed Medium | 2,966–6,509 | 5,040–11,311 | 52,514–105,585 | 62–135 | 549–1,127 | 6,248–25,156 | 8.1–14.3% | 2.40 G, 3.40 G | 9.3–20.2 MiB |
| Mixed Large | 4,432–7,898 | 7,739–14,295 | 79,490–128,476 | 92–151 | 781–1,346 | 10,580–26,835 | 7.5–10.8% | 2.97 G, 3.93 G | 14.2–25.1 MiB |
| Metro Small | 1,220–5,163 | 1,784–9,840 | 30,937–94,779 | 49–117 | 325–762 | 3,891–19,453 | 11.8–20.5% | 1.69 G, 2.45 G | 4.1–17.4 MiB |
| Metro Medium | 4,442–10,526 | 6,726–19,665 | 80,369–198,559 | 126–227 | 897–1,562 | 11,692–35,135 | 14.9–24.4% | 3.47 G, 5.20 G | 13.0–35.5 MiB |
| Metro Large | 7,908–16,304 | 13,112–27,697 | 154,566–278,268 | 212–365 | 1,448–2,516 | 20,317–51,756 | 15.4–25.1% | 6.10 G, 8.76 G | 25.3–50.1 MiB |

Instructions cover the layout, the parcel pass and the compile together. Built ground is the districts' share of the playable area, as in C52. `map.json` comes to about 3 kB a building, most of it bay positions.

**Density by district kind** (five maps, 3,837 ha of districts): garden suburb 6.3 buildings per hectare, village 5.2, town centre 11.6 and small centre 14.4 (a terrace of three to five units is one building), apartments 1.9, core 2.1, farm 2.7, industry 1.0. These come from the preset setbacks and block sizes; nothing was thinned to fit a budget.

**Loaded into the simulation** (`city_report`, seed 1 of each, four units a side crossing for 120 s):

| | Buildings | Props with trees | `map.json` | World build | Battle build | Memory after build | Memory after 120 s | The 120 s of battle | Tick p50 | Tick p95 | Slowest tick |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Mixed Small | 3,055 | 48,366 | 9.6 MiB | 1.03 G | 5.83 G | 201 MiB | 219 MiB | 89.7 G | 0.14 ms | 3.9 ms | 6 ms |
| Metro Large | 9,845 | 98,793 | 33.1 MiB | 2.42 G | 14.41 G | 575 MiB | 602 MiB | 149.8 G | 0.30 ms | 6.2 ms | 9 ms |

Builds and the battle are in instructions retired. No tick ran over 33 ms on either map. The same two seeds under the first town model (`layout-3`) had 2,207 and 9,276 buildings and cost 4.79 G and 13.01 G to build a battle on, 171 MiB and 548 MiB after 120 s, and 78.4 G and 135.6 G for the battle: a Mixed map holds about a third more buildings than it did, because smaller blocks have more street frontage, and costs about a fifth more to load and run.

**Checked by test** (`crates/mapgen/tests/parcels.rs`, on the compiled map, every type and size): the same request gives the same plan and map bytes; no building overlaps another, a carriageway, an apron or a forest, or leaves its district; every door faces a street or apron within 40 m with no building between; every street's pavement is joined to the roads that leave the map (Small maps); Open builds nothing above six floors and only Metro a highrise; one regional family per map; placed parts are their descriptors under a rigid transform; a district kind is built as densely on Large as on Small; a hamlet's lanes are dirt tracks; a district's parcels stand square to its streets, not to the town's centre; tuning one district kind's parcels moves nothing else; what cannot be built ends in a named diagnostic; every map loads through `contract::maps::resolve`. Native and Wasm give the same bytes for the recorded requests, Metro Large's plan and Mixed Large's map among them (`fixtures/parity/map-layout/`).

**What the pictures show** (`mapgen inspect`, whole maps and single settlements, against the Broken Arrow references). Garden suburbs read as detached houses on swinging streets; apartment districts as slabs and courtyard blocks, each behind its own parking apron, with open ground between; industry as sheds behind paved yards along access roads; a village as rows of houses along its road; a hamlet as farmsteads on a lane.

**Still wrong or unfinished.**

- **Every district is a grid.** A suburb's streets are one swing repeated in parallel, with no crescents, greens or cul-de-sac loops, and the picture of a large suburb reads as wood grain. Apartment slabs stand single file along the street, not in ranked rows across a lawn. An industrial district is one block on a road at the town's edge, but still one shed per parcel, not a few fenced compounds each with several buildings in one large yard.
- **Neighbouring districts' streets do not join.** Each grid stops at the avenue on its district's edge. The grids run the same way, but their streets are not placed to meet across the avenue, and many end as stubs at it or at the settlement's edge.
- **A district's ground reaches past its last buildings.** A block is cut to a depth and its parcels stand along the carriageways, so the back of a deep block is empty ground in the district's colour. It shows most in hamlets and villages.
- **A town centre is terraces only**: no square, no larger commercial footprint. Its main street is the country road or a 10 m avenue.
- **A village's streets are paved.** Only the farm district's are dirt lanes.
- **Links and run-ons are straight lines**: a link is a short diagonal, and a run-on may cross open ground inside a district.
- **Movement through town is not proved.** A force crosses each map, but no test drives a vehicle down a street or walks infantry from a door to it; pavement connectivity and clear ground before each door are what is measured.
- **No rejected-fit layer.** A place where nothing fitted is not recorded.
- **Every building is the prop type `building`** (400 hp, garrisonable), a tower and a shed alike, until C50.
- **The catalogue is placeholders.** Release coverage of the six categories waits for the specialist's sources (C13, C16–C19). What a district kind needs of its block (`ground_m`) is kept in step with the catalogue by hand.
