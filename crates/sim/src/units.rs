//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::{BTreeSet, VecDeque};

use contract::command::{Engagement, MoveDirection, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::{Armor, Face, HealthRules, MoverBody, PushClass, Rules, UnitKind};

use crate::digest::Digest;
use crate::garrison::{Garrison, Phase};
use crate::math::{Obb2, Rotation, V2, V3};
use crate::navigation::Mobility;
use crate::weapons::{Mount, Target};
use crate::world::PropId;

/// Infantry path clearance: a squad threads gaps a vehicle cannot.
const INFANTRY_HALF_WIDTH_M: f64 = 0.5;

/// One soldier: a body of his own (L4–L6). He stands on the ground at his
/// own position, or at his building slot while garrisoned; nothing places
/// him relative to his squad.
#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    pub position: V3,
    /// Ground velocity over the last tick.
    pub velocity: V2,
    pub hp: f64,
    /// Where the soldier fell: a permanent record that blocks nothing (M06).
    pub corpse: Option<Fallen>,
    /// His place in the squad's arrangement where the current move ends (D1).
    pub spot: Option<V2>,
    /// The waypoint of the squad's corridor he is walking toward.
    pub leg: usize,
    /// His lane: metres left of the corridor he walks (Q6).
    pub lateral: f64,
    /// Where his pace swing starts for this move, a share of its period.
    pub pace: f64,
    /// The tick he sets off on this move (the stagger).
    pub start: u64,
    /// His own route on the exact bodies: the final stretch to his spot, or
    /// back to the corridor (`movement::final_leg`).
    pub path: Vec<V2>,
    /// The side's knowledge revision his own route was last judged against,
    /// and the tick he last planned it.
    pub path_revision: u64,
    pub planned_at: u64,
    /// Where he takes up position while his squad holds (at rest or
    /// halted): cover the squad re-resolved, or a step out to fire (D3, D5).
    pub post: Option<V2>,
    /// The cover tier his spot or post gives, as resolved (D2+ publishes it).
    pub cover: Option<crate::cover::Tier>,
}

impl Soldier {
    pub fn new(id: u32, position: V3, hp: f64) -> Self {
        Soldier {
            id,
            position,
            velocity: V2::default(),
            hp,
            corpse: None,
            spot: None,
            leg: 0,
            lateral: 0.0,
            pace: 0.0,
            start: 0,
            path: Vec::new(),
            path_revision: 0,
            planned_at: 0,
            post: None,
            cover: None,
        }
    }
}

/// A fallen soldier's record: where, and the squad's heading at the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fallen {
    pub at: V3,
    pub yaw: f64,
}

impl Soldier {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    /// The one way a soldier dies: no health left (a lethal hit's overkill is
    /// kept) and a permanent record where it fell, facing `yaw`.
    pub fn fall(&mut self, at: V3, yaw: f64) {
        if self.hp > 0.0 {
            self.hp = 0.0;
        }
        self.corpse = Some(Fallen { at, yaw });
    }
}

#[derive(Clone, Debug)]
pub struct MoveOrder {
    pub destination: V2,
    pub policy: RoutePolicy,
    pub gesture: u64,
    /// A reverse move backs along the route, facing held (Q31).
    pub direction: MoveDirection,
    /// A right-drag's facing once there (Q9); `None`: the way it travels.
    pub facing: Option<f64>,
}

#[derive(Clone, Debug)]
pub enum UnitOrder {
    Move(MoveOrder),
    /// Move, halting while any weapon engages (W16).
    AttackMove(MoveOrder),
    /// Focus fire; pursue an identified target to regain a firing position,
    /// or its last reported place once identification lapses (W17).
    Attack {
        target: Target,
        last_known: Option<V2>,
    },
    /// Walk to `approach` beside the building, then enter it (L08).
    Garrison {
        building: PropId,
        approach: V2,
    },
    /// Leave the current building.
    Exit,
}

