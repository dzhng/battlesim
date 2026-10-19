use super::{PlayerIndex, TeamPlan, TeamStance};

impl TeamPlan {
    pub fn for_player(&self, player: PlayerIndex) -> Option<&super::PlayerPlan> {
        self.players.iter().find(|plan| plan.player == player)
    }

    pub fn parties_for(&self, player: PlayerIndex) -> Vec<&super::Party> {
        self.parties
            .iter()
            .filter(|party| party.owner == player)
            .collect()
    }

    pub fn assignment_for(&self, player: PlayerIndex) -> Option<crate::skirmish_ai::Assignment> {
        let plan = self.for_player(player)?;
        let fallback = self.outgunned.contains(&player);
        let (main_objective, staging_point) = if fallback {
            (plan.staging_point, plan.staging_point)
        } else {
            match self.stance {
                TeamStance::Push => (plan.main_objective.center, plan.staging_point),
                TeamStance::Hold | TeamStance::FallBack => (plan.staging_point, plan.staging_point),
            }
        };
        let parties = self
            .parties_for(player)
            .into_iter()
            .map(|party| crate::skirmish_ai::PartyTask {
                objective: party.objective.center,
                units: party.units.clone(),
                kind: party.kind,
            })
            .collect();
        Some(crate::skirmish_ai::Assignment {
            main_objective,
            staging_point,
            force_advance: fallback || self.stance != TeamStance::Hold,
            parties,
        })
    }
}
