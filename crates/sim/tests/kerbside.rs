//! A street with cars parked beside it. A road journey's lane is the free
//! width of the road: a vehicle moves over toward the middle past a body
//! beside its lane and comes back after it. It does not refuse the street,
//! does not shove a car it has room to pass, and two vehicles sent through
//! from opposite ends both get through.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{MoveState, OwnUnit};
use contract::scenario::ScenarioDefinition;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, V2};

use crate::common;

/// The street's middle and half its length.
const MIDDLE: [f64; 2] = [400.0, 400.0];
const HALF_LENGTH_M: f64 = 300.0;
/// From the street's middle to the terraces' walls either side.
const WALL_M: f64 = 7.5;
/// From the street's middle to the near side of a parked car: a metre off
/// the kerb of a 7 m street.
const PARKED_M: f64 = 4.55;
/// A parked car's half length and half width.
const CAR: [f64; 2] = [2.1, 0.9];
/// Vehicles start and end this far from the middle of the street.
const END_M: f64 = 260.0;
/// The cross streets are this far from the middle of the street, and the
/// street round the back of the block this far to its left.
const CROSS_M: f64 = 280.0;
const BLOCK_M: f64 = 80.0;

/// The point `along` metres down a street of `bearing` from its middle and
/// `right` metres to the right of that direction.
fn at(bearing: f64, along: f64, right: f64) -> [f64; 2] {
    let (sin, cos) = bearing.sin_cos();
    [
        MIDDLE[0] + cos * along + sin * right,
        MIDDLE[1] + sin * along - cos * right,
    ]
}

/// How far `p` lies to the right of the middle of a street of `bearing`.
fn across(bearing: f64, p: [f64; 3]) -> f64 {
    let (sin, cos) = bearing.sin_cos();
    (p[0] - MIDDLE[0]) * sin - (p[1] - MIDDLE[1]) * cos
}

/// A 7 m street 600 m long on the heading `bearing`, between two rows of
/// terraces, and a way round the block on its left: a cross street near
/// each end and a street parallel to it 80 m off. With `cars`, runs of four
/// parked cars stand bumper to bumper beside its right-hand kerb (as one
/// drives along `bearing`), a gap between runs.
fn street(bearing: f64, cars: bool) -> Value {
    // The terraces stop short of the cross streets.
    let wall = |side: f64| {
        json!({ "kind": "wall", "center": at(bearing, 0.0, side * (WALL_M + 0.5)),
            "yaw": bearing, "half_extents": [CROSS_M - 12.0, 0.5, 3] })
    };
    let road = |from: [f64; 2], to: [f64; 2]| json!({ "kind": "road", "shape": { "kind": "stroke", "points": [from, to], "width_m": 7 } });
    let mut props = vec![wall(1.0), wall(-1.0)];
    if cars {
        // Six runs of four, a run every 60 m.
        for run in 0..6 {
            for k in 0..4 {
                let along = -170.0 + 60.0 * run as f64 + 4.7 * k as f64;
                props.push(json!({ "kind": "parked_car",
                    "center": at(bearing, along, PARKED_M + CAR[1]),
                    "yaw": bearing, "half_extents": [CAR[0], CAR[1], 0.75] }));
            }
        }
    }
    json!({ "size": [800, 800], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": props,
        "surfaces": [
            road(at(bearing, -HALF_LENGTH_M, 0.0), at(bearing, HALF_LENGTH_M, 0.0)),
            road(at(bearing, -CROSS_M, 0.0), at(bearing, -CROSS_M, -BLOCK_M)),
            road(at(bearing, CROSS_M, 0.0), at(bearing, CROSS_M, -BLOCK_M)),
            road(at(bearing, -CROSS_M, -BLOCK_M), at(bearing, CROSS_M, -BLOCK_M)),
        ] })
}

fn scenario(map: Value, units: Value) -> ScenarioDefinition {
    serde_json::from_value(json!({
        "map": map, "rules": common::scenario_rules(), "units": units, "events": [], "scripts": [],
    }))
    .unwrap()
}

