# SA2 uniform regions: prove the original boundary winners without area work

**Depends on:** frozen dense navigation oracle and the retained exact pruning pass.
**Kind:** one architecture proof, production selection pending.

**Scope:** optional exact-preservation experiment under [current scale direction](../scale-direction.md).
Its old-winner constraints apply to this experiment, not every production navigation
candidate. A named behavior alternative may proceed through its own focused proof.

## Executed first cut

[The retained native proof](../assets/navigation-proof/uniform-parent/README.md) rejects
the scalar/minimum-f64-cost quotient, including a zero-prefix witness where the original
goal-pop schedule selects a one-ULP higher cost chain. Untouched public oracle checks
pass; all-plain smoothing hides that raw-chain difference. This is a concrete limit of
the candidate, not a proof that every exact uniform-region algorithm is impossible.
Repeated-work attribution now guides the next implementation cut. The separately
measured heuristic cache was rejected; no quotient has entered production.

## Question

Can a uniform rectangle's interior search be replaced by a compact calculation that
reproduces every boundary relaxation capable of changing the original winning parent
chain, with work bounded by its boundary and emitted chain rather than its area?

[The preceding work proof](../assets/navigation-proof/search-pruning/README.md) owns the
failed safe-bridge gate and its frozen measurements. Cost-bound pruning retains a large
family of equal-cost plain-ground alternatives. This cut tests elimination of that family;
it does not infer that preserving exact winners makes all faster algorithms impossible.

## Proof seam and ownership

Use a proof-local rectangle transducer: original search events enter a certified uniform
rectangle; equivalent boundary events and a reconstructible interior chain leave it.
This is an experiment interface, not a new public planner API. The ordinary graph remains
the comparison oracle. Candidate discovery may instead show that this seam is inadequate;
retain that failure and reslice before changing production.

| Input or output | Required meaning |
|---|---|
| Rectangle certificate | NavGrid's cells, footprint clearance, shared-edge/diagonal admission and temporary avoidance prove uniform interior and an explicit boundary halo. WorldGeometry supplies conservative source reach, not a second navigation graph. |
| Search context | Grid dimensions and original cell indices, exact continuous endpoints and snapped start/target, goal-dependent Euclidean heuristic, policy/mobility, exact separate cardinal/diagonal edge operations, and a fixed side-known-body/temporary-traffic snapshot. |
| Entry label | Actual f64 `g` bits, cell and parent provenance, plus the queue event that produced the relaxation. Labels are tentative and may improve later. |
| Boundary event | Exit cell, exact tentative `g` bits, reconstructible predecessor chain, and its ordering relative to original outside expansions. Boundary admission/cost remains the ordinary graph's responsibility. |
| Certificate unavailable | Explicit proof decline. The existing synchronous exact planner remains the fallback; decline is neither NoRoute nor a pending command. |

The [navigation owner](../../../crates/sim/src/navigation.rs) remains authoritative for
admission, costs, queue semantics and chain consumption. Keep the frozen oracle unchanged.
Proof-only tracing may instrument a separately identified copy to expose accepted boundary
updates and raw parent chains; it must still reproduce
the untouched oracle's complete public results. No test-only accessor belongs in production.
The experiment never publishes a route, owns movement state or changes a digest schema.

## Facts the proof must establish

**Floating-point costs.** A real-number octile distance is insufficient. The original
planner adds each rounded edge cost to its current `g`; cardinal/diagonal order can affect
the sum. A multiplication by step count, reassociation, quantization or epsilon-based tie
comparison needs proof of identical accepted `g` and parents for the certified domain.
State that domain explicitly, including incoming `g` exponent/rounding boundaries and
overflow/subnormal exclusions. Unsupported original inputs, including accepted nonfinite
or nonpositive mobility values, must retain the exact fallback and its existing quirks.

