# 07 — Bomb release and new rows

**Delivers:** D6′, D21b.

**Unlocks:** unpowered bombs dropped from a solved release point, and the ground-missile and iron-bomb rows.

## Seam
- **Solver:** a pure `crates/sim/src/flight/release.rs`, `release(carrier_pos: V3, carrier_vel: V3, gravity_scale, aim: V3) -> Release { fall_s, ahead_m, at: V2 }`. It is 2D ahead and analytic in the fall.
- **Bomb rows:** `WeaponBallistics` gains an unpowered delivery. Unpowered rows drop `range_m` and `speed_mps` and author `gravity_scale` (admission in `contract/src/weapons.rs` ~360).
  - `gravity_scale` is set so the fall from 200 m takes about 3 s.
  - Cluster scatter centres on the aim point.
- **Rename `air_bomb_guided` to the unguided iron bomb `air_bomb`,** and repoint every reference to it.
- **New row `air_ground_missile`:** extends `heli_atgm`; ground, anti-armour, guided while moving, about 2000 m range, motor about +200 m/s over the jet, 4 rounds.

## Tests (red first)
- `a_bomb_released_at_the_solved_point_lands_on_its_aim`
- `a_bomb_falls_ahead_carrying_the_jets_velocity`
- `a_bomb_falls_about_three_seconds_from_200m`
- `cluster_bombs_scatter_around_the_aim_point`
- `an_unpowered_row_with_range_is_refused`
- `the_ground_missile_pulls_ahead_of_its_jet`

## Firewall and delegation
- **Firewall:** no passes; jets don't fly runs yet. The solver is used by slice 08.
- **Delegated:** the exact gravity scale and the scatter shape.
