# Uniform rectangle winner proof

**Verdict: scalar cost quotient rejected.** A native proof-local experiment against the
unchanged dense oracle, not a production planner. Its runner varies asymmetric
rectangles, travel direction, mover class, policy and raw incoming f64 labels.

## Why a minimum cost label is insufficient

The smallest scalar witness is a 3 × 2-cell rectangle under Fastest with an incoming cost
one ULP below 1: original diagonal-then-cardinal accumulation and the scalar candidate
disagree by one ULP.

A zero-incoming 5 × 4-cell witness also exists. Original Shortest search wins with three
diagonal steps then one cardinal, ending at `4024f876ccdf6cda`. The valid
cardinal-then-three-diagonal walk ends at `4024f876ccdf6cd9`, one ULP lower, but the
rounded Euclidean queue still picks the former. So picking the least scalar f64 cost can
change the frozen winner, and real-number shortest-path arguments cannot justify this
quotient. Smoothing hides the difference on this all-plain fixture, so it is not a final
Plan/digest counterexample, nor proof that every exact quotient is impossible.

The [owning slice](../../../slices/SA2-uniform-parent-quotient.md) keeps the event-order,
interleaving and chain-consumer questions. No boundary-event architecture or alternate
route policy has been selected.

## Repeated-work attribution

On the 4 km bridge, with all eight original routes retained, heuristic evaluation repeats
about 2.17–2.63 times per distinct evaluated cell, with 1.90–2.53 million scratch tile gets
and 223–309 thousand entry lookups per route. These counts show repetition, not its share
of instructions; the counters themselves change work, so their timings are not a
performance comparison. The [heuristic-cache rejection](../h-cache-rejected/README.md)
shows why the count alone did not justify a cache.
