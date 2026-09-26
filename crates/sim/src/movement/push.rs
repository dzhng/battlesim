//! The kinematic shove (Q2, Q14, L8): no physics engine. A vehicle's hull
//! meets every body that stops vehicles box against box. A body strictly
//! lighter than the vehicle's push class slides out of the hull along the
//! axis of least overlap, turning a little when struck off-centre; anything
//! else, and a body the shove would drive into another, stops the vehicle.
//! Live vehicles are never shoved (Q15): they only block, as traffic.
use contract::map::MoverClass;
use contract::scenario::PushClass;

use crate::math::{Obb2, V2};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};

/// A shove clears the hull by this much, so the two no longer touch.
const CLEAR_M: f64 = 0.01;
/// Slide-and-turn passes before a shove that still overlaps gives up.
const PASSES: usize = 3;

/// A body a vehicle shoves this tick, and the pose it ends in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shove {
    pub prop: PropId,
    pub center: V2,
    pub yaw: f64,
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
) -> Option<Shove> {
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
    // No deeper into any other body than it already was.
    let deeper = |other: &Obb2| {
        let depth = |r: &Obb2| other.separation(r).map_or(0.0, |v| v.length());
        depth(&rect) > depth(&before) + 1e-9
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
        .filter(|(j, u)| *j != pusher && u.alive())
        .filter_map(|(_, u)| u.hull_box())
        .any(|h| deeper(&h));
    (!into_prop && !into_hull).then_some(Shove {
        prop: prop.id,
        center: rect.center,
        yaw: rect.yaw,
    })
}
