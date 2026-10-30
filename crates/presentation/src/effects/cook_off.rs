//! A cook-off: a hull the side watched die brews up. A beat
//! after the killing hit its ammunition goes: fireballs roar out of the
//! turret ring, the turret is thrown up tumbling and falls back onto the
//! deck where the wreck has it, hops once and settles. The hull heaves as
//! the ammunition goes and dips as the turret strikes it. Both are the
//! wreck's own pieces, so the whole wreck takes over without a seam. What
//! the blast throws clear lies round the wreck a while, then sinks away.
//!
//! The flight is a thrown body's: a parabola `rise_m` high, back down in
//! √(8h/g), tumbling whole turns (so it leaves the ring upright); the hop is
//! another, `bounce_m` high. The hull's jolts swing twice and die away
//! within `settle_s`.
use serde::Deserialize;

use crate::seed::Mulberry32;
use crate::world::Vec3;

use crate::GRAVITY;

/// `presentation.effects.cook_off`.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CookOffFeel {
    /// Seconds from the killing hit to the ammunition going.
    pub delay_s: f64,
    /// How high the turret is thrown, metres.
    pub rise_m: f64,
    /// Whole turns it tumbles end over end, and turns it spins upright, in flight.
    pub flip_turns: f64,
    pub spin_turns: f64,
    /// How high it hops off the deck, metres, and how far it rocks, degrees.
    pub bounce_m: f64,
    pub bounce_tilt_deg: f64,
    /// The hull's heave as the ammunition goes and dip as the turret strikes,
    /// metres; how far it rocks with each, degrees; seconds each dies away in.
    pub heave_m: f64,
    pub jolt_m: f64,
    pub rock_deg: f64,
    pub settle_s: f64,
    pub fireballs: Vec<Fireball>,
    /// The landing turret's dust, as a blast's plume of this scale.
    pub landing_dust: f64,
    /// Sparks off the deck as the turret lands.
    pub landing_sparks: f64,
    pub debris: DebrisFeel,
}

/// A fireball out of the ring, `after_s` after the ammunition goes, `up_m`
/// above the hull's top, of blast radius `radius_m`, with the share
/// `sparks` of a blast's sparks.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fireball {
    pub after_s: f64,
    pub up_m: f64,
    pub radius_m: f64,
    pub sparks: f64,
}

/// The thrown debris: seconds it lies still from the blast, then sinking.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebrisFeel {
    pub hold_s: f64,
    pub fade_s: f64,
}

impl CookOffFeel {
    pub fn validate(&self) -> Result<(), String> {
        let at = "presentation.effects.cook_off";
        let whole = |x: f64| x >= 0.0 && x.fract() == 0.0;
        let checks = [
            (self.delay_s >= 0.0, "delay_s must be ≥ 0"),
            (self.rise_m > 0.0, "rise_m must be positive"),
            (
                whole(self.flip_turns),
                "flip_turns must be a whole number ≥ 0",
            ),
            (self.spin_turns.is_finite(), "spin_turns must be a number"),
            (
                [
                    self.bounce_m,
                    self.bounce_tilt_deg,
                    self.heave_m,
                    self.jolt_m,
                    self.rock_deg,
                ]
                .iter()
                .all(|&x| x >= 0.0),
                "bounce, tilt, heave, jolt and rock must be ≥ 0",
            ),
            (self.settle_s > 0.0, "settle_s must be positive"),
            (!self.fireballs.is_empty(), "fireballs needs at least one"),
            (
                self.fireballs.iter().all(|b| {
                    b.after_s >= 0.0 && b.radius_m > 0.0 && (0.0..=1.0).contains(&b.sparks)
                }),
                "each fireball needs after_s ≥ 0, a positive radius and sparks in [0, 1]",
            ),
            (self.landing_dust >= 0.0, "landing_dust must be ≥ 0"),
            (
                whole(self.landing_sparks),
                "landing_sparks must be a whole number ≥ 0",
            ),
            (self.debris.hold_s >= 0.0, "debris.hold_s must be ≥ 0"),
            (self.debris.fade_s > 0.0, "debris.fade_s must be positive"),
        ];
        match checks.iter().find(|(ok, _)| !ok) {
            Some((_, why)) => Err(format!("{at}: {why}")),
            None => Ok(()),
        }
    }

    /// Seconds after the killing hit that the turret strikes the deck.
    pub fn impact_after(&self) -> f64 {
        self.delay_s + airborne(self.rise_m)
    }

    /// Seconds after the killing hit that every piece lies still.
    pub fn landed_after(&self) -> f64 {
        self.impact_after() + airborne(self.bounce_m).max(self.settle_s)
    }

    /// Seconds after the killing hit that the thrown debris is gone.
    pub fn debris_gone_after(&self) -> f64 {
        self.delay_s + self.debris.hold_s + self.debris.fade_s
    }

