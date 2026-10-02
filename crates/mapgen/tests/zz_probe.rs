use contract::encounter::EncounterRecipes;
use contract::scenario::Rules;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::parcels::fill_districts;
use mapgen::street_props::place_street_props;
use mapgen::CompileLimits;
use sim::encounter::PreparedMap;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const RECIPES: &str = include_str!("../../../fixtures/encounters.json");

#[test]
fn probe() {
    let presets = PresetDefinitions::from_json(PRESETS).unwrap();
    let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap()).unwrap();
    let rules: Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let recipes = EncounterRecipes::from_json(RECIPES).unwrap();
    let seed: u64 = std::env::var("PROBE_SEED").map_or(1, |s| s.parse().unwrap());
    let request = GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type: MapType::Metro,
        size: MapSize::Medium,
        limits: CompileLimits {
            max_authored_parts: 60_000,
            max_bay_positions: 600_000,
            max_ground_points: 200_000,
        },
    };
    let mut plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue,
        &presets,
    )
    .unwrap();
    let sites = plan.sites();
    plan.props = place_street_props(&plan, &request, &catalogue, &rules.catalog, &presets).unwrap();
    let map = mapgen::lower(
        &mapgen::CompileRequest::generated(&request, plan),
        &catalogue,
    )
    .unwrap()
    .map;
    let prepared = PreparedMap::new(&map, &rules);
    let out = sim::encounter::plan_encounter(
        &prepared.queries(&map, &sites),
        &rules,
        &recipes.recipes["assault"],
        1.into(),
    );
    println!("PLANNED {:?}", out.map(|_| ()).map_err(|e| e.len()));
}
