//! Passenger carriage behavior through the public battle authority.
use contract::command::{
    CommandEnvelope, Engagement, MoveDirection, Order, OrderError, RoutePolicy,
};
use contract::ids::{Side, UnitId};
use contract::observation::HoldPhase;
use serde_json::json;
use sim::battle::Battle;

fn setup(units: serde_json::Value) -> contract::scenario::ScenarioDefinition {
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    crate::common::scenario_with(&map.to_string(), units, json!([]), json!([]))
}

fn setup_with_map(
    map: serde_json::Value,
    units: serde_json::Value,
    events: serde_json::Value,
) -> contract::scenario::ScenarioDefinition {
    crate::common::scenario_with(&map.to_string(), units, events, json!([]))
}

fn command(seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    }
}

/// A command from the side's one player in the transport skirmish.
fn side_command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
    CommandEnvelope {
        side,
        player: Some(
            if side == Side::Blue {
                "blue-1"
            } else {
                "red-1"
            }
            .into(),
        ),
        seq,
        order,
        queued: false,
    }
}

/// Where a shot-down transport helicopter was hit, and where it came down.
struct HeliCrash {
    battle: Battle,
    hit: Option<sim::math::V3>,
    impact: Option<sim::math::V3>,
}

fn transport_heli_crash(seed: u64, leaves_map: bool) -> HeliCrash {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_transport_heli",
        json!({ "body": { "hull": { "hp": 1.0 } } }),
    );
    if leaves_map {
        rules["weapons"]["autocannon"]["range_m"] = json!(50.0);
        rules["weapons"]["autocannon"]["targets"]["low_air"]["range_m"] = json!(50.0);
        rules["weapons"]["autocannon"]["scatter_mrad"] = json!(100.0);
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

fn heli_move(seq: u64, goal: [f64; 2]) -> CommandEnvelope {
    command(
        seq,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 3,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    )
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
                assert!((at.xy() - impact.xy()).length() < 15.0);
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
            Order::ConfirmPurchase {
                variant: variant.into(),
                destination: [300.0, 100.0],
            },
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
    assert!(battle.unit(UnitId(1)).unwrap().retired);
}

fn transport_skirmish_setup() -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::test_game();
    for (kind, faction, category) in [
        ("test_apc", "us", "veh"),
        ("test_recon", "us", "inf"),
        ("test_tank", "eastern", "veh"),
        ("test_transport_heli", "us", "hel"),
    ] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"roster":{"factions":[faction],"category":category,"family_name":"Test Transport","variant":"Test"}}),
        );
    }
    sim::fixtures::patch_catalog(&mut rules, "units", "test_apc", json!({"mounts":[]}));
    serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]},
        "rules":rules,"units":[],"events":[],"scripts":[],
        "players":[{"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},{"id":"red-1","side":"red","faction":"eastern","controller":"passive_ai"}],
        "skirmish":{"rules":{"credits_per_minute":200,"starting_minutes":5,"preparation_s":0,"max_units":30,"max_players_per_side":10,"dispatch_interval_s":1,"capture_s":20,"majority_s":1800,"victory_score":1000},"sites":{"entries":[{"side":"blue","center":[400,10],"yaw":0.0},{"side":"red","center":[400,590],"yaw":0.0}],"objectives":[{"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null},{"id":"south","center":[200,200],"radius_m":50,"kind":"field","counterpart":"north"},{"id":"north","center":[600,400],"radius_m":50,"kind":"field","counterpart":"south"}]}}
    })).unwrap()
}

#[test]
fn a_squad_boards_a_stopped_carrier_and_leaves_the_field() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]},
            {"side":"red","kind":"test_rifle","position":[600,200]}
        ])),
        7,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    let passenger = battle.unit(UnitId(1)).unwrap();
    assert_eq!(battle.observe(Side::Red).identified.len(), 1);
    assert!(!passenger.on_field());
    assert_eq!(
        passenger.hold.as_ref().map(|h| h.phase),
        Some(sim::hold::Phase::Inside)
    );
    assert_eq!(
        battle
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(1))
            .and_then(|u| u.hold.map(|h| h.phase)),
        Some(HoldPhase::Inside)
    );
    assert!(battle
        .observe(Side::Red)
        .audible
        .iter()
        .all(|cue| cue.listener != UnitId(1)));
    assert_eq!(
        passenger.service,
        contract::observation::ServiceStatus::OutOfRange
    );
}

