//! Slice 02: a supply helicopter brakes, lands, serves, and packs again.

use super::air::{battle_with, fixture, open_map, order};
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use contract::observation::ServiceStatus;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::v2;

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
    for _ in 0..12 * b.rules().tick_hz as u64 {
        b.step();
        if b.unit(HELI).unwrap().landed() {
            break;
        }
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
            json!({"side":"red","kind":"test_rifle","position":[500,200],"engagement":"return_fire_only"})
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
    assert!(
        b.unit(HELI).unwrap().deployment.as_ref().unwrap().current > 0,
        "braking ends and deployment starts"
    );
}

#[test]
fn its_height_falls_with_setup_and_it_touches_down_before_it_serves() {
    let mut b = battle(json!([json!({
        "side": "blue", "kind": "test_tank", "position": [250, 200],
        "condition": {"hp": 1}, "engagement": "return_fire_only"
    })]));
    deploy(&mut b, true);
    let (setup, _, drop) = timing(&b);
    b.step();
    assert!(b.unit(HELI).unwrap().landing_goal.is_some());
    assert_eq!(
        b.unit(HELI).unwrap().deployment.as_ref().unwrap().current,
        0
    );
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
    b.step();
    assert!(b.unit(HELI).unwrap().landing_goal.is_some());
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
        json!({"side":"blue","kind":"test_heli","position":[205,200]}),
        json!({"side":"blue","kind":"test_supply","position":[205,200]})
    ]));
    set_up(&mut b);
    let at = b.unit(HELI).unwrap().position.xy();
    let hover = at + v2(3.0, 0.0);
    assert_eq!(b.accept(order(&[1], [hover.x, hover.y])).error, None);
    for _ in 0..20 * b.rules().tick_hz {
        b.step();
        if b.unit(UnitId(1)).unwrap().movement_goal().is_none() {
            break;
        }
    }
    assert!((b.unit(UnitId(1)).unwrap().position.xy() - hover).length() < 0.1);
    let neighbour = b.unit(UnitId(1)).unwrap().position.xy();
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    assert_eq!(b.unit(HELI).unwrap().position.xy(), at);
    assert_eq!(b.unit(UnitId(1)).unwrap().position.xy(), neighbour);
}

