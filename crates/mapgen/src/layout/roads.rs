//! The road network: a main road in from the middle of each edge, every
//! settlement joined to it, and as many extra links as the seed's richness.
use super::geometry::{
    add, bearing, cross, direction, distance, length, round_cm, scale, segment_crossing, sub,
    Point, PI, TAU,
};
use super::rng::Stream;
use super::sites::Site;
use super::Context;
use crate::Diagnostic;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{SurfaceArea, SurfaceKind};

/// The straight line one edge's main road follows: from its exit to the hub
/// (the main junction, by the map's centre), or to a junction on an earlier
/// arm. Fixed before any settlement is sited, so that settlements can be
/// strung along it.
pub struct Arm {
    pub exit: Point,
    pub target: Point,
    /// Straight-line road distance from `target` on to the hub.
    tail: f64,
    /// Where later arms join this one, and how far each has come to get there.
    joins: Vec<(Point, f64)>,
}

/// Each edge's road runs to the hub, or joins an earlier edge road on the way
/// when the journey still fits the transit time: a small map has slack for
/// T-junctions and forks, a large one has little.
pub fn skeleton(context: &Context, rng: &mut Stream) -> Vec<Arm> {
    let presets = context.presets;
    let roads = &presets.roads;
    let extent = context.extent;
    let jitter = |rng: &mut Stream| (2.0 * rng.unit() - 1.0) * roads.hub_jitter_m;
    let hub = round_cm([extent / 2.0 + jitter(rng), extent / 2.0 + jitter(rng)]);
    let budget = presets.transit.budget_m();
    let mut edges = [0, 1, 2, 3];
    for last in (1..4).rev() {
        edges.swap(last, rng.below(last as u64 + 1) as usize);
    }
    let mut arms: Vec<Arm> = Vec::new();
    for edge in edges {
        // North, east, south, west: the axis the edge runs along, and its side.
        let (axis, side) = [(0, extent), (1, extent), (0, 0.0), (1, 0.0)][edge];
        // Leave within the window about the midpoint, and no farther along
        // the edge than a straight road to the hub can afford.
        let across = (side - hub[1 - axis]).abs();
        let reach = libm::sqrt((budget * budget - across * across).max(0.0));
        let half_window = presets.transit.exit_window * extent / 2.0;
        let along = rng.range([
            (extent / 2.0 - half_window).max(hub[axis] - reach),
            (extent / 2.0 + half_window).min(hub[axis] + reach),
        ]);
        let mut exit = [side; 2];
        exit[axis] = along;
        let exit = round_cm(exit);
        // One possible junction on each earlier arm, at least a road step
        // from that arm's own end and from any other junction on it.
        let options: Vec<(usize, Point, f64)> = arms
            .iter()
            .enumerate()
            .filter_map(|(index, arm)| {
                let span = distance(arm.exit, arm.target);
                let limit = (roads.junction_reach_m - arm.tail).min(0.6 * span);
                let back = rng.range([roads.bend_step_m, limit]);
                let point = round_cm(add(
                    arm.target,
                    scale(sub(arm.exit, arm.target), back / span),
                ));
                let tail = arm.tail + distance(point, arm.target);
                // On this exit's side of the hub: a road that passed the hub
                // to join beyond it would double back.
                let (out, across) = (sub(exit, hub), sub(point, hub));
                let same_side = out[0] * across[0] + out[1] * across[1] >= 0.0;
                (limit >= roads.bend_step_m
                    && arm
                        .joins
                        .iter()
                        .all(|(join, _)| distance(*join, point) >= roads.bend_step_m)
                    && same_side
                    && distance(exit, point) + tail <= budget)
                    .then_some((index, point, tail))
            })
            .collect();
        let (join, pick) = (rng.chance(roads.junction_chance), rng.unit());
        let (target, tail) = match options.get((pick * options.len() as f64) as usize) {
            Some((index, point, tail)) if join => {
                arms[*index].joins.push((*point, distance(exit, *point)));
                (*point, *tail)
            }
            _ => (hub, 0.0),
        };
        arms.push(Arm {
            exit,
            target,
            tail,
            joins: Vec::new(),
        });
    }
    arms
}

