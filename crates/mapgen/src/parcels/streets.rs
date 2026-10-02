//! A district's streets: a grid along the edge of the district that runs
//! longest with the settlement's main street, bent where the preset says,
//! kept where it lies inside the district and joins the roads the
//! settlement already has. No street crosses water: a street joins the
//! nearest road it can reach on its own bank.
use super::space::Rect;
use super::Pass;
use crate::layout::geometry::{
    add, bearing, direction, distance, round_cm, scale, segment_bounds, segment_crossing,
    segment_distance, sub, Grid, Point, TAU,
};
use crate::layout::water::Water;
use crate::{Diagnostic, DistrictPlan, MapPlan, SettlementPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::BTreeSet;

/// How far a street runs past the middle of the one it meets, so the two
/// centrelines cross whatever a centimetre of rounding did to either.
const JOIN_OVERSHOOT_M: f64 = 0.5;
/// A street runs to an end that faces it when that end is no more than
/// this far past the first carriageway it would cross.
const FACING_PAST_M: f64 = 10.0;
/// Two carriageways closer than half a block and within this of parallel
/// are one street drawn twice.
const PARALLEL_COS: f64 = 0.866;

/// One carriageway's rounded centreline.
pub struct Way {
    pub samples: Vec<Point>,
    pub half_width: f64,
    /// `[min_x, min_y, max_x, max_y]` of the samples.
    pub bounds: [f64; 4],
}

/// Every carriageway on the map so far: the layout's roads, then each street
/// as it is laid.
pub struct Network<'a> {
    pub ways: Vec<Way>,
    /// Sample segments with their way's half width.
    segments: Vec<(Point, Point, f64)>,
    grid: Grid,
    extent: f64,
    water: Water<'a>,
    /// The least ground between a street's middle and the water's edge.
    clearance: f64,
}

