//! A unit its own cover keeps from firing is driven to other ground.
use super::{live_goal, BalancedDoctrine, Tuning};
use contract::ids::{Tick, UnitId};
use contract::scenario::Rules;

/// A unit its own cover holds from firing: since when (while it stands
/// held), how often it has been moved for it, and whether it is on its way.
#[derive(Clone, Debug, Default)]
pub(super) struct Held {
    since: Option<Tick>,
    moves: usize,
    driving: bool,
}

/// What a held unit is to do at this decision.
pub(super) enum Relocation {
    /// Nothing yet: it has not been held long enough, or it is not held.
    Stay,
    /// Already on its way to other ground: leave its move alone.
    Driving,
    /// Drive to this ground.
    To([f64; 2]),
}

impl BalancedDoctrine {
    /// A unit its own cover keeps from firing (`BlockedTrajectory` on any
    /// weapon at every decision while it stands, for its tuning's
    /// `relocate_after_s`) is driven to other ground `relocate_m` off, as a
    /// player would move it: the first of eight bearings a route reaches, starting from its heading, one
    /// bearing further round for each earlier move. Arriving, it is watched
    /// afresh, and moved again if it is still held.
    pub(super) fn relocation(
        &mut self,
        unit: &contract::observation::OwnUnit,
        tick: Tick,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Relocation {
        let after = Tuning::ticks(self.tuning.relocate_after_s, rules.tick_hz);
        let reach = self.tuning.relocate_m;
        let held = self.held.entry(unit.id).or_default();
        if held.driving {
            // A route that failed on the way ends the move as arriving does.
            if live_goal(unit).is_some() {
                return Relocation::Driving;
            }
            held.driving = false;
        }
        let blocked = matches!(
            unit.state,
            contract::observation::MoveState::Idle | contract::observation::MoveState::RouteBlocked
        ) && unit
            .mounts
            .iter()
            .any(|m| m.reason == contract::observation::ActionReason::BlockedTrajectory);
        if !blocked {
            held.since = None;
            return Relocation::Stay;
        }
        let since = *held.since.get_or_insert(tick);
        if tick.saturating_sub(since) < after {
            return Relocation::Stay;
        }
        let step = std::f64::consts::FRAC_PI_4;
        let goal = (0..8)
            .map(|k| unit.yaw + step * (held.moves + k) as f64)
            .map(|bearing| {
                [
                    unit.position[0] + libm::cos(bearing) * reach,
                    unit.position[1] + libm::sin(bearing) * reach,
                ]
            })
            .find(|&goal| route_time(unit.id, goal).is_some_and(|t| t.is_finite() && t >= 0.0));
        let Some(goal) = goal else {
            return Relocation::Stay;
        };
        held.since = None;
        held.moves += 1;
        held.driving = true;
        Relocation::To(goal)
    }
}
