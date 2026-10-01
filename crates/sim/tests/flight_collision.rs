//! Swept collision against analytic crossings: thin fast targets, bodies that
//! cross the path only between ticks, a turning hull, terrain in front of a
//! body, exact ties, near misses, and allegiance-blind first hits.

use crate::common::*;
use contract::ids::UnitId;
use sim::digest::Digest;
use sim::flight::{
    advance_projectiles, Body, BodyId, FlightEvent, Launch, Pose, Projectiles, Shape, Shooter,
    Struck,
};
use sim::math::{v3, V3};

fn round(origin: V3, velocity: V3, suppression_radius_m: f64) -> Launch {
    Launch {
        origin,
        velocity,
        gravity_scale: 1.0,
        lifetime_s: profile("rifle").lifetime_s,
        suppression_radius_m,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    }
}

fn thin_wall(id: u32, centre: V3) -> Mover {
    Mover::standing(
        id,
        id,
        Shape::Box {
            half: v3(0.15, 1.0, 1.0),
        },
        centre,
    )
}

#[test]
fn a_900_mps_round_hits_a_thin_moving_target_between_ticks() {
    let world = flat([1000.0, 400.0], "");
    // Faster than any gun round the game flies: the sweep's hardest case.
    let speed = 900.0;
    let dt = config().tick_s();
    // A 0.3 m thick target 12.3 m into the round's 30 m eighth-tick chord,
    // sliding sideways so it is centred on the line at the crossing.
    let face = 100.0 + 7.0 * speed * dt + 12.3;
    let crossing = (face - 100.0) / speed;
    let slide = 6.0;
    let target =
        thin_wall(4, v3(face + 0.15, 200.0 - slide * crossing, 0.0)).moving(v3(0.0, slide, 0.0));
    // A soldier-sized capsule 0.3 m wide, walking toward the round.
    let walker = Mover::standing(
        5,
        5,
        Shape::Capsule {
            radius: 0.15,
            height: 1.7,
        },
        v3(face + 0.15, 250.0, 0.0),
    )
    .moving(v3(-3.0, 0.0, 0.0));
    let origin_z = 1.0;
    for (body, at_x) in [
        (target, face),
        // Relative closing speed 903 m/s against the walker's front at x0 − 0.15.
        (walker, 100.0 + (face - 100.0) * speed / (speed + 3.0)),
    ] {
        let y = body.base.y + body.velocity.y * crossing;
        let mut store = Projectiles::new(config());
        store.launch(round(v3(100.0, y, origin_z), v3(speed, 0.0, 0.0), 0.0));
        let events = fly(&mut store, &world, 20, |k| vec![body.body(k, dt)]);
        let [(tick, hit)] = impacts(&events)[..] else {
            panic!("{events:?}")
        };
        assert_eq!(hit.struck, Struck::Body(BodyId(body.id)));
        assert_eq!(tick, 8, "struck mid-chord in the eighth tick");
        assert!(
            (hit.point.x - at_x).abs() < 1e-6,
            "{} vs {at_x}",
            hit.point.x
        );
        assert!(hit.normal.x < -0.99, "entered through the facing side");
    }
}

