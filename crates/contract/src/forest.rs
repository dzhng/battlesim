//! The one forest trunk candidate sequence and physical exclusion order.
//! Consumers supply indexed queries over the same authored geometry.
use crate::map::Forest;
use crate::random::Rng;
use crate::scenario::ForestRule;

/// Maximum physical foliage-depth integration step, shared by proofs.
pub const FOLIAGE_SAMPLE_M: f64 = 1.0;

pub fn forest_seed(index: usize, forest: &Forest) -> u64 {
    let initial = 0x9e37_79b9_7f4a_7c15 ^ index as u64;
    let hash = |h: u64, v: &f64| (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3);
    if let Some(rect) = forest.shape.exact_rectangle() {
        return rect.iter().fold(initial, hash);
    }
    match &forest.shape {
        crate::ground::GroundShape::Polygon { ring } => {
            ring.iter().flatten().fold(initial ^ 1, hash)
        }
        crate::ground::GroundShape::Stroke {
            centerline,
            width_m,
        } => centerline
            .control_points()
            .iter()
            .flatten()
            .fold(hash(initial ^ 2, width_m), hash),
    }
}

/// Authoritative physical queries; indices are an implementation choice.
pub trait TrunkQueries {
    /// A bounded proof may stop a candidate prefix. It must then refuse its
    /// incomplete result; the physical world's default always finishes.
    fn keep_sampling(&self) -> bool {
        true
    }
    fn road_near(&self, p: [f64; 2], margin: f64) -> bool;
    fn water_near(&self, p: [f64; 2], margin: f64) -> bool;
    fn body_near(&self, p: [f64; 2], margin: f64) -> bool;
    fn contains_ground(&self, p: [f64; 2]) -> bool;
}

/// Same seeded jitter and rejection order as the world that stands the trees.
pub fn trunk_positions(
    index: usize,
    forest: &Forest,
    rule: &ForestRule,
    queries: &impl TrunkQueries,
) -> Vec<[f64; 2]> {
    let [x0, y0, max_x, max_y] = forest.shape.limits();
    let step = rule.trunk_spacing_m;
    assert!(
        step.is_finite() && step > 0.,
        "forest spacing must be finite and positive"
    );
    assert!(
        [x0, y0, max_x, max_y]
            .iter()
            .all(|v| v.is_finite() && (v + step).is_finite() && v + step > *v),
        "forest lattice spacing must advance at every coordinate bound"
    );
    let mut rng = Rng::new(forest_seed(index, forest));
    let mut out = Vec::new();
    let mut y = y0 + step / 2.;
    while y <= max_y {
        let mut x = x0 + step / 2.;
        while x <= max_x {
            if !queries.keep_sampling() {
                return out;
            }
            let jx = (rng.unit() * 2. - 1.) * rule.trunk_jitter * step;
            let jy = (rng.unit() * 2. - 1.) * rule.trunk_jitter * step;
            let p = [x + jx, y + jy];
            let near_open = queries.road_near(p, rule.trunk_clearance_m)
                || queries.water_near(p, rule.trunk_clearance_m);
            let near_prop = queries.body_near(p, rule.trunk_clearance_m);
            if forest.shape.contains(p, 0.)
                && !near_open
                && !near_prop
                && queries.contains_ground(p)
            {
                out.push(p);
            }
            x += step;
        }
        y += step;
    }
    out
}

/// Closed physical box membership for trunk exclusion. Margin grows both
/// local half extents; a circumscribed circle is only a broad-phase bound.
pub fn body_contains(center: [f64; 2], half: [f64; 2], yaw: f64, p: [f64; 2], margin: f64) -> bool {
    let [x, y] = [p[0] - center[0], p[1] - center[1]];
    let (s, c) = libm::sincos(-yaw);
    (x * c - y * s).abs() <= half[0] + margin && (x * s + y * c).abs() <= half[1] + margin
}

/// Whether two trees' crowns meet (Q21): trunks within two crown radii. A
/// tree whose crown meets another's stands in a stand; a lone tree, however
/// big its crown, is no canopy.
pub fn crowns_meet(a: [f64; 2], b: [f64; 2], rule: &ForestRule) -> bool {
    libm::hypot(a[0] - b[0], a[1] - b[1]) <= 2. * rule.canopy_radius_m
}

/// Whether a stand's tree at `tree` puts the ground at `p` under its wood's
/// canopy (Q21): within one trunk spacing of it, so a stand's canopy closes
/// over the gaps its lattice leaves. The ground must also be the wood's own.
pub fn under_stand(tree: [f64; 2], p: [f64; 2], rule: &ForestRule) -> bool {
    libm::hypot(tree[0] - p[0], tree[1] - p[1]) <= rule.trunk_spacing_m
}

/// How far from a tree its standing or falling can change the canopy: the
/// ground under it, and under every tree whose crown meets its own.
pub fn canopy_reach(rule: &ForestRule) -> f64 {
    rule.trunk_spacing_m + 2. * rule.canopy_radius_m
}
