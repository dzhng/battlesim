//! The road network: a main road in from the middle of each edge, every
//! settlement joined to it, and as many extra links as the seed's richness.
//! Where a river runs, each road keeps to its bank and crosses by a bridge.
use super::crossings::{Crossings, Span};
use super::geometry::{
    add, bearing, cross, direction, distance, length, round_cm, scale, segment_crossing, sub,
    Point, PI, TAU,
};
use super::rng::Stream;
use super::sites::Site;
use super::water::Water;
use super::Context;
use crate::Diagnostic;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{Bridge, SurfaceArea, SurfaceKind};

/// The straight line one edge's main road follows: from its exit to the hub
/// (the main junction, by the map's centre), or to a junction on an earlier
/// arm. Fixed before any settlement is sited, so that settlements can be
/// strung along it.
pub struct Arm {
    pub exit: Point,
    pub target: Point,
    /// Straight-line road distance from `target` on to the hub.
    tail: f64,
    /// Where later arms join this one, and the exit each comes from.
    joins: Vec<(Point, Point)>,
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
                arms[*index].joins.push((*point, exit));
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
    water: &'a Water<'a>,
    crossings: Crossings<'a>,
    roads: Vec<(SurfaceKind, Vec<Point>)>,
    /// Every authored point a later road may join.
    vertices: Vec<Vertex>,
}

struct Vertex {
    at: Point,
    /// Its road's kind.
    kind: SurfaceKind,
    /// Which bank of the river it stands on (`Water::bank`).
    bank: bool,
}

/// How one stretch of road swings off its straight line.
struct Bend {
    amplitude: f64,
    waves: f64,
    phase: f64,
}

impl<'a> Network<'a> {
    fn new(context: &'a Context<'a>, water: &'a Water<'a>) -> Self {
        Self {
            context,
            water,
            crossings: Crossings::new(context, water),
            roads: Vec::new(),
            vertices: Vec::new(),
        }
    }

    fn vertex(&self, at: Point, kind: SurfaceKind) -> Vertex {
        Vertex {
            at,
            kind,
            bank: self.water.bank(at),
        }
    }

    /// Add a road, and build the bridges of `planned` it crosses by.
    fn add(&mut self, kind: SurfaceKind, points: Vec<Point>, planned: Vec<Span>) {
        self.crossings.build(planned, &points);
        let vertices: Vec<Vertex> = points.iter().map(|p| self.vertex(*p, kind)).collect();
        self.vertices.extend(vertices);
        self.roads.push((kind, points));
    }

