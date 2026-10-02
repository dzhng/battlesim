# C07: bounded publication at full extents

**Depends on:** C05 and G0's chosen delivery contract; move/reslice prerequisites earlier if G0 requires them before full-map loading. **Kind:** conditional on measured budget, at any fog resolution.

## Question
Can steady-state and snapshot delivery fit their distinct budgets without hidden-state leaks or unbounded full-map copies?

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
remain available; the publisher chooses the smallest payload. Variable-section
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
selects the smallest payload before writing or advancing any cursor.

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
