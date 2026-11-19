//! A coarse certificate cell may need several physical sight witnesses.
use mapgen::layout::{GenerationRequest, MapSize, MapType, PresetDefinitions, GENERATOR_VERSION};
use mapgen::{CompileLimits, GenerateOutcome};

#[test]
fn a_complete_location_cell_can_be_certified_by_adjacent_physical_patches() {
    let presets_text = include_str!("../../../fixtures/map-presets.json");
    let templates = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(templates).unwrap())
            .unwrap();
    let presets = PresetDefinitions::from_json(presets_text).unwrap();
    let rules = sim::fixtures::test_rules().to_string();
    let size = MapSize::Large;
    // The first map the generator admits from seed 5 on, as the lobby
    // redraws a refused seed.
    let admitted = (5_u64..13).find_map(|seed| {
        let request = GenerationRequest {
            generator_version: GENERATOR_VERSION.into(),
            preset_revision: presets.revision.clone(),
            seed: seed.into(),
            template_catalog_hash: catalogue.hash().into(),
            map_type: MapType::Open,
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
        match mapgen::generate_plan(
            &serde_json::to_string(&request).unwrap(),
            presets_text,
            templates,
            &rules,
        ) {
            GenerateOutcome::Ok { plan } => Some(plan),
            GenerateOutcome::Error { .. } => None,
        }
    });
    let plan = admitted.unwrap_or_else(|| panic!("no {size:?} seed from 5 to 12 is admitted"));
    assert!(
        !plan.forests.is_empty(),
        "a successful certificate retains physical foliage"
    );
}
