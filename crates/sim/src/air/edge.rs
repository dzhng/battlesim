use contract::encounter::SkirmishSites;
use contract::ids::Side;

use crate::math::{v2, V2};
use crate::world::WorldGeometry;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackEdge {
    pub a: V2,
    pub b: V2,
    pub inward: V2,
}

pub fn back_edge(side: Side, sites: &SkirmishSites, world: &WorldGeometry) -> BackEdge {
    let center = sites
        .entries
        .iter()
        .find(|entry| entry.side == side)
        .map(|entry| v2(entry.center[0], entry.center[1]))
        .expect("combatant entry site");
    let w = world.width();
    let h = world.depth();
    let candidates = [
        (v2(0.0, 0.0), v2(w, 0.0), v2(0.0, 1.0)),
        (v2(w, h), v2(0.0, h), v2(0.0, -1.0)),
        (v2(0.0, h), v2(0.0, 0.0), v2(1.0, 0.0)),
        (v2(w, 0.0), v2(w, h), v2(-1.0, 0.0)),
    ];
    let (a, b, inward) = *candidates
        .iter()
        .min_by(|(a, b, _), (c, d, _)| {
            let distance = |a: V2, b: V2| {
                let e = b - a;
                let t = ((center - a).dot(e) / e.dot(e)).clamp(0.0, 1.0);
                (center - (a + e * t)).length()
            };
            distance(*a, *b).total_cmp(&distance(*c, *d))
        })
        .unwrap();
    BackEdge { a, b, inward }
}

/// The point on `edge` nearest `toward`, and the inward heading there: where
/// a jet crosses the edge, coming in or going home.
pub fn entry(edge: &BackEdge, toward: V2) -> (V2, f64) {
    let edge_vec = edge.b - edge.a;
    let t = ((toward - edge.a).dot(edge_vec) / edge_vec.dot(edge_vec)).clamp(0.0, 1.0);
    let point = edge.a + edge_vec * t;
    (point, libm::atan2(edge.inward.y, edge.inward.x))
}

pub fn crossed(edge: &BackEdge, prev: V2, next: V2) -> bool {
    let a = (prev - edge.a).dot(edge.inward);
    let b = (next - edge.a).dot(edge.inward);
    a >= 0.0 && b < 0.0
}

/// Where and which way an arriving aircraft enters (D39): at the back-edge
/// point nearest its side's entry site, heading for `destination` when that
/// lies into the map, else straight in.
pub fn arrival(edge: &BackEdge, entry_site: V2, destination: V2) -> (V2, f64) {
    let (point, inward) = entry(edge, entry_site);
    let toward = destination - point;
    let heading = if toward.dot(edge.inward) > 0.0 {
        libm::atan2(toward.y, toward.x)
    } else {
        inward
    };
    (point, heading)
}
