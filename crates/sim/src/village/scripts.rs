//! Blue's tactical comparison scripts (encounter.md). Each is a small player
//! stand-in: it reads only blue's observation and what every player knows
//! (the static map and the objective), and issues ordinary commands through
//! the same path as input, so a run records like any played battle.
use std::collections::BTreeMap;

use contract::catalog::TypeIndex;
use contract::command::{Order, RoutePolicy, TargetRef};
use contract::ids::UnitId;
use contract::observation::{ContactSource, ObservationFrame, OwnUnit, ServiceStatus};
use contract::scenario::{Rules, ScenarioDefinition};

/// South of the ridge, clear of the forest: a direct line to every building,
/// and beyond what an AT team hidden in the forest can see.
const BOMBARD: [[f64; 2]; 2] = [[330.0, 830.0], [330.0, 860.0]];
/// Out of sight of the village's garrisons while the scout looks for armour.
const HOLD: [[f64; 2]; 2] = [[240.0, 810.0], [240.0, 850.0]];
/// The forest edge where an ambush would hide, cleared by infantry.
const FOREST_EDGE: [f64; 2] = [690.0, 840.0];
/// Just beyond the garrisons' rifle reach, looking into the village.
const SCOUT_POST: [f64; 2] = [360.0, 720.0];
const SUPPLY_POST: [f64; 2] = [300.0, 800.0];

/// Without armour sighted, the bombardment starts this late; with armour
/// under attack but still seen, this long after the attack.
const ARMOUR_WAIT_S: u64 = 90;
const ARMOUR_FIGHT_S: u64 = 60;
/// Bombardment: the first building this long after the tanks leave, then
/// one building every `SHELL_EVERY_S`; the push this long after they leave.
const FIRST_SHELL_S: u64 = 20;
const SHELL_EVERY_S: u64 = 40;
const PUSH_S: u64 = 150;
/// How often the tanks shell or sweep while the village is contested, and
/// how far from the centre a sweep reaches.
const RESHELL_S: u64 = 30;
const SWEEP_M: f64 = 70.0;
/// A firing area this close to a tank is fire at the tanks.
const CUE_RANGE_M: f64 = 400.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plan {
    /// Tanks drive straight down the road first; no scouting, no support.
    UnsupportedPush,
    /// Scout from cover and kill the armour it finds, shell the buildings
    /// from the southern flank while infantry clears the forest edge, push
    /// in together; the hurt rotate through the deployed supply and rejoin.
    ScoutSuppressFlank,
    /// Tanks drive down the road and fall back to the start `delay_s` after
    /// the first incoming-fire cue blue can legally observe (never the hidden
    /// launch).
    AmbushRetreat { delay_s: f64 },
}

impl Plan {
    /// A plan by its report name (`village_report`'s), for hosts that pick one
    /// by string. The ambush retreats take a delay and have no name here.
    pub fn named(name: &str) -> Option<Plan> {
        match name {
            "unsupported-road-push" => Some(Plan::UnsupportedPush),
            "scout-suppress-flank" => Some(Plan::ScoutSuppressFlank),
            _ => None,
        }
    }
}

/// A script's own memory: when it did what.
#[derive(Clone, Debug)]
pub struct Script {
    plan: Plan,
    /// The objective's centre and radius, and the map's buildings: public.
    zone: ([f64; 2], f64),
    buildings: Vec<[f64; 2]>,
    gesture: u64,
    started: bool,
    // Ambush retreat.
    /// Each tank's health last tick: a drop is a hit it felt.
    felt: BTreeMap<u32, f64>,
    first_cue: Option<u64>,
    retreated: bool,
    // Scout, suppress, flank.
    armour_attacked: Option<u64>,
    /// When the tanks left for the bombardment, and the next building.
    shelling: Option<(u64, usize)>,
    pushed: bool,
    reshelled: u64,
    /// Units sent back to the supply, waiting to rejoin, since when.
    resting: BTreeMap<u32, u64>,
    rejoined: u32,
}

impl Script {
    pub fn new(plan: Plan, setup: &ScenarioDefinition) -> Self {
        let encounter = setup.encounter.as_ref().expect("a scripted encounter");
        Script {
            plan,
            zone: (
                encounter.success_zone_center,
                encounter.success_zone_radius_m,
            ),
            buildings: setup
                .map
                .buildings
                .iter()
                .filter(|b| setup.rules.catalog.props().by_id(&b.kind).body.garrison)
                .map(|b| {
                    let owner = b
                        .parts
                        .iter()
                        .find(|p| p.prop == b.owner)
                        .expect("prepared building has an owner part");
                    b.geometry
                        .parts
                        .iter()
                        .find(|p| p.id == owner.part)
                        .expect("prepared owner has physical geometry")
                        .center
                })
                .collect(),
            gesture: 5_000_000,
            started: false,
            felt: BTreeMap::new(),
            first_cue: None,
            retreated: false,
            armour_attacked: None,
            shelling: None,
            pushed: false,
            reshelled: 0,
            resting: BTreeMap::new(),
            rejoined: 0,
        }
    }

