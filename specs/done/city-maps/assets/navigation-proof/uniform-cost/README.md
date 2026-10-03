> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../../evidence.md).

# Reuse the implicit material costs for one plan

**Keep the repeated-work correction; full SA2 work remains red.**

When the uniform-stencil certificate selects the implicit flat cell, its cardinal and
diagonal material costs depend only on that cell, the mobility, the policy and the two step
lengths. They are now computed once per plan on the stack; exceptional stencils still use
the cost owner. No arithmetic is reassociated, and the edge half-sum, g accumulation, queue,
tie/parent order, smoothing and tick semantics are unchanged. No cache or schema was added.

## Cause and parity

Instrumentation of the previous source counted 2.3–3.2 million heap comparisons and
418–568 thousand material-cost calls per bridge route, with nearly every expanded stencil
certified uniform. A red/green assertion bounding cost calls to two per plan plus
exceptional work failed before the change and passes after it.

The dense oracle matches 12,076 cases, all 1,800 village tick digests match (ending at
`3c2f8cd1ccdd54e2`), replay and the 15 narrow navigation tests pass, and all 192 process
route outputs equal the previous corpus.

## Measurement

Over twelve alternating serial pairs of matched release binaries, median eight-route
instructions fell 1.21% (6.9361 to 6.8524 billion). Heap is unchanged and median route
times are flat (−0.22% to +1.10%), so this claims no latency gain.

Per-input p95/max is 28.97–56.60 ms: all eight exceed 16 ms and seven exceed 33 ms. No
larger bridge arm was run. The remaining heap comparisons and sparse lookups are
hypotheses for another proved correction; another constant factor alone does not prove
bounded full-size work.
