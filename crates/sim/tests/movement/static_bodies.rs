//! Routes and recovery around static bodies.
use super::*;

#[test]
fn a_tank_routes_around_walls_and_arrives() {
    let mut b = Battle::new(
        &scenario(
            serde_json::json!([{ "side": "blue", "kind": "test_tank", "position": [60, 40] }]),
            serde_json::json!([]),
        ),
        1,
    );
    let mut o = Orders { seq: 0 };
    o.go(&mut b, &[0], [60.0, 110.0], 1, RoutePolicy::Shortest, false);
    let ticks = run(&mut b, &[0], 3000, |b| {
        let p = xy(&own(b, 0));
        for prop in b.world().props().filter(|p| p.blocks(MoverClass::Vehicle)) {
            assert!(
                !prop.footprint().contains(sim::math::v2(p[0], p[1]), 0.0),
                "inside a prop at {p:?}"
            );
        }
    });
    assert!(ticks < 3000);
    assert!(dist(xy(&own(&b, 0)), [60.0, 110.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 3,
        "searches {}",
        b.route_searches(Side::Blue)
    );
}

#[test]
fn a_new_obstacle_is_learned_on_contact_and_routed_around() {
    // A wall appears across the tank's straight path at tick 5; red is far away.
    let units = serde_json::json!([
        { "side": "blue", "kind": "test_tank", "position": [230, 120] },
        { "side": "red", "kind": "test_tank", "position": [370, 280] },
    ]);
    let events = serde_json::json!([{ "tick": 5, "add_prop":
        { "kind": "wall", "center": [260, 120], "yaw": 0, "half_extents": [0.5, 15, 2] } }]);
    let mut b = Battle::new(&scenario(units, events), 1);
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [300.0, 120.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    b.step();
    let first = own(&b, 0).route.clone();
    assert_eq!(first.len(), 1, "unknown wall: straight route");
    let wall = sim::math::v2(260.0, 120.0);
    let ticks = run(&mut b, &[0], 2000, |b| {
        let p = xy(&own(b, 0));
        assert!(
            (p[0] - wall.x).abs() > 0.5 || (p[1] - wall.y).abs() > 15.0,
            "drove through the wall at {p:?}"
        );
    });
    assert!(ticks < 2000 && dist(xy(&own(&b, 0)), [300.0, 120.0]) < 1.0);
    assert!(
        b.route_searches(Side::Blue) <= 4,
        "searches {}",
        b.route_searches(Side::Blue)
    );
    assert_eq!(b.route_searches(Side::Red), 0, "red never planned");
}

#[test]
fn a_tank_nosed_up_to_a_wall_backs_off_to_turn_and_drives_round_it() {
    // Its nose a hand's breadth from the wall's face: a pivot swings the
    // hull's front corners into the wall, so the crew reverses until the
    // turn clears, then drives round the wall's end to the far side.
    let units = serde_json::json!([
        { "side": "blue", "kind": "test_tank", "position": [255.9, 120], "yaw": 0 },
    ]);
    let events = serde_json::json!([{ "tick": 1, "add_prop":
        { "kind": "wall", "center": [260, 120], "yaw": 0, "half_extents": [0.5, 15, 2] } }]);
    let mut b = Battle::new(&scenario(units, events), 1);
    for _ in 0..3 {
        b.step();
    }
    let mut o = Orders { seq: 0 };
    o.go(
        &mut b,
        &[0],
        [280.0, 120.0],
        1,
        RoutePolicy::Shortest,
        false,
    );
    let limit = 60 * b.rules().tick_hz;
    let ticks = run(&mut b, &[0], limit, |b| {
        let hull = b.unit(UnitId(0)).unwrap().ground_footprint().unwrap();
        assert!(
            !b.world()
                .props()
                .filter(|p| p.blocks(MoverClass::Vehicle))
                .any(|p| hull.overlaps(&p.footprint())),
            "the hull is in a body at {:?}",
            hull.center
        );
    });
    let end = own(&b, 0);
    assert!(
        ticks < limit && dist(xy(&end), [280.0, 120.0]) < 1.0,
        "it never got round the wall: {:?} {:?}",
        end.position,
        end.state
    );
}
