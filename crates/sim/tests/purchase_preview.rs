//! A purchase's placement preview: where its ghost stands, and how.

use crate::common::command;
use crate::skirmish::{deck, match_on};
use contract::command::Order;
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures;

#[test]
fn a_purchase_preview_spreads_a_squad_round_its_destination_and_a_hull_stands_alone() {
    let mut rules = deck();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut battle = Battle::new(&match_on(rules), 1);
    let squad = battle
        .preview_purchase(Side::Blue, "test_rifle", [400.0, 300.0])
        .unwrap();
    let slots = battle
        .rules()
        .catalog
        .get(squad.kind)
        .slots()
        .unwrap()
        .len();
    assert!(slots > 1);
    assert_eq!(
        squad.spots.len(),
        slots,
        "one spot per soldier the squad fields"
    );
    assert!(squad
        .spots
        .iter()
        .all(|s| (s[0] - 400.0).hypot(s[1] - 300.0) < 15.0));
    let tank = battle
        .preview_purchase(Side::Blue, "test_tank", [400.0, 300.0])
        .unwrap();
    assert!(tank.spots.is_empty(), "a vehicle stands as one body");
}

#[test]
fn a_purchase_ghost_keeps_its_shape_as_the_cursor_moves_over_open_ground() {
    let mut rules = deck();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut battle = Battle::new(&match_on(rules), 1);
    let shape = |battle: &mut Battle, at: [f64; 2]| -> Vec<(i64, i64)> {
        battle
            .preview_purchase(Side::Blue, "test_rifle", at)
            .unwrap()
            .spots
            .iter()
            .map(|s| (((s[0] - at[0]) * 1e6) as i64, ((s[1] - at[1]) * 1e6) as i64))
            .collect()
    };
    let first = shape(&mut battle, [400.0, 300.0]);
    for at in [[400.5, 300.0], [403.0, 298.0], [410.0, 305.0]] {
        assert_eq!(
            shape(&mut battle, at),
            first,
            "the ghost slides with the cursor at {at:?}"
        );
    }
}

/// The ring a placement ghost shows is the area its squad holds once bought:
/// the preview names the radius the arrived squad publishes, round the
/// destination it is sent to. A vehicle has no area.
#[test]
fn a_purchase_preview_names_the_area_its_squad_holds_once_it_arrives() {
    let mut rules = deck();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({
            "roster": { "factions": ["us"], "category": "inf", "family_name": "Test squad", "variant": "Test" }
        }),
    );
    let mut battle = Battle::new(&match_on(rules), 1);
    let preview = battle
        .preview_purchase(Side::Blue, "test_rifle", [400.0, 300.0])
        .unwrap();
    let radius = preview.area_radius_m.expect("a squad's ghost has an area");
    let tank = battle
        .preview_purchase(Side::Blue, "test_tank", [400.0, 300.0])
        .unwrap();
    assert_eq!(tank.area_radius_m, None, "a vehicle has no area");
    let ack = battle.accept(command(
        Side::Blue,
        1,
        Order::ConfirmPurchase {
            variant: "test_rifle".into(),
            destination: preview.destination,
        },
    ));
    assert_eq!(ack.error, None);
    battle.accept(command(Side::Blue, 2, Order::Ready));
    battle.accept(command(Side::Red, 1, Order::Ready));
    let mut arrived = None;
    for _ in 0..120 * battle.rules().tick_hz {
        battle.step();
        let frame = battle.observe(Side::Blue);
        if frame.own.first().is_some_and(|u| u.goal.is_none()) {
            arrived = Some(frame.own[0].clone());
            break;
        }
    }
    let area = arrived
        .expect("the bought squad arrives")
        .area
        .expect("the bought squad holds an area");
    assert_eq!(area.radius, radius);
    assert_eq!(area.anchor, preview.destination);
}
