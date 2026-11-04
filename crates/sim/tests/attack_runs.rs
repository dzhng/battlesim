//! Repeated attack runs on a test field: each run lines up, commits
//! nose-on and fires, near the middle, an edge or a corner, and how runs end.
use crate::common;
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::air::pass::PassPhase;
use sim::fixtures::{self, test_rules};

/// The test jet's own numbers for the run tests: every bound they assert is
/// derived from these, not from the shipped catalog.
const RUN_CRUISE_KMH: f64 = 540.0;
const RUN_TURN_DEG_S: f64 = 60.0;
const RUN_EDGE_MARGIN_M: f64 = 300.0;
const RUN_ROLL_IN_S: f64 = 0.8;
const RUN_BOMB_CEILING_M: f64 = 200.0;
const RUN_STRAFE_CEILING_M: f64 = 130.0;
const RUN_GRAVITY_MPS2: f64 = 9.81;
const RUN_BOMB_GRAVITY_SCALE: f64 = 4.5;
const RUN_GUN_RANGE_M: f64 = 900.0;
/// The side of the square field the run tests fly over.
const RUN_FIELD_M: f64 = 4000.0;
/// Where the jet starts, heading east.
pub(crate) const RUN_START: [f64; 2] = [1000.0, 2000.0];
/// How long the run tests watch a jet attack.
pub(crate) const RUN_WINDOW_S: f64 = 120.0;

/// What the run tests' bounds are made of, from the numbers above.
struct RunBounds {
    speed: f64,
    /// The longest one run may take, from where the last one ended: flying
    /// straight out until a turn in leaves a run-in (2 s of flight plus a
    /// bomb's forward throw from its ceiling) and a turn diameter, a lap
    /// turning in, one more lap the edge guard may add, then the run-in.
    run_s: f64,
}

fn run_bounds() -> RunBounds {
    let speed = RUN_CRUISE_KMH / 3.6;
    let turn = RUN_TURN_DEG_S.to_radians();
    let lap_s = std::f64::consts::TAU / turn;
    let fall_s = (2.0 * RUN_BOMB_CEILING_M / (RUN_GRAVITY_MPS2 * RUN_BOMB_GRAVITY_SCALE)).sqrt();
    let leg = 2.0 * speed + speed * fall_s + 2.0 * speed / turn;
    RunBounds {
        speed,
        run_s: 2.0 * lap_s + 2.0 * leg / speed,
    }
}

/// The test rules for the run tests, with the test jet carrying `weapons`,
/// each on its own mount.
pub(crate) fn run_rules(weapons: &[&str]) -> serde_json::Value {
    let mut rules = test_rules();
    rules["air"]["fixed_wing"]["edge_margin_m"] = json!(RUN_EDGE_MARGIN_M);
    rules["air"]["fixed_wing"]["roll_in_s"] = json!(RUN_ROLL_IN_S);
    rules["air"]["fixed_wing"]["bomb_ceiling_m"] = json!(RUN_BOMB_CEILING_M);
    rules["air"]["fixed_wing"]["strafe_ceiling_m"] = json!(RUN_STRAFE_CEILING_M);
    rules["physics"]["gravity_mps2"] = json!(RUN_GRAVITY_MPS2);
    rules["weapons"]["air_bomb"]["gravity_scale"] = json!(RUN_BOMB_GRAVITY_SCALE);
    rules["weapons"]["air_bomb"]["ammo"] = json!(1000);
    rules["weapons"]["air_bomb"]["aim_s"] = json!(0.1);
    // Rearming between runs is not these tests' subject.
    rules["weapons"]["air_bomb"]["reload_s"] = json!(1.0);
    common::json_ground_reach(&mut rules["weapons"]["air_gun"], RUN_GUN_RANGE_M);
    // Mounts merge by id: the first takes the place of the jet's own gun.
    let mounts: Vec<_> = weapons
        .iter()
        .enumerate()
        .map(|(i, weapon)| {
            let id = if i == 0 {
                "gun".into()
            } else {
                format!("stores_{i}")
            };
            json!({
                "id": id, "name": "Stores", "weapons": [*weapon],
                "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0]
            })
        })
        .collect();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({
            "mobility": { "air": { "cruise_kmh": RUN_CRUISE_KMH, "turn_deg_s": RUN_TURN_DEG_S } },
            "mounts": mounts
        }),
    );
    rules
}

