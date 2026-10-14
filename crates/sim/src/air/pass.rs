//! Fixed-wing ground attack runs. A pass is a short-lived flight plan around
//! one ground aim point; weapon release remains owned by the flight solver.

use contract::random::Rng;
use contract::scenario::JetAir;

use crate::flight::Release;
use crate::math::{v2, V2, V3};

use super::jet::JetState;
use super::steer::Intent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassWeapon {
    Bomb,
    GroundMissile,
    Gun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassPhase {
    LineUp,
    Dive,
    Release,
    PullOut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassPlan {
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

fn ceiling(weapon: PassWeapon, air: &JetAir) -> f64 {
    match weapon {
        PassWeapon::Bomb => air.bomb_ceiling_m,
        PassWeapon::Gun | PassWeapon::GroundMissile => air.strafe_ceiling_m,
    }
}

/// Start a run. Existing altitude is retained when already under its weapon's
/// ceiling; otherwise a deterministic height in the permitted band is used.
pub fn begin(
    state: &JetState,
    aim: V3,
    weapon: PassWeapon,
    air: &JetAir,
    rng: &mut Rng,
) -> PassPlan {
    let max = ceiling(weapon, air);
    let current = state.pos.z;
    let agl = if current <= max {
        current.max(air.floor_agl_m)
    } else {
        air.floor_agl_m + rng.unit() * (max - air.floor_agl_m)
    };
    let heading = libm::atan2(aim.y - state.pos.y, aim.x - state.pos.x);
    PassPlan {
        phase: PassPhase::LineUp,
        weapon,
        agl,
        heading,
        cruise_agl: air.cruise_agl_m,
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

/// Advance the run and return the steering intent plus a one-tick release cue.
pub fn advance(
    plan: &mut PassPlan,
    state: &JetState,
    aim: V3,
    release: &Release,
) -> (Intent, Option<ReleaseCue>) {
    let distance = (state.pos.xy() - aim.xy()).length();
    let release_distance = (state.pos.xy() - release.at).length();
    let speed = state.speed_mps.max(1.0);
    match plan.phase {
        PassPhase::LineUp => {
            if distance <= speed * 2.0 {
                plan.phase = PassPhase::Dive;
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
            if distance > speed * 5.0 {
                plan.phase = PassPhase::LineUp;
                plan.released = false;
            }
            (line(plan, aim, plan.cruise_agl), None)
        }
    }
}
