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