impl UnitOrder {
    pub fn movement(&self) -> Option<&MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            _ => None,
        }
    }

    pub fn movement_mut(&mut self) -> Option<&mut MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub side: Side,
    pub kind: UnitKind,
    pub position: V3,
    pub yaw: f64,
    pub mobility: Mobility,
    /// Vehicle hull half extents (length, width, height); infantry use members.
    pub hull: Option<V3>,
    pub members: Vec<Soldier>,
    pub orders: VecDeque<UnitOrder>,
    /// Where an attack order is currently pursuing to, set each tick.
    pub pursuit: Option<V2>,
    /// The goal the current route was planned to.
    pub planned_goal: Option<V2>,
    /// Remaining waypoints of the current order, once planned.
    pub route: Option<Vec<V2>>,
    /// Where a squad's corridor runs from to its first remaining waypoint:
    /// the planning start, then the last waypoint every soldier passed.
    pub route_from: V2,
    pub state: MoveState,
    pub blocker: Option<UnitId>,
    /// Side knowledge revision the current route (or block) was judged against.
    pub planned_revision: u64,
    /// Failure-to-advance watch: best distance to the next waypoint and when.
    pub progress: (f64, u64),
    pub engagement: Engagement,
    pub mounts: Vec<Mount>,
    /// Enemies that attacked this unit, whom Return fire only may answer while
    /// the side still knows them (W13).
    pub attackers: BTreeSet<UnitId>,
    /// Last tick's weapon conclusions: attack-move halting and attack pursuit.
    pub reach: crate::weapons::Reach,
    /// Vehicle health; infantry health lives on each soldier.
    pub hp: f64,
    /// Infantry suppression in [0, 1] and the tick it last grew (P14).
    pub suppression: f64,
    pub suppressed_at: u64,
    /// Setup progress for units that deploy in place (L01); `None` otherwise.
    pub deployment: Option<crate::deployment::Deployment>,
    /// The squad's building while entering, inside or leaving it (L08).
    pub garrison: Option<Garrison>,
    /// A supply vehicle's remaining stock (L05); `None` for every other unit.
    pub stock: Option<u32>,
    /// Service received: progress toward the next item, and why or why not.
    pub progress_service: crate::supply::Progress,
    pub service: contract::observation::ServiceStatus,
    /// The bearing this unit looks along this tick, taken before fire
    /// (`sight::snapshot`): what spotting, the sweep and the publication read.
    pub sight_forward: f64,
    /// A squad's cover: what it was last resolved against, and when (Q11).
    pub cover: crate::cover::Watch,
    /// A wheeled vehicle's three-point turn in progress: the leg it drives
    /// against its order's direction (Q29).
    pub manoeuvre: Option<crate::movement::Manoeuvre>,
    /// The vehicle drove backwards this tick: an ordered reverse move or a
    /// three-point turn's reversing leg (the reverse whine's cue).
    pub reversing: bool,
    /// A tracked vehicle's ordered facing (Q9), still to pivot to at rest.
    pub turn_to: Option<f64>,
}

pub fn mobility(kind: UnitKind, rules: &Rules) -> Mobility {
    let m = &rules.movement;
    let vehicle = |off_road_mps, road_mps| Mobility {
        off_road_mps,
        road_mps,
        forest_multiplier: m.forest_vehicle_multiplier,
        half_width_m: hull(kind, rules).expect("a vehicle has a hull").y,
        class: MoverClass::Vehicle,
        push: body(kind, rules).push_class.unwrap_or(PushClass::None),
        drive: Some(drive(body(kind, rules))),
    };
    match kind {
        UnitKind::Rifle | UnitKind::Recon | UnitKind::At => Mobility {
            off_road_mps: m.infantry_mps,
            road_mps: m.infantry_mps * m.infantry_road_multiplier,
            forest_multiplier: m.forest_infantry_multiplier,
            half_width_m: INFANTRY_HALF_WIDTH_M,
            class: MoverClass::Infantry,
            push: PushClass::None,
            drive: None,
        },
        UnitKind::Tank => vehicle(m.tank_mps, m.tank_road_mps),
        UnitKind::Supply => vehicle(m.supply_mps, m.supply_road_mps),
        UnitKind::Jeep => vehicle(m.jeep_mps, m.jeep_road_mps),
    }
}

