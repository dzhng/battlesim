# SA2 search work: preserve the winning route while bounding planning

**Depends on:** SA2 sparse representation and frozen dense oracle. **Kind:** architecture proof.

## Question and seam

Can an exact planner bound first-route work across 12/15/18 km while keeping the original
winning waypoints, costs, parent/tie schedule, command visibility tick and battle digests?
Side-known revision + start/goal + footprint/push/policy + traffic still produces the
existing Plan. Resource exhaustion must be an explicit failed proof, never NoRoute.

## First reviewable proof

Keep the measured 4 km opposing-edge bridge as the first gate. Attribute expansions,
lookups and queue work; compare retired instructions and exact outputs with the frozen
planner before admitting larger arms. The chosen sparse correction reduces storage and
repeated classifications, but fastest bridge searches still expand 2.7–3.2 million nodes.
Changing priority to an octile estimate failed route parity; the indexed heap's small
instruction saving did not justify its memory/ownership cost. Those are rejected arms.

Investigate spatial lookup/cost reuse or a graph quotient that can prove elimination of
irrelevant work while reproducing the winning parent chain. A separately proved lower
bound used only for pruning could preserve original queue ordering; it needs a feasible
upper bound, original-winner retention and an explicit accumulated-f64 error allowance.
No hierarchy, approximation, quantized cost or changed tie order is a selected design.

Conservative diagonal-road rectangles also cause area-wide surface sampling and can
admit too many clearance tiles. A precision shape-region contract belongs to WorldGeometry;
NavGrid owns rasterization and clearance reach. Prove a safe build arm before requesting
analytic segment/capsule metadata from that owner.

## Bounded scheduling alternative and parity limits

A resumable search could retain heap/scratch and advance a bounded number of expansions
per worker batch. It needs explicit progress, cancellation and revision ownership, and
cannot publish partial routes or map exhaustion to NoRoute. Its outer state would be
Pending versus Complete(existing Plan), with route/clearance build also budgeted.

This alone does not preserve immediate tick semantics. Continuing simulation while a
route is pending changes movement and future digests; pausing the whole tick keeps route
inputs stable but does not reduce total tick latency. A production asynchronous command/
tick contract requires a separately named behavior decision. Do not implement scheduling
as an exact-route parity shortcut.

## Admission and completion

Stop at the first failed gate. Full-size bridge, sparse blocker, disconnected, stopping-rank,
road/forest/slope and arbitrary-angle native/wasm moving-unit arms are admitted only after
measured bounded work at the safe gate. Include both sides, retained capacities, rebuild
and temporary overlap. Full visual G0 stays open. If no exact architecture passes, expose
which original scheduling/rounding requirement prevents the bound before proposing a
route-changing alternative.
