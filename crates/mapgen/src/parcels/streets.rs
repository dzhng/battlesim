//! A district's streets: a grid fitted between its edges, along the road
//! the district fronts, bowed where the preset says. A street runs from a
//! junction to a junction: to the carriageway on the district's edge, where
//! it comes to it near enough square and clear of the junctions that
//! carriageway already has (or straight across from a street that ends on
//! its far side), and otherwise to its last crossing or the last ground it
//! serves. No street crosses water.
use super::Pass;
use crate::layout::geometry::{
    add, bearing, direction, distance, dot, ray_exit, round_cm, scale, segment_bounds,
    segment_crossing, segment_distance, sub, Grid, Point, TAU,
};
use crate::layout::water::Water;
use crate::{Diagnostic, DistrictPlan, MapPlan, SettlementPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::BTreeSet;

use super::space::Rect;

/// How far a street runs past the middle of the one it meets, so the two
/// centrelines cross whatever a centimetre of rounding did to either.
const JOIN_OVERSHOOT_M: f64 = 0.5;
/// A joint this near a carriageway's paving lies on it.
const ON_WAY_M: f64 = 1.0;
/// A district's edge carries a carriageway whose own edge is this near it.
const CARRIED_M: f64 = 1.0;

/// One carriageway's rounded centreline.
pub struct Way {
    pub samples: Vec<Point>,
    pub half_width: f64,
    /// `[min_x, min_y, max_x, max_y]` of the samples.
    pub bounds: [f64; 4],
    /// A road of the layout's network: not a town's street or avenue.
    road: bool,
}

/// Every carriageway on the map so far: the layout's roads, then each street
/// as it is laid.
pub struct Network<'a> {
    policy: &'a crate::layout::StreetGeometry,
    pub ways: Vec<Way>,
    /// Sample segments with their way's half width and its place in `ways`.
    segments: Vec<(Point, Point, f64, u32)>,
    grid: Grid,
    extent: f64,
    water: Water<'a>,
    /// The least ground between a street's middle and the water's edge.
    clearance: f64,
    /// Where a carriageway ends on another, and where two cross (each then
    /// listed arriving both ways).
    joints: Vec<Joint>,
    joint_grid: Grid,
}

/// One carriageway's end on another.
#[derive(Clone, Copy)]
struct Joint {
    at: Point,
    /// The unit vector it arrives along, and half its width.
    arriving: Point,
    half_width: f64,
    /// Whether both are roads of the layout's network: a junction of the
    /// roads, which a street keeps clear of.
    roads: bool,
}

/// How a street lands on the carriageway ahead of it.
enum Landing {
    /// Where its line crosses the carriageway.
    Free(Point),
    /// Square to it, round a turn at the point it runs on from.
    Turned(Point),
    /// On the point where a street already ends on the carriageway from its
    /// far side, in line with it: the two make a crossroads.
    Opposite(Point),
    /// Not at all: it would come to the carriageway at a slant, or beside
    /// a junction it cannot make a crossroads of.
    Refused,
}

impl Landing {
    fn point(&self) -> Option<Point> {
        match self {
            Landing::Free(at) | Landing::Turned(at) | Landing::Opposite(at) => Some(*at),
            Landing::Refused => None,
        }
    }
}

/// Where a street running on would cross a carriageway.
struct Met {
    /// Just past the centreline it crosses.
    point: Point,
    /// The stretch of centreline crossed, and half that carriageway's width.
    stretch: [Point; 2],
    half_width: f64,
    /// That carriageway, by its place in the network.
    way: usize,
}

