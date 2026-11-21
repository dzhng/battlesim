//! The fixed-wing flight law, the only writer of a winged aircraft's position,
//! heading, pitch and speed: it never stops, rolls into and out of its turns,
//! flies where its nose points, pitches to hold its height or onto a point it
//! is told to aim at, ramps its afterburner, and never goes below the layer's
//! floor.

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
    /// The nose's angle above the horizon, radians (negative diving). It
    /// flies along its nose: its climb is its speed times the pitch's
    /// tangent.
    pub pitch: f64,
    /// How fast its pitch is changing, radians a second (positive nose up):
    /// it eases toward the rate it is steered to, never steps.
    pub pitch_rate_rad_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedWingLimits {
    pub cruise_mps: f64,
    pub burner_mult: f64,
    /// The rate it turns at by choice.
    pub turn_rad_s: f64,
    /// The hardest it turns, only to keep off the map's edge.
    pub max_turn_rad_s: f64,
    /// Seconds it takes to roll from wings level to the rate it is steered
    /// to (`roll_rad_s2`).
    pub roll_in_s: f64,
    /// Seconds it takes to build its pitch rate from none to the rate it
    /// turns at (`pitch_in_s`); it pitches at the rates it turns at.
    pub pitch_in_s: f64,
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
    /// The turn rate it is steered to, as a share of its `turn_rad_s`:
    /// beyond one only up to its max.
    pub turn: f64,
    pub agl_m: f64,
    pub burner: bool,
    /// A point to hold the nose on instead of holding `agl_m`.
    pub nose_at: Option<NoseAt>,
}

/// A point a jet points its nose at (an attack's aim, a dogfight's
/// opponent), so a gun fixed along the nose bears on it, never descending
/// where pulling out at its hardest would take it under `lowest_agl_m`
/// above the ground: there it pulls out and holds that height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoseAt {
    pub point: V3,
    pub lowest_agl_m: f64,
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

    let reserve = reserve(limits);
    let wanted = steer.turn.clamp(-reserve, reserve) * limits.turn_rad_s;
    let roll = roll_rad_s2(limits, wanted) * dt;
    state.turn_rate_rad_s += (wanted - state.turn_rate_rad_s).clamp(-roll, roll);
    if state.turn_rate_rad_s != 0.0 {
        state.turn_sign = if state.turn_rate_rad_s < 0.0 { -1 } else { 1 };
    }
    state.heading = wrap_angle(state.heading + state.turn_rate_rad_s * dt);
    let direction = v2(libm::cos(state.heading), libm::sin(state.heading));
    let next_xy = state.pos.xy() + direction * (state.speed_mps * dt);
    let ground_z = ground(next_xy);
    let hold = |agl: f64| {
        let target_z = ground_z + agl.max(limits.floor_agl_m);
        let climb = ((target_z - state.pos.z) / level_off_s(limits, state.speed_mps))
            .clamp(-limits.climb_mps, limits.climb_mps);
        libm::atan(climb / state.speed_mps.max(f64::MIN_POSITIVE))
    };
    // It never descends where pulling out at its hardest would only just
    // keep it above the height it holds or the lowest it may dive to: there
    // it levels its nose at that rate.
    let lowest_agl = steer.nose_at.map_or(steer.agl_m, |nose| nose.lowest_agl_m);
    let lowest_z = ground_z + lowest_agl.max(limits.floor_agl_m);
    let (wanted, cap) = if state.pos.z - lowest_z <= pull_out_m(&state, limits, dt) {
        (hold(lowest_agl).max(0.0), limits.max_turn_rad_s)
    } else if let Some(nose) = steer.nose_at {
        let to = nose.point - state.pos;
        (libm::atan2(to.z, to.xy().length()), limits.turn_rad_s)
    } else {
        (hold(steer.agl_m), limits.turn_rad_s)
    };
    pitch_toward(&mut state, limits, wanted, cap, dt);
    let climbed = state.pos.z + state.speed_mps * libm::tan(state.pitch) * dt;
    let floor_z = ground_z + limits.floor_agl_m;
    if climbed < floor_z {
        // Ground rising faster than it climbs lifts it onto the floor: its
        // nose follows the path it was lifted along.
        state.pitch = libm::atan((floor_z - state.pos.z) / (state.speed_mps * dt));
    }
    state.pos = next_xy.with_z(climbed.max(floor_z));
    state
}

