use crate::math::{v2, v3, wrap_angle, V2, V3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JetState {
    pub pos: V3,
    pub heading: f64,
    pub speed_mps: f64,
    pub burner: f64,
    pub turn_sign: i8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JetLimits {
    pub cruise_mps: f64,
    pub burner_mult: f64,
    pub turn_rad_s: f64,
    pub climb_mps: f64,
    pub ramp_s: f64,
    pub floor_agl_m: f64,
    pub edge_margin_m: f64,
    pub cruise_agl_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steer {
    pub turn: f64,
    pub agl_m: f64,
    pub burner: bool,
}

/// Advance a jet by `dt` seconds over the supplied ground surface.
pub fn fly(
    mut state: JetState,
    steer: Steer,
    limits: &JetLimits,
    ground: &impl Fn(V2) -> f64,
    dt: f64,
) -> JetState {
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

    let turn = steer.turn.clamp(-1.0, 1.0) * limits.turn_rad_s * dt;
    if turn != 0.0 {
        state.turn_sign = if turn.is_sign_negative() { -1 } else { 1 };
    }
    state.heading = wrap_angle(state.heading + turn);
    let direction = v2(libm::cos(state.heading), libm::sin(state.heading));
    let next_xy = state.pos.xy() + direction * (state.speed_mps * dt);
    let ground_z = ground(next_xy);
    let target_z = ground_z + steer.agl_m.max(limits.floor_agl_m);
    let dz = (target_z - state.pos.z).clamp(-limits.climb_mps * dt, limits.climb_mps * dt);
    state.pos = next_xy.with_z((state.pos.z + dz).max(ground_z + limits.floor_agl_m));
    state
}

/// The velocity along the jet's nose. Height changes are applied by `fly`.
pub fn velocity(state: &JetState) -> V3 {
    v3(
        libm::cos(state.heading) * state.speed_mps,
        libm::sin(state.heading) * state.speed_mps,
        0.0,
    )
}
