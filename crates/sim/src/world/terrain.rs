//! The authoritative ground surface: a height grid triangulated along the
//! south-west → north-east diagonal of every cell. Height queries, normals and
//! ray hits all interpolate these exact triangles, and the renderer receives
//! the same vertices.
use crate::math::{v3, V3};
use contract::map::{MapDefinition, Relief};

pub struct HeightField {
    pub(crate) spacing: f64,
    /// Samples per axis (cells + 1).
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pub(crate) heights: Vec<f64>,
    /// The highest sample: no point of the surface stands above it.
    top: f64,
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
    pub fn build(map: &MapDefinition) -> Self {
        let spacing = map.height_grid_m;
        let nx = (map.size[0] / spacing).round() as usize + 1;
        let ny = (map.size[1] / spacing).round() as usize + 1;
        let mut heights = vec![0.0; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f64 * spacing;
                let y = j as f64 * spacing;
                let mut h = map
                    .relief
                    .iter()
                    .map(|r| relief_height(r, x, y))
                    .sum::<f64>();
                for w in &map.water {
                    if in_rect(w.rect, x, y) {
                        h = h.min(w.bed_z);
                    }
                }
                heights[j * nx + i] = h;
            }
        }
        let top = heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        HeightField {
            spacing,
            nx,
            ny,
            heights,
            top,
        }
    }

    pub fn sample(&self, i: usize, j: usize) -> f64 {
        self.heights[j * self.nx + i]
    }

    /// The highest ground height anywhere on the field.
    pub fn top(&self) -> f64 {
        self.top
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
    // (slice 39 measured +21% village_report instructions without it).
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
        loop {
            let cell_end = t_i.min(t_j).min(t1);
            let mut best: Option<(f64, V3)> = None;
            for tri in cell_triangles(self, i, j) {
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

    /// Triangle list over the whole field: vertices row-major, indices in the
    /// same diagonal split every query uses.
    pub fn mesh(&self) -> (Vec<V3>, Vec<u32>) {
        let mut vertices = Vec::with_capacity(self.nx * self.ny);
        for j in 0..self.ny {
            for i in 0..self.nx {
                vertices.push(v3(
                    i as f64 * self.spacing,
                    j as f64 * self.spacing,
                    self.sample(i, j),
                ));
            }
        }
        let mut indices = Vec::with_capacity((self.nx - 1) * (self.ny - 1) * 6);
        let at = |i: usize, j: usize| (j * self.nx + i) as u32;
        for j in 0..self.ny - 1 {
            for i in 0..self.nx - 1 {
                let (sw, se, ne, nw) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                indices.extend_from_slice(&[sw, se, ne, sw, ne, nw]);
            }
        }
        (vertices, indices)
    }
}

pub fn in_rect(r: [f64; 4], x: f64, y: f64) -> bool {
    x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
}

fn relief_height(relief: &Relief, x: f64, y: f64) -> f64 {
    match *relief {
        Relief::Ridge {
            center,
            peak_m,
            radius_m,
        } => {
            let d = (x - center[0]).hypot(y - center[1]);
            if d >= radius_m {
                0.0
            } else {
                let q = 1.0 - (d / radius_m).powi(2);
                peak_m * q * q
            }
        }
        Relief::Mesa {
            rect,
            height_m,
            side_degrees,
        } => {
            let dx = (rect[0] - x).max(x - (rect[0] + rect[2])).max(0.0);
            let dy = (rect[1] - y).max(y - (rect[1] + rect[3])).max(0.0);
            (height_m - dx.hypot(dy) * side_degrees.to_radians().tan()).max(0.0)
        }
    }
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
