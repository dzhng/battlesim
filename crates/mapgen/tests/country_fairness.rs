//! Regression requests whose roadside home groups straddle the map midpoint.
use mapgen::layout::{GenerationRequest, MapSize, MapType, PresetDefinitions, GENERATOR_VERSION};
use mapgen::{CompileLimits, GenerateOutcome};

#[test]
fn roadside_homes_balance_the_halves_their_buildings_actually_stand_in() {
    let presets_text = include_str!("../../../fixtures/map-presets.json");
    let templates = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(templates).unwrap())
            .unwrap();
    let presets = PresetDefinitions::from_json(presets_text).unwrap();
    let rules = sim::fixtures::test_rules().to_string();
    for (size, seed) in [(MapSize::Small, 7), (MapSize::Xl, 3)] {
        let request = GenerationRequest {
            generator_version: GENERATOR_VERSION.into(),
            preset_revision: presets.revision.clone(),
            seed: seed.into(),
            template_catalog_hash: catalogue.hash().into(),
            map_type: MapType::Mixed,
            size,
            entries_per_side: 1,
            profile: contract::generation::GenerationProfile::Standard,
            region: None,
            limits: CompileLimits {
                max_authored_parts: 250_000,
                max_bay_positions: 600_000,
                max_ground_points: 200_000,
            },
        };
        let plan = match mapgen::generate_plan(
            &serde_json::to_string(&request).unwrap(),
            presets_text,
            templates,
            &rules,
        ) {
            GenerateOutcome::Ok { plan } => plan,
            GenerateOutcome::Error { diagnostics } => {
                panic!("{size:?} seed {seed}: {diagnostics:?}")
            }
        };
        let country = mapgen::open_country::measure(&plan, &presets);
        assert!(
            country.homes.fair,
            "{size:?} seed {seed}: {:?}",
            country.homes
        );
    }
}
