//! Road ends. A stroke is cut square across its first and last point
//! (`contract::ground`), so the flat face of an end shows wherever no other
//! paving covers it. Both generation steps hand their carriageways to
//! [`close`], which leaves every end one of these:
//!
//! - **part of a through road.** Carriageways of one kind and width that meet
//!   end to end become one stroke through the point they shared, so the bend
//!   there is the centreline's own rounded one;
//! - **under the road it joins.** An end that touches another carriageway
//!   stops just past that carriageway's middle, where its face lies inside
//!   the other's width;
//! - **the outer edge of a corner.** Where unlike roads meet at a corner, the
//!   wider one runs on over the narrower one's end, and its own end is the
//!   corner's outer edge;
//! - **square on the map's edge**, where a road leaves the map;
//! - **at the last block it serves.** A road that ran on past it into open
//!   country is cut back to it.
//!
//! A wider road that ends on a narrower one shows its shoulders, which is
//! what a road that narrows looks like.
//!
//! Not closed yet, and counted by `tests/road_ends.rs`: two roads of one
//! width that fork at less than about 70°, which leave a bite between their
//! ends; three roads whose ends stand a few metres apart round one junction;
//! and two streets of one width laid side by side, which leave a step where
//! one stops.
//!
//! The plan's road graph joins two roads where their centrelines cross
//! (`layout::measure`), so no step here may leave an end touching a line
//! that a centimetre's rounding has moved off it: ends that share a point
//! keep it, or all run on past it far enough to cross.
//!
//! Arithmetic is `+ − × ÷` and `libm`, and every point written is a whole
//! centimetre, as everywhere in a plan.
use crate::layout::geometry::{
    add, cross, distance, dot, ring_distance, round_cm, scale, segment_bounds, segment_crossing,
    segment_distance, sub, Grid, Point,
};
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::{BTreeMap, BTreeSet};

/// Ends this near each other are one place.
const MEET_M: f64 = 0.5;
/// An end stops this far past the middle of the road it joins, so the two
/// centrelines cross whatever a centimetre of rounding did to either.
const OVERSHOOT_M: f64 = 0.25;
/// An end joins a carriageway whose edge is this near it.
const NEAR_M: f64 = 2.5;
/// Two street ends that turn back on each other this exactly (about 5°)
/// are not welded: the shared centreline has no bend to give them.
const REVERSE_COS: f64 = -0.996;
/// At a junction, two ends within this of straight (30°) are one road
/// through it.
const THROUGH_COS: f64 = 0.866;
/// A country road or a track is one road through a turn no sharper than
/// this (110°): a lane round the corner of a block, a few degrees past
/// square. Sharper, the two stay two roads that meet.
const ROAD_TURN_COS: f64 = -0.342;
/// A face along another carriageway's edge is covered by it.
const FLUSH_M: f64 = 0.02;
/// An end is moved no less than this.
const REACHED_M: f64 = 0.005;
/// An end this near a road's middle stands on it, and has not passed it.
const ON_MIDDLE_M: f64 = 0.1;
/// How far a centimetre's rounding may move a point.
const ROUNDING_M: f64 = 0.005;
/// A road nearly alongside is not one an end runs into (20°).
const ALONGSIDE_COS: f64 = 0.94;
/// A road leaves the map square to its edge over twice its width.
const GATE_WIDTHS: f64 = 2.0;
/// Lines this near parallel (about 1°) have no corner between them.
const SQUARE_SIN: f64 = 0.02;
/// A road cut back to a settlement stops this far past the last ground it
/// runs along, and is walked back to it in steps this long.
const TRIM_MARGIN_M: f64 = 1.0;
const TRIM_STEP_M: f64 = 2.0;
/// How many times the ways of a lost joint are pinned and the rest closed
/// again, before the plan's roads are left as they were laid.
const PIN_ROUNDS: usize = 4;
/// No end is cut back, and no gate laid, to leave a run shorter than this.
const SHORTEST_RUN_M: f64 = 1.0;

/// One carriageway's authored line.
#[derive(Clone)]
struct Way {
    kind: SurfaceKind,
    width: f64,
    points: Vec<Point>,
    /// Where it stands among the surfaces: its first piece's place.
    slot: usize,
    /// The places of every way it was laid as.
    pieces: Vec<usize>,
    /// Left exactly as laid: closing it lost a joint.
    pinned: bool,
}

/// One end of one way: the way, and whether it is the way's last point.
type End = (usize, bool);

impl Way {
    fn half(&self) -> f64 {
        self.width / 2.0
    }

    /// An end's point and the unit vector the way runs out of it along.
    fn end(&self, last: bool) -> (Point, Point) {
        let (at, before) = (self.points[self.at(last)], self.before(last));
        (at, scale(sub(at, before), 1.0 / distance(at, before)))
    }

    fn at(&self, last: bool) -> usize {
        if last {
            self.points.len() - 1
        } else {
            0
        }
    }

    /// The authored point next to an end.
    fn before(&self, last: bool) -> Point {
        if last {
            self.points[self.points.len() - 2]
        } else {
            self.points[1]
        }
    }

    fn shape(&self) -> Result<GroundShape, String> {
        GroundShape::stroke(self.points.clone(), self.width)
    }
}