/// A battle on the run field with `units` placed, the test jet first.
pub(crate) fn run_field(rules: serde_json::Value, units: serde_json::Value) -> sim::battle::Battle {
    let setup = serde_json::from_value(json!({
        "map": { "size": [RUN_FIELD_M, RUN_FIELD_M], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": units,
        "events": [], "scripts": []
    }))
    .unwrap();
    sim::battle::Battle::new(&setup, 1)
}

/// A test jet carrying `weapons` at `RUN_START` on the run field, heading
/// east, under an explicit attack order on the ground at `aim`.
fn run_battle(weapons: &[&str], aim: [f64; 2]) -> sim::battle::Battle {
    let mut battle = run_field(
        run_rules(weapons),
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    let ack = battle.accept(attack_ground(1, aim));
    assert_eq!(ack.error, None);
    battle
}

pub(crate) fn attack_ground(seq: u64, aim: [f64; 2]) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [aim[0], aim[1], 0.0],
            },
        },
    }
}

/// One run: from the tick the jet committed (nose on the aim) to the tick it
/// ended, with how far out it committed, how far off the nose the aim lay
/// then, and the rounds it fired or dropped.
#[derive(Debug)]
struct Run {
    start_s: f64,
    end_s: Option<f64>,
    commit_m: f64,
    off_deg: f64,
    fired: u32,
}

/// What a jet flew over a window: its runs, and each stretch of one phase
/// (`None` with no run planned) with when it began and how long it lasted.
struct Flown {
    runs: Vec<Run>,
    stretches: Vec<(Option<PassPhase>, f64, f64)>,
}

fn rounds(battle: &sim::battle::Battle) -> u32 {
    battle
        .unit(UnitId(0))
        .unwrap()
        .mounts
        .iter()
        .map(|m| m.shots)
        .sum()
}

fn fly_runs(battle: &mut sim::battle::Battle, aim: [f64; 2], window_s: f64) -> Flown {
    let hz = f64::from(battle.rules().tick_hz);
    let aim = sim::math::v2(aim[0], aim[1]);
    let mut flown = Flown {
        runs: Vec::new(),
        stretches: Vec::new(),
    };
    let mut current: Option<(Option<PassPhase>, f64)> = None;
    let mut rounds_at_commit = 0;
    for tick in 1..=(window_s * hz) as u64 {
        let before = rounds(battle);
        let at = battle.unit(UnitId(0)).unwrap().position.xy();
        battle.step();
        let now = tick as f64 / hz;
        let jet = battle.unit(UnitId(0)).unwrap();
        let phase = jet.air.unwrap().pass.map(|p| p.phase);
        if current.is_some_and(|(p, _)| p == phase) {
            continue;
        }
        if let Some((previous, began)) = current {
            flown.stretches.push((previous, began, now - began));
            if previous == Some(PassPhase::Dive) {
                let run = flown.runs.last_mut().unwrap();
                run.end_s = Some(now);
                run.fired = rounds(battle) - rounds_at_commit;
            }
        }
        if phase == Some(PassPhase::Dive) {
            // The jet committed from where it was when this tick began.
            let to_aim = aim - at;
            let off = sim::math::wrap_angle(libm::atan2(to_aim.y, to_aim.x) - jet.yaw);
            flown.runs.push(Run {
                start_s: now,
                end_s: None,
                commit_m: to_aim.length(),
                off_deg: off.to_degrees(),
                fired: 0,
            });
            rounds_at_commit = before;
        }
        current = Some((phase, now));
    }
    if let Some((phase, began)) = current {
        flown.stretches.push((phase, began, window_s - began));
    }
    flown
}

