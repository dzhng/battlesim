//! A transport helicopter's carriage: boarding and unloading from its low
//! hover, its riders through a crash, and a retirement that takes them home.
use super::*;

struct HeliCrash {
    battle: Battle,
    hit: Option<sim::math::V3>,
    impact: Option<sim::math::V3>,
}

fn transport_heli_crash(seed: u64, leaves_map: bool) -> HeliCrash {
    let mut rules = test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_transport_heli",
        json!({ "body": { "hull": { "hp": 1.0 } } }),
    );
    if leaves_map {
        common::json_ground_reach(&mut rules["weapons"]["autocannon"], 50.0);
        rules["weapons"]["autocannon"]["targets"]["low_air"]["range_m"] = json!(50.0);
        for fire in rules["weapons"]["autocannon"]["targets"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            fire["scatter_mrad"] = json!(100.0);
        }
    }
    let start = [100.0, 200.0];
    let goal = if leaves_map {
        [599.0, 200.0]
    } else {
        [500.0, 200.0]
    };
    let map = json!({"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            {"side":"blue","kind":"test_transport_heli","position":start,"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_rifle","position":start,"engagement":"return_fire_only"},
            {"side":"red","kind":"test_gun_jeep","position":if leaves_map {[580.0,200.0]} else {[500.0,80.0]},"engagement":"return_fire_only"}
        ],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, seed);
    assert_eq!(
        battle
            .accept(command(
                1,
                Order::Board {
                    units: vec![UnitId(1)],
                    carrier: UnitId(0),
                    gesture: 1
                },
            ))
            .error,
        None
    );
    for _ in 0..battle.rules().tick_hz * 10 {
        battle.step();
        if battle
            .unit(UnitId(1))
            .and_then(|unit| unit.hold.as_ref())
            .is_some_and(|hold| hold.phase == sim::hold::Phase::Inside)
        {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    if leaves_map {
        assert_eq!(battle.accept(heli_move(2, goal)).error, None);
        for _ in 0..battle.rules().tick_hz * 30 {
            battle.step();
            if battle.unit(UnitId(0)).unwrap().position.x > 530.0 {
                break;
            }
        }
    }
    assert_eq!(
        battle
            .accept(side_command(
                Side::Red,
                1,
                Order::SetEngagement {
                    units: vec![UnitId(2)],
                    policy: Engagement::FireAtWill
                },
            ))
            .error,
        None
    );
    if !leaves_map {
        assert_eq!(battle.accept(heli_move(2, goal)).error, None);
    }
    let mut hit = None;
    let mut impact = None;
    for _ in 0..battle.rules().tick_hz * 30 {
        battle.step();
        if let Some(crash) = battle.crashes().first() {
            hit.get_or_insert(battle.unit(UnitId(0)).unwrap().position);
            impact = Some(crash.position);
            let passenger = battle.unit(UnitId(1)).unwrap().position;
            assert!(
                (passenger - crash.position).length() < 1e-9,
                "passenger follows the falling crash"
            );
            assert!(
                battle
                    .unit(UnitId(1))
                    .unwrap()
                    .member_positions()
                    .all(|position| (position - crash.position).length() < 1e-9),
                "passenger members follow the falling crash"
            );
        }
        if !battle.unit(UnitId(0)).unwrap().alive() && battle.crashes().is_empty() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(0)).unwrap().alive());
    HeliCrash {
        battle,
        hit,
        impact,
    }
}

#[test]
fn soldiers_boarding_a_hovering_helicopter_stay_on_the_ground_until_inside() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_transport_heli","position":[300,300]},
            {"side":"blue","kind":"test_rifle","position":[300,300]},
        ])),
        17,
    );
    assert_eq!(
        battle
            .accept(command(
                1,
                Order::Board {
                    units: vec![UnitId(1)],
                    carrier: UnitId(0),
                    gesture: 1
                },
            ))
            .error,
        None
    );
    let mut entering_ticks = 0;
    let boarded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        let squad = battle.unit(UnitId(1)).expect("boarding squad");
        match squad.hold.as_ref().map(|hold| hold.phase) {
            Some(sim::hold::Phase::Inside) => return true,
            Some(sim::hold::Phase::Entering(_)) => entering_ticks += 1,
            _ => {}
        }
        for position in squad.member_positions() {
            let ground = battle
                .world()
                .surface_at(position.x, position.y)
                .expect("flat map surface")
                .z;
            assert!(
                (position.z - ground).abs() < 1e-6,
                "a soldier not yet inside floats at {} above ground {ground}",
                position.z
            );
        }
        false
    });
    assert!(boarded, "the squad never boarded the hovering helicopter");
    assert!(
        entering_ticks > 0,
        "the squad never stood beside the helicopter to board"
    );
    let hover = battle
        .unit(UnitId(0))
        .expect("transport helicopter")
        .position
        .z;
    let ground = battle
        .world()
        .surface_at(300.0, 300.0)
        .expect("flat map surface")
        .z;
    assert!(
        hover > ground + 1.0,
        "the helicopter boarded on the ground, not in a hover: {hover}"
    );
}

