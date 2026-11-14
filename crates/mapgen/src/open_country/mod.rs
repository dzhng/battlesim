//! Open country (M24, M25): what stands between the settlements and the
//! woods. A few homes along the country roads, short tree lines, copses and
//! single trees, and low cover in the fields. The target is interrupted sight
//! at every playable location, with most bearings open. Authored-outline
//! proximity is a construction heuristic, not a physical sight certificate.
//!
//! The pass runs on a plan whose districts are built. It adds lanes and
//! depots' paved yards to `surfaces`, parcels to `lots`, homes to
//! `buildings`, tree lines, copses and trees to `forests` (a tree line is a
//! stroke, a single tree a plot one trunk stands on) and low cover and
//! depots' bodies to `props`, then measures the open approaches again,
//! because a home or a tree ends one.
//!
//! - **Everything goes through one question** ([`Country::open`]): is this
//!   much ground clear of the map's edge, the settlements, the woods, every
//!   carriageway, water, bridges, the yards and trees already placed, and,
//!   for what blocks sight, the kept approach corridors.
//! - **A settlement keeps its widest approach in each half**, as a corridor:
//!   the front's width of ground along the bearing down the approach's
//!   middle, the line the encounter planner posts its overwatch on. Nothing
//!   that blocks sight stands in one; low cover may. The rest of what the
//!   layout measured as open is country like any other, and the plan's
//!   `approaches` afterwards are the kept ones, each as wide as it still is.
//!   Where the kept corridors of several settlements lie side by side and
//!   leave ground nothing can be seen from, a minor settlement's corridor
//!   gives way; the main settlement's never does.
//! - **Whole cells earn physical witnesses after authored bodies are final**:
//!   the coverage owner uses the contract's actual shared trunk candidates,
//!   canopy and fog-cell geometry, including town interiors and edges. Failed
//!   copse/tree-line proposals roll back; unsupported work or unfilled cells
//!   refuse the requested seed rather than silently preserving holes.
//! - **The halves are even by construction**: each kind is placed in the
//!   half that holds less of it, until the map holds its share.
//!
//! Construction sizes, densities and clearances come from `open_country`;
//! physical sight comes from resolved rules. Each step has its own named stream.
use crate::layout::geometry::{
    add, direction, distance, dot, ring_distance, round_cm, scale, sub, thinned, Outline, Point,
    PI, TAU,
};
use crate::layout::rng::Stream;
use crate::layout::water::Water;
use crate::layout::{corridor_start, GenerationRequest, LotRule, OutlineShape, PresetDefinitions};
use crate::parcels::lots::{placement, Choices, Fit, Frontage, Lot};
use crate::parcels::space::{Rect, Run};
use crate::parcels::streets::Network;
use crate::{BuildingPlacement, Diagnostic, DiagnosticCode, LotPlan, MapPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{AuthoredPropDefinition, Forest, PropDefinition, SurfaceArea, SurfaceKind};
use contract::templates::{PlacementFrame, TemplateGeometryCatalog};
use serde::Serialize;
use std::collections::BTreeMap;

/// A lane runs this far past the middle of the road it leaves, so the two
/// centrelines cross whatever a centimetre of rounding did to either.
const JOIN_OVERSHOOT_M: f64 = 0.5;

mod bodies;
mod completion;
mod country;
pub(crate) use completion::cover;
mod coverage;
mod measure;
mod rules;
mod steads;
pub use measure::*;
pub use rules::*;

/// A ring with its box, asked how near a point is.
struct Ground<'a> {
    ring: &'a [Point],
    limits: [f64; 4],
}

impl<'a> Ground<'a> {
    fn new(ring: &'a [Point]) -> Self {
        Self {
            ring,
            limits: contract::ground::limits(ring, 0.0),
        }
    }