/// The run contract, for a jet that started at `RUN_START` and attacked
/// `aim` for `window_s`: each run begins nose-on and is over (fired, or
/// broken) within the time to fly its own run-in; no phase is held longer
/// than one run's length; and runs that fire keep coming.
fn assert_runs_keep_firing(weapon: &str, aim: [f64; 2], flown: &Flown, window_s: f64) {
    let bounds = run_bounds();
    let approach_s = (sim::math::v2(aim[0], aim[1]) - sim::math::v2(RUN_START[0], RUN_START[1]))
        .length()
        / bounds.speed
        + bounds.run_s;
    let at = format!("{weapon} at {aim:?}");
    assert!(
        !flown.runs.is_empty(),
        "{at}: the jet never committed to a run"
    );
    for run in &flown.runs {
        assert!(
            run.off_deg.abs() <= 10.0,
            "{at}: a run began {:.0} deg off the aim, {:.0} m out, at {:.1}s",
            run.off_deg,
            run.commit_m,
            run.start_s
        );
        if let Some(end) = run.end_s {
            assert!(
                end - run.start_s <= run.commit_m / bounds.speed + 1.0,
                "{at}: a run committed {:.0} m out at {:.1}s lasted {:.1}s",
                run.commit_m,
                run.start_s,
                end - run.start_s
            );
        }
    }
    let first_end = flown.runs[0].end_s.unwrap_or(window_s);
    for &(phase, began, held) in &flown.stretches {
        let limit = if began < first_end {
            approach_s
        } else {
            bounds.run_s
        };
        assert!(
            held <= limit,
            "{at}: held {phase:?} for {held:.1}s from {began:.1}s, longer than {limit:.1}s"
        );
    }
    let mut last = 0.0;
    let mut limit = approach_s + bounds.run_s;
    for end in flown
        .runs
        .iter()
        .filter(|r| r.fired > 0)
        .filter_map(|r| r.end_s)
        .chain([window_s])
    {
        assert!(
            end - last <= limit,
            "{at}: no run fired between {last:.1}s and {end:.1}s; runs {:?}",
            flown.runs
        );
        last = end;
        limit = 2.0 * bounds.run_s;
    }
}

fn runs_keep_firing(aim: [f64; 2]) {
    for weapon in ["air_bomb", "air_gun"] {
        let mut battle = run_battle(&[weapon], aim);
        let flown = fly_runs(&mut battle, aim, RUN_WINDOW_S);
        assert_runs_keep_firing(weapon, aim, &flown, RUN_WINDOW_S);
    }
}

/// Strafing or bombing a ground target is back and forth: line up, commit
/// nose-on, fire or release, bank away, come round and go again. Every run
/// after the first must line up as well as the first.
#[test]
fn repeated_passes_each_dive_in_nose_on_and_fire() {
    // 1 km east of the jet, in the middle of the field (far inside the
    // edge guard).
    runs_keep_firing([2000.0, 2000.0]);
}

/// A target by the map's edge gets run after run like any other: the jet
/// commits when its nose comes on, fires or drops if it gets there before
/// the edge guard turns it away, and comes round again. It never circles
/// over the target unable to attack. A bomb is thrown its fall's flight
/// short of the aim, and the jet must roll into its turn away before the
/// edge: from its ceiling, it can't reach an aim 50 m from the edge before
/// the guard takes it (D4/D10′), so only the gun attacks one there.
#[test]
fn runs_at_an_aim_near_an_edge_keep_firing() {
    for from_edge in [100.0, 150.0] {
        runs_keep_firing([RUN_FIELD_M - from_edge, 2000.0]);
    }
    let aim = [RUN_FIELD_M - 50.0, 2000.0];
    let flown = fly_runs(&mut run_battle(&["air_gun"], aim), aim, RUN_WINDOW_S);
    assert_runs_keep_firing("air_gun", aim, &flown, RUN_WINDOW_S);
}

