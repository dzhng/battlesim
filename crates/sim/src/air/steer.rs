use crate::math::{v2, wrap_angle, V2};

use super::fixed_wing::{roll_rad_s2, FixedWingLimits, FixedWingState, Steer};

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

    /// Each edge as seen from `p`: the room to it, and its outward normal.
    pub(crate) fn edges(self, p: V2) -> [(f64, V2); 4] {
        [
            (p.x - self.min.x, v2(-1.0, 0.0)),
            (self.max.x - p.x, v2(1.0, 0.0)),
            (p.y - self.min.y, v2(0.0, -1.0)),
            (self.max.y - p.y, v2(0.0, 1.0)),
        ]
    }
}

/// What a fixed-wing pilot is trying to do this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Intent {
    Orbit {
        centre: V2,
    },
    Line {
        through: V2,
        heading: f64,
        agl: f64,
    },
    /// Chase `point`, moving at `vel`, aimed `lag_s` behind it, flown at
    /// `agl` metres above ground.
    Pursue {
        point: V2,
        vel: V2,
        lag_s: f64,
        agl: f64,
    },
    Exit {
        to: V2,
    },
}

/// The turn, as a share of its full rate, that brings the nose onto
/// `wanted`: as fast as it can still roll out of before the nose gets there,
/// this tick's step included, so it rolls out onto the heading rather than
/// through it.
fn turn_toward(current: f64, wanted: f64, limits: &FixedWingLimits, dt_s: f64) -> f64 {
    let error = wrap_angle(wanted - current);
    let roll = roll_rad_s2(limits);
    let step = roll * dt_s;
    let rate = (step * step + 2.0 * roll * error.abs()).sqrt() - step;
    (rate.copysign(error) / limits.turn_rad_s.max(f64::MIN_POSITIVE)).clamp(-1.0, 1.0)
}

/// How much later than an instant full-rate turn a jet's turn to the side
/// `sign` (+1 left, -1 right) carries it round, in seconds of flight, while
/// it rolls there from the rate it turns at now: none if it already turns
/// that way at full rate, half its roll-in from wings level.
pub(crate) fn roll_lag_s(state: &FixedWingState, limits: &FixedWingLimits, sign: f64) -> f64 {
    let full = limits.turn_rad_s.max(f64::MIN_POSITIVE);
    let short = (full - state.turn_rate_rad_s * sign).max(0.0);
    short * short / (2.0 * roll_rad_s2(limits) * full)
}

fn orbit_turn(state: &FixedWingState, limits: &FixedWingLimits, centre: V2) -> f64 {
    let radius = turn_radius(state, limits);
    let offset = state.pos.xy() - centre;
    let distance = offset.length();
    // It circles the way it is already going round the centre, which no
    // correction on the way in reverses; heading straight at it or away, the
    // way it was turning.
    let round = offset.x * libm::sin(state.heading) - offset.y * libm::cos(state.heading);
    let sign = if round.abs() > distance * 0.05 {
        round.signum()
    } else if state.turn_sign < 0 {
        -1.0
    } else {
        1.0
    };
    // At the centre, any way out is straight out: the one it heads.
    let radial = if distance > 1e-6 {
        libm::atan2(offset.y, offset.x)
    } else {
        state.heading
    };
    // The way round the circle, bent toward it by how far off it the jet
    // is: straight at the centre from two radii out, straight out of it
    // from the middle. It holds the full rate inside and on the circle,
    // none from two radii out, and rolls out its error over a roll-in's
    // swing, so it flies straight at a far goal, rolls onto the circle and
    // stays there, where the full rate gives the authored radius exactly
    // (speed / turn rate).
    let off = ((distance - radius) / radius).clamp(-1.0, 1.0);
    let wanted = radial + sign * core::f64::consts::FRAC_PI_2 * (1.0 + off);
    let swing = (limits.turn_rad_s * limits.roll_in_s).max(f64::MIN_POSITIVE);
    sign * (1.0 - off.max(0.0)) + wrap_angle(wanted - state.heading) / swing
}

fn turn_radius(state: &FixedWingState, limits: &FixedWingLimits) -> f64 {
    (state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE)).max(1e-6)
}

/// The way into the map from `point`, away from each edge within `reach`
/// of it, weighted by how far within; `None` when it lies clear of them all.
pub fn inward_from(point: V2, reach: f64, bounds: &Bounds) -> Option<V2> {
    let inward = bounds
        .edges(point)
        .into_iter()
        .fold(v2(0.0, 0.0), |sum, (room, outward)| {
            sum - outward * (reach - room).max(0.0)
        });
    (inward.dot(inward) > 0.0).then_some(inward)
}

/// How much nearer an edge a full-rate turn away from it still carries a
/// jet closing on it at `closing` (the cosine between its heading and the
/// edge's outward normal), turning on a circle of `radius`.
fn turn_away_reach(radius: f64, closing: f64) -> f64 {
    let closing = closing.max(0.0);
    radius * (1.0 - (1.0 - closing * closing).max(0.0).sqrt())
}

