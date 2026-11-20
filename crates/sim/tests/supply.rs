//! Recovery with finite stock (slice 13): who is served, all at once, at
//! what price, and what never happens (resurrection, negative or regrown stock).
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{OwnUnit, ServiceStatus};
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

mod soldiers;

fn battle(units: Value, seed: u64) -> Battle {
    let map =
        json!({ "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    // These scenarios test service mechanics independently of balance tuning.
    for weapon in setup.rules.weapons.values_mut() {
        if matches!(weapon.ammo, contract::weapons::AmmoCapacity::Rounds(_)) {
            weapon.resupply_rounds_per_s = Some(1.0);
        }
    }
    setup.rules.service.vehicle_hp_per_s = 2.0;
    setup.rules.service.soldier_replacement_s = 5.0;
    // Service tests exercise a sustained fight, independently of range tuning.
    common::ground_reach(setup.rules.weapons.get_mut("rifle").unwrap(), 450.0);
    Battle::new(&setup, seed)
}

fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

fn service() -> Value {
    common::game()["service"].clone()
}

/// Ticks until a freshly placed supply truck is fully deployed.
fn deploy_ticks() -> u64 {
    let supply = common::rules()
        .catalog
        .by_id("test_supply")
        .capabilities
        .clone();
    (supply.deploy.unwrap().seconds * 30.0) as u64
}

/// A supply truck's stock at the start (its type's).
fn full_stock() -> u32 {
    common::rules()
        .catalog
        .by_id("test_supply")
        .capabilities
        .supply
        .unwrap()
        .stock
}

fn stock(b: &Battle) -> u32 {
    own(b, Side::Blue, 0).unwrap().stock.unwrap()
}

#[test]
fn finite_ammunition_requires_a_positive_authored_resupply_rate() {
    let mut rules = common::rules();
    for rate in [
        None,
        Some(0.0),
        Some(-1.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        rules.weapons.get_mut("atgm").unwrap().resupply_rounds_per_s = rate;
        let error = sim::admit(&rules).expect_err("invalid delivery rates must be refused");
        assert_eq!(error.path, "weapons.atgm.resupply_rounds_per_s");
    }
    rules.weapons.get_mut("atgm").unwrap().resupply_rounds_per_s = Some(0.25);
    sim::admit(&rules).expect("a positive delivery rate is admitted");
    rules
        .weapons
        .get_mut("rifle")
        .unwrap()
        .resupply_rounds_per_s = None;
    sim::admit(&rules).expect("unlimited weapons need no supply rate");
}

fn holding(kind: &str, x: f64, condition: Value) -> Value {
    json!({ "side": "blue", "kind": kind, "position": [x, 200], "engagement": "return_fire_only", "condition": condition })
}

fn truck(stock: Option<u32>) -> Value {
    match stock {
        Some(n) => {
            json!({ "side": "blue", "kind": "test_supply", "position": [100, 200], "stock": n })
        }
        None => json!({ "side": "blue", "kind": "test_supply", "position": [100, 200] }),
    }
}

#[test]
fn nothing_is_served_before_full_deployment() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_at", 130.0, json!({ "spent": { "atgm": 2 } }))
        ]),
        1,
    );
    run(&mut b, deploy_ticks() - 5);
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(at.service, ServiceStatus::SourceNotDeployed);
    assert_eq!(at.mounts[1].ammo, vec![Some(2)]);
    assert_eq!(stock(&b), full_stock());
    run(&mut b, 10);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().service,
        ServiceStatus::Serving
    );
}

#[test]
fn rounds_refill_to_capacity_at_their_price_and_rate() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_at", 130.0, json!({ "spent": { "atgm": 2 } }))
        ]),
        2,
    );
    let full = stock(&b);
    let cost = crate::common::rules().weapons["atgm"].round_cost.unwrap();
    let period = b.rules().tick_hz as u64;
    run(&mut b, deploy_ticks() + period + 2);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().mounts[1].ammo,
        vec![Some(3)]
    );
    assert_eq!(stock(&b), full - cost);
    run(&mut b, period * 3);
    // Capacity is the authored initial ammunition: no more than 4.
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(at.mounts[1].ammo, vec![Some(4)]);
    assert_eq!(
        stock(&b),
        full - 2 * cost,
        "only what was missing is paid for"
    );
    assert_eq!(at.service, ServiceStatus::Full);
}

