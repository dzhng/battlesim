# Map generation and physical compilation

Four steps, one crate. The **layout generator** turns a seed, a map type and a size
into a `MapPlan`. The **parcel pass** fills that plan's districts with streets, parcels
and buildings chosen from a physical template catalogue. The **open-country pass**
furnishes the ground between the settlements and the woods, and the **street furniture
pass** stands parked cars, lamps, trees and yard stock among the buildings, as prop
types of the unit and prop catalog. The **compiler** turns any plan, generated or
authored, into the contract's final map. The battle and renderer
consume that compiled geometry; they never reinterpret a plan or look up a template
catalogue. The production library imports no simulation or appearance library.
Verification tools and tests load compiled maps into the simulation to check what
a battle needs.

## Physical and visual bounds

The selected map size is playable ground. Generated sizes align with the physical
height grid; authored sizes can round at its last cell. The map contract derives
that existing physical boundary from the same sample dimensions the simulation
uses. Its `render_margin_m` describes only the surrounding landscape, and `extents`
derives the rendered rectangle around both playable and physical ground.
The compiler preserves this field, including in compact saved maps. Preparation
and the page's world export publish the same derived bounds; neither creates a
second world or enlarges navigation, foliage, deployment or picking.

The renderer reuses its backdrop terrain, field plots and scenery placement for
the visual margin. An authored arena without a margin retains its existing display
environment: background land and distant scenery are presentation, outside the
map-owned rectangles. Resource measurements include that environment too. A visual
margin changes map/replay input identity, even though it adds no combat cover or
movement obstacle.

## Layout generation (`layout`)

`generate_layout(&GenerationRequest, &PresetDefinitions)` writes where settlements,
a river, roads with their bridges, and forests go. Its numbers are data: [`fixtures/map-presets.json`](../../fixtures/map-presets.json),
validated at load, whose `revision` a request pins beside `GENERATOR_VERSION`. A
request that names another revision or generator is refused, so an old request
cannot quietly yield a new map. Most construction numbers belong to those presets.
Source constants define numeric geometry cadence and bounded proof work; their owners document the
contracts they protect. The three playable extents are a user decision
(recorded in the [map rationale](../../specs/done/city-maps/README.md)).

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
  the map's centre inside the transit time, and so from one to the other. The two
  meet at one place: the later of them ends where the earlier did, at the main
  junction or across the arm the earlier joined, so the road from the bottom edge
  to the top passes through one junction. No two junctions of the edge roads lie
  nearer each other than `roads.junction_apart_m`. A road across the middle from
  side to side is drawn on a preset share of maps; the rest have one side road or
  none. `measure` reports the two journeys, the bottom-to-top journey and whether
  a side-to-side road exists (`transit`).
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
  meets a district it cannot build on. A cut that would end on a road or an
  earlier cut within `towns.align_m` of where one already ends on it from the
  far side, in line with it, is moved to end there, so the avenues either side
  of a road make a crossroads. An avenue runs the whole edge of a block that
  shares any of it with another block.
