use crate::common;
use crate::common::Seat;
use contract::command::{
    CommandEnvelope, Engagement, MoveDirection, Order, OrderError, RoutePolicy,
};
use contract::ids::{Side, UnitId};
use contract::observation::HoldPhase;
use contract::scenario::BailOutHost;
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
    let passenger = battle.unit(UnitId(1)).unwrap();
    assert!(passenger.retired);
    assert!(
        passenger.hold.is_none(),
        "a retired passenger still rides the helicopter"
    );
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
        "players": crate::common::human_duel(),
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
fn a_squad_boards_only_its_own_players_carrier() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200],"owner":"blue-2"},
            {"side":"blue","kind":"test_recon","position":[195,200],"owner":"blue-1"},
            {"side":"blue","kind":"test_apc","position":[200,210],"owner":"blue-1"}
        ])),
        7,
    );
    let board = |seq, carrier| CommandEnvelope {
        player: Some("blue-1".into()),
        ..command(
            seq,
            Order::Board {
                units: vec![UnitId(1)],
                carrier,
                gesture: 1,
            },
        )
    };
    assert_eq!(
        battle.accept(board(1, UnitId(0))).error,
        Some(OrderError::WrongPlayer { unit: UnitId(0) }),
        "a squad boarded its ally's carrier"
    );
    assert_eq!(battle.accept(board(2, UnitId(2))).error, None);
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

/// Where the APC's squad (unit 2) stands after it boards the APC (unit 1)
/// and unloads in place, with a transport helicopter (unit 0) hovering over
/// the APC. The helicopter carries the rifle squad (unit 3) when `riding`;
/// otherwise that squad stands far off.
fn unloaded_under_a_helicopter(riding: bool) -> Vec<sim::math::V3> {
    let rifles = if riding { [260, 200] } else { [500, 400] };
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_transport_heli","position":[260,200]},
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]},
            {"side":"blue","kind":"test_rifle","position":rifles}
        ])),
        7,
    );
    let mut seq = 0;
    let mut next = || {
        seq += 1;
        seq
    };
    let board = |seq, squad, carrier| {
        command(
            seq,
            Order::Board {
                units: vec![squad],
                carrier,
                gesture: 1,
            },
        )
    };
    assert_eq!(
        battle.accept(board(next(), UnitId(2), UnitId(1))).error,
        None
    );
    if riding {
        assert_eq!(
            battle.accept(board(next(), UnitId(3), UnitId(0))).error,
            None
        );
    }
    let aboard = |battle: &Battle, id: UnitId| !battle.unit(id).unwrap().on_field();
    let loaded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        aboard(&battle, UnitId(2)) && (!riding || aboard(&battle, UnitId(3)))
    });
    assert!(loaded, "the squads never boarded");
    assert_eq!(battle.accept(heli_move(next(), [200.0, 200.0])).error, None);
    for _ in 0..battle.rules().tick_hz * 20 {
        battle.step();
    }
    let over = battle.unit(UnitId(0)).unwrap().position.xy() - sim::math::v2(200.0, 200.0);
    assert!(over.length() < 1.0, "the helicopter is not over the APC");
    let unload = Order::Unload {
        units: vec![UnitId(1)],
        goal: None,
        gesture: 4,
        route: RoutePolicy::Shortest,
    };
    assert_eq!(battle.accept(command(next(), unload)).error, None);
    let out = (0..battle.rules().tick_hz * 10).any(|_| {
        battle.step();
        battle.unit(UnitId(2)).unwrap().hold.is_none()
    });
    assert!(out, "the squad never unloaded");
    battle.unit(UnitId(2)).unwrap().member_positions().collect()
}

#[test]
fn soldiers_riding_another_carrier_take_up_no_ground() {
    assert_eq!(
        unloaded_under_a_helicopter(true),
        unloaded_under_a_helicopter(false)
    );
}

/// A tank (unit 2), one hit from death, driving east until red's tank
/// (unit 3) kills it, with a loaded transport helicopter (unit 0, carrying
/// the rifle squad, unit 1) hovering at `hover`. Returns where the tank
/// stood and how fast it drove the tick before it died, its braking rate,
/// where its wreck lies, and where the helicopter hovered as it died.
fn battle_soldier_radius() -> f64 {
    sim::fixtures::test_game()["physics"]["soldier_radius_m"]
        .as_f64()
        .expect("a soldier's radius")
}

