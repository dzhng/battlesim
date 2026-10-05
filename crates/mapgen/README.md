# Map generation and physical compilation

One crate turns a seeded request into a plan and compiles generated or authored
plans into the shared physical map. Battles and renderers consume that compiled
geometry; they never reinterpret the plan or look up its physical template library.
[The library boundary](src/lib.rs) owns the production sequence. It imports no
simulation or appearance library; developer reports may use the simulation to
prove the resulting map supports a battle.

## Physical and visual bounds

The selected size is playable ground. The [map contract](../contract/src/map.rs)
derives the physical boundary from the height grid and a separate rendered
rectangle from the visual margin. The compiler preserves those inputs. Preparation,
picking, navigation and rendering must agree on their respective bounds without
turning background scenery into physical cover or movement ground.

A visual margin changes map/replay input identity even though it adds no combat
bodies. Resource measurements include it and the renderer's display environment;
a smaller playable rectangle cannot stand in for the complete rendered workload.

## Layout generation (`layout`)

[Layout](src/layout/) places settlements, rivers, roads, bridges and forests.
[Presets](../../fixtures/map-presets.json) own construction choices and finite-work
policy; [preset validation](src/layout/presets.rs) protects the physical and numeric
constraints. The size profile owns its central settlement class and surrounding settlement
counts. Open-approach depth follows the playable extent, so compact maps retain
room for the town and both opposing approaches. The shared
[generation contract](../contract/src/generation.rs) owns tier names and extents.
A request pins preset and generator revisions. A mismatch is refused
rather than quietly producing a new battlefield.

Named random streams isolate consumers: changing forest construction should not
move every road. Stable arithmetic, quantization and iteration order preserve
native/WebAssembly parity. [Measurement](src/layout/measure.rs) reads finished
geometry, not construction intentions. A search may steer toward a rule, but only
the completed plan can demonstrate that it meets the rule. Exhaustion retains the
requested seed and a named refusal; it cannot substitute a thinner map or another seed.

Settlements grow from their roads and built blocks, rather than stamping a town
outline over unrelated streets. Frontage must have room for an eligible parcel.
Secondary roads create distinct arms, while parks and open blocks give the town
air and let country reach between those arms. Junctions must read as ordinary
crossroads or T junctions, with room between unrelated entrances to a road.
[Town construction](src/layout/towns/) owns the detailed geometry and thresholds.

The opposing approach edges need connected road access to the center. A road graph
uses the same rounded centerlines as the paving, so a nominal join or straight-line
travel estimate cannot prove an actual usable connection. Roads cross settlement
ground straight and turn outside it so their frontage remains buildable.

Water is a hard layout feature. Roads remain on their bank except at admitted
[bridge crossings](src/layout/crossings.rs), and water stops an advertised open
approach. Rivers and terrain use the shared [river contract](../contract/src/river.rs),
so generator legality cannot differ from world admission.

Settlements, lots and measured approaches are plan evidence, not extra runtime
geometry. Their [encounter sites](../contract/src/encounter.rs) accompany the compiled
map so the simulation's planner can place forces on it. [Saved-map policy](../../fixtures/README.md#saved-maps)
explains the retained documents.

## Road ends (`joints`)

A stroke's flat end is visible wherever no other paving covers it.
[Joint closure](src/joints.rs) has one owner for layout roads and parcel streets.
It joins compatible through roads, covers branch ends with their receiving road,
places width changes at crossings and closes map-edge or dead-end frontage.
Those choices keep a road from appearing cut off, sprouting an acute fork or
leaving a wider shoulder exposed.

Closure must preserve the road graph. Rounding can move a line away from a shared
endpoint, so pavement touching is insufficient: the measured centerlines still
need a connection. A local cosmetic repair that would lose a joint is left alone.
[Road-end tests](tests/road_ends.rs) measure remaining defects rather than claiming
that every awkward multi-road junction has been solved. Numeric allowances and
current measured residuals belong to those tests and the retained reports.

## Parcels and buildings (`parcels`)

[Parcel construction](src/parcels/) fits lots to physical templates; it never
stretches templates to fill leftover land. Entrances face frontage, eligible
regional/category choices come from the descriptor library, and ground remains
open where no legal template fits. Every carriageway may supply frontage, including
a rural road running through a settlement.

Streets run between real junctions or end at their last served lots. They follow
surrounding roads instead of making parallel slivers, crowded forks or disconnected
grids. The finished measurement confirms connection to the town center. Parcel
identity derives from its district so tuning one kind does not rename unrelated
buildings. A district with no valid first parcel is a named refusal rather than an
unexplained empty town.

A dense district's block is structured, not one plaza ([courts](src/parcels/courts.rs)):
yards on the built parcels, car parks beside the carriageways, and the rest lawn,
crossed by paths and one lane a vehicle drives. Lawn is the absence of paving, not
a surface kind; paving is ground, not road: it changes no speed, sight or cover.

