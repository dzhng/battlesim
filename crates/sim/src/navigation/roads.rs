//! The map's roads as a graph, built once at load from the road strokes
//! every side knows (public terrain; no body any side has learned is in
//! it). Nodes sit along each stroke's centreline and wherever two roads'
//! surfaces meet; an arc is a straight run of one road between two nodes,
//! with its length and its surface's share of road speed. A long move asks
//! the graph for a way first ([`super::journey`]), then checks that way on
//! its side's grid: the graph says where roads lead, never what fits.
use std::collections::BTreeSet;

use crate::math::{v2, V2};
use crate::world::{SurfaceKind, WorldGeometry};

use super::NAV_CELL_M;

/// Arcs are bucketed in squares this wide, for finding the roads near a
/// point and the roads that meet.
const BUCKET_M: f64 = 64.0;
/// A stroke's sampled bends are thinned to chords that stay this close to
/// its centreline.
const CHORD_M: f64 = 0.25;
/// Two points of the graph closer than this are one node.
const SAME_M: f64 = 0.5;

/// A straight run of road between two nodes, driven either way.
#[derive(Clone, Copy, Debug)]
pub struct Arc {
    pub ends: [u32; 2],
    pub length: f64,
    /// The surface kind's share of a mover's road speed.
    pub factor: f64,
    /// Half the road's width.
    pub half_width: f64,
    /// Its road runs over a physical bridge deck.
    pub bridge: bool,
}

/// A point of an arc near a place off the road.
#[derive(Clone, Copy, Debug)]
pub struct Access {
    pub arc: u32,
    /// The nearest point of the arc, and how far along the arc it is from
    /// its first end.
    pub at: V2,
    pub along: f64,
    /// How far off the place is.
    pub distance: f64,
}

#[derive(Default)]
pub struct RoadNet {
    nodes: Vec<V2>,
    arcs: Vec<Arc>,
    /// Node `n`'s arcs are `incident[starts[n]..starts[n + 1]]`.
    starts: Vec<u32>,
    incident: Vec<u32>,
    /// Arc ids by bucket, `buckets_x` to a row.
    buckets: Vec<Vec<u32>>,
    buckets_x: usize,
    buckets_y: usize,
}

/// One straight piece of a road stroke while the graph is built.
struct Piece {
    stroke: usize,
    /// How far along its stroke it starts.
    along: f64,
    ends: [V2; 2],
    half_width: f64,
    factor: f64,
    /// Where it is cut, as shares of its length, and the node at each cut.
    cuts: Vec<(f64, u32)>,
}

impl RoadNet {
    /// The graph of `world`'s road strokes. A run the terrain does not carry
    /// (water with no deck, ground too steep) is left out.
    pub fn build(world: &WorldGeometry) -> Self {
        let mut nodes: Vec<V2> = Vec::new();
        let mut pieces: Vec<Piece> = Vec::new();
        for (stroke, (samples, width, factor)) in world.road_strokes().enumerate() {
            let line = thin(samples);
            let first = nodes.len() as u32;
            nodes.extend(&line);
            let mut along = 0.0;
            for (k, run) in line.windows(2).enumerate() {
                pieces.push(Piece {
                    stroke,
                    along,
                    ends: [run[0], run[1]],
                    half_width: width / 2.0,
                    factor,
                    cuts: vec![(0.0, first + k as u32), (1.0, first + k as u32 + 1)],
                });
                along += (run[1] - run[0]).length();
            }
        }
        let (nx, ny) = (
            (world.width() / BUCKET_M).ceil().max(1.0) as usize,
            (world.depth() / BUCKET_M).ceil().max(1.0) as usize,
        );
        // Where two roads' surfaces meet, both are cut and joined.
        let mut links: Vec<Arc> = Vec::new();
        for (i, j) in meeting(&pieces, nx, ny) {
            let (a, b) = (&pieces[i], &pieces[j]);
            let (s, t) = closest(a.ends, b.ends);
            let (p, q) = (lerp(a.ends, s), lerp(b.ends, t));
            let gap = (p - q).length();
            if gap > a.half_width + b.half_width {
                continue;
            }
            let (factor, half_width) = (a.factor.min(b.factor), a.half_width.min(b.half_width));
            let at = cut(&mut pieces[i], s, &mut nodes, None);
            let shared = (gap <= SAME_M).then_some(at);
            let to = cut(&mut pieces[j], t, &mut nodes, shared);
            if at != to {
                links.push(Arc {
                    ends: [at, to],
                    length: gap,
                    factor,
                    half_width,
                    bridge: false,
                });
            }
        }
        let mut arcs = links;
        for piece in &mut pieces {
            piece
                .cuts
                .sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let length = (piece.ends[1] - piece.ends[0]).length();
            for run in piece.cuts.windows(2) {
                if run[0].1 != run[1].1 {
                    arcs.push(Arc {
                        ends: [run[0].1, run[1].1],
                        length: (run[1].0 - run[0].0) * length,
                        factor: piece.factor,
                        half_width: piece.half_width,
                        bridge: false,
                    });
                }
            }
        }
        arcs.retain_mut(|arc| {
            match carried(
                world,
                nodes[arc.ends[0] as usize],
                nodes[arc.ends[1] as usize],
            ) {
                Some(bridge) => {
                    arc.bridge = bridge;
                    true
                }
                None => false,
            }
        });
        let mut net = RoadNet {
            nodes,
            arcs,
            buckets: vec![Vec::new(); nx * ny],
            buckets_x: nx,
            buckets_y: ny,
            ..RoadNet::default()
        };
        let mut starts = vec![0u32; net.nodes.len() + 1];
        for arc in &net.arcs {
            for end in arc.ends {
                starts[end as usize + 1] += 1;
            }
        }
        for n in 0..net.nodes.len() {
            starts[n + 1] += starts[n];
        }
        let mut next = starts.clone();
        net.incident = vec![0; net.arcs.len() * 2];
        for (id, arc) in net.arcs.iter().enumerate() {
            for end in arc.ends {
                net.incident[next[end as usize] as usize] = id as u32;
                next[end as usize] += 1;
            }
            let ends = arc.ends.map(|n| net.nodes[n as usize]);
            for bucket in buckets_along(ends, 0.0, nx, ny) {
                net.buckets[bucket].push(id as u32);
            }
        }
        net.starts = starts;
        net
    }