    /// Whether `p` is at least `gap` from the filled ring.
    fn clear(&self, p: Point, gap: f64) -> bool {
        self.clear_charged(p, gap, |_| true)
    }
    fn clear_charged(&self, p: Point, gap: f64, mut charge: impl FnMut(u64) -> bool) -> bool {
        if !charge(1) {
            return false;
        }
        let [x0, y0, x1, y1] = self.limits;
        p[0] < x0 - gap
            || p[0] > x1 + gap
            || p[1] < y0 - gap
            || p[1] > y1 + gap
            || (charge(2 * self.ring.len() as u64) && ring_distance(self.ring, p) >= gap)
    }
}

/// Distance from `p` to the rectangle; zero inside it.
fn rect_gap(rect: &Rect, p: Point) -> f64 {
    let offset = sub(p, rect.center);
    let along = dot(offset, rect.axis).abs() - rect.half[0];
    let across = dot(offset, [-rect.axis[1], rect.axis[0]]).abs() - rect.half[1];
    libm::hypot(along.max(0.0), across.max(0.0))
}

/// At least `least` away, and never on it.
fn beyond(gap: f64, least: f64) -> bool {
    gap >= least && gap > 0.0
}

/// The amounts placed so far, by half: `[top, bottom]`.
#[derive(Default, Clone, Copy)]
struct Tally {
    homes: [f64; 2],
    tree_line_m: [f64; 2],
    copses: [f64; 2],
    trees: [f64; 2],
    cover: [f64; 2],
    depots: [f64; 2],
}

/// A stretch of country road or track.
struct Road {
    run: Run,
    half_width: f64,
}

/// Everything the open country keeps clear of, and what the pass has
/// placed on it so far.
struct Country<'a> {
    request: &'a GenerationRequest,
    presets: &'a PresetDefinitions,
    rules: &'a Rules,
    size: [f64; 2],
    objectives: Vec<(Point, f64)>,
    towns: Vec<Ground<'a>>,
    woods: Vec<Ground<'a>>,
    network: Network<'a>,
    water: Water<'a>,
    /// Bridge decks with the straight run onto each.
    decks: Vec<Rect>,
    /// The kept approach corridors, and whether each is the main
    /// settlement's.
    corridors: Vec<(Rect, bool)>,
    /// The kept approaches: each one's settlement and middle bearing.
    kept: Vec<(usize, f64)>,
    /// Set while the ground no kept corridor leaves room beside is filled:
    /// only the main settlement's corridors are kept then.
    main_only: bool,
    roads: Vec<Road>,
    road_m: f64,
    proof_fill: bool,
    work: Option<std::rc::Rc<coverage::Work>>,
    yards: Vec<Rect>,
    /// Copses, trees and tree lines placed, as discs.
    groves: Vec<(Point, f64)>,
    bodies: Vec<(Point, f64)>,
    /// Each group's station, by what its pieces are.
    steads: Vec<(&'static str, Point)>,
    tally: Tally,
    next_prop: u32,
    surfaces: Vec<SurfaceArea>,
    lots: Vec<LotPlan>,
    buildings: Vec<BuildingPlacement>,
    forests: Vec<Forest>,
    props: Vec<AuthoredPropDefinition>,
}

/// What a place is asked for.
#[derive(Clone, Copy)]
struct Ask {
    radius: f64,
    /// Whether it blocks sight: a building or trees, kept out of the
    /// approach corridors.
    blocks_sight: bool,
    road_gap: f64,
    /// The yards it may touch, its own: those from this index on.
    own_from: usize,
}

impl<'a> Country<'a> {
    fn stream(&self, name: &str) -> Stream {
        crate::layout::stream(self.request, &format!("country/{name}"))
    }

    fn charge(&self, n: u64) -> bool {
        self.work.as_ref().is_none_or(|w| w.spend(n))
    }
    /// Construction density estimate; physical admission belongs to Coverage.
    fn estimated_open_km2(&self) -> f64 {
        let cell = self.rules.sight.cell_m;
        let size = self.size;
        let count = |extent: f64| (libm::ceil(extent / cell) as usize).max(1);
        let (columns, rows) = (count(size[0]), count(size[1]));
        // A density estimate only. Physical coverage is certified after
        // every authored body is placed, including street furniture.
        let open_count = (0..columns * rows)
            .filter(|index| {
                let p = [
                    ((*index % columns) as f64 + 0.5) * cell,
                    ((*index / columns) as f64 + 0.5) * cell,
                ];
                p[0] <= size[0]
                    && p[1] <= size[1]
                    && self
                        .towns
                        .iter()
                        .all(|town| town.clear(p, f64::MIN_POSITIVE))
                    && self
                        .woods
                        .iter()
                        .all(|wood| wood.clear(p, f64::MIN_POSITIVE))
                    && !self.water.near(p, 0.)
            })
            .count();
        open_count as f64 * cell * cell / 1e6
    }

