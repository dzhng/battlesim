//! Slice 04: fire gates, interception and sensor ranges use slant distance.

use crate::common;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ContactSource;
use serde_json::{json, Value};
use sim::battle::Battle;

fn map(size: [f64; 2]) -> String {
    json!({
        "size": size,
        "fog_cell_m": 8,
        "height_grid_m": 4,
        "slope_cutoff_deg": 35
    })
    .to_string()
}

fn high_air_fixture() -> Value {
    let mut fixture = common::game();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "sensors": { "ground_m": 700, "high_air_m": 3000 }
        }),
    );
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_jeep",
        json!({
            "sensors": { "ground_m": 1000, "low_air_m": 3000, "high_air_m": 3000 },
            "capabilities": {
                "radar_emitter": true
            },
            "mounts": [{
                "id": "AA", "name": "AA", "weapons": ["aa_ballistic"],
                "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32]
            }]
        }),
    );
    fixture
}

fn battle(fixture: &Value, units: Value, map: &str) -> Battle {
    battle_with_events(fixture, units, map, json!([]))
}

fn battle_with_events(fixture: &Value, units: Value, map: &str, events: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(map).unwrap(),
        "rules": fixture,
        "units": units,
        "events": events,
        "scripts": []
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

fn attack_identified(b: &mut Battle, unit: UnitId, target: UnitId) {
    attack_identified_for(b, Side::Blue, 1, unit, target);
}

fn attack_identified_for(b: &mut Battle, side: Side, seq: u64, unit: UnitId, target: UnitId) {
    b.step();
    let id = b
        .observe(side)
        .identified
        .first()
        .map(|seen| seen.id)
        .unwrap_or_else(|| panic!("{side:?} identifies target unit {target:?}"));
    let ack = b.accept(CommandEnvelope {
        side,
        player: None,
        seq,
        order: Order::Attack {
            units: vec![unit],
            target: TargetRef::Identified { id },
        },
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

fn aa_shots(b: &Battle, unit: UnitId) -> u32 {
    let unit_ref = b.unit(unit).expect("AA unit");
    let specs = b.rules().catalog.mounts(unit_ref.kind);
    unit_ref
        .mounts
        .iter()
        .zip(specs)
        .filter(|(_, spec)| matches!(spec.def.id.as_str(), "AA" | "AA launcher"))
        .map(|(mount, _)| mount.shots)
        .sum()
}

#[test]
fn reach_and_fire_gate_agree_so_nothing_freezes() {
    let fixture = high_air_fixture();
    let mut b = battle(
        &fixture,
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [100, 300] },
            // Flying away: where a shot would meet it lies past reach too.
            { "side": "red", "kind": "test_heli", "position": [1590, 300], "yaw": 0.0 }
        ]),
        &map([2200.0, 600.0]),
    );
    attack_identified(&mut b, UnitId(0), UnitId(1));
    for _ in 0..120 {
        b.step();
    }
    assert!(
        b.unit(UnitId(0)).unwrap().position.x > 100.0,
        "the ordered AA unit must close on an overhead target outside slant reach"
    );
}

#[test]
fn a_gun_holds_fire_at_a_jet_out_of_3d_reach_overhead() {
    let fixture = high_air_fixture();
    let mut b = battle(
        &fixture,
        json!([
            { "side": "blue", "kind": "test_jeep", "position": [100, 300] },
            { "side": "red", "kind": "test_jet", "position": [1585, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" }
        ]),
        &map([2200.0, 2200.0]),
    );
    for _ in 0..20 {
        b.step();
    }
    assert_eq!(
        aa_shots(&b, UnitId(0)),
        0,
        "the AA gun fired beyond 3D reach"
    );
}

fn interceptor_case(high: bool) -> Battle {
    let mut fixture = high_air_fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "soldiers",
        "test_aa_gunner",
        json!({ "mounts": [{ "id": "AA launcher", "weapons": ["aa_ballistic"] }] }),
    );
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_aa",
        json!({ "sensors": { "high_air_m": 1200 }, "capabilities": { "radar_emitter": true } }),
    );
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "capabilities": { "air": {
                "fuel_capacity_s": 180,
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }},
            "mounts": [{
                "id": "incoming", "name": "Incoming", "weapons": ["stinger"],
                "turret": true, "pivot_m": [0, 0, 1.25], "muzzle_m": [1.2, 0, 0]
            }]
        }),
    );
    fixture["weapons"]["stinger"]["interceptable"] = json!(true);
    fixture["weapons"]["stinger"]["targets"] = json!({
        "ground": { "range_m": 1500, "scatter_mrad": 0.5, "classes": ["infantry", "vehicle"] },
        "low_air": { "classes": ["air_rotor", "air_fixed_wing"], "range_m": 1500, "scatter_mrad": 0.5 },
        "high_air": { "classes": ["air_rotor", "air_fixed_wing"], "range_m": 1500, "scatter_mrad": 0.5 }
    });
    fixture["weapons"]["stinger"]["damage"] = json!(0);
    fixture["weapons"]["stinger"]["structural_damage"] = json!(0);
    fixture["weapons"]["stinger"]["blast_radius_m"] = json!(0);
    if !high {
        sim::fixtures::patch_catalog(
            &mut fixture,
            "units",
            "test_heli",
            json!({ "mobility": { "air": { "layer": "low_air", "flight": "rotor" } } }),
        );
    }
    let units = json!([
        { "side": "blue", "kind": "test_aa", "position": [800, 300] },
        { "side": "red", "kind": "test_heli", "position": [1000, 300], "yaw": std::f64::consts::PI }
    ]);
    battle_with_events(
        &fixture,
        units,
        &map([1800.0, 600.0]),
        json!([{ "tick": 5, "fire": { "unit": 1 } }]),
    )
}

