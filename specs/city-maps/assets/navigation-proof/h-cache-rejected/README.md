# Rejected first-visit heuristic cache

**Verdict: rejected.** The optional cache preserved all frozen routes but did not repay
its memory cost. `candidate.patch` reconstructs it from the pinned base with
`git apply --unidiff-zero`; it is evidence only. Production source and the cache-specific
test were restored after the arm. The source audit was clean, and the stale-generation
falsification failed the reused-planner parity test with NoRoute; the corrected version
and the complete frozen navigation oracle passed.

The matched production binaries used the same workspace release profile, thin LTO,
dependency lock, source paths and resource fixture. Twelve alternating serial processes
per arm preserved every original route. Root's incidental clippy completed before timing;
no overlapping timing sample was accepted. The protocol and complete raw process records
are retained. Each process reports grid construction separately and then both sides/classes/
policies; initial Shortest scratch is cold, later searches retain capacity.

Median eight-route retired instructions fell only 0.54%, from 7.6958 to 7.6546 billion.
Median route time rose from 37.34 to 44.69 ms. Peak requested heap rose from 31.429 to
36.925 MB. The cache adds one f64 to each retained scratch sample: 24 bytes instead of
16, or 8,192 extra bytes per 1,024-sample tile. The allocator captures its actual high-water
cost, including both side owners and retained capacities.

All per-input p95 route values fail the 16 ms gate; most also exceed 33 ms. Twelve samples
per input give an empirical p95 equal to that sample maximum, not a claimed population
tail bound. Full-size bridge arms remain unrun. Repeated heuristic work exists, but the
cache's low instruction saving, extra residency and slower route times reject this
particular implementation. It does not justify changing queue priority or route semantics.
