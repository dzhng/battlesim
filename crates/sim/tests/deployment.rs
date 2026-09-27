//! Reversible deployment contracts (slice 12): one progress value with equal
//! durations both ways, reversal from current progress, the packing gate on
//! movement, Stop returning to deployed, and readiness for service.
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit, Posture};
use serde_json::json;
use sim::battle::Battle;
use sim::deployment::{self, Deployment};

mod common;

const SUPPLY: UnitId = UnitId(0);
const TANK: UnitId = UnitId(1);

/// Deployment duration in ticks, from the one fixture owner.
fn duration() -> u32 {
    let v = common::village();
    let seconds = v["service"]["deploy_and_pack_s"].as_f64().unwrap();
    (seconds * v["tick_hz"].as_f64().unwrap()).round() as u32
}

fn battle() -> Battle {
    let map = json!({ "size": [600, 400], "height_grid_m": 4, "slope_cutoff_deg": 35 }).to_string();
    let units = json!([
        { "side": "blue", "kind": "supply", "position": [100, 200] },
        { "side": "blue", "kind": "tank", "position": [100, 300] },
    ]);
    Battle::new(&common::scenario(&map, units, json!([])), 12)
}

struct Commander {
    seq: u64,
}
impl Commander {
    fn new() -> Self {
        Commander { seq: 0 }
    }
    fn send(&mut self, b: &mut Battle, order: Order, queued: bool) {
        self.seq += 1;
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: self.seq,
            order,
            queued,
        });
        assert_eq!(ack.error, None, "{ack:?}");
    }
    fn move_to(&mut self, b: &mut Battle, unit: UnitId, goal: [f64; 2], queued: bool) {
        let order = Order::Move {
            units: vec![unit],
            gesture: self.seq + 100,
            goal,
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
        };
        self.send(b, order, queued);
    }
    fn stop(&mut self, b: &mut Battle, unit: UnitId) {
        self.send(b, Order::Stop { units: vec![unit] }, false);
    }
    fn deploy(&mut self, b: &mut Battle, unit: UnitId, deployed: bool) {
        let order = Order::SetDeployment {
            units: vec![unit],
            deployed,
        };
        self.send(b, order, false);
    }
}

fn own(b: &Battle, id: UnitId) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == id)
        .unwrap()
        .clone()
}

/// Completed setup ticks: the one progress value, as the authority holds it.
fn current(b: &Battle) -> u32 {
    b.unit(SUPPLY).unwrap().deployment.as_ref().unwrap().current
}

fn xy(b: &Battle, id: UnitId) -> [f64; 2] {
    let p = own(b, id).position;
    [p[0], p[1]]
}

fn run(b: &mut Battle, ticks: u32) {
    for _ in 0..ticks {
        b.step();
    }
}

/// Steps until `done`, returning how many ticks it took (panics past `limit`).
fn ticks_until(b: &mut Battle, limit: u32, mut done: impl FnMut(&Battle) -> bool) -> u32 {
    for n in 1..=limit {
        b.step();
        if done(b) {
            return n;
        }
    }
    panic!("not reached within {limit} ticks");
}

/// A supply vehicle deployed from spawn (a stopped supply unit sets up).
fn deployed() -> (Battle, Commander) {
    let mut b = battle();
    run(&mut b, duration());
    assert!(deployment::fully_deployed(b.unit(SUPPLY).unwrap()));
    (b, Commander::new())
}

#[test]
fn the_seam_reverses_from_current_progress_with_one_duration() {
    let n = 10;
    let at = |current| Deployment {
        current,
        duration: n,
        stationary: Posture::Deployed,
    };
    let step = |current, target| {
        let mut d = at(current);
        d.advance(target);
        d
    };
    let mut d = at(0);
    let mut steps = 0;
    while d.current < n {
        d.advance(Posture::Deployed);
        steps += 1;
    }
    assert_eq!(steps, n, "deploying takes the duration");
    let full = step(n, Posture::Deployed);
    assert!(full.fully_deployed() && !full.packed());
    assert_eq!(full.current, n, "progress saturates");
    let first = step(n, Posture::Packed);
    assert_eq!(first.current, n - 1);
    assert!(
        !first.fully_deployed(),
        "readiness ends on the first packing tick"
    );
    assert!(!first.packed());
    let back = step(first.current, Posture::Deployed);
    assert_eq!(back.current, n, "a reversal loses no time");
    let packed = step(1, Posture::Packed);
    assert!(packed.packed() && packed.current == 0);
    assert_eq!(step(0, Posture::Packed).current, 0);
}

#[test]
fn deploying_and_packing_take_the_same_duration() {
    let n = duration();
    let mut b = battle();
    let start = xy(&b, SUPPLY);
    let up = ticks_until(&mut b, 2 * n, |b| {
        own(b, SUPPLY).deployment.unwrap().progress == 1.0
    });
    assert_eq!(
        up, n,
        "a stopped supply unit deploys in the authored duration"
    );
    let mut c = Commander::new();
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    let almost = ticks_until(&mut b, 2 * n, |b| current(b) == 1);
    assert_eq!(xy(&b, SUPPLY), start, "nothing moves while packing");
    b.step();
    assert_eq!(current(&b), 0);
    assert_eq!(almost + 1, n, "packing takes exactly as long as deploying");
}

