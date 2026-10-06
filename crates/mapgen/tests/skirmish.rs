use contract::generation::GenerationRequest;

#[test]
fn impossible_compact_road_budget_is_refused_at_preset_admission() {
    let mut presets: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/map-presets.json")).unwrap();
    presets["skirmish"]["main_road_length_factor"] = serde_json::json!(-1.0);
    let failure = mapgen::layout::PresetDefinitions::from_json(&presets.to_string()).unwrap_err();
    assert!(failure.iter().any(|d| d.location.contains("skirmish")));
}

#[test]
fn compact_profile_is_admitted_without_changing_standard_extent() {
    let presets = mapgen::layout::PresetDefinitions::from_json(include_str!(
        "../../../fixtures/map-presets.json"
    ))
    .unwrap();
    let request = serde_json::json!({
        "generator_version": mapgen::layout::GENERATOR_VERSION,
        "preset_revision": presets.revision,
        "seed": "1",
        "template_catalog_hash": "unused-layout-only",
        "type": "open", "size": "small", "profile": "skirmish",
        "limits": { "max_authored_parts": 1000000,
            "max_bay_positions": 100000000, "max_ground_points": 10000000 }
    });
    let compact: GenerationRequest = serde_json::from_value(request.clone()).unwrap();
    let plan = mapgen::layout::generate_layout(&compact, &presets).unwrap();
    assert_eq!(plan.size, [1800.0; 2]);
    let mut standard = request;
    standard["profile"] = serde_json::json!("standard");
    let standard: GenerationRequest = serde_json::from_value(standard).unwrap();
    let plan = mapgen::layout::generate_layout(&standard, &presets).unwrap();
    assert_eq!(plan.size, [4000.0; 2]);
}

#[test]
fn compact_profile_completes_physical_compilation() {
    let presets = include_str!("../../../fixtures/map-presets.json");
    let descriptors = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalogue = contract::templates::TemplateGeometryCatalog::new(
        serde_json::from_str(descriptors).unwrap(),
    )
    .unwrap();
    let definitions = mapgen::layout::PresetDefinitions::from_json(presets).unwrap();
    let request = serde_json::json!({
        "generator_version": mapgen::layout::GENERATOR_VERSION,
        "preset_revision": definitions.revision,
        "seed": "1", "template_catalog_hash": catalogue.hash(),
        "type": "open", "size": "small", "profile": "skirmish",
        "limits": { "max_authored_parts": 1000000,
            "max_bay_positions": 100000000, "max_ground_points": 10000000 }
    });
    let result = mapgen::generate_map(
        &request.to_string(),
        presets,
        descriptors,
        include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
    );
    let mapgen::CompileOutcome::Ok { result } = result else {
        panic!("compact physical compilation refused: {result:?}");
    };
    assert_eq!(result.map.size, [1800.0; 2]);
    let sites = serde_json::to_value(&result.sites).unwrap();
    let objectives = sites["skirmish"]["objectives"].as_array().unwrap();
    assert_eq!(objectives.len(), 3);
    assert_eq!(
        objectives
            .iter()
            .filter(|o| o["center"][1] == 900.0)
            .count(),
        1
    );
    assert_eq!(sites["skirmish"]["entries"].as_array().unwrap().len(), 2);
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::stand_in_game()).unwrap();
    let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
    let reserved = result.sites.skirmish.as_ref().unwrap();
    assert!(reserved.all_reserved_objectives().all(|o| !result
        .map
        .forests
        .iter()
        .any(|f| f.shape.contains(o.center, o.radius_m))));
    let admitted =
        sim::map_analysis::admit_skirmish(&prepared.queries(&result.map, &result.sites), &rules)
            .unwrap();
    let selected = admitted.sites.skirmish.unwrap();
    assert_eq!(selected.objectives.len(), 3);
    for (i, o) in selected.objectives.iter().enumerate() {
        assert!(selected.objectives[..i].iter().all(|other| libm::hypot(
            o.center[0] - other.center[0],
            o.center[1] - other.center[1]
        ) >= 150.0));
        assert!(admitted.journeys.iter().any(|j| j.objective == o.id));
    }
}