**Parent and event order.** Original strict improvement keeps the first equal-cost parent.
A macro event cannot install an equal-cost boundary label before the outside predecessor
that originally won it. Establish the order of each relevant predecessor expansion under
the original f64 Euclidean priority and cell-index tie key, and the neighbor ordinal of its
relaxation. An exit's final cost and priority alone do not establish that order. The original
queue stores `f` and cell, then reads the cell's current `g` when popped; repeated entries,
later improvements, the stale rule and goal-pop termination must be covered. Equal queue
keys do not supply an extra insertion-order guarantee. Do not invent one for the quotient.

**Interleaving and re-entry.** Demonstrate that outside expansions, multiple entry labels
and rectangle exits cannot invalidate a skipped interior winner. A proof limited to one
settled entry and one exit may establish only that narrower contract. It cannot admit an
opposing-edge bridge with competing openings or a later improved entry. If a compact
boundary summary requires an area-sized Pareto set or per-cell replay, record its failed
work bound; do not hide the original area algorithm behind a different container.

**Chain consumers.** Preserve the unsmoothed winning chain wherever it can affect emitted
waypoints, infantry crossing centers and the existing smoothing decision. Compare the
complete Plan, snapped goal, route-time f64 values, known-body/avoidance behavior and
immediate command/tick digests. Equal mathematical distance or a visually identical line
does not replace these observations. This cut grants no smoothing-policy change.

## First reviewable artifact

Build a small deterministic CLI proof against
[the executable dense oracle](../assets/navigation-proof/README.md), with a retained source
identity and complete paired results. Start with asymmetric rectangles and one entry;
vary both diagonal orientations, opposing travel directions, incoming labels near rounding
boundaries, both policies and valid mover classes. Include equal and nearly equal paths.
The proof must show boundary update provenance and chain identity, not only final distance.

Then challenge that same seam with competing entries, entry improvements, multiple narrow
openings, a nonuniform exterior and infantry's crossing/no-corner rules. Include the frozen
safe bridge's red intermediate waypoint. A counterexample is a useful red result: retain
its smallest complete input and paired outputs before attempting another representation.
Do not weaken the oracle or update its expected route to accept the candidate.

Success requires both a mathematical event/rounding argument for the certified family
and executable counterexample searches. Exhaustive small grids and sampled labels expose
mistakes; they do not prove all f64 inputs. Each unproved family remains a named decline.
Review the argument independently before production integration. Record an unavailable
CLI review as unavailable and use the established independent source-review fallback.
Before selecting production, compare the same certified inputs and complete outputs on
native and wasm, and preserve the frozen village tick trace after integration.

## Work admission

Report work in terms of rectangle perimeter, active boundary labels/events and emitted
chain length, including preparation, label revisions and certificate checks. Establish
a bound on active labels/events as well as interior storage; a perimeter representation
with area-many events has not passed. Exclude an unreported full-grid preprocessing table,
memoization cache or all-cell shortest-path pass. Measure instructions retired, capacities,
requested heap and cold/warm latency separately; scope both sides and rebuild overlap.

Only a proved small family admits a serialized safe 4 km opposing-edge bridge arm. It must
preserve all frozen outputs and meet first-route p95 at most 16 ms and no completed-tick
route work above 33 ms. Report grid construction separately; the active S1 gate later
includes it. Freeze a repeat protocol before timing, coordinate the resource slot with
Root, and stop at a failed bound or gate. Full 12/15/18 km bridge arms remain unrun until
the safe gate passes, followed by the original disconnected/blocker/surface/parity matrix.

## Verdict and handoff

The deliverable is a proved certified domain with a bounded implementation experiment,
or a concrete counterexample/work obstruction naming the unresolved inherited contract.
Root reviews that result before selecting a production integration. Changing route tie
policy, priority, arithmetic, smoothing, tick visibility or asynchronous scheduling needs
a separately named behavior decision. A failed rectangle candidate establishes its own
limit; it does not authorize such a decision or close full SA2/G0.