#[test]
fn a_forty_percent_deployed_unit_needs_forty_percent_of_the_duration_to_pack() {
    let n = duration();
    let partial = n * 2 / 5;
    let mut b = battle();
    run(&mut b, partial);
    assert!((own(&b, SUPPLY).deployment.unwrap().progress - 0.4).abs() < 1e-9);
    let start = xy(&b, SUPPLY);
    let mut c = Commander::new();
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    let mut packing_ticks = 0;
    while current(&b) > 0 || packing_ticks == 0 {
        b.step();
        packing_ticks += 1;
        let u = own(&b, SUPPLY);
        assert_eq!(u.deployment.unwrap().target, Posture::Packed);
        if current(&b) > 0 {
            assert_eq!(u.state, MoveState::Packing);
            assert_eq!(xy(&b, SUPPLY), start, "no translation until packed");
        }
    }
    assert_eq!(
        packing_ticks, partial,
        "packing time is the completed share"
    );
    run(&mut b, 5);
    assert!(xy(&b, SUPPLY)[0] > start[0], "moves once packed");
    assert_eq!(own(&b, SUPPLY).state, MoveState::Moving);
}

#[test]
fn a_half_packed_reversal_deploys_in_half_the_duration() {
    let n = duration();
    let (mut b, mut c) = deployed();
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    run(&mut b, n / 2);
    let u = own(&b, SUPPLY);
    assert!((u.deployment.unwrap().progress - 0.5).abs() < 1e-9);
    assert_eq!(u.deployment.unwrap().target, Posture::Packed);
    c.stop(&mut b, SUPPLY);
    let back = ticks_until(&mut b, n, |b| {
        deployment::fully_deployed(b.unit(SUPPLY).unwrap())
    });
    assert_eq!(back, n / 2, "reversal continues from current progress");
}

#[test]
fn stop_clears_movement_and_returns_the_desired_state_to_deployed() {
    let (mut b, mut c) = deployed();
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    c.move_to(&mut b, SUPPLY, [300.0, 100.0], true);
    run(&mut b, 30);
    let u = own(&b, SUPPLY);
    assert_eq!(u.goal, Some([300.0, 200.0]));
    assert_eq!(u.queue.len(), 1);
    assert_eq!(u.deployment.unwrap().target, Posture::Packed);
    let before = current(&b);
    c.stop(&mut b, SUPPLY);
    b.step();
    let u = own(&b, SUPPLY);
    assert_eq!(u.goal, None, "the move is cleared");
    assert!(
        u.queue.is_empty() && u.route.is_empty(),
        "the queue is cleared"
    );
    assert_eq!(u.state, MoveState::Idle);
    assert_eq!(u.deployment.unwrap().target, Posture::Deployed);
    assert_eq!(
        current(&b),
        before + 1,
        "and setup resumes on the next tick"
    );
}

#[test]
fn stop_after_an_explicit_pack_sets_up_again() {
    let (mut b, mut c) = deployed();
    c.deploy(&mut b, SUPPLY, false);
    run(&mut b, 20);
    assert_eq!(own(&b, SUPPLY).deployment.unwrap().target, Posture::Packed);
    c.stop(&mut b, SUPPLY);
    b.step();
    assert_eq!(
        own(&b, SUPPLY).deployment.unwrap().target,
        Posture::Deployed
    );
}

#[test]
fn no_service_readiness_before_fully_deployed() {
    let n = duration();
    let mut b = battle();
    for k in 1..=n {
        b.step();
        let supply = b.unit(SUPPLY).unwrap();
        assert_eq!(deployment::fully_deployed(supply), k == n, "tick {k}");
        assert!(
            !deployment::fully_deployed(b.unit(TANK).unwrap()),
            "tanks never serve"
        );
    }
    let mut c = Commander::new();
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    b.step();
    assert!(
        !deployment::fully_deployed(b.unit(SUPPLY).unwrap()),
        "readiness ends as packing starts"
    );
}

#[test]
fn a_queued_move_while_deployed_packs_first_then_moves_and_sets_up_on_arrival() {
    let n = duration();
    let (mut b, mut c) = deployed();
    let start = xy(&b, SUPPLY);
    c.move_to(&mut b, SUPPLY, [160.0, 200.0], true);
    run(&mut b, n - 1);
    assert_eq!(own(&b, SUPPLY).state, MoveState::Packing);
    assert_eq!(xy(&b, SUPPLY), start, "still packing");
    b.step();
    assert_eq!(current(&b), 0);
    assert!(
        xy(&b, SUPPLY)[0] > start[0],
        "moves on the tick packing completes"
    );
    ticks_until(&mut b, 1200, |b| own(b, SUPPLY).state == MoveState::Idle);
    let u = own(&b, SUPPLY);
    assert!((u.position[0] - 160.0).abs() < 1.0);
    assert_eq!(
        u.deployment.unwrap().target,
        Posture::Deployed,
        "arrival sets up"
    );
    b.step();
    assert!(current(&b) > 0);
}

