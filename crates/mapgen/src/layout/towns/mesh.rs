//! How the pieces of a settlement's ground lie against each other.
use super::ground::{Leaf, CORNER_M, SHARED_M};
use crate::layout::geometry::{area, distance, segment_distance, Point};
use std::collections::{BTreeMap, BTreeSet};

/// Which leaves of a settlement's ground share an edge and which meet at a
/// corner. Every edge is split at each corner of a neighbour that stands on
/// it, so two leaves that share ground share whole pieces of edge.
pub(super) struct Mesh {
    pub(super) corners: Vec<Point>,
    /// Each leaf's edge pieces: (from corner, to corner, the leaf's edge).
    pub(super) pieces: Vec<Vec<(usize, usize, usize)>>,
    /// The leaf with a piece from the first corner to the second.
    pub(super) owner: BTreeMap<(usize, usize), usize>,
    /// The leaves that meet at each corner.
    pub(super) around: Vec<BTreeSet<usize>>,
    /// The leaves that share at least `SHARED_M` of edge with each leaf.
    beside: Vec<Vec<usize>>,
}

impl Mesh {
    pub(super) fn new(leaves: &[Leaf]) -> Self {
        // Corners by the two-corner-wide square each lies in: one nearer
        // than a corner's width is in that square or one beside it.
        let mut squares: BTreeMap<[i64; 2], Vec<usize>> = BTreeMap::new();
        let mut corners: Vec<Point> = Vec::new();
        let mut id = |p: Point| {
            let [x, y] = p.map(|v| libm::floor(v / (2.0 * CORNER_M)) as i64);
            let known = (x - 1..=x + 1)
                .flat_map(|x| (y - 1..=y + 1).map(move |y| [x, y]))
                .filter_map(|square| squares.get(&square))
                .flatten()
                .copied()
                .find(|corner| distance(corners[*corner], p) < CORNER_M);
            known.unwrap_or_else(|| {
                corners.push(p);
                squares.entry([x, y]).or_default().push(corners.len() - 1);
                corners.len() - 1
            })
        };
        let rings: Vec<Vec<usize>> = leaves
            .iter()
            .map(|leaf| leaf.ring.iter().map(|p| id(*p)).collect())
            .collect();
        let mut pieces = Vec::with_capacity(leaves.len());
        let mut owner = BTreeMap::new();
        for (leaf, ring) in rings.iter().enumerate() {
            let mut cut = Vec::new();
            for edge in 0..ring.len() {
                let (from, to) = (ring[edge], ring[(edge + 1) % ring.len()]);
                let (a, b) = (corners[from], corners[to]);
                let span = distance(a, b);
                let mut on: Vec<(f64, usize)> = (0..corners.len())
                    .filter(|corner| *corner != from && *corner != to)
                    .filter(|corner| segment_distance(a, b, corners[*corner]) < CORNER_M)
                    .map(|corner| (distance(a, corners[corner]) / span, corner))
                    .collect();
                on.sort_by(|x, y| x.0.total_cmp(&y.0));
                let mut last = from;
                for corner in on.into_iter().map(|(_, corner)| corner).chain([to]) {
                    cut.push((last, corner, edge));
                    owner.insert((last, corner), leaf);
                    last = corner;
                }
            }
            pieces.push(cut);
        }
        let mut around = vec![BTreeSet::new(); corners.len()];
        for (leaf, cut) in pieces.iter().enumerate() {
            for (from, to, _) in cut {
                around[*from].insert(leaf);
                around[*to].insert(leaf);
            }
        }
        let beside = pieces
            .iter()
            .map(|cut| {
                let mut shared: BTreeMap<usize, f64> = BTreeMap::new();
                for (from, to, _) in cut {
                    if let Some(other) = owner.get(&(*to, *from)) {
                        *shared.entry(*other).or_default() +=
                            distance(corners[*from], corners[*to]);
                    }
                }
                shared
                    .into_iter()
                    .filter(|(_, metres)| *metres >= SHARED_M)
                    .map(|(other, _)| other)
                    .collect()
            })
            .collect();
        Self {
            corners,
            pieces,
            owner,
            around,
            beside,
        }
    }

    /// Whether `leaf`, added to `ground`, would touch it at a corner where
    /// it shares no edge with it: the ground's edge would touch itself.
    pub(super) fn pinches(&self, leaf: usize, ground: &BTreeSet<usize>) -> bool {
        let across = |from: usize, to: usize| {
            self.owner
                .get(&(to, from))
                .is_some_and(|other| ground.contains(other))
        };
        self.pieces[leaf].iter().any(|(from, to, _)| {
            // Each corner once, as the end of the piece that arrives there:
            // the pieces of `leaf` on either side of it.
            let corner = *to;
            let onward = self.pieces[leaf]
                .iter()
                .find(|(start, ..)| *start == corner)
                .is_some_and(|(start, end, _)| across(*start, *end));
            !across(*from, *to)
                && !onward
                && self.around[corner]
                    .iter()
                    .any(|other| *other != leaf && ground.contains(other))
        })
    }

    /// The leaves that share at least `SHARED_M` of edge with `leaf`.
    pub(super) fn neighbours(&self, leaf: usize) -> &[usize] {
        &self.beside[leaf]
    }

    /// The edge of the union of `ground`, counter-clockwise: its largest
    /// loop, and the corners where the edge touched itself (none for ground
    /// that is one piece without a pinch).
    pub(super) fn outline(&self, ground: &BTreeSet<usize>) -> (Vec<Point>, Vec<usize>) {
        let mut next: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for leaf in ground {
            for (from, to, _) in &self.pieces[*leaf] {
                let across = self.owner.get(&(*to, *from));
                if !across.is_some_and(|other| ground.contains(other)) {
                    next.entry(*from).or_default().push(*to);
                }
            }
        }
        let pinched: Vec<usize> = next
            .iter()
            .filter(|(_, onward)| onward.len() > 1)
            .map(|(corner, _)| *corner)
            .collect();
        let mut best: Vec<Point> = Vec::new();
        while let Some(start) = next.keys().next().copied() {
            let mut ring = Vec::new();
            let mut at = start;
            while let Some(onward) = next.get_mut(&at) {
                let to = onward.pop().unwrap_or(start);
                if onward.is_empty() {
                    next.remove(&at);
                }
                ring.push(self.corners[at]);
                at = to;
                if at == start {
                    break;
                }
            }
            if ring.len() >= 3 && area(&ring) > area(&best) {
                best = ring;
            }
        }
        (best, pinched)
    }
}
