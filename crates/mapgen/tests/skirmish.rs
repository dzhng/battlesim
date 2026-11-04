#[path = "common/admitted.rs"]
mod admitted;

use contract::generation::GenerationRequest;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");

fn presets() -> mapgen::layout::PresetDefinitions {
    mapgen::layout::PresetDefinitions::from_json(PRESETS).unwrap()
}

fn test_players() -> Vec<contract::players::Player> {
    serde_json::from_value(serde_json::json!([
        {"id":"blue-1","side":"blue","faction":"us","controller":"active_ai"},
        {"id":"red-1","side":"red","faction":"eastern","controller":"active_ai"}
    ]))
    .unwrap()
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

fn request_with_entries(
    map_type: &str,
    seed: u64,
    template_catalog_hash: &str,
    entries_per_side: usize,
) -> serde_json::Value {
    let mut request = request(map_type, seed, template_catalog_hash);
    request["entries_per_side"] = serde_json::json!(entries_per_side);
    request
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
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
    let reserved = result.sites.skirmish.as_ref().unwrap();
    assert!(reserved.all_reserved_objectives().all(|o| !result
        .map
        .forests
        .iter()
        .any(|f| f.shape.contains(o.center, o.radius_m))));
    let admitted = sim::map_analysis::admit_skirmish(
        &prepared.queries(&result.map, &result.sites),
        &rules,
        &test_players(),
    )
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

#[test]
fn multi_entry_generation_lays_spaced_entries_on_joined_arms() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let (_, result) = admitted::admitted(1.., 1, |seed| {
        let mut request = request_with_entries("open", seed, catalogue.hash(), 3);
        request["size"] = serde_json::json!("large");
        match mapgen::generate_map(
            &request.to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        ) {
            mapgen::CompileOutcome::Ok { result } => Ok(*result),
            mapgen::CompileOutcome::Error { diagnostics } => Err(diagnostics),
        }
    })[0]
        .clone();
    let entries = &result.sites.skirmish.as_ref().unwrap().entries;
    assert_eq!(entries.len(), 6);
    for index in 1..3 {
        let blue = entries[index].center[0] / result.map.size[0];
        let red = entries[index + 3].center[0] / result.map.size[0];
        assert!((blue - red).abs() < 1e-12, "arm {index} was not mirrored");
    }
    for (i, a) in entries.iter().enumerate() {
        for b in &entries[..i] {
            if a.side == b.side {
                assert!(libm::hypot(a.center[0] - b.center[0], a.center[1] - b.center[1]) >= 150.0);
            }
        }
    }
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
    let admission = sim::map_analysis::admit_skirmish(
        &prepared.queries(&result.map, &result.sites),
        &rules,
        &test_players(),
    );
    assert!(
        admission.is_ok(),
        "multi-entry map was not admissible: {admission:?}"
    );
}

#[test]
fn three_entry_maps_put_flanks_on_opposite_sides_of_the_centre_exit() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json")).unwrap();
    let generated_rows: Vec<_> = corpus["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| {
            let map = row["map"].as_object()?;
            let map_type = map["type"].as_str()?;
            let size = map["size"].as_str()?;
            (matches!(size, "large" | "xl")).then_some((map_type, size, row["seed"].as_u64()?))
        })
        .collect();
    let cases = generated_rows
        .into_iter()
        .chain(["large", "xl"].into_iter().flat_map(|size| {
            [
                ("open", size, 1_u64),
                ("mixed", size, 2),
                ("metro", size, 3),
            ]
        }));
    for (map_type, size, seed) in cases {
        let mut request = request_with_entries(map_type, seed, catalogue.hash(), 3);
        request["size"] = serde_json::json!(size);
        let mapgen::CompileOutcome::Ok { result } = mapgen::generate_map(
            &request.to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        ) else {
            continue;
        };
        let entries = &result.sites.skirmish.as_ref().unwrap().entries;
        for side in [contract::ids::Side::Blue, contract::ids::Side::Red] {
            let side_entries: Vec<_> = entries.iter().filter(|entry| entry.side == side).collect();
            assert_eq!(side_entries.len(), 3, "{map_type} {size} seed {seed}");
            let centre = side_entries[0];
            let flank_sides: Vec<_> = side_entries
                .iter()
                .skip(1)
                .map(|entry| entry.center[0].total_cmp(&centre.center[0]))
                .collect();
            assert!(
                flank_sides.iter().any(|order| order.is_lt())
                    && flank_sides.iter().any(|order| order.is_gt()),
                "{map_type} {size} seed {seed} {side:?}: flanks {:?} around centre x={:.1}",
                side_entries
                    .iter()
                    .map(|entry| entry.center[0])
                    .collect::<Vec<_>>(),
                centre.center[0]
            );
        }
    }
}

