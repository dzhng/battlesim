//! The road network: a main road from the bottom edge to the top through
//! the main junction, on some maps another across the middle from side to
//! side, every settlement joined to them, and as many extra links as the
//! seed's richness. A road crosses a settlement's ground in a straight line
//! and turns outside it. Where a river runs, each road keeps to its bank and
//! crosses by a bridge.
use super::crossings::{Crossings, Span};
use super::geometry::{
    add, bearing, cross, direction, distance, dot, length, ray_exit, ring_distance, round_cm,
    scale, segment_crossing, segment_distance, sub, turn, Point, PI, TAU,
};
use super::rng::Stream;
use super::sites::Site;
use super::water::Water;
use super::Context;
use crate::Diagnostic;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{Bridge, SurfaceArea, SurfaceKind};

/// A point this near a settlement's limit stands on its ground.
const ON_GROUND_M: f64 = 0.5;
/// Two points nearer than this are one place: no road runs between them.
const SAME_PLACE_M: f64 = 1.0;
/// A stretch of road whose points lie this near one line is straight.
const STRAIGHT_M: f64 = 0.25;

/// The straight line one edge's main road follows: from its exit to the hub
/// (the main junction, by the map's centre), or to a junction on an earlier
/// arm. Fixed before any settlement is sited, so that settlements can be
/// strung along it.
pub struct Arm {
    /// North, east, south, west: 0 to 3.
    edge: usize,
    pub exit: Point,
    pub target: Point,
    /// Straight-line road distance from `target` on to the hub.
    tail: f64,
    /// Where later arms join this one.
    joins: Vec<Join>,
    /// Whether its edge's journey to the hub is held to the transit time:
    /// the top and the bottom edge, where the two sides start (M22).
    timed: bool,
}

struct Join {
    at: Point,
    /// The exit of the arm that joins here, and whether that arm is timed.
    exit: Point,
    timed: bool,
}

/// The main roads' lines, the bottom and top edges' first.
pub struct Skeleton {
    /// The main junction. The main settlement stands on it.
    pub hub: Point,
    pub arms: Vec<Arm>,
}

