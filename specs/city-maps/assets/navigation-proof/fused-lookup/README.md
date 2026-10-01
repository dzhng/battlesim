# One scratch lookup per relaxation

**Verdict: keep the lookup correction; the full SA2 work gate stays red.**

The scratch owner checks a tentative cost, runs the optional bound only for a fresh or
improving label, and stores its parent through one hash-table entry. The original strict
comparison (including first-visit NaN/Infinity behaviour), queue priority, neighbour order,
heuristic arithmetic and scratch lifetime are unchanged; no cache or public seam was added.

The dense oracle matches 12,076 cases, all 1,800 village tick digests match (ending at
`3c2f8cd1ccdd54e2`), and replay and the 15 narrow navigation tests pass.

## Measurement

Twelve alternating serial runs per arm of matched release binaries preserved all 192
original route outputs. Median eight-route instructions fell from 7.6936 to 7.0301
billion (8.63%); per-input median route time improved about 6–8%. Peak heap and retained
capacities are unchanged (16 bytes per scratch sample).

Per-input p95/max route times are still 28.06–43.22 ms: all eight exceed the 16 ms target
and seven exceed 33 ms. With twelve samples, p95 equals the sample maximum. The full
12/15/18 km bridge arms remain unrun.

## Open

The next change needs matched count/instruction attribution first. Queue comparisons,
exceptional-cell lookup and bound work are candidates to measure, not conclusions.
