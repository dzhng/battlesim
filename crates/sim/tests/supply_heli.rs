//! Slice 02: a supply helicopter brakes, lands, serves, and packs again.

use super::air::{battle_with, fixture, open_map, order};
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use contract::observation::ServiceStatus;
use serde_json::{json, Value};
use sim::battle::Battle;

const HELI: UnitId = UnitId(0);
const RECIPIENT: UnitId = UnitId(1);

fn units(extra: Value) -> Value {
    let mut rows = vec![json!({
        "side": "blue",
        "kind": "test_supply_heli",
        "position": [200, 200]
    })];
    rows.extend(extra.as_array().cloned().unwrap_or_default());
    Value::Array(rows)
}

fn battle(extra: Value) -> Battle {
    battle_with(&fixture(), open_map(), units(extra))
}

fn deploy(b: &mut Battle, deployed: bool) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: b.replay().accepted.len() as u64 + 1,
        order: Order::SetDeployment {
            units: vec![HELI],
            deployed,
        },
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

fn timing(b: &Battle) -> (u64, u64, u64) {
    let d = b
        .rules()
        .catalog
        .by_id("test_supply_heli")
        .capabilities
        .deploy
        .unwrap();
    (
        (d.seconds * b.rules().tick_hz as f64).round() as u64,
        (d.pack_seconds * b.rules().tick_hz as f64).round() as u64,
        (d.drop_seconds.unwrap() * b.rules().tick_hz as f64).round() as u64,
    )
}

fn ground(b: &Battle, id: UnitId) -> f64 {
    let p = b.unit(id).unwrap().position;
    b.world().height_at(p.x, p.y).unwrap()
}

fn set_up(b: &mut Battle) {
    let (setup, _, _) = timing(b);
    for _ in 0..setup {
        b.step();
    }
    assert!(b.unit(HELI).unwrap().landed());
}

fn quiet_fixture() -> Value {
    let mut rules = fixture();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply_heli",
        json!({"body":{"hull":{"hp":10000.0}}}),
    );
    rules
}

fn shots(b: &Battle, unit: u32, mount: &str) -> u32 {
    let u = b.unit(UnitId(unit)).unwrap();
    b.rules()
        .catalog
        .mounts(u.kind)
        .iter()
        .zip(&u.mounts)
        .filter(|(spec, _)| spec.def.id == mount)
        .map(|(_, m)| m.shots)
        .sum()
}

#[test]
fn landed_a_ground_gun_engages_it_and_an_air_only_weapon_does_not() {
    let mut b = battle_with(
        &quiet_fixture(),
        open_map(),
        json!([
            json!({"side":"blue","kind":"test_supply_heli","position":[200,200]}),
            json!({"side":"red","kind":"test_tank","position":[230,200]}),
            json!({"side":"red","kind":"test_aa","position":[230,220]})
        ]),
    );
    set_up(&mut b);
    for _ in 0..3 * b.rules().tick_hz {
        b.step();
    }
    assert!(
        shots(&b, 1, "cannon") > 0,
        "tank mounts: {:?}",
        b.unit(UnitId(1)).unwrap().mounts
    );
    assert_eq!(shots(&b, 2, "AA launcher"), 0);
}

#[test]
fn once_packed_the_air_only_weapon_engages_and_the_ground_gun_does_not() {
    let mut b = battle_with(
        &quiet_fixture(),
        open_map(),
        json!([
            json!({"side":"blue","kind":"test_supply_heli","position":[200,200]}),
            json!({"side":"red","kind":"test_tank","position":[230,200]}),
            json!({"side":"red","kind":"test_aa","position":[230,220]})
        ]),
    );
    set_up(&mut b);
    deploy(&mut b, false);
    let (_, pack, _) = timing(&b);
    for _ in 0..pack + 3 * b.rules().tick_hz as u64 {
        b.step();
    }
    assert!(!b.unit(HELI).unwrap().landed());
    assert!(
        b.unit(UnitId(2))
            .unwrap()
            .mounts
            .iter()
            .any(|m| m.shots > 0),
        "aa mounts: {:?}",
        b.unit(UnitId(2)).unwrap().mounts
    );
}