#[test]
fn a_helicopter_lifted_over_a_prop_still_boards_from_its_hover() {
    // A tall lamp beside the pickup lifts the low hover over its top.
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[{"kind":"lamp","center":[303,300],"yaw":0,"half_extents":[0.15,0.15,6]}],"buildings":[]});
    let mut battle = Battle::new(
        &setup_with_map(
            map,
            json!([
                {"side":"blue","kind":"test_transport_heli","position":[300,300]},
                {"side":"blue","kind":"test_rifle","position":[297,300]},
            ]),
            json!([]),
        ),
        17,
    );
    assert_eq!(
        battle
            .accept(command(
                1,
                Order::Board {
                    units: vec![UnitId(1)],
                    carrier: UnitId(0),
                    gesture: 1
                },
            ))
            .error,
        None
    );
    let boarded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|unit| unit.hold.as_ref())
            .is_some_and(|hold| hold.phase == sim::hold::Phase::Inside)
    });
    let hover = battle.unit(UnitId(0)).unwrap().position.z;
    assert!(
        boarded,
        "the squad never boarded under the lifted hover at {hover} m"
    );
}

#[test]
fn a_helicopter_boards_and_unloads_from_its_low_hover() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_transport_heli","position":[300,300]},
            {"side":"blue","kind":"test_rifle","position":[300,300]},
        ])),
        17,
    );
    assert_eq!(
        battle
            .accept(command(
                1,
                Order::Board {
                    units: vec![UnitId(1)],
                    carrier: UnitId(0),
                    gesture: 1
                },
            ))
            .error,
        None
    );
    let boarded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|unit| unit.hold.as_ref())
            .is_some_and(|hold| hold.phase == sim::hold::Phase::Inside)
    });
    assert!(boarded, "the squad never boarded from the low hover");
    let hover = battle
        .unit(UnitId(0))
        .expect("transport helicopter")
        .position
        .z;
    assert!(
        hover < battle.rules().air.low_air.cruise_agl_m,
        "boarding started before the helicopter sank: {hover}"
    );

    assert_eq!(
        battle
            .accept(command(
                2,
                Order::Unload {
                    into: None,
                    units: vec![UnitId(0)],
                    goal: None,
                    gesture: 2,
                    route: RoutePolicy::Shortest,
                },
            ))
            .error,
        None
    );
    let unloaded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .is_some_and(|unit| unit.hold.is_none())
    });
    assert!(unloaded, "the squad never unloaded from the hover");
    let surface = battle
        .world()
        .surface_at(300.0, 300.0)
        .expect("flat map surface");
    assert!(
        battle
            .unit(UnitId(1))
            .unwrap()
            .member_positions()
            .all(|position| (position.z - surface.z).abs() < 1e-6),
        "unloaded soldiers must stand on the ground beneath the hover"
    );
}

#[test]
fn a_helicopter_shot_down_keeps_its_passengers_until_it_hits_the_ground() {
    let mut saw_squad_loss = false;
    let mut saw_survivors = false;
    for seed in 1..=64 {
        let HeliCrash {
            battle,
            hit,
            impact,
        } = transport_heli_crash(seed, false);
        let (hit, impact) = (hit.expect("seen falling"), impact.expect("seen falling"));
        let passenger = battle.unit(UnitId(1)).expect("passenger squad");
        if passenger.alive() {
            saw_survivors = true;
            assert!(passenger.on_field() && passenger.hold.is_none());
            // They come out around where it came down, not where it was hit.
            for at in passenger.member_positions() {
                assert!(
                    (at.xy() - impact.xy()).length() < (at.xy() - hit.xy()).length(),
                    "seed {seed}: a survivor at {at:?} stands nearer the hit {hit:?} than the impact {impact:?}"
                );
                let search = battle.rules().bail_out[&BailOutHost::Air].search_m;
                assert!((at.xy() - impact.xy()).length() <= search + 1e-9);
            }
        } else {
            saw_squad_loss = true;
            assert!(passenger.members.iter().all(|member| !member.alive()));
        }
        if saw_squad_loss && saw_survivors {
            break;
        }
    }
    assert!(saw_squad_loss, "no seed lost the whole squad in the crash");
    assert!(saw_survivors, "no seed let the squad survive the crash");

    let off_map = transport_heli_crash(1, true).battle;
    let passenger = off_map.unit(UnitId(1)).unwrap();
    assert!(
        !passenger.alive() && passenger.members.iter().all(|member| !member.alive()),
        "a crash that leaves the map takes every passenger with it"
    );
}

#[test]
fn a_retiring_helicopter_takes_its_passengers_with_it() {
    let mut battle = Battle::new(&transport_skirmish_setup(), 41);
    for (seq, variant) in [(1, "test_transport_heli"), (2, "test_recon")] {
        let bought = battle.accept(side_command(
            Side::Blue,
            seq,
            Order::purchase(variant, [300.0, 100.0]),
        ));
        assert_eq!(bought.error, None);
    }
    battle.accept(side_command(Side::Blue, 3, Order::Ready));
    battle.accept(side_command(Side::Red, 1, Order::Ready));
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).is_some() {
            break;
        }
    }
    let board = battle.accept(side_command(
        Side::Blue,
        4,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    assert_eq!(board.error, None);
    let boarded = (0..battle.rules().tick_hz * 60).any(|_| {
        battle.step();
        !battle.unit(UnitId(1)).unwrap().on_field()
    });
    assert!(boarded, "the squad never boarded the helicopter");
    let refund = battle.accept(side_command(
        Side::Blue,
        5,
        Order::Refund {
            units: vec![UnitId(0)],
        },
    ));
    assert_eq!(refund.error, None);
    let retired = (0..battle.rules().tick_hz * 60).any(|_| {
        battle.step();
        battle.unit(UnitId(0)).unwrap().retired
    });
    assert!(retired, "the helicopter never retired");
    let passenger = battle.unit(UnitId(1)).unwrap();
    assert!(passenger.retired);
    assert!(
        passenger.hold.is_none(),
        "a retired passenger still rides the helicopter"
    );
}
