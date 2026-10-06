//! Streets as a town has them, judged on the finished plan: a street runs
//! from a junction to a junction or ends at the last lot it serves, so none
//! is a stub, none stops just short of the road ahead of it, and no two lie
//! side by side on the same ground. Each wart is counted over every type and
//! size and held at or near zero. The arithmetic here is this file's own,
//! not the generator's.
#[path = "common/limits.rs"]
mod limits;
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};
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
/// Every cell runs these seeds: a claim about streets is a claim about the
/// generator, not one map.
const SEEDS: [u64; 4] = [1, 2, 3, u64::MAX];

type Point = [f64; 2];

const CELL_M: f64 = 64.0;
/// A carriageway shorter than this many of its own widths is a stub.
const STUB_WIDTHS: f64 = 2.0;
/// A street that stops with a carriageway this near ahead of it should have
/// met it: nothing can be built on the ground between.
const SHORT_OF_M: f64 = 40.0;
/// The ground ahead of an end is searched along its line and this far to
/// either side of it (about 20°).
const AHEAD_SPREAD: f64 = 0.36;
/// Two carriageways whose pavings touch for longer than this while running
/// within `ALONGSIDE_COS` of the same way are one street drawn twice.
const ALONGSIDE_M: f64 = 20.0;
const ALONGSIDE_COS: f64 = 0.866;
/// Two streets that end on one road from opposite sides, this near each
/// other along it but not on one line, are a crossroads that missed: their
/// kerbs are nearer than a lot is wide, so nothing stands between them.
const STAGGER_M: [f64; 2] = [2.0, 20.0];
/// Two turns sharper than `DOGLEG_COS` (35°) nearer each other than this
/// many widths are a dogleg, not a bend.
const DOGLEG_WIDTHS: f64 = 3.0;
const DOGLEG_COS: f64 = 0.819;
/// A street's own paving this many widths or more along it from an end is
/// paving that end may join.
const OWN_WIDTHS: f64 = 6.0;
/// What the generator still leaves over the sweep's 36 maps, among some
/// forty thousand street ends and junctions. Each is a count to bring down.
const STOPS_SHORT: usize = 3;
const ALONGSIDE: usize = 1;
const STAGGER: usize = 19;
const DOGLEG: usize = 1;
/// How many of each wart a report lists.
const LISTED: usize = 8;

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
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn length(a: Point) -> f64 {
    a[0].hypot(a[1])
}
fn unit(a: Point) -> Point {
    let l = length(a);
    [a[0] / l, a[1] / l]
}

/// One carriageway's rounded line.
struct Way<'a> {
    area: &'a SurfaceArea,
    samples: &'a [Point],
    authored: &'a [Point],
    width: f64,
    length: f64,
}

/// The plan's carriageways, their stretches bucketed by the ground each
/// one's width covers.
struct Streets<'a> {
    ways: Vec<Way<'a>>,
    /// (way, first sample of the stretch)
    cells: HashMap<(i64, i64), Vec<(usize, usize)>>,
}

fn cell(p: Point) -> (i64, i64) {
    (
        (p[0] / CELL_M).floor() as i64,
        (p[1] / CELL_M).floor() as i64,
    )
}

impl<'a> Streets<'a> {
    fn new(plan: &'a MapPlan) -> Self {
        let mut ways = Vec::new();
        let mut cells: HashMap<(i64, i64), Vec<(usize, usize)>> = HashMap::new();
        for area in &plan.surfaces {
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
            for (at, pair) in samples.windows(2).enumerate() {
                let (low, high) = (
                    cell([
                        pair[0][0].min(pair[1][0]) - width_m,
                        pair[0][1].min(pair[1][1]) - width_m,
                    ]),
                    cell([
                        pair[0][0].max(pair[1][0]) + width_m,
                        pair[0][1].max(pair[1][1]) + width_m,
                    ]),
                );
                for j in low.1..=high.1 {
                    for i in low.0..=high.0 {
                        cells.entry((i, j)).or_default().push((ways.len(), at));
                    }
                }
            }
            ways.push(Way {
                area,
                samples,
                authored: centerline.control_points(),
                width: *width_m,
                length: samples.windows(2).map(|p| length(sub(p[1], p[0]))).sum(),
            });
        }
        Self { ways, cells }
    }

