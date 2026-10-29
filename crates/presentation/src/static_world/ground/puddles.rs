//! Puddles: standing water too small to carve (decision 29), look only. A
//! puddle is a pure function of the biome's seed and its cell
//! ([`PuddleGround::candidate`]), so wakes and both clients agree on it
//! whatever has been walked (decision 34). It lies where the biome's
//! chance for the surface under its centre allows, and takes that surface's
//! look (decision 28): mirror on a metalled road or paving, muddy on a
//! country road or a dirt track.
//! Open ground takes puddles only if the biome gives it a chance; summer's
//! doesn't.
use super::biome::{Biome, Puddles, AREA_KINDS};
use super::forests::Forest;
use super::shapes::{distance, stroke_inside, triangle_contains};
use super::{chunks, Source, SURFACE_STROKE, SURFACE_TRIANGLE};
use crate::seed::Key;

/// What the ground under a point is, for a puddle's chance and look: one of
/// the paved kinds (`biome::AREA_KINDS`) or `ground`.
const GROUND: &str = "ground";
/// Parts the puddles' stream from every other population's.
const PUDDLE_SALT: f64 = 7_331.0;
/// A stroke's area kind, its field in `SURFACE_STROKE_STRIDE`.
const STROKE_TAG: usize = 5;
const TRIANGLE_TAG: usize = 6;

/// One puddle: a disc of water standing over the ground.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Puddle {
    pub centre: [f64; 2],
    pub radius_m: f64,
    /// Its water's surface: a hair over the ground at its centre.
    pub surface_z: f64,
    /// Its deepest, at its centre; it shoals to nothing at its rim.
    pub depth_m: f64,
    /// Its look's row among the biome's puddle looks.
    pub look: usize,
}

impl Puddle {
    /// How far `(x, y)` lies inside the puddle's rim (negative outside).
    pub fn inside(&self, x: f64, y: f64) -> f64 {
        self.radius_m - distance([x, y], self.centre)
    }

    /// The water's depth at `(x, y)`: its shape's, not the ground's, since a
    /// puddle carves nothing.
    pub fn depth_at(&self, x: f64, y: f64) -> f64 {
        let r = distance([x, y], self.centre) / self.radius_m;
        self.depth_m * (1.0 - r * r).max(0.0)
    }
}

/// The paved kind (`biome::AREA_KINDS`) under `(x, y)`, or `ground`, and
/// the stroke's stretch that paves it, if a stroke does. Where kinds
/// overlap the earliest wins, as the simulation resolves them
/// (`Surfaces::at`): a road over a track, any carriageway over paving. The
/// tags run in that order.
fn surface<'a>(
    s: &'a Source,
    strokes: &'a [f32],
    x: f64,
    y: f64,
) -> (&'a str, Option<&'a [f32; SURFACE_STROKE]>) {
    let kinds = AREA_KINDS;
    let stroke = chunks::<SURFACE_STROKE>(strokes)
        .iter()
        .filter(|r| stroke_inside(&r[..], [x, y]) >= 0.0)
        .min_by(|a, b| a[STROKE_TAG].total_cmp(&b[STROKE_TAG]));
    let pt = |t: &[f32; SURFACE_TRIANGLE], k: usize| [t[k * 2] as f64, t[k * 2 + 1] as f64];
    let area = chunks::<SURFACE_TRIANGLE>(&s.surface_triangles)
        .iter()
        .filter(|t| triangle_contains(pt(t, 0), pt(t, 1), pt(t, 2), [x, y]))
        .map(|t| t[TRIANGLE_TAG])
        .min_by(f32::total_cmp);
    let (tag, stroke) = match (stroke, area) {
        (Some(r), Some(a)) if a < r[STROKE_TAG] => (Some(a), None),
        (Some(r), _) => (Some(r[STROKE_TAG]), Some(r)),
        (None, a) => (a, None),
    };
    let kind = tag.and_then(|t| kinds.get(t as usize).copied());
    (kind.unwrap_or(GROUND), stroke)
}