    /// Add a road that ends where it first meets a road at least as good as
    /// itself, in a T-junction both roads share as an authored point. It
    /// still crosses lesser roads.
    fn join(&mut self, kind: SurfaceKind, mut points: Vec<Point>, planned: Vec<Span>) {
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
            let (other_kind, other) = (self.roads[road].0, &self.roads[road].1);
            let point = if u == 0.0 {
                other[piece]
            } else if u == 1.0 {
                other[piece + 1]
            } else {
                let point = round_cm(add(
                    points[run],
                    scale(sub(points[run + 1], points[run]), t),
                ));
                if point != other[piece] && point != other[piece + 1] {
                    self.roads[road].1.insert(piece + 1, point);
                    let shared = self.vertex(point, other_kind);
                    self.vertices.push(shared);
                }
                point
            };
            points.truncate(run + 1);
            if points[run] != point {
                points.push(point);
            }
        }
        if points.len() >= 2 {
            self.add(kind, points, planned);
        }
    }

    /// A road's points as they run with the river there
    /// (`Crossings::carry`).
    fn carried(
        &self,
        kind: SurfaceKind,
        points: Vec<Point>,
        planned: &mut Vec<Span>,
    ) -> Result<Vec<Point>, Vec<Diagnostic>> {
        self.crossings
            .carry(kind, points, planned)
            .map_err(|message| self.context.fail("bridge", message))
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

    /// A swung road of `kind` from `a` to `b`, carried over any river
    /// between them, with the bridges it would build.
    fn bent(
        &self,
        kind: SurfaceKind,
        a: Point,
        b: Point,
        rng: &mut Stream,
    ) -> Result<(Vec<Point>, Vec<Span>), Vec<Diagnostic>> {
        let bend = self.bend(a, b, rng);
        let mut planned = Vec::new();
        let points = self.carried(kind, self.run(a, b, &bend, 1.0), &mut planned)?;
        Ok((points, planned))
    }

    /// Lay the main road of every arm of the skeleton, and say whether every
    /// edge's journey to the hub fits the transit time. With `swing`, a road
    /// runs through the settlements of `sites` near its line that the
    /// journey has time for (marking them `joined`) and is swung between its
    /// stops; without, it runs straight from stop to stop.
    fn arms(
        &mut self,
        skeleton: &[Arm],
        sites: &[Site],
        joined: &mut [bool],
        mut swing: Option<&mut Stream>,
    ) -> Result<bool, Vec<Diagnostic>> {
        let presets = self.context.presets;
        let class_of = |index: usize| presets.class(&sites[index].class_id);
        let centre_of = |index: usize| sites[index].outline.center;
        let budget = presets.transit.budget_m();
        let width = presets.roads.width_m(SurfaceKind::CountryRoad);
        let mut in_time = true;
        // Road distance to the hub from each junction, along the roads as built.
        let mut to_hub: Vec<(Point, f64)> = Vec::new();
        for arm in skeleton {
            let tail = to_hub
                .iter()
                .find(|(point, _)| *point == arm.target)
                .map_or(0.0, |(_, tail)| *tail);
            // Road distance to the hub from every point of a candidate road.
            let measured = |points: &[Point]| {
                let driven = driven(points, width);
                let whole = driven[points.len() - 1];
                points
                    .iter()
                    .zip(driven)
                    .map(|(point, run)| (*point, tail + whole - run))
                    .collect::<Vec<_>>()
            };
            // In time for its own edge, and for every arm that joins it: that
            // arm comes straight from its exit, by this road's bridges where
            // one is near.
            let fits = |points: &[Point], planned: &[Span]| {
                for (point, left) in measured(points) {
                    let come = if point == arm.exit {
                        0.0
                    } else if let Some((join, exit)) =
                        arm.joins.iter().find(|(join, _)| *join == point)
                    {
                        let straight = vec![*exit, *join];
                        let road = self.carried(
                            SurfaceKind::CountryRoad,
                            straight,
                            &mut planned.to_vec(),
                        )?;
                        driven(&road, width)[road.len() - 1]
                    } else {
                        continue;
                    };
                    if come + left > budget {
                        return Ok(false);
                    }
                }
                Ok::<_, Vec<Diagnostic>>(true)
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
                        && aside <= presets.roads.waypoint_reach_m
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
            // The road through `stops` as it runs with the river there: swung
            // leg by leg by `bends`, or straight where there are none.
            let through = |stops: &[Point], bends: &[Bend], strength: f64| {
                let mut planned = Vec::new();
                let mut points = vec![stops[0]];
                for (leg, ends) in stops.windows(2).enumerate() {
                    let run = match bends.get(leg) {
                        Some(bend) => self.run(ends[0], ends[1], bend, strength),
                        None => ends.to_vec(),
                    };
                    let run = self.carried(SurfaceKind::CountryRoad, run, &mut planned)?;
                    points.extend(run.into_iter().skip(1));
                }
                Ok::<_, Vec<Diagnostic>>((points, planned))
            };
            // How many times a road through these stops changes bank.
            let crossings = |stops: &[Point]| {
                let changes = stops
                    .windows(2)
                    .filter(|leg| self.water.bank(leg[0]) != self.water.bank(leg[1]));
                changes.count()
            };
            for (index, _) in near {
                let mut longer = stops.clone();
                longer.push(centre_of(index));
                longer.sort_by(|a, b| share(*a).total_cmp(&share(*b)));
                // A settlement across the water from the road is not on its
                // way: the road would cross to it and back.
                if crossings(&longer) > crossings(&stops) {
                    continue;
                }
                let (points, planned) = through(&longer, &[], 0.0)?;
                if fits(&points, &planned)? {
                    stops = longer;
                    joined[index] = true;
                }
            }
            let bends: Vec<Bend> = match swing.as_deref_mut() {
                Some(rng) => stops
                    .windows(2)
                    .map(|leg| self.bend(leg[0], leg[1], rng))
                    .collect(),
                None => Vec::new(),
            };
            // Straighten until the journey fits; the straight road is the
            // last tried.
            let (mut points, mut planned) = through(&stops, &bends, 1.0)?;
            let mut fit = fits(&points, &planned)?;
            for strength in [0.5, 0.25, 0.0] {
                if fit {
                    break;
                }
                (points, planned) = through(&stops, &bends, strength)?;
                fit = fits(&points, &planned)?;
            }
            in_time &= fit;
            let junctions = measured(&points);
            to_hub.extend(
                junctions
                    .into_iter()
                    .filter(|(point, _)| arm.joins.iter().any(|(join, _)| join == point)),
            );
            self.add(SurfaceKind::CountryRoad, points, planned);
        }
        Ok(in_time)
    }
}

/// How far a road of `width` has run at each of its authored points, along
/// the rounded centreline its surface is made on: the line `measure` drives.
/// A corner's curve through its point is longer than the two straight runs it
/// rounds. (The straight runs themselves for points the shared centreline
/// refuses, which then becomes the straight road.)
fn driven(points: &[Point], width: f64) -> Vec<f64> {
    let straight = || {
        let mut run = vec![0.0];
        for pair in points.windows(2) {
            run.push(run[run.len() - 1] + distance(pair[0], pair[1]));
        }
        run
    };
    let Ok(GroundShape::Stroke { centerline, .. }) = GroundShape::stroke(points.to_vec(), width)
    else {
        return straight();
    };
    // The rounded line passes through every authored point, in order.
    let mut run = Vec::with_capacity(points.len());
    let (mut length, mut next) = (0.0, 0);
    let samples = centerline.samples();
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 {
            length += distance(samples[index - 1], *sample);
        }
        if points.get(next) == Some(sample) {
            run.push(length);
            next += 1;
        }
    }
    if run.len() == points.len() {
        run
    } else {
        straight()
    }
}

