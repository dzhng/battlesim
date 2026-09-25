//! Launch solving and flight against closed-form ballistics: gravity endpoints,
//! analytic elevations, lead, evasion, reach, arc choice, and the flight
//! bounds. Village weapon rows supply speeds; the expectations are physics.
mod common;

use common::*;
use contract::ballistics::{FlightRules, Trajectory, WeaponBallistics};
use sim::flight::{
    predicted_path, prepare_launch, scatter_aim, solve_launch, Aim, ArcKind, Expiry, FlightConfig,
    FlightConfigError, FlightEvent, Launch, NoSolution, Projectiles, Struck,
};
use sim::math::{v3, V3};
use sim::rng::Rng;

const G: f64 = 9.81;

fn launch(origin: V3, velocity: V3) -> Launch {
    Launch {
        origin,
        velocity,
        lifetime_s: profile("rifle").lifetime_s,
        suppression_radius_m: 0.0,
        shooter: None,
    }
}

/// Elevation angles hitting a point `x` away and `y` higher at speed `v`.
fn analytic_elevations(v: f64, x: f64, y: f64) -> (f64, f64) {
    let root = (v.powi(4) - G * (G * x * x + 2.0 * y * v * v)).sqrt();
    (
        ((v * v - root) / (G * x)).atan(),
        ((v * v + root) / (G * x)).atan(),
    )
}

fn elevation(v: V3) -> f64 {
    v.z.atan2(v.xy().length())
}

#[test]
fn a_round_lands_at_the_analytic_gravity_endpoint() {
    assert_eq!(physics("gravity_mps2"), G);
    let world = flat([2000.0, 400.0], "");
    let mut store = Projectiles::new(config());
    let (o, v) = (v3(100.0, 200.0, 1.5), v3(60.0, 0.0, 60.0));
    store.launch(launch(o, v));
    let events = fly(&mut store, &world, 2000, |_| Vec::new());
    let [(tick, hit)] = impacts(&events)[..] else {
        panic!("one impact expected: {events:?}")
    };
    // z(t) = z0 + vz·t − g·t²/2 = 0.
    let t = (v.z + (v.z * v.z + 2.0 * G * o.z).sqrt()) / G;
    let expect = v3(o.x + v.x * t, o.y, 0.0);
    assert_eq!(hit.struck, Struck::Terrain);
    let dt = config().tick_s();
    let error = (hit.point - expect).length();
    assert!(
        error <= flight_rules().curve_chord_error_m,
        "endpoint off by {error} m"
    );
    let when = event_time(tick, &FlightEvent::Impact(hit), dt);
    assert!(
        (when - t).abs() < 1e-3,
        "landed at {when} s, analytic {t} s"
    );
    // The impact velocity is the exact ballistic velocity at that time.
    assert!((hit.velocity - (v + v3(0.0, 0.0, -G * when))).length() < 1e-9);
}

#[test]
fn a_stationary_ground_target_gets_the_analytic_low_elevation() {
    let world = flat([1000.0, 400.0], "");
    let grenade = profile("grenade");
    let o = v3(100.0, 200.0, physics("infantry_muzzle_m"));
    let target = v3(500.0, 200.0, 0.0);
    let aim = Aim {
        origin: o,
        target,
        target_velocity: V3::default(),
    };
    let s = solve_launch(&world, &config(), &grenade, &aim).unwrap();
    let (low, _) = analytic_elevations(grenade.speed_mps, 400.0, -o.z);
    assert_eq!(s.arc, ArcKind::Low);
    assert!((elevation(s.velocity) - low).abs() < 1e-9);
    assert!((s.velocity.length() - grenade.speed_mps).abs() < 1e-9);
    // The flown round comes down on the aim point.
    let mut store = Projectiles::new(config());
    store.launch(launch(o, s.velocity));
    let [(_, hit)] = impacts(&fly(&mut store, &world, 600, |_| Vec::new()))[..] else {
        panic!("one impact expected")
    };
    assert!((hit.point - target).length() < 0.02, "{:?}", hit.point);
}

#[test]
fn unequal_launch_and_target_heights_follow_the_analytic_arc_and_hit() {
    let world = flat([1000.0, 400.0], "");
    let hmg = profile("grenade");
    for (origin, target) in [
        // Up to a body 40 m higher, and down from 40 m to one on the ground.
        (v3(100.0, 200.0, 1.4), v3(350.0, 200.0, 40.0)),
        (v3(100.0, 200.0, 40.0), v3(350.0, 200.0, 0.85)),
    ] {
        let aim = Aim {
            origin,
            target,
            target_velocity: V3::default(),
        };
        let s = solve_launch(&world, &config(), &hmg, &aim).unwrap();
        let (low, _) = analytic_elevations(hmg.speed_mps, 250.0, target.z - origin.z);
        assert!((elevation(s.velocity) - low).abs() < 1e-9);
        let body = Mover::standing(7, 1, soldier_shape(), target - v3(0.0, 0.0, 0.85));
        let mut store = Projectiles::new(config());
        store.launch(launch(origin, s.velocity));
        let dt = config().tick_s();
        let events = fly(&mut store, &world, 600, |k| vec![body.body(k, dt)]);
        let [(_, hit)] = impacts(&events)[..] else {
            panic!("one impact expected")
        };
        assert_eq!(hit.struck, Struck::Body(sim::flight::BodyId(7)));
    }
}