fn concurrent_setup(stock: u32, rates: [f64; 3]) -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::test_rules();
    rules["service"]["radius_m"] = json!(80.0);
    for ((weapon, cost), rate) in [("tank_ap", 2), ("tank_he", 3), ("hmg", 1)]
        .into_iter()
        .zip(rates)
    {
        rules["weapons"][weapon]["ammo"] = json!(4);
        rules["weapons"][weapon]["round_cost"] = json!(cost);
        rules["weapons"][weapon]["resupply_rounds_per_s"] = json!(rate);
    }
    serde_json::from_value(json!({
        "rules": rules,
        "map": { "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "units": [
            truck(Some(stock)),
            holding("test_tank", 130.0,
                json!({"spent": {"tank_ap": 3, "tank_he": 3, "hmg": 3}}))
        ]
    }))
    .unwrap()
}

fn tank_ammo(b: &Battle) -> Vec<Vec<Option<u32>>> {
    own(b, Side::Blue, 1)
        .unwrap()
        .mounts
        .into_iter()
        .map(|m| m.ammo)
        .collect()
}

#[test]
fn all_mounts_and_ammunition_kinds_refill_together_at_their_rate_and_price() {
    let mut setup = concurrent_setup(100, [2.0, 1.0, 0.5]);
    // A second deployed source adds reserves, never another delivery clock.
    setup.units.push(
        serde_json::from_value(json!({
            "side": "blue", "kind": "test_supply", "position": [100, 240], "stock": 100
        }))
        .unwrap(),
    );
    let mut b = Battle::new(&setup, 2);
    let mut digests = Vec::new();
    // Include deployment and partial service progress in replay evidence.
    let mut served_ticks = 0;
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        digests.push(b.digest());
        if b.unit(UnitId(1)).unwrap().service == ServiceStatus::Serving {
            served_ticks += 1;
        }
        if served_ticks == b.rules().tick_hz / 2 - 1 {
            assert_eq!(tank_ammo(&b), vec![vec![Some(1), Some(1)], vec![Some(1)]]);
            assert_eq!(stock(&b), 100, "no payment before delivery");
        }
        if served_ticks == b.rules().tick_hz / 2 {
            assert_eq!(
                tank_ammo(&b),
                vec![vec![Some(2), Some(1)], vec![Some(1)]],
                "the fastest ammunition kind arrives first"
            );
            assert_eq!(stock(&b), 98, "only delivered rounds are paid for");
        }
        if served_ticks == b.rules().tick_hz {
            assert_eq!(tank_ammo(&b), vec![vec![Some(3), Some(2)], vec![Some(1)]]);
            assert_eq!(stock(&b), 93);
        }
        if served_ticks == 2 * b.rules().tick_hz {
            assert_eq!(tank_ammo(&b), vec![vec![Some(4), Some(3)], vec![Some(2)]]);
            assert_eq!(
                stock(&b),
                87,
                "each kind replenishes concurrently at its own rate"
            );
        }
    }
    assert!(served_ticks >= b.rules().tick_hz / 2, "service started");
    assert_eq!(tank_ammo(&b), vec![vec![Some(4), Some(4)], vec![Some(4)]]);
    assert_eq!(stock(&b), 82, "only missing rounds are paid for");
    assert_eq!(b.unit(UnitId(2)).unwrap().stock, Some(100));
    assert_eq!(b.unit(UnitId(1)).unwrap().service, ServiceStatus::Full);
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for expected in digests {
        replay.step();
        assert_eq!(replay.digest(), expected);
    }
}

