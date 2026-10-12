use contract::bundle::{
    base64_encode, BundleAsset, BundleMapIdentity, BundleMode, ControllerKind, PlayerSlot,
    ScenarioBundle, VictoryDefinition, FORMAT_VERSION,
};
use contract::ids::Side;
use contract::scenario::ScenarioDefinition;

fn bundle() -> ScenarioBundle {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let scenario: ScenarioDefinition = serde_json::from_value(fixture["scenario"].clone()).unwrap();
    let mut scenario = scenario;
    scenario.skirmish = None;
    let map_hash = contract::identity::json_hash(&scenario.map).unwrap();
    let rules_hash = contract::identity::json_hash(&scenario.rules).unwrap();
    let bytes = b"RIFF";
    ScenarioBundle {
        format_version: FORMAT_VERSION.into(),
        runtime_build: "test-build".into(),
        map: BundleMapIdentity {
            map_hash,
            template_catalog_hash: None,
        },
        rules_hash,
        scenario,
        slots: vec![PlayerSlot {
            id: "blue-1".into(),
            side: Side::Blue,
            faction: None,
            controller: ControllerKind::Human,
            units: Vec::new(),
        }],
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
fn bundle_round_trips_canonical_bytes_and_identity() {
    let bundle = bundle();
    let bytes = bundle.canonical_bytes().unwrap();
    let loaded = ScenarioBundle::from_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(loaded.canonical_bytes().unwrap(), bytes);
    assert_eq!(
        loaded.identity().unwrap().sha256,
        contract::identity::bytes_hash(&bytes)
    );
}

#[test]
fn bundle_refuses_changed_embedded_bytes_and_unknown_fields() {
    let bundle = bundle();
    let mut value: serde_json::Value =
        serde_json::from_slice(&bundle.canonical_bytes().unwrap()).unwrap();
    value["assets"][0]["bytes_base64"] = serde_json::Value::String(base64_encode(b"RIFF-corrupt"));
    let error = ScenarioBundle::from_json(&serde_json::to_string(&value).unwrap()).unwrap_err();
    assert!(error.to_string().contains("do not match sha256"));
    value["unexpected"] = serde_json::json!(true);
    let error = ScenarioBundle::from_json(&serde_json::to_string(&value).unwrap()).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn bundle_refuses_noncanonical_id_order_and_multiple_humans() {
    let mut bundle = bundle();
    bundle.slots.push(PlayerSlot {
        id: "alpha".into(),
        side: Side::Red,
        faction: None,
        controller: ControllerKind::Human,
        units: Vec::new(),
    });
    let error = bundle.validate().unwrap_err();
    assert!(
        error.to_string().contains("strictly sorted")
            || error.to_string().contains("at most one human")
    );
}