/// `centre`, `radius_m` moved into the nearer of the wheel ruts its stroke
/// `r` of kind `kind` carries, and no wider than the rut: rain stands in the
/// ruts, not on the strip between them. Unmoved where the kind has none.
fn in_rut(
    biome: &Biome,
    kind: &str,
    r: &[f32; SURFACE_STROKE],
    centre: [f64; 2],
    radius_m: f64,
) -> ([f64; 2], f64) {
    let Some(ruts) = biome.roads.get(kind).map(|road| &road.ruts) else {
        return (centre, radius_m);
    };
    let Some(&offset) = ruts.offsets_m.first() else {
        return (centre, radius_m);
    };
    let [ax, ay, bx, by] = [r[0], r[1], r[2], r[3]].map(f64::from);
    let (dx, dy) = (bx - ax, by - ay);
    let length = libm::hypot(dx, dy).max(1e-9);
    let t = (((centre[0] - ax) * dx + (centre[1] - ay) * dy) / (length * length)).clamp(0.0, 1.0);
    let (on_x, on_y) = (ax + dx * t, ay + dy * t);
    let (nx, ny) = (-dy / length, dx / length);
    let side = if (centre[0] - on_x) * nx + (centre[1] - on_y) * ny >= 0.0 {
        1.0
    } else {
        -1.0
    };
    (
        [on_x + nx * offset * side, on_y + ny * offset * side],
        radius_m.min(ruts.width_m),
    )
}

/// The ground puddles lie on: `source` with its paved `strokes` as drawn
/// (a country road through a town is a street), its `forests`, its `biome`,
/// and where its other water lies (`in_water`).
pub struct PuddleGround<'a> {
    pub source: &'a Source,
    pub strokes: &'a [f32],
    pub forests: &'a [Forest],
    pub biome: &'a Biome,
    pub in_water: &'a dyn Fn(f64, f64) -> bool,
}

impl PuddleGround<'_> {
    /// The puddle cell `(i, j)` holds, if any: a centre and a size drawn
    /// from the biome's seed and the cell alone, kept by the chance the
    /// surface under the centre gives, in its look. Nothing in a forest, in
    /// other water or off the map, and none moved by its rut onto another
    /// surface.
    pub fn candidate(&self, rules: &Puddles, i: i64, j: i64) -> Option<Puddle> {
        let s = self.source;
        let mut rng = Key::new()
            .num(PUDDLE_SALT)
            .num(self.biome.seed as f64)
            .num(i as f64)
            .num(j as f64)
            .rng();
        let cell = rules.cell_m;
        let centre = [
            (i as f64 + rng.sample()) * cell,
            (j as f64 + rng.sample()) * cell,
        ];
        let radius_m = rng.float(rules.radius_m[0], rules.radius_m[1]);
        let roll = rng.sample();
        let [x0, y0, x1, y1] = s.map;
        let on_map = centre[0] - radius_m >= x0
            && centre[0] + radius_m <= x1
            && centre[1] - radius_m >= y0
            && centre[1] + radius_m <= y1;
        let wooded = |c: [f64; 2]| {
            self.forests
                .iter()
                .any(|f| f.inside(s, c[0], c[1]) > -radius_m)
        };
        if !on_map || wooded(centre) {
            return None;
        }
        let (kind, stroke) = surface(s, self.strokes, centre[0], centre[1]);
        let on = rules.on.get(kind)?;
        let (centre, radius_m) = match stroke {
            Some(r) => in_rut(self.biome, kind, r, centre, radius_m),
            None => (centre, radius_m),
        };
        // Its rut keeps it on its own surface, and no part of it lies in
        // other water: its centre and its rim all round.
        if surface(s, self.strokes, centre[0], centre[1]).0 != kind {
            return None;
        }
        let rim = (0..8).map(|k| {
            let a = k as f64 * std::f64::consts::FRAC_PI_4;
            [
                centre[0] + radius_m * libm::cos(a),
                centre[1] + radius_m * libm::sin(a),
            ]
        });
        if std::iter::once(centre)
            .chain(rim)
            .any(|[x, y]| (self.in_water)(x, y))
        {
            return None;
        }
        (roll < on.chance).then(|| Puddle {
            centre,
            radius_m,
            surface_z: s.grid.height(centre[0], centre[1]) + rules.lift_m,
            depth_m: rules.depth_m,
            look: rules
                .looks
                .iter()
                .position(|l| l.name == on.look)
                .expect("validation names a look for every surface"),
        })
    }

    /// Every puddle on the map, cell by cell; each cell's is its own
    /// (`candidate`), so the order this walks in decides nothing.
    pub fn puddles(&self) -> Vec<Puddle> {
        let Some(rules) = self.biome.water.puddles.as_ref() else {
            return vec![];
        };
        let [x0, y0, x1, y1] = self.source.map;
        let cell = rules.cell_m;
        let mut out = vec![];
        for j in (y0 / cell).floor() as i64..=(y1 / cell).floor() as i64 {
            for i in (x0 / cell).floor() as i64..=(x1 / cell).floor() as i64 {
                out.extend(self.candidate(rules, i, j));
            }
        }
        out
    }
}

/// The puddle `(x, y)` lies in, among `puddles`, if any.
pub fn puddle_at(puddles: &[Puddle], x: f64, y: f64) -> Option<&Puddle> {
    puddles.iter().find(|p| p.inside(x, y) >= 0.0)
}
