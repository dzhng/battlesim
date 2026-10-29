use serde_json::json;

#[test]
fn every_prop_authors_its_own_stand_in_tint() {
    let documents = [json!({ "props": { "test_prop": {
        "body": { "blocks": { "infantry": false, "vehicle": false },
            "stops_rounds": false, "occludes": false, "weight_class": "light" },
        "appearance": { "drawn_by": "test_scenery" }
    }}})];
    let error = contract::catalog::resolve(&documents)
        .expect_err("a prop must author its stand-in colour")
        .to_string();
    assert!(
        error.contains("test_prop") && error.contains("stand_in_tint"),
        "{error}"
    );
}
