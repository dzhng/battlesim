# C07: bounded publication at full extents

**Depends on:** C05 and G0's chosen delivery contract; move/reslice prerequisites earlier if G0 requires them before full-map loading. **Kind:** conditional on measured budget, at any fog resolution.

## Question
Can steady-state and snapshot delivery fit their distinct budgets without hidden-state leaks or unbounded full-map copies?

## Latest measured checkpoint

The 60 s layout-7 controls at `95f7db1b` use Metro Large seed 4, map
`d8a0c783…`, rules `a59680c1…` and native/Wasm build `0666de00…`. Each arm
has 100 living units per side; late adds 20,000 corpses and 2,000 wrecks.

| Whole delivery | Early | Late |
|---|---:|---:|
| Mean / p95 / maximum B per active tick | 4,860 / 14,452 / 19,264 | 5,050 / 14,440 / 19,520 |
| Cold blue snapshot B | 440,772 | 866,496 |
| Blue resync at 60 s B | 1,058,448 | 1,511,840 |
| Red switch at 60 s B | 986,176 | 1,432,508 |
| Mean / maximum packing M instructions | 3.583 / 10.243 | 7.275 / 22.219 |

Both sampled active maxima fit the unchanged 19,800 B limit; the late margin
is 280 B. Observation construction is separate from packing. Resync costs
about 69 M instructions per arm, so it does not inherit the active-record
admission. Independent raw-word reconstruction covers all 3,602 records;
canonical group samples and the actual browser decoder agree at five ticks per
arm, with earlier views retained. These are correctness proofs, not decoder
throughput measurements. Active heap and browser copy/decoder overlap remain
unclaimed. The subsequent building-catalogue cutover needs its own identified
final admission; these controls are not relabelled as current-main evidence.

## Contract it unlocks
Implement G0's measured fog/known-prop publication representation, with the observation decoder cut over in the same commit. Possible measured arms include bounded changed tiles/masks and known-prop deltas with an initial snapshot. Sending the entire Large 8 m fog every sixth tick is not assumed sufficient; full snapshots and resubscription have their own byte/time/peak-memory budget.

The existing publication owner remains authoritative. Separate logical complete state from delivery encoding; a consumer reconstructs exactly the side's known state, including atomic building-part replacement and learned damage. Store subscriptions/cursors and snapshots in a bounded lifecycle selected at G0.

**Closes without code only when all full-extent delivery arms fit G0's budgets.** Any finer fog before this decision passes the same gate; 8 m is not an exemption.

## API seam
`sim::publication` and browser observation decoding/worker delivery. Producer/decoder layout and reset behavior are one contract.

## What the human can run or see
Matched publication tables: steady state, initial subscribe, side switch, map replacement and resubscription, split into fog/known props/other rows.

## Verification
- Native/wasm battle/replay parity; reconstructed state equals the complete-state oracle.
- Round trips include missed generation, reset, replacement, side switch, subscribe and observed-versus-hidden destruction.
- Full-extent bytes, delivery time, peak copies and decoder work fit G0's separate budgets; fog scene agreement remains ≥95%.
- A layout-only performance pass keeps digests unchanged; names any identity/encoding change separately.

## Delegated to the implementer
Packing/cursor implementation within G0's representation and budgets. A new observation scheme beyond its verdict requires reslicing.

## Must stay green
Side knowledge, exact reconstruction and one publication/decoder owner.

## Feedback that would change this slice
A snapshot or steady-state miss reopens that delivery arm, not map dimensions.

## Measured reslice: exact non-map group replacements

C05 found full-map fog delivery already sparse (~560 B/tick), while the
opening complete own rows exceed 60 KB and late corpses alone reach 522 KB.
The binding provisional steady-state allowance remains 19.8 KB/tick. C07
therefore also owns a lossless representation for the existing non-map groups;
this is an explicit extension of G0's fog/known-prop-only selected arm.

