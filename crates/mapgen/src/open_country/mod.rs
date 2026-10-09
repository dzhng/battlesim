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

mod coverage;
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
    steads: Vec<Point>,
    tally: Tally,
    next_prop: u32,
    lanes: Vec<SurfaceArea>,
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
    /// A yard it may touch: its own.
    own_yard: Option<usize>,
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
                self.charge(1)
                    && (ask.own_yard == Some(index) || beyond(rect_gap(yard, p), r + c.yard_m))
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
        let (group, choices) = rng
            .pick(h.groups.iter().zip(fits).map(|pair| (pair, pair.0.weight)))
            .expect("the open country has homestead groups");
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
        let lots_before = self.lots.len();
        // Each side of the line is walked on its own.
        let mut cursor = [0.0_f64; 2];
        for _ in 0..homes {
            // A category by its weight, then one of its templates.
            let eligible = choices.category(rng);
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
            let row = rng
                .pick(h.bodies.iter().map(|row| (row, row.weight)))
                .expect("a yard has bodies");
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
                wreck_of: None,
            },
        });
        self.bodies.push((at, radius));
        self.tally.cover[self.half(at)] += 1.0;
    }

    /// A few bodies of one kind about `p`.
    fn cover(&mut self, p: Point, rng: &mut Stream) -> bool {
        let rules = self.rules;
        let f = &rules.field_cover;
        let row = rng
            .pick(f.bodies.iter().map(|row| (row, row.weight)))
            .expect("field cover has bodies");
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
pub(crate) fn furnish(
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
                let mix = group
                    .mix
                    .iter()
                    .filter_map(|(name, weight)| Some((category_of(name)?, *weight)))
                    .filter(|(category, _)| preset.categories.contains(category));
                Choices::available(&fits, mix, family, preset.max_floors)
            })
            .collect();
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
            let steps = libm::ceil(whole.length() / presets.open_country.walk_m).max(1.0) as usize;
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
        let clearance = presets.rivers.bank_m() + presets.roads.dirt_track_width_m / 2.0;
        Self {
            request,
            presets,
            rules,
            size,
            objectives: crate::skirmish::objective_clearances(plan.skirmish.as_ref()).collect(),
            network: Network::new(plan, clearance, &presets.parcels.geometry),
            towns,
            woods,
            water,
            decks,
            corridors,
            kept,
            main_only: false,
            roads,
            road_m,
            proof_fill: false,
            work: None,
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

/// Complete-map certificate and bounded furnishing of any remaining cells.
/// Bodies are final before this runs; only forests can be appended here.
pub(crate) fn cover(
    mut plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
    physics: &contract::generation_physics::GenerationPhysics,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let fail = |message: String| {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some("ground_sight".into()),
            location: "$.presets.open_country".into(),
            message: format!(
                "{message} ({} {}, seed {})",
                request.map_type.name(),
                request.size.name(),
                request.seed.value()
            ),
        }]
    };
    let (map, _) = crate::materialize(
        &crate::CompileRequest::generated(request, plan.clone()),
        catalogue,
    )?;
    let cell = presets.open_country.sight.cell_m;
    let mut coverage = coverage::Coverage::new(&map, physics, cell).map_err(fail)?;
    let (added, kept) = {
        let mut country = Country::new(&plan, request, presets);
        country.proof_fill = true;
        country.work = Some(coverage.work());
        let initial = measure(&plan, presets);
        country.tally.copses = [initial.copses.top, initial.copses.bottom];
        country.tally.tree_line_m = [initial.tree_line_m.top, initial.tree_line_m.bottom];
        let mut rng = country.stream("ground_sight");
        let step = (cell / 2.).min(map.fog_cell_m);
        let span = libm::floor(coverage.reach / step) as i64;
        let mut offsets: Vec<_> = (-span..=span)
            .flat_map(|j| (-span..=span).map(move |i| [i as f64 * step, j as f64 * step]))
            .filter(|p| libm::hypot(p[0], p[1]) <= coverage.reach)
            .collect();
        offsets.sort_by(|a, b| {
            libm::hypot(a[0], a[1])
                .total_cmp(&libm::hypot(b[0], b[1]))
                .then(a[0].total_cmp(&b[0]))
                .then(a[1].total_cmp(&b[1]))
        });
        let nx = libm::ceil(plan.size[0] / cell) as usize;
        let ny = libm::ceil(plan.size[1] / cell) as usize;
        for y in 0..ny {
            for x in 0..nx {
                let low = [x as f64 * cell, y as f64 * cell];
                let high = [
                    (low[0] + cell).min(plan.size[0]),
                    (low[1] + cell).min(plan.size[1]),
                ];
                let at = scale(add(low, high), 0.5);
                let covered = coverage.covered(low, high);
                coverage.check().map_err(fail)?;
                if covered {
                    continue;
                }
                // Every fallback location is finite, and successful placement
                // still has to earn coverage through actual shared tree geometry.
                let mut found = false;
                for main_only in [false, true] {
                    country.main_only = main_only;
                    for offset in &offsets {
                        let p = add(at, *offset);
                        // Search feature centres in the same inward cone as
                        // the witness. This is a conservative proposal filter;
                        // actual patches must still earn the full certificate.
                        if !coverage.admits(p, low, high) {
                            coverage.check().map_err(fail)?;
                            continue;
                        }
                        for kind in &presets.open_country.sight.fill {
                            if country
                                .try_feature(
                                    *kind,
                                    p,
                                    &mut rng,
                                    &mut coverage,
                                    map.forests.len(),
                                    Some((low, high)),
                                )
                                .map_err(|message| {
                                    fail(format!(
                                        "{message} while covering clipped cell [{x},{y}] at {at:?}"
                                    ))
                                })?
                            {
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                        coverage.check().map_err(fail)?;
                    }
                    if found {
                        break;
                    }
                }
                if !found {
                    return Err(fail(format!("no physically certified sight feature covers clipped cell [{x},{y}] at {at:?}")));
                }
            }
        }
        // Whole-cell furnishing can need more features in one half. Earn
        // the existing per-kind fairness again by adding only real features
        // in the lesser half, using the same placement and proof owners.
        for kind in &presets.open_country.sight.fill {
            let (least, attempts) = match kind {
                FillKind::Copse => (
                    presets.open_country.fairness.copses,
                    presets.open_country.copses.attempts,
                ),
                FillKind::TreeLine => (
                    presets.open_country.fairness.tree_line_m,
                    presets.open_country.tree_lines.attempts,
                ),
            };
            let mut rng = country.stream(match kind {
                FillKind::Copse => "coverage_balance/copses",
                FillKind::TreeLine => "coverage_balance/tree_lines",
            });
            for _ in 0..presets.open_country.placed_max {
                let held = match kind {
                    FillKind::Copse => country.tally.copses,
                    FillKind::TreeLine => country.tally.tree_line_m,
                };
                if even(presets.open_country.fairness.rel, least, held[0], held[1]) {
                    break;
                }
                let half = usize::from(held[1] < held[0]);
                let mut stood = false;
                for _ in 0..attempts {
                    let p = country.anywhere(half, &mut rng);
                    if country
                        .try_feature(*kind, p, &mut rng, &mut coverage, map.forests.len(), None)
                        .map_err(fail)?
                    {
                        stood = true;
                        break;
                    }
                }
                if !stood {
                    return Err(fail(format!(
                        "no physical {kind:?} can restore country fairness"
                    )));
                }
            }
        }
        coverage.check().map_err(fail)?;
        (country.forests, country.kept)
    };
    plan.forests.extend(added);
    retain_approaches(&mut plan, presets, kept);
    let country = measure(&plan, presets);
    if !(country.homes.fair
        && country.tree_line_m.fair
        && country.copses.fair
        && country.trees.fair
        && country.cover.fair)
    {
        return Err(fail(
            "physical furnishing leaves uneven country halves".into(),
        ));
    }
    let metrics = crate::layout::measure(&plan, presets);
    if !(metrics.main_approach_top && metrics.main_approach_bottom) {
        return Err(fail(
            "physical furnishing closed a mandatory main firing lane".into(),
        ));
    }
    Ok(plan)
}
