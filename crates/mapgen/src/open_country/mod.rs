//! Open country (M24, M25): what stands between the settlements and the
//! woods. A few homes along the country roads, short tree lines, copses and
//! single trees, and low cover in the fields. The target is interrupted sight
//! at every playable location, with most bearings open. Authored-outline
//! proximity is a construction heuristic, not a physical sight certificate.
//!
//! The pass runs on a plan whose districts are built. It adds lanes to
//! `surfaces`, parcels to `lots`, homes to `buildings`, tree lines, copses
//! and trees to `forests` (a tree line is a stroke, a single tree a plot one
//! trunk stands on) and low cover to `props`, then measures the open
//! approaches again, because a home or a tree ends one.
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
//! - **Bare ground is furnished by proximity**: open ground is
//!   walked as cells, and a cell with no building, wood or tree line within
//!   `sight.reach_m` gets a copse or a tree line near it. The rule is judged
//!   by the simulation's own sight (`examples/sight_report`), not here.
//! - **The halves are even by construction**: each kind is placed in the
//!   half that holds less of it, until the map holds its share.
//!
//! Every number is a row of `open_country` in the presets, and each step
//! draws from its own named stream.
use crate::layout::geometry::{
    add, direction, distance, dot, ring_distance, round_cm, scale, sub, thinned, Outline, Point,
    PI, TAU,
};
use crate::layout::rng::Stream;
use crate::layout::water::Water;
use crate::layout::{corridor_start, GenerationRequest, LotRule, OutlineShape, PresetDefinitions};
use crate::parcels::lots::{placement, Fit, Frontage, Lot};
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
/// A tree line follows its road by points this far apart.
const LINE_STEP_M: f64 = 20.0;
/// Road is measured, and a settlement's edge sampled, this often.
const WALK_M: f64 = 50.0;
/// The most things of one kind a map is given, whatever its rows ask.
const PLACED_MAX: u32 = 4096;

mod rules;
use rules::category_of;
pub use rules::*;

/// One thing's amount in each half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Split {
    pub top: f64,
    pub bottom: f64,
    pub fair: bool,
}

/// What the open country holds, read from the plan's geometry alone.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CountryMetrics {
    /// Buildings outside every settlement's outline.
    pub homes: Split,
    /// Metres of tree line.
    pub tree_line_m: Split,
    /// Small woods: larger than a tree's plot, smaller than any wood.
    pub copses: Split,
    /// Single trees and the clumps beside yards.
    pub trees: Split,
    /// Bodies outside every settlement's outline.
    pub cover: Split,
}

/// Whether `p` lies in no settlement's outline.
fn outside(plan: &MapPlan, limits: &[[f64; 4]], p: Point) -> bool {
    !plan
        .settlements
        .iter()
        .zip(limits)
        .any(|(settlement, [x0, y0, x1, y1])| {
            p[0] >= *x0
                && p[0] <= *x1
                && p[1] >= *y0
                && p[1] <= *y1
                && polygon_contains(&settlement.outline, p)
        })
}

fn outline_limits(plan: &MapPlan) -> Vec<[f64; 4]> {
    plan.settlements
        .iter()
        .map(|settlement| contract::ground::limits(&settlement.outline, 0.0))
        .collect()
}

/// The parcels outside every settlement's outline: the yards of the homes
/// in open country.
pub fn country_lots(plan: &MapPlan) -> impl Iterator<Item = &LotPlan> {
    let limits = outline_limits(plan);
    plan.lots
        .iter()
        .filter(move |lot| outside(plan, &limits, lot.ring[0]))
}

/// The open country of a plan, by half, held to the presets' evenness.
pub fn measure(plan: &MapPlan, presets: &PresetDefinitions) -> CountryMetrics {
    let rules = &presets.open_country;
    let middle = plan.size[1] / 2.0;
    let limits = outline_limits(plan);
    let wood_floor = presets.wood_floor_m2();
    let mut tally = [[0.0; 2]; 5];
    let mut count = |row: usize, p: Point, amount: f64| {
        tally[row][usize::from(p[1] < middle)] += amount;
    };
    for building in &plan.buildings {
        let [x, y, _] = building.frame.translation;
        if outside(plan, &limits, [x, y]) {
            count(0, [x, y], 1.0);
        }
    }
    for forest in &plan.forests {
        match &forest.shape {
            GroundShape::Stroke { centerline, .. } => {
                for pair in centerline.samples().windows(2) {
                    count(
                        1,
                        scale(add(pair[0], pair[1]), 0.5),
                        distance(pair[0], pair[1]),
                    );
                }
            }
            GroundShape::Polygon { ring } => {
                let size = crate::layout::geometry::area(ring);
                let middle = crate::layout::geometry::centroid(ring);
                if size < rules.copses.area_m2[0] {
                    count(3, middle, 1.0);
                } else if size < wood_floor {
                    count(2, middle, 1.0);
                }
            }
        }
    }
    for prop in &plan.props {
        if outside(plan, &limits, prop.center) {
            count(4, prop.center, 1.0);
        }
    }
    let e = &rules.fairness;
    let split = |row: usize, least: f64| {
        let [top, bottom] = tally[row];
        Split {
            top,
            bottom,
            fair: even(e.rel, least, top, bottom),
        }
    };
    CountryMetrics {
        homes: split(0, e.homes),
        tree_line_m: split(1, e.tree_line_m),
        copses: split(2, e.copses),
        trees: split(3, e.trees),
        cover: split(4, e.cover),
    }
}

