//! Road ends. A stroke is cut square across its first and last point
//! (`contract::ground`), so the flat face of an end shows wherever no other
//! paving covers it. Both generation steps hand their carriageways to
//! [`close`], which leaves every end one of these:
//!
//! - **part of a through road.** Carriageways of one kind and width that meet
//!   end to end become one stroke through the point they shared, round
//!   whatever corner they make there: the outside of a bend, a switchback
//!   included, is the centreline's own round one, never a cut heel. At a
//!   junction the straightest pair is the road through it;
//! - **under the road it joins, and square to it.** An end that touches
//!   another carriageway stops just past that carriageway's middle, where
//!   its face lies inside the other's width. One that comes in at more of a
//!   slant than 30° off square leaves its line a little before the road and
//!   curves round to meet it square, so no acute fork is left and no corner
//!   of its face shows past the road's far edge;
//! - **under a road that crosses it, where it changes width.** A road is
//!   one width and one kind from junction to junction. Where a wider way
//!   and a narrower one meet end to end, one of the two is carried through
//!   the corner, along the other's line, to the first carriageway that
//!   crosses that line and covers the wider way's whole flat end: there the
//!   wider ends and the narrower starts, on one point just past the
//!   crossing road's middle. Whichever of the two stretches is shorter is
//!   the one that changes; where nothing crosses it so, it changes to its
//!   far end;
//! - **square on the map's edge**, where a road leaves the map;
//! - **at the last block it serves.** A road that ran on past it into open
//!   country is cut back to it.
//!
//! The steps, in order: `trim` (roads that run out past their settlement),
//! `undouble` (a street drawn beside another), `weld`, `tidy` (turns a few
//! metres from an end), `meet` (corners whose ends fell short or ran past),
//! `tidy` again, `weld` again, `carry` (unlike roads that meet end to end), `weld` once
//! more, `gate` (the map's edge) and `snap` (ends that join a road).
//!
//! Not closed yet, and counted by `tests/road_ends.rs`: a few junctions
//! where three ends stand a few metres apart without sharing a point, a
//! wider road that ends at a slant on a narrower one with no room to turn,
//! and a corner of unlike roads whose mending would have lost a joint.
//!
//! The plan's road graph joins two roads where their centrelines cross
//! (`layout::measure`), so no step here may leave an end touching a line
//! that a centimetre's rounding has moved off it: ends that share a point
//! keep it, or run on past the other's middle far enough to cross.
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

