//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::{BTreeSet, VecDeque};

use contract::command::{Engagement, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::MoveState;
use contract::scenario::{Armor, Face, HealthRules, Rules, UnitKind};

use crate::math::{v2, V2, V3};
use crate::navigation::Mobility;
use crate::weapons::{Mount, Target};

/// Spacing between squad members in their loose line; flexible, not rigid.
const SQUAD_SPACING_M: f64 = 2.5;
/// Infantry path clearance: a squad threads gaps a vehicle cannot.
const INFANTRY_HALF_WIDTH_M: f64 = 0.5;

#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    /// Offset in the squad frame (x forward, y left).
    pub offset: V2,
    pub hp: f64,
    /// Where the soldier fell: a permanent record that blocks nothing (M06).
    pub corpse: Option<V3>,
}

impl Soldier {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
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
}

impl UnitOrder {
    pub fn movement(&self) -> Option<&MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            UnitOrder::Attack { .. } => None,
        }
    }

    pub fn movement_mut(&mut self) -> Option<&mut MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            UnitOrder::Attack { .. } => None,
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

    /// World position of member `k`, standing at the squad's height.
    pub fn member_position(&self, k: usize) -> V3 {
        (self.position.xy() + self.members[k].offset.rotated(self.yaw)).with_z(self.position.z)
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

    /// Distance from a point to the hull box (0 inside).
    pub fn hull_distance(&self, p: V3) -> f64 {
        let h = self.hull.expect("vehicle");
        let d = (p.xy() - self.position.xy()).rotated(-self.yaw);
        let dz = p.z - (self.position.z + h.z);
        let ex = (d.x.abs() - h.x).max(0.0);
        let ey = (d.y.abs() - h.y).max(0.0);
        let ez = (dz.abs() - h.z).max(0.0);
        (ex * ex + ey * ey + ez * ez).sqrt()
    }

    /// The hull face facing `p` (see [`face_toward`]).
    pub fn hull_face(&self, p: V3) -> Face {
        let h = self.hull.expect("vehicle");
        let d = (p.xy() - self.position.xy()).rotated(-self.yaw);
        face_toward(d.with_z(p.z - (self.position.z + h.z)), h)
    }

    /// Radius of the unit's ground footprint, for traffic spacing.
    pub fn footprint_radius(&self) -> f64 {
        match self.hull {
            Some(h) => h.x.hypot(h.y),
            None => {
                self.members
                    .iter()
                    .filter(|s| s.alive())
                    .map(|s| s.offset.length())
                    .fold(0.0, f64::max)
                    + INFANTRY_HALF_WIDTH_M
            }
        }
    }

    /// Where movement should head now and by which policy: an ordered
    /// destination, or an attack's pursuit point.
    pub fn movement_goal(&self) -> Option<(V2, RoutePolicy)> {
        match self.orders.front()? {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some((o.destination, o.policy)),
            UnitOrder::Attack { .. } => self.pursuit.map(|p| (p, RoutePolicy::Shortest)),
        }
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