/// Whether a jet `room` metres from an edge, closing on it at `closing`
/// (negative while it draws away), is at its point of no return: one more
/// `step` (a tick's flight and the lag while it rolls into the turn) would
/// leave a full-rate turn away unable to keep it `margin` inside. A jet
/// already inside the margin is there too unless it is leaving faster than
/// a step's worth.
fn past_no_return(room: f64, closing: f64, radius: f64, step: f64, margin: f64) -> bool {
    room - margin <= turn_away_reach(radius, closing) + step * closing
}

/// The edge guard (D36), at a jet's point of no return on any edge: the
/// full-rate turn away from the edges it must turn away from now. `None`
/// while it can still fly as it means to, however near an edge. On a
/// straight edge this keeps it `edge_margin_m` inside; in a corner, turning
/// away from one edge carries it toward the other, into the margin but
/// inside the square.
fn edge_guard(
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    dt_s: f64,
) -> Option<f64> {
    let radius = turn_radius(state, limits);
    let forward = v2(libm::cos(state.heading), libm::sin(state.heading));
    let pressing = bounds
        .edges(state.pos.xy())
        .into_iter()
        .filter(|&(room, outward)| {
            // The way that turns it away from this edge; straight at it,
            // the way its roll takes longest.
            let cross = forward.x * outward.y - forward.y * outward.x;
            let away = if cross != 0.0 {
                -cross.signum()
            } else {
                -state.turn_rate_rad_s.signum()
            };
            let lag = roll_lag_s(state, limits, away);
            // Turning toward the edge, the nose swings on toward it until
            // its roll passes wings level.
            let toward = (-state.turn_rate_rad_s * away).max(0.0);
            let swung = state.heading - away * toward * toward / (2.0 * roll_rad_s2(limits));
            let closing = v2(libm::cos(swung), libm::sin(swung))
                .dot(outward)
                .max(forward.dot(outward));
            past_no_return(
                room,
                closing,
                radius,
                state.speed_mps * (dt_s + lag),
                limits.edge_margin_m,
            )
        })
        .fold(v2(0.0, 0.0), |sum, (_, outward)| sum + outward);
    if pressing.dot(pressing) == 0.0 {
        return None;
    }
    // The way that takes the nose off the edges it presses; straight at
    // them, toward the middle.
    let side = forward.x * pressing.y - forward.y * pressing.x;
    Some(if side != 0.0 {
        -side.signum()
    } else {
        let middle = (bounds.min + bounds.max) * 0.5 - state.pos.xy();
        turn_toward(state.heading, libm::atan2(middle.y, middle.x), limits, dt_s)
    })
}

/// Whether the edge guard takes the jet this tick (D36).
pub fn edge_guarded(
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    dt_s: f64,
) -> bool {
    edge_guard(state, limits, bounds, dt_s).is_some()
}

/// How far short of `aim` the edge guard would take the jet flying
/// straight at it along `toward` (a unit vector); 0 if it would reach the
/// aim first.
pub fn guard_short_of(
    aim: V2,
    toward: V2,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    dt_s: f64,
) -> f64 {
    let radius = turn_radius(state, limits);
    // Flying the run, it is wings level when the guard takes it.
    let level = FixedWingState {
        turn_rate_rad_s: 0.0,
        ..*state
    };
    let step = state.speed_mps * (dt_s + roll_lag_s(&level, limits, 1.0));
    // Short of the aim by `s`, an edge the line closes on at `closing` is
    // `room + s * closing` away: the guard takes the jet where that first
    // meets its point of no return.
    bounds
        .edges(aim)
        .into_iter()
        .filter(|&(_, outward)| toward.dot(outward) > 0.0)
        .map(|(room, outward)| {
            let closing = toward.dot(outward);
            (limits.edge_margin_m + turn_away_reach(radius, closing) + step * closing - room)
                / closing
        })
        .fold(0.0, f64::max)
}

fn intent_turn(
    intent: &Intent,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    dt_s: f64,
) -> (f64, f64) {
    match *intent {
        Intent::Orbit { centre } => (orbit_turn(state, limits, centre), limits.cruise_agl_m),
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
        Intent::Pursue {
            point,
            vel,
            lag_s,
            agl,
        } => {
            let aim = point - vel * lag_s.max(0.0);
            let direction = aim - state.pos.xy();
            let wanted = if direction.length() > 1e-9 {
                libm::atan2(direction.y, direction.x)
            } else {
                state.heading
            };
            (turn_toward(state.heading, wanted, limits, dt_s), agl)
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

/// Convert an intent into the bounded turn command consumed by [`super::fixed_wing::fly`].
pub fn steer(
    intent: &Intent,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    dt_s: f64,
) -> Steer {
    let (mut turn, agl_m) = intent_turn(intent, state, limits, dt_s);
    if !matches!(intent, Intent::Exit { .. }) {
        if let Some(away) = edge_guard(state, limits, bounds, dt_s) {
            turn = away;
        }
    }
    Steer {
        turn: turn.clamp(-1.0, 1.0),
        agl_m,
        burner: false,
    }
}
