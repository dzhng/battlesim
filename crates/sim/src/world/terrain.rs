//! The authoritative ground surface: a height grid triangulated along the
//! south-west → north-east diagonal of every cell. Height queries, normals and
//! ray hits interpolate these exact triangles. The public export keeps local
//! detail and coalesces only all-zero rectangles into equivalent flat triangles.
//! A sample is the land's relief as rivers and bridges carve it (`carve.rs`).
use super::carve::Carve;
use super::surfaces::SurfaceIndex;
use crate::math::{v2, v3, V3};
use contract::map::{MapDefinition, Relief};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

pub const HEIGHT_PAGE_SIZE: usize = 16;
const PAGE_SAMPLES: usize = HEIGHT_PAGE_SIZE * HEIGHT_PAGE_SIZE;

pub struct HeightField {
    pub(crate) spacing: f64,
    /// Samples per axis (cells + 1).
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pages: HashMap<usize, Box<[f64; PAGE_SAMPLES]>>,
    mesh: OnceLock<(Vec<V3>, Vec<u32>)>,
    samples: OnceLock<(Vec<u32>, Vec<f32>)>,
    /// The highest sample: no point of the surface stands above it.
    top: f64,
    bottom: f64,
    variation_regions: Vec<[f64; 4]>,
}

/// The two triangles of cell (i, j): corners in counter-clockwise order.
/// `A` = (SW, SE, NE), `B` = (SW, NE, NW).
fn cell_triangles(f: &HeightField, i: usize, j: usize) -> [[V3; 3]; 2] {
    let x0 = i as f64 * f.spacing;
    let y0 = j as f64 * f.spacing;
    let x1 = x0 + f.spacing;
    let y1 = y0 + f.spacing;
    let sw = v3(x0, y0, f.sample(i, j));
    let se = v3(x1, y0, f.sample(i + 1, j));
    let ne = v3(x1, y1, f.sample(i + 1, j + 1));
    let nw = v3(x0, y1, f.sample(i, j + 1));
    [[sw, se, ne], [sw, ne, nw]]
}

impl HeightField {
    /// `ground` holds the map's rivers.
    pub fn build(map: &MapDefinition, ground: &SurfaceIndex) -> Self {
        let spacing = map.height_grid_m;
        let nx = (map.size[0] / spacing).round() as usize + 1;
        let ny = (map.size[1] / spacing).round() as usize + 1;
        let relief = relief_bounds(map);
        let carve = Carve::new(map, ground, &relief);
        let mut variation_regions: Vec<_> = relief.iter().map(|(rect, _)| *rect).collect();
        variation_regions.extend_from_slice(carve.regions());
        let page_cols = nx.div_ceil(HEIGHT_PAGE_SIZE);
        let mut candidates = BTreeSet::new();
        for r in &variation_regions {
            let i0 = (r[0] / spacing).floor().max(0.0) as usize;
            let j0 = (r[1] / spacing).floor().max(0.0) as usize;
            let i1 = ((r[0] + r[2]) / spacing).ceil().max(0.0) as usize;
            let j1 = ((r[1] + r[3]) / spacing).ceil().max(0.0) as usize;
            if i0 >= nx || j0 >= ny {
                continue;
            }
            for py in j0 / HEIGHT_PAGE_SIZE..=j1.min(ny - 1) / HEIGHT_PAGE_SIZE {
                for px in i0 / HEIGHT_PAGE_SIZE..=i1.min(nx - 1) / HEIGHT_PAGE_SIZE {
                    candidates.insert(py * page_cols + px);
                }
            }
        }
        let mut pages = HashMap::new();
        let mut top = f64::NEG_INFINITY;
        let mut bottom = f64::INFINITY;
        let mut nonzero = 0;
        for id in candidates {
            let (i0, j0) = (
                (id % page_cols) * HEIGHT_PAGE_SIZE,
                (id / page_cols) * HEIGHT_PAGE_SIZE,
            );
            let mut page = Box::new([0.0; PAGE_SAMPLES]);
            let mut populated = false;
            for j in j0..(j0 + HEIGHT_PAGE_SIZE).min(ny) {
                for i in i0..(i0 + HEIGHT_PAGE_SIZE).min(nx) {
                    let (x, y) = (i as f64 * spacing, j as f64 * spacing);
                    let h = carve.height(map.relief_height(x, y), v2(x, y));
                    page[(j - j0) * HEIGHT_PAGE_SIZE + i - i0] = h;
                    top = top.max(h);
                    bottom = bottom.min(h);
                    nonzero += (h != 0.0) as usize;
                    populated |= h != 0.0;
                }
            }
            if populated {
                pages.insert(id, page);
            }
        }
        if nonzero < nx * ny {
            top = top.max(0.0);
            bottom = bottom.min(0.0);
        }
        HeightField {
            spacing,
            nx,
            ny,
            pages,
            top,
            bottom,
            variation_regions,
            mesh: OnceLock::new(),
            samples: OnceLock::new(),
        }
    }