/// The carriageway strokes of `surfaces` with their ends closed, in place:
/// a welded road stands where its first piece stood, and everything else
/// keeps its order.
///
/// `grounds` are the settlements' blocks: a road that stops in open country
/// past the last of them is cut back to it.
///
/// No joint is lost to it. Whatever two ways' centrelines crossed before,
/// they cross after or are one way; where a change would break that (a
/// street that touched a bend at a tangent, say), the ways concerned are
/// left exactly as they were laid and the rest are closed round them.
pub fn close(surfaces: Vec<SurfaceArea>, size: [f64; 2], grounds: &[&[Point]]) -> Vec<SurfaceArea> {
    let mut laid: Vec<Way> = Vec::new();
    let mut kept: Vec<Option<SurfaceArea>> = Vec::with_capacity(surfaces.len());
    for (slot, area) in surfaces.into_iter().enumerate() {
        match &area.shape {
            GroundShape::Stroke {
                centerline,
                width_m,
            } if area.kind.is_road() => {
                laid.push(Way {
                    kind: area.kind,
                    width: *width_m,
                    points: centerline.control_points().to_vec(),
                    slot,
                    pieces: vec![slot],
                    pinned: false,
                });
                kept.push(None);
            }
            _ => kept.push(Some(area)),
        }
    }
    let slots = kept.len();
    let joined_before = joined(&laid, size);
    let mut ways = closed(&laid, size, grounds);
    // A few rounds settle it: each pins the ways of the joints the last lost.
    for _ in 0..PIN_ROUNDS {
        let broken = lost(&joined_before, &ways, size, slots);
        if broken.is_empty() {
            break;
        }
        for way in &mut laid {
            way.pinned |= broken.contains(&way.slot);
        }
        ways = closed(&laid, size, grounds);
    }
    if !lost(&joined_before, &ways, size, slots).is_empty() {
        ways = laid;
    }
    for way in ways {
        let shape = way.shape().expect("every change to a way was checked");
        kept[way.slot] = Some(SurfaceArea {
            kind: way.kind,
            shape,
        });
    }
    kept.into_iter().flatten().collect()
}

/// Every step, on a copy of the ways as they were laid. A pinned way is
/// left out of each.
fn closed(laid: &[Way], size: [f64; 2], grounds: &[&[Point]]) -> Vec<Way> {
    let mut ways = laid.to_vec();
    trim(&mut ways, size, grounds);
    meet(&mut ways, size);
    let mut ways = weld(ways);
    let settled = corner(&mut ways);
    gate(&mut ways, size);
    snap(&mut ways, size, &settled);
    ways
}

/// The pairs of laid ways whose centrelines cross, each by its slot, lower
/// first: what the plan's road graph joins (`layout::measure`), by the same
/// crossing test. Pieces of one welded way are joined by being one way.
fn joined(ways: &[Way], size: [f64; 2]) -> BTreeSet<(usize, usize)> {
    let paving = Paving::new(ways, size);
    let mut pairs = BTreeSet::new();
    for (index, (way, a, b)) in paving.stretches.iter().enumerate() {
        paving.grid.any(segment_bounds(*a, *b, 0.0), |item| {
            let (other, c, d) = paving.stretches[item as usize];
            if item as usize > index && other != *way && segment_crossing(*a, *b, c, d).is_some() {
                let (low, high) = (
                    ways[*way].slot.min(ways[other].slot),
                    ways[*way].slot.max(ways[other].slot),
                );
                pairs.insert((low, high));
            }
            false
        });
    }
    pairs
}

/// The slots of every laid way that is part of a joint `before` has and
/// `closed` has lost, with the slots of the ways welded to them.
fn lost(
    before: &BTreeSet<(usize, usize)>,
    closed: &[Way],
    size: [f64; 2],
    slots: usize,
) -> BTreeSet<usize> {
    // The closed way each laid slot is now part of.
    let mut owner = vec![usize::MAX; slots];
    for (index, way) in closed.iter().enumerate() {
        for piece in &way.pieces {
            owner[*piece] = index;
        }
    }
    let after = joined(closed, size);
    let mut out = BTreeSet::new();
    for &(a, b) in before {
        let (i, j) = (owner[a], owner[b]);
        let (low, high) = (
            closed[i].slot.min(closed[j].slot),
            closed[i].slot.max(closed[j].slot),
        );
        if i != j && !after.contains(&(low, high)) {
            out.extend(closed[i].pieces.iter().chain(&closed[j].pieces).copied());
        }
    }
    out
}

/// Every end of every way, grouped where they meet.
fn nodes(ways: &[Way]) -> Vec<Vec<End>> {
    // Ends in whole-metre cells; an end meets those in its own and the
    // neighbouring cells that lie within `MEET_M`.
    let cell = |p: Point| [libm::floor(p[0]) as i64, libm::floor(p[1]) as i64];
    let mut cells: BTreeMap<[i64; 2], Vec<End>> = BTreeMap::new();
    let ends: Vec<End> = (0..ways.len())
        .flat_map(|way| [(way, false), (way, true)])
        .collect();
    for &(way, last) in &ends {
        cells
            .entry(cell(ways[way].end(last).0))
            .or_default()
            .push((way, last));
    }
    let mut grouped = vec![[false; 2]; ways.len()];
    let mut out: Vec<Vec<End>> = Vec::new();
    for &end in &ends {
        if grouped[end.0][usize::from(end.1)] {
            continue;
        }
        // Everything reachable from this end by hops of `MEET_M`.
        let mut members = vec![end];
        grouped[end.0][usize::from(end.1)] = true;
        let mut next = 0;
        while let Some(&(way, last)) = members.get(next) {
            next += 1;
            let at = ways[way].end(last).0;
            let [cx, cy] = cell(at);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    for &other in cells.get(&[cx + dx, cy + dy]).into_iter().flatten() {
                        if !grouped[other.0][usize::from(other.1)]
                            && distance(ways[other.0].end(other.1).0, at) <= MEET_M
                        {
                            grouped[other.0][usize::from(other.1)] = true;
                            members.push(other);
                        }
                    }
                }
            }
        }
        out.push(members);
    }
    out
}

/// The ways' rounded stretches, bucketed by the ground their width covers.
struct Paving {
    shapes: Vec<GroundShape>,
    /// Each way's half width.
    halves: Vec<f64>,
    /// (way, one rounded stretch of it)
    stretches: Vec<(usize, Point, Point)>,
    grid: Grid,
    widest: f64,
}

