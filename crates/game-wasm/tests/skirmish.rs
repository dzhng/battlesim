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
    use sim::entries;
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
    let rows = answer["entries"].as_array().unwrap();
    assert_eq!(rows.len(), sites.entries.len());
    let row = |side: Side, lane: u8| {
        let index = entries::lane_entry(&sites, side, entries::LaneNumber(lane)).unwrap();
        assert_eq!(rows[index.0]["lane"], lane);
        &rows[index.0]
    };
    // Slot k holds lane k; Blue's two players leave its third road open.
    assert_eq!(row(Side::Blue, 1)["holder"], "blue-1");
    assert_eq!(row(Side::Blue, 2)["holder"], "blue-2");
    assert!(row(Side::Blue, 3)["holder"].is_null());
    for lane in 1..=3 {
        assert_eq!(row(Side::Red, lane)["holder"], format!("red-{lane}"));
    }
    // The human, Blue 2, opens on its own lane.
    let blue_two = entries::lane_entry(&sites, Side::Blue, entries::LaneNumber(2)).unwrap();
    assert_eq!(answer["opening"], blue_two.0);
}