#[test]
fn a_hull_crossing_the_path_only_between_ticks_is_hit_at_the_analytic_time() {
    let world = flat([400.0, 400.0], "");
    let dt = config().tick_s();
    let tank = tank_shape();
    let Shape::Box { half } = tank else {
        unreachable!()
    };
    // The round covers x 0 → 10 m in the tick; the hull spans x 1.5..8.5 and
    // its leading side, 0.3 m short of the line at the tick's start, closes at
    // 15 m/s: it reaches the line at 0.6 of the tick, when the round is at x 6.
    let speed = 10.0 / dt;
    let hull = Mover::standing(2, 2, tank, v3(5.0, 100.0 - half.y - 0.3, 0.0)).moving(v3(
        0.0,
        0.3 / (0.6 * dt),
        0.0,
    ));
    let mut store = Projectiles::new(config());
    store.launch(round(v3(0.0, 100.0, 1.2), v3(speed, 0.0, 0.0), 0.0));
    let events = fly(&mut store, &world, 5, |k| vec![hull.body(k, dt)]);
    let [(1, hit)] = impacts(&events)[..] else {
        panic!("{events:?}")
    };
    assert_eq!(hit.struck, Struck::Body(BodyId(2)));
    assert!((hit.time - 0.6).abs() < 1e-9, "time {}", hit.time);
    assert!((hit.point.x - 6.0).abs() < 1e-6);
    assert!(hit.normal.y > 0.99, "struck on the hull's leading side");
    // Neither tick endpoint of the round is ever inside the hull.
    for s in [0.0, 1.0] {
        let p = v3(10.0 * s, 100.0, 1.2);
        let b = hull.pose(s * dt).base;
        assert!((p.x - b.x).abs() > half.x || (p.y - b.y).abs() > half.y);
    }
}

#[test]
fn a_turning_hull_is_hit_where_only_its_mid_tick_heading_reaches() {
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let half = v3(4.0, 0.25, 1.0);
    let centre = v3(50.0, 100.0, 0.0);
    // Swings from −60° to +60° in one tick; the round crosses its axis line
    // 3 m out at mid-tick. At either end heading, x = 53 is 6 m along the
    // axis: beyond its 4 m half length.
    let swing = Body {
        id: BodyId(1),
        unit: UnitId(1),
        shape: Shape::Box { half },
        from: Pose {
            base: centre,
            yaw: -60f64.to_radians(),
        },
        to: Pose {
            base: centre,
            yaw: 60f64.to_radians(),
        },
    };
    let fire = |bodies: &[Body]| {
        let mut store = Projectiles::new(config());
        store.launch(round(v3(53.0, 99.0, 1.0), v3(0.0, 2.0 / dt, 0.0), 0.0));
        let mut events = Vec::new();
        advance_projectiles(&mut store, &world, bodies, &mut events, &mut stop);
        events
    };
    let events = fire(&[swing]);
    let Some(FlightEvent::Impact(hit)) = events.last() else {
        panic!("{events:?}")
    };
    assert_eq!(hit.struck, Struck::Body(BodyId(1)));
    assert!(hit.time > 0.3 && hit.time <= 0.5, "time {}", hit.time);
    // The hit lies on the hull's surface at that instant, within 1 mm.
    let yaw = -60f64.to_radians() + 120f64.to_radians() * hit.time;
    let rel = (hit.point - centre - v3(0.0, 0.0, half.z))
        .xy()
        .rotated(-yaw);
    let q = [rel.x.abs() - half.x, rel.y.abs() - half.y];
    let outside = q.iter().map(|d| d.max(0.0).powi(2)).sum::<f64>().sqrt();
    let depth = -q.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        outside <= 1e-3 && depth <= 1e-3,
        "outside {outside}, depth {depth}"
    );
    // Held at either end heading, the hull is never touched.
    for yaw in [swing.from.yaw, swing.to.yaw] {
        let still = Body {
            from: Pose { yaw, ..swing.from },
            to: Pose { yaw, ..swing.to },
            ..swing
        };
        assert!(fire(&[still])
            .iter()
            .all(|e| !matches!(e, FlightEvent::Impact(_))));
    }
}

const RIDGE: &str = r#","relief":[{"kind":"ridge","center":[300,200],"peak_m":25,"radius_m":60}],"props":[{"kind":"wall","center":[300,300],"yaw":0,"half_extents":[0.15,6,2]}]"#;