impl Paving {
    fn new(ways: &[Way], size: [f64; 2]) -> Self {
        let shapes: Vec<GroundShape> = ways
            .iter()
            .map(|way| way.shape().expect("every way is a valid stroke"))
            .collect();
        let mut stretches = Vec::new();
        let mut grid = Grid::new(size, 64.0);
        for (way, shape) in shapes.iter().enumerate() {
            let GroundShape::Stroke { centerline, .. } = shape else {
                unreachable!()
            };
            for pair in centerline.samples().windows(2) {
                grid.insert(
                    segment_bounds(pair[0], pair[1], ways[way].half() + NEAR_M),
                    stretches.len() as u32,
                );
                stretches.push((way, pair[0], pair[1]));
            }
        }
        Self {
            shapes,
            halves: ways.iter().map(Way::half).collect(),
            stretches,
            grid,
            widest: ways.iter().map(Way::half).fold(0.0, f64::max),
        }
    }

    /// Whether a way other than `own` and `mate`, at least `half` wide
    /// either side, has its edge within `NEAR_M` of `p`.
    fn touched(&self, p: Point, own: usize, mate: usize, half: f64) -> bool {
        self.grid.any([p[0], p[1], p[0], p[1]], |item| {
            let way = self.stretches[item as usize].0;
            way != own
                && way != mate
                && self.halves[way] >= half
                && self.shapes[way].contains(p, NEAR_M)
        })
    }

    /// Whether the square end of way `own` would be covered by other ways
    /// if it stood at `at`, facing along `out_of`.
    fn hides(&self, at: Point, out_of: Point, own: usize) -> bool {
        let across = scale([-out_of[1], out_of[0]], self.halves[own]);
        [-1.0, -0.5, 0.0, 0.5, 1.0].into_iter().all(|share| {
            let p = add(at, scale(across, share));
            self.grid.any([p[0], p[1], p[0], p[1]], |item| {
                let way = self.stretches[item as usize].0;
                way != own && self.shapes[way].contains(p, FLUSH_M)
            })
        })
    }

    /// How far on from `at`, along `out_of`, a way `half` wide either side
    /// must run for its square end to stand past every narrower carriageway
    /// that touches `at`: past each of their stretches, as far as it lies
    /// within this way's width. `None` when no narrower one touches it.
    fn narrower_past(&self, at: Point, out_of: Point, half: f64, own: usize) -> Option<f64> {
        let normal = [-out_of[1], out_of[0]];
        let reach = 3.0 * half;
        let mut past: Option<f64> = None;
        self.grid.any(
            [at[0] - reach, at[1] - reach, at[0] + reach, at[1] + reach],
            |item| {
                let (way, c, d) = self.stretches[item as usize];
                let narrow = self.halves[way];
                if way == own || narrow >= half || !self.shapes[way].contains(at, NEAR_M) {
                    return false;
                }
                // The stretch's two edges, clipped to this way's width, and
                // the round joint at each of its ends.
                let run = scale(sub(d, c), 1.0 / distance(c, d));
                let aside = scale([-run[1], run[0]], narrow);
                let mut reach_of = |p: Point, grown: f64| {
                    if dot(sub(p, at), normal).abs() <= half {
                        let along = dot(sub(p, at), out_of) + grown;
                        past = Some(past.map_or(along, |known: f64| known.max(along)));
                    }
                };
                for side in [-1.0, 1.0] {
                    let (a, b) = (add(c, scale(aside, side)), add(d, scale(aside, side)));
                    let (from, to) = (dot(sub(a, at), normal), dot(sub(b, at), normal));
                    reach_of(a, 0.0);
                    reach_of(b, 0.0);
                    // Where the edge crosses this way's two sides.
                    for edge in [-half, half] {
                        let share = (edge - from) / (to - from);
                        if from != to && (0.0..=1.0).contains(&share) {
                            reach_of(add(a, scale(sub(b, a), share)), 0.0);
                        }
                    }
                }
                reach_of(c, narrow);
                reach_of(d, narrow);
                false
            },
        );
        past.map(|past| past.min(reach))
    }

    /// Whether the middle of a way other than `own` and `mate` crosses the
    /// run from `a` to `b` or ends on it.
    fn crossed(&self, a: Point, b: Point, own: usize, mate: usize) -> bool {
        self.grid.any(segment_bounds(a, b, 0.0), |item| {
            let (way, c, d) = self.stretches[item as usize];
            way != own
                && way != mate
                && (segment_crossing(a, b, c, d).is_some()
                    || segment_distance(a, b, c) <= MEET_M
                    || segment_distance(a, b, d) <= MEET_M)
        })
    }
}

