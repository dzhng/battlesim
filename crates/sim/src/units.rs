//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::{BTreeSet, VecDeque};

use contract::catalog::{Mobility as Moves, TypeIndex, UnitType};
use contract::command::{Engagement, MoveDirection, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::{Armor, Face, Rules};

use crate::digest::Digest;
use crate::garrison::{Garrison, Phase};
use crate::math::{Obb2, Rotation, V2, V3};
use crate::navigation::Mobility;
use crate::weapons::{Mount, Target};
use crate::world::PropId;

/// One soldier: a body of his own (L4–L6). He stands on the ground at his
/// own position, or at his building slot while garrisoned; nothing places
/// him relative to his squad.
#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    /// His place in his squad type's slots: which soldier kind he is and
    /// what he carries. Fixed when he joins.
    pub slot: usize,
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
    /// The lean point claimed with his spot or post, round the tall cover
    /// he fires past.
    pub lean: Option<crate::lean::Lean>,
    /// He leans out, firing, from `lean_since` until `leaning_until`;
    /// tucked in behind his cover otherwise. After a burst out he
    /// stays tucked in until `tucked_until`.
    pub lean_since: u64,
    pub leaning_until: u64,
    pub tucked_until: u64,
}

impl Soldier {
    pub fn new(id: u32, slot: usize, position: V3, hp: f64) -> Self {
        Soldier {
            id,
            slot,
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
            lean: None,
            lean_since: 0,
            leaning_until: 0,
            tucked_until: 0,
        }
    }

    /// The lean he is out on at `tick`: firing from his claimed lean point,
    /// and still at the place it belongs to.
    pub fn leaning(&self, tick: u64) -> Option<&crate::lean::Lean> {
        self.lean.as_ref().filter(|l| {
            tick < self.leaning_until
                && (l.from - self.position.xy()).length() <= crate::lean::AT_PLACE_M
        })
    }

    /// Where his body is at `tick`: out at his lean point while leaning,
    /// else where he stands. What rounds fly into and enemies aim at.
    pub fn exposed(&self, tick: u64) -> V3 {
        match self.leaning(tick) {
            Some(l) => l.at.with_z(self.position.z),
            None => self.position,
        }
    }
}

/// A fallen soldier's record: where, and the squad's heading at the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fallen {
    pub at: V3,
    pub yaw: f64,
    /// Fighting-floor owner retained even after the whole squad dies.
    pub(crate) support_building: Option<PropId>,
}

impl Soldier {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    /// The one way a soldier dies: no health left (a lethal hit's overkill is
    /// kept) and a persistent body record retaining its fall's facing.
    /// Elevated fighting-floor support can later disappear.
    pub fn fall(&mut self, at: V3, yaw: f64, support_building: Option<PropId>) {
        if self.hp > 0.0 {
            self.hp = 0.0;
        }
        self.corpse = Some(Fallen {
            at,
            yaw,
            support_building,
        });
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
    /// Its unit type, the catalog's index.
    pub kind: TypeIndex,
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
    /// Infantry suppression's hidden level in [0, 1] and the tick it last
    /// grew (P14); every effect reads its tier (`SuppressionRules::tier`).
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
    /// A squad's anchor: the centre of the area its soldiers fight
    /// in. Only an order moves it (a move's destination, an attack-move's
    /// halt, a script or a placement), never where the soldiers stand.
    pub anchor: Option<crate::cover::Anchor>,
    /// A wheeled vehicle's three-point turn in progress: the leg it drives
    /// against its order's direction (Q29).
    pub manoeuvre: Option<crate::movement::Manoeuvre>,
    /// The vehicle drove backwards this tick: an ordered reverse move or a
    /// three-point turn's reversing leg (the reverse whine's cue).
    pub reversing: bool,
    /// Actual accepted hull speed; negative while reversing.
    pub drive_speed_mps: f64,
    /// A tracked vehicle's ordered facing (Q9), still to pivot to at rest.
    pub turn_to: Option<f64>,
}

/// How a unit of type `t` moves: on foot, or by its drive (Q29, Q30).
pub fn mobility(t: &UnitType, rules: &Rules) -> Mobility {
    let m = &rules.movement;
    let (off_road_mps, road_mps) = t.mobility.speeds_mps();
    let vehicle = |drive| Mobility {
        off_road_mps,
        road_mps,
        forest_multiplier: m.forest_vehicle_multiplier,
        half_width_m: t.hull().expect("a vehicle has a hull").half_extents_m[1],
        class: MoverClass::Vehicle,
        push: t.hull().expect("a vehicle has a hull").push_class,
        drive: Some(drive),
    };
    match t.mobility {
        Moves::Foot { .. } => Mobility {
            off_road_mps,
            road_mps,
            forest_multiplier: m.forest_infantry_multiplier,
            half_width_m: rules.infantry_movement.path_clearance_m,
            class: MoverClass::Infantry,
            push: contract::scenario::PushClass::None,
            drive: None,
        },
        Moves::Tracked {
            turn_deg_s,
            reverse_fraction,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: true,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: 0.0,
            reverse_fraction,
            feel: m.drive,
        }),
        Moves::Wheeled {
            turn_deg_s,
            turning_radius_m,
            reverse_fraction,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: false,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: turning_radius_m,
            reverse_fraction,
            feel: m.drive,
        }),
    }
}

