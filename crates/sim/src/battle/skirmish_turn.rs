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
        let mut deciding = None;
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
            // A side's decisions are a tick's worth of planning. Its seats
            // decide together, as a team's push sets off together; the
            // opening is every side's at once, and after it the sides take
            // turns, a tick apart.
            let taken = self.tick > 0 && deciding.is_some_and(|other| other != side);
            if taken || !ai.wants_decision(self.tick) {
                self.skirmish_ai.insert(id, ai);
                continue;
            }
            deciding = Some(side);
            let player_plan =
                plans[side.index()].for_player(contract::players::PlayerIndex(index as u16));
            let entry_index =
                player_plan.map_or_else(|| crate::entries::centre(sites, side), |plan| plan.lane);
            let entry = crate::entries::site(sites, entry_index);
            let allies = crate::skirmish_ai::team::allies(
                &self.observations[side.index()],
                contract::players::PlayerIndex(index as u16),
            );
            let mut frame = self.observations[side.index()].clone();
            frame
                .own
                .retain(|unit| unit.owner == Some(contract::players::PlayerIndex(index as u16)));
            if let Some(view) = frame.skirmish.as_mut() {
                view.players
                    .retain(|player| player.index == contract::players::PlayerIndex(index as u16));
            }
            let assignment = plans[side.index()]
                .assignment_for(contract::players::PlayerIndex(index as u16), allies);
            let opening = self.tick == 0
                && frame.own.is_empty()
                && frame
                    .skirmish
                    .as_ref()
                    .is_some_and(|view| view.players.first().is_none_or(|p| p.pending.is_empty()))
                && !ai.has_purchase_plan()
                && openings.get(&id).is_some_and(|opening| !opening.is_empty());
            let (orders, opening_conditions) = if opening {
                let opening = openings.get(&id).expect("opening was checked");
                let conditions = opening
                    .iter()
                    .map(|purchase| purchase.condition.clone())
                    .collect();
                let mut orders: Vec<_> = opening
                    .iter()
                    .cloned()
                    .map(|purchase| Order::ConfirmPurchase {
                        variant: purchase.variant,
                        carrier: purchase.carrier,
                        destination: purchase.destination,
                    })
                    .collect();
                // An authored opening is the AI's preparation batch; once it
                // has been issued, the seat readies like any other.
                orders.push(Order::Ready);
                (orders, conditions)
            } else {
                // A decision weighs the same trip more than once: each is
                // worked out once.
                let mut asked = BTreeMap::new();
                (
                    ai.decide(
                        crate::skirmish_ai::DecisionInput {
                            side,
                            faction,
                            entry,
                            assignment: assignment.as_ref(),
                        },
                        &frame,
                        &self.rules,
                        |unit, destination| {
                            *asked
                                .entry((unit, destination.map(f64::to_bits)))
                                .or_insert_with(|| self.route_time(side, unit, destination))
                        },
                    ),
                    Vec::new(),
                )
            };
            self.skirmish_ai.insert(id.clone(), ai);
            commands.push((index, id, side, orders, opening_conditions));
        }
        for (index, player, side, orders, opening_conditions) in commands {
            // A refused purchase stops the turn's later purchases, so a
            // purchase plan never buys out of order. The turn's other orders,
            // readiness and movement, are still issued (I10).
            let mut purchase_refused = false;
            let mut opening_index = 0;
            for order in orders {
                // The cards a purchase names: two for a squad with its ride.
                let cards = match &order {
                    contract::command::Order::ConfirmPurchase { carrier, .. } => {
                        1 + usize::from(carrier.is_some())
                    }
                    _ => 0,
                };
                let purchase = cards > 0;
                if purchase && purchase_refused {
                    continue;
                }
                let ack = self.issue(CommandEnvelope {
                    side,
                    player: Some(player.clone()),
                    seq: self.player_next_seq[index],
                    order,
                    queued: false,
                });
                let accepted = ack.error.is_none();
                if purchase {
                    if accepted {
                        if let Some(condition) =
                            opening_conditions.get(opening_index).and_then(Clone::clone)
                        {
                            if let Some(reservation) = self
                                .skirmish
                                .as_mut()
                                .and_then(|s| s.players[index].reservations.back_mut())
                            {
                                reservation.data.condition = Some(condition);
                            }
                        }
                    }
                    opening_index += 1;
                }
                // A purchase may be refused: a purchase plan names cards the
                // battle may not sell, and tries them again. Any other order
                // comes from the AI's own view, so it is never malformed; the
                // world may still refuse it, out of a state the AI does not
                // see, such as its order rate or room to stand (I09).
                debug_assert!(
                    purchase || !ack.error.as_ref().is_some_and(OrderError::is_malformed),
                    "{player}'s AI order malformed: {:?}",
                    ack.error
                );
                if purchase {
                    if let Some(ai) = self.skirmish_ai.get_mut(&player) {
                        ai.purchase_result(accepted, cards);
                    }
                }
                self.ai_receipts.push(AiReceipt {
                    tick: self.tick,
                    player: player.clone(),
                    accepted,
                    error: ack.error,
                });
                purchase_refused |= purchase && !accepted;
            }
        }
    }
}
