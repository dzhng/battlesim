//! Swamps: standing shallow water on flat ground, beside the river where
//! the map has one (decision 5). They come in pairs of one size, one in each
//! half, so the halves hold the same wet ground; each keeps clear of
//! settlements, roads, woods, reserved approaches and objectives, bridges
//! and their approaches, the river's bank, and other swamps. Beside a river
//! a swamp lies along it, as a marsh on its floodplain does. They are drawn
//! from their own stream, after everything they keep clear of, so a seed's
//! other features never move.
use super::geometry::{distance, ring_distance, ring_gap, segment_ring_gap, Outline, Point, PI};
use super::rng::Stream;
use super::sites::Corridor;
use super::towns::Town;
use super::water::Water;
use super::Context;
use crate::Half;
use contract::encounter::SkirmishSites;
use contract::ground::GroundShape;
use contract::map::{Bridge, Forest, SurfaceArea};
use contract::swamp::Swamp;

/// What a swamp keeps clear of.
pub struct Clear<'a> {
    pub towns: &'a [Town],
    pub reserved: &'a [Corridor],
    pub water: &'a Water<'a>,
    pub bridges: &'a [Bridge],
    pub roads: &'a [SurfaceArea],
    pub woods: &'a [Forest],
    pub skirmish: Option<&'a SkirmishSites>,
}

pub fn place(context: &Context, clear: &Clear) -> Vec<Swamp> {
    let rules = &context.presets.swamps;
    let mut rng = context.stream("swamps");
    if !rng.chance(rules.chance[&context.request.map_type]) {
        return Vec::new();
    }
    let mut placed: Vec<Outline> = Vec::new();
    for _ in 0..rng.count(rules.pairs) {
        let radius = rng.range(rules.radius_m);
        let aspect = rng.range(rules.aspect);
        let size = PI * radius * radius * aspect;
        let Some(top) = draw(context, clear, &placed, size, aspect, Half::Top, &mut rng) else {
            continue;
        };
        let with_top: Vec<Outline> = placed.iter().cloned().chain([top.clone()]).collect();
        if let Some(bottom) = draw(
            context,
            clear,
            &with_top,
            size,
            aspect,
            Half::Bottom,
            &mut rng,
        ) {
            placed.extend([top, bottom]);
        }
    }
    placed
        .into_iter()
        .filter_map(|outline| {
            Some(Swamp {
                shape: GroundShape::polygon(outline.ring).ok()?,
                depth_m: rules.depth_m,
            })
        })
        .collect()
}

/// One swamp of `size` wholly inside `half`, clear of everything it keeps
/// clear of; `None` when no attempt finds room.
fn draw(
    context: &Context,
    clear: &Clear,
    placed: &[Outline],
    size: f64,
    aspect: f64,
    half: Half,
    rng: &mut Stream,
) -> Option<Outline> {
    let rules = &context.presets.swamps;
    let extent = context.extent;
    let (low, high) = match half {
        Half::Top => (extent / 2.0, extent),
        Half::Bottom => (0.0, extent / 2.0),
    };
    for _ in 0..rules.attempts {
        let at = [rng.range([0.0, extent]), rng.range([low, high])];
        // Beside a river its long axis follows the water's course there.
        let course = clear
            .water
            .crossings([0.0, at[1]], [extent, at[1]])
            .into_iter()
            .min_by(|a, b| (a.at[0] - at[0]).abs().total_cmp(&(b.at[0] - at[0]).abs()));
        let rotation = match course {
            Some(crossing) => crossing.downstream + rng.range([-0.25, 0.25]),
            None => rng.range([0.0, PI]),
        };
        let outline = Outline::draw(rules.shape(), size, aspect, rotation, rng).at(at);
        let reach = outline.reach;
        let inside = at[0] >= reach
            && at[0] <= extent - reach
            && at[1] >= low + reach
            && at[1] <= high - reach;
        if inside && fits(context, clear, placed, &outline) {
            return Some(outline);
        }
    }
    None
}

fn fits(context: &Context, clear: &Clear, placed: &[Outline], swamp: &Outline) -> bool {
    let rules = &context.presets.swamps;
    let (center, reach, ring) = (swamp.center, swamp.reach, &swamp.ring[..]);
    // Rings that cross or hold one another are never apart, whatever the gap.
    let apart = |other: &[Point], gap: f64| {
        let between = ring_gap(ring, other);
        between > 0.0 && between >= gap
    };
    let roads = clear.roads.iter().all(|road| match &road.shape {
        GroundShape::Stroke {
            centerline,
            width_m,
        } => {
            let clearance = width_m / 2.0 + rules.road_gap_m;
            centerline.samples().windows(2).all(|pair| {
                distance(pair[0], center) > reach + clearance + distance(pair[0], pair[1])
                    || segment_ring_gap(ring, pair[0], pair[1]) >= clearance
            })
        }
        GroundShape::Polygon { ring: other } => apart(other, rules.road_gap_m),
    });
    let woods = clear.woods.iter().all(|wood| match &wood.shape {
        GroundShape::Polygon { ring: other } => apart(other, rules.wood_gap_m),
        GroundShape::Stroke { .. } => true,
    });
    let river = clear.water.is_empty() || {
        let gap = clear.water.ring_gap(ring, rules.river_reach_m);
        gap >= rules.river_gap_m && gap < rules.river_reach_m
    };
    let decks = clear.bridges.iter().all(|deck| {
        let length = deck.half_extents[0] + context.presets.rivers.bridge.approach_m;
        distance(center, deck.center) >= reach + length + rules.bridge_gap_m
    });
    let objectives = clear.skirmish.is_none_or(|sites| {
        let mut circles = sites
            .all_reserved_objectives()
            .map(|o| (o.center, o.radius_m))
            .chain(sites.entries.iter().map(|e| (e.center, 0.0)));
        circles.all(|(c, r)| ring_distance(ring, c) >= r + rules.settlement_gap_m)
    });
    river
        && roads
        && woods
        && decks
        && objectives
        && !clear.reserved.iter().any(|c| c.blocks(center, reach))
        && clear
            .towns
            .iter()
            .all(|town| apart(&town.outline, rules.settlement_gap_m))
        && placed
            .iter()
            .all(|other| distance(center, other.center) >= reach + other.reach + rules.swamp_gap_m)
}
