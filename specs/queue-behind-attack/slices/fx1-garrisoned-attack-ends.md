# FX1 — a garrisoned unit's attack ends when its target dies (existing bug)

**Unlocks.** Orders queued behind a garrisoned attack (Exit, Garrison, Board, Refund)
run. Today `update_pursuit` skipped garrisoned units before checking whether the
attack was done, so the attack never ended.

**Seam.** `crates/sim/src/battle/attack_order.rs::update_pursuit`. A garrisoned unit
still never chases, but runs the end checks: the target seen dead ends it, a lost
track ends it (it can't chase to the last known spot), a vanished contact ends it.

**Test (written first, seen failing).** `crates/sim/tests/order_queue.rs`
`a_garrisoned_attack_ends_when_its_target_dies`: a garrisoned test squad attacks an
identified enemy with an Exit queued; once the enemy is dead the attack leaves the
queue and the squad walks out.

**Stays green.** The sim crate, including its digest and replay tests (known unknown 1:
a moved digest is this commit's named decision).

**Commit.** `fix(sim): a garrisoned unit's attack ends when its target dies`.