fn killed_driving_with_riders_overhead(hover: [f64; 2]) -> Rolled {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_transport_heli","position":[300,150],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_rifle","position":[300,150]},
            {"side":"blue","kind":"test_tank","position":[100,300],"condition":{"hp":1},"engagement":"return_fire_only"},
            {"side":"red","kind":"test_tank","position":[300,420],"yaw":-1.2,"engagement":"return_fire_only"}
        ])),
        1,
    );
    let board = Order::Board {
        units: vec![UnitId(1)],
        carrier: UnitId(0),
        gesture: 1,
    };
    assert_eq!(battle.accept(command(1, board)).error, None);
    let loaded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        !battle.unit(UnitId(1)).unwrap().on_field()
    });
    assert!(loaded, "the squad never boarded the helicopter");
    assert_eq!(battle.accept(heli_move(2, hover)).error, None);
    for _ in 0..battle.rules().tick_hz * 20 {
        battle.step();
    }
    let drive = Order::Move {
        units: vec![UnitId(2)],
        gesture: 4,
        goal: [600.0, 300.0],
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    };
    assert_eq!(battle.accept(command(3, drive)).error, None);
    for _ in 0..battle.rules().tick_hz * 4 {
        battle.step();
    }
    let tank_kind = battle.unit(UnitId(2)).unwrap().kind;
    let target = battle
        .observe(Side::Red)
        .identified
        .iter()
        .find(|u| u.kind == tank_kind)
        .map(|u| u.id)
        .expect("red sees blue's tank");
    let attack = CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(3)],
            target: contract::command::TargetRef::Identified { id: target },
        },
        queued: false,
    };
    assert_eq!(battle.accept(attack).error, None);
    let mut last = None;
    for _ in 0..battle.rules().tick_hz * 30 {
        let tank = battle.unit(UnitId(2)).unwrap();
        if !tank.alive() {
            break;
        }
        last = Some(([tank.position.x, tank.position.y], tank.drive_speed_mps));
        battle.step();
    }
    assert!(
        !battle.unit(UnitId(2)).unwrap().alive(),
        "red never killed the tank"
    );
    let (at, speed) = last.expect("it drove");
    let tank = battle.unit(UnitId(2)).unwrap();
    let braking = tank.ground().road_mps / battle.rules().movement.drive.wreck_stop_s;
    let wreck = battle
        .observe(Side::Blue)
        .known_props
        .iter()
        .find(|p| p.wreck_of == Some(tank_kind))
        .expect("blue knows its tank's wreck")
        .center;
    let heli = battle.unit(UnitId(0)).unwrap().position;
    assert!(
        !battle.unit(UnitId(1)).unwrap().on_field(),
        "the squad still rides"
    );
    Rolled {
        at,
        speed,
        braking,
        wreck,
        front: tank.hull.expect("a tank's hull").x,
        heli: [heli.x, heli.y],
    }
}

struct Rolled {
    at: [f64; 2],
    speed: f64,
    braking: f64,
    wreck: [f64; 2],
    /// From the tank's middle to its front.
    front: f64,
    heli: [f64; 2],
}

