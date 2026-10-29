//! The standing water: where the exported rivers and swamps and the puddles
//! hold it, how deep it stands and which look it draws ([`StandingWater`],
//! the one owner), and the water surface's cells over it.
use super::biome::{Biome, WaterLook};
use super::puddles::{puddle_at, Puddle};
use super::shapes::distance;
use super::tables::Grid;
use super::RIVER;
use crate::seed::{stream, weighted};
use sim::world::export::SWAMP_STRIDE as SWAMP;
use std::collections::HashSet;

/// The water surface is laid as square cells this wide, and reaches this far
/// past the water's edge: the fragment fades the edge in by depth, so the
/// cells only have to cover it.
const CELL_M: f64 = 8.0;
const MARGIN_M: f64 = 4.0;
const SURFACE_Z: usize = 10;
const RIVER_INDEX: usize = 11;
/// A swamp edge's fields (`SWAMP_STRIDE`): its swamp and its surface.
const SWAMP_INDEX: usize = 4;
const SWAMP_SURFACE_Z: usize = 5;
/// Part each kind of water's look pick from every other population's.
const LOOK_SALT: u32 = 0x10c4_a7e5;
const SWAMP_LOOK_SALT: u32 = 0x5a3b_9e21;
/// Floats a water cell (`cells`).
pub const CELL_FLOATS: usize = 8;

/// A kind of standing water: each picks its looks from its own list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    River,
    Swamp,
}

/// The look water feature `feature` of kind `kind` (its index among the
/// map's rivers or swamps) draws: one of `looks` by weight, seeded by the
/// biome's `seed`, the kind and the feature alone, as a tree stand is by its
/// cell, so no other feature moves it. A look of no weight is never drawn.
pub fn look(seed: u32, kind: Kind, feature: usize, looks: &[WaterLook]) -> usize {
    let salt = match kind {
        Kind::River => LOOK_SALT,
        Kind::Swamp => SWAMP_LOOK_SALT,
    };
    weighted(
        &mut stream(seed ^ salt, feature as i64),
        looks.iter().map(|l| l.weight),
    )
}

/// How far `(x, y)` lies inside the water's edge of the stretch `r`
/// (negative outside): the half width at the stretch's closest point, which
/// runs linearly between its ends, less the distance to that point.
pub(super) fn stretch_inside(r: &[f32], x: f64, y: f64) -> f64 {
    let [ax, ay, bx, by, half_a, half_b] = [r[0], r[1], r[2], r[3], r[4], r[5]].map(f64::from);
    let (rx, ry) = (bx - ax, by - ay);
    let length2 = rx * rx + ry * ry;
    let t = if length2 > 0.0 {
        (((x - ax) * rx + (y - ay) * ry) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let half = half_a + (half_b - half_a) * t;
    half - distance([x, y], [ax + rx * t, ay + ry * t])
}

/// One swamp's outline and water.
struct Swamp {
    /// Its outline's edges: ax, ay, bx, by.
    edges: Vec<[f64; 4]>,
    surface_z: f64,
}

impl Swamp {
    /// Every swamp in the exported outlines `edges` (`SWAMP_STRIDE` each),
    /// in map order.
    fn all(edges: &[f32]) -> Vec<Swamp> {
        let mut out: Vec<Swamp> = vec![];
        for e in edges.as_chunks::<SWAMP>().0 {
            let index = e[SWAMP_INDEX] as usize;
            if out.len() <= index {
                out.resize_with(index + 1, || Swamp {
                    edges: vec![],
                    surface_z: f64::from(e[SWAMP_SURFACE_Z]),
                });
            }
            out[index]
                .edges
                .push([e[0], e[1], e[2], e[3]].map(f64::from));
        }
        out
    }

    /// How far `(x, y)` lies inside the outline (negative outside): the
    /// distance to its nearest edge, signed by whether its edges cross a ray
    /// from the point an odd number of times.
    fn inside(&self, x: f64, y: f64) -> f64 {
        let mut nearest = f64::INFINITY;
        let mut within = false;
        for &[ax, ay, bx, by] in &self.edges {
            if (ay > y) != (by > y) && x < ax + (y - ay) * (bx - ax) / (by - ay) {
                within = !within;
            }
            let (rx, ry) = (bx - ax, by - ay);
            let t =
                (((x - ax) * rx + (y - ay) * ry) / (rx * rx + ry * ry).max(1e-12)).clamp(0.0, 1.0);
            nearest = nearest.min(distance([x, y], [ax + rx * t, ay + ry * t]));
        }
        if within {
            nearest
        } else {
            -nearest
        }
    }

    /// The outline's bounds: min x, min y, max x, max y.
    fn bounds(&self) -> [f64; 4] {
        self.edges.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, e| {
                [
                    b[0].min(e[0]),
                    b[1].min(e[1]),
                    b[2].max(e[0]),
                    b[3].max(e[1]),
                ]
            },
        )
    }
}

/// Standing water at a point: its surface's height, how deep it stands over
/// the ground there, and its look (a row of `TerrainParams.waterLooks`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wet {
    pub surface_z: f64,
    pub depth_m: f64,
    pub look: usize,
}