#[test]
fn terrain_and_props_in_front_are_struck_before_a_body_behind() {
    let world = flat([800.0, 400.0], RIDGE);
    let dt = config().tick_s();
    let speed = weapon("rifle").speed_mps;
    for (y, expect) in [(200.0, Struck::Terrain), (300.0, Struck::Prop(0))] {
        let behind = Mover::standing(9, 9, soldier_shape(), v3(420.0, y, 0.0));
        let from = v3(180.0, y, 1.4);
        // Aimed straight at the body's chest through the crest or wall.
        let dir = (behind.base + v3(0.0, 0.0, 1.2) - from).normalized();
        let mut store = Projectiles::new(config());
        store.launch(round(from, dir * speed, 0.0));
        let events = fly(&mut store, &world, 30, |k| vec![behind.body(k, dt)]);
        let [(_, hit)] = impacts(&events)[..] else {
            panic!("{events:?}")
        };
        assert_eq!(hit.struck, expect);
        assert!(hit.point.x < 360.0);
        if expect == Struck::Terrain {
            assert!(hit.point.x > 240.0);
            let ground = world.height_at(hit.point.x, hit.point.y).unwrap();
            assert!((hit.point.z - ground).abs() < 1e-6 && ground > 1.0);
        } else {
            assert!((hit.point.x - 299.85).abs() < 1e-6);
        }
    }
}

#[test]
fn an_exact_tie_goes_to_the_lowest_collider_id_whatever_the_order() {
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let cube = Shape::Box {
        half: v3(0.5, 0.5, 1.0),
    };
    // Both faces lie on x = 50 and both straddle the line y = 100.
    let a = Mover::standing(7, 1, cube, v3(50.5, 100.4, 0.0));
    let b = Mover::standing(3, 2, cube, v3(50.5, 99.6, 0.0));
    for order in [[a, b], [b, a]] {
        let mut store = Projectiles::new(config());
        store.launch(round(v3(10.0, 100.0, 1.0), v3(600.0, 0.0, 0.0), 0.0));
        let events = fly(&mut store, &world, 5, |k| {
            order.iter().map(|m| m.body(k, dt)).collect()
        });
        let [(_, hit)] = impacts(&events)[..] else {
            panic!("{events:?}")
        };
        assert_eq!(hit.struck, Struck::Body(BodyId(3)));
        assert!((hit.point.x - 50.0).abs() < 1e-9);
    }
}

#[test]
fn the_first_body_consumes_the_round_whatever_its_side() {
    let world = flat([400.0, 400.0], "");
    let dt = config().tick_s();
    // A friendly soldier of another unit stands in the line of fire ahead of
    // an enemy; the shooter's own squadmate, nearer still, is never struck.
    let shooter = Mover::standing(1, 10, soldier_shape(), v3(50.0, 100.0, 0.0));
    let own_squad = Mover::standing(4, 10, soldier_shape(), v3(80.0, 100.0, 0.0));
    let squadmate = Mover::standing(2, 11, soldier_shape(), v3(120.0, 100.0, 0.0));
    let enemy = Mover::standing(3, 20, soldier_shape(), v3(121.0, 100.0, 0.0));
    let mut store = Projectiles::new(config());
    // The muzzle is inside the shooter's own capsule.
    store.launch(Launch {
        shooter: Some(Shooter {
            unit: UnitId(10),
            body: BodyId(1),
            cover: None,
        }),
        ..round(v3(50.0, 100.0, 1.4), v3(850.0, 0.0, 0.0), 3.0)
    });
    let events = fly(&mut store, &world, 20, |k| {
        [shooter, own_squad, squadmate, enemy]
            .iter()
            .map(|m| m.body(k, dt))
            .collect()
    });
    let hits = impacts(&events);
    assert_eq!(hits.len(), 1, "{events:?}");
    assert_eq!(hits[0].1.struck, Struck::Body(BodyId(2)));
    assert!(store.active().is_empty());
    // The enemy just behind is within reach of the path, so it reports a near
    // miss; the firing unit is never suppressed by its own round.
    let misses: Vec<_> = events
        .iter()
        .filter_map(|(_, e)| match e {
            FlightEvent::NearMiss(m) => Some(m.unit),
            _ => None,
        })
        .collect();
    assert_eq!(misses, vec![UnitId(20)]);
}