/// A way that runs out past the last block on its line and stops in open
/// country, by no other carriageway and short of the map's edge, is cut
/// back to the last block it runs on or along, or to the last carriageway
/// that joins it if that comes first: a main street is laid to the end of
/// the ground a settlement may build on, and the settlement may not have
/// grown that far.
fn trim(ways: &mut [Way], size: [f64; 2], grounds: &[&[Point]]) {
    if ways.is_empty() || grounds.is_empty() {
        return;
    }
    let paving = Paving::new(ways, size);
    let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
    let limits: Vec<[f64; 4]> = grounds
        .iter()
        .map(|ring| contract::ground::limits(ring, 0.0))
        .collect();
    // Whether `p` is on a settlement's ground or within `reach` of its edge:
    // a road along a block's edge fronts that block.
    let serves = |p: Point, reach: f64| {
        grounds.iter().zip(&limits).any(|(ring, [x0, y0, x1, y1])| {
            p[0] >= x0 - reach
                && p[0] <= x1 + reach
                && p[1] >= y0 - reach
                && p[1] <= y1 + reach
                && ring_distance(ring, p) <= reach
        })
    };
    let mut cuts: Vec<(End, usize, Point)> = Vec::new();
    for (way, line) in ways.iter().enumerate().filter(|(_, line)| !line.pinned) {
        let reach = line.half() + TRIM_MARGIN_M;
        for last in [false, true] {
            let at = line.end(last).0;
            if !on_map(at) || serves(at, reach) || paving.touched(at, way, way, 0.0) {
                continue;
            }
            // Walk back from the end, a step at a time, to the first point
            // that serves a settlement or that another carriageway touches.
            let count = line.points.len();
            let cut = (0..count - 1).find_map(|step| {
                let (near, far) = if last {
                    (line.points[count - 1 - step], line.points[count - 2 - step])
                } else {
                    (line.points[step], line.points[step + 1])
                };
                let steps = libm::ceil(distance(near, far) / TRIM_STEP_M).max(1.0);
                (0..=steps as usize)
                    .map(|k| add(near, scale(sub(far, near), k as f64 / steps)))
                    .find(|p| serves(*p, reach) || paving.touched(*p, way, way, 0.0))
                    .map(|p| (step, round_cm(p)))
            });
            if let Some((step, to)) = cut {
                cuts.push(((way, last), step, to));
            }
        }
    }
    for ((way, last), step, to) in cuts {
        let original = ways[way].points.clone();
        let points = &mut ways[way].points;
        if last {
            points.truncate(points.len() - step);
        } else {
            points.drain(..step);
        }
        let at = if last { points.len() - 1 } else { 0 };
        points[at] = to;
        points.dedup();
        if points.len() < 2 || ways[way].shape().is_err() {
            ways[way].points = original;
        }
    }
}

/// Two ends that stop within each other's width, on lines that cross there,
/// are a corner whose ways each fell short of the other or ran past it. Both
/// are moved to where their lines cross, and so meet end to end. An end
/// that touches a third carriageway as wide as either is joining that one,
/// and is left; a narrower one they both end on is one they cross.
fn meet(ways: &mut [Way], size: [f64; 2]) {
    if ways.is_empty() {
        return;
    }
    let paving = Paving::new(ways, size);
    let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
    let lone: Vec<End> = nodes(ways)
        .into_iter()
        .filter(|node| node.len() == 1)
        .flatten()
        .filter(|(way, last)| !ways[*way].pinned && on_map(ways[*way].end(*last).0))
        .collect();
    let mut cells: BTreeMap<[i64; 2], Vec<End>> = BTreeMap::new();
    let cell = |p: Point| [0, 1].map(|k| libm::floor(p[k] / (2.0 * paving.widest)) as i64);
    for &(way, last) in &lone {
        cells
            .entry(cell(ways[way].end(last).0))
            .or_default()
            .push((way, last));
    }
    // Where the lines of two ends cross, when that makes them a corner.
    let corner = |from: End, to: End| {
        let (a, b) = (&ways[from.0], &ways[to.0]);
        let ((p, along), (q, other)) = (a.end(from.1), b.end(to.1));
        let reach = a.half() + b.half();
        let sin = cross(along, other);
        if from.0 == to.0 || distance(p, q) > reach || sin.abs() < SQUARE_SIN {
            return None;
        }
        let at = add(p, scale(along, cross(sub(q, p), other) / sin));
        let fits = |way: &Way, last: bool, out_of: Point| {
            dot(sub(at, way.before(last)), out_of) >= SHORTEST_RUN_M
        };
        // Nothing a third carriageway joins is cut off or moved away.
        (distance(at, p) <= reach
            && distance(at, q) <= reach
            && fits(a, from.1, along)
            && fits(b, to.1, other)
            && !paving.touched(p, from.0, to.0, a.half().min(b.half()))
            && !paving.touched(q, to.0, from.0, a.half().min(b.half()))
            && !paving.crossed(at, p, from.0, to.0)
            && !paving.crossed(at, q, to.0, from.0))
        .then_some(round_cm(at))
    };
    // Each end's nearest corner; a pair is moved when each is the other's.
    let nearest = |from: End| {
        let p = ways[from.0].end(from.1).0;
        let [cx, cy] = cell(p);
        let mut best: Option<(f64, End, Point)> = None;
        for dy in -1..=1 {
            for dx in -1..=1 {
                for &to in cells.get(&[cx + dx, cy + dy]).into_iter().flatten() {
                    let apart = distance(p, ways[to.0].end(to.1).0);
                    if best.is_none_or(|(known, ..)| apart < known) {
                        if let Some(at) = corner(from, to) {
                            best = Some((apart, to, at));
                        }
                    }
                }
            }
        }
        best.map(|(_, to, at)| (to, at))
    };
    let mut moves: Vec<(End, Point)> = Vec::new();
    for &from in &lone {
        if let Some((to, at)) = nearest(from) {
            if from < to && nearest(to).is_some_and(|(back, _)| back == from) {
                moves.extend([(from, at), (to, at)]);
            }
        }
    }
    apply(ways, moves);
}

