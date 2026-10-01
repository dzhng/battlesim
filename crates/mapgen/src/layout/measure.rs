//! What a plan is, measured from its geometry alone: areas by half, open
//! approaches, and the road graph's reach and journey times. The generator
//! holds its own output to the presets with these numbers, and the sweep
//! and the inspection picture report them.
use super::geometry::{
    add, area, area_above, direction, distance, ray_crossings, scale, segment_crossing, sub, Point,
    TAU,
};
use super::presets::PresetDefinitions;
use crate::{ApproachPlan, Half, MapPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::SurfaceKind;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

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
    /// Settlements with no road from inside their outline to the centre.
    pub unconnected_settlements: usize,
    /// Independent loops in the network: none for a tree of corridors.
    pub loops: usize,
    /// Places a road leaves the playable area.
    pub edge_exits: usize,
    /// Roads that meet at the junction nearest the centre: four or more is a
    /// central crossroads, fewer means an edge road joined another on its way.
    pub hub_roads: usize,
}

/// The fastest road journey from the middle stretch of one edge to the centre.
#[derive(Clone, Debug, Serialize)]
pub struct EdgeTransit {
    pub edge: &'static str,
    pub route_m: f64,
    pub drive_s: f64,
    /// Driving time plus the preset allowance for planning and turns.
    pub elapsed_s: f64,
    /// From the exact centre to the nearest road.
    pub connector_m: f64,
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
    pub roads: RoadMetrics,
    pub transit: Vec<EdgeTransit>,
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
    LayoutMetrics {
        settlements,
        urban_share: (town.0 + town.1) / playable,
        forest_share: (forest.0 + forest.1) / playable,
        plain_share: 1.0 - (town.0 + town.1 + forest.0 + forest.1) / playable,
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
        roads,
        transit,
        approaches_top: count(Half::Top),
        approaches_bottom: count(Half::Bottom),
        main_approach_top: main_has(Half::Top),
        main_approach_bottom: main_has(Half::Bottom),
        ground_points: crate::ground_points(plan),
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

/// Every open approach to a settlement whose class measures them: rays leave
/// the settlement's centre a hundred metres apart at full depth; a run of
/// neighbouring rays in one half, each open for the preset depth past the
/// settlement's edge, is an approach when its front is wide enough.
pub fn approaches(plan: &MapPlan, presets: &PresetDefinitions) -> Vec<ApproachPlan> {
    let rule = presets.approach;
    let obstacles: Vec<Obstacle> = plan
        .settlements
        .iter()
        .map(|s| s.outline.as_slice())
        .chain(forest_rings(plan))
        .map(Obstacle::new)
        .collect();
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
        let reach = obstacles[index].reach;
        let rays = (libm::ceil(TAU * (reach + rule.depth_m) / 100.0) as usize).clamp(64, 720);
        let step = TAU / rays as f64;
        // Per ray: the settlement's own edge, and the half its open run lies
        // in when that run is deep enough.
        let cast: Vec<(f64, Option<Half>)> = (0..rays)
            .map(|ray| {
                let toward = direction(step * ray as f64);
                let edge = ray_crossings(center, toward, &settlement.outline).fold(0.0, f64::max);
                let mut open_to = (0..2)
                    .map(|axis| {
                        if toward[axis] > 0.0 {
                            (plan.size[axis] - center[axis]) / toward[axis]
                        } else if toward[axis] < 0.0 {
                            -center[axis] / toward[axis]
                        } else {
                            f64::INFINITY
                        }
                    })
                    .fold(f64::INFINITY, f64::min);
                for (other, obstacle) in obstacles.iter().enumerate() {
                    let offset = sub(obstacle.center, center);
                    let along = offset[0] * toward[0] + offset[1] * toward[1];
                    let aside = (toward[0] * offset[1] - toward[1] * offset[0]).abs();
                    if other == index
                        || aside > obstacle.reach
                        || along < edge - obstacle.reach
                        || along > edge + rule.depth_m + obstacle.reach
                    {
                        continue;
                    }
                    open_to = ray_crossings(center, toward, obstacle.ring)
                        .filter(|hit| *hit > edge)
                        .fold(open_to, f64::min);
                }
                let half_way = center[1] + toward[1] * (edge + rule.depth_m / 2.0);
                let half = if half_way >= plan.size[1] / 2.0 {
                    Half::Top
                } else {
                    Half::Bottom
                };
                (edge, (open_to - edge >= rule.depth_m).then_some(half))
            })
            .collect();
        // Walk the circle from a ray where the label changes, so a run that
        // wraps past the last ray stays one run.
        let start = (0..rays)
            .find(|ray| cast[*ray].1 != cast[(*ray + rays - 1) % rays].1)
            .unwrap_or(0);
        let mut ray = 0;
        while ray < rays {
            let first = start + ray;
            let Some(half) = cast[first % rays].1 else {
                ray += 1;
                continue;
            };
            let mut front = 0.0;
            let mut last = first;
            while ray < rays && cast[(start + ray) % rays].1 == Some(half) {
                last = start + ray;
                front += step * (cast[last % rays].0 + rule.depth_m / 2.0);
                ray += 1;
            }
            if front >= rule.front_m {
                found.push(ApproachPlan {
                    settlement: index,
                    half,
                    // Microradians: a few millimetres at this range, and
                    // short enough for any JSON reader to read back exactly.
                    from_rad: libm::round(step * first as f64 * 1e6) / 1e6,
                    to_rad: libm::round(step * last as f64 * 1e6) / 1e6,
                    depth_m: rule.depth_m,
                    front_m: libm::round(front),
                });
            }
        }
    }
    found
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

/// The road graph: authored points are nodes, and two roads that cross
/// share a node at the crossing, because their surfaces overlap there.
fn roads(plan: &MapPlan, presets: &PresetDefinitions) -> (RoadMetrics, Vec<EdgeTransit>) {
    let segments: Vec<(Point, Point, SurfaceKind)> = plan
        .surfaces
        .iter()
        .filter(|area| area.kind.is_road())
        .flat_map(|area| {
            let points = match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.control_points(),
                GroundShape::Polygon { .. } => &[],
            };
            points.windows(2).map(|run| (run[0], run[1], area.kind))
        })
        .collect();
    let mut splits: Vec<Vec<(f64, Point)>> = vec![Vec::new(); segments.len()];
    for i in 0..segments.len() {
        for j in i + 1..segments.len() {
            let (a, b, _) = segments[i];
            let (c, d, _) = segments[j];
            let apart = (0..2).any(|axis| {
                a[axis].max(b[axis]) < c[axis].min(d[axis])
                    || c[axis].max(d[axis]) < a[axis].min(b[axis])
            });
            if apart {
                continue;
            }
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
    let mut km = [0.0, 0.0];
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
            km[usize::from(track)] += metres / 1000.0;
            links[ends[0]].push((ends[1], metres, metres / speed));
            links[ends[1]].push((ends[0], metres, metres / speed));
        }
    }
    let mut metrics = RoadMetrics {
        country_road_km: km[0],
        dirt_track_km: km[1],
        unconnected_settlements: plan.settlements.len(),
        loops: 0,
        edge_exits: 0,
        hub_roads: 0,
    };
    let centre = [plan.size[0] / 2.0, plan.size[1] / 2.0];
    let Some(hub) = (0..nodes.len())
        .min_by(|a, b| distance(nodes[*a], centre).total_cmp(&distance(nodes[*b], centre)))
    else {
        return (metrics, Vec::new());
    };
    // Fastest journey from the hub to every node: (seconds, metres).
    let mut best: Vec<Option<(f64, f64)>> = vec![None; nodes.len()];
    let mut queue = BinaryHeap::from([Reverse((Seconds(0.0), hub))]);
    best[hub] = Some((0.0, 0.0));
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
    let reached = best.iter().filter(|b| b.is_some()).count();
    // Loops of the hub's network: edges beyond a spanning tree's.
    let reached_edges: usize = (0..nodes.len())
        .filter(|node| best[*node].is_some())
        .map(|node| links[node].len())
        .sum::<usize>()
        / 2;
    metrics.loops = reached_edges + 1 - reached;
    metrics.hub_roads = links[hub].len();
    metrics.edge_exits = nodes
        .iter()
        .filter(|p| (0..2).any(|axis| p[axis] == 0.0 || p[axis] == plan.size[axis]))
        .count();
    metrics.unconnected_settlements = plan
        .settlements
        .iter()
        .filter(|settlement| {
            !(0..nodes.len()).any(|node| {
                best[node].is_some() && polygon_contains(&settlement.outline, nodes[node])
            })
        })
        .count();
    let window = presets.transit.exit_window;
    let transit = [
        ("north", 1, 1.0),
        ("east", 0, 1.0),
        ("south", 1, 0.0),
        ("west", 0, 0.0),
    ]
    .into_iter()
    .filter_map(|(edge, axis, side)| {
        let across = 1 - axis;
        (0..nodes.len())
            .filter(|node| {
                let p = nodes[*node];
                p[axis] == side * plan.size[axis]
                    && (p[across] - plan.size[across] / 2.0).abs()
                        <= window * plan.size[across] / 2.0
            })
            .filter_map(|node| best[node])
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(drive_s, route_m)| EdgeTransit {
                edge,
                route_m,
                drive_s,
                elapsed_s: drive_s + presets.transit.allowance_s,
                connector_m: distance(nodes[hub], centre),
            })
    })
    .collect();
    (metrics, transit)
}