The logical observation is unchanged. Each existing group retains its canonical
array order and exact float32 words. A transport group carries its logical
length, encoding tag and payload length; snapshots carry every word, replacements
carry ordered non-overlapping `(start, length, words)` ranges. Addresses are
local to the group, so losing a soldier's variable rows cannot move corpse or
known-prop addresses. A smaller snapshot replaces dense changes. Complete
headers, fog and ground retain their existing contracts and revision checks.

A new side, epoch or resync sends complete group snapshots. Missing a generation
fails the existing fog revision check before group decoding. Both cursors advance
only after successful admission/decoding. Logical and encoded records each stay
inside the 64 MiB bound. One bounded non-map baseline and reusable logical packing
buffer live in the publisher; no new map-cell staging array or codec dependency
is introduced. Producer and browser decoder ship atomically.

## Outcome: reconstruction pass

The producer and browser decoder now share the independent group replacement
contract. Complete logical records remain the oracle (`pack_logical`); the
publisher alone emits transport records. Existing paired native/Wasm records
change only publication hashes: battle digests and delivered fog hashes stay
unchanged. Known body order, building-part replacement and side knowledge remain
simulation-owned.

Focused proofs cover complete logical group bits, side switches/resync, exact
NaN payloads and signed zero, immutable prior observations, malformed ranges and
missed generations. A live casualty stream is compared with a matching battle's
fresh snapshot every tick: shrinking member sections, growing corpse rows and a
retained known prop reconstruct the same observation. Uniform unchanged own rows
need under 300 bytes, rather than a complete own-unit resend.

This is a green representation checkpoint, **not C07 budget closure**. The
19.8 KB steady-state gate still requires the matched full-size active early/late
sample. Snapshot bytes/time, peak native/Wasm/browser overlap and browser decoder
work remain integration gates; no allowance was raised. The publisher retains
four independently bounded record buffers (canonical staging, published wire,
pending wire, non-map baseline) plus its existing fog cursor. Exact fallible
reservations bound each record buffer's requested capacity; range enumeration
uses constant scratch space. This storage bound does not itself ratify G0's peak
memory budget. Reports attribute encoded group metadata/payload bytes, not the
logical size of retained corpse and prop arrays.


## Outcome: sparse fixed-row copies

The first representation checkpoint still missed the active delivery gate:
opening mean 15,643 B, p95 45,300 B, max 61,408 B; late mean 53,744 B,
p95 473,328 B. Late corpses contributed 37,747 B on average and up to 540,300 B.
The simulation sorts corpses by place; inserting newly learned rows therefore
moved old group-local word addresses and resent retained tails. Mean bytes alone
cannot ratify the provisional 19.8 KB gate.

Fixed-row groups now also support an exact source-copy/literal representation.
The group is assembled in canonical output order from retained source spans and
new literal spans. It handles scattered insertion, removal and reorder without
changing simulation identity or ordering. Exact word replacements and snapshots
remain available; the publisher chooses the smallest ordinary word payload. Variable-section
groups retain their word representation. The layout publishes named encodings
and operation fields; producer and decoder cut over together.

A native red tracer needed 2,992 B for three additions among 100 retained corpse
rows. The new representation stays below 512 B, including header and all group
metadata, and reconstructs every float bit. Native removal/reorder and browser
insertion/removal/reorder tests preserve exact canonical values, NaN payloads,
signed zero and earlier views. Invalid source bounds/alignment, literal lengths
and incomplete assembly reject without consuming the generation; a corrected
retry applies. Existing paired battle/fog/wire hashes remain unchanged, while the
layout metadata explicitly names the new arm.

Copy-operation enumeration uses constant space. Its temporary exact-reserved
index holds one 32-bit source address per prior fixed row: at most 12.8 MiB in
aggregate under the existing 64 MiB logical allowance and smallest fixed-row
width. Index sorting is bounded O(rows log rows); contiguous retained spans are
compared directly after their first lookup. Allocation and complete record
admission still precede transport commit.

