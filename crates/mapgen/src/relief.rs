//! Bounds on a map's relief over boxes, for steps that must know how high
//! or low the land can be somewhere without sampling it: per tile, the sum
//! of the most (and least) each feature can add anywhere in it, kept in
//! max/min pyramids so a box of any size costs a few reads. Every bound
//! holds for every point of the box, so it also holds for the height
//! field's triangles once a caller grows the box by a grid diagonal.
use contract::map::Relief;

/// A box's corners, `[x0, y0, x1, y1]`.
pub type Bounds = [f64; 4];

/// The least and most `feature` adds anywhere in `b`.
pub fn feature_span(feature: &Relief, b: Bounds) -> [f64; 2] {
    let corners = [[b[0], b[1]], [b[2], b[1]], [b[0], b[3]], [b[2], b[3]]];
    match *feature {
        Relief::Ridge {
            center, radius_m, ..
        } => {
            // Its height falls (or rises, for a dip) with distance alone.
            let near = libm::hypot(
                (b[0] - center[0]).max(center[0] - b[2]).max(0.0),
                (b[1] - center[1]).max(center[1] - b[3]).max(0.0),
            );
            let far = corners
                .iter()
                .map(|c| libm::hypot(c[0] - center[0], c[1] - center[1]))
                .fold(0.0, f64::max);
            let at = |d: f64| feature.height_at(center[0] + d.min(radius_m), center[1]);
            let (a, z) = (at(near), at(far));
            [a.min(z), a.max(z)]
        }
        Relief::Mesa {
            rect,
            height_m,
            side_degrees,
        } => {
            // Its height falls with the distance from its rect, which is
            // least at the boxes' nearest points and most at a corner.
            let gap = libm::hypot(
                (rect[0] - b[2]).max(b[0] - (rect[0] + rect[2])).max(0.0),
                (rect[1] - b[3]).max(b[1] - (rect[1] + rect[3])).max(0.0),
            );
            let most = (height_m - gap * libm::tan(side_degrees.to_radians())).max(0.0);
            let least = corners
                .iter()
                .map(|c| feature.height_at(c[0], c[1]))
                .fold(f64::INFINITY, f64::min);
            [least.min(most), most]
        }
    }
}

/// The least and most all of `relief` adds anywhere in `b`: the sums of
/// each feature's own, so overlapping features are never under-counted.
pub fn span(relief: &[Relief], b: Bounds) -> [f64; 2] {
    relief.iter().fold([0.0, 0.0], |[lo, hi], f| {
        let [a, z] = feature_span(f, b);
        [lo + a, hi + z]
    })
}

struct Level {
    tile: f64,
    nx: usize,
    ny: usize,
    /// Per tile, row-major: the least and most there.
    spans: Vec<[f64; 2]>,
}

/// Relief bounds over a map `size`, from tiles `tile` metres across.
pub struct ReliefBound {
    levels: Vec<Level>,
}

impl ReliefBound {
    pub fn new(relief: &[Relief], size: [f64; 2], tile: f64) -> Self {
        let nx = (libm::ceil(size[0] / tile) as usize).max(1);
        let ny = (libm::ceil(size[1] / tile) as usize).max(1);
        let mut spans = vec![[0.0, 0.0]; nx * ny];
        for f in relief {
            let reach = match f.footprint().0 {
                Some(r) => [r[0], r[1], r[0] + r[2], r[1] + r[3]],
                None => [0.0, 0.0, size[0], size[1]],
            };
            let at = |v: f64, n: usize| ((v / tile).floor().max(0.0) as usize).min(n - 1);
            for y in at(reach[1], ny)..=at(reach[3], ny) {
                for x in at(reach[0], nx)..=at(reach[2], nx) {
                    let b = [
                        x as f64 * tile,
                        y as f64 * tile,
                        (x + 1) as f64 * tile,
                        (y + 1) as f64 * tile,
                    ];
                    let [a, z] = feature_span(f, b);
                    let s = &mut spans[y * nx + x];
                    *s = [s[0] + a, s[1] + z];
                }
            }
        }
        let mut levels = vec![Level {
            tile,
            nx,
            ny,
            spans,
        }];
        while let Some(last) = levels.last().filter(|l| l.nx > 1 || l.ny > 1) {
            let (nx, ny) = (last.nx.div_ceil(2), last.ny.div_ceil(2));
            let mut spans = vec![[f64::INFINITY, f64::NEG_INFINITY]; nx * ny];
            for y in 0..last.ny {
                for x in 0..last.nx {
                    let [a, z] = last.spans[y * last.nx + x];
                    let s = &mut spans[(y / 2) * nx + x / 2];
                    *s = [s[0].min(a), s[1].max(z)];
                }
            }
            levels.push(Level {
                tile: last.tile * 2.0,
                nx,
                ny,
                spans,
            });
        }
        Self { levels }
    }

    /// The least and most the relief adds anywhere in `b` (clipped to the
    /// map): from the finest level at which the box spans at most seven
    /// tiles a side, whose coarser tiles only widen the span.
    pub fn span(&self, b: Bounds) -> [f64; 2] {
        let level = self
            .levels
            .iter()
            .find(|l| (b[2] - b[0]).max(b[3] - b[1]) <= 6.0 * l.tile)
            .unwrap_or(self.levels.last().unwrap());
        let at = |v: f64, n: usize| ((v / level.tile).floor().max(0.0) as usize).min(n - 1);
        let mut out = [f64::INFINITY, f64::NEG_INFINITY];
        for y in at(b[1], level.ny)..=at(b[3], level.ny) {
            for x in at(b[0], level.nx)..=at(b[2], level.nx) {
                let [a, z] = level.spans[y * level.nx + x];
                out = [out[0].min(a), out[1].max(z)];
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_hold_every_sampled_height() {
        let relief = vec![
            Relief::Ridge {
                center: [300.0, 420.0],
                peak_m: 7.0,
                radius_m: 180.0,
            },
            Relief::Ridge {
                center: [420.0, 380.0],
                peak_m: 5.0,
                radius_m: 120.0,
            },
            Relief::Mesa {
                rect: [700.0, 100.0, 60.0, 40.0],
                height_m: 4.0,
                side_degrees: 20.0,
            },
        ];
        let bound = ReliefBound::new(&relief, [1000.0, 1000.0], 64.0);
        for b in [
            [250.0, 380.0, 330.0, 470.0],
            [0.0, 0.0, 1000.0, 1000.0],
            [650.0, 60.0, 820.0, 180.0],
            [900.0, 900.0, 990.0, 990.0],
        ] {
            let [lo, hi] = bound.span(b);
            let [elo, ehi] = span(&relief, b);
            for j in 0..=20 {
                for i in 0..=20 {
                    let x = b[0] + (b[2] - b[0]) * i as f64 / 20.0;
                    let y = b[1] + (b[3] - b[1]) * j as f64 / 20.0;
                    let h = contract::map::relief_height(&relief, x, y);
                    assert!(h >= lo - 1e-9 && h <= hi + 1e-9, "{h} outside {lo}..{hi}");
                    assert!(h >= elo - 1e-9 && h <= ehi + 1e-9);
                }
            }
        }
        assert_eq!(bound.span([900.0, 900.0, 990.0, 990.0]), [0.0, 0.0]);
    }
}