#[test]
fn every_skirmish_profile_accepts_three_entries_without_river_mouths() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    for map_type in ["open", "mixed", "metro"] {
        for size in ["large", "xl"] {
            let mut request = request_with_entries(map_type, 1, catalogue.hash(), 3);
            request["size"] = serde_json::json!(size);
            let generated = admitted::admitted(1.., 1, |seed| {
                request["seed"] = serde_json::json!(seed.to_string());
                match mapgen::generate_map(
                    &request.to_string(),
                    PRESETS,
                    TEMPLATES,
                    include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
                ) {
                    mapgen::CompileOutcome::Ok { result } => Ok(*result),
                    mapgen::CompileOutcome::Error { diagnostics } => Err(diagnostics),
                }
            });
            let [(seed, result)] = generated.as_slice() else {
                unreachable!()
            };
            let entries = &result.sites.skirmish.as_ref().unwrap().entries;
            assert_eq!(entries.len(), 6, "{map_type} {size}, seed {seed}");
            for side in [contract::ids::Side::Blue, contract::ids::Side::Red] {
                let side_entries: Vec<_> = entries.iter().filter(|e| e.side == side).collect();
                for (i, a) in side_entries.iter().enumerate() {
                    for b in &side_entries[..i] {
                        assert!(
                            libm::hypot(a.center[0] - b.center[0], a.center[1] - b.center[1])
                                >= 150.0
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn entry_count_size_floor_refuses_small_and_medium_three_entry_requests() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    for size in ["small", "medium"] {
        let request = request_with_entries("open", 1, catalogue.hash(), 3);
        let mut request = request;
        request["size"] = serde_json::json!(size);
        let mapgen::CompileOutcome::Error { diagnostics } = mapgen::generate_map(
            &request.to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        ) else {
            panic!("{size} accepted a three-entry request");
        };
        assert!(diagnostics.iter().any(|d| d.location == "$.size"));
    }
}

#[test]
fn explicit_one_entry_keeps_the_legacy_map_hash() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    for map_type in ["open", "mixed", "metro"] {
        let legacy = mapgen::generate_map(
            &request(map_type, 1, catalogue.hash()).to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        );
        let explicit = mapgen::generate_map(
            &request_with_entries(map_type, 1, catalogue.hash(), 1).to_string(),
            PRESETS,
            TEMPLATES,
            include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
        );
        let (mapgen::CompileOutcome::Ok { result: a }, mapgen::CompileOutcome::Ok { result: b }) =
            (legacy, explicit)
        else {
            continue;
        };
        assert_eq!(a.identity.map_hash, b.identity.map_hash, "{map_type}");
    }
}

#[test]
fn one_entry_hashes_are_unchanged_for_the_skirmish_corpus_generated_rows() {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/skirmish-corpus.json")).unwrap();
    for row in corpus["rows"].as_array().unwrap() {
        let Some(map) = row["map"].as_object() else {
            continue;
        };
        let map_type = map["type"].as_str().unwrap();
        let size = map["size"].as_str().unwrap();
        let seed = row["seed"].as_u64().unwrap();
        let mut implicit = request(map_type, seed, catalogue.hash());
        implicit["size"] = serde_json::json!(size);
        let explicit = request_with_entries(map_type, seed, catalogue.hash(), 1);
        let explicit = {
            let mut request = explicit;
            request["size"] = serde_json::json!(size);
            request
        };
        let generated = |request: serde_json::Value| {
            mapgen::generate_map(
                &request.to_string(),
                PRESETS,
                TEMPLATES,
                include_str!("../../../fixtures/parity/map-layout/physical-rules.json"),
            )
        };
        let (
            mapgen::CompileOutcome::Ok { result: left },
            mapgen::CompileOutcome::Ok { result: right },
        ) = (generated(implicit), generated(explicit))
        else {
            continue;
        };
        assert_eq!(
            left.identity.map_hash, right.identity.map_hash,
            "{map_type} {size} seed {seed}"
        );
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