/// A vehicle's drive from its body row (Q29, Q30); `validate_bodies` has
/// checked every column it reads.
fn drive(b: &MoverBody) -> crate::navigation::Drive {
    let tracked = b.drive == Some(contract::scenario::DriveType::Tracked);
    crate::navigation::Drive {
        tracked,
        turn_rad_s: b.turn_deg_s.unwrap_or(0.0).to_radians(),
        radius_m: if tracked {
            0.0
        } else {
            b.turning_radius_m.unwrap_or(0.0)
        },
        reverse_fraction: b.reverse_speed_fraction.unwrap_or(0.0),
    }
}

/// A mover's body row (Q19): weight, push class, the wreck it leaves and
/// its loudness. `battle` checks every kind has one.
pub fn body(kind: UnitKind, rules: &Rules) -> &MoverBody {
    rules
        .bodies
        .get(&kind)
        .unwrap_or_else(|| panic!("bodies.{kind:?} is missing"))
}

/// Body tables the simulation can honour: every prop kind has a row (a
/// battle can leave any wreck anywhere), every unit kind a mover row, and
/// every vehicle a weight class, a push class and a wreck row; a transient
/// row lives a positive time.
pub fn validate_bodies(rules: &Rules) {
    for kind in contract::map::PropKind::ALL {
        let row = rules
            .props
            .get(&kind)
            .unwrap_or_else(|| panic!("props.{kind:?}: every prop kind needs a body row"));
        assert!(
            row.lifetime_s.is_none_or(|s| s > 0.0),
            "props.{kind:?}.lifetime_s must be positive"
        );
        // Integrity (Q17): `hp` and `destroyed` together, and a destroyed
        // state that ends: each `into` names a row, never back up the chain.
        assert_eq!(
            row.hp.is_some(),
            row.destroyed.is_some(),
            "props.{kind:?}: hp and destroyed go together"
        );
        assert!(
            row.hp.is_none_or(|h| h > 0.0) && (0.0..=1.0).contains(&row.armor),
            "props.{kind:?}: hp must be positive and armor within [0, 1]"
        );
        assert!(
            row.destroyed != Some(contract::scenario::Destroyed::Cleared)
                || crate::world::topples(kind),
            "props.{kind:?}: only a tree's destroyed state is cleared ground"
        );
        let mut next = row.destroyed;
        for _ in 0..=contract::map::PropKind::ALL.len() {
            match next {
                Some(contract::scenario::Destroyed::Into {
                    kind: into,
                    height_m,
                }) => {
                    assert!(height_m > 0.0, "props.{kind:?}.destroyed.into.height_m");
                    next = rules
                        .props
                        .get(&into)
                        .unwrap_or_else(|| panic!("props.{kind:?}: {into:?} needs a body row"))
                        .destroyed;
                }
                _ => {
                    next = None;
                    break;
                }
            }
        }
        assert!(next.is_none(), "props.{kind:?}: its destroyed states loop");
    }
    for kind in [
        UnitKind::Rifle,
        UnitKind::Recon,
        UnitKind::At,
        UnitKind::Tank,
        UnitKind::Supply,
        UnitKind::Jeep,
    ] {
        let b = body(kind, rules);
        assert!(b.loudness_m >= 0.0, "bodies.{kind:?}.loudness_m");
        if hull(kind, rules).is_some() {
            assert!(
                b.weight_class.is_some() && b.push_class.is_some(),
                "bodies.{kind:?}: a vehicle needs a weight class and a push class"
            );
            assert!(
                b.wreck.is_some_and(|w| rules.props.contains_key(&w)),
                "bodies.{kind:?}: a vehicle needs a wreck with a body row"
            );
            assert!(
                b.drive.is_some() && b.turn_deg_s.is_some_and(|t| t > 0.0),
                "bodies.{kind:?}: a vehicle needs a drive and a positive turn_deg_s"
            );
            assert!(
                b.reverse_speed_fraction
                    .is_some_and(|f| f > 0.0 && f <= 1.0),
                "bodies.{kind:?}.reverse_speed_fraction must lie in (0, 1]"
            );
            assert!(
                b.drive != Some(contract::scenario::DriveType::Wheeled)
                    || b.turning_radius_m.is_some_and(|r| r > 0.0),
                "bodies.{kind:?}: a wheeled vehicle needs a positive turning_radius_m"
            );
        }
    }
}

