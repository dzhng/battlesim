use contract::catalog::Faction;
use contract::ids::Side;
use contract::players::{ControllerKind, Player};
use contract::skirmish::SkirmishSetup;

fn player(id: String, side: Side) -> Player {
    Player {
        id,
        side,
        faction: Some(if side == Side::Blue {
            Faction::Us
        } else {
            Faction::Eastern
        }),
        controller: ControllerKind::ActiveAi,
        human_capable: false,
    }
}

fn players(blue: usize, red: usize) -> Vec<Player> {
    let seats = |side: Side, count: usize, name: &'static str| {
        (1..=count).map(move |n| player(format!("{name}-{n}"), side))
    };
    seats(Side::Blue, blue, "blue")
        .chain(seats(Side::Red, red, "red"))
        .collect()
}

/// The builder's setup seats every player it is given, with as many roads a
/// side as the larger side's size tier lays, the centre road listed first,
/// and objectives to fight over.
#[test]
fn the_synthetic_setup_seats_its_players_on_their_tiers_roads() {
    let size = [1200.0, 900.0];
    for (blue, red) in [(2, 1), (5, 4)] {
        let players = players(blue, red);
        let setup: SkirmishSetup = serde_json::from_str(
            &game_wasm::synthetic_skirmish_setup(
                &serde_json::to_string(&size).unwrap(),
                &serde_json::to_string(&players).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let roads = contract::skirmish_sizes::tier(blue.max(red)).entry_count;
        for side in Side::COMBATANTS {
            let xs: Vec<f64> = setup
                .sites
                .entries
                .iter()
                .filter(|entry| entry.side == side)
                .map(|entry| entry.center[0])
                .collect();
            assert_eq!(xs.len(), roads, "{blue}v{red} {side:?}: {xs:?}");
            assert_eq!(xs[0], size[0] / 2.0, "{blue}v{red} {side:?} centre road");
        }
        contract::players::validate_players_per_side(
            &players,
            setup.rules.max_players_per_side as usize,
        )
        .unwrap_or_else(|error| panic!("{blue}v{red}: {error}"));
        assert!(!setup.sites.objectives.is_empty(), "{blue}v{red}");
    }
}

/// The page's lanes are `sim::entries`' own: every entry's number and holder
/// in list order, and the opening on the human's lane.
#[test]
fn the_lane_answer_is_the_entry_owners() {
    use sim::entries::{self, EntryIndex};
    let sites = sim::skirmish::synthetic_sites([1200.0, 900.0], 3);
    let mut players = players(2, 3);
    players[1].controller = ControllerKind::Human;
    players[1].human_capable = true;
    let answer: serde_json::Value = serde_json::from_str(
        &game_wasm::skirmish_lanes(
            &serde_json::to_string(&sites).unwrap(),
            &serde_json::to_string(&players).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let sides: Vec<_> = players.iter().map(|player| player.side).collect();
    let homes = entries::homes(&sites, &sides);
    let rows = answer["entries"].as_array().unwrap();
    assert_eq!(rows.len(), sites.entries.len());
    for (index, row) in rows.iter().enumerate() {
        let lane = entries::lane_number(&sites, EntryIndex(index)).0;
        assert_eq!(row["lane"], lane);
        let holder = homes
            .iter()
            .position(|home| *home == Some(EntryIndex(index)))
            .map(|player| players[player].id.clone());
        assert_eq!(row["holder"], serde_json::json!(holder));
    }
    // Blue fields two players on three roads: one road is unheld.
    let blue_unheld = rows
        .iter()
        .filter(|row| row["side"] == "blue" && row["holder"].is_null())
        .count();
    assert_eq!(blue_unheld, 1);
    assert_eq!(answer["opening"], homes[1].unwrap().0);
}
