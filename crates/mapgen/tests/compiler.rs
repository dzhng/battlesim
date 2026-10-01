use contract::templates::{BuildingTemplateDescriptor, TemplateGeometryCatalog};
use mapgen::{lower, CompileRequest};
use serde_json::{json, Value};

fn catalogue() -> TemplateGeometryCatalog {
    TemplateGeometryCatalog::new(vec![serde_json::from_str::<BuildingTemplateDescriptor>(
        include_str!("../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap()])
    .unwrap()
}
fn request() -> CompileRequest {
    let catalogue = catalogue();
    serde_json::from_str(
        &json!({
            "generator_version":"labelled-api-v1", "preset_revision":"physical-proof-v1",
            "seed":"18446744073709551615", "template_catalog_hash":catalogue.hash(),
            "limits":{"max_authored_parts":3,"max_bay_positions":32,"max_ground_points":4096},
            "plan":{"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
                "props":[{"id":2,"kind":"tooth","center":[100,100],"yaw":0,"half_extents":[0.6,0.6,0.6]}],
                "buildings":[{"id":"compound","template_id":"asymmetric-api-compound","kind":"building","owner":0,
                    "parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],
                    "frame":{"translation":[10,20,5],"yaw":0.37}}]}
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn lowering_materializes_the_frozen_compound_into_one_global_authored_namespace() {
    let generated = lower(&request(), &catalogue()).unwrap();
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/native-asymmetric.json"
    ))
    .unwrap();
    #[derive(serde::Deserialize)]
    struct RawOriginal {
        materialized: Box<serde_json::value::RawValue>,
    }
    let raw: RawOriginal = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/native-asymmetric.json"
    ))
    .unwrap();
    // Add only C01's authoritative local intervals without re-parsing the old
    // physical float tokens through the ordinary serde JSON Value reader.
    let mut expected_json = raw.materialized.get().to_owned();
    for edge in &catalogue().templates()[0].edges {
        let id = format!("\"id\":{}", serde_json::to_string(&edge.id).unwrap());
        expected_json = expected_json.replacen(
            &id,
            &format!(
                "{id},\"span_m\":{}",
                serde_json::to_string(&edge.span_m).unwrap()
            ),
            1,
        );
    }
    let expected: contract::templates::MaterializedBuilding =
        serde_json::from_str(&expected_json).unwrap();
    assert_eq!(generated.map.buildings[0].geometry, expected);
    let parts = generated.map.authored_props().unwrap();
    assert_eq!(parts.iter().map(|p| p.0).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(parts[2].1.kind, "tooth");
    assert_eq!(parts[2].1.center, [100.0, 100.0]);
    assert_eq!(generated.map.buildings[0].owner, 0);
    assert_eq!(generated.identity.seed.value(), u64::MAX);
    assert_eq!(
        generated.identity.template_catalog_hash,
        original["catalogue"]["hash"]
    );
}

#[test]
fn admission_limits_refuse_the_named_physical_work_without_changing_map_identity() {
    let mut input = request();
    let baseline = lower(&input, &catalogue()).unwrap();
    input.limits.max_authored_parts = 2;
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::ComplexityLimit);
    assert_eq!(error[0].location, "$.limits.max_authored_parts");
    input.limits.max_authored_parts = 3;
    input.limits.max_bay_positions = baseline.report.bay_positions - 1;
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::ComplexityLimit);
    assert_eq!(error[0].location, "$.limits.max_bay_positions");
    input.limits.max_bay_positions = baseline.report.bay_positions;
    let exact = lower(&input, &catalogue()).unwrap();
    assert_eq!(
        serde_json::to_value(&exact.identity).unwrap(),
        serde_json::to_value(&baseline.identity).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&exact.map).unwrap(),
        serde_json::to_value(&baseline.map).unwrap()
    );
    assert_eq!(exact.report.authored_parts, 3);
    assert_eq!(exact.report.bay_positions, baseline.report.bay_positions);
}

#[test]
fn the_requested_catalogue_identity_is_verified_before_compiling() {
    let mut input = request();
    input.template_catalog_hash = "0".repeat(64);
    let errors = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(
        errors,
        vec![mapgen::Diagnostic {
            code: mapgen::DiagnosticCode::InvalidCatalogue,
            feature: None,
            location: "$.template_catalog_hash".into(),
            message: "requested physical catalogue hash differs from the supplied catalogue".into(),
        }]
    );
}

#[test]
fn the_plan_refuses_unowned_features_and_outside_physical_bounds_explicitly() {
    let mut input = request();
    input.plan.size = [0.0, 128.0];
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::InvalidBounds);
    assert_eq!(error[0].location, "$.plan.size");
    input = request();
    input.plan.buildings[0].frame.translation[0] = 1.0;
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::InvalidBounds);
    assert_eq!(error[0].feature.as_deref(), Some("compound"));
    input = request();
    input
        .plan
        .unsupported_fields
        .insert("land_regions".into(), json!([]));
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(
        error,
        vec![mapgen::Diagnostic {
            code: mapgen::DiagnosticCode::UnsupportedPlanField,
            feature: None,
            location: "$.plan.land_regions".into(),
            message: "land_regions has no admitted compiler geometry owner in this pass".into(),
        }]
    );
}

