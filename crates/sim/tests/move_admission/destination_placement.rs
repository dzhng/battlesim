//! Standing room and reachable destinations around walls and parked bodies.
use super::*;

/// A court 24 m deep between two buildings' walls, north and south, each
/// lined with cars parked nose in to it in bays 2.6 m apart, a metre clear
/// of the wall: an aisle about 14 m wide between the rows. Open paving
/// round it.
fn parked_court(units: serde_json::Value) -> Battle {
    let mut props = Vec::new();
    for (wall, bays) in [(82.0, 79.0), (58.0, 61.0)] {
        props.push(json!({"kind": "wall", "center": [60, wall], "yaw": 0,
            "half_extents": [30, 0.5, 3]}));
        for k in 0..10 {
            props.push(
                json!({"kind": "parked_car", "center": [48.3 + 2.6 * k as f64, bays],
                "yaw": std::f64::consts::FRAC_PI_2, "half_extents": [2.1, 0.9, 0.75]}),
            );
        }
    }
    let map = json!({"size": [160, 120], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": props});
    Battle::new(&common::scenario(&map.to_string(), units, json!([])), 1)
}

/// Every vehicle-stopping body under `unit`'s hull parked at `at`, facing the way
/// it drives in from where it stands.
fn bodies_under(battle: &Battle, unit: UnitId, at: [f64; 2]) -> Vec<String> {
    let u = battle.unit(unit).unwrap();
    let to = sim::math::v2(at[0], at[1]);
    let way = to - u.position.xy();
    let parked = sim::math::Obb2 {
        center: to,
        half: u.hull.unwrap().xy(),
        yaw: libm::atan2(way.y, way.x),
    };
    battle
        .world()
        .props()
        .filter(|b| b.blocks(contract::map::MoverClass::Vehicle))
        .filter(|b| b.footprint().overlaps(&parked))
        .map(|b| format!("{:?} at {:?}", b.kind, b.center))
        .collect()
}

/// The moment: a tank is sent to a point among cars parked nose in against
/// a wall. It cannot stand on a car (shoving it into the wall is not
/// parking), so it is sent to the nearest paving its whole hull stands clear
/// on, facing the way it drives in, a few metres short of the bays.
#[test]
fn a_vehicle_sent_onto_cars_parked_against_a_wall_parks_on_the_clear_ground_beside_them() {
    let mut battle = parked_court(json!([
        {"side": "blue", "kind": "test_tank", "position": [60, 20], "yaw": std::f64::consts::FRAC_PI_2}
    ]));
    let goal = [60.0, 79.0];
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0], goal),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let destination = &ack.placement.unwrap().destinations[0];
    assert!(destination.placed, "{destination:?}");
    let at = destination.goal;
    assert_eq!(
        bodies_under(&battle, UnitId(0), at),
        Vec::<String>::new(),
        "the tank's hull at {at:?}, facing the way it drives in, must stand clear"
    );
    let off = (at[0] - goal[0]).hypot(at[1] - goal[1]);
    assert!(
        off < 12.0,
        "the nearest clear ground is a few metres short of the bays, not {off:.1} m away at {at:?}"
    );
}

/// A lawn 30 m square walled round, centred on (60, 70), on open ground;
/// with `gate_m`, a gap that wide in the middle of its south wall.
fn walled_lawn(gate_m: f64, units: serde_json::Value) -> Battle {
    let wall = |center: [f64; 2], half: [f64; 2]| {
        json!({"kind": "wall", "center": center, "yaw": 0,
            "half_extents": [half[0], half[1], 1.5]})
    };
    let mut props = vec![
        wall([60.0, 85.0], [15.5, 0.5]),
        wall([45.0, 70.0], [0.5, 15.5]),
        wall([75.0, 70.0], [0.5, 15.5]),
    ];
    if gate_m > 0.0 {
        let side = (30.0 - gate_m) / 4.0;
        props.push(wall([45.0 + side, 55.0], [side, 0.5]));
        props.push(wall([75.0 - side, 55.0], [side, 0.5]));
    } else {
        props.push(wall([60.0, 55.0], [15.5, 0.5]));
    }
    let map = json!({"size": [160, 140], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": props});
    Battle::new(&common::scenario(&map.to_string(), units, json!([])), 1)
}

/// The moment: a tank is sent into the middle of a lawn walled all round,
/// with no gate a vehicle could take. "Go there" means the nearest ground
/// near there it can drive to: it pulls up against the outside of the wall
/// nearest the click, rather than the order being refused.
#[test]
fn a_vehicle_sent_into_a_walled_lawn_with_no_way_in_parks_outside_its_nearest_wall() {
    let mut battle = walled_lawn(
        0.0,
        json!([{"side": "blue", "kind": "test_tank", "position": [60, 20], "yaw": std::f64::consts::FRAC_PI_2}]),
    );
    let hull = battle.unit(UnitId(0)).unwrap().hull.unwrap();
    let radius = hull.x.hypot(hull.y);
    let goal = [60.0, 72.0];
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0], goal),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let destination = &ack.placement.unwrap().destinations[0];
    assert!(destination.placed, "{destination:?}");
    let at = destination.goal;
    let inside = (45.0..=75.0).contains(&at[0]) && (55.0..=85.0).contains(&at[1]);
    assert!(
        !inside,
        "a tank cannot get into the lawn, yet was sent to {at:?}"
    );
    assert_eq!(
        bodies_under(&battle, UnitId(0), at),
        Vec::<String>::new(),
        "the tank's hull at {at:?}, facing the way it drives in, must stand clear"
    );
    // The lawn's nearest edge is 13 m from the click; past the wall and a
    // hull's radius, it stands within a few metres of that.
    let off = (at[0] - goal[0]).hypot(at[1] - goal[1]);
    assert!(
        off < 13.0 + 0.5 + radius + 4.0,
        "parks just outside the nearest wall, not {off:.1} m off at {at:?}"
    );
}

