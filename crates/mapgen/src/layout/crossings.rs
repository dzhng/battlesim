//! Roads at the river: a road keeps to its own bank, a road's clearance from
//! the water, and crosses by a deck that ends on dry ground. A crossing near
//! a bridge already built uses that bridge.
//!
//! The layout writes one river at most, from the north edge to the south, so
//! the map has two banks (`Water::bank`), and a road whose ends stand on
//! different banks crosses once.
use super::geometry::{
    add, bearing, direction, distance, round_cm, scale, segment_bounds, segment_crossing, sub,
    turn, Grid, Point, PI,
};
use super::water::{Crossing, Water};
use super::Context;
use contract::map::{Bridge, SurfaceKind};
use core::f64::consts::FRAC_PI_2;

/// One bridge and the two points a road runs straight between to cross it.
#[derive(Clone, Debug)]
pub struct Span {
    pub bridge: Bridge,
    ends: [Point; 2],
}

pub struct Crossings<'a> {
    context: &'a Context<'a>,
    water: &'a Water<'a>,
    /// The line a road follows along either bank: the river's authored line
    /// moved the water's half width and the road's clearance to that side.
    beside: [Vec<Point>; 2],
    /// Every run of `beside`: its side and its first point.
    runs: Vec<(usize, usize)>,
    grid: Grid,
    built: Vec<Span>,
}

/// A place on a road where it meets the line beside the water, or one of
/// its ends.
struct Station {
    /// The road's run it is on, and how far along the road it is.
    run: usize,
    along: f64,
    at: Point,
    /// Where along the line beside the water: a point's index and the share
    /// of the way to the next. An end's is the line's point nearest it.
    beside: f64,
}

impl<'a> Crossings<'a> {
    pub fn new(context: &'a Context<'a>, water: &'a Water<'a>) -> Self {
        let clearance = context.presets.rivers.road_gap_m;
        let points = water
            .rivers()
            .first()
            .map_or(&[][..], |river| river.points());
        let beside = [-1.0, 1.0].map(|side| {
            (0..points.len())
                .map(|i| {
                    let (before, after) = (i.saturating_sub(1), (i + 1).min(points.len() - 1));
                    let heading = direction(bearing(points[before].xy, points[after].xy));
                    let out = scale(
                        [-heading[1], heading[0]],
                        side * (points[i].width_m / 2.0 + clearance),
                    );
                    add(points[i].xy, out)
                })
                .collect::<Vec<Point>>()
        });
        let mut grid = Grid::new([context.extent; 2], 64.0);
        let mut runs = Vec::new();
        for (side, line) in beside.iter().enumerate() {
            for (first, pair) in line.windows(2).enumerate() {
                grid.insert(segment_bounds(pair[0], pair[1], 0.0), runs.len() as u32);
                runs.push((side, first));
            }
        }
        Self {
            context,
            water,
            beside,
            runs,
            grid,
            built: Vec::new(),
        }
    }

    /// The bridges roads crossed by, in the order they were built.
    pub fn bridges(self) -> Vec<Bridge> {
        self.built.into_iter().map(|span| span.bridge).collect()
    }

    /// Keep the bridges of `planned` that the finished road `points` runs
    /// over. A road cut short at a junction may never reach its crossing,
    /// or may end just past it: it crosses when the run it leaves one end
    /// of the span by reaches the far end of the deck.
    pub fn build(&mut self, planned: Vec<Span>, points: &[Point]) {
        let approach = self.context.presets.rivers.bridge.approach_m;
        self.built.extend(planned.into_iter().filter(|span| {
            let over = approach + 2.0 * span.bridge.half_extents[0];
            points
                .windows(2)
                .any(|run| span.ends.contains(&run[0]) && distance(run[0], run[1]) >= over)
        }));
    }

