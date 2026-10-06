//! Supported faction content crosses the same battle and service seams as scenario units.
use contract::ids::UnitId;
use serde_json::json;
use sim::battle::Battle;

#[test]
fn faction_rifle_squad_fires_its_carried_weapons() {
    let mut fixture = sim::fixtures::game();
    sim::fixtures::patch_catalog(&mut fixture, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
    let setup = serde_json::from_value(json!({
        "map": { "size": [500, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [
            { "side": "blue", "kind": "rifle_squad", "position": [150, 200] },
            { "side": "red", "kind": "rifle", "position": [220, 200], "engagement": "return_fire_only" }
        ]
    })).unwrap();
    let mut battle = Battle::new(&setup, 1);
    for _ in 0..20 * battle.rules().tick_hz {
        battle.step();
    }
    let unit = battle.unit(UnitId(0)).unwrap();
    let mounts = battle.rules().catalog.mounts(unit.kind);
    for (state, spec) in unit.mounts.iter().zip(mounts) {
        assert!(state.shots > 0, "{} never fired", spec.def.name);
    }
}

#[test]
fn named_supply_truck_rearms_a_finite_launcher_until_stock_runs_out() {
    use contract::ids::Side;
    let mut fixture = sim::fixtures::game();
    fixture["service"]["round_costs"]["tow"] = json!(20);
    let setup = serde_json::from_value(json!({
        "map": { "size": [500, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [
            { "side": "blue", "kind": "us_m977_hemtt_general_resupply", "position": [100, 200], "stock": 20 },
            { "side": "blue", "kind": "us_atgm_team_bgm_71_tow_2a", "position": [130, 200], "engagement": "return_fire_only", "condition": { "spent": { "tow": 2 } } }
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
    let mut fixture = sim::fixtures::game();
    fixture["weapons"]["marksman_rifle"]["ammo"] = json!(2);
    sim::fixtures::patch_catalog(&mut fixture, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
    sim::fixtures::patch_catalog(
        &mut fixture,
        "soldiers",
        "roster_marksman_rifle_body",
        json!({ "hp": 1.0e6 }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [500, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [
            { "side": "blue", "kind": "europe_marksman_squad_longer_range_infantry", "position": [150, 200] },
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
