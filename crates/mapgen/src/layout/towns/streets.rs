//! A settlement's avenues: the streets along its blocks' edges where no
//! road runs.
use super::ground::{Bound, CORNER_M, MEET_M, SHARED_M};
use super::Plot;
use crate::layout::geometry::{
    add, distance, round_cm, scale, segment_crossing, segment_distance, sub, Point,
};
use crate::layout::sites::Site;
use std::collections::{BTreeMap, BTreeSet};

/// How far an avenue runs past the middle of the road it meets.
const JOIN_OVERSHOOT_M: f64 = 0.5;

/// Whether two stretches of carriageway meet.
pub(super) fn touch(a: [Point; 2], b: [Point; 2]) -> bool {
    segment_crossing(a[0], a[1], b[0], b[1]).is_some()
        || segment_distance(a[0], a[1], b[0]) <= MEET_M
        || segment_distance(a[0], a[1], b[1]) <= MEET_M
        || segment_distance(b[0], b[1], a[0]) <= MEET_M
        || segment_distance(b[0], b[1], a[1]) <= MEET_M
}

/// One stretch of a cut that may carry a street.
struct Stretch {
    ends: [Point; 2],
    /// The one block it bounds, when open ground lies on its other side;
    /// `None` between two blocks.
    edge_of: Option<usize>,
    /// Every built block with an edge along it.
    blocks: Vec<usize>,
}

