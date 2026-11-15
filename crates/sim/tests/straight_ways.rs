//! Whether a straight way fits a mover: the whole strip its footprint
//! sweeps, judged the same wherever along the line the way begins and ends.
use crate::common;
use contract::map::MapDefinition;
use sim::math::v2;
use sim::navigation::{NavBase, NavGrid};
use sim::world::WorldGeometry;
use std::sync::Arc;

/// A parked car standing askew of a straight line, its corner reaching
/// `poke` metres into the strip a jeep sweeps along that line (a negative
/// `poke` stays that far out of it). Every way along the line that passes
/// the car, begun and ended anywhere, fits or does not.
fn ways_past_an_askew_car(poke: f64) -> Vec<bool> {
    let rules = common::rules();
    let jeep = sim::units::ground_mobility(rules.catalog.by_id("test_jeep"), &rules).unwrap();
    let (half, yaw, line) = ([2.1, 0.9], 0.3_f64, 100.0);
    // The car's box grown by the jeep's half width on every side reaches
    // this far below its centre.
    let r = jeep.half_width_m;
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
            grid.route_fits(from, &[to], &jeep)
        })
        .collect()
}

#[test]
fn a_body_corner_in_the_swept_strip_closes_the_way_however_it_is_sampled() {
    let closed = ways_past_an_askew_car(0.03);
    assert!(
        closed.iter().all(|fits| !fits),
        "some ways slipped past: {closed:?}"
    );
    let open = ways_past_an_askew_car(-0.03);
    assert!(
        open.iter().all(|fits| *fits),
        "some ways were refused: {open:?}"
    );
}