#[test]
fn an_attacker_whose_infantry_all_ride_is_not_defeated() {
    // An unarmed carrier, so the riding squad is the attacker's only weapon.
    let mut game = crate::common::game();
    game["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({"units": {
            "test_unarmed_carrier": {
                "extends": "test_supply", "name": "Unarmed Carrier",
                "capabilities": {"carriage": {"seats": 8}}
            }
        }}));
    let mut def = setup(json!([
        {"side":"blue","kind":"test_unarmed_carrier","position":[200,200]},
        {"side":"blue","kind":"test_rifle","position":[195,200]},
        {"side":"red","kind":"test_rifle","position":[600,200]}
    ]));
    def.rules = serde_json::from_value(game).unwrap();
    def.encounter = Some(contract::scenario::EncounterRules {
        attacker: Side::Blue,
        success_zone_center: [600.0, 200.0],
        success_zone_radius_m: 20.0,
        hold_s: 30.0,
        max_assessment_s: 600.0,
    });
    let mut battle = Battle::new(&def, 7);
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    let boarded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        !battle.unit(UnitId(1)).unwrap().on_field()
    });
    assert!(boarded, "the squad never boarded the unarmed carrier");
    battle.step();
    assert_eq!(
        battle.encounter().map(|status| status.result),
        Some(contract::observation::EncounterResult::Running),
        "a riding squad is still the attacker's fighting strength"
    );
}

#[test]
fn a_squad_walks_to_a_distant_carrier_and_boards_it() {
    // The squad can never stand on the carrier's centre (the hull is in the
    // way), so its walk has to end where the hold's reach admits it.
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[220,240]},
            {"side":"blue","kind":"test_rifle","position":[200,240]},
            {"side":"red","kind":"test_rifle","position":[600,200]}
        ])),
        7,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    let aboard = (0..battle.rules().tick_hz * 40).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|u| u.hold.as_ref())
            .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
    });
    assert!(aboard, "the squad never boarded the carrier 20 m away");
}

#[test]
fn orders_aimed_at_a_passenger_are_refused_as_aboard() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        8,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 4) {
        battle.step();
    }
    let ack = battle.accept(command(
        2,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 2,
            goal: [300.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    ));
    assert_eq!(ack.error, Some(OrderError::Aboard { unit: UnitId(1) }));
}

#[test]
fn squads_beyond_the_seats_are_refused_closest_first() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[214,200]},
            {"side":"blue","kind":"test_recon","position":[202,200]}
        ])),
        9,
    );
    let ack = battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1), UnitId(2)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    assert_eq!(
        ack.refused.iter().map(|r| r.unit).collect::<Vec<_>>(),
        vec![UnitId(1)]
    );
    assert_eq!(ack.refused[0].reason, OrderError::CapacityFull);
}

#[test]
fn a_carrier_drives_to_a_squad_and_takes_it_aboard() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[250,200]},
            {"side":"blue","kind":"test_recon","position":[220,200]}
        ])),
        10,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Collect {
                units: vec![UnitId(0)],
                squads: vec![UnitId(1)],
                gesture: 1
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 12) {
        battle.step();
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    assert_eq!(
        battle
            .unit(UnitId(1))
            .unwrap()
            .hold
            .as_ref()
            .map(|h| h.phase),
        Some(sim::hold::Phase::Inside)
    );
}

