# 14 — Jet attitude

**Unlocks:** jets pitch along their flight path in dives and climbs, and bank into turns. Today `aircraft_attitude` (`crates/presentation/src/units/driver.rs` ~1197–1230) leans the model from acceleration and reads only xy, so a diving jet is drawn level.

## Seam
- `aircraft_attitude`: for `HighAir`, pitch comes from the angle of the velocity's path, and roll from the turn rate.
- Helicopters are unchanged.

## Tests
- `a_diving_jet_pitches_nose_down`
- `a_turning_jet_banks_into_the_turn`

## Visual gate
- **Variable:** pitch and roll. **Crop:** one jet, close, during a pass.
- Run `screenshot-critique`, then `preview-shots`.
