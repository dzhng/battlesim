#[path = "common/admitted.rs"]
mod admitted;

use contract::generation::GenerationRequest;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");

fn presets() -> mapgen::layout::PresetDefinitions {
    mapgen::layout::PresetDefinitions::from_json(PRESETS).unwrap()
}

/// A Small skirmish request of `map_type` (`"open"`, `"mixed"` or `"metro"`).
fn request(map_type: &str, seed: u64, template_catalog_hash: &str) -> serde_json::Value {
    serde_json::json!({
        "generator_version": mapgen::layout::GENERATOR_VERSION,
        "preset_revision": presets().revision,
        "seed": seed.to_string(), "template_catalog_hash": template_catalog_hash,
        "type": map_type, "size": "small", "profile": "skirmish",
        "limits": { "max_authored_parts": 1000000,
            "max_bay_positions": 100000000, "max_ground_points": 10000000 }
    })
}

/// The first `count` Small skirmish maps of `map_type` that generate.
fn maps(map_type: &str, count: usize) -> Vec<(u64, mapgen::GeneratedMap)> {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    admitted::admitted(1.., count, |seed| {
        match mapgen::generate_map(
            &request(map_type, seed, catalogue.hash()).to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        ) {
            mapgen::CompileOutcome::Ok { result } => Ok(*result),
            mapgen::CompileOutcome::Error { diagnostics } => Err(diagnostics),
        }
    })
}

#[test]
fn an_impossible_skirmish_road_budget_is_refused_at_preset_admission() {
    let mut presets: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    presets["skirmish"]["main_road_length_factor"] = serde_json::json!(-1.0);
    let failure = mapgen::layout::PresetDefinitions::from_json(&presets.to_string()).unwrap_err();
    assert!(failure.iter().any(|d| d.location.contains("skirmish")));
}

#[test]
fn the_skirmish_profile_is_admitted_without_changing_the_standard_extent() {
    let presets = presets();
    let layout = |request: &serde_json::Value| {
        let request: GenerationRequest = serde_json::from_value(request.clone()).unwrap();
        mapgen::layout::generate_layout(&request, &presets.for_request(&request))
    };
    for (seed, plan) in admitted::admitted(1.., 1, |seed| {
        layout(&request("open", seed, "unused-layout-only"))
    }) {
        assert_eq!(plan.size, [1800.0; 2], "seed {seed}");
    }
    for (seed, plan) in admitted::admitted(1.., 1, |seed| {
        let mut standard = request("open", seed, "unused-layout-only");
        standard["profile"] = serde_json::json!("standard");
        layout(&standard)
    }) {
        assert_eq!(plan.size, [4000.0; 2], "seed {seed}");
    }
}

#[test]
fn the_skirmish_profile_completes_physical_compilation() {
    let [(_, result)] = &maps("open", 1)[..] else {
        unreachable!()
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
/// stand nearer than that, on every map tried.
#[test]
fn the_junction_square_keeps_its_town() {
    /// What every reserved objective keeps clear past the ground it clears.
    const FIELD_MARGIN_M: f64 = 10.0;
    for map_type in ["mixed", "metro"] {
        for (seed, result) in maps(map_type, 2) {
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
}
