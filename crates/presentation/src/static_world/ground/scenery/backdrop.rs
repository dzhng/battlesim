//! The scenery past the map's edge, where nothing is simulated (the
//! `backdrop` population): hedgerows along the patchwork's plot edges with
//! trees in them, and copses, clear of the map and within the biome's reach,
//! each standing on the land's relief there (`relief::BackdropRelief`).
use super::super::biome::Trees;
use super::super::shapes::{centroid, polygon_contains};
use super::{push, Site, Size, Species, OPEN, SINK_M};
use crate::seed::{stream, Mulberry32};
use std::f64::consts::PI;

pub(super) fn backdrop(
    site: &Site,
    trees: &Trees,
    size: &[Size],
    hedge: usize,
    species: &mut Species,
    seed: u32,
) -> Vec<f32> {
    let mut out = vec![];
    let (clear_m, reach_m) = (trees.backdrop.clear_m, trees.backdrop.reach_m);
    let rules = &trees.hedgerows;
    let source = &site.ground.source;
    let z = |x: f64, y: f64| source.grid.min_height + source.relief.height(x, y) - SINK_M;
    let fits = |x: f64, y: f64, radius: f64| {
        let d = outside_box(source.map, x, y);
        d - radius >= clear_m
            && d <= reach_m
            && source.rendered.is_none_or(|b| {
                x - radius >= b[0] && y - radius >= b[1] && x + radius <= b[2] && y + radius <= b[3]
            })
    };
    let mut tree = |out: &mut Vec<f32>, rng: &mut Mulberry32, x: f64, y: f64| {
        let (kind, tint) = species.pick(rng, x, y, OPEN);
        let sz = rng.float(rules.tree_scale[0], rules.tree_scale[1]);
        let sxy = sz * rng.float(trees.forest.girth[0], trees.forest.girth[1]);
        if fits(x, y, size[kind].radius * sxy) {
            let yaw = rng.float(0.0, PI * 2.0);
            push(
                out,
                [x, y, z(x, y), yaw, sxy, sz, kind as f64],
                tint,
                rng,
                trees.colour_jitter,
            );
        }
    };
    for (k, plot) in site.ground.plots.plots.iter().enumerate() {
        let poly = &plot.outline;
        let n = poly.len() / 2;
        // Skip plots wholly inside the clear band or wholly past reach.
        let (nearest, farthest) =
            poly.as_chunks::<2>()
                .0
                .iter()
                .fold((f64::INFINITY, 0.0f64), |(lo, hi), p| {
                    let d = outside_box(source.map, p[0], p[1]);
                    (lo.min(d), hi.max(d))
                });
        if farthest < clear_m || nearest > reach_m {
            continue;
        }
        let mut rng = stream(seed, 1000 + k as i64);
        // Hedgerows along the plot's edges, a little inside it.
        for i in 0..n {
            let j = (i + 1) % n;
            let (ax, ay, bx, by) = (poly[i * 2], poly[i * 2 + 1], poly[j * 2], poly[j * 2 + 1]);
            let length = libm::hypot(bx - ax, by - ay);
            if rng.sample() >= rules.chance || length < rules.min_edge_m {
                continue;
            }
            let (dx, dy) = ((bx - ax) / length, (by - ay) / length);
            let yaw = libm::atan2(dy, dx);
            let at = |t: f64| {
                (
                    ax + dx * t - dy * rules.inset_m,
                    ay + dy * t + dx * rules.inset_m,
                )
            };
            let mut t = rules.spacing_m / 2.0;
            while t < length - rules.spacing_m / 4.0 {
                let (x, y) = at(t + rng.float(-0.15, 0.15) * rules.spacing_m);
                let sxy = rng.float(0.85, 1.1);
                if fits(x, y, size[hedge].radius * sxy) {
                    let turn = yaw + rng.float(-0.12, 0.12);
                    let sz = rng.float(0.9, 1.3);
                    push(
                        &mut out,
                        [x, y, z(x, y), turn, sxy, sz, hedge as f64],
                        rules.tint,
                        &mut rng,
                        trees.colour_jitter,
                    );
                }
                t += rules.spacing_m;
            }
            let mut t = rng.float(0.0, rules.tree_gap_m[1]);
            while t < length {
                let (x, y) = at(t);
                tree(&mut out, &mut rng, x, y);
                t += rng.float(rules.tree_gap_m[0], rules.tree_gap_m[1]);
            }
        }
        // A copse: a disc of trees about the plot's centre, inside the plot.
        if rng.sample() < trees.copses.chance {
            let [cx, cy] = centroid(poly);
            let radius = rng.float(trees.copses.radius_m[0], trees.copses.radius_m[1]);
            let step = trees.copses.spacing_m;
            let mut y = cy - radius;
            while y <= cy + radius {
                let mut x = cx - radius;
                while x <= cx + radius {
                    let px = x + rng.float(-0.35, 0.35) * step;
                    let py = y + rng.float(-0.35, 0.35) * step;
                    if libm::hypot(px - cx, py - cy) <= radius && polygon_contains(poly, [px, py]) {
                        tree(&mut out, &mut rng, px, py);
                    }
                    x += step;
                }
                y += step;
            }
        }
    }
    out
}

/// Distance from `(x, y)` to `box`, 0 inside.
fn outside_box(b: [f64; 4], x: f64, y: f64) -> f64 {
    libm::hypot(
        (b[0] - x).max(x - b[2]).max(0.0),
        (b[1] - y).max(y - b[3]).max(0.0),
    )
}