    /// The road `points` as it runs with the river there: kept to the bank
    /// it is on and, where its two ends stand on different banks, carried
    /// over by one bridge. A bridge it would have to build is added to
    /// `planned`. Both ends stay where they are.
    pub fn carry(
        &self,
        kind: SurfaceKind,
        points: Vec<Point>,
        planned: &mut Vec<Span>,
    ) -> Result<Vec<Point>, String> {
        if self.water.is_empty() {
            return Ok(points);
        }
        let crossings: Vec<(usize, Crossing)> = points
            .windows(2)
            .enumerate()
            .flat_map(|(run, ends)| {
                self.water
                    .crossings(ends[0], ends[1])
                    .into_iter()
                    .map(move |crossing| (run, crossing))
            })
            .collect();
        let carried = if crossings.len().is_multiple_of(2) {
            self.keep(&points)
        } else {
            // The middle crossing is where the road means to cross: a road
            // that wanders over the water before or after it is kept back.
            let (run, crossing) = crossings[crossings.len() / 2];
            let wanted = crossing.at;
            let heading = bearing(points[run], points[run + 1]);
            let span = self.span(kind, crossing, heading, planned)?;
            let near = usize::from(self.water.bank(span.ends[0]) != self.water.bank(points[0]));
            // The road turns for its bridge as far out as the bridge is from
            // where it meant to cross.
            let lead = distance(wanted, span.bridge.center) + distance(span.ends[0], span.ends[1]);
            let (before, after) = points.split_at(run + 1);
            let last = after.len() - 1;
            let mut head: Vec<Point> = before
                .iter()
                .enumerate()
                .filter(|(i, p)| *i == 0 || distance(**p, wanted) > lead)
                .map(|(_, p)| *p)
                .collect();
            head.push(span.ends[near]);
            let mut tail = vec![span.ends[1 - near]];
            tail.extend(
                after
                    .iter()
                    .enumerate()
                    .filter(|(i, p)| *i == last || distance(**p, wanted) > lead)
                    .map(|(_, p)| *p),
            );
            if !planned
                .iter()
                .chain(&self.built)
                .any(|known| known.ends == span.ends)
            {
                planned.push(span);
            }
            let mut carried = self.keep(&head);
            carried.extend(self.keep(&tail));
            carried
        };
        let mut out: Vec<Point> = Vec::with_capacity(carried.len());
        for point in carried {
            if out.last() != Some(&point) {
                out.push(point);
            }
        }
        Ok(out)
    }

    /// The bridge a road of `kind`, heading along `heading`, takes to cross
    /// where it meant to: the nearest one already there if it is near
    /// enough; or a new one on the road's own line, if that is not too far
    /// askew of the river; or one square across the first stretch of river,
    /// working outward, that a deck fits.
    fn span(
        &self,
        kind: SurfaceKind,
        crossing: Crossing,
        heading: f64,
        planned: &[Span],
    ) -> Result<Span, String> {
        let rules = &self.context.presets.rivers;
        let (wanted, reach) = (crossing.at, rules.bridge.reuse_m[&kind]);
        let away = |span: &&Span| distance(span.bridge.center, wanted);
        let nearest = planned
            .iter()
            .chain(&self.built)
            .min_by(|a, b| away(a).total_cmp(&away(b)));
        if let Some(span) = nearest.filter(|span| away(span) <= reach) {
            return Ok(span.clone());
        }
        let square = crossing.downstream + FRAC_PI_2;
        let askew = turn(heading, square).min(turn(heading + PI, square));
        if askew <= rules.bridge.skew_max_deg.to_radians() {
            if let Some(span) = self.deck(round_cm(wanted), heading) {
                return Ok(span);
            }
        }
        // The river's authored points, nearest where it meant to cross
        // first.
        let points = self.water.rivers()[0].points();
        let mut sites: Vec<(f64, usize)> = (1..points.len() - 1)
            .map(|index| (distance(points[index].xy, wanted), index))
            .collect();
        sites.sort_by(|a, b| a.0.total_cmp(&b.0));
        sites
            .into_iter()
            .take_while(|(off, _)| *off <= reach.max(rules.point_step_m))
            .find_map(|(_, index)| {
                let downstream = bearing(points[index - 1].xy, points[index + 1].xy);
                self.deck(points[index].xy, downstream + FRAC_PI_2)
            })
            .ok_or_else(|| {
                format!(
                    "no stretch of river within {reach} m of ({}, {}) takes a deck",
                    wanted[0], wanted[1]
                )
            })
    }

    /// A deck over the water at `center` heading along `heading`: long
    /// enough to end a landing past the water at its middle and both sides,
    /// with a straight run of road at each end. `None` when that takes a
    /// longer deck than the presets allow, or a run ends off the map or
    /// nearer the water than the deck does.
    fn deck(&self, center: Point, heading: f64) -> Option<Span> {
        let rule = &self.context.presets.rivers.bridge;
        // Microradians, as a building's yaw is: far below a centimetre over
        // the longest deck.
        let yaw = libm::round(heading * 1e6) / 1e6;
        let (heading, half_width) = (direction(yaw), rule.width_m / 2.0);
        let aside = [-heading[1], heading[0]];
        let at =
            |out: f64, across: f64| add(center, add(scale(heading, out), scale(aside, across)));
        // Each end is probed across its whole width, a landing apart at
        // most, so no water lies between two probes.
        let lanes = (libm::ceil(rule.width_m / rule.landing_m) as usize).max(2);
        let landed = |half_length: f64| {
            [-1.0, 1.0].iter().all(|end| {
                (0..=lanes).all(|lane| {
                    let across = half_width * (2.0 * lane as f64 / lanes as f64 - 1.0);
                    self.water
                        .gap(at(end * half_length, across), rule.landing_m)
                        >= rule.landing_m
                })
            })
        };
        // Whole metres, up to the longest deck the presets allow.
        let mut half_length = libm::ceil(rule.landing_m);
        while !landed(half_length) {
            half_length += 1.0;
            if 2.0 * half_length > rule.span_max_m {
                return None;
            }
        }
        let ends = [-1.0, 1.0].map(|end| round_cm(at(end * (half_length + rule.approach_m), 0.0)));
        let extent = self.context.extent;
        let dry = ends.iter().all(|end| {
            end.iter().all(|v| *v >= 0.0 && *v <= extent)
                && self.water.gap(*end, rule.landing_m) >= rule.landing_m
        });
        dry.then(|| Span {
            bridge: Bridge {
                deck: rule.deck.clone(),
                center,
                half_extents: [half_length, half_width],
                yaw,
                deck_z: rule.deck_z,
                thickness_m: rule.thickness_m,
            },
            ends,
        })
    }