#[test]
fn scarce_stock_is_paid_in_mount_order_and_each_kind_keeps_its_partial_progress() {
    let mut setup = concurrent_setup(3, [2.0; 3]);
    let reserve = serde_json::from_value(json!({
        "side": "blue", "kind": "test_supply", "position": [220, 240], "stock": 100
    }))
    .unwrap();
    setup.units.push(reserve);
    let mut b = Battle::new(&setup, 3);
    let hz = u64::from(b.rules().tick_hz);
    run(&mut b, 6 * hz);
    // AP pays first, then HE consumes the remainder; the HMG waits.
    assert_eq!(tank_ammo(&b), vec![vec![Some(2), Some(2)], vec![Some(1)]]);
    assert_eq!(
        stock(&b),
        0,
        "the final delivery consumes the remainder without overdrawing"
    );
    assert_eq!(b.unit(UnitId(1)).unwrap().service, ServiceStatus::NoStock);
    assert_eq!(
        b.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Move {
                units: vec![UnitId(2)],
                gesture: 1,
                goal: [160.0, 240.0],
                route: RoutePolicy::Shortest,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
            },
        })
        .error,
        None
    );
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
        if b.unit(UnitId(1)).unwrap().service == ServiceStatus::Serving {
            break;
        }
    }
    assert_eq!(
        b.unit(UnitId(1)).unwrap().service,
        ServiceStatus::Serving,
        "the reserve truck arrived and deployed: position {:?}, goal {:?}, deployment {:?}, stock {:?}",
        b.unit(UnitId(2)).unwrap().position,
        b.unit(UnitId(2)).unwrap().movement_goal(),
        b.unit(UnitId(2)).unwrap().deployment,
        b.unit(UnitId(2)).unwrap().stock,
    );
    assert_eq!(
        tank_ammo(&b),
        vec![vec![Some(2), Some(2)], vec![Some(2)]],
        "the HMG completes its paused delivery; the other kinds start new ones"
    );
    let period = b.rules().tick_hz / 2;
    run(&mut b, u64::from(period - 2));
    assert_eq!(tank_ammo(&b), vec![vec![Some(2), Some(2)], vec![Some(2)]]);
    b.step();
    assert_eq!(
        tank_ammo(&b),
        vec![vec![Some(3), Some(3)], vec![Some(2)]],
        "AP and HE keep their own clocks independently of the HMG"
    );
}

#[test]
fn one_supply_point_finishes_one_normal_delivery_and_replays_exactly() {
    let setup = concurrent_setup(1, [2.0; 3]);
    let mut b = Battle::new(&setup, 3);
    let period = b.rules().tick_hz / 2;
    let mut serving_ticks = 0;
    let mut digests = Vec::new();
    for _ in 0..5 * b.rules().tick_hz {
        b.step();
        digests.push(b.digest());
        if b.unit(UnitId(1)).unwrap().service == ServiceStatus::Serving {
            serving_ticks += 1;
        }
        if serving_ticks == period - 1 {
            assert_eq!(stock(&b), 1, "the remainder waits for normal delivery");
            assert_eq!(tank_ammo(&b), vec![vec![Some(1), Some(1)], vec![Some(1)]]);
        }
    }
    assert_eq!(serving_ticks, period, "no instant or extra delivery");
    assert_eq!(tank_ammo(&b), vec![vec![Some(2), Some(1)], vec![Some(1)]]);
    assert_eq!(stock(&b), 0);
    assert_eq!(b.unit(UnitId(1)).unwrap().service, ServiceStatus::NoStock);
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for expected in digests {
        replay.step();
        assert_eq!(replay.digest(), expected);
    }
}

#[test]
fn vehicles_are_repaired_but_the_truck_never_serves_itself() {
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "test_supply", "position": [100, 200], "condition": { "hp": 30 } },
            holding("test_tank", 130.0, json!({ "hp": 90 })),
        ]),
        3,
    );
    run(&mut b, deploy_ticks() + 30 * 6);
    let tank = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(tank.hp, 100.0, "repaired to the authored maximum");
    assert_eq!(own(&b, Side::Blue, 0).unwrap().hp, 30.0, "no self-repair");
    let per_hp = service()["stock_per_hp"].as_u64().unwrap() as u32;
    assert_eq!(stock(&b), full_stock() - 10 * per_hp);
}