#[test]
fn a_wreck_rolls_on_under_soldiers_riding_a_helicopter() {
    // Find the stretch the wreck rolls with the helicopter well away, then
    // hover over the ground its front rolls onto.
    let away = killed_driving_with_riders_overhead([300.0, 150.0]);
    let onto = away.front + (away.wreck[0] - away.at[0]) / 2.0;
    let Rolled {
        at,
        speed,
        braking,
        wreck,
        front,
        heli,
    } = killed_driving_with_riders_overhead([away.at[0] + onto, away.at[1]]);
    assert!(speed > 3.0, "it was driving: {speed} m/s");
    let rolled = speed * speed / (2.0 * braking);
    let ahead = wreck[0] - at[0];
    let radius = battle_soldier_radius();
    assert!(
        heli[0] - at[0] > front + radius
            && heli[0] - at[0] < front + rolled
            && (heli[1] - at[1]).abs() < 0.5,
        "the helicopter {heli:?} is not over the ground the tank's front rolls onto from {at:?} to {wreck:?}"
    );
    assert!(
        (ahead - rolled).abs() < speed / 30.0 + 0.05,
        "rolled {ahead} m under the riders, locked tracks stop it in {rolled} m"
    );
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

#[derive(Clone, Copy, PartialEq)]
enum UnloadDuringBoarding {
    /// No unload: the time an undisturbed boarding takes.
    None,
    Room,
    /// A wall dropped on the APC leaves no room, so the unload is refused.
    Blocked,
}

/// What a squad getting in went through while another got out.
struct BoardedThroughUnload {
    refused: bool,
    /// Whether the riding squad ended on the field.
    rider_out: bool,
    /// Ticks from the boarding squad's hold starting to its being inside.
    getting_in: u64,
    /// Whether the boarding squad's soldiers stood still until inside.
    stood_still: bool,
}

/// Squad A (unit 1) rides the APC (unit 0), which lets it out in place
/// unless `unload` is `None`; once A is halfway out, squad B (unit 2),
/// standing beside the APC, starts getting in.
fn boarding_through_an_unload(unload: UnloadDuringBoarding) -> BoardedThroughUnload {
    let events = if unload == UnloadDuringBoarding::Blocked {
        json!([{"tick":95,"add_prop":{"kind":"wall","center":[201,200],"yaw":0,"half_extents":[10,10,4]}}])
    } else {
        json!([])
    };
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    let mut battle = Battle::new(
        &setup_with_map(
            map,
            json!([
                {"side":"blue","kind":"test_apc","position":[200,200]},
                {"side":"blue","kind":"test_recon","position":[195,200]},
                {"side":"blue","kind":"test_recon","position":[195,202]}
            ]),
            events,
        ),
        15,
    );
    let rules = battle.rules();
    let timer = (rules.carriage.board_s * rules.tick_hz as f64).round() as u64;
    let board = |seq, squad| {
        command(
            seq,
            Order::Board {
                units: vec![squad],
                carrier: UnitId(0),
                gesture: 1,
            },
        )
    };
    assert_eq!(battle.accept(board(1, UnitId(1))).error, None);
    while battle.tick() < 90 {
        battle.step();
    }
    assert!(
        !battle.unit(UnitId(1)).unwrap().on_field(),
        "A never boarded"
    );
    let phase = |battle: &Battle, id: UnitId| {
        battle
            .unit(id)
            .unwrap()
            .hold
            .as_ref()
            .map(|hold| hold.phase)
    };
    if unload != UnloadDuringBoarding::None {
        let unload = Order::Unload {
            units: vec![UnitId(0)],
            goal: None,
            gesture: 2,
            route: Default::default(),
        };
        assert_eq!(battle.accept(command(2, unload)).refused, vec![]);
        let halfway = (0..timer * 2).any(|_| {
            battle.step();
            matches!(phase(&battle, UnitId(1)), Some(sim::hold::Phase::Exiting(n)) if 2 * u64::from(n) >= timer)
        });
        assert!(halfway, "A never got halfway out");
    }
    let seq = if unload == UnloadDuringBoarding::None {
        2
    } else {
        3
    };
    assert_eq!(battle.accept(board(seq, UnitId(2))).error, None);
    let mut refused = false;
    let mut started = None;
    let mut stood = None;
    let mut stood_still = true;
    let inside = (0..timer * 4).any(|_| {
        battle.step();
        refused |= battle.observe(Side::Blue).refusals.iter().any(|refusal| {
            refusal.unit == UnitId(0) && refusal.reason == OrderError::NoRoomToUnload
        });
        match phase(&battle, UnitId(2)) {
            Some(sim::hold::Phase::Inside) => return true,
            Some(sim::hold::Phase::Entering(_)) => {
                started.get_or_insert(battle.tick());
                let here: Vec<_> = battle.unit(UnitId(2)).unwrap().member_positions().collect();
                stood_still &= stood.get_or_insert_with(|| here.clone()) == &here;
            }
            _ => {}
        }
        false
    });
    assert!(inside, "B never got in");
    BoardedThroughUnload {
        refused,
        rider_out: battle.unit(UnitId(1)).unwrap().on_field(),
        getting_in: battle.tick() - started.expect("B started getting in"),
        stood_still,
    }
}

#[test]
fn a_squad_getting_in_keeps_getting_in_while_others_get_out() {
    let undisturbed = boarding_through_an_unload(UnloadDuringBoarding::None).getting_in;
    let b = boarding_through_an_unload(UnloadDuringBoarding::Room);
    assert!(!b.refused);
    assert!(b.rider_out, "the riding squad got out");
    assert!(b.stood_still, "a squad getting in was moved by the unload");
    assert_eq!(
        b.getting_in, undisturbed,
        "getting in took a different time"
    );
}

#[test]
fn a_refused_unload_cuts_short_no_one_getting_in() {
    let undisturbed = boarding_through_an_unload(UnloadDuringBoarding::None).getting_in;
    let b = boarding_through_an_unload(UnloadDuringBoarding::Blocked);
    assert!(b.refused, "the blocked unload was not refused");
    assert!(!b.rider_out, "the riding squad stays aboard");
    assert!(
        b.stood_still,
        "a squad getting in was moved by the refused unload"
    );
    assert_eq!(
        b.getting_in, undisturbed,
        "getting in took a different time"
    );
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
    let before = battle.observe(Side::Blue).seat().credits;
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
    let after = battle.observe(Side::Blue).seat().credits;
    assert!(after > before);
    assert!(battle.unit(UnitId(0)).unwrap().retired);
    assert!(battle.unit(UnitId(1)).unwrap().retired);
}

/// Blue's 1 hp APC with `squads` recon squads (units 1..=squads) boarding it
/// (for `board_s` when given), destroyed by a red gun jeep once every squad's
/// hold reaches a phase `ready` accepts, and still in such a phase when the
/// carrier dies. Returns the battle on the tick the carrier died and each
/// squad's soldiers' health before.
fn carrier_destroyed_once(
    squads: u32,
    board_s: Option<f64>,
    ready: fn(sim::hold::Phase) -> bool,
) -> (Battle, Vec<Vec<f64>>) {
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
    let squad_ids: Vec<UnitId> = (1..=squads).map(UnitId).collect();
    let jeep = UnitId(squads + 1);
    let mut units = vec![
        json!({"side":"blue","kind":"test_apc","position":[200,200],"engagement":"return_fire_only"}),
    ];
    for k in 0..squads {
        units.push(json!({"side":"blue","kind":"test_recon","position":[195, 200 + 6 * k],"engagement":"return_fire_only"}));
    }
    units.push(json!({"side":"red","kind":"test_gun_jeep","position":[260,200],"engagement":"return_fire_only"}));
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events":[], "scripts":[]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 42);
    assert!(battle
        .accept(command(
            1,
            Order::Board {
                units: squad_ids.clone(),
                carrier: UnitId(0),
                gesture: 1
            }
        ))
        .error
        .is_none());
    let phases = |battle: &Battle| -> Vec<Option<sim::hold::Phase>> {
        (squad_ids.iter())
            .map(|&id| battle.unit(id).unwrap().hold.as_ref().map(|h| h.phase))
            .collect()
    };
    let reached = (0..battle.rules().tick_hz * 10).any(|_| {
        battle.step();
        phases(&battle)
            .into_iter()
            .all(|phase| phase.is_some_and(ready))
    });
    assert!(reached, "the squads never reached the boarding phase");
    let health = (squad_ids.iter())
        .map(|&id| {
            battle
                .unit(id)
                .unwrap()
                .members
                .iter()
                .map(|s| s.hp)
                .collect()
        })
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
            units: vec![jeep],
            target: contract::command::TargetRef::Identified { id: carrier },
        },
        queued: false,
    };
    assert_eq!(battle.accept(attack).error, None);
    let destroyed = (0..battle.rules().tick_hz * 10).any(|_| {
        let phases = phases(&battle);
        assert!(
            phases.iter().all(|phase| phase.is_some_and(ready)),
            "a squad left the boarding phase before its carrier died: {phases:?}"
        );
        battle.step();
        !battle.unit(UnitId(0)).unwrap().alive()
    });
    assert!(destroyed, "the gun jeep never destroyed the carrier");
    (battle, health)
}

