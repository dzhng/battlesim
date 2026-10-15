//! Fixed-wing ground attack runs. A pass is a short-lived flight plan around
//! one ground aim point; weapon release remains owned by the flight solver.

use contract::random::Rng;
use contract::scenario::FixedWingFlight;

use crate::flight::Release;
use crate::math::{v2, wrap_angle, V2, V3};
use crate::weapons::nose_on_bearing;

use super::fixed_wing::{FixedWingLimits, FixedWingState};
use super::steer::Intent;
use crate::weapons::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassWeapon {
    Bomb,
    GroundMissile,
    Gun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassPhase {
    /// Flying straight on until a turn in on the aim leaves a full run-in.
    LineUp,
    /// Turning in on the aim, then flying at it until the dive.
    TurnIn,
    Dive,
    Release,
    PullOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassPlan {
    /// The order target this pass is flying against. Keeping it with the
    /// phase lets attack-move finish a leg before choosing a new reaction.
    pub target: Target,
    pub aim: V3,
    pub phase: PassPhase,
    pub weapon: PassWeapon,
    /// The seeded height used for the attack leg, in metres above ground.
    pub agl: f64,
    /// The approach bearing through the aim point.
    pub heading: f64,
    pub cruise_agl: f64,
    pub released: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReleaseCue {
    pub at: V2,
}

/// Limited ground stores take a run before the unlimited fixed gun.
pub fn choose_weapon(weapons: &[PassWeapon]) -> Option<PassWeapon> {
    weapons
        .iter()
        .copied()
        .find(|w| matches!(w, PassWeapon::Bomb | PassWeapon::GroundMissile))
        .or_else(|| weapons.iter().copied().find(|w| *w == PassWeapon::Gun))
}

fn ceiling(weapon: PassWeapon, air: &FixedWingFlight) -> f64 {
    match weapon {
        PassWeapon::Bomb => air.bomb_ceiling_m,
        PassWeapon::Gun | PassWeapon::GroundMissile => air.strafe_ceiling_m,
    }
}

/// Start a run. Existing altitude is retained when already under its weapon's
/// ceiling; otherwise a deterministic height in the permitted band is used.
pub fn begin(
    state: &FixedWingState,
    limits: &FixedWingLimits,
    aim: V3,
    weapon: PassWeapon,
    air: &FixedWingFlight,
    rng: &mut Rng,
) -> PassPlan {
    let max = ceiling(weapon, air);
    let floor = limits.floor_agl_m;
    let current = state.pos.z;
    let agl = if current <= max {
        current.max(floor)
    } else {
        floor + rng.unit() * (max - floor)
    };
    let heading = libm::atan2(aim.y - state.pos.y, aim.x - state.pos.x);
    PassPlan {
        target: Target::Ground(aim),
        aim,
        phase: PassPhase::LineUp,
        weapon,
        agl,
        heading,
        cruise_agl: limits.cruise_agl_m,
        released: false,
    }
}

fn line(plan: &PassPlan, aim: V3, agl: f64) -> Intent {
    Intent::Line {
        through: aim.xy(),
        heading: plan.heading,
        agl,
    }
}

/// The circle a full-rate turn toward `aim` flies, on the side it lies:
/// its centre and radius.
fn turn_circle(state: &FixedWingState, limits: &FixedWingLimits, aim: V2) -> (V2, f64) {
    let to_aim = aim - state.pos.xy();
    let side = wrap_angle(libm::atan2(to_aim.y, to_aim.x) - state.heading).signum();
    let radius = state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE);
    let left = v2(-libm::sin(state.heading), libm::cos(state.heading));
    (state.pos.xy() + left * (radius * side), radius)
}

/// Advance the run and return the steering intent plus a one-tick release cue.
///
/// The line-up is flown from where the jet is. It flies straight on until
/// turning in on the aim would leave it pointed at the aim a full run-in
/// out (the aim lies that far along a tangent from the turn's circle), then
/// turns in and holds the turn until its nose is on. The dive begins only
/// nose-on and within the run-in, so every pass, the first or a repeat,
/// dives in from the approach side.
pub fn advance(
    plan: &mut PassPlan,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    aim: V3,
    release: &Release,
) -> (Intent, Option<ReleaseCue>) {
    let to_aim = aim.xy() - state.pos.xy();
    let distance = to_aim.length();
    let bearing = libm::atan2(to_aim.y, to_aim.x);
    let release_distance = (state.pos.xy() - release.at).length();
    let speed = state.speed_mps.max(1.0);
    // The straight, nose-on leg a dive is flown along.
    let run_in = speed * 2.0;
    let nose_on = nose_on_bearing(state.heading, bearing);
    let (centre, radius) = turn_circle(state, limits, aim.xy());
    match plan.phase {
        PassPhase::LineUp => {
            if nose_on || (aim.xy() - centre).at_least_radius(libm::hypot(radius, run_in)) {
                plan.phase = PassPhase::TurnIn;
                plan.heading = bearing;
                (line(plan, aim, plan.agl), None)
            } else {
                let straight_on = Intent::Line {
                    through: state.pos.xy(),
                    heading: state.heading,
                    agl: plan.agl,
                };
                (straight_on, None)
            }
        }
        PassPhase::TurnIn => {
            plan.heading = bearing;
            if nose_on && distance <= run_in {
                plan.phase = PassPhase::Dive;
            } else if !nose_on && (aim.xy() - centre).inside_radius(radius) {
                // Pushed inside its own turn (by the edge guard): no turn
                // brings the nose on from here, so it lines up again.
                plan.phase = PassPhase::LineUp;
            }
            (line(plan, aim, plan.agl), None)
        }
        PassPhase::Dive => {
            let forward = v2(libm::cos(plan.heading), libm::sin(plan.heading));
            let passed_release = (state.pos.xy() - release.at).dot(forward) >= 0.0;
            if release_distance <= speed * 0.75 || passed_release {
                plan.phase = PassPhase::Release;
            }
            (line(plan, aim, plan.agl), None)
        }
        PassPhase::Release => {
            plan.phase = PassPhase::PullOut;
            plan.released = true;
            (
                line(plan, aim, plan.cruise_agl),
                Some(ReleaseCue { at: release.at }),
            )
        }
        PassPhase::PullOut => {
            // The jet has cleared the aim line once it is two speed-lengths
            // out; a five-length gate is outside the authored orbit radius.
            if distance > speed * 2.0 {
                plan.phase = PassPhase::LineUp;
                plan.released = false;
            }
            (line(plan, aim, plan.cruise_agl), None)
        }
    }
}