fn even(rel: f64, least: f64, top: f64, bottom: f64) -> bool {
    (top - bottom).abs() <= (rel * (top + bottom)).max(least)
}

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
        let [x0, y0, x1, y1] = self.limits;
        p[0] < x0 - gap
            || p[0] > x1 + gap
            || p[1] < y0 - gap
            || p[1] > y1 + gap
            || ring_distance(self.ring, p) >= gap
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

/// Which cells of open ground have something that cuts sight within reach.
struct Bare {
    cell: f64,
    columns: usize,
    rows: usize,
    reach: f64,
    /// Open ground: outside settlements, woods and water.
    open: Vec<bool>,
    seen: Vec<bool>,
}

impl Bare {
    fn middle(&self, index: usize) -> Point {
        [
            ((index % self.columns) as f64 + 0.5) * self.cell,
            ((index / self.columns) as f64 + 0.5) * self.cell,
        ]
    }

    /// Something that cuts sight stands at `p`.
    fn mark(&mut self, p: Point) {
        let span = |v: f64, count: usize| {
            let low = libm::floor((v - self.reach) / self.cell).max(0.0) as usize;
            let high = (libm::floor((v + self.reach) / self.cell).max(0.0) as usize).min(count - 1);
            low..=high
        };
        for row in span(p[1], self.rows) {
            for column in span(p[0], self.columns) {
                let index = row * self.columns + column;
                if !self.seen[index] && distance(self.middle(index), p) <= self.reach {
                    self.seen[index] = true;
                }
            }
        }
    }
}

/// The amounts placed so far, by half: `[top, bottom]`.
#[derive(Default)]
struct Tally {
    homes: [f64; 2],
    tree_line_m: [f64; 2],
    copses: [f64; 2],
    trees: [f64; 2],
    cover: [f64; 2],
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
    bare: Bare,
    yards: Vec<Rect>,
    /// Copses, trees and tree lines placed, as discs.
    groves: Vec<(Point, f64)>,
    bodies: Vec<(Point, f64)>,
    steads: Vec<Point>,
    tally: Tally,
    next_prop: u32,
    lanes: Vec<SurfaceArea>,
    lots: Vec<LotPlan>,
    buildings: Vec<BuildingPlacement>,
    forests: Vec<Forest>,
    props: Vec<AuthoredPropDefinition>,
}

/// One group's templates: each of its categories the catalogue can build,
/// with the category's weight.
type Choices<'a> = Vec<(f64, Vec<&'a Fit<'a>>)>;

/// What a place is asked for.
#[derive(Clone, Copy)]
struct Ask {
    radius: f64,
    /// Whether it blocks sight: a building or trees, kept out of the
    /// approach corridors.
    blocks_sight: bool,
    road_gap: f64,
    /// A yard it may touch: its own.
    own_yard: Option<usize>,
}

impl<'a> Country<'a> {
    fn stream(&self, name: &str) -> Stream {
        crate::layout::stream(self.request, &format!("country/{name}"))
    }

    fn half(&self, p: Point) -> usize {
        usize::from(p[1] < self.size[1] / 2.0)
    }