fn river_request(points: Value, surface_z: f64) -> CompileRequest {
    let mut input = serde_json::to_value(request()).unwrap();
    input["plan"]["rivers"] = json!([{ "points": points, "surface_z": surface_z }]);
    serde_json::from_value(input).unwrap()
}

#[test]
fn a_river_is_lowered_into_the_map_and_its_rounded_samples_are_counted() {
    let points = json!([
        {"xy":[0.0,64.0],"width_m":12.0,"depth_m":1.5},
        {"xy":[60.0,64.0],"width_m":14.0,"depth_m":1.75},
        {"xy":[128.0,110.0],"width_m":16.0,"depth_m":2.0}
    ]);
    let input = river_request(points.clone(), -0.5);
    let generated = lower(&input, &catalogue()).unwrap();
    assert_eq!(
        serde_json::to_value(&generated.map.rivers).unwrap(),
        json!([{ "points": points, "surface_z": -0.5 }])
    );
    // The bend is rounded: the samples every consumer walks, not the three
    // authored points, are what the allowance counts.
    let samples = generated.map.rivers[0].samples().len() as u64;
    assert!(samples > 10, "{samples} samples");
    assert_eq!(generated.report.ground_points, samples);
    let mut tight = input.clone();
    tight.limits.max_ground_points = samples - 1;
    let error = lower(&tight, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::ComplexityLimit);
    assert_eq!(error[0].location, "$.limits.max_ground_points");
    // The map loads as the world reads it.
    let saved = serde_json::to_string(&generated.map).unwrap();
    let loaded: contract::map::MapDefinition = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        loaded.rivers[0].samples(),
        generated.map.rivers[0].samples()
    );
}

#[test]
fn a_river_outside_the_map_or_one_the_terrain_cannot_carry_is_refused() {
    let point = |x: f64, y: f64, width: f64, depth: f64| json!({"xy":[x,y],"width_m":width,"depth_m":depth});
    for (points, surface_z, code, location) in [
        (
            json!([point(0.0, 64.0, 12.0, 1.5), point(128.5, 64.0, 12.0, 1.5)]),
            -0.5,
            mapgen::DiagnosticCode::InvalidBounds,
            "$.plan.rivers[0]",
        ),
        (
            json!([point(0.0, 64.0, 11.0, 1.5), point(128.0, 64.0, 12.0, 1.5)]),
            -0.5,
            mapgen::DiagnosticCode::InvalidRiver,
            "$.plan.rivers",
        ),
        (
            json!([point(0.0, 64.0, 12.0, 4.0), point(128.0, 64.0, 12.0, 1.5)]),
            -0.5,
            mapgen::DiagnosticCode::InvalidRiver,
            "$.plan.rivers",
        ),
        (
            json!([point(0.0, 64.0, 12.0, 1.5), point(128.0, 64.0, 12.0, 1.5)]),
            0.5,
            mapgen::DiagnosticCode::InvalidRiver,
            "$.plan.rivers",
        ),
    ] {
        let error = lower(&river_request(points, surface_z), &catalogue()).unwrap_err();
        assert_eq!(error[0].code, code, "{}", error[0].message);
        assert_eq!(error[0].location, location);
    }
}