    /// The stretches of other ways near `p`: (way, the stretch's ends).
    fn near(&self, own: usize, p: Point) -> impl Iterator<Item = (usize, Point, Point)> + '_ {
        self.cells
            .get(&cell(p))
            .into_iter()
            .flatten()
            .filter(move |(way, _)| *way != own)
            .map(|(way, at)| {
                let samples = self.ways[*way].samples;
                (*way, samples[*at], samples[*at + 1])
            })
    }

    /// The other way whose paving holds `p`.
    fn paved(&self, own: usize, p: Point) -> Option<usize> {
        let mut seen = None;
        for (way, _, _) in self.near(own, p) {
            if seen != Some(way) && self.ways[way].area.shape.contains(p, 0.02) {
                return Some(way);
            }
            seen = Some(way);
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum Wart {
    /// A carriageway shorter than two of its own widths.
    Stub,
    /// An end that stops in the open with a carriageway just ahead of it.
    StopsShort,
    /// Two carriageways laid side by side on the same ground, one of them a
    /// town's street.
    Alongside,
    /// A country road and a track, or two of either, side by side: a lane
    /// that peels off its road at a slant. The road network's, counted
    /// here and not held.
    RoadsAlongside,
    /// Two streets that meet one road from opposite sides, a few metres
    /// apart along it.
    Stagger,
    /// Two sharp turns a few metres apart.
    Dogleg,
}

fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    contract::ground::segment_distance(a, b, p)
}

/// Every wart of `plan`'s streets, with where it is, and how many ends and
/// junctions were looked at.
fn warts(plan: &MapPlan) -> (Vec<(Wart, Point, String)>, usize) {
    let streets = Streets::new(plan);
    let mut found = Vec::new();
    // (road joined, distance along it, side, where)
    let mut joints: BTreeMap<usize, Vec<(f64, bool, Point, Point)>> = BTreeMap::new();
    let mut ends = 0;
    for (index, way) in streets.ways.iter().enumerate() {
        let describe = || {
            format!(
                "{:?} {} m wide, {:.0} m long, {:?} .. {:?}",
                way.area.kind,
                way.width,
                way.length,
                way.authored[0],
                way.authored[way.authored.len() - 1]
            )
        };
        // (One that lies wholly under another carriageway is no street to
        // the eye: the joint pass leaves such a piece where a road changes
        // width under the road that crosses it.)
        let shows = way
            .samples
            .iter()
            .any(|p| streets.paved(index, *p).is_none());
        if way.length < STUB_WIDTHS * way.width && shows {
            found.push((Wart::Stub, way.samples[0], describe()));
        }
        let last = way.samples.len() - 1;
        for (at, before) in [
            (way.samples[0], way.samples[1]),
            (way.samples[last], way.samples[last - 1]),
        ] {
            ends += 1;
            let out_of = unit(sub(at, before));
            if let Some(other) = streets.paved(index, at) {
                // Where along the road it joins, and from which side.
                let road = &streets.ways[other];
                let mut along = 0.0;
                let mut best = (f64::INFINITY, 0.0, false, out_of);
                for pair in road.samples.windows(2) {
                    let step = sub(pair[1], pair[0]);
                    let span = length(step);
                    let away = segment_distance(pair[0], pair[1], at);
                    if away < best.0 {
                        // Where the street's own line crosses the road's
                        // middle, not where its end happens to stop: an end
                        // runs a little past the middle, along its own line.
                        let sin = step[0] * out_of[1] - step[1] * out_of[0];
                        let offset = sub(at, pair[0]);
                        let t = if sin.abs() > 0.2 * span {
                            (offset[0] * out_of[1] - offset[1] * out_of[0]) / sin
                        } else {
                            dot(offset, step) / (span * span)
                        };
                        best = (away, along + t * span, sin < 0.0, unit(step));
                    }
                    along += span;
                }
                // Only a street that comes in across the road, not one that
                // carries on from its end.
                if dot(best.3, out_of).abs() < ALONGSIDE_COS
                    && best.1 > road.width
                    && best.1 < road.length - road.width
                {
                    joints
                        .entry(other)
                        .or_default()
                        .push((best.1, best.2, at, out_of));
                }
                continue;
            }
            // A street that comes back round to itself ends on itself.
            let mut run = 0.0;
            let ordered: Vec<Point> = if at == way.samples[0] {
                way.samples.to_vec()
            } else {
                way.samples.iter().rev().copied().collect()
            };
            let own = ordered.windows(2).any(|pair| {
                let before = run;
                run += length(sub(pair[1], pair[0]));
                before > OWN_WIDTHS * way.width
                    && segment_distance(pair[0], pair[1], at) <= way.width / 2.0
            });
            if own {
                continue;
            }
            // What lies ahead of an end nothing covers.
            let across = [-out_of[1], out_of[0]];
            let ahead = (1..=SHORT_OF_M as usize).find(|step| {
                [-AHEAD_SPREAD, 0.0, AHEAD_SPREAD].into_iter().any(|aside| {
                    let reach = *step as f64;
                    let p = [0, 1].map(|k| at[k] + reach * (out_of[k] + aside * across[k]));
                    streets.paved(index, p).is_some()
                })
            });
            if let Some(gap) = ahead {
                found.push((
                    Wart::StopsShort,
                    at,
                    format!("{gap} m short of a road: {}", describe()),
                ));
            }
        }
        // Stretches of it that lie on another way's paving, running its way.
        let mut beside: BTreeMap<usize, (f64, Point)> = BTreeMap::new();
        for pair in way.samples.windows(2) {
            let step = sub(pair[1], pair[0]);
            let span = length(step);
            let pieces = (span / 2.0).ceil().max(1.0) as usize;
            for piece in 0..pieces {
                let p = [0, 1].map(|k| pair[0][k] + step[k] * (piece as f64 + 0.5) / pieces as f64);
                let mut touched: Vec<usize> = Vec::new();
                for (other, a, b) in streets.near(index, p) {
                    let reach = (way.width + streets.ways[other].width) / 2.0;
                    if other > index
                        && !touched.contains(&other)
                        && segment_distance(a, b, p) < reach
                        && dot(unit(step), unit(sub(b, a))).abs() >= ALONGSIDE_COS
                    {
                        touched.push(other);
                    }
                }
                for other in touched {
                    let run = beside.entry(other).or_insert((0.0, p));
                    run.0 += span / pieces as f64;
                }
            }
        }
        for (other, (run, at)) in beside {
            if run > ALONGSIDE_M {
                let street = |kind: SurfaceKind| kind == SurfaceKind::Road;
                let wart = if street(way.area.kind) || street(streets.ways[other].area.kind) {
                    Wart::Alongside
                } else {
                    Wart::RoadsAlongside
                };
                found.push((
                    wart,
                    at,
                    format!(
                        "{run:.0} m beside {:?} {} m wide: {}",
                        streets.ways[other].area.kind,
                        streets.ways[other].width,
                        describe()
                    ),
                ));
            }
        }
        for (turn, run) in way.authored.windows(4).enumerate() {
            let legs = [
                unit(sub(run[1], run[0])),
                unit(sub(run[2], run[1])),
                unit(sub(run[3], run[2])),
            ];
            if length(sub(run[2], run[1])) < DOGLEG_WIDTHS * way.width
                && dot(legs[0], legs[1]) < DOGLEG_COS
                && dot(legs[1], legs[2]) < DOGLEG_COS
            {
                found.push((
                    Wart::Dogleg,
                    run[1],
                    format!("turn {turn} of {}", describe()),
                ));
            }
        }
    }
    let mut junctions = 0;
    for (road, mut joined) in joints {
        junctions += joined.len();
        joined.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (index, a) in joined.iter().enumerate() {
            for b in &joined[index + 1..] {
                let apart = b.0 - a.0;
                if apart > STAGGER_M[1] {
                    break;
                }
                if a.1 != b.1 && apart >= STAGGER_M[0] {
                    found.push((
                        Wart::Stagger,
                        a.2,
                        format!(
                            "{apart:.1} m from the street at {:?}, across a {:?} {} m wide",
                            b.2, streets.ways[road].area.kind, streets.ways[road].width
                        ),
                    ));
                }
            }
        }
    }
    (found, ends + junctions)
}

/// Every wart of every map of the sweep, by kind, and how many ends and
/// junctions they were found among.
fn sweep() -> &'static (BTreeMap<Wart, Vec<String>>, usize) {
    static SWEEP: OnceLock<(BTreeMap<Wart, Vec<String>>, usize)> = OnceLock::new();
    SWEEP.get_or_init(|| {
        let mut all: BTreeMap<Wart, Vec<String>> = BTreeMap::new();
        let mut looked = 0;
        for map_type in TYPES {
            for size in SIZES {
                for seed in SEEDS {
                    let plan = plan(map_type, size, seed);
                    let (found, ends) = warts(&plan);
                    looked += ends;
                    for (wart, at, what) in found {
                        all.entry(wart).or_default().push(format!(
                            "{map_type:?} {size:?} seed {seed} at ({:.0}, {:.0}): {what}",
                            at[0], at[1]
                        ));
                    }
                }
            }
        }
        let counts: Vec<String> = all
            .iter()
            .map(|(wart, list)| format!("{wart:?} {}", list.len()))
            .collect();
        println!(
            "street warts among {looked} ends and junctions: {}",
            counts.join(", ")
        );
        (all, looked)
    })
}