struct Network<'a> {
    context: &'a Context<'a>,
    roads: Vec<(SurfaceKind, Vec<Point>)>,
    /// Every authored point a later road may join, with its road's kind.
    vertices: Vec<(Point, SurfaceKind)>,
}

/// How one stretch of road swings off its straight line.
struct Bend {
    amplitude: f64,
    waves: f64,
    phase: f64,
}

impl Network<'_> {
    fn add(&mut self, kind: SurfaceKind, points: Vec<Point>) {
        self.vertices.extend(points.iter().map(|p| (*p, kind)));
        self.roads.push((kind, points));
    }

    /// Add a road that ends where it first meets a road at least as good as
    /// itself, in a T-junction both roads share as an authored point. It
    /// still crosses lesser roads.
    fn join(&mut self, kind: SurfaceKind, mut points: Vec<Point>) {
        // (run of the new road, share along it, road met, its run, share along that)
        let mut meeting: Option<(usize, f64, usize, usize, f64)> = None;
        for (run, ends) in points.windows(2).enumerate() {
            for (road, (other_kind, other)) in self.roads.iter().enumerate() {
                if *other_kind > kind {
                    continue;
                }
                for (piece, other_ends) in other.windows(2).enumerate() {
                    let crossing = segment_crossing(ends[0], ends[1], other_ends[0], other_ends[1]);
                    let Some((t, u)) = crossing else { continue };
                    // Leaving a road it starts on is not meeting one.
                    let leaving = run == 0 && t == 0.0;
                    if !leaving && meeting.is_none_or(|(_, first, ..)| t < first) {
                        meeting = Some((run, t, road, piece, u));
                    }
                }
            }
            if meeting.is_some() {
                break;
            }
        }
        if let Some((run, t, road, piece, u)) = meeting {
            let other = &mut self.roads[road];
            let point = if u == 0.0 {
                other.1[piece]
            } else if u == 1.0 {
                other.1[piece + 1]
            } else {
                let point = round_cm(add(
                    points[run],
                    scale(sub(points[run + 1], points[run]), t),
                ));
                if point != other.1[piece] && point != other.1[piece + 1] {
                    other.1.insert(piece + 1, point);
                    self.vertices.push((point, other.0));
                }
                point
            };
            points.truncate(run + 1);
            if points[run] != point {
                points.push(point);
            }
        }
        if points.len() >= 2 {
            self.add(kind, points);
        }
    }

    fn bend(&self, a: Point, b: Point, rng: &mut Stream) -> Bend {
        let roads = &self.context.presets.roads;
        let reach = (roads.bend_amplitude * distance(a, b)).min(roads.bend_max_m);
        Bend {
            amplitude: reach * rng.range([0.3, 1.0]),
            waves: rng.range([0.5, 1.4]),
            phase: rng.range([0.0, TAU]),
        }
    }

    /// Authored points from `a` to `b`, a step apart, swung by `bend` scaled
    /// by `strength`. Both ends are exact and the road leaves each along the
    /// straight line, so it runs straight through whatever stands there.
    /// Every point stays in the map.
    fn run(&self, a: Point, b: Point, bend: &Bend, strength: f64) -> Vec<Point> {
        let span = distance(a, b);
        let steps = if strength == 0.0 {
            1
        } else {
            (libm::round(span / self.context.presets.roads.bend_step_m) as usize).max(1)
        };
        let along = sub(b, a);
        let normal = scale([-along[1], along[0]], 1.0 / span);
        let mut points = vec![a];
        for step in 1..steps {
            let t = step as f64 / steps as f64;
            let ease = libm::sin(PI * t);
            let swing = bend.amplitude
                * strength
                * ease
                * ease
                * libm::sin(TAU * bend.waves * t + bend.phase);
            let point = round_cm(add(add(a, scale(along, t)), scale(normal, swing)))
                .map(|v| v.clamp(0.0, self.context.extent));
            if points.last() != Some(&point) && point != b {
                points.push(point);
            }
        }
        points.push(b);
        points
    }

    fn bent(&self, a: Point, b: Point, rng: &mut Stream) -> Vec<Point> {
        let bend = self.bend(a, b, rng);
        self.run(a, b, &bend, 1.0)
    }
}

