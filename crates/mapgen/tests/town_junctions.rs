//! Junctions as a town has them, judged on the finished plan: no more than
//! four carriageways leave one place, no two of them leave it nearly side
//! by side, and a town's street meets the road through the town square and
//! straight. Each flaw is counted over every type and size and held at or
//! near zero. The arithmetic here is this file's own, not the generator's.
#[path = "common/limits.rs"]
mod limits;
use contract::ground::GroundShape;
use contract::map::SurfaceKind;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::parcels::fill_districts;
use mapgen::MapPlan;
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 3] = [MapSize::Medium, MapSize::Large, MapSize::Xl];
/// Every cell runs these seeds: a claim about junctions is a claim about
/// the generator, not one map.
const SEEDS: [u64; 4] = [1, 2, 3, u64::MAX];

type Point = [f64; 2];

/// A carriageway's line is walked in steps of this.
const STEP_M: f64 = 1.5;
/// Joints nearer each other than this are one place to the eye: their
/// pavings run together.
const PLACE_M: f64 = 12.0;
/// A carriageway is an arm of a place where it is this far from it: past
/// the paving of the junction itself and past the few metres over which a
/// branch is turned to meet its road.
const ARM_M: f64 = 30.0;
/// Two arms nearer each other than this are one carriageway drawn twice,
/// which `street_warts.rs` counts.
const SAME_ARM_DEG: f64 = 10.0;
/// Two arms of one place nearer each other than this leave a sliver of
/// ground between two carriageways.
const FORK_DEG: f64 = 45.0;
/// A street that comes to a town's road farther off square than this meets
/// it at a slant.
const SLANT_DEG: f64 = 25.0;
/// A street that turns more than this over the `HOOK_M` before the point
/// where it is an arm of a town's road ends in a hook. A corner farther
/// back is a street round a bend.
const HOOK_DEG: f64 = 35.0;
const HOOK_M: f64 = 25.0;
/// What the generator still leaves over the sweep's 36 maps. Each is a
/// count to bring down: lower one when its flaw is closed, never raise one.
const MANY_ARMS: usize = 0;
const FORKS: usize = 2;
const SLANTS: usize = 4;
const HOOKS: usize = 0;
/// How many of each flaw a report lists.
const LISTED: usize = 10;

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
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: limits::game_limits(),
    };
    let layout = generate_layout(&request, &presets)
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"));
    fill_districts(layout, &request, &catalogue, &presets)
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"))
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn length(a: Point) -> f64 {
    a[0].hypot(a[1])
}
fn bearing(a: Point, b: Point) -> f64 {
    (b[1] - a[1]).atan2(b[0] - a[0])
}
/// The angle between two bearings, 0 to π.
fn between(a: f64, b: f64) -> f64 {
    let turn = (a - b).rem_euclid(core::f64::consts::TAU);
    turn.min(core::f64::consts::TAU - turn)
}
fn cell(p: Point, size: f64) -> (i64, i64) {
    ((p[0] / size).floor() as i64, (p[1] / size).floor() as i64)
}