#[test]
fn an_explicit_pack_holds_a_stationary_unit_packed_and_deploy_cancels_movement() {
    let n = duration();
    let (mut b, mut c) = deployed();
    c.deploy(&mut b, SUPPLY, false);
    run(&mut b, 2 * n);
    let u = own(&b, SUPPLY);
    assert_eq!(u.deployment.unwrap().progress, 0.0, "packed and held there");
    assert_eq!(u.deployment.unwrap().target, Posture::Packed);
    assert_eq!(u.state, MoveState::Idle);

    c.move_to(&mut b, SUPPLY, [500.0, 200.0], false);
    run(&mut b, 60);
    let moving = xy(&b, SUPPLY);
    assert!(
        moving[0] > 110.0,
        "already packed, so it drives off at once"
    );
    c.deploy(&mut b, SUPPLY, true);
    b.step();
    let u = own(&b, SUPPLY);
    assert_eq!(u.goal, None, "deploying ends the move");
    assert_eq!(u.deployment.unwrap().target, Posture::Deployed);
    run(&mut b, n);
    assert!(deployment::fully_deployed(b.unit(SUPPLY).unwrap()));
    assert!(
        (xy(&b, SUPPLY)[0] - moving[0]).abs() < 1.0,
        "set up where it stopped"
    );
}

#[test]
fn repeated_reversals_neither_reset_nor_create_progress_or_actions() {
    let n = duration();
    let mut b = battle();
    run(&mut b, n * 3 / 5);
    let mut c = Commander::new();
    let mut at = xy(&b, SUPPLY);
    let mut expected = current(&b) as i64;
    let mut last = expected;
    for cycle in 0..24u32 {
        if cycle % 2 == 0 {
            c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
        } else {
            c.stop(&mut b, SUPPLY);
        }
        // Uneven intervals, so reversals land at many different progress values.
        for _ in 0..(17 + cycle * 7 % 23) {
            b.step();
            let now = current(&b) as i64;
            let step = if cycle % 2 == 0 { -1 } else { 1 };
            expected = (expected + step).clamp(0, n as i64);
            assert_eq!(
                now, expected,
                "cycle {cycle}: one tick of progress per tick"
            );
            assert!((now - last).abs() <= 1, "progress never jumps");
            last = now;
            let unit = b.unit(SUPPLY).unwrap();
            assert!(unit.orders.len() <= 1, "no orders pile up");
            assert!(unit.mounts.is_empty(), "no actions appear");
            if now > 0 {
                assert_eq!(xy(&b, SUPPLY), at, "never moves while any setup remains");
            }
            at = xy(&b, SUPPLY);
        }
    }
}

#[test]
fn units_that_do_not_deploy_ignore_deployment_and_move_at_once() {
    let mut b = battle();
    let mut c = Commander::new();
    assert!(own(&b, TANK).deployment.is_none());
    c.deploy(&mut b, TANK, true);
    c.move_to(&mut b, TANK, [300.0, 300.0], false);
    let start = xy(&b, TANK);
    run(&mut b, 30);
    assert!(xy(&b, TANK)[0] > start[0]);
    c.deploy(&mut b, TANK, true);
    b.step();
    assert!(
        own(&b, TANK).goal.is_some(),
        "a deploy order leaves a tank's move alone"
    );
}

#[test]
fn deployment_replays_to_identical_digests_and_enters_the_digest() {
    let n = duration();
    let mut b = battle();
    let mut quiet = battle();
    let mut c = Commander::new();
    run(&mut b, n / 3);
    run(&mut quiet, n / 3);
    c.deploy(&mut b, SUPPLY, false);
    b.step();
    quiet.step();
    // Same positions and orders; only the setup progress differs.
    assert_eq!(xy(&b, SUPPLY), xy(&quiet, SUPPLY));
    assert_ne!(b.digest(), quiet.digest());
    c.move_to(&mut b, SUPPLY, [300.0, 200.0], false);
    run(&mut b, 40);
    c.stop(&mut b, SUPPLY);
    run(&mut b, 40);
    let replay = b.replay();
    let setup = common::scenario(
        &json!({ "size": [600, 400], "height_grid_m": 4, "slope_cutoff_deg": 35 }).to_string(),
        json!([
            { "side": "blue", "kind": "supply", "position": [100, 200] },
            { "side": "blue", "kind": "tank", "position": [100, 300] },
        ]),
        json!([]),
    );
    let mut again = Battle::from_replay(&setup, &replay).unwrap();
    while again.tick() < b.tick() {
        again.step();
    }
    assert_eq!(again.digest(), b.digest());
}