    fn half(&self, p: Point) -> usize {
        usize::from(p[1] < self.size[1] / 2.0)
    }

    /// The one question: may something of `ask` stand at `p`.
    fn open(&self, p: Point, ask: Ask) -> bool {
        if !self.charge(1) {
            return false;
        }
        if self.objectives.iter().any(|(center, radius)| {
            libm::hypot(p[0] - center[0], p[1] - center[1]) < radius + ask.radius
        }) {
            return false;
        }
        let c = &self.rules.clear;
        let r = ask.radius;
        let edge = r + c.edge_m;
        if p[0] < edge || p[1] < edge || p[0] > self.size[0] - edge || p[1] > self.size[1] - edge {
            return false;
        }
        let road = r + ask.road_gap;
        let water = r + c.water_m;
        (self.proof_fill
            || self
                .towns
                .iter()
                .all(|town| town.clear_charged(p, r + c.settlement_m, |n| self.charge(n))))
            && self
                .woods
                .iter()
                .all(|wood| wood.clear_charged(p, r + c.forest_m, |n| self.charge(n)))
            && self.network.edge_gap(p, road, || self.charge(1)) >= road
            && self.water.gap_charged(p, water, || self.charge(1)) >= water
            && self
                .decks
                .iter()
                .all(|deck| self.charge(1) && beyond(rect_gap(deck, p), road))
            && self.yards.iter().enumerate().all(|(index, yard)| {
                self.charge(1) && (index >= ask.own_from || beyond(rect_gap(yard, p), r + c.yard_m))
            })
            && self
                .bodies
                .iter()
                .all(|(at, reach)| self.charge(1) && distance(*at, p) >= r + reach)
            && (!ask.blocks_sight
                || self.corridors.iter().all(|(corridor, main)| {
                    self.charge(1)
                        && ((self.main_only && !main)
                            || beyond(rect_gap(corridor, p), r + c.approach_m))
                }))
    }

    /// Trees of `radius` at `p` keep their distance from those placed.
    fn apart(&self, p: Point, radius: f64) -> bool {
        self.groves.iter().all(|(at, reach)| {
            self.charge(1) && distance(*at, p) >= radius + reach + self.rules.clear.yard_m
        })
    }

    fn trees(&self, radius: f64) -> Ask {
        Ask {
            radius,
            blocks_sight: true,
            road_gap: self.rules.clear.road_m,
            own_from: usize::MAX,
        }
    }

    /// A point of open ground in `half`, anywhere.
    fn anywhere(&self, half: usize, rng: &mut Stream) -> Point {
        let middle = self.size[1] / 2.0;
        let y = if half == 0 {
            [middle, self.size[1]]
        } else {
            [0.0, middle]
        };
        [rng.range([0.0, self.size[0]]), rng.range(y)]
    }

    /// A place on a country road or track: the road and how far along.
    fn station(&self, rng: &mut Stream) -> Option<(usize, f64)> {
        if self.road_m <= 0.0 {
            return None;
        }
        let mut at = rng.unit() * self.road_m;
        for (index, road) in self.roads.iter().enumerate() {
            if at < road.run.length() {
                return Some((index, at));
            }
            at -= road.run.length();
        }
        None
    }

