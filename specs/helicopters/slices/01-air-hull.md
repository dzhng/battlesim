# 01 — An airborne hull

**Status:** planned. **Depends on:** nothing. **Owns:** L1, L6, L7, D17 (concealment), D20, D37.

## Contract

Fly a hull at 20 m and nothing on the ground treats it as ground:
- tanks drive under it;
- soldiers are neither shoved by it nor take cover behind it;
- it fells no forest and leaves no treads;
- it gets no forest concealment;
- it neither captures nor contests objectives.

It is admitted as a unit type. A change of altitude alone isn't movement. The plan dies here if an airborne hull can't be separated from the ground rules cheaply.

## API seam

**Contract:**
- `Mobility::Air { cruise_kmh, turn_deg_s, climb_mps }` in `contract::catalog`, with its own speed cap and its own `HullLimits.air` row.
- Make `HullLimits::check` exhaustive: replace its `_ => continue` (`scenario.rs`) with an explicit skip for Foot only.
- Add an `Air` arm to `admit_skirmish`; air needs no route evidence.
- Add the `air` block in `fixtures/game.json` (D25). Its numbers are read from slice 03 on.

**Sim (one owner each):**
- `units::Motion { Ground(GroundMobility), Air(AirMobility) }` replaces the flat `units::Mobility`. Ground navigation and drive functions take `&GroundMobility`, so the compiler keeps a flier away from them (D21).
- `Unit::airborne()`.
- `hull_box()` is renamed `ground_footprint() -> Option<Obb2>` and returns `None` when airborne. Every ground-only call site then has to handle it:
  - traffic: `vehicle_conflict` and the `is_vehicle()` filters in `movement/mod.rs`;
  - `soldier::shove` and `Threat::of`;
  - `lean::hulls`, which feeds cover;
  - `push`;
  - `clear_lanes` and `treads()`;
  - the entry `apart` check.
- `sensing`: an airborne target gets no forest concealment.
- `moved` compares XY for every unit (D37).
- Objectives already exclude `Hel` (`objectives.rs:61`). Pin that with a test.

## What you can run or see

`cargo test -p sim --test air` (new file). It uses a test-local `test_heli` row that follows the `with_m1_family()` fixture pattern, placed at ground + 20 m.

## Verification

Tests:
- `a_tank_drives_under_a_hovering_helicopter`
- `soldiers_are_not_shoved_or_covered_by_it`
- `it_fells_no_forest_and_wears_no_tracks`
- `it_is_not_forest_concealed`
- `it_cannot_contest_an_objective`
- `an_air_hull_past_its_limits_is_refused`
- `altitude_change_alone_is_not_moving`
- `a_ground_battle_digest_is_unchanged` (D37)
- a twice-run digest match for a battle with a helicopter

## Delegated to the implementer

Internal names of the `Motion` arms and of the test helpers; how the `ground_footprint` call sites are reshaped.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the user wants helicopters to block or crush something on the ground, the airborne-is-not-ground rule needs an exception here.
