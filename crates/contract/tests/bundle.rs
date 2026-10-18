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
    let scenario: ScenarioDefinition = serde_json::from_value(fixture["scenario"].clone()).unwrap();
    let mut scenario = scenario;
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
        runtime_build: "test-build".into(),
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
fn bundle_allows_multiple_human_capable_players() {
    let mut bundle = bundle();
    bundle.scenario.players.push(Player {
        id: "alpha".into(),
        side: Side::Red,
        human_capable: true,
        faction: Some(contract::catalog::Faction::Eastern),
        controller: ControllerKind::Human,
    });
    assert!(bundle.validate().is_ok());
    assert_eq!(
        bundle.effective_controllers(Some("blue-1")).unwrap(),
        vec![
            ("blue-1".to_string(), ControllerKind::Human),
            ("red-1".to_string(), ControllerKind::ActiveAi),
            ("alpha".to_string(), ControllerKind::PassiveAi),
        ]
    );
}

#[test]
fn bundle_refuses_invalid_executable_sources_before_play() {
    let mut bundle = bundle();
    bundle
        .authored_scripts
        .push(contract::bundle::AuthoredScript {
            id: "bad-script".into(),
            source: "while true { event(\"x\"); }".into(),
            side: Side::Blue,
            player: None,
        });
    let error = bundle.validate().unwrap_err();
    assert!(error.to_string().contains("unknown statement"));
}

#[test]
fn controller_requests_normalize_to_one_human_or_passive_ai() {
    let bundle = bundle();
    assert_eq!(
        bundle.effective_controllers(Some("blue-1")).unwrap(),
        vec![
            ("blue-1".to_string(), ControllerKind::Human),
            ("red-1".to_string(), ControllerKind::ActiveAi),
        ]
    );
}

#[test]
fn neutral_players_cannot_become_human_controllers() {
    let mut bundle = bundle();
    bundle.scenario.players[0].side = Side::Neutral;
    bundle.scenario.players[0].controller = ControllerKind::PassiveAi;
    bundle.scenario.players[0].human_capable = true;
    let error = bundle.validate().unwrap_err();
    assert!(error.to_string().contains("neutral"));
}

#[test]
fn bundle_keeps_authored_scripts_and_victory_in_one_envelope_owner() {
    let mut bundle = bundle();
    bundle.scenario.victory = Some("return true".into());
    let error = bundle.validate().unwrap_err();
    assert!(error
        .to_string()
        .contains("victory belongs in the bundle envelope"));
}

#[test]
fn scenario_projection_preserves_player_owner_without_changing_bundle_identity() {
    let bundle = bundle();
    let scenario: serde_json::Value =
        serde_json::from_str(&bundle.scenario_json().unwrap()).unwrap();
    assert_eq!(scenario["units"], serde_json::json!([]));
    let mut with_unit = bundle;
    with_unit
        .scenario
        .units
        .push(contract::scenario::UnitSetup {
            side: Side::Blue,
            owner: Some("blue-1".into()),
            kind: "test_rifle".into(),
            position: [20.0, 20.0],
            yaw: 0.0,
            engagement: None,
            condition: None,
            stock: None,
        });
    let projected: serde_json::Value =
        serde_json::from_str(&with_unit.scenario_json().unwrap()).unwrap();
    assert_eq!(projected["units"][0]["owner"], "blue-1");
}

#[test]
fn authoring_accepts_editor_unit_ids_but_canonical_uploads_do_not_carry_them() {
    let mut draft = serde_json::to_value(bundle()).unwrap();
    draft["scenario"]["units"] = serde_json::json!([{
        "id": 0,
        "side": "blue",
        "kind": "test_rifle",
        "position": [20.0, 20.0],
        "yaw": 0.0
    }]);
    draft["scenario"]["units"][0]["owner"] = serde_json::json!("blue-1");

    let authored = ScenarioBundle::from_authoring_json(&serde_json::to_string(&draft).unwrap())
        .expect("the editor id is an authoring-only field");
    let canonical: serde_json::Value = serde_json::from_str(&authored.to_json().unwrap()).unwrap();
    assert_eq!(canonical["scenario"]["units"][0]["kind"], "test_rifle");
    assert!(canonical["scenario"]["units"][0].get("id").is_none());
}

#[test]
fn economy_mode_keeps_the_existing_skirmish_owner_and_changes_identity() {
    let sandbox = bundle();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    let mut economy = sandbox.clone();
    economy.mode = BundleMode::Economy;
    economy.scenario.skirmish = serde_json::from_value(fixture["scenario"]["skirmish"].clone())
        .expect("the shipped skirmish setup is a valid economy owner");
    assert!(economy.validate().is_ok());
    assert_ne!(
        sandbox.identity().unwrap().sha256,
        economy.identity().unwrap().sha256
    );
    let projected: serde_json::Value =
        serde_json::from_str(&economy.scenario_json().unwrap()).unwrap();
    assert!(projected["skirmish"].is_object());
}

#[test]
fn economy_mode_refuses_authored_forces_that_the_skirmish_owner_cannot_field() {
    let mut economy = bundle();
    economy.mode = BundleMode::Economy;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    economy.scenario.skirmish =
        Some(serde_json::from_value(fixture["scenario"]["skirmish"].clone()).unwrap());
    economy.scenario.units.push(
        serde_json::from_value(serde_json::json!({
            "side": "blue", "kind": "test_rifle", "position": [20.0, 20.0]
        }))
        .unwrap(),
    );
    assert!(economy
        .validate()
        .unwrap_err()
        .to_string()
        .contains("authored forces"));
}

#[test]
fn an_asset_diagnostic_names_the_asset() {
    let mut bundle = bundle();
    bundle.assets[0].provenance = String::new();
    let contract::bundle::BundleError::Invalid(errors) = bundle.validate().unwrap_err();
    assert_eq!(
        errors
            .iter()
            .map(|e| e.location.as_str())
            .collect::<Vec<_>>(),
        ["$.assets[0]"]
    );
}

#[test]
fn a_sandbox_bundle_names_its_players_and_every_unit_an_owner() {
    let unit = |owner: Option<&str>| contract::scenario::UnitSetup {
        side: Side::Blue,
        owner: owner.map(Into::into),
        kind: "test_rifle".into(),
        position: [20.0, 20.0],
        yaw: 0.0,
        engagement: None,
        condition: None,
        stock: None,
    };
    let locations = |bundle: &ScenarioBundle| match bundle.validate() {
        Err(contract::bundle::BundleError::Invalid(errors)) => {
            errors.into_iter().map(|e| e.location).collect::<Vec<_>>()
        }
        Ok(()) => Vec::new(),
    };

    let mut unowned = bundle();
    unowned.scenario.players.clear();
    unowned.scenario.units.push(unit(None));
    assert_eq!(
        locations(&unowned),
        ["$.scenario.players", "$.scenario.units[0].owner"]
    );

    let mut owned = bundle();
    owned.scenario.units.push(unit(Some("blue-1")));
    assert!(locations(&owned).is_empty());
}
