> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../../evidence.md).

# Road graph and local corridor: first architecture tracer

**Production architecture unselected; scale work remains red.** The proof follows the
long-move road preference and accepted planning delay, and allows different parents and
waypoints from the old planner. It grants no new passage, speed bonus or hidden-state
access. The 5 km trigger and 256 m access radius are experiment settings, not selections.

## Result

The road core splits segment junctions, projects endpoints onto nearby segment interiors
and searches physical graph edges. Crossing and overlapping-collinear connectivity both
have red/green cases. Review caught forward cost reused in reverse; the corrected graph
validates and costs each direction independently. Six small tests cover those seams,
disconnected or inaccessible road decline, trigger boundaries, mid-segment access and
known blockages. A declined road corridor never means NoRoute to the destination.

Two 256 m road/junction/bridge routes pass the existing NavGrid footprint, clearance and
material evaluator for infantry and vehicles. That validates physical admission, not
stopping ranks, steering, narrow gaps or side learning.

A general-region separator tracer returns 16 legal, repeatable small routes through one
or two straight bridge openings. Every route intentionally differs from the retained
planner, with 0.97–2.43% policy-cost stretch. That is narrow route-quality evidence, not
a selected tolerance.

## Determinism and limits

The road core gives 428 identical native/wasm bytes, including route point and cost f64
bits and work counts, with an analytical constant-speed validator. It builds no full-size
WorldGeometry or NavGrid and proves no map construction, pending-state replay or Battle
digest parity.

With 64/256/1,024 parallel non-intersecting 100 km roads, the O(S²) junction builder does
2,016/32,640/523,776 pair tests while the route relaxes five edges; at 100,000 segments
that is about five billion. Fine refinement still samples with journey length and
clearance construction is not resumable. The separator only finds one axis cut through
straight passages. Road-polygon connectivity, dense-city routing, a counted fair scheduler
and revision/cancel/supersession are not implemented.

The [uniform-cost proof](../uniform-cost/README.md) supplies the failed bridge budget for
comparison. The next cut, taken by [indexed refinement](../indexed-refinement/README.md),
is an indexed junction builder and resumable counted physical refinement.