    pub fn variation_regions(&self) -> &[[f64; 4]] {
        &self.variation_regions
    }

    // Inlined for the same reason as `triangle`, its caller in the fog sweep.
    #[inline(always)]
    pub fn sample(&self, i: usize, j: usize) -> f64 {
        if self.pages.is_empty() {
            return 0.0;
        }
        let id = (j / HEIGHT_PAGE_SIZE) * self.nx.div_ceil(HEIGHT_PAGE_SIZE) + i / HEIGHT_PAGE_SIZE;
        self.pages.get(&id).map_or(0.0, |p| {
            p[(j % HEIGHT_PAGE_SIZE) * HEIGHT_PAGE_SIZE + i % HEIGHT_PAGE_SIZE]
        })
    }

    /// The highest ground height anywhere on the field.
    pub fn top(&self) -> f64 {
        self.top
    }

    pub fn bottom(&self) -> f64 {
        self.bottom
    }

    pub fn width(&self) -> f64 {
        (self.nx - 1) as f64 * self.spacing
    }

    pub fn depth(&self) -> f64 {
        (self.ny - 1) as f64 * self.spacing
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        (0.0..=self.width()).contains(&x) && (0.0..=self.depth()).contains(&y)
    }

    /// Cell and in-cell fractions for a point inside bounds.
    fn locate(&self, x: f64, y: f64) -> (usize, usize, f64, f64) {
        let fx = x / self.spacing;
        let fy = y / self.spacing;
        let i = (fx.floor() as usize).min(self.nx - 2);
        let j = (fy.floor() as usize).min(self.ny - 2);
        (i, j, fx - i as f64, fy - j as f64)
    }

    /// Height of the triangle under (x, y): `height_normal`'s height, without
    /// paying for the normal (visibility sweeps ask millions of times).
    #[inline(always)]
    pub fn height(&self, x: f64, y: f64) -> Option<f64> {
        self.triangle(x, y).map(|(h, _, _)| h)
    }

    /// Height and upward unit normal of the triangle under (x, y).
    pub fn height_normal(&self, x: f64, y: f64) -> Option<(f64, V3)> {
        let (h, rise_x, rise_y) = self.triangle(x, y)?;
        let s = self.spacing;
        Some((h, v3(-(rise_x / s), -(rise_y / s), 1.0).normalized()))
    }

    /// The triangle under (x, y): its height there and its rise across one
    /// cell along x and along y.
    // Inlined, as is `height`: the fog sweep calls them per sample, and a
    // new caller elsewhere must not tip LLVM into out-of-line calls there
    // (without it, `village_report` once measured +21% instructions).
    #[inline(always)]
    fn triangle(&self, x: f64, y: f64) -> Option<(f64, f64, f64)> {
        if !self.contains(x, y) {
            return None;
        }
        let (i, j, u, v) = self.locate(x, y);
        let h00 = self.sample(i, j);
        let h11 = self.sample(i + 1, j + 1);
        Some(if u >= v {
            // Triangle A (SW, SE, NE).
            let h10 = self.sample(i + 1, j);
            let (rise_x, rise_y) = (h10 - h00, h11 - h10);
            (h00 + u * rise_x + v * rise_y, rise_x, rise_y)
        } else {
            // Triangle B (SW, NE, NW).
            let h01 = self.sample(i, j + 1);
            let (rise_x, rise_y) = (h11 - h01, h01 - h00);
            (h00 + v * rise_y + u * rise_x, rise_x, rise_y)
        })
    }

