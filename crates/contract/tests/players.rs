use contract::catalog::Faction;
use contract::encounter::EntrySite;
use contract::ids::Side;
use contract::players::{
    entry_count, side_faction, validate_entry_counts, validate_players, validate_players_per_side,
    ControllerKind, Player,
};
use serde_json::json;

fn player(id: &str, side: Side, faction: Faction) -> Player {
    Player {
        id: id.into(),
        side,
        faction: Some(faction),
        controller: ControllerKind::ActiveAi,
        human_capable: false,
    }
}

#[test]
fn skirmish_openings_must_name_a_player() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    value["scenario"]["skirmish"]["openings"] = serde_json::json!({
        "ghost": [{"variant":"test_tank","destination":[0,0]}]
    });
    let setup: contract::scenario::ScenarioDefinition =
        serde_json::from_value(value["scenario"].clone()).unwrap();
    assert!(setup
        .validate_player_contract()
        .unwrap_err()
        .contains("opening ghost"));
}

#[test]
fn match_rules_require_max_players_per_side() {
    let value = json!({
        "credits_per_minute": 1, "starting_minutes": 1, "preparation_s": 1,
        "max_units": 30, "dispatch_interval_s": 1, "capture_s": 1,
        "majority_s": 1, "victory_score": 1
    });
    assert!(serde_json::from_value::<contract::skirmish::MatchRules>(value).is_err());
}

#[test]
fn player_topology_rejects_invalid_sides_and_accepts_ai_only_matches() {
    let valid = (0..10)
        .map(|i| player(&format!("blue-{i}"), Side::Blue, Faction::Us))
        .chain((0..10).map(|i| player(&format!("red-{i}"), Side::Red, Faction::Eastern)))
        .collect::<Vec<_>>();
    assert!(validate_players(&valid).is_ok());
    let mut too_many = valid.clone();
    too_many.push(player("blue-10", Side::Blue, Faction::Us));
    assert!(validate_players(&too_many).is_ok());
    assert!(validate_players_per_side(&too_many, 10).is_err());

    let mut duplicate = valid.clone();
    duplicate[1].id = duplicate[0].id.clone();
    assert!(validate_players(&duplicate).is_err());

    let mut mixed = valid.clone();
    mixed[1].faction = Some(Faction::Europe);
    assert!(validate_players(&mixed).is_err());

    let mut factionless = valid.clone();
    factionless[0].faction = None;
    assert!(validate_players(&factionless).is_err());
    assert_eq!(side_faction(&valid, Side::Blue), Some(Faction::Us));
}

#[test]
fn player_topology_rejects_neutral_and_empty_sides() {
    let mut players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("red-1", Side::Red, Faction::Eastern),
    ];
    players[0].side = Side::Neutral;
    assert!(validate_players(&players).is_err());
    players[0].side = Side::Blue;
    players[1].side = Side::Blue;
    assert!(validate_players(&players).is_err());
}

#[test]
fn entry_count_is_one_two_or_three() {
    assert_eq!(entry_count(1), 1);
    assert_eq!(entry_count(2), 2);
    assert_eq!(entry_count(3), 3);
    assert_eq!(entry_count(10), 3);
}

#[test]
fn wrong_entry_count_is_refused_for_the_player_topology() {
    let players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("blue-2", Side::Blue, Faction::Us),
        player("red-1", Side::Red, Faction::Eastern),
    ];
    let entries = vec![
        EntrySite {
            side: Side::Blue,
            center: [0.0, 0.0],
            yaw: 0.0,
        },
        EntrySite {
            side: Side::Red,
            center: [1.0, 1.0],
            yaw: 0.0,
        },
    ];
    assert!(validate_entry_counts(&players, &entries).is_err());
}

fn entries(blue: usize, red: usize) -> Vec<EntrySite> {
    let site = |side, i: usize| EntrySite {
        side,
        center: [i as f64, 0.0],
        yaw: 0.0,
    };
    (0..blue)
        .map(|i| site(Side::Blue, i))
        .chain((0..red).map(|i| site(Side::Red, i)))
        .collect()
}

#[test]
fn both_sides_get_the_larger_sides_entry_count() {
    // A 2v1 is fair only when both edges have the same roads (D52).
    let players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("blue-2", Side::Blue, Faction::Us),
        player("red-1", Side::Red, Faction::Eastern),
    ];
    assert!(validate_entry_counts(&players, &entries(2, 2)).is_ok());
    assert!(validate_entry_counts(&players, &entries(2, 1)).is_err());
    assert!(validate_entry_counts(&players, &entries(1, 1)).is_err());
}