/// An end stops this far past the middle of the road it joins, so the two
/// centrelines cross whatever a centimetre of rounding did to either.
const OVERSHOOT_M: f64 = 0.25;
/// Two ends that turn back on each other this exactly (about 5°) are not
/// welded: the shared centreline has no bend to give them.
const REVERSE_COS: f64 = -0.996;
/// A face along another carriageway's edge is covered by it.
const FLUSH_M: f64 = 0.02;
/// An end is moved no less than this.
const REACHED_M: f64 = 0.005;
/// An end this near a road's middle stands on it, and has not passed it.
const ON_MIDDLE_M: f64 = 0.1;
/// How far a centimetre's rounding may move a point.
const ROUNDING_M: f64 = 0.005;
/// Lines this near parallel (about 1°) have no corner between them.
const SQUARE_SIN: f64 = 0.02;
const TRIM_STEP_M: f64 = 2.0;

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
    /// The stroke `points` make at `width`: rounded once, when they change.
    shape: GroundShape,
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

    /// Lay it along `points` instead, unless the shared centreline refuses
    /// the line they make: then it stays as it was. Says whether it moved.
    fn relay(&mut self, points: Vec<Point>) -> bool {
        match GroundShape::stroke(points.clone(), self.width) {
            Ok(shape) => {
                self.points = points;
                self.shape = shape;
                true
            }
            Err(_) => false,
        }
    }

    /// Its rounded centreline.
    fn samples(&self) -> &[Point] {
        match &self.shape {
            GroundShape::Stroke { centerline, .. } => centerline.samples(),
            GroundShape::Polygon { .. } => unreachable!("a way is a stroke"),
        }
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
/// they cross after, are one way, or both cross a third way at the same
/// junction; where a change would break that (a street that touched a bend
/// at a tangent, say), the ways concerned are left exactly as they were
/// laid and the rest are closed round them.
pub fn close(
    policy: &crate::layout::JointPolicy,
    surfaces: Vec<SurfaceArea>,
    size: [f64; 2],
    grounds: &[&[Point]],
) -> Vec<SurfaceArea> {
    let mut laid: Vec<Way> = Vec::new();
    let mut kept: Vec<Option<SurfaceArea>> = Vec::with_capacity(surfaces.len());
    for (slot, area) in surfaces.into_iter().enumerate() {
        let road = match &area.shape {
            GroundShape::Stroke {
                centerline,
                width_m,
            } if area.kind.is_road() => Some((centerline.control_points().to_vec(), *width_m)),
            _ => None,
        };
        match road {
            Some((points, width)) => {
                laid.push(Way {
                    kind: area.kind,
                    width,
                    points,
                    slot,
                    pieces: vec![slot],
                    pinned: false,
                    shape: area.shape,
                });
                kept.push(None);
            }
            None => kept.push(Some(area)),
        }
    }
    let slots = kept.len();
    let joined_before = joined(policy, &laid, size);
    let mut ways = closed(policy, &laid, size, grounds);
    // A few rounds settle it: each pins the ways of the joints the last lost.
    for _ in 0..policy.pin_rounds {
        let broken = lost(policy, &joined_before, &ways, size, slots);
        if broken.is_empty() {
            break;
        }
        for way in &mut laid {
            way.pinned |= broken.contains(&way.slot);
        }
        ways = closed(policy, &laid, size, grounds);
    }
    if !lost(policy, &joined_before, &ways, size, slots).is_empty() {
        ways = laid;
    }
    for way in ways {
        kept[way.slot] = Some(SurfaceArea {
            kind: way.kind,
            shape: way.shape,
        });
    }
    kept.into_iter().flatten().collect()
}

/// Every step, on a copy of the ways as they were laid. A pinned way is
/// left out of each.
fn closed(
    policy: &crate::layout::JointPolicy,
    laid: &[Way],
    size: [f64; 2],
    grounds: &[&[Point]],
) -> Vec<Way> {
    let mut ways = laid.to_vec();
    trim(policy, &mut ways, size, grounds);
    undouble(policy, &mut ways, size);
    // Welded first, so a corner's two ends are not taken for ends that
    // join the pieces they are made of; and again once ends have met.
    let mut ways = weld(policy, ways);
    tidy(policy, &mut ways);
    meet(policy, &mut ways, size);
    tidy(policy, &mut ways);
    // Unlike ways at a corner are carried to where their width may change,
    // and what that leaves alike and end to end is one way.
    let mut ways = weld(policy, carry(policy, weld(policy, ways), size));
    gate(policy, &mut ways, size);
    snap(policy, &mut ways, size);
    ways
}

/// The pairs of laid ways whose centrelines cross, each by its slot, lower
/// first, with where: what the plan's road graph joins (`layout::measure`),
/// by the same crossing test. Pieces of one welded way are joined by being
/// one way.
fn joined(
    policy: &crate::layout::JointPolicy,
    ways: &[Way],
    size: [f64; 2],
) -> BTreeMap<(usize, usize), Vec<Point>> {
    let paving = Paving::new(policy, ways, size);
    let mut pairs: BTreeMap<(usize, usize), Vec<Point>> = BTreeMap::new();
    for (index, (way, a, b)) in paving.stretches.iter().enumerate() {
        paving.grid.any(segment_bounds(*a, *b, 0.0), |item| {
            let (other, c, d) = paving.stretches[item as usize];
            if item as usize <= index || other == *way {
                return false;
            }
            if let Some((share, _)) = segment_crossing(*a, *b, c, d) {
                let (low, high) = (
                    ways[*way].slot.min(ways[other].slot),
                    ways[*way].slot.max(ways[other].slot),
                );
                let at = add(*a, scale(sub(*b, *a), share));
                let places = pairs.entry((low, high)).or_default();
                if places.last() != Some(&at) {
                    places.push(at);
                }
            }
            false
        });
    }
    pairs
}

/// The slots of every laid way that is part of a joint `before` has and
/// `closed` has lost, with the slots of the ways welded to them. A joint is
/// kept if its two ways still cross, are one way, or both cross one third
/// way within `policy.junction_m` of where they crossed each other: three roads
/// that shared a point are still one junction when two of them end on the
/// third. A laid way whose line two closed ways now share ([`carry`]) keeps
/// a joint that either of them keeps.
fn lost(
    policy: &crate::layout::JointPolicy,
    before: &BTreeMap<(usize, usize), Vec<Point>>,
    closed: &[Way],
    size: [f64; 2],
    slots: usize,
) -> BTreeSet<usize> {
    // The closed ways each laid slot is now part of, by those ways' slots.
    let mut owners: Vec<Vec<usize>> = vec![Vec::new(); slots];
    for way in closed {
        for piece in &way.pieces {
            owners[*piece].push(way.slot);
        }
    }
    let after = joined(policy, closed, size);
    // What each closed way crosses, and where.
    let mut crossings: BTreeMap<usize, Vec<(usize, Point)>> = BTreeMap::new();
    for (&(low, high), places) in &after {
        for at in places {
            crossings.entry(low).or_default().push((high, *at));
            crossings.entry(high).or_default().push((low, *at));
        }
    }
    let near = |way: usize, at: Point| -> BTreeSet<usize> {
        crossings
            .get(&way)
            .into_iter()
            .flatten()
            .filter(|(_, place)| distance(*place, at) <= policy.junction_m)
            .map(|(other, _)| *other)
            .collect()
    };
    let mut broken: BTreeSet<usize> = BTreeSet::new();
    for (&(a, b), places) in before {
        let kept = owners[a].iter().any(|&i| {
            owners[b].iter().any(|&j| {
                i == j
                    || after.contains_key(&(i.min(j), i.max(j)))
                    || places
                        .iter()
                        .all(|at| near(i, *at).intersection(&near(j, *at)).next().is_some())
            })
        });
        if !kept {
            broken.extend(owners[a].iter().chain(&owners[b]).copied());
        }
    }
    closed
        .iter()
        .filter(|way| broken.contains(&way.slot))
        .flat_map(|way| way.pieces.iter().copied())
        .collect()
}

/// Every end of every way, grouped where they meet.
fn nodes(policy: &crate::layout::JointPolicy, ways: &[Way]) -> Vec<Vec<End>> {
    // Cells cover the edited meeting tolerance, so neighbouring cells contain every eligible end.
    // The initial one-metre grid preserves default grouping and ordering.
    let cell_m = policy.meet_m.max(1.0);
    let cell = |p: Point| {
        [
            libm::floor(p[0] / cell_m) as i64,
            libm::floor(p[1] / cell_m) as i64,
        ]
    };
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
        // Everything reachable from this end by hops of `policy.meet_m`.
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
                            && distance(ways[other.0].end(other.1).0, at) <= policy.meet_m
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
struct Paving<'a> {
    policy: &'a crate::layout::JointPolicy,
    shapes: Vec<&'a GroundShape>,
    /// Each way's half width.
    halves: Vec<f64>,
    /// (way, one rounded stretch of it)
    stretches: Vec<(usize, Point, Point)>,
    grid: Grid,
    widest: f64,
}

impl<'a> Paving<'a> {
    fn new(policy: &'a crate::layout::JointPolicy, ways: &'a [Way], size: [f64; 2]) -> Self {
        let shapes: Vec<&GroundShape> = ways.iter().map(|way| &way.shape).collect();
        let mut stretches = Vec::new();
        let mut grid = Grid::new(size, 64.0);
        for (way, line) in ways.iter().enumerate() {
            for pair in line.samples().windows(2) {
                grid.insert(
                    segment_bounds(pair[0], pair[1], ways[way].half() + policy.near_m),
                    stretches.len() as u32,
                );
                stretches.push((way, pair[0], pair[1]));
            }
        }
        Self {
            policy,
            shapes,
            halves: ways.iter().map(Way::half).collect(),
            stretches,
            grid,
            widest: ways.iter().map(Way::half).fold(0.0, f64::max),
        }
    }

    /// Whether a way other than `own` and `mate`, at least `half` wide
    /// either side, has its edge within `self.policy.near_m` of `p`.
    fn touched(&self, p: Point, own: usize, mate: usize, half: f64) -> bool {
        self.grid.any([p[0], p[1], p[0], p[1]], |item| {
            let way = self.stretches[item as usize].0;
            way != own
                && way != mate
                && self.halves[way] >= half
                && self.shapes[way].contains(p, self.policy.near_m)
        })
    }

    /// Whether a way that is none of `ends`' has its edge within `self.policy.near_m`
    /// of `p`.
    fn passed(&self, p: Point, ends: &[End]) -> bool {
        self.grid.any([p[0], p[1], p[0], p[1]], |item| {
            let way = self.stretches[item as usize].0;
            ends.iter().all(|end| end.0 != way) && self.shapes[way].contains(p, self.policy.near_m)
        })
    }

    /// Whether the square end of way `own` would be covered by other ways
    /// if it stood at `at`, facing along `out_of`.
    fn hides(&self, at: Point, out_of: Point, own: usize) -> bool {
        self.covers(at, out_of, self.halves[own], &[own])
    }

    /// Whether a square end `half` wide either side, standing at `at` and
    /// facing along `out_of`, would be covered by ways that are none of
    /// `skip`.
    fn covers(&self, at: Point, out_of: Point, half: f64, skip: &[usize]) -> bool {
        let across = scale([-out_of[1], out_of[0]], half);
        [-1.0, -0.5, 0.0, 0.5, 1.0].into_iter().all(|share| {
            let p = add(at, scale(across, share));
            self.grid.any([p[0], p[1], p[0], p[1]], |item| {
                let way = self.stretches[item as usize].0;
                !skip.contains(&way) && self.shapes[way].contains(p, FLUSH_M)
            })
        })
    }

    /// Where the line of the way of `walked` may change width, walking it
    /// from that end, a corner it shares with way `mate`: how far along,
    /// and the stop. `mate` takes the line to there; the wider of the two
    /// is `wide` across.
    fn change(&self, ways: &[Way], walked: End, mate: usize, wide: f64) -> (f64, Stop) {
        let line = &ways[walked.0];
        // Its authored points and its rounded middle, from the corner.
        let (mut points, mut samples) = (line.points.clone(), line.samples().to_vec());
        if walked.1 {
            points.reverse();
            samples.reverse();
        }
        // The wider way ends past the crossing road's middle, whichever
        // side of it that way comes from.
        let ahead = if ways[mate].width > line.width {
            OVERSHOOT_M
        } else {
            -OVERSHOOT_M
        };
        let least = self.widest + self.policy.tail_spare_m;
        let mut length = 0.0;
        for pair in samples.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            // What crosses this stretch, the nearest first.
            let mut crossings: Vec<(f64, usize)> = Vec::new();
            self.grid.any(segment_bounds(a, b, 0.0), |item| {
                let (way, c, d) = self.stretches[item as usize];
                if way != walked.0 && way != mate {
                    if let Some((share, _)) = segment_crossing(a, b, c, d) {
                        crossings.push((share, way));
                    }
                }
                false
            });
            crossings.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
            crossings.dedup();
            for (share, crossing) in crossings {
                let along = length + share * distance(a, b);
                let at = add(a, scale(sub(b, a), share));
                if along < least {
                    continue;
                }
                let skip = [walked.0, mate];
                if let Some((onward, rest)) = self.cut(&points, at, ahead, crossing, skip, wide) {
                    return (along, Stop::Under(onward, rest));
                }
            }
            length += distance(a, b);
        }
        (length, Stop::Whole(points[1..].to_vec()))
    }

    /// The line `points` cut in two where way `crossing` crosses it at
    /// `at`, if the end of the wider part, `wide` across, is covered there
    /// by ways that are none of `skip`: the points after the first up to
    /// the cut, and the points from the cut on. The cut is `ahead` of `at`
    /// along the line; behind it if `ahead` is negative, when the wider
    /// part is the one from the cut on.
    fn cut(
        &self,
        points: &[Point],
        at: Point,
        ahead: f64,
        crossing: usize,
        skip: [usize; 2],
        wide: f64,
    ) -> Option<(Vec<Point>, Vec<Point>)> {
        // The authored run `at` lies on: none where the line rounds a turn.
        let mut run = points
            .windows(2)
            .position(|run| segment_distance(run[0], run[1], at) <= 2.0 * ROUNDING_M)?;
        let mut along = distance(points[run], at) + ahead;
        if along < 0.0 {
            run = run.checked_sub(1)?;
            along += distance(points[run], points[run + 1]);
        } else if along > distance(points[run], points[run + 1]) {
            along -= distance(points[run], points[run + 1]);
            run += 1;
        }
        let (from, to) = (points[run], *points.get(run + 1)?);
        if !(0.0..=distance(from, to)).contains(&along) {
            return None;
        }
        let end = round_cm(add(from, scale(sub(to, from), along / distance(from, to))));
        let mut onward = points[1..=run].to_vec();
        let mut rest = points[run + 1..].to_vec();
        // A turn a step from the new ends is dropped, as `tidy` drops one.
        while onward
            .last()
            .is_some_and(|turn| distance(*turn, end) < self.policy.shortest_run_m)
        {
            onward.pop();
        }
        while rest.len() > 1 && distance(rest[0], end) < self.policy.shortest_run_m {
            rest.remove(0);
        }
        // The wider part's last run, into the cut.
        let before = if ahead > 0.0 {
            onward.last().copied().unwrap_or(points[0])
        } else {
            rest[0]
        };
        if distance(rest[0], end) < self.policy.shortest_run_m
            || distance(before, end) < self.policy.shortest_run_m
        {
            return None;
        }
        let out_of = scale(sub(end, before), 1.0 / distance(before, end));
        let crosses = self.grid.any(segment_bounds(before, end, 0.0), |item| {
            let (way, c, d) = self.stretches[item as usize];
            way == crossing && segment_crossing(before, end, c, d).is_some()
        });
        (crosses && self.covers(end, out_of, wide / 2.0, &skip)).then(|| {
            onward.push(end);
            rest.insert(0, end);
            (onward, rest)
        })
    }

    /// The point of way `other`'s rounded middle nearest `p`, and whether it
    /// lies within the way's length rather than at one of its two ends.
    fn nearest_on(&self, other: usize, p: Point) -> Option<(Point, bool)> {
        let reach = 4.0 * self.widest + 64.0;
        let mut best: Option<(f64, Point)> = None;
        let mut ends: Option<(Point, Point)> = None;
        for &(way, c, d) in &self.stretches {
            if way != other {
                continue;
            }
            ends = Some((ends.map_or(c, |(first, _)| first), d));
            if c == d || (c[0] - p[0]).abs().min((d[0] - p[0]).abs()) > reach + distance(c, d) {
                continue;
            }
            let run = sub(d, c);
            let share = (dot(sub(p, c), run) / dot(run, run)).clamp(0.0, 1.0);
            let foot = add(c, scale(run, share));
            let apart = distance(foot, p);
            if best.is_none_or(|(known, _)| apart < known) {
                best = Some((apart, foot));
            }
        }
        let ((_, foot), (first, end)) = (best?, ends?);
        Some((
            foot,
            distance(foot, first) > self.policy.meet_m && distance(foot, end) > self.policy.meet_m,
        ))
    }

    /// Whether the middle of a way other than `own` and `mate` crosses the
    /// run from `a` to `b` or ends on it.
    fn crossed(&self, a: Point, b: Point, own: usize, mate: usize) -> bool {
        self.grid.any(segment_bounds(a, b, 0.0), |item| {
            let (way, c, d) = self.stretches[item as usize];
            way != own
                && way != mate
                && (segment_crossing(a, b, c, d).is_some()
                    || segment_distance(a, b, c) <= self.policy.meet_m
                    || segment_distance(a, b, d) <= self.policy.meet_m)
        })
    }
}

