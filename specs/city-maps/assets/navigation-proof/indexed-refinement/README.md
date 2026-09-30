# Indexed roads and counted physical refinement

**Proof core; production navigation remains unchanged.** The current direction requires
6/8/10 km playable presets and a 20 × 20 km playable architecture with bounded rendered
surroundings. Historical 100 km inputs in the preceding receipt retain their original
meaning. This cut proves narrower owners; it does not admit a populated 20 km battle.

## Junction and access preparation

The road core queries deterministic segment BVH bounds before testing intersections.
Bounds retain actual endpoints, query results return in source order, and the canonical
pair loop remains ascending. Node identity uses exact coordinate bits with signed zero
normalized to match Point equality. Nearby access queries still validate exact projection
distance. Seven small tests cover intersections, overlapping roads, directional physical
cost, inaccessible/disconnected corridors and mid-segment access. The new parallel-road
regression failed at 32,640 irrelevant pair tests before indexing and passes with zero.

Counted 20 km geographic arms with 64/256/1,024 disjoint parallel segments perform zero
intersection tests and 528/2,064/8,208 primitive-bound tests. The 82-road crossing lattice
performs its 1,681 actual pair tests. Complete road route/cost/work records are 428 identical
Native/Wasm bytes. Its validator is analytical; this is no physical world qualification.

Indexing does not cap real or overlapping candidate geography. Long diagonal bounds can
still overlap quadratically. The current core rebuilds topology and validates all edges
per plan; preparation/query/search are synchronous and not yet part of the counted job.
Road polygon topology remains unsupported. These are open architecture owners.

## Physical evaluation and scheduling

A resumable evaluator executes the existing sampled footprint, sub-cell admission,
stopping-class clearance transform, avoidance and material-cost calculation in original
arithmetic order. One credited operation cannot run a whole long segment: the old wrapper
fails that regression, then the resumable candidate passes. Thirty-six cold comparisons
across empty ground, a rotated wall and water/bridge geography return identical Option<f64>
cost bits to the original evaluator, for infantry/vehicles and both route policies.

Each active segment keeps one private clearance page, replacing it when its tile changes.
Construction counts allocation, initialization, forward/backward transform cells and page
extraction separately. Maximum temporary tile/page payload is about 35 KiB per active
segment, independent of map extent. Avoidance iteration is counted per footprint. Allocator,
table operations and constant-size transform relaxations are atomic categories requiring
calibration, rather than a theorem about retired instructions per credit. The corpus does
not cover avoidance arrays, every push rank, NaN input or arbitrary changing geometry.

The scheduler keeps one job per unit and advances stable round-robin counted work.
Supersession/cancel remove old jobs; a captured known-revision change invalidates its job
before commit. Both sides make progress, and one side's revision does not invalidate the
other. A physically invalid corridor is PhysicalDecline, never a resource NoRoute.
The scheduler trace repeats exactly. Native/Wasm returns all 37 physical/scheduler records
as 13,602 identical raw bytes, preserving integer cost bits without a JavaScript number
round-trip. Shared clearance history cannot change the work sequence.

This prototype has no authoritative Battle pending state or digest. Existing-route safety,
commit against a moving start, traffic/mobility invalidation, graph stages, event draining
and bounded general-region fallback require integration. The caller must preserve immutable
side-known inputs during each job and restart invalidated work. Those are explicit next
contracts, not implemented scheduling guarantees.

## Representative midpoint transit

One serialized native arm uses a named proof-only Jeep profile: road 110 km/h, off-road
55 km/h, with existing light-vehicle body, wheel radius, turning and follower unchanged.
It starts facing along a 12 m wide straight road, 2 m inside a 10 km map's midpoint edge,
and travels 4,998 m to centre. No combat, turns or local approach distance is present.

An external counted refiner holds the initially idle mover for 82 ticks (2.733 s), then
issues the existing Fastest command. The actual Battle follower reaches the arrival
condition at 166.3 simulated seconds from the order request. Physical planning consumes
665,934 credits; the largest observed 8,192-credit poll is 0.068542 ms / 996,223 instructions.
This is one observed maximum, not a population percentile or universal tick admission.
It excludes graph/world/grid preparation and the legacy planner still invoked by Fastest
acceptance; the process receipt includes those costs, but their active work is not scheduled.
Requested heap peak is 34,372,543 bytes; process RSS is 43,352,064 bytes. The complete
fixture, tick digests, planning samples and process instruction record are retained.

This meets the approved representative initial uncontested midpoint transit experiment.
It grants no heavier-vehicle/corner/turn-heavy promise. Rendered surroundings, whole startup,
mode selection and Battle's pending-order digest were not exercised. The external planner
is distinct from production accepted-command integration. The frozen compiled transit
source bundle predates the portable wrapper; its actual binary hash is separate.

## Reproduce and review

`identity.json` pairs portable sources and records; the two core test logs normalize only
the final empty stdout line, with their captured hashes preserved. `compiled-transit/identity.json`
pairs the measured transit binary with its exact compiled snapshots, including the old
manifest and runner. Production navigation is pinned by SHA-256; `candidate.rs` is only a
source-derived proof module. No production method or oracle was modified.

The standalone manifest points at the measured worktree dependencies. Stage its files in
checkout `throwaway/navigation-indexed`, use the isolated CARGO_TARGET_DIR and build the
native bin. Its default run writes fixture input and physical output; `transit` selects the
separately admitted transit arm. Build the lib for wasm32-unknown-unknown, then run
`physical-parity.mjs` against the native output. Core graph tests/counts/parity use the
retained rustc harnesses. `prepare.py` derives the source but writes the initial native
manifest; the portable lib stanza is a later recorded addition. It must not silently
replace the measured manifest when reproducing parity.

Independent source review is requested from Root. Configured Codex CLI review remains
unavailable because its model is unsupported by the current endpoint; no independent
CLI verdict or model override is claimed. Full SA2/G0 remains open.