#[test]
fn a_collected_squad_out_of_reach_walks_over_before_it_boards() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[250,200]},
            {"side":"blue","kind":"test_recon","position":[220,200]},
            {"side":"blue","kind":"test_recon","position":[220,240]}
        ])),
        10,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Collect {
                units: vec![UnitId(0)],
                squads: vec![UnitId(1), UnitId(2)],
                gesture: 1
            }
        ))
        .error
        .is_none());
    // A soldier on foot covers well under a metre a tick; the far squad is
    // 40 m from the first one the carrier reaches.
    let mut last = battle.unit(UnitId(2)).unwrap().position.xy();
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        let squad = battle.unit(UnitId(2)).unwrap();
        if !squad.on_field() {
            break;
        }
        let moved = (squad.position.xy() - last).length();
        assert!(moved < 1.0, "the far squad jumped {moved} m in one tick");
        last = squad.position.xy();
    }
    for _ in 0..(battle.rules().tick_hz * 10) {
        battle.step();
    }
    for id in [UnitId(1), UnitId(2)] {
        assert_eq!(
            battle.unit(id).unwrap().hold.as_ref().map(|h| h.phase),
            Some(sim::hold::Phase::Inside),
            "every collected squad ends up aboard"
        );
    }
}

/// Ticks unit 1 spends getting into carrier 0 after `order`.
fn ticks_getting_in(order: Order) -> usize {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]}
        ])),
        7,
    );
    assert!(battle.accept(command(1, order)).error.is_none());
    let mut entering = 0;
    for _ in 0..(battle.rules().tick_hz * 10) {
        battle.step();
        match battle
            .unit(UnitId(1))
            .unwrap()
            .hold
            .as_ref()
            .map(|h| h.phase)
        {
            Some(sim::hold::Phase::Entering(_)) => entering += 1,
            Some(sim::hold::Phase::Inside) => return entering,
            _ => {}
        }
    }
    panic!("the squad never got in");
}

#[test]
fn boarding_takes_as_long_whoever_gives_the_order() {
    let boarded = ticks_getting_in(Order::Board {
        units: vec![UnitId(1)],
        carrier: UnitId(0),
        gesture: 1,
    });
    let collected = ticks_getting_in(Order::Collect {
        units: vec![UnitId(0)],
        squads: vec![UnitId(1)],
        gesture: 1,
    });
    assert!(boarded > 0);
    assert_eq!(boarded, collected);
}

#[test]
fn transport_orders_replay_every_tick_exactly() {
    let setup = setup(json!([
        {"side":"blue","kind":"test_apc","position":[200,200]},
        {"side":"blue","kind":"test_recon","position":[195,200]}
    ]));
    let mut live = Battle::new(&setup, 11);
    live.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    let mut digests = vec![live.digest()];
    for _ in 0..180 {
        live.step();
        digests.push(live.digest());
    }
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    while replay.tick() < live.tick() {
        replay.step();
        assert_eq!(replay.digest(), digests[replay.tick() as usize]);
    }
}

#[test]
fn catalog_rejects_invalid_carriage_seats_and_squads() {
    let mut game = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut game,
        "units",
        "test_apc",
        json!({"capabilities":{"carriage":{"seats":7}}}),
    );
    assert!(serde_json::from_value::<contract::scenario::Rules>(game).is_err());
}

#[test]
fn capacity_lost_during_boarding_is_published_as_refusal() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]},
            {"side":"blue","kind":"test_recon","position":[196,200]}
        ])),
        12,
    );
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        ))
        .error
        .is_none());
    assert!(battle
        .accept(command(
            2,
            Order::Board {
                units: vec![UnitId(2)],
                carrier: UnitId(0),
                gesture: 2,
            },
        ))
        .error
        .is_none());
    let mut seen = false;
    for _ in 0..(battle.rules().tick_hz * 2) {
        battle.step();
        seen |= battle.observe(Side::Blue).refusals.iter().any(|refusal| {
            matches!(refusal.unit, UnitId(1) | UnitId(2))
                && refusal.reason == OrderError::CapacityFull
        });
    }
    assert!(seen);
}