pub fn hull(kind: UnitKind, rules: &Rules) -> Option<V3> {
    let h = match kind {
        UnitKind::Tank => rules.physics.tank_half_extents_m,
        UnitKind::Supply => rules.physics.supply_half_extents_m,
        UnitKind::Jeep => rules.physics.jeep_half_extents_m,
        _ => return None,
    };
    Some(crate::math::v3(h[0], h[1], h[2]))
}

pub fn cost(kind: UnitKind, rules: &Rules) -> u32 {
    let c = &rules.costs;
    match kind {
        UnitKind::Rifle => c.rifle,
        UnitKind::Recon => c.recon,
        UnitKind::At => c.at,
        UnitKind::Tank => c.tank,
        UnitKind::Supply => c.supply,
        UnitKind::Jeep => c.jeep,
    }
}

/// Authored vehicle health (0 for infantry, whose health is per soldier).
pub fn max_hp(kind: UnitKind, rules: &Rules) -> f64 {
    match kind {
        UnitKind::Tank => rules.health.tank,
        UnitKind::Supply => rules.health.supply,
        UnitKind::Jeep => rules.health.jeep,
        _ => 0.0,
    }
}

/// A vehicle kind's hull armour; `None` for infantry.
pub fn armor(kind: UnitKind, health: &HealthRules) -> Option<&Armor> {
    match kind {
        UnitKind::Tank => Some(&health.tank_armor),
        UnitKind::Supply => Some(&health.supply_armor),
        UnitKind::Jeep => Some(&health.jeep_armor),
        UnitKind::Rifle | UnitKind::Recon | UnitKind::At => None,
    }
}

pub fn squad_size(kind: UnitKind, rules: &Rules) -> u32 {
    match kind {
        UnitKind::Rifle => rules.health.rifle_squad_size,
        UnitKind::Recon => rules.health.recon_squad_size,
        UnitKind::At => rules.health.at_squad_size,
        UnitKind::Tank | UnitKind::Supply | UnitKind::Jeep => 0,
    }
}

impl Unit {
    pub fn is_vehicle(&self) -> bool {
        self.hull.is_some()
    }

    /// A squad stands where its living soldiers stand: their centroid. Called
    /// whenever soldiers move, arrive, fall or join; a vehicle is unchanged.
    pub fn settle(&mut self) {
        if self.is_vehicle() {
            return;
        }
        let (mut sum, mut n) = (V3::default(), 0.0);
        for s in self.members.iter().filter(|s| s.alive()) {
            sum = sum + s.position;
            n += 1.0;
        }
        if n > 0.0 {
            self.position = sum * (1.0 / n);
        }
    }

    /// At its building's perimeter slots (inside or leaving, not entering).
    pub fn garrisoned(&self) -> bool {
        self.garrison
            .as_ref()
            .is_some_and(|g| matches!(g.phase, Phase::Inside | Phase::Exiting(_)))
    }