/// Ways of one kind and width that meet end to end, made one way through
/// the point they shared. Two that meet alone are one road round whatever
/// corner they make. Where more meet, only those that carry nearly straight
/// on are joined, the straightest pair first: a road through a junction.
/// A bend welded there would leave the others touching its rounded corner
/// from outside, which is no crossing; they are left to [`corner`].
fn weld(ways: Vec<Way>) -> Vec<Way> {
    // The end each end is joined to.
    let mut link: BTreeMap<End, End> = BTreeMap::new();
    for node in nodes(&ways) {
        let junction = node.len() > 2;
        let mut free = node;
        loop {
            // (how straight, the two ends' places in `free`)
            let mut best: Option<(f64, usize, usize)> = None;
            for i in 0..free.len() {
                for j in i + 1..free.len() {
                    let (a, b) = (&ways[free[i].0], &ways[free[j].0]);
                    let alike = a.kind == b.kind && a.width == b.width;
                    if free[i].0 == free[j].0 || !alike || a.pinned || b.pinned {
                        continue;
                    }
                    // Straight on is out of one end and into the other.
                    let straight = -dot(a.end(free[i].1).1, b.end(free[j].1).1);
                    let sharpest = if junction {
                        THROUGH_COS
                    } else if a.kind == SurfaceKind::Road {
                        REVERSE_COS
                    } else {
                        ROAD_TURN_COS
                    };
                    if straight >= sharpest && best.is_none_or(|(known, ..)| straight > known) {
                        best = Some((straight, i, j));
                    }
                }
            }
            let Some((_, i, j)) = best else { break };
            link.insert(free[i], free[j]);
            link.insert(free[j], free[i]);
            free.remove(j);
            free.remove(i);
        }
    }
    if link.is_empty() {
        return ways;
    }
    // Follow each chain from an end nothing is joined to. A ring of ways,
    // every end joined, is left as it was: a stroke has two ends.
    let mut used = vec![false; ways.len()];
    let mut chains: Vec<Vec<End>> = Vec::new();
    for start in 0..ways.len() {
        for from_last in [false, true] {
            if used[start] || link.contains_key(&(start, from_last)) {
                continue;
            }
            // Each way of the chain, with the end the walk entered it by.
            let mut chain = vec![(start, from_last)];
            used[start] = true;
            let mut leaving = (start, !from_last);
            while let Some(&(next, entered)) = link.get(&leaving) {
                if used[next] {
                    break;
                }
                used[next] = true;
                chain.push((next, entered));
                leaving = (next, !entered);
            }
            chains.push(chain);
        }
    }
    let mut pieces: Vec<Option<Way>> = ways.into_iter().map(Some).collect();
    let mut out: Vec<Way> = Vec::new();
    for chain in chains {
        let piece = |way: usize| pieces[way].as_ref().expect("each way is in one chain");
        let mut points: Vec<Point> = Vec::new();
        for &(way, entered_last) in &chain {
            let mut run = piece(way).points.clone();
            if entered_last {
                run.reverse();
            }
            // The shared point is the earlier way's.
            let skip = usize::from(!points.is_empty());
            points.extend(run.into_iter().skip(skip));
        }
        points.dedup();
        let first = piece(chain[0].0);
        let welded = Way {
            kind: first.kind,
            width: first.width,
            points,
            slot: chain
                .iter()
                .map(|(way, _)| piece(*way).slot)
                .min()
                .expect("a chain has a way"),
            pieces: chain
                .iter()
                .flat_map(|(way, _)| piece(*way).pieces.clone())
                .collect(),
            pinned: false,
        };
        if chain.len() > 1 && welded.shape().is_ok() {
            for (way, _) in &chain {
                pieces[*way] = None;
            }
            out.push(welded);
        }
        // A line the shared centreline refuses stays as its pieces.
    }
    out.extend(pieces.into_iter().flatten());
    out.sort_by_key(|way| way.slot);
    out
}

/// Ends the weld left that meet at a corner: unlike ways, which cannot be
/// one stroke, and ways that meet at a junction without carrying straight
/// on. Each runs on past the point they shared far enough that the two
/// centrelines cross. Two of one width that turn no more than a right angle
/// each run on to the point their outer edges meet at, where each covers
/// the other's end. Between a narrower way and a wider one:
///
/// - where the narrower one's square end fits inside the wider one's width
///   (it carries on within about 45° of straight), it runs back into the
///   wider road until its end is covered, and the wider road's end shows a
///   shoulder either side of it: the road narrows;
/// - otherwise the wider way runs on until the narrower one leaves through
///   its side: the corner is the wider road's, its own end is the corner's
///   outer edge, and the narrower one's end lies under it.
///
/// Returns the ends it moved. Every end at such a place moves, or none
/// does: an end left on the point the others ran on from would touch lines
/// that rounding has moved off it. Ends that only carry straight on from
/// each other keep their point.
fn corner(ways: &mut [Way]) -> Vec<End> {
    let mut moves: Vec<(End, Point)> = Vec::new();
    for node in nodes(ways) {
        // A pinned way keeps its point, and so do the ends that share it.
        if node.iter().any(|(way, _)| ways[*way].pinned) {
            continue;
        }
        // Where the lines of two ends cross, as a distance on from the first.
        let crossing = |from: End, to: End| {
            let ((at, out_of), (q, heading)) = (ways[from.0].end(from.1), ways[to.0].end(to.1));
            let sin = cross(out_of, heading);
            (to.0 != from.0 && sin.abs() >= SQUARE_SIN)
                .then(|| (cross(sub(q, at), heading) / sin, sin.abs()))
        };
        // Whether the narrower end `from` fits inside the wider end `to`,
        // and then how far past their crossing it runs to be covered.
        let inside = |from: End, to: End| {
            let (a, b) = (ways[from.0].half(), ways[to.0].half());
            // Straight on is out of the wider end and into the narrower.
            let cos = -dot(ways[from.0].end(from.1).1, ways[to.0].end(to.1).1);
            let sin = cross(ways[from.0].end(from.1).1, ways[to.0].end(to.1).1).abs();
            (a < b && cos > 0.0 && a / cos + OVERSHOOT_M <= b).then(|| a * sin / cos)
        };
        // How far on from where it stands an end runs, before any wider
        // road is run over it: measured from where the lines cross, so an
        // end already run on is not run on again.
        let own = |from: End| {
            let mut run_on: Option<f64> = None;
            for &to in &node {
                let Some((crossing, sin)) = crossing(from, to) else {
                    continue;
                };
                let longest = 2.0 * ways[from.0].half().max(ways[to.0].half());
                // Far enough that the two lines cross inside both run-ons,
                // however shallow the angle between them.
                let least = OVERSHOOT_M.max(3.0 * ROUNDING_M / sin).min(longest);
                let (a, b) = (ways[from.0].half(), ways[to.0].half());
                let cos = -dot(ways[from.0].end(from.1).1, ways[to.0].end(to.1).1);
                let past = if let Some(depth) = inside(from, to) {
                    depth + OVERSHOOT_M
                } else if a == b && cos >= 0.0 {
                    // To where the two outer edges meet.
                    least.max(a * (1.0 - cos) / sin)
                } else {
                    least
                };
                run_on = Some(run_on.unwrap_or(f64::MIN).max(crossing + past));
            }
            run_on
        };
        for &from in &node {
            let Some(mut run_on) = own(from) else {
                continue;
            };
            let (at, out_of) = ways[from.0].end(from.1);
            let b = ways[from.0].half();
            let normal = [-out_of[1], out_of[0]];
            // Over every narrower end here that does not fit inside this one.
            for &to in &node {
                let a = ways[to.0].half();
                if to.0 == from.0 || a >= b || inside(to, from).is_some() {
                    continue;
                }
                let Some(theirs) = own(to) else { continue };
                let (q, heading) = ways[to.0].end(to.1);
                let face = add(q, scale(heading, theirs.max(0.0)));
                let body = scale(heading, -1.0);
                // Each edge of the narrower way, from its end back along its
                // body: the farthest on it comes while within this width.
                for side in [-1.0, 1.0] {
                    let start = add(face, scale([-heading[1], heading[0]], a * side));
                    let (aside, along) = (dot(sub(start, at), normal), dot(sub(start, at), out_of));
                    let (drift, gain) = (dot(body, normal), dot(body, out_of));
                    // Where the edge leaves this way's width, if it does.
                    let leaves = [-b, b]
                        .into_iter()
                        .filter(|_| drift != 0.0)
                        .map(|edge| (edge - aside) / drift)
                        .filter(|run| *run > 0.0)
                        .fold(f64::INFINITY, f64::min);
                    if aside.abs() <= b {
                        run_on = run_on.max(along + OVERSHOOT_M);
                        if leaves.is_finite() {
                            run_on = run_on.max(along + gain * leaves + OVERSHOOT_M);
                        }
                    }
                }
            }
            let longest = 3.0 * b;
            if run_on > REACHED_M {
                moves.push((from, round_cm(add(at, scale(out_of, run_on.min(longest))))));
            }
        }
    }
    let settled = moves.iter().map(|(end, _)| *end).collect();
    apply(ways, moves);
    settled
}