/// Where the ground holds standing water, from the exported rivers and
/// swamps: what the water surface is drawn over, and what stands in it. A
/// river's looks come first in the look rows, then a swamp's.
pub struct StandingWater<'a> {
    rivers: &'a [f32],
    swamps: Vec<Swamp>,
    puddles: &'a [Puddle],
    grid: &'a Grid,
    biome: &'a Biome,
}

impl<'a> StandingWater<'a> {
    /// The water of the exported `rivers` and `swamps`, and of `puddles`,
    /// over the ground `grid`, dressed by `biome`.
    pub fn new(
        rivers: &'a [f32],
        swamps: &[f32],
        puddles: &'a [Puddle],
        grid: &'a Grid,
        biome: &'a Biome,
    ) -> Self {
        Self {
            rivers,
            swamps: Swamp::all(swamps),
            puddles,
            grid,
            biome,
        }
    }

    /// The look row puddle `p` draws: after every river and swamp look.
    fn puddle_look(&self, p: &Puddle) -> usize {
        let w = &self.biome.water;
        w.rivers.len() + w.swamps.len() + p.look
    }

    /// The look row river `river` draws.
    fn river_look(&self, river: usize) -> usize {
        look(
            self.biome.seed,
            Kind::River,
            river,
            &self.biome.water.rivers,
        )
    }

    /// The look row swamp `swamp` draws: after every river look.
    fn swamp_look(&self, swamp: usize) -> usize {
        let w = &self.biome.water;
        w.rivers.len() + look(self.biome.seed, Kind::Swamp, swamp, &w.swamps)
    }

    /// The standing water at `(x, y)`, if any: a river's where the point is
    /// within its water (the first river's where two meet), else a swamp's
    /// inside its outline, where its surface stands over the ground, else a
    /// puddle's.
    pub fn at(&self, x: f64, y: f64) -> Option<Wet> {
        let wet = |surface_z: f64, look| {
            let depth_m = surface_z - self.grid.height(x, y);
            (depth_m > 0.0).then_some(Wet {
                surface_z,
                depth_m,
                look,
            })
        };
        if let Some(r) = self
            .rivers
            .as_chunks::<RIVER>()
            .0
            .iter()
            .find(|r| stretch_inside(&r[..], x, y) >= 0.0)
        {
            return wet(
                f64::from(r[SURFACE_Z]),
                self.river_look(r[RIVER_INDEX] as usize),
            );
        }
        if let Some((k, s)) = self
            .swamps
            .iter()
            .enumerate()
            .find(|(_, s)| s.inside(x, y) >= 0.0)
        {
            return wet(s.surface_z, self.swamp_look(k));
        }
        // A puddle carves nothing: its depth is its shape's.
        let p = puddle_at(self.puddles, x, y)?;
        let depth_m = p.depth_at(x, y);
        (depth_m > 0.0).then_some(Wet {
            surface_z: p.surface_z,
            depth_m,
            look: self.puddle_look(p),
        })
    }

