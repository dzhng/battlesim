//! Observation-bound player-match policy; all outcomes remain ordinary commands.
use contract::catalog::{AltitudeLayer, Category, TypeIndex};
use contract::command::{Order, TargetRef};
use contract::encounter::EntrySite;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ObservationFrame, ObservedTargetId};
use contract::random::{mix, Rng};
use contract::scenario::Rules;
use contract::skirmish::Phase;
use std::collections::BTreeMap;

mod airlift;
mod artillery;
mod capability;
mod doctrine;
mod purchase;
mod recon;
use purchase::lane_purchase_destination;
mod relocation;
mod smoke;
mod support;
pub mod team;
use artillery::{artillery_ground_guess, artillery_objective_guess};
use capability::{frame_has_layer_threat, unit_weapons};
pub use capability::{is_jet, unit_engages_aircraft_in};
pub(crate) use capability::{unit_can_engage_ground, unit_can_engage_target};
use purchase::{own_player, PurchasePlan};
use relocation::{Held, Relocation};
mod input;
mod transport;
mod tuning;
pub use input::{Assignment, DecisionInput, PartyTask, SkirmishPolicy};
pub use team::{PlayerPlan, TeamMemory, TeamPlan};
use tuning::Tuning;

fn attack_if_changed(
    orders: &mut Vec<Order>,
    unit: &contract::observation::OwnUnit,
    target: TargetRef,
) {
    if !matches!(unit.steps.first(), Some(contract::observation::OrderStep::Attack { target: Some(active), .. }) if *active == target)
    {
        orders.push(Order::Attack {
            units: vec![unit.id],
            target,
        });
    }
}

pub use doctrine::Doctrine;

#[derive(Clone, Debug)]
pub struct BalancedDoctrine {
    tuning: Tuning,
    next_decision: Tick,
    /// The target each own jet was last sent at. A jet keeps it while the
    /// side still identifies it, as a player's would, so a pass at it is not
    /// restarted at whatever is nearest at the next decision.
    jet_targets: BTreeMap<UnitId, ObservedTargetId>,
    reinforcement: usize,
    doctrine: Doctrine,
    started: bool,
    doctrine_since: Tick,
    pivot_count: u32,
    last_own_count: Option<usize>,
    last_air_evidence: Option<Tick>,
    response_delay_ticks: Option<Tick>,
    random: Rng,
    sightings: BTreeMap<ObservedTargetId, ArtillerySighting>,
    /// Own units their weapons' own cover holds (`BlockedTrajectory`).
    held: BTreeMap<UnitId, Held>,
    /// Own units' recent losses, which pop their smoke.
    hp_watch: smoke::HpWatch,
    /// An optional external purchase plan. When present it replaces all
    /// doctrine shopping and advances only when the battle admits a card.
    purchase_plan: Option<PurchasePlan>,
    group_advance: Option<Assignment>,
    /// Each carrier sent to collect a squad, by the last decision.
    rides: BTreeMap<UnitId, UnitId>,
    airlift: airlift::Mission,
}

#[derive(Clone, Debug)]
struct ArtillerySighting {
    tick: Tick,
    position: [f64; 3],
    velocity: [f64; 2],
}

impl Default for BalancedDoctrine {
    fn default() -> Self {
        Self {
            tuning: Tuning::shipped(),
            next_decision: 0,
            jet_targets: BTreeMap::new(),
            reinforcement: 0,
            doctrine: Doctrine::BalancedOpening,
            started: false,
            doctrine_since: 0,
            pivot_count: 0,
            last_own_count: None,
            last_air_evidence: None,
            response_delay_ticks: None,
            random: Rng::new(0),
            sightings: BTreeMap::new(),
            held: BTreeMap::new(),
            hp_watch: smoke::HpWatch::default(),
            purchase_plan: None,
            group_advance: None,
            rides: BTreeMap::new(),
            airlift: airlift::Mission::Idle,
        }
    }
}

