//! Observation-bound player-match policy; all outcomes remain ordinary commands.
use contract::catalog::{AltitudeLayer, Category, Faction, TypeIndex};
use contract::command::Order;
use contract::encounter::EntrySite;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::ObservationFrame;
use contract::scenario::Rules;
use contract::skirmish::Phase;
use std::collections::BTreeMap;

/// A side-scoped skirmish policy. Implementations emit the same ordinary
/// orders as a human controller; the battle host owns admission and replay.
pub trait SkirmishPolicy {
    fn decide(
        &mut self,
        side: Side,
        faction: Faction,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Doctrine {
    #[default]
    BalancedOpening,
    ReconScreen,
    InfantryPressure,
    TankSwarm,
    AirDenial,
    EmergencyDefense,
    Consolidate,
}

#[derive(Clone, Debug)]
pub struct BalancedDoctrine {
    next_decision: Tick,
    assigned: BTreeMap<UnitId, String>,
    reinforcement: usize,
    doctrine: Doctrine,
    started: bool,
    doctrine_since: Tick,
    pivot_count: u32,
    last_own_count: Option<usize>,
    last_air_evidence: Option<Tick>,
    response_delay_ticks: Option<Tick>,
    random: contract::random::Rng,
}

impl Default for BalancedDoctrine {
    fn default() -> Self {
        Self {
            next_decision: 0,
            assigned: BTreeMap::new(),
            reinforcement: 0,
            doctrine: Doctrine::BalancedOpening,
            started: false,
            doctrine_since: 0,
            pivot_count: 0,
            last_own_count: None,
            last_air_evidence: None,
            response_delay_ticks: None,
            random: contract::random::Rng::new(0),
        }
    }
}

impl BalancedDoctrine {
    pub fn new(seed: u64) -> Self {
        Self {
            reinforcement: (contract::random::mix(seed) as usize) % 5,
            random: contract::random::Rng::new(contract::random::mix(seed ^ 0x534b_4952_4d49_5348)),
            response_delay_ticks: None,
            ..Self::default()
        }
    }

    pub fn doctrine(&self) -> Doctrine {
        self.doctrine
    }

    pub fn pivot_count(&self) -> u32 {
        self.pivot_count
    }

    fn update_doctrine(&mut self, frame: &ObservationFrame, rules: &Rules, draw_unit: f64) {
        if !self.started {
            self.started = true;
            self.doctrine_since = frame.tick;
        }
        if self.response_delay_ticks.is_none() {
            // One fixed draw selects a bounded reaction delay. The delay is a
            // personality trait, not a license to ignore a threat forever.
            self.response_delay_ticks =
                Some(1 + (draw_unit * 3.0 * f64::from(rules.tick_hz)) as Tick);
        }
        let low_air = frame_has_layer_threat(frame, rules, AltitudeLayer::LowAir);
        let own_count = frame.own.len();
        let lost_unit = self
            .last_own_count
            .is_some_and(|previous| own_count < previous);
        self.last_own_count = Some(own_count);
        if low_air {
            self.last_air_evidence.get_or_insert(frame.tick);
        } else if self
            .last_air_evidence
            .is_some_and(|seen| frame.tick.saturating_sub(seen) > 30 * u64::from(rules.tick_hz))
        {
            self.last_air_evidence = None;
        }
        let min_dwell = 20 * u64::from(rules.tick_hz);
        let max_dwell = 180 * u64::from(rules.tick_hz);
        let dwell = frame.tick.saturating_sub(self.doctrine_since);
        let delayed_air = self.last_air_evidence.is_some_and(|seen| {
            frame.tick.saturating_sub(seen)
                >= self
                    .response_delay_ticks
                    .expect("reaction delay initialized")
        });
        let next = if lost_unit && dwell >= min_dwell {
            Some(Doctrine::EmergencyDefense)
        } else if delayed_air && dwell >= min_dwell {
            Some(Doctrine::AirDenial)
        } else if dwell >= max_dwell {
            Some(match self.doctrine {
                Doctrine::BalancedOpening => Doctrine::ReconScreen,
                Doctrine::ReconScreen => Doctrine::InfantryPressure,
                Doctrine::InfantryPressure => Doctrine::TankSwarm,
                Doctrine::TankSwarm => Doctrine::Consolidate,
                Doctrine::AirDenial => Doctrine::Consolidate,
                Doctrine::Consolidate => Doctrine::ReconScreen,
                Doctrine::EmergencyDefense => Doctrine::ReconScreen,
            })
        } else {
            None
        };
        if let Some(next) = next.filter(|next| *next != self.doctrine) {
            self.doctrine = next;
            self.doctrine_since = frame.tick;
            self.pivot_count = self.pivot_count.saturating_add(1);
        }
    }

    pub fn decide(
        &mut self,
        side: Side,
        faction: Faction,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        mut route_time: impl FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        self.decide_impl(side, faction, entry, frame, rules, &mut route_time)
    }

    fn decide_impl(
        &mut self,
        side: Side,
        faction: Faction,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        let Some(view) = &frame.skirmish else {
            return vec![];
        };
        if view.phase == Phase::Finished || frame.tick < self.next_decision {
            return vec![];
        }
        let draw = self.random.next_u64();
        let draw_unit = (draw >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        self.next_decision = frame.tick.saturating_add(5 * u64::from(rules.tick_hz));
        self.update_doctrine(frame, rules, draw_unit);
        self.assigned
            .retain(|id, _| frame.own.iter().any(|u| u.id == *id));
        let eligible: Vec<_> = rules
            .catalog
            .indices()
            .filter(|&kind| {
                let unit = rules.catalog.get(kind);
                unit.roster
                    .as_ref()
                    .is_some_and(|r| r.factions.contains(&faction))
            })
            .collect();
        let pick = |role: &str, category: Category, ordinal: usize| {
            let mut matching: Vec<_> = eligible
                .iter()
                .copied()
                .filter(|&k| rules.catalog.get(k).has_role(role))
                .collect();
            if matching.is_empty() {
                matching = eligible
                    .iter()
                    .copied()
                    .filter(|&k| {
                        let unit = rules.catalog.get(k);
                        unit.roster.as_ref().unwrap().category == category
                            && (category == Category::Sup || !rules.catalog.mounts(k).is_empty())
                    })
                    .collect();
            }
            matching.sort_by(|a, b| {
                rules
                    .catalog
                    .get(*a)
                    .cost
                    .cmp(&rules.catalog.get(*b).cost)
                    .then(rules.catalog.id(*a).cmp(rules.catalog.id(*b)))
            });
            matching
                .get(ordinal.min(matching.len().saturating_sub(1)))
                .copied()
        };
        let count = |kind: TypeIndex| {
            frame.own.iter().filter(|u| u.kind == kind).count()
                + view.pending.iter().filter(|p| p.kind == kind).count()
        };
        let low_air_demand = frame_has_layer_threat(frame, rules, AltitudeLayer::LowAir);
        let opening = match self.doctrine {
            Doctrine::TankSwarm => [
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 1),
                ("recon", Category::Rec, 0),
                ("at", Category::Inf, 0),
                ("infantry", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("logistics", Category::Sup, 0),
            ],
            Doctrine::AirDenial => [
                ("recon", Category::Rec, 0),
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 1),
                ("at", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("infantry", Category::Inf, 2),
                ("recon", Category::Rec, 0),
                ("logistics", Category::Sup, 0),
            ],
            _ => [
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 1),
                ("infantry", Category::Inf, 2),
                ("recon", Category::Rec, 0),
                ("at", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("logistics", Category::Sup, 0),
            ],
        };
        let desired: Vec<_> = opening
            .iter()
            .filter_map(|&(role, category, ordinal)| pick(role, category, ordinal))
            .collect();
        let mut orders = Vec::new();
        // Only units that can take an objective count toward its load: a
        // helicopter sent to fight over a flag never holds it (D20).
        let holds = |id: UnitId| {
            frame
                .own
                .iter()
                .find(|u| u.id == id)
                .is_some_and(|u| crate::objectives::can_hold(rules.catalog.get(u.kind)))
        };
        // The whole opening is bought in one decision, so the opponent is
        // ready at once and the player's own readiness starts the battle.
        let purchases: Vec<TypeIndex> = if view.phase == Phase::Preparation {
            let mut credits = view.credits;
            let mut slots = view.occupied_slots;
            let mut needed = BTreeMap::new();
            desired
                .iter()
                .copied()
                .filter(|&kind| {
                    let wanted = needed.entry(kind).or_insert(0);
                    *wanted += 1;
                    let cost = f64::from(rules.catalog.get(kind).cost);
                    let buy = count(kind) < *wanted && cost <= credits && slots < view.max_units;
                    if buy {
                        credits -= cost;
                        slots += 1;
                    }
                    buy
                })
                .collect()
        } else if view.occupied_slots < view.max_units {
            let purchase = {
                let truck = pick("logistics", Category::Sup, 0);
                if truck.is_some()
                    && !frame
                        .own
                        .iter()
                        .any(|u| rules.catalog.get(u.kind).has_role("logistics"))
                    && !view
                        .pending
                        .iter()
                        .any(|p| rules.catalog.get(p.kind).has_role("logistics"))
                {
                    truck.filter(|&k| f64::from(rules.catalog.get(k).cost) <= view.credits)
                } else {
                    // Helicopters reinforce, ordered like light vehicles (D35);
                    // a deck without one buys the next role in turn instead.
                    let roles = [
                        ("infantry", Category::Inf),
                        ("recon", Category::Rec),
                        ("at", Category::Inf),
                        ("light_vehicle", Category::Veh),
                        ("helicopter", Category::Hel),
                    ];
                    let selected = if low_air_demand {
                        eligible
                            .iter()
                            .copied()
                            .filter(|&kind| {
                                unit_can_target_layer(rules, kind, AltitudeLayer::LowAir)
                            })
                            .max_by(|a, b| {
                                candidate_score(rules, *a, draw_unit, Some(AltitudeLayer::LowAir))
                                    .total_cmp(&candidate_score(
                                        rules,
                                        *b,
                                        draw_unit,
                                        Some(AltitudeLayer::LowAir),
                                    ))
                                    .then_with(|| rules.catalog.id(*b).cmp(rules.catalog.id(*a)))
                            })
                            .filter(|&k| f64::from(rules.catalog.get(k).cost) <= view.credits)
                            .map(|k| (0, k))
                    } else {
                        (0..roles.len()).find_map(|offset| {
                            let index = (self.reinforcement + offset) % roles.len();
                            pick(roles[index].0, roles[index].1, 0)
                                .filter(|&k| f64::from(rules.catalog.get(k).cost) <= view.credits)
                                .map(|k| (index, k))
                        })
                    };
                    selected.map(|(index, k)| {
                        self.reinforcement = (index + 1) % roles.len();
                        k
                    })
                }
            };
            purchase.into_iter().collect()
        } else {
            vec![]
        };
        let mut bought: Vec<[f64; 2]> = Vec::new();
        for kind in purchases {
            let support = rules.catalog.get(kind).has_role("logistics");
            let destination = if support {
                entry.center
            } else {
                view.objectives
                    .iter()
                    .filter(|o| o.owner != Some(side))
                    .min_by(|a, b| {
                        let load = |o: &contract::skirmish::ObjectiveView| {
                            view.pending
                                .iter()
                                .map(|p| p.destination)
                                .chain(bought.iter().copied())
                                .filter(|d| *d == o.center)
                                .count()
                                + self
                                    .assigned
                                    .iter()
                                    .filter(|(u, id)| *id == &o.id && holds(**u))
                                    .count()
                        };
                        load(a)
                            .cmp(&load(b))
                            .then(
                                distance(entry.center, a.center)
                                    .total_cmp(&distance(entry.center, b.center)),
                            )
                            .then(a.id.cmp(&b.id))
                    })
                    .map_or(entry.center, |o| o.center)
            };
            bought.push(destination);
            orders.push(Order::ConfirmPurchase {
                variant: rules.catalog.id(kind).into(),
                destination,
            });
        }
        if view.phase == Phase::Active {
            let mut own: Vec<_> = frame.own.iter().collect();
            own.sort_by_key(|u| u.id);
            for unit in &own {
                if rules.catalog.get(unit.kind).has_role("logistics") {
                    continue;
                }
                let available: Vec<_> = view
                    .objectives
                    .iter()
                    .filter(|o| o.owner != Some(side))
                    .collect();
                if available.is_empty() {
                    continue;
                }
                if self.assigned.get(&unit.id).is_some_and(|id| {
                    available.iter().any(|o| {
                        &o.id == id
                            && (unit.goal.is_some()
                                || distance([unit.position[0], unit.position[1]], o.center)
                                    <= o.radius_m)
                    })
                }) {
                    continue;
                }
                let best = available
                    .iter()
                    .filter_map(|objective| {
                        route_time(unit.id, objective.center)
                            .filter(|t| t.is_finite() && *t >= 0.0)
                            .map(|t| (*objective, t))
                    })
                    .min_by(|(a, at), (b, bt)| {
                        let load = |id: &str| {
                            self.assigned
                                .iter()
                                .filter(|(unit_id, goal)| {
                                    **unit_id != unit.id && goal.as_str() == id && holds(**unit_id)
                                })
                                .count()
                        };
                        load(&a.id)
                            .cmp(&load(&b.id))
                            .then(at.total_cmp(bt))
                            .then(a.id.cmp(&b.id))
                    });
                if let Some((objective, _)) = best {
                    self.assigned.insert(unit.id, objective.id.clone());
                    if rules.catalog.get(unit.kind).has_role("recon") {
                        let goal = recon_post(unit, objective.center, entry.center);
                        if route_time(unit.id, goal).is_some() {
                            orders.push(Order::Move {
                                units: vec![unit.id],
                                gesture: frame.tick,
                                goal,
                                route: contract::command::RoutePolicy::Fastest,
                                direction: contract::command::MoveDirection::Forward,
                                facing: Some(libm::atan2(
                                    objective.center[1] - goal[1],
                                    objective.center[0] - goal[0],
                                )),
                            });
                            continue;
                        }
                    }
                    orders.push(Order::AttackMove {
                        units: vec![unit.id],
                        gesture: frame.tick,
                        goal: objective.center,
                    });
                }
            }
            // Support stays between the road-edge base and the observed combat
            // group. An impossible support route falls back to the admitted base.
            let combat: Vec<_> = own
                .iter()
                .filter(|u| !rules.catalog.get(u.kind).has_role("logistics"))
                .collect();
            let support = if combat.is_empty() {
                entry.center
            } else {
                let mut center = [0.0; 2];
                for u in &combat {
                    center[0] += u.position[0];
                    center[1] += u.position[1];
                }
                [
                    entry.center[0] + 0.4 * (center[0] / combat.len() as f64 - entry.center[0]),
                    entry.center[1] + 0.4 * (center[1] / combat.len() as f64 - entry.center[1]),
                ]
            };
            for truck in own
                .iter()
                .filter(|u| rules.catalog.get(u.kind).has_role("logistics"))
            {
                let here = [truck.position[0], truck.position[1]];
                if distance(here, support) > 40.0 {
                    if truck.goal.is_some_and(|old| distance(old, support) <= 40.0) {
                        continue;
                    }
                    let goal = if route_time(truck.id, support)
                        .is_some_and(|t| t.is_finite() && t >= 0.0)
                    {
                        support
                    } else if route_time(truck.id, entry.center)
                        .is_some_and(|t| t.is_finite() && t >= 0.0)
                    {
                        entry.center
                    } else {
                        continue;
                    };
                    if distance(here, goal) > 40.0 {
                        if truck.goal.is_none_or(|old| distance(old, goal) > 40.0) {
                            orders.push(Order::Move {
                                units: vec![truck.id],
                                gesture: frame.tick,
                                goal,
                                route: contract::command::RoutePolicy::Fastest,
                                direction: contract::command::MoveDirection::Forward,
                                facing: None,
                            });
                        }
                        continue;
                    }
                }
                if truck.goal.is_none()
                    && truck
                        .deployment
                        .is_some_and(|d| d.target == contract::observation::Posture::Packed)
                {
                    orders.push(Order::SetDeployment {
                        units: vec![truck.id],
                        deployed: true,
                    });
                }
            }
        }
        if view.phase == Phase::Preparation && !view.ready[side.index()] {
            orders.push(Order::Ready);
        }
        orders
    }
}

impl SkirmishPolicy for BalancedDoctrine {
    fn decide(
        &mut self,
        side: Side,
        faction: Faction,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        self.decide_impl(side, faction, entry, frame, rules, route_time)
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

fn recon_post(
    unit: &contract::observation::OwnUnit,
    objective: [f64; 2],
    entry: [f64; 2],
) -> [f64; 2] {
    let dx = objective[0] - entry[0];
    let dy = objective[1] - entry[1];
    let approach = libm::hypot(dx, dy).max(1.0);
    let range = unit.sight.range.max(1.0);
    let standoff = (range * 0.85).min(approach * 0.75);
    [
        objective[0] - dx / approach * standoff,
        objective[1] - dy / approach * standoff,
    ]
}

fn unit_can_target_layer(rules: &Rules, kind: TypeIndex, layer: AltitudeLayer) -> bool {
    rules.catalog.mounts(kind).iter().any(|mount| {
        mount
            .def
            .weapons
            .iter()
            .filter_map(|weapon| rules.weapons.get(weapon))
            .any(|weapon| weapon.targets.contains_key(&layer))
    })
}

fn frame_has_layer_threat(frame: &ObservationFrame, rules: &Rules, layer: AltitudeLayer) -> bool {
    frame
        .identified
        .iter()
        .any(|unit| rules.catalog.get(unit.kind).mobility.layer() == layer)
        || frame.contacts.iter().any(|contact| contact.layer == layer)
}

fn candidate_score(
    rules: &Rules,
    kind: TypeIndex,
    draw_unit: f64,
    demand: Option<AltitudeLayer>,
) -> f64 {
    let unit = rules.catalog.get(kind);
    let capability = demand.map_or(0.0, |layer| {
        if unit_can_target_layer(rules, kind, layer) {
            10_000.0
        } else {
            -10_000.0
        }
    });
    let affordability_bias = 1_000.0 - f64::from(unit.cost);
    let weapon_value = demand.map_or(0.0, |layer| {
        rules
            .catalog
            .mounts(kind)
            .iter()
            .flat_map(|mount| mount.def.weapons.iter())
            .filter_map(|id| rules.weapons.get(id))
            .filter(|weapon| weapon.targets.contains_key(&layer))
            .map(|weapon| weapon.damage * weapon.ballistics.range_m)
            .fold(0.0, f64::max)
            / f64::from(unit.cost.max(1))
    });
    let noise = (contract::random::mix(u64::from(kind.0) ^ draw_unit.to_bits()) as f64
        / u64::MAX as f64
        - 0.5)
        * 40.0;
    capability + affordability_bias + weapon_value + noise
}

#[cfg(test)]
mod tests;
