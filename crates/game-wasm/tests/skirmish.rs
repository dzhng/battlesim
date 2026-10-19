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
