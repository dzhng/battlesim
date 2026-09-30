# Uniform rectangle winner proof

**Verdict: scalar cost quotient rejected.** This is a native proof-local experiment,
not a production planner or full-size resource admission. The frozen dense oracle is
unchanged. `prepare.py` verifies its original hash, derives an instrumented copy under
`throwaway/`, and injects only the initial cost and boundary-update tracing. The derived
source and executable identities are pinned in `identity.json`.

The small runner varies asymmetric rectangles, travel orientation, mover class, policy
and raw incoming f64 labels. Its zero-label public results match the untouched executable
oracle. Every retained boundary relaxation carries the predecessor's popped priority,
current cost, cell, neighbor ordinal and parent-chain snapshot. The red admission arm
rejects the candidate; report mode succeeds by retaining that counterexample, not by
changing the oracle's expected result.

## Why a minimum cost label is insufficient

The smallest enumerated scalar witness is a 3 × 2-cell rectangle: Fastest, incoming cost
one ULP below 1. Original diagonal-then-cardinal accumulation and the scalar candidate
disagree by one ULP. The complete input, cost bits and accepted boundary updates are in
`paired-result.json`.

There is also a **zero-incoming** 5 × 4-cell witness. Original Shortest search wins with
three diagonal steps followed by one cardinal, ending at `4024f876ccdf6cda`. The valid
cardinal-then-three-diagonal walk ends at `4024f876ccdf6cd9`, one ULP lower. The original
rounded Euclidean queue/goal-pop schedule still chooses the former chain. Thus choosing
the least scalar f64 cost can change the frozen winner; all-real-number shortest-path
arguments alone cannot justify this quotient. On this all-plain fixture smoothing hides
the chain difference. It is not a final Plan/digest counterexample or proof that every
exact quotient is impossible.

The [owning slice](../../../slices/SA2-uniform-parent-quotient.md) retains event-order,
external interleaving and chain-consumer proof questions. No boundary-event architecture
or alternate route policy has been selected. The next implementation work measures
repeated search operations before further architecture selection.

## Repeated-work attribution

`prepare-attribution.py` derives counters from the separately pinned retained planner;
it does not edit production. The safe 4 km arm retains all eight original route strings.
Per route, heuristic evaluation repeats roughly 2.17–2.63 times per distinct evaluated
cell, with 1.90–2.53 million scratch tile gets plus 223–309 thousand entry lookups. These
counts expose repeated operations; they do not establish their share of retired
instructions. The seen-cell bitset adds 0.5 MB at this extent.

The counter package used its default release profile, without the workspace's thin LTO,
and its counters change work. Its printed wall time and total instructions are not a
production performance comparison. The matched [heuristic-cache rejection](../h-cache-rejected/README.md)
demonstrates why the count alone did not justify retaining a cache.

To reproduce the small proof, run `prepare.py` with the checkout root, then run its
generated Cargo manifest with this worktree's isolated `CARGO_TARGET_DIR`. `--assert-scalar`
is the expected red admission arm; the default emits the rejected proof. Attribution
requires the exact pinned retained navigation source and runs separately with serialized
resource ownership. Neither runner edits the oracle or supplies a production fallback.
