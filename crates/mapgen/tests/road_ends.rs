//! Where the generator's roads end, judged on the finished plan with the
//! contract's own membership: a stroke is cut square across its first and
//! last point, so an end that nothing hides is an end the player sees, and
//! a joint whose two roads do not cover each other's ends is a bite out of
//! the road. The arithmetic here is this file's own, not the generator's.
use contract::ground::{polygon_contains, GroundShape};
use contract::map::SurfaceArea;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::parcels::fill_districts;
use mapgen::{CompileLimits, MapPlan};
use std::collections::{BTreeMap, HashMap};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 3] = [MapSize::Small, MapSize::Medium, MapSize::Large];
/// Every cell runs these seeds: a claim about road ends is a claim about
/// the generator, not one map.
const SEEDS: [u64; 4] = [1, 2, 3, u64::MAX];

type Point = [f64; 2];

/// A face lying along another carriageway's edge, or along the map's, is
/// covered by it: plan coordinates are whole centimetres, so the two may
/// differ by one.
const FLUSH_M: f64 = 0.02;
/// Paving this near ahead of an end that does not reach it is a joint that
/// failed to close, not a road that stops.
const NEAR_M: f64 = 1.5;
/// A road that stops on a settlement's block, or this near one, serves it.
const SERVED_M: f64 = 30.0;
const CELL_M: f64 = 64.0;
/// How many ends of each failing kind a report lists.
const LISTED: usize = 12;

fn plan(map_type: MapType, size: MapSize, seed: u64) -> MapPlan {
    let presets = PresetDefinitions::from_json(PRESETS).unwrap();
    let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap()).unwrap();
    let request = GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type,
        size,
        limits: CompileLimits {
            max_authored_parts: 60_000,
            max_bay_positions: 600_000,
            max_ground_points: 200_000,
        },
    };
    let layout = generate_layout(&request, &presets)
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"));
    fill_districts(layout, &request, &catalogue, &presets)
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"))
}