    /// Place things of one kind until the map holds `total` of them and the
    /// halves are even, always in the half that holds less. `place` tries
    /// one in a half and says whether it stood; `held` is the amounts so
    /// far, in the unit `total` and the allowance `least` are in.
    fn spread(
        &mut self,
        total: f64,
        least: f64,
        attempts: u32,
        rng: &mut Stream,
        held: impl Fn(&Self) -> [f64; 2],
        mut place: impl FnMut(&mut Self, usize, &mut Stream) -> bool,
    ) {
        // Bounded: each thing is tried `attempts` times, and the pass stops
        // at the first that finds no room in the half that needs it.
        for _ in 0..self.rules.placed_max {
            let [top, bottom] = held(self);
            let uneven = !even(self.rules.fairness.rel, least, top, bottom);
            if top + bottom >= total && !uneven {
                break;
            }
            let half = usize::from(top > bottom);
            if !(0..attempts).any(|_| place(self, half, rng)) {
                break;
            }
        }
    }

    // ----- Trees -------------------------------------------------------

    /// Stand a small wood on `ring`.
    fn plant(&mut self, ring: Vec<Point>, center: Point, radius: f64) -> bool {
        let Ok(shape) = GroundShape::polygon(ring.into_iter().map(round_cm).collect()) else {
            return false;
        };
        self.forests.push(Forest { shape });
        self.groves.push((center, radius));
        true
    }

    fn copse(&mut self, p: Point, rng: &mut Stream, clear: impl Fn(Point, f64) -> bool) -> bool {
        let k = self.rules.copses;
        let shape = OutlineShape {
            exponent: 2.0,
            noise: self.presets.forests.outline_noise,
            points: k.outline_points,
        };
        let outline = Outline::draw(
            shape,
            rng.range(k.area_m2),
            rng.range(k.aspect),
            rng.range([0.0, PI]),
            rng,
        )
        .at(p);
        // Centimetre rounding must still leave a copse, rather than a
        // single-tree plot in the finished geometry's measurement.
        let center = crate::layout::geometry::centroid(&outline.ring);
        if !(crate::layout::geometry::area(&outline.ring) >= k.area_m2[0]
            && clear(outline.center, outline.reach)
            && self.open(outline.center, self.trees(outline.reach))
            && self.apart(outline.center, outline.reach)
            && self.plant(outline.ring, outline.center, outline.reach))
        {
            return false;
        }
        self.tally.copses[self.half(center)] += 1.0;

        true
    }

    fn lone_tree(&mut self, p: Point) -> bool {
        let half = self.rules.lone_trees.plot_m / 2.0;
        let p = round_cm(p);
        let radius = half * core::f64::consts::SQRT_2;
        // Square to the map: the trunk grid is, so one trunk stands on it.
        let ring = [[-half, -half], [half, -half], [half, half], [-half, half]]
            .map(|corner| add(p, corner))
            .to_vec();
        if !(self.open(p, self.trees(radius))
            && self.apart(p, radius)
            && self.plant(ring, p, radius))
        {
            return false;
        }
        self.tally.trees[self.half(p)] += 1.0;
        true
    }

    /// A tree line along `line`, broken into stretches by gaps. Every
    /// stretch must find room, or none is planted.
    fn tree_line(
        &mut self,
        line: &Run,
        road_gap: f64,
        rng: &mut Stream,
        clear: impl Fn(Point, f64) -> bool,
    ) -> bool {
        let t = self.rules.tree_lines;
        let Ok(work) = coverage::tree_line_work(line.length(), t) else {
            self.charge(u64::MAX);
            return false;
        };
        if !self.charge(work) {
            return false;
        }
        let half = t.width_m / 2.0;
        let ask = Ask {
            road_gap,
            ..self.trees(half)
        };
        let mut stretches: Vec<Vec<Point>> = Vec::new();
        let mut from = 0.0;
        while from + t.stretch_m[0] <= line.length() {
            let to = (from + rng.range(t.stretch_m)).min(line.length());
            // A stub too short to stand a row of trees joins the stretch.
            let to = if line.length() - to < t.stretch_m[0] + t.gap_m {
                line.length()
            } else {
                to
            };
            let steps = libm::ceil((to - from) / half).max(1.0) as usize;
            let points: Vec<Point> = (0..=steps)
                .map(|step| line.at(from + (to - from) * step as f64 / steps as f64))
                .collect();
            if !points
                .iter()
                .all(|p| clear(*p, half) && self.open(*p, ask) && self.apart(*p, half))
            {
                return false;
            }
            stretches.push(thinned(&points, 0.25).into_iter().map(round_cm).collect());
            from = to + t.gap_m;
        }
        let shapes: Vec<GroundShape> = stretches
            .iter()
            .filter_map(|points| GroundShape::stroke(points.clone(), t.width_m).ok())
            .collect();
        if shapes.is_empty() || shapes.len() != stretches.len() {
            return false;
        }
        for (shape, points) in shapes.into_iter().zip(stretches) {
            self.forests.push(Forest { shape });
            for pair in points.windows(2) {
                let length = distance(pair[0], pair[1]);
                let steps = libm::ceil(length / t.width_m).max(1.0) as usize;
                for step in 0..=steps {
                    let p = add(
                        pair[0],
                        scale(sub(pair[1], pair[0]), step as f64 / steps as f64),
                    );
                    self.groves.push((p, half));
                }
                let middle = scale(add(pair[0], pair[1]), 0.5);
                self.tally.tree_line_m[self.half(middle)] += length;
            }
        }
        true
    }