#[test]
fn a_ground_mover_routes_round_a_landed_heli() {
    let mut b = battle(json!([
        json!({"side":"blue","kind":"test_jeep","position":[170,200]})
    ]));
    set_up(&mut b);
    assert_eq!(b.accept(order(&[1], [250.0, 200.0])).error, None);
    b.step();
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
    }
    let jeep = b.unit(UnitId(1)).unwrap();
    assert!(
        (jeep.position.y - 200.0).abs() > 1e-3
            || jeep
                .route
                .as_ref()
                .is_some_and(|r| r.iter().any(|p| (p.y - 200.0).abs() > 1e-3)),
        "jeep: {jeep:?}"
    );
}

#[test]
fn a_landed_heli_is_never_shoved_and_fells_no_trees() {
    let map = json!({
        "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "forests": [{"shape":{"kind":"polygon","ring":[[180,180],[220,180],[220,220],[180,220]]}}]
    });
    let mut b = battle_with(
        &fixture(),
        map,
        json!([json!({"side":"blue","kind":"test_supply_heli","position":[200,200]})]),
    );
    set_up(&mut b);
    let at = b.unit(HELI).unwrap().position.xy();
    assert!(b.unit(HELI).unwrap().ground_footprint().is_some());
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    assert_eq!(b.unit(HELI).unwrap().position.xy(), at);
    assert!(b.world().forest_near(at, 20.0));
}

#[test]
fn landed_it_takes_forest_and_smoke_concealment() {
    let map = json!({
        "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "forests": [{"shape":{"kind":"polygon","ring":[[180,180],[220,180],[220,220],[180,220]]}}]
    });
    let mut b = battle_with(
        &fixture(),
        map,
        json!([
            json!({"side":"blue","kind":"test_supply_heli","position":[200,200]}),
            json!({"side":"blue","kind":"test_smoke_squad","position":[200,200]}),
            json!({"side":"red","kind":"test_rifle","position":[260,200],"engagement":"return_fire_only"})
        ]),
    );
    set_up(&mut b);
    assert_eq!(
        b.unit(HELI).unwrap().layer(),
        contract::catalog::AltitudeLayer::Ground
    );
    assert_eq!(
        b.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order: Order::PopSmoke {
                units: vec![UnitId(1)]
            },
            queued: false
        })
        .error,
        None
    );
    b.step();
    assert!(b.unit(HELI).unwrap().ground_footprint().is_some());
    assert!(b.world().smoke_concealment(200.0, 200.0) < 1.0);
}

#[test]
fn killed_landed_it_leaves_a_ground_wreck_and_no_crash_blast() {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_supply_heli",
        json!({"body":{"hull":{"hp":1.0}}}),
    );
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            json!({"side":"blue","kind":"test_supply_heli","position":[200,200]}),
            json!({"side":"red","kind":"test_tank","position":[300,200],"engagement":"return_fire_only"})
        ]),
    );
    set_up(&mut b);
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        if !b.unit(HELI).unwrap().alive() {
            break;
        }
    }
    assert!(!b.unit(HELI).unwrap().alive());
    assert!(b.crashes().is_empty());
    assert!(b
        .world()
        .props()
        .any(|p| p.wreck_of == Some(b.unit(HELI).unwrap().kind)));
}

