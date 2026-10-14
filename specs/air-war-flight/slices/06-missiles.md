# 06 — Missiles: speed, lifetime, fuse, aim

**Delivers:** D12/D17′, D29, D33.

**Unlocks:** fast-looking, fused air missiles whose reach matches their range.

## Seam
- **Rows** (`fixtures/game.json`): `air_missile`, `aa_medium`, `aa_long`, `stinger` and the family rows that extend them get `top_speed_mps` 290, with lifetimes matched to range so each missile reaches its authored range.
  - Anti-tank missiles are unchanged.
  - `air_missile` gains a `low_air` target row.
  - The SEAD missile keeps its own speed, and its lifetime still covers its range.
- **Proximity fuse:** a hook `ImpactResolver::aircraft_fuse(projectile) -> Option<(UnitId, f64)>`, built next to the existing `projectile_interceptor_reach` (`flight/mod.rs` ~626, `damage.rs` ~285). A missile aimed at an aircraft detonates within its `blast_radius_m` of that aircraft. There is no fuse against ground.
- **Seeker:** it steers at the centre of the target's box (`battle.rs` ~3146), and the target velocity keeps its z (`weapons.rs` ~491).
- **Re-run weapon admission.**

## You see
- `/labs/air-survivability`: a SAM chasing a jet, with the afterburner escape tuned by eye.
- `/labs/air-defense`: bursts beside turning jets.

## Tests (red first)
- `a_missile_bursts_within_its_blast_radius_of_a_passing_jet`
- `no_proximity_fuse_against_ground`
- `a_seeker_steers_at_the_box_centre`
- `a_climbing_targets_vertical_velocity_is_led`
- `an_air_missile_ends_at_its_lifetime`
- `an_air_missile_engages_a_helicopter`
- Contract test: `every_air_target_row_reaches_its_range_within_its_lifetime`

**No test about which side escapes or wins.**

## Visual gate
- **Variable:** the burst position against the jet. **Crop:** the jet and the missile end.
- Run `screenshot-critique`, then `preview-shots`.

## Delegated
The per-row lifetimes and acceleration.
