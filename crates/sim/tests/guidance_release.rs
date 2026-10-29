//! A guided missile let go: its launcher moves, is told to Stop or loses
//! sight, and the missile coasts straight on, then goes to ground; it never
//! takes its target back.

use contract::command::Order;
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

use crate::common::{self, Commander};
use crate::guidance::{ambush, horizontal, missile, move_to, own, until_launch};

#[test]
fn a_screen_breaks_guidance_even_while_the_scout_keeps_the_target_identified() {
    let map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] }).to_string();
    let units = json!([
        { "side": "blue", "kind": "test_at", "position": [100, 300] },
        { "side": "blue", "kind": "test_recon", "position": [450, 320] },
        { "side": "red", "kind": "test_tank", "position": [800, 300], "engagement": "return_fire_only" },
    ]);
    let setup = |events| {
        let mut s = common::scenario_with(&map, units.clone(), events, json!([]));
        let mut r = common::game();
        sim::fixtures::patch_catalog(
            &mut r,
            "units",
            "test_at",
            json!({ "sensors": { "ground_m": 100 } }),
        );
        common::json_ground_reach(&mut r["weapons"]["atgm"], 900.0);
        for w in r["weapons"].as_object_mut().unwrap().values_mut() {
            w["damage"] = json!(0);
        }
        s.rules = serde_json::from_value(r).unwrap();
        s
    };
    let launch = until_launch(&mut Battle::new(&setup(json!([])), 2));
    // Let the missile pass the screen; this isolates guidance loss from
    // the round physically striking the newly inserted wall.
    let screen_tick = launch + 30;
    let events = json!([{ "tick": screen_tick, "add_prop": { "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 60, 5] } }]);
    let mut b = Battle::new(&setup(events), 2);
    while b.tick() < screen_tick {
        b.step();
    }
    let view = b.observe(Side::Blue);
    assert!(!view.identified.is_empty(), "recon still spots the target");
    assert!(
        !missile(&b)
            .expect("the released missile still flies")
            .supported,
        "the operator's blocked LOS releases guidance immediately"
    );
}

#[test]
fn moving_releases_at_once_and_frees_the_crew() {
    let mut b = ambush(3);
    until_launch(&mut b);
    let gunner = |b: &Battle| {
        let unit = b.unit(UnitId(0)).unwrap();
        let id = unit.mounts.iter().find_map(|m| m.operator).unwrap();
        unit.members.iter().find(|s| s.id == id).unwrap().position
    };
    let mut o = Commander::new();
    o.ok(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    // The squad sets off staggered: support holds while the gunner stands,
    // and ends on the tick he steps off.
    let mut before = missile(&b).unwrap();
    for _ in 0..common::tick_hz() {
        let stood = gunner(&b);
        b.step();
        if gunner(&b) != stood {
            break;
        }
        before = missile(&b).expect("still flying");
        assert!(before.supported, "guided while the gunner stands");
    }
    let after = missile(&b).expect("still flying");
    assert!(!after.supported, "his movement releases support");
    // The point is a coast straight ahead at its speed, on the ground: the
    // tank 500 m off no longer draws it.
    let speed = horizontal(after.position, before.position) * common::tick_hz() as f64;
    let coast = speed * coast_s();
    let ahead = horizontal(after.point, before.position);
    assert!(
        (ahead - coast).abs() <= speed / common::tick_hz() as f64 + 1.0,
        "{ahead:.1} m ahead of release, coast {coast:.1} m"
    );
    assert!(
        (after.point[1] - before.position[1]).abs() < 1.0,
        "straight on"
    );
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
    // It dives into the ground at the point, never reaching the tank.
    let full = common::hull("test_tank").hp;
    let mut burst = None;
    for _ in 0..120 {
        b.step();
        burst = burst.or(b.observe(Side::Blue).blasts.first().map(|x| x.point));
    }
    let burst = burst.expect("it burst");
    assert!(
        horizontal(burst, after.point) < 1.0,
        "at its point: {burst:?}"
    );
    assert_eq!(
        own(&b, Side::Red, 1).unwrap().hp,
        full,
        "the tank is untouched"
    );
}

#[test]
fn stop_releases_and_the_point_never_moves_again() {
    let mut b = ambush(4);
    until_launch(&mut b);
    let mut o = Commander::new();
    o.ok(
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
    o.ok(&mut b, Side::Red, move_to(1, [600.0, 250.0]));
    while let Some(m) = missile(&b) {
        assert!(!m.supported, "never reacquires");
        assert_eq!(m.point, frozen.point, "the point stays put");
        b.step();
    }
}

#[test]
fn a_launcher_that_moves_with_its_missile_close_still_hits_a_still_target() {
    let mut b = ambush(5);
    until_launch(&mut b);
    // Release inside the coast distance of the tank's west face (x 598).
    while missile(&b).expect("in flight").position[0] < 540.0 {
        b.step();
    }
    let mut o = Commander::new();
    o.ok(&mut b, Side::Blue, move_to(0, [100.0, 200.0]));
    let full = common::hull("test_tank").hp;
    let damage = common::game()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    b.step();
    assert!(!missile(&b).expect("still flying").supported, "released");
    for _ in 0..60 {
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
    let mut o = Commander::new();
    let turn = common::game()["weapons"]["atgm"]["turn_deg_s"]
        .as_f64()
        .unwrap()
        .to_radians()
        / 30.0;
    let mut path: Vec<[f64; 3]> = Vec::new();
    for t in 0..240 {
        if t == delay {
            o.ok(&mut b, Side::Red, move_to(1, [600.0, 345.0]));
        }
        if come_back && t == delay + 60 {
            // Back out into view: a released missile does not reacquire.
            o.ok(&mut b, Side::Red, move_to(1, [600.0, 290.0]));
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
    let full = common::hull("test_tank").hp;
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

/// What a release does to a missile in flight. The AT team 500 m
/// from a stationary red tank; `screen_after` ticks after launch a wall rises
/// just in front of the team (a stand-in for smoke), behind the missile, so
/// the launcher loses its own sighting while the tank stays put.
struct Screened {
    /// The missile's last supported position and its velocity there.
    released_at: [f64; 3],
    velocity: [f64; 3],
    /// Where the missile burst.
    impact: [f64; 3],
    tank_hp: f64,
}

/// The screened battle, before its first step, and its launch tick.
fn screened_battle(screen_after: u64, seed: u64) -> (Battle, u64) {
    let units = json!([
        { "side": "blue", "kind": "test_at", "position": [100, 300] },
        { "side": "red", "kind": "test_tank", "position": [600, 300], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
    ]);
    let map =
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string();
    // Release timing is this experiment's input, independent of gameplay
    // missile tuning: a roughly 300 m/s missile passes the screen before it rises.
    let setup = |events| {
        let mut setup = common::scenario_with(&map, units.clone(), events, json!([]));
        let mut rules = common::scenario_rules();
        rules["weapons"]["atgm"]["speed_mps"] = json!(300.0);
        rules["weapons"]["atgm"]["top_speed_mps"] = json!(301.0);
        rules["weapons"]["atgm"]["accel_mps2"] = json!(1.0);
        setup.rules = serde_json::from_value(rules).unwrap();
        setup
    };
    let launch = until_launch(&mut Battle::new(&setup(json!([])), seed));
    let screen = json!([{ "tick": launch + screen_after, "add_prop":
        { "kind": "wall", "center": [140, 300], "yaw": 0, "half_extents": [0.5, 60, 5] } }]);
    let b = Battle::new(&setup(screen), seed);
    (b, launch)
}

fn screened(screen_after: u64, seed: u64) -> Screened {
    let (mut b, _) = screened_battle(screen_after, seed);
    let hz = common::tick_hz() as f64;
    let (mut prev, mut last): (Option<[f64; 3]>, Option<[f64; 3]>) = (None, None);
    let mut released = None;
    for _ in 0..600 {
        b.step();
        let obs = b.observe(Side::Blue);
        if let (Some(blast), Some((at, velocity))) = (obs.blasts.first(), released) {
            return Screened {
                released_at: at,
                velocity,
                impact: blast.point,
                tank_hp: own(&b, Side::Red, 1).map_or(0.0, |u| u.hp),
            };
        }
        match obs.guided.first() {
            Some(m) if m.supported => (prev, last) = (last, Some(m.position)),
            Some(_) if released.is_none() => {
                let (p, l) = (prev.expect("flew a tick"), last.unwrap());
                let v = [(l[0] - p[0]) * hz, (l[1] - p[1]) * hz, (l[2] - p[2]) * hz];
                released = Some((l, v));
            }
            _ => {}
        }
    }
    panic!("no release and burst");
}

fn coast_s() -> f64 {
    common::game()["guided"]["release_coast_s"]
        .as_f64()
        .unwrap()
}

#[test]
fn a_far_missile_that_loses_sight_coasts_and_goes_to_ground_short_of_a_still_target() {
    let full = common::hull("test_tank").hp;
    for seed in [21, 22] {
        let s = screened(15, seed);
        let speed = s.velocity[0].hypot(s.velocity[1]);
        let coast = speed * coast_s();
        let flew = horizontal(s.impact, s.released_at);
        // Within one tick of flight: the release is judged on the tick after
        // the sighting lapses.
        assert!(
            (flew - coast).abs() <= speed / common::tick_hz() as f64 + 1.0,
            "seed {seed}: burst {flew:.1} m past release, coast {coast:.1} m"
        );
        assert!(s.impact[2].abs() < 0.05, "on the ground: {:?}", s.impact);
        assert!(s.impact[0] < 500.0, "short of the tank: {:?}", s.impact);
        assert_eq!(s.tank_hp, full, "seed {seed}: the still tank is untouched");
    }
}

#[test]
fn a_close_missile_that_loses_sight_still_hits_a_still_target() {
    let full = common::hull("test_tank").hp;
    let damage = common::game()["weapons"]["atgm"]["damage"]
        .as_f64()
        .unwrap();
    for seed in [21, 22] {
        // Release just before reaching the tank, inside the coast distance.
        let s = screened(46, seed);
        assert!(
            600.0 - s.released_at[0] < s.velocity[0] * coast_s(),
            "seed {seed}: released within a coast of the tank: {:?}",
            s.released_at
        );
        assert!(
            (s.tank_hp - (full - damage)).abs() < 1e-9,
            "seed {seed}: normal damage: {}",
            s.tank_hp
        );
    }
}

#[test]
fn a_screened_release_repeats_from_its_seed() {
    let (mut a, launch) = screened_battle(15, 21);
    let (mut c, _) = screened_battle(15, 21);
    for _ in 0..launch + 120 {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}

#[test]
fn a_prompt_escape_is_beaten_by_a_second_launcher_in_crossfire() {
    let mut rules = sim::fixtures::test_game();
    rules["movement"]["drive"]["acceleration_s"] = json!(1e-6);
    rules["movement"]["drive"]["braking_s"] = json!(1e-6);
    let rules = serde_json::from_value(rules).unwrap();
    let mut b = Battle::new(
        &sim::maps::scenario("ambush", "crossfire", rules).unwrap(),
        10,
    );
    let full = b.unit(UnitId(0)).unwrap().hp;
    let mut saw_crossfire = false;
    for _ in 0..600 {
        b.step();
        let observation = b.observe(Side::Blue);
        let guided = &observation.guided;
        saw_crossfire |= guided.len() == 2
            && guided.iter().any(|m| !m.supported)
            && guided.iter().any(|m| m.supported);
        if guided.is_empty() && b.tick() > 300 {
            break;
        }
    }
    assert!(
        saw_crossfire,
        "one launcher released while the other kept sight"
    );
    assert!(
        b.unit(UnitId(0)).unwrap().hp < full,
        "the prompt escape survived despite the prepared crossfire"
    );
}