    /// How far the thrown debris has sunk `age` seconds after the killing
    /// hit, metres, or None while it is not drawn: still through its hold,
    /// then sinking, easing in, until its top (`top` above the ground) is under.
    pub fn debris_sink(&self, age: f64, top: f64) -> Option<f64> {
        if !(age >= self.delay_s && age < self.debris_gone_after()) {
            return None;
        }
        let u = (age - self.delay_s - self.debris.hold_s).max(0.0) / self.debris.fade_s;
        Some(top * u * u)
    }

    /// The thrown turret's rigid motion `age` seconds after the killing hit,
    /// in the wreck's frame, column-major: what moves the piece from where
    /// the wreck has it (centred `lies`) to where it is. Over the ring (the
    /// wreck's origin) until the ammunition goes; the identity once settled.
    /// `seed` (the wreck's) picks the way it tumbles.
    pub fn turret_motion(&self, age: f64, lies: Vec3, seed: f64) -> [f64; 16] {
        let s = ((age - self.delay_s) / airborne(self.rise_m)).clamp(0.0, 1.0);
        let left = 1.0 - s;
        let u = (age - self.impact_after()) / airborne(self.bounce_m);
        let offset = [
            -lies[0] * left,
            -lies[1] * left,
            hop(self.rise_m, s) + hop(self.bounce_m, u),
        ];
        let tumble = 2.0 * PI * self.flip_turns * left + self.bounce_tilt_deg * DEG * hop(1.0, u);
        let across = Mulberry32::new(seed as u32).sample() * PI * 2.0;
        let turn = axis_angle([libm::cos(across), libm::sin(across), 0.0], tumble);
        let q = multiply(
            axis_angle([0.0, 0.0, 1.0], 2.0 * PI * self.spin_turns * left),
            turn,
        );
        motion(q, offset, lies)
    }

    /// The hull's rigid motion `age` seconds after the killing hit, in the
    /// wreck's frame: a heave as the ammunition goes, rocking fore and aft,
    /// and a dip as the turret strikes, rolling (`seed`'s way); each dies
    /// away within `settle_s`. It turns about its base centre.
    pub fn hull_motion(&self, age: f64, seed: f64) -> [f64; 16] {
        let blast = jolt(age - self.delay_s, self.settle_s);
        let strike = jolt(age - self.impact_after(), self.settle_s);
        let offset = [0.0, 0.0, self.heave_m * blast - self.jolt_m * strike];
        let side = if (seed as u32) & 1 == 1 { 1.0 } else { -1.0 };
        let q = multiply(
            axis_angle([0.0, 1.0, 0.0], self.rock_deg * DEG * blast),
            axis_angle([1.0, 0.0, 0.0], side * self.rock_deg * DEG * strike),
        );
        motion(q, offset, [0.0; 3])
    }
}

const PI: f64 = std::f64::consts::PI;
const DEG: f64 = PI / 180.0;

/// Seconds a body thrown `rise` metres up is in the air.
fn airborne(rise: f64) -> f64 {
    (8.0 * rise / GRAVITY).sqrt()
}

/// Height on a hop's parabola `h` high, at `u` of its way (0 outside it).
fn hop(h: f64, u: f64) -> f64 {
    if u > 0.0 && u < 1.0 {
        4.0 * h * u * (1.0 - u)
    } else {
        0.0
    }
}

/// A jolt `t` seconds in: two swings, the first toward +1, dying away by `settle`.
fn jolt(t: f64, settle: f64) -> f64 {
    if !(t > 0.0 && t < settle) {
        return 0.0;
    }
    let u = t / settle;
    libm::sin(4.0 * PI * u) * (1.0 - u) * (1.0 - u)
}

type Quat = [f64; 4];

fn axis_angle(axis: Vec3, angle: f64) -> Quat {
    let (s, c) = (libm::sin(angle * 0.5), libm::cos(angle * 0.5));
    [s * axis[0], s * axis[1], s * axis[2], c]
}

