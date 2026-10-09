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
    let plan = mapgen::layout::generate_layout(&compact, &presets.for_request(&compact)).unwrap();
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
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
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

/// The town round the primary junction is a town to fight through, not a
/// square hollowed out as a field objective is: a field keeps its whole
/// capture circle and a margin past it clear, and the junction's buildings
/// stand nearer than that, on every seed tried.
#[test]
fn the_junction_square_keeps_its_town() {
    /// What every reserved objective keeps clear past the ground it clears.
    const FIELD_MARGIN_M: f64 = 10.0;
    let presets = include_str!("../../../fixtures/map-presets.json");
    let descriptors = include_str!("../../../fixtures/prototype-building-templates.json");
    let catalogue = contract::templates::TemplateGeometryCatalog::new(
        serde_json::from_str(descriptors).unwrap(),
    )
    .unwrap();
    let definitions = mapgen::layout::PresetDefinitions::from_json(presets).unwrap();
    for (map_type, seed) in [("mixed", 1), ("mixed", 2), ("metro", 1), ("metro", 2)] {
        let request = serde_json::json!({
            "generator_version": mapgen::layout::GENERATOR_VERSION,
            "preset_revision": definitions.revision,
            "seed": seed.to_string(), "template_catalog_hash": catalogue.hash(),
            "type": map_type, "size": "small", "profile": "skirmish",
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
            panic!("{map_type} seed {seed} refused: {result:?}");
        };
        let sites = result.sites.skirmish.as_ref().unwrap();
        let junction = sites
            .objectives
            .iter()
            .find(|o| o.kind == contract::encounter::ObjectiveSiteKind::Junction)
            .unwrap();
        let nearest = result
            .map
            .buildings
            .iter()
            .flat_map(|building| &building.geometry.parts)
            .map(|part| {
                // From the junction to the nearest point of the part's box.
                let (sin, cos) = libm::sincos(part.yaw);
                let [dx, dy] = [
                    junction.center[0] - part.center[0],
                    junction.center[1] - part.center[1],
                ];
                let local = [cos * dx + sin * dy, -sin * dx + cos * dy];
                libm::hypot(
                    (local[0].abs() - part.half_extents[0]).max(0.0),
                    (local[1].abs() - part.half_extents[1]).max(0.0),
                )
            })
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest < junction.radius_m + FIELD_MARGIN_M,
            "{map_type} seed {seed}: the nearest building stands {nearest:.0} m from the junction"
        );
    }
}
