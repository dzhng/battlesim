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

`mapgen::parcels::fill_districts` is implemented, and `generate` and `generate-map` (CLI and Wasm) return a plan and a map with towns in them ([crate guide](../../../crates/mapgen/README.md)). The generator's physical catalogue is [`fixtures/prototype-building-templates.json`](../../../fixtures/prototype-building-templates.json), derived from the accepted China-family building sets despite its historical filename. Each district kind's streets and setbacks are rows of [`fixtures/map-presets.json`](../../../fixtures/map-presets.json). The decisions the spec left open are in the choices ledger: [parcels and buildings](../choices.md), and the [town model](../choices.md) for what changed when districts became blocks.

### Facade clearance (2026-10-02)

A parcel reserves room for its building's art as well as its physical parts.
The existing global parcel admission already prevents physical building overlap;
the reported Mixed Small seed 1 passes that check. Its dense centre setbacks,
however, were narrower than the apartment kit's permitted facade overhang.
Both centre kinds now reserve 1.5 m on each side, the largest accepted kit's
side fit, through their existing general setback rows. The generator still
reads only physical templates and presets; appearance identities remain separate.
The uniform allowance also leaves that room when a centre selects a house.

The compiled-map regression grows every part by that allowance and checks that
it stays inside its own parcel, allowing only the existing centimetre-coordinate
rounding. It failed on the seed-1 courtyard apartment before the setback change
and passes afterward. The parcel file's broader proofs cover every type and size
at seed 1 and the maximum seed, including roads, entrances, district edges,
physical separation and rigid template placement.

A full production generation of Mixed Small seed 1 has no physical overlaps in
either arm. Pairs of permitted art envelopes that intersect fall from 167 to zero;
buildings fall from 3,195 to 3,050. These are conservative envelopes, not a count
of visible mesh intersections. Two real-route captures hold the original camera
targets at tactical and wide zoom, and their pixel differences demonstrate the
placement change. The independent fresh critique accepted this narrow spacing
change. Street-furniture art clearance and broad city visual acceptance remain
C54 work. The combined generator/preset revision, parity records and saved
Market Town include this change together with kerbside placement.

**What a generated map holds.** The layout's roads and the avenues between a settlement's blocks; inside each district, streets joined to them (surface kind `road`, 7 m, or dirt lanes in a hamlet); paved aprons before apartment blocks and sheds; and one building per placed template, each on a parcel cut to that template. A plan also records the parcels (`lots`).

**How a district is built.** A district is one block of its settlement, bounded by roads, avenues and the settlement's edge ([C52](C52-procedural-generator.md#outcome)). Its streets are a grid along the edge that runs longest with the settlement's main street, so neighbouring districts' long streets run the same way. Parcels front every carriageway in or along the district: its own streets, and the road or avenue on its edge.

**How a district's streets are laid, second pass** (`layout-10`; [ledger](../choices.md)). The grid is fitted between the district's edges: a whole block from a carriageway on an edge to the first street, half a block from an edge that faces the fields to the last. Each line then runs to the carriageway ahead of it and ends just past its middle; where a street already meets that carriageway from the far side within 30 m, the line is moved to meet it there, and two streets of one width become one street through the junction. A street that would come in at less than 45° turns to meet the road square. A line that meets nothing ends at the last lots (a long street) or on the last long street (a cross street). A piece of grid no road crosses is joined by one of its own streets carried on, never by a stub. A street or avenue that runs on less than 35 m past its last junction and stops in the open is cut back to that junction. Garden suburbs and villages have a cross street every 140 m and 160 m.

Over the same 100 seeds for each type and size (900 maps, none refused), against `layout-9`:

