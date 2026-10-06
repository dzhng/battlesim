//! Finite rounds: a supply truck's stock buys a launcher what it can pay
//! for, and each gun a squad carries spends its own.
use contract::ids::UnitId;
use serde_json::json;
use sim::battle::Battle;

#[test]
fn a_supply_truck_rearms_a_finite_launcher_until_its_stock_runs_out() {
    use contract::ids::Side;
    let mut fixture = sim::fixtures::stand_in_game();
    // A missile costs the whole of the truck's stock.
    fixture["weapons"]["atgm"]["ammo"] = json!(4);
    fixture["service"]["round_costs"]["atgm"] = json!(20);
    let setup = serde_json::from_value(json!({
        "map": { "size": [500, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [
            { "side": "blue", "kind": "supply", "position": [100, 200], "stock": 20 },
            { "side": "blue", "kind": "at", "position": [130, 200], "engagement": "return_fire_only", "condition": { "spent": { "atgm": 2 } } }
        ]
    })).unwrap();
    let mut battle = Battle::new(&setup, 2);
    let before = battle
        .unit(UnitId(1))
        .unwrap()
        .mounts
        .iter()
        .find(|m| m.ammo == vec![Some(2)])
        .expect("launcher starts two rounds short")
        .ammo
        .clone();
    for _ in 0..10 * battle.rules().tick_hz {
        battle.step();
    }
    let own = &battle.observe(Side::Blue).own;
    assert_eq!(
        own.iter().find(|u| u.id == UnitId(0)).unwrap().stock,
        Some(0)
    );
    let launcher = own
        .iter()
        .find(|u| u.id == UnitId(1))
        .unwrap()
        .mounts
        .iter()
        .find(|m| m.ammo.iter().any(Option::is_some))
        .unwrap();
    assert_eq!(before, vec![Some(2)]);
    assert_eq!(
        launcher.ammo,
        vec![Some(3)],
        "finite stock buys one missile, not a free complete refill"
    );
}

#[test]
fn finite_squad_guns_exhaust_independently() {
    let mut fixture = sim::fixtures::stand_in_game();
    fixture["weapons"]["marksman_rifle"]["ammo"] = json!(2);
    sim::fixtures::patch_catalog(&mut fixture, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
    // A pair of marksmen, each carrying a gun of his own.
    let gunner = |k: u32| {
        json!({
            "name": "Marksman", "description": "Test only.", "hp": 1.0e6,
            "appearance": ["rifle"],
            "mounts": [{ "name": format!("gun {k}"), "weapons": ["marksman_rifle"], "squad": true }]
        })
    };
    fixture["catalog"].as_array_mut().unwrap().push(json!({
        "soldiers": { "gunner_1": gunner(1), "gunner_2": gunner(2) },
        "units": { "marksmen": {
            "extends": "squad", "name": "Marksmen", "description": "Test only.",
            "roles": ["infantry"], "cost": 100,
            "body": { "squad": { "slots": ["gunner_1", "gunner_2"] } }
        } }
    }));
    let setup = serde_json::from_value(json!({
        "map": { "size": [500, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [
            { "side": "blue", "kind": "marksmen", "position": [150, 200] },
            { "side": "red", "kind": "rifle", "position": [250, 200], "engagement": "return_fire_only" }
        ]
    })).unwrap();
    let mut battle = Battle::new(&setup, 3);
    for _ in 0..20 * battle.rules().tick_hz {
        battle.step();
    }
    let unit = battle.unit(UnitId(0)).unwrap();
    assert_eq!(
        unit.mounts.len(),
        unit.members.len(),
        "each physical gun retains an independent resource owner"
    );
    for mount in &unit.mounts {
        assert_eq!(mount.shots, 2, "a gun consumes its own two rounds");
        assert_eq!(mount.ammo, vec![Some(0)]);
    }
}