fn own(b: &Battle, id: u32) -> OwnUnit {
    b.observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .clone()
}

/// Send `unit` to `goal` by road.
fn send(b: &mut Battle, seq: u64, unit: u32, goal: [f64; 2]) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq,
        order: Order::Move {
            units: vec![UnitId(unit)],
            gesture: seq,
            goal,
            route: RoutePolicy::Fastest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    });
    assert_eq!(
        ack.error,
        None,
        "{} (unit {unit}) to {goal:?}: {ack:?}",
        b.rules()
            .catalog
            .get(b.unit(UnitId(unit)).unwrap().kind)
            .name
    );
}

/// Where every parked car stands.
fn parked(b: &Battle) -> Vec<V2> {
    let car = common::kind("parked_car");
    b.world()
        .props()
        .filter(|prop| prop.kind == car)
        .map(|prop| prop.center)
        .collect()
}

fn near(b: &Battle, unit: u32, goal: [f64; 2]) -> bool {
    let p = own(b, unit).position;
    (v2(p[0], p[1]) - v2(goal[0], goal[1])).length() < 3.0
}

/// One vehicle of `kind` driven the length of the street, with the cars on
/// its right (`forward`) or on its left: the seconds it took.
fn drive(kind: &str, bearing: f64, cars: bool, forward: bool) -> f64 {
    let way = if forward { 1.0 } else { -1.0 };
    let (from, goal) = (
        at(bearing, -way * END_M, 0.0),
        at(bearing, way * END_M, 0.0),
    );
    let yaw = if forward {
        bearing
    } else {
        bearing + std::f64::consts::PI
    };
    let units = json!([{ "side": "blue", "kind": kind, "position": from, "yaw": yaw }]);
    let mut b = Battle::new(&scenario(street(bearing, cars), units), 1);
    let before = parked(&b);
    send(&mut b, 1, 0, goal);
    let hz = b.rules().tick_hz as u64;
    let what = format!(
        "a {kind} on a street at {:.0} degrees, cars {}",
        bearing.to_degrees(),
        match (cars, forward) {
            (false, _) => "nowhere",
            (true, true) => "on its right",
            (true, false) => "on its left",
        }
    );
    for _ in 0..600 * hz {
        b.step();
        assert!(
            b.load().planning_work
                <= b.rules().navigation.work_per_tick as u64 + sim::navigation::LARGEST_STEP
        );
        let unit = own(&b, 0);
        let hull = b.unit(UnitId(0)).unwrap().hull_box().unwrap();
        assert!(
            b.world()
                .props()
                .filter(|p| p.blocks(contract::map::MoverClass::Vehicle))
                .all(|p| !hull.overlaps(&p.footprint())),
            "{what}: its hull overlaps a body at tick {}",
            b.tick()
        );
        let off = across(bearing, unit.position);
        assert!(
            off.abs() < 3.5,
            "{what}: it left the carriageway, {off:.1} m right of the street's middle at {:?}",
            unit.position
        );
        assert_ne!(
            unit.state,
            MoveState::RouteBlocked,
            "{what}: the street is refused"
        );
        if unit.state == MoveState::Idle {
            break;
        }
    }
    assert!(
        near(&b, 0, goal),
        "{what}: it never arrived: {:?} at {:?}",
        own(&b, 0).state,
        own(&b, 0).position
    );
    for (was, now) in before.iter().zip(parked(&b)) {
        assert_eq!(*was, now, "{what}: a parked car was shoved");
    }
    b.tick() as f64 / hz as f64
}

