# C52: preset layouts and connected roads

**Depends on:** C04; G0's versioned preset/coverage verdicts. **Kind:** required early slice.

## Question
Can each type/size/seed create its intended settlement and natural-cover composition with useful plains and connected approaches?

## Contract it unlocks
`generate_layout(&GenerationRequest, &PresetDefinitions) -> Result<MapPlan, Vec<Diagnostic>>` in Rust mapgen. Request fields are lossless seed, Open/Mixed/Metro, Small/Medium/Large and pinned generator/preset/physical-catalogue identity. Fixed extents and category rules come from the completed map; per-type counts/shares/weights/tolerances come from G0's versioned preset data, not player sliders or TypeScript constants.

Generate settlement plots, roads, forests, optional rivers/bridges and connected traversable plains. Ordinary settlement/street dimensions stay stable as larger maps add towns. Metro has one dominant central city and smaller surrounding districts; Open remains predominantly country with small residential towns. Road topology may use a richer connected network or a few main corridors. Sparse woodland varies coverage, preserving C72's one forest rule.

Balance top/bottom town area and forest coverage within G0's chosen metric/tolerance (forest area or count), without mirrored shapes. Reports may include both metrics; exact equality of both is not required. Keep useful approaches at 1,800 m weapon scale and infantry/vehicle access across town/plain transitions. Every map has flat base terrain except local river shaping. C65 owns curve densification; C03/C63 own classification/distance.

Use named random streams for sites, roads, forests/rivers, parcels and dressing so cosmetic changes do not relocate physical layout. Bound retries and return the failing feature/seed/config; never substitute a seed.

## API seam
`mapgen::layout` writes C04 MapPlan. Thin CLI/wasm adapters call the same library. The developer workbench exposes seed/type/size and plan layers; optional inspection handles are tooling, not a new player control or a second generator.

## What the human can run or see
A workbench showing full-extent Open/Mixed/Metro at every size: settlement bounds, roads, forest/river layers, usable plains and top/bottom metrics. Accepted `visualizations/map-character.html` is schematic inspiration, not a golden town count or ratio.

## Verification
- Fixed requests produce canonical byte-stable plans and IDs; changing size preserves metre scale while adding settlements.
- Validate exact dimensions, Metro's central hierarchy, Open height eligibility inputs, per-type composition and approximate coverage tolerance.
- Graph and physical movement connectivity agree across roads/bridges; infantry and vehicles reach the plain from towns.
- Impossible requests terminate within G0's bounded-work/generation budget.
- Visual variable: composition/access only. Compare full-map layers with the accepted schematic using compare-screenshots, then run unprimed screenshot-critique last. Use preview-shots for non-blocking review.

## Delegated to the implementer
Road/plot construction algorithms and shape variation within the ratified preset limits. New numeric defaults are spec gaps, not discretion.

## Must stay green
Fixed sizes, type character, useful plains, shared physical curves and deterministic failure.

## Feedback that would change this slice
Rejected composition updates versioned preset data and its evidence; it cannot silently shrink map dimensions.

## Outcome

