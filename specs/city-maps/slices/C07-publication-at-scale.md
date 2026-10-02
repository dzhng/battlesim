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
length, snapshot tag and payload length; snapshots carry every word, replacements
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
