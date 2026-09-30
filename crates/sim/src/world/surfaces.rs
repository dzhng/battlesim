//! Sparse broad phase over the exact authored ground shapes.
use std::collections::{BTreeSet, HashMap};

use contract::map::{Rect, SurfaceArea, SurfaceKind, SurfaceShape};

use crate::math::{v2, V2};

const BUCKET_M: f64 = 128.0;

enum Primitive {
    Segment { area: usize, edge: usize },
    Polygon { area: usize },
}
impl Primitive {
    fn area(&self) -> usize {
        match *self {
            Self::Segment { area, .. } | Self::Polygon { area } => area,
        }
    }
}

pub(super) struct SurfaceIndex {
    areas: Vec<SurfaceArea>,
    primitives: Vec<Primitive>,
    buckets: HashMap<(i32, i32), Vec<usize>>,
    last: [i32; 2],
    triangles: Vec<f32>,
    boundaries: Vec<f32>,
}

impl SurfaceIndex {
    pub fn new(areas: &[SurfaceArea], size: [f64; 2]) -> Self {
        let mut index = Self {
            areas: areas.to_vec(),
            primitives: Vec::new(),
            buckets: HashMap::new(),
            last: size.map(|v| (v / BUCKET_M).floor() as i32),
            triangles: Vec::new(),
            boundaries: Vec::new(),
        };
        for (area, surface) in areas.iter().enumerate() {
            match &surface.shape {
                SurfaceShape::Stroke { points, width_m } => {
                    assert!(
                        width_m.is_finite() && *width_m >= 0.0,
                        "surface stroke width must be finite and nonnegative"
                    );
                    assert!(
                        points.iter().flatten().all(|v| v.is_finite()),
                        "surface points must be finite"
                    );
                    for (edge, points) in points.windows(2).enumerate() {
                        index.insert(
                            Primitive::Segment { area, edge },
                            limits(points, *width_m / 2.0),
                        );
                    }
                }
                SurfaceShape::Polygon { ring } => {
                    assert!(
                        ring.len() >= 3 && ring.iter().flatten().all(|v| v.is_finite()),
                        "surface polygon requires at least three finite vertices"
                    );
                    index.triangles.extend(triangulate(ring, surface.kind));
                    index.insert(Primitive::Polygon { area }, limits(ring, 0.0));
                }
            }
        }
        index.boundaries = index.build_boundaries();
        index
    }

    fn insert(&mut self, primitive: Primitive, bounds: [f64; 4]) {
        let id = self.primitives.len();
        self.primitives.push(primitive);
        let [x0, y0, x1, y1] = self.bucket_bounds(bounds);
        for j in y0..=y1 {
            for i in x0..=x1 {
                self.buckets.entry((i, j)).or_default().push(id);
            }
        }
    }

    fn bucket_bounds(&self, [x, y, max_x, max_y]: [f64; 4]) -> [i32; 4] {
        [
            ((x / BUCKET_M).floor() as i32).max(0),
            ((y / BUCKET_M).floor() as i32).max(0),
            ((max_x / BUCKET_M).floor() as i32).min(self.last[0]),
            ((max_y / BUCKET_M).floor() as i32).min(self.last[1]),
        ]
    }

    pub fn areas(&self) -> &[SurfaceArea] {
        &self.areas
    }
    pub fn triangles(&self) -> &[f32] {
        &self.triangles
    }
    pub fn boundaries(&self) -> &[f32] {
        &self.boundaries
    }