/// A way that runs out past the last block on its line and stops in open
/// country, by no other carriageway and short of the map's edge, is cut
/// back to the last block it runs on or along, or to the last carriageway
/// that joins it if that comes first: a main street is laid to the end of
/// the ground a settlement may build on, and the settlement may not have
/// grown that far.
fn trim(
    policy: &crate::layout::JointPolicy,
    ways: &mut [Way],
    size: [f64; 2],
    grounds: &[&[Point]],
) {
    if ways.is_empty() || grounds.is_empty() {
        return;
    }
    let paving = Paving::new(policy, ways, size);
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
        let reach = line.half() + policy.trim_margin_m;
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
        let mut points = ways[way].points.clone();
        if last {
            points.truncate(points.len() - step);
        } else {
            points.drain(..step);
        }
        let at = if last { points.len() - 1 } else { 0 };
        points[at] = to;
        points.dedup();
        if points.len() >= 2 {
            ways[way].relay(points);
        }
    }
}

/// A way laid beside another, less than their two half widths off and
/// running the same way, is the same road drawn twice for as long as the
/// two overlap, and its end is a step in that road's edge. It is cut back
/// to the last carriageway that crosses it or ends on it, if one does
/// within `policy.beside_widths` of its end: there the two meet across that road.
fn undouble(policy: &crate::layout::JointPolicy, ways: &mut [Way], size: [f64; 2]) {
    // (the end, how many of its points go, where it then ends)
    let mut cuts: Vec<(End, usize, Option<Point>)> = Vec::new();
    {
        let paving = Paving::new(policy, ways, size);
        let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
        let lone = nodes(policy, ways)
            .into_iter()
            .filter(|node| node.len() == 1)
            .flatten();
        for (way, last) in lone {
            let (at, out_of) = ways[way].end(last);
            if ways[way].pinned || !on_map(at) || paving.hides(at, out_of, way) {
                continue;
            }
            // The way it lies beside.
            let mut beside: Option<usize> = None;
            paving.grid.any([at[0], at[1], at[0], at[1]], |item| {
                let (other, c, d) = paving.stretches[item as usize];
                if other == way || c == d {
                    return false;
                }
                let run = scale(sub(d, c), 1.0 / distance(c, d));
                let along = dot(sub(at, c), run);
                let off = cross(sub(at, c), run).abs();
                if dot(out_of, run).abs() >= policy.alongside_cos
                    && (0.0..=distance(c, d)).contains(&along)
                    && off < paving.halves[way] + paving.halves[other]
                {
                    beside = Some(other);
                }
                beside.is_some()
            });
            let Some(beside) = beside else { continue };
            // Back along its own line, run by run, to the first carriageway
            // that crosses it or ends on it.
            let limit = policy.beside_widths * ways[way].width;
            let points: Vec<Point> = if last {
                ways[way].points.iter().rev().copied().collect()
            } else {
                ways[way].points.clone()
            };
            let mut walked = 0.0;
            'runs: for (step, run) in points.windows(2).enumerate() {
                let (from, to) = (run[0], run[1]);
                let length = distance(from, to);
                let heading = scale(sub(to, from), 1.0 / length);
                // The nearest to `from` of what crosses this run.
                let mut first: Option<f64> = None;
                paving.grid.any(segment_bounds(from, to, 0.0), |item| {
                    let (other, c, d) = paving.stretches[item as usize];
                    if other == way || other == beside || c == d {
                        return false;
                    }
                    let run = scale(sub(d, c), 1.0 / distance(c, d));
                    if dot(heading, run).abs() > policy.alongside_cos {
                        return false;
                    }
                    let mut note = |along: f64| {
                        if first.is_none_or(|known| along < known) {
                            first = Some(along);
                        }
                    };
                    if let Some((share, _)) = segment_crossing(from, to, c, d) {
                        note(share * length);
                    }
                    for end in [c, d] {
                        if segment_distance(from, to, end) <= policy.meet_m {
                            note(dot(sub(end, from), heading).clamp(0.0, length));
                        }
                    }
                    false
                });
                if let Some(along) = first {
                    if walked + along > limit || (step == 0 && along < policy.shortest_run_m) {
                        break 'runs;
                    }
                    // On the next point of its own line, it ends there;
                    // otherwise just short of the crossing road's middle,
                    // from this side.
                    if length - along <= policy.meet_m {
                        cuts.push(((way, last), step + 1, None));
                    } else {
                        let stop = add(from, scale(heading, along - OVERSHOOT_M));
                        cuts.push(((way, last), step, Some(round_cm(stop))));
                    }
                    break 'runs;
                }
                walked += length;
                if walked > limit {
                    break;
                }
            }
        }
    }
    for ((way, last), step, to) in cuts {
        let mut points = ways[way].points.clone();
        if !last {
            points.reverse();
        }
        points.truncate(points.len() - step);
        if let Some(to) = to {
            *points.last_mut().expect("a way has points") = to;
        }
        if !last {
            points.reverse();
        }
        points.dedup();
        if points.len() >= 2 {
            ways[way].relay(points);
        }
    }
}