#[test]
fn near_misses_report_the_closest_body_once_per_unit_per_tick() {
    let world = flat([600.0, 400.0], "");
    let dt = config().tick_s();
    let rifle = weapon("rifle");
    let r = physics("soldier_radius_m");
    let soldier = |id, unit, x, offset| {
        Mover::standing(id, unit, soldier_shape(), v3(x, 100.0 + offset, 0.0))
    };
    // Squad 5 flanks the line 1.0, 1.5 and 2.0 m off, all passed in one tick;
    // squad 6 is 2.5 m off; squad 8 is beyond the 3 m suppression radius.
    let bodies = [
        soldier(11, 5, 290.0, 1.5),
        soldier(12, 5, 300.0, -1.0),
        soldier(13, 5, 305.0, 2.0),
        soldier(21, 6, 298.0, 2.5),
        soldier(31, 8, 300.0, -3.0 - r - 0.01),
    ];
    let mut store = Projectiles::new(config());
    store.launch(round(
        v3(0.0, 100.0, 1.2),
        // Fast enough to pass squad 5's 15 m in one tick.
        v3(850.0, 0.0, 0.0),
        rifle.suppression_radius_m,
    ));
    let events = fly(&mut store, &world, 40, |k| {
        bodies.iter().map(|m| m.body(k, dt)).collect()
    });
    let misses: Vec<_> = events
        .iter()
        .filter_map(|(t, e)| match e {
            FlightEvent::NearMiss(m) => Some((*t, *m)),
            _ => None,
        })
        .collect();
    assert_eq!(misses.len(), 2, "{misses:?}");
    let by_unit = |u| misses.iter().find(|(_, m)| m.unit == UnitId(u)).unwrap().1;
    let five = by_unit(5);
    assert_eq!(five.body, BodyId(12));
    assert!(
        (five.distance - (1.0 - r)).abs() < 1e-6,
        "{}",
        five.distance
    );
    assert!((five.point.x - 300.0).abs() < 0.01);
    assert!((by_unit(6).distance - (2.5 - r)).abs() < 1e-6);
    // Events within a tick come in time order.
    assert!(misses[0].1.time <= misses[1].1.time);
}

#[test]
fn replaying_the_same_launches_reproduces_every_event_and_digest() {
    let world = flat([800.0, 400.0], RIDGE);
    let dt = config().tick_s();
    let run = || {
        let mut store = Projectiles::new(config());
        let movers: Vec<Mover> = (0..40)
            .map(|i| {
                let shape = if i % 5 == 0 {
                    tank_shape()
                } else {
                    soldier_shape()
                };
                Mover {
                    turn_rate: if i % 5 == 0 { 0.4 } else { 0.0 },
                    ..Mover::standing(
                        i,
                        i / 4,
                        shape,
                        v3(150.0 + 11.0 * i as f64, 80.0 + (i % 7) as f64 * 30.0, 0.0),
                    )
                    .moving(v3(1.5, -2.0 + (i % 3) as f64, 0.0))
                }
            })
            .collect();
        let mut log = Vec::new();
        let mut events = Vec::new();
        for tick in 1..=120u64 {
            for j in 0..6 {
                let a = (tick * 6 + j) as f64;
                store.launch(round(
                    v3(20.0, 40.0 + (a * 7.3) % 320.0, 1.4),
                    v3(700.0, ((a * 3.1) % 40.0) - 20.0, (a * 0.37) % 12.0),
                    3.0,
                ));
            }
            events.clear();
            let bodies: Vec<Body> = movers.iter().map(|m| m.body(tick, dt)).collect();
            advance_projectiles(&mut store, &world, &bodies, &mut events, &mut stop);
            assert!(events.windows(2).all(|w| w[0].time() <= w[1].time()));
            let mut d = Digest::default();
            store.digest(&mut d);
            log.push((events.clone(), d.finish()));
        }
        log
    };
    let first = run();
    assert!(first.iter().any(|(e, _)| e
        .iter()
        .any(|e| matches!(e, FlightEvent::Impact(i) if matches!(i.struck, Struck::Body(_))))));
    assert_eq!(first, run());
}
