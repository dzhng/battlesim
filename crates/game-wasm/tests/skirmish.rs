use contract::catalog::Faction;
use contract::ids::Side;
use contract::players::{ControllerKind, Player};

fn player(id: &str, side: Side, faction: Faction) -> Player {
    Player {
        id: id.into(),
        side,
        faction: Some(faction),
        controller: ControllerKind::ActiveAi,
        human_capable: false,
    }
}

/// The builder's setup is the one every skirmish gets, not a second copy:
/// the starter rules, and the synthetic roads and objectives for the
/// players it seats.
#[test]
fn the_synthetic_setup_is_setup_for_over_the_synthetic_sites() {
    let players = vec![
        player("blue-1", Side::Blue, Faction::Us),
        player("blue-2", Side::Blue, Faction::Us),
        player("red-1", Side::Red, Faction::Eastern),
    ];
    let size = [1200.0, 900.0];
    let got: serde_json::Value = serde_json::from_str(
        &game_wasm::synthetic_skirmish_setup(
            &serde_json::to_string(&size).unwrap(),
            &serde_json::to_string(&players).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let sites = sim::skirmish::synthetic_sites(size, contract::players::entries_per_side(&players));
    let want = serde_json::to_value(sim::skirmish::setup_for(&players, sites)).unwrap();
    assert_eq!(got, want);
    assert!(!got["sites"]["objectives"].as_array().unwrap().is_empty());
}