#[test]
fn a_destroyed_carrier_bails_out_its_passengers_hurt_and_pinned() {
    let (battle, health) =
        carrier_destroyed_once(1, None, |phase| phase == sim::hold::Phase::Inside);
    let rule = &battle.rules().bail_out[&BailOutHost::Ground];
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(
        squad.hold.is_none(),
        "no rider stays in a destroyed carrier"
    );
    for (soldier, before) in squad.members.iter().zip(&health[0]) {
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
    let (battle, health) = carrier_destroyed_once(1, Some(30.0), |phase| {
        matches!(phase, sim::hold::Phase::Entering(_))
    });
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(squad.on_field());
    assert_eq!(
        squad.members.iter().map(|s| s.hp).collect::<Vec<_>>(),
        health[0],
        "a squad standing beside the carrier is not hurt as if it rode inside"
    );
    assert!(
        squad.suppression < battle.rules().suppression.pinned.level,
        "a squad standing beside the carrier is not pinned by a bail-out"
    );
}

#[test]
fn two_squads_bailing_out_of_one_carrier_are_hurt_independently() {
    let (battle, health) =
        carrier_destroyed_once(2, None, |phase| phase == sim::hold::Phase::Inside);
    let lost: Vec<Vec<f64>> = [UnitId(1), UnitId(2)]
        .into_iter()
        .zip(&health)
        .map(|(id, before)| {
            let squad = battle.unit(id).unwrap();
            (squad.members.iter().zip(before))
                .map(|(s, b)| b - s.hp)
                .collect()
        })
        .collect();
    assert_ne!(lost[0], lost[1], "both squads drew the same injuries");
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

/// Blue's 1 hp APC (unit 0) with a recon squad (unit 1) standing at `squad`
/// and a red gun jeep (unit 2) ordered to destroy the carrier. `before_kill`
/// runs once before the jeep is ordered. Returns the battle on the tick the
/// carrier died.
fn carrier_killed_with_squad_at(squad: [f64; 2], before_kill: impl FnOnce(&mut Battle)) -> Battle {
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_apc",
        json!({ "body": { "hull": { "hp": 1.0 } } }),
    );
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]});
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": map, "rules": rules, "events":[], "scripts":[],
        "units": [
            {"side":"blue","kind":"test_apc","position":[200,200],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_recon","position":squad,"engagement":"return_fire_only"},
            {"side":"red","kind":"test_gun_jeep","position":[260,200],"engagement":"return_fire_only"}
        ]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 42);
    before_kill(&mut battle);
    battle.step();
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
        battle.step();
        !battle.unit(UnitId(0)).unwrap().alive()
    });
    assert!(destroyed, "the gun jeep never destroyed the carrier");
    battle
}