| | Buildings | Street km | Street strokes | Ground points | Generate and compile, instructions (median, most) | `map.json` |
|---|---|---|---|---|---|---|
| Open Small | 652–1,270 (549–1,133) | 11–25 (8–19) | 35–88 (38–93) | 3,926–11,783 | 1.19 G, 1.63 G (0.99, 1.35) | 1.9–3.8 MiB |
| Open Medium | 908–1,883 (874–1,685) | 17–39 (12–26) | 67–145 (63–138) | 7,244–17,429 | 1.78 G, 2.55 G (1.47, 2.15) | 2.6–5.4 MiB |
| Open Large | 1,513–2,420 (1,305–2,080) | 26–47 (17–32) | 107–184 (102–186) | 11,558–23,910 | 2.75 G, 3.78 G (2.24, 3.13) | 4.4–6.8 MiB |
| Mixed Small | 2,997–5,233 (2,720–5,011) | 66–115 (53–100) | 147–266 (172–323) | 10,158–24,271 | 4.80 G, 6.84 G (4.12, 5.88) | 10.1–17.9 MiB |
| Mixed Medium | 4,008–7,261 (3,056–6,543) | 89–161 (62–134) | 228–399 (242–471) | 16,656–36,373 | 6.91 G, 8.90 G (5.69, 7.67) | 13.1–23.5 MiB |
| Mixed Large | 5,210–8,610 (4,569–7,909) | 116–185 (93–150) | 308–496 (356–554) | 22,004–44,040 | 8.67 G, 11.31 G (7.26, 10.06) | 16.3–28.3 MiB |
| Metro Small | 1,677–5,627 (1,277–5,301) | 63–132 (50–117) | 137–271 (142–326) | 6,091–26,777 | 4.61 G, 6.59 G (3.80, 6.24) | 5.4–19.4 MiB |
| Metro Medium | 4,985–10,532 (4,573–10,689) | 155–278 (125–228) | 326–596 (384–679) | 21,150–54,794 | 10.16 G, 15.14 G (8.29, 12.34) | 15.3–35.0 MiB |
| Metro Large | 8,218–17,517 (8,142–16,518) | 266–425 (211–365) | 553–851 (617–1,068) | 33,715–82,944 | 18.14 G, 24.60 G (14.71, 21.27) | 27.6–58.1 MiB |

A map has about a quarter more street and a fifth more generation work: a fitted grid fills its district where the old one left the back of a block empty, suburbs have more cross streets, and a large town or a city has more frontage along its secondary roads. Fewer strokes carry it, because a street is one stroke from junction to junction. The most authored bodies on a Metro Large map is 53,933 of the 60,000 the game allows.

**How a district's streets meet the roads round it, third pass** (`layout-11`; [ledger](../choices.md)).