#[test]
fn ordinary_physical_coordinates_retain_the_supplied_f64_token() {
    let source = serde_json::to_string(&request())
        .unwrap()
        .replace("100.0,100.0", "40.000000000168804,100.0");
    assert!(source.contains("40.000000000168804"));
    let input: CompileRequest = serde_json::from_str(&source).unwrap();
    let map = lower(&input, &catalogue()).unwrap().map;
    assert_eq!(map.props[0].center[0].to_bits(), 0x4044000000005ccd);
    let saved = serde_json::to_string(&map).unwrap();
    let loaded: contract::map::MapDefinition = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        loaded.props[0].center[0].to_bits(),
        map.props[0].center[0].to_bits()
    );
}

#[test]
fn playable_bounds_admit_the_architecture_envelope_and_refuse_larger_maps() {
    let mut input = request();
    input.plan.size = [20_000.0, 20_000.0];
    assert_eq!(
        lower(&input, &catalogue()).unwrap().map.size,
        input.plan.size
    );
    for size in [[20_000.001, 20_000.0], [20_000.0, 20_000.001]] {
        input.plan.size = size;
        assert_eq!(
            lower(&input, &catalogue()).unwrap_err(),
            vec![mapgen::Diagnostic {
                code: mapgen::DiagnosticCode::InvalidBounds,
                feature: None,
                location: "$.plan.size".into(),
                message: "playable map bounds exceed the 20000 metre architecture envelope".into(),
            }]
        );
    }
}

#[test]
fn building_feature_ids_identify_one_placement_in_diagnostics() {
    let mut input = request();
    input.plan.buildings.push(input.plan.buildings[0].clone());
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(
        error[0],
        mapgen::Diagnostic {
            code: mapgen::DiagnosticCode::InvalidRequest,
            feature: Some("compound".into()),
            location: "$.plan.buildings[1].id".into(),
            message: "building feature IDs must be nonempty and unique".into(),
        }
    );
    input.plan.buildings.truncate(1);
    input.plan.buildings[0].id.clear();
    assert_eq!(
        lower(&input, &catalogue()).unwrap_err()[0].location,
        "$.plan.buildings[0].id"
    );
}

