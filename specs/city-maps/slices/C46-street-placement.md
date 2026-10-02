# C46: street placement

**Depends on:** C44, C53. **Kind:** slice.

## Question
Does street placement keep playable space?

## Contract it unlocks
`place_street_props(MapPlan, catalog, seed) → props`, inside `crates/mapgen`: curb setbacks; no overlap with buildings, entrances or the reserved plain's open corridor; infantry and vehicle routes preserved; bounded rejection attempts. Street trees use C74's one tree generator and a named random stream.

## API seam
`crates/mapgen`.

## What the human can run or see
A block traversal GIF and a top-down density PNG.

## Verification
- Deterministic placement.
- Route-preservation test.
- City digests change (named).
- Frame-cost row.

## Delegated to the implementer
Density and spacing data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every placement resolves to a body and an appearance.

## Feedback that would change this slice
Furniture that blocks required approaches changes placement constraints before visual density is accepted.

## Outcome — kerbside placement (2026-10-02)

Street furniture is ordinary physical map data, placed after buildings by
[`street_props`](../../../crates/mapgen/src/street_props.rs). Its existing
preset rows own density, parking runs, setbacks, door approaches and open
junction corners. The unit/prop catalogue supplies the widest vehicle and
validates every placed kind. Generator `layout-12` and presets
`layout-presets-11` name the combined kerbside and parcel-fit change; native
and Wasm records and the saved Market Town use those same inputs.

The lane is the free width of the road. A road journey tries clear lines
before a shove, moves over gently past a known body, holds its line through
a row, and rejoins its side when the next stretch is clear. Furniture can
therefore stand near the kerb and before terrace fronts: the lane margin
returns from 3.7 m to 0.9 m. Junctions and door approaches keep their existing
clearance rules.

A coarse cell touched by a roadside body does not prove that a vehicle's
segment touches it. Segment refinement reads exact footprints from the
side's own navigation grid, including its moved and removed body beliefs.
The original conservative standing and cell-node contract remains. A
refined segment must still keep its navigation width on traversable ground;
body refinement cannot erase a map edge, river bank or steep surface.
Traffic detours use the movement owner's clearance margin and do not shove
static bodies while making room for live hulls.

Every lane candidate and exact-body query keeps its cursor between planning
steps. Dense body buckets cannot turn one road check into an indivisible
scan. Those cursors participate in the simulation digest, so different
unfinished work is different battle state. Steering also retains a route's
clearance corners until reached or passed abeam: pruning a corner early had
removed the lateral waypoint needed by opposing jeep columns.

Squad progress cannot hide an individual soldier caught beside standing
infantry. A blocked corridor member uses the existing local fine planner to
walk round the nearby live bodies and rejoin, while the others keep moving.
The same body radii and retry throttle govern corridor and final approach;
recovering a path does not move the accepted destination or the obstruction.

**Physical proof.** The live kerbside tests drive the tank, truck and jeep down
the same parked street at grid-aligned and oblique headings, in both
directions. They check actual arrival, carriageway position, no overlap with
static bodies and exact unchanged parked-car positions. Opposing mixed
vehicles and opposing widest hulls arrive without hull overlap or shoving.
The road-journey tests also exercise two opposing columns. These fixtures
prove those traversals; they do not prove every traffic knot on every city.

**Placement proof.** The combined two-seed type/size sweep preserves 18
assault plans, 117,632 door standing positions and 738 sampled door routes.
It preserves 1,518 sampled vehicle routes within the existing detour limit,
including 506 widest-hull routes that require no shove. These are planner
proofs, distinct from the live street traversals. A physical-contact
regression distinguishes passing through a stamped cell from touching its
body, and still reports a real contact. The dense-query regression compares
a yielded route against the same route completed in one call while enforcing
the existing per-step work limit.

The top-down comparison holds the generator, seed, parcel spacing and crop
fixed while changing the parking margin. The traversal GIF follows a real
simulated tank and shows its physical hull and the parked bodies. These are
diagnostic geometry views; building and furniture art remains governed by
C45/C54. The combined parcel change is documented in C53. A car near a house
must be judged against that template's allowed facade fit, not an apartment's
larger allowance.

**Cost boundary.** The placement densities were previously measured in
instructions retired: adding furniture cost +0.4% of ticks and packing on
Mixed Small and +0.6% on Metro Large. Those rows predate this navigation pass
and are not measurements of its exact-body refinement. The per-tick work
checks prove yielding, not a renderer frame-cost row. C54 still owns the
rendered block traversal, street-art clearance and frame-cost acceptance.
