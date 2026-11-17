//! Whether a straight way fits a mover: the whole strip its footprint
//! sweeps, judged the same wherever along the line the way begins and ends.
//! A way that fits takes a time a mover can finish.
use crate::common;
use contract::map::MapDefinition;
use sim::math::v2;
use sim::navigation::{NavBase, NavGrid};
use sim::world::WorldGeometry;
use std::sync::Arc;

/// A parked car standing askew of a straight line, its corner reaching
/// `poke` metres into the strip `unit` sweeps along that line (a negative
/// `poke` stays that far out of it). Every way along the line that passes
/// the car, begun and ended anywhere: whether it fits, and how long it takes.
fn ways_past_an_askew_car(unit: &str, poke: f64) -> Vec<(bool, f64)> {
    let rules = common::rules();
    let mover = sim::units::ground_mobility(rules.catalog.by_id(unit), &rules).unwrap();
    let (half, yaw, line) = ([2.1, 0.9], 0.3_f64, 100.0);
    // The car's box grown by the mover's half width on every side reaches
    // this far below its centre.
    let r = mover.half_width_m;
    let reach = (half[0] + r) * yaw.sin() + (half[1] + r) * yaw.cos();
    let centre = [200.0, line + reach - poke];
    let map: MapDefinition = serde_json::from_value(serde_json::json!({
        "size": [400, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [{ "kind": "parked_car", "center": centre, "yaw": yaw,
                    "half_extents": [half[0], half[1], 0.75] }],
    }))
    .unwrap();
    let w = WorldGeometry::new(&map, &rules);
    let grid = NavGrid::new(Arc::new(NavBase::build(&w, w.props(), 0.3)));
    (0..40)
        .map(|k| {
            let from = v2(150.0 + 0.037 * k as f64, line);
            let to = v2(250.0 - 0.053 * k as f64, line);
            (
                grid.route_fits(from, &[to], &mover),
                grid.route_time(from, &[to], &mover),
            )
        })
        .collect()
}

#[test]
fn a_body_corner_in_the_swept_strip_closes_the_way_however_it_is_sampled() {
    let closed = ways_past_an_askew_car("test_jeep", 0.03);
    assert!(
        closed.iter().all(|(fits, _)| !fits),
        "some ways slipped past: {closed:?}"
    );
    let open = ways_past_an_askew_car("test_jeep", -0.03);
    assert!(
        open.iter().all(|(fits, _)| *fits),
        "some ways were refused: {open:?}"
    );
}

/// A way that passes beside a body is driven at the speed of the ground
/// under it, whether the mover could shove the body or not: a body it does
/// not touch neither stops nor slows it.
#[test]
fn a_way_past_a_body_takes_the_time_of_its_ground() {
    for unit in ["test_jeep", "test_supply"] {
        let beside = ways_past_an_askew_car(unit, -0.03);
        let clear = ways_past_an_askew_car(unit, -50.0);
        for (k, ((fits, time), (_, alone))) in beside.iter().zip(&clear).enumerate() {
            assert!(*fits, "{unit}: way {k} was refused");
            assert!(
                (time - alone).abs() < 1e-9,
                "{unit}: way {k} takes {time} s beside the car and {alone} s without it"
            );
        }
    }
}
