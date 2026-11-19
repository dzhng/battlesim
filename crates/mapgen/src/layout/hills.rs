//! Rolling hills: groups of broad rounded rises across the open country, on
//! plains maps in large sections and on mixed maps in a few, never on a
//! city. Each rise is mirrored through the map's centre, so the halves hold
//! the same high ground, and each keeps clear of settlements, rivers and
//! their banks, swamps, bridges and their approaches, the skirmish
//! objectives' clearings and the main settlement's kept approach lanes, so
//! everything built on flat ground and the main firing lanes stay flat: a
//! summit in a lane, where no tree may stand, could hide nothing from an
//! eye on it. The rest of the approaches and fronts may roll: open ground
//! is ground a force can advance over (`measure::approaches`), and every
//! hillside is drivable. A rise is as
//! broad as its place allows and as tall as its grade makes that breadth.
//! Where rises overlap the land never stands above the presets'
//! `max_height_m`, as bounded tile by tile (`crate::relief`). They are
//! drawn from their own stream, after everything they keep clear of, so a
//! seed's other features never move.
use super::geometry::{distance, ring_distance, Point, TAU};
use super::sites::Corridor as Reserved;
use super::towns::Town;
use super::water::Water;
use super::Context;
use crate::parcels::space::Rect;
use contract::encounter::SkirmishSites;
use contract::ground::GroundShape;
use contract::map::{Bridge, Relief};
use contract::swamp::Swamp;

/// The tile the height cap is bounded over.
const CAP_TILE_M: f64 = 64.0;
/// Places a group tries for each rise it draws.
const TRIES_PER_RISE: u32 = 8;
/// A rise too broad for its place narrows by this until it fits.
const SHRINK: f64 = 0.85;

/// What a hill keeps clear of.
pub struct Clear<'a> {
    pub towns: &'a [Town],
    /// The skirmish objectives' clearings; the main settlement's fronts
    /// are approaches and may roll.
    pub clearings: &'a [Reserved],
    pub water: &'a Water<'a>,
    pub bridges: &'a [Bridge],
    pub swamps: &'a [Swamp],
    pub skirmish: Option<&'a SkirmishSites>,
    /// The main settlement's kept approach lanes, where no tree may stand
    /// (`open_country::kept_corridors`).
    pub main_lanes: &'a [Rect],
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
            for _ in 0..count * TRIES_PER_RISE {
                if stood == count {
                    break;
                }
                let (angle, reach) = (rng.range([0.0, TAU]), group * libm::sqrt(rng.unit()));
                let center = [
                    middle[0] + reach * libm::cos(angle),
                    middle[1] + reach * libm::sin(angle),
                ];
                let mirror = [extent - center[0], extent - center[1]];
                // As broad as the ground here allows, its height in step.
                let grade = rng.range(rules.peak_grade);
                let mut radius_m = rng.range(rules.radius_m);
                while radius_m >= rules.radius_m[0]
                    && !(fits(context, clear, center, radius_m)
                        && fits(context, clear, mirror, radius_m))
                {
                    radius_m *= SHRINK;
                }
                if radius_m < rules.radius_m[0] {
                    continue;
                }
                // Near the edge no higher than a wood hides.
                let peak_m = if [center, mirror].iter().flatten().any(|&v| {
                    v - radius_m < rules.edge_gap_m || v + radius_m > extent - rules.edge_gap_m
                }) {
                    (grade * radius_m).min(rules.edge_height_m)
                } else {
                    grade * radius_m
                };
                let rise = |center: Point| Relief::Ridge {
                    center,
                    peak_m,
                    radius_m,
                };
                let before = rises.len();
                rises.extend([rise(center), rise(mirror)]);
                if capped(&rises, rules, extent, center, mirror, radius_m) {
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
    let lanes = clear
        .main_lanes
        .iter()
        .all(|lane| lane.gap(center) >= radius + rules.lane_gap_m);
    towns
        && river
        && lanes
        && swamps
        && decks
        && objectives
        && !clear
            .clearings
            .iter()
            .any(|c| c.blocks(center, radius + rules.objective_gap_m))
}

/// No tile the two newest rises reach could stand above its cap with every
/// rise's most there summed: `max_height_m`, or `edge_height_m` on a tile
/// within `edge_gap_m` of the map's edge.
fn capped(
    rises: &[Relief],
    rules: &super::presets::Hills,
    extent: f64,
    a: Point,
    b: Point,
    radius: f64,
) -> bool {
    [a, b].into_iter().all(|c| {
        let at = |v: f64| libm::floor((v - radius) / CAP_TILE_M) as i64;
        let to = |v: f64| libm::floor((v + radius) / CAP_TILE_M) as i64;
        (at(c[1])..=to(c[1])).all(|y| {
            (at(c[0])..=to(c[0])).all(|x| {
                let b =
                    [x as f64, y as f64, (x + 1) as f64, (y + 1) as f64].map(|v| v * CAP_TILE_M);
                let edge =
                    b[0].min(b[1]) < rules.edge_gap_m || b[2].max(b[3]) > extent - rules.edge_gap_m;
                let cap = if edge {
                    rules.edge_height_m
                } else {
                    rules.max_height_m
                };
                crate::relief::span(rises, b)[1] <= cap
            })
        })
    })
}