/// A soldier walking across the line of fire at 300 m.
fn crossing_soldier(speed: f64) -> Mover {
    Mover::standing(1, 9, soldier_shape(), v3(400.0, 190.0, 0.0)).moving(v3(0.0, speed, 0.0))
}

#[test]
fn leading_steady_observed_motion_hits_and_aiming_at_the_present_misses() {
    let world = flat([1000.0, 400.0], "");
    let rifle = profile("rifle");
    let dt = config().tick_s();
    let target = crossing_soldier(4.0);
    let origin = v3(100.0, 190.0, physics("infantry_muzzle_m"));
    let centre = target.base + v3(0.0, 0.0, physics("soldier_height_m") / 2.0);
    for (observed, should_hit) in [(target.velocity, true), (V3::default(), false)] {
        let aim = Aim {
            origin,
            target: centre,
            target_velocity: observed,
        };
        let s = solve_launch(&world, &config(), &rifle, &aim).unwrap();
        let mut store = Projectiles::new(config());
        store.launch(launch(origin, s.velocity));
        let hits = impacts(&fly(&mut store, &world, 100, |k| vec![target.body(k, dt)]));
        assert_eq!(
            hits[0].1.struck == Struck::Body(sim::flight::BodyId(1)),
            should_hit,
            "observed velocity {observed:?}: {hits:?}"
        );
    }
}

#[test]
fn an_unguided_round_is_dodged_by_changing_motion_after_launch() {
    let world = flat([1000.0, 400.0], "");
    let grenade = profile("grenade");
    let dt = config().tick_s();
    let walker = crossing_soldier(3.0);
    let origin = v3(250.0, 190.0, physics("infantry_muzzle_m"));
    let aim = Aim {
        origin,
        target: walker.base + v3(0.0, 0.0, 0.85),
        target_velocity: walker.velocity,
    };
    let s = solve_launch(&world, &config(), &grenade, &aim).unwrap();
    // Reverses ten ticks after launch; the round keeps its launch velocity.
    let reverse_at = 10;
    let dodger = |k: u64| {
        let turn = walker.pose(reverse_at as f64 * dt).base;
        let at = |tick: u64| {
            let t = tick as f64 * dt;
            let base = if tick <= reverse_at {
                walker.pose(t).base
            } else {
                turn - walker.velocity * (t - reverse_at as f64 * dt)
            };
            sim::flight::Pose { base, yaw: 0.0 }
        };
        sim::flight::Body {
            id: sim::flight::BodyId(1),
            unit: contract::ids::UnitId(9),
            shape: walker.shape,
            from: at(k - 1),
            to: at(k),
        }
    };
    for (keeps_walking, should_hit) in [(true, true), (false, false)] {
        let mut store = Projectiles::new(config());
        store.launch(launch(origin, s.velocity));
        let events = fly(&mut store, &world, 300, |k| {
            vec![if keeps_walking {
                walker.body(k, dt)
            } else {
                dodger(k)
            }]
        });
        let [(_, hit)] = impacts(&events)[..] else {
            panic!("one impact expected")
        };
        assert_eq!(
            hit.struck == Struck::Body(sim::flight::BodyId(1)),
            should_hit
        );
        if !should_hit {
            assert_eq!(hit.struck, Struck::Terrain);
            // It comes down on its launch line, just past where the walker
            // would have been: no correction after launch.
            let line = (s.intercept - origin).xy().normalized();
            let off = hit.point.xy() - origin.xy();
            assert!(line.cross(off).abs() < 1e-6, "left its launch line");
            let past = off.dot(line) - (s.intercept - origin).xy().length();
            assert!(
                (0.0..10.0).contains(&past),
                "landed {past} m past the intercept"
            );
        }
    }
}

