//! A ground mover's speed over the ground, the one speed movement and
//! planning both read. A road keeps road speed whatever lies under it. Off
//! it, a forest or a swamp slows a mover by its own factor, a swamp by the
//! mover's drive (foot, wheeled, tracked), and where both lie the slower
//! applies once. A swamp's depth never shapes speed; slopes do; aircraft
//! never feel the ground. Every factor here is a test row set wide apart,
//! never the shipped numbers.
use contract::command::RoutePolicy;
use contract::ids::UnitId;
use contract::scenario::Rules;
use serde_json::json;
use sim::battle::Battle;
use sim::math::{v2, V2};
use sim::navigation::{Footing, Leg, Mobility, NavBase, NavGrid, Plan, RoadNet};
use sim::world::WorldGeometry;
use std::sync::Arc;

use crate::common;

/// The test's swamp factors, by locomotion.
struct Swamp {
    foot: f64,
    wheeled: f64,
    tracked: f64,
}
const SWAMP: Swamp = Swamp {
    foot: 0.5,
    wheeled: 0.2,
    tracked: 0.8,
};
/// Between the swamp's factors: slower than a tracked hull's swamp, faster
/// than a wheeled one's, so an overlap is judged both ways round.
const FOREST: f64 = 0.6;

/// The test's swamp factors in `rules`, and with `forest` its forest's too.
fn slow_ground(rules: &mut Rules, forest: bool) {
    let s = &mut rules.movement.speed_factors;
    for (f, swamp) in [&mut s.foot, &mut s.wheeled, &mut s.tracked]
        .into_iter()
        .zip([SWAMP.foot, SWAMP.wheeled, SWAMP.tracked])
    {
        f.swamp = swamp;
        if forest {
            f.forest = FOREST;
        }
    }
}

fn rules() -> Rules {
    let mut rules = common::rules();
    slow_ground(&mut rules, true);
    // One trunk to a wood, so a planned line through it stays open.
    rules.forests.rule.trunk_spacing_m = 500.0;
    rules
}

