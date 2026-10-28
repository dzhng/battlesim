# 03 Sight bands and the radar signature

**Unlocks:** every unit authors its own sight per altitude band, and the radar signature is a flag. This deletes the high-air fallback table, the radar sight override and their TypeScript copy. No battle changes.

## Seam

- `Sensors.low_air_m: f64` and `Sensors.high_air_m: f64` are required; `0` means blind.
- One owner: `impl Sensors { pub fn reach_m(&self, layer: AltitudeLayer) -> f64 }`, which `sight::of_layer` (`crates/sim/src/sight.rs:78`) calls.
- `Capabilities.radar_emitter: bool`, default false, left out when false. `knowledge.rs:311-315` and `weapons.rs:553-557` read it directly.
- **Delete:**
  - `RadarCapability`
  - `UnaidedHighAirSight` and `SensorRules.unaided_high_air_m`
  - `UnitType::high_air_sight_m` (`catalog.rs:745`)
  - the emitter validation (`catalog.rs:1625-1637`)
  - the `ground_m` fallback in `sight.rs:82`
- **TypeScript copies are deleted, not ported:**
  - delete `highAirSight` and `UnaidedHighAirSight` (`packages/scene-assets/src/units.ts:75-98`);
  - make the unit type's bands required and add `radar_emitter?: boolean`;
  - make `apps/battle-lab/src/routes/airWar.tsx:371,376` and `web/scenes/air-roster.mjs:160-165` print the fields directly;
  - fix the stale reference in `web/scenes/air-audio.mjs:10`.

## Data

A one-off script in `throwaway/` writes each type's former effective value on the entry where it resolves:
- `low_air_m`: the resolved `ground_m` wherever it was absent;
- `high_air_m`: the old `detection_m` for the 9 emitters, otherwise 600 on foot, 600 for aircraft and 300 for ground vehicles.

Beware inheritance: a child that changes only `ground_m` under a parent that now authors `low_air_m` would inherit the parent's value. Write the value per resolved type wherever it differs. Edit `fixtures/parity/map-layout/physical-rules.json` by hand (all six frozen units) and never regenerate it (`fixtures/README.md`).

## Delegated to you

The scratch script, and the placement level for each value.

## Verification

- **Before/after:** dump `reach_m` for every band and every resolved type in the game, test and capture sets. The dumps must be identical.
- **Tests:**
  - **Delete,** because they test the removed fallback: the `None` arm of `crates/sim/tests/air.rs` `jeep_sees_the_helicopter`, `radar.rs:614`, and `contract/tests/catalog.rs:994,1027`.
  - **Merge** `radar.rs:544` and `air.rs` `low_air_sight_is_the_units_own…` into one test: each band's sight is the unit's own, and 0 is blind.
  - **Let the failing tests name the patches to fix.** Of the 31 test patches that set only `ground_m`, fix only those that break. Don't edit all 31.
- Every battle digest is identical; re-bless map layout per the [proof](../README.md#proof-that-nothing-changed).

## Must stay green

Air sight, radar warning receivers, anti-radiation targeting, the battle lab, and every digest. Delete enough to net-shrink `catalog.rs` and `air.rs`, which are over the size budget.
