//! Where every tree, shrub and piece of forest-floor dressing stands.
//! Placement is code; the instanced unit is an appearance, sized here only
//! by its unscaled height and crown reach (`Size`). Four populations, drawn
//! alike but owned differently:
//! - `forest`: the simulation's tree bodies, exactly one tree on each trunk.
//!   A forest's trunks are placed by the one forest rule, each crown under
//!   its forest's canopy over the simulation's own ground and within the
//!   canopy's radius of its trunk; a trunk outside every forest (a street's
//!   tree) is a tree of its own, as tall as its body (`trees.lone`). Which
//!   species is the biome's: a wood is stands, each mostly one family's.
//! - `understorey`: rows of hedge under a tree line (a strip of forest), each
//!   shrub's whole reach on the strip's own ground, off paving, water and
//!   other bodies: the simulation lets no sight across a strip.
//! - `backdrop`: past the map edge, where nothing is simulated: hedgerows
//!   along the patchwork's plot edges with trees in them, and copses.
//! - `dressing`: what lies on the forest floors with no body of its own,
//!   inside the edge the floor's verge wanders about, clear of trunks,
//!   bodies, paving and water, each kind in drifts. It is laid a cell of
//!   ground at a time, seeded by the cell, and only when asked for
//!   (`Scenery::dress`): a map's forests hold too many pieces to lay whole.
//!
//! Every record is `x, y, z, yaw, scale xy, scale z, kind, r, g, b`: `kind`
//! indexes `kinds`, the scales apply to the appearance's own size, the
//! colour multiplies its albedo. The float order is the page's TypeScript's.
use super::biome::{Biome, Trees};
use super::field::{Field, FOOTPRINT_M};
use super::forests::Kind;
use super::shapes::{centroid, polygon_contains};
use super::{chunks, Ground, FOREST_STROKE, FOREST_TRIANGLE, PROP};
use crate::seed::{stream, weighted, Mulberry32};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

pub const FLOATS: usize = 10;
/// The side of a cell of dressing: the ground is dressed, kept and drawn a
/// cell at a time.
pub const DRESSING_CELL_M: f64 = 64.0;
/// Trunks sink this far below the lowest ground around their foot; crown
/// tops keep this far under the canopy.
const SINK_M: f64 = 0.05;
const CANOPY_MARGIN_M: f64 = 0.05;
/// Seeds' salts, parting each population's streams from every other.
const STAND_SALT: u32 = 0x57a2d5;
const LONE_SALT: u32 = 0x10e7ee;
const UNDERSTOREY_SALT: u32 = 0x5b2ab5;
const DRESSING_SALT: u32 = 0xd2e551;
/// A shrub turns this far from its row's line, radians, either way.
const SHRUB_TURN: f64 = 0.12;
/// The side of the cells trunks and bodies are bucketed in for the dressing.
const CLEAR_CELL_M: f64 = 8.0;
/// No forest interior: a strip, or scenery past the map.
const OPEN: f64 = f64::NEG_INFINITY;

/// An appearance's unscaled size: its top, and its farthest reach from the
/// trunk's axis.
#[derive(Clone, Copy)]
pub struct Size {
    pub height: f64,
    pub radius: f64,
}

/// One number for cell `(i, j)` of a grid over the map.
fn cell_key(i: i64, j: i64) -> i64 {
    (j + 0x8000) * 0x10000 + (i + 0x8000)
}

fn push(out: &mut Vec<f32>, at: [f64; 7], tint: [f64; 3], rng: &mut Mulberry32, jitter: f64) {
    let v = rng.float(-1.0, 1.0) * jitter;
    let w = rng.float(-1.0, 1.0) * jitter * 0.5;
    out.extend(at.map(|f| f as f32));
    out.extend(
        [
            tint[0] * (1.0 + v) * (1.0 + w),
            tint[1] * (1.0 + v),
            tint[2] * (1.0 + v) * (1.0 - w),
        ]
        .map(|f| f as f32),
    );
}