#[test]
fn moving_recipients_wait() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_rifle", 140.0, json!({ "casualties": 1 })),
        ]),
        6,
    );
    run(&mut b, deploy_ticks() + 2);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [150.0, 300.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    run(&mut b, 3);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().service,
        ServiceStatus::Moving
    );
}

#[test]
fn incoming_fire_does_not_stop_service() {
    // A red scout spots for a red tank shelling blue's squad from 930 m; the
    // squad holds fire and could not reach anyway. Whenever the truck lives
    // through the shelling, the squad under fire is still reinforced. Where
    // the shells fall is chance, so the claim holds over several seeds.
    let map =
        json!({ "size": [1200, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    let units = json!([
        truck(None),
        // 70 m from the truck: inside its reach.
        holding("test_rifle", 170.0, json!({ "casualties": 7 })),
        { "side": "red", "kind": "test_recon", "position": [700, 260], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [1100, 200] },
    ]);
    let mut judged = 0;
    for seed in 1..=8 {
        let mut setup = common::scenario_with(&map, units.clone(), json!([]), json!([]));
        // Incoming fire is the control input; weapon balance is not under test.
        common::ground_reach(setup.rules.weapons.get_mut("tank_he").unwrap(), 1500.0);
        let mut b = Battle::new(&setup, seed);
        run(&mut b, deploy_ticks());
        let (mut under_fire, mut served_under_fire) = (false, false);
        for _ in 0..600 {
            b.step();
            if let Some(squad) = own(&b, Side::Blue, 1) {
                // Any near miss at all, below every tier too: the hidden level.
                let hit = b.unit(UnitId(1)).is_some_and(|u| u.suppression > 0.0);
                under_fire |= hit;
                served_under_fire |= hit && squad.service == ServiceStatus::Serving;
            }
        }
        if under_fire && own(&b, Side::Blue, 0).is_some() {
            judged += 1;
            assert!(
                served_under_fire,
                "seed {seed}: taking fire interrupted service"
            );
        }
    }
    assert!(judged > 0, "no seed kept the truck alive under fire");
}

#[test]
fn shared_stock_buys_a_final_round_with_the_remainder_then_stays_empty() {
    // Enough for one full-price round and a remainder toward the next.
    let cost = crate::common::rules().weapons["atgm"].round_cost.unwrap();
    let mut b = battle(
        json!([
            truck(Some(cost + 3)),
            holding("test_at", 130.0, json!({ "spent": { "atgm": 1 } })),
            holding("test_at", 140.0, json!({ "spent": { "atgm": 1 } })),
            holding("test_at", 150.0, json!({ "spent": { "atgm": 1 } })),
        ]),
        7,
    );
    run(&mut b, deploy_ticks() + 30 * 5);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().mounts[1].ammo,
        vec![Some(4)],
        "the lower id pays first"
    );
    assert_eq!(
        own(&b, Side::Blue, 2).unwrap().mounts[1].ammo,
        vec![Some(4)],
        "the remainder buys one final complete round"
    );
    let third = own(&b, Side::Blue, 3).unwrap();
    assert_eq!(third.mounts[1].ammo, vec![Some(3)]);
    assert_eq!(third.service, ServiceStatus::NoStock);
    assert_eq!(stock(&b), 0, "the final round consumes the remainder");
    run(&mut b, 30 * 30);
    assert_eq!(stock(&b), 0, "no regeneration");
    assert_eq!(
        own(&b, Side::Blue, 3).unwrap().mounts[1].ammo,
        vec![Some(3)]
    );
}

#[test]
fn a_moving_truck_serves_nobody() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_tank", 130.0, json!({ "hp": 50 }))
        ]),
        8,
    );
    run(&mut b, deploy_ticks() + 30);
    let hp = own(&b, Side::Blue, 1).unwrap().hp;
    assert!(hp > 50.0);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [300.0, 200.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    run(&mut b, 60);
    let tank = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(tank.hp, hp, "packing and moving: no service");
    assert_ne!(tank.service, ServiceStatus::Serving);
}