/// The plan's paving, bucketed by each surface's limits.
struct Paving<'a> {
    areas: &'a [SurfaceArea],
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl<'a> Paving<'a> {
    fn new(areas: &'a [SurfaceArea]) -> Self {
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (index, area) in areas.iter().enumerate() {
            let [x0, y0, x1, y1] = area.shape.limits().map(|v| (v / CELL_M).floor() as i64);
            for j in y0 - 1..=y1 + 1 {
                for i in x0 - 1..=x1 + 1 {
                    cells.entry((i, j)).or_default().push(index);
                }
            }
        }
        Self { areas, cells }
    }

    /// The carriageways other than `own` that hold `p`.
    fn others(&self, own: usize, p: Point) -> impl Iterator<Item = &SurfaceArea> {
        let key = (
            (p[0] / CELL_M).floor() as i64,
            (p[1] / CELL_M).floor() as i64,
        );
        self.cells
            .get(&key)
            .into_iter()
            .flatten()
            .filter(move |index| **index != own)
            .map(|index| &self.areas[*index])
            .filter(move |area| area.kind.is_road() && area.shape.contains(p, FLUSH_M))
    }

    fn other(&self, own: usize, p: Point) -> bool {
        self.others(own, p).next().is_some()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum End {
    /// Its whole face is off the map or under another carriageway.
    Hidden,
    /// It stops by nothing, on or beside a settlement's block.
    Serves,
    /// It stops across a narrower carriageway that carries on from it, in
    /// the open: the road narrows there, and its square shoulders show
    /// either side. A road changes width only under a road that crosses it.
    Shoulders,
    /// Its flat end stands out past the edge of a road it meets: nothing as
    /// wide as it covers any of its face, and either a carriageway joins it
    /// within two of its widths, or narrower ones cover part of the face and
    /// none carries on from it. The cut heel of a fork or a corner.
    Heel,
    /// A carriageway as wide as it covers part of its face and leaves the
    /// rest: a bite out of the joint, or a step in the road's edge.
    Notch,
    /// Paving lies just ahead of it that it does not reach.
    Gap,
    /// Part of its face is on the map's edge and part shows inside it.
    ShowsAtEdge,
    /// It stops in open ground, by nothing.
    Stranded,
}

impl End {
    fn sound(self) -> bool {
        matches!(self, End::Hidden | End::Serves)
    }
}

/// Of every ten thousand road ends, at most this many may be a bite, a step,
/// a heel or a pair of shoulders in a joint: the joints the joint pass does
/// not close yet.
/// Lower it as those are closed, never raise it.
const FLAWS_PER_TEN_THOUSAND: usize = 10;

/// Every carriageway stroke end of `plan`: what it is, where, and whose.
fn ends(plan: &MapPlan) -> Vec<(End, Point, usize)> {
    let paving = Paving::new(&plan.surfaces);
    let on_map = |p: Point| (0..2).all(|k| p[k] > FLUSH_M && p[k] < plan.size[k] - FLUSH_M);
    let mut out = Vec::new();
    for (index, area) in plan.surfaces.iter().enumerate() {
        let GroundShape::Stroke {
            centerline,
            width_m,
        } = &area.shape
        else {
            continue;
        };
        if !area.kind.is_road() {
            continue;
        }
        let samples = centerline.samples();
        let last = samples.len() - 1;
        for (at, before) in [(samples[0], samples[1]), (samples[last], samples[last - 1])] {
            let run = [at[0] - before[0], at[1] - before[1]];
            let length = run[0].hypot(run[1]);
            let out_of = [run[0] / length, run[1] / length];
            let across = [-out_of[1], out_of[0]];
            let half = width_m / 2.0;
            // A point `ahead` of the face and `aside` of the centreline.
            let point = |ahead: f64, aside: f64| {
                [0, 1].map(|k| at[k] + out_of[k] * ahead + across[k] * aside)
            };
            // Across the face, corner to corner.
            let face: Vec<Point> = [-1.0, -0.5, 0.0, 0.5, 1.0]
                .into_iter()
                .map(|share| point(0.0, half * share))
                .collect();
            let off_map = face.iter().filter(|p| !on_map(**p)).count();
            let hidden = face
                .iter()
                .filter(|p| !on_map(**p) || paving.other(index, **p))
                .count();
            // Nothing as wide as it covers any of its face: narrower roads
            // cannot hide it, however they leave.
            let narrows = {
                let wide = |p: Point| {
                    // (A yard that laps over a corner of it is no road.)
                    paving.others(index, p).any(|area| match &area.shape {
                        GroundShape::Stroke { width_m: other, .. } => other >= width_m,
                        GroundShape::Polygon { .. } => false,
                    })
                };
                hidden > 0 && !face.iter().any(|p| wide(*p))
            };
            // Paving a step or more past the face, anywhere across it.
            let ahead = (1..=6).any(|step| {
                face.iter().any(|p| {
                    let p = [0, 1].map(|k| p[k] + out_of[k] * NEAR_M * f64::from(step) / 6.0);
                    paving.other(index, p)
                })
            });
            // A carriageway carries on from the middle of the face, half a
            // width past it: farther than one that only crosses it reaches,
            // unless it crosses at less than 30°.
            let carries_on = paving.other(index, point(half, 0.0));
            // A carriageway on its own last two widths. (The yard in front
            // of a building is paving too, and no road.)
            let joined = (1..=8).any(|step| {
                [-1.0, 0.0, 1.0].into_iter().any(|share| {
                    let p = point(-width_m * f64::from(step) / 4.0, half * share);
                    paving
                        .others(index, p)
                        .any(|area| matches!(area.shape, GroundShape::Stroke { .. }))
                })
            });
            let served = plan
                .settlements
                .iter()
                .flat_map(|settlement| &settlement.districts)
                .any(|district| {
                    polygon_contains(&district.ring, at)
                        || contract::ground::edges(&district.ring).any(|(a, b)| {
                            contract::ground::segment_distance(*a, *b, at) <= SERVED_M
                        })
                });
            let kind = if hidden == face.len() {
                End::Hidden
            } else if off_map > 0 {
                End::ShowsAtEdge
            } else if narrows && carries_on {
                End::Shoulders
            } else if narrows {
                End::Heel
            } else if hidden > 0 {
                End::Notch
            } else if ahead {
                End::Gap
            } else if joined {
                End::Heel
            } else if served {
                End::Serves
            } else {
                End::Stranded
            };
            out.push((kind, at, index));
        }
    }
    out
}

/// No road end shows where it should not, over every type and size and
/// several seeds: each is hidden (off the map's edge, or under the road it
/// joins), or stops at a settlement it serves. None is a gap before a road,
/// a face showing at the map's edge, or a road stopping in open ground; and
/// no joint is bitten, shows a heel or shows the shoulders of a wider road
/// that narrows in the open, but for the few `FLAWS_PER_TEN_THOUSAND` allows.
#[test]
fn every_road_end_is_hidden_or_serves_something() {
    let mut wrong: BTreeMap<End, Vec<String>> = BTreeMap::new();
    let mut counts: BTreeMap<End, usize> = BTreeMap::new();
    for map_type in TYPES {
        for size in SIZES {
            for seed in SEEDS {
                let plan = plan(map_type, size, seed);
                for (kind, at, index) in ends(&plan) {
                    *counts.entry(kind).or_default() += 1;
                    if kind.sound() {
                        continue;
                    }
                    let GroundShape::Stroke {
                        centerline,
                        width_m,
                    } = &plan.surfaces[index].shape
                    else {
                        unreachable!()
                    };
                    let points = centerline.control_points();
                    // The carriageways whose authored lines pass within 15 m.
                    let near: Vec<String> = plan
                        .surfaces
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != index)
                        .filter_map(|(_, other)| match &other.shape {
                            GroundShape::Stroke {
                                centerline,
                                width_m,
                            } if other.kind.is_road() => {
                                let line = centerline.control_points();
                                let close = line.windows(2).any(|run| {
                                    contract::ground::segment_distance(run[0], run[1], at) <= 15.0
                                });
                                close.then(|| format!("{:?} {width_m} m {line:?}", other.kind))
                            }
                            _ => None,
                        })
                        .collect();
                    wrong.entry(kind).or_default().push(format!(
                        "{map_type:?} {size:?} seed {seed}: {:?} {width_m} m wide ends at ({:.2}, {:.2}); it runs {:?} .. {:?}\n    beside {}",
                        plan.surfaces[index].kind,
                        at[0],
                        at[1],
                        &points[..2],
                        &points[points.len() - 2..],
                        near.join("\n    beside "),
                    ));
                }
            }
        }
    }
    println!("road ends: {counts:?}");
    let report: Vec<String> = wrong
        .iter()
        .map(|(kind, ends)| {
            format!(
                "{} {kind:?}:\n  {}",
                ends.len(),
                ends.iter()
                    .take(LISTED)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        })
        .collect();
    let report = report.join("\n");
    let total: usize = counts.values().sum();
    let flaws: usize = [End::Notch, End::Heel, End::Shoulders]
        .into_iter()
        .map(|kind| wrong.remove(&kind).map_or(0, |ends| ends.len()))
        .sum();
    println!("{flaws} of {total} road ends are a bite, a step, a heel or shoulders\n{report}");
    assert!(wrong.is_empty(), "road ends show:\n{report}");
    assert!(
        flaws * 10_000 <= total * FLAWS_PER_TEN_THOUSAND,
        "{flaws} of {total} road ends are a bite, a step, a heel or shoulders in a joint:\n{report}"
    );
    // The sweep held what it claims to judge.
    assert!(counts[&End::Hidden] > 1000 && counts[&End::Serves] > 100);
}