#[test]
fn a_carrier_unloads_where_sent_and_queued_orders_follow() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        13,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());

    let unload = battle.accept(command(
        2,
        Order::Unload {
            units: vec![UnitId(0)],
            goal: Some([205.0, 200.0]),
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert!(unload.error.is_none());
    let mut follow_command = command(
        3,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 3,
            goal: [220.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    );
    follow_command.queued = true;
    let follow = battle.accept(follow_command);
    assert!(follow.error.is_none());
    for _ in 0..(battle.rules().tick_hz * 8) {
        battle.step();
    }
    assert!(battle.unit(UnitId(1)).unwrap().on_field());
    assert!(battle.unit(UnitId(0)).unwrap().position.x > 205.0);
}

#[test]
fn an_unload_with_no_room_is_refused_and_everyone_stays_aboard() {
    let blocked_map = json!({
        "size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[{"kind":"wall","center":[200,200],"yaw":0,"half_extents":[10,10,4]}],
        "buildings":[]
    });
    let mut in_place = Battle::new(
        &setup_with_map(
            blocked_map,
            json!([
                {"side":"blue","kind":"test_apc","position":[200,200]},
                {"side":"blue","kind":"test_rifle","position":[200,200]}
            ]),
            json!([]),
        ),
        14,
    );
    in_place.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(in_place.rules().tick_hz * 3) {
        in_place.step();
    }
    let refused = in_place.accept(command(
        2,
        Order::Unload {
            units: vec![UnitId(0)],
            goal: None,
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert_eq!(
        refused.refused,
        vec![contract::command::Refusal {
            unit: UnitId(0),
            reason: OrderError::NoRoomToUnload,
        }]
    );
    assert!(!in_place.unit(UnitId(1)).unwrap().on_field());
    assert!(in_place.unit(UnitId(1)).unwrap().alive());

    let arrival_map = json!({
        "size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "props":[],"buildings":[]
    });
    let mut arrival = Battle::new(
        &setup_with_map(
            arrival_map,
            json!([
                {"side":"blue","kind":"test_apc","position":[200,200]},
                {"side":"blue","kind":"test_rifle","position":[195,200]}
            ]),
            json!([{"tick":100,"add_prop":{"kind":"wall","center":[201,200],"yaw":0,"half_extents":[10,10,4]}}]),
        ),
        15,
    );
    arrival.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(arrival.rules().tick_hz * 2) {
        arrival.step();
    }
    let unload = arrival.accept(command(
        2,
        Order::Unload {
            units: vec![UnitId(0)],
            goal: Some([201.0, 200.0]),
            gesture: 2,
            route: Default::default(),
        },
    ));
    assert!(unload.error.is_none());
    let mut follow_command = command(
        3,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 3,
            goal: [220.0, 200.0],
            route: Default::default(),
            direction: Default::default(),
            facing: None,
        },
    );
    follow_command.queued = true;
    assert!(arrival.accept(follow_command).error.is_none());
    arrival.step();
    assert_eq!(published_unloads(&arrival, UnitId(0)), (true, vec![false]));
    let mut seen = false;
    let mut saw_exiting = false;
    for _ in 0..(arrival.rules().tick_hz * 5) {
        arrival.step();
        saw_exiting |= matches!(
            arrival
                .unit(UnitId(1))
                .unwrap()
                .hold
                .as_ref()
                .map(|hold| hold.phase),
            Some(sim::hold::Phase::Exiting(_))
        );
        seen |= arrival.observe(Side::Blue).refusals.iter().any(|refusal| {
            refusal.unit == UnitId(0) && refusal.reason == OrderError::NoRoomToUnload
        });
    }
    assert!(saw_exiting);
    assert!(seen);
    // Refused: the step that would have unloaded is gone, and nothing
    // ahead unloads.
    assert_eq!(published_unloads(&arrival, UnitId(0)), (false, vec![]));
    assert!(!arrival.unit(UnitId(1)).unwrap().on_field());
    assert!(arrival.unit(UnitId(1)).unwrap().alive());
    assert!(matches!(
        arrival.unit(UnitId(0)).unwrap().orders.front(),
        Some(sim::units::UnitOrder::Move(order)) if order.destination.x == 220.0
    ));
}

#[test]
fn refunding_a_loaded_carrier_refunds_its_passengers() {
    let mut battle = Battle::new(&transport_skirmish_setup(), 41);
    assert!(battle
        .accept(side_command(
            Side::Blue,
            1,
            Order::ConfirmPurchase {
                variant: "test_apc".into(),
                destination: [300.0, 100.0],
            },
        ))
        .error
        .is_none());
    assert!(battle
        .accept(side_command(
            Side::Blue,
            2,
            Order::ConfirmPurchase {
                variant: "test_recon".into(),
                destination: [300.0, 100.0],
            },
        ))
        .error
        .is_none());
    battle.accept(side_command(Side::Blue, 3, Order::Ready));
    battle.accept(side_command(Side::Red, 1, Order::Ready));
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).is_some() {
            break;
        }
    }
    assert!(battle.unit(UnitId(0)).is_some() && battle.unit(UnitId(1)).is_some());
    assert!(battle
        .accept(side_command(
            Side::Blue,
            4,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    let refused = battle.accept(side_command(
        Side::Blue,
        5,
        Order::Refund {
            units: vec![UnitId(1)],
        },
    ));
    assert_eq!(refused.error, Some(OrderError::Aboard { unit: UnitId(1) }));
    let before = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    assert!(battle
        .accept(side_command(
            Side::Blue,
            6,
            Order::Refund {
                units: vec![UnitId(0)]
            }
        ))
        .error
        .is_none());
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(0)).unwrap().retired {
            break;
        }
    }
    let after = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .unwrap()
        .credits;
    assert!(after > before);
    assert!(battle.unit(UnitId(0)).unwrap().retired);
    assert!(battle.unit(UnitId(1)).unwrap().retired);
}

/// Blue's 1 hp APC with a recon squad boarding it (for `board_s` when given),
/// destroyed by a red gun jeep once the squad's hold reaches a phase `ready`
/// accepts, and still in such a phase when the carrier dies. Returns the
/// battle on the tick the carrier died and each soldier's health before.
fn carrier_destroyed_once(
    board_s: Option<f64>,
    ready: fn(sim::hold::Phase) -> bool,
) -> (Battle, Vec<f64>) {
    let mut rules = sim::fixtures::test_game();
    if let Some(board_s) = board_s {
        rules["carriage"]["board_s"] = json!(board_s);
    }
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_apc",
        json!({ "body": { "hull": { "hp": 1.0 } } }),
    );
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map,
        "rules": rules,
        "units": [
            {"side":"blue","kind":"test_apc","position":[200,200],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_recon","position":[195,200],"engagement":"return_fire_only"},
            {"side":"red","kind":"test_gun_jeep","position":[260,200],"engagement":"return_fire_only"}
        ],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 42);
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    let reached = (0..battle.rules().tick_hz * 10).any(|_| {
        battle.step();
        let squad = battle.unit(UnitId(1)).unwrap();
        squad.hold.as_ref().is_some_and(|hold| ready(hold.phase))
    });
    assert!(reached, "the squad never reached the boarding phase");
    let health = battle
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .map(|s| s.hp)
        .collect();
    let carrier = battle
        .observe(Side::Red)
        .identified
        .iter()
        .find(|u| u.kind == battle.unit(UnitId(0)).unwrap().kind)
        .map(|u| u.id)
        .expect("red sees the carrier");
    let attack = CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(2)],
            target: contract::command::TargetRef::Identified { id: carrier },
        },
        queued: false,
    };
    assert_eq!(battle.accept(attack).error, None);
    let destroyed = (0..battle.rules().tick_hz * 10).any(|_| {
        let phase = battle
            .unit(UnitId(1))
            .unwrap()
            .hold
            .as_ref()
            .map(|h| h.phase);
        assert!(
            phase.is_some_and(ready),
            "the squad left the boarding phase before its carrier died: {phase:?}"
        );
        battle.step();
        !battle.unit(UnitId(0)).unwrap().alive()
    });
    assert!(destroyed, "the gun jeep never destroyed the carrier");
    (battle, health)
}

