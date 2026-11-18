//! Command admission and placement from a side's knowledge.

use super::*;

impl Battle {
    pub(super) fn player_faction(&self, side: Side) -> Option<contract::catalog::Faction> {
        self.player_ids.iter().find_map(|id| {
            self.skirmish.as_ref().and_then(|s| {
                s.players
                    .iter()
                    .find(|p| p.id == *id && p.side == side)
                    .map(|p| p.faction)
            })
        })
    }

    /// Validate and schedule a client's command for the next step. Every
    /// command whose sequence number is next for its side is recorded for
    /// replay, including ones rejected for their content, so a replay
    /// reproduces the same acks.
    pub fn accept(&mut self, command: CommandEnvelope) -> CommandAck {
        self.receive(command, Issuer::Client)
    }

    /// Schedule an order the battle's own AI, scripts or opponents give.
    pub(super) fn issue(&mut self, command: CommandEnvelope) -> CommandAck {
        self.receive(command, Issuer::Battle)
    }

    pub(super) fn receive(&mut self, command: CommandEnvelope, issuer: Issuer) -> CommandAck {
        let applied_tick = self.tick + 1;
        let ack = |error, placement, building, refused| CommandAck {
            seq: command.seq,
            applied_tick,
            error,
            refused,
            placement,
            building,
        };
        if self.replaying.is_some() {
            return ack(Some(OrderError::ReplayInProgress), None, None, Vec::new());
        }
        // A player under AI takes orders only from its controller. The refusal
        // comes before the command is recorded: the replay log does not say
        // who issued a command, so a replay could not tell this one from the
        // AI's own.
        if issuer == Issuer::Client && self.ai_controlled(command.player.as_deref()) {
            return ack(Some(OrderError::AiControlled), None, None, Vec::new());
        }
        self.admit(command.clone(), applied_tick).map_or_else(
            |e| ack(Some(e), None, None, Vec::new()),
            |(placement, building, refused)| {
                let error = (placement
                    .as_ref()
                    .is_some_and(|p| p.destinations.iter().all(|d| !d.placed))
                    || (building.as_ref().is_some_and(|b| b.entries.is_empty())
                        && refused.is_empty()))
                .then_some(OrderError::NoValidDestination);
                ack(error, placement, building, refused)
            },
        )
    }

    pub(super) fn spend_order(&mut self, side: usize, player: Option<usize>, tick: Tick) -> bool {
        let per_s = u64::from(self.rules.commands.orders_per_s);
        let hz = u64::from(self.rules.tick_hz);
        let bucket = player.map_or_else(
            || &mut self.order_bucket[side],
            |i| &mut self.player_order_bucket[i],
        );
        let (left, at) = bucket;
        *left = (*left + tick.saturating_sub(*at) * per_s).min(per_s * hz);
        *at = (*at).max(tick);
        if *left < hz {
            return false;
        }
        *left -= hz;
        true
    }

    pub(super) fn admit(
        &mut self,
        command: CommandEnvelope,
        applied_tick: Tick,
    ) -> Result<Admission, OrderError> {
        let side = command.side.index();
        let player = self.player_index(command.player.as_deref());
        let expected = self.expected_seq(command.side, player);
        if command.seq != expected {
            return Err(OrderError::OutOfSequence { expected });
        }
        if let Some(i) = player {
            self.player_next_seq[i] += 1;
        } else {
            self.next_seq[side] += 1;
        }
        self.accepted.push((applied_tick, command.clone()));
        if !self.spend_order(side, player, applied_tick) {
            return Err(OrderError::RateLimited);
        }
        let prepared = self.prepare(command, applied_tick.saturating_sub(1))?;
        let placement = prepared.placement.clone();
        let building = prepared.building.clone();
        let refused = prepared.refused.clone();
        self.pending.push(prepared);
        Ok((placement, building, refused))
    }