/// The moment: a tank, a truck and a jeep each come down a 7 m street
/// between terraces with cars parked along one kerb. Each drives the length
/// of it, either way: it does not go round the block instead, it passes
/// every car without touching it, and it takes little longer than on the
/// empty street.
#[test]
fn every_hull_drives_a_parked_street_end_to_end_without_shoving_a_car() {
    // A street along the grid, and two that cut across it.
    for bearing in [0.0f64, 30.0, 45.0].map(f64::to_radians) {
        for kind in ["tank", "supply", "jeep"] {
            let bare = drive(kind, bearing, false, true);
            for forward in [true, false] {
                let took = drive(kind, bearing, true, forward);
                assert!(
                    took < 1.5 * bare + 5.0,
                    "a {kind} took {took:.0} s down the parked street at {:.0} degrees, {bare:.0} s down the empty one",
                    bearing.to_degrees()
                );
            }
        }
    }
}

/// The moment: two vehicles are sent through the same parked street from
/// opposite ends and meet in it. Both get through: neither waits for ever,
/// and their hulls never overlap.
#[test]
fn two_vehicles_sent_through_a_parked_street_from_opposite_ends_both_arrive() {
    for bearing in [0.0f64, 30.0].map(f64::to_radians) {
        for (east, west) in [("tank", "supply"), ("supply", "jeep"), ("tank", "tank")] {
            let ends = [at(bearing, -END_M, 0.0), at(bearing, END_M, 0.0)];
            let units = json!([
                { "side": "blue", "kind": east, "position": ends[0], "yaw": bearing },
                { "side": "blue", "kind": west, "position": ends[1],
                  "yaw": bearing + std::f64::consts::PI },
            ]);
            let mut b = Battle::new(&scenario(street(bearing, true), units), 1);
            let cars_before = parked(&b);
            // Each to 60 m short of where the other starts.
            let goals = [
                at(bearing, END_M - 60.0, 0.0),
                at(bearing, 60.0 - END_M, 0.0),
            ];
            send(&mut b, 1, 0, goals[0]);
            send(&mut b, 2, 1, goals[1]);
            let what = format!(
                "a {east} and a {west} meeting on a street at {:.0} degrees",
                bearing.to_degrees()
            );
            let hz = b.rules().tick_hz as u64;
            for _ in 0..300 * hz {
                b.step();
                let hull = |id| b.unit(UnitId(id)).unwrap().hull_box().unwrap();
                assert!(
                    !hull(0).overlaps(&hull(1)),
                    "{what}: their hulls overlap at tick {}",
                    b.tick()
                );
                for id in [0, 1] {
                    assert!(
                        b.world()
                            .props()
                            .filter(|p| p.blocks(contract::map::MoverClass::Vehicle))
                            .all(|p| !hull(id).overlaps(&p.footprint())),
                        "{what}: unit {id} overlaps a static body"
                    );
                }
                if [0, 1]
                    .iter()
                    .all(|id| own(&b, *id).state == MoveState::Idle)
                {
                    break;
                }
            }
            assert_eq!(parked(&b), cars_before, "{what}: a parked car was shoved");
            for (id, goal) in [(0, goals[0]), (1, goals[1])] {
                assert!(
                    near(&b, id, goal),
                    "{what}: unit {id} never arrived: {:?} at {:?}, after {} s",
                    own(&b, id).state,
                    own(&b, id).position,
                    b.tick() / hz
                );
            }
        }
    }
}

