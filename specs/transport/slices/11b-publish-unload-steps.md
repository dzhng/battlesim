# 11b · The simulation publishes unload steps

**Owns:** the authority half of D21. **Depends on:** 09 and 11. **Outcome change:** none. **Publication layout:** changes once more; re-record the captures (as slice 03 did).

## Why
Slice 11 draws the unload mark from `web/src/battle/input/unloadPoints.ts`, which remembers the browser's own accepted Unload points from `CommandAck.placement`. That makes the browser a second owner of what a unit's queue does. The marks vanish after a reload, in a replay, and in native. The simulation is the one authority: it must publish which queued step unloads.

## Contract
- Each own unit's published goal and queue marks every step that ends in an unload: an `unload` flag, or an action kind on the step, matching how queue steps are published today (`crates/sim/src/publication.rs`, the own row and its `GROUP_LAYOUTS`).
- `observationView.ts` exposes it. The order line reads it.
- Delete `unloadPoints.ts`, its tests and its wiring.

## Verification
- A sim test: a queued drive-unload-drive publishes the unload flag on exactly the unload step, and the flag clears when the step completes or is refused.
- unitControl and order-line tests read the flag from the observation.
- Re-record the captures from one new `native-parity-capture` run (fixtures/README.md, Paired records), and re-bless the derived records.
- The transport scene's `controls` variant passes. The unload mark is pixel-identical to slice 11's.

## Delegated
The field's name and encoding.