fn board_carrier(seq: u64) -> CommandEnvelope {
    command(
        seq,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(0),
            gesture: seq,
        },
    )
}

#[test]
fn a_squad_cannot_board_a_destroyed_carrier() {
    let mut battle = carrier_killed_with_squad_at([150.0, 200.0], |_| {});
    assert_eq!(
        battle.accept(board_carrier(1)).error,
        Some(OrderError::Destroyed { unit: UnitId(0) })
    );
}

#[test]
fn a_squad_walking_to_a_carrier_that_dies_stops_and_hears_why() {
    let mut battle = carrier_killed_with_squad_at([120.0, 200.0], |battle| {
        assert_eq!(battle.accept(board_carrier(1)).error, None);
    });
    let mut refused = false;
    for _ in 0..battle.rules().tick_hz {
        battle.step();
        refused |=
            battle.observe(Side::Blue).refusals.iter().any(|r| {
                r.unit == UnitId(1) && r.reason == OrderError::Destroyed { unit: UnitId(0) }
            });
    }
    assert!(refused, "the squad was never told its carrier is gone");
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(
        squad.orders.is_empty(),
        "the squad still chases the wreck: {:?}",
        squad.orders
    );
}

/// Carrier 0 sent from 200 m to collect `squads`; squad 1 climbs into carrier
/// 2 beside it first. Returns the battle 5 s later and every refusal blue
/// heard about squad 1.
fn collected_squad_boards_another_carrier(
    squads: Vec<UnitId>,
) -> (Battle, Vec<contract::command::Refusal>) {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[400,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]},
            {"side":"blue","kind":"test_apc","position":[200,200]}
        ])),
        7,
    );
    let collect = battle.accept(command(
        1,
        Order::Collect {
            units: vec![UnitId(0)],
            squads,
            gesture: 1,
        },
    ));
    assert_eq!(collect.error, None);
    let board = battle.accept(command(
        2,
        Order::Board {
            units: vec![UnitId(1)],
            carrier: UnitId(2),
            gesture: 2,
        },
    ));
    assert_eq!(board.error, None);
    let mut refusals = Vec::new();
    for _ in 0..(battle.rules().tick_hz * 5) {
        battle.step();
        refusals.extend(
            (battle.observe(Side::Blue).refusals.iter())
                .filter(|r| r.unit == UnitId(1))
                .cloned(),
        );
    }
    (battle, refusals)
}