#[test]
fn killed_in_the_drop_it_crashes() {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_supply_heli",
        json!({"body":{"hull":{"hp":1.0}},"capabilities":{"deploy":{"seconds":10,"drop_seconds":8}}}),
    );
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            json!({"side":"blue","kind":"test_supply_heli","position":[200,200],"engagement":"return_fire_only"}),
            json!({"side":"red","kind":"test_jeep","position":[300,200]})
        ]),
    );
    b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::SetDeployment {
            units: vec![HELI],
            deployed: true,
        },
        queued: false,
    });
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        if !b.crashes().is_empty() {
            break;
        }
    }
    assert!(
        !b.crashes().is_empty(),
        "heli={:?} aa={:?} tick={}",
        b.unit(HELI).unwrap(),
        b.unit(UnitId(1)).unwrap(),
        b.tick()
    );
}

#[test]
fn its_door_guns_fire_while_landed() {
    let mut b = battle(json!([
        json!({"side":"red","kind":"test_jeep","position":[230,200]})
    ]));
    set_up(&mut b);
    for _ in 0..2 * b.rules().tick_hz {
        b.step();
    }
    assert!(shots(&b, 0, "HMG") > 0);
}

#[test]
fn a_packed_heli_never_reaches_ground_movement() {
    let mut b = battle(json!([
        json!({"side":"blue","kind":"test_jeep","position":[250,200]})
    ]));
    assert_eq!(b.accept(order(&[0], [500.0, 200.0])).error, None);
    b.step();
    assert!(b.unit(HELI).unwrap().airborne());
    let mut landed = battle(json!([]));
    set_up(&mut landed);
    deploy(&mut landed, false);
    let mut move_order = order(&[0], [500.0, 200.0]);
    move_order.seq = 2;
    assert_eq!(landed.accept(move_order).error, None);
    let (_, pack, _) = timing(&landed);
    for _ in 0..pack {
        landed.step();
    }
    assert!(!landed.unit(HELI).unwrap().landed());
}

#[test]
fn it_brakes_to_a_hover_before_it_starts_to_land() {
    let mut b = battle(json!([]));
    assert_eq!(b.accept(order(&[0], [500.0, 200.0])).error, None);
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    deploy(&mut b, true);
    let (.., drop) = timing(&b);
    for _ in 0..drop {
        if b.unit(HELI).unwrap().air.unwrap().velocity.xy().length() > 1e-3 {
            assert_eq!(
                b.unit(HELI).unwrap().deployment.as_ref().unwrap().current,
                0
            );
        }
        b.step();
    }
}

#[test]
fn its_height_falls_with_setup_and_it_touches_down_before_it_serves() {
    let mut b = battle(json!([json!({
        "side": "blue", "kind": "test_tank", "position": [205, 200],
        "condition": {"hp": 1}, "engagement": "return_fire_only"
    })]));
    deploy(&mut b, true);
    let (setup, _, drop) = timing(&b);
    let start = b.unit(HELI).unwrap().position.z;
    let mut previous = start;
    for tick in 0..setup {
        b.step();
        let heli = b.unit(HELI).unwrap();
        assert!(heli.position.z <= previous + 1e-9);
        assert!(heli.position.z >= ground(&b, HELI) - 1e-9);
        previous = heli.position.z;
        // It stands on the ground, a landed unit, from the end of its drop;
        // it serves only once set up.
        assert_eq!(heli.landed(), tick + 1 >= drop, "tick {}", tick + 1);
        if tick + 1 >= drop {
            assert!((heli.position.z - ground(&b, HELI)).abs() < 1e-6);
        }
        if tick + 1 < setup {
            assert_ne!(b.unit(RECIPIENT).unwrap().service, ServiceStatus::Serving);
        }
    }
    assert!(b.unit(HELI).unwrap().landed());
    assert_eq!(b.unit(RECIPIENT).unwrap().service, ServiceStatus::Serving);
}

#[test]
fn a_set_up_supply_heli_serves_its_reach_from_its_stock() {
    let mut b = battle(json!([json!({
        "side": "blue", "kind": "test_at", "position": [205, 200],
        "condition": {"spent": {"atgm": 3}}, "engagement": "return_fire_only"
    })]));
    let stock = b.unit(HELI).unwrap().stock.unwrap();
    let (setup, _, _) = timing(&b);
    for _ in 0..setup + 30 * b.rules().tick_hz as u64 {
        b.step();
    }
    assert!(b.unit(HELI).unwrap().stock.unwrap() < stock);
    assert_eq!(b.unit(RECIPIENT).unwrap().service, ServiceStatus::Serving);
}

