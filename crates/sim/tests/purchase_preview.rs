//! A purchase's placement preview: where its ghost stands, and how.

use crate::skirmish::{deck, match_on};
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