/// A way that ends on the map's edge at a slant turns square to the edge
/// before it, so its flat end lies along the edge and none of it shows.
fn gate(ways: &mut [Way], size: [f64; 2]) {
    for way in ways.iter_mut().filter(|way| !way.pinned) {
        for last in [false, true] {
            let (at, out_of) = way.end(last);
            // The edge it stands on and that edge's outward normal.
            let normal = [
                (at[0] <= 0.0, [-1.0, 0.0]),
                (at[0] >= size[0], [1.0, 0.0]),
                (at[1] <= 0.0, [0.0, -1.0]),
                (at[1] >= size[1], [0.0, 1.0]),
            ]
            .into_iter()
            .find_map(|(on, normal)| (on && dot(out_of, normal) > 0.0).then_some(normal));
            let Some(normal) = normal else { continue };
            // Square enough that no corner of its end shows inside the map.
            if way.half() * cross(out_of, normal).abs() <= ROUNDING_M {
                continue;
            }
            // How far in from the edge the point before the end stands.
            let room = dot(sub(at, way.before(last)), normal);
            let depth = (GATE_WIDTHS * way.width).min(room / 2.0);
            if depth < SHORTEST_RUN_M {
                continue;
            }
            let turn = round_cm(sub(at, scale(normal, depth)));
            let mut gated = way.points.clone();
            gated.insert(if last { gated.len() - 1 } else { 1 }, turn);
            let original = core::mem::replace(&mut way.points, gated);
            if way.shape().is_err() {
                way.points = original;
            }
        }
    }
}

/// An end that touches another carriageway and has not clearly passed its
/// middle runs on to just past it. A rounded bend carries the middle off the
/// authored run an end was laid to; and an end laid exactly on a middle
/// would touch a line that any later change to that road moves off it.
/// A way wider than the one it meets, whose end the narrower one would not
/// cover there (it comes in at a slant), runs on past that one's far edge
/// instead: it crosses the narrower road whole and its end is its own.
/// Nothing is cut back. Ends that share their point with another end, and
/// the ends in `settled`, are left where their corner put them.
fn snap(ways: &mut [Way], size: [f64; 2], settled: &[End]) {
    let paving = Paving::new(ways, size);
    let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
    let lone: Vec<End> = nodes(ways)
        .into_iter()
        .filter(|node| node.len() == 1)
        .flatten()
        .collect();
    let mut moves: Vec<(End, Point)> = Vec::new();
    for (way, last) in lone {
        {
            let (at, out_of) = ways[way].end(last);
            if !on_map(at) || ways[way].pinned || settled.contains(&(way, last)) {
                continue;
            }
            // The nearest crossing of its own line with a carriageway whose
            // width it touches: how far along from the end, signed, and how
            // far past the crossing the end belongs.
            let half = ways[way].half();
            let mut nearest: Option<(f64, f64)> = None;
            let reach = paving.widest + ways[way].half() + NEAR_M;
            let (from, to) = (sub(at, scale(out_of, reach)), add(at, scale(out_of, reach)));
            paving.grid.any(segment_bounds(from, to, 0.0), |item| {
                let (other, c, d) = paving.stretches[item as usize];
                if other == way || segment_distance(c, d, at) > ways[other].half() + NEAR_M {
                    return false;
                }
                let Some((t, _)) = segment_crossing(from, to, c, d) else {
                    return false;
                };
                let along = (2.0 * t - 1.0) * reach;
                // A road nearly alongside is one it already stands in, or
                // none of its own.
                let run = scale(sub(d, c), 1.0 / distance(c, d));
                let (cos, sin) = (dot(out_of, run).abs(), cross(out_of, run).abs());
                let meets = if segment_distance(c, d, at) <= ways[other].half() {
                    sin >= SQUARE_SIN
                } else {
                    cos <= ALONGSIDE_COS
                };
                if meets && nearest.is_none_or(|(known, _)| along.abs() < known.abs()) {
                    nearest = Some((along, ways[other].half()));
                }
                false
            });
            let Some((along, other_half)) = nearest else {
                continue;
            };
            // Just past the middle, unless it is already.
            let to_middle = (along > -ON_MIDDLE_M).then_some(along + OVERSHOOT_M);
            let stands = add(at, scale(out_of, to_middle.unwrap_or(0.0)));
            let run_on = if other_half < half && !paving.hides(stands, out_of, way) {
                // Past everything narrower that it stands on.
                paving
                    .narrower_past(at, out_of, half, way)
                    .map(|past| past + OVERSHOOT_M)
            } else {
                to_middle
            };
            if let Some(run_on) = run_on.filter(|run_on| *run_on > REACHED_M) {
                moves.push(((way, last), round_cm(add(at, scale(out_of, run_on)))));
            }
        }
    }
    apply(ways, moves);
}