impl<'a> Network<'a> {
    pub fn new(plan: &'a MapPlan, clearance: f64) -> Self {
        let mut network = Self {
            ways: Vec::new(),
            segments: Vec::new(),
            grid: Grid::new(plan.size, 64.0),
            extent: plan.size[0].max(plan.size[1]),
            water: Water::new(&plan.rivers, plan.size),
            clearance,
        };
        for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
            network.add(&area.shape);
        }
        network
    }

    /// Strokes only: an apron is paved ground, not a way through.
    pub fn add(&mut self, shape: &GroundShape) {
        let GroundShape::Stroke {
            centerline,
            width_m,
        } = shape
        else {
            return;
        };
        let samples = centerline.samples().to_vec();
        let half_width = width_m / 2.0;
        for pair in samples.windows(2) {
            self.grid.insert(
                segment_bounds(pair[0], pair[1], half_width),
                self.segments.len() as u32,
            );
            self.segments.push((pair[0], pair[1], half_width));
        }
        let [x0, y0, x1, y1] = contract::ground::limits(&samples, 0.0);
        self.ways.push(Way {
            samples,
            half_width,
            bounds: [x0, y0, x1, y1],
        });
    }

    /// Whether any carriageway's surface lies on the rectangle.
    pub fn covers(&self, rect: &Rect) -> bool {
        self.grid.any(rect.bounds(), |item| {
            let (a, b, half_width) = self.segments[item as usize];
            // A parcel may touch a carriageway's edge, not share ground with it.
            rect.segment_gap(a, b) < half_width - 0.01
        })
    }

    /// Open ground between `p` and the nearest carriageway's edge, negative
    /// on one; `within` when there is at least that much.
    pub fn edge_gap(&self, p: Point, within: f64) -> f64 {
        let mut gap = within;
        // A carriageway is found by its own width's box, so only the gap
        // asked for is searched.
        let bounds = [p[0] - within, p[1] - within, p[0] + within, p[1] + within];
        self.grid.any(bounds, |item| {
            let (a, b, half_width) = self.segments[item as usize];
            gap = gap.min(segment_distance(a, b, p) - half_width);
            false
        });
        gap
    }

    /// The bearing of the carriageway nearest `p`, when one's middle lies
    /// within `within`.
    pub fn heading_near(&self, p: Point, within: f64) -> Option<f64> {
        let mut best: Option<(f64, f64)> = None;
        let bounds = [p[0] - within, p[1] - within, p[0] + within, p[1] + within];
        self.grid.any(bounds, |item| {
            let (a, b, _) = self.segments[item as usize];
            let away = segment_distance(a, b, p);
            if away <= within && best.is_none_or(|(known, _)| away < known) {
                best = Some((away, bearing(a, b)));
            }
            false
        });
        best.map(|(_, heading)| heading)
    }

    /// Whether a straight street from `a` to `b` keeps clear of the water.
    fn dry(&self, a: Point, b: Point) -> bool {
        self.water.segment_gap(a, b, self.clearance) >= self.clearance
    }

    /// The nearest point of any centreline a straight street from `p` can
    /// reach dry, searched outward in doubling boxes.
    fn nearest(&self, p: Point) -> Option<Nearest> {
        let mut reach = 64.0;
        loop {
            let mut best: Option<(f64, Nearest)> = None;
            let bounds = [p[0] - reach, p[1] - reach, p[0] + reach, p[1] + reach];
            self.grid.any(bounds, |item| {
                let (a, b, half_width) = self.segments[item as usize];
                let ab = sub(b, a);
                let t = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1])
                    / (ab[0] * ab[0] + ab[1] * ab[1]);
                // Past either end the nearest point is that end itself.
                let (point, end) = if t <= 0.0 {
                    (a, true)
                } else if t >= 1.0 {
                    (b, true)
                } else {
                    (add(a, scale(ab, t)), false)
                };
                let away = distance(p, point);
                if best.as_ref().is_none_or(|(known, _)| away < *known) && self.dry(p, point) {
                    let along = bearing(a, b);
                    let nearest = Nearest {
                        point,
                        end,
                        along,
                        half_width,
                    };
                    best = Some((away, nearest));
                }
                false
            });
            match best {
                // Everything within `reach` was searched, so nothing is nearer.
                Some((away, nearest)) if away <= reach => return Some(nearest),
                _ if reach > 2.0 * self.extent => return best.map(|(_, nearest)| nearest),
                _ => reach *= 2.0,
            }
        }
    }

    /// Where a street that ends at `from`, heading along the unit vector
    /// `toward`, would first cross a carriageway if it ran on for up to
    /// `reach`: the point just past that centreline. `None` when nothing
    /// lies ahead, what does runs nearly alongside, or water lies between.
    fn ahead(&self, from: Point, toward: Point, reach: f64) -> Option<Point> {
        // From just past the end, so the street's own last run is not met.
        let start = add(from, scale(toward, 0.1));
        let end = add(from, scale(toward, reach));
        let mut first: Option<(f64, f64)> = None;
        self.grid.any(segment_bounds(start, end, 0.0), |item| {
            let (a, b, _) = self.segments[item as usize];
            if let Some((t, _)) = segment_crossing(start, end, a, b) {
                if first.is_none_or(|(known, _)| t < known) {
                    let along = sub(b, a);
                    let cos = (toward[0] * along[0] + toward[1] * along[1]) / distance(a, b);
                    first = Some((t, cos.abs()));
                }
            }
            false
        });
        let (t, _) = first.filter(|(_, cos)| *cos < PARALLEL_COS)?;
        let met = distance(start, end) * t + 0.1;
        let met = round_cm(add(from, scale(toward, met + JOIN_OVERSHOOT_M)));
        self.dry(from, met).then_some(met)
    }

    /// The nearest end of a carriageway that faces a street stopped at
    /// `from` and heading along the unit vector `toward`: within `reach`
    /// ahead, no farther aside than the two would overlap, and running the
    /// opposite way within the angle that makes two streets one.
    fn facing_end(&self, from: Point, toward: Point, reach: f64) -> Option<Point> {
        let mut best: Option<(f64, Point)> = None;
        for way in &self.ways {
            let last = way.samples.len() - 1;
            for (end, before) in [(0, 1), (last, last - 1)] {
                let (at, out_of) = (way.samples[end], sub(way.samples[end], way.samples[before]));
                let offset = sub(at, from);
                let ahead = offset[0] * toward[0] + offset[1] * toward[1];
                let aside = (offset[0] * toward[1] - offset[1] * toward[0]).abs();
                let facing = (out_of[0] * toward[0] + out_of[1] * toward[1])
                    / distance(way.samples[end], way.samples[before]);
                if ahead > 1.0
                    && ahead <= reach
                    && aside <= way.half_width
                    && facing <= -PARALLEL_COS
                    && best.is_none_or(|(known, _)| ahead < known)
                    && self.dry(from, at)
                {
                    best = Some((ahead, at));
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// How a candidate street edge sits against what is already laid:
    /// whether it only repeats a carriageway beside it, and whether it
    /// crosses one.
    fn meets(&self, p: Point, q: Point, clearance: f64) -> (bool, bool) {
        let along = sub(q, p);
        let length = distance(p, q);
        let (mut repeats, mut crosses) = (false, false);
        self.grid.any(segment_bounds(p, q, clearance), |item| {
            let (a, b, _) = self.segments[item as usize];
            let crossing = segment_crossing(p, q, a, b).is_some();
            let other = sub(b, a);
            let cos = (along[0] * other[0] + along[1] * other[1]) / (length * distance(a, b));
            if cos.abs() >= PARALLEL_COS {
                let gap = if crossing {
                    0.0
                } else {
                    segment_distance(a, b, p)
                        .min(segment_distance(a, b, q))
                        .min(segment_distance(p, q, a))
                        .min(segment_distance(p, q, b))
                };
                repeats |= gap < clearance;
            } else {
                crosses |= crossing;
            }
            repeats
        });
        (repeats, crosses && !repeats)
    }
}

struct Nearest {
    point: Point,
    /// Whether it is the end of a centreline segment, not a point within one.
    end: bool,
    /// The bearing of the centreline there.
    along: f64,
    /// Half the carriageway's width.
    half_width: f64,
}

/// Lay every district's streets, nearest the settlement's roads first, so
/// each district joins a network that already reaches the main road. A
/// district's streets are a grid along one of its edges, bent where its
/// preset says: the edge that runs longest with the settlement's main
/// street, so neighbouring districts' long streets run the same way.
pub fn lay(
    pass: &Pass,
    network: &mut Network,
    settlement: &SettlementPlan,
    surfaces: &mut Vec<SurfaceArea>,
) -> Result<(), Vec<Diagnostic>> {
    // The main street is the carriageway through the settlement's centre.
    let main = network
        .nearest(settlement.center)
        .ok_or_else(|| pass.fail(&settlement.id, "$.plan.surfaces", "the map has no road"))?
        .along;
    // (distance to the nearest road, district)
    let mut order: Vec<(f64, usize)> = Vec::new();
    for (index, district) in settlement.districts.iter().enumerate() {
        let road = network
            .nearest(district.anchor)
            .ok_or_else(|| pass.fail(&settlement.id, "$.plan.surfaces", "the map has no road"))?;
        order.push((distance(district.anchor, road.point), index));
    }
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut lay = |network: &mut Network, points: Vec<Point>, kind: SurfaceKind, feature: &str| {
        let width = match kind {
            SurfaceKind::DirtTrack => pass.presets.roads.dirt_track_width_m,
            _ => pass.presets.parcels.street_width_m,
        };
        let shape = GroundShape::stroke(points, width)
            .map_err(|message| pass.fail(feature, "$.presets.parcels", &message))?;
        network.add(&shape);
        surfaces.push(SurfaceArea { kind, shape });
        Ok::<(), Vec<Diagnostic>>(())
    };
    let mut dead_ends: Vec<DeadEnd> = Vec::new();
    // The surface of the street each dead end belongs to.
    let mut ends_of: Vec<SurfaceKind> = Vec::new();
    for (_, index) in order {
        let district = &settlement.districts[index];
        let surface = pass.district(district)?.streets.surface;
        // Its long streets run with the edge that goes farthest the main
        // street's way.
        let with_main = |(a, b): &(&Point, &Point)| {
            distance(**a, **b) * libm::fabs(libm::cos(bearing(**a, **b) - main))
        };
        let frontage = contract::ground::edges(&district.ring)
            .max_by(|a, b| with_main(a).total_cmp(&with_main(b)))
            .map_or(0.0, |(a, b)| bearing(*a, *b));
        let lattice = Lattice::cut(pass, network, district, frontage)?;
        for points in lattice.streets(network, district, &mut dead_ends) {
            lay(network, points, surface, &district.id)?;
        }
        ends_of.resize(dead_ends.len(), surface);
    }
    // A street that stops within a block of another carriageway runs on to
    // it, so districts' grids meet each other and the roads beside them.
    // The run-on starts on the street's own last point. Where another
    // street's end faces it on the way, it runs to that end instead of past
    // it: the two are one street, not two drawn side by side.
    for (end, surface) in dead_ends.into_iter().zip(ends_of) {
        let met = network.ahead(end.at, end.toward, end.reach);
        let facing = network
            .facing_end(end.at, end.toward, end.reach)
            .filter(|facing| {
                met.is_none_or(|met| {
                    distance(end.at, *facing) <= distance(end.at, met) + FACING_PAST_M
                })
            });
        if let Some(to) = facing.or(met) {
            lay(network, vec![end.at, to], surface, &settlement.id)?;
        }
    }
    Ok(())
}

/// Where a street stops without meeting another.
struct DeadEnd {
    at: Point,
    /// Unit vector the street was heading along.
    toward: Point,
    /// How far it may run on to meet a carriageway: one block.
    reach: f64,
}

/// One district's street grid: nodes a step apart along its long streets and
/// its cross streets, and the edges between neighbouring nodes that lie in
/// the district and do not repeat a carriageway already there.
struct Lattice {
    /// Each node's place on the map, by `row × columns + column`; only nodes
    /// on a street are used.
    positions: Vec<Point>,
    /// The long streets' edges first, each line's in order along it.
    edges: Vec<[usize; 2]>,
    long_edges: usize,
    /// The edges at each node.
    links: Vec<Vec<usize>>,
    /// Nodes of edges that cross a carriageway already laid.
    seeds: Vec<usize>,
    /// A straight grid is authored by its streets' ends; a bent one by
    /// every node.
    straight: bool,
    block_depth: f64,
}

impl Lattice {
    fn cut(
        pass: &Pass,
        network: &Network,
        district: &DistrictPlan,
        axis: f64,
    ) -> Result<Self, Vec<Diagnostic>> {
        let pattern = pass.district(district)?.streets;
        let step = pass.presets.parcels.street_step_m;
        // Whole steps per block, so every junction is a node.
        let per_length = (libm::round(pattern.block_length_m / step) as i64).max(1);
        let per_depth = (libm::round(pattern.block_depth_m / step) as i64).max(1);
        let (su, sv) = (
            pattern.block_length_m / per_length as f64,
            pattern.block_depth_m / per_depth as f64,
        );
        let mut rng = pass.stream(&format!("streets/{}", district.id));
        let offset = rng.unit() * pattern.block_length_m;
        let phases = [rng.range([0.0, TAU]), rng.range([0.0, TAU])];
        let (amplitude, wave) = pattern.bend.map_or((0.0, 0.0), |bend| {
            (bend.amplitude_m, TAU / bend.wavelength_m)
        });
        let (along, across) = (direction(axis), direction(axis + TAU / 4.0));
        // The district's reach in its own frame, in steps from the anchor.
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for vertex in &district.ring {
            let d = sub(*vertex, district.anchor);
            let local = [
                d[0] * along[0] + d[1] * along[1],
                d[0] * across[0] + d[1] * across[1],
            ];
            for i in 0..2 {
                low[i] = low[i].min(local[i] - amplitude);
                high[i] = high[i].max(local[i] + amplitude);
            }
        }
        let a0 = libm::floor((low[0] - offset) / su) as i64;
        let b0 = libm::floor(low[1] / sv) as i64;
        let columns = (libm::ceil((high[0] - offset) / su) as i64 - a0 + 1) as usize;
        let rows = (libm::ceil(high[1] / sv) as i64 - b0 + 1) as usize;
        let on_long = |row: usize| (row as i64 + b0).rem_euclid(per_depth) == 0;
        let on_cross = |column: usize| (column as i64 + a0).rem_euclid(per_length) == 0;
        let index = |column: usize, row: usize| row * columns + column;
        let mut positions = vec![district.anchor; columns * rows];
        let mut inside = vec![false; columns * rows];
        let nodes = (0..rows).flat_map(|row| (0..columns).map(move |column| (column, row)));
        for (column, row) in nodes {
            if !on_long(row) && !on_cross(column) {
                continue;
            }
            // Both street families swing, each by where it is along the other.
            let (u, v) = (
                (column as i64 + a0) as f64 * su + offset,
                (row as i64 + b0) as f64 * sv,
            );
            let (u, v) = (
                u + amplitude * libm::sin(wave * v + phases[1]),
                v + amplitude * libm::sin(wave * u + phases[0]),
            );
            let p = round_cm(add(district.anchor, add(scale(along, u), scale(across, v))));
            positions[index(column, row)] = p;
            inside[index(column, row)] = polygon_contains(&district.ring, p);
        }
        // Cross streets left out, block by block.
        let mut skipped = BTreeSet::new();
        for column in (0..columns).filter(|column| on_cross(*column)) {
            for band in 0..rows as i64 / per_depth + 2 {
                if rng.chance(pattern.cross_skip) {
                    skipped.insert((column, band));
                }
            }
        }
        let mut lattice = Self {
            positions,
            edges: Vec::new(),
            long_edges: 0,
            links: vec![Vec::new(); columns * rows],
            seeds: Vec::new(),
            straight: pattern.bend.is_none(),
            block_depth: pattern.block_depth_m,
        };
        let clearance = pattern.block_depth_m / 2.0;
        // A district no larger than one block of its own pattern is that
        // block: it has no streets but the carriageways that bound it.
        let one_block = high[0] - low[0] - 2.0 * amplitude <= pattern.block_length_m
            && high[1] - low[1] - 2.0 * amplitude <= pattern.block_depth_m;
        let consider = |lattice: &mut Self, i: usize, j: usize| {
            if one_block || !inside[i] || !inside[j] {
                return;
            }
            let (repeats, crosses) =
                network.meets(lattice.positions[i], lattice.positions[j], clearance);
            if repeats {
                return;
            }
            if crosses {
                lattice.seeds.push(i);
            }
            lattice.links[i].push(lattice.edges.len());
            lattice.links[j].push(lattice.edges.len());
            lattice.edges.push([i, j]);
        };
        for row in (0..rows).filter(|row| on_long(*row)) {
            for column in 0..columns - 1 {
                consider(&mut lattice, index(column, row), index(column + 1, row));
            }
        }
        lattice.long_edges = lattice.edges.len();
        for column in (0..columns).filter(|column| on_cross(*column)) {
            for row in 0..rows - 1 {
                let band = (row as i64 + b0).div_euclid(per_depth) - b0.div_euclid(per_depth);
                if !skipped.contains(&(column, band)) {
                    consider(&mut lattice, index(column, row), index(column, row + 1));
                }
            }
        }
        Ok(lattice)
    }

    /// The node `from` and every node joined to it by edges, marked `joined`.
    fn piece(&self, from: usize, joined: &mut [bool]) -> Vec<usize> {
        let mut piece = vec![from];
        let mut next = 0;
        joined[from] = true;
        while let Some(at) = piece.get(next).copied() {
            next += 1;
            for end in self.links[at].iter().flat_map(|edge| self.edges[*edge]) {
                if !joined[end] {
                    joined[end] = true;
                    piece.push(end);
                }
            }
        }
        piece
    }

    /// The district's streets as authored points: the grid's own, and the
    /// links that join every piece of it to the network. Where a street of
    /// the grid stops without meeting another is added to `dead_ends`.
    fn streets(
        &self,
        network: &Network,
        district: &DistrictPlan,
        dead_ends: &mut Vec<DeadEnd>,
    ) -> Vec<Vec<Point>> {
        let mut streets: Vec<Vec<Point>> = Vec::new();
        // A link from `from` to the nearest carriageway. It ends on a
        // segment's own end exactly, unrounded (running past a dead end
        // would cross nothing), and otherwise runs just past the centreline
        // it meets.
        let link = |from: Point| {
            let to = network.nearest(from).filter(|to| to.point != from)?;
            let gap = distance(from, to.point);
            let reach = if to.end { gap } else { gap + JOIN_OVERSHOOT_M };
            let target = add(from, scale(sub(to.point, from), reach / gap));
            Some(vec![from, if to.end { to.point } else { round_cm(target) }])
        };
        // A district too small for a grid has a lane from its anchor to the
        // nearest road, unless a road already runs through it or along its
        // edge (within its own half width of it, as its parcels front it).
        let served = |to: Nearest| {
            polygon_contains(&district.ring, to.point)
                || contract::ground::edges(&district.ring)
                    .any(|(a, b)| segment_distance(*a, *b, to.point) <= to.half_width)
        };
        if self.edges.is_empty() && !network.nearest(district.anchor).is_some_and(served) {
            streets.extend(link(district.anchor));
        }
        // Every piece of the grid is joined: the pieces a road crosses
        // already are, and each other piece gets one link, from its nearest
        // node to the nearest street, be that the network's or a piece
        // joined before it. The nodes a link starts or ends on are authored
        // points of the streets through them, so it meets them exactly.
        let mut pinned = BTreeSet::new();
        let mut joined = vec![false; self.positions.len()];
        let mut joined_nodes: Vec<usize> = Vec::new();
        for seed in &self.seeds {
            if !joined[*seed] {
                joined_nodes.extend(self.piece(*seed, &mut joined));
            }
        }
        for edge in &self.edges {
            if joined[edge[0]] {
                continue;
            }
            let piece = self.piece(edge[0], &mut joined);
            // (gap, node of this piece, node it links to, or the network)
            let mut best: Option<(f64, usize, Option<usize>)> = None;
            let mut offer = |gap: f64, from: usize, to: Option<usize>| {
                if best.is_none_or(|(known, ..)| gap < known) {
                    best = Some((gap, from, to));
                }
            };
            for from in &piece {
                let p = self.positions[*from];
                if let Some(to) = network.nearest(p) {
                    offer(distance(p, to.point), *from, None);
                }
                for to in &joined_nodes {
                    offer(distance(p, self.positions[*to]), *from, Some(*to));
                }
            }
            if let Some((_, from, to)) = best {
                pinned.insert(from);
                match to {
                    Some(to) => {
                        pinned.insert(to);
                        streets.push(vec![self.positions[from], self.positions[to]]);
                    }
                    None => streets.extend(link(self.positions[from])),
                }
            }
            joined_nodes.extend(piece);
        }
        // A street is a run of edges along one line.
        let mut emit = |nodes: &[usize], long: bool| {
            let points = self.street(nodes, long, &pinned);
            let last = points.len() - 1;
            for (node, (end, before)) in [nodes[0], nodes[nodes.len() - 1]]
                .into_iter()
                .zip([(0, 1), (last, last - 1)])
            {
                if self.links[node].len() == 1 && !pinned.contains(&node) {
                    let run = distance(points[before], points[end]);
                    dead_ends.push(DeadEnd {
                        at: points[end],
                        toward: scale(sub(points[end], points[before]), 1.0 / run),
                        reach: self.block_depth,
                    });
                }
            }
            streets.push(points);
        };
        let mut run: Vec<usize> = Vec::new();
        for (edge, ends) in self.edges.iter().enumerate() {
            if (run.last() != Some(&ends[0]) || edge == self.long_edges) && !run.is_empty() {
                emit(&run, edge <= self.long_edges);
                run.clear();
            }
            if run.is_empty() {
                run.push(ends[0]);
            }
            run.push(ends[1]);
        }
        if !run.is_empty() {
            emit(&run, self.edges.len() == self.long_edges);
        }
        streets
    }

    /// One street's authored points. A bent street shares every node with
    /// the streets it meets. A straight one is its ends and any pinned node,
    /// and where it ends on a street that carries on past it, it runs just
    /// past that street's middle. Two streets that both end on one node share
    /// it: the plan's joint pass makes them one street round the corner.
    fn street(&self, nodes: &[usize], long: bool, pinned: &BTreeSet<usize>) -> Vec<Point> {
        let ends = [nodes[0], nodes[nodes.len() - 1]];
        let mut points: Vec<Point> = nodes
            .iter()
            .filter(|i| !self.straight || ends.contains(i) || pinned.contains(i))
            .map(|i| self.positions[*i])
            .collect();
        let last = points.len() - 1;
        for (at, (end, toward)) in ends.into_iter().zip([(0, 1), (last, last - 1)]) {
            let meets = self.links[at]
                .iter()
                .filter(|edge| (**edge < self.long_edges) != long)
                .count()
                > 1;
            if self.straight && !pinned.contains(&at) && meets {
                let out = sub(points[end], points[toward]);
                let past = scale(
                    out,
                    JOIN_OVERSHOOT_M / distance(points[end], points[toward]),
                );
                points[end] = round_cm(add(points[end], past));
            }
        }
        points
    }
}