    /// A tree line beside a road in `half`.
    fn roadside_line(&mut self, half: usize, rng: &mut Stream) -> bool {
        let t = self.rules.tree_lines;
        let Some((road, s)) = self.station(rng) else {
            return false;
        };
        let length = rng.range(t.length_m);
        let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let (run, half_width) = (&self.roads[road].run, self.roads[road].half_width);
        if s + length > run.length() || self.half(run.at(s + length / 2.0)) != half {
            return false;
        }
        // Past the gap by a hand's width, so rounding cannot put a tree's
        // ground inside it.
        let offset = half_width + t.road_gap_m + t.width_m / 2.0 + 0.1;
        let steps = libm::ceil(length / self.rules.line_step_m).max(1.0) as usize;
        let mut points: Vec<Point> = Vec::new();
        for step in 0..=steps {
            let at = s + length * step as f64 / steps as f64;
            let (a, b) = (
                run.at((at - 1.0).max(0.0)),
                run.at((at + 1.0).min(run.length())),
            );
            if a == b {
                return false;
            }
            let along = scale(sub(b, a), 1.0 / distance(a, b));
            points.push(add(run.at(at), scale([-along[1], along[0]], side * offset)));
        }
        self.tree_line(&Run::new(points), t.road_gap_m, rng, |_, _| true)
    }

    /// A tree line across open ground about `p`: along or square to a road
    /// near enough to set the fields' lie, on any bearing otherwise.
    fn field_line(
        &mut self,
        p: Point,
        rng: &mut Stream,
        clear: impl Fn(Point, f64) -> bool,
    ) -> bool {
        let t = self.rules.tree_lines;
        let road_gap = self.rules.clear.road_m;
        let length = rng.range(t.length_m);
        let lie = self
            .network
            .heading_near(p, t.align_m, || self.charge(1))
            .map(|heading| heading + if rng.chance(0.5) { 0.0 } else { TAU / 4.0 })
            .unwrap_or_else(|| rng.range([0.0, PI]));
        let reach = scale(direction(lie), length / 2.0);
        let line = Run::new(vec![sub(p, reach), add(p, reach)]);
        self.tree_line(&line, road_gap, rng, clear)
    }

    /// One transaction owns physical proposal admission for coverage and
    /// per-kind balance. Rejected geometry never becomes a later exclusion.
    fn try_feature(
        &mut self,
        kind: FillKind,
        p: Point,
        rng: &mut Stream,
        coverage: &mut coverage::Coverage<'_>,
        start: usize,
        target: Option<(Point, Point)>,
    ) -> Result<bool, String> {
        let before = self.forests.len();
        let groves = self.groves.len();
        let tally = self.tally;
        let placed = match kind {
            FillKind::Copse => self.copse(p, rng, |p, r| coverage.body_clear(p, r)),
            FillKind::TreeLine => self.field_line(p, rng, |p, r| coverage.body_clear(p, r)),
        };
        if placed && coverage.try_forests(start + before, &self.forests[before..], target)? {
            return Ok(true);
        }
        self.forests.truncate(before);
        self.groves.truncate(groves);
        self.tally = tally;
        coverage.check()?;
        Ok(false)
    }
}

