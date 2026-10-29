//! Generation consumes the physical rules of the battle it will prepare.
//! Input identity must change even when those inputs leave layout bytes alone.
#[path = "common/limits.rs"]
mod limits;
use contract::generation::{GenerationRequest, MapSize, MapType};
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
        entries_per_side: 1,
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: limits::game_limits(),
    })
    .unwrap();
    let mut rules = sim::fixtures::test_game();
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
fn physical_generation_refuses_invalid_forest_concealment() {
    let mut rules = sim::fixtures::test_game();
    // Generation and the battle share the forest concealment contract.
    rules["forests"]["rule"]["concealment"] = serde_json::json!(2.0);
    assert!(
        contract::generation_physics::GenerationPhysics::from_rules_json(&rules.to_string())
            .is_err(),
        "generation admitted forest concealment the battle refuses"
    );
}

#[test]
fn generation_refuses_circular_sight_that_cannot_be_represented() {
    let mut rules = sim::fixtures::test_game();
    let catalog: contract::catalog::Catalog =
        serde_json::from_value(rules["catalog"].clone()).unwrap();
    // Every test unit sees farther than finite geometry holds: the smallest
    // circular range is what generation covers.
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

/// An aircraft never drives a street, so no aircraft row of the catalog may
/// move what the generator builds. The test adds a fake aircraft far wider
/// than any ground hull (raising the airframe limits it sets for itself),
/// and the map must come out byte for byte the same; only its input
/// identity, which pins the whole catalog, may differ.
#[test]
fn an_aircraft_row_moves_nothing_on_the_ground() {
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
        map_type: MapType::Mixed,
        size: MapSize::Small,
        entries_per_side: 1,
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: limits::game_limits(),
    })
    .unwrap();
    let build = |rules: &serde_json::Value| match generate_map(
        &request,
        presets,
        descriptors,
        &rules.to_string(),
    ) {
        CompileOutcome::Ok { result } => result,
        CompileOutcome::Error { diagnostics } => panic!("generation refused: {diagnostics:?}"),
    };
    // Airframes may be far wider than anything that drives.
    const WIDE_M: f64 = contract::HullLimits::SHIPPED.rotor.half_width_m;
    let mut rules = sim::fixtures::test_game();
    let before = build(&rules);
    let aircraft = rules["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|document| document["units"].as_object().into_iter().flatten())
        .find(|(_, unit)| unit["mobility"].get("air").is_some())
        .map(|(id, _)| id.clone())
        .expect("the test catalog has an aircraft");
    rules["catalog"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({ "units": {
        "test_wide_aircraft": {
            "extends": aircraft,
            "name": "Wide Test Aircraft",
            "body": { "hull": { "half_extents_m": [WIDE_M, WIDE_M, 2.0] } }
        }
    } }));
    let after = build(&rules);
    assert_ne!(
        before.identity.config_hash, after.identity.config_hash,
        "the fake aircraft did not reach the catalog"
    );
    assert!(
        serde_json::to_string(&before.map).unwrap() == serde_json::to_string(&after.map).unwrap(),
        "an aircraft row moved the generated map"
    );
    assert_eq!(before.identity.map_hash, after.identity.map_hash);
}
