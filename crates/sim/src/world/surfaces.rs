//! Sparse broad phase over the exact authored ground shapes: the paving and
//! the rivers.
use std::collections::{BTreeSet, HashMap};

use contract::ground::{
    cross, edges, limits, polygon_area, polygon_contains, segment_distance, stretch_contains,
    stretches, triangulate, GroundShape,
};
use contract::map::{Rect, SurfaceArea, SurfaceKind};
use contract::river::{section, River, Section};

use crate::math::{v2, V2};

const BUCKET_M: f64 = 128.0;

enum Primitive {
    /// One stretch of a stroke, with the ends it is cut square at
    /// (`contract::ground::stretches`).
    Segment {
        area: usize,
        edge: usize,
        cuts: u8,
    },
    Polygon {
        area: usize,
    },
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
    /// Each primitive's limits `[x, y, max_x, max_y]`: a point outside
    /// them, by more than rounding, is outside the primitive.
    limits: Vec<[f64; 4]>,
    buckets: HashMap<(i32, i32), Vec<usize>>,
    last: [i32; 2],
    triangles: Vec<f32>,
    boundaries: Vec<f32>,
    rivers: Vec<River>,
    /// Each bucket's river stretches: the river, and the first of the
    /// stretch's two rounded samples.
    stretches: HashMap<(i32, i32), Vec<(u32, u32)>>,
}

impl SurfaceIndex {
    pub fn new(areas: &[SurfaceArea], rivers: &[River], size: [f64; 2]) -> Self {
        let mut index = Self {
            areas: areas.to_vec(),
            primitives: Vec::new(),
            limits: Vec::new(),
            buckets: HashMap::new(),
            last: size.map(|v| (v / BUCKET_M).floor() as i32),
            triangles: Vec::new(),
            boundaries: Vec::new(),
            rivers: rivers.to_vec(),
            stretches: HashMap::new(),
        };
        for (river, definition) in rivers.iter().enumerate() {
            for (stretch, pair) in definition.samples().windows(2).enumerate() {
                let reach = pair[0].half_width_m.max(pair[1].half_width_m);
                let [x0, y0, x1, y1] =
                    index.bucket_bounds(limits(&[pair[0].xy, pair[1].xy], reach));
                for j in y0..=y1 {
                    for i in x0..=x1 {
                        index
                            .stretches
                            .entry((i, j))
                            .or_default()
                            .push((river as u32, stretch as u32));
                    }
                }
            }
        }
        for (area, surface) in areas.iter().enumerate() {
            match &surface.shape {
                GroundShape::Stroke {
                    centerline,
                    width_m,
                } => {
                    let half = *width_m / 2.0;
                    for (edge, (a, b, cuts)) in stretches(centerline.samples(), half).enumerate() {
                        index.insert(
                            Primitive::Segment { area, edge, cuts },
                            limits(&[a, b], half),
                        );
                    }
                }
                GroundShape::Polygon { ring } => {
                    for triangle in triangulate(ring).expect("validated ground ring") {
                        index
                            .triangles
                            .extend(triangle.into_iter().flatten().map(|v| v as f32));
                        index
                            .triangles
                            .push(super::export::surface_area_tag(surface.kind));
                    }
                    index.insert(Primitive::Polygon { area }, limits(ring, 0.0));
                }
            }
        }
        assert!(
            index.triangles.iter().all(|v| v.is_finite()),
            "surface export must fit finite f32 coordinates"
        );
        index.boundaries = index.build_boundaries();
        index
    }

