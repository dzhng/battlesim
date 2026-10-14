use crate::math::{wrap_angle, V2, V3};

use super::jet::{JetLimits, JetState, Steer};

/// The rectangle in which a fighting jet must remain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: V2,
    pub max: V2,
}

impl Bounds {
    pub const fn new(min: V2, max: V2) -> Self {
        Self { min, max }
    }

    fn centre(self) -> V2 {
        (self.min + self.max) * 0.5
    }
}

/// What a fixed-wing pilot is trying to do this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Intent {
    Orbit { centre: V2 },
    Line { through: V2, heading: f64, agl: f64 },
    Pursue { point: V3, vel: V3, lag_s: f64 },
    Exit { to: V2 },
}

fn turn_toward(current: f64, wanted: f64, limits: &JetLimits, dt_s: f64) -> f64 {
    let error = wrap_angle(wanted - current);
    (error / (limits.turn_rad_s * dt_s).max(f64::MIN_POSITIVE)).clamp(-1.0, 1.0)
}

fn orbit_turn(state: &JetState, limits: &JetLimits, centre: V2, dt_s: f64) -> f64 {
    let sign = if state.turn_sign < 0 { -1.0 } else { 1.0 };
    let radius = (state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE)).max(1e-6);
    let offset = state.pos.xy() - centre;
    let distance = offset.length();
    let radial = if distance > 1e-6 {
        libm::atan2(offset.y, offset.x)
    } else {
        state.heading - sign * core::f64::consts::FRAC_PI_2
    };
    // A tangent entry lets a jet outside the orbit close on it without flying
    // through the centre. Once on the circle, the full rate gives the authored
    // radius exactly (speed / turn rate).
    let tolerance = radius * 0.04 + 1.0;
    if (distance - radius).abs() <= radius * 0.15 + tolerance {
        let tangent = radial + sign * core::f64::consts::FRAC_PI_2;
        let error = wrap_angle(tangent - state.heading);
        if error.abs() > 0.12 {
            turn_toward(state.heading, tangent, limits, dt_s)
        } else {
            sign
        }
    } else if distance < radius {
        sign
    } else {
        turn_toward(state.heading, radial + core::f64::consts::PI, limits, dt_s)
    }
}

fn edge_guard(
    state: &JetState,
    limits: &JetLimits,
    bounds: Bounds,
    margin: f64,
    dt_s: f64,
) -> Option<f64> {
    let radius = (state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE)).max(1e-6);
    let p = state.pos.xy();
    let look = radius + margin.max(0.0);
    let near = p.x < bounds.min.x + look
        || p.x > bounds.max.x - look
        || p.y < bounds.min.y + look
        || p.y > bounds.max.y - look;
    near.then(|| {
        turn_toward(
            state.heading,
            libm::atan2(bounds.centre().y - p.y, bounds.centre().x - p.x),
            limits,
            dt_s,
        )
    })
}

fn intent_turn(intent: &Intent, state: &JetState, limits: &JetLimits, dt_s: f64) -> (f64, f64) {
    match *intent {
        Intent::Orbit { centre } => (orbit_turn(state, limits, centre, dt_s), limits.cruise_agl_m),
        Intent::Line {
            through,
            heading,
            agl,
        } => {
            let forward = crate::math::v2(libm::cos(heading), libm::sin(heading));
            let left = crate::math::v2(-forward.y, forward.x);
            let cross_track = (state.pos.xy() - through).dot(left);
            let lookahead = (state.speed_mps * 1.0).max(1.0);
            let wanted = heading - libm::atan2(cross_track, lookahead);
            (turn_toward(state.heading, wanted, limits, dt_s), agl)
        }
        Intent::Pursue { point, vel, lag_s } => {
            let aim = point - vel * lag_s.max(0.0);
            let direction = aim.xy() - state.pos.xy();
            let wanted = if direction.length() > 1e-9 {
                libm::atan2(direction.y, direction.x)
            } else {
                state.heading
            };
            (turn_toward(state.heading, wanted, limits, dt_s), point.z)
        }
        Intent::Exit { to } => {
            let direction = to - state.pos.xy();
            let wanted = if direction.length() > 1e-9 {
                libm::atan2(direction.y, direction.x)
            } else {
                state.heading
            };
            (
                turn_toward(state.heading, wanted, limits, dt_s),
                limits.cruise_agl_m,
            )
        }
    }
}

/// Convert an intent into the bounded turn command consumed by [`super::jet::fly`].
pub fn steer(
    intent: &Intent,
    state: &JetState,
    limits: &JetLimits,
    bounds: &Bounds,
    dt_s: f64,
) -> Steer {
    let (mut turn, agl_m) = intent_turn(intent, state, limits, dt_s);
    if !matches!(intent, Intent::Exit { .. }) {
        if let Some(edge) = edge_guard(state, limits, *bounds, limits.edge_margin_m, dt_s) {
            turn = edge;
        }
    }
    Steer {
        turn: turn.clamp(-1.0, 1.0),
        agl_m,
        burner: false,
    }
}