/// Furnish the open country of a plan whose districts are built
/// (`parcels::fill_districts`). The result is the same plan with its lanes,
/// yards, homes, trees and low cover added, and its open approaches
/// measured again.
pub(crate) fn furnish(
    mut plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let rules = &presets.open_country;
    let placed = {
        let mut country = Country::new(&plan, request, presets);
        let fits: Vec<Fit> = catalogue.templates().iter().map(Fit::new).collect();
        let family = crate::parcels::family(request, presets);
        let (preset, _) = presets.cell(request.map_type, request.size);
        let h = &rules.homesteads;
        let homes = &h.steading.pieces;
        let homesteads = h.steading.with(homes.eligible(&fits, preset, family));
        let open_km2 = country.estimated_open_km2();
        let e = rules.fairness;

        // A row's count, on average over its weights.
        let mean = |rows: &mut dyn Iterator<Item = (f64, [u32; 2])>| {
            let (mut weights, mut sum) = (0.0, 0.0);
            for (weight, [low, high]) in rows {
                weights += weight;
                sum += weight * f64::from(low + high) / 2.0;
            }
            sum / weights
        };
        let mut rng = country.stream("homesteads");
        let groups = libm::round(h.per_road_km * country.road_m / 1000.0);
        let per_group = mean(&mut homes.groups.iter().map(|group| (group.weight, group.homes)));
        country.spread(
            groups * per_group,
            e.homes,
            h.attempts,
            &mut rng,
            |country| country.tally.homes,
            |country, half, rng| country.stead(half, &homesteads, rng),
        );

        let d = &rules.depots;
        let mut rng = country.stream("depots");
        country.spread(
            libm::round(d.per_road_km * country.road_m / 1000.0),
            e.depots,
            d.attempts,
            &mut rng,
            |country| country.tally.depots,
            |country, half, rng| country.stead(half, &d.steading, rng),
        );

        let t = rules.tree_lines;
        let mut rng = country.stream("tree_lines");
        let length = (t.length_m[0] + t.length_m[1]) / 2.0;
        country.spread(
            libm::round(t.per_km2 * open_km2) * length,
            e.tree_line_m,
            t.attempts,
            &mut rng,
            |country| country.tally.tree_line_m,
            |country, half, rng| {
                if rng.chance(t.roadside_chance) {
                    country.roadside_line(half, rng)
                } else {
                    let p = country.anywhere(half, rng);
                    country.field_line(p, rng, |_, _| true)
                }
            },
        );

        let k = rules.copses;
        let mut rng = country.stream("copses");
        country.spread(
            libm::round(k.per_km2 * open_km2),
            e.copses,
            k.attempts,
            &mut rng,
            |country| country.tally.copses,
            |country, half, rng| {
                let p = country.anywhere(half, rng);
                country.copse(p, rng, |_, _| true)
            },
        );

        let l = rules.lone_trees;
        let mut rng = country.stream("lone_trees");
        let yard_trees: f64 = country.tally.trees.iter().sum();
        country.spread(
            yard_trees + libm::round(l.per_km2 * open_km2),
            e.trees,
            l.attempts,
            &mut rng,
            |country| country.tally.trees,
            |country, half, rng| {
                let p = country.anywhere(half, rng);
                country.lone_tree(p)
            },
        );

        let f = &rules.field_cover;
        let mut rng = country.stream("field_cover");
        let clusters = libm::round(f.per_km2 * open_km2);
        let bodies = mean(&mut f.bodies.iter().map(|row| (row.weight, row.count)));
        let in_yards: f64 = country.tally.cover.iter().sum();
        country.spread(
            in_yards + clusters * bodies,
            e.cover,
            f.attempts,
            &mut rng,
            |country| country.tally.cover,
            |country, half, rng| {
                let p = country.anywhere(half, rng);
                country.cover(p, rng)
            },
        );
        (
            country.surfaces,
            country.lots,
            country.buildings,
            country.forests,
            country.props,
            country.kept,
        )
    };
    let (surfaces, lots, buildings, forests, props, kept) = placed;
    plan.surfaces.extend(surfaces);
    plan.lots.extend(lots);
    plan.buildings.extend(buildings);
    plan.forests.extend(forests);
    plan.props.extend(props);

    let main = plan.approaches.iter().any(|a| a.settlement == 0);
    retain_approaches(&mut plan, presets, kept);
    let metrics = crate::layout::measure(&plan, presets);
    let fail = |feature: &str, location: &str, message: String| {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some(feature.into()),
            location: location.into(),
            message: format!(
                "{message} ({} {}, seed {})",
                request.map_type.name(),
                request.size.name(),
                request.seed.value()
            ),
        }]
    };
    if main && !(metrics.main_approach_top && metrics.main_approach_bottom) {
        return Err(fail(
            "approach",
            "$.presets.open_country.clear.approach_m",
            "the open country closed the main settlement's approach in a half".into(),
        ));
    }
    if metrics.roads.unbridged > 0 {
        return Err(fail(
            "lanes",
            "$.presets.open_country",
            format!(
                "{} runs of a homestead's or depot's lane enter the water",
                metrics.roads.unbridged
            ),
        ));
    }
    if metrics.ground_points > request.limits.max_ground_points {
        return Err(vec![Diagnostic {
            code: DiagnosticCode::ComplexityLimit,
            feature: None,
            location: "$.limits.max_ground_points".into(),
            message: format!(
                "the generated plan has {} ground points",
                metrics.ground_points
            ),
        }]);
    }
    Ok(plan)
}

