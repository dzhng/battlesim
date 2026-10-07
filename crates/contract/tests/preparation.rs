use contract::catalog::Faction;
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
            "profile": "skirmish",
            "size": "small",
            "limits": { "max_authored_parts": 1, "max_bay_positions": 1, "max_ground_points": 1 },
        }},
        "factions": ["us", "eastern"],
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
    let MapSource::Generated { request: map } = &request.map_source;
    assert_eq!(map.seed.value(), u64::MAX);
    assert_eq!(request.factions, [Faction::Us, Faction::Eastern]);
    assert_eq!(request.battle_seed, BATTLE_SEED_MAX);
    // Across the JSON boundary the seeds come back as the text that went in.
    let outcome: Value =
        serde_json::from_str(&check_request_json(&generated().to_string())).unwrap();
    assert_eq!(outcome["status"], "ok");
    assert_eq!(outcome["request"], generated());
}

#[test]
fn a_request_names_both_factions_and_nothing_it_no_longer_reads() {
    // Nothing is defaulted: a battle without both factions is refused, and
    // the message names the field.
    let mut without = generated();
    without.as_object_mut().unwrap().remove("factions");
    let (location, message) = refusal(&without);
    assert_eq!(location, "$");
    assert!(message.contains("factions"), "{message}");
    for factions in [
        json!(["us"]),
        json!(["us", "eastern", "europe"]),
        json!(null),
    ] {
        let mut request = generated();
        request["factions"] = factions.clone();
        refusal(&request);
    }
    let mut unknown = generated();
    unknown["factions"] = json!(["us", "atlantis"]);
    let (_, message) = refusal(&unknown);
    assert!(message.contains("atlantis"), "{message}");
    // The recipe and the planner's seed left the request: an old request
    // naming them is refused by name, not silently ignored.
    for (field, value) in [
        ("recipe_id", json!("assault")),
        ("encounter_seed", json!("1")),
    ] {
        let mut old = generated();
        old[field] = value;
        let (_, message) = refusal(&old);
        assert!(message.contains(field), "{message}");
    }
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
            refusal(&with(&["map_source", "request", "seed"], seed.clone())).0,
            "$",
            "{seed}"
        );
    }
    let (_, message) = refusal(&with(&["map_source", "request", "seed"], json!("007")));
    assert!(message.contains("canonical u64 decimal"), "{message}");
    // The battle seed is a whole number a JavaScript number holds exactly.
    assert_eq!(
        refusal(&with(&["battle_seed"], json!(BATTLE_SEED_MAX + 1))).0,
        "$.battle_seed"
    );
    for seed in [json!(-1), json!(1.5), json!("1")] {
        assert_eq!(refusal(&with(&["battle_seed"], seed)).0, "$");
    }
    // A battle is fought on a generated map: a saved map of the catalogue (a
    // test's or the menu's) is no source a battle can name.
    let (_, message) = refusal(&with(
        &["map_source"],
        json!({ "kind": "catalogue", "id": "geometry" }),
    ));
    assert!(message.contains("unknown variant `catalogue`"), "{message}");
    // Only the generator is a source, and a request holds nothing else.
    refusal(&with(&["map_source"], json!({ "kind": "saved" })));
    refusal(&with(&["map_source", "request", "type"], json!("huge")));
    refusal(&with(&["fallback_seed"], json!("1")));
}