    /// A side-scoped target reference as the sim's own target, if the side
    /// holds it right now and can aim at it (never an air contact, D22).
    pub(super) fn resolve_target(&self, side: Side, target: &TargetRef) -> Option<Target> {
        let knowledge = &self.knowledge[side.index()];
        match *target {
            TargetRef::Identified { id } => knowledge.unit_for(id).map(Target::Unit),
            TargetRef::Contact { id } => {
                knowledge.ground_contact(id).map(|c| Target::Contact(c.id))
            }
            // The aim point sits on the public terrain, whatever height the client sent.
            TargetRef::Ground { point } => self
                .world
                .height_at(point[0], point[1])
                .map(|z| Target::Ground(v3(point[0], point[1], z))),
        }
    }

    pub(super) fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        if self.finished() {
            return Err(OrderError::MatchFinished);
        }
        if command.side == Side::Neutral {
            return Err(OrderError::PassiveSide);
        }
        let skirmish_player = if let Some(skirmish) = &self.skirmish {
            let id = command.player.as_deref().ok_or(OrderError::MissingPlayer)?;
            let index = skirmish.player_index(id).ok_or(OrderError::MissingPlayer)?;
            if skirmish.players[index].side != command.side {
                return Err(OrderError::WrongFaction);
            }
            Some(index)
        } else {
            None
        };
        let units = match &command.order {
            Order::Refund { units } => {
                let Some(skirmish) = &self.skirmish else {
                    return Err(OrderError::NotSkirmish);
                };
                self.validate_command_units(command, units)?;
                if units.iter().any(|id| !skirmish.entered.contains_key(id)) {
                    return Err(OrderError::NotSkirmish);
                }
                units
            }
            Order::Ready | Order::ConfirmPurchase { .. } => {
                return skirmish_player.map(|_| ()).ok_or(OrderError::NotSkirmish);
            }
            Order::CancelPending { purchase } => {
                let index = skirmish_player.ok_or(OrderError::NotSkirmish)?;
                // Players number their purchases separately: a number names
                // another player's only when the canceller has none by it.
                let players = &self.skirmish.as_ref().unwrap().players;
                let holds = |p: &crate::skirmish::PlayerState| {
                    p.reservations.iter().any(|r| r.data.id == *purchase)
                };
                if !holds(&players[index])
                    && players
                        .iter()
                        .enumerate()
                        .any(|(other, p)| other != index && holds(p))
                {
                    return Err(OrderError::WrongFaction);
                }
                return Ok(());
            }
            Order::Move { units, goal, .. } | Order::AttackMove { units, goal, .. } => {
                if self.world.height_at(goal[0], goal[1]).is_none() {
                    return Err(OrderError::OutOfBounds);
                }
                units
            }
            Order::Attack { units, target } => {
                if let TargetRef::Contact { id } = *target {
                    let held = self.knowledge[command.side.index()].contact(id);
                    if held.is_some_and(|c| c.layer != AltitudeLayer::Ground) {
                        return Err(OrderError::AirContact);
                    }
                }
                if self.resolve_target(command.side, target).is_none() {
                    return Err(OrderError::UnknownTarget);
                }
                units
            }
            Order::Board { units, carrier, .. } => {
                self.validate_command_units(command, units)?;
                if units
                    .iter()
                    .any(|id| self.units[id.0 as usize].hull.is_some())
                {
                    return Err(OrderError::NotInfantry { unit: units[0] });
                }
                let Some(c) = self.units.get(carrier.0 as usize) else {
                    return Err(OrderError::NotACarrier);
                };
                if c.side != command.side
                    || c.unit_type(&self.rules).capabilities.carriage.is_none()
                {
                    return Err(OrderError::NotACarrier);
                }
                if !c.alive() {
                    return Err(OrderError::Destroyed { unit: *carrier });
                }
                // A squad rides only its own player's carrier, as a carrier
                // collects only its own player's squads.
                self.validate_player(command, *carrier)?;
                units
            }
            Order::Collect { units, squads, .. } => {
                self.validate_command_units(command, units)?;
                for &squad in squads {
                    self.validate_command_units(command, &[squad])?;
                    if self.units[squad.0 as usize].hull.is_some() {
                        return Err(OrderError::NotInfantry { unit: squad });
                    }
                }
                if units.iter().any(|id| {
                    self.units[id.0 as usize]
                        .unit_type(&self.rules)
                        .capabilities
                        .carriage
                        .is_none()
                }) {
                    return Err(OrderError::NotACarrier);
                }
                units
            }
            Order::Unload {
                units, goal, into, ..
            } => {
                self.validate_command_units(command, units)?;
                // Squads go into another carrier only as they would board it.
                if let Some(contract::command::UnloadInto::Carrier { carrier }) = into {
                    let c = (self.units.get(carrier.0 as usize))
                        .filter(|c| c.side == command.side && !units.contains(carrier))
                        .filter(|c| c.unit_type(&self.rules).capabilities.carriage.is_some())
                        .ok_or(OrderError::NotACarrier)?;
                    if !c.alive() {
                        return Err(OrderError::Destroyed { unit: *carrier });
                    }
                    self.validate_player(command, *carrier)?;
                }
                if let Some(goal) = goal {
                    if self.world.height_at(goal[0], goal[1]).is_none() {
                        return Err(OrderError::OutOfBounds);
                    }
                }
                if units.iter().any(|id| {
                    self.units[id.0 as usize]
                        .unit_type(&self.rules)
                        .capabilities
                        .carriage
                        .is_none()
                }) {
                    return Err(OrderError::NotACarrier);
                }
                units
            }
            Order::Stop { units }
            | Order::OccupyBuilding { units, .. }
            | Order::SetAfterburner { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::PopSmoke { units }
            | Order::ExitBuilding { units } => units,
            // Gesture tokens are scoped by side at application; nothing else to check.
            Order::UpgradeMove { .. } => return Ok(()),
        };
        self.validate_command_units(command, units)
    }

