//! Reversible deployment (L01, L02): one progress value per deploying unit,
//! counted in whole ticks so equal durations and reversals are exact, plus the
//! posture the unit should hold while it has nowhere to go. Movement intent
//! always heads toward packed; readiness for service and movement both read
//! this owner, never a copy.
use contract::observation::{DeploymentState, Posture};
use contract::scenario::{Rules, UnitKind};

use crate::units::Unit;

#[derive(Clone, Debug)]
pub struct Deployment {
    /// Ticks of setup completed, in `0..=duration`.
    pub current: u32,
    /// Ticks to deploy fully, and equally to pack fully.
    pub duration: u32,
    /// The posture held while the unit has no movement intent: deployed by
    /// default (a stopped unit sets up), packed after an explicit Pack.
    pub stationary: Posture,
}

impl Deployment {
    /// Move one tick toward `target`. Deploying and packing take the same
    /// `duration` ticks, and a reversal continues from wherever progress stands.
    pub fn advance(&mut self, target: Posture) {
        self.current = match target {
            Posture::Deployed => (self.current + 1).min(self.duration),
            Posture::Packed => self.current.saturating_sub(1),
        };
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

/// The deployment a fresh unit of `kind` starts with: packed, and (with no
/// orders yet) heading to deployed. `None` for units that do not set up.
pub fn initial(kind: UnitKind, rules: &Rules) -> Option<Deployment> {
    let seconds = match kind {
        UnitKind::Supply => rules.service.deploy_and_pack_s,
        UnitKind::Rifle | UnitKind::Recon | UnitKind::At | UnitKind::Tank | UnitKind::Jeep => {
            return None
        }
    };
    Some(Deployment {
        current: 0,
        duration: ((seconds * rules.tick_hz as f64).round() as u32).max(1),
        stationary: Posture::Deployed,
    })
}

/// Where a unit's progress is heading now: packed whenever it has somewhere to
/// go, otherwise its stationary posture.
pub fn target(unit: &Unit) -> Option<Posture> {
    let d = unit.deployment.as_ref()?;
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
        unit.deployment.as_mut().unwrap().advance(target);
    }
}

/// What the owning side sees of a unit's deployment.
pub fn state(unit: &Unit) -> Option<DeploymentState> {
    let d = unit.deployment.as_ref()?;
    Some(DeploymentState {
        progress: d.progress(),
        target: target(unit)?,
    })
}