This remains a green representation checkpoint, **not budget closure**. Root's
matched active sample must establish bytes and instruction cost after integration;
p95/max, cold/side/resync snapshots, genuine newly learned bursts, peak overlap
and browser throughput remain visible gates. No information is dropped and no
byte, snapshot or memory allowance is raised.


## Outcome: variable word spans

Fixed-row copies remove retained corpse tail resends, but current layout-6 active
Large contact still averages about 17 KB while p95 is about 48.5 KB and max
66.7 KB. A route/member-section length change shifts own-group addresses; one
public tracer resent 53,568 B after one squad changed route among 80 squads.
Bitmask and XOR-varint estimates still reached 43,836/50,672 B for own rows.

The existing copy grammar now declares `copyAlignments` per group: complete
fixed rows retain their prior alignment; variable groups copy arbitrary word
spans. One span enumerator uses exact eight-word old anchors sampled every eight
words, extends matching spans wordwise, and emits literals between them. There
is no unit identity, hash collision assumption, second baseline or compression
dependency. Snapshot and replacement remain alternatives; complete preflight
selects the smallest ordinary word payload before writing or advancing any cursor.

Matched standalone codec replay of 450 captured Metro Large/layout-6/seed-4
contact transitions (ticks 0–450, same frozen simulation/rules) measures the arm
without another battle run. Bytes include each group's metadata. Instructions
include identical probe input staging copies in both arms; they exclude production
logical packing, observation construction, other groups, the battle step and
browser decoding. A separate bitwise reconstruction check passes every captured
own and identified transition.

| Group | Before mean / p95 / max B | Word spans mean / p95 / max B | Before mean / p95 / max M instructions | Word spans mean / p95 / max M instructions |
|---|---:|---:|---:|---:|
| Own | 7,128 / 27,200 / 51,968 | 4,170 / 8,556 / 10,656 | 0.288 / 0.362 / 0.432 | 1.840 / 2.937 / 3.401 |
| Identified | 290 / 660 / 2,200 | 248 / 436 / 840 | 0.022 / 0.026 / 0.038 | 0.065 / 0.095 / 0.166 |

The public tracer now needs 376 B and reconstructs every own float bit against a
fresh publisher snapshot. Browser proofs cover a live producer route edit,
fresh-snapshot equality, explicit arbitrary-word copies, route insertion/removal,
malformed operation retry, missed generations, resync and retained earlier views.
Canonical fixture vectors are unchanged; only copy-alignment layout metadata is
added. Existing paired battle, wire and fog identities stay pinned.

An index holds one exact-reserved u32 per eight old variable words. Across all
groups its bound stays at 12.8 MiB, since the smallest fixed row is five words
and all indices partition the single admitted 64 MiB baseline. Sorting is
O(oldWords log oldWords); each candidate scan visits at most newWords positions,
with an eight-word binary lookup or direct matching extension. Planning and
emission repeat the bounded scan without retaining edit lists. Complete encoded
size admission and fallible reservations still precede publication commit.

The measured maximum own burst falls about 80%, at an explicit additional
1.55 M mean encoding instructions per transition. This is an accepted delivery
tradeoff, **not closure of the 19.8 KB whole-record gate**. Actual 60-second early
and late p95/max, complete packing/tick cost, snapshot/peak overlap and browser
throughput remain the integration owner's admission checks. No allowance changes.


## Outcome: unchanged static decoded views

`ObservationDecoder` now owns one group baseline containing the existing word
buffers and the immutable decoded corpse/known-prop arrays. An unchanged static
word buffer reuses its array, rows and coordinates after validating the current
header count against the complete fixed-row payload. Changed words or counts
construct new views; epoch/side invalidation starts fresh. The baseline commits
only after all groups, fog and ground have decoded successfully.

