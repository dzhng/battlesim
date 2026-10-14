# 01 — Air rules split, one height owner

**Delivers:** D1, D2, D19.

**Unlocks:** separate helicopter and jet rules, with one function answering "how high".

## Seam
`AirRules` (`crates/contract/src/scenario.rs`) becomes:
```rust
AirRules {
    rotor: RotorAir { cruise_agl_m, clearance_m, ceiling_agl_m, obstacle_m, acceleration_s, settle_share, settle_max_m },
    jet: JetAir { cruise_agl_m /*250*/, floor_agl_m /*100*/, strafe_ceiling_m /*130*/, bomb_ceiling_m /*200*/,
                  dogfight_band_agl_m: [f64; 2] /*150,350*/, dogfight_break_m /*1200*/, afterburner_ramp_s /*2*/, edge_margin_m },
    separation_m,
}
AirRules::cruise_agl_m(layer) -> f64
```
- **Validation:** `jet.floor_agl_m` ≤ every jet ceiling ≤ `jet.cruise_agl_m`, and the rotor's existing checks still apply.
- **The function's readers:**
  - spawn (`battle.rs` ~512 and ~652);
  - the order ghost (`apps/battle-lab/src/unitGhosts.ts`): delete the literal 1000 and read the lookup through a TS helper beside `highAir` in `packages/scene-assets/src/units.ts`;
  - native capture (`native/client/src/capture.rs`);
  - presentation input (`crates/presentation/src/world.rs` `cruise_agl_m`), which becomes per layer.
- **Shadow:** it falls along the sun onto the ground, thrown from the aircraft's actual height above that ground (`presentation.rs` ~1536), not from a constant cruise height.
- **Routing margin:** the helicopter margin (`navigation/air.rs`) comes from the data value `hull_limits.low_air.half_length_m`; each aircraft is admitted against its own layer's hull limit.
- **Re-recorded:** the three parity fixtures that embed `air`, and the helicopter beats.
- **Jets still use the old mover in this slice.** The old mover reads `cruise_agl_m(layer)`, so jets already fly at 250 m on the old law.

## You see
On `/labs/air-high`, the jet, its order ghost and its card sit at 250 m, while the helicopter sits at 30 m.

## Tests (red first)
- `air_rules_refuse_a_jet_floor_above_its_ceilings`
- `a_helicopters_routing_margin_is_its_largest_low_air_hull`
- `a_jet_spawns_at_the_jet_cruise_height`
- Presentation tests:
  - `a_shadow_falls_along_the_sun_from_the_aircrafts_actual_height`
  - `the_ghost_reads_the_layer_height` (TS)

## Visual gate
- **Variable:** ghost and shadow height. **Crop:** a jet and its ghost against the ground.
- Compare against slice 00's before shot with `compare-screenshots`, run `screenshot-critique` last, then a non-blocking `preview-shots`.

## Firewall, delegation and what stays green
- **Firewall:** no motion-law change.
- **Delegated:** the field names inside the blocks.
- **Stays green:** `air_flight.rs`, `air_supply.rs`, `air_closing.rs` (re-recorded beats) and every helicopter scene.
