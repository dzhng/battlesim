# Road graph and local corridor: first architecture tracer

**Production architecture unselected; scale work remains red.** This proof follows the
current long-move road preference and accepted planning delay. It permits different old
parents/waypoints. It grants no new physical passage, speed bonus or hidden-state access.
The numerical trigger is a configurable experiment; 5 km is not a global requirement.
The tested 256 m access radius is provisional, not a selected production radius.

## Concrete first result

The independent road core splits segment junctions, projects endpoints onto nearby segment
interiors and searches physical graph edges. Crossing and overlapping-collinear connectivity
both have red/green cases. A review then exposed reuse of forward cost in reverse; the
retained rejected source/test demonstrates it, and the corrected graph validates/costs both
directions independently. Six tiny tests cover those seams, disconnected/inaccessible road
decline, configurable trigger boundaries, mid-segment access and known-blockage inputs.
A declined road corridor never means NoRoute to the destination.

Two 256 m road/junction/bridge routes use the existing NavGrid footprint/sub-cell/clearance
and material segment evaluator: infantry and vehicle both pass route_fits. The proof-only
source-derived module exposes that evaluator inside this experiment; no production hook,
state field or planner branch was added. Those examples validate physical admission, not
all stopping ranks, steering, narrow-gap or side-learning integration.

The general-region separator tracer returns 16 legal, repeatable small routes through
one or two straight bridge openings. Compared with the pinned retained planner, all routes
intentionally differ; policy-cost stretch is about 0.97–2.43%. Both alternatives use the
same segment-cost evaluator for comparison. Synthetic 6/12 m/s mobility is inherited
from the resource fixture for this admission tracer; it selects no foot/vehicle tuning. This is narrow route-quality evidence, not a
selected tolerance, a shortest-path theorem or full-size acceptance. Raw durations are
uncontrolled functionality telemetry, not admitted timing measurements.

## Determinism and limits

The standalone road core produces 428 identical native/wasm bytes, including complete
route point/cost f64 bits and work counts. The parity harness's passage validator is an
analytical synthetic constant-speed function. A single 100 km road needs six nodes in
that core; it creates no 100 km WorldGeometry/NavGrid and proves no full map construction,
physical-validation runtime, pending-state replay or Battle digest parity. The simple
known-blockage test changes only the supplied belief validator: existing SideGeometry
learning/fog semantics still need an outer integration tracer.

A separately counted core arm with 64/256/1,024 parallel, nonintersecting 100 km road
segments performs 2,016/32,640/523,776 intersection pair tests while the selected route
relaxes five edges. This executes only analytical graph geometry, without full-size fine
world construction. It confirms repeated irrelevant preparation, not a timing acceptance
claim. The derived count source changes only the loop counter; raw records are retained.

Source uses an O(S²) junction builder and linear node deduplication, not a bounded
production road-preparation owner. Its intersection pair counts grow from roughly half a million at 1,000 segments to five billion at
100,000. Fine physical segment refinement still samples with journey length, and ordinary
clearance construction is not resumable. The separator tracer only discovers one axis cut
with straight passages; winding passages and general geography decline. No road-polygon
connectivity source, arbitrary dense-city route proof, counted fair scheduler or live
revision/cancel/supersession proof is implemented. These are explicit red owners, not
budget-driven Blocked outcomes. Larger measured arms remain unadmitted.

The preceding [uniform-cost receipt](../uniform-cost/README.md) supplies the actual failed
safe bridge budget. It is frozen comparison evidence; its equality requirement does not
select behavior for this named alternative. The next useful cut is an indexed road
junction/access builder and resumable, counted physical refinement with real owner costs,
followed by bounded general-region routing and the outer knowledge/order lifecycle.

## Reproduction and review boundary

`road_graph.rs` and `road_parity.rs` are independent core proof sources. Build with
`rustc --edition=2021 --test road_graph.rs` for tests; the parity harness uses `-O` natively
and `-O --target wasm32-unknown-unknown --crate-type cdylib` for wasm. Call wasm `run`,
read `result_ptr` and copy the returned byte length from memory. `road-native.hex` and
`road-wasm.hex` retain the full records. Toolchain, source and compiled binary hashes are
pinned in the identity. No compiler or wall-clock race chooses a route in this core.

The original full physical tracer staged under checkout `throwaway/navigation-corridor`;
`prepare.py` pins and derives the retained production navigation source, including its
experiment-only wrapper. `candidate.rs.snapshot` is its exact compiled source. The raw
physical outputs and build receipt identify this separate scope. Original dense oracle,
route/digest receipts and production navigation remain untouched. Shape/diff review found
no added production owner; directed validation corrected the concrete source defect.
CLI review rejected the configured model before analysis, without a verdict or override.
Full independent source review, scheduler and integrated native/wasm physics gates remain open.