    pub(super) fn validate_command_units(
        &self,
        command: &CommandEnvelope,
        units: &[UnitId],
    ) -> Result<(), OrderError> {
        self.validate_units(command.side, units)?;
        for &unit in units {
            self.validate_player(command, unit)?;
        }
        Ok(())
    }

    /// A unit a player owns takes orders only from that player.
    pub(super) fn validate_player(
        &self,
        command: &CommandEnvelope,
        unit: UnitId,
    ) -> Result<(), OrderError> {
        let Some(owner) = self.units[unit.0 as usize].owner.as_deref() else {
            return Ok(());
        };
        match command.player.as_deref() {
            None => Err(OrderError::MissingPlayer),
            Some(player) if player == owner => Ok(()),
            Some(_) => Err(OrderError::WrongPlayer { unit }),
        }
    }

    /// Every unit named exists, is the side's own and is alive.
    pub(super) fn validate_units(&self, side: Side, units: &[UnitId]) -> Result<(), OrderError> {
        if units.is_empty() {
            return Err(OrderError::NoUnits);
        }
        for &unit in units {
            match self.units.get(unit.0 as usize) {
                None => return Err(OrderError::UnknownUnit { unit }),
                Some(u) if u.side != side => return Err(OrderError::NotOwnUnit { unit }),
                Some(u) if !u.alive() => return Err(OrderError::Destroyed { unit }),
                Some(u) if !u.on_field() => return Err(OrderError::Aboard { unit }),
                Some(_) => {}
            }
        }
        Ok(())
    }

