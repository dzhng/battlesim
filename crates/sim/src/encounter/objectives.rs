//! Eligibility and ordering of encounter objective candidates.
use super::*;

impl Planner<'_> {
    /// Whether an open approach lies on the attacker's side of its
    /// settlement: its bearing points toward the edge the attacker starts on.
    pub(super) fn faces_attacker(&self, approach: &contract::encounter::Approach) -> bool {
        let north = heading(approach_bearing(approach.from_rad, approach.to_rad))[1];
        match self.recipe.attacker_edge {
            Half::Bottom => north < 0.0,
            Half::Top => north > 0.0,
        }
    }

    /// The settlements in the order they are tried as the objective: the
    /// recipe's preference first, then those with a measured open approach
    /// on the attacker's side, then the largest, then plan order.
    pub(super) fn objective_order(&self) -> Vec<usize> {
        let settlements = &self.q.sites.settlements;
        let attacker = self.recipe.attacker_edge;
        let preferred = |i: usize| match self.recipe.objective.settlement {
            SettlementPreference::Main => i == 0,
            SettlementPreference::AttackerHalf => self.half_of(settlements[i].center) == attacker,
            SettlementPreference::DefenderHalf => self.half_of(settlements[i].center) != attacker,
        };
        let open = |i: usize| {
            self.q
                .sites
                .approaches
                .iter()
                .any(|a| a.settlement == i && self.faces_attacker(a))
        };
        let mut order: Vec<usize> = (0..settlements.len()).collect();
        order.sort_by(|&a, &b| {
            preferred(b)
                .cmp(&preferred(a))
                .then(open(b).cmp(&open(a)))
                .then(
                    settlements[b]
                        .area_m2()
                        .total_cmp(&settlements[a].area_m2()),
                )
                .then(a.cmp(&b))
        });
        order
    }
}
