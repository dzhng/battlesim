//! The fixed-wing flight law, the only writer of a winged aircraft's position,
//! heading and speed: it never stops, rolls into and out of its turns,
//! ramps its afterburner, and holds its layer's height, never below the
//! layer's floor.

use crate::math::{v2, v3, wrap_angle, V2, V3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedWingState {
    pub pos: V3,
    pub heading: f64,
    pub speed_mps: f64,
    pub burner: f64,
    /// How fast it is turning now, radians a second (positive to the
    /// left): it rolls toward the rate it is steered to, never steps.
    pub turn_rate_rad_s: f64,
    pub turn_sign: i8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedWingLimits {
    pub cruise_mps: f64,
    pub burner_mult: f64,
    pub turn_rad_s: f64,
    /// Seconds it takes to roll from wings level to its full turn rate.
    pub roll_in_s: f64,
    pub climb_mps: f64,
    pub ramp_s: f64,
    pub floor_agl_m: f64,
    pub edge_margin_m: f64,
    pub cruise_agl_m: f64,
    /// The nose-on cone either side of its heading (`nose_on_deg`).
    pub nose_on_rad: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steer {
    /// The turn rate it is steered to, as a share of its full rate.
    pub turn: f64,
    pub agl_m: f64,
    pub burner: bool,
}

/// Advance a fixed-wing aircraft by `dt` seconds over the supplied ground surface.
pub fn fly(
    mut state: FixedWingState,
    steer: Steer,
    limits: &FixedWingLimits,
    ground: &impl Fn(V2) -> f64,
    dt: f64,
) -> FixedWingState {
    if dt <= 0.0 {
        return state;
    }
    let ramp = (dt / limits.ramp_s.max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
    let wanted_burner = if steer.burner { 1.0 } else { 0.0 };
    state.burner += if wanted_burner > state.burner {
        ramp
    } else {
        -ramp
    };
    state.burner = state.burner.clamp(0.0, 1.0);
    state.speed_mps = (limits.cruise_mps * (1.0 + state.burner * (limits.burner_mult - 1.0)))
        .max(limits.cruise_mps);

    let wanted = steer.turn.clamp(-1.0, 1.0) * limits.turn_rad_s;
    let roll = roll_rad_s2(limits) * dt;
    state.turn_rate_rad_s += (wanted - state.turn_rate_rad_s).clamp(-roll, roll);
    if state.turn_rate_rad_s != 0.0 {
        state.turn_sign = if state.turn_rate_rad_s < 0.0 { -1 } else { 1 };
    }
    state.heading = wrap_angle(state.heading + state.turn_rate_rad_s * dt);
    let direction = v2(libm::cos(state.heading), libm::sin(state.heading));
    let next_xy = state.pos.xy() + direction * (state.speed_mps * dt);
    let ground_z = ground(next_xy);
    let target_z = ground_z + steer.agl_m.max(limits.floor_agl_m);
    let dz = (target_z - state.pos.z).clamp(-limits.climb_mps * dt, limits.climb_mps * dt);
    state.pos = next_xy.with_z((state.pos.z + dz).max(ground_z + limits.floor_agl_m));
    state
}

/// How fast its turn rate can change, radians a second squared: from wings
/// level to its full rate over its roll-in.
pub fn roll_rad_s2(limits: &FixedWingLimits) -> f64 {
    limits.turn_rad_s / limits.roll_in_s.max(f64::MIN_POSITIVE)
}

/// The velocity along the aircraft's nose. Height changes are applied by `fly`.
pub fn velocity(state: &FixedWingState) -> V3 {
    v3(
        libm::cos(state.heading) * state.speed_mps,
        libm::sin(state.heading) * state.speed_mps,
        0.0,
    )
}