Unchanged static delivery constructs no per-row wrappers, field closures, section
objects, final views or coordinate arrays. With 20,000 corpses and 2,000 known
props, this avoids rebuilding 22,000 decoded rows and their 24,000 coordinate
arrays per unchanged frame. The decoder retains only the latest two static view
arrays alongside its word baseline; no complete observation or second word copy
is retained. This is a construction contract, not a heap-byte or throughput
measurement. Changed static groups still rebuild their complete view arrays.

Array/row identity, own-row changes, static word/count changes, retained prior
views, malformed cached counts, late record failure and corrected-generation
retry are pinned. A count-check deletion mutant fails. Side invalidation discards
stale snapshots and opens a fresh baseline. Existing consumer inspection found
no mutation of these returned static views; static array types are readonly.
Producer schema, canonical values and battle/publication identities are unchanged.
Browser allocation/peak/throughput and the 19.8 KB whole-record gate remain open.


## Outcome: stable static identities through the pose feed

`ObservationFeed` now converts corpses only when the immutable decoded corpse
array changes, retaining one converted fallen list. A changed floor/position
view rebuilds it without altering earlier input or converted rows. `PoseDriver`
already gates reconciliation on fallen-list identity; its existing owner still
advances death, blend, fade and expiry every frame and resets on clock rollback.
No second driver cache or cap/timing rule is introduced. The session's existing
known-prop JSON key is memoized by known-prop array identity at its one owner.

A bounded synthetic construction probe sends 50 fresh observations sharing
20,000 frozen corpses, with no living units and the normal pose cap. The old feed
builds 50 lists/1,000,000 converted rows and reads source positions 1,000,000
times; the new feed builds one list/20,000 rows and reads positions 20,000 times.
Capped corpse pose values remain unchanged throughout. This counts construction
and input reads, not heap bytes, retired instructions, frame time or GPU work.

The public feed tracer fails before the change and passes with stable array/row
identity, frozen input and changed-floor reconstruction. The existing feed/driver
suites preserve death/fade/cap/reset behavior. Producer, decoder, world, camera
and frame orchestration are unchanged. No upload or visual-throughput verdict is
claimed; browser admission and whole-record budgets remain open.


## Outcome: compact lossless group carriers

Post-span-copy full-Metro contact still exceeds the provisional **19,800-byte
whole-record gate**: early p95/max are 20,284/26,260 B and late 20,344/25,356 B.
The peak includes substantial fog and ground delivery; shortening own literals
alone with XOR varints is insufficient. A captured full 1,800-transition stream
per window supplies a matched encoding experiment without another battle run.

One optional compact arm serializes the existing selected snapshot, replacement
or source-copy form. Its grammar lives in the producer's `groupDelivery.packed`
layout. Span selection, fixed-row alignment and the eight-word variable index
remain the same. Small integers use variable-length bytes; literals retain all
32 float bits, choosing raw bits or their exact XOR with the old word at the
output address. Cold snapshots use raw bits only. The producer selects compact
storage only when smaller, measures both sizes in the same operation traversal,
and writes directly into its already admitted output. The ordinary form wins
first; this does not search for a global minimum across all packed forms. There is no byte staging
buffer, second baseline, entity-specific predictor or compression dependency.

A standalone exact codec replay across all groups of both frozen corpora leaves
fog/ground bytes intact and reconstructs every original raw word. Excluding cold
tick zero, whole-record bytes change as follows; p95 uses floor((n−1)·0.95).

| Window | Before mean / p95 / max B | Compact mean / p95 / max B |
|---|---:|---:|
| Early | 8,830 / 20,284 / 26,260 | 4,796 / 14,344 / 19,772 |
| Late | 9,307 / 20,344 / 25,356 | 5,025 / 14,296 / 18,220 |

Separately, the actual prior and new producer functions replay all captured own
and identified groups with exact reconstruction. Identical probe staging copies
and index sorting are included; logical observation packing, the battle step and
browser decoding are excluded. Fresh snapshots now visit every literal while
counting compact storage; active counters do not establish cold startup cost.
Own mean instructions rise from 2.210 to 2.312 M
early and 2.271 to 2.398 M late; identified rises from 0.121 to 0.134/0.135 M.
A selected-payload-only counter is smaller and is not used as full packing cost.

