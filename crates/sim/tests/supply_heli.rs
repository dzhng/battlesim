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
