# Navigation representation proof

The original dense planner is frozen in `dense-navigation.rs` (SHA-256
`e387c1bcb173380360cc1db315c395bfdc3fb6569e43e8b1ddf614da02e86356`).
It is an executable oracle for the proof example, never a production backend.
The baseline route and village traces were captured before representation replacement.

## Proved scope

The candidate stores flat open cells implicitly, classifies exceptional surfaces and
side-known bodies, calculates capped clearance in local tiles, and keeps route scratch
only in visited tiles. The frozen public oracle compares fits, snap, exact waypoints,
stopping ranks, temporary avoidance and accumulated route-time values. Partition
boundaries and nearly collinear long segments are included. The default oracle matched 9,664 cases; its 4.096 km Shortest-only arm matched 9,632
cases (the same small corpus plus 32 long nearly collinear routes), at 189.7 MB peak RSS.
The village trace compares all 1,800 original digests; its final digest is `3c2f8cd1ccdd54e2`.

The two focused tests first failed on eager cell residency and disconnected-water search,
then passed after implicit storage and an exact closed-column/row certificate. The route
scheduler retains original neighbour order, strict parent updates and Euclidean priority.
An older entry with greater `f` cannot improve a neighbour after its current-cost entry
has already relaxed it. Within an expansion, repeated neighbour fit/crossing/cost answers
are reused. Clearance outside every conservative surface/body region is analytic.

A uniform endpoint rectangle certifies the existing single smoothed segment when it is
open for the mover and contains no relevant cost exception. Faster terrain anywhere
invalidates this certificate for Fastest. Conservative source regions, side-known body
bounds, clearance reach and temporary traffic are included. The goal still uses original
fit/snap rules; the single-column crossing quirk remains in the original graph.

## Allocation contract

All numbers are per side unless stated otherwise. `NavigationStorage` reports actual
container lengths/capacities and latest search counts. It does not enter battle digests or
publication. Requested heap is measured by the probe allocator; RSS includes other memory.

| Owner | Residency and multiplicity |
|---|---|
| Classified cells | Hash entries only for exceptions to implicit open/edge values; capacity, keys, control bytes and padding still count. Known bodies are per-side believed bodies. Dense exceptional regions can still grow with area. |
| Clearance | Lazy 32×32 f64 tiles, 8 KiB payload each; an 8-cell halo computes the same 16 m capped two-pass transform. Fields share equal stopping ranks. Ranks 1–4 can be distinct; rank zero is unused. No footprint-specific duplicate field. |
| Search | Lazy 32×32 tiles; each payload has 8-byte g, 4-byte parent and 4-byte generation per sample (16 KiB). Tiles retain capacity between routes. Generation rollover clears stamps. |
| Queue | Local Open vector, 16 bytes per entry on this target, grown only by accepted improvements; latest peak capacity is reported. Stale entries do not return NoRoute. |
| Path/smoothing | Temporary cell, point, turn and cost vectors; original sampled costs and greedy smoothing retained. |
| Rebuild | A new side grid may overlap the old grid until replacement; each side's retained clearance/search tiles participate in that peak. Resource rows do not certify every rebuild/catalog occupancy. |

For an entirely visited square, tile padding alone bounds retained search payload as
`16 × 1024 × ceil(floor(L/2)/32)^2`: 579.08 / 904.81 / 1302.92 MB per side
at 12/15/18 km. Fully occupied clearance would add half that per distinct stopping rank.
These are conditional worst-case payloads, not empty-map allocations or universal admission.
Hash-table overhead, known exceptions, queue/path growth and rebuild overlap are additional.

## Measured arms

`native/` contains allocator stage rows paired with `time -l` output. All arms ran serially.
The allocator checks live requested allocation and conservative realloc overlap against
4 GiB before calling the system allocator; exit 70 is an experiment failure, never NoRoute.

At 12/15/18 km, empty and two-sparse-crate worlds passed all eight opposing-side infantry/
vehicle shortest/fastest routes. Each sparse side retained 40 exceptional cells and no
clearance/search/queue samples for the certified route. At 18 km its navigation-grid live
heap increment was 3,336 bytes per side; routes took 0.56–1.33 ms in that recorded candidate.
Those arms used the pre-SA1 dense world, whose 372 MB allocation is reported separately.
They do not prove occupied city geometry, arbitrary road routing, bridge searches or full
native/wasm movement boundary parity.

The admitted 4 km bridge has a full water strip with one bridge opening. Its route outputs
remain exact across the storage and queue candidates, including red Shortest's intermediate
point `(1451,829)`.

| 4 km bridge candidate | Peak requested heap | Eight-route instructions | Result |
|---|---:|---:|---|
| Per-cell visited hash | 282.33 MB | 239.19 billion | Storage rejected |
| Visited sample tiles; exact stale check | 116.52 MB | 235.58 billion | Storage improved; search red |
| Neighbour reuse; analytic clearance lookup first | 116.52 MB | 150.27 billion | Selected; search red |
| One active heap entry per cell | 140.10 MB | 146.35 billion | Rejected: 2.6% instruction saving costs 20% heap plus queue machinery |

Temporary timing instrumentation separated A* (0.85–2.74 s) from reconstruction/smoothing
(1–8 ms) before neighbour reuse. The selected fastest vehicle routes still take 1.52–1.71 s,
with 2.7–3.2 million expansions. The full-size bridge work arms remain unexecuted; the
4 km failure is sufficient to keep that gate red.

A stronger octile *priority* failed exact parity: road/infantry/Fastest changed its first
waypoint `(9,9)` to `(7,7)`. See `rejected-octile.txt`. The original priority was restored.
The rejected indexed-heap patch is evidence only; it applies to the preceding unformatted
candidate and is absent from production. Multiplying sampled costs instead of accumulating
original f64 additions was not adopted; timing showed smoothing was not the failed owner.

## Reproduction and remaining gate

Use the main shared node_modules and a unique worktree CARGO_TARGET_DIR as AGENTS.md requires.
`cargo run -p sim --release --example navigation_equivalence` runs the safe dense oracle.
The admitted `4096` argument adds only shortest near-collinear long cases; it suppresses the
broad empty Fastest search. `navigation_resources <extent> <empty|sparse|bridge|disconnected>`
produces allocator and route rows; obtain the shared resource slot and preflight first.
Full opposing-edge moving-unit native/wasm parity remains pending on integrated
terrain/ground/delivery owners; no unexecuted boundary harness is promoted as proof.

The CLI Codex review was unavailable before analysis: its configured `gpt-6.1-sol` is
unsupported by the CLI ChatGPT endpoint. No independent CLI pass is claimed and no global
configuration was altered. Consumer/root review and focused gates cover this correction.

[Exact search work](../../slices/SA2-exact-search-work.md) remains a separate architecture
proof. Full SA2 and complete visual G0 remain open.
