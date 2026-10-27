# 11: Swamp slowdown per drive

**Lane:** sim. **Depends on:** 10. **Visual variable:** none. The evidence is a
battle report.

## Contract

- **Rules row:** `movement.swamp { foot, wheeled, tracked }` (decision 3).
  `MovementRules` gains `deny_unknown_fields`. Every movement multiplier,
  forest's included, is admitted only in (0, 1]. That one rule also fixes the
  found bug where a 0 divides by zero in `SlowGround::stretch`.
- **Mobility:** `ground_mobility` (`crates/sim/src/units/motion.rs`) resolves
  the row by drive into `Mobility.swamp_multiplier`, as it does for
  `forest_multiplier`. Planning never reads `Drive` itself.
- **One combination:** `Mobility::speed` applies road (which always wins),
  else the slower of forest and swamp, then slope, then craters for vehicles
  (decision 4). One `Footing { road_factor, forest, swamp, slope_deg }` value,
  built from a `Surface` or a `Cell`, replaces the positional arguments at all
  five read sites:
  - vehicle movement, `movement/mod.rs`;
  - soldier movement, `movement/soldier.rs`;
  - `cell_cost`, `navigation.rs`;
  - road arcs, `navigation/journey.rs`;
  - the estimate, `SlowGround::stretch`.
- **`SlowGround`:** counts each tile once by class (open, forest, swamp, or
  both), so an overlap is never counted twice.
- **Deleted:** the private copy of `speed` in
  `examples/navigation_quality/dense.rs`. The example uses the real function.

## Seam tests

These use test rows with extreme factors, never the shipped numbers.

- each drive is slowed by its own factor;
- aircraft ignore swamps;
- a road across a swamp keeps road speed;
- where forest and swamp overlap, the slower factor applies once, in both
  orders, in movement and in planning;
- depth never shapes speed: rim and centre are equal once slope is divided out;
- the planner goes round a swamp when dry ground is faster;
- a zero multiplier is refused when the rules load.

## Evidence

A paired battle report on the `wetlands` map, before and after. Record the
shift; don't retune in the same pass (`tweak-mechanics`).

## Re-recorded, as the one named digest decision

The rules hash changes for every battle. Re-record `fixtures/parity/*`,
including `publication/` and `capture/`.

## Native

None.

## Delegated to the implementer

- the first shipped factor values (tuning, not requirements);
- `Footing`'s exact shape.

## Must stay green

Replays are deterministic. Every movement scenario except those that now pass
through a swamp.
