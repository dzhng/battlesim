# 02 — Jet flight law

**Delivers:** D3′, D9, D20, D34, plus D19's "jets skip the air grid".

**Unlocks:** a jet never stops. It flies where its nose points, at cruise or afterburner speed only, and holds its height above the ground.

## Seam
A new module `crates/sim/src/air/jet.rs`, a pure law:
```rust
JetState { pos: V3, heading: f64, speed_mps: f64, burner: f64 /*0..1 ramp*/, turn_sign: i8 }
JetLimits { cruise_mps, burner_mult, turn_rad_s, climb_mps, ramp_s, floor_agl_m }
Steer { turn: f64 /*-1..1 of max rate*/, agl_m: f64, burner: bool }
fly(JetState, Steer, &JetLimits, ground: &impl Fn(V2) -> f64, dt) -> JetState
velocity(&JetState) -> V3
```
- **Dispatch:** `movement/mod.rs` sends `HighAir` units to `air::step_jet`. For now `step_jet` steers straight at the movement goal; slice 03 replaces that with real steering.
- `AirState.velocity` becomes `V3`. Update publication, digest and flares to match.
- **Jets never call `plan()` on the air grid.**
- **Delete from `movement/air.rs`:** the jet use of settle and hover, the `reach.face` turn, and the dogfight weave and chase. Slice 10 rebuilds dogfights; until then a linked jet just flies its goal.
- **Fuel burns** at `dt × fuel_multiplier` inside `step_jet`.
- **Data:** `roster_profile_fixed_wing` goes to 650 km/h and 72°/s.
- **Height:** smoothed above the ground beneath the jet, with no prop probes, and **hard-clamped at or above `floor_agl_m`** while the jet is alive.

## You see
`/labs/air-high`: the jet flies straight over the highrise at 250 m and never stops. Record the GIF and trace.

## Tests (red first; `crates/sim/tests/jet_flight.rs`)
- `a_jet_never_flies_slower_than_cruise` (red: it hovers today)
- `afterburner_ramps_to_speed_over_its_ramp_and_back`
- `afterburner_burns_fuel_at_its_multiplier`
- `a_jet_flies_where_its_nose_points_and_turns_no_faster_than_its_rate`
- `a_jet_never_drops_below_the_floor_over_relief`
- `a_jet_holds_cruise_over_rolling_ground_without_steps`
- `a_jet_flies_straight_over_a_61m_tower`

## Visual gate
- **Variable:** a straight path that never stops. **Crop:** the trace sheet.
- Compare against slice 00's before trace, run `screenshot-critique` last, then `preview-shots`.

## Firewall, delegation and what stays green
- **Firewall:** no circling, weapons or orders.
- **Delegated:** the smoothing constant, and the internal shape of `Steer`.
- **Stays green:** every helicopter test.
