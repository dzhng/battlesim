//! The encounter's referee: judges the hold from authoritative state, as a
//! referee must, and publishes the verdict.
use contract::observation::{EncounterResult, EncounterStatus};
use contract::scenario::EncounterRules;

use crate::units::Unit;

/// The referee: blue succeeds after an eligible ground combat unit of the
/// attacker holds the zone with no living defender in it for the hold time;
/// fails when the attacker has no combat unit left; past the assessment time,
/// inconclusive (play continues and may still succeed).
#[derive(Clone, Debug, Default)]
pub struct Referee {
    held_ticks: u64,
    result: Option<EncounterResult>,
}

impl Referee {
    pub fn judge(
        &mut self,
        rules: &EncounterRules,
        catalog: &contract::catalog::Catalog,
        units: &[Unit],
        tick: u64,
        tick_hz: u32,
    ) -> EncounterStatus {
        let hz = tick_hz as f64;
        let c = rules.success_zone_center;
        let inside = |u: &Unit| {
            libm::hypot(u.position.x - c[0], u.position.y - c[1]) <= rules.success_zone_radius_m
        };
        // A combat unit carries a weapon: its components say so, not its role.
        let combat = |u: &Unit| u.alive() && !catalog.mounts(u.kind).is_empty();
        let attackers: Vec<&Unit> = units
            .iter()
            .filter(|u| u.side == rules.attacker && combat(u))
            .collect();
        let held = attackers.iter().any(|u| inside(u))
            && !units
                .iter()
                .any(|u| u.side != rules.attacker && u.alive() && inside(u));
        self.held_ticks = if held { self.held_ticks + 1 } else { 0 };
        if matches!(self.result, None | Some(EncounterResult::Inconclusive)) {
            if self.held_ticks as f64 >= rules.hold_s * hz {
                self.result = Some(EncounterResult::Captured);
            } else if attackers.is_empty() {
                self.result = Some(EncounterResult::Defeated);
            } else if tick as f64 >= rules.max_assessment_s * hz {
                self.result = Some(EncounterResult::Inconclusive);
            }
        }
        EncounterStatus {
            held_s: self.held_ticks as f64 / hz,
            result: self.result.unwrap_or(EncounterResult::Running),
        }
    }

    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.held_ticks)
            .u64(self.result.map_or(u64::MAX, |r| r as u64));
    }
}