    /// Earliest intersection of the segment `origin + dir * t`, t ∈ [0, max_t],
    /// with the ground triangles. Walks cells along the XY projection so no hit
    /// is rounded to a cell. Returns (t, normal).
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<(f64, V3)> {
        // Clip to the map rectangle in XY.
        let (mut t0, mut t1) = (0.0f64, max_t);
        for (o, d, hi) in [
            (origin.x, dir.x, self.width()),
            (origin.y, dir.y, self.depth()),
        ] {
            if d.abs() < 1e-12 {
                if o < 0.0 || o > hi {
                    return None;
                }
            } else {
                let (a, b) = ((0.0 - o) / d, (hi - o) / d);
                t0 = t0.max(a.min(b));
                t1 = t1.min(a.max(b));
            }
        }
        if t0 > t1 {
            return None;
        }
        let at = |t: f64| origin + dir * t;
        let start = at(t0);
        let (mut i, mut j, _, _) = self.locate(start.x, start.y);
        let step_i: isize = if dir.x > 0.0 { 1 } else { -1 };
        let step_j: isize = if dir.y > 0.0 { 1 } else { -1 };
        let next_boundary = |cell: usize, step: isize, o: f64, d: f64| -> f64 {
            if d.abs() < 1e-12 {
                return f64::INFINITY;
            }
            let edge = (cell as f64 + if step > 0 { 1.0 } else { 0.0 }) * self.spacing;
            (edge - o) / d
        };
        let delta_i = if dir.x.abs() < 1e-12 {
            f64::INFINITY
        } else {
            self.spacing / dir.x.abs()
        };
        let delta_j = if dir.y.abs() < 1e-12 {
            f64::INFINITY
        } else {
            self.spacing / dir.y.abs()
        };
        let mut t_i = next_boundary(i, step_i, origin.x, dir.x);
        let mut t_j = next_boundary(j, step_j, origin.y, dir.y);
        // A cell whose highest corner lies below the ray all across the
        // cell's span holds no hit: its triangles are not tested. Only for a
        // ray not near vertical, so the span's rounding moves its height by
        // far less than the millimetre of slack.
        let bounded = dir.xy().length() >= 0.05;
        let mut t_enter = t0;
        loop {
            let cell_end = t_i.min(t_j).min(t1);
            let mut best: Option<(f64, V3)> = None;
            let above = bounded && {
                let low = (origin.z + dir.z * t_enter).min(origin.z + dir.z * cell_end);
                let top = self
                    .sample(i, j)
                    .max(self.sample(i + 1, j))
                    .max(self.sample(i, j + 1))
                    .max(self.sample(i + 1, j + 1));
                low > top + 1e-3
            };
            t_enter = cell_end;
            let tris = if above {
                &[][..]
            } else {
                &cell_triangles(self, i, j)[..]
            };
            for &tri in tris {
                if let Some(t) = ray_triangle(origin, dir, tri) {
                    // Accept hits within this cell's span (with a tolerance so
                    // an edge hit is never skipped between neighbours).
                    if t >= t0 - 1e-9 && t <= cell_end + 1e-9 && best.is_none_or(|b| t < b.0) {
                        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalized();
                        best = Some((t, n));
                    }
                }
            }
            if best.is_some() {
                return best;
            }
            if cell_end >= t1 {
                return None;
            }
            if t_i < t_j {
                let ni = i as isize + step_i;
                if ni < 0 || ni as usize >= self.nx - 1 {
                    return None;
                }
                i = ni as usize;
                t_i += delta_i;
            } else {
                let nj = j as isize + step_j;
                if nj < 0 || nj as usize >= self.ny - 1 {
                    return None;
                }
                j = nj as usize;
                t_j += delta_j;
            }
        }
    }

    /// Exact sampled triangles, with all-zero rectangles coalesced.
    pub fn mesh(&self) -> &(Vec<V3>, Vec<u32>) {
        self.mesh.get_or_init(|| {
            let cols = self.nx.div_ceil(HEIGHT_PAGE_SIZE);
            let regions: Vec<[usize; 4]> = self
                .pages
                .keys()
                .map(|id| {
                    let (i, j) = (
                        (id % cols) * HEIGHT_PAGE_SIZE,
                        (id / cols) * HEIGHT_PAGE_SIZE,
                    );
                    [
                        i,
                        j,
                        (i + HEIGHT_PAGE_SIZE - 1).min(self.nx - 1),
                        (j + HEIGHT_PAGE_SIZE - 1).min(self.ny - 1),
                    ]
                })
                .collect();
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut ids = BTreeMap::new();
            self.mesh_rect(
                [0, 0, self.nx - 1, self.ny - 1],
                &regions,
                &mut vertices,
                &mut indices,
                &mut ids,
            );
            (vertices, indices)
        })
    }

