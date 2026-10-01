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

`mapgen::parcels::fill_districts` is implemented, and `generate` and `generate-map` (CLI and Wasm) now return a plan and a map with towns in them ([crate guide](../../../crates/mapgen/README.md)). The catalogue it is proved with is [`fixtures/prototype-building-templates.json`](../../../fixtures/prototype-building-templates.json): 29 labelled placeholder boxes of the regional family `prototype`, which certify no real dimension or appearance. Each district kind's streets and setbacks are rows of [`fixtures/map-presets.json`](../../../fixtures/map-presets.json), revision `layout-presets-3`. The decisions the spec left open are in the [choices ledger](../choices.md#c53-parcels-and-buildings).

**What a generated map now holds.** In-town streets (surface kind `road`, 7 m) joined to each settlement's road, paved aprons before apartment blocks and sheds, and one building per placed template, each on a parcel cut to that template. A plan also records the parcels (`lots`).

**Scale, over 100 seeds for each type and size (900 maps, none refused):**

| | Buildings | Parts (props) | Bay positions | Street km | Street strokes | Ground points | Built ground | Generate and compile, instructions (median, most) | `map.json` |
|---|---|---|---|---|---|---|---|---|---|
| Open Small | 404–853 | 632–1,299 | 6,493–13,303 | 7–16 | 54–104 | 2,805–5,628 | 2.4–4.4% | 0.5 G, 0.8 G | 1.2–2.4 MiB |
| Open Medium | 613–1,385 | 938–1,986 | 9,890–20,610 | 10–24 | 81–153 | 4,296–9,186 | 2.0–4.2% | 0.9 G, 1.4 G | 1.7–3.7 MiB |
| Open Large | 1,008–2,091 | 1,476–2,889 | 16,153–31,402 | 16–37 | 122–220 | 7,609–15,038 | 2.2–4.0% | 1.5 G, 2.3 G | 2.8–5.4 MiB |
| Mixed Small | 1,583–3,610 | 2,594–6,128 | 32,210–57,624 | 42–75 | 239–377 | 6,115–16,520 | 10.9–17.6% | 2.1 G, 3.0 G | 5.0–11.0 MiB |
| Mixed Medium | 2,319–4,633 | 3,782–7,479 | 40,535–75,575 | 57–105 | 313–525 | 10,719–24,803 | 7.4–13.3% | 2.8 G, 4.1 G | 7.0–13.6 MiB |
| Mixed Large | 3,077–5,311 | 4,848–8,291 | 55,348–83,188 | 68–115 | 406–588 | 14,339–25,030 | 6.6–9.9% | 3.5 G, 4.4 G | 9.3–15.0 MiB |
| Metro Small | 1,529–4,229 | 2,236–6,747 | 30,994–68,792 | 46–95 | 204–406 | 8,245–20,939 | 11.3–22.0% | 2.3 G, 3.4 G | 4.7–12.4 MiB |
| Metro Medium | 3,506–8,610 | 4,981–13,264 | 68,654–141,701 | 107–202 | 428–709 | 19,123–44,695 | 14.7–24.4% | 4.9 G, 7.4 G | 10.0–24.6 MiB |
| Metro Large | 5,967–13,275 | 8,911–21,552 | 118,870–218,617 | 172–303 | 643–965 | 30,552–68,938 | 15.6–23.9% | 8.3 G, 12.0 G | 17.6–39.0 MiB |

Instructions cover the layout, the parcel pass and the compile together. Wall time moved with the machine's load: a map took 0.2–2 s (median by cell) in this sweep, and 0.03–1.3 s in an earlier, quieter one. Built ground is the districts' share of the playable area, as in C52. `map.json` comes to about 3 kB a building, most of it bay positions.

**Density by district kind** (four maps, 3,757 ha of districts): garden suburb 6.5 buildings per hectare, village 5.7, town centre 11.3 (a terrace of three to five units is one building), apartments 1.8, core 2.1, farm 2.1, industry 0.8. These come from the preset setbacks and block sizes; nothing was thinned to fit a budget.

**Loaded into the simulation** (`city_report`, seed 1 of each, three units a side for 30 s; the machine was busy, so the tick times are upper bounds):

| | Buildings | Props with trees | World build | Battle build | Memory after build | Memory after 30 s | Tick p50 | Tick p95 |
|---|---|---|---|---|---|---|---|---|
| Mixed Small | 2,207 | 46,544 | 0.96 G | 1.13 G | 92 MiB | 329 MiB | 0.19 ms | 18.8 ms |
| Metro Small | 2,726 | 24,311 | 0.68 G | 0.91 G | 103 MiB | 388 MiB | 0.16 ms | 16.1 ms |
| Mixed Large | 3,934 | 116,298 | 2.21 G | 2.50 G | 169 MiB | 475 MiB | 0.55 ms | 51.2 ms |
| Metro Large | 9,274 | 94,387 | 2.26 G | 2.86 G | 310 MiB | 1,457 MiB | 0.55 ms | 15.8 ms |

Every size builds and runs. Tick 1, where six units plan a route, took 10–46 s; that is the route-planning spike SA2 is replacing and is left out of the table. Mixed Large's props are mostly trees.

**Checked by test** (`crates/mapgen/tests/parcels.rs`, on the compiled map, every type and size): the same request gives the same plan and map bytes; no building overlaps another, a carriageway, an apron or a forest, or leaves its district; every door faces a street or apron within 40 m with no building between; every street's pavement is joined to the roads that leave the map (Small maps); Open builds nothing above six floors and only Metro a highrise; one regional family per map; placed parts are their descriptors under a rigid transform; a district kind is built as densely on Large as on Small; tuning one district kind's parcels moves nothing else; what cannot be built ends in a named diagnostic; every map loads through `contract::maps::resolve`. Native and Wasm give the same bytes for eleven recorded requests, Metro Large's plan and Mixed Large's map among them (`fixtures/parity/map-layout/`).

**What the pictures show** (`mapgen inspect`, whole maps and single districts, against the Broken Arrow references). Garden suburbs read as detached houses on swinging streets; apartment districts as slabs and courtyard blocks, each behind its own parking apron, with open ground between; industry as sheds behind paved yards along access roads; a hamlet as farmsteads on a track.

**Still wrong or unfinished.**

- **Every district is a grid.** A suburb's streets are one swing repeated in parallel, with no crescents, greens or cul-de-sac loops. Apartment slabs stand single file along the street, not in ranked rows across a lawn. An industrial estate is one shed per parcel, not a few fenced compounds each with several buildings in one large yard.
- **Towns are still C52's rings of sectors**, so the street grids turn at straight sector edges and leave triangles of open ground between them.
- **A town centre is terraces only**: no square, no wider main street, no larger commercial footprint.
- **Metro's core is a field of towers** (about a hundred highrises on Large), where the reference has one tower as an accent. That is the `core` district's 75% highrise weight, a C52 preset.
- **Hamlets and villages get 7 m paved streets** where a lane would be a dirt track; streets are one kind and one width map-wide.
- **Links and run-ons are straight lines**: a link is a short diagonal, and a run-on may cross a field or a wood between two districts.
- **Movement through town is not proved.** A force crosses each map, but no test drives a vehicle down a street or walks infantry from a door to it; pavement connectivity and clear ground before each door are what is measured.
- **No workbench and no rejected-fit layer.** The picture is `mapgen inspect`; a place where nothing fitted is not recorded.
- **Every building is the prop type `building`** (400 hp, garrisonable), a tower and a shed alike, until C50.
- **The catalogue is placeholders.** Release coverage of the six categories waits for the specialist's sources (C13, C16–C19).