/// Whether, with this water on the map, a straight main road in from every
/// edge still reaches the hub within the transit time: what a river's course
/// is held to before anything is placed beside it.
pub fn in_time(context: &Context, skeleton: &[Arm], water: &Water) -> bool {
    Network::new(context, water)
        .arms(skeleton, &[], &mut [], None)
        .unwrap_or(false)
}

/// The map's roads and the bridges they cross its rivers by.
pub fn build(
    context: &Context,
    skeleton: &[Arm],
    sites: &[Site],
    water: &Water,
    richness: f64,
    mut rng: Stream,
) -> Result<(Vec<SurfaceArea>, Vec<Bridge>), Vec<Diagnostic>> {
    let presets = context.presets;
    let roads = &presets.roads;
    let extent = context.extent;
    let mut network = Network::new(context, water);
    let class_of = |index: usize| presets.class(&sites[index].class_id);
    let centre_of = |index: usize| sites[index].outline.center;
    let mut joined = vec![false; sites.len()];
    network.arms(skeleton, sites, &mut joined, Some(&mut rng))?;

    // Every other settlement joins the nearest road good enough for it,
    // largest first, so a town never hangs off a track. `SurfaceKind` orders
    // better roads first.
    let mut order: Vec<usize> = (0..sites.len()).collect();
    order.sort_by_key(|index| core::cmp::Reverse(class_of(*index).rank));
    for index in order {
        let kind = class_of(index).road;
        let serves = |vertex: &Vertex| vertex.kind <= kind;
        let on_road = joined[index]
            || network.vertices.iter().any(|vertex| {
                serves(vertex) && polygon_contains(&sites[index].outline.ring, vertex.at)
            });
        if on_road {
            joined[index] = true;
            continue;
        }
        let from = centre_of(index);
        let bank = water.bank(from);
        let target = network
            .vertices
            .iter()
            .filter(|vertex| serves(vertex) && vertex.at != from)
            .map(|vertex| {
                // A neighbour's centre is a better junction than a point on
                // the open road beside it, and a road on its own bank than
                // one a bridge away.
                let neighbour =
                    (0..sites.len()).any(|other| joined[other] && centre_of(other) == vertex.at);
                let weight = if neighbour { 0.85 } else { 1.0 };
                let crossing = if vertex.bank == bank {
                    0.0
                } else {
                    presets.rivers.bridge.worth_m
                };
                (distance(from, vertex.at) * weight + crossing, vertex.at)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, target)) = target else {
            return Err(context.fail(
                &format!("settlement-{index}"),
                "no road for it to join".into(),
            ));
        };
        let (points, planned) = network.bent(kind, from, target, &mut rng)?;
        // The road carries straight on through the centre to the far side,
        // as the settlement's main street, rather than ending in its middle.
        let onward = bearing(points[1], from);
        let far = sites[index].outline.edge(onward) * roads.main_street_reach;
        let street = vec![from, round_cm(add(from, scale(direction(onward), far)))];
        network.join(kind, points, planned);
        network.add(kind, street, Vec::new());
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
            let (points, planned) = network.bent(kind, pa, pb, &mut rng)?;
            network.join(kind, points, planned);
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
        let on_edge = |along: f64| {
            round_cm(match edge {
                0 => [along, extent],
                1 => [extent, along],
                2 => [along, 0.0],
                _ => [0.0, along],
            })
        };
        // It leaves the map clear of a river's mouth. A mouth is under two
        // gaps and the water wide, so one of these steps along the edge is
        // clear of it.
        let clear = presets.rivers.junction_gap_m;
        let Some(exit) = [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0]
            .into_iter()
            .map(|step| on_edge(along + step * clear))
            .find(|exit| water.gap(*exit, clear) >= clear)
        else {
            continue;
        };
        let target = (1..sites.len())
            .filter(|index| class_of(*index).road == SurfaceKind::CountryRoad)
            .map(centre_of)
            .min_by(|a, b| distance(exit, *a).total_cmp(&distance(exit, *b)));
        if let Some(target) = target {
            let (points, planned) =
                network.bent(SurfaceKind::CountryRoad, exit, target, &mut rng)?;
            network.join(SurfaceKind::CountryRoad, points, planned);
        }
    }

    let surfaces = network
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
        .collect::<Result<_, _>>()?;
    Ok((surfaces, network.crossings.bridges()))
}
