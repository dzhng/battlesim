use contract::encounter::EncounterSites;
#[test]
fn duplicate_objective_identity_is_refused() {
    let sites: EncounterSites = serde_json::from_value(serde_json::json!({
        "settlements": [], "approaches": [], "skirmish": {
            "entries": [{"side":"blue","center":[500,990],"yaw":-1.57},
                {"side":"red","center":[500,10],"yaw":1.57}],
            "objectives": [
                {"id":"flag","center":[500,500],"radius_m":50,"kind":"junction","counterpart":null},
                {"id":"flag","center":[250,250],"radius_m":50,"kind":"field","counterpart":null},
                {"id":"other","center":[250,750],"radius_m":50,"kind":"field","counterpart":null}
            ]
        }
    })).unwrap();
    assert!(sites.validate().unwrap_err().contains("identity"));
}
