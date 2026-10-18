use contract::bundle::{
    base64_encode, BundleAsset, BundleMapIdentity, BundleMode, ControllerKind, ScenarioBundle,
    VictoryDefinition, FORMAT_VERSION,
};
use contract::ids::Side;
use contract::players::Player;
use contract::scenario::ScenarioDefinition;

fn bundle() -> ScenarioBundle {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let mut scenario: ScenarioDefinition =
        serde_json::from_value(fixture["scenario"].clone()).unwrap();
    scenario.skirmish = None;
    scenario.players = vec![
        Player {
            id: "blue-1".into(),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            human_capable: true,
            controller: ControllerKind::Human,
        },
        Player {
            id: "red-1".into(),
            side: Side::Red,
            faction: Some(contract::catalog::Faction::Eastern),
            human_capable: false,
            controller: ControllerKind::ActiveAi,
        },
    ];
    let map_hash = contract::identity::json_hash(&scenario.map).unwrap();
    let rules_hash = contract::identity::json_hash(&scenario.rules).unwrap();
    let bytes = b"RIFF";
    ScenarioBundle {
        format_version: FORMAT_VERSION.into(),
        runtime_build: "wasm-test-build".into(),
        map: BundleMapIdentity {
            map_hash,
            template_catalog_hash: None,
        },
        rules_hash,
        scenario,
        mode: BundleMode::Sandbox,
        victory: VictoryDefinition {
            source: "return true".into(),
        },
        authored_scripts: Vec::new(),
        assets: vec![BundleAsset {
            id: "intro-audio".into(),
            media_type: "audio/wav".into(),
            sha256: contract::identity::bytes_hash(bytes),
            provenance: "user-upload".into(),
            bytes_base64: Some(base64_encode(bytes)),
            builtin: None,
        }],
    }
}

#[test]
fn wasm_boundary_agrees_with_native_canonical_identity() {
    let bundle = bundle();
    let canonical = bundle.to_json().unwrap();
    let native = bundle.identity().unwrap();
    let result: serde_json::Value =
        serde_json::from_str(&game_wasm::check_scenario_bundle(&canonical)).unwrap();
    assert_eq!(result["status"], "ok");
    assert_eq!(result["identity"]["sha256"], native.sha256);
    assert_eq!(
        game_wasm::canonical_scenario_bundle(&canonical).unwrap(),
        canonical
    );
}

#[test]
fn wasm_boundary_refuses_noncanonical_upload_before_preparation() {
    let bundle = bundle();
    let canonical = bundle.to_json().unwrap();
    let noncanonical = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&canonical).unwrap(),
    )
    .unwrap();
    let result: serde_json::Value =
        serde_json::from_str(&game_wasm::check_scenario_bundle(&noncanonical)).unwrap();
    assert_eq!(result["status"], "error");
    assert_eq!(result["diagnostics"][0]["code"], "non_canonical");
}

#[test]
fn bundle_details_uses_resolved_snapshot_without_generation() {
    let canonical = bundle().to_json().unwrap();
    let details: serde_json::Value =
        serde_json::from_str(&game_wasm::scenario_bundle_details(&canonical).unwrap()).unwrap();
    assert_eq!(
        details["identity"]["sha256"],
        bundle().identity().unwrap().sha256
    );
    assert_eq!(details["size"], serde_json::json!([800.0, 600.0]));
    assert!(details["scenario"].as_str().unwrap().contains("\"map\""));
    assert_eq!(details["assets"][0]["id"], "intro-audio");
}

#[test]
fn wasm_controller_admission_keeps_empty_human_slots_passive() {
    let canonical = bundle().to_json().unwrap();
    let controllers: Vec<(String, contract::bundle::ControllerKind)> =
        serde_json::from_str(&game_wasm::scenario_bundle_controllers(&canonical, None).unwrap())
            .unwrap();
    assert_eq!(
        controllers,
        vec![
            ("blue-1".into(), contract::bundle::ControllerKind::PassiveAi),
            ("red-1".into(), contract::bundle::ControllerKind::ActiveAi),
        ]
    );
}

#[test]
fn wasm_scenario_contract_accepts_multi_player_sides_without_panicking() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let mut scenario = fixture["scenario"].clone();
    let mut extra = scenario["players"][0].clone();
    extra["id"] = serde_json::json!("blue-2");
    scenario["players"].as_array_mut().unwrap().push(extra);
    scenario["skirmish"]["sites"]["entries"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"side":"blue","center":[200,10],"yaw":1.57}));
    scenario["skirmish"]["sites"]["entries"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"side":"red","center":[200,590],"yaw":-1.57}));
    let result: serde_json::Value = serde_json::from_str(
        &game_wasm::check_scenario_player_contract(&scenario.to_string()),
    )
    .unwrap();
    assert_eq!(result["status"], "ok");
}

/// Upload a canonical bundle edited by `edit`, with its content identities
/// recomputed so only the edit can be refused.
fn upload(edit: impl FnOnce(&mut ScenarioBundle)) -> serde_json::Value {
    let mut bundle = bundle();
    edit(&mut bundle);
    bundle.map.map_hash = contract::identity::json_hash(&bundle.scenario.map).unwrap();
    bundle.rules_hash = contract::identity::json_hash(&bundle.scenario.rules).unwrap();
    let bytes = serde_json::to_string(&bundle).unwrap();
    serde_json::from_str(&game_wasm::check_scenario_bundle(&bytes)).unwrap()
}

fn locations(result: &serde_json::Value) -> Vec<String> {
    assert_eq!(result["status"], "error", "{result}");
    result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["location"].as_str().unwrap().to_owned())
        .collect()
}

fn prop<T: serde::de::DeserializeOwned>(kind: &str) -> T {
    serde_json::from_value(serde_json::json!({
        "kind": kind,
        "center": [100.0, 100.0],
        "yaw": 0.0,
        "half_extents": [1.0, 1.0, 1.0]
    }))
    .unwrap()
}

#[test]
fn a_map_the_world_cannot_build_is_refused_with_diagnostics() {
    let unknown_prop = upload(|bundle| bundle.scenario.map.props.push(prop("no-such-prop")));
    assert_eq!(locations(&unknown_prop), ["$.scenario.map.props[0].kind"]);
    let no_fog_cells = upload(|bundle| bundle.scenario.map.fog_cell_m = 0.0);
    assert_eq!(locations(&no_fog_cells), ["$.scenario.map.fog_cell_m"]);
}
