//! A settlement's ground as it is cut into blocks: along the roads that
//! cross it first, then into rows a block deep behind each frontage and
//! across each row. Every piece is convex, so a cut is one straight line.
use crate::layout::geometry::{
    add, area, centroid, cross, distance, dot, length, scale, sub, Point,
};
use crate::layout::presets::{BlockRule, Towns};
use crate::layout::rng::Stream;
use crate::layout::roads::Road;
use contract::map::SurfaceKind;

/// Two points nearer than this are one corner: two leaves each work out
/// where a cut crosses the edge between them, a hair apart.
pub(super) const CORNER_M: f64 = 5e-3;
/// A cut that passes a corner nearer than this passes through it: the plan
/// is on a centimetre grid, and a corner and a cut a millimetre apart would
/// be two corners to one leaf and one to its neighbour.
pub(super) const SNAP_M: f64 = 0.01;
/// A piece of ground smaller than this is no piece at all.
const SLIVER_M2: f64 = 1.0;

/// Where a cut ends on another.
#[derive(Clone, Copy)]
pub(super) struct Tee {
    /// How far along the cut it ends on.
    at: f64,
    /// Whether it comes from that cut's left.
    from_left: bool,
    /// The unit vector it runs along.
    along: Point,
    /// Whether it is a road ending on a road's line: a junction of the
    /// settlement's roads, which the cuts between blocks keep clear of.
    road: bool,
}

/// The least and the greatest of `values`.
fn interval(values: impl Iterator<Item = f64>) -> [f64; 2] {
    values.fold([f64::INFINITY, f64::NEG_INFINITY], |[low, high], value| {
        [low.min(value), high.max(value)]
    })
}

/// One straight line the ground was cut along.
pub(super) struct Cut {
    pub(super) origin: Point,
    /// Unit vector along it.
    pub(super) along: Point,
    /// The best road that runs along it, where one does.
    pub(super) road: Option<SurfaceKind>,
    /// The stretches a road covers, as distances along it with the road's
    /// own points at their ends.
    pub(super) paved: Vec<([f64; 2], [Point; 2])>,
    /// Where other cuts end on it: its junctions.
    pub(super) tees: Vec<Tee>,
}

impl Cut {
    pub(super) fn at(&self, p: Point) -> f64 {
        dot(sub(p, self.origin), self.along)
    }
    pub(super) fn aside(&self, p: Point) -> f64 {
        cross(self.along, sub(p, self.origin))
    }
}

/// What bounds a piece of ground along one edge.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Bound {
    /// The settlement's own limit.
    Limit,
    Cut(usize),
}

/// A convex piece of the settlement's ground, counter-clockwise.
/// `bounds[i]` is what lies along the edge from `ring[i]`.
#[derive(Clone)]
pub(super) struct Leaf {
    pub(super) ring: Vec<Point>,
    pub(super) bounds: Vec<Bound>,
}

impl Leaf {
    pub(super) fn edge(&self, index: usize) -> (Point, Point) {
        (self.ring[index], self.ring[(index + 1) % self.ring.len()])
    }

    /// The part of the leaf left (or right) of the line through `origin`
    /// along `along`, with `bound` along the new edge.
    pub(super) fn clipped(
        &self,
        origin: Point,
        along: Point,
        left: bool,
        bound: Bound,
    ) -> Option<Leaf> {
        let side = |p: Point| {
            let aside = cross(along, sub(p, origin));
            if aside.abs() < SNAP_M {
                0.0
            } else if left {
                aside
            } else {
                -aside
            }
        };
        let count = self.ring.len();
        let mut ring: Vec<Point> = Vec::with_capacity(count + 2);
        let mut bounds: Vec<Bound> = Vec::with_capacity(count + 2);
        let mut push = |p: Point, bound: Bound| {
            // A corner on the line is entered and left at one point: the
            // later edge is the one that leaves it.
            if ring
                .last()
                .is_some_and(|last| distance(*last, p) < CORNER_M)
            {
                bounds.pop();
                ring.pop();
            }
            ring.push(p);
            bounds.push(bound);
        };
        for index in 0..count {
            let (a, b) = self.edge(index);
            let (sa, sb) = (side(a), side(b));
            let crossing = || add(a, scale(sub(b, a), sa / (sa - sb)));
            match (sa >= 0.0, sb >= 0.0) {
                (true, true) => push(a, self.bounds[index]),
                (true, false) => {
                    push(a, self.bounds[index]);
                    push(crossing(), bound);
                }
                (false, true) => push(crossing(), self.bounds[index]),
                (false, false) => (),
            }
        }
        if ring.len() > 1 && distance(ring[0], ring[ring.len() - 1]) < CORNER_M {
            ring.pop();
            bounds.pop();
        }
        (ring.len() >= 3 && area(&ring) >= SLIVER_M2).then_some(Leaf { ring, bounds })
    }