    /// World positions of living members.
    pub fn member_positions(&self) -> impl Iterator<Item = V3> + '_ {
        self.members
            .iter()
            .filter(|s| s.alive())
            .map(|s| s.position)
    }

    /// A squad lives while any member does; a vehicle while its hull has health.
    pub fn alive(&self) -> bool {
        match self.hull {
            Some(_) => self.hp > 0.0,
            None => self.members.iter().any(|s| s.alive()),
        }
    }

    /// The hull's armour, for vehicles.
    pub fn armor<'a>(&self, health: &'a HealthRules) -> Option<&'a Armor> {
        armor(self.kind, health)
    }

    /// Fold the unit's complete carried state into `d`.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.id.0 as u64)
            .f64(self.position.x)
            .f64(self.position.y)
            .f64(self.position.z)
            .f64(self.yaw);
        d.u64(self.state as u64).u64(self.reversing as u64);
        d.opt_f64(self.turn_to);
        match self.manoeuvre {
            Some(m) => d.u64(1).f64(m.turn).f64(m.driven_m),
            None => d.u64(0),
        };
        d.u64(self.orders.len() as u64);
        for o in &self.orders {
            match o {
                UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                    d.u64(matches!(o, UnitOrder::Move(_)) as u64);
                    d.f64(m.destination.x)
                        .f64(m.destination.y)
                        .u64(m.policy as u64)
                        .u64(m.gesture)
                        .u64(m.direction as u64);
                    d.opt_f64(m.facing);
                }
                UnitOrder::Attack { target, last_known } => {
                    d.u64(2);
                    target.digest(d);
                    d.opt_v2(*last_known);
                }
                UnitOrder::Garrison { building, approach } => {
                    d.u64(3)
                        .u64(*building as u64)
                        .f64(approach.x)
                        .f64(approach.y);
                }
                UnitOrder::Exit => {
                    d.u64(4);
                }
            }
        }
        d.opt_v2(self.pursuit).opt_v2(self.planned_goal);
        d.u64(self.route.is_some() as u64);
        for p in self.route.iter().flatten() {
            d.f64(p.x).f64(p.y);
        }
        d.f64(self.route_from.x).f64(self.route_from.y);
        d.u64(self.blocker.is_some() as u64);
        if let Some(b) = self.blocker {
            d.u64(b.0 as u64);
        }
        d.u64(self.planned_revision)
            .f64(self.progress.0)
            .u64(self.progress.1);
        d.f64(self.hp).f64(self.suppression).u64(self.suppressed_at);
        d.u64(self.stock.map_or(u64::MAX, u64::from));
        let p = self.progress_service;
        d.f64(p.ammo_s)
            .f64(p.hp_s)
            .f64(p.soldier_s)
            .u64(self.service as u64);
        d.f64(self.sight_forward);
        self.cover.digest(d);
        crate::garrison::digest(self, d);
        d.u64(self.members.len() as u64);
        for s in &self.members {
            d.u64(s.id as u64)
                .f64(s.hp)
                .f64(s.position.x)
                .f64(s.position.y)
                .f64(s.position.z);
            d.f64(s.velocity.x).f64(s.velocity.y);
            d.opt_v2(s.spot).u64(s.leg as u64);
            d.f64(s.lateral).f64(s.pace).u64(s.start);
            d.u64(s.path.len() as u64);
            for p in &s.path {
                d.f64(p.x).f64(p.y);
            }
            d.u64(s.path_revision).u64(s.planned_at);
            d.opt_v2(s.post).u64(s.cover.map_or(u64::MAX, |t| t as u64));
            d.u64(s.corpse.is_some() as u64);
            if let Some(Fallen { at: p, yaw }) = s.corpse {
                d.f64(p.x).f64(p.y).f64(p.z).f64(yaw);
            }
        }
        d.u64(self.engagement as u64)
            .u64(self.reach.can_engage as u64)
            .u64(self.reach.needs_closer as u64);
        d.u64(self.attackers.len() as u64);
        for a in &self.attackers {
            d.u64(a.0 as u64);
        }
        d.u64(self.deployment.is_some() as u64);
        if let Some(dep) = &self.deployment {
            d.u64(dep.current as u64)
                .u64(dep.duration as u64)
                .u64(dep.stationary as u64);
        }
        d.u64(self.mounts.len() as u64);
        for m in &self.mounts {
            m.digest(d);
        }
    }

    /// The hull's ground footprint, for vehicles.
    pub fn hull_box(&self) -> Option<Obb2> {
        self.hull.map(|h| Obb2 {
            center: self.position.xy(),
            yaw: self.yaw,
            half: h.xy(),
        })
    }

    /// Distance from a point to the hull box (0 inside).
    pub fn hull_distance(&self, p: V3) -> f64 {
        self.hull_frame().distance(p)
    }

    /// The hull box at the unit's pose, for measuring many points against it.
    pub fn hull_frame(&self) -> HullFrame {
        let half = self.hull.expect("vehicle");
        HullFrame {
            center: self.position.xy(),
            mid_z: self.position.z + half.z,
            half,
            to_local: Rotation::new(-self.yaw),
        }
    }

    /// The hull face facing `p` (see [`face_toward`]) at the unit's pose.
    pub fn hull_face(&self, p: V3) -> Face {
        hull_face_at(self.position, self.yaw, self.hull.expect("vehicle"), p)
    }

    /// Radius of the unit's ground footprint, for traffic spacing and sensing
    /// reach (a garrison spans its building's perimeter).
    pub fn footprint_radius(&self) -> f64 {
        match self.hull {
            Some(h) => h.x.hypot(h.y),
            None => {
                self.member_positions()
                    .map(|p| (p.xy() - self.position.xy()).length())
                    .fold(0.0, f64::max)
                    + INFANTRY_HALF_WIDTH_M
            }
        }
    }

    /// Where movement should head now and by which policy: an ordered
    /// destination, an attack's pursuit point or a building's approach. A unit
    /// with a building (entering, inside or leaving) goes nowhere.
    pub fn movement_goal(&self) -> Option<(V2, RoutePolicy)> {
        if self.garrison.is_some() {
            return None;
        }
        match self.orders.front()? {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some((o.destination, o.policy)),
            UnitOrder::Attack { .. } => self.pursuit.map(|p| (p, RoutePolicy::Shortest)),
            UnitOrder::Garrison { approach, .. } => Some((*approach, RoutePolicy::Shortest)),
            UnitOrder::Exit => None,
        }
    }

    /// Which way the current order drives: a reverse move backs (Q31).
    pub fn direction(&self) -> MoveDirection {
        match self.orders.front() {
            Some(UnitOrder::Move(o)) if self.garrison.is_none() => o.direction,
            _ => MoveDirection::Forward,
        }
    }

    /// The movement gate: a deploying unit translates only when fully packed.
    pub fn may_translate(&self) -> bool {
        self.deployment
            .as_ref()
            .is_none_or(crate::deployment::Deployment::packed)
    }

    /// Attack-move pauses its advance while it can engage something.
    pub fn halted(&self) -> bool {
        matches!(self.orders.front(), Some(UnitOrder::AttackMove(_))) && self.reach.can_engage
    }

    pub fn attack_target(&self) -> Option<Target> {
        match self.orders.front()? {
            UnitOrder::Attack { target, .. } => Some(*target),
            _ => None,
        }
    }
}