#[test]
fn runs_at_an_aim_in_a_corner_keep_firing() {
    runs_keep_firing([RUN_FIELD_M - 100.0, RUN_FIELD_M - 100.0]);
}

/// A run at a target that dies is over at once: the jet doesn't fly on at
/// the wreck.
#[test]
fn a_run_ends_when_its_target_dies() {
    let mut battle = run_field(
        run_rules(&["air_gun"]),
        json!([
            { "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 },
            { "side": "blue", "kind": "test_recon", "position": [2000, 2100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [2000, 2000], "yaw": 0.0, "engagement": "return_fire_only", "condition": { "hp": 1 } }
        ]),
    );
    for _ in 0..30 {
        battle.step();
    }
    let tank = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    let phase = |b: &sim::battle::Battle| {
        b.unit(UnitId(0))
            .unwrap()
            .air
            .unwrap()
            .pass
            .map(|p| p.phase)
    };
    let mut in_run_when_it_died = None;
    for _ in 0..(60.0 * f64::from(battle.rules().tick_hz)) as u64 {
        let before = phase(&battle);
        battle.step();
        if !battle.unit(UnitId(2)).unwrap().alive() {
            in_run_when_it_died = Some(before == Some(PassPhase::Dive));
            break;
        }
    }
    assert_eq!(
        in_run_when_it_died,
        Some(true),
        "the tank must die to a run's fire"
    );
    battle.step();
    battle.step();
    assert_ne!(
        phase(&battle),
        Some(PassPhase::Dive),
        "the jet flew on at the dead tank"
    );
}

/// Every order a jet takes ends the run it was flying, whatever the order
/// and even if it repeats the attack: on the next tick no release cue or
/// dogfight link survives, and the jet is either off the attack or lining
/// up afresh from where it is, as a re-issued attack does, which then fires.
#[test]
fn every_order_ends_the_run_at_once() {
    let aim = [2000.0, 2000.0];
    let elsewhere = [1000.0, 3000.0];
    let orders: Vec<(&str, Order)> = vec![
        (
            "move",
            Order::Move {
                units: vec![UnitId(0)],
                gesture: 2,
                goal: elsewhere,
                route: contract::command::RoutePolicy::Shortest,
                direction: contract::command::MoveDirection::Forward,
                facing: None,
            },
        ),
        (
            "attack-move",
            Order::AttackMove {
                units: vec![UnitId(0)],
                gesture: 2,
                goal: elsewhere,
            },
        ),
        (
            "stop",
            Order::Stop {
                units: vec![UnitId(0)],
            },
        ),
        ("attack elsewhere", attack_ground(2, elsewhere).order),
        ("the same attack", attack_ground(2, aim).order),
        (
            "engagement",
            Order::SetEngagement {
                units: vec![UnitId(0)],
                policy: contract::command::Engagement::FireAtWill,
            },
        ),
        (
            "afterburner",
            Order::SetAfterburner {
                units: vec![UnitId(0)],
                enabled: false,
            },
        ),
    ];
    let aim_v = sim::math::v2(aim[0], aim[1]);
    let bearing = |b: &sim::battle::Battle| {
        let to_aim = aim_v - b.unit(UnitId(0)).unwrap().position.xy();
        libm::atan2(to_aim.y, to_aim.x)
    };
    let plan = |b: &sim::battle::Battle| b.unit(UnitId(0)).unwrap().air.unwrap().pass;
    for phase in [PassPhase::LineUp, PassPhase::Dive] {
        for (name, order) in &orders {
            let mut battle = run_battle(&["air_gun"], aim);
            let hz = f64::from(battle.rules().tick_hz);
            // Mid-run: in a dive, where a fresh line-up would commit along a
            // different line than the one the dive is flying.
            let mut reached = false;
            for _ in 0..(60.0 * hz) as u64 {
                battle.step();
                reached = plan(&battle).is_some_and(|p| {
                    p.phase == phase
                        && (phase == PassPhase::LineUp || p.heading != bearing(&battle))
                });
                if reached {
                    break;
                }
            }
            assert!(reached, "{name}: the jet never reached {phase:?}");
            let from = bearing(&battle);
            let ack = battle.accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 2,
                queued: false,
                order: order.clone(),
            });
            assert_eq!(ack.error, None, "{name}");
            battle.step();
            let air = battle.unit(UnitId(0)).unwrap().air.unwrap();
            assert!(
                !air.pass_release,
                "{name} in {phase:?}: a release cue survived"
            );
            assert_eq!(air.dogfight_target, None, "{name} in {phase:?}");
            // Still attacking the same aim, it lines up afresh: either not
            // yet committed, or committed along the line from where it was.
            if let Some(p) = air.pass.filter(|p| p.aim.xy() == aim_v) {
                assert!(
                    p.phase == PassPhase::LineUp || p.heading == from,
                    "{name} in {phase:?}: the jet kept flying its old run {p:?}"
                );
            }
        }
    }
    // A re-issued attack on the same target starts a fresh run that fires.
    let mut battle = run_battle(&["air_gun"], aim);
    for _ in 0..(60.0 * f64::from(battle.rules().tick_hz)) as u64 {
        battle.step();
        if plan(&battle).is_some_and(|p| p.phase == PassPhase::Dive) {
            break;
        }
    }
    battle.accept(attack_ground(2, aim));
    let flown = fly_runs(&mut battle, aim, run_bounds().run_s * 2.0);
    assert!(
        flown.runs.iter().any(|r| r.fired > 0),
        "the re-issued attack never fired: {:?}",
        flown.runs
    );
}