    /// The leaf drawn in by `by` metres on every side: `None` where nothing
    /// of it is left.
    pub(super) fn inset(&self, by: f64) -> Option<Leaf> {
        let mut inner = self.clone();
        for index in 0..self.ring.len() {
            let (a, b) = self.edge(index);
            let along = scale(sub(b, a), 1.0 / distance(a, b));
            // Counter-clockwise: the inside lies to an edge's left.
            let origin = add(a, scale([-along[1], along[0]], by));
            inner = inner.clipped(origin, along, true, self.bounds[index])?;
        }
        Some(inner)
    }

    /// Its sharpest corner, in radians.
    pub(super) fn sharpest_corner(&self) -> f64 {
        let count = self.ring.len();
        (0..count)
            .map(|at| {
                let (before, corner, after) = (
                    self.ring[(at + count - 1) % count],
                    self.ring[at],
                    self.ring[(at + 1) % count],
                );
                let (back, on) = (sub(before, corner), sub(after, corner));
                libm::atan2(cross(back, on).abs(), back[0] * on[0] + back[1] * on[1])
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// Where the line through `origin` along `along` enters and leaves the
    /// leaf, with what bounds the leaf there.
    pub(super) fn ends(&self, origin: Point, along: Point) -> Vec<(Point, Bound)> {
        (0..self.ring.len())
            .filter_map(|index| {
                let (a, b) = self.edge(index);
                let (sa, sb) = (cross(along, sub(a, origin)), cross(along, sub(b, origin)));
                ((sa >= 0.0) != (sb >= 0.0))
                    .then(|| (add(a, scale(sub(b, a), sa / (sa - sb))), self.bounds[index]))
            })
            .collect()
    }

    /// How far the leaf reaches to either side of the line.
    pub(super) fn reach(&self, origin: Point, along: Point) -> [f64; 2] {
        interval(self.ring.iter().map(|p| cross(along, sub(*p, origin))))
    }
}

/// One settlement's ground as it is cut.
pub(super) struct Ground {
    pub(super) policy: crate::layout::TownGeometry,
    pub(super) cuts: Vec<Cut>,
    pub(super) leaves: Vec<Leaf>,
}

impl Ground {
    /// Cut along the straight piece of road from `a` to `b`: every leaf it
    /// runs through is cut from edge to edge along its line, so a road that
    /// ends inside a block is carried on as that block's edge. A piece on
    /// the line of a cut already made is part of that cut, and paves it.
    pub(super) fn cut(&mut self, a: Point, b: Point, kind: SurfaceKind) {
        let known = self.cuts.iter().position(|cut| {
            cut.aside(a).abs() <= self.policy.sliver_m / 2.0
                && cut.aside(b).abs() <= self.policy.sliver_m / 2.0
        });
        let id = known.unwrap_or(self.cuts.len());
        if known.is_none() {
            self.cuts.push(Cut {
                origin: a,
                along: scale(sub(b, a), 1.0 / distance(a, b)),
                road: None,
                paved: Vec::new(),
                tees: Vec::new(),
            });
        }
        // A road that ends on another cut's line makes a junction there.
        for (end, far) in [(a, b), (b, a)] {
            for (other, cut) in self.cuts.iter_mut().enumerate() {
                if other != id && cut.aside(end).abs() <= self.policy.sliver_m / 2.0 {
                    let tee = Tee {
                        at: cut.at(end),
                        from_left: cut.aside(far) > 0.0,
                        along: scale(sub(end, far), 1.0 / distance(end, far)),
                        road: true,
                    };
                    cut.tees.push(tee);
                }
            }
        }
        let cut = &mut self.cuts[id];
        cut.road = Some(cut.road.map_or(kind, |known| known.min(kind)));
        let (from, to) = (cut.at(a), cut.at(b));
        let (span, ends) = if from <= to {
            ([from, to], [a, b])
        } else {
            ([to, from], [b, a])
        };
        cut.paved.push((span, ends));
        let (origin, along) = (cut.origin, cut.along);
        let mut pieces = Vec::with_capacity(self.leaves.len() + 4);
        for leaf in core::mem::take(&mut self.leaves) {
            let [low, high] = leaf.reach(origin, along);
            // Where the line runs inside the leaf, as distances along it.
            let inside = interval(contract::ground::edges(&leaf.ring).filter_map(|(c, d)| {
                let (sc, sd) = (cross(along, sub(*c, origin)), cross(along, sub(*d, origin)));
                ((sc >= 0.0) != (sd >= 0.0)).then(|| {
                    let p = add(*c, scale(sub(*d, *c), sc / (sc - sd)));
                    dot(sub(p, origin), along)
                })
            }));
            let shared = inside[1].min(span[1]) - inside[0].max(span[0]);
            let halves = (low < -self.policy.sliver_m
                && high > self.policy.sliver_m
                && shared >= self.policy.shared_m)
                .then(|| {
                    Some([
                        leaf.clipped(origin, along, true, Bound::Cut(id))?,
                        leaf.clipped(origin, along, false, Bound::Cut(id))?,
                    ])
                })
                .flatten();
            match halves {
                Some(halves) => pieces.extend(halves),
                None => pieces.push(leaf),
            }
        }
        self.leaves = pieces;
    }

    /// Note the place `at` where two of the map's roads meet: each cut that
    /// runs through it has a junction of the roads there, whether or not
    /// the road that makes it crosses the settlement's ground.
    pub(super) fn junction(&mut self, at: Point) {
        for cut in &mut self.cuts {
            if cut.aside(at).abs() <= self.policy.sliver_m / 2.0 {
                let tee = Tee {
                    at: cut.at(at),
                    from_left: true,
                    along: [-cut.along[1], cut.along[0]],
                    road: true,
                };
                cut.tees.push(tee);
            }
        }
    }

    /// The line `(origin, along)` across `leaf`, moved so that each end of
    /// it within `reach` of a junction on the cut it ends on lies on that
    /// junction: slid along that cut where one end has one, turned to pass
    /// through both where both do. Only a junction made from the cut's far
    /// side by a cut between blocks that runs in line with this one, and
    /// one no street of this side ends at yet: the two then make a
    /// crossroads, where two streets ending at one point on the same side
    /// would be a fork. The end of a road that stops along the cut is such
    /// a place too, for a line that would pass just beyond it. `None` where
    /// neither end has one.
    fn aligned(
        &self,
        leaf: &Leaf,
        (origin, along): (Point, Point),
        reach: f64,
    ) -> Option<(Point, Point)> {
        let middle = centroid(&leaf.ring);
        let junctions: Vec<(Point, Point)> = leaf
            .ends(origin, along)
            .into_iter()
            .filter_map(|(end, bound)| {
                let Bound::Cut(id) = bound else { return None };
                let cut = &self.cuts[id];
                let at = cut.at(end);
                let side = cut.aside(middle) > 0.0;
                let taken = |tee: f64| {
                    cut.tees.iter().any(|other| {
                        other.from_left == side && (other.at - tee).abs() <= self.policy.shared_m
                    })
                };
                // Or the end of a road that stops along the cut, where the
                // line would pass just beyond it: the two then share that
                // point, and the road does not stop a few metres short of a
                // street.
                let paved = |at: f64| {
                    cut.paved
                        .iter()
                        .any(|(span, _)| span[0] <= at && at <= span[1])
                };
                let road_ends = cut
                    .paved
                    .iter()
                    .flat_map(|(span, _)| *span)
                    .filter(|end| !paved(at) && (end - at).abs() <= reach);
                let tee = cut
                    .tees
                    .iter()
                    .filter(|tee| {
                        !tee.road
                            && tee.from_left != side
                            && (tee.at - at).abs() <= reach
                            && cross(tee.along, along).abs() <= self.policy.in_line_sin
                    })
                    .map(|tee| tee.at)
                    .filter(|tee| !taken(*tee))
                    .chain(road_ends)
                    .min_by(|a, b| (a - at).abs().total_cmp(&(b - at).abs()))?;
                Some((end, add(cut.origin, scale(cut.along, tee))))
            })
            .collect();
        match junctions[..] {
            [(end, junction)] => Some((add(origin, sub(junction, end)), along)),
            [(_, first), (_, second)] if distance(first, second) > self.policy.sliver_m => {
                let through = scale(sub(second, first), 1.0 / distance(first, second));
                // The same way round as it was drawn, so each side stays its side.
                Some((
                    first,
                    if dot(through, along) < 0.0 {
                        scale(through, -1.0)
                    } else {
                        through
                    },
                ))
            }
            _ => None,
        }
    }

    /// Whether the line `(origin, along)` across `leaf` ends clear of the
    /// junctions it makes no crossroads of: neither end lies within `clear`
    /// of the place a road ends on the cut that end lies on, nor within
    /// `reach` of where another cut ends on it from either side. A road
    /// leaves another at a junction of its own, a block from the nearest
    /// street, and two streets that miss each other across a road miss by
    /// more than a lot's width. An end on a junction (the one `aligned`
    /// moved it to) is clear of it.
    fn clear(
        &self,
        leaf: &Leaf,
        (origin, along): (Point, Point),
        [reach, clear]: [f64; 2],
    ) -> bool {
        leaf.ends(origin, along).into_iter().all(|(end, bound)| {
            let Bound::Cut(id) = bound else { return true };
            let cut = &self.cuts[id];
            let at = cut.at(end);
            cut.tees.iter().all(|tee| {
                let away = (tee.at - at).abs();
                away <= self.policy.meet_m || away >= if tee.road { clear } else { reach }
            })
        })
    }

    /// The edge a leaf fronts: its longest along a cut, a road's counting
    /// `on_road` times its length; its longest of any kind when no cut
    /// bounds it.
    fn frontage(&self, leaf: &Leaf, on_road: f64) -> usize {
        // (whether a cut, weighted length)
        let weight = |index: usize| {
            let (a, b) = leaf.edge(index);
            match leaf.bounds[index] {
                Bound::Limit => (false, distance(a, b)),
                Bound::Cut(id) if self.cuts[id].road.is_some() => (true, on_road * distance(a, b)),
                Bound::Cut(_) => (true, distance(a, b)),
            }
        };
        (0..leaf.ring.len())
            .max_by(|a, b| {
                let (a, b) = (weight(*a), weight(*b));
                a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
            })
            .unwrap_or(0)
    }

    /// Whether the leaf holds a rectangle `along` metres long against one of
    /// its cut edges and `deep` metres from it, `back` metres in from that
    /// edge: room for a parcel on a street. With `fronts`, only against the
    /// stretches of edge it lists: (edge, from, to), in metres along it.
    pub(super) fn holds(
        &self,
        leaf: &Leaf,
        [along, deep]: [f64; 2],
        back: f64,
        fronts: Option<&[(usize, f64, f64)]>,
    ) -> bool {
        let whole: Vec<(usize, f64, f64)> = (0..leaf.ring.len())
            .filter(|edge| leaf.bounds[*edge] != Bound::Limit)
            .map(|edge| (edge, 0.0, f64::INFINITY))
            .collect();
        fronts.unwrap_or(&whole).iter().any(|(edge, from, to)| {
            let (a, b) = leaf.edge(*edge);
            let span = distance(a, b);
            let toward = scale(sub(b, a), 1.0 / span);
            let inward = scale([-toward[1], toward[0]], back + deep);
            // Tried a metre apart along the stretch: parcels are tried a few
            // metres apart, from wherever the last one ended.
            let places = libm::floor(to.min(span) - from - along);
            (0..=places.max(-1.0) as i64).any(|place| {
                let start = add(a, scale(toward, from + place as f64));
                [start, add(start, scale(toward, along))]
                    .into_iter()
                    .all(|corner| {
                        contract::ground::polygon_contains(&leaf.ring, add(corner, inward))
                    })
            })
        })
    }

    /// A leaf's frame: a corner of its frontage, the unit vector along it,
    /// its depth from the frontage and the span it covers along it.
    fn frame(&self, leaf: &Leaf, on_road: f64) -> (Point, Point, f64, [f64; 2]) {
        let (a, b) = leaf.edge(self.frontage(leaf, on_road));
        let along = scale(sub(b, a), 1.0 / distance(a, b));
        let depth = leaf.reach(a, along)[1];
        let span = interval(leaf.ring.iter().map(|p| dot(sub(*p, a), along)));
        (a, along, depth, span)
    }

    /// Cut every leaf deeper than a block into a row along its frontage and
    /// the ground behind, and every row longer than a block across. A cut
    /// runs a few degrees off square, so blocks are not all rectangles on
    /// one grid. The cuts are avenues. What is left behind a last row may
    /// be too shallow to build on: it stays open.
    ///
    /// With `clear`, a cut is placed clear of the junctions on the cuts it
    /// ends on; without, where it is drawn.
    pub(super) fn subdivide(
        &mut self,
        rule: &BlockRule,
        towns: &Towns,
        clear: bool,
        rng: &mut Stream,
    ) {
        let ([shallow, deep], [short, long]) = (rule.depth_m, rule.length_m);
        let mut next = 0;
        // Each cut leaves two pieces shallower or shorter than the one it
        // cut, by a block's least depth or length at least.
        while next < self.leaves.len() {
            let leaf = self.leaves[next].clone();
            let (corner, along, depth, [from, to]) = self.frame(&leaf, towns.road_frontage);
            let inward = [-along[1], along[0]];
            // A cut turns about its middle by `skew`, which carries its ends
            // `lift` off square. It is square where a turn would take a
            // piece outside `[least, most]`. Answers the turn, the place
            // drawn and the places it might have been.
            let skewed = |across: f64, [least, most]: [f64; 2], rng: &mut Stream| {
                let skew = (2.0 * rng.unit() - 1.0) * towns.block_skew_deg.to_radians();
                let lift = across / 2.0 * libm::tan(skew.abs());
                if least + lift <= most - lift {
                    let places = [least + lift, most - lift];
                    (skew, rng.range(places), places)
                } else {
                    (0.0, rng.range([least, most]), [least, most])
                }
            };
            let turned = |by: Point, skew: f64| {
                let (sin, cos) = (libm::sin(skew), libm::cos(skew));
                [by[0] * cos - by[1] * sin, by[0] * sin + by[1] * cos]
            };
            // (the turn, the place drawn, the places allowed, whether a row)
            let cut = if depth > deep {
                Some((skewed(to - from, [shallow, deep], rng), true))
            } else if to - from > long {
                // A row of up to two blocks is cut about its middle, so
                // neither is a sliver; a longer one loses a block.
                Some((
                    if to - from <= 2.0 * long {
                        let [low, high] = towns.block_split;
                        skewed(depth, [(to - from) * low, (to - from) * high], rng)
                    } else {
                        skewed(depth, [short, long], rng)
                    },
                    false,
                ))
            } else {
                None
            };
            // The line of a row's cut `at` back from the frontage, or of a
            // cut across the row `at` along it.
            let line = |skew: f64, at: f64, row: bool| {
                if row {
                    let middle = add(corner, scale(along, (from + to) / 2.0));
                    (add(middle, scale(inward, at)), turned(along, skew))
                } else {
                    let middle = add(corner, scale(inward, depth / 2.0));
                    (add(middle, scale(along, from + at)), turned(inward, skew))
                }
            };
            let id = self.cuts.len();
            let halve = |(origin, along): (Point, Point)| {
                let halves = [
                    leaf.clipped(origin, along, true, Bound::Cut(id))?,
                    leaf.clipped(origin, along, false, Bound::Cut(id))?,
                ];
                Some((origin, along, halves))
            };
            // A cut that would end on another within reach of where a cut
            // already ends on it from the far side is moved to end there:
            // one crossroads, not two junctions a few metres apart. And it
            // keeps clear of the other junctions, the roads' by a block (or
            // half the depth of one, where blocks are small): the place
            // nearest the one drawn, of those a block may be cut at, where
            // it does. Where none does it is cut as drawn.
            let rooms = if clear {
                [towns.align_m, towns.junction_clear_m.min(shallow / 2.0)]
            } else {
                [0.0; 2]
            };
            let halves = cut.and_then(|((skew, drawn, [low, high]), row)| {
                let step = (high - low) / self.policy.places as f64;
                let places = (0..=self.policy.places)
                    .flat_map(|k| [drawn + k as f64 * step, drawn - k as f64 * step])
                    .filter(|at| *at >= low && *at <= high);
                let moved = |at: f64| {
                    let line = line(skew, at, row);
                    self.aligned(&leaf, line, towns.align_m).unwrap_or(line)
                };
                places
                    .map(moved)
                    .filter(|line| self.clear(&leaf, *line, rooms))
                    .find_map(halve)
                    .or_else(|| halve(moved(drawn)))
                    .or_else(|| halve(line(skew, drawn, row)))
            });
            match halves {
                Some((origin, along, [left, right])) => {
                    let middle = centroid(&leaf.ring);
                    for (end, bound) in leaf.ends(origin, along) {
                        if let Bound::Cut(other) = bound {
                            let cut = &mut self.cuts[other];
                            let toward = if dot(sub(end, middle), along) >= 0.0 {
                                along
                            } else {
                                scale(along, -1.0)
                            };
                            let tee = Tee {
                                at: cut.at(end),
                                from_left: cut.aside(middle) > 0.0,
                                along: toward,
                                road: false,
                            };
                            cut.tees.push(tee);
                        }
                    }
                    self.cuts.push(Cut {
                        origin,
                        along,
                        road: None,
                        paved: Vec::new(),
                        tees: Vec::new(),
                    });
                    self.leaves[next] = left;
                    self.leaves.push(right);
                }
                None => next += 1,
            }
        }
    }
}

/// The straight pieces of `roads` on the convex ground `ring`.
pub(super) fn pieces(
    policy: &crate::layout::TownGeometry,
    ring: &[Point],
    roads: &[Road],
) -> Vec<(Point, Point, SurfaceKind)> {
    let mut found = Vec::new();
    for (kind, points) in roads {
        for run in points.windows(2) {
            let (a, b) = (run[0], run[1]);
            let step = sub(b, a);
            // The shares of the run inside every edge's half-plane.
            let (mut from, mut to): (f64, f64) = (0.0, 1.0);
            for (c, d) in contract::ground::edges(ring) {
                let edge = sub(*d, *c);
                let (start, slope) = (cross(edge, sub(a, *c)), cross(edge, step));
                if slope == 0.0 {
                    if start < 0.0 {
                        to = -1.0;
                    }
                } else if slope > 0.0 {
                    from = from.max(-start / slope);
                } else {
                    to = to.min(-start / slope);
                }
            }
            if (to - from) * length(step) >= policy.shared_m {
                found.push((add(a, scale(step, from)), add(a, scale(step, to)), *kind));
            }
        }
    }
    found
}
