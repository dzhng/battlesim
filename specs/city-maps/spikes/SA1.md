# SA1: bounded exact terrain verdict

**Status:** terrain/export owner proved and implemented; complete G0/S1 remains open. Evidence is frozen under [terrain-baseline](../assets/terrain-baseline/README.md).

## Selected contract

The native height field stores only nonzero vertex pages, with zero as the exact implicit baseline. Height/normal/ray queries keep the original 4 m samples, southwest-to-northeast triangle rule, relief summation and water precedence. A cached mesh coalesces only rectangles whose sampled height is identically zero; nonzero page boundaries and water classification regions retain cell detail. Public sampled heights are independent of mesh vertex ordering.

The wasm public export now carries a grid descriptor (`nx`, `ny`, `spacing`, `pageSize`, `minHeight`), sorted vertex-page IDs and row-major f32 samples within each present page. Picking, scenery, fog and grass consume that one sampled surface. Fog and grass share one immutable GPU height buffer; the directory and height bits occupy one existing storage binding. This is a hard consumer cutover, without a dense-grid fallback.

## Resource evidence

| Arm | Height sample export | Mesh positions + indices | Packed draw | GPU height buffer |
|---|---:|---:|---:|---:|
| Flat 12/15/18 km | 0 B | 72 B | 240 B | 12 B |
| 18 km, 32 m river across map | 289,896 B | 3,195,228 B | 20,938,560 B | 606,876 B |

All four actual Metal arms returned identical native/CPU/GPU heights for 1,090 points each, including distant edges and page/river joins. Default device limits remained eight storage buffers per stage and 128 MiB per binding. No validation errors or resources remained after release.

Serial native total peaks were 193,168,146 / 312,628,659 / 427,878,726 B for flat 12/15/18 km, and 439,116,012 B for the large river. These include the old dense foliage owner and its export staging. They are **not** complete battle memory measurements. River mesh construction took 29.14 ms and triangle classification 6.42 ms in this run; wall times are descriptive, not load-independent performance guarantees.

## Parity and review

- All 2,048 frozen query records and all 1,800 village tick digests matched. Eighteen world-geometry tests, the affected web terrain/world/scenery/fog suites, typecheck and Rust clippy passed.
- Production geometry, ground and fog scenes passed on Apple Metal. The geometry comparison covers five frames/crops; ground covers seventeen. Geometry pixelmatch ratios were zero; bridge/ridge crops were byte-identical. The largest ground pixelmatch ratio was 0.00034, with grayscale MAE at most 0.11715 on a 0–255 scale.
- Root inspection and the unprimed screenshot critic found no new cracks, missing ground, detached ground features or shadow/fog confusion. Existing faceted pits, weak low-angle crater relief and faint tracks occur on both sides and stay with the deferred visual work.
- Preview showed ridge, bridge and village-scar comparisons as a non-blocking checkpoint. Source shape/diff/docs review retained one native owner and one shared GPU owner; corrected stale comments about duplicated height buffers. The configured independent CLI review was unavailable, so it supplies no acceptance evidence.

## Limits and next owners

This unlocks SA1's terrain/export representation only. It does not prove arbitrary relief density, whole-world browser startup, active battles, city residency, complete overview throughput, or selectable released appearances. Navigation remains SA2; sparse ground/foliage and bounded scar throughput remain SA3; incremental fog delivery remains SA4. Large public picking/query bounds belong to C33/C57 rather than an infinite ray endpoint. Full check/verify runs once over the integrated representation wave before merge.
