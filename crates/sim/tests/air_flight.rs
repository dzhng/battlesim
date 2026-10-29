//! Flight (D4, D6, D7, D21, D24): an aircraft holds cruise height over the ground,
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

/// Where every flight here is ordered.
const GOAL: [f64; 2] = [500.0, 200.0];

/// Fly the helicopter (unit 0, at [100, 200]) to `GOAL`; each tick's
/// position, then whether it arrived.
fn fly(map: Value) -> (Battle, Vec<sim::math::V3>) {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map,
        json!([{ "side": "blue", "kind": "test_heli", "position": [100, 200] }]),
    );
    assert_eq!(b.accept(order(&[0], GOAL)).error, None);
    let mut track = Vec::new();
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
        track.push(b.unit(UnitId(0)).unwrap().position);
    }
    (b, track)
}

fn arrived(b: &Battle) -> f64 {
    (b.unit(UnitId(0)).unwrap().position.xy() - v2(GOAL[0], GOAL[1])).length()
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
fn cruises_at_its_height_to_its_goal() {
    let (b, track) = fly(map(json!([]), json!([])));
    assert!(arrived(&b) < 1.0, "{:.1} m short", arrived(&b));
    let cruise = b.rules().air.low_air.cruise_agl_m;
    for p in &track {
        assert!((p.z - cruise).abs() < 1e-6, "left cruise height: {p:?}");
    }
    let fastest = track
        .windows(2)
        .map(|w| (w[1].xy() - w[0].xy()).length())
        .fold(0.0, f64::max)
        * b.rules().tick_hz as f64;
    // Never past its cruise speed; 400 m is too short to reach it at a
    // helicopter's acceleration, but most of the way.
    let top = cruise_mps(&b);
    assert!(
        fastest <= top + 1e-6 && fastest > 0.6 * top,
        "flew at {fastest:.1} m/s of {top:.1}"
    );
}

/// The helicopter's cruise speed, from its own flight data.
fn cruise_mps(b: &Battle) -> f64 {
    let sim::units::Motion::Air(flight) = b.unit(UnitId(0)).unwrap().motion else {
        panic!("unit 0 is no aircraft");
    };
    flight.cruise_mps
}

#[test]
fn pops_over_a_roof_and_comes_back_down() {
    // A roof whose clearance stands above cruise height.
    let (b, track) = fly(map(json!([block(0, [300.0, 200.0], 6.0, 25.0)]), json!([])));
    assert!(arrived(&b) < 1.0);
    let air = &b.rules().air;
    let over = height_near(&track, 300.0, 6.0);
    assert!(
        (over - (25.0 + air.rotor.clearance_m)).abs() < 0.5,
        "over the 25 m roof it flew at {over:.1} m"
    );
    assert!(
        (track.last().unwrap().z - air.low_air.cruise_agl_m).abs() < 0.5,
        "it came back down"
    );
}

#[test]
fn clears_forest_canopy_and_fells_nothing() {
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
    let air = &b.rules().air;
    assert!(
        (over - (canopy + air.rotor.clearance_m).max(air.low_air.cruise_agl_m)).abs() < 0.5,
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
    let margin = sim::navigation::air::ROUTING_MARGIN_M;
    // The 4 m air grid rounds a corner by at most one cell; the authored
    // hull margin remains the routing bound, with that grid quantisation.
    assert!(
        clear + 1.0 > margin,
        "it passed {clear:.1} m from the tower"
    );
    let ceiling = b.rules().air.rotor.ceiling_agl_m;
    assert!(track.iter().all(|p| p.z <= ceiling + 1e-9));
}

/// A goal walled in on every side by bodies taller than the ceiling: it
/// flies round to the open air nearest the goal and waits there, never
/// through a wall.
#[test]
fn stops_short_of_a_goal_walled_in_by_towers() {
    // Four walls 61 m tall, 4 m thick, round a courtyard 100 m across.
    let wall = |id: u32, center: [f64; 2], half: [f64; 2]| {
        json!({ "id": id, "kind": "ruin", "center": center, "yaw": 0,
            "half_extents": [half[0], half[1], 30.5] })
    };
    let (gx, gy, ring) = (GOAL[0], GOAL[1], 50.0);
    let walls = json!([
        wall(0, [gx - ring, gy], [2.0, ring + 2.0]),
        wall(1, [gx + ring, gy], [2.0, ring + 2.0]),
        wall(2, [gx, gy - ring], [ring + 2.0, 2.0]),
        wall(3, [gx, gy + ring], [ring + 2.0, 2.0]),
    ]);
    let (b, track) = fly(map(walls, json!([])));
    for p in &track {
        let inside = (b.world().props_near(p.xy(), 0.0).into_iter())
            .any(|w| w.footprint().contains(p.xy(), 0.0));
        assert!(!inside, "it flew into a wall at {p:?}");
    }
    // It holds just outside a wall, across it from the courtyard, where the
    // goal is nearest: not at a corner, nor off in the open.
    let rest = b.unit(UnitId(0)).unwrap().position.xy();
    let (out, along) = {
        let (dx, dy) = ((rest.x - gx).abs(), (rest.y - gy).abs());
        (dx.max(dy) - ring - 2.0, dx.min(dy))
    };
    let margin = sim::navigation::air::ROUTING_MARGIN_M;
    assert!(
        out > 0.0 && out < 2.0 * margin && along < ring / 2.0,
        "it came to rest at {rest:?}"
    );
    let last = track[track.len() - 2].xy();
    assert!((rest - last).length() < 1e-3, "still flying at {rest:?}");
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
            arrived(&b) < 0.05,
            "tower {offset}: {:.2} m short",
            arrived(&b)
        );
        // It never turns back on its way: a skimmed corner once turned it
        // back 100 m. Past the goal it runs on by no more than its designed
        // overshoot, then settles back.
        let settle = b.rules().air.rotor.settle_max_m;
        let mut furthest = f64::NEG_INFINITY;
        for p in &track {
            furthest = furthest.max(p.x);
            assert!(
                p.x > furthest - settle - 2.0,
                "tower {offset}: turned back from x {furthest:.2} to {:.2}",
                p.x
            );
            let d = (p.xy() - v2(GOAL[0], GOAL[1])).length();
            // Its overshoot, and a little sideways carry from the turn onto
            // its last leg.
            assert!(
                p.x < GOAL[0] || d <= settle + 0.5,
                "tower {offset}: ran {d:.2} m past its goal"
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
        player: None,
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

/// From a hover it gathers speed over seconds, not at once: no faster than
/// its cruise over the air's acceleration time.
#[test]
fn gathers_speed_gently_from_a_hover() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map(json!([]), json!([])),
        json!([{ "side": "blue", "kind": "test_heli", "position": [100, 200] }]),
    );
    assert_eq!(b.accept(order(&[0], GOAL)).error, None);
    let hz = b.rules().tick_hz;
    let cruise = cruise_mps(&b);
    let accel = cruise / b.rules().air.rotor.acceleration_s;
    let mut last = 0.0;
    for t in 1..=4 * hz {
        b.step();
        let speed = b.unit(UnitId(0)).unwrap().drive_speed_mps;
        assert!(
            speed - last <= accel / hz as f64 + 1e-9,
            "jumped {:.2} m/s in a tick",
            speed - last
        );
        last = speed;
        if t == 2 * hz {
            assert!(speed < cruise * 0.5, "{speed:.1} m/s after 2 s");
        }
    }
}

/// Braking for its goal it flares a beat late: it runs a little past, then
/// eases back onto the point without turning round, and settles there.
#[test]
fn runs_past_its_goal_and_settles_back_without_turning() {
    let (b, track) = fly(map(json!([]), json!([])));
    let air = &b.rules().air;
    let past = track
        .iter()
        .map(|p| p.x - GOAL[0])
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(past > 0.5, "it stopped dead: {past:.2} m past");
    assert!(
        past <= air.rotor.settle_max_m + 0.05,
        "ran {past:.2} m past"
    );
    let heli = b.unit(UnitId(0)).unwrap();
    assert!(arrived(&b) < 0.05, "settled {:.2} m off", arrived(&b));
    assert!(
        sim::math::wrap_angle(heli.yaw).abs() < 0.1,
        "turned round: yaw {:.2}",
        heli.yaw
    );
    assert_eq!(heli.orders.len(), 0, "the move completes once settled");
}

/// A short hop overshoots by its share of the hop, not the long flight's.
#[test]
fn a_short_hop_overshoots_little() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        map(json!([]), json!([])),
        json!([{ "side": "blue", "kind": "test_heli", "position": [470, 200] }]),
    );
    assert_eq!(b.accept(order(&[0], GOAL)).error, None);
    let mut past = f64::NEG_INFINITY;
    for _ in 0..20 * b.rules().tick_hz {
        b.step();
        past = past.max(b.unit(UnitId(0)).unwrap().position.x - GOAL[0]);
    }
    let air = &b.rules().air;
    assert!(
        past <= air.rotor.settle_share * 30.0 + 0.05,
        "ran {past:.2} m past a 30 m hop"
    );
    assert!(arrived(&b) < 0.05);
}