    pub fn is_empty(&self) -> bool {
        self.arcs.is_empty()
    }

    pub fn node(&self, n: u32) -> V2 {
        self.nodes[n as usize]
    }

    pub fn arc(&self, id: u32) -> &Arc {
        &self.arcs[id as usize]
    }

    /// The arcs that meet at node `n`.
    pub fn arcs_at(&self, n: u32) -> &[u32] {
        &self.incident[self.starts[n as usize] as usize..self.starts[n as usize + 1] as usize]
    }

    /// The nearest point of every arc within `radius` of `p`, in arc order.
    pub(super) fn arc_count(&self) -> usize {
        self.arcs.len()
    }

    pub(super) fn access(&self, arc: u32, p: V2) -> Access {
        let ends = self.arcs[arc as usize].ends.map(|n| self.node(n));
        let share = project(ends, p);
        let at = lerp(ends, share);
        Access {
            arc,
            at,
            along: share * self.arcs[arc as usize].length,
            distance: (at - p).length(),
        }
    }

    pub fn near(&self, p: V2, radius: f64) -> Vec<Access> {
        if self.is_empty() {
            return Vec::new();
        }
        let span = |v: f64, n: usize| {
            let at = |v: f64| ((v / BUCKET_M).floor().max(0.0) as usize).min(n - 1);
            at(v - radius)..=at(v + radius)
        };
        let mut ids = BTreeSet::new();
        for j in span(p.y, self.buckets_y) {
            for i in span(p.x, self.buckets_x) {
                ids.extend(&self.buckets[j * self.buckets_x + i]);
            }
        }
        ids.into_iter()
            .filter_map(|arc| {
                let access = self.access(arc, p);
                (access.distance <= radius).then_some(access)
            })
            .collect()
    }
}

/// A stroke's samples with the points a chord can stand in for dropped: the
/// straight runs stay whole, each bend keeps a few points.
fn thin(samples: &[[f64; 2]]) -> Vec<V2> {
    let points: Vec<V2> = samples.iter().map(|p| v2(p[0], p[1])).collect();
    let mut kept = vec![points[0]];
    let mut anchor = 0;
    for end in 2..points.len() {
        let chord = [points[anchor], points[end]];
        let strays = (anchor + 1..end)
            .any(|k| (lerp(chord, project(chord, points[k])) - points[k]).length() > CHORD_M);
        if strays {
            anchor = end - 1;
            kept.push(points[anchor]);
        }
    }
    kept.push(*points.last().expect("a stroke has samples"));
    kept
}