Transient indices retain the prior aggregate 12.8 MiB bound. Compact count/write
uses constant scratch, preserves the existing complete 64 MiB logical and wire
admission, and adds no allocation beyond the already reserved output. Bit counting
is bounded by the logical record and operations; no extra sparse-index search
pass is introduced. Packed carriers may have NaN bit patterns: browser decoding
reads a u32 alias, never float numbers, and writes reconstructed u32 bits. Existing
unchanged nonempty zero-payload replacements retain their original word/view
identities. Empty snapshots recreate empty views. Compact packing applies to the
selected ordinary form; it does not guarantee the smallest compressed form
across all alternatives.

Public cold delivery is strictly smaller and matches its complete logical oracle.
Special float proofs include subnormals, infinities, signed zero and distinct NaN
payloads; a NaN baseline is used to recover a finite residual value. Malformed
forms, tags, varints, truncation, source alignment/bounds, ranges and padding leave
all cursors available for a corrected generation. Epoch/side/resync and retained
views keep their existing contract. Paired native/Wasm emission and real worker
copy/transfer proofs pin the raw carrier seam. Canonical fixture values, digests
and fog identities remain unchanged; transport identities and its layout change.

These frozen-stream maxima justify this representation arm, **not city-scale
closure**. Early headroom is only 28 B. Root still owns matched live early/late
whole-record admission, packing/step cost, full snapshots, peak overlap and browser
throughput. No quantization, information masking or allowance increase occurs.

### Integration with current main

The newer combat stream joins the paired transport gate. Compact serialization
changes 78 of its 80 wire hashes; every battle digest, fog hash and input is
identical. Native and Wasm replay the same firing/impact stream. The authority's
external module fixture now implements the required public-world handoff, returning
no world for its raw-carrier test. Combined authority, observation, delivery and
preparation checks pass; the optimized native/Wasm engine identity also matches.

## Historical layout-6 delivery and requested heap