/// Distance from `(x, y)` to `box`, 0 inside.
fn outside_box(b: [f64; 4], x: f64, y: f64) -> f64 {
    libm::hypot(
        (b[0] - x).max(x - b[2]).max(0.0),
        (b[1] - y).max(y - b[3]).max(0.0),
    )
}

/// `simplex2d` of the `math` package, as seeded by `create(seed)`.
struct Simplex {
    perm: [u8; 512],
    grad: [[f64; 2]; 512],
}

impl Simplex {
    fn new(seed: i32) -> Simplex {
        const P: [u8; 256] = [
            151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103,
            30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197,
            62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20,
            125, 136, 171, 168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83,
            111, 229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
            65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135,
            130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64, 52, 217, 226, 250, 124,
            123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58, 17,
            182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153,
            101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178,
            185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
            241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184,
            84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29,
            24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
        ];
        const GRAD: [[f64; 2]; 12] = [
            [1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [0.0, 1.0],
            [0.0, -1.0],
            [0.0, 1.0],
            [0.0, -1.0],
        ];
        let seed = if seed < 256 {
            seed | seed.wrapping_shl(8)
        } else {
            seed
        };
        let mut s = Simplex {
            perm: [0; 512],
            grad: [[0.0; 2]; 512],
        };
        for (i, p) in P.iter().enumerate() {
            let v = p
                ^ (if i & 1 == 1 {
                    seed & 255
                } else {
                    (seed >> 8) & 255
                }) as u8;
            (s.perm[i], s.perm[i + 256]) = (v, v);
            (s.grad[i], s.grad[i + 256]) = (GRAD[v as usize % 12], GRAD[v as usize % 12]);
        }
        s
    }

    /// The noise at `(x, y)`, in [-1, 1].
    fn sample(&self, x: f64, y: f64) -> f64 {
        let f2 = 0.5 * (3f64.sqrt() - 1.0);
        let g2 = (3.0 - 3f64.sqrt()) / 6.0;
        let s = (x + y) * f2;
        let (i, j) = ((x + s).floor(), (y + s).floor());
        let t = (i + j) * g2;
        let (x0, y0) = (x - i + t, y - j + t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let corners = [
            (x0, y0, 0, 0),
            (x0 - i1 as f64 + g2, y0 - j1 as f64 + g2, i1, j1),
            (x0 - 1.0 + 2.0 * g2, y0 - 1.0 + 2.0 * g2, 1, 1),
        ];
        let (i, j) = (
            (i as i64 as i32 & 255) as usize,
            (j as i64 as i32 & 255) as usize,
        );
        let mut n = [0.0; 3];
        for (k, &(cx, cy, di, dj)) in corners.iter().enumerate() {
            let mut t = 0.5 - cx * cx - cy * cy;
            if t >= 0.0 {
                t *= t;
                let g = self.grad[i + di + self.perm[j + dj] as usize];
                n[k] = t * t * (g[0] * cx + g[1] * cy);
            }
        }
        70.0 * (n[0] + n[1] + n[2])
    }
}

/// How thick a kind's drift lies, 0 to 1, from its field's value in
/// [-1, 1]: half the ground lies in a drift, with a short ramp at its edge.
fn drifted(value: f64) -> f64 {
    (value / 0.3 + 0.5).clamp(0.0, 1.0)
}

/// A tree's species: its kind and tint.
type Pick = (usize, [f64; 3]);

/// The biome's mix of species over the ground. The land is cut into stands,
/// the cells nearest seeded points `stands.size_m` apart, each one family's
/// (drawn by the families' weights). A tree is first, by weight, the odd one
/// of no family, where that species may stand; otherwise of its stand's
/// family (`stands.purity` of the time, else of any), a species of it by
/// weight.
struct Species {
    /// Per species: kind, tint, weight, interior; families' members and weights.
    rows: Vec<(usize, [f64; 3], f64, Option<f64>)>,
    odd: Vec<usize>,
    families: Vec<(f64, Vec<usize>)>,
    size_m: f64,
    purity: f64,
    seed: u32,
    points: HashMap<i64, (f64, f64, usize)>,
}

impl Species {
    fn new(trees: &Trees, kinds: &[String], seed: u32) -> Species {
        let kind = |name: &str| kinds.iter().position(|k| k == name).unwrap();
        let rows = trees
            .species
            .iter()
            .map(|s| (kind(&s.appearance), s.tint, s.weight, s.interior_m))
            .collect();
        let mut families: Vec<(&str, f64, Vec<usize>)> = vec![];
        let mut odd = vec![];
        for (i, s) in trees.species.iter().enumerate() {
            let Some(family) = &s.family else {
                odd.push(i);
                continue;
            };
            match families.iter_mut().find(|f| f.0 == family) {
                Some(f) => {
                    f.1 += s.weight;
                    f.2.push(i);
                }
                None => families.push((family, s.weight, vec![i])),
            }
        }
        Species {
            rows,
            odd,
            families: families.into_iter().map(|(_, w, m)| (w, m)).collect(),
            size_m: trees.stands.size_m,
            purity: trees.stands.purity,
            seed,
            points: HashMap::new(),
        }
    }

    /// The seeded point of stand cell `(i, j)`, and its family.
    fn point(&mut self, i: i64, j: i64) -> (f64, f64, usize) {
        let key = (i + 0x8000) * 0x10000 + (j + 0x8000);
        let (size, seed) = (self.size_m, self.seed);
        let families = &self.families;
        *self.points.entry(key).or_insert_with(|| {
            let mut rng = stream(seed ^ STAND_SALT, key);
            let x = (i as f64 + rng.float(0.0, 1.0)) * size;
            let y = (j as f64 + rng.float(0.0, 1.0)) * size;
            (x, y, weighted(&mut rng, families.iter().map(|f| f.0)))
        })
    }

    fn stand(&mut self, x: f64, y: f64) -> usize {
        let (ci, cj) = (
            (x / self.size_m).floor() as i64,
            (y / self.size_m).floor() as i64,
        );
        let (mut nearest, mut best) = (self.point(ci, cj).2, f64::INFINITY);
        for j in cj - 1..=cj + 1 {
            for i in ci - 1..=ci + 1 {
                let p = self.point(i, j);
                let d = libm::pow(p.0 - x, 2.0) + libm::pow(p.1 - y, 2.0);
                if d < best {
                    (nearest, best) = (p.2, d);
                }
            }
        }
        nearest
    }

    /// The species a tree at `(x, y)` is drawn as, standing `inside` metres
    /// within its forest's edge (`OPEN` where it has none).
    fn pick(&mut self, rng: &mut Mulberry32, x: f64, y: f64, inside: f64) -> Pick {
        let total = self.rows.iter().fold(0.0, |a, r| a + r.2);
        let mut r = rng.float(0.0, total);
        for &s in &self.odd {
            r -= self.rows[s].2;
            if r < 0.0 {
                if inside >= self.rows[s].3.unwrap_or(OPEN) {
                    return (self.rows[s].0, self.rows[s].1);
                }
                break;
            }
        }
        let family = if rng.sample() < self.purity {
            self.stand(x, y)
        } else {
            weighted(rng, self.families.iter().map(|f| f.0))
        };
        let members = &self.families[family].1;
        let s = members[weighted(rng, members.iter().map(|&m| self.rows[m].2))];
        (self.rows[s].0, self.rows[s].1)
    }
}

/// The scenery of a map: the three populations placed whole, and what the
/// dressing is laid from a cell at a time.
pub struct Scenery {
    /// Appearance names, indexed by each record's `kind`.
    pub kinds: Vec<String>,
    pub forest: Vec<f32>,
    /// Per `forest` tree: the simulation's trunk it stands on (a prop id),
    /// so a published fall finds the tree it felled.
    pub forest_ids: Vec<u32>,
    pub understorey: Vec<f32>,
    pub backdrop: Vec<f32>,
    pub dressing: Dressing,
}

/// The forest floors' dressing, laid a cell of ground at a time.
pub struct Dressing {
    /// The most pieces a cell holds.
    pub capacity: usize,
    /// The appearances it lays, as indices into `kinds`, in first-use order.
    pub kinds: Vec<usize>,
    /// The tallest any piece stands and the farthest any reaches from its
    /// foot; the ground's lowest and highest points.
    pub top_m: f64,
    pub reach_m: f64,
    pub ground: [f64; 2],
    /// The cells a forest's primitives reach, `i, j` pairs.
    pub cells: Vec<i32>,
    /// Per kind row: its kind, its weight, its drift field.
    rows: Vec<(usize, Simplex)>,
    /// The trunks' and bodies' clearance circles by `CLEAR_CELL_M` cell.
    clear: HashMap<i64, Vec<[f64; 3]>>,
    /// The ground's rules read out to the dressing's margins.
    field: Field,
}

/// What placement reads of the static world.
struct Site<'a> {
    ground: &'a Ground,
    field: Field,
    trunks: Vec<[f32; 2]>,
    trunk_ids: Vec<u32>,
    trunk_heights: Vec<f32>,
    /// Every other prop's footprint circle, `x, y, radius`.
    obstacles: Vec<[f32; 3]>,
    /// The foliage export's columns and cell, and the cells foliage covers.
    foliage: (i64, f64, HashSet<i64>),
}

impl Site<'_> {
    fn height(&self, x: f64, y: f64) -> f64 {
        self.ground.source.grid.height(x, y)
    }