/// The share of the way along `ends` nearest `p`.
fn project(ends: [V2; 2], p: V2) -> f64 {
    let d = ends[1] - ends[0];
    let length2 = d.dot(d);
    if length2 > 0.0 {
        ((p - ends[0]).dot(d) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn lerp(ends: [V2; 2], share: f64) -> V2 {
    ends[0] + (ends[1] - ends[0]) * share
}

/// The shares along two segments where they come closest.
fn closest(a: [V2; 2], b: [V2; 2]) -> (f64, f64) {
    let (d1, d2, r) = (a[1] - a[0], b[1] - b[0], a[0] - b[0]);
    let (aa, ee, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let c = d1.dot(r);
    let bb = d1.dot(d2);
    let denominator = aa * ee - bb * bb;
    // Parallel runs come equally close all along: take the first end's.
    let mut s = if denominator > 1e-12 {
        ((bb * f - c * ee) / denominator).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = if ee > 0.0 { (bb * s + f) / ee } else { 0.0 };
    if !(0.0..=1.0).contains(&t) {
        t = t.clamp(0.0, 1.0);
        s = if aa > 0.0 {
            ((bb * t - c) / aa).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    (s, t)
}

/// The buckets a run passes within `margin` of, walking it a bucket at a
/// time so a long diagonal claims only the buckets along it.
fn buckets_along(ends: [V2; 2], margin: f64, nx: usize, ny: usize) -> BTreeSet<usize> {
    let steps = ((ends[1] - ends[0]).length() / BUCKET_M).ceil().max(1.0) as usize;
    let at = |v: f64, n: usize| ((v / BUCKET_M).floor().max(0.0) as usize).min(n - 1);
    let mut out = BTreeSet::new();
    for k in 0..steps {
        let (a, b) = (
            lerp(ends, k as f64 / steps as f64),
            lerp(ends, (k + 1) as f64 / steps as f64),
        );
        for j in at(a.y.min(b.y) - margin, ny)..=at(a.y.max(b.y) + margin, ny) {
            for i in at(a.x.min(b.x) - margin, nx)..=at(a.x.max(b.x) + margin, nx) {
                out.insert(j * nx + i);
            }
        }
    }
    out
}

/// Every pair of pieces that share a bucket and are not near each other
/// along one stroke (a bend's pieces lie within the road's width of each
/// other without the road meeting itself): the pairs whose surfaces may
/// meet.
fn meeting(pieces: &[Piece], nx: usize, ny: usize) -> BTreeSet<(usize, usize)> {
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); nx * ny];
    for (id, piece) in pieces.iter().enumerate() {
        for bucket in buckets_along(piece.ends, piece.half_width, nx, ny) {
            buckets[bucket].push(id);
        }
    }
    let mut pairs = BTreeSet::new();
    for bucket in &buckets {
        for (k, &i) in bucket.iter().enumerate() {
            for &j in &bucket[k + 1..] {
                let (a, b) = (&pieces[i], &pieces[j]);
                let end = a.along + (a.ends[1] - a.ends[0]).length();
                if a.stroke != b.stroke || b.along - end > 4.0 * a.half_width {
                    pairs.insert((i, j));
                }
            }
        }
    }
    pairs
}

/// Cut `piece` at `share` of its length and return the node there: an
/// existing cut's node if one is close, `shared` if another road already
/// has a node at this very place, or a new node.
fn cut(piece: &mut Piece, share: f64, nodes: &mut Vec<V2>, shared: Option<u32>) -> u32 {
    let length = (piece.ends[1] - piece.ends[0]).length();
    if let Some(&(_, node)) = piece
        .cuts
        .iter()
        .find(|(at, _)| (at - share).abs() * length <= SAME_M)
    {
        return node;
    }
    let node = shared.unwrap_or_else(|| {
        nodes.push(lerp(piece.ends, share));
        nodes.len() as u32 - 1
    });
    piece.cuts.push((share, node));
    node
}

/// Whether the terrain carries a mover all the way from `a` to `b`.
fn carried(world: &WorldGeometry, a: V2, b: V2) -> Option<bool> {
    let steps = ((b - a).length() / NAV_CELL_M).ceil().max(1.0) as usize;
    let mut bridge = false;
    for k in 0..=steps {
        let p = lerp([a, b], k as f64 / steps as f64);
        let surface = world.surface_at(p.x, p.y)?;
        if !surface.traversable {
            return None;
        }
        bridge |= surface.kind == SurfaceKind::Bridge;
    }
    Some(bridge)
}