The combined prototype-map/rule identity and cost control live in
[C05](C05-measuring-tools.md#frozen-layout-6-combined-contact-measurement).
These records precede current admission and preserve separate workload identities.

| Encoded delivery | Early | Late |
|---|---:|---:|
| Approximate mean B/tick | 17,144 | 16,781 |
| p95 B/tick | 48,600 | 48,540 |
| Max B/tick | 66,368 | 66,668 |
| Corpse mean / max B | 12 / 12 | 52 / 1,268 |
| Other rows mean / max B | 15,760 / 56,392 | 15,360 / 55,312 |
| Cold initial blue snapshot B | 452,836 | 952,120 |
| Red resubscription at 60 s B | 1,195,300 | 1,711,004 |

The sparse corpse copies removed the retained-tail amplification from this arm,
but other rows still missed the provisional 19.8 KB/tick limit. Complete snapshots
include exact fog and ground state; their packing at 60 s reached 71–76 M
instructions. Browser copy/decoder cost and peak overlap were not established
by this control. Raw reports live in `throwaway/scale-lane/`.

This historical layout-6 full-world delivery uses Metro Large seed 4 (`33bbd0d9…`),
13,006 buildings and 22,577 parts, with the central `city-arena-1` stress
recipe, 100 living units per side and battle seed 4. Each arm runs 60 s;
late adds 20,000 corpses and 2,000 wrecks. A scratch requested-allocation
counter measures native heap separately from clocks and instruction cost.
Its steady wire costs are the pre-compact rows in
[the captured codec comparison](#outcome-compact-lossless-group-carriers).

| Historical full-world delivery | Early | Late |
|---|---:|---:|
| Cold blue snapshot B | 452,692 | 946,972 |
| Blue resync at 60 s B | 1,044,468 | 1,579,216 |
| Red switch at 60 s B | 990,320 | 1,500,996 |
| Active peak requested heap B | 399,905,882 | 420,671,630 |
| Peak with retained blue/red publishers B | 401,825,004 | 423,182,142 |
| Rounds / final digest | 67 / `f7397266251bd45b` | 438 / `ce9463bc51b5b494` |

Native requested heap stayed below the 4 GiB ceiling on this workload. It excludes
allocator-internal reallocation overlap, browser transfer/decoding and GPU
resources. Profiling allocation hooks and loaded clocks are not throughput
admission. The provisional 19.8 KB whole-record gate was red at p95/max;
no budget is raised. Browser startup/peak overlap and full generated early/late
throughput were not established by this control.

## Outcome: packed learned ground runs

Source `13beacf9`, engine `6b3eb26d…`, current layout-7 catalogue `6b0a5e8b…`
produces native/Wasm-identical Metro Large seed-4 map `7ea9ba8a…`: 12,887
buildings, 22,442 authored parts, 228,030 bays and 46,813 ground points, within
existing generation limits. The plain-System early city-arena-1 arm completes
9,000 ticks: whole wire mean 5,896.323 B, p95 14,184 B, maximum 33,496 B;
packing mean/max 3.582/10.909 M instructions; Observation 7.224/10.278 M.
Cold blue is 440,356 B/2.298 M instructions; final same-side blue resync is
3,283,044 B/183.109 M, red switch 3,284,512 B/177.970 M. Digest ends
`89458a3615b14cd5`, with 147 living units, 330 corpses, ten wrecks and 9,440
rounds. These snapshot costs are excluded from active statistics.

The frozen late arm stops at a navigation panic; only 4,101 complete records
(ticks 0–4,100) survive. Its partial active mean/p95/max is
6,043.860/14,008/21,728 B. It is **not five-minute admission**. Neither passing
means nor p95 close the unchanged 19,800 B whole-record maximum gate. The early
worst record at tick 5,082 is 4,332 B fog, 21,008 B learned ground (1,313 runs)
and 8,156 B other delivery. Late partial worst tick 1,662 is 3,552/9,168/9,008 B.

The selected correction changes only the final ground tail. The existing
publication layout declares its sole grammar; canonical four-word runs remain
the oracle. Ordered tile deltas and masked nonzero marks use the existing raw
carrier/count/write primitives without a ground baseline, new producer buffer,
dependency or deferred knowledge. Absolute and tile-delta integer-varint arms
still leave early maxima 22,992 and 20,432 B; masked marks estimate 18,656 B
(including a conservative form byte). Actual count+emit reconstructs every raw
word in the 9,001 early and 4,101 partial late captured records, costing early
mean/max 0.012/0.168 M instructions. This is codec replay, **not a fresh whole
battle**. Final production delivery needs both complete arms after the unrelated
navigation fault is fixed.

The public native burst first fails at 38,548 B for 2,380 learned runs, then fits
the delivery allowance while reproducing every canonical mark/run and digest.
The browser shares one raw-u32 reader between groups and ground, admits both the
canonical record and minimum encoded run size before allocating the sole owned
run array, validates padding/order/edges/marks, and commits cursors only after
the entire frame succeeds. A raw NaN carrier reconstructs finite exact rows;
malformed tails permit corrected same-generation retry and preserve prior views.
Paired fixtures change only ground delivery metadata and eight wire identities;
canonical vectors, battle digests and fog identities remain unchanged.

Separate frozen-browser diagnostics replay captured records through Chromium's
Float32Array copy and production decoder without a scene/GPU: early decode
mean/p95/max 1.136/1.5/3.4 ms, cold 8.9 ms, final blue/red 4.8 ms; partial late
1.646/3.4/7.1 ms, cold 15.7 ms. Clocks are diagnostics, not load-independent
instructions; this excludes actual Wasm-source copying and worker transfer.
After early snapshots, two credits occupy 6,567,556 B and replacement overlap
reaches 7,007,912 B. Active old/new non-map buffers reach 144,856 B early and
1,235,800 B partial late; current/previous fog reaches 390,632 B. After GC,
Chromium reports JS used/backing storage 2.312/13.039 MB early and
4.034/2.035 MB partial late, excluding released captured-file staging. These
retain current/previous observations, not arbitrary consumer histories.

The separate short allocator probe includes setup/build input overlap but no
stream/oracle copies: early build peak 361,271,607 B; post-input-drop battle
264,759,368 B; cold publisher adds 1,149,352 B. Late post-input-drop battle is
277,866,447 B; cold publisher adds 2,567,772 B. These are cold/tick-one live
allocation components, **not five-minute peak, process RSS, Wasm/page/worker,
GroundView or GPU admission**. Full active peak and real worker/renderer overlap
remain open. Raw evidence lives in ignored `throwaway/scale-lane/admission-13-*`.


The emitted captured-tail transform (without the conservative form-byte charge)
reaches early mean/p95/max 5,403.171/10,860/18,652 B, partial late
5,537.118/10,384/15,280 B. Current decoder replay reproduces all captured frames
and fresh snapshot samples; final early blue/red snapshot wire becomes
1,277,804/1,284,268 B without changing canonical runs. Two credits occupy
2,562,072 B; maximum replacement overlap is 3,002,428 B. Post-GC JS used/backing
storage is 2.327/9.039 MB early; these are bounded captured components, not
full-world admission.

A separate warmed Chromium process probe stages capture parsing and oracles
outside counter brackets and reads the renderer processes' macOS retired
instructions/CPU nanoseconds. Copy-only costs 30.873 M instructions over 9,001
early records and 22.801 M over 4,101 partial late records. Decoder-only costs
184.279 B / 118.252 B total (about 20.47/28.83 M per record), including browser
dispatch, V8 allocation and GC. Combined final blue/red copy+decode costs
261.976/241.067 M instructions; warmed-process cold-blue first-epoch decode costs
43.104 M early and 87.850 M partial late. Source/Wasm publication construction,
actual worker transfer, learned-ground application, rendering and GPU remain
outside these brackets. This is process CPU evidence, not engine cold startup or
an isolated before/after decoder optimization claim.

### Warm cover-facing parity correction

The current generated battle first diverged between Native and Wasm at tick 30
while its published float32 words still matched. Ordered state diagnostics
isolated one squad's cover-facing yaw: Native `3fc58dcf04f55e57`, Wasm
`3fc58dcf04f55e58`, from the same bearing `(17, 100)`. Positions, member fields,
world props and projectiles matched. Cover's existing bearing calculation now
uses the already pinned `libm::atan2`; this is a named Native last-bit state
correction, with no publication schema or mechanics change.

A two-squad inline-map record exercises the public Battle and paired publication
owners through construction and thirty ticks, including side changes and resync.
The actual pre-fix Wasm stream matched Native through tick 29 and failed its
full-state digest at tick 30. Float32 publication alone would not detect this
regression. Fresh Wasm passes the paired file; Native publication and cover
checks, clippy and formatting pass. Existing paired records are unchanged. Whole
generated-battle cross-target and scale admission remain open.

### Ricochet scatter parity correction

The fresh layout-9 battle next diverged at tick 60, after matching Native and
Wasm through tick 59. A projectile's first ricochet produced different float64
velocity while its published float32 words matched. Temporary paired captures
proved identical impact point/normal/velocity, hull, rules, RNG state and scatter
basis. The first differing intermediate was the azimuth cosine at
`0.6995236543718834`: Native `3fe87c19b1c1f765`, Wasm `3fe87c19b1c1f764`.

The existing deflection primitive now uses pinned `libm` for all its cosine/sine
evaluations, keeping the eligibility and two scatter draws, cone, minimum angle
and speed rules unchanged. A public `damage::decide` regression supplies the
captured numerical inputs and fixed rules, pins portable bounce-velocity bits
and RNG state, and fails on the old Native evaluator. The initial two-unit
paired-battle candidate passed old Wasm and was discarded as unearned. No
permanent diagnostic API or new parity format is introduced. Whole generated
Native/Wasm parity and five-minute scale admission remain open.

Fresh optimized Native/Wasm builds match every digest and raw publication bit
from construction through tick 60 on the frozen layout-9 input. Corrected Native
tick 60 is `025ee91951e30912`, matching old and fresh Wasm; prior ticks and wire
identities are unchanged. Native ricochet, flight-collision and publication
checks, the actual Wasm publication/decoder boundary file, clippy and formatting
pass. Independent read-only review is clean. This is a sixty-tick causal proof,
not whole-battle or throughput admission.

### Shared rotation parity correction

The per-tick admission gate next stopped at tick 175: only a squad's suppression
first differed in persistent state, with unchanged float32 publication. Paired
transient event capture showed impact-normal and hit-time differences already
at tick 174, despite matching state digests. Identical prop poses at yaw
`-1.785965` produced different sine components; a shifted impact endpoint then
changed a near miss's distance and suppression. Neither suppression nor distance
is rounded or given an exception.

The existing cached `Rotation` owner now uses pinned `libm::sincos`, preserving
one argument reduction and the same application arithmetic for cached transforms
and `V2::rotated`. Public Native `WorldGeometry::raycast` is old-red/new-green
on one rotated wall; actual Wasm `WorldView` checks the same finite float64 normal
bits directly, so float32 exports cannot hide the fault. The wall exercises the
query's existing requirement that a body stops rounds.
No new API, wrapper or codec is added, and unproven hypot differences remain
outside this correction. Full five-minute admission remains open.

Fresh optimized Native/Wasm builds match every authoritative digest and raw
publication bit from construction through tick 175 on the frozen layout-9
input. Corrected Native tick 175 is `cfa643113f4b1b86`, matching old and fresh
Wasm, with prior state/wire identities unchanged. Native geometry, collision and
publication checks and actual Wasm world-query/publication boundary checks pass.
This is a bounded causal proof; full early/late state, delivery, memory and
throughput admission still require the per-tick-gated five-minute runs.


### Portable authoritative evaluators

The next gated run agreed through tick 212, then diverged only in retained
structural damage at tick 213 while publication bits still matched. Captured
blast point, footprint pose, local coordinates and outside-distance components
were identical. `Obb2::distance` evaluated the same `hypot` inputs as Native
`401e7a0ec2255684` versus actual Wasm `401e7a0ec2255683`; pinned `libm` matched
Wasm. That changed blast damage to prop 37790 and its retained integrity. A small
public rectangle-distance regression is old-red/new-green on those inputs.

After repeated measured failures across construction, aiming, ricochet and
geometry, authoritative sine, cosine, paired sine/cosine, bearing and 2D norm
evaluations now consistently use the already pinned software library. This is a
named Native precision decision across simulation and shared physical contracts,
not a performance optimization. Arguments, formulas, arithmetic order, RNG
consumption and rules are unchanged. Existing combined rotation pairs remain
combined. Generation already used the selected library; its remaining standard
trig calls are test-only. Other numeric families are outside this decision.

The parent's separate forest-density reversal changes the current rules input.
The original tick-213 evidence remains a frozen causal control. Current paired
publication records were refreshed for that input; only authoritative digest
leaves changed, with float32 publications and fog identities unchanged. Temporary
diagnostic state exports and event instrumentation were removed. The full
five-minute state, bandwidth, memory and browser gates remain open.

Fresh optimized Native/Wasm builds agree on every digest and raw publication bit
from construction through tick 213 on the original frozen input. Corrected
Native tick 213 is `de32e92713f65f6c`, matching the prior Wasm result; all earlier
state and publication identities are unchanged. Current-input Native publication,
geometry, collision and the three admitted town-journey proofs pass, alongside
actual Wasm world-query/publication boundary checks. This closes the captured
numerical fault, not current-input five-minute workload or cost admission.