fn retain_approaches(plan: &mut MapPlan, presets: &PresetDefinitions, kept: Vec<(usize, f64)>) {
    // A home or a tree ends an open approach. The plan records the kept
    // ones, each as wide as its finished ground measures round its middle.
    let mut measured: Vec<Option<crate::ApproachPlan>> = crate::layout::approaches(plan, presets)
        .into_iter()
        .map(Some)
        .collect();
    let mut approaches = Vec::new();
    for (settlement, middle) in kept {
        let step = crate::layout::bearing_step(
            &plan.settlements[settlement],
            presets.approach.depth_m(plan.size[0].min(plan.size[1])),
        );
        let holds = |approach: &crate::ApproachPlan| {
            let to = if approach.to_rad < approach.from_rad {
                approach.to_rad + TAU
            } else {
                approach.to_rad
            };
            // The middle of a run of bearings may lie between two of them.
            approach.settlement == settlement
                && [middle, middle + TAU, middle - TAU]
                    .iter()
                    .any(|at| *at >= approach.from_rad - 0.51 * step && *at <= to + 0.51 * step)
        };
        if let Some(found) = measured
            .iter_mut()
            .find(|approach| approach.as_ref().is_some_and(holds))
        {
            approaches.extend(found.take());
        }
    }
    approaches.sort_by(|a, b| {
        a.settlement
            .cmp(&b.settlement)
            .then(a.from_rad.total_cmp(&b.from_rad))
    });
    plan.approaches = approaches;
}

pub(crate) fn admit_coverage(
    request: &GenerationRequest,
    presets: &PresetDefinitions,
    physics: &contract::generation_physics::GenerationPhysics,
) -> Result<(), Vec<Diagnostic>> {
    let range = physics.circular_range_m().map_err(|message| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidPhysicalRules,
            feature: None,
            location: "$.rules".into(),
            message,
        }]
    })?;
    coverage::preflight(
        [request.extent_m(); 2],
        presets.terrain.fog_cell_m,
        presets.open_country.sight.cell_m,
        range,
    )
    .and_then(|()| {
        coverage::tree_line_work(
            presets.open_country.tree_lines.length_m[1],
            presets.open_country.tree_lines,
        )
        .map(|_| ())
    })
    .map_err(|message| {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some("ground_sight".into()),
            location: "$.presets.open_country.sight.cell_m".into(),
            message,
        }]
    })
}