/// The streets along the built blocks' edges that no road covers. A street
/// runs between two blocks where it leads on to another street or road at
/// both ends. Along a block's edge that faces open ground, and where a
/// street would end in the fields, one runs only where it is needed: to
/// lead to a street between blocks, or to give a block's parcels a front
/// (`fronted` says whether a block has one, given the streets). An edge no
/// street or road leads to stays a field's edge.
pub(super) fn avenues(
    site: &Site,
    plot: &Plot,
    built: &BTreeSet<usize>,
    fronted: &dyn Fn(usize, &[[Point; 2]]) -> bool,
) -> Vec<[Point; 2]> {
    let (ground, mesh) = (&plot.ground, &plot.mesh);
    // Along each cut, the pieces of edge a built block stands on, with the
    // block when no other is built across the piece.
    type Piece = ([f64; 2], [Point; 2], Option<usize>, Vec<usize>);
    let mut along: BTreeMap<usize, Vec<Piece>> = BTreeMap::new();
    // A block's edge along a cut that another block shares any of, off a
    // road, is a street for its whole length: the street between two blocks
    // runs on to the corner of the longer one, not to where the shorter one
    // stops.
    let backed = |from: usize, to: usize| {
        mesh.owner
            .get(&(to, from))
            .is_some_and(|other| built.contains(other))
    };
    let shared: BTreeSet<(usize, usize)> = built
        .iter()
        .flat_map(|leaf| {
            mesh.pieces[*leaf]
                .iter()
                .filter(move |(from, to, _)| backed(*from, *to))
                .filter_map(move |(from, to, edge)| {
                    let Bound::Cut(id) = ground.leaves[*leaf].bounds[*edge] else {
                        return None;
                    };
                    let cut = &ground.cuts[id];
                    let middle = cut.at(scale(add(mesh.corners[*from], mesh.corners[*to]), 0.5));
                    let paved = cut
                        .paved
                        .iter()
                        .any(|(span, _)| span[0] <= middle && middle <= span[1]);
                    (!paved).then_some((*leaf, id))
                })
        })
        .collect();
    for leaf in built {
        for (from, to, edge) in &mesh.pieces[*leaf] {
            let Bound::Cut(id) = ground.leaves[*leaf].bounds[*edge] else {
                continue;
            };
            let backed = backed(*from, *to) || shared.contains(&(*leaf, id));
            let cut = &ground.cuts[id];
            let (a, b) = (mesh.corners[*from], mesh.corners[*to]);
            let (s, t) = (cut.at(a), cut.at(b));
            let edge_of = (!backed).then_some(*leaf);
            along.entry(id).or_default().push(if s <= t {
                ([s, t], [a, b], edge_of, vec![*leaf])
            } else {
                ([t, s], [b, a], edge_of, vec![*leaf])
            });
        }
    }
    let mut stretches: Vec<Stretch> = Vec::new();
    for (id, mut pieces) in along {
        let cut = &ground.cuts[id];
        pieces.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]).then(a.2.cmp(&b.2)));
        // Joined end to end into runs, each between two blocks or along one.
        let mut runs: Vec<Piece> = Vec::new();
        for (span, ends, edge_of, blocks) in pieces {
            match runs.last_mut() {
                Some((run, run_ends, known, beside))
                    if *known == edge_of && span[0] <= run[1] + CORNER_M =>
                {
                    if span[1] > run[1] {
                        run[1] = span[1];
                        run_ends[1] = ends[1];
                    }
                    beside.extend(blocks);
                }
                _ => runs.push((span, ends, edge_of, blocks)),
            }
        }
        // A run too short to be a street of its own is part of the run it
        // carries on from, so the street has no gap there.
        let mut joined: Vec<Piece> = Vec::new();
        for (span, ends, edge_of, blocks) in runs {
            match joined.last_mut() {
                Some((run, run_ends, known, beside))
                    if span[0] <= run[1] + CORNER_M
                        && (span[1] - span[0]).min(run[1] - run[0]) < SHARED_M =>
                {
                    if span[1] - span[0] > run[1] - run[0] {
                        *known = edge_of;
                    }
                    if span[1] > run[1] {
                        run[1] = span[1];
                        run_ends[1] = ends[1];
                    }
                    beside.extend(blocks);
                }
                _ => joined.push((span, ends, edge_of, blocks)),
            }
        }
        let runs = joined;
        // Less what a road already paves: the street starts on the road's
        // own end, so the two share that point.
        let mut paved = cut.paved.clone();
        paved.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
        for (mut run, mut ends, edge_of, mut blocks) in runs {
            blocks.sort_unstable();
            blocks.dedup();
            for (road, road_ends) in &paved {
                if road[1] <= run[0] + CORNER_M || road[0] >= run[1] - CORNER_M {
                    continue;
                }
                if road[0] > run[0] + SHARED_M {
                    stretches.push(Stretch {
                        ends: [ends[0], road_ends[0]],
                        edge_of,
                        blocks: blocks.clone(),
                    });
                }
                run[0] = road[1];
                ends[0] = road_ends[1];
            }
            if run[1] - run[0] >= SHARED_M {
                stretches.push(Stretch {
                    ends,
                    edge_of,
                    blocks,
                });
            }
        }
    }

    // Which stretches meet, and which a road leads to directly.
    let count = stretches.len();
    let meets: Vec<Vec<usize>> = (0..count)
        .map(|a| {
            (0..count)
                .filter(|b| a != *b && touch(stretches[a].ends, stretches[*b].ends))
                .collect()
        })
        .collect();
    let rooted: Vec<bool> = stretches
        .iter()
        .map(|stretch| {
            plot.roads
                .iter()
                .any(|(a, b, _)| touch(stretch.ends, [*a, *b]))
        })
        .collect();
    // The stretches of `kept` a road leads to, directly or by others.
    let reach = |kept: &[bool]| {
        let mut reached = vec![false; count];
        let mut walk: Vec<usize> = (0..count).filter(|i| kept[*i] && rooted[*i]).collect();
        for start in &walk {
            reached[*start] = true;
        }
        while let Some(from) = walk.pop() {
            for other in &meets[from] {
                if kept[*other] && !reached[*other] {
                    reached[*other] = true;
                    walk.push(*other);
                }
            }
        }
        reached
    };
    let streets_of = |reached: &[bool]| -> Vec<[Point; 2]> {
        (0..count)
            .filter(|i| reached[*i])
            .map(|i| stretches[i].ends)
            .collect()
    };
    let mut kept = vec![true; count];
    let whole = reach(&kept);
    // The edges that face open ground are dropped, longest first, wherever
    // every street between blocks is still led to and the edge's own block
    // still has a front for its parcels.
    let mut edges: Vec<usize> = (0..count)
        .filter(|i| stretches[*i].edge_of.is_some())
        .collect();
    edges.sort_by(|a, b| {
        let span = |i: &usize| distance(stretches[*i].ends[0], stretches[*i].ends[1]);
        span(b).total_cmp(&span(a)).then(a.cmp(b))
    });
    let all = streets_of(&whole);
    let needed: BTreeSet<usize> = built
        .iter()
        .copied()
        .filter(|leaf| fronted(*leaf, &all))
        .collect();
    // Whether every block that had a front keeps one with these streets.
    let still_fronted = |blocks: &[usize], reached: &[bool]| {
        let streets = streets_of(reached);
        blocks
            .iter()
            .all(|leaf| !needed.contains(leaf) || fronted(*leaf, &streets))
    };
    for edge in edges {
        kept[edge] = false;
        let reached = reach(&kept);
        let between_led =
            (0..count).all(|i| stretches[i].edge_of.is_some() || reached[i] == whole[i]);
        if !between_led || !still_fronted(&stretches[edge].blocks, &reached) {
            kept[edge] = true;
        }
    }
    // Then the streets that end in the fields, one at a time, wherever the
    // rest are still led to and the blocks beside still have a front.
    loop {
        let reached = reach(&kept);
        let open_end = |at: usize| {
            stretches[at].ends.iter().any(|end| {
                let point = [*end, *end];
                !plot.roads.iter().any(|(a, b, _)| touch(point, [*a, *b]))
                    && !(0..count).any(|other| {
                        other != at && reached[other] && touch(point, stretches[other].ends)
                    })
            })
        };
        let dead_end = (0..count).filter(|at| reached[*at]).find(|at| {
            if stretches[*at].edge_of.is_none() || !open_end(*at) {
                return false;
            }
            let mut without = kept.clone();
            without[*at] = false;
            let left = reach(&without);
            (0..count).all(|other| other == *at || left[other] == reached[other])
                && still_fronted(&stretches[*at].blocks, &left)
        });
        match dead_end {
            Some(at) => kept[at] = false,
            None => break,
        }
    }
    let reached = reach(&kept);

    // An end that meets another carriageway, a road or one of these streets,
    // runs just past its middle, so its square end lies inside that
    // carriageway's width (where a rounded bend has carried the middle
    // farther off, the plan's joint pass runs it on). An end on a road's own
    // end or on another street's stops there: the two share that point. So
    // does one that meets no road at the settlement's limit.
    let streets = streets_of(&reached);

    // The one point that the streets' ends within half a metre of `p` share,
    // when another street ends there too.
    let shared = |p: Point| {
        let ends = || {
            streets
                .iter()
                .flatten()
                .copied()
                .filter(|end| distance(*end, p) <= MEET_M)
        };
        (ends().count() > 1)
            .then(|| ends().min_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))))
            .flatten()
    };
    let limit = &site.outline.ring;
    let on_limit = |p: Point| {
        contract::ground::edges(limit).any(|(a, b)| segment_distance(*a, *b, p) <= MEET_M)
    };
    // The end of a stretch of road that lies at `p`, to the half metre.
    let road_end = |p: Point| {
        ground
            .cuts
            .iter()
            .flat_map(|cut| cut.paved.iter().flat_map(|(_, ends)| ends))
            .copied()
            .find(|end| distance(*end, p) <= MEET_M)
    };
    streets
        .iter()
        .map(|[a, b]| {
            let (a, b) = (*a, *b);
            let toward = scale(sub(b, a), 1.0 / distance(a, b));
            let on_road = |p: Point| plot.roads.iter().any(|(a, b, _)| touch([p, p], [*a, *b]));
            let end = |p: Point, way: f64| match road_end(p).or(shared(p)) {
                Some(end) => end,
                None if on_limit(p) && !on_road(p) => p,
                None => add(p, scale(toward, way * JOIN_OVERSHOOT_M)),
            };
            [round_cm(end(a, -1.0)), round_cm(end(b, 1.0))]
        })
        .collect()
}
