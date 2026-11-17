//! Authored events and opponents issuing ordinary battle commands.

use super::*;

impl Battle {
    /// Give one player's own policy a fixed purchase plan. The policy emits
    /// ordinary purchase commands through its normal per-player sequence.
    pub fn script_purchases(
        &mut self,
        player: impl Into<String>,
        purchases: Vec<TypeIndex>,
        repeat_from: Option<usize>,
    ) {
        let player = player.into();
        if let Some(ai) = self.skirmish_ai.get_mut(&player) {
            ai.set_purchase_plan_with_repeat(purchases, repeat_from);
        }
    }

    pub(super) fn controller_turn(&mut self) {
        if self.replaying.is_some() || self.controllers.is_empty() || self.skirmish.is_some() {
            return;
        }
        enum Intent {
            Attack(TargetRef),
            Move([f64; 2]),
        }
        let mut orders: Vec<(Side, String, Vec<UnitId>, Intent)> = Vec::new();
        for (player, controller) in &self.controllers {
            if *controller != ControllerKind::ActiveAi {
                continue;
            }
            let Some(unit) = self
                .units
                .iter()
                .find(|unit| unit.owner.as_deref() == Some(player.as_str()) && unit.alive())
            else {
                continue;
            };
            if unit.side == Side::Neutral {
                continue;
            }
            let frame = self.observations[unit.side.index()].clone();
            let ids: Vec<_> = self
                .units
                .iter()
                .filter(|candidate| {
                    candidate.owner.as_deref() == Some(player.as_str()) && candidate.alive()
                })
                .map(|candidate| candidate.id)
                .collect();
            if ids.is_empty() {
                continue;
            }
            if let Some(target) = frame.identified.iter().max_by(|a, b| {
                let score = |candidate: &contract::observation::IdentifiedUnit| {
                    let dx = candidate.position[0] - unit.position.x;
                    let dy = candidate.position[1] - unit.position.y;
                    f64::from(candidate.cost) / (dx * dx + dy * dy + 1.0).sqrt()
                };
                score(a)
                    .partial_cmp(&score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.id.cmp(&a.id))
            }) {
                orders.push((
                    unit.side,
                    player.clone(),
                    ids,
                    Intent::Attack(TargetRef::Identified { id: target.id }),
                ));
            } else if let Some(objective) = frame.skirmish.as_ref().and_then(|view| {
                view.objectives
                    .iter()
                    .find(|objective| objective.owner.is_none())
            }) {
                orders.push((
                    unit.side,
                    player.clone(),
                    ids,
                    Intent::Move(objective.center),
                ));
            }
        }
        for (side, player, units, intent) in orders {
            let seq = self.next_seq[side.index()];
            let order = match intent {
                Intent::Attack(target) => Order::Attack { units, target },
                Intent::Move(goal) => Order::Move {
                    units,
                    gesture: 0,
                    goal,
                    route: contract::command::RoutePolicy::Shortest,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                },
            };
            let _ = self.issue(CommandEnvelope {
                side,
                player: Some(player),
                seq,
                order,
                queued: false,
            });
        }
    }

    /// Execute authored code from the side-scoped observation produced this
    /// tick. Replay consumes the accepted command log and never reruns code.
    pub(super) fn run_authored_scripts(&mut self) {
        if self.finished() {
            return;
        }
        if self.replaying.is_some() {
            if let Some(trace) = self.replay_script_trace.remove(&self.tick) {
                self.apply_script_trace(&trace);
            }
            return;
        }
        let mut outputs = Vec::new();
        for (index, (script, side, player)) in self.authored_scripts.iter().enumerate() {
            // Each script draws its own stream: the seed hashes the battle
            // seed, the tick, the side and the script's place in the scenario.
            let seed = [self.tick, side.index() as u64, index as u64]
                .into_iter()
                .fold(
                    contract::random::mix(self.seed ^ 0x5343_5249_5054_0000),
                    |h, v| contract::random::mix(h ^ v),
                );
            let input = ScriptInput::new(*side, &self.observations[side.index()], seed);
            let input = match player.as_deref() {
                Some(player) => input.with_player(player.to_owned()),
                None => input,
            };
            match script.run(&input) {
                Ok(output) => outputs.push(output),
                Err(error) => {
                    self.script_error = Some(error.to_string());
                    self.script_terminal_tick = Some(self.tick);
                    self.script_trace.push(ScriptTrace {
                        tick: self.tick,
                        events: Vec::new(),
                        victory: None,
                        error: self.script_error.clone(),
                    });
                    return;
                }
            }
        }
        let mut trace_victory = false;
        if let Some(script) = &self.victory_script {
            let input = ScriptInput::new(
                Side::Blue,
                &self.observations[Side::Blue.index()],
                self.seed ^ 0x5649_4354_4f52_5900 ^ self.tick,
            );
            match script.run(&input) {
                Ok(output) => {
                    if output.victory == Some(true) {
                        self.script_victory = Some(true);
                        self.script_terminal_tick = Some(self.tick);
                        trace_victory = true;
                    }
                }
                Err(error) => {
                    self.script_error = Some(error.to_string());
                    self.script_terminal_tick = Some(self.tick);
                    self.script_trace.push(ScriptTrace {
                        tick: self.tick,
                        events: Vec::new(),
                        victory: None,
                        error: self.script_error.clone(),
                    });
                    return;
                }
            }
        }
        let mut trace_events = Vec::new();
        for output in outputs {
            trace_events.extend(output.events.iter().cloned());
            self.script_events.extend(output.events);
            if output.victory == Some(true) {
                self.script_victory = Some(true);
                trace_victory = true;
            }
            for command in output.commands {
                let seq = self.next_seq[command.side.index()];
                let _ = self.issue(CommandEnvelope {
                    side: command.side,
                    player: command.player,
                    seq,
                    order: command.order,
                    queued: command.queued,
                });
            }
        }
        if !trace_events.is_empty() || trace_victory {
            self.script_trace.push(ScriptTrace {
                tick: self.tick,
                events: trace_events,
                victory: trace_victory.then_some(true),
                error: None,
            });
        }
    }

    pub(super) fn apply_script_trace(&mut self, trace: &ScriptTrace) {
        if let Some(error) = &trace.error {
            self.script_error = Some(error.clone());
            self.script_terminal_tick = Some(trace.tick);
        }
        self.script_events.extend(trace.events.clone());
        if trace.victory == Some(true) {
            self.script_victory = Some(true);
            self.script_terminal_tick = Some(trace.tick);
        }
    }

    /// The opponent reads its side's fresh observation and commands through
    /// the same path as a player; its commands are recorded, so a replay
    /// replays them and never reruns it. Its memory is outside the digest:
    /// continuing live play from a replay or a snapshot would need it.
    pub(super) fn opponent_turn(&mut self) {
        if self.replaying.is_some() {
            return;
        }
        let mut commands = Vec::new();
        if let Some((op, defender)) = self.opponent.as_mut() {
            let side = op.side;
            commands.extend(
                defender
                    .decide(op, &self.observations[side.index()], &self.rules)
                    .into_iter()
                    .map(|order| (side, order)),
            );
        }
        for (side, order) in commands {
            let seq = self.next_seq[side.index()];
            let _ = self.issue(CommandEnvelope {
                side,
                player: None,
                seq,
                order,
                queued: false,
            });
        }
    }
}
