//! Observation-bound player-match policy; all outcomes remain ordinary commands.
use contract::catalog::{AltitudeLayer, Category, Faction, TypeIndex};
use contract::command::{Order, TargetRef};
use contract::encounter::EntrySite;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ObservationFrame, ObservedTargetId};
use contract::scenario::Rules;
use contract::skirmish::Phase;
use contract::weapons::{TargetClass, WeaponDefinition};
use std::collections::BTreeMap;

mod transport;

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

/// A policy's view of one player. The simulation remains side-observing, while
/// economy and own-unit queries are narrowed to the player's ownership.
#[derive(Clone, Copy)]
pub struct PlayerLens<'a> {
    pub frame: &'a ObservationFrame,
    pub me: contract::players::PlayerIndex,
}

impl<'a> PlayerLens<'a> {
    pub fn own(&'a self) -> impl Iterator<Item = &'a contract::observation::OwnUnit> + 'a {
        self.frame
            .own
            .iter()
            .filter(move |unit| unit.owner == Some(self.me))
    }
    pub fn economy(&self) -> Option<&'a contract::skirmish::SkirmishView> {
        self.frame.skirmish.as_ref()
    }
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

/// The doctrine's timings and distances, authored in
/// `fixtures/skirmish-ai.json`: its playbook's numbers, not battle rules.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Tuning {
    /// Seconds between decisions.
    decision_s: f64,
    /// The longest delay before it answers air evidence; each AI draws its
    /// own once, up to this.
    reaction_max_s: f64,
    /// Seconds air evidence keeps it ready to answer after the last sighting.
    air_memory_s: f64,
    /// A doctrine holds at least `min_dwell_s` before a threat turns it, and
    /// moves on after `max_dwell_s`.
    min_dwell_s: f64,
    max_dwell_s: f64,
    /// Seconds a sighting or contact stays fresh enough for its artillery.
    sighting_horizon_s: f64,
    /// Its supply trucks wait this share of the way from its entry to its
    /// combat units, and move only when farther than `truck_slack_m` off.
    supply_share: f64,
    truck_slack_m: f64,
    /// Its artillery aims this share of a contact's radius off the contact's
    /// centre, and this share of an objective's radius past the objective's
    /// centre, each radius capped at `aim_cap_m`.
    contact_aim_share: f64,
    objective_aim_share: f64,
    aim_cap_m: f64,
    /// Its recon stands this share of its sight short of an objective, and
    /// at most this share of the way back along its approach.
    recon_sight_share: f64,
    recon_approach_share: f64,
    /// How wide the random term in a purchase's score spreads.
    purchase_noise: f64,
    /// How long it lets a unit of its own stand held by its own cover
    /// (`BlockedTrajectory`, from any weapon) before it moves it, and how
    /// far: farther than the cover a unit is likely to stand under.
    relocate_after_s: f64,
    relocate_m: f64,
    /// With no identified enemy in reach, its carriers let squads out this
    /// share of the way from its entry to an objective short of it.
    unload_share: f64,
    /// A squad rides only when the ride sets it down at least this much
    /// nearer its objective; nearer than that, it walks.
    ride_min_m: f64,
    /// A loaded carrier keeps its drop point until a new one lies farther
    /// than this from it, and lets its squad out within this of it.
    reaim_m: f64,
}

impl Tuning {
    fn shipped() -> Self {
        serde_json::from_str(include_str!("../../../../fixtures/skirmish-ai.json"))
            .expect("authored skirmish AI tuning")
    }

    /// `s` seconds in ticks at `hz`.
    fn ticks(s: f64, hz: u32) -> Tick {
        (s * f64::from(hz)) as Tick
    }
}

#[derive(Clone, Debug)]
pub struct BalancedDoctrine {
    tuning: Tuning,
    next_decision: Tick,
    assigned: BTreeMap<UnitId, String>,
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
    random: contract::random::Rng,
    sightings: BTreeMap<ObservedTargetId, ArtillerySighting>,
    /// Own units their weapons' own cover holds (`BlockedTrajectory`).
    held: BTreeMap<UnitId, Held>,
}