/// A way that turns within half a road's width or so of an end loses that turn:
/// the end's last run is then long enough to be moved, met or turned along.
/// (A street's link to the road it joins is a metre or two long.) A turn
/// another way ends on is kept.
fn tidy(policy: &crate::layout::JointPolicy, ways: &mut [Way]) {
    // Long enough for the widest road there is to turn along.
    let widest = ways.iter().map(Way::half).fold(0.0, f64::max);
    let cell = |p: Point| [libm::floor(p[0]) as i64, libm::floor(p[1]) as i64];
    let mut ends: BTreeMap<[i64; 2], Vec<Point>> = BTreeMap::new();
    for way in ways.iter() {
        for last in [false, true] {
            let at = way.end(last).0;
            ends.entry(cell(at)).or_default().push(at);
        }
    }
    let joined = |p: Point| {
        let [cx, cy] = cell(p);
        (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                ends.get(&[cx + dx, cy + dy])
                    .is_some_and(|ends| ends.iter().any(|end| distance(*end, p) <= policy.meet_m))
            })
        })
    };
    for way in ways.iter_mut() {
        for last in [false, true] {
            let (at, before) = (way.end(last).0, way.before(last));
            let short = distance(at, before) < widest + policy.tail_spare_m;
            if way.pinned || way.points.len() < 3 || !short || joined(before) {
                continue;
            }
            let mut points = way.points.clone();
            points.remove(if last { points.len() - 2 } else { 1 });
            way.relay(points);
        }
    }
}