fn polygon(ring: [[f64; 2]; 4]) -> String {
    format!(
        r#"{{"kind":"polygon","ring":{}}}"#,
        serde_json::to_string(&ring).unwrap()
    )
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [[f64; 2]; 4] {
    [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

/// A flat `width` × 300 m map with a 0.6 m swamp over `swamp`, plus extra
/// map fields (leading comma).
fn map_json(width: f64, swamp: [[f64; 2]; 4], extra: &str) -> String {
    format!(
        r#"{{"size":[{width},300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"swamps":[{{"shape":{},"depth_m":0.6}}]{extra}}}"#,
        polygon(swamp)
    )
}

/// The test swamp (x 100 to 200), a wood over its east half and on east
/// to x 250, and a road across it at y 150.
fn world() -> WorldGeometry {
    let extra = format!(
        r#","forests":[{{"shape":{}}}],"surfaces":[{{"kind":"road","shape":{{"kind":"stroke","points":[[0,150],[400,150]],"width_m":8}}}}]"#,
        polygon(rect(150.0, 60.0, 250.0, 240.0))
    );
    WorldGeometry::new(
        &common::physical_map(&map_json(400.0, rect(100.0, 60.0, 200.0, 240.0), &extra)),
        &rules(),
    )
}

fn mobility(kind: &str, rules: &Rules) -> Mobility {
    sim::units::ground_mobility(rules.catalog.by_id(kind), rules).unwrap()
}

fn speed(w: &WorldGeometry, m: &Mobility, x: f64, y: f64) -> f64 {
    m.speed(w.surface_at(x, y).unwrap().footing())
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs()
}

#[test]
fn each_drive_is_slowed_by_its_own_factor_and_a_wood_over_it_once() {
    let (w, rules) = (world(), rules());
    for (kind, factor) in [
        ("test_rifle", SWAMP.foot),
        ("test_jeep", SWAMP.wheeled),
        ("test_tank", SWAMP.tracked),
    ] {
        let m = mobility(kind, &rules);
        assert_eq!(
            speed(&w, &m, 50.0, 100.0),
            m.off_road_mps,
            "{kind} on dry ground"
        );
        assert!(w.surface_at(125.0, 100.0).unwrap().swamp);
        assert!(
            close(speed(&w, &m, 125.0, 100.0), m.off_road_mps * factor),
            "{kind} in the swamp"
        );
        assert!(
            close(speed(&w, &m, 225.0, 100.0), m.off_road_mps * FOREST),
            "{kind} in the wood"
        );
        let both = speed(&w, &m, 175.0, 100.0);
        assert!(
            close(both, m.off_road_mps * factor.min(FOREST)),
            "{kind} in both: {both}"
        );
    }
}

#[test]
fn a_road_across_a_swamp_keeps_road_speed() {
    let (w, rules) = (world(), rules());
    for kind in ["test_rifle", "test_jeep", "test_tank"] {
        let m = mobility(kind, &rules);
        let road = speed(&w, &m, 50.0, 150.0);
        assert!(road > m.off_road_mps * SWAMP.tracked, "{kind} on the road");
        for x in [125.0, 175.0] {
            assert_eq!(
                speed(&w, &m, x, 150.0),
                road,
                "{kind} on the causeway at {x}"
            );
        }
        // Even were a road's ground marked wet, road speed would win.
        let causeway = w.surface_at(125.0, 150.0).unwrap().footing();
        let wet = Footing {
            swamp: true,
            forest: true,
            ..causeway
        };
        assert_eq!(m.speed(wet), road, "{kind}: road speed wins");
    }
}

/// Speed comes from the swamp, not its depth: on the easing edge, where the
/// bed is shallower and slopes, a mover is as slow as in the middle once
/// the slope's own slowdown is divided out.
#[test]
fn depth_never_shapes_speed() {
    let (w, rules) = (world(), rules());
    let m = mobility("test_jeep", &rules);
    let middle = w.surface_at(125.0, 100.0).unwrap();
    for x in [101.0, 103.5, 106.0, 109.0] {
        let rim = w.surface_at(x, 100.0).unwrap();
        assert!(
            rim.swamp && rim.z > middle.z && rim.slope_deg > 0.0,
            "edge at {x}"
        );
        let flat = speed(&w, &m, x, 100.0) / m.speed_factors.slope.factor(rim.slope_deg);
        assert!(close(flat, speed(&w, &m, 125.0, 100.0)), "at {x}");
    }
}

/// Planning and movement see the same ground: what a planned step costs on
/// a cell is what moving over its middle takes, in and round the swamp,
/// the wood, both, and on the causeway.
#[test]
fn planning_costs_each_cell_at_the_speed_movement_takes_there() {
    let (w, rules) = (world(), rules());
    let grid = NavGrid::new(Arc::new(NavBase::build(&w, w.props(), 0.3)));
    for kind in ["test_rifle", "test_jeep"] {
        let m = mobility(kind, &rules);
        let mut compared = 0;
        for j in (40..125).step_by(3) {
            for i in (20..135).step_by(3) {
                let c = v2(i as f64 * 2.0 + 1.0, j as f64 * 2.0 + 1.0);
                let planned = grid.route_time(c, &[c + v2(0.4, 0.0)], &m) / 0.4;
                if planned.is_finite() {
                    let moving = speed(&w, &m, c.x, c.y);
                    assert!(close(planned, 1.0 / moving), "{kind} at {c:?}");
                    compared += 1;
                }
            }
        }
        assert!(compared > 900, "{kind}: {compared} cells compared");
    }
}

/// Seconds a vehicle takes to drive `from` to `to` (x) along y 120 across
/// the swamp at x 100 to 200 on a 600 m map, from a standing start at x 20.
fn crossing(kind: &str, from: f64, to: f64) -> f64 {
    let mut setup = common::scenario_with(
        &map_json(600.0, rect(100.0, 60.0, 200.0, 240.0), ""),
        json!([{ "side": "blue", "kind": kind, "position": [20, 120], "yaw": 0.0 }]),
        json!([]),
        json!([{ "tick": 1, "side": "blue",
            "order": { "kind": "move", "units": [0], "gesture": 1, "goal": [580, 120], "route": "shortest" } }]),
    );
    slow_ground(&mut setup.rules, false);
    let mut b = Battle::new(&setup, 1);
    let dt = 1.0 / common::tick_hz() as f64;
    let (mut start, mut ticks) = (None, 0u32);
    for tick in 0..240 * common::tick_hz() {
        b.step();
        let x = b.unit(UnitId(0)).unwrap().position.x;
        if start.is_none() && x >= from {
            start = Some(tick);
        }
        if x >= to {
            ticks = tick - start.unwrap();
            break;
        }
    }
    assert!(ticks > 0, "{kind} never reached {to}");
    ticks as f64 * dt
}

/// Driven for real: a vehicle's pace across the middle of the swamp is its
/// dry pace times its own drive's factor.
#[test]
fn a_vehicle_wades_the_swamp_at_its_drives_factor() {
    for (kind, factor) in [("test_jeep", SWAMP.wheeled), ("test_tank", SWAMP.tracked)] {
        let wet = crossing(kind, 130.0, 170.0);
        let dry = crossing(kind, 330.0, 370.0);
        let ratio = dry / wet;
        assert!(
            (ratio - factor).abs() < 0.03 * factor,
            "{kind}: {ratio} for {factor}"
        );
    }
}

/// Aircraft never touch the ground's rules: a helicopter crosses the swamp
/// exactly as it crosses dry land.
#[test]
fn aircraft_ignore_swamps() {
    let track = |swamps: bool| {
        let map = if swamps {
            map_json(600.0, rect(100.0, 60.0, 200.0, 240.0), "")
        } else {
            r#"{"size":[600,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#.into()
        };
        let mut setup = common::scenario_with(
            &map,
            json!([{ "side": "blue", "kind": "test_heli", "position": [20, 120], "yaw": 0.0 }]),
            json!([]),
            json!([{ "tick": 1, "side": "blue",
                "order": { "kind": "move", "units": [0], "gesture": 1, "goal": [580, 120], "route": "shortest" } }]),
        );
        slow_ground(&mut setup.rules, false);
        let mut b = Battle::new(&setup, 1);
        (0..40 * common::tick_hz())
            .map(|_| {
                b.step();
                let p = b.unit(UnitId(0)).unwrap().position;
                [p.x, p.y]
            })
            .collect::<Vec<_>>()
    };
    let wet = track(true);
    assert!(wet.last().unwrap()[0] > 250.0, "the helicopter crossed");
    assert_eq!(wet, track(false));
}

/// Where dry ground round a swamp is faster, the planner takes it; where
/// wading is faster, it wades.
#[test]
fn the_planner_goes_round_a_swamp_when_dry_ground_is_faster() {
    // A swamp band from the south edge to y 200, open to the north.
    let w = WorldGeometry::new(
        &common::physical_map(&map_json(400.0, rect(150.0, 0.0, 250.0, 200.0), "")),
        &rules(),
    );
    let grid = NavGrid::new(Arc::new(NavBase::build(&w, w.props(), 0.3)));
    let roads = RoadNet::build(&w);
    let wades = |m: &Mobility| {
        let (from, goal) = (v2(50.0, 100.0), v2(350.0, 100.0));
        let leg = Leg {
            from,
            goal,
            m,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        };
        let Plan::Route(route) = sim::navigation::plan(&grid, &roads, leg, &rules().navigation).0
        else {
            panic!("no route")
        };
        let mut a: V2 = from;
        route.iter().any(|&b| {
            let wet = (0..=100).any(|k| {
                let p = a + (b - a) * (k as f64 / 100.0);
                w.surface_at(p.x, p.y).unwrap().swamp
            });
            a = b;
            wet
        })
    };
    let mut m = mobility("test_jeep", &rules());
    assert!(
        !wades(&m),
        "a wheeled hull at {} goes round",
        m.speed_factors.swamp
    );
    m.speed_factors.swamp = 0.95;
    assert!(wades(&m), "at 0.95 it wades straight across");
}

/// A swamp no mover can cross would freeze a unit in it and divide the
/// planner's estimate by zero, so a zero factor is refused by name.
#[test]
fn a_zero_swamp_factor_is_refused_when_the_rules_load() {
    let mut game = common::game();
    game["movement"]["speed_factors"]["wheeled"]["swamp"] = json!(0.0);
    let refused = serde_json::from_value::<Rules>(game).unwrap_err();
    assert!(
        refused.to_string().contains("speed_factors.wheeled.swamp"),
        "{refused}"
    );
}

/// An unknown movement key is refused, not silently ignored.
#[test]
fn the_movement_rules_refuse_an_unknown_key() {
    let mut game = common::game();
    game["movement"]["swamp_speed"] = json!(0.5);
    let refused = serde_json::from_value::<Rules>(game).unwrap_err();
    assert!(refused.to_string().contains("swamp_speed"), "{refused}");
}

/// A slope slows every mover continuously, to a floor near the cutoff.
#[test]
fn slopes_slow_a_mover_to_a_floor() {
    let m = mobility("test_tank", &rules());
    let flat = |slope_deg| Footing {
        paved: [None, None],
        forest: false,
        swamp: false,
        slope_deg,
        crater_fill: 0.0,
    };
    assert_eq!(m.speed(flat(0.0)), m.off_road_mps);
    assert!(close(m.speed(flat(25.0)), m.off_road_mps * 0.5));
    assert!(
        close(m.speed(flat(34.0)), m.off_road_mps * 0.35),
        "the floor near the cutoff"
    );
}