    /// How many hurt units were served and sent back into the fight.
    pub fn rejoined(&self) -> u32 {
        self.rejoined
    }

    fn token(&mut self) -> u64 {
        self.gesture += 1;
        self.gesture
    }

    // Order builders: nothing to order with no units.
    fn mv(&mut self, units: Vec<UnitId>, goal: [f64; 2]) -> Option<Order> {
        (!units.is_empty()).then(|| Order::Move {
            units,
            gesture: self.token(),
            goal,
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        })
    }

    fn attack_move(&mut self, units: Vec<UnitId>, goal: [f64; 2]) -> Option<Order> {
        (!units.is_empty()).then(|| Order::AttackMove {
            units,
            gesture: self.token(),
            goal,
        })
    }

    fn attack(units: Vec<UnitId>, target: TargetRef) -> Option<Order> {
        (!units.is_empty()).then_some(Order::Attack { units, target })
    }

    /// This tick's orders (none queued).
    pub fn orders(&mut self, frame: &ObservationFrame, rules: &Rules) -> Vec<Order> {
        match self.plan {
            Plan::UnsupportedPush => self.unsupported(frame, rules),
            Plan::AmbushRetreat { delay_s } => self.ambush(frame, rules, delay_s),
            Plan::ScoutSuppressFlank => self.supported(frame, rules),
        }
    }

    fn unsupported(&mut self, frame: &ObservationFrame, rules: &Rules) -> Vec<Order> {
        if self.started {
            return Vec::new();
        }
        self.started = true;
        let village = self.zone.0;
        self.attack_move(of(frame, rules, "mbt"), village)
            .into_iter()
            .collect()
    }