    pub(super) fn prepare(
        &mut self,
        mut command: CommandEnvelope,
        planning_tick: Tick,
    ) -> Result<PreparedCommand, OrderError> {
        self.validate(&command)?;
        let player = self.player_index(command.player.as_deref());
        if matches!(command.order, Order::Ready) {
            self.skirmish.as_mut().unwrap().players[player.expect("validated skirmish player")]
                .ready = true;
        }
        if let Order::CancelPending { purchase } = command.order {
            self.skirmish
                .as_mut()
                .unwrap()
                .cancel(player.expect("validated skirmish player"), purchase)?;
        }
        if let Order::ConfirmPurchase { .. } = &command.order {
            let player = player.expect("validated skirmish player");
            self.reserve_purchase(&command, player, planning_tick)?;
        }
        let (building, building_refused) = if let Order::OccupyBuilding {
            units, building, ..
        } = &command.order
        {
            let (plan, refused) = self.plan_building(
                command.side,
                &BuildingPreviewRequest {
                    units: units.clone(),
                    building: *building,
                    queued: command.queued,
                },
            )?;
            (Some(plan), refused)
        } else {
            (None, Vec::new())
        };
        if let Order::Unload {
            units,
            goal,
            into: Some(into),
            ..
        } = &mut command.order
        {
            let beside = crate::carriage::set_down_beside(
                &self.world,
                &self.sides[command.side.index()],
                self.authored_props,
                &self.units,
                units,
                *into,
                *goal,
                &self.rules,
            )?;
            *goal = Some(beside);
        }
        let mut stops = BTreeMap::new();
        let placement = match &command.order {
            Order::Refund { units } => Some(MovePlacement {
                gesture: 0,
                destinations: (units.iter())
                    .map(|&unit| {
                        let (goal, facing) = self.way_home(command.side, unit);
                        contract::command::MoveDestination {
                            unit,
                            goal,
                            placed: true,
                            facing,
                            spots: vec![],
                        }
                    })
                    .collect(),
            }),
            Order::Move {
                units,
                gesture,
                goal,
                facing,
                direction,
                route,
                ..
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        facing: *facing,
                        direction: *direction,
                        route: *route,
                        queued: command.queued,
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            Order::AttackMove {
                units,
                gesture,
                goal,
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        queued: command.queued,
                        ..Default::default()
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            Order::Unload {
                units,
                gesture,
                goal: Some(goal),
                route,
                ..
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        route: *route,
                        queued: command.queued,
                        ..Default::default()
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            _ => None,
        };
        let refused = self.refuse_at_command(&mut command, building_refused)?;
        Ok(PreparedCommand {
            command,
            placement,
            building,
            stops,
            refused,
        })
    }

    /// Plan a building order on this side's own knowledge, without
    /// accepting it. A queued order sets off from where the side's orders
    /// given this tick leave each squad.
    pub fn preview_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
    ) -> Result<BuildingPlacement, OrderError> {
        self.plan_building(side, request).map(|(plan, _)| plan)
    }

    /// [`Self::preview_building`], with the squads too big for the whole
    /// building refused one by one.
    pub(super) fn plan_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
    ) -> Result<(BuildingPlacement, Vec<contract::command::Refusal>), OrderError> {
        self.validate_units(side, &request.units)?;
        garrison::occupy(
            &self.world,
            &self.sides[side.index()],
            self.authored_props,
            &self.move_source(side),
            request,
            &self.rules,
        )
    }

    pub(super) fn move_source(&self, side: Side) -> Vec<Unit> {
        let mut source = self.units.clone();
        for unit in &mut source {
            if unit.side != side {
                let track = self.knowledge[side.index()].track(unit.id);
                for (k, soldier) in unit.members.iter_mut().enumerate() {
                    let seen =
                        track.is_some_and(|t| t.last_seen == self.tick && t.members.contains(&k));
                    soldier.hp = if seen { 1.0 } else { 0.0 };
                    if !seen {
                        soldier.position = track.map_or(Default::default(), |t| t.position);
                    }
                    soldier.path.clear();
                    soldier.spot = None;
                    soldier.post = None;
                    soldier.lean = None;
                    soldier.cover = None;
                }
                unit.cover = Default::default();
                unit.attackers.clear();
                unit.suppression = 0.0;
                for mount in &mut unit.mounts {
                    mount.stop();
                }
                if let Some(track) = track {
                    unit.position = track.position;
                    unit.yaw = track.yaw;
                    unit.hp = 1.0;
                } else {
                    unit.hp = 0.0;
                }
            }
        }
        // Admission sees earlier commands exactly as execution will apply them.
        for pending in &self.pending {
            if pending.command.side == side {
                self.apply_units(pending.clone(), &mut source);
            }
        }
        source
    }
}
