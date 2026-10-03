> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../evidence.md).

# Navigation representation proof

The original dense planner is frozen in
[`dense.rs`](../../../../../crates/sim/examples/navigation_quality/dense.rs). It was the
executable oracle of the `navigation_equivalence` example while the planner had to
reproduce its routes exactly. Since SA2's counted planner (a named route change) the
example is `navigation_quality`: it asserts what must still agree and measures how far
the new routes' costs are from the dense planner's. It is never a production backend.
Raw evidence: tag `city-maps-evidence-2026-09-30`.

## Proved scope

The candidate stores flat open cells implicitly, classifies exceptional surfaces and
side-known bodies, computes capped clearance in local tiles, and keeps route scratch only
in visited tiles. The oracle compares fits, snap, exact waypoints, stopping ranks,
temporary avoidance and accumulated route-time values, including partition boundaries and
nearly collinear long segments. The default oracle matched 9,664 cases; its 4.096 km
Shortest-only arm matched 9,632 (the small corpus plus 32 long nearly collinear routes) at
189.7 MB peak RSS. All 1,800 original village digests match, ending at `3c2f8cd1ccdd54e2`.

The route scheduler keeps original neighbour order, strict parent updates and Euclidean
priority. An older queue entry with greater `f` cannot improve a neighbour its current-cost
entry already relaxed. Repeated neighbour fit/crossing/cost answers are reused within an
expansion, and clearance outside every conservative surface/body region is analytic.

A uniform endpoint rectangle certifies the existing single smoothed segment when it is
open for the mover and holds no relevant cost exception. Faster terrain anywhere
invalidates the certificate for Fastest. The goal still uses the original fit/snap rules;
the single-column crossing quirk stays in the original graph.

## Allocation contract

Numbers are per side. `NavigationStorage` reports container lengths, capacities and latest
search counts; it enters neither battle digests nor publication.

| Owner | Residency |
|---|---|
| Classified cells | Hash entries only for exceptions to implicit open/edge values. Dense exceptional regions can still grow with area. |
| Clearance | Lazy 32×32 f64 tiles (8 KiB each); an 8-cell halo gives the same 16 m capped transform. Fields are shared by stopping rank (1–4). |
| Search | Lazy 32×32 tiles of g, parent and generation (16 KiB each), capacity retained between routes. |
| Queue | Open vector of 16-byte entries, grown only by accepted improvements. Stale entries never return NoRoute. |
| Path/smoothing | Temporary vectors; original sampled costs and greedy smoothing retained. |
| Rebuild | Old and new side grids may overlap until replacement. |

For an entirely visited square, retained search payload is bounded by
`16 × 1024 × ceil(floor(L/2)/32)^2`: 579 / 905 / 1,303 MB per side at 12/15/18 km. Full
clearance adds half that per distinct stopping rank. These are conditional worst cases,
not empty-map allocations.

## Measured arms

At 12/15/18 km, empty and two-sparse-crate worlds passed all eight opposing-side
infantry/vehicle shortest/fastest routes. Each sparse side kept 40 exceptional cells and
no clearance/search/queue samples for the certified route; at 18 km the navigation heap
grew 3,336 bytes per side and routes took 0.56–1.33 ms. Those arms used the pre-SA1 dense
world. They do not prove occupied city geometry, arbitrary road routing, bridge searches
or native/wasm movement parity.

The 4 km bridge (a full water strip with one opening) keeps exact route outputs across
every candidate, including red Shortest's intermediate point `(1451,829)`:

| 4 km bridge candidate | Peak requested heap | Eight-route instructions | Result |
|---|---:|---:|---|
| Per-cell visited hash | 282.33 MB | 239.19 billion | Rejected |
| Visited sample tiles; exact stale check | 116.52 MB | 235.58 billion | Storage improved; search red |
| Neighbour reuse; analytic clearance first | 116.52 MB | 150.27 billion | Selected; search red |
| One active heap entry per cell | 140.10 MB | 146.35 billion | Rejected: 2.6% fewer instructions for 20% more heap |

A* dominated (0.85–2.74 s) over reconstruction and smoothing (1–8 ms). The selected fastest
vehicle routes still took 1.52–1.71 s with 2.7–3.2 million expansions, so the bridge gate
stays red and the full-size bridge arms were not run. A stronger octile priority broke
exact parity (road/infantry/Fastest moved its first waypoint from `(9,9)` to `(7,7)`) and
was reverted.

## Reproduction and what is open

`cargo run -p sim --release --example navigation_quality` runs the comparison with the
dense planner; the `4096` argument adds only the long Shortest cases. `navigation_resources <extent>
<empty|sparse|bridge|disconnected>` produces allocator and route rows.

Full opposing-edge moving-unit native/wasm parity waits on the integrated
terrain/ground/delivery owners. [Exact search work](../../evidence.md)
is a separate proof; full SA2 and visual G0 remain open. The
[pruning proof](search-pruning/README.md) later reduced planning work while keeping the
original routes. The [counted road-core proof](counted-core/README.md) holds the accepted
2 km activation contract; it activates no production route job.