`mapgen::layout::generate_layout` is implemented, with its CLI commands, Wasm exports and the inspection picture ([crate guide](../../../crates/mapgen/README.md)). The presets are data in [`fixtures/map-presets.json`](../../../fixtures/map-presets.json), revision `layout-presets-6`; the generator is `layout-4`. The decisions the spec left open are in the choices ledger: the [layout generator](../choices.md#c52-layout-generator), [rivers](../choices.md#rivers-in-the-layout-generator), and the [town model and edge roads](../choices.md#town-model-and-edge-roads), which replaced the first town model and road rule after the user saw its picture (M22, M23).

**What a layout holds.** Settlements with their districts, country roads and dirt tracks, each settlement's avenues (the streets between its blocks), forests, on a share of maps a river and its bridges ([below](#rivers)), and the measured open approaches. Parcels, district streets and buildings are C53's.

**Roads (M22).** The two sides start at the top and the bottom, so those two edges are the ones held to a road. One runs from each to the main junction, which lies within 500 m of the map's centre, inside 215 s at 110 km/h (15 s of that is the allowance for planning and turns). The two together are a road from the bottom edge to the top. Half of maps also have a road across the middle from the left edge to the right. Of the others, six in ten have one road in from the left or the right, and the rest have none. A road from an edge runs to the junction or forks onto an earlier one up to 1.5 km out, so four roads meet at the centre on a fifth to a third of maps, at every size. The main settlement stands on the junction. Every other settlement stands on a road or is joined to the nearest road good enough for its class, and a rich network links neighbours and brings more roads in from the edges.

**Towns (M23).** A settlement's site is the ground it may build on. Its roads cross that ground in straight lines. The ground is cut along them and then into blocks a few degrees off square, and the settlement builds outward block by block from where its roads meet, farther in some directions than in others, until it covers its class's share of the ground (35–50% for a hamlet, 58–80% for a town or a city). Each built block is one district of one use. Blocks take their use a few at a time, so a town is a patch of terraces here and an estate of apartment blocks there. A centre or a core lies where the roads meet, and industry takes blocks on a country road at the edge. What is left open is field, with a wood on some of it. A settlement's outline is the edge of its built blocks. A hamlet is a row of lots each side of a lane; a village is a few rows along its road.

**The seam.**

- **Plan.** `SettlementPlan.center` is the point of the settlement's roads nearest the middle of its ground. `outline` is the edge of its districts: a simple ring, which a ray from the centre may cross more than once. `districts` are convex blocks, nearest the centre first, each with the id, kind, area, anchor, floor limit and categories it had. `ApproachPlan` keeps its fields: `from_rad`..`to_rad` is a run of bearings whose corridors are open, and `front_m` is the corridor's width. `surfaces` holds each settlement's avenues as strokes of kind `road` (10 m), or `dirt_track` in a hamlet.
- **Presets.** `roads` gains the junction's offset, the chances of a road across the middle and of a lone side road, where a side road may leave its edge, the margin past a settlement's ground where a road turns, the sharpest turn, and how near a passing road counts as a main street. `transit` gains the junction's reach from the exact centre. A `towns` block holds the avenue width and how a settlement grows and cuts its blocks. A class has `built_share`, `block` (depth and length), `ribbon`, `neighbourhood` and `zones` in place of bands of sectors. A district kind has `ground_m`, the ground its largest parcel needs, and `streets.surface`. `types.<type>.centre` is a class only. `forests` and `retries` gain the woods beside towns. All are validated at load.
- **`measure`.** `transit` is `top` and `bottom` (the journey from each edge to the main junction), `centre_m` (the junction's distance from the exact centre), `top_bottom` and `east_west`. Each journey is `route_m`, `drive_s` and `elapsed_s`, and is absent when no road makes it, so `east_west` says whether a through road from side to side exists. `roads.centre_roads` is the most country roads meeting at one junction by the centre. A plan is refused unless `top`, `bottom` and `top_bottom` exist, the first two inside `transit.max_s`.

**Measured over 100 seeds for each type and size** (900 maps through the parcel pass and the compiler, none refused):

| | Settlements | Built ground | Main settlement's share of it | Forest | Country road km | Track km | Four roads meet at the centre | Road from side to side | Top or bottom edge to the centre | Bottom edge to top |
|---|---|---|---|---|---|---|---|---|---|---|
| Open Small | 8–10 | 2.3–4.7% | 35–66% | 4.6–21.8% | 8–27 | 5–25 | 25% | 51% | 114–182 s | 212–307 s, 6.0–8.9 km |
| Open Medium | 15–19 | 2.3–4.3% | 16–40% | 4.0–21.9% | 14–44 | 9–43 | 25% | 59% | 147–200 s | 277–374 s, 8.0–11.0 km |
| Open Large | 25–29 | 2.5–3.7% | 13–27% | 4.3–21.1% | 26–56 | 17–75 | 30% | 49% | 179–215 s | 343–401 s, 10.0–11.8 km |
| Mixed Small | 7–11 | 10.4–17.8% | 51–77% | 4.1–16.9% | 12–29 | 0–16 | 24% | 42% | 114–180 s | 211–292 s, 6.0–8.5 km |
| Mixed Medium | 13–17 | 8.1–14.3% | 36–63% | 4.1–16.9% | 20–44 | 4–24 | 19% | 49% | 147–209 s | 277–388 s, 8.0–11.4 km |
| Mixed Large | 22–27 | 7.5–10.8% | 27–47% | 4.1–17.0% | 30–66 | 11–53 | 28% | 63% | 179–215 s | 342–405 s, 10.0–11.9 km |
| Metro Small | 3–9 | 11.8–20.5% | 64–97% | 2.0–8.0% | 8–27 | 0–11 | 18% | 40% | 114–175 s | 211–301 s, 6.0–8.8 km |
| Metro Medium | 8–12 | 14.9–24.4% | 63–84% | 2.1–7.9% | 19–42 | 0–15 | 24% | 51% | 146–203 s | 277–389 s, 8.0–11.4 km |
| Metro Large | 15–19 | 15.4–25.1% | 60–82% | 2.1–8.0% | 29–57 | 4–22 | 19% | 47% | 179–215 s | 342–398 s, 10.0–11.7 km |

Every one of the 900 has all settlements and streets joined by road to the centre, top and bottom built ground and forest inside S7's tolerances, a corridor of open ground 400 m wide and 1,800 m deep to the main settlement in each half, and the two edge roads in time. With a river the same holds through its bridges. The layout alone costs 62–401 million instructions (median by cell, without a river), most of it the towns: cutting and growing them, and laying their avenues. What a whole map costs is in the [C53 outcome](C53-parcels-and-buildings.md#outcome). Native and Wasm give the same bytes for the recorded requests (`fixtures/parity/map-layout/`).

**Metro's urban share (M18).** The city may build on 16–24% of the playable area and builds on 66–80% of that, so built ground on Small Metro comes to 11.8–20.5% and the approach rule held on 100 of 100 seeds. The 24% ceiling was found on the first town model ([ledger](../choices.md#c52-layout-generator)) and was not searched again.

**What the user's references changed.** Two Broken Arrow screenshots of a Mixed map, given during the slice, shaped the towns: a district is one use (a garden suburb, an apartment estate, an industrial compound) rather than a blend; a larger settlement leaves fields and woods beside and between its districts; settlements stand on a main road that runs through them, with industry where the road enters; and every district has a stable id, an area and an anchor point so the encounter planner can put objectives on it. The first town model met those as rings of sectors about one point, which the user rejected as a dartboard. The blocks above are the answer to that.

**Checked by test** (`crates/mapgen/tests/layout.rs`, `towns.rs`, every type and size): the same request gives the same plan bytes; a bigger map adds settlements and keeps their dimensions; Metro has one dominant city and Mixed one clearly larger town; every plan is connected, fair and approachable, with the two edge roads in time and the junction by the centre; a share of maps has a road from side to side and a share has none; main roads meet in a crossroads on some maps and fork on others; an approach in the plan is open ground when walked; districts are single-use, addressable, convex and do not overlap; no point of a settlement has district edges fanning out from it; the outline follows the districts' own edges; no settlement is a filled round shape; no road runs through the inside of a district; a road runs through every settlement it serves; no road doubles back on itself; industry stands on main roads; presets that break a rule are refused at load, and a search that runs out is a named diagnostic.

**Still wrong or unfinished.**

Five unprimed reviews of the pictures were run against the question "does any town read as a diagram rather than a place?", with changes between them. The last answered no for towns, villages and hamlets, and still yes, in a new sense, for cities and large towns. Nothing is a dartboard, a pie or a set of rings any more.

- **A city is a compact disc about one crossroads, with its core as a bullseye.** It grows by distance from where its roads meet, so its edge lies at a roughly constant radius however ragged, and M23 asks for the core in the middle. It has one crossroads and four exits for 4–5 km of streets: no second radial, no bypass.
- **A large town on one road is a slab along that road.** Its blocks are rows behind the road, 8° off square at most, so the outline is a staircase of quadrilaterals and its districts are tiles. A town with a second road at an angle reads better.
- **Every map is one template.** The largest settlement is in the middle with a road through it from top to bottom (M22 asks for the road; the generator puts the settlement on it), satellites are spread evenly round it, and woods are scattered evenly between.
- **Open ground is blank.** No fields, hedges or plots meet a settlement's edge, and a wood keeps 25 m from every district, so each settlement stands in a bare halo. A settlement has no park, square or open block inside it.
- **Zoning is patchy, not graded.** Apartment estates and terraces can stand on a city's outer edge with suburb nearer in. Many cities and large towns draw no industry at all, and a small town can draw several patches of it.
- **A road that joins a settlement ends at the far end of its main street.** It reads as a dead end on the whole map.
- **A road is straight inside a settlement and bends at its edge**, and a road that only passes a settlement can run along its edge as that edge's street.
- **A track that ends on a town's main street crosses the town on its own diagonal**, and the blocks are cut along it.
- **Near-parallel roads and acute joins still occur**: two roads from one edge to one town, a track beside a road for a few hundred metres before it joins, tracks linking every hamlet to three or four neighbours on a rich Open map. A road still forks at a bridge's end.
- **Hamlets are a mini-grid as often as a row**: two lanes with rungs between them.
- **Movement is measured on the plan's road graph here.** What the simulation makes of a generated map is in the [rivers](#rivers) section and the [C53 outcome](C53-parcels-and-buildings.md#outcome).

## Rivers

The layout generator writes rivers and bridges; the decisions are in the [choices ledger](../choices.md#rivers-in-the-layout-generator) and the rules in the [crate guide](../../../crates/mapgen/README.md). The river rules are the same under `layout-4`. The measurements in this section were taken on `layout-3`, before the towns and roads beside the river became the ones in the outcome above, whose own table includes its river maps.

**The seam.**
- **Plan:** `MapPlan.rivers` (already admitted) is now written: none or one `contract::river::River`. New: `MapPlan.bridges`, rows of the contract's own `Bridge` (`deck`, `center`, `half_extents`, `yaw`, `deck_z`, `thickness_m`), omitted when empty.
- **Compiler:** `bridges` pass into `MapDefinition.bridges` unchanged. A deck that is not a finite box inside the playable rectangle is `invalid_bounds` at `$.plan.bridges[i]`. One whose end stands over water, or too near it for a ramp, is `invalid_river` at the same place, by the contract's rule, which is now callable for one deck (`contract::river::validate_bridge`).
- **Presets:** a `rivers` block (width, bank grade, freeboard, meander, how close a river's points lie, what keeps how far from the water, and `bridge`: deck type and size, landing, straight run, longest span, greatest skew, reuse distances), `types.<type>.river_chance`, `types.<type>.siting.beside_river`, `fairness.river` and `retries.river`. All are validated at load: a preset whose meander could bend tighter than the water and a road beside it are wide, or whose deck would end on the ramp, is refused.
- **Versions:** a request pins the generator and the presets revision, and one that names another is refused. A seed whose type draws no river gives the plan it would with rivers switched off.
- **`measure`:** new `river` (rivers, `top_km`, `bottom_km`, `fair`, `width_m`, `water_m2`, `bridges_top`, `bridges_bottom`) and `roads.unbridged`. The road graph has no link where a road enters water off a deck, so `unconnected_settlements` and `transit` are measured through the bridges. `plain_share` no longer counts water.
- **Refusals:** `river` (no course), `bridge` (a road found no stretch that takes a deck), `bridges` (a finished plan with a road in the water), `fairness.river`.
- **`mapgen inspect`:** draws the water and the decks, rings each bridge on a whole map, and takes `bridge-<n>` as a crop.

**What a river map is.** One river from the north edge to the south, 16–38 m wide, with a bed that falls one in four from each edge and a surface 1.2 m under the land. It passes the main settlement, never through a settlement or a wood (40 m and 10 m clear), and never through an approach to the main settlement. Each half holds a like length of it. Every road that has to cross does so once, by a deck 14 m wide that ends 9 m past the water; a road that does not have to cross follows the bank instead.

**Measured, a river on every map** (300 seeds for each type and size, 2,700 layouts, none refused; every one connected through its bridges, fair in town, forest and river, with an approach to the main settlement in each half):

| | River km | Bridges, median (range) | A half with no bridge | Approaches, both halves, mean (without a river) | Ground points, median (mean without a river) | Slowest edge to centre |
|---|---|---|---|---|---|---|
| Open Small | 6.6–11.2 | 2 (1–5) | 122 of 300 | 5.9 (9.1) | 6,219 (1,758) | 176 s |
| Open Medium | 8.9–14.6 | 3 (1–7) | 88 | 13.6 (19.0) | 8,818 (2,857) | 212 s |
| Open Large | 11.0–18.9 | 3 (1–7) | 54 | 25.4 (31.6) | 11,760 (4,644) | 215 s |
| Mixed Small | 6.6–10.5 | 2 (1–4) | 130 | 5.9 (8.8) | 5,996 (1,713) | 172 s |
| Mixed Medium | 8.7–14.5 | 2 (1–7) | 113 | 16.4 (22.5) | 8,422 (2,697) | 215 s |
| Mixed Large | 11.1–19.3 | 3 (1–8) | 61 | 33.3 (43.2) | 11,276 (3,865) | 215 s |
| Metro Small | 6.5–9.4 | 2 (1–4) | 210 | 5.5 (7.7) | 5,313 (1,199) | 174 s |
| Metro Medium | 8.7–12.2 | 2 (1–5) | 176 | 15.7 (20.0) | 7,326 (1,890) | 204 s |
| Metro Large | 11.0–15.7 | 2 (1–5) | 144 | 29.3 (34.9) | 9,609 (2,770) | 215 s |

**Measured, the shipped shares** (river chance 0.5 for Open and Mixed, 0.4 for Metro; 100 seeds a cell through the parcel pass and the compiler, 900 maps, none refused). 425 maps have a river and 475 do not. All 475 measure exactly as they did before this work: the same settlements, roads, forests, approaches and transit, and the same building, part, bay, street and ground point counts. Between maps with and without a river, settlements, built ground, forest and road length agree to within a few percent; what moves is the count of approaches (water ends one) and the ground points (a river adds 4,000–7,500 of them, its rounded samples).

| Generate and compile, instructions (median; most) | Before | Now, no river | Now, with a river |
|---|---|---|---|
| Open Small | 0.49 G; 0.74 G | 0.50 G; 0.74 G | 0.59 G; 0.87 G |
| Open Large | 1.40 G; 2.13 G | 1.34 G; 2.12 G | 1.60 G; 2.19 G |
| Mixed Small | 2.02 G; 2.90 G | 2.00 G; 2.91 G | 2.10 G; 3.59 G |
| Mixed Large | 3.32 G; 4.24 G | 3.30 G; 3.86 G | 3.55 G; 4.78 G |
| Metro Small | 2.25 G; 3.20 G | 2.17 G; 3.20 G | 2.29 G; 2.97 G |
| Metro Large | 7.94 G; 11.46 G | 8.23 G; 11.51 G | 8.13 G; 11.47 G |

The same seed without a river costs 0.1–0.8% more than before. The layout alone costs two to four times as much with a river (Open Large: 217 M instructions against 102 M).

**In the simulation.** A test loads generated river maps into the world (`crates/mapgen/tests/river_world.rs`): before each end of every deck the land can be stood on across the deck's width, the deck is stepped onto from that land's own height, it is ground over water from end to end, and the game's road graph reaches every settlement from the centre. `city_report` then sent six units a side across six river maps (300 s, edge to edge), and across the same seeds with the river off:

| Map | Bridges | Over the water at 300 s | Way covered, river / none | Route blocked | Still planning | Longest hold for a route | Costliest tick (instructions), river / none | Planning work, river / none |
|---|---|---|---|---|---|---|---|---|
| Open Small, seed 3 | 4 | 3 of 12 | 0.37 / 0.37 | 0 | 0 | 10 s | 135 M / 99 M | 1.2 M / 1.5 M |
| Mixed Small, seed 4 | 3 | 5 of 12 | 0.45 / 0.35 | 0 | 0 | 11 s | 123 M / 96 M | 1.5 M / 3.1 M |
| Open Medium, seed 1 | 2 | 3 of 12 | 0.22 / 0.26 | 1 | 1 | 300 s | 1,011 M / 115 M | 36.0 M / 2.3 M |
| Mixed Medium, seed 2 | 4 | 4 of 12 | 0.29 / 0.36 | 0 | 1 | 134 s | 169 M / 117 M | 20.9 M / 1.1 M |
| Mixed Large, seed 35 | 4 | 0 of 12 | 0.13 / 0.24 | 1 | 0 | 206 s | 185 M / 114 M | 24.7 M / 4.5 M |
| Metro Large, seed 4 | 3 | 3 of 12 | 0.29 / 0.28 | 0 | 0 | 43 s | 167 M / 125 M | 5.2 M / 0.5 M |

A unit over the water crossed by a deck: nothing stands on water. So units do cross by the generated bridges. Ticks are given in instructions because the machine was busy with another agent's browser runs: by the clock, the river runs had 0 to 9 ticks over 33 ms and the river-free runs 0 to 44.

Two things go wrong on river maps that do not on the same seeds without a river.

- **Planning.** On the 8 km and 10 km maps two units were refused a route and two were still planning at 300 s. Three ticks on Open Medium retired about 1.0 G instructions each, against 0.12 G at most without a river. Every start and goal was dry ground a kilometre or more from the water. This is the simulation's long search having to find a bridge (L09).
- **Columns meeting head on.** Given 900 s, seven of the eight vehicles on Mixed Small reached their goals on the far bank. On Open Small three of eight did: four blue vehicles stood waiting in a line behind a red jeep that was planning, on a road 700 m from the river, and were still there at the end. Without the river seven of eight arrived. A river sends both sides down the few roads that lead to a bridge, where they meet. (Before the riverside roads were thinned, the same seed's run had six of eight arrive, so this depends on small changes in timing.)

Neither is fixed here: both are in the simulation, which this work does not change.

**Checked by test** (`crates/mapgen/tests/rivers.rs`, `river_world.rs`, the CLI and compiler tests): the seed decides whether a map has a river, as often as the presets say; a seed without one is byte for byte the map with rivers switched off, and a river map is the same every run; the river runs north edge to south, a like length in each half, and the compiler's river rule accepts it; no settlement or wood reaches the water or its bank and the river's line runs through neither; no approach has water in it and the main settlement keeps one in each half; a road meets water only on a deck, every deck carries a straight road from end to end over water with dry corners; with the decks removed the same plan measures as cut; a built town beside a river has no parcel, building, yard or street on the bank and every street joined on its own bank; a road beside the water is written with the few points its line needs; a main road with corners at a bridge is timed along its rounded line and stays inside the transit limit; a river the map cannot hold is a named refusal; presets the terrain or a road cannot carry are refused at load. Native and Wasm agree on thirteen generation records, river plans and a compiled river map among them, and on three bridge cases of the compiler.

**Still wrong or unfinished.**

- **No river runs through a town**, and no town has a waterfront. Settlements stand 40 m or more back from the water with a field between, and their streets ignore it.
- **Rivers only run north to south, and there is one at most.** Every river map is a valley across the front.
- **Bridges are not balanced between the halves.** In 1,098 of 2,700 river maps one half has no bridge of its own (649 of those maps have a single bridge), and in 620 the halves differ by two bridges or more. The count by half is measured and not held to a rule.
- **A road onto a bridge it did not aim for turns sharply.** A road that runs along the river and then crosses, or that is led to a bridge already built, makes two corners of 60–90° at the deck's ends. A main road near the river follows its bends for a kilometre or more.
- **Two bridges can stand a few hundred metres apart** when a track's bridge was built before a country road came by more than 600 m off.
- **Bends are smooth, never tight.** No loops, oxbows, islands or tributaries; the width swells by 15% and no more.
- **Long moves on river maps can fail to plan, and opposing columns can jam on a bridge road,** in the simulation (above).
- **The layout costs two to four times as much with a river**, 45–115 M instructions more: 7–10% of generating and compiling an Open map, and 1–4% of a Mixed or Metro one. Where it goes was not measured.
