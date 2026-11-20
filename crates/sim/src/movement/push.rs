//! The kinematic shove (Q2, Q14, L8): no physics engine. A vehicle's hull
//! meets every body that stops vehicles box against box. A body strictly
//! lighter than the vehicle's push class slides out of the hull along the
//! axis of least overlap, turning a little when struck off-centre; one
//! struck nearly square on the nose glances off it aside, toward the side of
//! the pusher's centreline it sits on, and so leaves the hull's path rather
//! than being driven ahead of it down a street. Where
//! that slide would drive it into another body (a fence panel clipped at its
//! end, pushed endwise into the next), it goes the next shortest way out of
//! the hull, aside or ahead, never back through it. Anything heavier, and a
//! body no way out frees, stops the vehicle.
//! A tree it could shove is knocked down instead (Q16), wherever it stands.
//! Live vehicles are never shoved (Q15): they only block, as traffic.
use contract::ids::Side;
use contract::map::MoverClass;
use contract::scenario::PushClass;

use crate::math::{v2, v3, Obb2, V2, V3};
use crate::units::Unit;
use crate::world::{Prop, PropId, WorldGeometry};
use contract::scenario::Rules;

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
/// `here` never stops it, so it can always back out), and those it shoves
/// (not one it overlaps at `here` and presses no deeper into: backing off
/// bins pinned against a car is not shoving them).
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
        let Some(into) = next.separation(&body) else {
            continue;
        };
        let now = here.separation(&body);
        if push.pushes(prop.body.weight_class) {
            // Leaving a body it already touches shoves nothing.
            if now.is_none_or(|now| into.length() > now.length()) {
                out.shoved.push(prop);
            }
        } else if now.is_none() && out.solid.is_none() {
            out.solid = Some(prop.id);
        }
    }
    out
}

/// The bodies `unit`'s hull is pressed against and shoves: those its last
/// shove left `CLEAR_M` ahead of it.
pub fn pressed(world: &WorldGeometry, rules: &Rules, unit: &Unit) -> Vec<PropId> {
    let (Some(hull), push) = (unit.ground_footprint(), unit.push_class(rules)) else {
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

/// The footprints of the bodies that stop vehicles within touch of `unit`'s
/// hull: what it is pressed against.
pub fn pressing(world: &WorldGeometry, unit: &Unit) -> Vec<Obb2> {
    let Some(hull) = unit.ground_footprint() else {
        return Vec::new();
    };
    let reach = Obb2 {
        half: hull.half + v2(TOUCH_M, TOUCH_M),
        ..hull
    };
    world
        .props_near(hull.center, reach.half.length())
        .into_iter()
        .filter(|prop| prop.blocks(MoverClass::Vehicle))
        .map(Prop::footprint)
        .filter(|body| reach.overlaps(body))
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
/// of least overlap, turned by `rules.turn_deg_per_m` per metre slid when
/// struck off the pusher's centreline; glanced aside when that axis lies
/// within `rules.aside_deg` of the pusher's travel, first to the side it
/// sits on, then the other. `None` when the shove would drive it into
/// another body that stops vehicles or another live hull (no chain shoves),
/// or off the map.
pub fn shove(
    world: &WorldGeometry,
    units: &[Unit],
    pusher: Pusher,
    prop: &Prop,
    rules: &contract::scenario::PushingRules,
) -> Option<Shove> {
    let turn_rad_per_m = rules.turn_deg_per_m.to_radians();
    let (aside_sin, aside_cos) = libm::sincos(rules.aside_deg.to_radians());
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
    // Slid out of the hull and turned; or, toward `aside` (a unit vector
    // across the pusher's travel) when it would go nearly straight ahead,
    // glanced off the nose unturned, clearing the face it leaves.
    let slide = |aside: Option<V2>| {
        let mut rect = before;
        for _ in 0..PASSES {
            let Some(mtv) = hull.separation(&rect) else {
                break;
            };
            let depth = mtv.length();
            let out = mtv * (1.0 / depth);
            if let Some(aside) = aside.filter(|_| out.dot(heading) > aside_cos) {
                // No farther in a pass than a square strike takes it: one
                // leaning the other way is cleared over the passes.
                let way = heading * aside_cos + aside * aside_sin;
                rect.center = rect.center + way * (depth / way.dot(out).max(aside_cos) + CLEAR_M);
                continue;
            }
            // The contact: on the pusher's centreline, beside the body's middle.
            let contact = hull.center + heading * heading.dot(rect.center - hull.center);
            let arm = (contact - rect.center).cross(out) / rect.half.length().max(1e-6);
            rect.center = rect.center + out * (depth + CLEAR_M);
            rect.yaw += turn_rad_per_m * depth * arm.clamp(-1.0, 1.0);
        }
        rect
    };
    // The side of the pusher's centreline the body sits on (its left when
    // dead centre), then the other side, then straight out.
    let left = v2(-heading.y, heading.x);
    let side = if left.dot(before.center - hull.center) >= 0.0 {
        left
    } else {
        -left
    };
    let slides = [Some(side), Some(-side), None].map(slide);
    // No deeper into any other body than it already was, beyond touching.
    let frees = |rect: &Obb2| {
        let deeper = |other: &Obb2| {
            let depth = |r: &Obb2| other.separation(r).map_or(0.0, |v| v.length());
            depth(rect) > depth(&before).max(CLEAR_M) + 1e-9
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
        hull.separation(rect).is_none()
            && world.height_at(rect.center.x, rect.center.y).is_some()
            && !into_prop
            && !into_hull
    };
    let rect = slides
        .into_iter()
        .chain(exits(&hull, &before, heading))
        .find(|rect| frees(rect))?;
    Some(Shove {
        prop: prop.id,
        center: rect.center,
        yaw: rect.yaw,
        by,
        heading,
        contact: contact(&rect),
    })
}

/// The ways `body` can slide straight out of `hull`, shortest first: along
/// each box's axes, aside from or ahead of the pusher's `heading`, never back
/// through the hull.
fn exits(hull: &Obb2, body: &Obb2, heading: V2) -> Vec<Obb2> {
    let project = |r: &Obb2, axis: V2| {
        r.half.x * v2(1.0, 0.0).rotated(r.yaw).dot(axis).abs()
            + r.half.y * v2(0.0, 1.0).rotated(r.yaw).dot(axis).abs()
    };
    let d = body.center - hull.center;
    let mut ways: Vec<(f64, V2)> = [hull.yaw, body.yaw]
        .into_iter()
        .flat_map(|yaw| [v2(1.0, 0.0).rotated(yaw), v2(0.0, 1.0).rotated(yaw)])
        .flat_map(|axis| [axis, -axis])
        .filter(|out| out.dot(heading) > -1e-9)
        .map(|out| (project(hull, out) + project(body, out) - d.dot(out), out))
        .filter(|(depth, _)| *depth > 0.0)
        .collect();
    ways.sort_by(|a, b| a.0.total_cmp(&b.0));
    ways.into_iter()
        .map(|(depth, out)| Obb2 {
            center: body.center + out * (depth + CLEAR_M),
            ..*body
        })
        .collect()
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
