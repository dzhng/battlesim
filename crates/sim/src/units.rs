//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::VecDeque;

use contract::command::RoutePolicy;
use contract::ids::{Side, UnitId};
use contract::observation::MoveState;
use contract::scenario::{Rules, UnitKind};

use crate::math::{v2, V2, V3};
use crate::navigation::Mobility;

/// Spacing between squad members in their loose line; flexible, not rigid.
const SQUAD_SPACING_M: f64 = 2.5;
/// Infantry path clearance: a squad threads gaps a vehicle cannot.
const INFANTRY_HALF_WIDTH_M: f64 = 0.5;

#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    /// Offset in the squad frame (x forward, y left).
    pub offset: V2,
    pub alive: bool,
}

#[derive(Clone, Debug)]
pub struct MoveOrder {
    pub destination: V2,
    pub policy: RoutePolicy,
    pub gesture: u64,
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
    pub orders: VecDeque<MoveOrder>,
    /// Remaining waypoints of the current order, once planned.
    pub route: Option<Vec<V2>>,
    pub state: MoveState,
    pub blocker: Option<UnitId>,
    /// Side knowledge revision the current route (or block) was judged against.
    pub planned_revision: u64,
    /// Failure-to-advance watch: best distance to the next waypoint and when.
    pub progress: (f64, u64),
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

pub fn squad_size(kind: UnitKind, rules: &Rules) -> u32 {
    match kind {
        UnitKind::Rifle => rules.squads.rifle_squad_size,
        UnitKind::Recon => rules.squads.recon_squad_size,
        UnitKind::At => rules.squads.at_squad_size,
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

    /// World positions of living members, standing at the squad's height.
    pub fn member_positions(&self) -> impl Iterator<Item = V3> + '_ {
        self.members
            .iter()
            .filter(|s| s.alive)
            .map(|s| (self.position.xy() + s.offset.rotated(self.yaw)).with_z(self.position.z))
    }

    /// Radius of the unit's ground footprint, for traffic spacing.
    pub fn footprint_radius(&self) -> f64 {
        match self.hull {
            Some(h) => h.x.hypot(h.y),
            None => {
                self.members
                    .iter()
                    .filter(|s| s.alive)
                    .map(|s| s.offset.length())
                    .fold(0.0, f64::max)
                    + INFANTRY_HALF_WIDTH_M
            }
        }
    }

    pub fn current_destination(&self) -> Option<V2> {
        self.orders.front().map(|o| o.destination)
    }
}