#[test]
fn native_cli_saves_the_same_physical_map_and_lossless_identity() {
    let directory =
        std::env::temp_dir().join(format!("mapgen-native-proof-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let request_path = directory.join("request.json");
    let catalogue_path = directory.join("catalogue.json");
    let output_path = directory.join("saved");
    let input: CompileRequest = serde_json::from_str(include_str!(
        "../../../fixtures/parity/map-compiler/request.json"
    ))
    .unwrap();
    let expected = lower(&input, &catalogue()).unwrap();
    std::fs::write(&request_path, serde_json::to_string(&input).unwrap()).unwrap();
    std::fs::write(
        &catalogue_path,
        serde_json::to_string(catalogue().templates()).unwrap(),
    )
    .unwrap();
    let process = std::process::Command::new(env!("CARGO_BIN_EXE_mapgen"))
        .arg("lower")
        .arg(&request_path)
        .arg(&catalogue_path)
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        process.status.success(),
        "{}",
        String::from_utf8_lossy(&process.stderr)
    );
    #[derive(serde::Deserialize)]
    struct RawOutcome {
        status: String,
        result: Box<serde_json::value::RawValue>,
    }
    let result: RawOutcome = serde_json::from_slice(&process.stdout).unwrap();
    assert_eq!(result.status, "ok");
    assert_eq!(
        result.result.get(),
        serde_json::to_string(&expected).unwrap()
    );
    let saved = std::fs::read_to_string(output_path.join("map.json")).unwrap();
    let map: contract::map::MapDefinition = serde_json::from_str(&saved).unwrap();
    let sources = std::fs::read_to_string(output_path.join("SOURCES.json")).unwrap();
    let resolved = contract::maps::resolve(
        &saved,
        &sources,
        &catalogue().canonical_json().unwrap(),
        contract::maps::MapAdmission {
            max_authored_parts: input.limits.max_authored_parts,
            max_bay_positions: input.limits.max_bay_positions,
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&resolved.identity).unwrap(),
        json!({"kind":"generated", "generation":expected.identity}),
    );
    assert_eq!(
        serde_json::to_value(&resolved.definition).unwrap(),
        serde_json::to_value(&map).unwrap()
    );
    let sources: Value = serde_json::from_str(&sources).unwrap();
    assert_eq!(
        sources["inputs"],
        json!([
            {"kind":"supplied", "label":"request", "sha256":contract::identity::bytes_hash(&std::fs::read(&request_path).unwrap())},
            {"kind":"supplied", "label":"catalogue", "sha256":contract::identity::bytes_hash(&std::fs::read(&catalogue_path).unwrap())},
        ])
    );
    assert_eq!(saved, serde_json::to_string(&expected.map).unwrap());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn saved_map_scalars_preserve_the_compiled_physical_identity() {
    let mut input = request();
    input.plan.slope_cutoff_deg = 40.000000000168804;
    input.plan.fog_cell_m = 40.000000000168804;
    input.plan.height_grid_m = 40.000000000168804;
    input.plan.size[0] = 140.0000000001688;
    let result = lower(&input, &catalogue()).unwrap();
    let saved = serde_json::to_string(&result.map).unwrap();
    let reloaded: contract::map::MapDefinition = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        reloaded.slope_cutoff_deg.to_bits(),
        input.plan.slope_cutoff_deg.to_bits()
    );
    assert_eq!(
        reloaded.fog_cell_m.to_bits(),
        input.plan.fog_cell_m.to_bits()
    );
    assert_eq!(
        reloaded.height_grid_m.to_bits(),
        input.plan.height_grid_m.to_bits()
    );
    assert_eq!(reloaded.size, input.plan.size);
    assert_eq!(
        contract::identity::json_hash(&reloaded).unwrap(),
        result.identity.map_hash
    );
}

#[test]
fn native_cli_replays_the_frozen_acceptance_and_refusal_records() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Corpus {
        cases: Vec<Record>,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Record {
        name: String,
        request_json: String,
        native_outcome: String,
        exit_code: i32,
    }
    let mut corpus: Corpus = serde_json::from_str(include_str!(
        "../../../fixtures/parity/map-compiler/paired-records.json"
    ))
    .unwrap();
    // `BLESS_PARITY=1` rewrites the native half for a named change; the web
    // test then holds Wasm to it.
    let bless = std::env::var_os("BLESS_PARITY").is_some();
    let directory =
        std::env::temp_dir().join(format!("mapgen-replay-proof-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let request_path = directory.join("request.json");
    let catalogue_path = directory.join("catalogue.json");
    std::fs::write(
        &catalogue_path,
        format!(
            "[{}]",
            include_str!("../../../fixtures/parity/templates/asymmetric.json")
        ),
    )
    .unwrap();
    for record in &mut corpus.cases {
        std::fs::write(&request_path, &record.request_json).unwrap();
        let process = std::process::Command::new(env!("CARGO_BIN_EXE_mapgen"))
            .arg("lower")
            .arg(&request_path)
            .arg(&catalogue_path)
            .output()
            .unwrap();
        let stdout = String::from_utf8(process.stdout).unwrap();
        if bless {
            record.exit_code = process.status.code().unwrap();
            record.native_outcome = stdout.trim_end_matches('\n').into();
            continue;
        }
        assert_eq!(
            process.status.code(),
            Some(record.exit_code),
            "{}",
            record.name
        );
        assert_eq!(
            stdout,
            record.native_outcome.clone() + "\n",
            "{}",
            record.name
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
    if bless {
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/parity/map-compiler/paired-records.json"
        );
        std::fs::write(file, serde_json::to_string_pretty(&corpus).unwrap() + "\n").unwrap();
    }
}

#[test]
fn generation_identity_requires_named_version_identifiers() {
    for field in ["generator_version", "preset_revision"] {
        let mut input = request();
        match field {
            "generator_version" => input.generator_version.clear(),
            _ => input.preset_revision = "  ".into(),
        }
        assert_eq!(
            lower(&input, &catalogue()).unwrap_err(),
            vec![mapgen::Diagnostic {
                code: mapgen::DiagnosticCode::InvalidRequest,
                feature: None,
                location: format!("$.{field}"),
                message: "generation version identifiers must be nonempty".into(),
            }]
        );
    }
}

fn ground_request(surfaces: Value, forests: Value) -> CompileRequest {
    let mut source = serde_json::to_value(request()).unwrap();
    source["plan"]["surfaces"] = surfaces;
    source["plan"]["forests"] = forests;
    serde_json::from_value(source).unwrap()
}

fn road(points: Value) -> Value {
    json!({"kind":"country_road","shape":{"kind":"stroke","points":points,"width_m":8}})
}

fn wood(ring: Value) -> Value {
    json!({"shape":{"kind":"polygon","ring":ring}})
}

#[test]
fn empty_ground_layers_leave_the_map_identity_unchanged() {
    let input = ground_request(json!([]), json!([]));
    assert_eq!(
        lower(&input, &catalogue()).unwrap().identity,
        lower(&request(), &catalogue()).unwrap().identity
    );
}

/// C04: a plan's roads and forests become the map's, in the contract's own
/// shapes, so the battle and the renderer never reinterpret the plan.
#[test]
fn roads_and_forests_lower_into_the_map_and_its_identity() {
    let input = ground_request(
        json!([road(json!([[0, 60], [60, 60], [60, 128]]))]),
        json!([wood(json!([[80, 10], [120, 10], [120, 40], [80, 40]]))]),
    );
    let generated = lower(&input, &catalogue()).unwrap();
    assert_eq!(
        serde_json::to_value(&generated.map.surfaces).unwrap(),
        serde_json::to_value(&input.plan.surfaces).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&generated.map.forests).unwrap(),
        serde_json::to_value(&input.plan.forests).unwrap()
    );
    // The road's rounded bend counts, not just its three authored points.
    assert!(
        generated.report.ground_points > 3 + 4,
        "{}",
        generated.report.ground_points
    );
    let bare = lower(&request(), &catalogue()).unwrap();
    assert_ne!(generated.identity.map_hash, bare.identity.map_hash);
    assert_ne!(generated.identity.config_hash, bare.identity.config_hash);
    // The compiled map loads as a battle map: same schema, nothing extra.
    let saved = serde_json::to_string(&generated.map).unwrap();
    serde_json::from_str::<contract::map::MapDefinition>(&saved).unwrap();
}

#[test]
fn a_ground_shape_with_a_point_outside_the_map_is_refused_by_name() {
    for (input, location) in [
        (
            ground_request(json!([road(json!([[0, 60], [200, 60]]))]), json!([])),
            "$.plan.surfaces[0]",
        ),
        (
            ground_request(
                json!([road(json!([[0, 60], [100, 60]]))]),
                json!([wood(json!([[80, -1], [120, 10], [120, 40]]))]),
            ),
            "$.plan.forests[0]",
        ),
    ] {
        let error = lower(&input, &catalogue()).unwrap_err();
        assert_eq!(error.len(), 1, "{error:?}");
        assert_eq!(error[0].code, mapgen::DiagnosticCode::InvalidBounds);
        assert_eq!(error[0].location, location);
    }
}

#[test]
fn ground_beyond_the_callers_point_allowance_is_refused() {
    let mut input = ground_request(
        json!([road(json!([[0, 60], [60, 60], [60, 128]]))]),
        json!([]),
    );
    let needed = lower(&input, &catalogue()).unwrap().report.ground_points;
    input.limits.max_ground_points = needed - 1;
    let error = lower(&input, &catalogue()).unwrap_err();
    assert_eq!(error[0].code, mapgen::DiagnosticCode::ComplexityLimit);
    assert_eq!(error[0].location, "$.limits.max_ground_points");
    input.limits.max_ground_points = needed;
    assert!(lower(&input, &catalogue()).is_ok());
}

#[test]
fn an_implicit_ground_base_cannot_admit_an_infinite_box_height() {
    let mut input = request();
    input.plan.props[0].geometry.half_extents[2] = 1e308;
    assert_eq!(
        lower(&input, &catalogue()).unwrap_err(),
        vec![mapgen::Diagnostic {
            code: mapgen::DiagnosticCode::InvalidBounds,
            feature: None,
            location: "$.plan.authored_parts[2]".into(),
            message: "physical box requires finite positive extents and a finite pose".into(),
        }]
    );
}
