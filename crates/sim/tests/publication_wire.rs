//! Publication wire regressions for deployment posture and touchdown state.
use contract::ids::Side;
use serde_json::json;
use sim::battle::Battle;
use sim::publication;

use crate::common;

#[test]
fn deployment_and_grounded_are_published_for_own_and_identified_aircraft() {
    let map = json!({
        "size": [800, 600],
        "fog_cell_m": 8,
        "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "props": []
    });
    let scenario = common::scenario(
        &map.to_string(),
        json!([
            {"side": "blue", "kind": "test_supply_heli", "position": [200, 200]},
            {"side": "blue", "kind": "test_supply", "position": [260, 260]},
            {"side": "red", "kind": "test_rifle", "position": [320, 260]}
        ]),
        json!([]),
    );
    let mut battle = Battle::new(&scenario, 1);
    battle.step();
    assert!(!battle.observe(Side::Blue).own[0].grounded);
    assert!(
        !battle
            .observe(Side::Red)
            .identified
            .iter()
            .find(|unit| unit.kind == common::unit_kind("test_supply_heli"))
            .expect("red sees the supply helicopter while it descends")
            .grounded
    );
    while !battle.unit(contract::ids::UnitId(0)).unwrap().landed() {
        battle.step();
    }
    let layout = publication::layout_json(&battle);
    let mut publisher = publication::Publisher::new();
    let words: Vec<u32> = publisher
        .publish(&battle, Side::Red)
        .unwrap()
        .iter()
        .map(|word| word.to_bits())
        .collect();
    let observation = publication::reader::Reader::new(&layout)
        .unwrap()
        .decode(&words)
        .unwrap()
        .unwrap();
    let enemy = observation
        .identified
        .iter()
        .find(|unit| unit.kind.as_ref() == "test_supply_heli")
        .expect("red sees the supply helicopter");
    assert!(enemy.grounded);
    assert!(enemy.deployment.is_some());
    let enemy_truck = observation
        .identified
        .iter()
        .find(|unit| unit.kind.as_ref() == "test_supply")
        .expect("red sees the supply truck");
    assert!(enemy_truck.deployment.is_some());
    assert!(observation.own.iter().any(|unit| unit.deployment.is_none()));

    let own_words: Vec<u32> = publisher
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|word| word.to_bits())
        .collect();
    let own = publication::reader::Reader::new(&layout)
        .unwrap()
        .decode(&own_words)
        .unwrap()
        .unwrap();
    assert!(own.own[0].grounded);
    assert!(own.own[0].deployment.is_some());
}
