//! The kinematic shove (Q2, Q14, L8): no physics engine. A vehicle's hull
//! meets every body that stops vehicles box against box. A body strictly
//! lighter than the vehicle's push class slides out of the hull along the
//! axis of least overlap, turning a little when struck off-centre; anything
//! else, and a body the shove would drive into another, stops the vehicle.
//! A tree it could shove is knocked down instead (Q16), wherever it stands.
//! Live vehicles are never shoved (Q15): they only block, as traffic.
use contract::ids::Side;
use contract::map::MoverClass;
use contract::scenario::PushClass;

use crate::math::{v2, v3, Obb2, V2, V3};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

/// Bodies within this much of each other touch without pressing: a shove
/// clears the hull by it, and may leave the body up to it inside another,
/// as a fence panel slides along the end of the next one in its line.
const CLEAR_M: f64 = 0.01;
/// Corners this near the nearest one share its contact.
const TOUCH_M: f64 = 0.1;
/// Slide-and-turn passes before a shove that still overlaps gives up.
const PASSES: usize = 3;

/// A body a vehicle shoves this tick, and the pose it ends in; a tree
/// keeps its pose and is knocked down (its row `topples`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shove {
    pub prop: PropId,
    pub center: V2,
    pub yaw: f64,
    /// The pusher's side: it learns at once what it moved (contact).
    pub by: Side,
    /// The pusher's horizontal travel: the way a body it knocks down falls.
    pub heading: V2,
    /// Where the hull meets the body, halfway up the lower of the two.
    pub contact: V3,
}

/// What a hull moving to `next` from `here` meets among the bodies that
/// stop vehicles: the first it cannot move (a body it already overlaps at
/// `here` never stops it, so it can always back out), and those it shoves.
pub struct Meet<'a> {
    pub solid: Option<PropId>,
    pub shoved: Vec<&'a Prop>,
}

pub fn meet<'a>(world: &'a WorldGeometry, next: &Obb2, here: &Obb2, push: PushClass) -> Meet<'a> {
    let mut out = Meet {
        solid: None,
        shoved: Vec::new(),
    };
    for prop in world.props_near(next.center, next.half.length()) {
        if !prop.blocks(MoverClass::Vehicle) {
            continue;
        }
        let body = prop.footprint();
        if next.separation(&body).is_none() {
            continue;
        }
        if push.pushes(prop.body.weight_class) {
            out.shoved.push(prop);
        } else if here.separation(&body).is_none() && out.solid.is_none() {
            out.solid = Some(prop.id);
        }
    }
    out
}

/// The bodies `unit`'s hull is pressed against and shoves: those its last
/// shove left `CLEAR_M` ahead of it.
pub fn pressed(world: &WorldGeometry, unit: &Unit) -> Vec<PropId> {
    let (Some(hull), push) = (unit.ground_footprint(), unit.ground().push) else {
        return Vec::new();
    };
    world
        .props_near(hull.center, hull.half.length())
        .into_iter()
        .filter(|prop| {
            let body = prop.footprint();
            let reach = Obb2 {
                half: body.half + v2(2.0 * CLEAR_M, 2.0 * CLEAR_M),
                ..body
            };
            prop.blocks(MoverClass::Vehicle)
                && push.pushes(prop.body.weight_class)
                && hull.separation(&reach).is_some()
        })
        .map(|prop| prop.id)
        .collect()
}

/// A vehicle driving into bodies this tick: the unit, where its hull goes
/// and its horizontal travel.
#[derive(Clone, Copy)]
pub struct Pusher {
    pub unit: usize,
    pub hull: Obb2,
    pub heading: V2,
}

/// Where `prop` goes when `pusher` shoves it (Q2): slid out along the axis
/// of least overlap, turned by `turn_rad_per_m` per metre slid when struck
/// off the pusher's centreline. `None` when the shove would drive it into
/// another body that stops vehicles or another live hull (no chain shoves),
/// or off the map.
pub fn shove(
    world: &WorldGeometry,
    units: &[Unit],
    pusher: Pusher,
    prop: &Prop,
    turn_rad_per_m: f64,
) -> Option<Shove> {
    let Pusher {
        unit: pusher,
        hull,
        heading,
    } = pusher;
    let by = units[pusher].side;
    let before = prop.footprint();
    let contact = |body: &Obb2| {
        let c = contact_point(&hull, body);
        let half_height = units[pusher]
            .hull
            .map_or(prop.half.z, |h| h.z.min(prop.half.z));
        v3(c.x, c.y, prop.base_z + half_height)
    };
    if prop.body.topples {
        return Some(Shove {
            prop: prop.id,
            center: prop.center,
            yaw: prop.yaw,
            by,
            heading,
            contact: contact(&before),
        });
    }
    let mut rect = before;
    for _ in 0..PASSES {
        let Some(mtv) = hull.separation(&rect) else {
            break;
        };
        let depth = mtv.length();
        let out = mtv * (1.0 / depth);
        // The contact: on the pusher's centreline, beside the body's middle.
        let contact = hull.center + heading * heading.dot(rect.center - hull.center);
        let arm = (contact - rect.center).cross(out) / rect.half.length().max(1e-6);
        rect.center = rect.center + out * (depth + CLEAR_M);
        rect.yaw += turn_rad_per_m * depth * arm.clamp(-1.0, 1.0);
    }
    if hull.separation(&rect).is_some() || world.height_at(rect.center.x, rect.center.y).is_none() {
        return None;
    }
    // No deeper into any other body than it already was, beyond touching.
    let deeper = |other: &Obb2| {
        let depth = |r: &Obb2| other.separation(r).map_or(0.0, |v| v.length());
        depth(&rect) > depth(&before).max(CLEAR_M) + 1e-9
    };
    let reach = rect.half.length() + 1.0;
    let into_prop = world
        .props_near(rect.center, reach)
        .into_iter()
        .filter(|q| q.id != prop.id && q.blocks(MoverClass::Vehicle))
        .any(|q| deeper(&q.footprint()));
    let into_hull = units
        .iter()
        .enumerate()
        .filter(|(j, u)| *j != pusher && u.on_field())
        .filter_map(|(_, u)| u.ground_footprint())
        .any(|h| deeper(&h));
    (!into_prop && !into_hull).then_some(Shove {
        prop: prop.id,
        center: rect.center,
        yaw: rect.yaw,
        by,
        heading,
        contact: contact(&rect),
    })
}

/// Where the hull meets the shoved body: the corners of either box that
/// touch the other, averaged (one corner on a slant, the middle of the shared
/// stretch where two faces lie flush).
fn contact_point(hull: &Obb2, prop: &Obb2) -> V2 {
    let gaps: Vec<(V2, f64)> = hull
        .corners()
        .into_iter()
        .map(|c| (c, prop.distance(c)))
        .chain(prop.corners().into_iter().map(|c| (c, hull.distance(c))))
        .collect();
    let least = gaps.iter().fold(f64::INFINITY, |m, g| m.min(g.1));
    let touching: Vec<V2> = gaps
        .iter()
        .filter(|g| g.1 <= least + TOUCH_M)
        .map(|g| g.0)
        .collect();
    touching.iter().fold(v2(0.0, 0.0), |s, c| s + *c) * (1.0 / touching.len() as f64)
}
