//! Generation consumes the physical rules of the battle it will prepare.
//! Input identity must change even when those inputs leave layout bytes alone.
use contract::generation::{CompileLimits, GenerationRequest, MapSize, MapType};
use contract::templates::TemplateGeometryCatalog;
use mapgen::{generate_map, CompileOutcome};

#[test]
fn physical_rules_are_explicit_generation_inputs_and_identity() {
    let presets = include_str!("../../../fixtures/map-presets.json");
    let descriptors = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalogue =
        TemplateGeometryCatalog::new(serde_json::from_str(descriptors).unwrap()).unwrap();
    let definitions = mapgen::layout::PresetDefinitions::from_json(presets).unwrap();
    let request = serde_json::to_string(&GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: definitions.revision,
        seed: 1.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type: MapType::Open,
        size: MapSize::Medium,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 60000,
            max_bay_positions: 600000,
            max_ground_points: 200000,
        },
    })
    .unwrap();
    let mut rules = sim::fixtures::game();
    let build = |rules: &serde_json::Value| match generate_map(
        &request,
        presets,
        descriptors,
        &rules.to_string(),
    ) {
        CompileOutcome::Ok { result } => result,
        CompileOutcome::Error { diagnostics } => {
            panic!("explicit physical rules were refused: {diagnostics:?}")
        }
    };
    let before = build(&rules);
    // Eye height is part of physical sight. This tiny change leaves the
    // construction geometry alone, but cannot leave its input identity alone.
    rules["physics"]["infantry_eye_m"] = serde_json::json!(1.61);
    let after = build(&rules);
    assert_eq!(before.identity.map_hash, after.identity.map_hash);
    assert_ne!(before.identity.config_hash, after.identity.config_hash);
}

#[test]
fn physical_generation_refuses_a_floor_the_world_cannot_build() {
    let mut rules = sim::fixtures::game();
    // A resolved input still needs the same physical floor contract as a
    // battle; silently admitting this density would make world construction fail.
    rules["forests"]["rule"]["logs_per_ha"] = serde_json::json!(101.0);
    assert!(
        contract::generation_physics::GenerationPhysics::from_rules_json(&rules.to_string())
            .is_err(),
        "generation admitted a forest floor the battle refuses"
    );
}

#[test]
fn generation_refuses_circular_sight_that_cannot_be_represented() {
    let mut rules = sim::fixtures::game();
    let catalog: contract::catalog::Catalog =
        serde_json::from_value(rules["catalog"].clone()).unwrap();
    let mut resolved = serde_json::to_value(catalog).unwrap();
    for unit in resolved[0]["units"].as_object_mut().unwrap().values_mut() {
        unit["sensors"]["ground_m"] = serde_json::json!(1e308);
        unit["sensors"]["sight_shape"] =
            serde_json::json!({"front":1e308,"side":1e308,"rear":1e308});
    }
    rules["catalog"] = resolved;
    assert!(
        contract::generation_physics::GenerationPhysics::from_rules_json(&rules.to_string())
            .is_err(),
        "generation admitted a circular range that overflows finite sight geometry"
    );
}