    fn wet_or_paved(&self, x: f64, y: f64) -> f64 {
        self.field
            .paved([x, y, FOOTPRINT_M])
            .max(self.field.water([x, y, FOOTPRINT_M]))
    }

    /// Whether the simulation's foliage covers the fog cell at `(x, y)` or one beside it.
    fn foliage_near(&self, x: f64, y: f64) -> bool {
        let (columns, cell, cells) = &self.foliage;
        let (i, j) = ((x / cell).floor() as i64, (y / cell).floor() as i64);
        (-1..=1).any(|dj| {
            (-1..=1).any(|di| {
                i + di >= 0 && i + di < *columns && cells.contains(&((j + dj) * columns + i + di))
            })
        })
    }

    /// The lowest ground round a trunk's foot at `(x, y)`.
    fn foot(&self, x: f64, y: f64) -> f64 {
        (0..4).fold(self.height(x, y), |foot, k| {
            let a = k as f64 * PI / 2.0;
            foot.min(self.height(x + 0.5 * libm::cos(a), y + 0.5 * libm::sin(a)))
        })
    }
}

impl Ground {
    /// Every appearance the biome places: its trees, its shrubs and its dressing.
    pub fn scenery_kinds(&self) -> Vec<String> {
        let t = &self.biome.trees;
        let names = t.species.iter().map(|s| &s.appearance);
        let names = names.chain([&t.hedgerows.appearance, &t.understorey.appearance]);
        let mut out: Vec<String> = vec![];
        for name in names.chain(
            self.biome
                .forest_floor
                .dressing
                .kinds
                .iter()
                .map(|k| &k.appearance),
        ) {
            if !out.contains(name) {
                out.push(name.clone());
            }
        }
        out
    }

