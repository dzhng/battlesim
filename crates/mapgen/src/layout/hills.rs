//! Rolling hills: groups of low rounded rises across the open country, on
//! plains maps in large sections and on mixed maps in a few, never on a
//! city. Each rise is mirrored through the map's centre, so the halves hold
//! the same high ground, and each keeps clear of settlements, rivers and
//! their banks, swamps, bridges and their approaches, reserved sites and
//! approach corridors, so everything built or measured on flat ground stays
//! flat. Where rises overlap the land never stands above the presets'
//! `max_height_m`, as bounded tile by tile (`crate::relief`), the same bound
//! the ground-sight certificate reads. They are drawn from their own
//! stream, after everything they keep clear of, so a seed's other features
//! never move.
use super::geometry::{distance, ring_distance, Point, TAU};
use super::measure::Corridor;
use super::sites::Corridor as Reserved;
use super::towns::Town;
use super::water::Water;
use super::Context;
use contract::encounter::SkirmishSites;
use contract::ground::GroundShape;
use contract::map::{Bridge, Relief};
use contract::swamp::Swamp;

/// The tile the height cap is bounded over.
const CAP_TILE_M: f64 = 64.0;
/// Points about a rise's rim that must lie outside every approach corridor.
const RIM_SAMPLES: usize = 24;

/// What a hill keeps clear of.
pub struct Clear<'a> {
    pub towns: &'a [Town],
    pub reserved: &'a [Reserved],
    pub water: &'a Water<'a>,
    pub bridges: &'a [Bridge],
    pub swamps: &'a [Swamp],
    pub skirmish: Option<&'a SkirmishSites>,
    pub corridors: &'a [Corridor],
}

pub fn place(context: &Context, clear: &Clear) -> Vec<Relief> {
    let rules = &context.presets.hills;
    let Some(density) = rules.types.get(&context.request.map_type) else {
        return Vec::new();
    };
    let mut rng = context.stream("hills");
    if !rng.chance(density.chance) {
        return Vec::new();
    }
    let extent = context.extent;
    let pairs = libm::round(density.pairs_per_km2 * extent * extent / 1e6) as u32;
    let mut rises: Vec<Relief> = Vec::new();
    for _ in 0..pairs {
        let group = rng.range(rules.group_radius_m);
        let count = rng.count(rules.rises);
        for _ in 0..rules.attempts {
            // A group's middle in the top half; its mirror is in the bottom.
            let middle = [rng.range([0.0, extent]), rng.range([extent / 2.0, extent])];
            let mut stood = 0;
            for _ in 0..count * 3 {
                if stood == count {
                    break;
                }
                let (angle, reach) = (rng.range([0.0, TAU]), group * libm::sqrt(rng.unit()));
                let center = [
                    middle[0] + reach * libm::cos(angle),
                    middle[1] + reach * libm::sin(angle),
                ];
                let radius_m = rng.range(rules.radius_m);
                let peak_m = rng.range(rules.peak_m);
                let rise = |center: Point| Relief::Ridge {
                    center,
                    peak_m,
                    radius_m,
                };
                let mirror = [extent - center[0], extent - center[1]];
                if !(fits(context, clear, center, radius_m)
                    && fits(context, clear, mirror, radius_m))
                {
                    continue;
                }
                let before = rises.len();
                rises.extend([rise(center), rise(mirror)]);
                if capped(&rises, rules.max_height_m, center, mirror, radius_m) {
                    stood += 1;
                } else {
                    rises.truncate(before);
                }
            }
            if stood > 0 {
                break;
            }
        }
    }
    rises
}

/// A rise of `radius` about `center` stands wholly on the map, clear of
/// everything it keeps clear of.
fn fits(context: &Context, clear: &Clear, center: Point, radius: f64) -> bool {
    let rules = &context.presets.hills;
    let extent = context.extent;
    if center.iter().any(|&v| v < radius || v > extent - radius) {
        return false;
    }
    let towns = clear
        .towns
        .iter()
        .all(|t| ring_distance(&t.outline, center) >= radius + rules.settlement_gap_m);
    let within = radius + rules.river_gap_m;
    let river = clear.water.is_empty() || clear.water.gap(center, within) >= within;
    let swamps = clear.swamps.iter().all(|s| match &s.shape {
        GroundShape::Polygon { ring } => ring_distance(ring, center) >= radius + rules.swamp_gap_m,
        GroundShape::Stroke { .. } => false,
    });
    let decks = clear.bridges.iter().all(|deck| {
        let length = deck.half_extents[0] + context.presets.rivers.bridge.approach_m;
        distance(center, deck.center) >= radius + length + rules.bridge_gap_m
    });
    let objectives = clear.skirmish.is_none_or(|sites| {
        let mut circles = sites
            .all_reserved_objectives()
            .map(|o| (o.center, o.radius_m))
            .chain(sites.entries.iter().map(|e| (e.center, 0.0)));
        circles.all(|(c, r)| distance(center, c) >= radius + r + rules.objective_gap_m)
    });
    let rim = radius + rules.corridor_gap_m;
    let corridors = (0..=RIM_SAMPLES).all(|i| {
        let p = if i == RIM_SAMPLES {
            center
        } else {
            let a = TAU * i as f64 / RIM_SAMPLES as f64;
            [
                center[0] + rim * libm::cos(a),
                center[1] + rim * libm::sin(a),
            ]
        };
        !clear.corridors.iter().any(|c| c.contains(p))
    });
    towns
        && river
        && swamps
        && decks
        && objectives
        && corridors
        && !clear
            .reserved
            .iter()
            .any(|c| c.blocks(center, radius + rules.objective_gap_m))
}

/// No tile the two newest rises reach could stand above `cap` with every
/// rise's most there summed.
fn capped(rises: &[Relief], cap: f64, a: Point, b: Point, radius: f64) -> bool {
    [a, b].into_iter().all(|c| {
        let at = |v: f64| libm::floor((v - radius) / CAP_TILE_M) as i64;
        let to = |v: f64| libm::floor((v + radius) / CAP_TILE_M) as i64;
        (at(c[1])..=to(c[1])).all(|y| {
            (at(c[0])..=to(c[0])).all(|x| {
                let b =
                    [x as f64, y as f64, (x + 1) as f64, (y + 1) as f64].map(|v| v * CAP_TILE_M);
                crate::relief::span(rises, b)[1] <= cap
            })
        })
    })
}
