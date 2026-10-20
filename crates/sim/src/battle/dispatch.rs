//! Skirmish reinforcement arrival: when each purchase leaves the queue and
//! where it enters the field.

use super::*;

impl Battle {
    pub(super) fn dispatch_reinforcements(&mut self) {
        use crate::encounter::legality::{apart, footprint, stands_on};
        let Some(skirmish) = &mut self.skirmish else {
            return;
        };
        if skirmish.phase != contract::skirmish::Phase::Active {
            return;
        }
        let hz = f64::from(self.rules.tick_hz);
        for player_index in 0..skirmish.players.len() {
            let side = skirmish.players[player_index].side;
            let index = side.index();
            let centre = crate::entries::centre(&skirmish.setup.sites, side);
            let centre_site = crate::entries::site(&skirmish.setup.sites, centre);
            let entry_site = v2(centre_site.center[0], centre_site.center[1]);
            let edge = crate::air::edge::back_edge(side, &skirmish.setup.sites, &self.world);
            // A bought aircraft's wait off the map is its own (D38, D41): its
            // clock starts with combat, and it enters when the clock runs out,
            // whatever the queue holds. Until then its approach is drawn.
            for reservation in &mut skirmish.players[player_index].reservations {
                let t = self.rules.catalog.get(reservation.data.kind);
                if reservation.data.arrival_tick.is_some() {
                    continue;
                }
                let Some(arrival) = t.mobility.arrival() else {
                    continue;
                };
                let mut rng = contract::random::Rng::new(contract::random::mix(
                    self.seed
                        ^ u64::from(reservation.data.id.0)
                        ^ (player_index as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15),
                ));
                let [min, max] = arrival.delay_s;
                let delay_s = min + (max - min) * rng.unit();
                reservation.data.arrival_tick = Some(self.tick + (delay_s * hz).round() as u64);
            }
            let mut arrived = Vec::new();
            let mut i = 0;
            while i < skirmish.players[player_index].reservations.len() {
                let reservation = &skirmish.players[player_index].reservations[i];
                match reservation.data.arrival_tick {
                    Some(due) if due <= self.tick => {
                        arrived.push(
                            skirmish.players[player_index]
                                .reservations
                                .remove(i)
                                .unwrap(),
                        );
                        continue;
                    }
                    Some(due) => {
                        let t = self.rules.catalog.get(reservation.data.kind);
                        let units::Motion::Air(flight) = units::motion(t, &self.rules) else {
                            unreachable!("only an aircraft waits off the map")
                        };
                        let destination = v2(
                            reservation.data.destination[0],
                            reservation.data.destination[1],
                        );
                        let (point, heading) =
                            crate::air::edge::arrival(&edge, entry_site, destination);
                        let z = self.world.height_at(point.x, point.y).unwrap_or(0.0)
                            + flight.cruise_agl_m;
                        self.air_transits[index].push(AirTransit {
                            kind: reservation.data.kind,
                            point: [point.x, point.y, z],
                            heading,
                            speed_mps: flight.cruise_mps,
                            edge_tick: due,
                            dir: AirTransitDirection::In,
                        });
                    }
                    None => {}
                }
                i += 1;
            }
            // Each entry fields one unit per dispatch interval, in queue order,
            // among the purchases that don't wait off the map. A purchase that
            // can't leave this tick holds only its own entry: the search goes
            // on through the player's purchases for the others (L9).
            let mut held = vec![false; skirmish.setup.sites.entries.len()];
            while let Some(next) =
                skirmish.players[player_index]
                    .reservations
                    .iter()
                    .position(|r| {
                        let entry = r.data.entry;
                        r.data.arrival_tick.is_none()
                            && !held[entry]
                            && self.tick >= skirmish.next_dispatch[entry]
                            && (skirmish.dispatch_turn[entry] == player_index
                                || !skirmish
                                    .entry_turn_has_purchase(crate::entries::EntryIndex(entry)))
                    })
            {
                let entry_index = crate::entries::EntryIndex(
                    skirmish.players[player_index].reservations[next].data.entry,
                );
                held[entry_index.0] = true;
                let selected = crate::entries::site(&skirmish.setup.sites, entry_index);
                let selected_point = v2(selected.center[0], selected.center[1]);
                let selected_yaw = selected.yaw;
                let reservation = &mut skirmish.players[player_index].reservations[next];
                let t = self.rules.catalog.get(reservation.data.kind);
                // An aircraft flies in at cruise height over whatever
                // stands there; a ground unit needs its entry site clear.
                let clear = match units::ground_mobility(t, &self.rules) {
                    None => true,
                    Some(mobility) => {
                        let at = selected_point;
                        let candidate = footprint(t, &self.rules, at, selected_yaw);
                        stands_on(
                            &self.world,
                            self.sides[index].grid(&self.world, self.authored_props),
                            &self.rules,
                            t,
                            &mobility,
                            at,
                            selected_yaw,
                        )
                        .is_ok()
                            && self
                                .units
                                .iter()
                                .filter(|u| u.on_field() && !u.airborne())
                                .all(|u| {
                                    apart(
                                        &candidate,
                                        &footprint(
                                            self.rules.catalog.get(u.kind),
                                            &self.rules,
                                            u.position.xy(),
                                            u.yaw,
                                        ),
                                        0.0,
                                    )
                                })
                    }
                };
                reservation.data.blocked = !clear;
                if clear {
                    arrived.push(
                        skirmish.players[player_index]
                            .reservations
                            .remove(next)
                            .unwrap(),
                    );
                    skirmish.next_dispatch[entry_index.0] = self.tick
                        + u64::from(skirmish.setup.rules.dispatch_interval_s)
                            * u64::from(self.rules.tick_hz);
                }
                skirmish.dispatch_turn[entry_index.0] =
                    skirmish.next_entry_player(entry_index, player_index);
            }
            for reservation in arrived {
                let t = self.rules.catalog.get(reservation.data.kind);
                let destination = v2(
                    reservation.data.destination[0],
                    reservation.data.destination[1],
                );
                let fixed_wing = matches!(
                    units::motion(t, &self.rules),
                    units::Motion::Air(f) if f.model == contract::catalog::FlightModel::FixedWing
                );
                let selected_entry = if fixed_wing {
                    centre
                } else {
                    crate::entries::EntryIndex(reservation.data.entry)
                };
                let selected_site = crate::entries::site(&skirmish.setup.sites, selected_entry);
                // Every aircraft enters already flying, at its back-edge point
                // nearest the entry site (D39); a ground unit at the site.
                let (at, yaw) = if matches!(units::motion(t, &self.rules), units::Motion::Air(_)) {
                    let selected_point = v2(selected_site.center[0], selected_site.center[1]);
                    crate::air::edge::arrival(&edge, selected_point, destination)
                } else {
                    (
                        v2(selected_site.center[0], selected_site.center[1]),
                        selected_site.yaw,
                    )
                };
                let controller = skirmish.players[player_index].controller.clone();
                let setup = contract::scenario::UnitSetup {
                    side,
                    owner: Some(skirmish.players[player_index].id.clone()),
                    kind: self.rules.catalog.id(reservation.data.kind).to_string(),
                    position: [at.x, at.y],
                    yaw,
                    engagement: None,
                    condition: None,
                    stock: None,
                };
                let mut unit = spawn_unit(
                    self.units.len(),
                    &setup,
                    &self.world,
                    &self.rules,
                    &self.arsenal,
                    &mut self.last_soldier,
                    self.seed,
                );
                if let (units::Motion::Air(flight), Some(air)) = (unit.motion, unit.air.as_mut()) {
                    let speed = flight.cruise_mps;
                    air.velocity = v3(libm::cos(yaw) * speed, libm::sin(yaw) * speed, 0.0);
                }
                super::controllers::put_under(&mut unit, &controller);
                unit.settle();
                unit.orders.push_back(UnitOrder::Move(MoveOrder {
                    destination,
                    policy: RoutePolicy::Fastest,
                    gesture: 0,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                    short: None,
                }));
                skirmish.entered.insert(
                    unit.id,
                    crate::skirmish::EnteredUnit {
                        price: reservation.price,
                        entered_tick: self.tick,
                        player: player_index,
                    },
                );
                skirmish.record_spawn(
                    player_index,
                    unit.id,
                    unit.kind,
                    reservation.price,
                    self.tick,
                );
                self.units.push(unit);
            }
        }
    }
}