#[test]
fn the_low_hover_clears_a_landed_heli() {
    let mut b = battle(json!([
        json!({"side":"blue","kind":"test_heli","position":[205,200],"condition":{"hp":1},"engagement":"return_fire_only"})
    ]));
    set_up(&mut b);
    for _ in 0..5 * b.rules().tick_hz {
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

fn prop_ring_map(b: &Battle, margin: f64) -> Value {
    let reach = b.rules().air.rotor.landing_reach_m;
    let props = vec![json!({
        "kind": "crate", "center": [200.0, 200.0], "yaw": 0,
        "half_extents": [reach + margin, reach + margin, 1.0]
    })];
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
}

#[test]
fn ordered_onto_an_obstacle_it_lands_on_the_nearest_clear_spot() {
    let mut b = battle_with(
        &fixture(),
        json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": [{"kind":"crate","center":[200,200],"yaw":0,"half_extents":[8,8,1]}] }),
        units(json!([])),
    );
    assert_eq!(b.accept(order(&[0], [200.0, 200.0])).error, None);
    for _ in 0..20 * b.rules().tick_hz {
        b.step();
    }
    let heli = b.unit(HELI).unwrap();
    assert!(heli.landed());
    let at = heli.position.xy();
    let radius = b
        .rules()
        .catalog
        .by_id("test_supply_heli")
        .landing()
        .unwrap()
        .rotor_radius_m;
    assert!((at - v2(200.0, 200.0)).length() <= b.rules().air.rotor.landing_reach_m + 1e-6);
    assert!(
        b.world()
            .props_near(at, radius)
            .iter()
            .all(|p| !p.footprint().contains(at, 0.0)),
        "heli={heli:?} at={at:?} props={:?}",
        b.world()
            .props_near(at, radius)
            .iter()
            .map(|p| p.center)
            .collect::<Vec<_>>()
    );
}

#[test]
fn with_no_clear_spot_in_reach_it_stays_packed_aloft_and_says_why() {
    let probe = battle(json!([]));
    let map = prop_ring_map(&probe, 0.5);
    let mut b = battle_with(&fixture(), map, units(json!([])));
    assert_eq!(b.accept(order(&[0], [200.0, 200.0])).error, None);
    for _ in 0..5 * b.rules().tick_hz {
        b.step();
    }
    let heli = b.unit(HELI).unwrap();
    assert!(!heli.landed());
    assert_eq!(heli.state, contract::observation::MoveState::NoLandingSpot);
}

#[test]
fn the_search_is_deterministic() {
    let map = open_map();
    let mut a = battle_with(&fixture(), map.clone(), units(json!([])));
    let mut b = battle_with(&fixture(), map, units(json!([])));
    a.accept(order(&[0], [200.0, 200.0]));
    b.accept(order(&[0], [200.0, 200.0]));
    for _ in 0..10 * a.rules().tick_hz {
        a.step();
        b.step();
    }
    assert_eq!(a.digest(), b.digest());
}

/// Told to stop at cruise, it glides a long way before it hovers (D2). The
/// spot it lands on is chosen where it comes to rest, so a body lying there
/// sends it to the nearest clear spot rather than under it.
#[test]
fn stopped_at_cruise_it_lands_clear_of_what_lies_where_it_comes_to_rest() {
    let start = json!([{"side":"blue","kind":"test_supply_heli","position":[40,200]}]);
    let fly_and_stop = |b: &mut Battle| {
        assert_eq!(b.accept(order(&[0], [590.0, 200.0])).error, None);
        for _ in 0..b.rules().tick_hz * 3 / 2 {
            b.step();
        }
        deploy(b, true);
    };
    // Where it comes to rest with nothing in the way.
    let mut probe = battle_with(&fixture(), open_map(), start.clone());
    fly_and_stop(&mut probe);
    while probe
        .unit(HELI)
        .unwrap()
        .air
        .unwrap()
        .velocity
        .xy()
        .length()
        > 1e-6
    {
        probe.step();
    }
    let rest = probe.unit(HELI).unwrap().position.xy();
    // The same flight with a crate lying there.
    let mut map = open_map();
    map["props"] =
        json!([{"kind":"crate","center":[rest.x, rest.y],"yaw":0,"half_extents":[8,8,1]}]);
    let mut b = battle_with(&fixture(), map, start);
    fly_and_stop(&mut b);
    for _ in 0..30 * b.rules().tick_hz {
        b.step();
        if b.unit(HELI).unwrap().landed() {
            break;
        }
    }
    let heli = b.unit(HELI).unwrap();
    assert!(heli.landed(), "it lands: {heli:?}");
    let at = heli.position.xy();
    assert!(
        b.world()
            .props_near(at, 0.0)
            .iter()
            .all(|p| !p.footprint().contains(at, 0.0)),
        "landed on the crate at {at:?} (rest {rest:?})"
    );
    assert!((at - rest).length() <= b.rules().air.rotor.landing_reach_m + 1e-6);
}

#[test]
fn a_shift_queued_flight_checks_a_fresh_landing_spot_at_its_destination() {
    let mut map = open_map();
    let goal = [400.0, 200.0];
    map["props"] = json!([{"kind":"crate","center":goal,"yaw":0,"half_extents":[8,8,1]}]);
    let mut b = battle_with(&fixture(), map, units(json!([])));
    set_up(&mut b);
    let mut command = order(&[0], goal);
    command.queued = true;
    assert_eq!(b.accept(command).error, None);
    let mut took_off = false;
    let mut landed_again = false;
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(HELI).unwrap();
        took_off |= !heli.landed();
        if took_off && heli.landed() {
            let at = heli.position.xy();
            assert!(
                b.world()
                    .props_near(at, 0.0)
                    .iter()
                    .all(|p| !p.footprint().contains(at, 0.0)),
                "landed on obstacle at {at:?}"
            );
            assert!((at - v2(goal[0], goal[1])).length() <= b.rules().air.rotor.landing_reach_m);
            landed_again = true;
            break;
        }
    }
    assert!(landed_again, "the queued flight lands near its destination");
}

#[test]
fn without_a_landing_spot_it_still_climbs_clear_of_the_obstacle() {
    let probe = battle(json!([]));
    let mut map = prop_ring_map(&probe, 1.0);
    map["props"][0]["half_extents"][2] = json!(40);
    let mut b = battle_with(&fixture(), map, units(json!([])));
    let start_z = b.unit(HELI).unwrap().position.z;
    for _ in 0..5 * b.rules().tick_hz {
        b.step();
    }
    let heli = b.unit(HELI).unwrap();
    assert_eq!(heli.state, contract::observation::MoveState::NoLandingSpot);
    assert!(
        heli.position.z > start_z,
        "a refused landing continues normal height following"
    );
}

#[test]
fn landing_waits_at_a_checked_spot_on_an_unequal_tick_lattice() {
    let mut rules = fixture();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply_heli",
        json!({
            "capabilities":{"deploy":{"seconds":2,"pack_seconds":3,"drop_seconds":1}}
        }),
    );
    for blocked in [false, true] {
        let mut map = open_map();
        if blocked {
            map["props"] =
                json!([{"kind":"crate","center":[200,200],"yaw":0,"half_extents":[8,8,1]}]);
        }
        let mut b = battle_with(&rules, map, units(json!([])));
        let mut ready = false;
        for _ in 0..30 * b.rules().tick_hz {
            b.step();
            let heli = b.unit(HELI).unwrap();
            let d = heli.deployment.as_ref().unwrap();
            if d.current > 0 {
                let goal = heli
                    .landing_goal
                    .expect("deployment requires a checked spot");
                assert!(
                    (heli.position.xy() - goal).length() <= 0.05,
                    "deployment waits for the hop"
                );
                assert!(!heli.gliding(), "deployment waits for braking");
            }
            if d.fully_deployed() {
                ready = true;
                break;
            }
        }
        assert!(
            ready,
            "a checked clear spot eventually sets up, blocked={blocked}"
        );
    }
}

