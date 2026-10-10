//! Flight (D4, D6, D7, D21, D24): an aircraft holds 20 m over the ground,
//! climbs early to clear anything taller by 10 m, never above 40 m, flies
//! around bodies taller than 30 m, and parts from other aircraft.

use super::air::{battle_with, fixture, order};
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, V2};

/// A 600 × 400 map with `props` and `forests`.
fn map(props: Value, forests: Value) -> Value {
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": props, "forests": forests })
}

/// A block `half` wide and `height` tall at `center`.
fn block(id: u32, center: [f64; 2], half: f64, height: f64) -> Value {
    json!({ "id": id, "kind": "ruin", "center": center, "yaw": 0,
        "half_extents": [half, half, height / 2.0] })
}

/// Fly the helicopter (unit 0, at [100, 200]) to [500, 200]; each tick's
/// position, then whether it arrived.
fn fly(map: Value) -> (Battle, Vec<sim::math::V3>) {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map,
        json!([{ "side": "blue", "kind": "test_heli", "position": [100, 200] }]),
    );
    assert_eq!(b.accept(order(&[0], [500.0, 200.0])).error, None);
    let mut track = Vec::new();
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
        track.push(b.unit(UnitId(0)).unwrap().position);
    }
    (b, track)
}

fn arrived(b: &Battle) -> f64 {
    (b.unit(UnitId(0)).unwrap().position.xy() - v2(500.0, 200.0)).length()
}

/// The highest it flew within `reach` of `x` along the track.
fn height_near(track: &[sim::math::V3], x: f64, reach: f64) -> f64 {
    track
        .iter()
        .filter(|p| (p.x - x).abs() <= reach)
        .map(|p| p.z)
        .fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn cruises_at_20m_to_its_goal() {
    let (b, track) = fly(map(json!([]), json!([])));
    assert!(arrived(&b) < 1.0, "{:.1} m short", arrived(&b));
    for p in &track {
        assert!((p.z - 20.0).abs() < 1e-6, "left cruise height: {p:?}");
    }
    let fastest = track
        .windows(2)
        .map(|w| (w[1].xy() - w[0].xy()).length())
        .fold(0.0, f64::max)
        * b.rules().tick_hz as f64;
    assert!(
        (fastest - 220.0 / 3.6).abs() < 0.5,
        "cruise {fastest:.1} m/s"
    );
}

#[test]
fn pops_over_a_roof_and_comes_back_down() {
    let (b, track) = fly(map(json!([block(0, [300.0, 200.0], 6.0, 15.0)]), json!([])));
    assert!(arrived(&b) < 1.0);
    let over = height_near(&track, 300.0, 6.0);
    assert!(
        (over - 25.0).abs() < 0.5,
        "over the 15 m roof it flew at {over:.1} m"
    );
    assert!(
        (track.last().unwrap().z - 20.0).abs() < 0.5,
        "it came back down"
    );
}

#[test]
fn rises_2m_over_forest_canopy_and_fells_nothing() {
    let forest = json!([{ "shape": { "kind": "polygon",
        "ring": [[260, 150], [340, 150], [340, 250], [260, 250]] } }]);
    let fixture = fixture();
    let trees = |b: &Battle| b.world().props().count();
    let before = trees(&battle_with(
        &fixture,
        map(json!([]), forest.clone()),
        json!([]),
    ));
    let (b, track) = fly(map(json!([]), forest));
    let over = height_near(&track, 300.0, 20.0);
    let canopy = b.rules().forests.rule.canopy_height_m;
    assert!(
        (over - (canopy + 10.0)).abs() < 0.5,
        "over the forest it flew at {over:.1} m"
    );
    assert_eq!(trees(&b), before, "flying over felled trees");
}

#[test]
fn routes_around_a_tower_over_30m_and_never_passes_the_ceiling() {
    let tower = block(0, [300.0, 200.0], 10.0, 61.0);
    let (b, track) = fly(map(json!([tower]), json!([])));
    assert!(arrived(&b) < 1.0, "{:.1} m short", arrived(&b));
    let clear = track
        .iter()
        .map(|p| (p.x - 300.0).abs().max((p.y - 200.0).abs()) - 10.0)
        .fold(f64::INFINITY, f64::min);
    // Its whole airframe, not just its centre, stays clear of the walls.
    let half_length = b.unit(UnitId(0)).unwrap().hull.unwrap().x;
    assert!(clear > half_length, "it passed {clear:.1} m from the tower");
    let ceiling = b.rules().air.ceiling_agl_m;
    assert!(track.iter().all(|p| p.z <= ceiling + 1e-9));
}

/// Wherever the tower stands across its line, the helicopter flies past it
/// and on to its goal without turning back for a corner it skimmed by at
/// cruise, or overshooting a goal too near that corner to stop for.
#[test]
fn flies_on_past_a_tower_without_turning_back() {
    for offset in 0..12 {
        let tower = block(0, [300.0, 200.0 + 2.5 * offset as f64], 14.5, 61.0);
        let (b, track) = fly(map(json!([tower]), json!([])));
        assert!(
            arrived(&b) < 1.0,
            "tower {offset}: {:.1} m short",
            arrived(&b)
        );
        // A grid corner a cell short of the goal may still settle it a
        // metre past and back; a skimmed corner turned it back 100 m.
        let mut furthest = f64::NEG_INFINITY;
        for p in &track {
            furthest = furthest.max(p.x);
            assert!(
                p.x > furthest - 2.0,
                "tower {offset}: turned back from x {furthest:.1} to {:.1}",
                p.x
            );
        }
    }
}

#[test]
fn turns_on_the_spot_to_an_ordered_facing() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map(json!([]), json!([])),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [300.0, 200.0],
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: Some(std::f64::consts::PI),
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    for _ in 0..5 * b.rules().tick_hz {
        b.step();
    }
    let heli = b.unit(UnitId(0)).unwrap();
    assert!((heli.position.xy() - v2(300.0, 200.0)).length() < 1e-6);
    let off = sim::math::wrap_angle(heli.yaw - std::f64::consts::PI).abs();
    assert!(off < 1e-6, "facing off by {off}");
}

#[test]
fn overlapping_helicopters_part_softly() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map(json!([]), json!([])),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
            { "side": "blue", "kind": "test_heli", "position": [301, 200] },
        ]),
    );
    let gap = |b: &Battle| -> f64 {
        let at = |id| -> V2 { b.unit(UnitId(id)).unwrap().position.xy() };
        (at(0) - at(1)).length()
    };
    let first = {
        b.step();
        gap(&b)
    };
    // Softly: no jump in one tick.
    assert!(first < 1.5, "jumped apart to {first:.2} m");
    for _ in 0..20 * b.rules().tick_hz {
        b.step();
    }
    let apart = b.rules().air.separation_m;
    assert!(gap(&b) > apart * 0.9, "still {:.1} m apart", gap(&b));
}
