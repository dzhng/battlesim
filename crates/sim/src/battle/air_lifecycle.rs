//! Dogfight completion, fuel withdrawal and departure settlement.

use super::*;

impl Battle {
    /// End each jet's dogfight that is over (`air::dogfight::broken`): past
    /// the break range and no longer closing, unless it is coming round.
    pub(super) fn break_dogfights(&mut self) {
        for i in 0..self.units.len() {
            let Some(target) = self.units[i]
                .air
                .as_ref()
                .and_then(|air| air.dogfight_target)
            else {
                continue;
            };
            let (unit, other) = (&self.units[i], &self.units[target.0 as usize]);
            if crate::air::dogfight::broken(unit, other, &self.rules.air.fixed_wing) {
                crate::air::dogfight::clear(&mut self.units[i]);
            }
        }
    }

    /// A finite-endurance aircraft turns for home while it still has enough
    /// fuel to reach it: a jet flies for the nearest point on its side's back
    /// edge, a helicopter for its friendly entry site. The ordinary
    /// withdrawal route then retires it there and settles its weighted
    /// refund, so a combat order cannot strand an airframe after its tank is
    /// effectively empty.
    pub(super) fn trigger_fuel_withdrawals(&mut self) {
        let Some(sites) = self.skirmish.as_ref().map(|s| s.setup.sites.clone()) else {
            return;
        };
        for index in 0..self.units.len() {
            let unit = &self.units[index];
            if !unit.alive() || unit.retired || unit.withdrawing {
                continue;
            }
            let Some(air_state) = unit.air.as_ref() else {
                continue;
            };
            let Some(capability) = unit.unit_type(&self.rules).capabilities.air.as_ref() else {
                continue;
            };
            let entry = if unit.fixed_wing() {
                let edge = crate::air::edge::back_edge(unit.side, &sites, &self.world);
                crate::air::edge::entry(&edge, unit.position.xy()).0
            } else {
                let selected =
                    self.select_entry(unit.side, unit.kind, [unit.position.x, unit.position.y]);
                let entry = crate::entries::site(&sites, selected);
                v2(entry.center[0], entry.center[1])
            };
            let units::Motion::Air(flight) = unit.motion else {
                continue;
            };
            let speed = flight.cruise_mps
                * if air_state.afterburner {
                    capability.afterburner.map_or(1.0, |a| a.speed_factor)
                } else {
                    1.0
                };
            let return_s = (unit.position.xy() - entry).length() / speed.max(1.0) + 2.0;
            if air_state.fuel_s > return_s {
                continue;
            }
            let unit = &mut self.units[index];
            if let Some(air) = unit.air.as_mut() {
                air.afterburner = false;
            }
            crate::air::break_off(unit);
            unit.enqueue(
                UnitOrder::Move(MoveOrder {
                    destination: entry,
                    policy: RoutePolicy::Fastest,
                    gesture: 0,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                    short: None,
                }),
                false,
            );
            unit.withdrawing = true;
        }
    }

    pub(super) fn retire_withdrawals(&mut self) {
        let Some(mut skirmish) = self.skirmish.take() else {
            return;
        };
        let mut retiring = Vec::new();
        for (index, unit) in self.units.iter().enumerate() {
            if !unit.alive() || !unit.withdrawing {
                continue;
            }
            if unit.fixed_wing() {
                let edge =
                    crate::air::edge::back_edge(unit.side, &skirmish.setup.sites, &self.world);
                let dt = 1.0 / f64::from(self.rules.tick_hz);
                let previous = unit.position.xy() - unit.flight_velocity().xy() * dt;
                if !crate::air::edge::crossed(&edge, previous, unit.position.xy()) {
                    continue;
                }
                let air = unit.air.as_ref().expect("an aircraft's flight state");
                self.air_transits[unit.side.index()].push(AirTransit {
                    kind: unit.kind,
                    point: [unit.position.x, unit.position.y, unit.position.z],
                    heading: unit.yaw,
                    speed_mps: air.velocity.xy().length(),
                    edge_tick: self.tick,
                    dir: AirTransitDirection::Out,
                });
            }
            // Proximity lets ground and rotor units leave before the exact edge point.
            if !unit.fixed_wing() {
                let half = unit.hull.map_or(2.0, |half| half.x.max(half.y));
                if !crate::entries::retire_at(
                    &skirmish.setup.sites,
                    unit.side,
                    [unit.position.x, unit.position.y],
                    half,
                ) {
                    continue;
                }
            }
            retiring.push(index);
        }
        let carriers: Vec<UnitId> = retiring
            .iter()
            .map(|&index| self.units[index].id)
            .filter(|&carrier| {
                self.units[carrier.0 as usize]
                    .unit_type(&self.rules)
                    .capabilities
                    .carriage
                    .is_some()
            })
            .collect();
        for carrier in carriers {
            retiring.extend(crate::carriage::passengers(&self.units, carrier));
        }
        retiring.sort_unstable();
        retiring.dedup();
        for index in retiring {
            let unit = &mut self.units[index];
            let Some(entered) = skirmish.entered.get(&unit.id) else {
                continue;
            };
            let (health, ammo) = crate::withdrawal::condition(unit, &self.rules, &self.arsenal);
            let fuel = crate::withdrawal::fuel(unit, &self.rules);
            let age = self.tick.saturating_sub(entered.entered_tick) as f64
                / f64::from(self.rules.tick_hz);
            let refund = unit
                .unit_type(&self.rules)
                .capabilities
                .air
                .as_ref()
                .map(|air| (air.refund.health, air.refund.ammunition, air.refund.fuel))
                .map_or_else(
                    || crate::withdrawal::fraction(health, ammo, age),
                    |weights| {
                        crate::withdrawal::weighted_fraction(health, ammo, fuel, weights, age)
                    },
                );
            let credits = (f64::from(entered.price) * refund * crate::skirmish::CREDIT_SCALE as f64)
                .round() as u64;
            let player = entered.player;
            skirmish.players[player].wallet += credits;
            skirmish.record_refund(
                player,
                unit.id,
                unit.kind,
                credits as f64 / crate::skirmish::CREDIT_SCALE as f64,
                self.tick,
            );
            unit.retired = true;
            unit.withdrawing = false;
            unit.orders.clear();
            unit.route = None;
            unit.hold = None;
        }
        self.skirmish = Some(skirmish);
    }
}