#[test]
fn an_unreachable_target_has_no_firing_solution() {
    let world = flat([2000.0, 400.0], "");
    let grenade = profile("grenade");
    let o = v3(100.0, 200.0, 1.4);
    // Flat-ground maximum range is v²/g ≈ 652 m.
    let max_range = grenade.speed_mps.powi(2) / G;
    let beyond = Aim {
        origin: o,
        target: v3(100.0 + max_range + 20.0, 200.0, 0.0),
        target_velocity: V3::default(),
    };
    assert_eq!(
        solve_launch(&world, &config(), &grenade, &beyond),
        Err(NoSolution::OutOfReach)
    );
    // Reachable in space but not within the round's lifetime.
    let short_lived = config()
        .profile(&WeaponBallistics {
            lifetime_s: Some(3.0),
            ..weapon("grenade")
        })
        .unwrap();
    let far = Aim {
        target: v3(500.0, 200.0, 0.0),
        ..beyond
    };
    assert!(solve_launch(&world, &config(), &grenade, &far).is_ok());
    assert_eq!(
        solve_launch(&world, &config(), &short_lived, &far),
        Err(NoSolution::OutOfReach)
    );
    // A target outrunning the round.
    let fleeing = Aim {
        target: v3(300.0, 200.0, 0.0),
        target_velocity: v3(90.0, 0.0, 0.0),
        ..beyond
    };
    assert_eq!(
        solve_launch(&world, &config(), &grenade, &fleeing),
        Err(NoSolution::OutOfReach)
    );
}

const RIDGE: &str = r#","relief":[{"kind":"ridge","center":[300,200],"peak_m":25,"radius_m":60}]"#;

fn indirect(name: &str) -> sim::flight::LaunchProfile {
    config()
        .profile(&WeaponBallistics {
            trajectory: Trajectory::Indirect,
            ..weapon(name)
        })
        .unwrap()
}

#[test]
fn the_high_arc_is_used_only_by_indirect_fire_and_never_by_ignoring_a_ridge() {
    let world = flat([800.0, 400.0], RIDGE);
    let o = v3(200.0, 200.0, 1.4);
    let behind = Aim {
        origin: o,
        target: v3(400.0, 200.0, 0.0),
        target_velocity: V3::default(),
    };
    // Direct fire: the low arc meets the ridge, so there is no solution.
    let Err(NoSolution::Blocked { arc, point }) =
        solve_launch(&world, &config(), &profile("grenade"), &behind)
    else {
        panic!("direct fire over the ridge must be blocked")
    };
    assert_eq!(arc.arc, ArcKind::Low);
    assert!(
        (240.0..360.0).contains(&point.x) && point.z > 0.0,
        "{point:?}"
    );
    assert!((world.height_at(point.x, point.y).unwrap() - point.z).abs() < 1e-6);
    // Indirect fire lobs over and lands on the aim point.
    let s = solve_launch(&world, &config(), &indirect("grenade"), &behind).unwrap();
    assert_eq!(s.arc, ArcKind::High);
    let (_, high) = analytic_elevations(80.0, 200.0, -1.4);
    assert!((elevation(s.velocity) - high).abs() < 1e-9);
    let mut store = Projectiles::new(config());
    store.launch(launch(o, s.velocity));
    let [(_, hit)] = impacts(&fly(&mut store, &world, 900, |_| Vec::new()))[..] else {
        panic!("one impact expected")
    };
    assert!((hit.point - behind.target).length() < 0.02);

    // In the open both are clear: direct keeps the low arc, indirect the high.
    let open = flat([800.0, 400.0], "");
    let low = solve_launch(&open, &config(), &profile("grenade"), &behind).unwrap();
    let high = solve_launch(&open, &config(), &indirect("grenade"), &behind).unwrap();
    assert_eq!((low.arc, high.arc), (ArcKind::Low, ArcKind::High));
    assert!(high.time_of_flight_s > low.time_of_flight_s);
}

#[test]
fn chords_per_tick_follow_the_chord_error_and_the_declared_bound_is_enforced() {
    // Village: a 30 Hz chord sags g·dt²/8 ≈ 1.4 mm, inside 2 cm, so one chord.
    assert_eq!(config().subsegments_per_tick(), 1);
    let with_error = |e: f64| {
        FlightConfig::new(
            &FlightRules {
                curve_chord_error_m: e,
                ..flight_rules()
            },
            tick_hz(),
        )
    };
    // n = ⌈√(g·dt² / 8e)⌉.
    let tight = with_error(1e-5).unwrap();
    assert_eq!(tight.subsegments_per_tick(), 12);
    // Every flown chord stays within the chord error of the true parabola.
    let (o, v) = (v3(0.0, 0.0, 0.0), v3(300.0, 0.0, 200.0));
    let path = predicted_path(&tight, o, v, 1.0);
    let h = tight.tick_s() / 12.0;
    let worst = path
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let t = (i as f64 + 0.5) * h;
            let curve = o + v * t + v3(0.0, 0.0, -0.5 * G * t * t);
            ((w[0] + w[1]) * 0.5 - curve).length()
        })
        .fold(0.0, f64::max);
    assert!(worst <= 1e-5 && worst > 0.0, "worst chord sag {worst}");
    assert_eq!(
        with_error(1e-6).err(),
        Some(FlightConfigError::TooManySubsegments {
            required: 37,
            max: 32
        })
    );
}