    /// The road `points`, which starts and ends on one bank, with every
    /// stretch that strays nearer the water than a road may, or over it,
    /// taken along the line beside the water instead.
    fn keep(&self, points: &[Point]) -> Vec<Point> {
        let rules = &self.context.presets.rivers;
        let bank = self.water.bank(points[0]);
        let last = points.len() - 1;
        let mut run_start = vec![0.0];
        for pair in points.windows(2) {
            run_start.push(run_start[run_start.len() - 1] + distance(pair[0], pair[1]));
        }
        let strays = |along: f64| {
            let run = run_start
                .partition_point(|start| *start <= along)
                .clamp(1, last)
                - 1;
            let share = (along - run_start[run]) / (run_start[run + 1] - run_start[run]);
            let p = add(points[run], scale(sub(points[run + 1], points[run]), share));
            self.water.bank(p) != bank
                // A point of the line itself is a road's clearance from the
                // water, give or take the rounding of a bend.
                || self.water.gap(p, rules.road_gap_m) < rules.road_gap_m - rules.bank_m() / 2.0
        };
        // The side whose line lies on this road's bank.
        let side = usize::from(self.water.bank(self.beside[0][self.beside[0].len() / 2]) != bank);
        let line = &self.beside[side];
        // An end's place on the line is the line's point nearest it: the run
        // of road onto a bridge starts nearer the water than the line, and a
        // road that strays between there and a meeting follows the line too.
        let end = |at: Point, run: usize, along: f64| {
            let nearest = (0..line.len())
                .min_by(|i, j| distance(line[*i], at).total_cmp(&distance(line[*j], at)));
            Station {
                run,
                along,
                at,
                beside: nearest.unwrap_or(0) as f64,
            }
        };
        let mut stations = vec![end(points[0], 0, 0.0)];
        stations.extend(self.meetings(points, &run_start, side));
        stations.push(end(points[last], last - 1, run_start[last]));
        let mut out = vec![points[0]];
        for pair in stations.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            if strays((from.along + to.along) / 2.0) {
                // The line's own points between the two stations.
                let (low, high) = (from.beside.min(to.beside), from.beside.max(to.beside));
                let between = libm::ceil(low) as usize..=libm::floor(high) as usize;
                if to.beside >= from.beside {
                    out.extend(between.map(|i| line[i]));
                } else {
                    out.extend(between.rev().map(|i| line[i]));
                }
            } else {
                // The road's own points between them: after the run the
                // first is on, up to the run the second is on.
                out.extend(&points[from.run + 1..to.run + 1]);
            }
            out.push(to.at);
        }
        out.into_iter().map(round_cm).collect()
    }

    /// Every place the road `points` meets the line beside the water on one
    /// side, in order along the road.
    fn meetings(&self, points: &[Point], run_start: &[f64], side: usize) -> Vec<Station> {
        let line = &self.beside[side];
        let mut meetings = Vec::new();
        for (run, ends) in points.windows(2).enumerate() {
            let mut near = Vec::new();
            self.grid.any(segment_bounds(ends[0], ends[1], 0.0), |id| {
                near.push(id as usize);
                false
            });
            near.sort_unstable();
            near.dedup();
            for (_, first) in near
                .into_iter()
                .map(|id| self.runs[id])
                .filter(|(on, _)| *on == side)
            {
                let met = segment_crossing(ends[0], ends[1], line[first], line[first + 1]);
                // A meeting at a point of either line belongs to the run
                // that starts there.
                if let Some((share, on)) = met.filter(|(share, on)| {
                    (*share < 1.0 || run + 2 == points.len())
                        && (*on < 1.0 || first + 2 == line.len())
                }) {
                    meetings.push(Station {
                        run,
                        along: run_start[run] + share * (run_start[run + 1] - run_start[run]),
                        at: add(ends[0], scale(sub(ends[1], ends[0]), share)),
                        beside: first as f64 + on,
                    });
                }
            }
        }
        meetings.sort_by(|a, b| a.along.total_cmp(&b.along));
        meetings
    }
}
