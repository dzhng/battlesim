//! Seeded layout: where a map's settlements, river, roads and forests go. The
//! output is an ordinary `MapPlan` for the parcel pass and the compiler; the
//! presets are data.
mod crossings;
mod forests;
pub(crate) mod geometry;
mod measure;
mod presets;
mod rivers;
pub(crate) mod rng;
mod roads;
mod sites;
mod towns;
pub(crate) mod water;

pub use measure::{
    approach_corridors, corridor_start, measure, Corridor, HalfSplit, Journey, LayoutMetrics,
    RiverMetrics, RoadMetrics, TransitMetrics,
};
pub use measure::{approaches, bearing_step};
pub use presets::*;

use crate::{Diagnostic, DiagnosticCode, MapPlan};
use std::collections::BTreeMap;

/// A request pins this; a change that moves any generated point renames it.
pub const GENERATOR_VERSION: &str = "layout-13";

pub use contract::generation::{GenerationRequest, MapSize, MapType};

/// The request's own random stream of that name. The type and size are
/// part of the key, so one seed gives each of the nine maps its own
/// layout, and each consumer draws without moving another's.
pub(crate) fn stream(request: &GenerationRequest, name: &str) -> rng::Stream {
    let (map_type, size) = (request.map_type.name(), request.size.name());
    rng::Stream::new(request.seed.value(), &format!("{map_type}/{size}/{name}"))
}

/// One request's fixed inputs, shared by every generation step.
struct Context<'a> {
    request: &'a GenerationRequest,
    presets: &'a PresetDefinitions,
    preset: &'a TypePreset,
    cell: &'a SizePreset,
    extent: f64,
}

impl Context<'_> {
    fn stream(&self, name: &str) -> rng::Stream {
        stream(self.request, name)
    }

    /// A bounded search ran out. Names the feature, the preset cell and the
    /// seed, so the caller can report it; the generator never tries another seed.
    fn fail(&self, feature: &str, message: String) -> Vec<Diagnostic> {
        let (map_type, size) = (self.request.map_type.name(), self.request.size.name());
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some(feature.into()),
            location: format!("$.presets.types.{map_type}.sizes.{size}"),
            message: format!(
                "{message} ({map_type} {size}, seed {})",
                self.request.seed.value()
            ),
        }]
    }
}

pub fn generate_layout(
    request: &GenerationRequest,
    presets: &PresetDefinitions,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let pins = [
        (
            "generator_version",
            &request.generator_version,
            GENERATOR_VERSION,
        ),
        (
            "preset_revision",
            &request.preset_revision,
            presets.revision.as_str(),
        ),
    ];
    let stale: Vec<_> = pins
        .into_iter()
        .filter(|(_, requested, current)| requested.as_str() != *current)
        .map(|(field, requested, current)| Diagnostic {
            code: DiagnosticCode::InvalidRequest,
            feature: None,
            location: format!("$.{field}"),
            message: format!("the request pins {requested:?}; this generator is {current:?}"),
        })
        .collect();
    if !stale.is_empty() {
        return Err(stale);
    }
    let (preset, cell) = presets.cell(request.map_type, request.size);
    let context = Context {
        request,
        presets,
        preset,
        cell,
        extent: request.size.extent_m(),
    };
    // What kind of country this seed is: a few corridors or a road network,
    // sparse woodland or large forests.
    let mut style = context.stream("style");
    let (richness, woodland) = (style.unit(), style.unit());

    // The main roads' lines come first, so settlements can stand on them.
    let mut road_draws = context.stream("roads");
    let skeleton = roads::skeleton(&context, &mut road_draws);
    let mut source = rivers::Source::new(&context, &skeleton);
    let (placed, rivers) = sites::place(&context, &skeleton, &mut source)?;
    let water = water::Water::new(&rivers, [context.extent; 2]);
    let (roads, bridges) = roads::build(
        &context,
        &skeleton,
        &placed.sites,
        &water,
        richness,
        road_draws,
    )?;
    // A settlement grows from the roads across its ground.
    let towns = towns::grow(&context, &placed.sites, &roads)?;
    let forests = forests::grow(&context, &towns, &placed.reserved, &water, woodland);
    let mut surfaces = roads
        .iter()
        .enumerate()
        .map(|(index, road)| roads::surface(&context, index, road))
        .collect::<Result<Vec<_>, _>>()?;
    surfaces.extend(towns::surfaces(&context, &towns)?);
    let blocks: Vec<&[geometry::Point]> = towns
        .iter()
        .flat_map(|town| town.blocks.iter().map(|block| &block.ring[..]))
        .collect();
    let surfaces = crate::joints::close(
        &context.presets.joints,
        surfaces,
        [context.extent; 2],
        &blocks,
    );
    let settlements = placed
        .sites
        .iter()
        .zip(&towns)
        .enumerate()
        .map(|(index, (site, town))| towns::settlement(&context, index, site, town))
        .collect();
    let mut plan = MapPlan {
        size: [context.extent; 2],
        render_margin_m: presets.terrain.render_margin_m,
        fog_cell_m: presets.terrain.fog_cell_m,
        height_grid_m: presets.terrain.height_grid_m,
        slope_cutoff_deg: presets.terrain.slope_cutoff_deg,
        props: Vec::new(),
        buildings: Vec::new(),
        surfaces,
        forests,
        rivers,
        bridges,
        settlements,
        approaches: Vec::new(),
        lots: Vec::new(),
        regional_family: None,
        courts: Vec::new(),
        unsupported_fields: BTreeMap::new(),
    };
    plan.approaches = measure::approaches(&plan, presets);
    verify(&context, &plan)?;
    Ok(plan)
}