#[test]
fn every_round_has_a_bounded_lifetime() {
    let too_long = WeaponBallistics {
        lifetime_s: Some(45.0),
        ..weapon("rifle")
    };
    assert_eq!(
        config().profile(&too_long),
        Err(FlightConfigError::LifetimeExceedsBound {
            lifetime_s: 45.0,
            max_s: 30.0
        })
    );
    // Straight up, still climbing at 30 s: it expires exactly at its lifetime.
    let world = flat([200.0, 200.0], "");
    let mut store = Projectiles::new(config());
    store.launch(launch(v3(100.0, 100.0, 2.0), v3(0.0, 0.0, 850.0)));
    let events = fly(&mut store, &world, 2000, |_| Vec::new());
    let [(tick, FlightEvent::Expired(e))] = events[..] else {
        panic!("{events:?}")
    };
    assert_eq!((tick, e.cause), (900, Expiry::Lifetime));
    assert!(store.active().is_empty());
    // A flat shot out of the closed map ends there.
    store.launch(launch(v3(100.0, 100.0, 50.0), v3(850.0, 0.0, 0.0)));
    let events = fly(&mut store, &world, 100, |_| Vec::new());
    assert!(matches!(events[..], [(_, FlightEvent::Expired(e))] if e.cause == Expiry::LeftMap));
}

#[test]
fn spread_is_a_truncated_gaussian_per_axis_across_the_line_of_fire() {
    let (o, aim) = (v3(0.0, 0.0, 1.4), v3(300.0, 400.0, 1.4));
    let range = (aim - o).length();
    let forward = (aim - o).normalized();
    let sigma_mrad = 8.0;
    let mut rng = Rng::new(42);
    let n = 200_000;
    let (mut sum, mut sum_sq, mut worst) = (0.0, 0.0, 0.0f64);
    for _ in 0..n {
        let p = scatter_aim(o, aim, sigma_mrad, &mut rng);
        let offset = p - aim;
        // Displacement lies in the aim plane.
        assert!(offset.dot(forward).abs() < 1e-9);
        // Horizontal axis: signed angle across the line of fire.
        let across = offset.dot(forward.cross(v3(0.0, 0.0, 1.0)).normalized());
        let angle = (across / range).atan() / (sigma_mrad * 1e-3);
        sum += angle;
        sum_sq += angle * angle;
        worst = worst.max(angle.abs());
    }
    let mean = sum / n as f64;
    let sd = (sum_sq / n as f64 - mean * mean).sqrt();
    // A unit normal truncated at ±3 has sd √(1 − 6φ(3)/(2Φ(3)−1)) ≈ 0.98658.
    assert!(mean.abs() < 0.01, "mean {mean}");
    assert!((sd - 0.98658).abs() < 0.01, "sd {sd}");
    assert!(worst <= 3.0 + 1e-9 && worst > 2.5, "worst {worst}");
    // Same seed, same draws.
    let draw = |seed| scatter_aim(o, aim, sigma_mrad, &mut Rng::new(seed));
    assert_eq!(draw(7), draw(7));
    assert_ne!(draw(7), draw(8));
}

#[test]
fn a_prepared_launch_is_the_scattered_solution_and_refuses_a_blocked_aim() {
    let world = flat([800.0, 400.0], RIDGE);
    let grenade = profile("grenade");
    let aim = Aim {
        origin: v3(200.0, 100.0, 1.4),
        target: v3(400.0, 100.0, 0.0),
        target_velocity: V3::default(),
    };
    let mut rng = Rng::new(3);
    let (launch, fired) =
        prepare_launch(&world, &config(), &grenade, &aim, 15.0, &mut rng, None).unwrap();
    assert_eq!(launch.origin, aim.origin);
    assert!((launch.velocity.length() - grenade.speed_mps).abs() < 1e-9);
    assert_eq!(fired.arc, ArcKind::Low);
    let mut replay = Rng::new(3);
    let scattered = scatter_aim(aim.origin, aim.target, 15.0, &mut replay);
    assert!((fired.intercept - scattered).length() < 1e-9);
    assert_eq!(rng, replay);
    // Behind the ridge a direct weapon does not fire at all.
    let blocked = Aim {
        origin: v3(200.0, 200.0, 1.4),
        ..aim
    };
    let blocked = Aim {
        target: v3(400.0, 200.0, 0.0),
        ..blocked
    };
    assert!(matches!(
        prepare_launch(&world, &config(), &grenade, &blocked, 15.0, &mut rng, None),
        Err(NoSolution::Blocked { .. })
    ));
}
