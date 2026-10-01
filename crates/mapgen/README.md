# Map generation and physical compilation

Three steps, one crate. The **layout generator** turns a seed, a map type and a size
into a `MapPlan`. The **parcel pass** fills that plan's districts with streets, parcels
and buildings chosen from a physical template catalogue. The **compiler** turns any
plan, generated or authored, into the contract's final map. The battle and renderer
consume that compiled geometry; they never reinterpret a plan or look up a template
catalogue. The crate imports no simulation or appearance library; its tests load one
generated map into the simulation's world, to hold a bridge to what a battle needs.

## Layout generation (`layout`)

`generate_layout(&GenerationRequest, &PresetDefinitions)` writes where settlements,
a river, roads with their bridges, and forests go. Its numbers are data: [`fixtures/map-presets.json`](../../fixtures/map-presets.json),
validated at load, whose `revision` a request pins beside `GENERATOR_VERSION`. A
request that names another revision or generator is refused, so an old request
cannot quietly yield a new map. The only constants in code are the three playable
extents, which are a user decision (M04 in the [map brief](../../specs/city-maps/procedural-maps.md)).

- **The same request gives the same plan bytes on every target.** Each consumer draws
  from its own named stream of the seed (`rng`), so a change to one (forests, a
  town's districts) cannot move what another placed. Arithmetic is `+ − × ÷` and
  `libm`, as in `contract::curve`; plan coordinates are whole centimetres.
- **A rule is met because the finished plan measures as meeting it.** `measure` reads
  only the plan's geometry: top/bottom areas, open approaches, the river, and the
  road graph's reach and journey times. That graph is the rounded centrelines the
  surfaces are made of, so two roads are joined exactly when their pavement is, and
  a run of road in the water is driven only where a deck carries it. The generator
  steers toward each rule as it builds, then holds its output to `measure` and
  refuses a plan that fails. Steer by the geometry `measure` reads: a main road is
  timed along its rounded line, which is longer than the runs between its points.
- **The two sides start at the top and the bottom, so those two edges are the ones
  held to a road** (`roads`, M22). A road runs from each to the main junction by
  the map's centre inside the transit time, and so from one to the other. A road
  across the middle from side to side is drawn on a preset share of maps; the rest
  have one side road or none. `measure` reports the two journeys, the bottom-to-top
  journey and whether a side-to-side road exists (`transit`).
- **A search that runs out is a named refusal.** The diagnostic names the feature,
  the preset cell and the seed. The generator never tries another seed and never
  returns a thinner map than the presets describe. Within the one seed, the main
  settlement and the river are drawn again, a bounded number of times, when the
  rest of the map does not fit beside them.
- **A settlement grows from its roads** (`towns`, M23). A site is only the ground a
  settlement may build on. Once the roads are laid, that ground is cut along them,
  each piece is cut into blocks a road's depth deep, a few degrees off square, and
  the settlement is built block by block outward from where its roads meet, farther
  in some directions than in others, until it covers its class's share of the
  ground. A block is one district of one use, bounded by roads, streets and the
  settlement's edge, and blocks take their use a few neighbours at a time. The
  settlement's outline is the edge of its blocks, and the cuts between blocks are
  its avenues: surfaces the layout writes itself. A block is built only where a
  road or an avenue leads to it and one of its class's district kinds has room for
  a parcel on that frontage (`districts.<kind>.ground_m`), so the parcel pass never
  meets a district it cannot build on.
- **A road crosses a settlement's ground in a straight line and turns outside it.**
  That is what lets a road be a block's edge. A main road through a settlement
  enters and leaves by gates past its limit. A settlement off the main roads has a
  main street along its ground, and the road that joins it to the network leaves by
  the street's end unless that would be a sharp turn (`roads.turn_max_deg`).
- **A road ends on another where the two make a plain junction**: on a point where a
  road passes, coming in across it and not alongside, or on a road's end that it
  carries straight on from. Never where three roads already meet, and never at the
  map's edge. A road whose two ends would lie on one road is not laid.
- **Fields and woods reach in beside a settlement.** The blocks it leaves open are
  fields, and a wood is tried on some of them (`forests.infill_chance`): a wood of
  its own shape that keeps the same distance from the districts as any other.
- **Settlements are plan-level.** `MapPlan.settlements` holds each settlement's
  outline, its centre (where its main streets meet) and its districts, nearest the
  centre first, each with a stable id, an area and an anchor point.
  `MapPlan.approaches` holds the measured corridors of open ground: the front's
  width of ground, open for the rule's depth past the last of the settlement's own
  ground along a bearing. Neither reaches the map: the parcel pass turns districts
  into `buildings`, and the encounter planner reads both.
- **A river is a hard feature everything else is placed beside.** A seed-chosen
  share of each type's maps has one river (`rivers`, on a stream of its own, so a
  seed without one is the map it was before rivers existed). It runs from the north
  edge to the south, so each half holds a like length of it. Its course is drawn
  as soon as the main settlement's outline is, and is taken only if the main roads
  can still reach the centre in time by its bridges; the approaches, the other
  settlements and the woods then keep to its banks. Water ends an open approach:
  it is ground no force advances over.
