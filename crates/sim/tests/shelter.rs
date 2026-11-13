//! What a body's columns do for the soldiers near it, read through test
//! rows of this file's own: a ruin that still blocks keeps covering the
//! soldier behind it; a body with no cover tier that stops no rounds never
//! shields anyone; a body that blocks no one and occludes nothing hides no
//! one and slows no one (L2, L3).
use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::cover;
use sim::ground::GroundLayer;
use sim::math::{v2, V2};

use crate::common;

/// The rows under test, each an ordinary body: a shed that falls to a
/// lower ruin which still blocks, a screen with no cover and no stop, and a
/// net that blocks, stops and hides nothing.
fn rows() -> Value {
    json!({ "props": {
        "test_shed": {
            "body": {"blocks": {"infantry": true, "vehicle": true}, "stops_rounds": true, "occludes": true, "weight_class": "immovable", "cover_tier": "heavy", "hp": 100},
            "destroyed": {"into": {"prop": "test_shed_ruin", "height_m": 1.0}},
            "appearance": {"status": "systems_only", "stand_in_tint": [0.5, 0.5, 0.5], "drawn_by": "test_shed"}
        },
        "test_shed_ruin": {
            "body": {"blocks": {"infantry": true, "vehicle": true}, "stops_rounds": false, "occludes": false, "weight_class": "immovable", "cover_tier": "light"},
            "appearance": {"status": "systems_only", "stand_in_tint": [0.3, 0.3, 0.3], "drawn_by": "test_shed_ruin"}
        },
        "test_screen": {
            "body": {"blocks": {"infantry": true, "vehicle": true}, "stops_rounds": false, "occludes": false, "weight_class": "light"},
            "appearance": {"status": "systems_only", "stand_in_tint": [0.4, 0.45, 0.3], "drawn_by": "test_screen"}
        },
        "test_net": {
            "body": {"blocks": {"infantry": false, "vehicle": false}, "stops_rounds": false, "occludes": false, "weight_class": "light"},
            "appearance": {"status": "systems_only", "stand_in_tint": [0.35, 0.4, 0.25], "drawn_by": "test_net"}
        }
    }})
}

fn rules_json() -> Value {
    let mut rules = common::game();
    rules["catalog"].as_array_mut().unwrap().push(rows());
    rules
}

fn rules() -> Rules {
    serde_json::from_value(rules_json()).unwrap()
}

fn prop(kind: &str, center: [f64; 2], half: [f64; 3]) -> Value {
    json!({ "kind": kind, "center": center, "yaw": 0, "half_extents": half })
}

fn setup(props: Value, units: Value, events: Value) -> ScenarioDefinition {
    let map = json!({ "size": [400, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props });
    serde_json::from_value(json!({
        "map": map, "rules": rules_json(), "units": units, "events": events, "scripts": []
    }))
    .unwrap()
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

#[test]
fn a_ruined_prop_keeps_covering_the_soldier_behind_it() {
    let r = rules();
    let shed = [200.0, 100.0];
    let bursts: Vec<Value> = (1..=4)
        .map(|tick| json!({ "tick": tick, "burst": { "point": shed, "weapon": "tank_he" } }))
        .collect();
    let mut b = Battle::new(
        &setup(
            json!([prop("test_shed", shed, [3.0, 3.0, 2.0])]),
            json!([{ "side": "red", "kind": "test_rifle", "position": [380, 180], "engagement": "return_fire_only" }]),
            json!(bursts),
        ),
        1,
    );
    // Just west of the shed, shot at from the east: behind it.
    let behind = v2(196.6, 100.0);
    let threat = v2(320.0, 100.0);
    let tier = |b: &Battle| {
        let ground = GroundLayer::new(b.world().width(), b.world().depth(), &r.ground);
        cover::at(b.world(), &ground, &[], &r, behind, threat)
    };
    let standing = tier(&b);
    assert!(standing.is_some(), "the standing shed covers him");
    run(&mut b, 6);
    let ruin = b
        .world()
        .props()
        .find(|p| r.catalog.props().id(p.kind) == "test_shed_ruin")
        .expect("the shed fell to its ruin");
    assert!(ruin.half.z < 2.0, "the ruin stands lower than the shed");
    let ruined = tier(&b);
    assert_eq!(
        ruined, ruin.body.cover_tier,
        "the ruin covers by its own row"
    );
    assert!(ruined.is_some() && ruined < standing);
}

/// Red's rifles on a blue squad for twenty seconds, with `screen` (or nothing)
/// a metre in front of blue: each blue soldier's health, tick by tick.
fn volleys_at_blue(screen: Option<&str>) -> Vec<Vec<f64>> {
    let props = screen.map_or(json!([]), |kind| {
        json!([prop(kind, [146.5, 100.0], [0.3, 12.0, 1.5])])
    });
    let mut b = Battle::new(
        &setup(
            props,
            json!([
                { "side": "blue", "kind": "test_rifle", "position": [140, 100], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": [205, 100] },
            ]),
            json!([]),
        ),
        3,
    );
    (0..b.rules().tick_hz as u64 * 20)
        .map(|_| {
            b.step();
            b.unit(UnitId(0))
                .unwrap()
                .members
                .iter()
                .map(|m| m.hp)
                .collect()
        })
        .collect()
}

#[test]
fn a_body_with_no_cover_and_no_stop_never_shields() {
    let open = volleys_at_blue(None);
    assert!(
        open.last() != open.first(),
        "red's rifles hurt blue in the open"
    );
    assert_eq!(
        volleys_at_blue(Some("test_screen")),
        open,
        "the screen in front of blue changes no round's spread or reach"
    );
}

/// A blue squad walking east across `over` (or open ground) toward a red
/// one watching: the tick red first identified it, and where blue stood
/// each second.
fn walk_past_red(over: Option<&str>) -> (u64, Vec<V2>) {
    let props = over.map_or(json!([]), |kind| {
        json!([prop(kind, [150.0, 100.0], [10.0, 10.0, 2.5])])
    });
    let mut b = Battle::new(
        &setup(
            props,
            json!([
                { "side": "blue", "kind": "test_rifle", "position": [110, 100], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_rifle", "position": [280, 100], "engagement": "return_fire_only" },
            ]),
            json!([]),
        ),
        1,
    );
    common::order(
        &mut b,
        Side::Blue,
        1,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [190.0, 100.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    );
    let hz = b.rules().tick_hz as u64;
    let mut seen = None;
    let mut path = Vec::new();
    for tick in 1..=hz * 40 {
        b.step();
        if seen.is_none() && !b.observe(Side::Red).identified.is_empty() {
            seen = Some(tick);
        }
        if tick % hz == 0 {
            path.push(b.unit(UnitId(0)).unwrap().position.xy());
        }
    }
    (seen.expect("red saw blue"), path)
}

#[test]
fn a_body_that_blocks_no_one_and_occludes_nothing_hides_nothing() {
    let open = walk_past_red(None);
    assert!(
        open.1.last().unwrap().x > 180.0,
        "blue walked across the net's ground, which stands between it and red"
    );
    assert_eq!(
        walk_past_red(Some("test_net")),
        open,
        "blue walks and is seen as if the net were not there"
    );
}
