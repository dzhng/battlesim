//! Retirement values follow original finite rows, not round counts or replenishment history.
use crate::common::command;
use crate::common::Seat;
use crate::skirmish::setup;
use contract::command::Order;
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures::{self, test_rules};

#[test]
fn finite_resources_are_an_unweighted_mean_and_unlimited_rows_do_not_dilute_it() {
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({"roster":{"factions":["us"],"category":"veh","family_name":"Test","variant":"Tank"}}),
    );
    let setup = serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":rules,"units":[{"side":"blue","kind":"test_tank","position":[100,100],
            "condition":{"spent":{"tank_ap":20,"tank_he":7}}},
            {"side":"blue","kind":"test_rifle","position":[200,100]}]
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

#[test]
fn a_new_order_cancels_withdrawal_without_a_refund() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::purchase("test_tank", [400.0, 150.0]),
    ));
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    battle.accept(command(Side::Blue, 3, Order::Refund { units: vec![id] }));
    battle.step();
    assert!(battle.observe(Side::Blue).own[0].withdrawing);
    let credits = battle.observe(Side::Blue).seat().credits;
    battle.accept(command(Side::Blue, 4, Order::Stop { units: vec![id] }));
    for _ in 0..450 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(!status.own[0].withdrawing);
    assert_eq!(status.seat().unit_count, 1);
    assert!((status.seat().credits - credits - 50.0).abs() < 0.00001);
}

#[test]
fn changing_the_return_route_cancels_withdrawal_without_a_refund() {
    let setup = setup();
    let mut battle = Battle::new(&setup, 1);
    battle.accept(command(
        Side::Blue,
        1,
        Order::purchase("test_tank", [400.0, 150.0]),
    ));
    for (side, seq) in [(Side::Blue, 2), (Side::Red, 1)] {
        battle.accept(command(side, seq, Order::Ready));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    battle.accept(command(Side::Blue, 3, Order::Refund { units: vec![id] }));
    battle.step();
    assert!(battle.observe(Side::Blue).own[0].withdrawing);
    let credits = battle.observe(Side::Blue).seat().credits;
    battle.accept(command(
        Side::Blue,
        4,
        Order::UpgradeMove {
            gesture: 0,
            route: contract::command::RoutePolicy::Shortest,
        },
    ));
    for _ in 0..450 {
        battle.step();
    }
    let status = battle.observe(Side::Blue);
    assert!(!status.own[0].withdrawing);
    assert_eq!(status.seat().unit_count, 1);
    assert!((status.seat().credits - credits - 50.0).abs() < 0.00001);
}

#[test]
fn a_destroyed_withdrawing_unit_never_refunds_or_retires() {
    let mut setup = setup();
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({
            "cost":200,"roster":{"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"},
            "body":{"hull":{"hp":0.01}},"sensors":{"ground_m":500},"mobility":{"tracked":{"offroad_kmh":5,"road_kmh":5}}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup.skirmish.as_mut().unwrap().sites.entries[1].center = [400.0, 110.0];
    let mut battle = Battle::new(&setup, 7);
    for (side, destination) in [(Side::Blue, [400.0, 150.0]), (Side::Red, [400.0, 110.0])] {
        assert!(battle
            .accept(command(side, 1, Order::purchase("test_tank", destination)))
            .error
            .is_none());
        battle.accept(command(side, 2, Order::Ready));
    }
    battle.step();
    for side in Side::COMBATANTS {
        let id = battle.observe(side).own[0].id;
        battle.accept(command(
            side,
            3,
            Order::SetEngagement {
                units: vec![id],
                policy: contract::command::Engagement::ReturnFireOnly,
            },
        ));
    }
    for _ in 0..450 {
        battle.step();
    }
    let id = battle.observe(Side::Blue).own[0].id;
    let enemy = battle.observe(Side::Red).own[0].id;
    let before = battle.observe(Side::Blue).seat().credits;
    let start = battle.tick();
    battle.accept(command(Side::Blue, 4, Order::Refund { units: vec![id] }));
    battle.accept(command(
        Side::Red,
        4,
        Order::Attack {
            units: vec![enemy],
            target: contract::command::TargetRef::Identified {
                id: battle.observe(Side::Red).identified[0].id,
            },
        },
    ));
    for _ in 0..1000 {
        battle.step();
    }
    assert!(!battle.unit(id).unwrap().alive());
    assert!(
        !battle.unit(id).unwrap().retired,
        "combat destruction must not become a retirement"
    );
    let after = battle.observe(Side::Blue).seat().credits;
    let passive = (battle.tick() - start) as f64 / f64::from(battle.rules().tick_hz) * 200.0 / 60.0;
    assert!(
        (after - before - passive).abs() < 0.00001,
        "no refund for a destroyed unit"
    );
}

#[test]
fn refund_condition_counts_casualties_lost_ammunition_carriers_and_empty_trucks() {
    let mut setup = crate::common::scenario_with(
        &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([{"side":"blue","kind":"test_rifle","position":[100,200]}, {"side":"blue","kind":"test_supply","position":[200,200]}]),
        json!([]),
        json!([]),
    );
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_grenadier",
        json!({"mounts":[{"id":"grenade launcher","weapons":["grenade"],"special":false}]}),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let battle = Battle::new(&setup, 1);
    let mut squad = battle.unit(contract::ids::UnitId(0)).unwrap().clone();
    let mut truck = battle.unit(contract::ids::UnitId(1)).unwrap().clone();
    let condition = |unit: &sim::units::Unit| {
        sim::withdrawal::condition(unit, battle.rules(), battle.arsenal())
    };
    assert_eq!(condition(&squad), (1.0, 1.0));
    squad.members[0].hp = 0.0;
    assert_eq!(condition(&squad),(7.0/8.0,0.0),"dead grenadier retains neither original health nor usable finite grenades; unlimited rifles do not dilute ammo");
    truck.hp = truck.max_hp(battle.rules()) / 2.0;
    truck.stock = Some(0);
    assert_eq!(condition(&truck), (0.5, 0.0));
}