/// A unit its own cover holds from firing: since when (while it stands
/// held), how often it has been moved for it, and whether it is on its way.
#[derive(Clone, Debug, Default)]
struct Held {
    since: Option<Tick>,
    moves: usize,
    driving: bool,
}

/// What a held unit is to do at this decision.
enum Relocation {
    /// Nothing yet: it has not been held long enough, or it is not held.
    Stay,
    /// Already on its way to other ground: leave its move alone.
    Driving,
    /// Drive to this ground.
    To([f64; 2]),
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
            assigned: BTreeMap::new(),
            jet_targets: BTreeMap::new(),
            reinforcement: 0,
            doctrine: Doctrine::BalancedOpening,
            started: false,
            doctrine_since: 0,
            pivot_count: 0,
            last_own_count: None,
            last_air_evidence: None,
            response_delay_ticks: None,
            random: contract::random::Rng::new(0),
            sightings: BTreeMap::new(),
            held: BTreeMap::new(),
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

    /// A unit its own cover keeps from firing (`BlockedTrajectory` on any
    /// weapon at every decision while it stands, for its tuning's
    /// `relocate_after_s`) is driven to other ground `relocate_m` off, as a
    /// player would move it: the first of eight bearings a route reaches, starting from its heading, one
    /// bearing further round for each earlier move. Arriving, it is watched
    /// afresh, and moved again if it is still held.
    fn relocation(
        &mut self,
        unit: &contract::observation::OwnUnit,
        tick: Tick,
        rules: &Rules,
        route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
    ) -> Relocation {
        let after = Tuning::ticks(self.tuning.relocate_after_s, rules.tick_hz);
        let reach = self.tuning.relocate_m;
        let held = self.held.entry(unit.id).or_default();
        if held.driving {
            // A route that failed on the way ends the move as arriving does.
            if unit.goal.is_some() && unit.state != contract::observation::MoveState::RouteBlocked {
                return Relocation::Driving;
            }
            held.driving = false;
        }
        let blocked = unit.state == contract::observation::MoveState::Idle
            && unit
                .mounts
                .iter()
                .any(|m| m.reason == contract::observation::ActionReason::BlockedTrajectory);
        if !blocked {
            held.since = None;
            return Relocation::Stay;
        }
        let since = *held.since.get_or_insert(tick);
        if tick.saturating_sub(since) < after {
            return Relocation::Stay;
        }
        let step = std::f64::consts::FRAC_PI_4;
        let goal = (0..8)
            .map(|k| unit.yaw + step * (held.moves + k) as f64)
            .map(|bearing| {
                [
                    unit.position[0] + libm::cos(bearing) * reach,
                    unit.position[1] + libm::sin(bearing) * reach,
                ]
            })
            .find(|&goal| route_time(unit.id, goal).is_some_and(|t| t.is_finite() && t >= 0.0));
        let Some(goal) = goal else {
            return Relocation::Stay;
        };
        held.since = None;
        held.moves += 1;
        held.driving = true;
        Relocation::To(goal)
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
            recon_post(unit, &self.tuning, objective, entry.center)
        } else {
            objective
        }
    }

    /// Where a carrier at `from` lets a squad out for `goal` (D18), with
    /// no enemy in reach `unload_share` of the way from entry to goal short
    /// of it.
    fn drop_point(
        &self,
        frame: &ObservationFrame,
        rules: &Rules,
        entry: &EntrySite,
        goal: [f64; 2],
        from: [f64; 2],
    ) -> [f64; 2] {
        let standoff = self.tuning.unload_share * distance(entry.center, goal);
        transport::unload_point(frame, rules, goal, from, standoff)
    }

    /// Whether `unit`, a squad on foot, rides for `goal`: a ride from where
    /// it stands would set it down `ride_min_m` nearer or more.
    fn worth_riding(
        &self,
        frame: &ObservationFrame,
        rules: &Rules,
        entry: &EntrySite,
        unit: &contract::observation::OwnUnit,
        goal: [f64; 2],
    ) -> bool {
        if unit.members.is_empty() {
            return false;
        }
        let here = [unit.position[0], unit.position[1]];
        let out = self.drop_point(frame, rules, entry, goal, here);
        distance(here, goal) - distance(out, goal) >= self.tuning.ride_min_m
    }

    /// A loaded carrier's order: drive its squad toward the fight (the
    /// nearest enemy on the ground it can engage) or else its objective, and
    /// let it out at the drop point, or where it is once it is there or in
    /// an enemy's reach. None while anyone boards or leaves, or the order
    /// already stands.
    fn unload(
        &self,
        side: Side,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        carrier: &contract::observation::OwnUnit,
    ) -> Option<Order> {
        let passenger = transport::passengers(frame, carrier.id).min_by_key(|u| u.id)?;
        if carrier.withdrawing || !transport::all_inside(frame, carrier.id) {
            return None;
        }
        let here = [carrier.position[0], carrier.position[1]];
        let available = || {
            frame
                .skirmish
                .iter()
                .flat_map(|view| &view.objectives)
                .filter(|o| o.owner != Some(side))
        };
        let objective = self
            .assigned
            .get(&passenger.id)
            .and_then(|id| available().find(|o| &o.id == id))
            .or_else(|| {
                available().min_by(|a, b| {
                    distance(here, a.center)
                        .total_cmp(&distance(here, b.center))
                        .then(a.id.cmp(&b.id))
                })
            });
        let order = |goal| Order::Unload {
            units: vec![carrier.id],
            goal,
            gesture: frame.tick,
            route: contract::command::RoutePolicy::Fastest,
        };
        let goal = match nearest_ground_target(frame, rules, passenger.kind, here) {
            Some(target) => [target.position[0], target.position[1]],
            None => match objective {
                Some(objective) => self.heading(rules, entry, passenger, objective.center),
                // Nothing to fight and every objective held: out here.
                None => return carrier.goal.is_none().then(|| order(None)),
            },
        };
        let out = self.drop_point(frame, rules, entry, goal, here);
        let slack = self.tuning.reaim_m;
        if distance(here, out) <= slack {
            carrier.goal.is_none().then(|| order(None))
        } else {
            carrier
                .goal
                .is_none_or(|old| distance(old, out) > slack)
                .then(|| order(Some(out)))
        }
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
                Some(1 + Tuning::ticks(draw_unit * self.tuning.reaction_max_s, rules.tick_hz));
        }
        let low_air = frame_has_layer_threat(frame, rules, AltitudeLayer::LowAir);
        let own_count = frame.own.len();
        let lost_unit = self
            .last_own_count
            .is_some_and(|previous| own_count < previous);
        self.last_own_count = Some(own_count);
        if low_air {
            self.last_air_evidence.get_or_insert(frame.tick);
        } else if self.last_air_evidence.is_some_and(|seen| {
            frame.tick.saturating_sub(seen) > Tuning::ticks(self.tuning.air_memory_s, rules.tick_hz)
        }) {
            self.last_air_evidence = None;
        }
        let min_dwell = Tuning::ticks(self.tuning.min_dwell_s, rules.tick_hz);
        let max_dwell = Tuning::ticks(self.tuning.max_dwell_s, rules.tick_hz);
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
        self.update_doctrine(frame, rules, draw_unit);
        self.assigned
            .retain(|id, _| frame.own.iter().any(|u| u.id == *id));
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
        // The cheapest cards of `role` (all of `category` when no role is
        // named, or the deck has no card of it), the `ordinal`-th of them.
        // An unarmed carrier only carries: pairing alone buys it.
        let unarmed_carrier =
            |k: TypeIndex| transport::seats(rules, k) > 0 && rules.catalog.mounts(k).is_empty();
        let pick = |role: Option<&str>, category: Category, ordinal: usize| {
            let mut matching: Vec<_> = role
                .map(|role| {
                    eligible
                        .iter()
                        .copied()
                        .filter(|&k| rules.catalog.get(k).has_role(role) && !unarmed_carrier(k))
                        .collect()
                })
                .unwrap_or_default();
            if matching.is_empty() && role != Some("artillery") {
                matching = eligible
                    .iter()
                    .copied()
                    .filter(|&k| !unarmed_carrier(k))
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
        let high_air_demand = frame_has_layer_threat(frame, rules, AltitudeLayer::HighAir);
        let opening = match self.doctrine {
            Doctrine::TankSwarm => [
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 1),
                ("recon", Category::Rec, 0),
                ("at", Category::Inf, 0),
                ("infantry", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("artillery", Category::Sup, 0),
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
                ("artillery", Category::Sup, 0),
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
                ("artillery", Category::Sup, 0),
                ("logistics", Category::Sup, 0),
            ],
        };
        let desired: Vec<_> = opening
            .iter()
            .filter_map(|&(role, category, ordinal)| pick(Some(role), category, ordinal))
            .collect();
        // AA is insurance every opening carries, picked by what its weapons
        // engage. It goes first, so it is bought before any other role
        // competes for credits or slots.
        let high_air_defender = best_air_defender(
            rules,
            &self.tuning,
            &eligible,
            draw_unit,
            AltitudeLayer::HighAir,
        );
        let desired: Vec<_> = high_air_defender.iter().copied().chain(desired).collect();
        // Each squad's carrier follows it (D17), and rides on it: it is
        // bought only for a squad the player holds or buys with it.
        let desired: Vec<(TypeIndex, bool)> = desired
            .into_iter()
            .flat_map(|kind| {
                let size = rules.catalog.get(kind).squad_size();
                std::iter::once((kind, false)).chain(
                    transport::carrier_for(rules, &eligible, size).map(|carrier| (carrier, true)),
                )
            })
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
            let mut slots = view.unit_count;
            let mut needed = BTreeMap::new();
            let mut squad_held = false;
            desired
                .iter()
                .copied()
                .filter(|&(kind, rides)| {
                    let wanted = needed.entry(kind).or_insert(0);
                    *wanted += 1;
                    let cost = f64::from(rules.catalog.get(kind).cost);
                    let held = count(kind) >= *wanted;
                    let buy = !held
                        && (!rides || squad_held)
                        && cost <= credits
                        && slots < view.max_units;
                    if buy {
                        credits -= cost;
                        slots += 1;
                    }
                    squad_held = held || buy;
                    buy
                })
                .map(|(kind, _)| kind)
                .collect()
        } else if view.unit_count < view.max_units {
            let purchase = {
                let truck = pick(Some("logistics"), Category::Sup, 0);
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
                    // the air category (fixed-wing aircraft, by no role of
                    // their own) is next in the same rotation. A deck
                    // without one buys the next role; an unaffordable one is
                    // saved for.
                    let roles = [
                        (Some("infantry"), Category::Inf),
                        (Some("recon"), Category::Rec),
                        (Some("at"), Category::Inf),
                        (Some("light_vehicle"), Category::Veh),
                        (Some("artillery"), Category::Sup),
                        (Some("helicopter"), Category::Hel),
                        (None, Category::Air),
                    ];
                    let selected = if let Some(layer) = [
                        (AltitudeLayer::HighAir, high_air_demand),
                        (AltitudeLayer::LowAir, low_air_demand),
                    ]
                    .into_iter()
                    .find_map(|(layer, demanded)| demanded.then_some(layer))
                    {
                        best_air_defender(rules, &self.tuning, &eligible, draw_unit, layer)
                            .filter(|&k| f64::from(rules.catalog.get(k).cost) <= view.credits)
                            .map(|k| (Some(0), k))
                    } else if let Some(want) = transport::want(
                        rules,
                        &eligible,
                        frame
                            .own
                            .iter()
                            .map(|u| u.kind)
                            .chain(view.pending.iter().map(|p| p.kind)),
                    ) {
                        // One carrier per squad comes before the rotation,
                        // and is saved for like its next role: a lost
                        // transport is rebought, a lost squad replaced (D17).
                        let (transport::Want::Carrier(k) | transport::Want::Squad(k)) = want;
                        (f64::from(rules.catalog.get(k).cost) <= view.credits).then_some((None, k))
                    } else {
                        // The next role in the rotation is a plan the AI
                        // saves for: credits it can't yet spend on it wait,
                        // so an expensive card is bought as surely as a
                        // cheap one. Only a role the deck has no card for
                        // is passed over.
                        (0..roles.len())
                            .find_map(|offset| {
                                let index = (self.reinforcement + offset) % roles.len();
                                pick(roles[index].0, roles[index].1, 0).map(|k| (Some(index), k))
                            })
                            .filter(|&(_, k)| f64::from(rules.catalog.get(k).cost) <= view.credits)
                    };
                    selected.map(|(index, k)| {
                        if let Some(index) = index {
                            self.reinforcement = (index + 1) % roles.len();
                        }
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
            // Support, carriers and the squads they will carry wait at base:
            // a squad rides out from there (D17).
            let waits = rules.catalog.get(kind).has_role("logistics")
                || transport::seats(rules, kind) > 0
                || transport::carrier_for(rules, &eligible, rules.catalog.get(kind).squad_size())
                    .is_some();
            let destination = if waits {
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
            let carries = |u: &contract::observation::OwnUnit| transport::seats(rules, u.kind) > 0;
            // Carriers take their turn last, once every squad has claimed
            // its ride.
            let mut own: Vec<_> = frame.own.iter().collect();
            own.sort_by_key(|u| (carries(u), u.id));
            // Carriers with no one aboard, free to collect a squad, and the
            // squad each one is sent for at this decision.
            let mut free: Vec<_> = own
                .iter()
                .copied()
                .filter(|u| {
                    carries(u)
                        && !u.withdrawing
                        && u.state != contract::observation::MoveState::RouteBlocked
                        && transport::passengers(frame, u.id).next().is_none()
                })
                .collect();
            let mut collect: BTreeMap<UnitId, UnitId> = BTreeMap::new();
            for unit in &own {
                // Passengers take no orders.
                if rules.catalog.get(unit.kind).has_role("logistics")
                    || transport::carrier_of(unit).is_some()
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
                    if transport::passengers(frame, unit.id).next().is_some() {
                        orders.extend(self.unload(side, entry, frame, rules, unit));
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
                            distance3(unit.position, a.position)
                                .total_cmp(&distance3(unit.position, b.position))
                                .then(a.id.cmp(&b.id))
                        })
                    {
                        if jet {
                            self.jet_targets.insert(unit.id, target.id);
                        }
                        // A squad far from a fight on the ground rides to it.
                        let fight = [target.position[0], target.position[1]];
                        if rules.catalog.get(target.kind).mobility.layer() == AltitudeLayer::Ground
                            && self.worth_riding(frame, rules, entry, unit, fight)
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
                        orders.push(Order::Attack {
                            units: vec![unit.id],
                            target: TargetRef::Identified { id: target.id },
                        });
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
                            orders.push(Order::Attack {
                                units: vec![unit.id],
                                target: TargetRef::Ground { point },
                            });
                            continue;
                        }
                    }
                }
                let available: Vec<_> = view
                    .objectives
                    .iter()
                    .filter(|o| o.owner != Some(side))
                    .collect();
                if available.is_empty() {
                    continue;
                }
                let busy = self
                    .assigned
                    .get(&unit.id)
                    .and_then(|id| available.iter().copied().find(|o| &o.id == id))
                    .filter(|o| {
                        unit.goal.is_some()
                            || distance([unit.position[0], unit.position[1]], o.center)
                                <= o.radius_m
                    });
                let objective = match busy {
                    Some(objective) => objective,
                    None => {
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
                                            **unit_id != unit.id
                                                && goal.as_str() == id
                                                && holds(**unit_id)
                                        })
                                        .count()
                                };
                                load(&a.id)
                                    .cmp(&load(&b.id))
                                    .then(at.total_cmp(bt))
                                    .then(a.id.cmp(&b.id))
                            });
                        let Some((objective, _)) = best else {
                            continue;
                        };
                        self.assigned.insert(unit.id, objective.id.clone());
                        objective
                    }
                };
                // A squad heading far for its objective stops for the
                // nearest free carrier to collect it (D17).
                let heading = self.heading(rules, entry, unit, objective.center);
                if self.worth_riding(frame, rules, entry, unit, heading)
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
                if busy.is_some() {
                    continue;
                }
                if rules.catalog.get(unit.kind).has_role("recon") {
                    let goal = recon_post(unit, &self.tuning, objective.center, entry.center);
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
                let share = self.tuning.supply_share;
                [
                    entry.center[0] + share * (center[0] / combat.len() as f64 - entry.center[0]),
                    entry.center[1] + share * (center[1] / combat.len() as f64 - entry.center[1]),
                ]
            };
            for truck in own
                .iter()
                .filter(|u| rules.catalog.get(u.kind).has_role("logistics"))
            {
                let here = [truck.position[0], truck.position[1]];
                let slack = self.tuning.truck_slack_m;
                if distance(here, support) > slack {
                    if truck
                        .goal
                        .is_some_and(|old| distance(old, support) <= slack)
                    {
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
                    if distance(here, goal) > slack {
                        if truck.goal.is_none_or(|old| distance(old, goal) > slack) {
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

fn distance3(a: [f64; 3], b: [f64; 3]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

fn unit_can_engage_target(rules: &Rules, kind: TypeIndex, target: TypeIndex) -> bool {
    let target = rules.catalog.get(target);
    let (layer, class) = (target.mobility.layer(), target.target_class());
    unit_weapons(rules, kind).any(|weapon| weapon.engages(layer, Some(class)).is_some())
}

/// The nearest enemy to `from` that the side identifies on the ground and
/// `kind` can engage: where a ride toward the fight goes.
fn nearest_ground_target<'a>(
    frame: &'a ObservationFrame,
    rules: &Rules,
    kind: TypeIndex,
    from: [f64; 2],
) -> Option<&'a contract::observation::IdentifiedUnit> {
    frame
        .identified
        .iter()
        .filter(|t| rules.catalog.get(t.kind).mobility.layer() == AltitudeLayer::Ground)
        .filter(|t| unit_can_engage_target(rules, kind, t.kind))
        .min_by(|a, b| {
            let at = |t: &contract::observation::IdentifiedUnit| {
                distance(from, [t.position[0], t.position[1]])
            };
            at(a).total_cmp(&at(b)).then(a.id.cmp(&b.id))
        })
}

/// The weapon rows `kind`'s mounts carry.
fn unit_weapons(rules: &Rules, kind: TypeIndex) -> impl Iterator<Item = &WeaponDefinition> {
    rules
        .catalog
        .mounts(kind)
        .iter()
        .flat_map(|mount| &mount.def.weapons)
        .filter_map(|weapon| rules.weapons.get(weapon))
}

/// Whether `weapon` engages an aircraft in `layer`, of either flight.
fn engages_aircraft(weapon: &WeaponDefinition, layer: AltitudeLayer) -> bool {
    [TargetClass::AirRotor, TargetClass::AirFixedWing]
        .into_iter()
        .any(|class| weapon.engages(layer, Some(class)).is_some())
}

/// A ground point is eligible only when one mounted indirect weapon can
/// reach it. Separate envelopes must never be merged across a dead zone.
fn artillery_in_range(
    unit: &contract::observation::OwnUnit,
    rules: &Rules,
    point: [f64; 3],
) -> bool {
    let range = distance3(unit.position, point);
    rules
        .catalog
        .mounts(unit.kind)
        .iter()
        .flat_map(|mount| &mount.def.weapons)
        .filter_map(|id| rules.weapons.get(id))
        .filter(|weapon| weapon.ballistics.trajectory == contract::ballistics::Trajectory::Indirect)
        .any(|weapon| {
            weapon
                .against(AltitudeLayer::Ground)
                .is_some_and(|fire| range >= weapon.min_range_m && range <= fire.range_m)
        })
}

/// Contacts have no emitter identity or motion. Retained sightings have a
/// side-scoped observed handle, so they can predict only their own last view.
fn artillery_ground_guess(
    unit: &contract::observation::OwnUnit,
    frame: &ObservationFrame,
    rules: &Rules,
    tuning: &Tuning,
    sightings: &BTreeMap<ObservedTargetId, ArtillerySighting>,
) -> Option<[f64; 3]> {
    let horizon = Tuning::ticks(tuning.sighting_horizon_s, rules.tick_hz);
    let contacts = frame
        .contacts
        .iter()
        .filter(|contact| {
            contact.layer == AltitudeLayer::Ground
                && !contact.aloft
                && contact.evidence_tick <= frame.tick
                && frame.tick <= contact.expires_tick
                && frame.tick - contact.evidence_tick <= horizon
        })
        .filter_map(|contact| {
            let angle = f64::from(contact.id.0) * 2.399963229728653;
            let bias = contact.radius.min(tuning.aim_cap_m) * tuning.contact_aim_share;
            let point = [
                contact.center[0] + libm::cos(angle) * bias,
                contact.center[1] + libm::sin(angle) * bias,
                contact.center[2],
            ];
            // Reverse the uncertain displacement at a range edge; a report
            // cannot turn into an exact emitter fix merely to make it fire.
            let opposite = [
                contact.center[0] - libm::cos(angle) * bias,
                contact.center[1] - libm::sin(angle) * bias,
                contact.center[2],
            ];
            [point, opposite]
                .into_iter()
                .find(|point| artillery_in_range(unit, rules, *point))
                .map(|point| (point, contact.id.0))
        });
    let predicted = sightings
        .iter()
        .filter(|(id, seen)| {
            !frame.identified.iter().any(|target| target.id == **id)
                && seen.tick <= frame.tick
                && frame.tick - seen.tick <= horizon
        })
        .filter_map(|(id, seen)| {
            let seconds = (frame.tick - seen.tick) as f64 / f64::from(rules.tick_hz);
            let point = [
                seen.position[0] + seen.velocity[0] * seconds,
                seen.position[1] + seen.velocity[1] * seconds,
                seen.position[2],
            ];
            artillery_in_range(unit, rules, point).then_some((point, id.0))
        });
    contacts
        .chain(predicted)
        .min_by(|(a, ai), (b, bi)| {
            distance3(unit.position, *a)
                .total_cmp(&distance3(unit.position, *b))
                .then(ai.cmp(bi))
        })
        .map(|(point, _)| point)
}

/// Public enemy-held or contested objectives support a fallible route guess.
/// Neutral empty flags alone reveal no enemy presence and do not justify fire.
fn artillery_objective_guess(
    unit: &contract::observation::OwnUnit,
    side: Side,
    entry: &EntrySite,
    frame: &ObservationFrame,
    rules: &Rules,
    tuning: &Tuning,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> Option<[f64; 3]> {
    frame
        .skirmish
        .as_ref()?
        .objectives
        .iter()
        .filter(|objective| {
            objective.owner.is_some_and(|owner| owner != side)
                || objective.contested
                || objective.capturing.is_some_and(|owner| owner != side)
        })
        .filter_map(|objective| {
            // Aim at the far approach to the objective from our public entry,
            // never at a hidden occupant or a guaranteed centre-point hit.
            let dx = objective.center[0] - entry.center[0];
            let dy = objective.center[1] - entry.center[1];
            let length = libm::hypot(dx, dy);
            if length <= 0.0 {
                return None;
            }
            let offset = objective.radius_m.min(tuning.aim_cap_m) * tuning.objective_aim_share;
            let point = [
                objective.center[0] + dx / length * offset,
                objective.center[1] + dy / length * offset,
                unit.position[2],
            ];
            if !artillery_in_range(unit, rules, point) {
                return None;
            }
            route_time(unit.id, [point[0], point[1]])
                .filter(|time| time.is_finite() && *time >= 0.0)
                .map(|time| (point, time, &objective.id))
        })
        .min_by(|(_, at, ai), (_, bt, bi)| at.total_cmp(bt).then(ai.cmp(bi)))
        .map(|(point, _, _)| point)
}

fn recon_post(
    unit: &contract::observation::OwnUnit,
    tuning: &Tuning,
    objective: [f64; 2],
    entry: [f64; 2],
) -> [f64; 2] {
    let dx = objective[0] - entry[0];
    let dy = objective[1] - entry[1];
    let approach = libm::hypot(dx, dy).max(1.0);
    let range = unit.sight.range.max(1.0);
    let standoff = (range * tuning.recon_sight_share).min(approach * tuning.recon_approach_share);
    [
        objective[0] - dx / approach * standoff,
        objective[1] - dy / approach * standoff,
    ]
}

/// Whether `kind` flies on wings (its flight model), whatever its layer.
pub fn is_jet(rules: &Rules, kind: TypeIndex) -> bool {
    rules.catalog.get(kind).mobility.flight() == Some(contract::catalog::FlightModel::FixedWing)
}

/// Whether one of `kind`'s weapons engages an aircraft, of either flight,
/// in `layer`; a row that only shoots down missiles there does not.
pub fn unit_engages_aircraft_in(rules: &Rules, kind: TypeIndex, layer: AltitudeLayer) -> bool {
    unit_weapons(rules, kind).any(|weapon| engages_aircraft(weapon, layer))
}

/// The best card of `eligible` against aircraft in `layer`, by purchase
/// score; a tie goes to the smaller unit id.
fn best_air_defender(
    rules: &Rules,
    tuning: &Tuning,
    eligible: &[TypeIndex],
    draw_unit: f64,
    layer: AltitudeLayer,
) -> Option<TypeIndex> {
    let score = |kind| candidate_score(rules, tuning, kind, draw_unit, Some(layer));
    eligible
        .iter()
        .copied()
        .filter(|&kind| unit_engages_aircraft_in(rules, kind, layer))
        .max_by(|&a, &b| {
            score(a)
                .total_cmp(&score(b))
                .then_with(|| rules.catalog.id(b).cmp(rules.catalog.id(a)))
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
    tuning: &Tuning,
    kind: TypeIndex,
    draw_unit: f64,
    demand: Option<AltitudeLayer>,
) -> f64 {
    let unit = rules.catalog.get(kind);
    let capability = demand.map_or(0.0, |layer| {
        if unit_engages_aircraft_in(rules, kind, layer) {
            10_000.0
        } else {
            -10_000.0
        }
    });
    let affordability_bias = 1_000.0 - f64::from(unit.cost);
    let weapon_value = demand.map_or(0.0, |layer| {
        unit_weapons(rules, kind)
            .filter(|weapon| engages_aircraft(weapon, layer))
            .map(|weapon| weapon.damage * weapon.ballistics.range_m)
            .fold(0.0, f64::max)
            / f64::from(unit.cost.max(1))
    });
    let noise = (contract::random::mix(u64::from(kind.0) ^ draw_unit.to_bits()) as f64
        / u64::MAX as f64
        - 0.5)
        * tuning.purchase_noise;
    capability + affordability_bias + weapon_value + noise
}

#[cfg(test)]
mod tests;
