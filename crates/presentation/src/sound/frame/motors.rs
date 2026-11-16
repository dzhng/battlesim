//! A flying round's loop: it follows connected stretches of the same round,
//! heard where the round is, until it lands.
use std::sync::Arc;

use super::{clamp01, profile, Bus, LoopKey, LoopWant, Ordered, Sound};
use crate::effects::publication::Segment;
use crate::world::Vec3;

pub(super) struct Motor {
    kind: Arc<str>,
    from: Vec3,
    to: Vec3,
    t0: f64,
}

/// Where a flying round's stretch ends, as a loop's key.
fn end_key(p: &Vec3) -> [u64; 3] {
    p.map(f64::to_bits)
}

impl Sound {
    /// Adds `s`, a round still flying after the tick from `t0`, to `motors`,
    /// carrying over the loop of the stretch it continues.
    pub(super) fn follow_motor(
        &mut self,
        motors: &mut Ordered<[u64; 3], Motor>,
        s: &Segment,
        t0: f64,
    ) {
        let end = *s.path.last().expect("two points");
        motors.set(
            end_key(&end),
            Motor {
                kind: s.kind.clone(),
                from: s.path[0],
                to: end,
                t0,
            },
        );
        let old = LoopKey::Motor(end_key(&s.path[0]));
        if let Some(v) = self.loops.remove(&old) {
            self.loops.set(LoopKey::Motor(end_key(&end)), v);
        }
    }

    /// Each flying round's loop at presentation time `clock`, where it is.
    pub(super) fn motor_wants(&self, clock: f64, wants: &mut Vec<LoopWant>) {
        let p = &self.tables.audio;
        for (key, m) in &self.motors.0 {
            let u = clamp01((clock - m.t0) / self.dt);
            let at = [0, 1, 2].map(|i| m.from[i] + u * (m.to[i] - m.from[i]));
            let row = self
                .tables
                .looks
                .get(&m.kind)
                .sound
                .motor
                .as_ref()
                .expect("motor was admitted");
            wants.push(LoopWant {
                key: LoopKey::Motor(*key),
                sound: row.sound.clone(),
                bus: Bus::Effects,
                gain: row.gain,
                rate: 1.0,
                position: Some(at),
                priority: 0.0,
                distance: p.distance[profile::WEAPONS],
            });
        }
    }
}