    /// The scenery, its appearances sized `sizes` in `scenery_kinds` order.
    pub fn scenery(&self, sizes: &[Size]) -> Scenery {
        let (b, s) = (&self.biome, &self.source);
        let kinds = self.scenery_kinds();
        let d = &b.forest_floor.dressing;
        let mut site = Site {
            ground: self,
            field: self
                .field(false, &[[d.clear_m, d.edge_m, d.clear_m]; 32])
                .expect("the field of the ground resolved"),
            trunks: vec![],
            trunk_ids: vec![],
            trunk_heights: vec![],
            obstacles: vec![],
            foliage: (0, 1.0, HashSet::new()),
        };
        for p in s.props.as_chunks::<PROP>().0 {
            if s.drawn_by(p) == Some(super::DrawnBy::Forest) {
                site.trunks.push([p[3], p[4]]);
                site.trunk_ids
                    .push((p[0] as u64 + p[1] as u64 * (1 << 16)) as u32);
                site.trunk_heights.push(2.0 * p[8]);
            } else {
                site.obstacles
                    .push([p[3], p[4], libm::hypot(p[6] as f64, p[7] as f64) as f32]);
            }
        }
        if let [columns, _, cell, cells @ ..] = &s.foliage[..] {
            let cells = cells
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| c[1] as i64 * *columns as i64 + c[0] as i64);
            site.foliage = (*columns as i64, *cell as f64, cells.collect());
        }
        let mut species = Species::new(&b.trees, &kinds, b.seed);
        let kind = |name: &str| kinds.iter().position(|k| k == name).unwrap();
        let (forest, forest_ids) = forests(&site, &b.trees, sizes, &mut species, b.seed);
        Scenery {
            understorey: understorey(&site, b, sizes, kind(&b.trees.understorey.appearance)),
            backdrop: backdrop(
                &site,
                &b.trees,
                sizes,
                kind(&b.trees.hedgerows.appearance),
                &mut species,
                b.seed,
            ),
            dressing: dressing(site, b, &kinds, sizes),
            kinds,
            forest,
            forest_ids,
        }
    }

    /// The pieces of dressing cell `(i, j)`: the same every time.
    pub fn dress(&self, scenery: &Scenery, i: i64, j: i64) -> Vec<f32> {
        let b = &self.biome;
        let (rules, d) = (&b.forest_floor.dressing, &scenery.dressing);
        let field = &d.field;
        let m = DRESSING_CELL_M;
        let mut out = vec![];
        let mut rng = stream(b.seed ^ DRESSING_SALT, cell_key(i, j));
        let clear = |x: f64, y: f64| {
            let key = cell_key(
                (x / CLEAR_CELL_M).floor() as i64,
                (y / CLEAR_CELL_M).floor() as i64,
            );
            d.clear.get(&key).is_none_or(|l| {
                l.iter().all(|c| {
                    libm::pow(c[0] - x, 2.0) + libm::pow(c[1] - y, 2.0) >= libm::pow(c[2], 2.0)
                })
            })
        };
        for _ in 0..d.capacity {
            let (x, y) = (
                (i as f64 + rng.float(0.0, 1.0)) * m,
                (j as f64 + rng.float(0.0, 1.0)) * m,
            );
            let k = weighted(&mut rng, rules.kinds.iter().map(|r| r.weight));
            let row = &rules.kinds[k];
            // Between its drifts a kind thins to `1 - drift` of its pieces.
            let thick = drifted(d.rows[k].1.sample(x / rules.drift_m, y / rules.drift_m));
            if rng.sample() >= 1.0 - row.drift * (1.0 - thick) {
                continue;
            }
            let at = [x, y, FOOTPRINT_M];
            if field.forest(at) < rules.edge_m || !clear(x, y) {
                continue;
            }
            if field.paved(at).max(field.water(at)) > -rules.clear_m {
                continue;
            }
            let scale = rng.float(row.scale[0], row.scale[1]);
            let yaw = rng.float(0.0, PI * 2.0);
            let z = scale * rng.float(rules.squat, 1.0);
            let at = [
                x,
                y,
                self.source.grid.height(x, y) - SINK_M,
                yaw,
                scale,
                z,
                d.rows[k].0 as f64,
            ];
            push(&mut out, at, row.tint, &mut rng, rules.colour_jitter);
        }
        out
    }
}