    fn mesh_rect(
        &self,
        r: [usize; 4],
        regions: &[[usize; 4]],
        vertices: &mut Vec<V3>,
        indices: &mut Vec<u32>,
        ids: &mut BTreeMap<(usize, usize), u32>,
    ) {
        let [x0, y0, x1, y1] = r;
        let detailed = regions
            .iter()
            .any(|p| p[0] <= x1 && p[2] >= x0 && p[1] <= y1 && p[3] >= y0);
        if !detailed || (x1 - x0 == 1 && y1 - y0 == 1) {
            let mut at = |i, j| {
                *ids.entry((i, j)).or_insert_with(|| {
                    let id = vertices.len() as u32;
                    vertices.push(v3(
                        i as f64 * self.spacing,
                        j as f64 * self.spacing,
                        self.sample(i, j),
                    ));
                    id
                })
            };
            let (sw, se, ne, nw) = (at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1));
            indices.extend_from_slice(&[sw, se, ne, sw, ne, nw]);
            return;
        }
        let xm = (x0 + x1) / 2;
        let ym = (y0 + y1) / 2;
        let xs = if x1 - x0 > 1 {
            vec![(x0, xm), (xm, x1)]
        } else {
            vec![(x0, x1)]
        };
        let ys = if y1 - y0 > 1 {
            vec![(y0, ym), (ym, y1)]
        } else {
            vec![(y0, y1)]
        };
        for (a, b) in ys {
            for &(c, d) in &xs {
                self.mesh_rect([c, a, d, b], regions, vertices, indices, ids);
            }
        }
    }

    pub fn export_samples(&self) -> &(Vec<u32>, Vec<f32>) {
        self.samples.get_or_init(|| {
            let mut ids: Vec<_> = self.pages.keys().copied().collect();
            ids.sort_unstable();
            let heights = ids
                .iter()
                .flat_map(|id| self.pages[id].iter().map(|h| *h as f32))
                .collect();
            (ids.into_iter().map(|id| id as u32).collect(), heights)
        })
    }
}

/// Each relief feature's box, where the land can differ from flat, and the
/// most it adds to the land.
fn relief_bounds(map: &MapDefinition) -> Vec<([f64; 4], f64)> {
    let grow = |r: [f64; 4], reach: f64| {
        [
            r[0] - reach,
            r[1] - reach,
            r[2] + 2.0 * reach,
            r[3] + 2.0 * reach,
        ]
    };
    map.relief
        .iter()
        .map(|r| {
            let (rect, peak) = match *r {
                Relief::Ridge {
                    center,
                    radius_m,
                    peak_m,
                } => (
                    [
                        center[0] - radius_m,
                        center[1] - radius_m,
                        2.0 * radius_m,
                        2.0 * radius_m,
                    ],
                    peak_m,
                ),
                Relief::Mesa {
                    rect,
                    height_m,
                    side_degrees,
                } => {
                    let slope = side_degrees.to_radians().tan();
                    let rect = if slope > 0.0 {
                        grow(rect, height_m.max(0.0) / slope)
                    } else {
                        // A non-falling authored mesa may affect every sample.
                        [0.0, 0.0, map.size[0], map.size[1]]
                    };
                    (rect, height_m)
                }
            };
            // A sampled contribution affects the adjacent triangle cell too.
            (grow(rect, map.height_grid_m), peak)
        })
        .collect()
}

/// Möller–Trumbore, two-sided. Returns t ≥ 0.
pub(crate) fn ray_triangle(origin: V3, dir: V3, tri: [V3; 3]) -> Option<f64> {
    let e1 = tri[1] - tri[0];
    let e2 = tri[2] - tri[0];
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - tri[0];
    let u = s.dot(p) * inv;
    if !(-1e-9..=1.0 + 1e-9).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t >= 0.0).then_some(t)
}
