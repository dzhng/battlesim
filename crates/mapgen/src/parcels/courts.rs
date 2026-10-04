//! Courts: the paved interior of a dense district's block. A district is one
//! convex block with its streets laid inside it, so its ring paved as one
//! polygon lies between every one of its buildings; its streets draw over
//! it, a carriageway winning where paved kinds overlap. Along an edge that
//! carries no carriageway the court is drawn in to its nearest parcel, so
//! the paving ends at the parcels' rears and never runs out into the fields.
use super::streets::Network;
use super::Pass;
use crate::layout::geometry::{area, cross, distance, round_cm, sub, Point};
use crate::{CourtPlan, Diagnostic, LotPlan, MapPlan};
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};

/// The court of every district of `plan` whose presets pave one, with the
/// paving each is laid as. `lots` are the parcels the pass cut.
pub fn lay(
    pass: &Pass,
    network: &Network,
    plan: &MapPlan,
    lots: &[LotPlan],
) -> Result<(Vec<CourtPlan>, Vec<SurfaceArea>), Vec<Diagnostic>> {
    let (mut courts, mut paving) = (Vec::new(), Vec::new());
    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
        if !pass.district(district)?.props.courts.paved {
            continue;
        }
        let mut block = district.ring.clone();
        if area(&block) < 0.0 {
            block.reverse();
        }
        let prefix = format!("{}/", district.id);
        let corners: Vec<Point> = lots
            .iter()
            .filter(|lot| lot.id.starts_with(&prefix))
            .flat_map(|lot| lot.ring.iter().copied())
            .collect();
        let mut ring = block.clone();
        for (i, a) in block.iter().enumerate() {
            let b = block[(i + 1) % block.len()];
            if network.runs_along(*a, b, false) {
                continue;
            }
            let nearest = corners
                .iter()
                .map(|p| inward(*a, b, *p))
                .fold(f64::INFINITY, f64::min);
            if nearest.is_finite() && nearest > 0.0 {
                ring = keep_inside(&ring, *a, b, nearest);
            }
        }
        let mut ring: Vec<Point> = ring.into_iter().map(round_cm).collect();
        ring.dedup();
        if ring.len() > 1 && ring[0] == ring[ring.len() - 1] {
            ring.pop();
        }
        let Ok(shape) = GroundShape::polygon(ring.clone()) else {
            continue;
        };
        courts.push(CourtPlan {
            id: format!("{}/court", district.id),
            district: district.id.clone(),
            ring,
        });
        paving.push(SurfaceArea {
            kind: SurfaceKind::Paving,
            shape,
        });
    }
    Ok((courts, paving))
}

/// How far `p` lies to the left of the line from `a` to `b`: inside a
/// counter-clockwise ring whose edge that is.
fn inward(a: Point, b: Point, p: Point) -> f64 {
    cross(sub(b, a), sub(p, a)) / distance(a, b)
}

/// The part of the convex `ring` at least `inset` to the left of the line
/// from `a` to `b` (Sutherland–Hodgman against one half-plane).
fn keep_inside(ring: &[Point], a: Point, b: Point, inset: f64) -> Vec<Point> {
    let depth = |p: Point| inward(a, b, p) - inset;
    let mut out = Vec::new();
    for (i, p) in ring.iter().enumerate() {
        let q = ring[(i + 1) % ring.len()];
        let (dp, dq) = (depth(*p), depth(q));
        if dp >= 0.0 {
            out.push(*p);
        }
        if (dp >= 0.0) != (dq >= 0.0) {
            let t = dp / (dp - dq);
            out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
        }
    }
    out
}
