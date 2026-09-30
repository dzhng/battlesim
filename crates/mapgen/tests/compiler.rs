use contract::templates::{BuildingTemplateDescriptor, TemplateGeometryCatalog};
use mapgen::{lower, CompileRequest};
use serde_json::{json, Value};

fn catalogue() -> TemplateGeometryCatalog {
    TemplateGeometryCatalog::new(vec![serde_json::from_str::<BuildingTemplateDescriptor>(
        include_str!("../../../specs/city-maps/assets/template-geometry/asymmetric.json"),
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
            "limits":{"max_authored_parts":3,"max_bay_positions":32},
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
        "../../../specs/city-maps/assets/template-geometry/native-asymmetric.json"
    ))
    .unwrap();
    #[derive(serde::Deserialize)]
    struct RawOriginal {
        materialized: Box<serde_json::value::RawValue>,
    }
    let raw: RawOriginal = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/template-geometry/native-asymmetric.json"
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
    for field in ["land_regions", "rivers", "forests"] {
        input = request();
        input
            .plan
            .unsupported_fields
            .insert(field.into(), json!([]));
        let error = lower(&input, &catalogue()).unwrap_err();
        assert_eq!(
            error,
            vec![mapgen::Diagnostic {
                code: mapgen::DiagnosticCode::UnsupportedPlanField,
                feature: None,
                location: format!("$.plan.{field}"),
                message: format!("{field} has no admitted compiler geometry owner in this pass"),
            }]
        );
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
        "../../../specs/city-maps/assets/map-compiler/request.json"
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
    let identity: contract::identity::GenerationIdentity =
        serde_json::from_str(&std::fs::read_to_string(output_path.join("SOURCES.json")).unwrap())
            .unwrap();
    assert_eq!(identity, expected.identity);
    assert_eq!(
        contract::identity::json_hash(&map).unwrap(),
        identity.map_hash
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
    #[derive(serde::Deserialize)]
    struct Corpus {
        cases: Vec<Record>,
    }
    #[derive(serde::Deserialize)]
    struct Record {
        name: String,
        request_json: String,
        native_outcome: String,
        exit_code: i32,
    }
    let corpus: Corpus = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/map-compiler/paired-records.json"
    ))
    .unwrap();
    let directory =
        std::env::temp_dir().join(format!("mapgen-replay-proof-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let request_path = directory.join("request.json");
    let catalogue_path = directory.join("catalogue.json");
    std::fs::write(
        &catalogue_path,
        format!(
            "[{}]",
            include_str!("../../../specs/city-maps/assets/template-geometry/asymmetric.json")
        ),
    )
    .unwrap();
    for record in corpus.cases {
        std::fs::write(&request_path, &record.request_json).unwrap();
        let process = std::process::Command::new(env!("CARGO_BIN_EXE_mapgen"))
            .arg("lower")
            .arg(&request_path)
            .arg(&catalogue_path)
            .output()
            .unwrap();
        assert_eq!(
            process.status.code(),
            Some(record.exit_code),
            "{}",
            record.name
        );
        assert_eq!(
            String::from_utf8(process.stdout).unwrap(),
            record.native_outcome + "\n",
            "{}",
            record.name
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
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

#[test]
fn the_shared_surface_schema_accepts_empty_layers_but_refuses_unadmitted_geometry() {
    let source = serde_json::to_string(&request()).unwrap();
    let empty = source.replace("\"size\":", "\"surfaces\":[],\"size\":");
    let input: CompileRequest = serde_json::from_str(&empty).unwrap();
    assert_eq!(
        lower(&input, &catalogue()).unwrap().identity,
        lower(&request(), &catalogue()).unwrap().identity
    );
    let nonempty = source.replace("\"size\":",r#""surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[0,0],[10,10]],"width_m":4}}],"size":"#);
    let input: CompileRequest = serde_json::from_str(&nonempty).unwrap();
    assert_eq!(
        lower(&input, &catalogue()).unwrap_err(),
        vec![mapgen::Diagnostic {
            code: mapgen::DiagnosticCode::UnsupportedPlanField,
            feature: None,
            location: "$.plan.surfaces".into(),
            message: "surfaces require shared contract geometry admission in this pass".into(),
        }]
    );
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
