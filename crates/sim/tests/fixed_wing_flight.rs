use crate::common::air_field::{wing_limits, AirField, Track};
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use sim::air::{fly, steer, velocity, Bounds, FixedWingState, Intent, Steer};
use sim::math::{v2, v3, V2};

fn state() -> FixedWingState {
    FixedWingState {
        pos: v3(0.0, 0.0, 250.0),
        heading: 0.0,
        speed_mps: 100.0,
        burner: 0.0,
        turn_sign: 0,
    }
}

#[test]
fn a_placed_jet_records_one_sample_per_tick() {
    let mut battle = AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    let samples = Track::record(&mut battle, 60);
    assert_eq!(samples.len(), 60);
    assert_eq!(samples.last().unwrap().tick, 60);
}

#[test]
fn a_jet_never_flies_slower_than_cruise() {
    let mut battle = AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    let cruise = battle.unit(UnitId(0)).unwrap().motion;
    let samples = Track::record(&mut battle, 120);
    let cruise = match cruise {
        sim::units::Motion::Air(flight) => flight.cruise_mps,
        _ => panic!("test_jet must be air"),
    };
    for sample in samples {
        assert!(
            sample.vel.length() >= cruise - 1e-6,
            "speed {}",
            sample.vel.length()
        );
    }
}

#[test]
fn afterburner_ramps_to_speed_over_its_ramp_and_back() {
    let l = wing_limits();
    let mut s = state();
    s = fly(
        s,
        Steer {
            turn: 0.0,
            agl_m: 250.0,
            burner: true,
        },
        &l,
        &|_| 0.0,
        1.0,
    );
    assert!((s.speed_mps - 125.0).abs() < 1e-9);
    s = fly(
        s,
        Steer {
            turn: 0.0,
            agl_m: 250.0,
            burner: true,
        },
        &l,
        &|_| 0.0,
        1.0,
    );
    assert!((s.speed_mps - 150.0).abs() < 1e-9);
    s = fly(
        s,
        Steer {
            turn: 0.0,
            agl_m: 250.0,
            burner: false,
        },
        &l,
        &|_| 0.0,
        1.0,
    );
    assert!((s.speed_mps - 125.0).abs() < 1e-9);
}

#[test]
fn switching_the_afterburner_off_eases_back_to_cruise_over_its_ramp() {
    let mut battle = AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    let toggle = |battle: &mut sim::battle::Battle, seq, enabled| {
        battle.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq,
            queued: false,
            order: Order::SetAfterburner {
                units: vec![UnitId(0)],
                enabled,
            },
        });
    };
    let hz = u64::from(battle.rules().tick_hz);
    let ramp_ticks = (battle.rules().air.fixed_wing.afterburner_ramp_s * hz as f64).ceil() as u64;
    let speed = |battle: &sim::battle::Battle| battle.unit(UnitId(0)).unwrap().drive_speed_mps;
    let cruise = speed(&battle).max(1.0);
    for _ in 0..hz {
        battle.step();
    }
    let cruise = speed(&battle).max(cruise);
    toggle(&mut battle, 1, true);
    for _ in 0..ramp_ticks + 1 {
        battle.step();
    }
    let burning = speed(&battle);
    assert!(
        burning > cruise * 1.2,
        "afterburner speed {burning} over cruise {cruise}"
    );
    toggle(&mut battle, 2, false);
    for _ in 0..ramp_ticks / 2 {
        battle.step();
    }
    let easing = speed(&battle);
    assert!(
        easing > cruise + 1.0 && easing < burning - 1.0,
        "halfway through the ramp it flies {easing}, between {cruise} and {burning}"
    );
    for _ in 0..ramp_ticks {
        battle.step();
    }
    assert!((speed(&battle) - cruise).abs() < 1e-6, "back at cruise");
}

#[test]
fn afterburner_burns_fuel_at_its_multiplier() {
    let mut battle = AirField::new(3000.0)
        .aircraft("test_jet", [300.0, 1500.0], 0.0)
        .battle();
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::SetAfterburner {
            units: vec![UnitId(0)],
            enabled: true,
        },
    });
    let before = battle.unit(UnitId(0)).unwrap().air.unwrap().fuel_s;
    battle.step();
    let after = battle.unit(UnitId(0)).unwrap().air.unwrap().fuel_s;
    let dt = 1.0 / battle.rules().tick_hz as f64;
    assert!((before - after - 2.0 * dt).abs() < 1e-9);
}

