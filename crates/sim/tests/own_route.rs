//! A soldier's own route always ends where it is going. A target within
//! the fine grid cell he stands in gives the route search no waypoints;
//! he then walks straight to it, as he does when no route is found.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;
use sim::math::V2;

fn soldier(b: &Battle) -> V2 {
    b.unit(UnitId(0))
        .unwrap()
        .member_positions()
        .next()
        .unwrap()
        .xy()
}

#[test]
fn a_soldier_whose_target_is_within_one_fine_cell_walks_straight_to_it() {
    // One soldier, and a search window of an odd number of fine cells: the
    // window is centred between him and his target, so a target this close
    // falls in his own cell.
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        json!({"body":{"squad":{"slots":["test_rifleman"]}}}),
    );
    rules["infantry_movement"]["window_m"] = json!(40.5);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [120, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [{"side":"blue","kind":"test_rifle","position":[60.0,60.0],"engagement":"return_fire_only"}],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let here = soldier(&b);
    let goal = [here.x + 0.02, here.y];
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
    });
    let spot = ack.placement.expect("placed").destinations[0].spots[0];
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
    }
    let at = soldier(&b);
    assert!(
        (at.x - spot[0]).hypot(at.y - spot[1]) < 0.05,
        "he stands on his spot {spot:?}, not {at:?}"
    );
}
