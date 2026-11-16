# S2 — queued orders admitted behind attacks; queued orders take effect at their turn

**Unlocks.** C1, C2 (decision n).

**Seams.**
- `battle/move_placement.rs`: delete `after_attack`. `from` already plans from the last
  queued move's end or the unit's position (decision 1).
- `UnitOrder::Attack` gains a "waits its turn" mark, set when it joins a non-empty queue.
  `apply_units` sets fire at will and the dogfight target only for an attack that takes
  effect now; `attack_order.rs` applies them when a waiting attack reaches the front
  (before the fixed-wing and garrison skips, so jets get their dogfight at their turn).
  Folded into the digest only when set. (A queued AttackMove still chooses fire at will
  when queued: see `choices.md`, p.)
- Docs: `NoValidDestination` doc in `crates/contract/src/command.rs`.

**Tests** (`crates/sim/tests/order_queue.rs`, end to end, test units):
- a move queued behind an attack on a unit runs once the target dies;
- a queued attack leaves the unit's engagement unchanged until its turn;
- a jet's queued attack on an enemy jet starts no dogfight while it flies its move.
`move_admission.rs`: a queued Move / AttackMove / Unload-to-a-point behind an attack is placed.

**Note.** Behind an attack-ground, a queued move now waits forever; S3 resolves it.

**Commit.** `feat(sim): orders queue behind an attack and take effect at their turn`.
