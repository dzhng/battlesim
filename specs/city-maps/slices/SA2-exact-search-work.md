> The current scale direction permits named route/timing alternatives.
> [Hierarchical road routing](SA2-hierarchical-road-routing.md) owns that active proposal.
> This exact-work slice and its frozen receipts remain historical preservation evidence.

# SA2 search work: preserve the winning route while bounding planning

**Depends on:** SA2 sparse representation and frozen dense oracle. **Kind:** architecture proof.

**Current selection policy:** [scale direction](../scale-direction.md) permits named
route/timing changes and proposals for simpler mechanics. The exact-preservation
question and results below describe that investigation, not a mandatory constraint
on all candidate architectures. Keep this evidence frozen; compare the recommended
alternatives before spending further work solely preserving old raw parents.

## Question and seam

Can an exact planner bound first-route work across [the current M04 extents](../procedural-maps.md#closed-decisions) while keeping the original
winning waypoints, costs, parent/tie schedule, command visibility tick and battle digests?
Side-known revision + start/goal + footprint/push/policy + traffic still produces the
existing Plan. Resource exhaustion must be an explicit failed proof, never NoRoute.

## First reviewable proof

Keep the measured 4 km opposing-edge bridge as the first gate: first-route p95 ≤16 ms,
no completed-tick route work above 33 ms. Attribute expansions,
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

### Cost bounds and lookup proof: partial improvement, failed work gate

[The exact pruning proof](../assets/navigation-proof/search-pruning/README.md) owns its
source, full mathematical argument and measurements. A valid same-grid walk supplies
only an upper bound. Octile and mandatory-opening lower bounds prune strict losers with
an accumulated-f64 allowance; the original Euclidean queue, parents, route, smoothing
and tick timing remain authoritative. A local certificate reuses implicit nine-cell
answers without adding a cache. Invalid proof assumptions leave the original search.

All eight safe 4 km bridge routes preserve their frozen outputs. Eight-route work falls
from 150.27 to 7.69 billion instructions and requested heap from 116.52 to 31.43 MB, but
31.5–46.8 ms routes fail the budget. No p95 repeat distribution or larger bridge arms
were admitted. This is a separately reviewed exact planning improvement; full SA2 is red.

### Scratch relaxation lookup: retained correction, failed scale gate

[The matched lookup proof](../assets/navigation-proof/fused-lookup/README.md) joins
the existing cost check, optional bound and parent write through one scratch-table entry.
It reduces instructions without adding per-cell payload, and retains every frozen route
and village tick digest. Its paired repeat distribution still fails the original
16/33 ms gate. Larger bridge arms remain unrun. The next repeated-work owner needs
independent attribution before a further correction; the scalar quotient and heuristic
cache experiments did not select another architecture.

### Implicit material reuse: retained work reduction, flat timing

[The measured material-cost proof](../assets/navigation-proof/uniform-cost/README.md)
attributes repeated calls under the existing uniform stencil, then computes its two
constant costs once per plan. It adds no retained payload and preserves the frozen
routes/digests. Instructions improve slightly; route timing is flat and the original
safe gate still fails. Heap comparison counts identify a possible next owner, not a
selected queue implementation or measured share of route cost.

### Optional preservation proof: eliminate states while preserving the winning parent chain

The remaining 200–278 thousand expansions are mostly equal-cost plain-ground states.
For an approach with `a` orthogonal and `b` diagonal steps, all their permutations can
visit roughly `(a+1)*(b+1)` positions on shortest completions. Cost-bound pruning must
retain such alternatives within its rounding allowance. At fixed approach angles this
grows with map area: the 18 km prediction is about 20.25 times the 4 km corridor, not a
measured admission. Another constant-factor lookup saving does not establish scale.

[The uniform-region boundary-winner proof](SA2-uniform-parent-quotient.md) is an optional
focused cut. It tests whether grouping plain states or calculating their parent chain
can preserve strict f64 winners without area work. That slice owns the proof-local
boundary-label seam, rounding/event-order questions and safe-gate admission. Production
architecture remains unselected. Named scheduling or route-changing alternatives may
be proposed under the current policy; none is selected by this proof.

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
and temporary overlap. Full visual G0 stays open. Compare named route-changing and
scheduling alternatives under the current policy without waiting for every exact
architecture to fail; record their player consequences and measured work explicitly.