#[test]
fn a_jet_flies_where_its_nose_points_and_turns_no_faster_than_its_rate() {
    let l = wing_limits();
    let s = fly(
        state(),
        Steer {
            turn: 1.0,
            agl_m: 250.0,
            burner: false,
        },
        &l,
        &|_| 0.0,
        0.5,
    );
    assert!((s.heading - 0.5).abs() < 1e-9);
    let v = velocity(&s);
    assert!(v.x > 0.0 && v.y > 0.0);
}

#[test]
fn a_jet_never_drops_below_the_floor_over_relief() {
    let l = wing_limits();
    let s = fly(
        state(),
        Steer {
            turn: 0.0,
            agl_m: 0.0,
            burner: false,
        },
        &l,
        &|_| 500.0,
        1.0,
    );
    assert!(s.pos.z - 500.0 >= l.floor_agl_m - 1e-9);
}

#[test]
fn a_jet_holds_cruise_over_rolling_ground_without_steps() {
    let l = wing_limits();
    let mut s = state();
    for i in 0..20 {
        s = fly(
            s,
            Steer {
                turn: 0.0,
                agl_m: 250.0,
                burner: false,
            },
            &l,
            &|p| 20.0 * (p.x / 100.0).sin(),
            0.1,
        );
        assert!(
            (s.pos.z - (20.0 * (s.pos.x / 100.0).sin() + 250.0)).abs() < 30.0 * 0.1 + 1e-9,
            "step {i}"
        );
    }
}

#[test]
fn a_jet_flies_straight_over_a_61m_tower() {
    let l = wing_limits();
    let mut s = state();
    for _ in 0..60 {
        s = fly(
            s,
            Steer {
                turn: 0.0,
                agl_m: 250.0,
                burner: false,
            },
            &l,
            &|p| {
                if p.x > 500.0 && p.x < 600.0 {
                    61.0
                } else {
                    0.0
                }
            },
            0.1,
        );
    }
    assert!(s.heading.abs() < 1e-9);
    assert!(s.pos.x > 590.0);
}