#[test]
fn service_repeats_from_its_seed() {
    let make = || {
        battle(
            json!([
                truck(None),
                holding("test_rifle", 130.0, json!({ "casualties": 3 })),
                holding("test_tank", 140.0, json!({ "hp": 40 }))
            ]),
            9,
        )
    };
    let (mut a, mut c) = (make(), make());
    for _ in 0..(deploy_ticks() + 300) {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}

#[test]
fn a_unit_is_served_while_firing_and_between_its_shots() {
    let mut rules = sim::fixtures::test_rules();
    rules["service"]["vehicle_hp_per_s"] = json!(2.0);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({"body": {"hull": {"hp": 100}}}),
    );
    // An unarmed target survives the whole fight and cannot disrupt service.
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({"body": {"hull": {"hp": 1.0e6}}}),
    );
    let setup = serde_json::from_value(json!({
        "rules": rules,
        "map": { "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "units": [
            truck(None),
            { "side": "blue", "kind": "test_jeep", "position": [140, 200], "condition": { "hp": 20 } },
            { "side": "red", "kind": "test_supply", "position": [300, 200] }
        ]
    })).unwrap();
    let mut b = Battle::new(&setup, 11);
    run(&mut b, deploy_ticks());
    let initial_hp = b.unit(UnitId(1)).unwrap().hp;
    let mut shots = b.unit(UnitId(1)).unwrap().mounts[0].shots;
    let (mut served_firing, mut served_between) = (false, false);
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        let unit = b.unit(UnitId(1)).unwrap();
        let now = unit.mounts[0].shots;
        if sim::weapons::engaged(unit) {
            assert_eq!(unit.service, ServiceStatus::Serving);
            served_firing |= now > shots;
            served_between |= now == shots;
        }
        shots = now;
    }
    assert!(served_firing, "service continues on firing ticks");
    assert!(served_between, "service continues between shots");
    assert!(
        b.unit(UnitId(1)).unwrap().hp > initial_hp,
        "repaired during combat"
    );
}

#[test]
fn an_empty_truck_never_blocks_a_stocked_one() {
    let mut b = battle(
        json!([
            truck(Some(0)),
            { "side": "blue", "kind": "test_supply", "position": [110, 230] },
            holding("test_tank", 130.0, json!({ "hp": 90 })),
        ]),
        12,
    );
    run(&mut b, deploy_ticks() + 30 * 6);
    assert_eq!(own(&b, Side::Blue, 2).unwrap().hp, 100.0);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().stock.unwrap(),
        full_stock() - 10
    );
}

#[test]
fn trucks_are_never_serviced_even_by_each_other() {
    let mut b = battle(
        json!([
            truck(None),
            { "side": "blue", "kind": "test_supply", "position": [110, 230], "condition": { "hp": 20 } },
        ]),
        13,
    );
    run(&mut b, deploy_ticks() + 30 * 10);
    assert_eq!(own(&b, Side::Blue, 1).unwrap().hp, 20.0);
}