#[test]
fn a_landing_hop_never_flies_through_an_air_blocking_wall() {
    let mut rules = fixture();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply_heli",
        json!({
            "mobility":{"air":{"rotor_radius_m":15}}
        }),
    );
    let mut map = open_map();
    map["props"] = json!([
        {"kind":"crate","center":[179,200],"yaw":0,"half_extents":[20,100,1]},
        {"kind":"crate","center":[216,200],"yaw":0,"half_extents":[1,100,30]}
    ]);
    let mut b = battle_with(&rules, map, units(json!([])));
    for _ in 0..15 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(HELI).unwrap();
        let wall = b.world().props().find(|p| p.center.x == 216.0).unwrap();
        assert!(
            !wall.footprint().contains(heli.position.xy(), 0.0) || heli.position.z >= wall.top_z(),
            "the direct hop crossed a wall below its top: {:?}",
            heli.position
        );
    }
    assert_eq!(
        b.unit(HELI).unwrap().state,
        contract::observation::MoveState::NoLandingSpot
    );
}

#[test]
fn a_landing_hop_can_escape_a_wall_and_its_routing_margin() {
    let mut map = open_map();
    map["props"] = json!([
        {"kind":"crate","center":[205,200],"yaw":0,"half_extents":[1,100,30]}
    ]);
    for start_x in [200.0, 205.0] {
        let mut b = battle_with(
            &fixture(),
            map.clone(),
            json!([
                {"side":"blue","kind":"test_supply_heli","position":[start_x,200]}
            ]),
        );
        for _ in 0..15 * b.rules().tick_hz {
            b.step();
        }
        let heli = b.unit(HELI).unwrap();
        assert!(
            heli.landed(),
            "it can hop away from a nearby wall: {heli:?}"
        );
        let radius = b
            .rules()
            .catalog
            .by_id("test_supply_heli")
            .landing()
            .unwrap()
            .rotor_radius_m;
        assert!(b
            .world()
            .props()
            .all(|p| p.footprint().distance(heli.position.xy()) > radius));
    }
}