/// Two ends that stop within each other's width, or a width or two past
/// each other, on lines that cross there, are a corner whose ways each fell
/// short of the other or ran past it. Both
/// are moved to where their lines cross, and so meet end to end. An end
/// that touches a third carriageway as wide as either is joining that one,
/// and is left; a narrower one they both end on is one they cross.
///
/// Two alike ways laid side by side, less than a width apart, whose ends
/// run past each other or face each other, are one road drawn twice: both
/// ends are moved to the point half way between them.
fn meet(policy: &crate::layout::JointPolicy, ways: &mut [Way], size: [f64; 2]) {
    if ways.is_empty() {
        return;
    }
    let paving = Paving::new(policy, ways, size);
    let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
    let lone: Vec<End> = nodes(policy, ways)
        .into_iter()
        .filter(|node| node.len() == 1)
        .flatten()
        .filter(|(way, last)| !ways[*way].pinned && on_map(ways[*way].end(*last).0))
        .collect();
    let mut cells: BTreeMap<[i64; 2], Vec<End>> = BTreeMap::new();
    // Two ends that may be a corner are no farther apart than one cell.
    let span = (2.0 + 4.0 * policy.stub_widths).max(2.0 * policy.beside_widths) * paving.widest;
    let cell = |p: Point| [0, 1].map(|k| libm::floor(p[k] / span) as i64);
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
        if from.0 == to.0 || sin.abs() < SQUARE_SIN {
            return None;
        }
        let at = add(p, scale(along, cross(sub(q, p), other) / sin));
        let fits = |way: &Way, last: bool, out_of: Point| {
            dot(sub(at, way.before(last)), out_of) >= policy.shortest_run_m
        };
        // An end may have fallen short by the two half widths, or run past
        // by a stub's length more.
        let near = |way: &Way, end: Point, out_of: Point| {
            let past = dot(sub(end, at), out_of);
            (-reach..=reach + policy.stub_widths * way.width).contains(&past)
        };
        // Nothing a third carriageway joins is cut off or moved away.
        (near(a, p, along)
            && near(b, q, other)
            && fits(a, from.1, along)
            && fits(b, to.1, other)
            && !paving.touched(p, from.0, to.0, a.half().min(b.half()))
            && !paving.touched(q, to.0, from.0, a.half().min(b.half()))
            && !paving.crossed(at, p, from.0, to.0)
            && !paving.crossed(at, q, to.0, from.0))
        .then_some(round_cm(at))
    };
    // Where two alike ways laid side by side, running past each other or
    // stopping short, become one: half way between their ends.
    let beside = |from: End, to: End| {
        let (a, b) = (&ways[from.0], &ways[to.0]);
        let ((p, along), (q, other)) = (a.end(from.1), b.end(to.1));
        let alike = a.kind == b.kind && a.width == b.width;
        if from.0 == to.0 || !alike || dot(along, other) > -policy.alongside_cos {
            return None;
        }
        let aside = cross(sub(q, p), along).abs();
        let past = -dot(sub(q, p), along);
        let at = scale(add(p, q), 0.5);
        // Each keeps a last run long enough that the step aside is slight.
        let fits = |way: &Way, last: bool, out_of: Point| {
            dot(sub(at, way.before(last)), out_of) >= (2.0 * aside).max(policy.shortest_run_m)
        };
        (aside <= a.width
            && (-a.width..=policy.beside_widths * a.width).contains(&past)
            && fits(a, from.1, along)
            && fits(b, to.1, other)
            && !paving.touched(p, from.0, to.0, 0.0)
            && !paving.touched(q, to.0, from.0, 0.0)
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
                        if let Some(at) = corner(from, to).or_else(|| beside(from, to)) {
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
/// the point they shared, round whatever corner they make there, a
/// switchback included: its outside is the centreline's own round bend.
/// Where more than two meet, the straightest pair is joined first: the road
/// through the junction. The others then end on that road, and [`snap`]
/// brings them to it square. Where a wider road ends at the junction too,
/// narrower ends are joined only if they carry nearly straight on.
fn weld(policy: &crate::layout::JointPolicy, ways: Vec<Way>) -> Vec<Way> {
    // The end each end is joined to.
    let mut link: BTreeMap<End, End> = BTreeMap::new();
    for node in nodes(policy, &ways) {
        let widest = node
            .iter()
            .filter(|end| !ways[end.0].pinned)
            .map(|end| ways[end.0].width)
            .fold(0.0, f64::max);
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
                    let sharpest = if a.width < widest {
                        policy.through_cos
                    } else {
                        REVERSE_COS
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
        // A line the shared centreline refuses stays as its pieces.
        let Ok(shape) = GroundShape::stroke(points.clone(), first.width) else {
            continue;
        };
        if chain.len() == 1 {
            continue;
        }
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
            shape,
        };
        for (way, _) in &chain {
            pieces[*way] = None;
        }
        out.push(welded);
    }
    out.extend(pieces.into_iter().flatten());
    out.sort_by_key(|way| way.slot);
    out
}

/// Where one way's width takes over another's line.
enum Stop {
    /// Under a carriageway that crosses the line: the keeper's points from
    /// the corner to there, and what is left of the other way from there on,
    /// both in the order the line is walked from the corner.
    Under(Vec<Point>, Vec<Point>),
    /// At the line's far end: the keeper's points from the corner to it.
    Whole(Vec<Point>),
}

/// One way of mending a corner of unlike ways: the way whose kind and width
/// go through the corner, by its end there; the way whose line it takes, by
/// its end there; how far along that line; and where it stops.
struct Carry {
    keeper: End,
    other: End,
    length: f64,
    stop: Stop,
}

/// A road changes width, and kind, only where another road crosses it.
/// Ends the weld left at one point are unlike ways, and the widest of them
/// would stop there with its flat end showing either side of the narrower
/// one. So one of the two is carried through the corner to the first place
/// on the other's line where the change is hidden:
///
/// - under a carriageway that crosses the line and covers the wider way's
///   whole flat end: the wider way ends just past that carriageway's middle,
///   and the narrower one starts on the same point, under it;
/// - or, where nothing crosses it so, the line's far end: one way is one
///   width from end to end, and what its far end meets is the next round's
///   corner if it is still unlike.
///
/// Either the wider way takes the narrower one's line from the corner
/// (among several narrower ways, the one that carries on straightest), or
/// the narrower one takes the wider way's line back from it: whichever
/// stretch is shorter. The corner is then the keeper's own round bend, and
/// any other end at the point joins it as it would anywhere along it
/// ([`snap`]).
///
/// A place a carriageway passes through is left, and so is a wider end
/// that something other than the ways it meets already covers.
fn carry(policy: &crate::layout::JointPolicy, mut ways: Vec<Way>, size: [f64; 2]) -> Vec<Way> {
    for _ in 0..policy.carry_rounds {
        // The places unlike ends meet, one of them the widest: that end,
        // and all of them.
        let unlike: Vec<(End, Vec<End>)> = nodes(policy, &ways)
            .into_iter()
            .filter(|node| node.len() >= 2 && !node.iter().any(|end| ways[end.0].pinned))
            .filter_map(|node| {
                let widest = node.iter().map(|end| ways[end.0].width).fold(0.0, f64::max);
                let mut wide = node.iter().filter(|end| ways[end.0].width == widest);
                match (wide.next(), wide.next()) {
                    (Some(&wide), None) => Some((wide, node)),
                    _ => None,
                }
            })
            .collect();
        if unlike.is_empty() {
            break;
        }
        // Each corner's ways of mending it, the shorter first.
        let mut corners: Vec<Vec<Carry>> = Vec::new();
        {
            let paving = Paving::new(policy, &ways, size);
            for (wide, node) in &unlike {
                let (wide, widest) = (*wide, ways[wide.0].width);
                let (at, out_of) = ways[wide.0].end(wide.1);
                let met: Vec<usize> = node.iter().map(|end| end.0).collect();
                if paving.passed(at, node) || paving.covers(at, out_of, widest / 2.0, &met) {
                    continue;
                }
                // The other end that leaves most nearly straight on.
                let onward = node.iter().filter(|end| end.0 != wide.0).max_by(|a, b| {
                    let leaving = |end: &End| -dot(out_of, ways[end.0].end(end.1).1);
                    leaving(a).total_cmp(&leaving(b))
                });
                let Some(&narrow) = onward else { continue };
                let mut found: Vec<Carry> = [(wide, narrow), (narrow, wide)]
                    .into_iter()
                    .map(|(keeper, other)| {
                        let (length, stop) = paving.change(&ways, other, keeper.0, widest);
                        Carry {
                            keeper,
                            other,
                            length,
                            stop,
                        }
                    })
                    .collect();
                found.sort_by(|a, b| a.length.total_cmp(&b.length));
                corners.push(found);
            }
        }
        if corners.is_empty() {
            break;
        }
        // A way is changed once a round: what the next corner found on it
        // is found again on the line it has now.
        let mut changed = vec![false; ways.len()];
        let mut gone = vec![false; ways.len()];
        for found in corners {
            for carry in found {
                let (keeper, other) = (carry.keeper.0, carry.other.0);
                if changed[keeper] || changed[other] {
                    break;
                }
                // The keeper's line with the corner last, then on along
                // the other's from the point the two shared.
                let turn = ways[other].end(carry.other.1).0;
                let mut points = ways[keeper].points.clone();
                if !carry.keeper.1 {
                    points.reverse();
                }
                *points.last_mut().expect("a way has points") = turn;
                let (onward, rest) = match carry.stop {
                    Stop::Under(onward, rest) => (onward, Some(rest)),
                    Stop::Whole(onward) => (onward, None),
                };
                points.extend(onward);
                points.dedup();
                if !carry.keeper.1 {
                    points.reverse();
                }
                let before = ways[keeper].clone();
                if !ways[keeper].relay(points) {
                    continue;
                }
                match rest {
                    Some(mut rest) => {
                        if carry.other.1 {
                            rest.reverse();
                        }
                        if !ways[other].relay(rest) {
                            ways[keeper] = before;
                            continue;
                        }
                    }
                    None => gone[other] = true,
                }
                let pieces = ways[other].pieces.clone();
                ways[keeper].pieces.extend(pieces);
                (changed[keeper], changed[other]) = (true, true);
                break;
            }
        }
        if !changed.contains(&true) {
            break;
        }
        let mut index = 0;
        ways.retain(|_| {
            index += 1;
            !gone[index - 1]
        });
    }
    ways
}

/// A way that ends on the map's edge at a slant turns square to the edge
/// before it, so its flat end lies along the edge and none of it shows.
fn gate(policy: &crate::layout::JointPolicy, ways: &mut [Way], size: [f64; 2]) {
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
            let depth = (policy.gate_widths * way.width).min(room / 2.0);
            if depth < policy.shortest_run_m {
                continue;
            }
            let turn = round_cm(sub(at, scale(normal, depth)));
            let mut gated = way.points.clone();
            gated.insert(if last { gated.len() - 1 } else { 1 }, turn);
            way.relay(gated);
        }
    }
}

/// What a lone end does about the carriageway it meets.
enum Join {
    /// Run on, or back, to this point.
    To(Point),
    /// Leave its line at the first point, straighten at the second and end
    /// on the third.
    Square(Point, Point, Point),
}

/// An end that touches another carriageway ends just past that
/// carriageway's rounded middle, square to it:
///
/// - one that stops short runs on to it (a rounded bend carries the middle
///   off the authored run an end was laid to, and an end laid exactly on a
///   middle would touch a line that any later change to that road moves);
/// - one that comes in at a slant (but not nearly alongside: within 20° it
///   is a lane peeling off the road, and is left), or whose square end the
///   road would not cover as it comes, leaves its line a little before the
///   road and meets
///   it square, or swings its whole last run square if that run is short:
///   no fork is left sharper than `policy.slant_cos` allows, and no corner of its
///   end shows past the road's edge;
/// - one that crossed the road and stops a width or two past it is cut back
///   to it: a stub, which serves nothing and shows its end.
///
/// Ends that share their point with other ends only are left.
fn snap(policy: &crate::layout::JointPolicy, ways: &mut [Way], size: [f64; 2]) {
    let paving = Paving::new(policy, ways, size);
    let on_map = |p: Point| (0..2).all(|k| p[k] > 0.0 && p[k] < size[k]);
    // The points ways turn at, by whole-metre cell.
    let cell = |p: Point| [libm::floor(p[0]) as i64, libm::floor(p[1]) as i64];
    let mut turns: BTreeMap<[i64; 2], Vec<Point>> = BTreeMap::new();
    for way in ways.iter() {
        for p in &way.points[1..way.points.len() - 1] {
            turns.entry(cell(*p)).or_default().push(*p);
        }
    }
    let on_a_way = |p: Point| {
        let [cx, cy] = cell(p);
        (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                turns.get(&[cx + dx, cy + dy]).is_some_and(|turns| {
                    turns.iter().any(|turn| distance(*turn, p) <= policy.meet_m)
                })
            })
        })
    };
    // An end alone, or one of several at a point a way runs through: each
    // of those joins that way.
    let lone: Vec<End> = nodes(policy, ways)
        .into_iter()
        .filter(|node| node.len() == 1 || on_a_way(ways[node[0].0].end(node[0].1).0))
        .flatten()
        .collect();
    let mut joins: Vec<(End, Join)> = Vec::new();
    for (way, last) in lone {
        let (at, out_of) = ways[way].end(last);
        if !on_map(at) || ways[way].pinned {
            continue;
        }
        let width = ways[way].width;
        // The nearest crossing of its own line with a carriageway it
        // touches, or crossed within a stub's length: how far along from the
        // end (signed), that carriageway, and its heading there.
        let mut nearest: Option<(f64, usize, Point)> = None;
        let reach = paving.widest + policy.stub_widths * width + policy.near_m;
        let (from, to) = (sub(at, scale(out_of, reach)), add(at, scale(out_of, reach)));
        paving.grid.any(segment_bounds(from, to, 0.0), |item| {
            let (other, c, d) = paving.stretches[item as usize];
            if other == way || c == d {
                return false;
            }
            let Some((t, _)) = segment_crossing(from, to, c, d) else {
                return false;
            };
            let along = (2.0 * t - 1.0) * reach;
            let apart = segment_distance(c, d, at);
            let run = scale(sub(d, c), 1.0 / distance(c, d));
            let (cos, sin) = (dot(out_of, run).abs(), cross(out_of, run).abs());
            // A road it stands in, however it lies; one ahead that it is
            // near and not alongside; or one behind that it crossed.
            let half = paving.halves[other];
            let meets = if apart <= half {
                sin >= SQUARE_SIN
            } else if along > 0.0 {
                apart <= half + policy.near_m && cos <= policy.alongside_cos
            } else {
                -along <= half + policy.stub_widths * width && cos <= policy.alongside_cos
            };
            if meets && nearest.is_none_or(|(known, ..)| along.abs() < known.abs()) {
                nearest = Some((along, other, run));
            }
            false
        });
        let Some((along, other, run)) = nearest else {
            continue;
        };
        let meeting = add(at, scale(out_of, along));
        let before = ways[way].before(last);
        // Where it would stand if it only ran on or was cut back.
        let stands = add(meeting, scale(out_of, OVERSHOOT_M));
        // One that runs nearly alongside the road is a lane that peels off
        // it, not a branch: squaring it would move the join far down the
        // road and take the lane away from what stands along it.
        let cos = dot(out_of, run).abs();
        let slant = cos > policy.slant_cos && cos <= policy.alongside_cos;
        if slant || (cos <= policy.slant_cos && !paving.hides(stands, out_of, way)) {
            // It leaves its line where the road's middle is still a good
            // way off to the side, and goes straight to the nearest point
            // of that middle.
            let sin = cross(out_of, run).abs();
            let squared = [policy.square_widths, policy.square_widths / 2.0]
                .into_iter()
                .find_map(|widths| {
                    let depth = paving.halves[other] + widths * width;
                    let back = depth / sin;
                    // The turn is taken in two halves, each this far from where
                    // the two lines meet: a curve, not a corner.
                    let ease = widths * width / policy.square_widths;
                    if distance(before, meeting) < back + ease + policy.shortest_run_m {
                        return None;
                    }
                    let turn = sub(meeting, scale(out_of, back));
                    let (foot, inner) = paving.nearest_on(other, turn)?;
                    let gap = distance(turn, foot);
                    if !inner || gap < paving.halves[other] + widths * width / 2.0 {
                        return None;
                    }
                    let square = scale(sub(foot, turn), 1.0 / gap);
                    let landing = round_cm(add(foot, scale(square, OVERSHOOT_M)));
                    let clear = !paving.crossed(turn, at, way, other)
                        && !paving.crossed(turn, landing, way, other);
                    clear.then_some(Join::Square(
                        round_cm(sub(turn, scale(out_of, ease))),
                        round_cm(add(turn, scale(square, ease.min(gap / 2.0)))),
                        landing,
                    ))
                });
            // With no room for that and a short last run, the whole run
            // swings to go straight to the road's middle.
            let swung = || {
                let (foot, inner) = paving.nearest_on(other, before)?;
                let gap = distance(before, foot);
                let short = distance(before, at) <= policy.swing_widths * width;
                if !inner
                    || !short
                    || gap < paving.halves[other] + width / 2.0 + policy.shortest_run_m
                {
                    return None;
                }
                let landing = round_cm(add(foot, scale(sub(foot, before), OVERSHOOT_M / gap)));
                let clear = !paving.crossed(before, at, way, other)
                    && !paving.crossed(before, landing, way, other);
                (clear && distance(landing, at) > REACHED_M).then_some(Join::To(landing))
            };
            if let Some(join) = squared.or_else(swung) {
                joins.push(((way, last), join));
                continue;
            }
        }
        if along > -ON_MIDDLE_M {
            if along + OVERSHOOT_M > REACHED_M {
                joins.push(((way, last), Join::To(round_cm(stands))));
            }
        } else if !paving.hides(at, out_of, way)
            && paving.hides(stands, out_of, way)
            && !paving.crossed(stands, at, way, other)
        {
            // A stub past the road it crossed.
            joins.push(((way, last), Join::To(round_cm(stands))));
        }
    }
    for ((way, last), join) in joins {
        let mut points = ways[way].points.clone();
        let at = ways[way].at(last);
        match join {
            Join::To(to) => points[at] = to,
            Join::Square(leave, straighten, landing) => {
                points[at] = landing;
                if last {
                    points.splice(at..at, [leave, straighten]);
                } else {
                    points.splice(1..1, [straighten, leave]);
                }
            }
        }
        points.dedup();
        ways[way].relay(points);
    }
}