- **A road keeps to its bank and crosses by a bridge** (`crossings`). Where a road
  would wander over the water it follows the bank instead; where its ends lie on
  opposite banks it crosses once, by a bridge already near or a new one, on its
  own line if that is not far askew of the river and square across it otherwise.
  `MapPlan.bridges` are the contract's own `Bridge` rows, each long enough to be
  stepped onto from dry land (`water` asks the contract's river distance).

What the presets mean, and why they hold the values they do, is in the
[C52 slice](../../specs/city-maps/slices/C52-procedural-generator.md) and its
entries in the [choices ledger](../../specs/city-maps/choices.md).

## Road ends (`joints`)

A stroke is cut square across its first and last point (`contract::ground`), so
the flat face of a road's end shows wherever no other paving covers it. Both
steps hand their carriageways to one pass, `joints::close`: the layout its roads
and avenues, the parcel pass those again with its streets, before any parcel is
cut along them. After it, a road's end is one of these, and
`tests/road_ends.rs` holds the finished plans to that over every type and size:

- **Part of a through road.** Carriageways of one kind and width that meet end to
  end become one stroke through the point they shared, and the bend there is the
  centreline's own rounded one. Two that meet alone are one road round whatever
  corner they make (a country road or a track up to 110°, a street any). At a
  junction of three or more, only ends that carry nearly straight on are joined.
  Two ends that stop within each other's width on lines that cross there are
  first brought to that crossing.
- **Under the road it joins.** An end that touches another carriageway stops a
  quarter of a metre past that carriageway's rounded middle, where its face lies
  inside the other's width.
- **The outer edge of a corner.** Where unlike roads meet at a corner the wider
  one runs on until the narrower one leaves through its side, and covers the
  narrower one's end. A narrower road that carries nearly straight on runs back
  into the wider one instead, and the wider one's end shows a shoulder either
  side: the road narrows. A wider road that comes onto a narrower one at a slant
  crosses it whole. Two roads of one width that fork too sharply to be one road
  are closed like unlike ones: the first runs on over the second one's end.
- **Square on the map's edge.** A road that leaves the map at a slant turns
  square to the edge over its last two widths, so its end lies along the edge.
- **At the last block it serves.** A road that runs out past the last block on
  its line and stops in open country is cut back to that block, or to the last
  road that joins it.

The plan's road graph joins two roads where their centrelines cross, so the pass
never leaves an end touching a line that rounding has moved off it: ends that
share a point keep it, or all run on past it far enough to cross. It checks
itself the same way. Whatever two roads' centrelines crossed before, they cross
after or are one road; where a change would break that, the roads concerned are
left exactly as they were laid and the rest are closed round them.

Not closed yet, and counted by the test (1.1 in a thousand ends): three or more
roads of one width that meet at sharp angles or with their ends a few metres
apart, and two streets laid side by side where one stops.

## Parcels and buildings (`parcels`)

`fill_districts(plan, &request, &catalogue, &presets)` takes a layout and returns the
same plan with its towns built: streets and paved aprons added to `surfaces`, the
parcels it cut in `lots` (plan-only), and one `buildings` row per placed template.
A district kind's streets and setbacks are rows of the same presets file.

- **A parcel is cut to its template, never the reverse.** A parcel is the template's
  own footprint plus the district's setbacks, placed by translation and rotation so
  that the template's entrances face the street. Where no eligible template fits, the
  ground stays open. A template is eligible when its descriptor is complete (floors,
  entrances, bays), its category is one of the district's, it is within the map
  type's floor limit and it belongs to the one regional family the seed drew for the
  map.
- **Parcels front every carriageway, not only the streets the pass lays.** The
  country road or track through a village is its main street, and a district's edge
  is a road or an avenue as often as not: a carriageway within its own half width of
  the edge fronts the district.
- **Every street is joined to the settlement's road.** A district's streets are a
  grid along one of its edges: the edge that runs longest with the settlement's
  main street, so neighbouring grids run the same way and meet on the avenue
  between them. A district kind's streets are
  paved, or dirt lanes (`streets.surface`). A piece of that grid no road crosses
  gets one link to the nearest street, so no pavement is stranded, and a street that
  stops within a block of another carriageway runs on to it. No street crosses
  water: the nearest street is the nearest it can reach on its own bank. `measure`
  then confirms on the finished plan that every street has a way to the centre. A
  street that stops facing another street's end, within a block, runs to that
  end and not past it: the two are one street, not two laid side by side.
- **Ids are derived, not counted across the map.** A parcel is
  `<district id>/lot-<n>` and the building on it has the same id, so tuning one
  district kind renames nothing elsewhere. Building parts take the plan's prop ids
  in order.
- **An empty district is a refusal**, like any other search that runs out: the
  diagnostic names the district and its preset row. A district's first parcel is
  always built on; the kind's `coverage` applies to the rest.

The catalogue is a list of physical descriptors. [`fixtures/prototype-building-templates.json`](../../fixtures/prototype-building-templates.json)
is the one the generator is proved with: placeholder boxes at believable metre scale
in the six categories, all of the regional family `prototype`. Real sources replace
it through the same descriptor contract and a new catalogue hash. The measured
outcome and the open questions are in the
[C53 slice](../../specs/city-maps/slices/C53-parcels-and-buildings.md).

## Compilation

Generation configuration pins the lossless map seed, generator/preset versions,
canonical plan and physical catalogue. Execution limits are separate preparation
policy. The map's content hash identifies physical output independently of its
execution allowance or presentation assets.

The shared identity owner lives in `contract::identity`. A seed is canonical decimal
text at JSON boundaries. Typed field order defines canonical bytes; authored sequence
order is meaningful because implicit IDs occupy the remaining dense namespace. Input
whitespace and object-key order do not change identity. Version labels are supplied
by preparation callers, so this is content identity rather than verified source provenance.

The compiler admits physical buildings, ordinary authored bodies, and ground: roads,
tracks, sidewalks, forests, rivers and bridges in the contract's shared shapes, which
pass into the map unchanged. Explicit body IDs enter the contract-owned dense namespace. A ground
shape's authored points must lie inside the playable rectangle (a stroke or a river may
overhang the edge by its width). A river the terrain cannot carry, and a bridge whose end stands over
water or too near it for a ramp, is refused as `invalid_river`, by the one rule the
world loads maps with (`contract::river::validate`); a deck that is not a finite box
inside the playable rectangle is `invalid_bounds`. A river's rounded samples count
toward `max_ground_points` with the strokes'. Land
regions and source/art fit remain prerequisites for their compiler arms; a requested
unsupported feature produces a named error rather than disappearing from output.