#[test]
fn it_packs_on_the_ground_and_climbs_only_once_packed() {
    let mut b = battle(json!([]));
    let (setup, pack, _) = timing(&b);
    for _ in 0..setup {
        b.step();
    }
    deploy(&mut b, false);
    let z = b.unit(HELI).unwrap().position.z;
    for _ in 0..pack {
        b.step();
        if b.unit(HELI).unwrap().deployment.as_ref().unwrap().current > 0 {
            assert!((b.unit(HELI).unwrap().position.z - z).abs() < 1e-6);
        }
    }
    assert!(!b.unit(HELI).unwrap().landed());
    let before_xy = b.unit(HELI).unwrap().position.xy();
    b.accept(order(&[0], [500.0, 200.0]));
    b.step();
    assert!(b.unit(HELI).unwrap().position.z > z);
    assert_eq!(b.unit(HELI).unwrap().position.xy(), before_xy);
}

#[test]
fn packed_mid_drop_it_flies_on_from_where_it_was() {
    let mut b = battle(json!([]));
    deploy(&mut b, true);
    let (_, _, drop) = timing(&b);
    for _ in 0..drop / 2 {
        b.step();
    }
    let z = b.unit(HELI).unwrap().position.z;
    deploy(&mut b, false);
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    assert!(!b.unit(HELI).unwrap().landed());
    assert!(b.unit(HELI).unwrap().position.z >= z);
}

#[test]
fn its_vertical_velocity_matches_its_sink() {
    let mut b = battle(json!([]));
    deploy(&mut b, true);
    let before = b.unit(HELI).unwrap().position.z;
    b.step();
    let heli = b.unit(HELI).unwrap();
    let velocity = heli.air.unwrap().velocity.z;
    assert!(velocity < 0.0);
    assert!((velocity - (heli.position.z - before) * b.rules().tick_hz as f64).abs() < 1e-9);
}

#[test]
fn a_landed_heli_neither_drifts_nor_pushes_other_aircraft() {
    let mut b = battle(json!([
        json!({"side":"blue","kind":"test_supply_heli","position":[205,200]}),
        json!({"side":"blue","kind":"test_supply","position":[205,200]})
    ]));
    let (setup, _, _) = timing(&b);
    for _ in 0..setup {
        b.step();
    }
    let at = b.unit(HELI).unwrap().position.xy();
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    assert_eq!(b.unit(HELI).unwrap().position.xy(), at);
}

#[test]
fn the_low_hover_clears_a_landed_heli() {
    let mut b = battle(json!([
        json!({"side":"blue","kind":"test_heli","position":[205,200],"condition":{"hp":1},"engagement":"return_fire_only"})
    ]));
    let (setup, _, _) = timing(&b);
    for _ in 0..setup {
        b.step();
    }
    let landed = b.unit(HELI).unwrap();
    assert!(
        b.unit(UnitId(1)).unwrap().position.z - b.unit(UnitId(1)).unwrap().hull.unwrap().z
            > landed.position.z + landed.hull.unwrap().z
    );
}

#[test]
fn a_landing_replays_exactly() {
    let mut b = battle(json!([]));
    let (setup, _, _) = timing(&b);
    for _ in 0..setup + 5 {
        b.step();
    }
    let record = b.replay();
    let setup = serde_json::from_value(json!({"map": open_map(), "rules": fixture(), "units": units(json!([])), "events": [], "scripts": []})).unwrap();
    let mut replay = Battle::from_replay(&setup, &record).unwrap();
    while replay.tick() < b.tick() {
        replay.step();
    }
    assert_eq!(replay.digest(), b.digest());
}
