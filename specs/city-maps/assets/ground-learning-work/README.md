# Native learned-ground work

The workload: clear and seal the ground, run two independent learning owners, serialize through production, keep two credit-sized copies, resynchronize, and edit hidden and revealed corners. It excludes Battle, wasm, JavaScript and GPU ownership, so its memory verdict is not whole-battle admission. The probe enforces a 4 GiB requested-heap ceiling before allocating; hitting it is a failed resource arm, never lost ground or `NoRoute`.

## Baseline: memory fits, CPU fails

At 18 km the baseline keeps all 324 million cleared cells and sends a 21.5 MB snapshot. Peak requested heap is 551 MB and maximum RSS 646 MB. Values, side separation, retained copies and the one-cell reveal delta pass.

CPU work fails. Each initial full learning pass retires about one trillion instructions (44–47 s). Unchanged learning retires 3.85 billion instructions, 209–213 ms per side, and one visible edit still costs 205 ms. The snapshot is an exceptional reconnect cost; unchanged and one-edit learning recur every tick and exceed the simulation budget. This drove the [work correction](../../slices/SA3-exact-learning-work.md).

## Corrected learning owner

Truth indexes the latest edit of each touched tile. A side keeps its previous visibility words and visits only edited tiles or newly visible ground. Fully visible pages share immutable truth; edits copy on write, and only changed visible values get new learning stamps. Partial edge holes stay blank and unstamped. The per-fog-cell sync store is gone.

All value, revision, side and credit assertions pass at the fixed extents. Recurring stages cover no-op, one visible edit, a hidden edit and a later reveal:

| Extent | Recurring learning | Initial learning per side | Snapshot pack | Requested heap peak |
|---|---:|---:|---:|---:|
| 12 km | 0.047–0.058 ms | 89–90 ms | 18 ms | 258 MB |
| 15 km | 0.066–0.076 ms | 140–141 ms | 29 ms | 406 MB |
| 18 km | 0.101–0.170 ms | 224–225 ms | 44 ms | 581 MB |

At 18 km stable learning retires about 2.8 million instructions instead of 3.85 billion, and initial learning 4.15 billion instead of a trillion. RSS falls to 615 MB; requested peak grows slightly because the edit index replaces work. The probe's 324 million public clear calls are an artificial setup, not gameplay.

Disabling new-visibility selection fails the regression; a 1 km instruction tracer failed on the old owner and passes on the new one. The observation and digest corpus, ground delivery and rebuilt wasm codec checks all pass. No learned value is evicted and no physical rule or delivery schema changed.

This passes the uniform-field recurring-learning gate. Still required: arbitrary entropy, broad repeated reveal and churn, whole Battle, wasm and browser residency, and the active-frame matrix. These single runs are not a p95 distribution.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
