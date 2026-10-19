//! Per-player skirmish AI turn orchestration.

use super::*;

impl Battle {
    pub(super) fn skirmish_ai_turn(&mut self) {
        if self.replaying.is_some() || self.finished() {
            return;
        }
        let Some(skirmish) = &self.skirmish else {
            return;
        };
        let sites = &skirmish.setup.sites;
        let openings = &skirmish.setup.openings;
        let player_defs = &skirmish.players;
        self.team_memory[Side::Blue.index()].latch_group_readiness(
            &self.rules,
            &self.observations[Side::Blue.index()],
            self.team_tuning.group_radius_m,
        );
        self.team_memory[Side::Red.index()].latch_group_readiness(
            &self.rules,
            &self.observations[Side::Red.index()],
            self.team_tuning.group_radius_m,
        );
        crate::skirmish_ai::team::plan(
            Side::Blue,
            &self.rules,
            &self.observations[Side::Blue.index()],
            player_defs,
            sites,
            &mut self.team_memory[Side::Blue.index()],
            self.team_tuning,
        );
        crate::skirmish_ai::team::plan(
            Side::Red,
            &self.rules,
            &self.observations[Side::Red.index()],
            player_defs,
            sites,
            &mut self.team_memory[Side::Red.index()],
            self.team_tuning,
        );
        let plans = [
            self.team_memory[Side::Blue.index()]
                .cached_plan()
                .expect("blue plan"),
            self.team_memory[Side::Red.index()]
                .cached_plan()
                .expect("red plan"),
        ];
        let mut commands = Vec::new();
        for (index, player) in player_defs.iter().enumerate() {
            if player.controller != contract::players::ControllerKind::ActiveAi {
                continue;
            }
            let id = player.id.clone();
            let side = player.side;
            let faction = player.faction;
            let Some(mut ai) = self.skirmish_ai.remove(&id) else {
                continue;
            };
            if !ai.wants_decision(self.tick) {
                self.skirmish_ai.insert(id, ai);
                continue;
            }
            let player_plan =
                plans[side.index()].for_player(contract::players::PlayerIndex(index as u16));
            let entry_index =
                player_plan.map_or_else(|| crate::entries::centre(sites, side), |plan| plan.lane);
            let entry = crate::entries::site(sites, entry_index);
            let mut frame = self.observations[side.index()].clone();
            frame
                .own
                .retain(|unit| unit.owner == Some(contract::players::PlayerIndex(index as u16)));
            if let Some(view) = frame.skirmish.as_mut() {
                view.players
                    .retain(|player| player.index == contract::players::PlayerIndex(index as u16));
                if let Some(state) = self.skirmish.as_ref().and_then(|s| s.players.get(index)) {
                    view.credits = state.wallet as f64 / crate::skirmish::CREDIT_SCALE as f64;
                    view.unit_count = frame.own.len() as u32 + state.reservations.len() as u32;
                    view.pending = state.reservations.iter().map(|r| r.data.clone()).collect();
                    view.ready[side.index()] = state.ready;
                }
            }
            let assignment =
                plans[side.index()].assignment_for(contract::players::PlayerIndex(index as u16));
            let orders = if self.tick == 0
                && frame.own.is_empty()
                && frame
                    .skirmish
                    .as_ref()
                    .is_some_and(|view| view.players.first().is_none_or(|p| p.pending.is_empty()))
                && !ai.has_purchase_plan()
                && openings.get(&id).is_some_and(|opening| !opening.is_empty())
            {
                openings
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .cloned()
                    .map(|purchase| Order::ConfirmPurchase {
                        variant: purchase.variant,
                        destination: purchase.destination,
                    })
                    .collect()
            } else {
                ai.decide(
                    crate::skirmish_ai::DecisionInput {
                        side,
                        faction,
                        entry,
                        assignment: assignment.as_ref(),
                    },
                    &frame,
                    &self.rules,
                    |unit, destination| self.route_time(side, unit, destination),
                )
            };
            self.skirmish_ai.insert(id.clone(), ai);
            commands.push((index, id, side, orders));
        }
        for (index, player, side, orders) in commands {
            for order in orders {
                let purchase = matches!(order, contract::command::Order::ConfirmPurchase { .. });
                let ack = self.issue(CommandEnvelope {
                    side,
                    player: Some(player.clone()),
                    seq: self.player_next_seq[index],
                    order,
                    queued: false,
                });
                let accepted = ack.error.is_none();
                if purchase {
                    if let Some(ai) = self.skirmish_ai.get_mut(&player) {
                        ai.purchase_result(accepted);
                    }
                }
                self.ai_receipts.push(AiReceipt {
                    tick: self.tick,
                    player: player.clone(),
                    accepted,
                    error: ack.error,
                });
                if purchase && !accepted {
                    break;
                }
            }
        }
    }
}