    fn ambush(&mut self, frame: &ObservationFrame, rules: &Rules, delay_s: f64) -> Vec<Order> {
        let tick = frame.tick;
        let tanks = of(frame, rules, "mbt");
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            let village = self.zone.0;
            out.extend(self.mv(tanks.clone(), village));
        }
        // The first legal cue of fire at the tanks: a hit one of them felt,
        // or a fresh firing area near them (a launch reveals itself map-wide).
        let mut hit = false;
        for u in frame.own.iter().filter(|u| is(rules, u.kind, "mbt")) {
            hit |= self.felt.insert(u.id.0, u.hp).is_some_and(|was| u.hp < was);
        }
        let near = |c: [f64; 2]| {
            frame.own.iter().any(|u| {
                is(rules, u.kind, "mbt")
                    && libm::hypot(u.position[0] - c[0], u.position[1] - c[1]) <= CUE_RANGE_M
            })
        };
        let fired = frame.contacts.iter().any(|c| {
            c.source == ContactSource::Firing && c.evidence_tick == tick && near(c.center)
        });
        if (fired || hit) && self.first_cue.is_none() {
            self.first_cue = Some(tick);
        }
        let due = self
            .first_cue
            .map(|t| t + (delay_s * rules.tick_hz as f64).round() as u64);
        if !self.retreated && due.is_some_and(|d| tick >= d) {
            self.retreated = true;
            for (k, t) in tanks.iter().enumerate() {
                out.extend(self.mv(vec![*t], HOLD[k % 2]));
            }
        }
        out
    }

    fn supported(&mut self, frame: &ObservationFrame, rules: &Rules) -> Vec<Order> {
        let tick = frame.tick;
        let s = |seconds: u64| seconds * rules.tick_hz as u64;
        let (village, zone_m) = self.zone;
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            out.extend(self.mv(of(frame, rules, "recon"), SCOUT_POST));
            out.extend(self.mv(of(frame, rules, "logistics"), SUPPLY_POST));
            for (k, t) in of(frame, rules, "mbt").into_iter().enumerate() {
                out.extend(self.mv(vec![t], HOLD[k % 2]));
            }
        }
        let resting = &self.resting;
        let ready: Vec<UnitId> = of(frame, rules, "mbt")
            .into_iter()
            .filter(|t| !resting.contains_key(&t.0))
            .collect();
        // First the enemy armour the scout finds: both tanks on it at once.
        let armour = frame.identified.iter().find(|e| is(rules, e.kind, "mbt"));
        if let (Some(tank), None) = (armour, self.armour_attacked) {
            if let Some(order) = Self::attack(ready.clone(), TargetRef::Identified { id: tank.id })
            {
                self.armour_attacked = Some(tick);
                out.push(order);
            }
        }
        // Then the bombardment: once the armour is gone or has been fought
        // for a while, or when none was found in time.
        let bombard = match self.armour_attacked {
            Some(at) => armour.is_none() || tick >= at + s(ARMOUR_FIGHT_S),
            None => armour.is_none() && tick >= s(ARMOUR_WAIT_S),
        };
        if self.shelling.is_none() && bombard && !ready.is_empty() {
            self.shelling = Some((tick, 0));
            for (k, t) in ready.iter().enumerate() {
                out.extend(self.mv(vec![*t], BOMBARD[k % 2]));
            }
        }
        // Bring the buildings down in turn with ground attacks (HE), the
        // occupants spilling out suppressed; infantry clears the forest edge.
        if let Some((left, next)) = self.shelling {
            let due = left + s(FIRST_SHELL_S + SHELL_EVERY_S * next as u64);
            if next < self.buildings.len() && tick >= due {
                let b = self.buildings[next];
                let point = [b[0], b[1], 0.0];
                if let Some(order) = Self::attack(ready.clone(), TargetRef::Ground { point }) {
                    out.push(order);
                    self.shelling = Some((left, next + 1));
                    if next == 1 {
                        let line = [of(frame, rules, "infantry"), of(frame, rules, "at")].concat();
                        out.extend(self.attack_move(line, FOREST_EDGE));
                    }
                }
            }
            if !self.pushed && tick >= left + s(PUSH_S) {
                self.pushed = true;
                self.reshelled = tick;
                let all = self.fighters(frame, rules);
                out.extend(self.attack_move(all, village));
            }
        }
        // In the village but not holding it: someone still contests it (the
        // published hold status says so, never who or where). Shell the next
        // building standing, or, with every building down, sweep the zone to
        // flush whoever hides in the ruins.
        let arrived = frame.own.iter().any(|u| {
            !is(rules, u.kind, "logistics")
                && libm::hypot(u.position[0] - village[0], u.position[1] - village[1]) <= zone_m
        });
        let contested = arrived && frame.encounter.is_some_and(|e| e.held_s == 0.0);
        if self.pushed && contested && tick >= self.reshelled + s(RESHELL_S) {
            self.reshelled = tick;
            let turn = (tick / s(RESHELL_S)) as usize;
            let standing: Vec<[f64; 2]> = self
                .buildings
                .iter()
                .copied()
                .filter(|b| {
                    !frame.known_props.iter().any(|p| {
                        p.replaces.is_some()
                            && libm::hypot(p.center[0] - b[0], p.center[1] - b[1]) < 5.0
                    })
                })
                .collect();
            if let Some(b) = standing.get(turn % standing.len().max(1)) {
                let point = [b[0], b[1], 0.0];
                out.extend(Self::attack(ready.clone(), TargetRef::Ground { point }));
            } else {
                let a = turn as f64 * std::f64::consts::FRAC_PI_2;
                let goal = [
                    village[0] + SWEEP_M * libm::cos(a),
                    village[1] + SWEEP_M * libm::sin(a),
                ];
                let all = self.fighters(frame, rules);
                out.extend(self.attack_move(all, goal));
            }
        }
        // Rotation: the hurt fall back to the supply and rejoin when whole.
        for u in &frame.own {
            if is(rules, u.kind, "logistics") || is(rules, u.kind, "recon") {
                continue;
            }
            match self.resting.get(&u.id.0) {
                None if hurt(u, rules) => {
                    self.resting.insert(u.id.0, tick);
                    out.extend(self.mv(vec![u.id], near(SUPPLY_POST, u.id.0)));
                }
                Some(&since) if u.service == ServiceStatus::Full && tick > since + s(10) => {
                    self.resting.remove(&u.id.0);
                    self.rejoined += 1;
                    out.extend(self.attack_move(vec![u.id], near(village, u.id.0)));
                }
                _ => {}
            }
        }
        out
    }

    /// Every combat unit not resting at the supply.
    fn fighters(&self, frame: &ObservationFrame, rules: &Rules) -> Vec<UnitId> {
        frame
            .own
            .iter()
            .filter(|u| {
                !(is(rules, u.kind, "logistics")
                    || is(rules, u.kind, "recon")
                    || self.resting.contains_key(&u.id.0))
            })
            .map(|u| u.id)
            .collect()
    }
}

/// Whether the unit type `kind` carries `role`.
fn is(rules: &Rules, kind: TypeIndex, role: &str) -> bool {
    rules.catalog.get(kind).has_role(role)
}

/// This side's units with one role.
fn of(frame: &ObservationFrame, rules: &Rules, role: &str) -> Vec<UnitId> {
    frame
        .own
        .iter()
        .filter(|u| is(rules, u.kind, role))
        .map(|u| u.id)
        .collect()
}

/// Badly hurt: a tank under half health, a squad under half strength.
fn hurt(u: &OwnUnit, rules: &Rules) -> bool {
    let t = rules.catalog.get(u.kind);
    match t.hull() {
        Some(hull) => t.has_role("mbt") && u.hp < 0.5 * hull.hp,
        None => (u.members.len() as f64) < 0.5 * t.squad_size() as f64,
    }
}

/// Spread per-unit destinations around a shared point.
fn near(p: [f64; 2], id: u32) -> [f64; 2] {
    let a = id as f64 * 1.3;
    [p[0] + 25.0 * libm::cos(a), p[1] + 25.0 * libm::sin(a)]
}