/// Driving rules the vehicle motion model divides and eases by. (Each unit
/// type's own numbers are the catalog's checks, at load.)
pub fn validate_drive(rules: &Rules) {
    let d = &rules.movement.drive;
    assert!(
        d.acceleration_s.is_finite()
            && d.acceleration_s > 0.0
            && d.braking_s.is_finite()
            && d.braking_s > 0.0,
        "movement.drive acceleration and braking times must be finite and positive"
    );
    for (name, v) in [
        ("turn_in_place_deg", d.turn_in_place_deg),
        ("abeam_m", d.abeam_m),
        ("abeam_deg", d.abeam_deg),
        ("turning_deg", d.turning_deg),
    ] {
        assert!(v > 0.0, "movement.drive.{name} must be positive");
    }
    assert!(
        d.min_leg_m >= 0.0 && d.circle_margin_m >= 0.0,
        "movement.drive.min_leg_m and circle_margin_m must not be negative"
    );
    assert!(
        d.turn_slow > 0.0 && d.turn_slow <= 1.0,
        "movement.drive.turn_slow must be in (0, 1]: a turning vehicle never stops"
    );
}

impl Unit {
    /// Admit an order with the same replacement and deployment semantics everywhere.
    pub(crate) fn enqueue(&mut self, order: UnitOrder, queued: bool) {
        if let Some(d) = self.deployment.as_mut() {
            d.stationary = contract::observation::Posture::Deployed;
        }
        if !queued {
            self.orders.clear();
            self.route = None;
            self.planned_goal = None;
            self.state = MoveState::Idle;
            self.turn_to = None;
        }
        self.orders.push_back(order);
    }
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

    /// Its type's record.
    pub fn unit_type<'a>(&self, rules: &'a Rules) -> &'a UnitType {
        rules.catalog.get(self.kind)
    }

    /// The size of the area a contact this unit causes covers: the fixture's
    /// factor over its type's footprint radius, from the catalog alone (a
    /// hull's half-diagonal, or half a full squad's spread plus a soldier's
    /// body), so it says "vehicle-sized" or "squad-sized", never how many
    /// soldiers are left or how they stand.
    pub fn contact_radius(&self, rules: &Rules) -> f64 {
        let t = self.unit_type(rules);
        let footprint = match t.hull() {
            Some(h) => libm::hypot(h.half_extents_m[0], h.half_extents_m[1]),
            None => {
                crate::arrangement::spread(&rules.infantry_movement, t.squad_size()) / 2.0
                    + rules.physics.soldier_radius_m
            }
        };
        rules.sensors.contact_radius_factor * footprint
    }

    /// A vehicle's full health (0 for a squad, whose health is per soldier).
    pub fn max_hp(&self, rules: &Rules) -> f64 {
        self.unit_type(rules).hull().map_or(0.0, |h| h.hp)
    }

    /// The hull's armour, for vehicles.
    pub fn armor<'a>(&self, rules: &'a Rules) -> Option<&'a Armor> {
        self.unit_type(rules).hull().map(|h| &h.armor)
    }

    /// Fold the unit's complete carried state into `d`.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.id.0 as u64)
            .f64(self.position.x)
            .f64(self.position.y)
            .f64(self.position.z)
            .f64(self.yaw);
        d.u64(self.state as u64).u64(self.reversing as u64);
        if self.is_vehicle() {
            d.f64(self.drive_speed_mps);
        }
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
        match self.anchor {
            Some(a) => d.u64(1).f64(a.at.x).f64(a.at.y).u64(a.halt as u64),
            None => d.u64(0),
        };
        crate::garrison::digest(self, d);
        d.u64(self.members.len() as u64);
        for s in &self.members {
            d.u64(s.id as u64)
                .u64(s.slot as u64)
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
            d.u64(s.lean.is_some() as u64);
            if let Some(l) = &s.lean {
                l.digest(d);
            }
            d.u64(s.lean_since).u64(s.leaning_until).u64(s.tucked_until);
            d.u64(s.corpse.is_some() as u64);
            if let Some(Fallen {
                at: p,
                yaw,
                support_building,
            }) = s.corpse
            {
                d.f64(p.x).f64(p.y).f64(p.z).f64(yaw);
                if let Some(owner) = support_building {
                    d.u64(u64::MAX).u64(owner as u64);
                }
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
            if dep.deploy_step != dep.pack_step {
                d.u64(dep.deploy_step as u64).u64(dep.pack_step as u64);
            }
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
    /// reach (a garrison spans its building's perimeter): a squad's reaches
    /// its outermost soldier's body, of `soldier_radius_m`.
    pub fn footprint_radius(&self, soldier_radius_m: f64) -> f64 {
        match self.hull {
            Some(h) => libm::hypot(h.x, h.y),
            None => {
                self.member_positions()
                    .map(|p| (p.xy() - self.position.xy()).length())
                    .fold(0.0, f64::max)
                    + soldier_radius_m
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
/// per point. The hoist is explicit because, left to the optimiser, an
/// unrelated change once tipped an inlining decision and cost the endurance
/// battle 40% of its instructions.
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

    /// Whether some point of the segment `a`→`b` might lie within `margin`
    /// of the box: `false` only when every point is certainly farther. A
    /// conservative bound (the box's circumscribed sphere, with a millimetre
    /// of slack for rounding), so a caller that skips the segment on `false`
    /// gets the answer it would have measured point by point.
    pub fn may_come_within(&self, a: V3, b: V3, margin: f64) -> bool {
        let c = self.center.with_z(self.mid_z);
        let ab = b - a;
        let len2 = ab.dot(ab);
        let t = if len2 > 0.0 {
            ((c - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + ab * t - c).length() <= margin + self.half.length() + 1e-3
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