/// A vehicle's hull box at one pose with its rotation worked out once, so a
/// caller measuring many points (the friendly-fire check samples a whole
/// trajectory against each friendly hull) pays one sine and cosine, not one
/// per point. Slice 29: leaving that hoist to the optimiser cost the
/// endurance battle 40% of its instructions when an unrelated change tipped
/// an inlining decision.
pub struct HullFrame {
    center: V2,
    /// Height of the box's middle.
    mid_z: f64,
    half: V3,
    to_local: Rotation,
}

impl HullFrame {
    /// Distance from `p` to the box (0 inside).
    pub fn distance(&self, p: V3) -> f64 {
        let d = self.to_local.apply(p.xy() - self.center);
        let dz = p.z - self.mid_z;
        let ex = (d.x.abs() - self.half.x).max(0.0);
        let ey = (d.y.abs() - self.half.y).max(0.0);
        let ez = (dz.abs() - self.half.z).max(0.0);
        (ex * ex + ey * ey + ez * ez).sqrt()
    }
}

/// The face of a hull with half extents `half`, standing on `base` and
/// heading `yaw`, that `p` lies beyond.
pub fn hull_face_at(base: V3, yaw: f64, half: V3, p: V3) -> Face {
    let d = (p.xy() - base.xy()).rotated(-yaw);
    face_toward(d.with_z(p.z - (base.z + half.z)), half)
}

/// The face of a box with half extents `half` (x forward) that a point at
/// `local` (box frame, from its centre) lies beyond: the roof when above more
/// than beside, otherwise front, rear or side by the dominant scaled axis (P10).
pub fn face_toward(local: V3, half: V3) -> Face {
    let (sx, sy, sz) = (local.x / half.x, local.y / half.y, local.z / half.z);
    if sz > sx.abs() && sz > sy.abs() {
        Face::Roof
    } else if sx.abs() >= sy.abs() {
        if sx > 0.0 {
            Face::Front
        } else {
            Face::Rear
        }
    } else {
        Face::Side
    }
}
