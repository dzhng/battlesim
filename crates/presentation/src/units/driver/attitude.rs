//! How a flying aircraft leans: its pitch and roll from how its drawn pose
//! moves, and the matrix that tilts it.
use super::*;

/// Nose up about +Y, then right side down about +X, column-major
/// (`mat4.fromYRotation(-pitch)` then `rotateX(roll)`).
pub(super) fn tilt(pitch: f64, roll: f64) -> [f64; 16] {
    let (s, c) = libm::sincos(-pitch);
    let (sx, cx) = libm::sincos(roll);
    let y = [0.0, 1.0, 0.0];
    let z = [s, 0.0, c];
    let mut m = [
        c, 0.0, -s, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for i in 0..3 {
        m[4 + i] = y[i] * cx + z[i] * sx;
        m[8 + i] = z[i] * cx - y[i] * sx;
    }
    m
}

#[derive(Default)]
pub(super) struct Flight {
    /// Its drawn ground velocity and acceleration, eased.
    velocity: [f64; 2],
    accel: [f64; 2],
    /// A fixed-wing aircraft's published 3D flight velocity, eased.
    path_velocity: [f64; 3],
    /// The (pitch, roll) they gave it last, held while the clock stands still.
    attitude: Option<(f64, f64)>,
}

/// How a winged aircraft follows its published flight path: pitch is the
/// path angle, and roll is the bank its full-rate turn needs (the angle that
/// tilts its lift into that turn, at most `max_deg`) in proportion to the
/// share of that rate its published yaw turns at now. So the bank builds
/// with the turn as the jet rolls in, and reaches its cap only at full rate.
/// A type that states no full rate banks for the rate it turns at.
fn fixed_wing_attitude(
    f: &mut Flight,
    feel: &TiltFeel,
    (was, was_yaw): (Vec3, f64),
    unit: &FeedUnit,
    full_rate: Option<f64>,
    dt: f64,
) -> Option<(f64, f64)> {
    let TiltFeel {
        gain,
        max_deg,
        smoothing_s,
    } = *feel;
    let k = 1.0 - libm::exp(-dt / smoothing_s);
    let measured = [
        (unit.position[0] - was[0]) / dt,
        (unit.position[1] - was[1]) / dt,
        (unit.position[2] - was[2]) / dt,
    ];
    for (velocity, next) in f.path_velocity.iter_mut().zip(measured) {
        *velocity += (next - *velocity) * k;
    }
    let horizontal = libm::hypot(f.path_velocity[0], f.path_velocity[1]);
    let max = max_deg * PI / 180.0;
    let limit = |angle: f64| (gain * angle).clamp(-max, max);
    let target_pitch = limit(libm::atan2(f.path_velocity[2], horizontal));
    let turn_rate = delta_angle(was_yaw, unit.yaw) / dt;
    let full = full_rate.unwrap_or(turn_rate.abs()).max(f64::MIN_POSITIVE);
    let share = (turn_rate / full).clamp(-1.0, 1.0);
    let target_roll = -limit(libm::atan2(horizontal * full, GRAVITY)) * share;
    let (old_pitch, old_roll) = f.attitude.unwrap_or_default();
    let pitch = old_pitch + (target_pitch - old_pitch) * k;
    let roll = old_roll + (target_roll - old_roll) * k;
    f.attitude = (pitch.abs() + roll.abs() > 1e-9).then_some((pitch, roll));
    f.attitude
}

/// How a flying aircraft leans, from how its drawn pose (position and yaw)
/// moved since the last frame (`was`). A winged aircraft follows its flight
/// path (see `fixed_wing_attitude`). Any other aircraft leans by its
/// acceleration: nose down to speed up, up to brake, into a turn, by the
/// angle that tilts its lift into its acceleration. Held while the clock
/// stands still; level, it carries no tilt.
pub(super) fn aircraft_attitude(
    f: &mut Flight,
    feel: &AttitudeFeel,
    was: (Vec3, f64),
    unit: &FeedUnit,
    ty: &UnitType,
    dt: f64,
) -> Option<(f64, f64)> {
    if dt == 0.0 {
        return f.attitude;
    }
    // The flight model, never the layer, decides how it tilts.
    let flight = ty.flight.unwrap_or(FlightModel::Rotor);
    let feel = feel.flight(flight);
    if flight == FlightModel::FixedWing {
        return fixed_wing_attitude(f, feel, was, unit, ty.turn_rad_s, dt);
    }
    let (was, _) = was;
    let k = 1.0 - libm::exp(-dt / feel.smoothing_s);
    let v = [
        (unit.position[0] - was[0]) / dt,
        (unit.position[1] - was[1]) / dt,
    ];
    let [ox, oy] = f.velocity;
    f.velocity = [ox + (v[0] - ox) * k, oy + (v[1] - oy) * k];
    let a = [(f.velocity[0] - ox) / dt, (f.velocity[1] - oy) / dt];
    f.accel = [
        f.accel[0] + (a[0] - f.accel[0]) * k,
        f.accel[1] + (a[1] - f.accel[1]) * k,
    ];
    let max = feel.max_deg * PI / 180.0;
    let tilt_by = |a: f64| (feel.gain * libm::atan(a / GRAVITY)).clamp(-max, max);
    let (s, c) = libm::sincos(unit.yaw);
    let pitch = -tilt_by(f.accel[0] * c + f.accel[1] * s);
    let roll = -tilt_by(-f.accel[0] * s + f.accel[1] * c);
    f.attitude = (pitch.abs() + roll.abs() > 1e-9).then_some((pitch, roll));
    f.attitude
}
