//! Retirement values follow original finite rows, not round counts or replenishment history.
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

#[test]
fn finite_resources_are_an_unweighted_mean_and_unlimited_rows_do_not_dilute_it() {
    let mut rules = sim::fixtures::stand_in_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "tank",
        json!({"roster":{"factions":["us"],"category":"veh","family_name":"Test","variant":"Tank"}}),
    );
    let setup = serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":rules,"units":[{"side":"blue","kind":"tank","position":[100,100],
            "condition":{"spent":{"tank_ap":20,"tank_he":7}}},
            {"side":"blue","kind":"rifle","position":[200,100]}]
    }))
    .unwrap();
    let battle = Battle::new(&setup, 1);
    let tank = battle.unit(UnitId(0)).unwrap();
    assert_eq!(
        battle.observe(Side::Blue).own[0].mounts[0].ammo,
        vec![Some(0), Some(8)]
    );
    assert_eq!(
        sim::withdrawal::condition(tank, battle.rules(), battle.arsenal()),
        (1.0, ((0.0 + 8.0 / 15.0) / 2.0)),
        "empty AP and half HE average one quarter; unlimited HMG is ignored"
    );
}
