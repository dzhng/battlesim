//! An attack order closing on a target it cannot fire on yet: it heads for
//! where the target is, and works its way out again only once the target has
//! moved far for how far off it is.
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

use crate::common::{self, Commander};
use crate::weapons::{map, own, run, scenario_with};

#[test]
fn a_far_pursuit_follows_its_target_without_a_new_route_for_each_few_metres() {
    // Red walks 100 m across blue's front, 850 m off and out of rifle reach;
    // the scout keeps it in sight for the rifles sent after it.
    let across = json!([{ "tick": 61, "side": "red", "order":
        { "kind": "move", "units": [2], "gesture": 1, "goal": [950, 400], "route": "shortest" } }]);
    let mut setup = scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_recon", "position": [100, 300] },
            { "side": "blue", "kind": "test_rifle", "position": [100, 250] },
            { "side": "red", "kind": "test_rifle", "position": [950, 300], "engagement": "return_fire_only" },
        ]),
        json!([]),
        across,
    );
    let mut optics = common::game();
    sim::fixtures::patch_catalog(
        &mut optics,
        "units",
        "test_recon",
        json!({"sensors":{"ground_m":1000}}),
    );
    setup.rules.catalog = serde_json::from_value::<contract::scenario::Rules>(optics)
        .unwrap()
        .catalog;
    let mut b = Battle::new(&setup, 5);
    run(&mut b, 60);
    let id = b.observe(Side::Blue).identified[0].id;
    Commander::new().ok(
        &mut b,
        Side::Blue,
        Order::Attack {
            units: vec![UnitId(1)],
            target: TargetRef::Identified { id },
        },
    );
    let red = |b: &Battle| own(b, Side::Red, 2).position;
    let from = red(&b);
    let mut ticks = 0;
    while (red(&b)[1] - from[1]).abs() < 60.0 {
        b.step();
        ticks += 1;
        assert!(ticks < 3000, "red never crossed: {:?}", red(&b));
    }
    let searches = b.route_searches(Side::Blue);
    assert!(
        searches <= 3,
        "{searches} routes for a target that moved 60 m, 800 m away"
    );
    // The squad still closes on where its target went, and opens fire.
    let shots = run(&mut b, 4500);
    assert!(
        shots.iter().any(|shot| shot.1 == 1),
        "the rifles never reached their target: {:?} after {:?}",
        own(&b, Side::Blue, 1).position,
        red(&b)
    );
}