    fn build_boundaries(&self) -> Vec<f32> {
        let winding: Vec<_> = self
            .areas
            .iter()
            .map(|area| match &area.shape {
                SurfaceShape::Polygon { ring } => polygon_area(ring).signum(),
                SurfaceShape::Stroke { .. } => 0.0,
            })
            .collect();
        let mut out = Vec::new();
        for (id, area) in self.areas.iter().enumerate() {
            let SurfaceShape::Polygon { ring } = &area.shape else {
                continue;
            };
            for (a, b) in edges(ring) {
                let candidates = self.boundary_candidates(*a, *b);
                let mut cuts = vec![0.0, 1.0];
                for &other in &candidates {
                    if other == id {
                        continue;
                    }
                    let SurfaceShape::Polygon { ring } = &self.areas[other].shape else {
                        unreachable!()
                    };
                    for (c, d) in edges(ring) {
                        split_edge(*a, *b, *c, *d, &mut cuts);
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup();
                for span in cuts.windows(2) {
                    let mid = (span[0] + span[1]) / 2.0;
                    let point = on_edge(*a, *b, mid);
                    let covered = candidates.iter().any(|&other| {
                        if other == id {
                            return false;
                        }
                        let SurfaceShape::Polygon { ring } = &self.areas[other].shape else {
                            unreachable!()
                        };
                        for (c, d) in edges(ring) {
                            if cross(*a, *b, *c) != 0.0 || cross(*a, *b, *d) != 0.0 {
                                continue;
                            }
                            let (lo, hi) = (edge_parameter(*a, *b, *c), edge_parameter(*a, *b, *d));
                            if mid <= lo.min(hi) || mid >= lo.max(hi) {
                                continue;
                            }
                            let axis = edge_axis(*a, *b);
                            let same_direction = (b[axis] > a[axis]) == (d[axis] > c[axis]);
                            let same_facing = same_direction == (winding[id] == winding[other]);
                            // Opposite interiors make a shared edge internal. Coincident
                            // exterior edges keep one copy, preferring the road tag.
                            return !same_facing
                                || boundary_order(self.areas[other].kind, other)
                                    < boundary_order(area.kind, id);
                        }
                        polygon_contains(ring, v2(point[0], point[1]))
                    });
                    if covered {
                        continue;
                    }
                    let (a, b) = (on_edge(*a, *b, span[0]), on_edge(*a, *b, span[1]));
                    out.extend([
                        a[0] as f32,
                        a[1] as f32,
                        b[0] as f32,
                        b[1] as f32,
                        super::export::surface_area_tag(area.kind),
                    ]);
                }
            }
        }
        out
    }

    fn boundary_candidates(&self, a: [f64; 2], b: [f64; 2]) -> Vec<usize> {
        let [x, y, max_x, max_y] = limits(&[a, b], 0.0);
        let raw = [x, y, max_x, max_y].map(|v| (v / BUCKET_M).floor() as i32);
        // The query index is clipped to the map. Exported authoring edges can
        // lie beyond it, where the complete source still owns their boundary.
        if raw[0] < 0 || raw[1] < 0 || raw[2] > self.last[0] || raw[3] > self.last[1] {
            return self
                .areas
                .iter()
                .enumerate()
                .filter_map(|(i, area)| {
                    matches!(area.shape, SurfaceShape::Polygon { .. }).then_some(i)
                })
                .collect();
        }
        let mut ids = BTreeSet::new();
        for j in raw[1]..=raw[3] {
            for i in raw[0]..=raw[2] {
                if let Some(primitives) = self.buckets.get(&(i, j)) {
                    for &primitive in primitives {
                        if let Primitive::Polygon { area } = self.primitives[primitive] {
                            ids.insert(area);
                        }
                    }
                }
            }
        }
        ids.into_iter().collect()
    }

    /// A road wins a sidewalk overlap; water/bridges are resolved by the world.
    pub fn at(&self, p: V2) -> Option<SurfaceKind> {
        let key = (
            (p.x / BUCKET_M).floor() as i32,
            (p.y / BUCKET_M).floor() as i32,
        );
        let mut result = None;
        for &id in self.buckets.get(&key)? {
            let primitive = &self.primitives[id];
            if self.contains(primitive, p, 0.0) {
                let kind = self.areas[primitive.area()].kind;
                if kind == SurfaceKind::Road {
                    return Some(kind);
                }
                result = Some(kind);
            }
        }
        result
    }

    pub fn road_near(&self, p: V2, margin: f64) -> bool {
        let [x0, y0, x1, y1] =
            self.bucket_bounds([p.x - margin, p.y - margin, p.x + margin, p.y + margin]);
        for j in y0..=y1 {
            for i in x0..=x1 {
                if self.buckets.get(&(i, j)).is_some_and(|ids| {
                    ids.iter().any(|&id| {
                        let primitive = &self.primitives[id];
                        self.areas[primitive.area()].kind == SurfaceKind::Road
                            && self.contains(primitive, p, margin)
                    })
                }) {
                    return true;
                }
            }
        }
        false
    }

    fn contains(&self, primitive: &Primitive, p: V2, margin: f64) -> bool {
        match primitive {
            Primitive::Segment { area, edge } => {
                let SurfaceShape::Stroke { points, width_m } = &self.areas[*area].shape else {
                    unreachable!()
                };
                segment_distance(points[*edge], points[*edge + 1], p) <= width_m / 2.0 + margin
            }
            Primitive::Polygon { area } => {
                let SurfaceShape::Polygon { ring } = &self.areas[*area].shape else {
                    unreachable!()
                };
                polygon_contains(ring, p)
                    || (margin > 0.0
                        && edges(ring).any(|(a, b)| segment_distance(*a, *b, p) <= margin))
            }
        }
    }

    pub fn navigation_regions(&self) -> Vec<Rect> {
        self.primitives
            .iter()
            .filter(|primitive| self.areas[primitive.area()].kind == SurfaceKind::Road)
            .map(|primitive| match primitive {
                Primitive::Segment { area, edge } => {
                    let SurfaceShape::Stroke { points, width_m } = &self.areas[*area].shape else {
                        unreachable!()
                    };
                    bounds(&points[*edge..*edge + 2], *width_m / 2.0)
                }
                Primitive::Polygon { area } => {
                    let SurfaceShape::Polygon { ring } = &self.areas[*area].shape else {
                        unreachable!()
                    };
                    bounds(ring, 0.0)
                }
            })
            .collect()
    }
}

fn bounds(points: &[[f64; 2]], margin: f64) -> Rect {
    let [x, y, max_x, max_y] = limits(points, margin);
    [x, y, max_x - x, max_y - y]
}

// The broad phase keeps actual maxima: reconstructing them from a width can
// round below a closed bucket boundary, dropping an otherwise exact hit.
fn limits(points: &[[f64; 2]], margin: f64) -> [f64; 4] {
    let min = [0, 1].map(|i| points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min) - margin);
    let max = [0, 1].map(|i| {
        points
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
            + margin
    });
    [min[0], min[1], max[0], max[1]]
}

fn edges(ring: &[[f64; 2]]) -> impl Iterator<Item = (&[f64; 2], &[f64; 2])> {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
}

fn polygon_contains(ring: &[[f64; 2]], p: V2) -> bool {
    let mut inside = false;
    for (a, b) in edges(ring) {
        let cross = (b[0] - a[0]) * (p.y - a[1]) - (b[1] - a[1]) * (p.x - a[0]);
        if cross == 0.0
            && p.x >= a[0].min(b[0])
            && p.x <= a[0].max(b[0])
            && p.y >= a[1].min(b[1])
            && p.y <= a[1].max(b[1])
        {
            return true;
        }
        if (a[1] > p.y) != (b[1] > p.y) && p.x < a[0] + (b[0] - a[0]) * (p.y - a[1]) / (b[1] - a[1])
        {
            inside = !inside;
        }
    }
    inside
}

fn segment_distance(a: [f64; 2], b: [f64; 2], p: V2) -> f64 {
    let a = v2(a[0], a[1]);
    let ab = v2(b[0], b[1]) - a;
    let len2 = ab.dot(ab);
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - (a + ab * t)).length()
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn polygon_area(ring: &[[f64; 2]]) -> f64 {
    (1..ring.len() - 1)
        .map(|i| cross(ring[0], ring[i], ring[i + 1]))
        .sum()
}

fn edge_axis(a: [f64; 2], b: [f64; 2]) -> usize {
    usize::from((b[1] - a[1]).abs() > (b[0] - a[0]).abs())
}

fn edge_parameter(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let axis = edge_axis(a, b);
    (p[axis] - a[axis]) / (b[axis] - a[axis])
}

fn on_edge(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    if t == 0.0 {
        a
    } else if t == 1.0 {
        b
    } else {
        [0, 1].map(|i| a[i] + (b[i] - a[i]) * t)
    }
}

fn split_edge(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2], cuts: &mut Vec<f64>) {
    let ab = v2(b[0] - a[0], b[1] - a[1]);
    let cd = v2(d[0] - c[0], d[1] - c[1]);
    let ac = v2(c[0] - a[0], c[1] - a[1]);
    let denominator = ab.x * cd.y - ab.y * cd.x;
    if denominator != 0.0 {
        let t = (ac.x * cd.y - ac.y * cd.x) / denominator;
        let u = (ac.x * ab.y - ac.y * ab.x) / denominator;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            cuts.push(t);
        }
    } else if cross(a, b, c) == 0.0 && cross(a, b, d) == 0.0 {
        for p in [c, d] {
            let t = edge_parameter(a, b, p);
            if (0.0..=1.0).contains(&t) {
                cuts.push(t);
            }
        }
    }
}