#[test]
fn a_carrier_stops_driving_to_a_squad_another_carrier_took() {
    let (battle, refusals) = collected_squad_boards_another_carrier(vec![UnitId(1)]);
    assert!(
        refusals
            .iter()
            .any(|r| r.reason == OrderError::Aboard { unit: UnitId(1) }),
        "the carrier was never told the squad is aboard another"
    );
    let carrier = battle.unit(UnitId(0)).unwrap();
    assert!(
        carrier.orders.is_empty(),
        "the carrier still drives to where the squad stood: {:?}",
        carrier.orders
    );
}

#[test]
fn a_squad_listed_twice_boards_once_and_waits_for_nothing() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_recon","position":[195,200]}
        ])),
        7,
    );
    // Queued, so a second copy would wait behind the first.
    let mut board = command(
        1,
        Order::Board {
            units: vec![UnitId(1), UnitId(1)],
            carrier: UnitId(0),
            gesture: 1,
        },
    );
    board.queued = true;
    let ack = battle.accept(board);
    assert_eq!(ack.error, None);
    assert!(ack.refused.is_empty(), "{:?}", ack.refused);
    let aboard = (0..battle.rules().tick_hz * 10).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|u| u.hold.as_ref())
            .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
    });
    assert!(aboard, "the squad never got in");
    let squad = battle.unit(UnitId(1)).unwrap();
    assert!(
        squad.orders.is_empty(),
        "a second copy of the order is left queued: {:?}",
        squad.orders
    );
}

#[test]
fn a_squad_listed_twice_for_collection_is_refused_once() {
    let (_, refusals) = collected_squad_boards_another_carrier(vec![UnitId(1), UnitId(1)]);
    assert_eq!(
        refusals,
        vec![contract::command::Refusal {
            unit: UnitId(1),
            reason: OrderError::Aboard { unit: UnitId(1) },
        }]
    );
}

/// Blue's APC (unit 0) beside a building, and a recon squad (unit 1) already
/// inside it.
fn squad_in_a_building() -> Battle {
    let map = json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],
        "buildings":[crate::common::building(0, "china-apartment-point-20x20-7f", [400.0, 300.0], 0.0)]});
    let mut battle = Battle::new(
        &setup_with_map(
            map,
            json!([
                {"side":"blue","kind":"test_apc","position":[360,300]},
                {"side":"blue","kind":"test_recon","position":[380,300]}
            ]),
            json!([]),
        ),
        7,
    );
    let garrison = battle.accept(command(
        1,
        Order::Garrison {
            units: vec![UnitId(1)],
            building: 0,
        },
    ));
    assert_eq!(garrison.error, None);
    let inside = (0..battle.rules().tick_hz * 30).any(|_| {
        battle.step();
        battle.unit(UnitId(1)).unwrap().garrisoned()
    });
    assert!(inside, "the squad never got into the building");
    battle
}

/// Step until squad 1 rides carrier 0, at most `seconds`.
fn rides_carrier_within(battle: &mut Battle, seconds: u32) -> bool {
    (0..battle.rules().tick_hz * seconds).any(|_| {
        battle.step();
        battle
            .unit(UnitId(1))
            .and_then(|u| u.hold.as_ref())
            .is_some_and(|h| {
                h.host.carrier() == Some(UnitId(0)) && h.phase == sim::hold::Phase::Inside
            })
    })
}

#[test]
fn a_squad_in_a_building_ordered_to_board_leaves_and_boards() {
    let mut battle = squad_in_a_building();
    assert_eq!(battle.accept(board_carrier(2)).error, None);
    assert!(
        rides_carrier_within(&mut battle, 30),
        "the squad never left the building for the carrier"
    );
}

#[test]
fn a_carrier_collects_a_squad_from_a_building() {
    let mut battle = squad_in_a_building();
    let collect = battle.accept(command(
        2,
        Order::Collect {
            units: vec![UnitId(0)],
            squads: vec![UnitId(1)],
            gesture: 2,
        },
    ));
    assert_eq!(collect.error, None);
    assert!(
        rides_carrier_within(&mut battle, 40),
        "the carrier never took the squad out of the building"
    );
}