`MapPlan.size` is the playable rectangle. Admission follows the architecture envelope
in the city-map scale policy; it defines no rendered surroundings.
Physical box bounds use the template contract's numeric evaluator for admission.
Simulation collision arithmetic and its remaining cross-runtime proof are independent.

## Boundaries

The library's complete outcome is shared by the CLI (`mapgen`, whose usage text lists
its commands) and WASM. A refusal returns diagnostics and no plan or map. Successful
file preparation writes the final map and the shared `MapSources` envelope. Supplied
request, preset and catalogue receipts hash exactly the bytes used and assert no Git
history; generation identity and stdout outcomes are unchanged. Acquisition and
catalogue publication belong to C09/C60. The required execution limits cover authored
parts, emitted bay positions and ground points (polygon vertices plus rounded stroke
samples) before materialization. They do not claim a bound on all input bytes, terrain,
navigation, runtime trees or the complete battle's memory.

`mapgen inspect` draws a plan's layers at metre scale for review, with the numbers
`measure` reports: the whole map, or one settlement, district or bridge close enough
to read its parcels, entrances and the roads onto a deck. `mapgen catalogue` prints a descriptor list's canonical
form, whose hash a request pins and the map loader resolves against. The
`layout_sweep` example runs every type and size over a range of seeds and reports
the layout, what was built on it and what the compiled map costs; it is how a preset
change is judged.