/// Move each end, unless the shared centreline refuses the way it makes.
fn apply(ways: &mut [Way], moves: Vec<(End, Point)>) {
    for ((way, last), to) in moves {
        let at = ways[way].at(last);
        let original = core::mem::replace(&mut ways[way].points[at], to);
        if ways[way].shape().is_err() {
            ways[way].points[at] = original;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: [f64; 2] = [1000.0, 1000.0];

    fn way(kind: SurfaceKind, width: f64, points: &[Point]) -> SurfaceArea {
        SurfaceArea {
            kind,
            shape: GroundShape::stroke(points.to_vec(), width).unwrap(),
        }
    }

    fn lines(surfaces: &[SurfaceArea]) -> Vec<Vec<Point>> {
        surfaces
            .iter()
            .map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.control_points().to_vec(),
                GroundShape::Polygon { ring } => ring.clone(),
            })
            .collect()
    }

    /// Whether every point across the square end at `at` (the way running
    /// out of it along `out_of`, `half` wide either side) is paved by a way
    /// of `closed` other than `own`.
    fn covered(closed: &[SurfaceArea], own: usize, at: Point, out_of: Point, half: f64) -> bool {
        [-1.0, -0.5, 0.0, 0.5, 1.0].into_iter().all(|share| {
            let p = add(at, scale([-out_of[1], out_of[0]], half * share));
            closed
                .iter()
                .enumerate()
                .any(|(index, area)| index != own && area.shape.contains(p, FLUSH_M))
        })
    }

    #[test]
    fn two_streets_that_meet_end_to_end_become_one_street_round_the_corner() {
        let closed = close(
            vec![
                way(SurfaceKind::Road, 7.0, &[[100.0, 100.0], [200.0, 100.0]]),
                way(SurfaceKind::Road, 7.0, &[[200.0, 100.0], [200.0, 220.0]]),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed),
            [vec![[100.0, 100.0], [200.0, 100.0], [200.0, 220.0]]]
        );
        // The outside of the bend is road: no square bite at the corner.
        assert!(closed[0].shape.contains([202.0, 98.0], 0.0));
    }

    #[test]
    fn corner_streets_that_ran_past_each_other_are_brought_to_one_point() {
        // Each stops a little past the other's middle, as two streets laid
        // to meet at a corner do: neither covers the other's end.
        let closed = close(
            vec![
                way(SurfaceKind::Road, 7.0, &[[100.0, 100.0], [200.5, 100.0]]),
                way(SurfaceKind::Road, 7.0, &[[200.0, 99.5], [200.0, 220.0]]),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed),
            [vec![[100.0, 100.0], [200.0, 100.0], [200.0, 220.0]]]
        );
    }

    #[test]
    fn a_side_road_that_stops_short_runs_on_to_just_past_the_middle() {
        // It stops at the through road's near edge.
        let closed = close(
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[100.0, 300.0], [500.0, 300.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[300.0, 100.0], [300.0, 296.0]],
                ),
            ],
            SIZE,
            &[],
        );
        let track = &lines(&closed)[1];
        assert_eq!(track[0], [300.0, 100.0]);
        assert_eq!(track[1], [300.0, 300.0 + OVERSHOOT_M]);
        assert!(covered(&closed, 1, track[1], [0.0, 1.0], 2.0));
        // The through road is not touched.
        assert_eq!(lines(&closed)[0], [[100.0, 300.0], [500.0, 300.0]]);
    }

    #[test]
    fn a_road_that_already_ends_under_the_one_it_joins_is_left_alone() {
        let surfaces = vec![
            way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[100.0, 300.0], [500.0, 300.0]],
            ),
            way(
                SurfaceKind::DirtTrack,
                4.0,
                &[[300.0, 100.0], [300.0, 301.5]],
            ),
        ];
        let closed = close(surfaces.clone(), SIZE, &[]);
        assert_eq!(lines(&closed), lines(&surfaces));
    }

    #[test]
    fn where_unlike_roads_meet_at_a_corner_the_wider_runs_over_the_narrower() {
        for (turn, name) in [(60.0_f64, "shallow"), (90.0, "square"), (125.0, "sharp")] {
            // A road comes east to (400, 400); a track leaves it there,
            // turning `turn` degrees to the left.
            let leaving = [turn.to_radians().cos(), turn.to_radians().sin()];
            let far = round_cm(add([400.0, 400.0], scale(leaving, 150.0)));
            let closed = close(
                vec![
                    way(
                        SurfaceKind::CountryRoad,
                        8.0,
                        &[[200.0, 400.0], [400.0, 400.0]],
                    ),
                    way(SurfaceKind::DirtTrack, 4.0, &[[400.0, 400.0], far]),
                ],
                SIZE,
                &[],
            );
            let (road, track) = (&lines(&closed)[0], &lines(&closed)[1]);
            // The track's end is under the road, wherever it now stands.
            let back = scale(leaving, -1.0);
            assert!(
                covered(&closed, 1, track[0], back, 2.0),
                "{name}: {track:?}"
            );
            // The road's own end stands clear past the track: none of its
            // face is under the track, so the corner has no bite in it.
            assert!(
                road[1][0] > 400.0 && road[1][1] == 400.0,
                "{name}: {road:?}"
            );
            for share in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                let p = [road[1][0], 400.0 + 4.0 * share];
                assert!(
                    !closed[1].shape.contains(p, 0.0),
                    "{name}: {road:?} at {p:?}"
                );
            }
        }
    }

    #[test]
    fn a_narrower_road_that_carries_nearly_straight_on_ends_inside_the_wider() {
        let closed = close(
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[200.0, 400.0], [400.0, 400.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 400.0], [540.0, 430.0]],
                ),
            ],
            SIZE,
            &[],
        );
        let track = &lines(&closed)[1];
        let run = sub(track[0], track[1]);
        let back = scale(run, 1.0 / distance(track[0], track[1]));
        assert!(track[0][0] < 400.0, "{track:?}");
        assert!(covered(&closed, 1, track[0], back, 2.0), "{track:?}");
    }

    #[test]
    fn a_road_that_leaves_the_map_at_a_slant_turns_square_to_the_edge() {
        let closed = close(
            vec![way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[500.0, 500.0], [700.0, 1000.0]],
            )],
            SIZE,
            &[],
        );
        let road = &lines(&closed)[0];
        assert_eq!(road, &[[500.0, 500.0], [700.0, 984.0], [700.0, 1000.0]]);
        // Both corners of its square end lie on the edge.
        let GroundShape::Stroke { centerline, .. } = &closed[0].shape else {
            unreachable!()
        };
        let samples = centerline.samples();
        assert_eq!(samples[samples.len() - 2][0], 700.0);
        // A road already square to the edge is left alone.
        let square = vec![way(
            SurfaceKind::CountryRoad,
            8.0,
            &[[0.0, 300.0], [400.0, 300.0]],
        )];
        assert_eq!(lines(&close(square.clone(), SIZE, &[])), lines(&square));
    }

    #[test]
    fn a_road_that_runs_out_past_its_settlement_is_cut_back_to_it() {
        let ground: Vec<Point> = vec![
            [300.0, 300.0],
            [500.0, 300.0],
            [500.0, 500.0],
            [300.0, 500.0],
        ];
        let closed = close(
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[0.0, 400.0], [400.0, 400.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 400.0], [760.0, 400.0]],
                ),
            ],
            SIZE,
            &[&ground],
        );
        let track = &lines(&closed)[1];
        let end = track[track.len() - 1];
        assert!(
            end[0] >= 500.0 && end[0] <= 500.0 + 2.0 + TRIM_MARGIN_M + TRIM_STEP_M,
            "{track:?}"
        );
        // One that stops on the settlement's ground, or by another road, stays.
        let inside = vec![way(
            SurfaceKind::DirtTrack,
            4.0,
            &[[350.0, 350.0], [450.0, 450.0]],
        )];
        assert_eq!(
            lines(&close(inside.clone(), SIZE, &[&ground])),
            lines(&inside)
        );
    }

    #[test]
    fn roads_that_meet_at_a_point_without_carrying_straight_on_still_cross() {
        // Two country roads make a corner and a track comes to its point
        // from outside the turn. One road round that corner would leave the
        // track touching its bend at a tangent: no crossing for the plan's
        // road graph to join them by.
        let closed = close(
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[2055.11, 1474.87], [2223.49, 1420.17], [2436.85, 1350.85]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[1818.82, 1757.28], [2020.28, 1514.94], [2055.11, 1474.87]],
                ),
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[2014.13, 1850.66], [2047.17, 1554.66], [2055.11, 1474.87]],
                ),
            ],
            [6000.0, 6000.0],
            &[],
        );
        assert_eq!(closed.len(), 3);
        let samples: Vec<&[Point]> = closed
            .iter()
            .map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.samples(),
                GroundShape::Polygon { .. } => unreachable!(),
            })
            .collect();
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            let crosses = samples[a].windows(2).any(|run| {
                samples[b]
                    .windows(2)
                    .any(|other| segment_crossing(run[0], run[1], other[0], other[1]).is_some())
            });
            assert!(
                crosses,
                "ways {a} and {b} no longer cross: {:?}",
                lines(&closed)
            );
        }
    }

    #[test]
    fn closing_twice_changes_nothing_more() {
        let surfaces = vec![
            way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[100.0, 300.0], [500.0, 300.0]],
            ),
            way(
                SurfaceKind::DirtTrack,
                4.0,
                &[[300.0, 100.0], [300.0, 296.0]],
            ),
            way(SurfaceKind::Road, 7.0, &[[500.0, 300.0], [640.0, 380.0]]),
            way(SurfaceKind::Road, 7.0, &[[640.0, 380.0], [640.0, 520.0]]),
            way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[500.0, 500.0], [700.0, 1000.0]],
            ),
        ];
        let once = close(surfaces, SIZE, &[]);
        let twice = close(once.clone(), SIZE, &[]);
        assert_eq!(lines(&twice), lines(&once));
    }
}