- **A junction is a T or a crossroads, and the roads' junctions are their own.**
  A cut between blocks ends on another no nearer than `towns.align_m` to a
  junction it makes no crossroads of, and no nearer than a block
  (`towns.junction_clear_m`, or half a block's depth where blocks are small) to
  where one of the map's roads ends on or crosses another: it is tried at the
  places a block may be cut at, nearest the one drawn first. An avenue that
  would meet a road more than 22° off square stops at the last corner of a
  block a row of lots before it, and the blocks between front the road. An
  avenue that starts on a road's end and runs on along its line is that road
  carried on, at its kind and width, to the next junction. A settlement whose
  ground cut this way holds no block a parcel fits on (a hamlet round a fork)
  is cut again as drawn.
- **A large town and a city have more than one road** (`classes.<class>.side_roads`).
  Once a settlement of such a class has its main road, secondary roads are laid:
  each leaves one of the roads through the centre part of the way out, at a
  junction of its own clear of every other road's end, and turns into the widest
  sector no road runs out through yet. It runs beside the next road round that
  sector where that takes it off its own road within the row's angles, and square
  off its own road otherwise, straight to a gate past the settlement's ground,
  where a later road may carry on from it. So a settlement's roads run side by
  side or meet near square, never at a slant: the ground is cut along each, rows
  of blocks lie along both sides of it, and the streets between two of them meet
  both square. A secondary road is not laid where it would cross a road at less
  than 45°. Growth follows the roads, so the outline has arms along them and
  bays between. They draw from a stream of their own. No other class has the row.
- **A town has air in it.** A class's `parks` are blocks near its centre that it
  built round and then left open: the smallest of the `towns.park_reach_blocks`
  built blocks nearest the centre that has built blocks on every side and room
  for trees. A park has an avenue all round it and a wood of its own shape,
  drawn in from the streets by the woods' usual distance from districts. A
  class's `open_blocks` is the chance a block away from the centre is never
  built, and its built share falls by as much. A park that ends up open to the
  fields is ground like any other open block.
- **A road crosses a settlement's ground in a straight line and turns outside it.**
  That is what lets a road be a block's edge. A main road through a settlement
  enters and leaves by gates past its limit. A settlement off the main roads has a
  main street along its ground, and the road that joins it to the network leaves by
  the street's end unless that would be a sharp turn (`roads.turn_max_deg`).
- **A road ends on another where the two make a plain junction**: on a point where a
  road passes, coming in across it and not alongside, or on the end of a road of its
  own kind that it carries straight on from. Never where three roads already meet,
  never at the map's edge, and never on the end of a road of another kind: a track
  turns off a country road, it does not carry on from where the paving stops. A road
  whose two ends would lie on one road is not laid, and neither is a link that has
  nowhere on a settlement's roads to join.
- **Fields and woods reach in beside a settlement.** The blocks it leaves open are
  fields, and a wood is tried on some of them (`forests.infill_chance`): a wood of
  its own shape that keeps the same distance from the districts as any other.
- **Settlements are plan-level.** `MapPlan.settlements` holds each settlement's
  outline, its centre (where its main streets meet) and its districts, nearest the
  centre first, each with a stable id, an area and an anchor point.
  `MapPlan.approaches` holds the measured corridors of open ground: the front's
  width of ground, open for the rule's depth past the last of the settlement's own
  ground along a bearing. Neither reaches the map: the parcel pass turns districts
  into `buildings`. The encounter planner (`sim::encounter`) reads both as
  `contract::encounter::EncounterSites`, which `MapPlan::sites` makes: generation's
  outcome carries the sites beside the map, and `generate-map` saves them as
  `sites.json`, beside the map in its saved form
  ([`fixtures/README.md`](../../fixtures/README.md#saved-maps)).
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
[generation report](../../specs/done/city-maps/generation-report.md) and
[choices ledger](../../specs/done/city-maps/choices.md).

## Road ends (`joints`)

A stroke is cut square across its first and last point (`contract::ground`), so
the flat face of a road's end shows wherever no other paving covers it. Both
steps hand their carriageways to one pass, `joints::close`: the layout its roads
and avenues, the parcel pass those again with its streets, before any parcel is
cut along them. After it, a road's end is one of these, and
`tests/road_ends.rs` holds the finished plans to that over every type and size:

- **Part of a through road.** Carriageways of one kind and width that meet end to
  end become one stroke through the point they shared, round whatever corner
  they make there short of an exact reversal. The outside of that bend, a
  switchback included, is the centreline's own round one: a sharp joint is one
  road round a bend, never two flat ends. At a junction of three or more the
  straightest pair is the road through it. Two ends that stop within each
  other's width (or a width or two past each other) on lines that cross there
  are first brought to that crossing.
- **Under the road it joins, and square to it.** An end that touches another
  carriageway stops a quarter of a metre past that carriageway's rounded middle,
  where its face lies inside the other's width. A branch that comes in more than
  30° off square leaves its line a width or two before the road and curves round
  to meet it square, so no acute fork is left and no corner of its end shows
  past the road's far edge. One within 20° of running alongside is a lane
  peeling off, and keeps its line. An end that stops a width or two past a road
  it crossed is cut back to it.
- **Under a road that crosses it, where it changes width.** A road changes width,
  and kind, only where another road crosses it: it is one road from junction to
  junction. Where a wider way and a narrower one meet end to end (an avenue and a
  street at a block's corner, a country road and the street that carries on from
  it), one of them is carried through the corner along the other's line, round its
  own bend, to the first carriageway that crosses that line and covers the wider
  way's whole flat end. The wider way ends there, just past the crossing road's
  middle, and the narrower one starts on the same point. A side road that only
  joins the line covers one shoulder, so the change is not made at it. Of the two
  stretches (the narrower way's, on from the corner, and the wider way's, back from
  it) the shorter is the one that changes; where nothing crosses it so, it changes
  all the way to its far end, which is a junction, the map's edge or a dead end.
  Where two narrower ways leave a corner, the one that carries on straighter is
  the one considered, and the other joins the road as it would anywhere along it.
- **Square on the map's edge.** A road that leaves the map at a slant turns
  square to the edge over its last two widths, so its end lies along the edge.
- **At the last block it serves.** A road that runs out past the last block on
  its line and stops in open country is cut back to that block, or to the last
  road that joins it.

Before joints are made the pass tidies what would block them: a street drawn
beside another for a few metres is cut back to the road that crosses it (or the
two become one), and a turn within a few metres of an end is dropped, so an
end's last run is long enough to move or to turn along.

The plan's road graph joins two roads where their centrelines cross, so the pass
never leaves an end touching a line that rounding has moved off it: ends that
share a point keep it, or run on past the other's middle far enough to cross.
It checks itself the same way. Whatever two roads' centrelines crossed before,
they cross after, are one road, or both cross a third road at the same junction;
where a change would break that, the roads concerned are left exactly as they
were laid and the rest are closed round them.

Not closed yet, and counted by the test (under 3 in ten thousand ends are a bite, a
heel or a wider road's shoulders showing): junctions where three ends stand a few
metres apart without sharing a point, a wider road that ends at a slant on a
narrower one with no room to turn, and a corner of unlike roads left as laid
because mending it would have lost a joint.

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
- **A street runs from a junction to a junction, or ends at the last lot it
  serves.** A district's streets are a grid fitted between its edges, along the
  edge that most of the carriageways round it run with or square to, a road's
  counting double: so a district beside a road has its streets beside that
  road and square to it. Between edges that serve alike it is the one that
  runs longest with the settlement's main street, so neighbouring grids run
  the same way. Where the preset has a `bend`, each street family bows once or
  less along the district, by an amount that changes from one side of the
  district to the other (and is nothing somewhere between): no two streets
  bend alike, and none ripples. A whole block lies between a carriageway on an edge
  and the first street, and half a block between an edge that faces the fields
  and the last one, so no street lies a few metres inside a road and no back
  land is left behind the last row of lots. A district kind's streets are
  paved, or dirt lanes (`streets.surface`). `tests/street_warts.rs` counts what
  breaks these rules over every type and size.
  - **Each line of the grid runs to the carriageway ahead of it** (within
    `parcels.run_on_m` past the district's edge), and stops just past its
    middle. Where a street already meets that carriageway from the far side
    within `towns.align_m`, in line with it and with no other junction beside
    it, the line is moved to meet it there: one crossroads, not two junctions
    a few metres apart. A street as wide ends on that street's own end and
    the joint pass makes them one street through the junction; one of another
    width lands opposite it.
  - **A street meets the road it joins square, or does not meet it.** One that
    would come to a carriageway up to 20° off square lands on its own line.
    Up to 28° it turns at its last crossing and runs square to the
    carriageway, a bend three widths or more from it. At more of a slant it
    does not land. Nor does it land within a few widths of a junction it
    makes no crossroads of, or within `parcels.junction_clear_m` of a junction
    of the layout's roads: a junction has four arms at most, and a road
    leaves another at a junction of its own. A street that may not land ends
    at its last crossing with its own grid, or, where none is near, a row of
    lots short of the carriageway; the ground between fronts that carriageway.
    A street's own run-on may be any length; a street that turns off another
    is at least `parcels.street_step_m` long, and one that is a single run
    from a crossing to the road ahead at least four widths, so no link is a
    stub.
  - **Where nothing lies ahead** a long street stops a verge inside the
    district's edge, at the last lots, and a cross street stops on the last
    long street. Two streets of one grid never come to a road within three
    widths of each other.
  - **A carriageway joins another only of its own width end to end.** A street
    that comes to the open end of a wider avenue meets it just short of that
    end, as a side street. A road changes width where another crosses it
    ([Road ends](#road-ends-joints)).
  - A piece of the grid no road crosses gets one link: a street of the grid
    carried on to the first carriageway ahead, or to a street of its own grid
    already joined. No street crosses water. `measure` then confirms on the
    finished plan that every street has a way to the centre.
  - After every district is laid, a street or avenue that still stops in the
    open with a carriageway within `parcels.run_on_m` ahead runs on to it where
    it may land on it; where another street's end faces it, to that end. One
    that runs on less than `parcels.tail_min_m` past its last junction and
    stops in the open is cut back to that junction, and one that stops within
    `parcels.run_on_m` of a carriageway it may not land on is cut back until a
    row of lots lies between. Where that would leave a district no parcel (a
    hamlet's one lane), the layout's avenues keep their length.
  - `tests/town_junctions.rs` counts, over every type and size, the places
    where five carriageways meet, where two leave one place less than 45°
    apart, where a town's street meets a country road more than 25° off
    square, and where one turns sharply just before it.
- **Ids are derived, not counted across the map.** A parcel is
  `<district id>/lot-<n>` and the building on it has the same id, so tuning one
  district kind renames nothing elsewhere. Building parts take the plan's prop ids
  in order.
- **An empty district is a refusal**, like any other search that runs out: the
  diagnostic names the district and its preset row. A district's first parcel is
  always built on; the kind's `coverage` applies to the rest.

The catalogue is a list of physical descriptors. [`fixtures/prototype-building-templates.json`](../../fixtures/prototype-building-templates.json)
is the one the generator builds towns from: complete buildings in the six
categories, each row the descriptor of the city art set that dresses it
([scene-assets](../../packages/scene-assets/README.md), "City buildings"), so it
is derived and never edited by hand. Another regional family arrives through the
same descriptor contract and a new catalogue hash. The measured
outcome and retained limits are in the
[completion evidence](../../specs/done/city-maps/evidence.md).

## Open country (`open_country`)

Complete generation furnishes the country with homes along roads, short tree lines,
copses, single trees and low cover, then admits physical ground-sight coverage after
street furniture and every authored building part are final. The density pass is an
internal construction stage; callers use `generate_plan` or `generate_map` so they
cannot mistake that stage for the finished map.

Physical range has one owner: resolved generation physics. Coverage selects actual
circular ground observers from the unit catalog, excluding aircraft and directional
lobes, and uses their ground-standing eye heights and the fog target height. The
smallest current circle is the jeep's 450 m. Elevated garrison observers, aircraft,
future destruction and a side's combined multi-eye opening field are separate
contracts. Hills remain deferred within city-maps; this certificate supports flat
land, carved rivers and bridge heights, and refuses unsupported height profiles.

Construction densities, sizes and clearances belong to the presets. The coverage
policy supplies a cell size and an ordered list of allowed copse/tree-line proposals;
it has no independently tuned sight range. The existing density estimate remains a
construction heuristic and supplies no witness.

The certificate reads the contract's **same seeded trunk candidates and exclusion
predicates** as the world, using indexed complete authored bodies, roads, water and
closed sampled-ground bounds. An authored forest outline does not count as a tree.
All bodies are final before witnesses are collected; later furnishing adds only
forests. World placement ignores earlier trunks when excluding new trunks, so forest
order and index still supply the same seed without a second placement algorithm.

A witness is a fully occupied 3×3 native fog-cell patch. Occupancy comes from accepted
trunks' actual canopy coverage or tall opaque authored bodies on dry supporting
ground. Its centre also earns a physical interior margin against the shared foliage
integration step or body footprint. Each clipped coverage rectangle, including town
interiors, water and map edges, must have a witness whose distance from **every
rectangle corner** fits the earned reach. Distance is convex, so its maximum over
the whole rectangle lies at a corner: this admits every continuous position without
inventing an extra margin around the cell centre. Testing water too is deliberately
stronger than the playable-ground requirement.

For an in-bounds circular reveal, the 3×3 patch's inscribed disk catches the nearest
native ray and both neighbours, including radial sampling error. The earned reach
also stops these rays before any sample can enter the chosen far cell. Every other
ray stays more than a cell diagonal from that far sample, so it cannot refill the
same published bit. Ray count, floored far range and post-foliage reach are checked
for **each** circular ground observer, since flooring is not monotonic across range
multiples. Unsupported sampling, attenuation, height or angular margins refuse the
request.

The witness bearing over the complete clipped rectangle also earns an in-bounds
far-cell margin for every circular ground observer. Its coordinate intervals include
every observer position, and reserve the native ray's angular error and a whole fog
cell diagonal. An outward-only obstruction at an edge cannot satisfy this rule.
Together these margins prove nearby physical sight-interrupting surroundings and
an in-bounds missing published far bit at every covered ground location. House variety, visual interest, normal-camera readability,
most bearings remaining open and combined opening fog still require their own
reports and rendered review. No per-cell scene or ray queries are hidden in admission.

A missing cell searches near-first at the native fog cadence, using the existing
copse and tree-line geometry and a named random stream. A failed proposal rolls back
its authored forest, exclusions, tally and raster contribution. Unbuilt settlement
outline ground may supply a feature only where actual bodies and roads leave room;
the ordinary home/density pass keeps its settlement separation policy. The main
settlement's measured 1,800×400 m firing corridors remain protected, including both
sampled middle bearings, the actual middle and existing placement clearance. The
wider layout reservation is construction headroom. Minor corridors may give way;
all advertised approaches are remeasured afterwards. Extra features restore the
existing per-kind half fairness. A balancing feature must contain accepted physical
trunks, but need not earn another dense fog patch: the complete map already has its
coverage certificate, and a thin tree line remains real scenery. Both uses share the
same staging and rollback owner; fairness and mandatory-lane failures also refuse.

**Proof work has one operation envelope**, independent of authored compile limits.
It forecasts raster/index and fallback-list dimensions before allocation and candidate
lattice work before sampling; edited pathological spacing or resolution receives a
named `ground_sight` refusal. The shared sampler can stop an exhausted proof prefix;
the world always finishes its original sequence. The 512-million operation envelope
charges source-edge tests, bucket references and reads, actual geometry predicates,
foliage/raster reads and placement exclusions, and stops exhausted work. These are
charged structural proof units, not every CPU operation or retired instructions.
Grid-directory initialization and fallback-list/sort dimensions are separately
forecast by admission and are not in the running charged count. This is not a bound
on the full battle's memory, simulation, renderer or runtime forest floor.

The coverage owner's synthetic fully forested architecture-extent test stresses the
raster and the shared trunk sampler without allocating runtime tree bodies. It
asserts that the certificate finishes within its proof envelope. This is a proof
stress case; it does not admit the full architecture's physical world, startup,
renderer or memory. Actual generated-map costs remain measured by their own
harnesses.

The density pass still uses real catalogue homes and their doors, yard lanes that
join existing carriageways, and broken tree-line stretches with vehicle gaps. The
`sight_report` furnished arm and the default `layout_sweep` run complete production
generation; the bare sight arm is explicitly before country/furniture/coverage.
Finite infantry samples and directional lobes remain diagnostics, not the whole-map
certificate. A seed refusal is retained rather than replaced by another seed.

## Street furniture (`street_props`)

`place_street_props(&plan, &request, &templates, &catalog, &presets)` answers the bodies
to add to a built plan's `props`: ordinary map props (a catalog prop type, a centre, a
heading and a box), with no ids, so they take the ids after the buildings' parts. What
is placed, how big each body is and how much of it each district kind gets are rows of
the presets: `street_props` (each body's box and the room it keeps, the parking rule,
the construction site) and each district's `props` (its share of kerb parked along, its
verge rows, its yard stock, its chance of a site). The catalog supplies two things: every
placed kind must be one of its prop types, and its widest hull sizes the lane.

- **One legality check for every body.** A body stands on the map, off every
  carriageway, no nearer a carriageway's middle than the lane (`lane_margin_m` past the
  widest hull), clear of buildings by a soldier's width, out of each door's way to the
  street, apart from the bodies already placed by the room each keeps, off the water, off
  every bridge deck and the run onto it, and out of every measured open approach. A kind
  is its rows; no rule names one.
- **The lane is the simulation's.** A vehicle normally keeps right of a road's middle.
  Its navigation owner refines coarse false blockage against actual known bodies and
  tries clear lines before pushing. Furniture can therefore stand at the kerb while
  leaving the widest hull a usable street. `tests/street_props.rs` asks the simulation's
  own navigation on bare and dressed maps: the assault still plans, a squad still
  stands at every door and walks to it, and every hull still drives every street,
  the widest without shoving a body. Actual kerbside traversal and traffic yielding
  are separate movement proofs; a placement or planned route alone is insufficient.
- **Cars park in runs,** bumper to bumper with no way through, a squad's width or more
  between runs, along one side of a street and both of an avenue. Lamps and street
  trees (`street_tree`: the forests' trunk as a body, with its own binding) are evenly
  spaced and placed first, so a run ends at one; the scattered rows
  fill what the cars left. Nothing beside a street stands within `corner_clear_m` of
  another carriageway's edge, so a junction's corners are open.
- **Yard stock stands on its building's own parcel,** against a wall with no door in it.
  **A construction site** takes a parcel the parcel pass left open: a cabin, a fence
  round it with a gate on the street, a skip and pallets. The gate's way to the street
  is kept open like a door's.
- **Bounded and stable.** A body that has no legal ground where its row puts it slides
  along its street by `slide_m` at most, or tries `attempts` places in a yard, and is
  otherwise left out. Ways, sides and districts are walked in the plan's order, and
  each (way, side, kind of row) draws from its own stream.

Not placed: roadblocks, wrecks and road barriers. A roadblock is something a defender
prepares, so it belongs to the encounter planner.

## Compilation

Generation configuration pins the lossless map seed, generator/preset versions,
canonical plan, physical template catalogue and physical battle inputs. Generated maps
pin the contract-owned `GenerationPhysics` extracted from the resolved rules: the
unit and prop catalog, forest rules, infantry eye and fog target heights. The native
CLI takes the existing rules and catalog files explicitly; WASM takes the resolved
rules record the preparation worker also gives the battle. There is no second
physical-settings file. The profile conservatively pins the whole resolved catalog,
including its names, presentation bindings and combat columns; unrelated top-level
weapon and balance rules stay outside it. The geometry hash remains independent
of these input columns. Direct authored-plan compilation has no rules input.

Execution limits are separate preparation
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
toward `max_ground_points` with the strokes'. A requested unsupported feature, including a land-region plan field, produces a
named error rather than disappearing from output. Visual yard/field composition
belongs to the renderer’s plot owner, using compiled buildings and roads; source/art
fit belongs to the asset release gate.

`MapPlan.size` is the playable rectangle. Admission follows the architecture envelope
in the city-map scale policy. `render_margin_m` extends presentation around that
rectangle through the shared extents contract; it adds no physical ground.
Physical box bounds use the template contract's numeric evaluator for admission.
Simulation collision arithmetic and its remaining cross-runtime proof are independent.

## Boundaries

The library's complete outcome is shared by the CLI (`mapgen`, whose usage text lists
its commands) and WASM. A refusal returns diagnostics and no plan or map. Successful
file preparation writes the final map, its sites and the shared `MapSources` envelope. Supplied
request, preset, template catalogue, rules and unit/prop catalog receipts hash exactly
the input bytes and assert no Git history. Generation configuration identity includes
the canonical physical input hash even when a rule change leaves map geometry alone. Acquisition and
catalogue publication belong to C09/C60. The required execution limits cover authored
parts, emitted bay positions and ground points (polygon vertices plus rounded stroke
samples) before materialization. They do not claim a bound on all input bytes, terrain,
navigation, runtime trees or the complete battle's memory.

`mapgen inspect` draws a plan's layers at metre scale for review, with the numbers
`measure` reports: the whole map, or one settlement, district or bridge close enough
to read its parcels, entrances, street furniture and the roads onto a deck. `mapgen catalogue` prints a descriptor list's canonical
form, whose hash a request pins and the map loader resolves against. The
`layout_sweep` example runs every type and size over a range of seeds and reports
the layout, what was built on it, what the open country was furnished with, the
street furniture among the buildings and what the compiled map costs; it is how a
preset change is judged.

The [`battle_sweep` example](examples/battle_sweep.rs) follows generation through
assault placement and a short battle, using the same documents and admission as
the game. Every requested seed retains its outcome, including refusals; no seed
stands in for another. Per-unit progress follows the acknowledged destination,
which can differ from the group's clicked point. Goal proximity and time in each
movement state are separate observations: a short run without arrival does not
prove a long route is stuck. Costs exclude progress/report formatting; native tick
instruction brackets include counter and timing overhead. The usage lives with
the example.