impl<'a> Network<'a> {
    pub fn new(
        plan: &'a MapPlan,
        clearance: f64,
        policy: &'a crate::layout::StreetGeometry,
    ) -> Self {
        let mut network = Self {
            policy,
            ways: Vec::new(),
            segments: Vec::new(),
            grid: Grid::new(plan.size, 64.0),
            extent: plan.size[0].max(plan.size[1]),
            water: Water::new(&plan.rivers, plan.size),
            clearance,
            joints: Vec::new(),
            joint_grid: Grid::new(plan.size, 64.0),
        };
        for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
            network.add_way(&area.shape, area.kind != SurfaceKind::Road);
        }
        // The junctions the plan already has: a later street that comes to
        // the same road from its other side meets it there, or keeps clear.
        let mut joints: Vec<Joint> = Vec::new();
        for (index, way) in network.ways.iter().enumerate() {
            let last = way.samples.len() - 1;
            for (end, before) in [(0, 1), (last, last - 1)] {
                let (end, before) = (way.samples[end], way.samples[before]);
                if let Some(other) = network.joined_to(index, end) {
                    joints.push(Joint {
                        at: end,
                        arriving: scale(sub(end, before), 1.0 / distance(end, before)),
                        half_width: way.half_width,
                        roads: way.road && network.ways[other].road,
                    });
                }
            }
        }
        // Where two roads of the layout share a point of their lines, the
        // roads meet, whichever way their strokes were joined there.
        for (index, way) in network.ways.iter().enumerate().filter(|(_, way)| way.road) {
            for at in &way.samples {
                let meets = network.grid.any([at[0], at[1], at[0], at[1]], |item| {
                    let (a, b, _, other) = network.segments[item as usize];
                    other as usize != index
                        && network.ways[other as usize].road
                        && segment_distance(a, b, *at) <= ON_WAY_M
                });
                if meets {
                    joints.push(Joint {
                        at: *at,
                        arriving: [1.0, 0.0],
                        half_width: way.half_width,
                        roads: true,
                    });
                }
            }
        }
        // And where two cross with neither ending there.
        let ends_near = |way: usize, at: Point, within: f64| {
            let samples = &network.ways[way].samples;
            distance(samples[0], at) <= within || distance(samples[samples.len() - 1], at) <= within
        };
        for (index, (a, b, half_width, way)) in network.segments.iter().enumerate() {
            network.grid.any(segment_bounds(*a, *b, 0.0), |other| {
                let (c, d, other_half, crossed) = network.segments[other as usize];
                if other as usize <= index || crossed == *way {
                    return false;
                }
                let Some((share, _)) = segment_crossing(*a, *b, c, d) else {
                    return false;
                };
                let at = add(*a, scale(sub(*b, *a), share));
                if ends_near(*way as usize, at, other_half + ON_WAY_M)
                    || ends_near(crossed as usize, at, half_width + ON_WAY_M)
                {
                    return false;
                }
                let roads = network.ways[*way as usize].road && network.ways[crossed as usize].road;
                for (from, to, half_width) in [(*a, *b, *half_width), (c, d, other_half)] {
                    let along = scale(sub(to, from), 1.0 / distance(from, to));
                    for arriving in [along, scale(along, -1.0)] {
                        joints.push(Joint {
                            at,
                            arriving,
                            half_width,
                            roads,
                        });
                    }
                }
                false
            });
        }
        for joint in joints {
            network.join(joint);
        }
        network
    }

    /// Strokes only: an apron is paved ground, not a way through.
    pub fn add(&mut self, shape: &GroundShape) {
        self.add_way(shape, false);
    }

    /// Add a carriageway: one of the layout's roads where `road`.
    fn add_way(&mut self, shape: &GroundShape, road: bool) {
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
            self.segments
                .push((pair[0], pair[1], half_width, self.ways.len() as u32));
        }
        let [x0, y0, x1, y1] = contract::ground::limits(&samples, 0.0);
        self.ways.push(Way {
            samples,
            half_width,
            bounds: [x0, y0, x1, y1],
            road,
        });
    }

    /// Record that a carriageway ends on another.
    fn join(&mut self, joint: Joint) {
        let at = joint.at;
        self.joint_grid
            .insert([at[0], at[1], at[0], at[1]], self.joints.len() as u32);
        self.joints.push(joint);
    }

    /// Record that a street `half_width` wide either side ends on another
    /// carriageway at `at`, arriving along the unit vector `arriving`.
    fn join_street(&mut self, at: Point, arriving: Point, half_width: f64) {
        self.join(Joint {
            at,
            arriving,
            half_width,
            roads: false,
        });
    }

    /// How a street `half` wide either side, at `from` and heading along
    /// the unit vector `toward`, lands on the carriageway of `met`. Where it
    /// would come to it more than 20 degrees off square it turns at `from`
    /// to meet it square, and where that turn would be more than 28
    /// degrees, or leave too short a run, it does not land. Nor within
    /// `clear` of a junction of the layout's roads: a road leaves another
    /// at a junction of its own. Where a street ends on the carriageway from its
    /// other side within `within`, in line with it and with no other
    /// junction beside it, it lands there and the two make a crossroads: a
    /// street as wide ends on that very point, so the two are one street
    /// through the junction, and one of another width lands opposite it,
    /// just past the carriageway's middle from its own side, so each
    /// crosses the middle and no two unlike ends meet. It does not land
    /// within a few widths of any other junction: a junction has four arms
    /// at most, and none at a slant to another. Otherwise it lands where
    /// its line crosses. Never on a landing of `taken`.
    fn landing(
        &self,
        from: Point,
        met: &Met,
        toward: Point,
        [within, clear]: [f64; 2],
        half: f64,
        taken: &[Point],
    ) -> Landing {
        let [a, b] = met.stretch;
        let run = scale(sub(b, a), 1.0 / distance(a, b));
        let slant = dot(run, toward).abs();
        if slant > self.policy.turn_sin {
            return Landing::Refused;
        }
        let crowd = self.policy.crowd_widths * 2.0 * half;
        if slant > self.policy.slant_sin {
            // Square to the carriageway, at its nearest point.
            let Some((away, foot, stretch)) = self.ways[met.way]
                .samples
                .windows(2)
                .map(|pair| {
                    let run = sub(pair[1], pair[0]);
                    let share = (dot(sub(from, pair[0]), run) / dot(run, run)).clamp(0.0, 1.0);
                    let foot = add(pair[0], scale(run, share));
                    (distance(from, foot), foot, [pair[0], pair[1]])
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
            else {
                return Landing::Refused;
            };
            let run = scale(
                sub(stretch[1], stretch[0]),
                1.0 / distance(stretch[0], stretch[1]),
            );
            let toward = scale(sub(foot, from), 1.0 / away);
            let point = round_cm(add(from, scale(toward, away + JOIN_OVERSHOOT_M)));
            // Only where the carriageway is straight enough there that the
            // nearest point is the square one, and no junction is beside it.
            let beside = |joint: &Joint| {
                distance(joint.at, point) <= if joint.roads { clear } else { crowd }
                    && segment_distance(
                        sub(stretch[0], scale(run, clear)),
                        add(stretch[1], scale(run, clear)),
                        joint.at,
                    ) <= met.half_width + ON_WAY_M
            };
            let reach = clear.max(crowd);
            let bounds = [
                point[0] - reach,
                point[1] - reach,
                point[0] + reach,
                point[1] + reach,
            ];
            return if away < self.policy.turn_widths * 2.0 * half
                || dot(run, toward).abs() > self.policy.slant_sin
                || !self.dry(from, point)
                || self
                    .joint_grid
                    .any(bounds, |item| beside(&self.joints[item as usize]))
            {
                Landing::Refused
            } else {
                Landing::Turned(point)
            };
        }
        let p = met.point;
        let reach = within.max(clear).max(crowd);
        let along = scale(run, reach);
        let (from, to) = (sub(a, along), add(b, along));
        let mut near: Vec<Joint> = Vec::new();
        let bounds = [p[0] - reach, p[1] - reach, p[0] + reach, p[1] + reach];
        self.joint_grid.any(bounds, |item| {
            let joint = self.joints[item as usize];
            if segment_distance(from, to, joint.at) <= met.half_width + ON_WAY_M {
                near.push(joint);
            }
            false
        });
        let away = |joint: &Joint| distance(joint.at, p);
        if near.iter().any(|joint| joint.roads && away(joint) <= clear) {
            return Landing::Refused;
        }
        // The nearest street that ends here from the far side, in line, at
        // a junction of its own.
        let partner = near
            .iter()
            .filter(|joint| {
                away(joint) <= within
                    && dot(joint.arriving, toward) <= -self.policy.in_line_cos
                    && near
                        .iter()
                        .filter(|other| distance(other.at, joint.at) <= crowd)
                        .count()
                        == 1
            })
            .min_by(|x, y| away(x).total_cmp(&away(y)));
        let Some(joint) = partner else {
            return if near.iter().any(|joint| away(joint) <= crowd) {
                Landing::Refused
            } else {
                Landing::Free(p)
            };
        };
        let middle = add(a, scale(run, dot(sub(joint.at, a), run)));
        let landing = if joint.half_width == half {
            joint.at
        } else {
            round_cm(add(middle, scale(toward, JOIN_OVERSHOOT_M)))
        };
        if taken.contains(&landing) {
            Landing::Refused
        } else {
            Landing::Opposite(landing)
        }
    }

    /// Whether any carriageway's surface lies on the rectangle.
    pub fn covers(&self, rect: &Rect) -> bool {
        self.grid.any(rect.bounds(), |item| {
            let (a, b, half_width, _) = self.segments[item as usize];
            // A parcel may touch a carriageway's edge, not share ground with it.
            rect.segment_gap(a, b) < half_width - 0.01
        })
    }

    /// Open ground between `p` and the nearest carriageway's edge, negative
    /// on one; `within` when there is at least that much.
    pub fn edge_gap(&self, p: Point, within: f64, charge: impl FnMut() -> bool) -> f64 {
        let mut gap = within;
        // A carriageway is found by its own width's box, so only the gap
        // asked for is searched.
        let bounds = [p[0] - within, p[1] - within, p[0] + within, p[1] + within];
        self.grid.any_charged(
            bounds,
            |item| {
                let (a, b, half_width, _) = self.segments[item as usize];
                gap = gap.min(segment_distance(a, b, p) - half_width);
                false
            },
            charge,
        );
        gap
    }

    /// The bearing of the carriageway nearest `p`, when one's middle lies
    /// within `within`.
    pub fn heading_near(&self, p: Point, within: f64, charge: impl FnMut() -> bool) -> Option<f64> {
        let mut best: Option<(f64, f64)> = None;
        let bounds = [p[0] - within, p[1] - within, p[0] + within, p[1] + within];
        self.grid.any_charged(
            bounds,
            |item| {
                let (a, b, ..) = self.segments[item as usize];
                let away = segment_distance(a, b, p);
                if away <= within && best.is_none_or(|(known, _)| away < known) {
                    best = Some((away, bearing(a, b)));
                }
                false
            },
            charge,
        );
        best.map(|(_, heading)| heading)
    }

    /// Whether a carriageway runs along the edge from `a` to `b`: its
    /// middle within its own half width of the edge, going the edge's way,
    /// for most of the edge's length. With `road`, only a road of the
    /// layout's network.
    pub fn runs_along(&self, a: Point, b: Point, road: bool) -> bool {
        let edge = scale(sub(b, a), 1.0 / distance(a, b));
        let carried = [0.25, 0.5, 0.75]
            .into_iter()
            .filter(|share| {
                let p = add(a, scale(sub(b, a), *share));
                let reach = CARRIED_M;
                let bounds = [p[0] - reach, p[1] - reach, p[0] + reach, p[1] + reach];
                self.grid.any(bounds, |item| {
                    let (c, d, half_width, way) = self.segments[item as usize];
                    (!road || self.ways[way as usize].road)
                        && segment_distance(c, d, p) <= half_width + CARRIED_M
                        && (dot(edge, sub(d, c)) / distance(c, d)).abs() >= self.policy.parallel_cos
                })
            })
            .count();
        carried >= 2
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
                let (a, b, half_width, _) = self.segments[item as usize];
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

    /// The first carriageway a street at `from`, heading along the unit
    /// vector `toward`, would cross if it ran on for up to `reach`: how far
    /// on, the stretch crossed and its half width.
    fn crossing(
        &self,
        from: Point,
        toward: Point,
        reach: f64,
    ) -> Option<(f64, [Point; 2], f64, usize)> {
        // From just past the end, so the street's own last run is not met.
        let start = add(from, scale(toward, 0.1));
        let end = add(from, scale(toward, reach));
        let mut first: Option<(f64, [Point; 2], f64, usize)> = None;
        self.grid.any(segment_bounds(start, end, 0.0), |item| {
            let (a, b, half_width, way) = self.segments[item as usize];
            if let Some((t, _)) = segment_crossing(start, end, a, b) {
                if first.is_none_or(|(known, ..)| t < known) {
                    first = Some((t, [a, b], half_width, way as usize));
                }
            }
            false
        });
        first.map(|(t, stretch, half_width, way)| {
            (distance(start, end) * t + 0.1, stretch, half_width, way)
        })
    }

    /// Where a street that ends at `from`, heading along the unit vector
    /// `toward`, would first cross a carriageway if it ran on for up to
    /// `reach`. `None` when nothing lies ahead or water lies between.
    fn ahead(&self, from: Point, toward: Point, reach: f64) -> Option<Met> {
        let (met, stretch, half_width, way) = self.crossing(from, toward, reach)?;
        let point = round_cm(add(from, scale(toward, met + JOIN_OVERSHOOT_M)));
        self.dry(from, point).then_some(Met {
            point,
            stretch,
            half_width,
            way,
        })
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
                    && facing <= -self.policy.parallel_cos
                    && best.is_none_or(|(known, _)| ahead < known)
                    && self.dry(from, at)
                {
                    best = Some((ahead, at));
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// How a candidate stretch of street sits against what is already
    /// laid: whether it only repeats a carriageway that runs beside it,
    /// less than `clearance` off for most of its length, and whether it
    /// crosses one.
    fn meets(&self, p: Point, q: Point, clearance: f64) -> (bool, bool) {
        let length = distance(p, q);
        let along = scale(sub(q, p), 1.0 / length);
        let (mut beside, mut crosses) = (0.0, false);
        let mut seen = BTreeSet::new();
        self.grid.any(segment_bounds(p, q, clearance), |item| {
            let (a, b, ..) = self.segments[item as usize];
            let cos = dot(along, sub(b, a)) / distance(a, b);
            if cos.abs() < self.policy.parallel_cos {
                crosses |= segment_crossing(p, q, a, b).is_some();
            } else if seen.insert(item) {
                // The part of the stretch the carriageway lies beside.
                let (s, t) = (dot(sub(a, p), along), dot(sub(b, p), along));
                let (from, to) = (s.min(t).max(0.0), s.max(t).min(length));
                let middle = add(p, scale(along, (from + to) / 2.0));
                if to > from && segment_distance(a, b, middle) < clearance {
                    beside += to - from;
                }
            }
            false
        });
        let repeats = beside > length / 2.0;
        (repeats, crosses && !repeats)
    }

    /// Whether the end `at` of the way `own` lies on another carriageway.
    fn joined(&self, own: usize, at: Point) -> bool {
        self.joined_to(own, at).is_some()
    }

    /// The other carriageway the end `at` of the way `own` lies on.
    fn joined_to(&self, own: usize, at: Point) -> Option<usize> {
        let mut found = None;
        self.grid.any([at[0], at[1], at[0], at[1]], |item| {
            let (a, b, half_width, other) = self.segments[item as usize];
            if other as usize != own && segment_distance(a, b, at) <= half_width {
                found = Some(other as usize);
            }
            found.is_some()
        });
        found
    }

    /// Whether a street that stops at `from`, heading along the unit vector
    /// `toward`, stops short of a carriageway: one lies within `reach`
    /// ahead of it, on its line or a little to either side.
    fn short_of(&self, from: Point, toward: Point, reach: f64) -> bool {
        let across = [-toward[1], toward[0]];
        [-self.policy.ahead_spread, 0.0, self.policy.ahead_spread]
            .into_iter()
            .any(|aside| {
                let heading = add(toward, scale(across, aside));
                let heading = scale(heading, 1.0 / libm::hypot(heading[0], heading[1]));
                self.crossing(from, heading, reach).is_some()
            })
    }

    /// The nearest end of a carriageway that stops in the open ahead of a
    /// street at its first point heading along its second, a unit vector: within
    /// `reach` ahead, and no farther off its line than `aside` allows for
    /// that distance and that carriageway's half width. `own` is the way
    /// the street belongs to, where it is laid already, and `half_width`
    /// half the street's width.
    fn end_ahead(
        &self,
        own: Option<usize>,
        [from, toward]: [Point; 2],
        reach: f64,
        half_width: f64,
        facing: f64,
        aside: impl Fn(f64, f64) -> f64,
    ) -> Option<Point> {
        let mut best: Option<(f64, Point)> = None;
        for (index, way) in self.ways.iter().enumerate() {
            let bounds = way.bounds;
            // Only a carriageway as wide as the street: a road changes
            // width under a road that crosses it, never end to end.
            if Some(index) == own
                || way.half_width != half_width
                || from[0] < bounds[0] - reach
                || from[0] > bounds[2] + reach
                || from[1] < bounds[1] - reach
                || from[1] > bounds[3] + reach
            {
                continue;
            }
            let last = way.samples.len() - 1;
            for (end, before) in [(0, 1), (last, last - 1)] {
                let (at, before) = (way.samples[end], way.samples[before]);
                let offset = sub(at, from);
                let ahead = dot(offset, toward);
                let off = (offset[0] * toward[1] - offset[1] * toward[0]).abs();
                if ahead > 1.0
                    && ahead <= reach
                    && dot(sub(at, before), toward) <= facing * distance(at, before)
                    && off <= aside(ahead, way.half_width)
                    && best.is_none_or(|(known, _)| ahead < known)
                    && !self.joined(index, at)
                    && self.dry(from, at)
                {
                    best = Some((ahead, at));
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// Where a street at `from` heading along the unit vector `toward`
    /// meets a carriageway that stops in the open just short of its line:
    /// the point of that carriageway half the street's width back from its
    /// end, when the end lies within `reach` ahead and `within` aside and
    /// the carriageway runs across the street's line.
    fn short_end(
        &self,
        from: Point,
        toward: Point,
        reach: f64,
        within: f64,
        half: f64,
    ) -> Option<Met> {
        let mut best: Option<(f64, Met)> = None;
        for (index, way) in self.ways.iter().enumerate() {
            let bounds = way.bounds;
            if from[0] < bounds[0] - reach
                || from[0] > bounds[2] + reach
                || from[1] < bounds[1] - reach
                || from[1] > bounds[3] + reach
            {
                continue;
            }
            let last = way.samples.len() - 1;
            for (end, before) in [(0, 1), (last, last - 1)] {
                let (at, before) = (way.samples[end], way.samples[before]);
                let run = distance(at, before);
                let along = scale(sub(at, before), 1.0 / run);
                let offset = sub(at, from);
                let ahead = dot(offset, toward);
                let aside = (offset[0] * toward[1] - offset[1] * toward[0]).abs();
                if ahead > 1.0
                    && ahead <= reach
                    && aside <= within
                    && run > 2.0 * half
                    && dot(along, toward).abs() < self.policy.parallel_cos
                    && best.as_ref().is_none_or(|(known, _)| ahead < *known)
                    && !self.joined(index, at)
                {
                    // Just past its middle, as any street meets a road.
                    let on = sub(at, scale(along, half));
                    let gap = distance(from, on);
                    let point = round_cm(add(
                        from,
                        scale(sub(on, from), (gap + JOIN_OVERSHOOT_M) / gap),
                    ));
                    if self.dry(from, point) {
                        let met = Met {
                            point,
                            stretch: [before, at],
                            half_width: way.half_width,
                            way: index,
                        };
                        best = Some((ahead, met));
                    }
                }
            }
        }
        best.map(|(_, met)| met)
    }

    /// The end a street already laid turns to: no farther off its line
    /// than `self.policy.corner_tan` of the way there. The two make a corner at it.
    fn corner_end(&self, own: usize, from: Point, toward: Point, reach: f64) -> Option<Point> {
        // Not one that runs the street's own way: that is a street beside it.
        let half = self.ways[own].half_width;
        self.end_ahead(
            Some(own),
            [from, toward],
            reach,
            half,
            self.policy.facing_cos,
            |ahead, half_width| self.policy.corner_tan * ahead + half_width,
        )
    }

    /// The end a street of a grid left stopped carries on to: one that
    /// faces it, so little off its line that the two are one street.
    fn gentle_end(&self, from: Point, toward: Point, reach: f64, half: f64) -> Option<Point> {
        self.end_ahead(
            None,
            [from, toward],
            reach,
            half,
            -self.policy.facing_cos,
            |ahead, _| self.policy.own_tan * ahead,
        )
    }

    /// The end a line of a grid is moved to carry on from: one that faces
    /// it, no farther off its line than `within`.
    fn open_end(
        &self,
        from: Point,
        toward: Point,
        reach: f64,
        within: f64,
        half: f64,
    ) -> Option<Point> {
        self.end_ahead(
            None,
            [from, toward],
            reach,
            half,
            -self.policy.facing_cos,
            |_, _| within,
        )
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
/// district's streets are a grid along one of its edges, bowed where its
/// preset says: the edge that most of its other edges run with or square
/// to, a road's counting double, so that its streets run beside the
/// carriageways round it and meet them square. Between edges that serve
/// alike it is the one that runs longest with the settlement's main street,
/// so neighbouring districts' long streets run the same way.
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
        let width = pass.street_width(kind);
        let shape = GroundShape::stroke(points, width)
            .map_err(|message| pass.fail(feature, "$.presets.parcels", &message))?;
        network.add(&shape);
        surfaces.push(SurfaceArea { kind, shape });
        Ok::<(), Vec<Diagnostic>>(())
    };
    for (_, index) in order {
        let district = &settlement.districts[index];
        let surface = pass.district(district)?.streets.surface;
        // Its long streets run with the edge whose line the carriageways
        // round it best agree with: each edge that runs along that line or
        // square to it, within the slant a street may meet a road at,
        // counts its length, twice over where a road runs along it and a
        // quarter where nothing does.
        let edges: Vec<(Point, Point, f64)> = contract::ground::edges(&district.ring)
            .map(|(a, b)| {
                let weight = if network.runs_along(*a, *b, true) {
                    pass.presets.parcels.geometry.road_edge
                } else if network.runs_along(*a, *b, false) {
                    1.0
                } else {
                    pass.presets.parcels.geometry.open_edge
                };
                (*a, *b, weight * distance(*a, *b))
            })
            .collect();
        let agreed = |axis: f64| -> f64 {
            edges
                .iter()
                .filter(|(a, b, _)| {
                    let off = libm::sin(2.0 * (bearing(*a, *b) - axis)).abs();
                    off <= 2.0
                        * pass.presets.parcels.geometry.slant_sin
                        * libm::sqrt(
                            1.0 - pass.presets.parcels.geometry.slant_sin
                                * pass.presets.parcels.geometry.slant_sin,
                        )
                })
                .map(|(.., weight)| weight)
                .sum()
        };
        let with_main =
            |(a, b, _): &(Point, Point, f64)| libm::fabs(libm::cos(bearing(*a, *b) - main));
        let frontage = edges
            .iter()
            .max_by(|x, y| {
                let axis = |(a, b, _): &(Point, Point, f64)| bearing(*a, *b);
                agreed(axis(x))
                    .total_cmp(&agreed(axis(y)))
                    .then(x.2.total_cmp(&y.2))
                    .then(with_main(x).total_cmp(&with_main(y)))
            })
            .map_or(0.0, |(a, b, _)| bearing(*a, *b));
        let lattice = Lattice::cut(pass, network, district, frontage)?;
        let (streets, joints) = lattice.streets(network, district);
        for points in streets {
            lay(network, points, surface, &district.id)?;
        }
        for (joint, arriving) in joints {
            network.join_street(joint, arriving, pass.street_width(surface) / 2.0);
        }
    }
    Ok(())
}

/// Run every street that stops in the open just short of a carriageway on
/// to it: the layout's avenues and lanes in `plan`, and the streets `laid`
/// on it. A street runs straight on to the first carriageway ahead within
/// the presets' run, where it may land on it (near enough square, and
/// clear of that carriageway's junctions or straight across from a street
/// on its far side). Where another street's end faces it on the way, it
/// runs to that end instead of past it: the two are one street, not two
/// drawn side by side. Where nothing crosses its line but another street
/// stops in the open just off it, the two meet in a corner at that end. A
/// country road is the layout's to end.
pub fn run_on(
    pass: &Pass,
    network: &mut Network,
    plan: &MapPlan,
    laid: &mut Vec<SurfaceArea>,
) -> Result<(), Vec<Diagnostic>> {
    let reach = pass.presets.parcels.run_on_m;
    // Each way's kind, in the network's order: the plan's, then the laid.
    let kinds: Vec<SurfaceKind> = plan
        .surfaces
        .iter()
        .chain(laid.iter())
        .filter(|area| area.kind.is_road() && matches!(area.shape, GroundShape::Stroke { .. }))
        .map(|area| area.kind)
        .collect();
    for (own, kind) in kinds.into_iter().enumerate() {
        if kind == SurfaceKind::CountryRoad {
            continue;
        }
        let (samples, width) = (
            &network.ways[own].samples,
            2.0 * network.ways[own].half_width,
        );
        let last = samples.len() - 1;
        for (at, before) in [(samples[0], samples[1]), (samples[last], samples[last - 1])] {
            let on_map = (0..2).all(|k| at[k] > 1.0 && at[k] < plan.size[k] - 1.0);
            if !on_map || network.joined(own, at) {
                continue;
            }
            let toward = scale(sub(at, before), 1.0 / distance(at, before));
            let rules = [
                pass.presets.towns.align_m,
                pass.presets.parcels.junction_clear_m,
            ];
            let met = network.ahead(at, toward, reach).and_then(|met| {
                network
                    .landing(at, &met, toward, rules, width / 2.0, &[])
                    .point()
            });
            let facing = network.facing_end(at, toward, reach).filter(|facing| {
                met.is_none_or(|met| {
                    distance(at, *facing)
                        <= distance(at, met) + pass.presets.parcels.geometry.facing_past_m
                })
            });
            let half = network.ways[own].half_width;
            let within = pass.presets.towns.align_m;
            let to = facing
                .or(met)
                .or_else(|| network.corner_end(own, at, toward, reach))
                .or_else(|| Some(network.short_end(at, toward, reach, within, half)?.point));
            if let Some(to) = to {
                let shape = GroundShape::stroke(vec![at, to], width)
                    .map_err(|message| pass.fail("streets", "$.presets.parcels", &message))?;
                network.add(&shape);
                network.join_street(to, scale(sub(to, at), 1.0 / distance(to, at)), width / 2.0);
                laid.push(SurfaceArea { kind, shape });
            }
        }
    }
    Ok(())
}

/// Cut back every street and avenue of `surfaces` that runs on a short way
/// past its last junction and stops in the open, to that junction: a tail
/// shorter than `parcels.tail_min_m` has no room for a lot of its own, and
/// reads as a street that overshot its corner. One of the surfaces from
/// `laid` on that stops in the open within `parcels.run_on_m` of a
/// carriageway ahead of it (one it might not land on) is cut back until
/// that much ground lies between, a row of lots deep, and to its last
/// junction where that leaves a short tail. A country road is the layout's
/// to end.
pub fn trim_tails(pass: &Pass, surfaces: &mut [SurfaceArea], laid: usize, size: [f64; 2]) {
    let least = pass.presets.parcels.tail_min_m;
    let reach = pass.presets.parcels.run_on_m;
    // (surface, half width, authored points, their box)
    let ways: Vec<(usize, f64, Vec<Point>, [f64; 4])> = surfaces
        .iter()
        .enumerate()
        .filter(|(_, area)| area.kind.is_road())
        .filter_map(|(index, area)| match &area.shape {
            GroundShape::Stroke {
                centerline,
                width_m,
            } => {
                let points = centerline.control_points().to_vec();
                let bounds = contract::ground::limits(&points, *width_m);
                Some((index, width_m / 2.0, points, bounds))
            }
            GroundShape::Polygon { .. } => None,
        })
        .collect();
    // How far along `points` the point nearest `p` lies, and how far off.
    let along = |points: &[Point], p: Point| {
        let (mut run, mut best) = (0.0, (f64::INFINITY, 0.0));
        for pair in points.windows(2) {
            let step = sub(pair[1], pair[0]);
            let span = distance(pair[0], pair[1]);
            let share = (dot(sub(p, pair[0]), step) / (span * span)).clamp(0.0, 1.0);
            let away = distance(p, add(pair[0], scale(step, share)));
            if away < best.0 {
                best = (away, run + share * span);
            }
            run += span;
        }
        best
    };
    let mut cuts: Vec<(usize, Vec<Point>)> = Vec::new();
    for (index, half, points, bounds) in &ways {
        if surfaces[*index].kind == SurfaceKind::CountryRoad {
            continue;
        }
        let length: f64 = points
            .windows(2)
            .map(|pair| distance(pair[0], pair[1]))
            .sum();
        let within = |other: &(usize, f64, Vec<Point>, [f64; 4]), margin: f64| {
            other.0 != *index
                && other.3[0] <= bounds[2] + margin
                && other.3[2] >= bounds[0] - margin
                && other.3[1] <= bounds[3] + margin
                && other.3[3] >= bounds[1] - margin
        };
        let near = |other: &&(usize, f64, Vec<Point>, [f64; 4])| within(other, 0.0);
        // How far ahead of the end `at`, reached from `before`, the nearest
        // carriageway's middle lies, on its line or a little to either side.
        let ahead = |at: Point, before: Point| -> Option<f64> {
            let toward = scale(sub(at, before), 1.0 / distance(at, before));
            let across = [-toward[1], toward[0]];
            let mut gap: Option<f64> = None;
            for aside in [
                -pass.presets.parcels.geometry.ahead_spread,
                0.0,
                pass.presets.parcels.geometry.ahead_spread,
            ] {
                let heading = add(toward, scale(across, aside));
                let heading = scale(heading, 1.0 / libm::hypot(heading[0], heading[1]));
                let (start, end) = (add(at, scale(heading, 0.1)), add(at, scale(heading, reach)));
                for (_, _, other, _) in ways.iter().filter(|other| within(other, reach)) {
                    for cross in other.windows(2) {
                        if let Some((share, _)) = segment_crossing(start, end, cross[0], cross[1]) {
                            let met = share * reach;
                            gap = Some(gap.map_or(met, |known: f64| known.min(met)));
                        }
                    }
                }
            }
            gap
        };
        // Where other carriageways cross it or end on it, along it.
        let mut junctions: Vec<f64> = Vec::new();
        // Whether its first and its last point lie on another carriageway.
        let mut joined = [false; 2];
        for (_, other_half, other, _) in ways.iter().filter(near) {
            for (end, at) in [points[0], points[points.len() - 1]]
                .into_iter()
                .enumerate()
            {
                joined[end] |= along(other, at).0 <= *other_half;
            }
            for end in [other[0], other[other.len() - 1]] {
                let (away, at) = along(points, end);
                if away <= *half {
                    junctions.push(at);
                }
            }
            let mut run = 0.0;
            for pair in points.windows(2) {
                for cross in other.windows(2) {
                    if let Some((share, _)) = segment_crossing(pair[0], pair[1], cross[0], cross[1])
                    {
                        junctions.push(run + share * distance(pair[0], pair[1]));
                    }
                }
                run += distance(pair[0], pair[1]);
            }
        }
        let on_map = |p: Point| (0..2).all(|k| p[k] > 1.0 && p[k] < size[k] - 1.0);
        // The stretch kept: from just before its first junction to just
        // past its last, where the tail beyond is short and open.
        // The point `target` along it.
        let at = |target: f64| {
            let mut run = 0.0;
            for pair in points.windows(2) {
                let span = distance(pair[0], pair[1]);
                if target <= run + span {
                    return add(pair[0], scale(sub(pair[1], pair[0]), (target - run) / span));
                }
                run += span;
            }
            points[points.len() - 1]
        };
        let (mut from, mut to) = (0.0, length);
        // How far an open end is cut back: `place` answers the point that
        // far in from it. To its nearest junction, `tail` along from it,
        // where the tail is short or would be once the end stood clear of
        // the carriageway ahead; and otherwise just clear of that.
        let back = |tail: f64, place: &dyn Fn(f64) -> Point| -> f64 {
            let most = (length - 4.0 * half).max(0.0);
            let mut clear = 0.0;
            while *index >= laid && clear < most {
                match ahead(place(clear), place(clear + 1.0)) {
                    Some(gap) => clear += reach - gap + 1.0,
                    None => break,
                }
            }
            if tail > JOIN_OVERSHOOT_M + 1.0 && tail - clear < least {
                tail - JOIN_OVERSHOOT_M
            } else if clear < most {
                clear
            } else {
                0.0
            }
        };
        let count = points.len();
        if !joined[0] && on_map(points[0]) {
            let first = junctions.iter().copied().fold(f64::INFINITY, f64::min);
            from = back(first, &|cut| at(cut));
        }
        if !joined[1] && on_map(points[count - 1]) {
            let last = junctions.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            to = length - back(length - last, &|cut| at(length - cut));
        }
        if (from == 0.0 && to == length) || to - from < 4.0 * half {
            continue;
        }
        let mut kept = vec![round_cm(at(from))];
        let mut run = 0.0;
        for pair in points.windows(2) {
            run += distance(pair[0], pair[1]);
            if run > from + 1.0 && run < to - 1.0 {
                kept.push(pair[1]);
            }
        }
        kept.push(round_cm(at(to)));
        cuts.push((*index, kept));
    }
    for (index, points) in cuts {
        let GroundShape::Stroke { width_m, .. } = surfaces[index].shape else {
            continue;
        };
        if let Ok(shape) = GroundShape::stroke(points, width_m) {
            surfaces[index].shape = shape;
        }
    }
}

/// How far one line of a grid is moved off its own place, by where along it:
/// a straight line through the two places its ends were moved to.
#[derive(Clone, Copy, Default)]
struct Tilt {
    at: f64,
    shift: f64,
    slope: f64,
}

impl Tilt {
    /// Through `(where along, how far aside)` at each end.
    fn through(a: [f64; 2], b: [f64; 2]) -> Self {
        let run = b[0] - a[0];
        if run.abs() < 1.0 {
            Self {
                at: a[0],
                shift: (a[1] + b[1]) / 2.0,
                slope: 0.0,
            }
        } else {
            Self {
                at: a[0],
                shift: a[1],
                slope: (b[1] - a[1]) / run,
            }
        }
    }

    fn value(&self, along: f64) -> f64 {
        self.shift + self.slope * (along - self.at)
    }
}

/// How one family of a district's streets swings off its straight lines: one
/// slow wave along the street, reaching farther on one side of the district
/// than on the other, so no two streets bend alike and some run straight.
#[derive(Clone, Copy, Default)]
struct Bow {
    /// How far the swing reaches at the district's two sides across the
    /// family, either way.
    reach: [f64; 2],
    /// Those two sides, in the frame.
    span: [f64; 2],
    wave: f64,
    phase: f64,
}

impl Bow {
    fn at(&self, along: f64, across: f64) -> f64 {
        let width = self.span[1] - self.span[0];
        let share = if width > 1.0 {
            ((across - self.span[0]) / width).clamp(0.0, 1.0)
        } else {
            0.5
        };
        (self.reach[0] + (self.reach[1] - self.reach[0]) * share)
            * libm::sin(self.wave * along + self.phase)
    }
}

/// A district's own frame: along its long streets and across them, with the
/// swing each street family takes.
struct Frame {
    origin: Point,
    /// Unit vectors along the long streets and across them.
    axes: [Point; 2],
    /// The long streets' swing and the cross streets'.
    bows: [Bow; 2],
}

impl Frame {
    fn local(&self, p: Point) -> Point {
        let d = sub(p, self.origin);
        [dot(d, self.axes[0]), dot(d, self.axes[1])]
    }

    /// How far the swing carries the frame's point `[u, v]`.
    fn swing(&self, [u, v]: Point) -> Point {
        [self.bows[1].at(v, u), self.bows[0].at(u, v)]
    }

    /// The map's point for the frame's `local`, swung, on the plan's grid.
    fn place(&self, local: Point) -> Point {
        let [u, v] = add(local, self.swing(local));
        round_cm(add(
            self.origin,
            add(scale(self.axes[0], u), scale(self.axes[1], v)),
        ))
    }

    /// The frame's point that `place` carries to the map's point `p`, near
    /// enough: the swing is gentle, so it is much the same there.
    fn unplace(&self, p: Point) -> Point {
        let local = self.local(p);
        sub(local, self.swing(sub(local, self.swing(local))))
    }
}

/// Where one end of a line of the grid stops.
#[derive(Clone, Copy)]
enum Stop {
    /// On a carriageway, at this point; `true` where a street already ends
    /// there and the line was moved to meet it.
    Met(Point, bool),
    /// On a carriageway it would have come to at a slant, at this point
    /// square to it.
    Turned(Point),
    /// At its last crossing: a carriageway lies ahead that it may not land
    /// on.
    Refused,
    /// In the open, at this point by the district's edge.
    Open(Point),
    /// At the line's last node.
    Node,
}

/// One line of the grid: its nodes in order, each end's stop among them,
/// and which neighbours a street joins.
struct Line {
    long: bool,
    nodes: Vec<usize>,
    kept: Vec<bool>,
}

/// One district's street grid: nodes a step apart along its long streets and
/// its cross streets, and the stretches between neighbouring nodes that lie
/// in the district and do not repeat a carriageway already there.
struct Lattice {
    /// Each node's place on the map; only nodes on a street are used.
    positions: Vec<Point>,
    lines: Vec<Line>,
    /// The stretches at each node: the node at its other end, and whether
    /// it is a long street's.
    links: Vec<Vec<(usize, bool)>>,
    /// Nodes of stretches that cross or end on a carriageway already laid.
    seeds: Vec<usize>,
    /// Nodes where a line ends on a carriageway, and where one ends in the
    /// open at the district's edge.
    met: BTreeSet<usize>,
    open: BTreeSet<usize>,
    /// Nodes where a street turns to meet a carriageway square.
    bends: BTreeSet<usize>,
    /// A straight grid is authored by its streets' ends; a bent one by
    /// every node.
    straight: bool,
    /// The grid's four headings.
    headings: [Point; 4],
    /// The shortest link that is a street of its own, and how far one may
    /// reach.
    link_least: f64,
    link_reach: f64,
    /// How far a link's end is moved to meet a street across the road, and
    /// how clear of a road's other junctions it lands.
    landing: [f64; 2],
    /// Half its streets' width.
    half_width: f64,
}

impl Lattice {
    fn cut(
        pass: &Pass,
        network: &Network,
        district: &DistrictPlan,
        axis: f64,
    ) -> Result<Self, Vec<Diagnostic>> {
        let pattern = pass.district(district)?.streets;
        let parcels = &pass.presets.parcels;
        let step = parcels.street_step_m;
        let width = pass.street_width(pattern.surface);
        let mut rng = pass.stream(&format!("streets/{}", district.id));
        let mut frame = Frame {
            origin: district.anchor,
            axes: [direction(axis), direction(axis + TAU / 4.0)],
            bows: [Bow::default(); 2],
        };
        let [along, across] = frame.axes;
        // The district's reach in its own frame, and how much of each of
        // its four sides a carriageway runs along.
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for vertex in &district.ring {
            let local = frame.local(*vertex);
            for i in 0..2 {
                low[i] = low[i].min(local[i]);
                high[i] = high[i].max(local[i]);
            }
        }
        // Each family bows once or less along the district, by an amount
        // that changes from one side of it to the other.
        if let Some(bend) = pattern.bend {
            frame.bows = [0, 1].map(|family| {
                let extent = high[family] - low[family];
                let length = rng.range(pass.presets.parcels.geometry.bow_lengths) * extent;
                let most = bend
                    .amplitude_m
                    .min(pass.presets.parcels.geometry.bow_reach * extent);
                Bow {
                    reach: [0, 1].map(|_| (2.0 * rng.unit() - 1.0) * most),
                    span: [low[1 - family], high[1 - family]],
                    wave: TAU / length.max(bend.wavelength_m),
                    phase: rng.range([0.0, TAU]),
                }
            });
        }
        let frame = frame;
        let mut carried = [[0.0; 2]; 2];
        let mut total = [[0.0; 2]; 2];
        for (a, b) in contract::ground::edges(&district.ring) {
            let (from, to) = (frame.local(*a), frame.local(*b));
            // An edge that runs more along the frame than across bounds
            // the district across it.
            let bounds = usize::from((to[0] - from[0]).abs() >= (to[1] - from[1]).abs());
            let middle = (low[bounds] + high[bounds]) / 2.0;
            let side = usize::from((from[bounds] + to[bounds]) / 2.0 > middle);
            total[bounds][side] += distance(*a, *b);
            if network.runs_along(*a, *b, false) {
                carried[bounds][side] += distance(*a, *b);
            }
        }
        // Streets are fitted between the district's edges: a whole block
        // from a carriageway on an edge to the first street, and half a
        // block from an edge that faces the fields, where the last street
        // has a row of lots on either side. So no street lies a few metres
        // inside a road, and no back land is left behind the last one.
        // (first line's place less one block, blocks, nodes a block, node
        // spacing)
        let fit = |axis: usize, block: f64| {
            let open = |side: usize| 2.0 * carried[axis][side] < total[axis][side];
            let reach = high[axis] - low[axis];
            // The blocks the district's edges close: one between two
            // carriageways, half of one beside each open edge.
            let closed =
                1.0 - 0.5 * f64::from(u8::from(open(0))) - 0.5 * f64::from(u8::from(open(1)));
            // A block between two carriageways may be a quarter deeper than
            // the preset before a street cuts it in two. Ground with no
            // carriageway on either side gets its one street when it is
            // three quarters of a block across: a row of lots each side.
            let streets = libm::round(reach / block - closed / 2.0 - 0.25).max(0.0);
            if streets == 0.0 {
                // One block, with nodes across it for the other streets.
                let per = (libm::round(reach / step) as usize).max(1);
                return (low[axis], 1, per, reach / per as f64);
            }
            let spacing = reach / (streets + closed);
            let from = low[axis] - if open(0) { spacing / 2.0 } else { 0.0 };
            let per = (libm::round(spacing / step) as usize).max(1);
            (from, streets as usize + 1, per, spacing / per as f64)
        };
        let (u0, blocks_u, per_u, su) = fit(0, pattern.block_length_m);
        let (v0, blocks_v, per_v, sv) = fit(1, pattern.block_depth_m);
        let (columns, rows) = (blocks_u * per_u + 1, blocks_v * per_v + 1);
        let on_long = |row: usize| row.is_multiple_of(per_v) && row > 0 && row < rows - 1;
        let on_cross =
            |column: usize| column.is_multiple_of(per_u) && column > 0 && column < columns - 1;
        let index = |column: usize, row: usize| row * columns + column;
        // A node's place in the frame, with its lines moved by their tilts.
        let local = |column: usize, row: usize, rows: &[Tilt], columns: &[Tilt]| {
            let (u, v) = (u0 + column as f64 * su, v0 + row as f64 * sv);
            match (on_long(row), on_cross(column)) {
                (true, true) => {
                    // Where the two moved lines cross.
                    let (r, c) = (rows[row], columns[column]);
                    let moved = (u + c.shift + c.slope * (v + r.shift - r.slope * r.at - c.at))
                        / (1.0 - c.slope * r.slope);
                    [moved, v + r.value(moved)]
                }
                (true, false) => [u, v + rows[row].value(u)],
                (false, true) => [u + columns[column].value(v), v],
                (false, false) => [u, v],
            }
        };
        let mut row_tilts = vec![Tilt::default(); rows];
        let mut column_tilts = vec![Tilt::default(); columns];
        let place_all = |row_tilts: &[Tilt], column_tilts: &[Tilt]| {
            let mut positions = vec![district.anchor; columns * rows];
            for row in 0..rows {
                for column in 0..columns {
                    if on_long(row) || on_cross(column) {
                        positions[index(column, row)] =
                            frame.place(local(column, row, row_tilts, column_tilts));
                    }
                }
            }
            positions
        };
        let mut positions = place_all(&row_tilts, &column_tilts);
        // A node on the district's very edge lies on the carriageway there:
        // the line stops on that, not at the node.
        let within_district = |positions: &[Point]| -> Vec<bool> {
            (0..columns * rows)
                .map(|node| {
                    let (column, row) = (node % columns, node / columns);
                    (on_long(row) || on_cross(column))
                        && polygon_contains(&district.ring, positions[node])
                        && contract::ground::edges(&district.ring)
                            .all(|(a, b)| segment_distance(*a, *b, positions[node]) >= width / 2.0)
                })
                .collect()
        };
        let inside = within_district(&positions);

        // Each line's run of nodes inside the district, and where its two
        // ends stop: on the carriageway ahead, where one lies on the
        // district's edge or within a short run past it, and otherwise in
        // the open just inside the edge.
        let run_on = parcels.run_on_m;
        let clear = parcels.junction_clear_m;
        // A line is moved no farther than `within` to meet a street across
        // the road, and never to a junction of `taken`: one a line of this
        // grid already runs to.
        let stop = |positions: &[Point],
                    from: usize,
                    behind: Option<usize>,
                    outward: Point,
                    within: f64,
                    taken: &[Point]| {
            let p = positions[from];
            // A bent street heads the way its last stretch runs, and
            // failing that along the grid; a straight one along the grid.
            let bent = behind
                .filter(|behind| pattern.bend.is_some() && positions[*behind] != p)
                .map(|behind| {
                    scale(
                        sub(p, positions[behind]),
                        1.0 / distance(p, positions[behind]),
                    )
                });
            let headings = bent.into_iter().chain([outward]);
            let exits = headings.map(|heading| (heading, ray_exit(&district.ring, p, heading)));
            // The carriageway ahead. (where, heading)
            let met = exits.clone().find_map(|(heading, exit)| {
                Some((network.ahead(p, heading, exit + run_on)?, heading))
            });
            // Where nothing crosses its line, a carriageway that stops in the
            // open just off it: the line is moved to carry on from that end.
            let open_end = || {
                exits
                    .clone()
                    .find_map(|(heading, exit)| {
                        network.open_end(p, heading, exit + run_on, within, width / 2.0)
                    })
                    .filter(|end| !taken.contains(end))
            };
            // Or a carriageway that stops just short of its line: the line
            // is moved to meet it before its end.
            let short_end = || {
                exits.clone().find_map(|(heading, exit)| {
                    let met = network.short_end(p, heading, exit + run_on, within, width / 2.0)?;
                    Some(met.point)
                })
            };
            match (met, exits.clone().next()) {
                (Some((met, heading)), _) => {
                    match network.landing(p, &met, heading, [within, clear], width / 2.0, taken) {
                        Landing::Opposite(joint) => Stop::Met(joint, true),
                        Landing::Free(at) => Stop::Met(at, false),
                        Landing::Turned(at) => Stop::Turned(at),
                        Landing::Refused => Stop::Refused,
                    }
                }
                (None, _) if open_end().is_some() => Stop::Met(open_end().unwrap_or(p), true),
                (None, _) if short_end().is_some() => Stop::Met(short_end().unwrap_or(p), true),
                (None, Some((heading, exit)))
                    if exit - width / 2.0 - pass.presets.parcels.geometry.end_margin_m >= step =>
                {
                    Stop::Open(round_cm(add(
                        p,
                        scale(
                            heading,
                            exit - width / 2.0 - pass.presets.parcels.geometry.end_margin_m,
                        ),
                    )))
                }
                _ => Stop::Node,
            }
        };
        // (whether a long street, its row or column, first and last node
        // inside, the two stops)
        let mut runs: Vec<(bool, usize, [usize; 2], [Stop; 2])> = Vec::new();
        let mut taken: Vec<Point> = Vec::new();
        // Where this grid's streets come to a carriageway.
        let mut landings: Vec<Point> = Vec::new();
        let one_block = blocks_u == 1 && blocks_v == 1;
        for (long, count, across_count) in [(true, rows, columns), (false, columns, rows)] {
            for line in (0..count).filter(|line| {
                !one_block
                    && if long {
                        on_long(*line)
                    } else {
                        on_cross(*line)
                    }
            }) {
                let node = |at: usize| {
                    if long {
                        index(at, line)
                    } else {
                        index(line, at)
                    }
                };
                let (Some(first), Some(last)) = (
                    (0..across_count).find(|at| inside[node(*at)]),
                    (0..across_count).rfind(|at| inside[node(*at)]),
                ) else {
                    continue;
                };
                let axis = if long { along } else { across };
                // Less than half way to the next line of its own family.
                let between = if long {
                    sv * per_v as f64
                } else {
                    su * per_u as f64
                };
                let within = pass.presets.towns.align_m.min(0.4 * between);
                let mut stops = [Stop::Node; 2];
                for (end, (from, behind, outward)) in [
                    (first, first + 1, scale(axis, -1.0)),
                    (last, last.wrapping_sub(1), axis),
                ]
                .into_iter()
                .enumerate()
                {
                    let behind = (first < last).then(|| node(behind));
                    stops[end] = stop(&positions, node(from), behind, outward, within, &taken);
                    if let Stop::Met(at, true) = stops[end] {
                        taken.push(at);
                    }
                    // Two streets of one grid do not come to a road within
                    // a few widths of each other: the second stops at its
                    // last crossing.
                    if let Stop::Met(at, _) | Stop::Turned(at) = stops[end] {
                        if landings.iter().any(|known| {
                            distance(*known, at)
                                < pass.presets.parcels.geometry.crowd_widths * width
                        }) {
                            stops[end] = Stop::Refused;
                        } else {
                            landings.push(at);
                        }
                    }
                }
                // A line whose end was moved to a street across the road
                // is tilted to pass through that point.
                if stops.iter().any(|stop| matches!(stop, Stop::Met(_, true))) {
                    let (lengthwise, sideways) = if long { (0, 1) } else { (1, 0) };
                    let own = if long {
                        v0 + line as f64 * sv
                    } else {
                        u0 + line as f64 * su
                    };
                    let control = |stop: &Stop, end: usize| match stop {
                        Stop::Met(at, true) => {
                            let at = frame.unplace(*at);
                            [at[lengthwise], at[sideways] - own]
                        }
                        Stop::Met(at, false) | Stop::Open(at) => {
                            [frame.unplace(*at)[lengthwise], 0.0]
                        }
                        Stop::Turned(_) | Stop::Refused | Stop::Node => {
                            let spacing = if long { su } else { sv };
                            let start = if long { u0 } else { v0 };
                            [start + end as f64 * spacing, 0.0]
                        }
                    };
                    let tilt = Tilt::through(control(&stops[0], first), control(&stops[1], last));
                    if long {
                        row_tilts[line] = tilt;
                    } else {
                        column_tilts[line] = tilt;
                    }
                }
                runs.push((long, line, [first, last], stops));
            }
        }
        positions = place_all(&row_tilts, &column_tilts);
        // What lies in the district now that its lines are moved, and each
        // line's run of it.
        let inside = within_district(&positions);
        runs.retain_mut(|(long, line, ends, _)| {
            let count = if *long { columns } else { rows };
            let node = |at: usize| {
                if *long {
                    index(at, *line)
                } else {
                    index(*line, at)
                }
            };
            match (
                (0..count).find(|at| inside[node(*at)]),
                (0..count).rfind(|at| inside[node(*at)]),
            ) {
                (Some(first), Some(last)) => {
                    *ends = [first, last];
                    true
                }
                _ => false,
            }
        });

        // Cross streets left out, block by block.
        let mut skipped = BTreeSet::new();
        for column in (0..columns).filter(|column| on_cross(*column)) {
            for band in 0..blocks_v {
                if rng.chance(pattern.cross_skip) {
                    skipped.insert((column, band));
                }
            }
        }
        let reach = libm::hypot(high[0] - low[0], high[1] - low[1]) + run_on;
        let mut lattice = Self {
            positions,
            lines: Vec::new(),
            links: vec![Vec::new(); columns * rows],
            seeds: Vec::new(),
            met: BTreeSet::new(),
            open: BTreeSet::new(),
            bends: BTreeSet::new(),
            straight: pattern.bend.is_none(),
            headings: [along, scale(along, -1.0), across, scale(across, -1.0)],
            link_least: step,
            link_reach: reach,
            landing: [pass.presets.towns.align_m, clear],
            half_width: width / 2.0,
        };
        // A carriageway nearer beside a street than a row of lots is deep
        // is one the street only repeats.
        let clearance = pattern.block_depth_m / 3.0;
        // The stretches between the grid's own nodes first.
        for (long, line, [first, last], _) in &runs {
            let node = |at: usize| {
                if *long {
                    index(at, *line)
                } else {
                    index(*line, at)
                }
            };
            let nodes: Vec<usize> = (*first..=*last).map(node).collect();
            let kept = (*first..*last)
                .map(|at| {
                    let (i, j) = (node(at), node(at + 1));
                    let left_out = !*long && skipped.contains(&(*line, at / per_v));
                    if left_out || !inside[i] || !inside[j] {
                        return false;
                    }
                    let (repeats, crosses) =
                        network.meets(lattice.positions[i], lattice.positions[j], clearance);
                    if repeats {
                        return false;
                    }
                    if crosses {
                        lattice.seeds.push(i);
                    }
                    lattice.links[i].push((j, *long));
                    lattice.links[j].push((i, *long));
                    true
                })
                .collect();
            lattice.lines.push(Line {
                long: *long,
                nodes,
                kept,
            });
        }
        // Then each line's ends, from its last node on a street to where it
        // stops. An end left out as a cross street between two long ones is
        // left out here too.
        let mut refused: Vec<(usize, bool)> = Vec::new();
        for (number, (long, line, [first, last], stops)) in runs.iter().enumerate() {
            for (end, stop) in stops.iter().enumerate() {
                let (at, last_node) = match end {
                    0 => (*first, lattice.lines[number].nodes[0]),
                    _ => (*last, *lattice.lines[number].nodes.last().unwrap()),
                };
                let band = if end == 0 { at.saturating_sub(1) } else { at } / per_v;
                let (to, met, turned) = match stop {
                    Stop::Met(to, _) => (*to, true, false),
                    Stop::Turned(to) => (*to, true, true),
                    // A long street runs on to the last lots at the town's
                    // edge; a cross street ends on the last long street,
                    // where it has no lot of its own to serve beyond.
                    Stop::Open(to) if *long => {
                        let from = lattice.positions[last_node];
                        let heading = scale(sub(*to, from), 1.0 / distance(*to, from));
                        if network.short_of(*to, heading, run_on) {
                            refused.push((number, end == 0));
                            continue;
                        }
                        (*to, false, false)
                    }
                    Stop::Refused => {
                        refused.push((number, end == 0));
                        continue;
                    }
                    Stop::Open(_) | Stop::Node => continue,
                };
                let from = lattice.positions[last_node];
                // A run that would be the whole of its street is one only
                // if it is long enough to be a street.
                let alone = !lattice.links[last_node]
                    .iter()
                    .any(|(_, other)| other == long);
                if lattice.links[last_node].is_empty()
                    || (!*long && skipped.contains(&(*line, band.min(blocks_v - 1))))
                    || distance(from, to) < pass.presets.parcels.geometry.end_margin_m
                    || (alone && distance(from, to) < pass.presets.parcels.geometry.alone_widths * width)
                    // A run to a carriageway is no repeat of it; a run to
                    // the district's edge beside one is.
                    || (!met && network.meets(from, to, clearance).0)
                {
                    continue;
                }
                lattice.stub(number, end == 0, last_node, to, met, turned);
            }
        }
        // A street the grid left stopped inside the district, where its next
        // stretch would only have repeated a carriageway, runs on to the
        // carriageway ahead of it where it may land on it.
        for number in 0..lattice.lines.len() {
            for first in [true, false] {
                let line = &lattice.lines[number];
                let end = if first {
                    line.kept
                        .iter()
                        .position(|kept| *kept)
                        .map(|at| (at, at + 1))
                } else {
                    line.kept
                        .iter()
                        .rposition(|kept| *kept)
                        .map(|at| (at + 1, at))
                };
                let Some((node, inner)) =
                    end.map(|(at, inner)| (line.nodes[at], line.nodes[inner]))
                else {
                    continue;
                };
                if lattice.links[node].len() != 1
                    || lattice.met.contains(&node)
                    || lattice.open.contains(&node)
                {
                    continue;
                }
                let (p, q) = (lattice.positions[node], lattice.positions[inner]);
                let heading = scale(sub(p, q), 1.0 / distance(p, q));
                let within = pass.presets.towns.align_m;
                let rules = [within, clear];
                let mut turned = false;
                let onward = match network.ahead(p, heading, run_on) {
                    Some(met) => {
                        let landing = network.landing(p, &met, heading, rules, width / 2.0, &taken);
                        if landing.point().is_none() {
                            refused.push((number, first));
                        }
                        turned = matches!(landing, Landing::Turned(_));
                        landing.point()
                    }
                    None => network.gentle_end(p, heading, run_on, width / 2.0),
                };
                if onward.is_none() && network.short_of(p, heading, run_on) {
                    refused.push((number, first));
                }
                let crowded = |to: &Point| {
                    landings.iter().any(|known| {
                        distance(*known, *to) < pass.presets.parcels.geometry.crowd_widths * width
                    })
                };
                if let Some(to) = onward.filter(|to| !crowded(to)) {
                    landings.push(to);
                    lattice.stub(number, first, node, to, true, turned);
                }
            }
        }
        // A street that may not land on the carriageway ahead of it ends at
        // its last crossing: the ground beyond fronts that carriageway.
        for (number, first) in refused {
            lattice.retreat(network, number, first, run_on);
        }
        let links = &lattice.links;
        lattice.seeds.retain(|node| !links[*node].is_empty());
        Ok(lattice)
    }

    /// Cut the first (or last) end of line `number` back to its last
    /// crossing with a street of the other family, or, where none crosses
    /// it so near, to where no carriageway lies within `reach` ahead of it:
    /// a row of lots then stands between its end and that carriageway. A
    /// street that is all within that reach goes altogether: its ground
    /// fronts the carriageways at either end of it.
    fn retreat(&mut self, network: &Network, number: usize, first: bool, reach: f64) {
        let line = &self.lines[number];
        let long = line.long;
        let crossed = |node: usize| self.links[node].iter().any(|(_, other)| *other != long);
        // The stretches from that end in, up to the node it ends at.
        let mut dropped: Vec<usize> = Vec::new();
        let stretches: Vec<usize> = if first {
            (0..line.kept.len()).collect()
        } else {
            (0..line.kept.len()).rev().collect()
        };
        for stretch in stretches.into_iter().skip_while(|at| !line.kept[*at]) {
            if !line.kept[stretch] {
                break;
            }
            let (outer, inner) = if first {
                (line.nodes[stretch], line.nodes[stretch + 1])
            } else {
                (line.nodes[stretch + 1], line.nodes[stretch])
            };
            // An end that found a landing after all keeps it.
            if dropped.is_empty() && (self.met.contains(&outer) || self.open.contains(&outer)) {
                return;
            }
            let (p, q) = (self.positions[outer], self.positions[inner]);
            let heading = scale(sub(p, q), 1.0 / distance(p, q));
            if crossed(outer) || !network.short_of(p, heading, reach) {
                break;
            }
            dropped.push(stretch);
        }
        for stretch in dropped {
            let (a, b) = (
                self.lines[number].nodes[stretch],
                self.lines[number].nodes[stretch + 1],
            );
            self.lines[number].kept[stretch] = false;
            self.links[a].retain(|(other, _)| *other != b);
            self.links[b].retain(|(other, _)| *other != a);
            // The far end's own landing goes with the street.
            for node in [a, b] {
                if self.links[node].is_empty() {
                    self.met.remove(&node);
                    self.open.remove(&node);
                }
            }
        }
    }

    /// Run the end `from` of line `number` (its first, or its last) on to
    /// `to`: on a carriageway where `met`, and round a turn at `from` where
    /// `turned`.
    fn stub(
        &mut self,
        number: usize,
        first: bool,
        from: usize,
        to: Point,
        met: bool,
        turned: bool,
    ) {
        let long = self.lines[number].long;
        let node = self.positions.len();
        self.positions.push(to);
        self.links.push(vec![(from, long)]);
        self.links[from].push((node, long));
        if met {
            self.met.insert(node);
            self.seeds.push(node);
        } else {
            self.open.insert(node);
        }
        if turned {
            self.bends.insert(from);
        }
        let line = &mut self.lines[number];
        let at = line
            .nodes
            .iter()
            .position(|other| *other == from)
            .unwrap_or(0);
        if first {
            line.nodes.insert(at, node);
            line.kept.insert(at, true);
        } else {
            line.nodes.insert(at + 1, node);
            line.kept.insert(at, true);
        }
    }

    /// The node `from` and every node joined to it by stretches of street,
    /// marked `joined`.
    fn piece(&self, from: usize, joined: &mut [bool]) -> Vec<usize> {
        let mut piece = vec![from];
        let mut next = 0;
        joined[from] = true;
        while let Some(at) = piece.get(next).copied() {
            next += 1;
            for (end, _) in &self.links[at] {
                if !joined[*end] {
                    joined[*end] = true;
                    piece.push(*end);
                }
            }
        }
        piece
    }

    /// The district's streets as authored points: the grid's own, and the
    /// links that join every piece of it to the network; and the points
    /// where they end on a carriageway.
    fn streets(
        &self,
        network: &Network,
        district: &DistrictPlan,
    ) -> (Vec<Vec<Point>>, Vec<(Point, Point)>) {
        let mut streets: Vec<Vec<Point>> = Vec::new();
        // (where, arriving along): each end on a carriageway.
        let mut joints: Vec<(Point, Point)> = self
            .met
            .iter()
            .map(|node| {
                let (at, from) = (
                    self.positions[*node],
                    self.positions[self.links[*node][0].0],
                );
                (at, scale(sub(at, from), 1.0 / distance(at, from)))
            })
            .collect();
        // A link from `from` to the nearest carriageway. It ends on a
        // segment's own end exactly, unrounded (running past a dead end
        // would cross nothing), and otherwise runs just past the centreline
        // it meets.
        let nearest = |from: Point| {
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
        let empty = self.lines.iter().all(|line| !line.kept.contains(&true));
        if empty && !network.nearest(district.anchor).is_some_and(served) {
            streets.extend(nearest(district.anchor));
        }
        // Every piece of the grid is joined: the pieces a road crosses or
        // that end on one already are, and each other piece gets one link.
        // A link is a street of the grid carried on: it leaves one of the
        // piece's nodes along the grid and runs to the first carriageway
        // ahead, the shortest such run that is long enough to be a street.
        // Only where the grid's own headings meet nothing does a link take
        // the straight line to the nearest street. The nodes a link starts
        // or ends on are authored points of the streets through them, so it
        // meets them exactly.
        let mut pinned = BTreeSet::new();
        let mut joined = vec![false; self.positions.len()];
        let mut joined_nodes: Vec<usize> = Vec::new();
        for seed in &self.seeds {
            if !joined[*seed] {
                joined_nodes.extend(self.piece(*seed, &mut joined));
            }
        }
        for start in 0..self.positions.len() {
            if joined[start] || self.links[start].is_empty() {
                continue;
            }
            let piece = self.piece(start, &mut joined);
            // (length, node of this piece, where it ends, the node there)
            let mut along_grid: Option<(f64, usize, Point, Option<usize>)> = None;
            for from in &piece {
                let p = self.positions[*from];
                for heading in self.headings {
                    // Not back along a street the node is already on.
                    let taken = self.links[*from].iter().any(|(other, _)| {
                        let out = sub(self.positions[*other], p);
                        dot(out, heading) > 0.5 * distance(self.positions[*other], p)
                    });
                    if taken {
                        continue;
                    }
                    let met = network.ahead(p, heading, self.link_reach);
                    let reach = met
                        .as_ref()
                        .map_or(self.link_reach, |met| distance(p, met.point));
                    // Where it may land on the carriageway ahead.
                    let met = met.and_then(|met| {
                        network
                            .landing(p, &met, heading, self.landing, self.half_width, &[])
                            .point()
                    });
                    // A node of the grid already joined, on the way there.
                    let own = joined_nodes
                        .iter()
                        .map(|node| (*node, sub(self.positions[*node], p)))
                        .filter(|(_, offset)| {
                            let ahead = dot(*offset, heading);
                            let aside = (offset[0] * heading[1] - offset[1] * heading[0]).abs();
                            ahead > 1.0 && ahead < reach && aside <= network.policy.own_tan * ahead
                        })
                        .min_by(|a, b| dot(a.1, heading).total_cmp(&dot(b.1, heading)));
                    let (gap, to, node) = match (own, met) {
                        (Some((node, offset)), _) => {
                            (dot(offset, heading), self.positions[node], Some(node))
                        }
                        (None, Some(to)) => (reach, to, None),
                        (None, None) => continue,
                    };
                    // A street of the piece that ends here and runs on is
                    // that street, however short the run; one that turns off
                    // here is a street of its own, and must be long enough
                    // to be one.
                    let runs_on = self.links[*from].iter().any(|(other, _)| {
                        let out = sub(self.positions[*other], p);
                        dot(out, heading) < -0.5 * distance(self.positions[*other], p)
                    });
                    let least = if node.is_some() || runs_on {
                        1.0
                    } else {
                        self.link_least
                    };
                    if gap >= least && along_grid.is_none_or(|(known, ..)| gap < known) {
                        along_grid = Some((gap, *from, to, node));
                    }
                }
            }
            if let Some((_, from, to, node)) = along_grid {
                pinned.insert(from);
                let p = self.positions[from];
                streets.push(vec![p, to]);
                match node {
                    Some(node) => {
                        pinned.insert(node);
                    }
                    None => joints.push((to, scale(sub(to, p), 1.0 / distance(to, p)))),
                }
            } else {
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
                        None => streets.extend(nearest(self.positions[from])),
                    }
                }
            }
            joined_nodes.extend(piece);
        }
        // A street is a run of stretches along one line.
        for line in &self.lines {
            let mut from = 0;
            while from < line.kept.len() {
                if !line.kept[from] {
                    from += 1;
                    continue;
                }
                let to = (from..line.kept.len())
                    .find(|at| !line.kept[*at])
                    .unwrap_or(line.kept.len());
                let nodes = &line.nodes[from..=to];
                let points = self.street(nodes, line.long, &pinned);
                from = to;
                if points.len() < 2 {
                    continue;
                }
                streets.push(points);
            }
        }
        (streets, joints)
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
            .filter(|i| {
                !self.straight || ends.contains(i) || pinned.contains(i) || self.bends.contains(i)
            })
            .map(|i| self.positions[*i])
            .collect();
        points.dedup();
        if points.len() < 2 {
            return points;
        }
        let last = points.len() - 1;
        for (at, (end, toward)) in ends.into_iter().zip([(0, 1), (last, last - 1)]) {
            let meets = self.links[at]
                .iter()
                .filter(|(_, other)| *other != long)
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
