//! The water surface's cells along the exported rivers.
use super::shapes::distance;
use super::{js, RIVER};
use std::collections::HashSet;

/// The water surface is laid as square cells this wide, and reaches this far
/// past the water's edge: the fragment cuts the edge by distance, feathered
/// over a pixel, so the cells only have to cover it.
const CELL_M: f64 = 8.0;
const MARGIN_M: f64 = 4.0;
const SURFACE_Z: usize = 10;

/// How far `(x, y)` lies inside the water's edge of the stretch `r`
/// (negative outside): the half width at the stretch's closest point, which
/// runs linearly between its ends, less the distance to that point.
fn stretch_inside(r: &[f32], x: f64, y: f64) -> f64 {
    let [ax, ay, bx, by, half_a, half_b] = [r[0], r[1], r[2], r[3], r[4], r[5]].map(f64::from);
    let (rx, ry) = (bx - ax, by - ay);
    let length2 = rx * rx + ry * ry;
    let t = if length2 > 0.0 {
        js::min(1.0, js::max(0.0, ((x - ax) * rx + (y - ay) * ry) / length2))
    } else {
        0.0
    };
    let half = half_a + (half_b - half_a) * t;
    half - distance([x, y], [ax + rx * t, ay + ry * t])
}

/// The water surface's square cells, four floats each: the south-west
/// corner, the surface height and the side, in metres. Cells never overlap,
/// so the blended surface is drawn once however the river bends; where
/// rivers meet, the cell is the first river's.
pub fn cells(rivers: &[f32]) -> Vec<f32> {
    let mut out = vec![];
    let mut laid = HashSet::new();
    let cell = CELL_M;
    // A cell is kept when its centre is within the water's reach plus its
    // own half diagonal: every cell the water touches, and a few it does not.
    let slack = MARGIN_M + cell * std::f64::consts::FRAC_1_SQRT_2;
    for r in rivers.as_chunks::<RIVER>().0 {
        let [ax, ay, bx, by, half_a, half_b] = [r[0], r[1], r[2], r[3], r[4], r[5]].map(f64::from);
        let reach = js::max(half_a, half_b) + slack;
        let first = |a: f64, b: f64| ((js::min(a, b) - reach) / cell).floor();
        let last = |a: f64, b: f64| ((js::max(a, b) + reach) / cell).floor();
        let mut j = first(ay, by);
        while j <= last(ay, by) {
            let mut i = first(ax, bx);
            while i <= last(ax, bx) {
                if stretch_inside(r, (i + 0.5) * cell, (j + 0.5) * cell) >= -slack
                    && laid.insert((i as i64, j as i64))
                {
                    out.extend([
                        (i * cell) as f32,
                        (j * cell) as f32,
                        r[SURFACE_Z],
                        cell as f32,
                    ]);
                }
                i += 1.0;
            }
            j += 1.0;
        }
    }
    out
}