/// Once its ordered target dies, a jet circles where the attack ended (the
/// doc's "where its last order or run ended"), not where it was before the
/// order, kilometres back.
#[test]
fn a_jet_circles_where_its_attack_ended() {
    const CRUISE_KMH: f64 = 540.0;
    const TURN_DEG_S: f64 = 60.0;
    let radius = CRUISE_KMH / 3.6 / TURN_DEG_S.to_radians();
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mobility": { "air": { "cruise_kmh": CRUISE_KMH, "turn_deg_s": TURN_DEG_S } } }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [6000, 2000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [400, 1000], "yaw": 0.0, "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_recon", "position": [3500, 1100], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [3600, 1000], "yaw": 0.0, "engagement": "return_fire_only", "condition": { "hp": 1 } }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    for _ in 0..30 {
        battle.step();
    }
    let tank = battle
        .observe(Side::Blue)
        .identified
        .first()
        .map(|t| t.id)
        .expect("the recon sees the tank");
    let start = battle.unit(UnitId(0)).unwrap().position.xy();
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank },
        },
    });
    assert_eq!(ack.error, None);
    let hz = f64::from(battle.rules().tick_hz);
    let mut ended = None;
    for _ in 0..(120.0 * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap();
        if jet.orders.is_empty() {
            ended = Some(jet.position.xy());
            break;
        }
    }
    let ended = ended.expect("the attack never ended");
    assert!(
        !battle.unit(UnitId(2)).unwrap().alive(),
        "the attack ended with the tank alive"
    );
    assert!(
        (ended - start).length() > 10.0 * radius,
        "the attack ended near the start"
    );
    // A circle round where it ended passes within its diameter of it.
    for _ in 0..(60.0 * hz) as u64 {
        battle.step();
        let jet = battle.unit(UnitId(0)).unwrap().position.xy();
        assert!(
            (jet - ended).length() <= 3.0 * radius,
            "{:.0} m from where its attack ended, {:.0} m from where it began",
            (jet - ended).length(),
            (jet - start).length()
        );
    }
}