    fn insert(&mut self, primitive: Primitive, bounds: [f64; 4]) {
        let id = self.primitives.len();
        self.primitives.push(primitive);
        self.limits.push(bounds);
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
                GroundShape::Polygon { ring } => polygon_area(ring).signum(),
                GroundShape::Stroke { .. } => 0.0,
            })
            .collect();
        let mut out = Vec::new();
        for (id, area) in self.areas.iter().enumerate() {
            let GroundShape::Polygon { ring } = &area.shape else {
                continue;
            };
            for (a, b) in edges(ring) {
                let candidates = self.boundary_candidates(*a, *b);
                let mut cuts = vec![0.0, 1.0];
                for &other in &candidates {
                    if other == id {
                        continue;
                    }
                    let GroundShape::Polygon { ring } = &self.areas[other].shape else {
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
                        let GroundShape::Polygon { ring } = &self.areas[other].shape else {
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
                        polygon_contains(ring, point)
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
                    matches!(area.shape, GroundShape::Polygon { .. }).then_some(i)
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

    /// Where kinds overlap the earliest `SurfaceKind` wins (a road over a
    /// track, any carriageway over a sidewalk); water and bridges are
    /// resolved by the world.
    pub fn at(&self, p: V2) -> Option<SurfaceKind> {
        let key = (
            (p.x / BUCKET_M).floor() as i32,
            (p.y / BUCKET_M).floor() as i32,
        );
        let mut result = None;
        // Far more than the rounding of a containment test.
        const HAIR_M: f64 = 1e-6;
        for &id in self.buckets.get(&key)? {
            let [x, y, max_x, max_y] = self.limits[id];
            if p.x < x - HAIR_M || p.x > max_x + HAIR_M || p.y < y - HAIR_M || p.y > max_y + HAIR_M
            {
                continue;
            }
            let primitive = &self.primitives[id];
            if self.contains(primitive, p, 0.0) {
                let kind = self.areas[primitive.area()].kind;
                if kind == SurfaceKind::Road {
                    return Some(kind);
                }
                result = Some(result.map_or(kind, |best: SurfaceKind| best.min(kind)));
            }
        }
        result
    }

    pub fn rivers(&self) -> &[River] {
        &self.rivers
    }

    /// The cross-sections at `p` of every stretch whose water lies within
    /// `reach` of it, each with its river's index. A stretch may be given
    /// more than once, and stretches farther off may be given too.
    pub fn river_sections(&self, p: V2, reach: f64) -> impl Iterator<Item = (usize, Section)> + '_ {
        let [x0, y0, x1, y1] =
            self.bucket_bounds([p.x - reach, p.y - reach, p.x + reach, p.y + reach]);
        (y0..=y1)
            .flat_map(move |j| (x0..=x1).map(move |i| (i, j)))
            .filter_map(|key| self.stretches.get(&key))
            .flatten()
            .map(move |&(river, stretch)| {
                let samples = self.rivers[river as usize].samples();
                let stretch = stretch as usize;
                (
                    river as usize,
                    section(&samples[stretch], &samples[stretch + 1], [p.x, p.y]),
                )
            })
    }

    /// Whether `p` is water: inside the closed edge of any river's stretch.
    pub fn water_at(&self, p: V2) -> bool {
        !self.rivers.is_empty() && self.river_sections(p, 0.0).any(|(_, s)| s.inside_m >= 0.0)
    }

    /// Whether `p` is in the water of river `river`.
    pub fn in_river(&self, river: usize, p: V2) -> bool {
        self.river_sections(p, 0.0)
            .any(|(id, s)| id == river && s.inside_m >= 0.0)
    }

    /// Whether `p` is water or within `margin` of its edge.
    pub fn water_near(&self, p: V2, margin: f64) -> bool {
        !self.rivers.is_empty()
            && self
                .river_sections(p, margin)
                .any(|(_, s)| s.inside_m >= -margin)
    }

    pub fn road_near(&self, p: V2, margin: f64) -> bool {
        let [x0, y0, x1, y1] =
            self.bucket_bounds([p.x - margin, p.y - margin, p.x + margin, p.y + margin]);
        for j in y0..=y1 {
            for i in x0..=x1 {
                if self.buckets.get(&(i, j)).is_some_and(|ids| {
                    ids.iter().any(|&id| {
                        let primitive = &self.primitives[id];
                        self.areas[primitive.area()].kind.is_road()
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
            Primitive::Segment { area, edge, cuts } => {
                let GroundShape::Stroke {
                    centerline,
                    width_m,
                } = &self.areas[*area].shape
                else {
                    unreachable!()
                };
                let points = centerline.samples();
                stretch_contains(
                    points[*edge],
                    points[*edge + 1],
                    *cuts,
                    width_m / 2.0,
                    [p.x, p.y],
                    margin,
                )
            }
            Primitive::Polygon { area } => {
                let GroundShape::Polygon { ring } = &self.areas[*area].shape else {
                    unreachable!()
                };
                polygon_contains(ring, [p.x, p.y])
                    || (margin > 0.0
                        && edges(ring).any(|(a, b)| segment_distance(*a, *b, [p.x, p.y]) <= margin))
            }
        }
    }

    pub fn navigation_regions(&self) -> Vec<Rect> {
        self.primitives
            .iter()
            .filter(|primitive| self.areas[primitive.area()].kind.is_road())
            .map(|primitive| match primitive {
                Primitive::Segment { area, edge, .. } => {
                    let GroundShape::Stroke {
                        centerline,
                        width_m,
                    } = &self.areas[*area].shape
                    else {
                        unreachable!()
                    };
                    bounds(&centerline.samples()[*edge..*edge + 2], *width_m / 2.0)
                }
                Primitive::Polygon { area } => {
                    let GroundShape::Polygon { ring } = &self.areas[*area].shape else {
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
    (u8::from(!kind.is_road()), id)
}
