//! What the presets and generation refuse: rows that cannot describe a
//! country, and inputs whose coverage could never be met or bounded.
use super::*;

/// A row the pass cannot place from is refused when the presets load.
#[test]
fn rows_that_cannot_describe_a_country_are_refused() {
    let refused = |pointer: &str, value: serde_json::Value| {
        let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
        *source.pointer_mut(pointer).unwrap() = value;
        PresetDefinitions::from_json(&source.to_string())
            .err()
            .map(|errors| errors[0].location.clone())
    };
    // A copse as large as a wood could not be told from one.
    assert_eq!(
        refused(
            "/open_country/copses/area_m2",
            serde_json::json!([300, 50_000])
        )
        .as_deref(),
        Some("$.presets.open_country.copses")
    );
    // A gap no wider than nothing is no gap.
    assert_eq!(
        refused("/open_country/tree_lines/gap_m", serde_json::json!(0)).as_deref(),
        Some("$.presets.open_country.tree_lines")
    );
    // A home of no building category is refused as a district's would be.
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["open_country"]["homesteads"]["groups"][0]["mix"] = serde_json::json!({ "castle": 1 });
    let errors = PresetDefinitions::from_json(&source.to_string()).unwrap_err();
    assert!(errors[0].message.contains("castle"), "{errors:?}");
}

fn coverage_refusal(presets: &str, rules: &serde_json::Value, reason: &str) {
    let request = serde_json::to_string(&request(MapType::Open, MapSize::Medium, 1)).unwrap();
    match mapgen::generate_map(&request, presets, TEMPLATES, &rules.to_string()) {
        mapgen::CompileOutcome::Error { diagnostics } => assert!(
            diagnostics
                .iter()
                .any(|d| d.code == mapgen::DiagnosticCode::GenerationFailed
                    && d.feature.as_deref() == Some("ground_sight")),
            "{reason} needs a named physical coverage refusal: {diagnostics:?}"
        ),
        mapgen::CompileOutcome::Ok { .. } => panic!("generation silently admitted {reason}"),
    }
}

#[test]
fn generation_refuses_foliage_below_every_ground_eye() {
    let mut rules = test_rules();
    rules["forests"]["rule"]["canopy_height_m"] = serde_json::json!(0.5);
    coverage_refusal(PRESETS, &rules, "foliage below every ground eye");
}

#[test]
fn generation_refuses_when_no_patch_has_legal_ground() {
    let mut presets: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    // A legal physical input can leave no place inside the playable
    // rectangle where the country placement rule permits a patch.
    presets["open_country"]["clear"]["edge_m"] = serde_json::json!(3100.0);
    coverage_refusal(
        &presets.to_string(),
        &test_rules(),
        "no legal patch placement",
    );
}

#[test]
fn generation_refuses_an_unbounded_fallback_tree_line() {
    let mut presets: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    // Reach the coverage fallback without any initial rural furnishing.
    for kind in ["tree_lines", "copses", "lone_trees", "field_cover"] {
        presets["open_country"][kind]["per_km2"] = serde_json::json!(0.);
    }
    presets["open_country"]["homesteads"]["per_road_km"] = serde_json::json!(0.);
    presets["open_country"]["tree_lines"]["width_m"] = serde_json::json!(1e-9);
    presets["open_country"]["sight"]["fill"] = serde_json::json!(["tree_line"]);
    coverage_refusal(
        &presets.to_string(),
        &test_rules(),
        "unbounded fallback tree-line sampling and grove allocation",
    );
}

#[test]
fn generation_refuses_an_unbounded_coverage_resolution() {
    let mut presets: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    presets["open_country"]["sight"]["cell_m"] = serde_json::json!(1e-9);
    coverage_refusal(
        &presets.to_string(),
        &test_rules(),
        "unbounded coverage grid and placement lattice",
    );
}