/// Seconds over which it levels off onto a height: the time its pitch takes
/// to swing from level to its climb angle at the rate it turns at, after
/// its pitch rate builds.
fn level_off_s(limits: &FixedWingLimits, speed_mps: f64) -> f64 {
    let climb_angle = libm::atan(limits.climb_mps / speed_mps.max(f64::MIN_POSITIVE));
    limits.pitch_in_s + climb_angle / limits.turn_rad_s.max(f64::MIN_POSITIVE)
}

/// Ease the pitch toward `wanted` at no more than `cap` radians a second:
/// the pitch rate builds from none to `cap` over `pitch_in_s` and asks only
/// for a rate it can still ease out of before the nose reaches `wanted`, so
/// it settles onto it rather than through it.
fn pitch_toward(
    state: &mut FixedWingState,
    limits: &FixedWingLimits,
    wanted: f64,
    cap: f64,
    dt: f64,
) {
    let accel = cap / limits.pitch_in_s.max(f64::MIN_POSITIVE);
    let step = accel * dt;
    let error = wanted - state.pitch;
    let rate = ((step * step + 2.0 * accel * error.abs()).sqrt() - step)
        .min(cap)
        .copysign(error);
    state.pitch_rate_rad_s += (rate - state.pitch_rate_rad_s).clamp(-step, step);
    state.pitch += state.pitch_rate_rad_s * dt;
}

/// How far it sinks pulling its nose up from a dive to level at its max
/// rate (`max_turn_rad_s`, the rate kept to clear what it would hit): the
/// nose first stops any drop it is still making, then one more tick of the
/// dive, half the time its pitch rate takes to build at the dive's sink,
/// and the sink of an arc at that rate: nothing climbing.
pub fn pull_out_m(state: &FixedWingState, limits: &FixedWingLimits, dt: f64) -> f64 {
    let rate = limits.max_turn_rad_s.max(f64::MIN_POSITIVE);
    let accel = rate / limits.pitch_in_s.max(f64::MIN_POSITIVE);
    let dropping = (-state.pitch_rate_rad_s).max(0.0);
    let stop_s = dropping / accel;
    let pitch = state.pitch - dropping * stop_s * 0.5;
    if pitch >= 0.0 {
        return 0.0;
    }
    let sink = state.speed_mps * libm::tan(-pitch);
    sink * (dt + stop_s + 0.5 * limits.pitch_in_s)
        - state.speed_mps * libm::log(libm::cos(pitch)) / rate
}

/// Seconds it takes to push its nose over from level into a dive of
/// `angle_rad` at the rate it turns at, its pitch rate built and eased off
/// over `pitch_in_s`.
pub fn push_over_s(limits: &FixedWingLimits, angle_rad: f64) -> f64 {
    angle_rad / limits.turn_rad_s.max(f64::MIN_POSITIVE) + limits.pitch_in_s
}

/// Its max turn rate as a share of the rate it turns at by choice.
pub fn reserve(limits: &FixedWingLimits) -> f64 {
    (limits.max_turn_rad_s / limits.turn_rad_s.max(f64::MIN_POSITIVE)).max(1.0)
}

/// How fast its turn rate can change on its way to `rate` (radians a
/// second), in radians a second squared: from wings level to that rate over
/// its roll-in, and never slower than to `turn_rad_s`. Turning by choice it
/// rolls gently; breaking to its max off an edge, it rolls as hard.
pub fn roll_rad_s2(limits: &FixedWingLimits, rate: f64) -> f64 {
    rate.abs().max(limits.turn_rad_s) / limits.roll_in_s.max(f64::MIN_POSITIVE)
}

/// The velocity along the aircraft's nose: its speed over the ground along
/// its heading, climbing or sinking by its pitch.
pub fn velocity(state: &FixedWingState) -> V3 {
    v3(
        libm::cos(state.heading) * state.speed_mps,
        libm::sin(state.heading) * state.speed_mps,
        libm::tan(state.pitch) * state.speed_mps,
    )
}

/// The unit vector along a nose at `heading` and `pitch`: where a gun fixed
/// in a winged airframe points.
pub fn nose(heading: f64, pitch: f64) -> V3 {
    let (sin_p, cos_p) = libm::sincos(pitch);
    v3(
        libm::cos(heading) * cos_p,
        libm::sin(heading) * cos_p,
        sin_p,
    )
}