    /// The plot kind the standing water at `(x, y)` floods, by name: its
    /// swamp look's `ground`. None on dry ground, and in a river, which
    /// floods no plot (its bed is the water's own).
    pub fn flooded(&self, x: f64, y: f64) -> Option<&'a str> {
        let w = &self.biome.water;
        let look = self.at(x, y)?.look.checked_sub(w.rivers.len())?;
        w.swamps.get(look)?.ground.as_deref()
    }

    /// The water something stands in with its feet or hull at `at`: the
    /// standing water there, if its surface is above them. A deck over a
    /// river, or anything else above the water, is dry.
    pub fn under(&self, [x, y, z]: [f64; 3]) -> Option<Wet> {
        self.at(x, y).filter(|w| z < w.surface_z)
    }

    /// The water surface's square cells (`CELL_FLOATS` each): the
    /// south-west corner, the surface height and the side, in metres, the
    /// look it draws, then a puddle's centre and radius (zeros for a river's
    /// or a swamp's, whose depth is the ground's). A river's and a swamp's
    /// cells never overlap, so the blended surface is drawn once however the
    /// water bends; where two waters meet, the cell is the first one's
    /// (rivers before swamps), its surface and its look. A puddle is one cell
    /// over its disc; no puddle lies in other water.
    pub fn cells(&self) -> Vec<f32> {
        let mut out = vec![];
        let mut laid = HashSet::new();
        let cell = CELL_M;
        // A cell is kept when its centre is within the water's reach plus
        // its own half diagonal: every cell the water touches, and a few it
        // does not.
        let slack = MARGIN_M + cell * std::f64::consts::FRAC_1_SQRT_2;
        let mut lay =
            |[x0, y0, x1, y1]: [f64; 4], inside: &dyn Fn(f64, f64) -> f64, z: f64, look: usize| {
                let span = |a: f64, b: f64| {
                    ((a - slack) / cell).floor() as i64..=((b + slack) / cell).floor() as i64
                };
                for j in span(y0, y1) {
                    for i in span(x0, x1) {
                        let (x, y) = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
                        if inside(x, y) >= -slack && laid.insert((i, j)) {
                            let corner = [i as f64 * cell, j as f64 * cell];
                            out.extend([
                                corner[0] as f32,
                                corner[1] as f32,
                                z as f32,
                                cell as f32,
                                look as f32,
                                0.0,
                                0.0,
                                0.0,
                            ]);
                        }
                    }
                }
            };
        for r in self.rivers.as_chunks::<RIVER>().0 {
            let [ax, ay, bx, by, half_a, half_b] =
                [r[0], r[1], r[2], r[3], r[4], r[5]].map(f64::from);
            let half = half_a.max(half_b);
            let bounds = [
                ax.min(bx) - half,
                ay.min(by) - half,
                ax.max(bx) + half,
                ay.max(by) + half,
            ];
            let inside = |x, y| stretch_inside(&r[..], x, y);
            lay(
                bounds,
                &inside,
                f64::from(r[SURFACE_Z]),
                self.river_look(r[RIVER_INDEX] as usize),
            );
        }
        for (k, s) in self.swamps.iter().enumerate() {
            let inside = |x, y| s.inside(x, y);
            lay(s.bounds(), &inside, s.surface_z, self.swamp_look(k));
        }
        // A puddle is one cell over its disc, which carries its centre and
        // radius: its depth is its shape's (`Puddle::depth_at`), drawn so.
        for p in self.puddles {
            let [cx, cy] = p.centre;
            let r = p.radius_m;
            out.extend(
                [
                    cx - r,
                    cy - r,
                    p.surface_z,
                    2.0 * r,
                    self.puddle_look(p) as f64,
                    cx,
                    cy,
                    r,
                ]
                .map(|v| v as f32),
            );
        }
        out
    }
}