/// The same lawn with a gate a tank fits through: it drives in and parks
/// where it was sent.
#[test]
fn a_vehicle_sent_into_a_walled_lawn_through_its_gate_parks_where_it_was_sent() {
    let mut battle = walled_lawn(
        12.0,
        json!([{"side": "blue", "kind": "test_tank", "position": [60, 20], "yaw": std::f64::consts::FRAC_PI_2}]),
    );
    let goal = [60.0, 72.0];
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0], goal),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    let destination = &ack.placement.unwrap().destinations[0];
    assert!(destination.placed, "{destination:?}");
    assert_eq!(destination.goal, goal);
}

/// The moment: a column of vehicles comes into a court along its row of
/// bays and is sent to the far end of the row: its formation lays every
/// member on a parked car. Every member gets a destination it can reach and
/// stand on, each hull clear of every car and wall facing the way it drives in.
#[test]
fn a_group_sent_into_a_court_lined_with_parked_cars_is_placed_whole() {
    let west = std::f64::consts::PI;
    let mut battle = parked_court(json!([
        {"side": "blue", "kind": "test_tank", "position": [100, 79], "yaw": west},
        {"side": "blue", "kind": "test_supply", "position": [112, 79], "yaw": west},
        {"side": "blue", "kind": "test_jeep", "position": [122, 79], "yaw": west}
    ]));
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[0, 1, 2], [48.0, 79.0]),
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
    for destination in ack.placement.unwrap().destinations {
        assert!(destination.placed, "{destination:?}");
        assert_eq!(
            bodies_under(&battle, destination.unit, destination.goal),
            Vec::<String>::new(),
            "{destination:?}"
        );
    }
}

/// The other half of that promise: a squad sent into a yard walled on every
/// side walks there, finds no way in, and stops outside: it neither passes
/// through a wall nor keeps searching for ever.
#[test]
fn a_squad_sent_into_a_closed_yard_stops_outside_it() {
    let wall = |center: [f64; 2], half: [f64; 2]| {
        format!(
            r#"{{"kind":"wall","center":[{},{}],"yaw":0,"half_extents":[{},{},2]}}"#,
            center[0], center[1], half[0], half[1]
        )
    };
    let yard = [
        wall([216.0, 120.0], [1.0, 17.0]),
        wall([184.0, 120.0], [1.0, 17.0]),
        wall([200.0, 103.0], [17.0, 1.0]),
        wall([200.0, 137.0], [17.0, 1.0]),
    ]
    .join(",");
    let mut b = strip(&format!(r#","props":[{yard}]"#), json!([]));
    assert_eq!(placed(&mut b, &[1], [200.0, 120.0]), [true]);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: move_to(&[1], [200.0, 120.0]),
        queued: false,
    });
    assert_eq!(ack.error, None);
    let inside = |b: &Battle| {
        b.unit(UnitId(1)).unwrap().members.iter().any(|s| {
            let p = s.position.xy();
            (185.0..215.0).contains(&p.x) && (104.0..136.0).contains(&p.y)
        })
    };
    for _ in 0..120 * b.rules().tick_hz as u64 {
        b.step();
        assert!(
            !inside(&b),
            "a soldier got into the closed yard at tick {}",
            b.tick()
        );
    }
    let squad = b.unit(UnitId(1)).unwrap();
    assert_eq!(
        squad.state,
        contract::observation::MoveState::RouteBlocked,
        "it stops at {:?} with no way left",
        squad.position.xy()
    );
}

#[test]
fn a_reachable_goal_places_formation_slots_beside_a_closed_pocket() {
    let mut rules = common::scenario_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "wall",
        json!({"body":{"occludes":false}}),
    );
    let mut setup = common::scenario(
        r#"{"size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[
            {"kind":"wall","center":[240,140],"yaw":0,"half_extents":[20,3,2]},
            {"kind":"wall","center":[240,180],"yaw":0,"half_extents":[20,3,2]},
            {"kind":"wall","center":[220,160],"yaw":0,"half_extents":[3,20,2]},
            {"kind":"wall","center":[260,160],"yaw":0,"half_extents":[3,20,2]}]}"#,
        json!([
            {"side":"blue","kind":"test_rifle","position":[160,160],"yaw":0},
            {"side":"blue","kind":"test_rifle","position":[190,160],"yaw":0}
        ]),
        json!([]),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    let mut battle = Battle::new(&setup, 1);
    // The click is on open ground east of the yard, but the rear squad's
    // intended formation slot falls inside it. The slot search must continue.
    let goal = [285.0, 160.0];
    let single = battle
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0)],
                goal,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(single[0].placed, "the ordered goal itself is reachable");
    assert_eq!(single[0].goal, goal);
    let destinations = battle
        .preview_move(
            Side::Blue,
            &contract::command::MovePreviewRequest {
                units: vec![UnitId(0), UnitId(1)],
                goal,
                ..Default::default()
            },
        )
        .unwrap();
    for destination in destinations {
        assert!(
            destination.placed,
            "reachable ground beside the pocket remains available: {destination:?}"
        );
        let p = destination.goal;
        assert!(
            p[0] < 219.0 || p[0] > 261.0 || p[1] < 139.0 || p[1] > 181.0,
            "marker must remain outside the pocket: {p:?}"
        );
    }
}