/// Two jets strafe the same target in trail, the wingman a little above
/// his lead. The wingman's rounds reach the lead's line only after the lead
/// has flown on, so the lead's current pose is clear of them and the pose
/// it flies into is not: the wingman holds rather than shoot it down.
#[test]
fn a_wingman_in_trail_never_strafes_through_his_lead() {
    for behind_m in [50.0, 70.0, 100.0, 140.0] {
        let mut battle = trail_strafe(behind_m);
        let jets = [UnitId(0), UnitId(1)];
        let mut fired = 0;
        for _ in 0..1200 {
            let blue: std::collections::BTreeMap<_, _> = battle
                .rounds()
                .filter(|(_, r)| jets.contains(&r.unit))
                .map(|(p, r)| (p.id, r.unit))
                .collect();
            battle.step();
            fired += battle
                .rounds()
                .filter(|(p, r)| r.unit == UnitId(1) && !blue.contains_key(&p.id))
                .count();
            for event in battle.flight_events() {
                let sim::flight::FlightEvent::Impact(hit) = event else {
                    continue;
                };
                let Some(shooter) = blue.get(&hit.projectile) else {
                    continue;
                };
                let struck = |jet: &UnitId| {
                    hit.struck
                        == sim::flight::Struck::Body(sim::flight::BodyId(
                            sim::weapons::VEHICLE_BODY_BASE + jet.0,
                        ))
                };
                if let Some(jet) = jets.iter().find(|j| *j != shooter && struck(j)) {
                    panic!(
                        "{behind_m} m in trail: {shooter:?} shot {jet:?} at tick {}",
                        battle.tick()
                    );
                }
            }
        }
        // Holding is not a freeze: once his lead is clear he strafes too.
        assert!(fired > 0, "{behind_m} m in trail: the wingman never fired");
    }
}

/// A test-jet lead at (500, 2000) and its wingman `behind_m` behind it,
/// both strafing the ground under a red tank 2.5 km east with their guns.
/// The strafe band reaches well below cruise, so their seeded pass heights
/// stand the wingman some metres above his lead.
fn trail_strafe(behind_m: f64) -> sim::battle::Battle {
    let mut rules = test_rules();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jet",
        json!({ "mounts": [{
            "id": "test_mount_0", "name": "Test Mount", "weapons": ["air_gun"],
            "turret": false, "pivot_m": [0, 0, 1.2], "muzzle_m": [7, 0, 0]
        }] }),
    );
    rules["air"]["high_air"]["floor_agl_m"] = json!(40);
    rules["air"]["fixed_wing"]["strafe_ceiling_m"] = json!(240);
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [500, 2000], "yaw": 0.0 },
            { "side": "blue", "kind": "test_jet", "position": [500.0 - behind_m, 2000], "yaw": 0.0 },
            { "side": "red", "kind": "test_tank", "position": [3000, 2000], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0), UnitId(1)],
            target: TargetRef::Ground {
                point: [3000.0, 2000.0, 0.0],
            },
        },
    });
    assert_eq!(ack.error, None);
    battle
}