fn boundary_order(kind: SurfaceKind, id: usize) -> (u8, usize) {
    (u8::from(kind == SurfaceKind::Sidewalk), id)
}

/// Ear clipping fills each authoritative ring. The exposed union boundary is
/// exported separately, so triangulation diagonals cannot become paved edges.
fn triangulate(ring: &[[f64; 2]], kind: SurfaceKind) -> Vec<f32> {
    let n = ring.len();
    for i in 0..n {
        assert!(
            ring[i] != ring[(i + 1) % n],
            "surface polygon has a repeated adjacent vertex"
        );
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (a, b, c, d) = (ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n]);
            let turns = [
                cross(a, b, c),
                cross(a, b, d),
                cross(c, d, a),
                cross(c, d, b),
            ];
            assert!(
                turns.iter().all(|v| v.is_finite()),
                "surface polygon arithmetic must be finite"
            );
            let intersects =
                turns[0].signum() != turns[1].signum() && turns[2].signum() != turns[3].signum();
            let on = |a: [f64; 2], b: [f64; 2], p: [f64; 2], turn: f64| {
                turn == 0.0 && (0..2).all(|k| p[k] >= a[k].min(b[k]) && p[k] <= a[k].max(b[k]))
            };
            assert!(
                !intersects
                    && !on(a, b, c, turns[0])
                    && !on(a, b, d, turns[1])
                    && !on(c, d, a, turns[2])
                    && !on(c, d, b, turns[3]),
                "surface polygon must be simple"
            );
        }
    }
    let area = polygon_area(ring);
    assert!(
        area.is_finite() && area != 0.0,
        "surface polygon must have finite nonzero area"
    );
    let winding = area.signum();
    // Collinear intermediate vertices do not change the closed boundary.
    let mut boundary: Vec<usize> = (0..n).collect();
    loop {
        let removable = (0..boundary.len()).find(|&i| {
            let len = boundary.len();
            cross(
                ring[boundary[(i + len - 1) % len]],
                ring[boundary[i]],
                ring[boundary[(i + 1) % len]],
            ) == 0.0
        });
        let Some(i) = removable else {
            break;
        };
        assert!(boundary.len() > 3, "surface polygon is degenerate");
        boundary.remove(i);
    }
    let mut remaining = boundary.clone();
    let mut out = Vec::with_capacity((remaining.len() - 2) * 7);
    while remaining.len() >= 3 {
        let len = remaining.len();
        let ear = (0..len)
            .find(|&i| {
                let (a, b, c) = (
                    remaining[(i + len - 1) % len],
                    remaining[i],
                    remaining[(i + 1) % len],
                );
                cross(ring[a], ring[b], ring[c]) * winding > 0.0
                    && remaining.iter().all(|&p| {
                        p == a
                            || p == b
                            || p == c
                            || [
                                cross(ring[a], ring[b], ring[p]),
                                cross(ring[b], ring[c], ring[p]),
                                cross(ring[c], ring[a], ring[p]),
                            ]
                            .iter()
                            .any(|t| *t * winding < 0.0)
                    })
            })
            .expect("surface polygon cannot be triangulated");
        let ids = [
            remaining[(ear + len - 1) % len],
            remaining[ear],
            remaining[(ear + 1) % len],
        ];
        for i in 0..3 {
            out.extend(ring[ids[i]].map(|v| v as f32));
        }
        out.push(super::export::surface_area_tag(kind));
        remaining.remove(ear);
        if len == 3 {
            break;
        }
    }
    assert!(
        out.iter().all(|v| v.is_finite()),
        "surface export must fit finite f32 coordinates"
    );
    out
}