/// A road from the bottom edge and one from the top always run to the hub or
/// join another arm near it, inside the transit time. A road across the
/// middle from side to side is drawn on a share of maps; the others have one
/// side road or none. Two arms that meet at the hub from opposite edges are
/// one straight line through it.
pub fn skeleton(context: &Context, rng: &mut Stream) -> Skeleton {
    let presets = context.presets;
    let roads = &presets.roads;
    let extent = context.extent;
    let offset = roads
        .hub_offset_m
        .map(|reach| (2.0 * rng.unit() - 1.0) * reach);
    let hub = round_cm([extent / 2.0 + offset[0], extent / 2.0 + offset[1]]);
    let budget = presets.transit.budget_m();
    let either = |rng: &mut Stream, pair: [usize; 2]| {
        if rng.chance(0.5) {
            pair
        } else {
            [pair[1], pair[0]]
        }
    };
    let spine = either(rng, [0, 2]);
    let sides = either(rng, [1, 3]);
    let through = rng.chance(roads.cross_road_chance);
    let lone = rng.chance(roads.side_road_chance);
    // (edge, timed, the share of the edge it may leave by). The road across
    // the middle comes first, so the spine may fork onto it.
    let window = presets.transit.exit_window;
    let order: Vec<(usize, bool, f64)> = if through {
        vec![
            (sides[0], false, window),
            (sides[1], false, window),
            (spine[0], true, window),
            (spine[1], true, window),
        ]
    } else {
        let mut order = vec![(spine[0], true, window), (spine[1], true, window)];
        if lone {
            order.push((sides[0], false, roads.side_exit_window));
        }
        order
    };
    // The axis an edge runs along, and its side: north, east, south, west.
    let place = |edge: usize| [(0, extent), (1, extent), (0, 0.0), (1, 0.0)][edge];
    // Where along its edge an arm may leave: within the window about the
    // midpoint and, when timed, no farther along the edge than a straight
    // road to the hub can afford.
    let span_of = |edge: usize, timed: bool, window: f64| {
        let (axis, side) = place(edge);
        let across = (side - hub[1 - axis]).abs();
        let reach = if timed {
            libm::sqrt((budget * budget - across * across).max(0.0))
        } else {
            f64::INFINITY
        };
        let half_window = window * extent / 2.0;
        [
            (extent / 2.0 - half_window).max(hub[axis] - reach),
            (extent / 2.0 + half_window).min(hub[axis] + reach),
        ]
    };
    // Where the straight line from `along` on one edge through the hub
    // meets the opposite edge.
    let through_hub = |edge: usize, along: f64| {
        let (axis, side) = place(edge);
        let (_, far) = place((edge + 2) % 4);
        hub[axis] + (hub[axis] - along) * (far - hub[1 - axis]) / (hub[1 - axis] - side)
    };
    let mut arms: Vec<Arm> = Vec::new();
    for (place_in_order, (edge, timed, window)) in order.iter().copied().enumerate() {
        let (axis, side) = place(edge);
        let mut span = span_of(edge, timed, window);
        // An arm whose opposite arm comes later leaves where that arm can
        // carry its line straight on through the hub.
        if let Some((other, timed, window)) = order[place_in_order + 1..]
            .iter()
            .copied()
            .find(|(other, ..)| *other == (edge + 2) % 4)
        {
            let ends = span_of(other, timed, window).map(|along| through_hub(other, along));
            let carried = [
                span[0].max(ends[0].min(ends[1])),
                span[1].min(ends[0].max(ends[1])),
            ];
            if carried[0] <= carried[1] {
                span = carried;
            }
        }
        let on_edge = |along: f64| {
            let mut exit = [side; 2];
            exit[axis] = along;
            round_cm(exit)
        };
        let mut exit = on_edge(rng.range(span));
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
                        .all(|join| distance(join.at, point) >= roads.bend_step_m)
                    && same_side
                    && (!timed || distance(exit, point) + tail <= budget))
                    .then_some((index, point, tail))
            })
            .collect();
        let (join, pick) = (rng.chance(roads.junction_chance), rng.unit());
        let (target, tail) = match options.get((pick * options.len() as f64) as usize) {
            Some((index, point, tail)) if join => {
                arms[*index].joins.push(Join {
                    at: *point,
                    exit,
                    timed,
                });
                (*point, *tail)
            }
            _ => {
                // The arm from the opposite edge, when it too ends at the
                // hub, is carried straight on through it.
                let opposite = arms
                    .iter()
                    .find(|arm| arm.edge == (edge + 2) % 4 && arm.target == hub);
                if let Some(opposite) = opposite {
                    let along = through_hub(opposite.edge, opposite.exit[axis]);
                    exit = on_edge(along.clamp(span[0], span[1]));
                }
                (hub, 0.0)
            }
        };
        arms.push(Arm {
            edge,
            exit,
            target,
            tail,
            joins: Vec::new(),
            timed,
        });
    }
    Skeleton { hub, arms }
}

struct Network<'a> {
    context: &'a Context<'a>,
    water: &'a Water<'a>,
    /// The settlements whose ground roads cross in a straight line.
    sites: &'a [Site],
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

/// A place a main road runs through: a point, or the centre of the
/// settlement `through` whose ground it crosses.
#[derive(Clone, Copy, PartialEq)]
struct Stop {
    at: Point,
    through: Option<usize>,
}

