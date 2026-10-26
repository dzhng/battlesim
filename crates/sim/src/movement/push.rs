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

use crate::math::{Obb2, V2};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

/// Bodies within this much of each other touch without pressing: a shove
/// clears the hull by it, and may leave the body up to it inside another,
/// as a fence panel slides along the end of the next one in its line.
const CLEAR_M: f64 = 0.01;
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
    /// Where the pusher's hull meets the prop, for sparks.
    pub contact: V2,
    /// The pusher's speed in m/s, for spark intensity.
    pub speed_mps: f64,
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

/// Where `prop` goes when the hull at `hull`, driving along `heading`,
/// shoves it (Q2): slid out along the axis of least overlap, turned by
/// `turn_rad_per_m` per metre slid when struck off the pusher's centreline.
/// `None` when the shove would drive it into another body that stops
/// vehicles or another live hull (no chain shoves), or off the map.
pub fn shove(
    world: &WorldGeometry,
    units: &[Unit],
    pusher: usize,
    hull: &Obb2,
    heading: V2,
    prop: &Prop,
    turn_rad_per_m: f64,
    speed_mps: f64,
) -> Option<Shove> {
    let by = units[pusher].side;
    if prop.body.topples {
        return Some(Shove {
            prop: prop.id,
            center: prop.center,
            yaw: prop.yaw,
            by,
            heading,
            contact: prop.center,
            speed_mps,
        });
    }
    let before = prop.footprint();
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
        contact: contact_point(hull, &rect),
        speed_mps,
    })
}

/// The contact point between hull and the (now-pushed) prop: the point on
/// the prop's edge closest to the hull's center.
fn contact_point(hull: &Obb2, prop: &Obb2) -> V2 {
    use crate::math::v2;
    let to_hull = hull.center - prop.center;
    let len = to_hull.length();
    if len < 1e-9 {
        return prop.center;
    }
    let dir = to_hull * (1.0 / len);
    let (cos, sin) = (prop.yaw.cos(), prop.yaw.sin());
    let local_dir = v2(cos * dir.x + sin * dir.y, -sin * dir.x + cos * dir.y);
    let local_contact = v2(
        local_dir.x.signum() * prop.half.x,
        local_dir.y.signum() * prop.half.y,
    );
    let contact_dist = (local_contact.x / local_dir.x.abs().max(1e-9))
        .min(local_contact.y / local_dir.y.abs().max(1e-9));
    let local_edge = local_dir * contact_dist.min(len);
    v2(
        prop.center.x + cos * local_edge.x - sin * local_edge.y,
        prop.center.y + sin * local_edge.x + cos * local_edge.y,
    )
}
