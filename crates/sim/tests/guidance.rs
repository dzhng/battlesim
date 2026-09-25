//! Supported AT guidance (slice 10): own-sight launch and support, immediate
//! release on move/Stop/lost sight, no reacquisition, a fixed last point.
use contract::command::{CommandEnvelope, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, GuidedMissile, OwnUnit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::steer;
use sim::math::v3;

mod common;

fn battle(props: Value, units: Value, seed: u64) -> Battle {
    let map =
        json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
            .to_string();
    Battle::new(
        &common::scenario_with(&map, units, json!([]), json!([])),
        seed,
    )
}

fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

struct Orders(u64, u64);
impl Orders {
    fn send(&mut self, b: &mut Battle, side: Side, order: Order) {
        let seq = match side {
            Side::Blue => {
                self.0 += 1;
                self.0
            }
            Side::Red => {
                self.1 += 1;
                self.1
            }
        };
        let ack = b.accept(CommandEnvelope {
            side,
            seq,
            order,
            queued: false,
        });
        assert_eq!(ack.error, None, "{ack:?}");
    }
}

fn missile(b: &Battle) -> Option<GuidedMissile> {
    b.observe(Side::Blue).guided.first().cloned()
}

/// Step until blue has a guided missile in flight; returns the tick.
fn until_launch(b: &mut Battle) -> u64 {
    for _ in 0..600 {
        b.step();
        if missile(b).is_some() {
            return b.tick();
        }
    }
    panic!("no launch");
}

/// Blue's AT team 500 m from a red tank, a wall just north of the line of fire.
fn ambush(seed: u64) -> Battle {
    battle(
        json!([{ "kind": "wall", "center": [560, 336], "yaw": 0, "half_extents": [0.5, 30, 4] }]),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300], "engagement": "fire_at_will" },
            { "side": "red", "kind": "tank", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
        ]),
        seed,
    )
}

fn move_to(unit: u32, goal: [f64; 2]) -> Order {
    Order::Move {
        units: vec![UnitId(unit)],
        gesture: 1,
        goal,
        route: RoutePolicy::Shortest,
    }
}

/// The village rules with a quick ATGM (0.5 s aim, 1 s reload) so a next
/// round is ready while one is still in flight, and any `tweak` applied.
fn quick(props: Value, units: Value, seed: u64, tweak: impl Fn(&mut Value)) -> Battle {
    let mut rules = common::village();
    rules["weapons"]["atgm"]["aim_s"] = json!(0.5);
    rules["weapons"]["atgm"]["reload_s"] = json!(1.0);
    tweak(&mut rules);
    let map =
        json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props });
    let setup = serde_json::from_value(
        json!({ "map": map, "rules": rules, "units": units, "events": [], "scripts": [] }),
    )
    .unwrap();
    Battle::new(&setup, seed)
}

#[test]
fn a_launcher_fires_and_guides_on_its_own_sight() {
    let mut b = ambush(1);
    until_launch(&mut b);
    let at = own(&b, Side::Blue, 0).unwrap();
    assert!(at.mounts.iter().any(|m| m.guiding), "the launcher guides");
    assert!(missile(&b).unwrap().supported);
}

#[test]
fn a_ready_next_round_waits_while_one_is_guided_and_fires_once_released() {
    // 560 m: about 3 s of flight, while the quick reload takes 1 s.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |_| {},
    );
    until_launch(&mut b);
    let mut waited = false;
    for _ in 0..45 {
        b.step();
        let atgm = own(&b, Side::Blue, 0).unwrap().mounts[1].clone();
        assert!(
            b.observe(Side::Blue).guided.len() <= 1,
            "one missile at a time"
        );
        if atgm.loaded.is_some() && atgm.aim >= 1.0 {
            assert_eq!(atgm.reason, ActionReason::Guiding);
            waited = true;
        }
    }
    assert!(waited, "a ready round waited on the guided one");
    // Stop releases; the crew launches again while the first still flies.
    let mut o = Orders(0, 0);
    o.send(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    let mut both = false;
    for _ in 0..40 {
        b.step();
        let g = &b.observe(Side::Blue).guided;
        both |= g.len() == 2 && g.iter().filter(|m| m.supported).count() == 1;
    }
    assert!(both, "a second missile while the released first still flew");
}