pub fn build(
    context: &Context,
    skeleton: &[Arm],
    sites: &[Site],
    richness: f64,
    mut rng: Stream,
) -> Result<Vec<SurfaceArea>, Vec<Diagnostic>> {
    let presets = context.presets;
    let roads = &presets.roads;
    let extent = context.extent;
    let mut network = Network {
        context,
        roads: Vec::new(),
        vertices: Vec::new(),
    };
    let class_of = |index: usize| presets.class(&sites[index].class_id);
    let centre_of = |index: usize| sites[index].outline.center;
    let budget = presets.transit.budget_m();
    let mut joined = vec![false; sites.len()];
    // Road distance to the hub from each junction, along the roads as built.
    let mut to_hub: Vec<(Point, f64)> = Vec::new();

    for arm in skeleton {
        let tail = to_hub
            .iter()
            .find(|(point, _)| *point == arm.target)
            .map_or(0.0, |(_, tail)| *tail);
        // Road distance to the hub from every point of a candidate road.
        let measured = |points: &[Point]| {
            let mut left = tail;
            let mut each = vec![(points[points.len() - 1], left)];
            for run in points.windows(2).rev() {
                left += distance(run[0], run[1]);
                each.push((run[0], left));
            }
            each
        };
        // In time for its own edge, and for every arm that joins it.
        let fits = |points: &[Point]| {
            measured(points).iter().all(|(point, left)| {
                let come = if *point == arm.exit {
                    Some(0.0)
                } else {
                    arm.joins
                        .iter()
                        .find(|(join, _)| join == point)
                        .map(|(_, come)| *come)
                };
                come.is_none_or(|come| come + left <= budget)
            })
        };
        let line = sub(arm.target, arm.exit);
        let span = length(line);
        let share = |point: Point| {
            let offset = sub(point, arm.exit);
            (offset[0] * line[0] + offset[1] * line[1]) / (span * span)
        };
        // The road stops at its junctions and runs through the centre of
        // every settlement on or near its line that the journey has time
        // for, largest first.
        let mut stops: Vec<Point> = [arm.exit, arm.target]
            .into_iter()
            .chain(arm.joins.iter().map(|(join, _)| *join))
            .collect();
        stops.sort_by(|a, b| share(*a).total_cmp(&share(*b)));
        let mut near: Vec<(usize, f64)> = (0..sites.len())
            .filter_map(|index| {
                let aside = (cross(line, sub(centre_of(index), arm.exit)) / span).abs();
                ((0.05..=0.95).contains(&share(centre_of(index)))
                    && aside <= roads.waypoint_reach_m
                    && !stops.contains(&centre_of(index)))
                .then_some((index, aside))
            })
            .collect();
        near.sort_by(|a, b| {
            class_of(b.0)
                .rank
                .cmp(&class_of(a.0).rank)
                .then(a.1.total_cmp(&b.1))
        });
        for (index, _) in near {
            let mut longer = stops.clone();
            longer.push(centre_of(index));
            longer.sort_by(|a, b| share(*a).total_cmp(&share(*b)));
            if fits(&longer) {
                stops = longer;
                joined[index] = true;
            }
        }
        let bends: Vec<Bend> = stops
            .windows(2)
            .map(|leg| network.bend(leg[0], leg[1], &mut rng))
            .collect();
        // Straighten until the journey fits; the straight one always does.
        let points = [1.0, 0.5, 0.25, 0.0]
            .into_iter()
            .map(|strength| {
                let mut points = vec![arm.exit];
                for (leg, bend) in stops.windows(2).zip(&bends) {
                    points.extend(
                        network
                            .run(leg[0], leg[1], bend, strength)
                            .into_iter()
                            .skip(1),
                    );
                }
                points
            })
            .find(|points| fits(points))
            .unwrap_or(stops);
        let junctions = measured(&points);
        to_hub.extend(
            junctions
                .into_iter()
                .filter(|(point, _)| arm.joins.iter().any(|(join, _)| join == point)),
        );
        network.add(SurfaceKind::CountryRoad, points);
    }

    // Every other settlement joins the nearest road good enough for it,
    // largest first, so a town never hangs off a track. `SurfaceKind` orders
    // better roads first.
    let mut order: Vec<usize> = (0..sites.len()).collect();
    order.sort_by_key(|index| core::cmp::Reverse(class_of(*index).rank));
    for index in order {
        let kind = class_of(index).road;
        let serves = |vertex: &(Point, SurfaceKind)| vertex.1 <= kind;
        let on_road = joined[index]
            || network.vertices.iter().any(|vertex| {
                serves(vertex) && polygon_contains(&sites[index].outline.ring, vertex.0)
            });
        if on_road {
            joined[index] = true;
            continue;
        }
        let from = centre_of(index);
        let target = network
            .vertices
            .iter()
            .filter(|vertex| serves(vertex) && vertex.0 != from)
            .map(|vertex| {
                // A neighbour's centre is a better junction than a point on
                // the open road beside it.
                let neighbour =
                    (0..sites.len()).any(|other| joined[other] && centre_of(other) == vertex.0);
                let weight = if neighbour { 0.85 } else { 1.0 };
                (distance(from, vertex.0) * weight, vertex.0)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, target)) = target else {
            return Err(context.fail(
                &format!("settlement-{index}"),
                "no road for it to join".into(),
            ));
        };
        let points = network.bent(from, target, &mut rng);
        // The road carries straight on through the centre to the far side,
        // as the settlement's main street, rather than ending in its middle.
        let onward = bearing(points[1], from);
        let far = sites[index].outline.edge(onward) * roads.main_street_reach;
        let street = vec![from, round_cm(add(from, scale(direction(onward), far)))];
        network.join(kind, points);
        network.add(kind, street);
        joined[index] = true;
    }

    // A richer network links neighbours directly (Gabriel pairs: no third
    // settlement inside the circle on the pair's diameter), unless one road
    // already runs through both. The main settlement has the edge roads.
    let within = |index: usize, points: &[Point]| {
        points
            .iter()
            .any(|p| polygon_contains(&sites[index].outline.ring, *p))
    };
    for a in 1..sites.len() {
        for b in a + 1..sites.len() {
            let (pa, pb) = (centre_of(a), centre_of(b));
            let span = distance(pa, pb);
            let middle = scale(add(pa, pb), 0.5);
            let neighbours = span <= roads.link_max_m
                && (0..sites.len())
                    .all(|c| c == a || c == b || distance(centre_of(c), middle) >= span / 2.0);
            if !neighbours || !rng.chance(richness) {
                continue;
            }
            let linked = network
                .roads
                .iter()
                .any(|(_, points)| within(a, points) && within(b, points));
            if linked {
                continue;
            }
            // The rougher of the two settlements' roads.
            let kind = class_of(a).road.max(class_of(b).road);
            let points = network.bent(pa, pb, &mut rng);
            network.join(kind, points);
        }
    }

    // And brings more roads in from the edges, clear of the main exits, each
    // to the nearest settlement a country road serves.
    let window = presets.transit.exit_window;
    let corridors =
        ((richness * f64::from(roads.corridors_max + 1)) as u32).min(roads.corridors_max);
    for _ in 0..corridors {
        let edge = rng.below(4);
        let mut along = rng.range([0.08, 0.5 - window / 2.0 - 0.04]);
        if rng.chance(0.5) {
            along = 1.0 - along;
        }
        let along = along * extent;
        let exit = round_cm(match edge {
            0 => [along, extent],
            1 => [extent, along],
            2 => [along, 0.0],
            _ => [0.0, along],
        });
        let target = (1..sites.len())
            .filter(|index| class_of(*index).road == SurfaceKind::CountryRoad)
            .map(centre_of)
            .min_by(|a, b| distance(exit, *a).total_cmp(&distance(exit, *b)));
        if let Some(target) = target {
            let points = network.bent(exit, target, &mut rng);
            network.join(SurfaceKind::CountryRoad, points);
        }
    }

    network
        .roads
        .into_iter()
        .enumerate()
        .map(|(index, (kind, points))| {
            let width = roads.width_m(kind);
            let ends = vec![points[0], points[points.len() - 1]];
            // A bend the shared centreline refuses falls back to the straight road.
            GroundShape::stroke(points, width)
                .or_else(|_| GroundShape::stroke(ends, width))
                .map(|shape| SurfaceArea { kind, shape })
                .map_err(|message| context.fail(&format!("road-{index}"), message))
        })
        .collect()
}
