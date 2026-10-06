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
    }))
    .unwrap();
    assert!(sites.validate().unwrap_err().contains("identity"));
}

#[test]
fn reserved_candidates_require_unique_finite_reciprocal_geometry() {
    let mut sites: EncounterSites = serde_json::from_value(serde_json::json!({
        "settlements": [], "approaches": [], "skirmish": {
            "entries": [{"side":"blue","center":[500,990],"yaw":-1.57},
                {"side":"red","center":[500,10],"yaw":1.57}],
            "objectives": [
                {"id":"hub","center":[500,500],"radius_m":50,"kind":"junction","counterpart":null},
                {"id":"a","center":[250,750],"radius_m":50,"kind":"field","counterpart":"b"},
                {"id":"b","center":[250,250],"radius_m":50,"kind":"field","counterpart":"a"}
            ],
            "candidates": [
                {"id":"c","center":[750,750],"radius_m":50,"kind":"field","counterpart":"d"},
                {"id":"d","center":[750,250],"radius_m":50,"kind":"field","counterpart":"c"}
            ]
        }
    }))
    .unwrap();
    sites.validate().unwrap();
    let original = sites.clone();
    sites.skirmish.as_mut().unwrap().candidates[0].id = "hub".into();
    assert!(sites.validate().unwrap_err().contains("identity"));
    sites = original.clone();
    sites.skirmish.as_mut().unwrap().candidates[0].center[0] = f64::NAN;
    assert!(sites.validate().unwrap_err().contains("geometry"));
    sites = original;
    sites.skirmish.as_mut().unwrap().candidates[0].counterpart = Some("a".into());
    assert!(sites.validate().unwrap_err().contains("reciprocal"));
}