/// A kerbside route remains a counted, yielding job even when every lane
/// alternative fails. The same route is produced whether work is spread
/// over ticks or completed together.
#[test]
fn lane_refinement_yields_without_changing_the_route() {
    use sim::navigation::{Journey, Leg, NavBase, NavGrid, RoadNet};
    use sim::world::WorldGeometry;
    for bearing in [0.0f64, 30.0, 45.0].map(f64::to_radians) {
        let setup = scenario(street(bearing, true), json!([]));
        let world = WorldGeometry::new(&setup.map, &setup.rules);
        let base = std::sync::Arc::new(NavBase::build(
            &world,
            world.props(),
            setup.rules.physics.soldier_radius_m,
        ));
        let roads = RoadNet::build(&world);
        let mobility = sim::units::mobility(setup.rules.catalog.by_id("tank"), &setup.rules);
        let from = at(bearing, -END_M, 0.0);
        let to = at(bearing, END_M, 0.0);
        let leg = Leg {
            from: v2(from[0], from[1]),
            goal: v2(to[0], to[1]),
            m: &mobility,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        };
        let grid = NavGrid::new(base.clone());
        let mut incremental = Journey::new(&grid, &roads, None, leg, &setup.rules.navigation);
        for _ in 0..100_000 {
            let spent = incremental.advance(&grid, &roads, 1);
            assert!(
                spent <= 1 + sim::navigation::LARGEST_STEP,
                "{spent} work in a one-work advance"
            );
            if incremental.plan().is_some() {
                break;
            }
        }
        let together =
            sim::navigation::plan(&NavGrid::new(base), &roads, leg, &setup.rules.navigation).0;
        assert!(matches!(
            incremental.plan(),
            Some(sim::navigation::Plan::Route(_))
        ));
        assert_eq!(incremental.plan().unwrap(), &together);
    }
}

/// Many small known bodies share a roadside bucket. Looking past them
/// still yields to the tick allowance; spreading work changes no route.
#[test]
fn a_dense_roadside_body_query_yields_without_changing_the_route() {
    use sim::navigation::{Journey, Leg, NavBase, NavGrid, RoadNet};
    use sim::world::WorldGeometry;
    let props:Vec<_>=(0..2048).map(|k|json!({"kind":"parked_car","center":[288.0+(k%64) as f64*0.45,121.0+(k/64) as f64*0.125],"yaw":0,"half_extents":[0.02,0.02,0.5]})).collect();
    let setup = scenario(
        json!({"size":[1000,240],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":props,"surfaces":[{"kind":"road","shape":{"kind":"stroke","points":[[100,120],[700,120]],"width_m":7}}]}),
        json!([]),
    );
    let world = WorldGeometry::new(&setup.map, &setup.rules);
    let base = std::sync::Arc::new(NavBase::build(
        &world,
        world.props(),
        setup.rules.physics.soldier_radius_m,
    ));
    let roads = RoadNet::build(&world);
    let mobility = sim::units::mobility(setup.rules.catalog.by_id("supply"), &setup.rules);
    let avoid = [sim::math::Obb2 {
        center: v2(200.0, 20.0),
        yaw: 0.0,
        half: v2(2.0, 1.0),
    }];
    let leg = Leg {
        from: v2(100.0, 120.0),
        goal: v2(700.0, 120.0),
        m: &mobility,
        policy: RoutePolicy::Fastest,
        avoid: &avoid,
    };
    let grid = NavGrid::new(base.clone());
    let mut job = Journey::new(&grid, &roads, None, leg, &setup.rules.navigation);
    for _ in 0..1_000_000 {
        let spent = job.advance(&grid, &roads, 1);
        assert!(
            spent <= 1 + sim::navigation::LARGEST_STEP,
            "{spent} work in one advance"
        );
        if job.plan().is_some() {
            break;
        }
    }
    let together =
        sim::navigation::plan(&NavGrid::new(base), &roads, leg, &setup.rules.navigation).0;
    assert!(matches!(job.plan(), Some(sim::navigation::Plan::Route(_))));
    assert_eq!(job.plan().unwrap(), &together);
}

/// The moment: a wide truck is sent down the parked street to where a car
/// stands abandoned askew in the middle of it. It cannot stand there nor
/// squeeze past the car, and the nearest room beyond the car is a drive
/// round the block. It pulls up short of the car on its way: a detour may
/// gain it ground, at most a fifth of the way it adds.
#[test]
fn a_truck_sent_onto_an_abandoned_car_stops_short_of_it_rather_than_loop_round() {
    stops_short_of_the_wreck(None);
}

/// With no detour worth any ground, it stops short all the more.
#[test]
fn with_no_detour_worth_its_ground_a_truck_stops_short() {
    stops_short_of_the_wreck(Some(0.0));
}

/// The wide truck sent onto the abandoned car, the detour ratio `ratio`
/// (the shipped one by default), pulls up short of the car on its way.
fn stops_short_of_the_wreck(ratio: Option<f64>) {
    let bearing = 0.0;
    // The abandoned car, half across the middle and turned off the street.
    let wreck = at(bearing, 20.0, 0.3);
    let mut map = street(bearing, true);
    map["props"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "parked_car",
        "center": wreck, "yaw": bearing + 0.45, "half_extents": [CAR[0], CAR[1], 0.75] }));
    // A truck at the wheeled limits: as wide, as long and as wide-turning as
    // any wheeled hull a battle may field.
    let mut rules = common::scenario_rules();
    sim::fixtures::with_units_at_limits(&mut rules);
    if let Some(ratio) = ratio {
        rules["navigation"]["stop_short_detour_ratio"] = json!(ratio);
    }
    let from = at(bearing, -200.0, 0.0);
    let units =
        json!([{ "side": "blue", "kind": "limit_wheeled", "position": from, "yaw": bearing }]);
    let setup = serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    // Sent just past the car's middle: the truck cannot stand there, nor
    // get past the car on this street.
    send(&mut b, 1, 0, at(bearing, 22.0, 0.0));
    let hz = b.rules().tick_hz as u64;
    let (mut furthest, mut driven, mut last) = (f64::NEG_INFINITY, 0.0, v2(from[0], from[1]));
    for _ in 0..300 * hz {
        b.step();
        let p = own(&b, 0).position;
        let here = v2(p[0], p[1]);
        driven += (here - last).length();
        last = here;
        furthest = furthest.max(p[0] - MIDDLE[0]);
        if own(&b, 0).state == MoveState::Idle {
            break;
        }
    }
    let short = 20.0 - (last.x - MIDDLE[0]);
    assert!(
        furthest < 20.0,
        "it drove past the car, {:.1} m down the street, and came back",
        furthest
    );
    // Short of the car, by the way straight down the street to wherever it
    // found room: how far short is the standing-room rule's to say.
    let straight = 220.0 - short;
    assert!(
        short >= 0.0 && driven < straight + 5.0,
        "it stopped {short:.1} m short of the car after {driven:.0} m ({straight:.0} m straight)"
    );
}

