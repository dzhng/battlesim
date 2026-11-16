use super::*;
use serde_json::json;

#[test]
fn referee_and_seat_bindings_are_read_only_then_apply_the_recorded_pair() {
    // Only a declared faction is needed to admit these otherwise empty seats.
    let catalog: serde_json::Value = serde_json::from_str(&sim::fixtures::catalog_view()).unwrap();
    let faction = &catalog["cards"][0]["roster"]["factions"][0];
    let players: Vec<contract::players::Player> = serde_json::from_value(json!([
        {"id":"blue-1","side":"blue","faction":faction,"controller":"human","human_capable":true},
        {"id":"blue-2","side":"blue","faction":faction,"controller":"passive_ai","human_capable":false},
        {"id":"red-1","side":"red","faction":faction,"controller":"human","human_capable":true}
    ])).unwrap();
    let rules = sim::skirmish::starter_rules();
    let scenario = json!({
        "rules": sim::fixtures::test_rules(),
        "map": {"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "players": players,
        "skirmish": {"rules": rules, "sites": sim::skirmish::synthetic_sites([800.0,600.0], contract::players::entries_per_side(&players))},
        "units": []
    }).to_string();
    let mut handle = BattleHandle::new(&scenario, 43.0).unwrap();
    let before = (handle.digest(), handle.replay_json().unwrap());
    let view: serde_json::Value = serde_json::from_str(&handle.referee_view().unwrap()).unwrap();
    assert_eq!((handle.digest(), handle.replay_json().unwrap()), before);
    assert_eq!(view["target"], json!(rules.victory_score));
    assert_eq!(view["seats"][0]["living"], 0);
    handle.hand_over("blue-1").unwrap();
    handle.reset_seat("blue-1", true).unwrap();
    handle.step();
    let view: serde_json::Value = serde_json::from_str(&handle.referee_view().unwrap()).unwrap();
    assert_eq!(view["seats"][0]["living"], 0);
    assert_eq!(view["seats"][1]["living"], 0);
    assert_eq!(view["seats"][0]["credits"], view["opening_credits"]);
    let replay = handle.replay_json().unwrap();
    let mut replayed = BattleHandle::from_replay(&scenario, &replay).unwrap();
    replayed.step();
    assert_eq!(handle.digest(), replayed.digest());
}