#[test]
fn a_move_goal_is_circled_at_speed_over_turn_rate() {
    let l = wing_limits();
    let mut s = state();
    s.pos = v3(-250.0, 0.0, 250.0);
    let centre = v2(0.0, 0.0);
    let bounds = Bounds::new(v2(-2_000.0, -2_000.0), v2(2_000.0, 2_000.0));
    let radius = s.speed_mps / l.turn_rad_s;
    let mut samples = Vec::new();
    for tick in 0..400 {
        let command = steer(&Intent::Orbit { centre }, &s, &l, &bounds, 1.0 / 30.0);
        s = fly(s, command, &l, &|_| 0.0, 1.0 / 30.0);
        if tick > 200 {
            samples.push((s.pos.xy() - centre).length());
        }
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    assert!(
        (mean - radius).abs() < 85.0,
        "mean radius {mean}, expected {radius}"
    );
}

#[test]
fn a_jet_flying_to_a_far_goal_holds_a_straight_heading() {
    let l = wing_limits();
    let mut s = state();
    s.pos = v3(-1_000.0, 0.0, 250.0);
    let bounds = Bounds::new(v2(-2_000.0, -2_000.0), v2(2_000.0, 2_000.0));
    let intent = Intent::Pursue {
        point: v2(1_000.0, 0.0),
        vel: v2(0.0, 0.0),
        lag_s: 0.0,
        agl: 250.0,
    };
    for _ in 0..120 {
        let command = steer(&intent, &s, &l, &bounds, 1.0 / 30.0);
        let before = s.heading;
        s = fly(s, command, &l, &|_| 0.0, 1.0 / 30.0);
        assert!((s.heading - before).abs() < 1e-9);
    }
}

#[test]
fn a_circling_jet_holds_cruise_over_raised_ground() {
    let l = wing_limits();
    let mut s = state();
    s.pos = v3(-150.0, 0.0, 250.0);
    let bounds = Bounds::new(v2(-2_000.0, -2_000.0), v2(2_000.0, 2_000.0));
    for tick in 0..300 {
        let command = steer(
            &Intent::Orbit {
                centre: v2(0.0, 0.0),
            },
            &s,
            &l,
            &bounds,
            1.0 / 30.0,
        );
        s = fly(s, command, &l, &|_| 80.0, 1.0 / 30.0);
        if tick > 120 {
            assert!((s.pos.z - 80.0 - l.cruise_agl_m).abs() < 2.0);
        }
    }
}

#[test]
fn a_jet_circles_the_way_it_was_already_turning() {
    let l = wing_limits();
    let mut s = state();
    s.turn_sign = -1;
    s.pos = v3(100.0, 0.0, 250.0);
    let bounds = Bounds::new(v2(-2_000.0, -2_000.0), v2(2_000.0, 2_000.0));
    for _ in 0..120 {
        let command = steer(
            &Intent::Orbit {
                centre: v2(0.0, 0.0),
            },
            &s,
            &l,
            &bounds,
            1.0 / 30.0,
        );
        s = fly(s, command, &l, &|_| 0.0, 1.0 / 30.0);
    }
    assert!(s.turn_sign < 0, "turn sign changed to {}", s.turn_sign);
}

#[test]
fn an_attack_circles_the_targets_current_position() {
    let l = wing_limits();
    let mut s = state();
    let target = v2(0.0, 0.0);
    let bounds = Bounds::new(v2(-2_000.0, -2_000.0), v2(2_000.0, 2_000.0));
    for _ in 0..200 {
        let command = steer(
            &Intent::Pursue {
                point: target,
                vel: v2(10.0, 0.0),
                lag_s: 2.0,
                agl: 100.0,
            },
            &s,
            &l,
            &bounds,
            1.0 / 30.0,
        );
        s = fly(s, command, &l, &|_| 0.0, 1.0 / 30.0);
    }
    let aim = target - v2(20.0, 0.0);
    assert!((s.pos.xy() - aim).length() > 0.0);
}

#[test]
fn a_circle_by_the_edge_turns_back_inside_the_square() {
    let l = wing_limits();
    let mut s = state();
    s.pos = v3(1_850.0, 1_000.0, 250.0);
    s.heading = 0.0;
    let bounds = Bounds::new(v2(0.0, 0.0), v2(2_000.0, 2_000.0));
    for _ in 0..600 {
        let command = steer(
            &Intent::Orbit {
                centre: v2(1_950.0, 1_000.0),
            },
            &s,
            &l,
            &bounds,
            1.0 / 30.0,
        );
        s = fly(s, command, &l, &|_| 0.0, 1.0 / 30.0);
        assert!(s.pos.x >= 0.0 && s.pos.x <= 2_000.0 && s.pos.y >= 0.0 && s.pos.y <= 2_000.0);
    }
}

/// The edge guard keeps a jet its margin inside a straight edge, whatever
/// it is trying to do: a jet steering straight off the map, from any
/// heading, never comes nearer than `edge_margin_m`. In a corner, turning
/// away from one edge carries it toward the other: it may cross into the
/// margin there, but never out of the square.
#[test]
fn the_edge_guard_holds_a_jet_its_margin_inside_every_edge() {
    let l = wing_limits();
    let side = 2_000.0;
    let bounds = Bounds::new(v2(0.0, 0.0), v2(side, side));
    let dt = 1.0 / 30.0;
    // Two steps of slack: the guard looks one step ahead, and a tick's turn
    // toward the edge between its looks costs about another.
    let slack = 2.0 * l.cruise_mps * dt;
    // The margin on a straight edge, kept out of either corner's reach.
    let straight = |p: V2| {
        let mid = |c: f64| (c - side * 0.5).abs() < side * 0.5 - l.edge_margin_m * 2.0;
        let room_x = p.x.min(side - p.x);
        let room_y = p.y.min(side - p.y);
        (mid(p.y) && room_x < room_y) || (mid(p.x) && room_y < room_x)
    };
    for start in [
        v2(1_000.0, 1_000.0),
        v2(1_600.0, 1_000.0),
        v2(1_600.0, 1_600.0),
    ] {
        for k in 0..24 {
            let out = f64::from(k) * 15_f64.to_radians();
            let mut s = state();
            s.pos = start.with_z(250.0);
            s.heading = out;
            for tick in 0..900 {
                // Always steering out of the square along `out`.
                let intent = Intent::Line {
                    through: s.pos.xy(),
                    heading: out,
                    agl: 250.0,
                };
                let command = steer(&intent, &s, &l, &bounds, dt);
                s = fly(s, command, &l, &|_| 0.0, dt);
                let p = s.pos.xy();
                let room = p.x.min(p.y).min(side - p.x).min(side - p.y);
                let nearest = if straight(p) {
                    l.edge_margin_m - slack
                } else {
                    0.0
                };
                assert!(
                    room >= nearest,
                    "from {start:?} steering {:.0} deg: {room:.0} m from an edge at {p:?}, tick {tick}",
                    out.to_degrees()
                );
            }
        }
    }
}

/// The guard takes a jet only at its point of no return, where it must turn
/// away now to stay its margin inside: within a turn radius plus the margin
/// of an edge, a jet flying along it, or toward it while it can still turn
/// away in time, flies as it means to.
#[test]
fn the_edge_guard_leaves_a_jet_that_can_still_turn_away() {
    let l = wing_limits();
    let side = 2_000.0;
    let bounds = Bounds::new(v2(0.0, 0.0), v2(side, side));
    let radius = l.cruise_mps / l.turn_rad_s;
    let dt = 1.0 / 30.0;
    // Half a turn radius nearer the east edge than a radius plus the margin.
    let x = side - l.edge_margin_m - radius * 0.5;
    for heading in [90_f64, 45.0, -45.0, 180.0] {
        let heading = heading.to_radians();
        let mut s = state();
        s.pos = v3(x, 1_000.0, 250.0);
        s.heading = heading;
        let intent = Intent::Line {
            through: s.pos.xy(),
            heading,
            agl: 250.0,
        };
        let command = steer(&intent, &s, &l, &bounds, dt);
        assert_eq!(
            command.turn,
            0.0,
            "turned off a {:.0} deg heading it could still turn away from",
            heading.to_degrees()
        );
    }
}

#[test]
fn a_queued_order_follows_once_the_circle_is_reached() {
    let mut battle = AirField::new(3_000.0)
        .aircraft("test_jet", [300.0, 1_500.0], 0.0)
        .battle();
    let first = v2(1_000.0, 1_500.0);
    let second = v2(2_000.0, 1_500.0);
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [first.x, first.y],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 2,
        queued: true,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 2,
            goal: [second.x, second.y],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    for _ in 0..600 {
        battle.step();
    }
    assert!((battle.unit(UnitId(0)).unwrap().position.xy() - second).length() < 2_000.0);
}

/// A jet that reaches its move goal keeps circling that goal once the
/// order is done (choice 03), rather than drifting onto a circle of its own
/// wherever the order happened to end.
#[test]
fn an_idle_jet_circles_the_goal_it_reached() {
    let mut battle = AirField::new(3_000.0)
        .aircraft("test_jet", [300.0, 1_500.0], 0.0)
        .battle();
    let goal = v2(1_300.0, 1_700.0);
    battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [goal.x, goal.y],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    let sim::units::Motion::Air(flight) = battle.unit(UnitId(0)).unwrap().motion else {
        panic!("test_jet flies");
    };
    let radius = flight.cruise_mps / flight.turn_rad_s;
    battle.step();
    assert!(!battle.unit(UnitId(0)).unwrap().orders.is_empty());
    let mut ticks = 0;
    while !battle.unit(UnitId(0)).unwrap().orders.is_empty() {
        battle.step();
        ticks += 1;
        assert!(ticks < 3_000, "the jet never reached its goal");
    }
    // One turn to settle onto the circle (its entry may loop through the
    // centre first), then several whole turns: the mean position is the
    // circle's centre.
    let turn_ticks = (std::f64::consts::TAU / flight.turn_rad_s * 30.0).round() as usize;
    for _ in 0..turn_ticks {
        battle.step();
    }
    let mut sum = v2(0.0, 0.0);
    for _ in 0..turn_ticks * 4 {
        battle.step();
        sum = sum + battle.unit(UnitId(0)).unwrap().position.xy();
    }
    let centre = sum * (1.0 / (turn_ticks * 4) as f64);
    assert!(
        (centre - goal).length() < radius * 0.25,
        "circles {centre:?}, {:.0} m from its goal (turn radius {radius:.0} m)",
        (centre - goal).length()
    );
}