/// A bombing run as a viewer sees it: the jet releases wings level on its
/// straight run and begins rolling into its break at once, building its
/// turn from nothing rather than snapping to it. Its turn rate never steps
/// or passes its full rate, and the bomb, released on a straight run, still
/// falls onto the aim.
#[test]
fn a_bomb_run_rolls_into_its_break_from_the_release() {
    let aim = [2000.0, 2000.0];
    let mut rules = run_rules(&["air_bomb"]);
    rules["weapons"]["air_bomb"]["targets"]["ground"]["scatter_mrad"] = json!(0.0);
    let blast_m = rules["weapons"]["air_bomb"]["blast_radius_m"]
        .as_f64()
        .unwrap();
    let mut battle = run_field(
        rules,
        json!([{ "side": "blue", "kind": "test_jet", "position": RUN_START, "yaw": 0.0 }]),
    );
    assert_eq!(battle.accept(attack_ground(1, aim)).error, None);
    let sim::units::Motion::Air(flight) = battle.unit(UnitId(0)).unwrap().motion else {
        panic!("test_jet flies");
    };
    let wing = battle.rules().air.fixed_wing;
    let dt = 1.0 / f64::from(battle.rules().tick_hz);
    let roll = flight.turn_rad_s / wing.roll_in_s * dt;
    let level = flight.turn_rad_s * 0.01;
    let (mut yaw, mut rate) = (battle.unit(UnitId(0)).unwrap().yaw, 0.0);
    let (mut released, mut bomb, mut landed) = (None, None, None);
    let (mut rolling, mut broke) = (None, None);
    for tick in 1..=(RUN_WINDOW_S / dt) as u64 {
        battle.step();
        let now = tick as f64 * dt;
        let jet = battle.unit(UnitId(0)).unwrap();
        let next = sim::math::wrap_angle(jet.yaw - yaw) / dt;
        assert!(
            (next - rate).abs() <= roll + 1e-9,
            "{now:.2}s: the turn rate stepped from {rate:.3} to {next:.3} rad/s"
        );
        assert!(
            next.abs() <= flight.turn_rad_s + 1e-9,
            "{now:.2}s: {next:.3} rad/s"
        );
        (yaw, rate) = (jet.yaw, next);
        if bomb.is_none() {
            bomb = battle
                .rounds()
                .find(|(_, r)| r.unit == UnitId(0))
                .map(|(p, _)| p.id);
            released = bomb.map(|_| now);
            if released.is_some() {
                assert!(rate.abs() < level, "it released turning at {rate:.3} rad/s");
            }
        }
        if let Some(since) = released.map(|at| now - at) {
            if rolling.is_none() && rate.abs() >= level {
                rolling = Some(since);
            }
            if broke.is_none() && rate.abs() >= flight.turn_rad_s * 0.99 {
                broke = Some(since);
            }
        }
        landed = landed.or_else(|| {
            battle.flight_events().iter().find_map(|e| match e {
                sim::flight::FlightEvent::Impact(i) if Some(i.projectile) == bomb => Some(i.point),
                _ => None,
            })
        });
        if broke.is_some() && landed.is_some() {
            break;
        }
    }
    released.expect("the jet never released its bomb");
    let rolling = rolling.expect("the jet never rolled into its break");
    assert!(
        rolling <= 2.0 * dt + 1e-9,
        "it flew on straight for {rolling:.2}s after the release"
    );
    let broke = broke.expect("the jet never reached its full turn rate");
    assert!(
        broke >= wing.roll_in_s * 0.9,
        "it reached its full turn rate {broke:.2}s after the release, under its roll-in"
    );
    let miss =
        (landed.expect("the bomb never landed").xy() - sim::math::v2(aim[0], aim[1])).length();
    assert!(miss < blast_m, "the bomb landed {miss:.0} m from the aim");
}

/// A jet flying along an edge at an aim near it bombs on its first
/// approach, rolling onto the aim and running straight in from its own
/// side: the edge it runs parallel to never sends it out on a lap to come
/// back at the aim from inside the map.
#[test]
fn a_bomb_run_along_an_edge_comes_straight_in_from_the_jets_side() {
    let aim = [2500.0, 700.0];
    let mut battle = run_field(
        run_rules(&["air_bomb"]),
        json!([{ "side": "blue", "kind": "test_jet", "position": [500.0, 700.0], "yaw": 0.45 }]),
    );
    assert_eq!(battle.accept(attack_ground(1, aim)).error, None);
    let dt = 1.0 / f64::from(battle.rules().tick_hz);
    let speed = run_bounds().speed;
    // Straight there, with a lap's worth of time to spare to line up.
    let lap_s = std::f64::consts::TAU / RUN_TURN_DEG_S.to_radians();
    let window_s = (aim[0] - 500.0) / speed + lap_s;
    for _ in 0..(window_s / dt) as u64 {
        battle.step();
        if battle.rounds().any(|(_, r)| r.unit == UnitId(0)) {
            let yaw = battle.unit(UnitId(0)).unwrap().yaw;
            assert!(
                sim::math::wrap_angle(yaw).abs() < std::f64::consts::FRAC_PI_4,
                "it released heading {:.0} deg, not in from its own side",
                yaw.to_degrees()
            );
            return;
        }
    }
    panic!("no bomb left the jet on its first approach, within {window_s:.1}s");
}