fn forests(
    site: &Site,
    trees: &Trees,
    size: &[Size],
    species: &mut Species,
    seed: u32,
) -> (Vec<f32>, Vec<u32>) {
    let (mut out, mut ids) = (vec![], vec![]);
    let rules = &trees.forest;
    let mut rng = stream(seed, 1);
    let mut planted = vec![false; site.trunk_ids.len()];
    let mut trunk = 0;
    let source = &site.ground.source;
    for f in &site.ground.forests {
        let canopy = f.canopy as f64;
        let [first, end] = f.trunks;
        while trunk < site.trunk_ids.len() && site.trunk_ids[trunk] < first {
            trunk += 1;
        }
        while trunk < site.trunk_ids.len() && site.trunk_ids[trunk] < end {
            let [x, y] = site.trunks[trunk].map(f64::from);
            // One tree: its crown a share of its appearance's width, its top
            // under the canopy over the lowest ground its crown covers.
            let inside = if f.kind == Kind::Stroke {
                OPEN
            } else {
                f.inside(source, x, y)
            };
            let (kind, tint) = species.pick(&mut rng, x, y, inside);
            let k = size[kind];
            let mut sz = canopy * rng.float(rules.top[0], rules.top[1]) / k.height;
            let sxy = rng.float(rules.girth[0], rules.girth[1]);
            let radius = k.radius * sxy;
            let foot = site.foot(x, y);
            let z = foot - SINK_M;
            let mut floor = foot;
            for ring in 1..=4 {
                for n in 0..16 {
                    let (a, r) = (n as f64 * PI / 8.0, radius * ring as f64 / 4.0);
                    floor = floor.min(site.height(x + r * libm::cos(a), y + r * libm::sin(a)));
                }
            }
            sz = sz.min((floor + canopy - CANOPY_MARGIN_M - z) / k.height);
            let yaw = rng.float(0.0, PI * 2.0);
            push(
                &mut out,
                [x, y, z, yaw, sxy, sz, kind as f64],
                tint,
                &mut rng,
                trees.colour_jitter,
            );
            ids.push(site.trunk_ids[trunk]);
            planted[trunk] = true;
            trunk += 1;
        }
    }
    // A trunk no forest generated stands alone: a tree as tall as its own
    // body, seeded by the body, so one tree's look never moves another's.
    let lone = &trees.lone;
    let alone: Vec<usize> = (0..trees.species.len())
        .filter(|&i| lone.species.contains(&trees.species[i].appearance))
        .collect();
    for t in (0..planted.len()).filter(|&t| !planted[t]) {
        let mut own = stream(seed ^ LONE_SALT, site.trunk_ids[t] as i64);
        let [x, y] = site.trunks[t].map(f64::from);
        let s = alone[weighted(&mut own, alone.iter().map(|&i| species.rows[i].2))];
        let (kind, tint) = (species.rows[s].0, species.rows[s].1);
        ids.push(site.trunk_ids[t]);
        let top = site.trunk_heights[t] as f64 * own.float(lone.top[0], lone.top[1]);
        let yaw = own.float(0.0, PI * 2.0);
        let girth = own.float(lone.girth[0], lone.girth[1]);
        let at = [
            x,
            y,
            site.foot(x, y) - SINK_M,
            yaw,
            girth,
            top / size[kind].height,
            kind as f64,
        ];
        push(&mut out, at, tint, &mut own, trees.colour_jitter);
    }
    (out, ids)
}

