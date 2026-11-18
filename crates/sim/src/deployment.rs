//! Reversible deployment (L01, L02): one progress value per deploying unit,
//! counted in whole ticks so equal durations and reversals are exact, plus the
//! posture the unit should hold while it has nowhere to go. Movement intent
//! always heads toward packed; readiness for service and movement both read
//! this owner, never a copy.
use contract::catalog::UnitType;
use contract::observation::{DeploymentState, Posture};
use contract::scenario::Rules;

use crate::units::Unit;

#[derive(Clone, Debug)]
pub struct Deployment {
    /// Integer setup progress, in `0..=duration`.
    pub current: u32,
    /// Common tick lattice, so either direction and reversals stay exact.
    pub duration: u32,
    pub deploy_step: u32,
    pub pack_step: u32,
    /// Progress at which an aircraft's descent ends and it stands on the
    /// ground; zero for a deployer that never flies.
    pub drop_duration: u32,
    /// The landed state is a latch, not a calculation from progress (P1).
    pub landed: bool,
    /// Height at the start of the current descent, for the one landing law.
    pub landing_start_z: Option<f64>,
    /// The posture held while the unit has no movement intent: deployed by
    /// default (a stopped unit sets up), packed after an explicit Pack.
    pub stationary: Posture,
}

impl Deployment {
    /// Move one tick toward `target`, continuing from current progress on reversal.
    pub fn advance(&mut self, target: Posture) {
        self.current = match target {
            Posture::Deployed => (self.current + self.deploy_step).min(self.duration),
            Posture::Packed => self.current.saturating_sub(self.pack_step),
        };
        if self.lands() && target == Posture::Deployed && self.current >= self.drop_duration {
            self.landed = true;
        }
        if self.lands() && target == Posture::Packed && self.current == 0 {
            self.landed = false;
            self.landing_start_z = None;
        }
    }

    /// It sets up by landing: an aircraft's deployment.
    pub fn lands(&self) -> bool {
        self.drop_duration > 0
    }

    /// Setup complete: the one readiness predicate service consumes.
    pub fn fully_deployed(&self) -> bool {
        self.current == self.duration
    }

    /// Fully packed: the movement gate is open.
    pub fn packed(&self) -> bool {
        self.current == 0
    }

    pub fn progress(&self) -> f64 {
        self.current as f64 / self.duration as f64
    }
}

/// The deployment a fresh unit of type `t` starts with: packed, and (with
/// no orders yet) heading to deployed. `None` for types that do not set up.
pub fn initial(t: &UnitType, rules: &Rules) -> Option<Deployment> {
    let timings = t.capabilities.deploy?;
    let ticks = |seconds: f64| ((seconds * rules.tick_hz as f64).round() as u32).max(1);
    let deploy = ticks(timings.seconds);
    let pack = ticks(timings.pack_seconds);
    let (mut a, mut b) = (deploy, pack);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let duration = (deploy / a)
        .checked_mul(pack)
        .expect("deployment tick lattice fits u32");
    Some(Deployment {
        current: 0,
        duration,
        deploy_step: duration / deploy,
        pack_step: duration / pack,
        // On the same lattice as `current`: the drop's ticks of deploy steps.
        drop_duration: t.landing().map_or(0, |landing| {
            ticks(landing.drop_seconds) * (duration / deploy)
        }),
        landed: false,
        landing_start_z: None,
        stationary: Posture::Deployed,
    })
}

/// Where a unit's progress is heading now: packed whenever it has somewhere to
/// go, otherwise its stationary posture.
pub fn target(unit: &Unit) -> Option<Posture> {
    let d = unit.deployment.as_ref()?;
    if d.lands() {
        let at_spot = unit.landing_goal.is_some_and(|goal| {
            (goal - unit.position.xy()).inside_radius(crate::movement::air::ARRIVAL_M)
        });
        return Some(
            if d.stationary == Posture::Deployed
                && at_spot
                && unit.movement_goal().is_none()
                && !unit.gliding()
            {
                Posture::Deployed
            } else {
                Posture::Packed
            },
        );
    }
    Some(if unit.movement_goal().is_some() {
        Posture::Packed
    } else {
        d.stationary
    })
}

/// Service eligibility as far as setup goes: a deploying unit at full progress.
/// Units that never deploy never serve.
pub fn fully_deployed(unit: &Unit) -> bool {
    unit.deployment
        .as_ref()
        .is_some_and(Deployment::fully_deployed)
}

/// Advance every deploying unit one tick toward its target.
pub fn advance_all(units: &mut [Unit]) {
    for unit in units.iter_mut() {
        let Some(target) = target(unit) else {
            continue;
        };
        let d = unit.deployment.as_mut().unwrap();
        if d.lands() && target == Posture::Deployed && d.current == 0 {
            d.landing_start_z.get_or_insert(unit.position.z);
        }
        d.advance(target);
    }
}

/// The descent share consumed by the landing law, in `[0, 1]`.
pub fn drop_progress(unit: &Unit) -> Option<f64> {
    let d = unit.deployment.as_ref()?;
    (d.lands()
        && d.current > 0
        && !d.landed
        && unit.landing_goal.is_some()
        && target(unit) == Some(Posture::Deployed))
    .then(|| (d.current as f64 / d.drop_duration as f64).clamp(0.0, 1.0))
}

/// The latched landed state (P1).
pub fn landed(unit: &Unit) -> bool {
    unit.deployment.as_ref().is_some_and(|d| d.landed)
}

/// A deploying rotor, or one holding a checked landing spot, does not
/// participate in air separation while it settles onto that spot.
pub fn separation_exempt(unit: &Unit) -> bool {
    let Some(d) = unit.deployment.as_ref() else {
        return false;
    };
    d.lands() && (d.current > 0 || unit.landing_goal.is_some())
}

/// What the owning side sees of a unit's deployment.
pub fn state(unit: &Unit) -> Option<DeploymentState> {
    let d = unit.deployment.as_ref()?;
    Some(DeploymentState {
        progress: d.progress(),
        target: target(unit)?,
    })
}
