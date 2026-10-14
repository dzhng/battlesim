# 05 — Carrier velocity

**Delivers:** D21.

**Unlocks:** every round fired or dropped from an aircraft starts with the aircraft's velocity.

## Seam
- `flight::Launch` gains `carrier: V3`, filled only by airborne shooters from one source, `Unit::flight_velocity() -> V3` (`flight/solve.rs` ~540–636).
- **Gun aim** is solved in the shooter's frame.
- **Motor rounds** keep their thrust cap relative to world speed. Record the choice of `top_speed` against launch speed in `choices.md`.
- **Measure the launch-solve cache** (`solve.rs` ~79–108, keyed on exact floats) in instructions retired, before and after. Moving shooters make every key unique, so record the cost. If it exceeds the budget, simplify the cache rather than adding machinery.

## Tests (red first)
- `a_round_fired_from_a_jet_starts_with_its_velocity`
- `a_ground_shooters_round_is_unchanged`

**Ground battle digests must not change.**

## Firewall
No row changes, and no bombs (slice 07).
