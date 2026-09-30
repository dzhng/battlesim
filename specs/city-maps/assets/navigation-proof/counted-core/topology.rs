use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);
impl Point {
    pub fn distance(self, b: Self) -> f64 {
        let (dx, dy) = (self.0 - b.0, self.1 - b.1);
        (dx * dx + dy * dy).sqrt()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
}
#[path = "road_index.rs"]
mod road_index;
pub use road_index::{QueryCursor, QueryStep, RoadIndex};
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub a: u32,
    pub b: u32,
    pub segment: Segment,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct Preparation {
    pub bounds_nodes: usize,
    pub bounds_primitives: usize,
    pub pairs: usize,
    pub cuts: usize,
    pub sort_comparisons: usize,
    pub vertex_operations: usize,
    pub arcs: usize,
    pub index_comparisons: usize,
    pub index_bounds: usize,
    pub query_sort_comparisons: usize,
}
pub struct Prepared {
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub incident: Vec<Vec<u32>>,
    pub index: RoadIndex,
    pub identity: u64,
    pub preparation: Preparation,
}
pub fn hash_word(mut v: u64) -> u64 {
    v ^= v >> 30;
    v = v.wrapping_mul(0xbf58476d1ce4e5b9);
    v ^= v >> 27;
    v = v.wrapping_mul(0x94d049bb133111eb);
    v ^ (v >> 31)
}
pub fn fold(a: u64, b: u64) -> u64 {
    hash_word(a ^ b.wrapping_add(0x9e3779b97f4a7c15))
}
pub fn point_hash(p: Point) -> u64 {
    fold(p.0.to_bits(), p.1.to_bits())
}
fn key(p: Point) -> (u64, u64) {
    (
        if p.0 == 0.0 { 0 } else { p.0.to_bits() },
        if p.1 == 0.0 { 0 } else { p.1.to_bits() },
    )
}
fn delta(a: Point, b: Point) -> Point {
    Point(a.0 - b.0, a.1 - b.1)
}
fn cross(a: Point, b: Point) -> f64 {
    a.0 * b.1 - a.1 * b.0
}
fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}
fn lerp(a: Point, d: Point, t: f64) -> Point {
    Point(a.0 + d.0 * t, a.1 + d.1 * t)
}
impl Prepared {
    // Loading only: counts actual candidate, sort and insertion work; never a request poll.
    pub fn prepare(roads: &[Segment]) -> Arc<Self> {
        let index = RoadIndex::new(roads);
        let mut metrics = road_index::Metrics::default();
        let mut work = Preparation::default();
        let mut split: Vec<Vec<(f64, Point)>> =
            roads.iter().map(|r| vec![(0.0, r.a), (1.0, r.b)]).collect();
        for i in 0..roads.len() {
            for j in index
                .query(index.bounds(i), &mut metrics)
                .into_iter()
                .filter(|&j| j > i)
            {
                work.pairs += 1;
                let (a, b) = (roads[i], roads[j]);
                let da = delta(a.b, a.a);
                let db = delta(b.b, b.a);
                let d = delta(b.a, a.a);
                let denom = cross(da, db);
                if denom != 0.0 {
                    let t = cross(d, db) / denom;
                    let u = cross(d, da) / denom;
                    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                        let p = lerp(a.a, da, t);
                        split[i].push((t, p));
                        split[j].push((u, p));
                        work.cuts += 2;
                    }
                } else if cross(d, da) == 0.0 {
                    for (owner, other) in [(i, j), (j, i)] {
                        let r = roads[owner];
                        let ab = delta(r.b, r.a);
                        let len = dot(ab, ab);
                        if len == 0.0 {
                            continue;
                        }
                        for p in [roads[other].a, roads[other].b] {
                            let t = dot(delta(p, r.a), ab) / len;
                            if (0.0..=1.0).contains(&t) {
                                split[owner].push((t, p));
                                work.cuts += 1;
                            }
                        }
                    }
                }
            }
        }
        work.index_comparisons = index.build_comparisons;
        work.index_bounds = index.build_bounds;
        work.query_sort_comparisons = metrics.sort_comparisons;
        work.bounds_nodes = metrics.nodes;
        work.bounds_primitives = metrics.primitives;
        let mut points = Vec::new();
        let mut incident: Vec<Vec<u32>> = Vec::new();
        let mut ids = BTreeMap::new();
        let mut edges = Vec::new();
        for list in &mut split {
            list.sort_by(|a, b| {
                work.sort_comparisons += 1;
                a.0.total_cmp(&b.0)
            });
            list.dedup_by(|a, b| a.1 == b.1);
            for pair in list.windows(2) {
                let mut ends = [0; 2];
                for (k, p) in [pair[0].1, pair[1].1].into_iter().enumerate() {
                    work.vertex_operations += 1;
                    ends[k] = if let Some(&id) = ids.get(&key(p)) {
                        id
                    } else {
                        let id = u32::try_from(points.len()).expect("preparation index admission");
                        points.push(p);
                        incident.push(Vec::new());
                        ids.insert(key(p), id);
                        id
                    };
                }
                let id = u32::try_from(edges.len()).expect("preparation edge admission");
                edges.push(Edge {
                    a: ends[0],
                    b: ends[1],
                    segment: Segment {
                        a: pair[0].1,
                        b: pair[1].1,
                    },
                });
                incident[ends[0] as usize].push(id);
                incident[ends[1] as usize].push(id);
                work.arcs += 2;
            }
        }
        let segments: Vec<_> = edges.iter().map(|e| e.segment).collect();
        let index = RoadIndex::new(&segments);
        work.index_comparisons += index.build_comparisons;
        work.index_bounds += index.build_bounds;
        let mut identity = fold(points.len() as u64, edges.len() as u64);
        for &p in &points {
            identity = fold(identity, point_hash(p));
        }
        for e in &edges {
            identity = fold(identity, fold(e.a as u64, e.b as u64));
        }
        Arc::new(Self {
            points,
            edges,
            incident,
            index,
            identity,
            preparation: work,
        })
    }
    pub fn point(&self, id: u32) -> Point {
        self.points[(id - 2) as usize]
    }
}