/// Hold one wart to `allowed` over the sweep, listing the first few. The
/// counts allowed are what the generator leaves today: lower one when its
/// wart is closed, never raise one.
fn hold(wart: Wart, allowed: usize) {
    let (all, _) = sweep();
    let found = all.get(&wart).map_or(&[][..], |list| &list[..]);
    assert!(
        found.len() <= allowed,
        "{} carriageways are {wart:?}, and {allowed} may be:\n  {}",
        found.len(),
        found
            .iter()
            .take(LISTED)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// A link shorter than a couple of street widths is not a street: it is the
/// lump where two streets missed each other.
#[test]
fn no_street_is_shorter_than_two_of_its_widths() {
    hold(Wart::Stub, 0);
}

/// A street runs on to the road ahead of it, or stops at the last lot it
/// serves with nothing ahead: it never stops a few tens of metres short.
/// (`layout-9` left 120 over these maps.)
#[test]
fn no_street_stops_just_short_of_the_road_ahead() {
    hold(Wart::StopsShort, STOPS_SHORT);
}

/// Two streets side by side on the same ground are one street drawn twice,
/// or a stub lying along a road like a parking strip. (`layout-9`: 115.)
#[test]
fn no_two_streets_run_alongside_within_their_widths() {
    hold(Wart::Alongside, ALONGSIDE);
}

/// Streets either side of a road meet it at one crossroads or well apart.
/// (`layout-9`: 1,538.)
#[test]
fn streets_across_a_road_from_each_other_line_up() {
    hold(Wart::Stagger, STAGGER);
}

/// A street bends; it does not jog through two sharp turns in a few metres.
/// (`layout-9`: 11.)
#[test]
fn no_street_doglegs() {
    hold(Wart::Dogleg, DOGLEG);
}