#[test]
fn a_destroyed_carrier_bails_out_its_passengers_hurt_and_pinned() {
    let (battle, health) = carrier_destroyed_once(None, |phase| phase == sim::hold::Phase::Inside);
    let rule = &battle.rules().bail_out[&contract::scenario::BailOutHost::Ground];
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(
        squad.hold.is_none(),
        "no rider stays in a destroyed carrier"
    );
    for (soldier, before) in squad.members.iter().zip(health) {
        if !soldier.alive() {
            continue;
        }
        let lost = before - soldier.hp;
        assert!(
            lost >= rule.hp_lost[0] - 1e-9 && lost <= rule.hp_lost[1] + 1e-9,
            "a soldier who got out lost {lost} hp"
        );
    }
    if squad.alive() {
        assert!(squad.on_field(), "the survivors stand on the field");
        assert!(
            squad.suppression >= battle.rules().suppression.pinned.level,
            "the survivors are pinned"
        );
    }
}

#[test]
fn a_squad_still_getting_in_is_spared_when_its_carrier_is_destroyed() {
    // Boarding outlasts the gun jeep's kill, so the carrier dies mid-boarding.
    let (battle, health) = carrier_destroyed_once(Some(30.0), |phase| {
        matches!(phase, sim::hold::Phase::Entering(_))
    });
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(squad.on_field());
    assert_eq!(
        squad.members.iter().map(|s| s.hp).collect::<Vec<_>>(),
        health,
        "a squad standing beside the carrier is not hurt as if it rode inside"
    );
    assert!(
        squad.suppression < battle.rules().suppression.pinned.level,
        "a squad standing beside the carrier is not pinned by a bail-out"
    );
}

