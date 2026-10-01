# Rejected first-visit heuristic cache

**Verdict: rejected.** The optional cache preserved every frozen route but did not repay
its memory. Production source was restored after the arm.

Over twelve alternating serial runs per arm of matched release binaries, median
eight-route instructions fell only 0.54% (7.6958 to 7.6546 billion), median route time rose
from 37.34 to 44.69 ms, and peak requested heap rose from 31.43 to 36.93 MB. The cache adds
one f64 per scratch sample (24 bytes instead of 16, 8 KiB more per 1,024-sample tile).

All per-input p95 route times fail the 16 ms gate. Heuristic work does repeat, but this
implementation's small saving, extra residency and slower routes reject it. It does not
justify changing queue priority or route semantics.
