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

#[test]
fn a_patch_changes_only_the_rules_it_names_and_refuses_what_rules_lack() {
    let base: MatchRules =
        serde_json::from_str(include_str!("../../../fixtures/skirmish.json")).unwrap();
    let patch = |value: serde_json::Value| base.patched(value.as_object().unwrap());
    let patched = patch(json!({"preparation_s": base.preparation_s + 60})).unwrap();
    assert_eq!(patched.preparation_s, base.preparation_s + 60);
    assert_eq!(
        patched.capture_s, base.capture_s,
        "an unnamed rule keeps its value"
    );
    assert!(patch(json!({"income": 5}))
        .unwrap_err()
        .starts_with("income"));
    assert!(patch(json!({"capture_s": "soon"})).is_err());
    // An optional rule may be patched in even where the base leaves it out.
    assert!(patch(json!({"all_objectives_victory": false})).is_ok());
}