/// Hold the finished plan to the preset rules by measuring it, so a rule is
/// met because the geometry meets it and not because a step meant it to.
fn verify(context: &Context, plan: &MapPlan) -> Result<(), Vec<Diagnostic>> {
    let presets = context.presets;
    let metrics = measure(plan, presets);
    let mut errors = Vec::new();
    let mut check = |ok: bool, feature: &str, message: String| {
        if !ok {
            errors.extend(context.fail(feature, message));
        }
    };
    check(
        metrics.roads.unconnected_settlements == 0,
        "roads",
        format!(
            "{} settlements have no road to the centre",
            metrics.roads.unconnected_settlements
        ),
    );
    check(
        metrics.roads.unbridged == 0,
        "bridges",
        format!(
            "{} runs of road enter the water off any deck",
            metrics.roads.unbridged
        ),
    );
    check(
        metrics.river.fair,
        "fairness.river",
        format!(
            "{:.2} km of river lie in the top half and {:.2} km in the bottom",
            metrics.river.top_km, metrics.river.bottom_km
        ),
    );
    for (name, split) in [("town", &metrics.town), ("forest", &metrics.forest)] {
        check(
            split.fair,
            &format!("fairness.{name}"),
            format!(
                "{name} area is {:.0} m² in the top half and {:.0} m² in the bottom",
                split.top_m2, split.bottom_m2
            ),
        );
    }
    check(
        metrics.main_approach_top && metrics.main_approach_bottom,
        "approach",
        format!(
            "the main settlement lacks {} m of open ground across {} m in a half",
            presets.approach.depth_m, presets.approach.front_m
        ),
    );
    let transit = &metrics.transit;
    let in_time = |journey: &Option<Journey>| {
        journey
            .as_ref()
            .is_some_and(|journey| journey.elapsed_s <= presets.transit.max_s)
    };
    check(
        in_time(&transit.top)
            && in_time(&transit.bottom)
            && transit.top_bottom.is_some()
            && transit.centre_m <= presets.transit.centre_reach_m,
        "transit",
        format!(
            "the top and bottom edges have no road to the centre and each other within {} s: {transit:?}",
            presets.transit.max_s
        ),
    );
    check(
        metrics.urban_share <= context.cell.urban_share_max,
        "urban_share",
        format!(
            "settlements cover {:.3} of the map, above the ceiling {}",
            metrics.urban_share, context.cell.urban_share_max
        ),
    );
    let [low, high] = context.preset.forest_share;
    let slack = presets.forests.share_tolerance;
    check(
        metrics.forest_share >= low - slack && metrics.forest_share <= high + slack,
        "forest_share",
        format!(
            "forest covers {:.3} of the map, outside {low}..{high}",
            metrics.forest_share
        ),
    );
    if metrics.ground_points > context.request.limits.max_ground_points {
        errors.push(Diagnostic {
            code: DiagnosticCode::ComplexityLimit,
            feature: None,
            location: "$.limits.max_ground_points".into(),
            message: format!(
                "the generated plan has {} ground points",
                metrics.ground_points
            ),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
