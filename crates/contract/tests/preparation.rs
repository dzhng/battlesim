use contract::maps::MapSource;
use contract::preparation::{check_request_json, PrepareBattleRequest, BATTLE_SEED_MAX};
use serde_json::{json, Value};

fn generated() -> Value {
    json!({
        "map_source": { "kind": "generated", "request": {
            "generator_version": "layout-5",
            "preset_revision": "layout-presets-6",
            "seed": "18446744073709551615",
            "template_catalog_hash": "ed9981b358116490fd50585874a615ac427af819628e097cc2bf4216cda4f2cb",
            "type": "mixed",
            "size": "small",
            "limits": { "max_authored_parts": 1, "max_bay_positions": 1, "max_ground_points": 1 },
        }},
        "recipe_id": "assault",
        "encounter_seed": "9007199254740993",
        "battle_seed": BATTLE_SEED_MAX,
    })
}

fn refusal(request: &Value) -> (String, String) {
    let outcome: Value = serde_json::from_str(&check_request_json(&request.to_string())).unwrap();
    assert_eq!(outcome["status"], "error", "{outcome}");
    let diagnostic = &outcome["diagnostics"][0];
    assert_eq!(diagnostic["code"], "invalid_request");
    (
        diagnostic["location"].as_str().unwrap().into(),
        diagnostic["message"].as_str().unwrap().into(),
    )
}

#[test]
fn a_request_keeps_seeds_a_javascript_number_cannot_hold() {
    let request = PrepareBattleRequest::from_json(&generated().to_string()).unwrap();
    let MapSource::Generated { request: map } = &request.map_source else {
        panic!("the request names a generated map");
    };
    assert_eq!(map.seed.value(), u64::MAX);
    assert_eq!(request.encounter_seed.value(), (1 << 53) + 1);
    assert_eq!(request.battle_seed, BATTLE_SEED_MAX);
    // Across the JSON boundary the seeds come back as the text that went in.
    let outcome: Value =
        serde_json::from_str(&check_request_json(&generated().to_string())).unwrap();
    assert_eq!(outcome["status"], "ok");
    assert_eq!(outcome["request"], generated());
}

#[test]
fn a_catalogue_request_names_a_saved_map_and_its_encounter() {
    let mut request = generated();
    request["map_source"] = json!({ "kind": "catalogue", "id": "village" });
    request["recipe_id"] = json!("lean");
    let request = PrepareBattleRequest::from_json(&request.to_string()).unwrap();
    assert!(matches!(request.map_source, MapSource::Catalogue { id } if id.as_str() == "village"));
}

#[test]
fn a_request_that_cannot_be_prepared_names_the_field_at_fault() {
    let with = |path: &[&str], value: Value| {
        let mut request = generated();
        let mut at = &mut request;
        for key in path {
            at = &mut at[*key];
        }
        *at = value;
        request
    };
    // A seed is canonical decimal text: a number may already be rounded.
    for seed in [
        json!(7),
        json!("007"),
        json!("-1"),
        json!("18446744073709551616"),
        json!(""),
    ] {
        assert_eq!(
            refusal(&with(&["encounter_seed"], seed.clone())).0,
            "$",
            "{seed}"
        );
        refusal(&with(&["map_source", "request", "seed"], seed.clone()));
    }
    let (_, message) = refusal(&with(&["encounter_seed"], json!("007")));
    assert!(message.contains("canonical u64 decimal"), "{message}");
    // The battle seed is a whole number a JavaScript number holds exactly.
    assert_eq!(
        refusal(&with(&["battle_seed"], json!(BATTLE_SEED_MAX + 1))).0,
        "$.battle_seed"
    );
    for seed in [json!(-1), json!(1.5), json!("1")] {
        assert_eq!(refusal(&with(&["battle_seed"], seed)).0, "$");
    }
    // The encounter and a catalogue map are addresses, never paths.
    assert_eq!(
        refusal(&with(&["recipe_id"], json!("../assault"))).0,
        "$.recipe_id"
    );
    let (_, message) = refusal(&with(
        &["map_source"],
        json!({ "kind": "catalogue", "id": "../village" }),
    ));
    assert!(message.contains("catalogue id"), "{message}");
    // Only the two sources exist, and a request holds nothing else.
    refusal(&with(&["map_source"], json!({ "kind": "village" })));
    refusal(&with(&["map_source", "request", "type"], json!("huge")));
    refusal(&with(&["fallback_seed"], json!("1")));
}
