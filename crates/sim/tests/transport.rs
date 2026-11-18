use crate::common;
use contract::command::{
    CommandEnvelope, Engagement, MoveDirection, Order, OrderError, RoutePolicy,
};
use contract::ids::{Side, UnitId};
use contract::observation::HoldPhase;
use contract::scenario::BailOutHost;
use serde_json::json;
use sim::battle::Battle;
use sim::fixtures::test_rules;

mod helicopter;
mod unloading;

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

pub(crate) fn side_command(side: Side, seq: u64, order: Order) -> CommandEnvelope {
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

pub(crate) fn transport_skirmish_setup() -> contract::scenario::ScenarioDefinition {
    let mut rules = test_rules();
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
        into: None,
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
#[path = "transport/death_roll.rs"]
mod death_roll;

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
    let mut game = test_rules();
    sim::fixtures::patch_catalog(
        &mut game,
        "units",
        "test_apc",
        json!({"capabilities":{"carriage":{"seats":7}}}),
    );
    assert!(serde_json::from_value::<contract::scenario::Rules>(game).is_err());
}

#[test]
fn a_squad_boarding_a_carrier_another_squad_fills_finds_no_room() {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_apc","position":[200,200]},
            {"side":"blue","kind":"test_rifle","position":[195,200]},
            {"side":"blue","kind":"test_recon","position":[196,200]}
        ])),
        12,
    );
    let board = |seq, unit| {
        command(
            seq,
            Order::Board {
                units: vec![UnitId(unit)],
                carrier: UnitId(0),
                gesture: seq,
            },
        )
    };
    let hold = |b: &Battle, id| b.unit(UnitId(id)).unwrap().hold.as_ref().map(|h| h.phase);
    assert!(battle.accept(board(1, 1)).error.is_none());
    battle.step();
    assert!(matches!(
        hold(&battle, 1),
        Some(sim::hold::Phase::Entering(_))
    ));
    // The command promises nothing and refuses nothing: room is decided on
    // arrival.
    let ack = battle.accept(board(2, 2));
    assert_eq!((ack.error, ack.refused), (None, vec![]));
    let mut heard = Vec::new();
    for _ in 0..(battle.rules().tick_hz * 2) {
        battle.step();
        heard.extend(battle.observe(Side::Blue).refusals.clone());
    }
    assert_eq!(
        heard,
        [contract::command::Refusal {
            unit: UnitId(2),
            reason: OrderError::CapacityFull
        }]
    );
    assert_eq!(hold(&battle, 1), Some(sim::hold::Phase::Inside));
    let late = battle.unit(UnitId(2)).unwrap();
    assert!(late.hold.is_none() && late.orders.is_empty());
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
            into: None,
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
    let mut rules = test_rules();
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

/// Blue's 1 hp APC (unit 0) with a recon squad (unit 1) standing at `squad`
/// and a red gun jeep (unit 2) ordered to destroy the carrier. `before_kill`
/// runs once before the jeep is ordered. Returns the battle on the tick the
/// carrier died.
fn carrier_killed_with_squad_at(squad: [f64; 2], before_kill: impl FnOnce(&mut Battle)) -> Battle {
    let mut rules = test_rules();
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
    let order = Order::OccupyBuilding {
        units: vec![UnitId(1)],
        building: 0,
        gesture: 0,
    };
    assert_eq!(battle.accept(command(1, order)).error, None);
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