fn multiply(a: Quat, b: Quat) -> Quat {
    let ([ax, ay, az, aw], [bx, by, bz, bw]) = (a, b);
    [
        ax * bw + aw * bx + ay * bz - az * by,
        ay * bw + aw * by + az * bx - ax * bz,
        az * bw + aw * bz + ax * by - ay * bx,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Rotation `q` about `origin`, then translation `v` (`mat4.fromRotationTranslationScaleOrigin`, unit scale).
fn motion(q: Quat, v: Vec3, o: Vec3) -> [f64; 16] {
    let [x, y, z, w] = q;
    let (x2, y2, z2) = (x + x, y + y, z + z);
    let (xx, xy, xz) = (x * x2, x * y2, x * z2);
    let (yy, yz, zz) = (y * y2, y * z2, z * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);
    let m = [
        1.0 - (yy + zz),
        xy + wz,
        xz - wy,
        xy - wz,
        1.0 - (xx + zz),
        yz + wx,
        xz + wy,
        yz - wx,
        1.0 - (xx + yy),
    ];
    [
        m[0],
        m[1],
        m[2],
        0.0,
        m[3],
        m[4],
        m[5],
        0.0,
        m[6],
        m[7],
        m[8],
        0.0,
        v[0] + o[0] - (m[0] * o[0] + m[3] * o[1] + m[6] * o[2]),
        v[1] + o[1] - (m[1] * o[0] + m[4] * o[1] + m[7] * o[2]),
        v[2] + o[2] - (m[2] * o[0] + m[5] * o[1] + m[8] * o[2]),
        1.0,
    ]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn feel() -> CookOffFeel {
        let feel: CookOffFeel = serde_json::from_value(serde_json::json!({
            "delay_s": 0.3, "rise_m": 6, "flip_turns": 1, "spin_turns": 0.6,
            "fireballs": [{ "after_s": 0, "up_m": 0.5, "radius_m": 5, "sparks": 1 }],
            "bounce_m": 0.4, "bounce_tilt_deg": 6, "heave_m": 0.25, "jolt_m": 0.08,
            "rock_deg": 2, "settle_s": 0.8, "landing_dust": 1.5, "landing_sparks": 8,
            "debris": { "hold_s": 10, "fade_s": 3 }
        }))
        .unwrap();
        feel.validate().unwrap();
        feel
    }

    /// Where the thrown turret lies on the wreck: aft of and beside its ring.
    const LIES: Vec3 = [-0.3, 0.45, 2.1];

    fn apply(m: &[f64; 16], p: Vec3) -> Vec3 {
        std::array::from_fn(|i| m[i] * p[0] + m[4 + i] * p[1] + m[8 + i] * p[2] + m[12 + i])
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn is_identity(m: &[f64; 16]) -> bool {
        let id: [f64; 16] = std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 });
        m.iter().zip(id).all(|(a, b)| close(*a, b))
    }

    #[test]
    fn the_turret_leaves_its_ring_rises_as_thrown_and_lands_where_the_wreck_has_it() {
        let f = feel();
        let flight = airborne(f.rise_m);
        let at = |age: f64| apply(&f.turret_motion(age, LIES, 17.0), LIES);
        // Over the ring until the ammunition goes, and as it does.
        for age in [0.0, f.delay_s] {
            let p = at(age);
            assert!(close(p[0], 0.0) && close(p[1], 0.0) && close(p[2], LIES[2]));
        }
        // The throw's height at the top of its arc; on the deck as it strikes.
        assert!(close(at(f.delay_s + flight / 2.0)[2], LIES[2] + f.rise_m));
        assert!(close(at(f.impact_after())[2], LIES[2]));
        let hop = airborne(f.bounce_m);
        assert!(close(
            at(f.impact_after() + hop / 2.0)[2],
            LIES[2] + f.bounce_m
        ));
        // Then exactly as the wreck has it: the whole takes over.
        for age in [f.landed_after(), 60.0] {
            assert!(is_identity(&f.turret_motion(age, LIES, 17.0)));
        }
    }

    #[test]
    fn the_hull_heaves_as_the_ammunition_goes_dips_as_the_turret_strikes_and_settles() {
        let f = feel();
        let lift = |age: f64| apply(&f.hull_motion(age, 17.0), [0.0, 0.0, 1.0])[2] - 1.0;
        assert!(close(lift(f.delay_s * 0.5), 0.0));
        assert!(lift(f.delay_s + f.settle_s / 8.0) > f.heave_m * 0.5);
        assert!(lift(f.impact_after() + f.settle_s / 8.0) < -f.jolt_m * 0.5);
        for age in [f.landed_after(), 60.0] {
            assert!(is_identity(&f.hull_motion(age, 17.0)));
        }
    }

    #[test]
    fn thrown_debris_lies_still_through_its_hold_then_sinks_and_is_gone() {
        let f = feel();
        let (top, lands) = (0.4, f.delay_s);
        assert_eq!(f.debris_sink(lands - 0.01, top), None);
        for age in [lands, lands + f.debris.hold_s] {
            assert_eq!(f.debris_sink(age, top), Some(0.0));
        }
        let fading: Vec<f64> = [0.1, 0.4, 0.7, 0.99]
            .map(|u| {
                f.debris_sink(lands + f.debris.hold_s + u * f.debris.fade_s, top)
                    .unwrap()
            })
            .to_vec();
        assert!(fading[0] > 0.0 && fading.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(f.debris_sink(f.debris_gone_after(), top), None);
    }

    #[test]
    fn a_throw_with_no_height_or_debris_that_never_fades_is_refused() {
        let mut f = feel();
        f.rise_m = 0.0;
        assert!(f.validate().unwrap_err().contains("rise_m"));
        let mut f = feel();
        f.debris.fade_s = 0.0;
        assert!(f.validate().unwrap_err().contains("fade_s"));
    }
}