/// One carriageway: its rounded line walked in even steps.
struct Way {
    kind: SurfaceKind,
    width: f64,
    points: Vec<Point>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum Flaw {
    /// Five carriageways or more leave one place, one of them a town's
    /// street.
    ManyArms,
    /// The same among country roads and tracks: the road network's,
    /// counted here and not held.
    RoadManyArms,
    /// Two carriageways leave one place less than 45° apart, one of them a
    /// town's street.
    Fork,
    /// The same between country roads and tracks: the road network's,
    /// counted here and not held.
    RoadFork,
    /// A town's street meets a country road more than 25° off square.
    Slant,
    /// A town's street turns sharply just before the country road it meets.
    Hook,
}

/// One carriageway leaving a place: its bearing from the place, the way and
/// where along it the arm was taken.
struct Arm {
    bearing: f64,
    way: usize,
    at: usize,
    /// +1 where the way runs on from `at` away from the place, −1 where it
    /// runs back.
    outward: i64,
}

/// Every flaw of `plan`'s junctions, with where it is, and how many places
/// were looked at.
fn flaws(plan: &MapPlan) -> (Vec<(Flaw, Point, String)>, usize) {
    let mut ways: Vec<Way> = Vec::new();
    for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
        let GroundShape::Stroke {
            centerline,
            width_m,
        } = &area.shape
        else {
            continue;
        };
        let mut points = Vec::new();
        for pair in centerline.samples().windows(2) {
            let span = length(sub(pair[1], pair[0]));
            let pieces = (span / STEP_M).ceil().max(1.0) as usize;
            for piece in 0..pieces {
                let share = piece as f64 / pieces as f64;
                points.push([0, 1].map(|k| pair[0][k] + (pair[1][k] - pair[0][k]) * share));
            }
        }
        points.extend(centerline.samples().last());
        ways.push(Way {
            kind: area.kind,
            width: *width_m,
            points,
        });
    }
    // Every walked point by the ground it stands on.
    let mut cells: HashMap<(i64, i64), Vec<(usize, usize)>> = HashMap::new();
    for (way, line) in ways.iter().enumerate() {
        for (at, p) in line.points.iter().enumerate() {
            cells.entry(cell(*p, ARM_M)).or_default().push((way, at));
        }
    }
    let near = |p: Point, reach: i64| {
        let (i, j) = cell(p, ARM_M);
        (j - reach..=j + reach)
            .flat_map(move |j| (i - reach..=i + reach).map(move |i| (i, j)))
            .filter_map(|at| cells.get(&at))
            .flatten()
            .copied()
    };
    // Joints: where one way's end lies on another's paving, and where two
    // ways' lines cross (a walked point of each within a step of the other,
    // neither near its own end).
    let mut joints: Vec<Point> = Vec::new();
    for (way, line) in ways.iter().enumerate() {
        let last = line.points.len() - 1;
        for end in [0, last] {
            let p = line.points[end];
            let on = near(p, 1).any(|(other, at)| {
                other != way && length(sub(ways[other].points[at], p)) <= ways[other].width / 2.0
            });
            if on {
                joints.push(p);
            }
        }
        for (at, p) in line.points.iter().enumerate() {
            let crossed = near(*p, 1).any(|(other, there)| {
                other > way && length(sub(ways[other].points[there], *p)) <= STEP_M / 2.0
            });
            if crossed && at > 0 && at < last {
                joints.push(*p);
            }
        }
    }
    // Joints within `PLACE_M` of each other, by chains, are one place.
    let mut place_of: Vec<usize> = (0..joints.len()).collect();
    fn root(place_of: &mut [usize], mut at: usize) -> usize {
        while place_of[at] != at {
            place_of[at] = place_of[place_of[at]];
            at = place_of[at];
        }
        at
    }
    let mut by_cell: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, p) in joints.iter().enumerate() {
        by_cell.entry(cell(*p, PLACE_M)).or_default().push(index);
    }
    for (index, p) in joints.iter().enumerate() {
        let (i, j) = cell(*p, PLACE_M);
        for dj in -1..=1 {
            for di in -1..=1 {
                for other in by_cell.get(&(i + di, j + dj)).into_iter().flatten() {
                    if *other < index && length(sub(joints[*other], *p)) <= PLACE_M {
                        let (a, b) = (root(&mut place_of, index), root(&mut place_of, *other));
                        place_of[a] = b;
                    }
                }
            }
        }
    }
    let mut places: BTreeMap<usize, Vec<Point>> = BTreeMap::new();
    for (index, joint) in joints.iter().enumerate() {
        let place = root(&mut place_of, index);
        places.entry(place).or_default().push(*joint);
    }

    let mut found = Vec::new();
    let looked = places.len();
    for members in places.values() {
        let count = members.len() as f64;
        let middle = [0, 1].map(|k| members.iter().map(|p| p[k]).sum::<f64>() / count);
        // How far the place's own joints lie from its middle: a way that
        // comes this near, and a little more, comes to the place.
        let spread = members
            .iter()
            .map(|p| length(sub(*p, middle)))
            .fold(0.0, f64::max);
        let reach = ARM_M + spread;
        // Each way's walked points near the place, in order.
        let mut by_way: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (way, at) in near(middle, 2) {
            if length(sub(ways[way].points[at], middle)) < reach {
                by_way.entry(way).or_default().push(at);
            }
        }
        let mut arms: Vec<Arm> = Vec::new();
        for (way, mut at) in by_way {
            at.sort_unstable();
            let line = &ways[way];
            let away = |at: usize| length(sub(line.points[at], middle));
            // Runs of consecutive points inside the reach.
            let mut from = 0;
            while from < at.len() {
                let mut to = from;
                while to + 1 < at.len() && at[to + 1] == at[to] + 1 {
                    to += 1;
                }
                let run = &at[from..=to];
                from = to + 1;
                let nearest = run.iter().map(|at| away(*at)).fold(f64::INFINITY, f64::min);
                if nearest > spread + line.width / 2.0 + 1.0 {
                    continue;
                }
                // An arm where the run leaves the reach, or stops in the
                // open well clear of the place.
                for (end, outward) in [(run[0], -1), (run[run.len() - 1], 1)] {
                    let leaves = if outward < 0 {
                        end > 0
                    } else {
                        end + 1 < line.points.len()
                    };
                    if leaves || away(end) > reach / 2.0 {
                        arms.push(Arm {
                            bearing: bearing(middle, line.points[end]),
                            way,
                            at: end,
                            outward,
                        });
                    }
                }
            }
        }
        arms.sort_by(|a, b| a.bearing.total_cmp(&b.bearing));
        // One carriageway drawn twice is one arm: the wider way's.
        let mut distinct: Vec<Arm> = Vec::new();
        for arm in arms {
            match distinct
                .iter_mut()
                .find(|known| between(known.bearing, arm.bearing) < SAME_ARM_DEG.to_radians())
            {
                Some(known) => {
                    if ways[arm.way].width > ways[known.way].width {
                        *known = arm;
                    }
                }
                None => distinct.push(arm),
            }
        }
        let arms = distinct;
        if arms.len() < 3 {
            continue;
        }
        let name = |arm: &Arm| {
            format!(
                "{:?} {:.0}°",
                ways[arm.way].kind,
                arm.bearing.to_degrees().rem_euclid(360.0)
            )
        };
        let listed = || arms.iter().map(name).collect::<Vec<_>>().join(", ");
        let street = |arm: &Arm| ways[arm.way].kind == SurfaceKind::Road;
        if arms.len() >= 5 {
            // Two roads of the network that leave a place side by side are
            // its own fork, counted below: they are one way out of the
            // place, and it is the streets that make it five.
            let mut ways_out = 0;
            for (index, arm) in arms.iter().enumerate() {
                let before = &arms[(index + arms.len() - 1) % arms.len()];
                let beside = !street(arm)
                    && !street(before)
                    && between(arm.bearing, before.bearing) < FORK_DEG.to_radians();
                ways_out += usize::from(!beside);
            }
            let flaw = if ways_out >= 5 && arms.iter().any(street) {
                Flaw::ManyArms
            } else {
                Flaw::RoadManyArms
            };
            found.push((flaw, middle, listed()));
        }
        for (index, a) in arms.iter().enumerate() {
            let b = &arms[(index + 1) % arms.len()];
            if between(a.bearing, b.bearing) < FORK_DEG.to_radians() {
                let flaw = if street(a) || street(b) {
                    Flaw::Fork
                } else {
                    Flaw::RoadFork
                };
                found.push((flaw, middle, format!("{} and {}", name(a), name(b))));
            }
        }
        // A country road through the place: two arms of one road, and the
        // way it runs here.
        let through = arms.iter().enumerate().find_map(|(index, a)| {
            arms[index + 1..]
                .iter()
                .find(|b| {
                    b.way == a.way
                        && ways[a.way].kind == SurfaceKind::CountryRoad
                        && between(a.bearing, b.bearing) > 2.6
                })
                .map(|b| bearing(ways[a.way].points[a.at], ways[b.way].points[b.at]))
        });
        let Some(road) = through else { continue };
        for arm in arms.iter().filter(|arm| street(arm)) {
            let off_square = |heading: f64| {
                let angle = between(heading, road);
                (angle - core::f64::consts::FRAC_PI_2).abs()
            };
            if off_square(arm.bearing) > SLANT_DEG.to_radians() {
                found.push((
                    Flaw::Slant,
                    middle,
                    format!(
                        "{} meets the road {:.0}° off square",
                        name(arm),
                        off_square(arm.bearing).to_degrees()
                    ),
                ));
            }
            // The way it runs from the arm on, away from the place.
            let line = &ways[arm.way];
            let steps = (HOOK_M / STEP_M) as i64;
            let far = arm.at as i64 + arm.outward * steps;
            if far >= 0 && (far as usize) < line.points.len() {
                let onward = bearing(line.points[arm.at], line.points[far as usize]);
                let turned = between(onward, arm.bearing);
                if turned > HOOK_DEG.to_radians() {
                    found.push((
                        Flaw::Hook,
                        middle,
                        format!(
                            "{} turns {:.0}° in its last {} m before the road",
                            name(arm),
                            turned.to_degrees(),
                            ARM_M + HOOK_M
                        ),
                    ));
                }
            }
        }
    }
    (found, looked)
}

