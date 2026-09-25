//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::{BTreeSet, VecDeque};

use contract::command::{Engagement, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::MoveState;
use contract::scenario::{Armor, Face, HealthRules, Rules, UnitKind};

use crate::digest::Digest;
use crate::garrison::{Garrison, Phase};
use crate::math::{v2, Obb2, V2, V3};
use crate::navigation::Mobility;
use crate::weapons::{Mount, Target};
use crate::world::PropId;

/// Spacing between squad members in their loose line; flexible, not rigid.
const SQUAD_SPACING_M: f64 = 2.5;
/// Infantry path clearance: a squad threads gaps a vehicle cannot.
const INFANTRY_HALF_WIDTH_M: f64 = 0.5;

#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    /// Offset in the squad frame (x forward, y left).
    pub offset: V2,
    /// The offset the soldier walks back to after being scattered (a collapse).
    pub formation: V2,
    pub hp: f64,
    /// Where the soldier fell: a permanent record that blocks nothing (M06).
    pub corpse: Option<V3>,
}

impl Soldier {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    /// The one way a soldier dies: no health left (a lethal hit's overkill is
    /// kept) and a permanent record where it fell.
    pub fn fall(&mut self, at: V3) {
        if self.hp > 0.0 {
            self.hp = 0.0;
        }
        self.corpse = Some(at);
    }
}

#[derive(Clone, Debug)]
pub struct MoveOrder {
    pub destination: V2,
    pub policy: RoutePolicy,
    pub gesture: u64,
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
}

pub fn mobility(kind: UnitKind, rules: &Rules) -> Mobility {
    let m = &rules.movement;
    let b = &rules.bodies;
    match kind {
        UnitKind::Rifle | UnitKind::Recon | UnitKind::At => Mobility {
            off_road_mps: m.infantry_mps,
            road_mps: m.infantry_mps * m.infantry_road_multiplier,
            forest_multiplier: m.forest_infantry_multiplier,
            half_width_m: INFANTRY_HALF_WIDTH_M,
        },
        UnitKind::Tank => Mobility {
            off_road_mps: m.tank_mps,
            road_mps: m.tank_road_mps,
            forest_multiplier: m.forest_vehicle_multiplier,
            half_width_m: b.tank_half_extents_m[1],
        },
        UnitKind::Supply => Mobility {
            off_road_mps: m.supply_mps,
            road_mps: m.supply_road_mps,
            forest_multiplier: m.forest_vehicle_multiplier,
            half_width_m: b.supply_half_extents_m[1],
        },
    }
}

pub fn hull(kind: UnitKind, rules: &Rules) -> Option<V3> {
    let h = match kind {
        UnitKind::Tank => rules.bodies.tank_half_extents_m,
        UnitKind::Supply => rules.bodies.supply_half_extents_m,
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
    }
}

/// Authored vehicle health (0 for infantry, whose health is per soldier).
pub fn max_hp(kind: UnitKind, rules: &Rules) -> f64 {
    match kind {
        UnitKind::Tank => rules.health.tank,
        UnitKind::Supply => rules.health.supply,
        _ => 0.0,
    }
}

pub fn squad_size(kind: UnitKind, rules: &Rules) -> u32 {
    match kind {
        UnitKind::Rifle => rules.health.rifle_squad_size,
        UnitKind::Recon => rules.health.recon_squad_size,
        UnitKind::At => rules.health.at_squad_size,
        UnitKind::Tank | UnitKind::Supply => 0,
    }
}

/// Two staggered ranks centred on the unit, facing +X.
pub fn squad_offsets(size: u32) -> Vec<V2> {
    let per_rank = size.div_ceil(2).max(1);
    (0..size)
        .map(|k| {
            let rank = (k / per_rank) as f64;
            let file = (k % per_rank) as f64 - (per_rank - 1) as f64 / 2.0;
            v2(
                -rank * SQUAD_SPACING_M,
                -file * SQUAD_SPACING_M + rank * SQUAD_SPACING_M * 0.5,
            )
        })
        .collect()
}

impl Unit {
    pub fn is_vehicle(&self) -> bool {
        self.hull.is_some()
    }

    /// World position of member `k`: its perimeter slot while garrisoned,
    /// otherwise its place in the squad at the squad's height.
    pub fn member_position(&self, k: usize) -> V3 {
        if let Some(slot) = self.garrison.as_ref().and_then(|g| g.seat(k)) {
            return slot.position;
        }
        (self.position.xy() + self.members[k].offset.rotated(self.yaw)).with_z(self.position.z)
    }

    /// At its building's perimeter slots (inside or leaving, not entering).
    pub fn garrisoned(&self) -> bool {
        self.garrison
            .as_ref()
            .is_some_and(|g| matches!(g.phase, Phase::Inside | Phase::Exiting(_)))
    }

    /// World positions of living members.
    pub fn member_positions(&self) -> impl Iterator<Item = V3> + '_ {
        (0..self.members.len())
            .filter(|&k| self.members[k].alive())
            .map(|k| self.member_position(k))
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
        match self.kind {
            UnitKind::Tank => Some(&health.tank_armor),
            UnitKind::Supply => Some(&health.supply_armor),
            _ => None,
        }
    }

    /// Fold the unit's complete carried state into `d`.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.id.0 as u64)
            .f64(self.position.x)
            .f64(self.position.y)
            .f64(self.position.z)
            .f64(self.yaw);
        d.u64(self.state as u64);
        d.u64(self.orders.len() as u64);
        for o in &self.orders {
            match o {
                UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                    d.u64(matches!(o, UnitOrder::Move(_)) as u64);
                    d.f64(m.destination.x)
                        .f64(m.destination.y)
                        .u64(m.policy as u64)
                        .u64(m.gesture);
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
        crate::garrison::digest(self, d);
        d.u64(self.members.len() as u64);
        for s in &self.members {
            d.u64(s.id as u64)
                .f64(s.hp)
                .f64(s.formation.x)
                .f64(s.formation.y);
            d.f64(s.offset.x).f64(s.offset.y);
            d.u64(s.corpse.is_some() as u64);
            if let Some(p) = s.corpse {
                d.f64(p.x).f64(p.y).f64(p.z);
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
        let h = self.hull.expect("vehicle");
        let d = self.hull_box().expect("vehicle").to_local(p.xy());
        let dz = p.z - (self.position.z + h.z);
        let ex = (d.x.abs() - h.x).max(0.0);
        let ey = (d.y.abs() - h.y).max(0.0);
        let ez = (dz.abs() - h.z).max(0.0);
        (ex * ex + ey * ey + ez * ez).sqrt()
    }

    /// The hull face facing `p` (see [`face_toward`]).
    pub fn hull_face(&self, p: V3) -> Face {
        let h = self.hull.expect("vehicle");
        let d = self.hull_box().expect("vehicle").to_local(p.xy());
        face_toward(d.with_z(p.z - (self.position.z + h.z)), h)
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
