# Indexed roads and counted physical refinement

**Proof core; production navigation is unchanged.** The target is 6/8/10 km playable
presets inside a 20 × 20 km architecture with bounded rendered surroundings. This cut
proves narrower owners; it does not admit a populated 20 km battle.
Raw evidence: tag `city-maps-evidence-2026-09-30`.

## Junction and access preparation

The road core queries a deterministic segment BVH before testing intersections. Bounds
keep actual endpoints, results come back in source order, and node identity uses exact
coordinate bits (signed zero normalised). Seven small tests cover intersections,
overlapping roads, directional cost, inaccessible or disconnected corridors and
mid-segment access. A parallel-road regression went from 32,640 irrelevant pair tests to zero.

Counted 20 km arms with 64/256/1,024 disjoint parallel segments do zero intersection
tests and 528/2,064/8,208 bound tests; an 82-road lattice does its 1,681 real pair tests.
Native and Wasm route/cost/work records are 428 identical bytes.

Open: indexing does not cap overlapping geography (long diagonal bounds can still overlap
quadratically); topology is rebuilt and all edges validated per plan; preparation, query
and search are synchronous and uncounted; road polygon topology is unsupported.

## Physical evaluation and scheduling

A resumable evaluator runs the existing sampled footprint, sub-cell admission, clearance
transform, avoidance and material cost in the original arithmetic order, so one credit can
no longer run a whole long segment. Thirty-six cold comparisons (empty ground, a rotated
wall, water/bridge; infantry and vehicles; both policies) return identical cost bits.
Each active segment keeps one private clearance page of about 35 KiB, independent of map
extent. The corpus does not cover avoidance arrays, every push rank, NaN input or changing
geometry.

The scheduler keeps one job per unit and advances counted work round-robin. Supersession
and cancel remove old jobs; a known-revision change invalidates a job before commit, per
side. A physically invalid corridor is PhysicalDecline, never NoRoute. Native and Wasm
return all 37 physical/scheduler records as 13,602 identical bytes.

Open: authoritative Battle pending state and digest, existing-route safety, commit against
a moving start, traffic/mobility invalidation, event draining and a bounded general-region
fallback.

## Representative midpoint transit

A proof-only Jeep profile (road 110 km/h, off-road 55 km/h; existing body, wheels, turning
and follower) starts on a 12 m straight road 2 m inside a 10 km map's edge and drives
4,998 m to the centre. The counted refiner holds it for 82 ticks (2.733 s), then issues
Fastest; the real Battle follower arrives 166.3 simulated seconds after the order. Planning
spends 665,934 credits; the largest 8,192-credit poll took 0.069 ms / 996,223 instructions
(one observed maximum, not a percentile). Peak heap is 34.4 MB, RSS 43.4 MB. Graph, world
and grid preparation and the legacy planner still called by Fastest are excluded.

This meets the approved uncontested midpoint transit experiment. It promises nothing for
heavier vehicles, corners or turn-heavy routes, and does not exercise surroundings,
startup, mode selection or Battle's pending-order digest. Full SA2/G0 remains open.