/// The shrubs under every strip of forest: rows of hedge along its length,
/// as many as fit its width, each row's shrubs end to end and the next
/// row's half a spacing on, drawn together over the last `taper_m` before
/// the strip's ends, where the ground's band under it narrows to a point.
fn understorey(site: &Site, b: &Biome, size: &[Size], kind: usize) -> Vec<f32> {
    let mut out = vec![];
    let (trees, taper) = (&b.trees, b.forest_floor.tree_line.taper_m);
    let rules = &trees.understorey;
    let shrub = size[kind];
    // Under the simulation's foliage, as a crown is, and clear of paving,
    // water and every body but a tree.
    let clear = |x: f64, y: f64, reach: f64| {
        let open = |x: f64, y: f64| site.wet_or_paved(x, y) <= 0.0 && site.foliage_near(x, y);
        open(x, y)
            && (0..8).all(|k| {
                let a = k as f64 * PI / 4.0;
                open(x + reach * libm::cos(a), y + reach * libm::sin(a))
            })
            && site
                .obstacles
                .iter()
                .all(|o| libm::hypot(o[0] as f64 - x, o[1] as f64 - y) >= o[2] as f64 + reach)
    };
    for (index, f) in site.ground.forests.iter().enumerate() {
        if f.kind != Kind::Stroke {
            continue;
        }
        let s = chunks::<FOREST_STROKE>(&site.ground.source.forest_strokes[f.strokes.clone()]);
        let mut rng = stream(b.seed ^ UNDERSTOREY_SALT, index as i64);
        let run = |r: &[f32; FOREST_STROKE]| {
            libm::hypot(r[2] as f64 - r[0] as f64, r[3] as f64 - r[1] as f64)
        };
        let mut starts = vec![];
        let mut length = 0.0;
        for r in s {
            starts.push(length);
            length += run(r);
        }
        let room = s[0][4] as f64 - shrub.radius * rules.length[1];
        let rows = if room > 0.0 {
            1 + (2.0 * room / rules.row_m).floor() as usize
        } else {
            1
        };
        let count = (length / rules.spacing_m).floor() as usize;
        for row in 0..rows {
            let offset = (row as f64 - (rows - 1) as f64 / 2.0) * rules.row_m;
            let mut stretch = 0;
            for n in 0..count {
                let shift = n as f64 + 0.5 + (row % 2) as f64 * 0.5 + rng.float(-0.15, 0.15);
                let along = shift * length / count as f64;
                let sway = rng.float(-rules.sway_m, rules.sway_m);
                let long = rng.float(rules.length[0], rules.length[1]);
                let tall = rng.float(rules.height[0], rules.height[1]);
                let turn = rng.float(-SHRUB_TURN, SHRUB_TURN);
                if rng.sample() < rules.gap || along >= length {
                    continue;
                }
                while stretch + 1 < starts.len() && starts[stretch + 1] <= along {
                    stretch += 1;
                }
                let r = &s[stretch];
                let l = run(r);
                let (dx, dy) = (
                    (r[2] as f64 - r[0] as f64) / l,
                    (r[3] as f64 - r[1] as f64) / l,
                );
                let aside =
                    (offset + sway) * ((along / taper).min(1.0)).min((length - along) / taper);
                let x = r[0] as f64 + dx * (along - starts[stretch]) - dy * aside;
                let y = r[1] as f64 + dy * (along - starts[stretch]) + dx * aside;
                // Its whole reach on the strip's own ground: a shrub is drawn
                // smaller to fit, and not at all where it would be smaller than its kind.
                let scale = long.min(f.inside(&site.ground.source, x, y) / shrub.radius);
                if scale < rules.length[0] || !clear(x, y, shrub.radius * scale) {
                    continue;
                }
                // On the lowest ground under its length.
                let reach = shrub.radius * scale / 2.0;
                let foot = site
                    .height(x, y)
                    .min(site.height(x + dx * reach, y + dy * reach))
                    .min(site.height(x - dx * reach, y - dy * reach));
                let at = [
                    x,
                    y,
                    foot - SINK_M,
                    libm::atan2(dy, dx) + turn,
                    scale,
                    tall,
                    kind as f64,
                ];
                push(&mut out, at, rules.tint, &mut rng, trees.colour_jitter);
            }
        }
    }
    out
}