    /// The one question: may something of `ask` stand at `p`.
    fn open(&self, p: Point, ask: Ask) -> bool {
        let c = &self.rules.clear;
        let r = ask.radius;
        let edge = r + c.edge_m;
        if p[0] < edge || p[1] < edge || p[0] > self.size[0] - edge || p[1] > self.size[1] - edge {
            return false;
        }
        let road = r + ask.road_gap;
        let water = r + c.water_m;
        self.towns
            .iter()
            .all(|town| town.clear(p, r + c.settlement_m))
            && self.woods.iter().all(|wood| wood.clear(p, r + c.forest_m))
            && self.network.edge_gap(p, road) >= road
            && self.water.gap(p, water) >= water
            && self
                .decks
                .iter()
                .all(|deck| beyond(rect_gap(deck, p), road))
            && self.yards.iter().enumerate().all(|(index, yard)| {
                ask.own_yard == Some(index) || beyond(rect_gap(yard, p), r + c.yard_m)
            })
            && self
                .bodies
                .iter()
                .all(|(at, reach)| distance(*at, p) >= r + reach)
            && (!ask.blocks_sight
                || self.corridors.iter().all(|(corridor, main)| {
                    (self.main_only && !main) || beyond(rect_gap(corridor, p), r + c.approach_m)
                }))
    }

    /// Trees of `radius` at `p` keep their distance from those placed.
    fn apart(&self, p: Point, radius: f64) -> bool {
        self.groves
            .iter()
            .all(|(at, reach)| distance(*at, p) >= radius + reach + self.rules.clear.yard_m)
    }