/// Every flaw of every map of the sweep, by kind, and how many places they
/// were found among.
fn sweep() -> &'static (BTreeMap<Flaw, Vec<String>>, usize) {
    static SWEEP: OnceLock<(BTreeMap<Flaw, Vec<String>>, usize)> = OnceLock::new();
    SWEEP.get_or_init(|| {
        let mut all: BTreeMap<Flaw, Vec<String>> = BTreeMap::new();
        let mut looked = 0;
        for map_type in TYPES {
            for size in SIZES {
                for seed in SEEDS {
                    let plan = plan(map_type, size, seed);
                    let (found, places) = flaws(&plan);
                    looked += places;
                    for (flaw, at, what) in found {
                        all.entry(flaw).or_default().push(format!(
                            "{map_type:?} {size:?} seed {seed} at ({:.0}, {:.0}): {what}",
                            at[0], at[1]
                        ));
                    }
                }
            }
        }
        let counts: Vec<String> = all
            .iter()
            .map(|(flaw, list)| format!("{flaw:?} {}", list.len()))
            .collect();
        println!(
            "junction flaws among {looked} places: {}",
            counts.join(", ")
        );
        (all, looked)
    })
}

/// Hold one flaw to `allowed` over the sweep, listing the first few.
fn hold(flaw: Flaw, allowed: usize) {
    let (all, _) = sweep();
    let found = all.get(&flaw).map_or(&[][..], |list| &list[..]);
    assert!(
        found.len() <= allowed,
        "{} places are {flaw:?}, and {allowed} may be:\n  {}",
        found.len(),
        found
            .iter()
            .take(LISTED)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// A crossroads is the most a junction is: a secondary road leaves its main
/// road at a junction of its own, not where a street already crosses.
#[test]
fn no_junction_has_five_arms() {
    hold(Flaw::ManyArms, MANY_ARMS);
}

/// Two carriageways that leave one place leave it at least 45° apart: no
/// street peels off another and leaves a sliver of grass between them.
#[test]
fn no_street_forks_off_at_less_than_45_degrees() {
    hold(Flaw::Fork, FORKS);
}

/// A town's streets meet the road through it square, the way streets meet
/// an avenue, and not at whatever slant its grid happens to make with it.
#[test]
fn a_towns_streets_meet_its_roads_square() {
    hold(Flaw::Slant, SLANTS);
}

/// A street that meets a road square does so on its own line: it does not
/// run at a slant and turn in its last few metres.
#[test]
fn no_street_hooks_into_the_road_it_meets() {
    hold(Flaw::Hook, HOOKS);
}
