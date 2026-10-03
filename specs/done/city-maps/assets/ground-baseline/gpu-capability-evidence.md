# Ground GPU capability

The GPU ground source stores exact learned values in one bounded sampled-word pool and samples them bit-for-bit like the original 2D texture. It passes a fixed-extent resource matrix on Apple Metal. Other hardware, arbitrary entropy and complete G0 and visual acceptance stay open. The [frozen oracle](README.md) is unchanged.

## Ownership

The CPU learned receiver keeps every five-byte ground value in lossless tile pages. The GPU source owns one 8 MiB sampled-word pool and a directory that grows from eight bytes up to the fixed-extent bound. Each tile is an exact uniform word, a run block, or 256 dense words. A cache uses the most common word as its default and stores every differing page, including holes; a complete global cache is admitted only when all exceptions fit the pool, otherwise drawing falls back to bounded regions. Clearing projects as `tracks = max(raw tracks, cleared)` only at the consumer; native and wire bytes stay raw. Every four-neighbor tap resolves through this owner across representation and tile joins. GPU residency never changes side knowledge.

Foliage records carry exact axis coordinates, not float global cell ids. Learned clearing crosses the wasm seam as sorted tile spans; no full foliage clone or world-sized clear mask exists. Missing tiles are found from one occupancy bit per tile (158 KB at 18 km), with no per-cell values.

## How the representation got here

A first materialized halo-atlas cache took 760 ms tactical and 11.6 s overview CPU per unchanged frame at 18 km, uploading up to 2.1 GB. Uniform directory words cut that to 29 and 361 ms but still rebuilt directory work per region. The common-word cache with exceptions brought a fully learned 18 km field to one cache and one draw: unchanged frames at 0.7–0.9 ms CPU with zero upload. The sampled-word pool then held 20,000 distributed varying pages in one cache (1.0 MB first upload, zero repeat), where the regional candidate had taken 316 ms and 34.7 MB per unchanged overview.

Review found and fixed three defects, each with a red regression: allocation before reclamation could reject a capacity-preserving tile swap; directory growth left stale bindings; and a fitting but fragmented pool failed to allocate (fixed by bounded largest-block-first compaction).

## Exact sampling

Cell-centre values were exact from the start, but between cells manual reconstruction differed from hardware filtering (up to 0.0029 linear, 0.0014 cubic). Closing that took three discoveries:

- Metal uses Q8 coordinate weights and Q4 byte-colour rounding. Hardware matches correctly rounded CPU normalization exactly while shader division is one ULP off, so a 16 KiB normalization table lives in the pool; churn and compaction cannot overwrite it.
- A local-window reference rounds `uv*size` before subtracting its origin, unlike the original direct call, so it cannot certify the original coordinate path. Against a full dense original texture, the optimizer cancelled a cubic tap's division and multiplication; observing the normalized bits first removes it. Sampled axes are admitted up to 65,536 so each Q8 coordinate stays exact.
- Three cubic outputs were still one ULP off; making the outer fused multiply-add explicit preserves the hardware contraction.

The final query checks 2,156 queries across four channels, including 35 run, row and page joins, against original 2D hardware linear and cubic values: zero error, zero ULP difference. A known coordinate mutant fails eleven channels before the restored source passes.

Initialization runs a cached, finite hardware compatibility probe before grass or frames read the source. It describes the tested device only; it is not a sampler or compiler guarantee, and attempts to make it a compiler guard stayed green on a known mutant and were dropped. A nearest-sampler initialization fails, releases every allocation, and a compatible retry passes. Other-hardware fallback belongs to C07.

## Resource matrix

Nine production BattleFrame arms ran serially on Apple Metal-3 at 1280×720 with native world exports and the C02 map fog field: uniform all-learned clearing; that plus eight local edits and a missing tile; and that plus 20,000 distributed single-edit pages. Steady p95 complete frame time:

| Extent | Uniform tactical / overview | Mixed tactical / overview | 20k pages tactical / overview |
| --- | --- | --- | --- |
| 12 km | 8.3 / 4.2 ms | 6.5 / 5.2 ms | 6.6 / 7.3 ms |
| 15 km | 5.6 / 4.2 ms | 6.5 / 5.4 ms | 6.1 / 7.9 ms |
| 18 km | 5.7 / 4.2 ms | 6.1 / 5.8 ms | 6.5 / 7.3 ms |

Every steady frame uploads zero bytes; the worst observed steady frame was 10.1 ms. At 18 km with 20k pages, CPU p95 is about 0.4 ms and the first frame is 87 ms with a 1.0 MB upload. Frame textures total 216 MB; receiver payload is 9.1 MB and heap after application 388 MB, measured separately rather than as an overlap peak. Validation lists and final allocation counts are empty. No learned value was evicted, withheld or sampled lossily.

## Production pixels

The production ground scene passes its thirteen original content, knowledge and grass checks plus initialization, retry and sampled-value checks, with unchanged bars (crater colour difference 121 against >60; learned mark 70/0 against >24/<6). Seventeen paired captures differ by at most 0.0055 diagnostic distance. Unprimed critiques found only historical limitations: weak tracks, soft grazing craters, vegetation in damage, repeated crater fields and infantry readability. Large overview shots are too blurred to judge sampled values; the direct query proof above does that.

## Open

A 20k single-edit corpus is not 20k full-entropy pages, arbitrary all-cell entropy, an active battle or an overall 4 GiB admission. Native learning and transport pressure belong to the [learning work](../ground-learning-work/README.md) and [transport](../ground-transport/README.md) evidence. Source churn at maximum cardinality, other devices, water and motion, and complete G0 and visual acceptance remain open.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
