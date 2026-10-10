# 03 — Flight: route, altitude, separation

**Status:** planned. **Depends on:** 01. **Owns:** D4, D6, D7, D21, D24, D25, D26, D32, L5, L8 (catalog test), L11.

## Contract

A helicopter flies anywhere with a single move order:
- it cruises at 20 m, rises to top + 10 m over anything taller, and never exceeds 40 m;
- it routes around any building more than 30 m tall;
- it ignores water and slope;
- it turns on the spot when hovering;
- overlapping helicopters drift apart.

## API seam

- New `sim::movement::air::step_aircraft`, dispatched before the vehicle branch. It never reaches `request_route`, `RoutePlanner` or `drive` (L5).
- New `sim::navigation::air::AirGrid`:
  - `build(known tall bodies, &AirRules)` and `route(from, to) -> Vec<V2>`, pure functions;
  - held per side in `SideGeometry`, built from that side's knowledge (D32);
  - rebuilt on obstacle revision.
- A world query for the tallest body top under a footprint, built on `Prop::top_z()`.
- New digested state: `Unit.air: Option<AirState { velocity, agl_target, aim_yaw }>`. The body yaw is kept separate from the travel direction.
- Fixtures:
  - `test_heli` in a new `fixtures/units/test/aircraft.json`: utility profile from D5, HMG on a turret;
  - new authored test map `fixtures/maps/air/`: a village, a forest and one `china-tower-20f` (61 m).
- `every_unit_type_sets_up_fires_each_mount_and_moves` (`crates/sim/tests/catalog.rs`) learns about altitude and passes for `test_heli`.

## What you can run or see

`cargo test -p sim --test air` (flight cases), and `--test navigation`.

## Verification

Tests:
- `cruises_at_20m`
- `pops_over_a_roof_and_comes_back_down`
- `rises_2m_over_forest_canopy` (D24)
- `routes_around_a_roof_over_30m`
- `never_exceeds_the_ceiling`
- `ignores_water_and_slope`
- `turns_on_the_spot_when_hovering`
- `overlapping_helicopters_part_softly`
- `air_grid_uses_side_knowledge`

Bound grid search cost with a test that counts expanded cells, not with wall time.

## Delegated to the implementer

Air grid cell size and search algorithm; the climb lookahead; the separation-push constant; the turn and climb rates on `test_heli`. There must be no altitude teleport.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the user wants helicopters to fly between towers in streets, D4's obstacle cut changes.
