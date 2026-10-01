//! Recovery with finite stock (slice 13): who is served, in what order, at
//! what price, and what never happens (resurrection, negative or regrown stock).
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{OwnUnit, ServiceStatus};
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

fn battle(units: Value, seed: u64) -> Battle {
    let map =
        json!({ "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    // Service tests exercise a sustained fight, independently of range tuning.
    setup
        .rules
        .weapons
        .get_mut("rifle")
        .unwrap()
        .ballistics
        .range_m = 450.0;
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
    common::village()["service"].clone()
}

/// Ticks until a freshly placed supply truck is fully deployed.
fn deploy_ticks() -> u64 {
    let supply = common::rules().catalog.by_id("supply").capabilities.clone();
    (supply.deploy.unwrap().seconds * 30.0) as u64
}

/// A supply truck's stock at the start (its type's).
fn full_stock() -> u32 {
    common::rules()
        .catalog
        .by_id("supply")
        .capabilities
        .supply
        .unwrap()
        .stock
}

fn stock(b: &Battle) -> u32 {
    own(b, Side::Blue, 0).unwrap().stock.unwrap()
}

fn holding(kind: &str, x: f64, condition: Value) -> Value {
    json!({ "side": "blue", "kind": kind, "position": [x, 200], "engagement": "return_fire_only", "condition": condition })
}

fn truck(stock: Option<u32>) -> Value {
    match stock {
        Some(n) => json!({ "side": "blue", "kind": "supply", "position": [100, 200], "stock": n }),
        None => json!({ "side": "blue", "kind": "supply", "position": [100, 200] }),
    }
}

#[test]
fn nothing_is_served_before_full_deployment() {
    let mut b = battle(
        json!([
            truck(None),
            holding("at", 130.0, json!({ "spent": { "atgm": 2 } }))
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
            holding("at", 130.0, json!({ "spent": { "atgm": 2 } }))
        ]),
        2,
    );
    let full = stock(&b);
    let cost = service()["round_costs"]["atgm"].as_u64().unwrap() as u32;
    let period = (30.0 / service()["ammo_rounds_per_s"].as_f64().unwrap()) as u64;
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

#[test]
fn vehicles_are_repaired_but_the_truck_never_serves_itself() {
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "supply", "position": [100, 200], "condition": { "hp": 30 } },
            holding("tank", 130.0, json!({ "hp": 90 })),
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
fn casualties_are_replaced_by_new_soldiers_and_the_fallen_stay() {
    let mut b = battle(
        json!([
            truck(None),
            holding("rifle", 130.0, json!({ "casualties": 2 }))
        ]),
        4,
    );
    let area = own(&b, Side::Blue, 1).unwrap().area;
    let corpses: Vec<_> = b.observe(Side::Blue).corpses.clone();
    assert_eq!(corpses.len(), 2);
    let every = service()["soldier_replacement_s"].as_f64().unwrap();
    run(&mut b, deploy_ticks() + (every * 30.0) as u64 * 2 + 4);
    let squad = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(squad.members.len(), 8, "back to authored strength");
    assert_eq!(
        squad.area, area,
        "replacements do not enlarge the squad area"
    );
    // Replacements join on free ground, spaced from their squadmates.
    let spacing = common::village()["infantry_movement"]["spacing_m"]
        .as_f64()
        .unwrap();
    for (i, p) in squad.members.iter().enumerate() {
        for q in &squad.members[i + 1..] {
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            assert!(d >= spacing - 1e-9, "two soldiers {d:.2} m apart");
        }
    }
    assert_eq!(
        b.observe(Side::Blue).corpses,
        corpses,
        "the old records remain"
    );
    assert_eq!(squad.service, ServiceStatus::Full);
}

#[test]
fn a_garrisoned_squad_is_reinforced_inside_its_building() {
    // A rifle squad two men short holds the building beside a deployed truck.
    // Its replacements join it inside, each on a free facade slot of the
    // building, where the garrison's own soldiers stand.
    let map = json!({ "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [{ "kind": "building", "center": [150, 200], "yaw": 0, "half_extents": [12, 9, 4] }] })
    .to_string();
    let units = json!([
        truck(None),
        { "side": "blue", "kind": "rifle", "position": [150, 214], "engagement": "return_fire_only",
          "condition": { "casualties": 2 } },
    ]);
    let scripts = json!([{ "tick": 1, "side": "blue", "order": { "kind": "garrison", "units": [1], "building": 0 } }]);
    let mut b = Battle::new(&common::scenario_with(&map, units, json!([]), scripts), 4);
    let every = service()["soldier_replacement_s"].as_f64().unwrap();
    let enter = common::village()["garrison"]["enter_exit_s"]
        .as_f64()
        .unwrap();
    run(
        &mut b,
        deploy_ticks().max((enter * 30.0) as u64 + 30) + (every * 30.0) as u64 * 2 + 30,
    );
    let squad = b.unit(UnitId(1)).unwrap();
    assert!(squad.garrisoned(), "the squad holds the building");
    assert_eq!(
        squad.members.iter().filter(|s| s.alive()).count(),
        8,
        "back to authored strength inside"
    );
    let g = squad.garrison.as_ref().unwrap();
    let mut seats = Vec::new();
    for (k, s) in squad.members.iter().enumerate().filter(|(_, s)| s.alive()) {
        let seat = g.seat(k).expect("every living soldier holds a slot");
        assert_eq!(s.position, seat.position, "each stands on his slot");
        seats.push(g.seats[k]);
    }
    seats.sort();
    seats.dedup();
    assert_eq!(seats.len(), 8, "no two share a slot");
    assert_eq!(own(&b, Side::Blue, 1).unwrap().service, ServiceStatus::Full);
}

#[test]
fn an_eliminated_squad_is_never_resurrected() {
    let mut b = battle(
        json!([
            truck(None),
            holding("at", 130.0, json!({ "casualties": 3 }))
        ]),
        5,
    );
    run(&mut b, deploy_ticks() + 30 * 20);
    assert!(own(&b, Side::Blue, 1).is_none());
    assert_eq!(stock(&b), full_stock());
}

#[test]
fn moving_or_firing_recipients_wait() {
    let mut b = battle(
        json!([
            truck(None),
            { "side": "blue", "kind": "rifle", "position": [140, 230], "condition": { "casualties": 1 } },
            // The rifles plink at a tank they cannot hurt (W09).
            { "side": "red", "kind": "tank", "position": [480, 230], "engagement": "return_fire_only" },
        ]),
        6,
    );
    run(&mut b, deploy_ticks() + 2);
    let mut firing_seen = false;
    for _ in 0..120 {
        b.step();
        firing_seen |= own(&b, Side::Blue, 1).is_some_and(|u| u.service == ServiceStatus::Firing);
    }
    assert!(firing_seen, "a recipient firing this tick is not served");
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
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
        holding("rifle", 170.0, json!({ "casualties": 7 })),
        { "side": "red", "kind": "recon", "position": [700, 260], "engagement": "return_fire_only" },
        { "side": "red", "kind": "tank", "position": [1100, 200] },
    ]);
    let mut judged = 0;
    for seed in 1..=8 {
        let mut setup = common::scenario_with(&map, units.clone(), json!([]), json!([]));
        // Incoming fire is the control input; weapon balance is not under test.
        setup
            .rules
            .weapons
            .get_mut("tank_he")
            .unwrap()
            .ballistics
            .range_m = 1500.0;
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
fn shared_stock_is_paid_whole_in_unit_order_and_never_regrows() {
    // Stock for one ATGM round (20) only; two AT teams both need one.
    let cost = service()["round_costs"]["atgm"].as_u64().unwrap() as u32;
    let mut b = battle(
        json!([
            truck(Some(cost + 3)),
            holding("at", 130.0, json!({ "spent": { "atgm": 1 } })),
            holding("at", 140.0, json!({ "spent": { "atgm": 1 } })),
        ]),
        7,
    );
    run(&mut b, deploy_ticks() + 30 * 5);
    assert_eq!(
        own(&b, Side::Blue, 1).unwrap().mounts[1].ammo,
        vec![Some(4)],
        "the lower id pays first"
    );
    let second = own(&b, Side::Blue, 2).unwrap();
    assert_eq!(second.mounts[1].ammo, vec![Some(3)]);
    assert_eq!(second.service, ServiceStatus::NoStock);
    assert_eq!(stock(&b), 3, "never partly paid, never below zero");
    run(&mut b, 30 * 30);
    assert_eq!(stock(&b), 3, "no regeneration");
}

#[test]
fn a_moving_truck_serves_nobody() {
    let mut b = battle(
        json!([truck(None), holding("tank", 130.0, json!({ "hp": 50 }))]),
        8,
    );
    run(&mut b, deploy_ticks() + 30);
    let hp = own(&b, Side::Blue, 1).unwrap().hp;
    assert!(hp > 50.0);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
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
                holding("rifle", 130.0, json!({ "casualties": 3 })),
                holding("tank", 140.0, json!({ "hp": 40 }))
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
fn a_unit_in_a_fight_is_not_served_between_its_shots() {
    // Rifles plinking at a tank they cannot hurt (W09) stay engaged the whole
    // time: no replacement arrives while they fight.
    let mut b = battle(
        json!([
            truck(None),
            { "side": "blue", "kind": "rifle", "position": [140, 230], "condition": { "casualties": 2 } },
            { "side": "red", "kind": "tank", "position": [480, 230], "engagement": "return_fire_only" },
        ]),
        11,
    );
    run(&mut b, deploy_ticks());
    // The tank answers and may kill some, but none is replaced. Soldiers may
    // shift into the craters its shells dig: moving, still unserved.
    let mut standing = own(&b, Side::Blue, 1).unwrap().members.len();
    for _ in 0..30 * 15 {
        b.step();
        let Some(squad) = own(&b, Side::Blue, 1) else {
            break;
        };
        assert!(squad.members.len() <= standing, "replaced while fighting");
        standing = squad.members.len();
        assert!(
            matches!(squad.service, ServiceStatus::Firing | ServiceStatus::Moving),
            "{:?}",
            squad.service
        );
    }
}

#[test]
fn an_empty_truck_never_blocks_a_stocked_one() {
    let mut b = battle(
        json!([
            truck(Some(0)),
            { "side": "blue", "kind": "supply", "position": [110, 230] },
            holding("tank", 130.0, json!({ "hp": 90 })),
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
            { "side": "blue", "kind": "supply", "position": [110, 230], "condition": { "hp": 20 } },
        ]),
        13,
    );
    run(&mut b, deploy_ticks() + 30 * 10);
    assert_eq!(own(&b, Side::Blue, 1).unwrap().hp, 20.0);
}

#[test]
fn ammunition_comes_before_soldiers_and_an_empty_launcher_reloads() {
    // All four missiles spent and one soldier down.
    let mut b = battle(
        json!([
            truck(None),
            holding(
                "at",
                130.0,
                json!({ "spent": { "atgm": 4 }, "casualties": 1 })
            )
        ]),
        14,
    );
    assert_eq!(own(&b, Side::Blue, 1).unwrap().mounts[1].loaded, None);
    run(&mut b, deploy_ticks() + 30 * 3 + 4);
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(at.members.len(), 2, "no soldier before the missiles");
    assert_eq!(at.mounts[1].ammo, vec![Some(3)]);
    run(&mut b, 30 * 2 + 30 * 10);
    let at = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(at.mounts[1].ammo, vec![Some(4)]);
    assert!(
        at.mounts[1].loaded.is_some(),
        "the refilled launcher reloads"
    );
    assert_eq!(at.members.len(), 3, "then the soldier");
}

#[test]
fn replacements_get_fresh_ids_unique_across_the_battle() {
    let mut b = battle(
        json!([
            truck(None),
            holding("rifle", 130.0, json!({ "casualties": 2 })),
            holding("recon", 150.0, json!({}))
        ]),
        15,
    );
    let first_ids = 8 + 4;
    run(&mut b, deploy_ticks() + 30 * 11);
    assert_eq!(own(&b, Side::Blue, 1).unwrap().members.len(), 8);
    let newest = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .map(|s| s.id)
        .max();
    assert_eq!(
        newest,
        Some(first_ids as u32 + 2),
        "two new ids after every authored one"
    );
}

#[test]
fn a_truck_on_the_move_serves_nobody() {
    let mut b = battle(
        json!([truck(None), holding("tank", 130.0, json!({ "hp": 50 }))]),
        16,
    );
    run(&mut b, deploy_ticks() + 30);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
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
