# Current-path allocation baseline

Measured resource results for [S0](../../spikes/S0.md), taken before the systems architecture changes. The spike owns the numbers and conclusions.

Elapsed times vary with host load; allocator sizes and boundary outputs are the reproducible part. RSS includes allocator and platform overhead. The native experiment ceiling is enforced by admission arithmetic before allocating, not by intercepting an arbitrary out-of-memory.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
