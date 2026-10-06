//! Whether a unit may start a battle where it is put: its footprint on
//! ground it can stand on, clear of every body, of water and of the units
//! already placed. The questions are the world's and navigation's own, the
//! ones a moving unit is held to; nothing here decides what blocks what.
use contract::catalog::UnitType;
use contract::map::MoverClass;
use contract::scenario::Rules;

use super::MapQueries;
use crate::arrangement;
use crate::math::{v2, Obb2, V2};
use crate::navigation::Mobility;
use crate::world::Prop;

/// What a unit covers where it starts: a vehicle's hull, or the ground a
/// squad spreads over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Footprint {
    Hull(Obb2),
    Squad { centre: V2, radius: f64 },
}

/// Why a place is not legal ground for a unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Illegal {
    /// Part of the footprint lies off the map.
    OffMap,
    /// Part of it lies in water, or on ground too steep to stand on.
    NotGround,
    /// A body stands in it.
    Body,
    /// Navigation has no cell there the unit fits.
    NoRoom,
    /// Fewer standing places than the squad has soldiers.
    NoStandingRoom,
    /// Another unit's footprint is within the clearance.
    Unit,
}

impl Illegal {
    pub fn describe(self) -> &'static str {
        match self {
            Illegal::OffMap => "part of the footprint lies off the map",
            Illegal::NotGround => "part of the footprint lies in water or on a slope too steep",
            Illegal::Body => "a body stands in the footprint",
            Illegal::NoRoom => "navigation has no room for the unit there",
            Illegal::NoStandingRoom => "the squad's spread holds too few standing places",
            Illegal::Unit => "another unit's footprint is within the clearance",
        }
    }
}

/// How far `a` and `b` are from touching: zero or less when they meet.
fn gap(a: &Footprint, b: &Footprint) -> f64 {
    match (a, b) {
        (Footprint::Hull(x), Footprint::Hull(y)) => hull_gap(x, y),
        (Footprint::Hull(h), Footprint::Squad { centre, radius })
        | (Footprint::Squad { centre, radius }, Footprint::Hull(h)) => h.distance(*centre) - radius,
        (
            Footprint::Squad { centre, radius },
            Footprint::Squad {
                centre: other,
                radius: reach,
            },
        ) => distance(*centre, *other) - radius - reach,
    }
}

/// The least distance between two rectangles' corners and sides: zero when
/// they overlap.
fn hull_gap(a: &Obb2, b: &Obb2) -> f64 {
    if a.overlaps(b) {
        return 0.0;
    }
    let from = |x: &Obb2, y: &Obb2| {
        corners(x)
            .into_iter()
            .map(|c| y.distance(c))
            .fold(f64::INFINITY, f64::min)
    };
    from(a, b).min(from(b, a))
}

pub fn corners(o: &Obb2) -> [V2; 4] {
    [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
        .map(|(x, y)| o.center + v2(o.half.x * x, o.half.y * y).rotated(o.yaw))
}

/// Exact distance (a square root, the same on every target).
pub fn distance(a: V2, b: V2) -> f64 {
    let d = b - a;
    (d.x * d.x + d.y * d.y).sqrt()
}

/// Whether two footprints keep `clearance` between them.
pub fn apart(a: &Footprint, b: &Footprint, clearance: f64) -> bool {
    gap(a, b) > clearance
}

/// The ground a unit of type `t` covers at `at`, heading `yaw`.
pub fn footprint(t: &UnitType, rules: &Rules, at: V2, yaw: f64) -> Footprint {
    match t.hull() {
        Some(hull) => Footprint::Hull(Obb2 {
            center: at,
            yaw,
            half: v2(hull.half_extents_m[0], hull.half_extents_m[1]),
        }),
        None => Footprint::Squad {
            centre: at,
            radius: arrangement::spread(&rules.infantry_movement, t.squad_size()) / 2.0,
        },
    }
}

/// Whether the map lets a unit of type `t` stand at `at`: every part of a
/// hull on ground a mover may stand on with no body in it and room in
/// navigation's clearance; a squad with room at its middle and a standing
/// place in its spread for each soldier, reachable on foot from the middle.
pub fn stands(
    q: &MapQueries,
    rules: &Rules,
    t: &UnitType,
    m: &Mobility,
    at: V2,
    yaw: f64,
) -> Result<(), Illegal> {
    stands_on(q.world, q.grid, rules, t, m, at, yaw)
}

/// Shared standing admission for prepared encounters and physical reinforcements.
pub fn stands_on(
    world: &crate::world::WorldGeometry,
    grid: &crate::navigation::NavGrid,
    rules: &Rules,
    t: &UnitType,
    m: &Mobility,
    at: V2,
    yaw: f64,
) -> Result<(), Illegal> {
    let inside = |p: V2| p.x >= 0.0 && p.y >= 0.0 && p.x <= world.width() && p.y <= world.depth();
    match footprint(t, rules, at, yaw) {
        Footprint::Hull(hull) => {
            let c = corners(&hull);
            let mut points = vec![at];
            for k in 0..4 {
                points.push(c[k]);
                points.push((c[k] + c[(k + 1) % 4]) * 0.5);
            }
            if !points.iter().all(|&p| inside(p)) {
                return Err(Illegal::OffMap);
            }
            if !points.iter().all(|p| world.traversable_at(p.x, p.y)) {
                return Err(Illegal::NotGround);
            }
            let reach = distance(v2(0.0, 0.0), hull.half);
            let blocked = world
                .props_near(at, reach)
                .iter()
                .any(|p| p.blocks(MoverClass::Vehicle) && p.footprint().overlaps(&hull));
            if blocked {
                return Err(Illegal::Body);
            }
            if !grid.fits_at(at, m) {
                return Err(Illegal::NoRoom);
            }
            Ok(())
        }
        Footprint::Squad { centre, radius } => {
            if !inside(centre) {
                return Err(Illegal::OffMap);
            }
            if !world.traversable_at(centre.x, centre.y) {
                return Err(Illegal::NotGround);
            }
            let solid = |p: &Prop| p.blocks(MoverClass::Infantry);
            let soldier = rules.physics.soldier_radius_m;
            if !arrangement::standing_room(world, centre, soldier, &solid) {
                return Err(Illegal::Body);
            }
            if !grid.fits_at(centre, m) {
                return Err(Illegal::NoRoom);
            }
            // A lattice of places the squad's spacing apart over its spread:
            // the squad stands when enough of them are standing room a
            // soldier can walk to from the middle.
            let spacing = rules.infantry_movement.spacing_m;
            let n = (radius / spacing).floor() as i32;
            let mut places = 0usize;
            for i in -n..=n {
                for j in -n..=n {
                    let offset = v2(i as f64 * spacing, j as f64 * spacing);
                    if offset.x * offset.x + offset.y * offset.y > radius * radius {
                        continue;
                    }
                    let p = centre + offset;
                    if inside(p)
                        && arrangement::standing_room(world, p, soldier, &solid)
                        && arrangement::reachable(world, centre, p, soldier, &solid)
                    {
                        places += 1;
                    }
                }
            }
            if places < t.squad_size() {
                return Err(Illegal::NoStandingRoom);
            }
            Ok(())
        }
    }
}
