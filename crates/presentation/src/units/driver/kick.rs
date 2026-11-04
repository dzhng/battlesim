//! The hull recoil spring, driven by the last published round.
use super::*;

/// A hull's springs (`HullKick`): their decay and ringing rates, and the
/// speed a full kick starts them at so that they swing out to 1.
struct Springs {
    decay: f64,
    ring: f64,
    start: f64,
}

impl Springs {
    fn of(kick: &HullKick) -> Self {
        let omega = 2.0 * PI * kick.hz;
        let decay = kick.damping * omega;
        let ring = omega * (1.0 - kick.damping * kick.damping).sqrt();
        // Kicked from rest at speed v, it swings out to (v/ring)·e^(−decay·t)·sin(ring·t),
        // furthest where tan(ring·t) = ring/decay.
        let peak = libm::atan2(ring, decay) / ring;
        let start = ring / (libm::exp(-decay * peak) * libm::sin(ring * peak));
        Self { decay, ring, start }
    }

    /// A damped spring's state (`x`, `v`) run on `dt` seconds, exactly: the
    /// same wherever the frames fall.
    fn ring_on(&self, x: f64, v: f64, dt: f64) -> (f64, f64) {
        let e = libm::exp(-self.decay * dt);
        let c = libm::cos(self.ring * dt);
        let n = libm::sin(self.ring * dt) / self.ring;
        (
            e * (x * c + (v + self.decay * x) * n),
            e * (v * c
                - ((self.ring * self.ring + self.decay * self.decay) * x + self.decay * v) * n),
        )
    }
}

/// A hull's swing below which it is at rest, in kicks.
const KICK_REST: f64 = 1e-4;

/// Ring a hull on the ground on its springs through `dt`, then kick it for
/// each new round its mounts fired since `shots`, by the round's kick, away
/// from where the round went: its motion, or None at rest. `k` is its roof's
/// throw along its own x and y, in kicks, and how fast. The roof swings
/// along its kick: the hull turns about its base, and slides with it.
pub(super) fn kick_hull(
    k: &mut [f64; 4],
    shots: &[f64],
    unit: &FeedUnit,
    ty: &UnitType,
    feel: &HullKick,
    dt: f64,
) -> Option<[f64; 16]> {
    let springs = Springs::of(feel);
    let [x, y, vx, vy] = k;
    (*x, *vx) = springs.ring_on(*x, *vx, dt);
    (*y, *vy) = springs.ring_on(*y, *vy, dt);
    for (i, m) in unit.mounts.iter().enumerate() {
        let fired = m.shots - shots.get(i).copied().unwrap_or(m.shots);
        let kick = match ty.mounts.get(i).and_then(|row| row.weapons.get(m.round)) {
            Some(row) if fired > 0.0 => fired * row.kick.hull,
            _ => 0.0,
        };
        if kick <= 0.0 {
            continue;
        }
        let rel = m.bearing - unit.yaw;
        *vx -= libm::cos(rel) * kick * springs.start;
        *vy -= libm::sin(rel) * kick * springs.start;
    }
    let swing = libm::hypot(*x, *y);
    if swing < KICK_REST && libm::hypot(*vx, *vy) < KICK_REST * springs.ring {
        *k = [0.0; 4];
        return None;
    }
    // About the level axis across its swing, the roof thrown along it (just
    // kicked, it has not swung yet: no turn about any axis).
    let across = if swing == 0.0 { 1.0 } else { swing };
    let (s, c) = libm::sincos(feel.rock_deg * PI / 180.0 * swing / 2.0);
    let q = [-*y / across * s, *x / across * s, 0.0, c];
    Some(crate::units::skin::trs(
        [feel.slide_m * *x, feel.slide_m * *y, 0.0],
        q,
        [1.0; 3],
    ))
}
