# 11: Swamp slowdown per drive

**Lane:** sim. **Depends on:** 10. **Visual variable:** none. The evidence is a
battle report.

## Contract

- **Rules row:** `movement.swamp { foot, wheeled, tracked }` (decision 3).
  `units::validate` already refuses forest multipliers outside (0, 1]
  ([PR #17](https://github.com/dzhng/battlesim/pull/17)). The swamp's three
  factors join that same check.
- **`MovementRules` gains `deny_unknown_fields`.** `crates/contract/src/scenario.rs`
  is over its file-size budget and may not grow, so first move the movement
  rules (`MovementRules`, `DriveRules`) into their own contract file, as a
  separate commit with no behaviour change.
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

**Recorded 2026-10-28** (shipped factors foot 0.5, wheeled 0.3, tracked
0.55, against every factor at 1.0; same seed). The `wetlands` water
encounter's squad (`capture_rifle`, foot) and tank (`capture_us_tank`,
tracked) each move alone from their encounter positions north-west of the
swamp to a point beyond it: through its middle to (440, 20), and past its
north-east end to (470, 70). Time is to arriving, length is the ground the
unit's centre covered. The scratch test that made these is
`throwaway/evidence/swamp_evidence.rs` (not committed).

| Unit | Route | Goal | Swamp factors | Factors 1.0 | Shift |
| --- | --- | --- | --- | --- | --- |
| squad | shortest | (440, 20) | 130.6 s, 259 m | 95.7 s, 259 m | +36% time |
| squad | fastest | (440, 20) | 111.0 s, 281 m | 95.7 s, 259 m | +16% time, +22 m round the deep part |
| squad | shortest | (470, 70) | 108.4 s, 254 m | 94.6 s, 254 m | +15% time |
| squad | fastest | (470, 70) | 103.3 s, 262 m, stays dry | 94.6 s, 254 m | +9% time, +8 m round it |
| tank | shortest | (440, 20) | 54.4 s, 233 m | 41.4 s, 233 m | +31% time |
| tank | fastest | (440, 20) | 48.0 s, 265 m | 41.4 s, 233 m | +16% time, +32 m |
| tank | shortest | (470, 70) | 43.3 s, 226 m | 39.7 s, 226 m | +9% time |
| tank | fastest | (470, 70) | 40.3 s, 235 m, stays dry | 39.7 s, 226 m | +2% time, +9 m round it |

Read: ordered straight through (`shortest`), the squad loses about a third
of its time across the swamp and the tank a little less; asked for the
quickest way (`fastest`), both mostly skirt the water and pay about half
that. No factor was retuned in this pass.

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
