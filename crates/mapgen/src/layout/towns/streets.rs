//! A settlement's avenues: the streets along its blocks' edges where no
//! road runs.
use super::ground::{Bound, CORNER_M};
use super::Plot;
use crate::layout::geometry::{
    add, cross, distance, dot, round_cm, scale, segment_crossing, segment_distance, sub, Point,
};
use crate::layout::sites::Site;
use contract::map::SurfaceKind;
use std::collections::{BTreeMap, BTreeSet};

/// How far an avenue runs past the middle of the road it meets.
const JOIN_OVERSHOOT_M: f64 = 0.5;
/// A street within this of a road's line (a sine) runs along it.
const ALONG_SIN: f64 = 0.02;

/// Whether two stretches of carriageway meet.
pub(super) fn touch(policy: &crate::layout::TownGeometry, a: [Point; 2], b: [Point; 2]) -> bool {
    segment_crossing(a[0], a[1], b[0], b[1]).is_some()
        || segment_distance(a[0], a[1], b[0]) <= policy.meet_m
        || segment_distance(a[0], a[1], b[1]) <= policy.meet_m
        || segment_distance(b[0], b[1], a[0]) <= policy.meet_m
        || segment_distance(b[0], b[1], a[1]) <= policy.meet_m
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
/// both ends; `lined` are the blocks that count as the far side of one, the
/// built blocks and the parks. One that would meet a road at a slant stops
/// at the last corner of a block `short` or more before it: the blocks
/// between front the road. Each street comes with the kind of the road it
/// carries on from, where it does. Along a block's edge that faces open ground, and where a
/// street would end in the fields, one runs only where it is needed: to
/// lead to a street between blocks, or to give a block's parcels a front
/// (`fronted` says whether a block has one, given the streets). An edge no
/// street or road leads to stays a field's edge.
pub(super) fn avenues(
    site: &Site,
    plot: &Plot,
    built: &BTreeSet<usize>,
    lined: &BTreeSet<usize>,
    fronted: &dyn Fn(usize, &[[Point; 2]]) -> bool,
    short: f64,
) -> Vec<([Point; 2], Option<SurfaceKind>)> {
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
            .is_some_and(|other| lined.contains(other))
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
        // The blocks' corners along the cut.
        let mut corners: Vec<f64> = pieces.iter().flat_map(|piece| piece.0).collect();
        corners.sort_by(f64::total_cmp);
        // Where a road crosses the cut's line, or ends on it, at a slant.
        let slanted: Vec<f64> = plot
            .roads
            .iter()
            .filter_map(|(a, b, _)| {
                let (from, to) = (cut.aside(*a), cut.aside(*b));
                let step = sub(*b, *a);
                let slant = dot(cut.along, step).abs() / distance(*a, *b);
                // On the line's two sides, or with an end on it.
                let meets = from.min(to) <= plot.ground.policy.meet_m
                    && from.max(to) >= -plot.ground.policy.meet_m;
                (meets
                    && slant > plot.ground.policy.avenue_slant_sin
                    && (from - to).abs() > plot.ground.policy.meet_m)
                    .then(|| cut.at(add(*a, scale(step, from / (from - to)))))
            })
            .collect();
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
                        && (span[1] - span[0]).min(run[1] - run[0])
                            < plot.ground.policy.shared_m =>
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
        // A run that meets a road at a slant loses the stretch either side
        // of it, back to the first corner far enough off.
        let mut straight: Vec<Piece> = Vec::new();
        for (span, ends, edge_of, blocks) in joined {
            let mut kept = vec![span];
            for at in slanted.iter().filter(|at| {
                **at >= span[0] - plot.ground.policy.meet_m
                    && **at <= span[1] + plot.ground.policy.meet_m
            }) {
                let before = corners
                    .iter()
                    .copied()
                    .rfind(|corner| *corner <= at - short);
                let after = corners.iter().copied().find(|corner| *corner >= at + short);
                kept = kept
                    .into_iter()
                    .flat_map(|[from, to]| {
                        [
                            [from, to.min(before.unwrap_or(f64::NEG_INFINITY))],
                            [from.max(after.unwrap_or(f64::INFINITY)), to],
                        ]
                    })
                    .filter(|[from, to]| to - from >= plot.ground.policy.shared_m)
                    .collect();
            }
            for [from, to] in kept {
                let place = |at: f64, end: Point| {
                    if (at - cut.at(end)).abs() <= CORNER_M {
                        end
                    } else {
                        add(cut.origin, scale(cut.along, at))
                    }
                };
                let ends = [place(from, ends[0]), place(to, ends[1])];
                straight.push(([from, to], ends, edge_of, blocks.clone()));
            }
        }
        let runs = straight;
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
                if road[0] > run[0] + plot.ground.policy.shared_m {
                    stretches.push(Stretch {
                        ends: [ends[0], road_ends[0]],
                        edge_of,
                        blocks: blocks.clone(),
                    });
                }
                run[0] = road[1];
                ends[0] = road_ends[1];
            }
            if run[1] - run[0] >= plot.ground.policy.shared_m {
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
                .filter(|b| {
                    a != *b && touch(&plot.ground.policy, stretches[a].ends, stretches[*b].ends)
                })
                .collect()
        })
        .collect();
    let rooted: Vec<bool> = stretches
        .iter()
        .map(|stretch| {
            plot.roads
                .iter()
                .any(|(a, b, _)| touch(&plot.ground.policy, stretch.ends, [*a, *b]))
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
                !plot
                    .roads
                    .iter()
                    .any(|(a, b, _)| touch(&plot.ground.policy, point, [*a, *b]))
                    && !(0..count).any(|other| {
                        other != at
                            && reached[other]
                            && touch(&plot.ground.policy, point, stretches[other].ends)
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
                .filter(|end| distance(*end, p) <= plot.ground.policy.meet_m)
        };
        (ends().count() > 1)
            .then(|| ends().min_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))))
            .flatten()
    };
    let limit = &site.outline.ring;
    let on_limit = |p: Point| {
        contract::ground::edges(limit)
            .any(|(a, b)| segment_distance(*a, *b, p) <= plot.ground.policy.meet_m)
    };
    // The end of a stretch of road that lies at `p`, to the half metre.
    let road_end = |p: Point| {
        ground
            .cuts
            .iter()
            .flat_map(|cut| cut.paved.iter().flat_map(|(_, ends)| ends))
            .copied()
            .find(|end| distance(*end, p) <= plot.ground.policy.meet_m)
    };
    // A street that starts on a road's end and runs on along its line is
    // that road carried on to the next junction: a road keeps its kind and
    // width from junction to junction.
    let carried = |p: Point, toward: Point| {
        ground
            .cuts
            .iter()
            .filter(|cut| cross(cut.along, toward).abs() <= ALONG_SIN)
            .find(|cut| {
                cut.paved
                    .iter()
                    .flat_map(|(_, ends)| ends)
                    .any(|end| distance(*end, p) <= plot.ground.policy.meet_m)
            })
            .and_then(|cut| cut.road)
    };
    streets
        .iter()
        .map(|[a, b]| {
            let (a, b) = (*a, *b);
            let toward = scale(sub(b, a), 1.0 / distance(a, b));
            let on_road = |p: Point| {
                plot.roads
                    .iter()
                    .any(|(a, b, _)| touch(&plot.ground.policy, [p, p], [*a, *b]))
            };
            let end = |p: Point, way: f64| match road_end(p).or(shared(p)) {
                Some(end) => end,
                None if on_limit(p) && !on_road(p) => p,
                None => add(p, scale(toward, way * JOIN_OVERSHOOT_M)),
            };
            (
                [round_cm(end(a, -1.0)), round_cm(end(b, 1.0))],
                carried(a, toward).or(carried(b, toward)),
            )
        })
        .collect()
}