#[test]
fn an_interceptor_engages_an_incoming_missile_at_any_height() {
    for high in [false, true] {
        let mut b = interceptor_case(high);
        let launched_before = b.load().rounds_launched;
        for _ in 0..240 {
            b.step();
            if aa_shots(&b, UnitId(0)) > 0 {
                break;
            }
        }
        assert!(
            b.load().rounds_launched > launched_before,
            "the {} incoming missile never flew",
            if high { "high" } else { "low" }
        );
        assert!(
            aa_shots(&b, UnitId(0)) > 0,
            "the interceptor did not engage the {} incoming round",
            if high { "high" } else { "low" }
        );
    }
}

#[test]
fn radar_detection_range_is_3d() {
    let mut fixture = high_air_fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "capabilities": { "radar_receiver": { "range_m": 450 } }
        }),
    );
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_jeep",
        json!({
            "capabilities": {
                "radar_emitter": true
            }
        }),
    );
    let near = battle(
        &fixture,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [100, 300] },
            { "side": "red", "kind": "test_jeep", "position": [400, 300] }
        ]),
        &map([800.0, 600.0]),
    );
    assert_eq!(
        near.observe(Side::Blue)
            .contacts
            .iter()
            .filter(|contact| contact.source == ContactSource::Radar)
            .count(),
        1
    );
    let far = battle(
        &fixture,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [100, 300] },
            { "side": "red", "kind": "test_jeep", "position": [500, 300] }
        ]),
        &map([800.0, 600.0]),
    );
    assert!(far
        .observe(Side::Blue)
        .contacts
        .iter()
        .all(|contact| contact.source != ContactSource::Radar));
}

#[test]
fn a_ground_order_on_a_hill_stays_ground() {
    let fixture = common::game();
    let map = json!({
        "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "relief": [{ "kind": "mesa", "rect": [0, 0, 800, 600], "height_m": 20, "side_degrees": 45 }]
    })
    .to_string();
    let mut b = battle(
        &fixture,
        json!([{ "side": "blue", "kind": "test_jeep", "position": [350, 300] }]),
        &map,
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [400.0, 300.0, 999.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    for _ in 0..30 {
        b.step();
    }
    let target = b.observe(Side::Blue).own[0].mounts[0]
        .target
        .as_ref()
        .expect("ground order remains visible on the mount");
    assert!(matches!(target, TargetRef::Ground { point } if point[2] > 0.0));
}
