# Ground GPU capability checkpoint

This checkpoint proves exact sampled storage and the uniform-source correction, not the complete SA3 frame gate or deferred appearance. The original value/order/digest oracle beside this file is unchanged.

## Production ownership

The CPU learned receiver retains every five-byte ground value in lossless tile pages. The GPU owns one bounded cache: a 1400×1400 RGBA8 page texture (7,840,000 bytes) and one 1024×16 RG32Uint directory (131,072 bytes). A directory record carries either an exact uniform visible-channel word or a nonuniform page slot. Clearing projects as `tracks=max(raw tracks,cleared)` only at the consumer; native/wire bytes remain raw. Nonuniform pages preserve two cells of filter halo and the original cubic reconstruction.

Terrain color draws visit bounded world regions using the existing physical mesh. A region writes the same cache only after the preceding consumer submission. Grass preparation reads its own admitted window before terrain batches reuse the cache. Complete CPU knowledge survives camera movement; GPU residency never changes side knowledge.

Sparse foliage records carry exact axis coordinates rather than float global cell IDs. Learned clearing crosses the wasm seam as sorted tile/span pairs; no full foliage clone or world-sized clear mask is constructed.

## Measured correction

Serialized Apple Metal-3, default device, 8192 texture limit; 1280×720 production BattleFrame built from actual WorldView exports. All 324,000,000 cells at 18 km are learned cleared with raw tracks zero and projected tracks 255, including the northeast edge. Gitignored raw evidence: `throwaway/city-spike/sa3/materialized-ground.json` and `uniform-ground.json`.

| Arm | Tactical first / unchanged CPU | Overview first / unchanged CPU | Tactical / overview upload bytes |
| --- | --- | --- | --- |
| Materialized bounded pages | 759.7 / 761.5 ms | 11801.0 / 11573.2 ms | 136,014,400 / 2,149,249,600 |
| Uniform directory words | 38.33 / 28.73 ms | 373.06 / 361.10 ms | 2,555,904 / 40,140,800 |

The corrected arm uploads zero atlas texels. All GPU validation lists are empty and every buffer/texture/count returns to zero after disposal. Complete-frame time is 40.44 ms tactical first and 374.71 ms overview first. Receiver retained typed payload is 8,859,375 bytes; run input is 20,250,000 bytes; measured JS heap after application is 326,242,278 bytes. Payload estimates exclude map/object bookkeeping; the reported heap does not.

**Verdict remains red:** 20 tactical and 324 overview region rebuilds still repeat directory work. The source-compression correction is proved, while exact view admission and general directory strategy need another measured cut. Uniform all-touched evidence does not establish arbitrary-entropy admission or native peak overlap. Root publication enforces its separately evidenced atomic record bound without dropping values.

## Tests and review

Six pure receiver tests and five pure GPU-boundary tests pass. Exact 20×20 uploaded halo bytes are compared with learned cells; deliberately clamping the x halo to the page core fails. Uniform/nonuniform joins, distant eviction/reload, unchanged frames, retained deltas and clearing-only wear are covered. Independent Codex review found allocation-before-reclamation could reject a capacity-preserving tile swap; a red one-slot swap regression now passes after all demotions release slots before promotions allocate.

The earlier materialized-cache production ground scene passed the original content probes and stayed within the historical pixel tolerance. The newest uniform shader still needs the complete rebuilt run-wire production route, before/after comparison and unprimed screenshot critique. No artwork acceptance is claimed. Full native/wasm frozen parity awaits integration of the coordinated publication wire rather than a decoder paired with the obsolete wasm layout.