/// Blue's published unload flags for unit `id`: its goal's, then each
/// queued step's.
fn published_unloads(battle: &Battle, id: UnitId) -> (bool, Vec<bool>) {
    let observation = battle.observe(Side::Blue);
    let own = observation.own.iter().find(|u| u.id == id).unwrap();
    assert_eq!(own.queue.len(), own.queue_unloads.len());
    (own.goal_unloads, own.queue_unloads.clone())
}

#[test]
fn the_unload_step_of_a_queued_drive_unload_drive_is_published() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]}
        ])),
        16,
    );
    battle.accept(command(
        1,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    ));
    for _ in 0..(battle.rules().tick_hz * 60) {
        battle.step();
        if !battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(!battle.unit(UnitId(1)).unwrap().on_field());
    let drive = |seq, goal| Order::Move {
        units: vec![UnitId(0)],
        gesture: seq,
        goal,
        route: Default::default(),
        direction: Default::default(),
        facing: None,
    };
    assert!(battle
        .accept(command(2, drive(2, [240.0, 200.0])))
        .error
        .is_none());
    let mut unload = command(
        3,
        Order::Unload {
            units: vec![UnitId(0)],
            goal: Some([280.0, 200.0]),
            gesture: 3,
            route: Default::default(),
        },
    );
    unload.queued = true;
    assert!(battle.accept(unload).error.is_none());
    let mut onward = command(4, drive(4, [320.0, 200.0]));
    onward.queued = true;
    assert!(battle.accept(onward).error.is_none());
    battle.step();
    assert_eq!(
        published_unloads(&battle, UnitId(0)),
        (false, vec![true, false])
    );

    // The unload step becomes the goal: the flag goes with it.
    let mut heading_to_unload = false;
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.observe(Side::Blue).own[0].queue.len() == 1 {
            heading_to_unload = true;
            break;
        }
    }
    assert!(heading_to_unload);
    assert_eq!(published_unloads(&battle, UnitId(0)), (true, vec![false]));

    // Unloaded and driving on: no step unloads any more.
    for _ in 0..(battle.rules().tick_hz * 30) {
        battle.step();
        if battle.unit(UnitId(1)).unwrap().on_field() {
            break;
        }
    }
    assert!(battle.unit(UnitId(1)).unwrap().on_field());
    battle.step();
    assert_eq!(published_unloads(&battle, UnitId(0)), (false, vec![]));
}