fn backdrop(
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
    let z = source.grid.min_height - SINK_M;
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
                [x, y, z, yaw, sxy, sz, kind as f64],
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
                        [x, y, z, turn, sxy, sz, hedge as f64],
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

/// The cells of side `cell_m` the forests' own primitives reach (`i, j`
/// pairs, in first-reached order): a rectangle, each stretch of a strip with
/// its width, each triangle of a polygon, by its box.
fn forest_cells(site: &Site, cell_m: f64) -> Vec<i32> {
    let (mut seen, mut out) = (HashSet::new(), vec![]);
    let mut reach = |x0: f64, y0: f64, x1: f64, y1: f64| {
        for j in (y0 / cell_m).floor() as i32..=(y1 / cell_m).floor() as i32 {
            for i in (x0 / cell_m).floor() as i32..=(x1 / cell_m).floor() as i32 {
                if seen.insert((i, j)) {
                    out.extend([i, j]);
                }
            }
        }
    };
    let s = &site.ground.source;
    for f in &site.ground.forests {
        if let (Kind::Rectangle, Some([x, y, w, h])) = (f.kind, f.rect.map(|r| r.map(f64::from))) {
            reach(x, y, x + w, y + h);
        }
        for r in chunks::<FOREST_STROKE>(&s.forest_strokes[f.strokes.clone()]) {
            let [ax, ay, bx, by, half] = [r[0], r[1], r[2], r[3], r[4]].map(f64::from);
            reach(
                ax.min(bx) - half,
                ay.min(by) - half,
                ax.max(bx) + half,
                ay.max(by) + half,
            );
        }
        for t in chunks::<FOREST_TRIANGLE>(&s.forest_triangles[f.triangles.clone()]) {
            let [ax, ay, bx, by, cx, cy] = [t[0], t[1], t[2], t[3], t[4], t[5]].map(f64::from);
            let lo = |a: f64, b: f64, c: f64| a.min(b).min(c);
            let hi = |a: f64, b: f64, c: f64| a.max(b).max(c);
            reach(
                lo(ax, bx, cx),
                lo(ay, by, cy),
                hi(ax, bx, cx),
                hi(ay, by, cy),
            );
        }
    }
    out
}

fn dressing(site: Site, b: &Biome, kinds: &[String], size: &[Size]) -> Dressing {
    let rules = &b.forest_floor.dressing;
    let row_kinds: Vec<usize> = rules
        .kinds
        .iter()
        .map(|r| kinds.iter().position(|k| *k == r.appearance).unwrap())
        .collect();
    // Clear of every trunk (by `trunk_clear_m` from its axis) and every body
    // (by `clear_m` from its footprint's circle), bucketed where they reach.
    let mut clear: HashMap<i64, Vec<[f64; 3]>> = HashMap::new();
    let circles = site
        .trunks
        .iter()
        .map(|t| [t[0] as f64, t[1] as f64, rules.trunk_clear_m]);
    let circles = circles.chain(
        site.obstacles
            .iter()
            .map(|o| [o[0] as f64, o[1] as f64, o[2] as f64 + rules.clear_m]),
    );
    for [x, y, r] in circles {
        let at = |v: f64| (v / CLEAR_CELL_M).floor() as i64;
        for j in at(y - r)..=at(y + r) {
            for i in at(x - r)..=at(x + r) {
                clear.entry(cell_key(i, j)).or_default().push([x, y, r]);
            }
        }
    }
    let grid = &site.ground.source.grid;
    let high = grid.heights.iter().fold(0.0f64, |h, &v| h.max(v as f64));
    let most = |f: &dyn Fn(usize) -> f64| (0..rules.kinds.len()).fold(0.0f64, |m, k| m.max(f(k)));
    let seed = (b.seed ^ DRESSING_SALT) as i32;
    Dressing {
        capacity: (DRESSING_CELL_M * DRESSING_CELL_M * rules.per_ha / 10000.0).round() as usize,
        kinds: row_kinds
            .iter()
            .enumerate()
            .filter(|&(i, k)| !row_kinds[..i].contains(k))
            .map(|(_, &k)| k)
            .collect(),
        top_m: most(&|k| size[row_kinds[k]].height * rules.kinds[k].scale[1]),
        reach_m: most(&|k| size[row_kinds[k]].radius * rules.kinds[k].scale[1]),
        ground: [grid.min_height, high],
        cells: forest_cells(&site, DRESSING_CELL_M),
        rows: row_kinds
            .iter()
            .enumerate()
            .map(|(k, &kind)| (kind, Simplex::new(seed.wrapping_add(k as i32))))
            .collect(),
        clear,
        field: site.field,
    }
}
