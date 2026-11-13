//! Who pops smoke on their own (D3, D4): in a team skirmish, the AI player
//! pops its losing carrier's smoke, while a human player's carrier losing
//! the same fight beside it keeps every charge until its player says so.
use super::*;
use contract::players::PlayerIndex;
use sim::fixtures;
use std::f64::consts::FRAC_PI_2;

/// The share of its strength a unit loses before the skirmish AI pops.
fn loss_share() -> f64 {
    let tuning: Value =
        serde_json::from_str(include_str!("../../../../fixtures/skirmish-ai.json")).unwrap();
    tuning["smoke_loss_share"].as_f64().unwrap()
}

/// Blue's human and AI players, each able to buy the smoke tank, against a
/// red human who can buy test tanks.
fn team_match() -> ScenarioDefinition {
    let mut rules = fixtures::test_rules();
    for (kind, faction) in [(TANK, "us"), ("test_tank", "eastern")] {
        fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({ "roster": {
            "factions": [faction], "category": "veh", "family_name": "Test", "variant": "Test" } }),
        );
    }
    // Sturdy enough to lose a fight for a while before it is destroyed.
    fixtures::patch_catalog(
        &mut rules,
        "units",
        TANK,
        json!({ "body": { "hull": { "hp": 500 } } }),
    );
    serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules, "units": [],
        "players": [
            {"id": "blue-1", "side": "blue", "faction": "us", "controller": "human", "human_capable": true},
            {"id": "blue-2", "side": "blue", "faction": "us", "controller": "active_ai"},
            {"id": "red-1", "side": "red", "faction": "eastern", "controller": "human", "human_capable": true}
        ],
        "skirmish": { "rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 60, "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": { "entries": [
                { "side": "blue", "center": [300, 10], "yaw": FRAC_PI_2 },
                { "side": "blue", "center": [500, 10], "yaw": FRAC_PI_2 },
                { "side": "red", "center": [300, 590], "yaw": -FRAC_PI_2 },
                { "side": "red", "center": [500, 590], "yaw": -FRAC_PI_2 }
            ], "objectives": [
                { "id": "center", "center": [400, 300], "radius_m": 50, "kind": "junction", "counterpart": null },
                { "id": "south", "center": [200, 200], "radius_m": 50, "kind": "field", "counterpart": "north" },
                { "id": "north", "center": [600, 400], "radius_m": 50, "kind": "field", "counterpart": "south" }
            ] }
        }
    }))
    .unwrap()
}

fn order(b: &mut Battle, side: Side, player: &str, seq: u64, order: Order) {
    let ack = b.accept(CommandEnvelope {
        side,
        player: Some(player.into()),
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None, "{player}: {ack:?}");
}

fn buy(kind: &str, at: [f64; 2]) -> Order {
    Order::ConfirmPurchase {
        variant: kind.into(),
        destination: at,
    }
}

/// A blue player's carrier, as blue sees it: its strength and charges, or
/// `None` once it is gone or before it is bought.
fn carrier(battle: &Battle, player: u16) -> Option<(f64, u32)> {
    let blue = battle.observe(Side::Blue);
    let own = blue
        .own
        .iter()
        .find(|u| u.owner == Some(PlayerIndex(player)))?;
    Some((own.hp, own.smoke.expect("a smoke carrier")))
}

/// Whether `player` sent a pop order.
fn popped_by(battle: &Battle, player: &str) -> bool {
    (battle.replay().accepted.iter()).any(|(_, command)| {
        command.player.as_deref() == Some(player) && matches!(command.order, Order::PopSmoke { .. })
    })
}

#[test]
fn only_an_ai_players_losing_carrier_pops_smoke_on_its_own() {
    let mut battle = Battle::new(&team_match(), 5);
    let tank = battle.rules().catalog.index(TANK).unwrap();
    battle.script_purchases("blue-2", vec![tank], None);
    order(
        &mut battle,
        Side::Blue,
        "blue-1",
        1,
        buy(TANK, [450.0, 45.0]),
    );
    order(&mut battle, Side::Blue, "blue-1", 2, Order::Ready);
    for (seq, x) in (1..).zip([310.0, 350.0, 390.0, 430.0]) {
        order(
            &mut battle,
            Side::Red,
            "red-1",
            seq,
            buy("test_tank", [x, 230.0]),
        );
    }
    order(&mut battle, Side::Red, "red-1", 5, Order::Ready);
    let capacity = smoke(TANK).capacity;
    let (mut human_lost, mut ai_popped) = (0.0_f64, false);
    let mut full = None;
    for _ in 0..seconds(120.0) {
        battle.step();
        if let Some((hp, charges)) = carrier(&battle, 0) {
            let full = *full.get_or_insert(hp);
            human_lost = human_lost.max(1.0 - hp / full);
            assert_eq!(charges, capacity, "the human's carrier popped on its own");
        }
        ai_popped |= carrier(&battle, 1).is_some_and(|(_, charges)| charges < capacity);
        if ai_popped && human_lost > loss_share() {
            break;
        }
    }
    assert!(
        ai_popped && popped_by(&battle, "blue-2"),
        "the AI's losing carrier never popped"
    );
    assert!(
        human_lost > loss_share(),
        "the human's carrier lost only {human_lost} of its strength"
    );
    assert!(!popped_by(&battle, "blue-1"), "the human sent no pop");
}
