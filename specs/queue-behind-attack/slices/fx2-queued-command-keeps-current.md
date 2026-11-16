# FX2 — a queued command breaks nothing off (existing bug)

**Unlocks.** Shift-clicking an order onto a unit no longer cancels what it is doing:
today every queue-joining command clears `withdrawing` and breaks off a jet's run and
dogfight (`take_order`), so a carrier driving home never retires once a move is queued
behind it.

**Seam.** `crates/sim/src/battle/apply_command.rs::apply_units`. `take_order` runs for
orders that replace the queue: Stop, the setting toggles, an in-place Unload, and the
queue-joining orders only when not queued. The redundant `dogfight::clear` in the Move
branch goes (`take_order` already clears the dogfight).

**Test (written first, seen failing).** `crates/sim/tests/order_queue.rs`
`a_queued_order_does_not_cancel_a_drive_home`: a carrier ordered home, with a move
shift-queued behind it, still retires. (A jet-run version can't see the bug: a broken
run is replanned on the same tick at the same aim.)

**Stays green.** `attack_runs`, `attack_passes`, `dogfight`, `transport_refund`,
digests (no built-in issuer queues).

**Commit.** `fix(sim): a queued command keeps the unit's current order`.
