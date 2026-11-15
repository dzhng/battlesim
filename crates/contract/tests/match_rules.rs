use contract::skirmish::MatchRules;
use serde_json::json;

#[test]
fn legacy_match_rules_keep_their_serialized_identity_and_accept_an_explicit_default() {
    let base: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/skirmish.json")).unwrap();
    let legacy: MatchRules = serde_json::from_value(base.clone()).unwrap();
    let mut explicit = base.clone();
    explicit["all_objectives_victory"] = json!(true);
    let explicit: MatchRules = serde_json::from_value(explicit).unwrap();
    assert!(legacy.all_objectives_victory);
    let serialized = serde_json::to_value(legacy).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().keys().collect::<Vec<_>>(),
        base.as_object().unwrap().keys().collect::<Vec<_>>()
    );
    assert_eq!(serde_json::to_value(explicit).unwrap(), serialized);
}