/// Every need is served at once, each on its own clock: a squad's replacement
/// and a tank's repair never wait for their ammunition.
#[test]
fn every_need_is_served_at_once_and_an_empty_launcher_reloads() {
    // All four missiles spent and one soldier down; a damaged tank half out of
    // armour-piercing rounds.
    let mut b = battle(
        json!([
            truck(None),
            holding(
                "test_at",
                130.0,
                json!({ "spent": { "atgm": 4 }, "casualties": 1 })
            ),
            holding(
                "test_tank",
                150.0,
                json!({ "hp": 90, "spent": { "tank_ap": 10 } })
            ),
        ]),
        14,
    );
    assert_eq!(own(&b, Side::Blue, 1).unwrap().mounts[1].loaded, None);
    let hz = b.rules().tick_hz as f64;
    let service = b.rules().service.clone();
    let replacement = (service.soldier_replacement_s * hz) as u64;
    let repair = (10.0 / service.vehicle_hp_per_s * hz) as u64;
    let ap_full = (10.0 / b.rules().weapons["tank_ap"].resupply_rounds_per_s.unwrap() * hz) as u64;
    assert!(
        replacement.max(repair) < ap_full,
        "the scene needs the tank's rounds to outlast the other services"
    );
    run(&mut b, deploy_ticks() + replacement.max(repair) + 4);
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(
        at.members.len(),
        3,
        "the replacement joins on his own clock"
    );
    let tank = own(&b, Side::Blue, 2).unwrap();
    assert_eq!(tank.hp, 100.0, "repaired while the rounds still come");
    assert!(tank.mounts[0].ammo[0] < Some(20), "rounds still coming");
    run(&mut b, ap_full);
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(at.mounts[1].ammo, vec![Some(4)]);
    assert!(
        at.mounts[1].loaded.is_some(),
        "the refilled launcher reloads"
    );
    assert_eq!(at.service, ServiceStatus::Full);
    assert_eq!(own(&b, Side::Blue, 2).unwrap().mounts[0].ammo[0], Some(20));
}

#[test]
fn a_truck_on_the_move_serves_nobody() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_tank", 130.0, json!({ "hp": 50 }))
        ]),
        16,
    );
    run(&mut b, deploy_ticks() + 30);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [140.0, 260.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    // Pack fully, then drive: still in reach, never serving.
    run(&mut b, deploy_ticks() + 5);
    let hp = own(&b, Side::Blue, 1).unwrap().hp;
    let at0 = own(&b, Side::Blue, 0).unwrap().position;
    run(&mut b, 60);
    assert_ne!(
        own(&b, Side::Blue, 0).unwrap().position,
        at0,
        "the truck is driving"
    );
    assert_eq!(own(&b, Side::Blue, 1).unwrap().hp, hp);
}

/// A truck refills a hulled unit's spent flares as it does a Trophy's
/// charges: one per flare service time, at the flare price, up to capacity.
#[test]
fn test_a_truck_refills_a_vehicles_spent_flares() {
    let mut rules = sim::fixtures::test_rules();
    rules["service"]["flare_service_s"] = json!(2.0);
    rules["service"]["stock_per_flare"] = json!(3);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "capabilities": { "flares": { "capacity": 2, "reload_s": 30 } } }),
    );
    let setup = serde_json::from_value(json!({
        "rules": rules,
        "map": { "size": [1000, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "units": [
            { "side": "blue", "kind": "test_supply", "position": [100, 200], "stock": 40 },
            { "side": "blue", "kind": "test_jeep", "position": [120, 200] }
        ]
    }))
    .unwrap();
    let battle = Battle::new(&setup, 7);
    let hz = u64::from(battle.rules().tick_hz);
    let mut units: Vec<_> = (0..2)
        .map(|i| battle.unit(UnitId(i)).unwrap().clone())
        .collect();
    let deployment = units[0].deployment.as_mut().unwrap();
    deployment.current = deployment.duration;
    units[1].flares = Some(sim::units::Flares {
        charges: 0,
        ready_tick: 0,
    });
    let mut next_soldier = 0;
    let mut serve = |units: &mut [sim::units::Unit], ticks: u64| {
        for _ in 0..ticks {
            sim::supply::service(
                battle.world(),
                units,
                battle.windows(),
                battle.arsenal(),
                battle.rules(),
                &[false, false],
                &mut next_soldier,
            );
        }
    };
    let state = |units: &[sim::units::Unit]| (units[1].flares.unwrap().charges, units[0].stock);
    serve(&mut units, 2 * hz - 1);
    assert_eq!(state(&units), (0, Some(40)), "the first flare takes 2 s");
    serve(&mut units, 1);
    assert_eq!(state(&units), (1, Some(37)), "one flare for 3 stock");
    serve(&mut units, 2 * hz);
    assert_eq!(state(&units), (2, Some(34)));
    serve(&mut units, 4 * hz);
    assert_eq!(state(&units), (2, Some(34)), "never past capacity");
    assert_eq!(units[1].service, ServiceStatus::Full);
}