/// The moment: a truck as long as any wheeled hull may be is sent to the
/// middle of the street, between parked cars on both kerbs. Its box, nose to tail
/// along the street, stands clear of them: it pulls up where it was sent, as
/// a driver would, rather than looking for a square wide enough to turn
/// round in. It can back out the way it came.
#[test]
fn a_long_truck_pulls_up_beside_parked_cars_where_it_was_sent() {
    let bearing = 0.0;
    let mut rules = common::scenario_rules();
    sim::fixtures::with_units_at_limits(&mut rules);
    let from = at(bearing, -200.0, 0.0);
    let units =
        json!([{ "side": "blue", "kind": "limit_wheeled", "position": from, "yaw": bearing }]);
    // Cars along both kerbs: the street is narrower than the truck is long.
    let mut map = street(bearing, true);
    for k in 0..4 {
        map["props"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "kind": "parked_car",
            "center": at(bearing, -50.0 + 4.7 * k as f64, -(PARKED_M + CAR[1])),
            "yaw": bearing, "half_extents": [CAR[0], CAR[1], 0.75] }));
    }
    let setup = serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    // Between the third run of parked cars and the cars across the street.
    let goal = at(bearing, -50.0 + 7.0, 0.0);
    send(&mut b, 1, 0, goal);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..120 * hz {
        b.step();
        if own(&b, 0).state == MoveState::Idle {
            break;
        }
    }
    let p = own(&b, 0).position;
    let off = (v2(p[0], p[1]) - v2(goal[0], goal[1])).length();
    assert!(
        off < 2.0,
        "it stopped {off:.1} m from where it was sent, at {p:?}"
    );
}