/// Move each end, unless the shared centreline refuses the way it makes.
fn apply(ways: &mut [Way], moves: Vec<(End, Point)>) {
    for ((way, last), to) in moves {
        let mut points = ways[way].points.clone();
        points[ways[way].at(last)] = to;
        ways[way].relay(points);
    }
}

#[cfg(test)]
mod tests {
    fn policy() -> crate::layout::JointPolicy {
        crate::layout::PresetDefinitions::from_json(include_str!(
            "../../../fixtures/map-presets.json"
        ))
        .unwrap()
        .joints
    }
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
            &policy(),
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
            &policy(),
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
            &policy(),
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
        let closed = close(&policy(), surfaces.clone(), SIZE, &[]);
        assert_eq!(lines(&closed), lines(&surfaces));
    }

    #[test]
    fn unlike_roads_that_meet_alone_are_one_road_with_the_kind_and_width_of_the_longer() {
        for (turn, name) in [
            (12.0_f64, "nearly straight"),
            (60.0, "shallow"),
            (90.0, "square"),
            (140.0, "sharp"),
        ] {
            // A road comes east to (400, 400); a shorter track leaves it
            // there, turning `turn` degrees to the left. Nothing crosses
            // either: there is nowhere for the road to narrow unseen.
            let leaving = [turn.to_radians().cos(), turn.to_radians().sin()];
            let far = round_cm(add([400.0, 400.0], scale(leaving, 150.0)));
            let closed = close(
                &policy(),
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
            assert_eq!(
                lines(&closed),
                [vec![[200.0, 400.0], [400.0, 400.0], far]],
                "{name}"
            );
            assert_eq!(closed[0].kind, SurfaceKind::CountryRoad, "{name}");
            // It is the road's width to its far end, and the outside of the
            // corner is the road's own round bend.
            let across = [-leaving[1], leaving[0]];
            let end = sub(far, scale(leaving, 1.0));
            for (aside, paved) in [(3.9, true), (-3.9, true), (4.2, false), (-4.2, false)] {
                assert_eq!(
                    closed[0]
                        .shape
                        .contains(add(end, scale(across, aside)), 0.0),
                    paved,
                    "{name}: {aside} m aside"
                );
            }
            let outward = {
                let sum = sub([1.0, 0.0], leaving);
                scale(sum, 1.0 / distance(sum, [0.0, 0.0]))
            };
            let outside = |reach: f64| add([400.0, 400.0], scale(outward, reach));
            assert!(
                closed[0].shape.contains(outside(3.9), 0.0),
                "{name}: the corner is cut"
            );
            assert!(
                !closed[0].shape.contains(outside(4.6), 0.0),
                "{name}: something stands out past the corner"
            );
        }
    }

    #[test]
    fn a_wider_road_carries_on_round_a_corner_to_the_road_that_crosses() {
        // An avenue turns into a street at (400, 400). A side street joins
        // that street from the west 50 m on, and a street crosses it 100 m
        // on: only the crossing one covers the avenue's end either side.
        let closed = close(
            &policy(),
            vec![
                way(SurfaceKind::Road, 10.0, &[[200.0, 400.0], [400.0, 400.0]]),
                way(SurfaceKind::Road, 7.0, &[[400.0, 400.0], [400.0, 700.0]]),
                way(SurfaceKind::Road, 7.0, &[[300.0, 450.0], [400.25, 450.0]]),
                way(SurfaceKind::Road, 7.0, &[[300.0, 500.0], [500.0, 500.0]]),
            ],
            SIZE,
            &[],
        );
        // The avenue ends just past the crossing street's middle, and the
        // street starts there.
        assert_eq!(
            lines(&closed)[..2],
            [
                vec![[200.0, 400.0], [400.0, 400.0], [400.0, 500.25]],
                vec![[400.0, 500.25], [400.0, 700.0]]
            ]
        );
        for aside in [-5.0, -2.5, 0.0, 2.5, 5.0] {
            assert!(
                closed[3].shape.contains([400.0 + aside, 500.25], 0.0),
                "the avenue's end shows {aside} m aside"
            );
        }
        // The avenue is its own width past the side street, and the street
        // its own past the crossing.
        assert!(closed[0].shape.contains([404.9, 470.0], 0.0));
        // The parcel pass closes the layout's roads again: nothing moves.
        assert_eq!(
            lines(&close(&policy(), closed.clone(), SIZE, &[])),
            lines(&closed)
        );
        assert!(!closed
            .iter()
            .any(|area| area.shape.contains([404.0, 520.0], 0.0)));
    }

    #[test]
    fn a_narrower_road_reaches_back_to_a_crossing_nearer_on_the_wider_ones_line() {
        // A street crosses the avenue 50 m before the corner; nothing
        // crosses the street that leaves it for 300 m.
        let closed = close(
            &policy(),
            vec![
                way(SurfaceKind::Road, 10.0, &[[200.0, 400.0], [400.0, 400.0]]),
                way(SurfaceKind::Road, 7.0, &[[400.0, 400.0], [400.0, 700.0]]),
                way(SurfaceKind::Road, 7.0, &[[350.0, 300.0], [350.0, 500.0]]),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed)[..2],
            [
                vec![[200.0, 400.0], [350.25, 400.0]],
                vec![[350.25, 400.0], [400.0, 400.0], [400.0, 700.0]]
            ]
        );
        for aside in [-5.0, -2.5, 0.0, 2.5, 5.0] {
            assert!(closed[2].shape.contains([350.25, 400.0 + aside], 0.0));
        }
    }

    #[test]
    fn two_tracks_that_meet_alone_are_one_track_round_the_bend_however_sharp() {
        // Both leave (400, 400), 30° apart: a switchback.
        let closed = close(
            &policy(),
            vec![
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 400.0], [600.0, 400.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 400.0], [573.21, 500.0]],
                ),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed),
            [vec![[600.0, 400.0], [400.0, 400.0], [573.21, 500.0]]]
        );
        // The outside of the bend is round: track for its half width out
        // from the point, away from both arms, and nothing a metre farther.
        let outward = [
            -(15.0_f64.to_radians().cos()),
            -(15.0_f64.to_radians().sin()),
        ];
        let outside = |reach: f64| add([400.0, 400.0], scale(outward, reach));
        assert!(closed[0].shape.contains(outside(1.9), 0.0));
        assert!(!closed[0].shape.contains(outside(4.5), 0.0));
    }

    #[test]
    fn a_branch_that_comes_in_at_a_slant_bends_to_meet_the_road_square() {
        // A track meets a road 34° off its line.
        let closed = close(
            &policy(),
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[100.0, 300.0], [500.0, 300.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[150.0, 200.0], [300.0, 300.0]],
                ),
            ],
            SIZE,
            &[],
        );
        let track = &lines(&closed)[1];
        assert_eq!(track.len(), 4, "{track:?}");
        // Its last run crosses the road's middle square, from its own side.
        // It keeps its line until the road's edge is about two of its
        // widths off, turns in two halves, and goes straight across to the
        // road's middle.
        let on_its_line = cross(sub(track[1], track[0]), [150.0, 100.0]).abs();
        assert!(on_its_line < 2.0 && track[1][1] < 290.0, "{track:?}");
        assert_eq!(track[2], [285.0, 294.0]);
        assert_eq!(track[3], [285.0, 300.0 + OVERSHOOT_M]);
        assert!(covered(&closed, 1, track[3], [0.0, 1.0], 2.0));
        assert_eq!(lines(&closed)[0], [[100.0, 300.0], [500.0, 300.0]]);
    }

    #[test]
    fn a_lane_that_peels_off_nearly_alongside_a_road_keeps_its_line() {
        // 3° off the track it leaves, as a farm's lane is laid.
        let laid = vec![
            way(
                SurfaceKind::DirtTrack,
                4.0,
                &[[100.0, 300.0], [300.0, 300.0], [600.0, 300.0]],
            ),
            way(
                SurfaceKind::DirtTrack,
                4.0,
                &[[300.0, 300.0], [500.0, 289.5]],
            ),
        ];
        let closed = close(&policy(), laid, SIZE, &[]);
        let lane = &lines(&closed)[1];
        assert_eq!(lane.len(), 2, "{lane:?}");
        assert!(distance(lane[0], [300.0, 300.0]) < 0.5, "{lane:?}");
        assert_eq!(lane[1], [500.0, 289.5]);
    }

    #[test]
    fn a_stub_just_past_a_crossing_is_cut_back_to_it() {
        // The track crosses the road and stops 2 m past its far edge.
        let closed = close(
            &policy(),
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[100.0, 300.0], [500.0, 300.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[300.0, 100.0], [300.0, 306.0]],
                ),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed)[1],
            [[300.0, 100.0], [300.0, 300.0 + OVERSHOOT_M]]
        );
        // One that runs on well past the road is a road of its own.
        let long = vec![
            way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[100.0, 300.0], [500.0, 300.0]],
            ),
            way(
                SurfaceKind::DirtTrack,
                4.0,
                &[[300.0, 100.0], [300.0, 340.0]],
            ),
        ];
        assert_eq!(
            lines(&close(&policy(), long.clone(), SIZE, &[])),
            lines(&long)
        );
    }

    #[test]
    fn a_road_that_leaves_the_map_at_a_slant_turns_square_to_the_edge() {
        let closed = close(
            &policy(),
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
        assert_eq!(
            lines(&close(&policy(), square.clone(), SIZE, &[])),
            lines(&square)
        );
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
            &policy(),
            vec![way(
                SurfaceKind::CountryRoad,
                8.0,
                &[[0.0, 400.0], [400.0, 400.0], [760.0, 400.0]],
            )],
            SIZE,
            &[&ground],
        );
        let road = &lines(&closed)[0];
        let end = road[road.len() - 1];
        assert!(
            end[0] >= 500.0 && end[0] <= 500.0 + 4.0 + policy().trim_margin_m + TRIM_STEP_M,
            "{road:?}"
        );
        // One that stops on the settlement's ground, or by another road, stays.
        let inside = vec![way(
            SurfaceKind::DirtTrack,
            4.0,
            &[[350.0, 350.0], [450.0, 450.0]],
        )];
        assert_eq!(
            lines(&close(&policy(), inside.clone(), SIZE, &[&ground])),
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
            &policy(),
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
        // The two roads are one road round the corner, and the track bends
        // to cross it.
        assert_eq!(closed.len(), 2);
        let samples: Vec<&[Point]> = closed
            .iter()
            .map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.samples(),
                GroundShape::Polygon { .. } => unreachable!(),
            })
            .collect();
        let crosses = samples[0].windows(2).any(|run| {
            samples[1]
                .windows(2)
                .any(|other| segment_crossing(run[0], run[1], other[0], other[1]).is_some())
        });
        assert!(crosses, "they no longer cross: {:?}", lines(&closed));
    }

    #[test]
    fn a_road_that_ends_on_two_tracks_carries_on_down_the_straighter_one() {
        let closed = close(
            &policy(),
            vec![
                way(
                    SurfaceKind::CountryRoad,
                    8.0,
                    &[[200.0, 400.0], [400.0, 400.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 400.0], [550.0, 430.0]],
                ),
                way(
                    SurfaceKind::DirtTrack,
                    4.0,
                    &[[400.0, 250.0], [400.0, 400.0]],
                ),
            ],
            SIZE,
            &[],
        );
        // The road is the track that leaves 11° off its line too.
        assert_eq!(closed.len(), 2);
        let (road, side) = (&lines(&closed)[0], &lines(&closed)[1]);
        assert_eq!(road, &[[200.0, 400.0], [400.0, 400.0], [550.0, 430.0]]);
        assert_eq!(closed[0].kind, SurfaceKind::CountryRoad);
        // The other track joins the road as it would anywhere along it:
        // just past the road's middle, under its width.
        assert_eq!(side[0], [400.0, 250.0]);
        assert!(
            side[1][0] == 400.0 && (400.0..=401.0).contains(&side[1][1]),
            "{side:?}"
        );
        assert!(covered(&closed, 1, side[1], [0.0, 1.0], 2.0));
    }

    #[test]
    fn a_street_drawn_beside_another_is_cut_back_to_the_road_that_crosses_it() {
        // One street ends on a cross street from the west; the next starts
        // 40 m short of that cross street, 4 m to one side: a step in the
        // road's edge where it starts.
        let closed = close(
            &policy(),
            vec![
                way(SurfaceKind::Road, 7.0, &[[300.0, 100.0], [300.0, 500.0]]),
                way(SurfaceKind::Road, 7.0, &[[100.0, 300.0], [300.25, 300.0]]),
                way(SurfaceKind::Road, 7.0, &[[260.0, 304.0], [600.0, 304.0]]),
            ],
            SIZE,
            &[],
        );
        assert_eq!(lines(&closed)[1], [[100.0, 300.0], [300.25, 300.0]]);
        assert_eq!(lines(&closed)[2], [[299.75, 304.0], [600.0, 304.0]]);
    }

    #[test]
    fn two_streets_drawn_side_by_side_with_nothing_between_become_one() {
        let closed = close(
            &policy(),
            vec![
                way(SurfaceKind::Road, 7.0, &[[100.0, 300.0], [400.0, 300.0]]),
                way(SurfaceKind::Road, 7.0, &[[380.0, 303.0], [700.0, 303.0]]),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed),
            [vec![[100.0, 300.0], [390.0, 301.5], [700.0, 303.0]]]
        );
    }

    #[test]
    fn a_turn_a_few_metres_from_an_end_is_dropped_so_the_corner_can_be_made() {
        // The street's first 3 m are a link to the avenue's end.
        let closed = close(
            &policy(),
            vec![
                way(SurfaceKind::Road, 10.0, &[[200.0, 400.0], [400.0, 400.0]]),
                way(
                    SurfaceKind::Road,
                    7.0,
                    &[[400.0, 400.0], [401.0, 403.0], [400.0, 550.0]],
                ),
            ],
            SIZE,
            &[],
        );
        assert_eq!(
            lines(&closed),
            [vec![[200.0, 400.0], [400.0, 400.0], [400.0, 550.0]]]
        );
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
        let once = close(&policy(), surfaces, SIZE, &[]);
        let twice = close(&policy(), once.clone(), SIZE, &[]);
        assert_eq!(lines(&twice), lines(&once));
    }
}
