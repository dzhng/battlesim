//! What a plan is, measured from its geometry alone: areas by half, open
//! approaches, its river, and the road graph's reach and journey times. The
//! generator holds its own output to the presets with these numbers, and the
//! sweep and the inspection picture report them.
use super::geometry::{
    add, area, area_above, direction, distance, dot, ray_crossings, scale, segment_bounds,
    segment_crossing, sub, Grid, Point, TAU,
};
use super::presets::PresetDefinitions;
use super::water::Water;
use crate::{ApproachPlan, Half, MapPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::SurfaceKind;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// A road that ends on another runs no farther than this past its middle.
const OVERRUN_M: f64 = 1.0;

#[derive(Clone, Debug, Serialize)]
pub struct HalfSplit {
    pub top_m2: f64,
    pub bottom_m2: f64,
    /// Within the preset tolerance.
    pub fair: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoadMetrics {
    pub country_road_km: f64,
    pub dirt_track_km: f64,
    /// In-town streets, and how much of them no road joins to the centre.
    pub street_km: f64,
    pub unconnected_street_km: f64,
    /// Settlements with no road from their ground to the centre.
    pub unconnected_settlements: usize,
    /// Independent loops in the network of country roads and tracks: none
    /// for a tree of corridors. Streets are not counted.
    pub loops: usize,
    /// Places a road leaves the playable area.
    pub edge_exits: usize,
    /// The most country roads that meet at one junction in the centre: four
    /// or more is a central crossroads, fewer means the main roads fork or
    /// pass through.
    pub centre_roads: usize,
    /// Runs of road that enter water off any deck. Nothing drives them, so
    /// no journey and no settlement's road to the centre uses one.
    pub unbridged: usize,
}

/// The map's water and the decks over it.
#[derive(Clone, Debug, Serialize)]
pub struct RiverMetrics {
    pub rivers: usize,
    /// Length of river in each half, along its rounded middle.
    pub top_km: f64,
    pub bottom_km: f64,
    /// Within the preset tolerance.
    pub fair: bool,
    /// The narrowest and the widest water; zero without a river.
    pub width_m: [f64; 2],
    pub water_m2: f64,
    /// Bridges, by the half each stands in.
    pub bridges_top: usize,
    pub bridges_bottom: usize,
}

/// The fastest road journey between two places.
#[derive(Clone, Debug, Serialize)]
pub struct Journey {
    pub route_m: f64,
    pub drive_s: f64,
    /// Driving time plus the preset allowance for planning and turns.
    pub elapsed_s: f64,
}

/// The road journeys the two sides make (M22). They start at the top and
/// the bottom edge; an edge is left by the middle stretch of it.
#[derive(Clone, Debug, Serialize)]
pub struct TransitMetrics {
    /// From the top edge, and from the bottom, to the main junction by the
    /// map's centre. `None` when no road makes the journey.
    pub top: Option<Journey>,
    pub bottom: Option<Journey>,
    /// From the exact centre to that junction.
    pub centre_m: f64,
    /// From the bottom edge to the top, through any bridges on the way.
    pub top_bottom: Option<Journey>,
    /// From the left edge to the right: `Some` on a map with a road across
    /// the middle from side to side.
    pub east_west: Option<Journey>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutMetrics {
    pub settlements: BTreeMap<String, usize>,
    pub urban_share: f64,
    pub forest_share: f64,
    pub plain_share: f64,
    /// The main settlement's share of all built ground.
    pub main_settlement_share: f64,
    pub town: HalfSplit,
    pub forest: HalfSplit,
    pub woods: usize,
    pub river: RiverMetrics,
    pub roads: RoadMetrics,
    pub transit: TransitMetrics,
    pub approaches_top: usize,
    pub approaches_bottom: usize,
    /// The approach rule: the main settlement has one in each half.
    pub main_approach_top: bool,
    pub main_approach_bottom: bool,
    pub ground_points: u64,
}

pub fn measure(plan: &MapPlan, presets: &PresetDefinitions) -> LayoutMetrics {
    let extent = plan.size;
    let playable = extent[0] * extent[1];
    let middle = extent[1] / 2.0;
    let split = |rings: &mut dyn Iterator<Item = &[Point]>| {
        rings.fold((0.0, 0.0), |(top, bottom), ring| {
            let above = area_above(ring, middle);
            (top + above, bottom + area(ring) - above)
        })
    };
    let town = split(
        &mut plan
            .settlements
            .iter()
            .flat_map(|s| &s.districts)
            .map(|d| d.ring.as_slice()),
    );
    let forest = split(&mut forest_rings(plan));
    let mut settlements = BTreeMap::new();
    for settlement in &plan.settlements {
        *settlements.entry(settlement.class.clone()).or_insert(0) += 1;
    }
    let main = main_settlement(plan, presets);
    let main_area = main.map_or(0.0, |index| built(&plan.settlements[index]));
    let found = approaches(plan, presets);
    let count = |half| found.iter().filter(|a| a.half == half).count();
    let main_has = |half| {
        found
            .iter()
            .any(|a| Some(a.settlement) == main && a.half == half)
    };
    let (roads, transit) = roads(plan, presets);
    let river = river(plan, presets);
    LayoutMetrics {
        settlements,
        urban_share: (town.0 + town.1) / playable,
        forest_share: (forest.0 + forest.1) / playable,
        plain_share: 1.0 - (town.0 + town.1 + forest.0 + forest.1 + river.water_m2) / playable,
        main_settlement_share: main_area / (town.0 + town.1).max(f64::MIN_POSITIVE),
        town: HalfSplit {
            top_m2: town.0,
            bottom_m2: town.1,
            fair: presets.fairness.town.allows(town.0, town.1, playable),
        },
        forest: HalfSplit {
            top_m2: forest.0,
            bottom_m2: forest.1,
            fair: presets.fairness.forest.allows(forest.0, forest.1, playable),
        },
        woods: plan.forests.len(),
        river,
        roads,
        transit,
        approaches_top: count(Half::Top),
        approaches_bottom: count(Half::Bottom),
        main_approach_top: main_has(Half::Top),
        main_approach_bottom: main_has(Half::Bottom),
        ground_points: crate::ground_points(plan),
    }
}

fn river(plan: &MapPlan, presets: &PresetDefinitions) -> RiverMetrics {
    let middle = plan.size[1] / 2.0;
    // Metres of river north and south of the midline, and its water.
    let (mut top, mut bottom, mut water_m2) = (0.0, 0.0, 0.0);
    let mut width_m = [f64::INFINITY, 0.0_f64];
    for pair in plan
        .rivers
        .iter()
        .flat_map(|river| river.samples().windows(2))
    {
        let run = distance(pair[0].xy, pair[1].xy);
        if pair[0].xy[1] + pair[1].xy[1] >= 2.0 * middle {
            top += run;
        } else {
            bottom += run;
        }
        water_m2 += run * (pair[0].half_width_m + pair[1].half_width_m);
        for sample in pair {
            width_m = [
                width_m[0].min(2.0 * sample.half_width_m),
                width_m[1].max(2.0 * sample.half_width_m),
            ];
        }
    }
    let bridges_top = plan
        .bridges
        .iter()
        .filter(|bridge| bridge.center[1] >= middle)
        .count();
    RiverMetrics {
        rivers: plan.rivers.len(),
        top_km: top / 1000.0,
        bottom_km: bottom / 1000.0,
        fair: presets.fairness.river.allows(top, bottom, plan.size[1]),
        width_m: if plan.rivers.is_empty() {
            [0.0; 2]
        } else {
            width_m
        },
        water_m2,
        bridges_top,
        bridges_bottom: plan.bridges.len() - bridges_top,
    }
}

fn forest_rings(plan: &MapPlan) -> impl Iterator<Item = &[Point]> {
    plan.forests
        .iter()
        .filter_map(|forest| match &forest.shape {
            GroundShape::Polygon { ring } => Some(ring.as_slice()),
            GroundShape::Stroke { .. } => None,
        })
}

fn built(settlement: &crate::SettlementPlan) -> f64 {
    settlement.districts.iter().map(|d| area(&d.ring)).sum()
}

/// The settlement of the highest class, and among those the most built ground.
fn main_settlement(plan: &MapPlan, presets: &PresetDefinitions) -> Option<usize> {
    let rank = |index: usize| {
        presets
            .classes
            .get(&plan.settlements[index].class)
            .map(|class| class.rank)
    };
    (0..plan.settlements.len()).max_by(|a, b| {
        rank(*a)
            .cmp(&rank(*b))
            .then(built(&plan.settlements[*a]).total_cmp(&built(&plan.settlements[*b])))
            // The first of equals, whichever way the comparison is walked.
            .then(b.cmp(a))
    })
}

/// Something a line of open ground stops at.
struct Obstacle<'a> {
    center: Point,
    reach: f64,
    ring: &'a [Point],
}

impl<'a> Obstacle<'a> {
    fn new(ring: &'a [Point]) -> Self {
        let sum = ring.iter().fold([0.0, 0.0], |sum, p| add(sum, *p));
        let center = scale(sum, 1.0 / ring.len() as f64);
        let reach = ring
            .iter()
            .map(|p| distance(center, *p))
            .fold(0.0, f64::max);
        Self {
            center,
            reach,
            ring,
        }
    }
}

/// Where a corridor `front_m` wide that runs out from `center` along the
/// unit vector `toward` leaves the ground of `outline`, as a distance from
/// `center`: the farthest the outline reaches along the bearing inside the
/// corridor, by each of its edges cut to the corridor's width.
pub fn corridor_start(outline: &[Point], center: Point, toward: Point, front_m: f64) -> f64 {
    let aside = [-toward[1], toward[0]];
    let place = |p: Point| {
        let offset = sub(p, center);
        [dot(offset, toward), dot(offset, aside)]
    };
    let half_front = front_m / 2.0;
    let mut edge: f64 = 0.0;
    for (a, b) in contract::ground::edges(outline) {
        let (a, b) = (place(*a), place(*b));
        let (low, high) = if a[1] <= b[1] { (a, b) } else { (b, a) };
        if high[1] < -half_front || low[1] > half_front {
            continue;
        }
        for side in [low[1].max(-half_front), high[1].min(half_front)] {
            let share = if high[1] > low[1] {
                (side - low[1]) / (high[1] - low[1])
            } else {
                0.0
            };
            edge = edge.max(low[0] + (high[0] - low[0]) * share);
        }
    }
    edge
}

/// The farthest a settlement's outline lies from its centre.
fn reach(settlement: &crate::SettlementPlan) -> f64 {
    settlement
        .outline
        .iter()
        .map(|p| distance(settlement.center, *p))
        .fold(0.0, f64::max)
}

/// How many bearings round a settlement an approach is judged along: a
/// hundred metres apart at the corridor's far end.
fn bearings(settlement: &crate::SettlementPlan, depth_m: f64) -> usize {
    (libm::ceil(TAU * (reach(settlement) + depth_m) / 100.0) as usize).clamp(64, 720)
}

/// One bearing's corridor of a measured open approach: the ground the
/// approach rule found open, and the generator keeps open.
#[derive(Clone, Copy, Debug)]
pub struct Corridor {
    center: Point,
    toward: Point,
    /// Where it starts and ends along the bearing, from the centre.
    along: [f64; 2],
    half_front: f64,
}

impl Corridor {
    pub fn contains(&self, p: Point) -> bool {
        let offset = sub(p, self.center);
        let along = dot(offset, self.toward);
        let aside = dot(offset, [-self.toward[1], self.toward[0]]);
        along >= self.along[0] && along <= self.along[1] && aside.abs() <= self.half_front
    }
}

/// The corridors of every approach the plan records, one a bearing.
pub fn approach_corridors(plan: &MapPlan) -> Vec<Corridor> {
    let mut corridors = Vec::new();
    for approach in &plan.approaches {
        let Some(settlement) = plan.settlements.get(approach.settlement) else {
            continue;
        };
        let step = TAU / bearings(settlement, approach.depth_m) as f64;
        let (first, last) = (
            libm::round(approach.from_rad / step) as usize,
            libm::round(approach.to_rad / step) as usize,
        );
        for bearing in first..=last {
            let toward = direction(step * bearing as f64);
            let edge = corridor_start(
                &settlement.outline,
                settlement.center,
                toward,
                approach.front_m,
            );
            corridors.push(Corridor {
                center: settlement.center,
                toward,
                along: [edge, edge + approach.depth_m],
                half_front: approach.front_m / 2.0,
            });
        }
    }
    corridors
}

/// Every open approach to a settlement whose class measures them. An
/// approach is a corridor of open ground as wide as the preset front and as
/// deep as the preset depth, running out from the settlement's edge along one
/// bearing: its edge there is the farthest its outline reaches along that
/// bearing inside the corridor. Bearings are tried a hundred metres apart at
/// full depth, and a run of neighbouring bearings in one half whose
/// corridors are open is one approach. Open ground is ground a force can
/// advance over: a settlement, a wood and water each end it.
pub fn approaches(plan: &MapPlan, presets: &PresetDefinitions) -> Vec<ApproachPlan> {
    let rule = &presets.approach;
    let depth_m = rule.depth_m(plan.size[0].min(plan.size[1]));
    let water = super::water::rings(&plan.rivers);
    // What stands in the open country ends an approach as a wood does, but
    // is smaller than the gap between a corridor's lanes, so each is asked
    // as a disc: a yard, a copse, a single tree, and a tree line as a disc
    // every half width along it.
    let wood_floor = presets.wood_floor_m2();
    let disc = |ring: &[Point]| {
        let obstacle = Obstacle::new(ring);
        (obstacle.center, obstacle.reach)
    };
    let mut small: Vec<(Point, f64)> = crate::open_country::country_lots(plan)
        .map(|lot| disc(&lot.ring))
        .collect();
    for forest in &plan.forests {
        match &forest.shape {
            GroundShape::Polygon { ring } if area(ring) < wood_floor => small.push(disc(ring)),
            GroundShape::Polygon { .. } => {}
            GroundShape::Stroke {
                centerline,
                width_m,
            } => {
                let half = width_m / 2.0;
                for pair in centerline.samples().windows(2) {
                    let steps = libm::ceil(distance(pair[0], pair[1]) / half).max(1.0) as usize;
                    small.extend((0..=steps).map(|step| {
                        let share = step as f64 / steps as f64;
                        (add(pair[0], scale(sub(pair[1], pair[0]), share)), half)
                    }));
                }
            }
        }
    }
    let obstacles: Vec<Obstacle> = plan
        .settlements
        .iter()
        .map(|s| s.outline.as_slice())
        .chain(forest_rings(plan).filter(|ring| area(ring) >= wood_floor))
        .chain(water.iter().map(Vec::as_slice))
        .map(Obstacle::new)
        .collect();
    // The corridor's lanes: lines along the bearing, at most a hundred
    // metres apart, from one side of the front to the other.
    let lanes = (libm::ceil(rule.front_m / 100.0) as usize).max(1);
    let mut found = Vec::new();
    for (index, settlement) in plan.settlements.iter().enumerate() {
        if !presets
            .classes
            .get(&settlement.class)
            .is_some_and(|class| class.approach)
        {
            continue;
        }
        let center = settlement.center;
        let farthest = reach(settlement);
        let bearings = bearings(settlement, depth_m);
        let step = TAU / bearings as f64;
        // What stands near enough for a corridor to reach.
        let near: Vec<&Obstacle> = obstacles
            .iter()
            .enumerate()
            .filter(|(other, obstacle)| {
                *other != index
                    && distance(obstacle.center, center)
                        <= farthest + depth_m + rule.front_m + obstacle.reach
            })
            .map(|(_, obstacle)| obstacle)
            .collect();
        let near_small: Vec<(Point, f64)> = small
            .iter()
            .filter(|(at, reach)| {
                distance(*at, center) <= farthest + depth_m + rule.front_m + reach
            })
            .copied()
            .collect();
        // Per bearing: the half its corridor lies in, when it is open.
        let open: Vec<Option<Half>> = (0..bearings)
            .map(|bearing| {
                let toward = direction(step * bearing as f64);
                let aside = [-toward[1], toward[0]];
                let place = |p: Point| {
                    let offset = sub(p, center);
                    [
                        offset[0] * toward[0] + offset[1] * toward[1],
                        offset[0] * aside[0] + offset[1] * aside[1],
                    ]
                };
                let edge = corridor_start(&settlement.outline, center, toward, rule.front_m);
                let bare = near_small.iter().all(|(at, reach)| {
                    let [along, off] = place(*at);
                    along < edge - reach
                        || along > edge + depth_m + reach
                        || off.abs() > rule.front_m / 2.0 + reach
                });
                let clear = bare
                    && (0..=lanes).all(|lane| {
                        let across = rule.front_m * (lane as f64 / lanes as f64 - 0.5);
                        let from = add(center, add(scale(toward, edge), scale(aside, across)));
                        let to = add(from, scale(toward, depth_m));
                        let inside = [from, to].iter().all(|p| {
                            (0..2).all(|axis| p[axis] >= 0.0 && p[axis] <= plan.size[axis])
                        });
                        inside
                            && near.iter().all(|obstacle| {
                                let [along, off] = place(obstacle.center);
                                if (off - across).abs() > obstacle.reach
                                    || along < edge - obstacle.reach
                                    || along > edge + depth_m + obstacle.reach
                                {
                                    return true;
                                }
                                !polygon_contains(obstacle.ring, from)
                                    && ray_crossings(from, toward, obstacle.ring)
                                        .all(|hit| hit > depth_m)
                            })
                    });
                let half_way = center[1] + toward[1] * (edge + depth_m / 2.0);
                clear.then_some(if half_way >= plan.size[1] / 2.0 {
                    Half::Top
                } else {
                    Half::Bottom
                })
            })
            .collect();
        // Walk the circle from a bearing where the label changes, so a run
        // that wraps past the last bearing stays one run.
        let start = (0..bearings)
            .find(|bearing| open[*bearing] != open[(*bearing + bearings - 1) % bearings])
            .unwrap_or(0);
        let mut bearing = 0;
        while bearing < bearings {
            let first = start + bearing;
            let Some(half) = open[first % bearings] else {
                bearing += 1;
                continue;
            };
            let mut last = first;
            while bearing < bearings && open[(start + bearing) % bearings] == Some(half) {
                last = start + bearing;
                bearing += 1;
            }
            found.push(ApproachPlan {
                settlement: index,
                half,
                // Microradians: a few millimetres at this range, and short
                // enough for any JSON reader to read back exactly.
                from_rad: libm::round(step * first as f64 * 1e6) / 1e6,
                to_rad: libm::round(step * last as f64 * 1e6) / 1e6,
                depth_m,
                front_m: rule.front_m,
            });
        }
    }
    found
}

/// The turn between the bearings an approach to `settlement` is measured
/// along: a hundred metres apart at the corridors' far end.
pub fn bearing_step(settlement: &crate::SettlementPlan, depth_m: f64) -> f64 {
    let farthest = settlement
        .outline
        .iter()
        .map(|p| distance(settlement.center, *p))
        .fold(0.0, f64::max);
    TAU / (libm::ceil(TAU * (farthest + depth_m) / 100.0) as usize).clamp(64, 720) as f64
}

/// Time as a heap key. Journey times are finite, so the total order is the
/// numeric one.
#[derive(PartialEq)]
struct Seconds(f64);
impl Eq for Seconds {}
impl PartialOrd for Seconds {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Seconds {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The road graph: the points of each rounded centreline are nodes, and two
/// roads that cross share a node at the crossing, because their surfaces
/// overlap there. A run of road in the water is driven only where a deck
/// carries it.
fn roads(plan: &MapPlan, presets: &PresetDefinitions) -> (RoadMetrics, TransitMetrics) {
    let water = Water::new(&plan.rivers, plan.size);
    let on_deck = |p: Point| {
        plan.bridges.iter().any(|bridge| {
            let (sin, cos) = libm::sincos(bridge.yaw);
            let d = sub(p, bridge.center);
            (d[0] * cos + d[1] * sin).abs() <= bridge.half_extents[0]
                && (d[1] * cos - d[0] * sin).abs() <= bridge.half_extents[1]
        })
    };
    // Walked a metre at a time, where the run meets water at all.
    let sunk = |a: Point, b: Point| {
        let steps = libm::ceil(distance(a, b));
        water.segment_gap(a, b, 0.0) < 0.0
            && (0..=steps as usize).any(|step| {
                let p = add(a, scale(sub(b, a), step as f64 / steps.max(1.0)));
                water.gap(p, 0.0) < 0.0 && !on_deck(p)
            })
    };
    let mut unbridged = 0;
    let centrelines = plan
        .surfaces
        .iter()
        .filter(|area| area.kind.is_road())
        .filter_map(|area| match &area.shape {
            GroundShape::Stroke { centerline, .. } => Some((centerline, area.kind)),
            GroundShape::Polygon { .. } => None,
        });
    let segments: Vec<(Point, Point, SurfaceKind)> = centrelines
        .clone()
        .flat_map(|(line, kind)| {
            line.samples()
                .windows(2)
                .map(move |run| (run[0], run[1], kind))
        })
        .collect();
    let mut grid = Grid::new(plan.size, 128.0);
    for (index, (a, b, _)) in segments.iter().enumerate() {
        grid.insert(segment_bounds(*a, *b, 0.0), index as u32);
    }
    let mut splits: Vec<Vec<(f64, Point)>> = vec![Vec::new(); segments.len()];
    // The segment each other one was last tried against, so a pair that
    // shares several buckets is tried once.
    let mut tried = vec![usize::MAX; segments.len()];
    for i in 0..segments.len() {
        let (a, b, _) = segments[i];
        let mut near = Vec::new();
        grid.any(segment_bounds(a, b, 0.0), |item| {
            let j = item as usize;
            if j > i && tried[j] != i {
                tried[j] = i;
                near.push(j);
            }
            false
        });
        near.sort_unstable();
        for j in near {
            let (c, d, _) = segments[j];
            let Some((t, u)) = segment_crossing(a, b, c, d) else {
                continue;
            };
            // An end that lies on the other road is the shared node itself.
            let point = match (t, u) {
                (0.0, _) => a,
                (1.0, _) => b,
                (_, 0.0) => c,
                (_, 1.0) => d,
                _ => add(a, scale(sub(b, a), t)),
            };
            if t > 0.0 && t < 1.0 {
                splits[i].push((t, point));
            }
            if u > 0.0 && u < 1.0 {
                splits[j].push((u, point));
            }
        }
    }
    let mut ids: BTreeMap<[u64; 2], usize> = BTreeMap::new();
    let mut nodes: Vec<Point> = Vec::new();
    let mut links: Vec<Vec<(usize, f64, f64)>> = Vec::new();
    // Kilometres of country road, dirt track and street.
    let mut km = [0.0; 3];
    // Each link's kind beside it, to total what the hub cannot reach.
    let mut streets: Vec<(usize, f64)> = Vec::new();
    // Links of road and track, and of country road alone, by their ends.
    let mut country: Vec<[usize; 2]> = Vec::new();
    // Each end of a link of country road: its node, the node at the link's
    // other end and the link's length.
    let mut main_ends: Vec<(usize, usize, f64)> = Vec::new();
    let mps = presets.transit.road_mps();
    for ((a, b, kind), mut cuts) in segments.into_iter().zip(splits) {
        cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
        let chain: Vec<Point> = [a]
            .into_iter()
            .chain(cuts.into_iter().map(|cut| cut.1))
            .chain([b])
            .collect();
        let track = kind == SurfaceKind::DirtTrack;
        let speed = if track {
            mps * presets.roads.dirt_track_speed_factor
        } else {
            mps
        };
        for piece in chain.windows(2) {
            let ends = [piece[0], piece[1]].map(|p| {
                *ids.entry(p.map(f64::to_bits)).or_insert_with(|| {
                    nodes.push(p);
                    links.push(Vec::new());
                    nodes.len() - 1
                })
            });
            if ends[0] == ends[1] {
                continue;
            }
            let metres = distance(piece[0], piece[1]);
            let row = match kind {
                SurfaceKind::DirtTrack => 1,
                SurfaceKind::Road => 2,
                _ => 0,
            };
            km[row] += metres / 1000.0;
            if sunk(piece[0], piece[1]) {
                unbridged += 1;
                continue;
            }
            if kind == SurfaceKind::Road {
                streets.push((ends[0], metres));
            } else {
                country.push(ends);
            }
            if kind == SurfaceKind::CountryRoad {
                main_ends.extend([(ends[0], ends[1], metres), (ends[1], ends[0], metres)]);
            }
            links[ends[0]].push((ends[1], metres, metres / speed));
            links[ends[1]].push((ends[0], metres, metres / speed));
        }
    }
    let mut metrics = RoadMetrics {
        country_road_km: km[0],
        dirt_track_km: km[1],
        street_km: km[2],
        unconnected_street_km: 0.0,
        unconnected_settlements: plan.settlements.len(),
        loops: 0,
        edge_exits: 0,
        centre_roads: 0,
        unbridged,
    };
    let mut transit = TransitMetrics {
        top: None,
        bottom: None,
        centre_m: f64::INFINITY,
        top_bottom: None,
        east_west: None,
    };
    // The hub is the main junction: the authored point of a country road
    // nearest the main settlement's centre, which stands on it. (Nearest the
    // map's middle on a plan without settlements.)
    let centre = [plan.size[0] / 2.0, plan.size[1] / 2.0];
    let main =
        main_settlement(plan, presets).map_or(centre, |index| plan.settlements[index].center);
    let authored = centrelines
        .filter(|(_, kind)| *kind == SurfaceKind::CountryRoad)
        .flat_map(|(line, _)| line.control_points())
        .filter_map(|p| ids.get(&p.map(f64::to_bits)).copied());
    let Some(hub) =
        authored.min_by(|a, b| distance(nodes[*a], main).total_cmp(&distance(nodes[*b], main)))
    else {
        return (metrics, transit);
    };
    // Fastest journey to every node from the nearest of `from`:
    // (seconds, metres).
    let fastest = |from: &[usize]| {
        let mut best: Vec<Option<(f64, f64)>> = vec![None; nodes.len()];
        let mut queue = BinaryHeap::new();
        for node in from {
            best[*node] = Some((0.0, 0.0));
            queue.push(Reverse((Seconds(0.0), *node)));
        }
        while let Some(Reverse((Seconds(seconds), node))) = queue.pop() {
            let Some((known, travelled)) = best[node] else {
                continue;
            };
            if known < seconds {
                continue;
            }
            for (next, metres, time) in &links[node] {
                let arrival = seconds + time;
                if best[*next].is_none_or(|(known, _)| arrival < known) {
                    best[*next] = Some((arrival, travelled + metres));
                    queue.push(Reverse((Seconds(arrival), *next)));
                }
            }
        }
        best
    };
    let best = fastest(&[hub]);
    // Loops of the hub's network of roads and tracks: links beyond a
    // spanning tree's.
    let reached: Vec<[usize; 2]> = country
        .into_iter()
        .filter(|ends| best[ends[0]].is_some())
        .collect();
    let mut joined: Vec<usize> = reached.iter().flatten().copied().collect();
    joined.sort_unstable();
    joined.dedup();
    metrics.loops = (reached.len() + 1).saturating_sub(joined.len());
    metrics.unconnected_street_km = streets
        .iter()
        .filter(|(node, _)| best[*node].is_none())
        .map(|(_, metres)| metres / 1000.0)
        .sum();
    // A road that ends on another runs a hair past its middle, so that the
    // two cross: that hair is no road out of the junction.
    let mut main_ends: Vec<usize> = main_ends
        .into_iter()
        .filter(|(_, far, metres)| *metres >= OVERRUN_M || links[*far].len() > 1)
        .map(|(node, ..)| node)
        .collect();
    main_ends.sort_unstable();
    metrics.centre_roads = main_ends
        .chunk_by(|a, b| a == b)
        .filter(|ends| distance(nodes[ends[0]], centre) <= presets.transit.centre_reach_m)
        .map(|ends| ends.len())
        .max()
        .unwrap_or(0);
    metrics.edge_exits = nodes
        .iter()
        .filter(|p| (0..2).any(|axis| p[axis] == 0.0 || p[axis] == plan.size[axis]))
        .count();
    // A road serves a settlement when it runs through its outline or along
    // its edge: a town's edge is often a road.
    metrics.unconnected_settlements = plan
        .settlements
        .iter()
        .filter(|settlement| {
            let ring = &settlement.outline;
            let [x0, y0, x1, y1] = contract::ground::limits(ring, 1.0);
            let near = |a: Point, b: Point| {
                a[0].min(b[0]) <= x1
                    && a[0].max(b[0]) >= x0
                    && a[1].min(b[1]) <= y1
                    && a[1].max(b[1]) >= y0
            };
            !(0..nodes.len()).any(|node| {
                best[node].is_some()
                    && links[node].iter().any(|(next, ..)| {
                        let (a, b) = (nodes[node], nodes[*next]);
                        near(a, b)
                            && (polygon_contains(ring, a)
                                || contract::ground::edges(ring).any(|(c, d)| {
                                    segment_crossing(a, b, *c, *d).is_some()
                                        || super::geometry::segment_distance(a, b, *c) <= 1.0
                                }))
                    })
            })
        })
        .count();
    // Where roads leave each edge by its middle stretch: north, east,
    // south, west.
    let window = presets.transit.exit_window;
    let [north, east, south, west] =
        [(1, 1.0), (0, 1.0), (1, 0.0), (0, 0.0)].map(|(axis, side)| {
            let across = 1 - axis;
            (0..nodes.len())
                .filter(|node| {
                    let p = nodes[*node];
                    p[axis] == side * plan.size[axis]
                        && (p[across] - plan.size[across] / 2.0).abs()
                            <= window * plan.size[across] / 2.0
                })
                .collect::<Vec<usize>>()
        });
    let journey = |best: &[Option<(f64, f64)>], to: &[usize]| {
        to.iter()
            .filter_map(|node| best[*node])
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(drive_s, route_m)| Journey {
                route_m,
                drive_s,
                elapsed_s: drive_s + presets.transit.allowance_s,
            })
    };
    transit.top = journey(&best, &north);
    transit.bottom = journey(&best, &south);
    transit.centre_m = distance(nodes[hub], centre);
    transit.top_bottom = journey(&fastest(&south), &north);
    transit.east_west = journey(&fastest(&west), &east);
    (metrics, transit)
}