impl<'a> Network<'a> {
    fn new(context: &'a Context<'a>, water: &'a Water<'a>, sites: &'a [Site]) -> Self {
        Self {
            context,
            water,
            sites,
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

    /// The road `points` with every stretch across a settlement's ground
    /// made one straight line, from where it enters to where it leaves (or
    /// to its own end there). A stretch already straight is left alone, and
    /// so is one that turns at a point of `pins`, which other roads meet.
    fn straightened(&self, mut points: Vec<Point>, pins: &[Point]) -> Vec<Point> {
        for site in self.sites {
            let ring = &site.outline.ring;
            let count = points.len();
            let inside: Vec<bool> = points.iter().map(|p| polygon_contains(ring, *p)).collect();
            // Where along the run from `a` to `b` it crosses the ring.
            let crossings = |a: Point, b: Point| {
                contract::ground::edges(ring)
                    .filter_map(move |(c, d)| segment_crossing(a, b, *c, *d))
                    .map(|(share, _)| share)
            };
            let touches = |run: &usize| {
                inside[*run]
                    || inside[run + 1]
                    || crossings(points[*run], points[run + 1]).next().is_some()
            };
            let (Some(first), Some(last)) = (
                (0..count - 1).find(touches),
                (0..count - 1).rfind(|run| touches(run)),
            ) else {
                continue;
            };
            let at = |run: usize, share: f64| {
                add(points[run], scale(sub(points[run + 1], points[run]), share))
            };
            let entry = if inside[first] {
                points[first]
            } else {
                let share = crossings(points[first], points[first + 1]).fold(1.0, f64::min);
                round_cm(at(first, share))
            };
            let exit = if inside[last + 1] {
                points[last + 1]
            } else {
                let share = crossings(points[last], points[last + 1]).fold(0.0, f64::max);
                round_cm(at(last, share))
            };
            let between = &points[first + 1..=last];
            if entry == exit
                || between
                    .iter()
                    .all(|p| segment_distance(entry, exit, *p) <= STRAIGHT_M)
                || between.iter().any(|p| pins.contains(p))
            {
                continue;
            }
            let mut straight = points[..=first].to_vec();
            straight.extend([entry, exit]);
            straight.extend(&points[last + 1..]);
            straight.dedup();
            points = straight;
        }
        points
    }

    /// Add a road, and build the bridges of `planned` it crosses by.
    fn add(&mut self, kind: SurfaceKind, points: Vec<Point>, planned: Vec<Span>, pins: &[Point]) {
        let points = self.straightened(points, pins);
        self.lay(kind, points, planned);
    }

    /// Record the finished road `points` with the bridges of `planned` it
    /// crosses by.
    fn lay(&mut self, kind: SurfaceKind, points: Vec<Point>, planned: Vec<Span>) {
        self.crossings.build(planned, &points);
        let vertices: Vec<Vertex> = points.iter().map(|p| self.vertex(*p, kind)).collect();
        self.vertices.extend(vertices);
        self.roads.push((kind, points));
    }

    /// Add a road that ends where it first meets a road at least as good as
    /// itself, in a T-junction both roads share as an authored point. It
    /// still crosses lesser roads.
    fn join(&mut self, kind: SurfaceKind, points: Vec<Point>, planned: Vec<Span>, pins: &[Point]) {
        let mut points = self.straightened(points, pins);
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
        // A road cut back to the road it set out from goes nowhere, and one
        // that ends on the road it left only doubles a stretch of it.
        let ends = [points[0], points[points.len() - 1]];
        let doubles = self
            .roads
            .iter()
            .any(|(_, other)| ends.iter().all(|end| other.contains(end)));
        if points.len() >= 2 && !doubles {
            self.lay(kind, points, planned);
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

    /// The settlement whose ground `p` stands on, by its place in `sites`.
    fn ground(&self, p: Point) -> Option<usize> {
        self.sites
            .iter()
            .position(|site| ring_distance(&site.outline.ring, p) <= ON_GROUND_M)
    }

    /// Where a road that leaves `from`, on `site`'s ground, along the unit
    /// vector `toward` turns: a margin past the ground's edge, or nearer it
    /// where a river leaves a road less room than that.
    fn gate(&self, site: &Site, from: Point, toward: Point) -> Point {
        let presets = self.context.presets;
        let clear = presets.rivers.road_gap_m;
        let edge = ray_exit(&site.outline.ring, from, toward);
        let at = |margin: f64| {
            round_cm(add(from, scale(toward, edge + margin)))
                .map(|v| v.clamp(0.0, self.context.extent))
        };
        [1.0, 0.5]
            .into_iter()
            .map(|share| at(share * presets.roads.gate_margin_m))
            .find(|gate| self.water.gap(*gate, clear) >= clear)
            .unwrap_or(at(0.0))
    }

    /// A road of `kind` from `a` to `b`, carried over any river between
    /// them, with the bridges it would build. It runs straight across the
    /// ground of a settlement either end stands on and swings between.
    fn route(
        &self,
        kind: SurfaceKind,
        a: Point,
        b: Point,
        rng: &mut Stream,
    ) -> Result<(Vec<Point>, Vec<Span>), Vec<Diagnostic>> {
        if a == b {
            return Ok((vec![a], Vec::new()));
        }
        let toward = scale(sub(b, a), 1.0 / distance(a, b));
        let (home, away) = (self.ground(a), self.ground(b));
        let leave = home.map_or(a, |site| self.gate(&self.sites[site], a, toward));
        let enter = away.map_or(b, |site| {
            self.gate(&self.sites[site], b, scale(toward, -1.0))
        });
        // The gates stand in order along the way, with a road between them.
        let apart = dot(sub(enter, leave), toward);
        let mut points = if (home.is_some() && home == away) || apart <= SAME_PLACE_M {
            vec![a, b]
        } else {
            let bend = self.bend(leave, enter, rng);
            let mut points = vec![a];
            points.extend(self.run(leave, enter, &bend, 1.0));
            points.push(b);
            points
        };
        points.dedup();
        let mut planned = Vec::new();
        let points = self.carried(kind, points, &mut planned)?;
        Ok((points, planned))
    }

    /// Whether a road of `kind` from `from` may end on the road point `p`,
    /// and then how awkwardly (0 or 1). It may not where roads already meet
    /// (no more than a crossroads forms anywhere), where a road leaves the
    /// map, or on the end of a road of another kind: a road changes kind
    /// and width where it meets a road that passes, never end to end in the
    /// open. It joins easily where a road passes and it does not come in
    /// alongside, or on a road's end that it carries on from without a
    /// sharp turn.
    fn joins(&self, kind: SurfaceKind, from: Point, p: Point) -> Option<u8> {
        let sharpest = self.context.presets.roads.turn_max_deg.to_radians();
        let arriving = bearing(from, p);
        let mut legs = 0;
        let mut easy = true;
        for (other, points) in &self.roads {
            let last = points.len() - 1;
            for (index, _) in points.iter().enumerate().filter(|(_, q)| **q == p) {
                let end = index == 0 || index == last;
                if end && *other != kind {
                    return None;
                }
                legs += if end { 1 } else { 2 };
                let before = index.checked_sub(1).map(|at| points[at]);
                for onward in before.into_iter().chain(points.get(index + 1).copied()) {
                    let turn = turn(arriving, bearing(p, onward));
                    easy &= turn <= sharpest || (!end && turn <= PI - sharpest);
                }
            }
        }
        let extent = self.context.extent;
        let inside = p.iter().all(|v| *v > 0.0 && *v < extent);
        (legs <= 2 && inside).then_some(u8::from(!easy))
    }

    /// The point a road of `kind` to settlement `index` from `from` ends
    /// on: the nearest point of the roads already on its ground that serve
    /// it and that it may join, one it joins easily before any other, or
    /// its centre when none is. None where it may not join the centre
    /// either: the road is not laid.
    fn approach(&self, kind: SurfaceKind, index: usize, from: Point) -> Option<Point> {
        let site = &self.sites[index];
        let serving = self.context.presets.class(&site.class_id).road;
        let margin = self.context.presets.roads.gate_margin_m + SAME_PLACE_M;
        self.vertices
            .iter()
            .filter(|vertex| {
                vertex.kind <= serving
                    && distance(vertex.at, site.outline.center) <= site.outline.reach + margin
                    && ring_distance(&site.outline.ring, vertex.at) <= margin
            })
            .filter_map(|vertex| {
                let awkward = self.joins(kind, from, vertex.at)?;
                Some((awkward, distance(from, vertex.at), vertex.at))
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)))
            .map(|(_, _, at)| at)
            .or_else(|| {
                let centre = site.outline.center;
                self.joins(kind, from, centre).map(|_| centre)
            })
    }

    /// The authored line through `stops`: a settlement's centre between the
    /// two gates the road crosses its ground by, in a straight line. Beside
    /// each point, whether the road runs straight on to the next.
    fn gated(&self, stops: &[Stop], straight_from: Option<Point>) -> Vec<(Point, bool)> {
        let mut line: Vec<(Point, bool)> = Vec::new();
        let mut straight = false;
        for (index, stop) in stops.iter().enumerate() {
            straight |= Some(stop.at) == straight_from;
            let site = stop.through.map(|site| &self.sites[site]);
            match (site, index.checked_sub(1), stops.get(index + 1)) {
                (Some(site), Some(before), Some(after)) => {
                    let (before, after) = (stops[before].at, after.at);
                    let toward = scale(sub(after, before), 1.0 / distance(before, after));
                    let gates = [-1.0, 1.0].map(|way| self.gate(site, stop.at, scale(toward, way)));
                    // A gate past the neighbouring stop is left out.
                    if distance(stop.at, gates[0]) < distance(stop.at, before) {
                        line.push((gates[0], true));
                    }
                    let leaves = distance(stop.at, gates[1]) < distance(stop.at, after);
                    line.push((stop.at, leaves || straight));
                    if leaves {
                        line.push((gates[1], straight));
                    }
                }
                _ => line.push((stop.at, straight)),
            }
        }
        line
    }

    /// Lay settlement `index`'s secondary roads, where its class has them
    /// (`classes.<class>.side_roads`). Each leaves one of the roads that
    /// pass its centre part of the way out, turns into the widest sector
    /// of its ground no road runs out through yet, and runs straight to
    /// the ground's edge and a gate past it, or to the first road it meets.
    /// A later road may carry on from the gate.
    fn side_roads(&mut self, index: usize) {
        let presets = self.context.presets;
        let site = &self.sites[index];
        let class = presets.class(&site.class_id);
        let Some(rule) = class.side_roads else {
            return;
        };
        let kind = class.road;
        // Its own stream: a settlement with none draws nothing, and the
        // rest of the network is laid as it was.
        let mut rng = self.context.stream(&format!("side-roads/{index}"));
        let centre = site.outline.center;
        let ring = &site.outline.ring;
        // The roads out of its centre: where each passes nearest it, and
        // the way it runs on from there. (road, from, unit direction)
        let near = site.outline.reach * presets.roads.through_reach;
        let mut legs: Vec<(usize, Point, Point)> = Vec::new();
        for (road, (other, points)) in self.roads.iter().enumerate() {
            if *other > kind {
                continue;
            }
            for run in points.windows(2) {
                let step = sub(run[1], run[0]);
                let share = (dot(sub(centre, run[0]), step) / dot(step, step)).clamp(0.0, 1.0);
                let from = add(run[0], scale(step, share));
                if distance(from, centre) > near {
                    continue;
                }
                for end in [run[0], run[1]] {
                    if distance(end, from) < SAME_PLACE_M {
                        continue;
                    }
                    let along = scale(sub(end, from), 1.0 / distance(end, from));
                    // One leg a way out: a road's two runs through a point
                    // are one leg each way.
                    if !legs.iter().any(|(_, _, known)| dot(*known, along) > 0.94) {
                        legs.push((road, from, along));
                    }
                }
            }
        }
        if legs.is_empty() {
            return;
        }
        // The bearings a road already runs out along, seen from the centre.
        let mut taken: Vec<f64> = legs
            .iter()
            .map(|(_, _, along)| libm::atan2(along[1], along[0]))
            .collect();
        let mut left: Vec<Vec<f64>> = vec![Vec::new(); legs.len()];
        let clear = presets.rivers.road_gap_m;
        for _ in 0..rng.count(rule.count) {
            // The widest sector between two of them, and its middle.
            taken.sort_by(f64::total_cmp);
            let (width, middle) = (0..taken.len())
                .map(|at| {
                    let (from, to) = (taken[at], taken[(at + 1) % taken.len()]);
                    let width = (to - from).rem_euclid(TAU);
                    let width = if taken.len() == 1 { TAU } else { width };
                    (width, from + width / 2.0)
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .unwrap_or((TAU, 0.0));
            if width < presets.roads.turn_max_deg.to_radians() {
                break;
            }
            let into = direction(middle);
            // It leaves the leg nearest that sector, the one with fewer
            // side roads first, turning toward the sector.
            let Some(leg) = (0..legs.len()).max_by(|a, b| {
                let toward = |leg: &usize| dot(legs[*leg].2, into);
                (toward(a) - 0.2 * left[*a].len() as f64)
                    .total_cmp(&(toward(b) - 0.2 * left[*b].len() as f64))
                    .then(b.cmp(a))
            }) else {
                break;
            };
            let (road, from, along) = legs[leg];
            let side = if cross(along, into) >= 0.0 { 1.0 } else { -1.0 };
            let edge = ray_exit(ring, from, along);
            // A few draws for a place clear of the others on this leg.
            let drawn = (0..4)
                .map(|_| {
                    (
                        rng.range(rule.from) * edge,
                        rng.range(rule.turn_deg).to_radians(),
                    )
                })
                .collect::<Vec<_>>();
            let Some((out, turn)) = drawn.into_iter().find(|(out, _)| {
                left[leg]
                    .iter()
                    .all(|other| (other - out).abs() >= rule.apart_m)
            }) else {
                continue;
            };
            let start = round_cm(add(from, scale(along, out)));
            let heading = libm::atan2(along[1], along[0]) + side * turn;
            let gate = self.gate(site, start, direction(heading));
            // On the road it leaves, on dry ground all the way.
            let points = &self.roads[road].1;
            let Some(run) = points.windows(2).position(|run| {
                segment_distance(run[0], run[1], start) <= STRAIGHT_M
                    && distance(run[0], start) >= SAME_PLACE_M
                    && distance(run[1], start) >= SAME_PLACE_M
            }) else {
                continue;
            };
            if self.water.segment_gap(start, gate, clear) < clear
                || distance(start, gate) < out.min(edge - out) / 2.0
            {
                continue;
            }
            self.roads[road].1.insert(run + 1, start);
            let shared = self.vertex(start, self.roads[road].0);
            self.vertices.push(shared);
            let before = self.roads.len();
            self.join(kind, vec![start, gate], Vec::new(), &[start]);
            if let Some((_, laid)) = self.roads.get(before) {
                let end = laid[laid.len() - 1];
                taken.push(bearing(centre, end));
                left[leg].push(out);
            }
        }
    }

    /// Lay the main road of every arm of the skeleton, and say whether the
    /// top and bottom edges' journeys to the hub fit the transit time. With
    /// `swing`, a road runs through the settlements near its line that the
    /// journey has time for (marking them `joined`) and is swung between its
    /// stops; without, it runs straight from stop to stop.
    fn arms(
        &mut self,
        skeleton: &Skeleton,
        joined: &mut [bool],
        mut swing: Option<&mut Stream>,
    ) -> Result<bool, Vec<Diagnostic>> {
        let presets = self.context.presets;
        let sites = self.sites;
        let class_of = |index: usize| presets.class(&sites[index].class_id);
        let centre_of = |index: usize| sites[index].outline.center;
        let budget = presets.transit.budget_m();
        let width = presets.roads.width_m(SurfaceKind::CountryRoad);
        let mut in_time = true;
        // Road distance to the hub from each junction, along the roads as built.
        let mut to_hub: Vec<(Point, f64)> = Vec::new();
        for arm in &skeleton.arms {
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
            // one is near. Only the top and bottom edges are timed.
            let fits = |points: &[Point], planned: &[Span]| {
                for (point, left) in measured(points) {
                    let come = if point == arm.exit {
                        if !arm.timed {
                            continue;
                        }
                        0.0
                    } else if let Some(join) =
                        arm.joins.iter().find(|join| join.at == point && join.timed)
                    {
                        let straight = vec![join.exit, join.at];
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
            // The road stops at its junctions and, where it ends at the hub,
            // at the gate of the main settlement, from which it runs
            // straight in. It also runs through every settlement on or near
            // its line that the journey has time for, largest first.
            let fixed = |at: Point| Stop { at, through: None };
            let mut stops: Vec<Stop> = [arm.exit, arm.target]
                .into_iter()
                .chain(arm.joins.iter().map(|join| join.at))
                .map(fixed)
                .collect();
            let main_gate = sites
                .first()
                .filter(|main| arm.target == skeleton.hub && main.outline.center == arm.target)
                .map(|main| self.gate(main, arm.target, scale(line, -1.0 / span)))
                .filter(|gate| (0.02..0.98).contains(&share(*gate)));
            stops.extend(main_gate.map(fixed));
            stops.sort_by(|a, b| share(a.at).total_cmp(&share(b.at)));
            let mut near: Vec<(usize, f64)> = (1..sites.len())
                .filter_map(|index| {
                    let aside = (cross(line, sub(centre_of(index), arm.exit)) / span).abs();
                    let ring = &sites[index].outline.ring;
                    // One main road through a settlement: a second would
                    // meet the first in its middle.
                    (!joined[index]
                        && (0.05..=0.95).contains(&share(centre_of(index)))
                        && aside <= presets.roads.waypoint_reach_m
                        && !stops.iter().any(|stop| polygon_contains(ring, stop.at)))
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
            let through = |stops: &[Stop], bends: &[Bend], strength: f64| {
                let line = self.gated(stops, main_gate);
                let mut planned = Vec::new();
                let mut points = vec![line[0].0];
                for (leg, ends) in line.windows(2).enumerate() {
                    let ((from, straight), (to, _)) = (ends[0], ends[1]);
                    let run = match bends.get(leg) {
                        Some(bend) if !straight => self.run(from, to, bend, strength),
                        _ => vec![from, to],
                    };
                    let run = self.carried(SurfaceKind::CountryRoad, run, &mut planned)?;
                    points.extend(run.into_iter().skip(1));
                }
                Ok::<_, Vec<Diagnostic>>((points, planned))
            };
            // How many times a road through these stops changes bank.
            let crossings = |stops: &[Stop]| {
                let changes = stops
                    .windows(2)
                    .filter(|leg| self.water.bank(leg[0].at) != self.water.bank(leg[1].at));
                changes.count()
            };
            for (index, _) in near {
                let mut longer = stops.clone();
                longer.push(Stop {
                    at: centre_of(index),
                    through: Some(index),
                });
                longer.sort_by(|a, b| share(a.at).total_cmp(&share(b.at)));
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
            // One bend for each leg of the gated line: at most three legs a stop.
            let bends: Vec<Bend> = match swing.as_deref_mut() {
                Some(rng) => {
                    let line = self.gated(&stops, main_gate);
                    line.windows(2)
                        .map(|leg| self.bend(leg[0].0, leg[1].0, rng))
                        .collect()
                }
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
                    .filter(|(point, _)| arm.joins.iter().any(|join| join.at == *point)),
            );
            let pins: Vec<Point> = stops.iter().map(|stop| stop.at).collect();
            self.add(SurfaceKind::CountryRoad, points, planned, &pins);
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

/// Whether, with this water on the map, a straight main road in from the top
/// and the bottom edge still reaches the hub within the transit time: what a
/// river's course is held to before anything is placed beside it.
pub fn in_time(context: &Context, skeleton: &Skeleton, water: &Water) -> bool {
    Network::new(context, water, &[])
        .arms(skeleton, &mut [], None)
        .unwrap_or(false)
}

/// One of the map's roads: its kind and its authored points.
pub type Road = (SurfaceKind, Vec<Point>);

/// The map's roads and the bridges they cross its rivers by.
pub fn build(
    context: &Context,
    skeleton: &Skeleton,
    sites: &[Site],
    water: &Water,
    richness: f64,
    mut rng: Stream,
) -> Result<(Vec<Road>, Vec<Bridge>), Vec<Diagnostic>> {
    let presets = context.presets;
    let roads = &presets.roads;
    let extent = context.extent;
    let mut network = Network::new(context, water, sites);
    let class_of = |index: usize| presets.class(&sites[index].class_id);
    let centre_of = |index: usize| sites[index].outline.center;
    let mut joined = vec![false; sites.len()];
    network.arms(skeleton, &mut joined, Some(&mut rng))?;

    // Every other settlement joins the nearest road good enough for it,
    // largest first, so a town never hangs off a track. `SurfaceKind` orders
    // better roads first.
    let mut order: Vec<usize> = (0..sites.len()).collect();
    order.sort_by_key(|index| core::cmp::Reverse(class_of(*index).rank));
    for index in order {
        let kind = class_of(index).road;
        let outline = &sites[index].outline;
        let serves = |vertex: &Vertex| vertex.kind <= kind;
        // A road good enough for it that passes by the middle of its ground
        // is its main street already.
        let from = centre_of(index);
        let on_road = joined[index]
            || network
                .roads
                .iter()
                .filter(|(other, _)| *other <= kind)
                .flat_map(|(_, points)| points.windows(2))
                .any(|run| {
                    segment_distance(run[0], run[1], from) <= outline.reach * roads.through_reach
                });
        if on_road {
            joined[index] = true;
            network.side_roads(index);
            continue;
        }
        let bank = water.bank(from);
        let target = network
            .vertices
            .iter()
            .filter(|vertex| serves(vertex) && vertex.at != from)
            .filter_map(|vertex| {
                let awkward = network.joins(kind, from, vertex.at)?;
                // A road on its own bank is nearer than one a bridge away.
                let crossing = if vertex.bank == bank {
                    0.0
                } else {
                    presets.rivers.bridge.worth_m
                };
                Some((awkward, distance(from, vertex.at) + crossing, vertex.at))
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let Some((_, _, target)) = target else {
            return Err(context.fail(
                &format!("settlement-{index}"),
                "no road for it to join".into(),
            ));
        };
        // Its main street runs the length of its ground, through the centre,
        // and the road turns for the network past the end nearer to it. A
        // road that would have to turn sharply there runs straight for the
        // network from the centre instead.
        let along = if turn(outline.axis, bearing(from, target)) <= PI / 2.0 {
            outline.axis
        } else {
            outline.axis + PI
        };
        let end = network.gate(&sites[index], from, direction(along));
        let easy = turn(along, bearing(end, target)) <= roads.turn_max_deg.to_radians();
        let heading = if easy { along } else { bearing(from, target) };
        let gate = network.gate(&sites[index], from, direction(heading));
        let (mut points, planned) = network.route(kind, gate, target, &mut rng)?;
        points.insert(0, from);
        let far = outline.edge(heading + PI) * roads.main_street_reach;
        let far = round_cm(add(from, scale(direction(heading + PI), far)));
        network.join(kind, points, planned, &[from, gate]);
        // The road carries straight on past the centre as the settlement's
        // main street: one road, so parcels front it without a break there.
        let street = network.vertex(far, kind);
        match network.roads.last_mut() {
            Some((_, points)) if points[0] == from => {
                points.insert(0, far);
                network.vertices.push(street);
            }
            _ => network.add(kind, vec![far, from], Vec::new(), &[]),
        }
        joined[index] = true;
        network.side_roads(index);
    }

    // A richer network links neighbours directly (Gabriel pairs: no third
    // settlement inside the circle on the pair's diameter), unless one road
    // already runs through both. The main settlement has the edge roads.
    let reach = roads.gate_margin_m + SAME_PLACE_M;
    let within = |index: usize, points: &[Point]| {
        points
            .iter()
            .any(|p| ring_distance(&sites[index].outline.ring, *p) <= reach)
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
            // The rougher of the two settlements' roads, between the points
            // of their own roads that lie nearest each other.
            let kind = class_of(a).road.max(class_of(b).road);
            let Some(from) = network.approach(kind, a, pb) else {
                continue;
            };
            let Some(to) = network.approach(kind, b, from) else {
                continue;
            };
            // Two settlements that already share a road's point are linked.
            if distance(from, to) < SAME_PLACE_M {
                continue;
            }
            let (points, planned) = network.route(kind, from, to, &mut rng)?;
            network.join(kind, points, planned, &[]);
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
        // A road comes in from its edge: it is not laid to a settlement it
        // would have to run along the edge to reach.
        let inward = [-PI / 2.0, PI, PI / 2.0, 0.0][edge as usize];
        let ahead = |index: &usize| {
            turn(inward, bearing(exit, centre_of(*index))) <= roads.turn_max_deg.to_radians()
        };
        let target = (1..sites.len())
            .filter(|index| class_of(*index).road == SurfaceKind::CountryRoad)
            .filter(ahead)
            .min_by(|a, b| distance(exit, centre_of(*a)).total_cmp(&distance(exit, centre_of(*b))));
        if let Some(target) = target {
            let Some(to) = network.approach(SurfaceKind::CountryRoad, target, exit) else {
                continue;
            };
            if distance(exit, to) < SAME_PLACE_M {
                continue;
            }
            let (points, planned) = network.route(SurfaceKind::CountryRoad, exit, to, &mut rng)?;
            network.join(SurfaceKind::CountryRoad, points, planned, &[]);
        }
    }
    Ok((network.roads, network.crossings.bridges()))
}

/// A road's surface: its rounded line at its kind's width.
pub fn surface(
    context: &Context,
    index: usize,
    road: &Road,
) -> Result<SurfaceArea, Vec<Diagnostic>> {
    let (kind, points) = road;
    let width = context.presets.roads.width_m(*kind);
    let ends = vec![points[0], points[points.len() - 1]];
    // A bend the shared centreline refuses falls back to the straight road.
    GroundShape::stroke(points.clone(), width)
        .or_else(|_| GroundShape::stroke(ends, width))
        .map(|shape| SurfaceArea { kind: *kind, shape })
        .map_err(|message| context.fail(&format!("road-{index}"), message))
}
