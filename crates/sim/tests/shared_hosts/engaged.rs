//! A garrison is engaged where its enemy sees it: at its windows.
use contract::command::{Engagement, Order};
use contract::ids::{Side, UnitId};
use contract::observation::HoldPhase;
use serde_json::json;
use sim::battle::Battle;
use sim::weapons::Target;

use super::{accepted, living, occupy, phase, BUILDING, TEMPLATE};
use crate::common;

/// Two blue rifle squads fill a house between them (the test seats exactly
/// both), soldiers on every facade; a red tank off its corner sees two. Once it may fire at
/// will, it fires at the soldiers it sees at the windows, though the middle
/// of each squad its side identifies lies inside the walls.
#[test]
fn a_tank_that_sees_a_shared_garrison_at_its_windows_fires_on_it() {
    let (first, second, tank) = (0u32, 1u32, 2u32);
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [370.0, 300.0],
          "engagement": "return_fire_only" },
        { "side": "blue", "kind": "test_rifle", "position": [340.0, 300.0],
          "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [450.0, 270.0],
          "engagement": "return_fire_only" },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    let start = Battle::new(&setup, 1);
    setup.rules.buildings.capacity_soldiers =
        (living(&start, first) + living(&start, second)) as u32;
    let mut b = Battle::new(&setup, 1);
    accepted(&mut b, 1, occupy(first));
    accepted(&mut b, 2, occupy(second));
    for t in 0.. {
        if [first, second].map(|id| phase(&b, id)) == [Some(HoldPhase::Inside); 2] {
            break;
        }
        assert!(t < 3000, "both squads inside within 3000 ticks");
        b.step();
    }

    let building = b.world().prop(BUILDING).cloned().unwrap();
    let inside_walls = |p: [f64; 3]| {
        building
            .footprint()
            .contains(sim::math::v2(p[0], p[1]), 0.0)
    };
    let ack = b.accept(common::command(
        Side::Red,
        1,
        Order::SetEngagement {
            units: vec![UnitId(tank)],
            policy: Engagement::FireAtWill,
        },
    ));
    assert!(ack.error.is_none(), "{ack:?}");
    for _ in 0..1500 {
        b.step();
        if !b.rounds().any(|(_, r)| r.unit == UnitId(tank)) {
            continue;
        }
        let middles: Vec<_> = (b.observe(Side::Red).identified.iter())
            .map(|e| e.position)
            .collect();
        assert!(
            !middles.is_empty() && middles.iter().all(|&p| inside_walls(p)),
            "every squad red identifies has its middle inside the walls: {middles:?}"
        );
        let locks: Vec<_> = (b.unit(UnitId(tank)).unwrap().mounts.iter())
            .filter_map(|m| m.lock.as_ref().map(|l| l.target))
            .collect();
        assert!(
            locks.iter().any(|t| [first, second]
                .iter()
                .any(|&id| *t == Target::Unit(UnitId(id)))),
            "the tank fires at a squad in the house: {locks:?}"
        );
        return;
    }
    panic!(
        "the tank never fires at the garrison it sees: {:?}",
        b.unit(UnitId(tank)).unwrap().mounts[0].reason
    );
}
