# S1 — one owner for a cycle's end; the per-mount ordered-fire report (no behaviour change)

**Unlocks.** C4 and C5, which S3 reads. Also frees ratchet room in `weapons.rs`.

**Seams.**
- `crates/sim/src/weapons/cycle.rs`: `Cycle::spend_round(def, …) -> CycleEnd`, moved
  verbatim from `weapons.rs::fire` (decrement, instant refill, burst reset, burst
  cooldown). `Ended` when the round empties the magazine (read *before* the instant
  refill — the rifle trap), lands on a burst boundary, or spends a magazine-less shot.
  Running dry (the loop clearing a cycle whose ammo kind is spent) reports `Ended` too.
- `Shot` gains `cycle_ended: bool`.
- `weapons::tick::advance` also returns, per unit with an ordered target, the set of
  mount rows firing at it this tick — assessed `Ok` **and** past the garrison window
  gate (`NoFacingSlot`, `ChangingPosition` don't count; reloading and traversing do) —
  and whether any row can damage it (`compatible`). Exact type names are delegated.

**Tests.** A unit test in `weapons/cycle.rs` (complex boundary logic, justified): inline
weapon shapes — a burst dividing the magazine, a short last burst, an instant refill, no
magazine — assert which rounds report `Ended`. No-change proof: `exact_numbers`,
`finite_ammo`, `weapons`, `squad_aim` and the digest tests unchanged.

**Delegated.** Type and field names; whether the report is a return value or a field
`Battle::fire` hands on.

**Commit.** `refactor(sim): one owner for a weapon's cycle end and the ordered-fire report`.