    fn trees(&self, radius: f64) -> Ask {
        Ask {
            radius,
            blocks_sight: true,
            road_gap: self.rules.clear.road_m,
            own_yard: None,
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
        for _ in 0..PLACED_MAX {
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

    // ----- Homes -------------------------------------------------------

    /// One group of homes at a place on a road in `half`.
    fn stead(&mut self, half: usize, fits: &[Choices], rng: &mut Stream) -> bool {
        let (rules, presets) = (self.rules, self.presets);
        let h = &rules.homesteads;
        let Some((road, s)) = self.station(rng) else {
            return false;
        };
        let half_width = self.roads[road].half_width;
        let (at, ahead) = {
            let run = &self.roads[road].run;
            (run.at(s), run.at((s + 1.0).min(run.length())))
        };
        if at == ahead
            || self.half(at) != half
            || self
                .steads
                .iter()
                .any(|other| distance(*other, at) < h.apart_m)
        {
            return false;
        }
        let along = scale(sub(ahead, at), 1.0 / distance(at, ahead));
        let total: f64 = h.groups.iter().map(|group| group.weight).sum();
        let mut pick = rng.unit() * total;
        let index = h
            .groups
            .iter()
            .position(|group| {
                pick -= group.weight;
                pick < 0.0
            })
            .unwrap_or(h.groups.len() - 1);
        let (group, choices) = (&h.groups[index], &fits[index]);
        if choices.is_empty() {
            return false;
        }
        let homes = rng.count(group.homes);
        let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let lane = rng.chance(h.lane_chance);
        let lane_m = rng.range(h.lane_m);
        let verge = presets.parcels.verge_m;
        // The line the yards front, and how far from its middle they start.
        let (front, offset, lane) = if lane {
            let out = scale([-along[1], along[0]], side);
            let (from, to) = (
                round_cm(sub(at, scale(out, JOIN_OVERSHOOT_M))),
                round_cm(add(at, scale(out, lane_m))),
            );
            let width = presets.roads.dirt_track_width_m;
            // The lane's own ground, from the road's edge to its end.
            let ground = Ask {
                radius: width / 2.0,
                blocks_sight: false,
                road_gap: 0.0,
                own_yard: None,
            };
            let first = half_width + width;
            let steps = libm::ceil((lane_m - first) / width).max(1.0) as usize;
            let free = (0..=steps).all(|step| {
                let reach = first + (lane_m - first) * step as f64 / steps as f64;
                let p = add(at, scale(out, reach));
                self.open(p, ground) && self.apart(p, ground.radius)
            });
            let Ok(shape) = GroundShape::stroke(vec![from, to], width) else {
                return false;
            };
            if !free {
                return false;
            }
            // Yards stand along the lane's far end.
            let near = add(at, scale(out, (lane_m * 0.35).max(first + verge)));
            (
                Run::new(vec![near, to]),
                width / 2.0 + verge,
                Some((shape, [from, to], width / 2.0)),
            )
        } else {
            let run = &self.roads[road].run;
            let reach = f64::from(homes) * 60.0 + h.spread_m;
            let mut line: Vec<Point> = Vec::new();
            for step in 0..=8 {
                let p = run.at((s + reach * f64::from(step) / 8.0).min(run.length()));
                if line.last() != Some(&p) {
                    line.push(p);
                }
            }
            if line.len() < 2 {
                return false;
            }
            (Run::new(line), half_width + verge, None)
        };
        let rule = LotRule {
            front_m: h.yard.front_m,
            side_m: h.yard.side_m,
            rear_m: h.yard.rear_m,
            coverage: 1.0,
            apron_m: 0.0,
        };
        let stead = self.steads.len();
        let (lots_before, yards_before) = (self.lots.len(), self.yards.len());
        // Each side of the line is walked on its own.
        let mut cursor = [0.0_f64; 2];
        for _ in 0..homes {
            // A category by its weight, then one of its templates.
            let weights: f64 = choices.iter().map(|(weight, _)| weight).sum();
            let mut pick = rng.unit() * weights;
            let (_, eligible) = choices
                .iter()
                .find(|(weight, _)| {
                    pick -= weight;
                    pick < 0.0
                })
                .unwrap_or(&choices[choices.len() - 1]);
            let fit = eligible[rng.below(eligible.len() as u64) as usize];
            let across = if lane.is_some() || homes > 2 {
                usize::from(rng.chance(0.5))
            } else {
                usize::from(side < 0.0)
            };
            let frontage = Frontage {
                run: &front,
                side: if across == 0 { 1.0 } else { -1.0 },
                offset,
            };
            let gap = rng.range([0.0, h.spread_m]);
            let Some(lot) = frontage.lot(cursor[across], fit, &rule) else {
                continue;
            };
            // A lane is a carriageway its yards keep off, like any other.
            let on_lane = lane.as_ref().is_some_and(|(_, [from, to], half)| {
                lot.rect.segment_gap(*from, *to) < half - 0.01
            });
            if on_lane || !self.yard_fits(&lot.rect) {
                cursor[across] += presets.parcels.lot_step_m * 4.0;
                continue;
            }
            cursor[across] += 2.0 * lot.rect.half[0] + gap;
            let id = format!(
                "country/stead-{stead}/lot-{}",
                self.lots.len() - lots_before
            );
            self.yards.push(lot.rect);
            self.lots.push(LotPlan {
                id: id.clone(),
                ring: lot.rect.corners().map(round_cm).to_vec(),
            });
            let building = placement(
                id,
                &lot,
                fit,
                &rule,
                &presets.parcels.prop_kind,
                &mut self.next_prop,
            );
            self.furnish_yard(&lot, fit, &building.frame, rng);
            self.buildings.push(building);
            self.tally.homes[half] += 1.0;
        }
        if self.lots.len() == lots_before {
            // No yard found room: a lane would lead nowhere, so none is laid.
            return false;
        }
        if let Some((shape, ..)) = lane {
            self.network.add(&shape);
            self.lanes.push(SurfaceArea {
                kind: SurfaceKind::DirtTrack,
                shape,
            });
        }
        self.steads.push(at);
        for index in yards_before..self.yards.len() {
            let middle = self.yards[index].center;
            self.bare.mark(middle);
        }
        true
    }

    /// A yard may stand here: on open ground at every corner, off every
    /// carriageway, yard and kept corridor.
    fn yard_fits(&self, rect: &Rect) -> bool {
        let c = &self.rules.clear;
        let corner = Ask {
            radius: 0.0,
            blocks_sight: false,
            road_gap: 0.0,
            own_yard: None,
        };
        let grown = |by: f64| Rect {
            half: [rect.half[0] + by, rect.half[1] + by],
            ..*rect
        };
        rect.corners()
            .into_iter()
            .chain([rect.center])
            .all(|p| self.open(p, corner) && self.apart(p, 0.0))
            && !self.network.covers(rect)
            && !self
                .yards
                .iter()
                .any(|yard| grown(c.yard_m).overlaps(yard, 0.0))
            && !self
                .corridors
                .iter()
                .any(|(corridor, _)| grown(c.approach_m).overlaps(corridor, 0.0))
            && !self.towns.iter().any(|town| rect.touches(town.ring))
            && !self.woods.iter().any(|wood| rect.touches(wood.ring))
    }

    /// The clump of trees behind a yard and the body beside its house.
    fn furnish_yard(&mut self, lot: &Lot, fit: &Fit, frame: &PlacementFrame, rng: &mut Stream) {
        let rules = self.rules;
        let h = &rules.homesteads;
        let own = self.yards.len() - 1;
        let depth = 2.0 * lot.rect.half[1];
        if rng.chance(h.tree_chance) {
            let sides = [rng.range(h.tree_plot_m), rng.range(h.tree_plot_m)];
            let reach = (lot.rect.half[0] - sides[0] / 2.0).max(0.0);
            let plot = Rect {
                center: lot.point(rng.range([-reach, reach]), depth + sides[1] / 2.0 + 0.5),
                axis: lot.rect.axis,
                half: [sides[0] / 2.0, sides[1] / 2.0],
            };
            let radius = libm::hypot(plot.half[0], plot.half[1]);
            let ask = Ask {
                own_yard: Some(own),
                ..self.trees(radius)
            };
            if self.open(plot.center, ask) && self.apart(plot.center, radius) {
                self.plant(plot.corners().to_vec(), plot.center, radius);
                self.tally.trees[self.half(plot.center)] += 1.0;
            }
        }
        if rng.chance(h.body_chance) {
            let total: f64 = h.bodies.iter().map(|row| row.weight).sum();
            let mut pick = rng.unit() * total;
            let row = h
                .bodies
                .iter()
                .find(|row| {
                    pick -= row.weight;
                    pick < 0.0
                })
                .unwrap_or(&h.bodies[h.bodies.len() - 1]);
            // Beside the house, in the side setback, nose to the road.
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            let box_half = (fit.max[0] - fit.min[0]) / 2.0;
            let at = lot.point(
                side * (box_half + h.yard.side_m / 2.0),
                h.yard.front_m + row.half_extents_m[0] + rng.range([0.0, 3.0]),
            );
            let radius = libm::hypot(row.half_extents_m[0], row.half_extents_m[1]);
            let doors = fit
                .template
                .materialize(*frame)
                .ok()
                .and_then(|placed| placed.entrances)
                .unwrap_or_default();
            let clear = doors.iter().all(|door| {
                let from = [door.position[0], door.position[1]];
                let to = add(from, scale(door.normal, h.yard.front_m));
                crate::layout::geometry::segment_distance(from, to, at)
                    >= radius + h.yard.door_clear_m
            });
            let ask = Ask {
                radius,
                blocks_sight: false,
                road_gap: rules.clear.road_m,
                own_yard: Some(own),
            };
            let yaw = libm::atan2(lot.inward[1], lot.inward[0]);
            if clear && self.open(at, ask) {
                self.body(row, at, yaw, radius);
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

    fn copse(&mut self, p: Point, rng: &mut Stream) -> bool {
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
        if !(self.open(outline.center, self.trees(outline.reach))
            && self.apart(outline.center, outline.reach)
            && self.plant(outline.ring, outline.center, outline.reach))
        {
            return false;
        }
        self.tally.copses[self.half(outline.center)] += 1.0;
        self.bare.mark(outline.center);
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
    fn tree_line(&mut self, line: &Run, road_gap: f64, rng: &mut Stream) -> bool {
        let t = self.rules.tree_lines;
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
                .all(|p| self.open(*p, ask) && self.apart(*p, half))
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
                self.bare.mark(middle);
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
        let steps = libm::ceil(length / LINE_STEP_M).max(1.0) as usize;
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
        self.tree_line(&Run::new(points), t.road_gap_m, rng)
    }

    /// A tree line across open ground about `p`: along or square to a road
    /// near enough to set the fields' lie, on any bearing otherwise.
    fn field_line(&mut self, p: Point, rng: &mut Stream) -> bool {
        let t = self.rules.tree_lines;
        let road_gap = self.rules.clear.road_m;
        let length = rng.range(t.length_m);
        let lie = self
            .network
            .heading_near(p, t.align_m)
            .map(|heading| heading + if rng.chance(0.5) { 0.0 } else { TAU / 4.0 })
            .unwrap_or_else(|| rng.range([0.0, PI]));
        let reach = scale(direction(lie), length / 2.0);
        let line = Run::new(vec![sub(p, reach), add(p, reach)]);
        self.tree_line(&line, road_gap, rng)
    }

    // ----- Bare ground -------------------------------------------------

    /// Stand a copse or a tree line by every cell of open ground that has
    /// nothing to cut its sight within reach.
    fn fill_bare(&mut self) {
        let sight = &self.rules.sight;
        let mut rng = self.stream("sight");
        let mut cells: Vec<usize> = (0..self.bare.open.len())
            .filter(|index| self.bare.open[*index])
            .collect();
        for index in (1..cells.len()).rev() {
            cells.swap(index, rng.below(index as u64 + 1) as usize);
        }
        let weights: f64 = sight.fill.values().sum();
        for cell in &cells {
            if self.bare.seen[*cell] {
                continue;
            }
            let middle = self.bare.middle(*cell);
            for attempt in 0..sight.attempts {
                // Near the bare ground first, then as far off as still cuts
                // its sight: a kept corridor has room only beside it.
                let wider = f64::from(attempt) / f64::from(sight.attempts);
                let scatter = sight.scatter + (1.0 - sight.scatter) * wider;
                let away = sight.reach_m * scatter * libm::sqrt(rng.unit());
                let p = add(middle, scale(direction(rng.range([0.0, TAU])), away));
                let mut pick = rng.unit() * weights;
                let kind = sight
                    .fill
                    .iter()
                    .find(|(_, weight)| {
                        pick -= **weight;
                        pick < 0.0
                    })
                    .map_or(FillKind::Copse, |(kind, _)| *kind);
                let stood = match kind {
                    FillKind::Copse => self.copse(p, &mut rng),
                    FillKind::TreeLine => self.field_line(p, &mut rng),
                };
                if stood && self.bare.seen[*cell] {
                    break;
                }
            }
        }
        // Ground still bare had no luck: every place within reach of it is
        // then tried in turn, nearest first, for a copse. Where kept
        // corridors lie all round it and none has room, it is tried once
        // more with only the main settlement's corridors kept.
        let step = sight.cell_m / 2.0;
        let span = libm::floor(sight.reach_m / step) as i64;
        let mut ring: Vec<(f64, Point)> = (-span..=span)
            .flat_map(|j| (-span..=span).map(move |i| [i as f64 * step, j as f64 * step]))
            .map(|offset| (libm::hypot(offset[0], offset[1]), offset))
            .filter(|(away, _)| *away <= sight.reach_m)
            .collect();
        ring.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then(a.1[0].total_cmp(&b.1[0]))
                .then(a.1[1].total_cmp(&b.1[1]))
        });
        for main_only in [false, true] {
            self.main_only = main_only;
            for cell in &cells {
                if self.bare.seen[*cell] {
                    continue;
                }
                let middle = self.bare.middle(*cell);
                for (_, offset) in &ring {
                    if self.copse(add(middle, *offset), &mut rng) {
                        break;
                    }
                }
            }
        }
        self.main_only = false;
    }

    // ----- Bodies ------------------------------------------------------

    fn body(&mut self, row: &Body, at: Point, yaw: f64, radius: f64) {
        let at = round_cm(at);
        self.props.push(AuthoredPropDefinition {
            id: None,
            geometry: PropDefinition {
                kind: row.kind.clone(),
                center: at,
                yaw: libm::round(yaw * 1e6) / 1e6,
                half_extents: row.half_extents_m,
                base_z: None,
            },
        });
        self.bodies.push((at, radius));
        self.tally.cover[self.half(at)] += 1.0;
    }

    /// A few bodies of one kind about `p`.
    fn cover(&mut self, p: Point, rng: &mut Stream) -> bool {
        let rules = self.rules;
        let f = &rules.field_cover;
        let total: f64 = f.bodies.iter().map(|row| row.weight).sum();
        let mut pick = rng.unit() * total;
        let row = f
            .bodies
            .iter()
            .find(|row| {
                pick -= row.weight;
                pick < 0.0
            })
            .unwrap_or(&f.bodies[f.bodies.len() - 1]);
        let radius = libm::hypot(row.half_extents_m[0], row.half_extents_m[1]);
        let ask = Ask {
            radius: radius + f.gap_m,
            blocks_sight: false,
            road_gap: rules.clear.road_m,
            own_yard: None,
        };
        let heading = rng.range([0.0, TAU]);
        // The cluster is found whole before any of it stands, so its own
        // bodies are judged against each other here and not as strangers.
        let mut cluster: Vec<(Point, f64)> = Vec::new();
        for index in 0..rng.count(row.count) {
            let (at, yaw) = if row.stacked {
                // Side by side across the heading.
                let step = 2.0 * row.half_extents_m[1] + 0.1;
                let across = direction(heading + TAU / 4.0);
                (add(p, scale(across, step * f64::from(index))), heading)
            } else if index == 0 {
                (p, heading)
            } else {
                let away = rng.range([0.0, f.spread_m]);
                (
                    add(p, scale(direction(rng.range([0.0, TAU])), away)),
                    rng.range([0.0, TAU]),
                )
            };
            let apart = row.stacked
                || cluster
                    .iter()
                    .all(|(other, _)| distance(*other, at) >= 2.0 * radius + f.gap_m);
            let free = self
                .groves
                .iter()
                .all(|(grove, reach)| distance(*grove, at) >= radius + reach);
            if apart && free && self.open(at, ask) {
                cluster.push((at, yaw));
            } else if index == 0 {
                return false;
            }
        }
        for (at, yaw) in cluster {
            self.body(row, at, yaw, radius);
        }
        true
    }
}

/// Furnish the open country of a plan whose districts are built
/// (`parcels::fill_districts`). The result is the same plan with its lanes,
/// yards, homes, trees and low cover added, and its open approaches
/// measured again.
pub fn furnish(
    mut plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let rules = &presets.open_country;
    let placed = {
        let mut country = Country::new(&plan, request, presets);
        let fits: Vec<Fit> = catalogue.templates().iter().filter_map(Fit::new).collect();
        let family = crate::parcels::family(request, presets);
        let (preset, _) = presets.cell(request.map_type, request.size);
        // Each group's templates: its categories, of the map's one family,
        // within the type's floors and categories.
        let eligible: Vec<Choices> = rules
            .homesteads
            .groups
            .iter()
            .map(|group| {
                group
                    .mix
                    .iter()
                    .map(|(name, weight)| {
                        let category = category_of(name);
                        let of_it: Vec<&Fit> = fits
                            .iter()
                            .filter(|fit| {
                                Some(fit.template.category) == category
                                    && fit.template.regional_family == family
                                    && preset.categories.contains(&fit.template.category)
                                    && preset
                                        .max_floors
                                        .is_none_or(|most| fit.floors() <= most as usize)
                            })
                            .collect();
                        (*weight, of_it)
                    })
                    .filter(|(_, of_it)| !of_it.is_empty())
                    .collect()
            })
            .collect();
        let open_km2 = country.bare.open.iter().filter(|open| **open).count() as f64
            * rules.sight.cell_m
            * rules.sight.cell_m
            / 1e6;
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
        let h = &rules.homesteads;
        let mut rng = country.stream("homesteads");
        let groups = libm::round(h.per_road_km * country.road_m / 1000.0);
        let homes = mean(&mut h.groups.iter().map(|group| (group.weight, group.homes)));
        country.spread(
            groups * homes,
            e.homes,
            h.attempts,
            &mut rng,
            |country| country.tally.homes,
            |country, half, rng| country.stead(half, &eligible, rng),
        );

        // Bare ground next: what cuts sight goes where nothing does yet.
        country.fill_bare();

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
                    country.field_line(p, rng)
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
                country.copse(p, rng)
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
            country.lanes,
            country.lots,
            country.buildings,
            country.forests,
            country.props,
            country.kept,
        )
    };
    let (lanes, lots, buildings, forests, props, kept) = placed;
    plan.surfaces.extend(lanes);
    plan.lots.extend(lots);
    plan.buildings.extend(buildings);
    plan.forests.extend(forests);
    plan.props.extend(props);

    // A home or a tree ends an open approach. The plan records the kept
    // ones, each as wide as its finished ground measures round its middle.
    let main = plan.approaches.iter().any(|a| a.settlement == 0);
    let mut measured: Vec<Option<crate::ApproachPlan>> = crate::layout::approaches(&plan, presets)
        .into_iter()
        .map(Some)
        .collect();
    let mut approaches = Vec::new();
    for (settlement, middle) in kept {
        let step =
            crate::layout::bearing_step(&plan.settlements[settlement], presets.approach.depth_m);
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
            "$.presets.open_country.homesteads",
            format!("{} runs of lane enter the water", metrics.roads.unbridged),
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

impl<'a> Country<'a> {
    fn new(
        plan: &'a MapPlan,
        request: &'a GenerationRequest,
        presets: &'a PresetDefinitions,
    ) -> Self {
        let rules = &presets.open_country;
        let size = plan.size;
        let towns: Vec<Ground> = plan
            .settlements
            .iter()
            .map(|settlement| Ground::new(&settlement.outline))
            .collect();
        let woods: Vec<Ground> = plan
            .forests
            .iter()
            .filter_map(|forest| match &forest.shape {
                GroundShape::Polygon { ring } => Some(Ground::new(ring)),
                GroundShape::Stroke { .. } => None,
            })
            .collect();
        let water = Water::new(&plan.rivers, size);
        let bridge = &presets.rivers.bridge;
        let decks = plan
            .bridges
            .iter()
            .map(|deck| Rect {
                center: deck.center,
                axis: direction(deck.yaw),
                half: [
                    deck.half_extents[0] + bridge.approach_m,
                    deck.half_extents[1],
                ],
            })
            .collect();
        // A settlement's widest approach in each half.
        let unwrapped = |approach: &crate::ApproachPlan| {
            if approach.to_rad < approach.from_rad {
                approach.to_rad + TAU
            } else {
                approach.to_rad
            }
        };
        let middle_of =
            |approach: &crate::ApproachPlan| (approach.from_rad + unwrapped(approach)) / 2.0;
        let mut widest: BTreeMap<(usize, u8), usize> = BTreeMap::new();
        for (index, approach) in plan.approaches.iter().enumerate() {
            let width = |approach: &crate::ApproachPlan| unwrapped(approach) - approach.from_rad;
            let key = u8::from(approach.half == crate::Half::Top);
            let best = widest.entry((approach.settlement, key)).or_insert(index);
            if width(approach) > width(&plan.approaches[*best]) {
                *best = index;
            }
        }
        let mut keep: Vec<usize> = widest.into_values().collect();
        keep.sort_unstable();
        keep.dedup();
        // The corridor of each: the bearings measured either side of its
        // middle, and the middle itself.
        let mut corridors = Vec::new();
        let mut kept = Vec::new();
        for approach in keep.into_iter().map(|index| &plan.approaches[index]) {
            let settlement = &plan.settlements[approach.settlement];
            let step = crate::layout::bearing_step(settlement, approach.depth_m);
            let (first, last) = (
                libm::round(approach.from_rad / step),
                libm::round(unwrapped(approach) / step),
            );
            let middle = (first + last) / 2.0;
            kept.push((approach.settlement, middle_of(approach)));
            for bearing in [
                libm::floor(middle) * step,
                libm::ceil(middle) * step,
                middle_of(approach),
            ] {
                let toward = direction(bearing);
                let edge = corridor_start(
                    &settlement.outline,
                    settlement.center,
                    toward,
                    approach.front_m,
                );
                let corridor = Rect {
                    center: add(
                        settlement.center,
                        scale(toward, edge + approach.depth_m / 2.0),
                    ),
                    axis: toward,
                    half: [approach.depth_m / 2.0, approach.front_m / 2.0],
                };
                corridors.push((corridor, approach.settlement == 0));
            }
        }
        // Country roads and tracks, cut where they run through a settlement.
        let mut roads = Vec::new();
        for area in &plan.surfaces {
            let (
                SurfaceKind::CountryRoad | SurfaceKind::DirtTrack,
                GroundShape::Stroke {
                    centerline,
                    width_m,
                },
            ) = (area.kind, &area.shape)
            else {
                continue;
            };
            let whole = Run::new(centerline.samples().to_vec());
            let steps = libm::ceil(whole.length() / WALK_M).max(1.0) as usize;
            let mut stretch: Vec<Point> = Vec::new();
            for step in 0..=steps {
                let p = whole.at(whole.length() * step as f64 / steps as f64);
                let country = towns
                    .iter()
                    .all(|town| town.clear(p, rules.clear.settlement_m));
                if country && stretch.last() != Some(&p) {
                    stretch.push(p);
                }
                if (!country || step == steps) && !stretch.is_empty() {
                    if stretch.len() > 1 {
                        roads.push(Road {
                            run: Run::new(core::mem::take(&mut stretch)),
                            half_width: width_m / 2.0,
                        });
                    }
                    stretch.clear();
                }
            }
        }
        let road_m = roads.iter().map(|road| road.run.length()).sum();
        let cell = rules.sight.cell_m;
        let count = |extent: f64| (libm::ceil(extent / cell) as usize).max(1);
        let (columns, rows) = (count(size[0]), count(size[1]));
        let mut bare = Bare {
            cell,
            columns,
            rows,
            reach: rules.sight.reach_m,
            open: vec![false; columns * rows],
            seen: vec![false; columns * rows],
        };
        for index in 0..columns * rows {
            let p = bare.middle(index);
            bare.open[index] = p[0] <= size[0]
                && p[1] <= size[1]
                && towns.iter().all(|town| town.clear(p, f64::MIN_POSITIVE))
                && woods.iter().all(|wood| wood.clear(p, f64::MIN_POSITIVE))
                && water.gap(p, 0.0) >= 0.0;
        }
        // What already cuts sight: every building, and the woods' edges.
        for building in &plan.buildings {
            let [x, y, _] = building.frame.translation;
            bare.mark([x, y]);
        }
        for wood in &woods {
            for (a, b) in contract::ground::edges(wood.ring) {
                let steps = libm::ceil(distance(*a, *b) / WALK_M).max(1.0) as usize;
                for step in 0..steps {
                    bare.mark(add(*a, scale(sub(*b, *a), step as f64 / steps as f64)));
                }
            }
        }
        let clearance = presets.rivers.bank_m() + presets.roads.dirt_track_width_m / 2.0;
        Self {
            request,
            presets,
            rules,
            size,
            network: Network::new(plan, clearance),
            towns,
            woods,
            water,
            decks,
            corridors,
            kept,
            main_only: false,
            roads,
            road_m,
            bare,
            yards: Vec::new(),
            groves: Vec::new(),
            bodies: Vec::new(),
            steads: Vec::new(),
            tally: Tally::default(),
            next_prop: plan
                .buildings
                .iter()
                .map(|building| building.parts.len() as u32)
                .sum(),
            lanes: Vec::new(),
            lots: Vec::new(),
            buildings: Vec::new(),
            forests: Vec::new(),
            props: Vec::new(),
        }
    }
}