[The generated template library](../../fixtures/prototype-building-templates.json)
is derived from accepted [city sets](../../packages/scene-assets/README.md#city-buildings),
not edited independently. Art dresses physical descriptors; a new regional family
enters through that same contract and changes the library identity. [Completion
evidence](../../specs/done/city-maps/evidence.md) records accepted scope and measurements.

## Open country (`open_country`)

[Country construction](src/open_country/) furnishes ground between settlements,
then proves physical sight interruption after authored bodies and street furniture
are final. Callers use the complete library sequence; an internal density pass is
not an admitted map. Construction density is a heuristic and supplies no witness.

[Coverage](src/open_country/coverage.rs) derives reach from resolved generation
physics, using actual circular ground observers and native fog sampling. It has no
independent sight-range knob. Elevated garrisons, aircraft, directional lobes,
future destruction and a side's combined opening view are different contracts.
Unsupported height, sampling or attenuation assumptions produce a refusal.

The certificate uses the contract's same seeded trunk candidates and exclusions
as world construction. A forest outline alone is not a tree. Witness occupancy
comes from actual accepted canopy coverage or opaque physical bodies, with dry
support and an interior margin. All authored bodies are final before collection;
new furnishing adds forests under that shared sampler rather than a second tree
placement algorithm.

A witness must cover every position in a clipped location rectangle, including
edges and settlement interiors. Bounding every corner earns the maximum distance
over that rectangle; testing only its center would leave gaps. Native angular and
radial sampling margins must also leave an isolated missing far fog cell inside
the map for every supported observer. A nearby feature outside the edge cannot
prove interruption of an entirely open in-map circle. These conservative margins
are why finite infantry samples do not replace the continuous certificate.

Missing coverage stages legal forest proposals near-first. A rejected proposal
rolls back geometry, exclusions, counts and raster effects together. The main
advertised firing approaches remain protected and are remeasured after furnishing;
per-kind fairness still applies. A balancing feature needs real accepted trunks,
but cannot stand in for the coverage proof.

Proof work has an operation envelope independent of authored compile limits.
Allocation dimensions and potentially explosive candidate work are forecast
before allocation; the sampler can stop an exhausted proof prefix. Exhaustion is
a named refusal, not an absence-of-feature result. Those charged structural units
are not CPU instructions and do not bound the full battle's memory or renderer.
A synthetic coverage stress that omits runtime bodies proves only that certificate's
work bound, not admission or performance of a complete physical world.

Actual native visibility, normal-camera readability, composition and most bearings
remaining open need separate reports and pictures. The [generation report](../../specs/done/city-maps/generation-report.md)
explains that evidence and the certificate's accepted assumptions.

## Street furniture (`street_props`)

[Street furniture](src/street_props/) adds ordinary catalog props after buildings
and roads exist. One legality rule keeps bodies off carriageways and water, clear
of entrances and bridge approaches, apart from existing bodies and outside measured
open approaches. Dimensions and mix belong to presets; the resolved catalog owns
physical properties and the widest supported hull used to reserve a lane.

[Courts](src/street_props/courts.rs) dress the dense districts' blocks. A court's
inside is infantry ground: a vehicle is owed a way in (a car park's aisle, a yard's
vehicle gate, a lawn's lane), not a way everywhere, and every group keeps a squad's
way round its open sides. Building sites, yard boundaries, fenced groups and garden
boundaries go through one fence routine, so two neighbours share one boundary.
Amenity groups are presets data, placed whole or not at all; a region's signature
group is drawn only on that region's maps. [Furniture tests](tests/street_props.rs)
hold courts and lawns to a measured fill.

[Gardens](src/street_props/mod.rs) dress the rear setbacks of garden suburbs and
villages, never the front garden. Courts and gardens are dressed last, after the
open country's cover has certified the map's sight, and keep off every forest so
they fell no tree the certificate counted. At the request's authored-part limit,
courts take their share first and gardens what is left: on the largest maps that
limit, not the presets' density, decides how thickly they are dressed.

Kerbside parking, yard stock, courts and gardens must leave actual squad access and vehicle travel.
A planned route or successful placement alone is insufficient; [furniture tests](tests/street_props.rs)
ask the simulation's navigation, while traversal and traffic remain separate
movement proofs. Bounded local retries may omit optional furniture when it cannot
fit. Defender-prepared roadblocks belong to encounter planning rather than scenery.

## Compilation

Generation identity pins the seed, revisions, canonical plan, physical templates
and resolved generation physics. [Identity](../contract/src/identity.rs) and
[generation physics](../contract/src/generation_physics.rs) have shared contract
owners. JSON object-key order and whitespace do not change canonical identity;
authored sequence order matters where it defines implicit IDs. Source receipts
hash supplied bytes separately and do not assert unverified Git provenance.

The physical profile conservatively pins the resolved unit/prop catalog, including
presentation columns. A rule edit can therefore change generation identity while
leaving geometry unchanged. Execution allowances are separate preparation policy;
presentation assets are not the physical geometry hash. Direct authored-plan
compilation has no generation-rules input.

Compilation admits shared building, body and ground shapes before materialization.
Unsupported features and invalid river, bridge or body bounds fail by name rather
than disappearing. Declared execution limits bound their owned structures, not all
possible input bytes, terrain, navigation, trees or complete battle memory.
Visual plot composition belongs to rendering; source/art fit belongs to asset release.

## Boundaries

[The CLI](src/main.rs) and WebAssembly expose the same complete library outcome.
Failure returns diagnostics and no partial map. Successful file preparation writes
the map, its sites and shared provenance envelope. [Map admission](../contract/src/maps.rs)
resolves saved sources, and [scene-assets](../../packages/scene-assets/README.md#city-buildings)
owns physical catalog publication from accepted art sets.

Inspection reads retained plan geometry rather than regenerating it for every
crop. Battle probes follow acknowledged destinations, which may differ from a
group's clicked point. A short run without arrival cannot prove a long route is
stuck; resource brackets must separate simulation work from report formatting.

## Developer tools

The CLI's usage text owns commands and file inputs. [Examples](examples/) own
layout sweeps, sampled sight diagnostics, short battle probes and the native
[map workbench](../../apps/map-workbench/README.md) report. Run one through
`cargo run -p mapgen --release --example <name> -- <arguments>`; its source usage
defines the workload. [Tests](tests/) own current generation contracts and thresholds.

[The generation rationale](../../specs/done/city-maps/README.md) links frozen reports
and the parameter snapshot used for design feedback. Those historical values are
evidence; current settings belong to presets and their validators.
