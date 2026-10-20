//! Conservative projected bounds for asset-opted static cards.
use crate::{static_world::walk::Record, view::View, world::Corners};

/// Full fitted bounds, including an off-centre authored origin.
pub(super) fn card_bounds(corners: &Corners, record: &Record) -> [f64; 6] {
    let mut bounds = [f64::INFINITY; 6];
    bounds[3..].fill(f64::NEG_INFINITY);
    let c = libm::cos(record[3] as f64);
    let s = libm::sin(record[3] as f64);
    for k in 0..8 {
        let p: [f64; 3] = std::array::from_fn(|a| {
            (if k & (1 << a) == 0 {
                corners.min[a]
            } else {
                corners.max[a]
            }) * record[12 + a] as f64
        });
        let world = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]];
        for a in 0..3 {
            let v = world[a] + record[a] as f64;
            bounds[a] = bounds[a].min(v);
            bounds[a + 3] = bounds[a + 3].max(v);
        }
    }
    bounds
}

/// Perspective extents of all eight bounds corners. A box crossing the eye
/// plane cannot be small. The enclosing world box errs towards keeping meshes.
pub(super) fn projected_bounds_px(view: &View, bounds: &[f64; 6]) -> f64 {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    let m = &view.view;
    for k in 0..8 {
        let p: [f64; 3] =
            std::array::from_fn(|a| bounds[a + if k & (1 << a) == 0 { 0 } else { 3 }]);
        let v: [f64; 3] = std::array::from_fn(|a| {
            m[a] as f64 * p[0] + m[4 + a] as f64 * p[1] + m[8 + a] as f64 * p[2] + m[12 + a] as f64
        });
        if v[2] >= -1e-3 {
            return f64::INFINITY;
        }
        for a in 0..2 {
            let px = v[a] * view.pixels_per_metre / -v[2];
            lo[a] = lo[a].min(px);
            hi[a] = hi[a].max(px);
        }
    }
    (hi[0] - lo[0]).max(hi[1] - lo[1])
}