#[test]
fn the_launcher_dying_releases_its_missile() {
    // A single fragile launcher; a red scout squad nearby shoots it.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "recon", "position": [100, 330] },
        ]),
        2,
        |r| {
            r["health"]["at_squad_size"] = json!(1);
            r["health"]["soldier"] = json!(1);
        },
    );
    until_launch(&mut b);
    let mut released = None;
    for _ in 0..120 {
        b.step();
        if own(&b, Side::Blue, 0).is_none() {
            released = missile(&b);
            break;
        }
    }
    let m = released.expect("the launcher died while its missile flew");
    b.step();
    let m2 = missile(&b).unwrap_or(m.clone());
    assert!(!m2.supported, "death releases support");
    assert!(m2.point[2].abs() < 1e-6, "to a point on the ground");
}

#[test]
fn team_identification_is_not_enough_to_launch() {
    // The AT team is walled off from the tank; a scout sees it for the team.
    let mut b = battle(
        json!([{ "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 40, 5] }]),
        json!([
            { "side": "blue", "kind": "at", "position": [100, 300] },
            { "side": "blue", "kind": "recon", "position": [160, 380] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        2,
    );
    for _ in 0..300 {
        b.step();
        assert!(missile(&b).is_none(), "never launched on shared sight");
    }
    assert!(
        !b.observe(Side::Blue).identified.is_empty(),
        "the team does see it"
    );
    let atgm = &own(&b, Side::Blue, 0).unwrap().mounts[1];
    assert_eq!(atgm.reason, ActionReason::NoOwnSight);
}

#[test]
fn moving_releases_at_once_and_frees_the_crew() {
    let mut b = ambush(3);
    until_launch(&mut b);
    let before = missile(&b).unwrap();
    let mut o = Orders(0, 0);
    o.send(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    b.step();
    let after = missile(&b).expect("still flying");
    assert!(!after.supported, "movement releases support");
    // The last point, fixed on the ground beneath.
    assert!((after.point[0] - before.point[0]).abs() < 1.0);
    assert!(
        after.point[2].abs() < 1e-6,
        "on the ground: {:?}",
        after.point
    );
    let at0 = own(&b, Side::Blue, 0).unwrap().position;
    b.step();
    assert!(missile(&b).is_some(), "the missile flies on");
    assert_ne!(
        own(&b, Side::Blue, 0).unwrap().position,
        at0,
        "the crew is free to move"
    );
}

#[test]
fn stop_releases_and_the_point_never_moves_again() {
    let mut b = ambush(4);
    until_launch(&mut b);
    let mut o = Orders(0, 0);
    o.send(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    b.step();
    let frozen = missile(&b).unwrap();
    assert!(!frozen.supported);
    // Red's tank drives off; the released missile never follows or reacquires.
    o.send(&mut b, Side::Red, move_to(1, [600.0, 250.0]));
    while let Some(m) = missile(&b) {
        assert!(!m.supported, "never reacquires");
        assert_eq!(m.point, frozen.point, "the point stays put");
        b.step();
    }
}

#[test]
fn a_stationary_target_at_the_last_point_still_takes_the_hit() {
    let mut b = ambush(5);
    until_launch(&mut b);
    let mut o = Orders(0, 0);
    o.send(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    let full = common::village()["health"]["tank"].as_f64().unwrap();
    let damage = common::village()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    for _ in 0..240 {
        b.step();
    }
    let tank = own(&b, Side::Red, 1).unwrap();
    assert!(
        (tank.hp - (full - damage)).abs() < 1e-9,
        "normal damage: {}",
        tank.hp
    );
}

/// The tank drives behind the wall `delay` ticks after the launch; returns its
/// hp and whether the missile was released. Along the way: once released it is
/// never supported again (even when the tank comes back into view), and it
/// never turns faster than its limit.
fn retreat(delay: u64, seed: u64, come_back: bool) -> (f64, bool) {
    let mut b = ambush(seed);
    until_launch(&mut b);
    let mut released = false;
    let mut o = Orders(0, 0);
    let turn = common::village()["weapons"]["atgm"]["turn_deg_s"]
        .as_f64()
        .unwrap()
        .to_radians()
        / 30.0;
    let mut path: Vec<[f64; 3]> = Vec::new();
    for t in 0..240 {
        if t == delay {
            o.send(&mut b, Side::Red, move_to(1, [600.0, 345.0]));
        }
        if come_back && t == delay + 60 {
            // Back out into view: a released missile does not reacquire.
            o.send(&mut b, Side::Red, move_to(1, [600.0, 290.0]));
        }
        b.step();
        if let Some(m) = missile(&b) {
            assert!(!(released && m.supported), "never reacquires");
            released |= !m.supported;
            path.push(m.position);
        }
    }
    for w in path.windows(3) {
        let (a, c) = (sub(w[1], w[0]), sub(w[2], w[1]));
        let cos = dot(a, c) / (dot(a, a).sqrt() * dot(c, c).sqrt());
        assert!(
            cos.clamp(-1.0, 1.0).acos() <= turn + 1e-6,
            "turned faster than its limit"
        );
    }
    (own(&b, Side::Red, 1).map_or(0.0, |u| u.hp), released)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[test]
fn a_prompt_retreat_out_of_sight_escapes_and_a_late_one_does_not() {
    let full = common::village()["health"]["tank"].as_f64().unwrap();
    for seed in [6, 7, 8] {
        let (prompt, lost) = retreat(0, seed, false);
        assert!(lost, "the launcher lost sight");
        assert_eq!(prompt, full, "seed {seed}: prompt retreat escapes");
        let (late, _) = retreat(120, seed, false);
        assert!(late < full, "seed {seed}: a late retreat is hit");
    }
}

#[test]
fn a_released_missile_does_not_reacquire_a_target_back_in_view() {
    // `retreat` asserts it on every tick while the tank drives back out.
    for seed in [6, 7] {
        let (_, released) = retreat(0, seed, true);
        assert!(released);
    }
}

#[test]
fn steering_is_bounded_by_the_turn_rate() {
    let v = v3(180.0, 0.0, 0.0);
    let max = 60f64.to_radians() / 30.0;
    let turned = steer(v, v3(0.0, 1.0, 0.0), max);
    let angle = (turned.dot(v) / (turned.length() * v.length())).acos();
    assert!((angle - max).abs() < 1e-9 && (turned.length() - 180.0).abs() < 1e-9);
    // Within the limit it points straight at the goal.
    let small = steer(v, v3(1.0, 0.001, 0.0), max);
    assert!((small.y / small.x - 0.001).abs() < 1e-9);
}

#[test]
fn a_guided_missile_ends_at_its_lifetime() {
    // Launch solving refuses a target beyond a missile's lifetime, so a guided
    // round ends (strikes or expires) within it; expiry itself is the shared
    // flight rule pinned in slice 07.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "at", "position": [40, 300] },
            { "side": "red", "kind": "tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        3,
        |r| {
            r["weapons"]["atgm"]["lifetime_s"] = json!(4.0);
            r["weapons"]["atgm"]["range_m"] = json!(1400);
            r["weapons"]["atgm"]["speed_mps"] = json!(150.0);
        },
    );
    let launched = until_launch(&mut b);
    let mut ended = None;
    for _ in 0..200 {
        b.step();
        if missile(&b).is_none() {
            ended = Some(b.tick() - launched);
            break;
        }
    }
    let ended = ended.expect("the missile ended");
    assert!(ended <= 4 * 30 + 1, "within its lifetime: {ended} ticks");
}

#[test]
fn guided_flight_replays_identically() {
    let (mut a, mut c) = (ambush(11), ambush(11));
    for _ in 0..300 {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}
