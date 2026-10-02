# SA4 verdict: visibility delivery is incremental and lossless

**Verdict:** the publication representation and its browser consumer pass their isolated physical/resource proof. Unchanged visibility sends no field payload. Native authoritative visibility remains dense. Full battle/frame admission and complete G0/GG stay open under the systems handoff.

## Frozen preservation evidence

The oracle identity file pinned the original publication/decoder sources at `b538ecd` (raw evidence: tag `city-maps-evidence-2026-09-30`). The oracle (`fixtures/parity/fog/oracle.json`, at tag `data-files-before-prune-2026-10-01`) contained the original complete decoded observations and authoritative frames/digests for moving units on the geometry lab, including occlusion, side changes and resynchronization. The current native test preserves every digest and visibility word; the wasm/browser decoder test preserves every complete decoded observation and retained earlier frame. Authoritative f64 state is pinned by its digest; canonical JSON readers can round an individual f64 value, so the JSON frame is not an exact floating-point oracle.

Falsifying the decoder's copy on changed visibility fails retained tick 1. Falsifying client epoch invalidation delivers an obsolete blue tick after switching to red and fails the race test. Both corrected paths pass. The original unchanged-field test failed with equal snapshot/steady sizes before implementation.

## Contract and resource verdicts

| Question | Verdict | Evidence and limit |
|---|---|---|
| Exact gained/lost visibility and complete observations? | Pass | Frozen complete observations; native/wasm digests; high-churn wall and ordered reconstruction tests |
| Prior observation lifetime? | Pass | Later changed fields copy; unchanged fields share; frozen retained frames and mutation falsification |
| Side/resync/replacement and withheld credit? | Pass | One shared side/epoch with ground; races before/after the first callback return obsolete in-flight credit; existing authority tests preserve consecutive ticks and consumer stalls; a new decoder starts a replacement stream |
| Padding and exact indices/revisions? | Pass | Word/revision limbs are exact; all fixed extents at 2/4/8 m encode within the declared bound; partial final words have zero padding; revisions above float32's exact integer range are decoded exactly; wasm32 dimension multiplication cannot wrap past the bound |
| Initial snapshots versus steady delivery? | Pass in isolation | Native stages and the Chrome worker/decoder probe (`browser.json`), below |
| Whole active-battle publication budget? | Open | The small frozen movement workload is under the provisional target. S1/C07 must measure the actual full-size active workload; an empty field cannot ratify 19.8 KB/tick |

At the current 8 m fog profile:

| Extent | Original snapshot and unchanged record | Candidate snapshot / unchanged record | Native peak requested heap | Worker wasm high-water |
|---|---:|---:|---:|---:|
| 12 km | 562,588 B | 562,612 / 108 B | 258,416,358 B | 239,927,296 B |
| 15 km | 878,996 B | 879,020 / 108 B | 403,091,662 B | 373,489,664 B |
| 18 km | 1,265,716 B | 1,265,740 / 108 B | 580,666,734 B | 538,247,168 B |

The active comparison (`active.json`) reduces total wire bytes over the frozen run from 136,804 to 103,528. Four records open streams, 76 are deltas, 67 carry no fog payload; the largest sparse fog payload is 180 B. A high-churn change uses a snapshot within the same epoch when its indexed replacements would be larger. Snapshot headers add the cursor fields and whole-word padding; this accounts for the small initial-record increase.

Each full-size native arm was admitted against S0's 4 GiB ceiling using its frozen empty-Battle peak plus publication cursor, staging and credit overlap. The browser arm constructs only the worker Battle and production authority/decoder, with consecutive completed ticks and side switches. It does not construct a main-thread WorldView, GroundView, terrain mesh or GPU frame, so SA1/SA2/SA3 failures are still failures. Requested heap, wasm memory pages and elapsed time are distinct measurements; timings vary with machine load.

## Selected representation

The Publisher owns the last delivered visibility words and a transport revision outside Battle::digest. Every word travels as two 16-bit limbs. An indexed replacement adds one exact word index. The producer chooses the smaller of replacements and a complete field; reopening the side/epoch always sends a complete field. Ground payload values and revisions retain their contract. Indexed words earn the smaller contract here: the frozen sparse payload peaks at 180 B and unchanged ticks carry none. Run tables or a compression library would add another decoding path without resolving a failed measured budget; they remain unselected until the actual active workload demonstrates that need.

The maximum fog payload is the fixed Large extent at the finest accepted 2 m grid: 2,531,250 words and 20,250,000 bytes. Both ends reject fields beyond that declared bound. This is a codec bound, not admission of the existing dense 2 m world. The Chrome maximum-codec arm transfers no world state: it decodes a 20,250,108 B record, applies a 120 B sparse record at the highest valid word index, then replaces the field while preserving both earlier observations. The snapshot, delta copy and replacement own 30,375,000 B of retained fog words; the input records are separately bounded. Finite atomic snapshots fit the measured transport allocation allowance, so no fragment queue or partial-observation state machine is introduced. Full-size fine-grid startup/active latency remains a workload measurement for C02/S1/C07.

The consumer retains one reconstruction baseline. A changed field creates a new array, so a retained earlier observation cannot change underneath its reader; unchanged fields reuse the baseline. The client records the requested side even before its first callback. Stale in-flight observations after an explicit side switch are returned as credit without exposing old-side fog beside invalidated ground. Advance completion still follows returned tick credits. The producer's ordinary scheduling, credit pool and command order are unchanged.

Producer/consumer use one format. The wasm resynchronization method now names the entire observation stream; no old method or stateless decoder compatibility wrapper remains.

## Next use

C07 consumes this bounded publication contract and S1 measures its active workload. SA3 now shares the stream through its exact run seam; [transport evidence](SA3-transport.md) owns the paired decoder/admission verdict. Run the integrated closeout gates once after all systems commits are combined; this isolated verdict supplies no new artwork or visual acceptance.
