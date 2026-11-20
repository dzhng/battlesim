//! What the battle hands a skirmish AI player at each decision: its side,
//! entry and the side planner's assignment, and the policy that answers
//! them with ordinary orders.

use contract::catalog::Faction;
use contract::command::Order;
use contract::encounter::EntrySite;
use contract::ids::{Side, UnitId};
use contract::observation::ObservationFrame;
use contract::scenario::Rules;

use super::team;

/// A side-scoped skirmish policy. Implementations emit the same ordinary
/// orders as a human controller; the battle host owns admission and replay.
pub trait SkirmishPolicy {
    fn decide(
        &mut self,
        input: DecisionInput<'_>,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order>;
}

/// The assignment a side planner gives one AI player for this decision.
#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub main_objective: [f64; 2],
    pub staging_point: [f64; 2],
    pub force_advance: bool,
    /// The player's parties (help, hold, claim): units sent from its main
    /// group to another objective.
    pub parties: Vec<PartyTask>,
    /// What the player's teammates field and have bought, by kind: the
    /// side's force beyond the player's own frame, so a purchase that
    /// answers the side's threat counts the whole team's answer.
    pub allies: Vec<contract::catalog::TypeIndex>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartyTask {
    pub objective: [f64; 2],
    pub units: Vec<UnitId>,
    pub kind: team::PartyKind,
}

#[derive(Clone, Copy)]
pub struct DecisionInput<'a> {
    pub side: Side,
    pub faction: Faction,
    pub entry: &'a EntrySite,
    pub assignment: Option<&'a Assignment>,
}