- *The grid's line.* A district's long streets run with the edge that most of the carriageways round it run along or square to, a road's length counting double and an open edge's a quarter: a district beside a road has streets beside that road and square to it. Between edges that serve alike it is still the one that runs longest with the main street.
- *Landing.* A street lands on the carriageway ahead on its own line where it comes to it within 20° of square. Up to 28° off it turns at its last crossing and runs square to the carriageway, a bend 21 m or more from it. Beyond that it does not land. It does not land within three of its widths of a junction it makes no crossroads of, or within 60 m (`parcels.junction_clear_m`) of a junction of the layout's roads. It makes a crossroads only with a street that ends on the far side in line with it, within 25°, where no other junction is by.
- *Not landing.* A street that may not land is cut back to its last crossing with its own grid, or, where none is near, until 60 m of ground lies between its end and the carriageway (a row of lots), and it goes altogether where all of it is nearer than that. Streets and avenues the run-on step leaves stopped short are cut back the same way; where that would leave a district no parcel (a hamlet's one lane), the avenues keep their length.
- *Bends.* A `bend` is now one bow or less along the district: its wave is 1.2 to 2.4 times the district's length (or the preset wavelength, where longer), it reaches no farther than 4% of that length, and its reach changes from one side of the district to the other, so no two streets bend alike and those near the middle run straight.

Over the same 100 seeds for each type and size (900 maps, none refused), against `layout-10`:

| | Buildings | Authored bodies | Street km | Street strokes | Generate and compile, instructions (median, most) | `map.json` |
|---|---|---|---|---|---|---|
| Open Small | 646–1,309 (652–1,270) | 1,551–2,983 (1,531–2,910) | 10–24 (11–25) | 30–90 (35–88) | 1.21 G, 1.84 G (1.19, 1.63) | 2.1–3.9 MiB |
| Open Medium | 989–1,915 (908–1,883) | 2,140–4,045 (2,134–4,196) | 15–35 (17–39) | 62–131 (67–145) | 1.83 G, 3.15 G (1.78, 2.55) | 2.7–5.4 MiB |
| Open Large | 1,578–2,517 (1,513–2,420) | 3,434–5,366 (3,415–5,359) | 23–45 (26–47) | 99–180 (107–184) | 2.87 G, 4.07 G (2.75, 3.78) | 4.6–7.0 MiB |
| Mixed Small | 2,592–4,715 (2,997–5,233) | 7,476–12,922 (8,365–14,837) | 58–97 (66–115) | 149–248 (147–266) | 4.35 G, 5.51 G (4.80, 6.84) | 8.4–15.9 MiB |
| Mixed Medium | 3,527–6,716 (4,008–7,261) | 9,734–17,834 (11,682–20,545) | 81–149 (89–161) | 211–382 (228–399) | 6.13 G, 8.64 G (6.91, 8.90) | 11.1–20.8 MiB |
| Mixed Large | 4,905–8,170 (5,210–8,610) | 14,066–22,029 (14,988–23,252) | 105–166 (116–185) | 294–467 (308–496) | 7.90 G, 10.52 G (8.67, 11.31) | 15.6–26.3 MiB |
| Metro Small | 1,117–5,240 (1,677–5,627) | 5,546–16,053 (7,482–17,828) | 39–121 (63–132) | 79–277 (137–271) | 3.84 G, 6.13 G (4.61, 6.59) | 3.9–17.2 MiB |
| Metro Medium | 4,139–9,936 (4,985–10,532) | 15,491–30,005 (18,933–33,691) | 128–237 (155–278) | 296–495 (326–596) | 8.98 G, 12.88 G (10.16, 15.14) | 13.5–32.6 MiB |
| Metro Large | 7,530–16,919 (8,218–17,517) | 29,534–49,459 (32,394–53,933) | 232–398 (266–425) | 510–862 (553–851) | 16.13 G, 20.92 G (18.14, 24.60) | 24.0–54.8 MiB |

A Mixed or Metro map has about a tenth less street and fewer buildings and bodies: streets that would have met a road at a slant or beside a junction stop a block back, parks and open blocks take ground, and the lots there front the road. An Open map is much as it was (its largest has 3% more buildings; villages have no parks). The most authored bodies on a Metro Large map is 49,459 of the 60,000 the game allows, down from 53,933.

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

- **Every district is a grid.** A suburb's streets are one swing repeated in parallel with a cross street every 140 m (second pass), with no crescents, greens or cul-de-sac loops. Apartment slabs stand single file along the street, not in ranked rows across a lawn. An industrial district is one block on a road at the town's edge, but still one shed per parcel, not a few fenced compounds each with several buildings in one large yard.
- **Neighbouring districts' streets join where one lies within 30 m of the other, in line with it** (second and third pass). Farther apart they meet the avenue as two T-junctions; 19 pairs on the 36 test maps stand 2 to 20 m apart and are counted by `tests/street_warts.rs`, with 3 ends that stop short of a road (two of them country roads), 1 pair side by side and 1 double bend. `tests/town_junctions.rs` counts 2 forks under 45° and 4 streets that meet a country road 25° to 28° off square.
- **A street that may not meet a road stops a block short of it** (third pass), so a district beside a slanting edge has dead ends, and a grid whose every line is refused becomes an island joined by one link. Streets that end at the last lots by a town's edge read as stubs.
- **A road's own shallow fork is still built round.** Where two country roads or a road and a track meet at a slant inside a settlement (64 places on the 36 test maps), streets and avenues keep clear of the junction, and the roads still fork.
- **A district's ground can still reach past its last buildings** where a block is deeper than its grid's last row; the fitted grid leaves much less of it (second pass).
- **A town centre is terraces only**: no square, no larger commercial footprint. Its main street is the country road or a 10 m avenue.
- **A village's streets are paved.** Only the farm district's are dirt lanes.
- **A run-on is a straight line**, and may cross open ground inside a district. A link is a street of the grid carried on (second pass).
- **Movement through town is not proved.** A force crosses each map, but no test drives a vehicle down a street or walks infantry from a door to it; pavement connectivity and clear ground before each door are what is measured.
- **No rejected-fit layer.** A place where nothing fitted is not recorded.
- **Every building is the prop type `building`** (400 hp, garrisonable), a tower and a shed alike, until C50.
- **Block dimensions remain preset data.** What a district kind needs of its block (`ground_m`) is kept in step with the accepted physical catalogue by hand.
