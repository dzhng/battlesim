use contract::catalog::Faction;
use contract::encounter::EntrySite;
use contract::generation::MapSize;
use contract::ids::Side;
use contract::players::{
    validate_entry_counts, validate_players, validate_players_per_side, ControllerKind, Player,
    Seating,
};
use contract::skirmish_sizes::{floor_for_entries, tier};
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
    assert!(validate_players(&valid, Seating::Skirmish).is_ok());
    let mut too_many = valid.clone();
    too_many.push(player("blue-10", Side::Blue, Faction::Us));
    assert!(validate_players(&too_many, Seating::Skirmish).is_ok());
    assert!(validate_players_per_side(&too_many, 10).is_err());

    let mut duplicate = valid.clone();
    duplicate[1].id = duplicate[0].id.clone();
    assert!(validate_players(&duplicate, Seating::Skirmish).is_err());

    let mut mixed = valid.clone();
    mixed[1].faction = Some(Faction::Europe);
    assert!(validate_players(&mixed, Seating::Skirmish).is_err());

    let mut factionless = valid.clone();
    factionless[0].faction = None;
    assert!(validate_players(&factionless, Seating::Skirmish).is_err());
}

#[test]
fn player_topology_rejects_neutral_and_empty_sides() {
    let mut players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("red-1", Side::Red, Faction::Eastern),
    ];
    players[0].side = Side::Neutral;
    assert!(validate_players(&players, Seating::Skirmish).is_err());
    players[0].side = Side::Blue;
    players[1].side = Side::Blue;
    assert!(validate_players(&players, Seating::Skirmish).is_err());
}

#[test]
fn an_authored_scenario_may_seat_a_factionless_neutral_player() {
    let mut players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("civilians", Side::Neutral, Faction::Us),
    ];
    assert!(validate_players(&players, Seating::Authored).is_err());
    players[1].faction = None;
    assert!(validate_players(&players, Seating::Authored).is_ok());
    assert!(validate_players(&players, Seating::Skirmish).is_err());
    players[1].human_capable = true;
    assert!(validate_players(&players, Seating::Authored).is_err());
}

#[test]
fn entry_count_is_one_two_or_three() {
    assert_eq!(tier(1).entry_count, 1);
    assert_eq!(tier(2).entry_count, 2);
    assert_eq!(tier(3).entry_count, 3);
    assert_eq!(tier(10).entry_count, 3);
}

#[test]
fn entry_count_follows_the_shared_size_fixture() {
    assert_eq!(tier(4).entry_count, 3);
    assert_eq!(tier(5).entry_count, 3);
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

#[test]
fn skirmish_size_floor_comes_from_the_shared_fixture() {
    assert_eq!(tier(1).size, MapSize::Small);
    assert_eq!(tier(2).size, MapSize::Medium);
    assert_eq!(tier(3).size, MapSize::Large);
    assert_eq!(tier(4).size, MapSize::Large);
    assert_eq!(tier(5).size, MapSize::Xl);
    assert_eq!(tier(10).size, MapSize::Xl);
}

#[test]
fn the_generator_admits_the_size_the_lobby_picks_for_any_side() {
    // The layout refuses a map below its entry count's floor; the lobby
    // picks a side's tier. This checks the size table: for every side size,
    // the tier's map is at least the floor for the tier's roads.
    let max = contract::skirmish_sizes::tiers()
        .iter()
        .map(|tier| tier.max_players.unwrap_or(tier.min_players))
        .max()
        .unwrap();
    for players in 1..=max + 1 {
        let tier = tier(players);
        let floor = floor_for_entries(tier.entry_count).expect("a size seats the tier's roads");
        assert!(
            floor <= tier.size,
            "{players} players: {floor:?} > {:?}",
            tier.size
        );
    }
}

#[test]
fn generation_request_defaults_to_one_entry_and_round_trips_explicit_count() {
    let base = serde_json::json!({
        "generator_version": "layout-18",
        "preset_revision": "r1",
        "seed": "1",
        "template_catalog_hash": "hash",
        "type": "open",
        "size": "small",
        "profile": "skirmish",
        "limits": {"max_authored_parts": 1, "max_bay_positions": 1, "max_ground_points": 1}
    });
    let request: contract::generation::GenerationRequest = serde_json::from_value(base).unwrap();
    assert_eq!(request.entries_per_side, 1);
    let mut explicit = serde_json::to_value(request).unwrap();
    explicit["entries_per_side"] = serde_json::json!(3);
    let request: contract::generation::GenerationRequest =
        serde_json::from_value(explicit).unwrap();
    assert_eq!(request.entries_per_side, 3);
}

#[test]
fn a_skirmish_scenario_with_an_authored_unit_is_refused() {
    // Skirmish players buy every unit; one authored unit would have no
    // purchase, and without an owner no player to credit it to.
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-objectives.json"
    ))
    .unwrap();
    value["scenario"]["units"] = json!([{
        "kind": "test_tank", "side": "blue", "position": [400.0, 300.0], "yaw": 0.0
    }]);
    let setup: contract::scenario::ScenarioDefinition =
        serde_json::from_value(value["scenario"].clone()).unwrap();
    assert!(setup
        .validate_player_contract()
        .unwrap_err()
        .contains("authored"));
}