impl BalancedDoctrine {
    pub fn new(seed: u64) -> Self {
        Self {
            reinforcement: (mix(seed) as u32 % 5) as usize,
            random: Rng::new(mix(seed ^ 0x534b_4952_4d49_5348)),
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

    pub(crate) fn wants_decision(&self, tick: Tick) -> bool {
        tick >= self.next_decision
    }

    /// Where `unit` heads for `objective`: a recon unit's post short of it,
    /// any other unit its centre.
    fn heading(
        &self,
        rules: &Rules,
        entry: &EntrySite,
        unit: &contract::observation::OwnUnit,
        objective: [f64; 2],
    ) -> [f64; 2] {
        if rules.catalog.get(unit.kind).has_role("recon") {
            recon::post(unit, &self.tuning, objective, entry.center)
        } else {
            objective
        }
    }

    pub fn decide(
        &mut self,
        input: DecisionInput<'_>,
        frame: &ObservationFrame,
        rules: &Rules,
        mut route_time: impl FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        self.decide_impl(input, frame, rules, &mut route_time)
    }

    fn decide_impl(
        &mut self,
        input: DecisionInput<'_>,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        let DecisionInput {
            side,
            faction,
            entry,
            assignment: context,
        } = input;
        if context != self.group_advance.as_ref() {
            self.group_advance = None;
        }
        let Some(view) = &frame.skirmish else {
            return vec![];
        };
        let Some(me) = own_player(view) else {
            return vec![];
        };
        let horizon = Tuning::ticks(self.tuning.sighting_horizon_s, rules.tick_hz);
        self.sightings
            .retain(|_, seen| frame.tick.saturating_sub(seen.tick) <= horizon);
        for target in &frame.identified {
            if rules.catalog.get(target.kind).mobility.layer() == AltitudeLayer::Ground {
                self.sightings.insert(
                    target.id,
                    ArtillerySighting {
                        tick: frame.tick,
                        position: target.position,
                        velocity: target.velocity,
                    },
                );
            }
        }
        if view.phase == Phase::Finished || frame.tick < self.next_decision {
            return vec![];
        }
        let draw = self.random.next_u64();
        let draw_unit = (draw >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        self.next_decision = frame
            .tick
            .saturating_add(Tuning::ticks(self.tuning.decision_s, rules.tick_hz));
        doctrine::update(self, frame, rules, draw_unit);
        self.held
            .retain(|id, _| frame.own.iter().any(|u| u.id == *id));
        self.jet_targets.retain(|id, target| {
            frame.own.iter().any(|u| u.id == *id)
                && frame.identified.iter().any(|t| t.id == *target)
        });
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
        // A losing unit's smoke comes first, so the order rate limit never
        // drops its pop (D21).
        let smoke = match view.phase {
            Phase::Active => smoke::smoke_orders(
                &frame.own,
                &mut self.hp_watch,
                &self.tuning,
                rules,
                frame.tick,
                route_time,
            ),
            _ => vec![],
        };
        let mut orders = smoke.clone();
        // Emergency airlift is considered before the doctrine's ordinary
        // role rotation. A flying supplier is a one-at-a-time reserve, so a
        // present one is dispatched and only an absent one is bought.
        let emergency = (view.phase == Phase::Active && !self.airlift.is_busy())
            .then(|| {
                airlift::emergency(
                    frame,
                    rules,
                    &self.tuning,
                    &airlift::groups(frame, rules, context),
                    route_time,
                )
            })
            .flatten();
        let emergency_buy = emergency.clone().and_then(|need| {
            airlift::respond(&mut self.airlift, need, frame, rules, me, &eligible)
        });
        let purchases = emergency_buy.into_iter().collect::<Vec<_>>();
        let purchases = if purchases.is_empty() && !(emergency.is_some() && self.airlift.is_busy())
        {
            self.shopping(frame, rules, me, view.phase, &eligible)
        } else {
            purchases
        };
        for purchase::Buy { kind, carrier } in purchases {
            // Support, carriers and the squads they will carry wait at base:
            // a squad rides out from there (D17), as a squad bought aboard
            // its ride is driven out (`unload`).
            let waits = rules.catalog.get(kind).has_role("logistics")
                || crate::carriage::seats(rules, kind) > 0
                || transport::carrier_for(rules, &eligible, rules.catalog.get(kind).squad_size())
                    .is_some();
            let departure =
                lane_purchase_destination(entry, entry.center, self.tuning.purchase_advance_m);
            let destination = if let Some(context) = context {
                // Reserve just past the lane entry. Entries still owns the
                // nearest-entry choice, while the unit clears the footprint
                // immediately instead of blocking the next dispatch.
                lane_purchase_destination(
                    entry,
                    context.staging_point,
                    self.tuning.purchase_advance_m,
                )
            } else if waits {
                departure
            } else {
                view.objectives
                    .iter()
                    .filter(|o| o.owner != Some(side))
                    .min_by(|a, b| {
                        distance(entry.center, a.center)
                            .total_cmp(&distance(entry.center, b.center))
                            .then(a.id.cmp(&b.id))
                    })
                    .map_or(departure, |o| {
                        if distance(entry.center, o.center) > 0.0 {
                            o.center
                        } else {
                            departure
                        }
                    })
            };
            orders.push(Order::ConfirmPurchase {
                variant: rules.catalog.id(kind).into(),
                carrier: carrier.map(|carrier| rules.catalog.id(carrier).into()),
                destination,
            });
        }
        if view.phase == Phase::Active {
            let carries =
                |u: &contract::observation::OwnUnit| crate::carriage::seats(rules, u.kind) > 0;
            // Carriers take their turn last, once every squad has claimed
            // its ride.
            let mut own: Vec<_> = frame.own.iter().collect();
            own.sort_by_key(|u| (carries(u), u.id));
            // Carriers with no one aboard, free to collect a squad, and the
            // squad each one is sent for at this decision.
            let boarding = transport::boarding_pairs(frame);
            let mut free = transport::free_carriers(rules, frame, &self.rides, &boarding);
            let mut collect: BTreeMap<UnitId, UnitId> = BTreeMap::new();
            let group_ready = context.is_some_and(|context| {
                let radius = self.tuning.team.group_radius_m;
                let positions = team::group_positions(
                    rules,
                    own.iter().copied().filter(|unit| {
                        if !team::is_group_unit(rules, unit) {
                            return false;
                        }
                        if context
                            .parties
                            .iter()
                            .any(|party| party.units.contains(&unit.id))
                        {
                            return false;
                        }
                        if carries(unit)
                            && (transport::passengers(frame, unit.id).next().is_some()
                                || own.iter().any(|squad| {
                                    !carries(squad) && transport::coming_for(rules, unit, squad)
                                }))
                        {
                            return false;
                        }
                        if unit.engagement == contract::command::Engagement::FireAtWill
                            && frame
                                .identified
                                .iter()
                                .any(|target| unit_can_engage_target(rules, unit.kind, target.kind))
                        {
                            return false;
                        }
                        if rules.catalog.get(unit.kind).has_role("artillery") {
                            let has_ground_task = artillery_ground_guess(
                                unit,
                                frame,
                                rules,
                                &self.tuning,
                                &self.sightings,
                            )
                            .is_some();
                            let has_objective_task = !has_ground_task
                                && artillery_objective_guess(
                                    unit,
                                    side,
                                    entry,
                                    frame,
                                    rules,
                                    &self.tuning,
                                    route_time,
                                )
                                .is_some();
                            if has_ground_task || has_objective_task {
                                return false;
                            }
                        }
                        let position = [unit.position[0], unit.position[1]];
                        distance(position, context.staging_point) <= radius
                            || live_goal(unit)
                                .is_some_and(|goal| distance(goal, context.staging_point) <= radius)
                    }),
                );
                let ready = context.force_advance
                    || team::group_ready(&positions, context.staging_point, radius);
                if ready {
                    self.group_advance = Some(context.clone());
                }
                ready || self.group_advance.is_some()
            });
            let rides = transport::RidePlan {
                frame,
                rules,
                input,
                group_ready,
            };
            for unit in &own {
                // Passengers take no orders, nor does a unit backing out.
                if boarding.contains(&unit.id)
                    || rules.catalog.get(unit.kind).has_role("logistics")
                    || transport::carrier_of(unit).is_some()
                    || smoke::backing_out(unit, &smoke)
                {
                    continue;
                }
                // A carrier collects the squad that claimed it, or drives a
                // loaded squad to its drop point. Empty, it fights as its
                // mounts let it, or waits for a squad with none.
                if carries(unit) {
                    if let Some(&squad) = collect.get(&unit.id) {
                        let standing = own
                            .iter()
                            .find(|u| u.id == squad)
                            .is_some_and(|squad| transport::coming_for(rules, unit, squad));
                        if !standing {
                            orders.push(Order::Collect {
                                units: vec![unit.id],
                                squads: vec![squad],
                                gesture: frame.tick,
                            });
                        }
                        continue;
                    }
                    if let Some(passenger) =
                        transport::passengers(frame, unit.id).min_by_key(|u| u.id)
                    {
                        orders.extend(rides.unload(self, unit, passenger));
                        continue;
                    }
                    if rules.catalog.mounts(unit.kind).is_empty() {
                        continue;
                    }
                }
                match self.relocation(unit, frame.tick, rules, route_time) {
                    Relocation::Stay => {}
                    Relocation::Driving => continue,
                    Relocation::To(goal) => {
                        orders.push(Order::Move {
                            units: vec![unit.id],
                            gesture: frame.tick,
                            goal,
                            route: contract::command::RoutePolicy::Fastest,
                            direction: contract::command::MoveDirection::Forward,
                            facing: None,
                        });
                        continue;
                    }
                }
                // A team push (or a lane fallback/help task) is an explicit
                // movement order. Give that order priority over a stale
                // attack target so every lane the team sends actually starts
                // toward its assigned point in this decision interval.
                if let Some(context) = context.filter(|context| {
                    context.force_advance
                        && team::is_group_unit(rules, unit)
                        && !(rules.catalog.get(unit.kind).has_role("artillery")
                            || rules.catalog.get(unit.kind).mobility.layer()
                                != AltitudeLayer::Ground)
                }) {
                    let objective = context
                        .parties
                        .iter()
                        .find(|party| party.units.contains(&unit.id))
                        .map_or(context.main_objective, |help| help.objective);
                    if team::holds_backline(rules, unit) {
                        orders.extend(team::backline_order(
                            rules,
                            frame,
                            unit,
                            entry.center,
                            context.staging_point,
                            self.tuning.reaim_m,
                        ));
                        continue;
                    }
                    if rides.worth_riding(self, unit, objective)
                        && transport::wait_for_ride(
                            rules,
                            unit,
                            &mut free,
                            &mut collect,
                            &mut orders,
                            route_time,
                        )
                    {
                        continue;
                    }
                    if live_goal(unit).is_none_or(|old| {
                        distance(old, objective) > self.tuning.team.group_radius_m
                    }) {
                        orders.push(Order::AttackMove {
                            units: vec![unit.id],
                            gesture: frame.tick,
                            goal: objective,
                        });
                    }
                    continue;
                }
                if unit.engagement == contract::command::Engagement::FireAtWill {
                    let jet = is_jet(rules, unit.kind);
                    if jet && self.jet_targets.contains_key(&unit.id) {
                        continue;
                    }
                    if let Some(target) = frame
                        .identified
                        .iter()
                        .filter(|target| unit_can_engage_target(rules, unit.kind, target.kind))
                        .min_by(|a, b| {
                            body_distance(unit.position, a.position)
                                .total_cmp(&body_distance(unit.position, b.position))
                                .then(a.id.cmp(&b.id))
                        })
                    {
                        if jet {
                            self.jet_targets.insert(unit.id, target.id);
                        }
                        // A squad far from a fight on the ground rides to it.
                        let fight = [target.position[0], target.position[1]];
                        if rules.catalog.get(target.kind).mobility.layer() == AltitudeLayer::Ground
                            && rides.worth_riding(self, unit, fight)
                            && transport::wait_for_ride(
                                rules,
                                unit,
                                &mut free,
                                &mut collect,
                                &mut orders,
                                route_time,
                            )
                        {
                            continue;
                        }
                        attack_if_changed(
                            &mut orders,
                            unit,
                            TargetRef::Identified { id: target.id },
                        );
                        continue;
                    }
                    if rules.catalog.get(unit.kind).has_role("artillery") {
                        if let Some(point) = artillery_ground_guess(
                            unit,
                            frame,
                            rules,
                            &self.tuning,
                            &self.sightings,
                        )
                        .or_else(|| {
                            artillery_objective_guess(
                                unit,
                                side,
                                entry,
                                frame,
                                rules,
                                &self.tuning,
                                route_time,
                            )
                        }) {
                            attack_if_changed(&mut orders, unit, TargetRef::Ground { point });
                            continue;
                        }
                    }
                }
                if let Some(context) = context {
                    let help = context
                        .parties
                        .iter()
                        .find(|party| party.units.contains(&unit.id));
                    let catalog = rules.catalog.get(unit.kind);
                    let objective = help.map_or(context.main_objective, |help| help.objective);
                    if team::holds_backline(rules, unit) {
                        orders.extend(team::backline_order(
                            rules,
                            frame,
                            unit,
                            entry.center,
                            context.staging_point,
                            self.tuning.reaim_m,
                        ));
                        continue;
                    }
                    if catalog.has_role("recon") {
                        let goal = recon::post(unit, &self.tuning, objective, entry.center);
                        if route_time(unit.id, goal).is_some() {
                            orders.push(Order::Move {
                                units: vec![unit.id],
                                gesture: frame.tick,
                                goal,
                                route: contract::command::RoutePolicy::Fastest,
                                direction: contract::command::MoveDirection::Forward,
                                facing: Some(libm::atan2(
                                    objective[1] - goal[1],
                                    objective[0] - goal[0],
                                )),
                            });
                        }
                        continue;
                    }
                    if help.is_some() {
                        // A party squad rides when it is worth it, in any
                        // stance: its carrier drives it to the party's goal.
                        if rides.worth_riding(self, unit, objective)
                            && transport::wait_for_ride(
                                rules,
                                unit,
                                &mut free,
                                &mut collect,
                                &mut orders,
                                route_time,
                            )
                        {
                            continue;
                        }
                        if live_goal(unit)
                            .is_some_and(|old| distance(old, objective) <= self.tuning.reaim_m)
                        {
                            continue;
                        }
                        orders.push(Order::AttackMove {
                            units: vec![unit.id],
                            gesture: frame.tick,
                            goal: objective,
                        });
                        continue;
                    }
                    if view
                        .objectives
                        .iter()
                        .find(|candidate| candidate.center == objective)
                        .is_some_and(|candidate| candidate.owner == Some(side))
                    {
                        continue;
                    }
                    let goal = if group_ready {
                        objective
                    } else {
                        context.staging_point
                    };
                    if rides.worth_riding(self, unit, goal)
                        && transport::wait_for_ride(
                            rules,
                            unit,
                            &mut free,
                            &mut collect,
                            &mut orders,
                            route_time,
                        )
                    {
                        continue;
                    }
                    if live_goal(unit)
                        .is_some_and(|old| distance(old, goal) <= self.tuning.team.group_radius_m)
                    {
                        continue;
                    }
                    if !team::is_group_unit(rules, unit) {
                        continue;
                    }
                    // The authoritative movement owner will report a blocked
                    // route if terrain refuses it; issuing the group order is
                    // still preferable to silently abandoning the lane.
                    orders.push(Order::AttackMove {
                        units: vec![unit.id],
                        gesture: frame.tick,
                        goal,
                    });
                    continue;
                }
                if live_goal(unit).is_some() {
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
                let Some((objective, _)) = available
                    .iter()
                    .filter_map(|objective| {
                        route_time(unit.id, objective.center)
                            .filter(|t| t.is_finite() && *t >= 0.0)
                            .map(|t| (*objective, t))
                    })
                    .min_by(|(a, at), (b, bt)| at.total_cmp(bt).then(a.id.cmp(&b.id)))
                else {
                    continue;
                };
                // A squad heading far for its objective stops for the
                // nearest free carrier to collect it (D17).
                let heading = self.heading(rules, entry, unit, objective.center);
                if rides.worth_riding(self, unit, heading)
                    && transport::wait_for_ride(
                        rules,
                        unit,
                        &mut free,
                        &mut collect,
                        &mut orders,
                        route_time,
                    )
                {
                    continue;
                }
                if rules.catalog.get(unit.kind).has_role("recon") {
                    let goal = recon::post(unit, &self.tuning, objective.center, entry.center);
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
            self.rides = collect;
            support::support_orders(
                &own,
                rules,
                entry.center,
                &self.tuning,
                frame.tick,
                route_time,
                &mut orders,
            );
            airlift::orders(
                &mut self.airlift,
                frame,
                rules,
                entry.center,
                &self.tuning,
                route_time,
                &mut orders,
            );
        }
        if view.phase == Phase::Preparation && !me.ready {
            orders.push(Order::Ready);
        }
        orders
    }
}

impl SkirmishPolicy for BalancedDoctrine {
    fn decide(
        &mut self,
        input: DecisionInput<'_>,
        frame: &ObservationFrame,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Vec<Order> {
        self.decide_impl(input, frame, rules, route_time)
    }
}

/// A unit's movement goal while its route still leads there: a blocked
/// route is a goal to replace, not one to keep waiting on.
fn live_goal(unit: &contract::observation::OwnUnit) -> Option<[f64; 2]> {
    unit.goal
        .filter(|_| unit.state != contract::observation::MoveState::RouteBlocked)
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

fn body_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    crate::math::body_distance(
        crate::math::v3(a[0], a[1], a[2]),
        crate::math::v3(b[0], b[1], b[2]),
    )
}

fn ground_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

#[cfg(test)]
mod tests;
