//! Road access is measured to the plan's main settlement, not whichever
//! satellite happened to grow the most built ground.
use mapgen::layout::{measure, PresetDefinitions};
use mapgen::MapPlan;
use serde_json::json;

#[test]
fn a_larger_satellite_does_not_move_the_main_road_destination() {
    let square = |x: f64, y: f64, half: f64| {
        vec![
            [x - half, y - half],
            [x + half, y - half],
            [x + half, y + half],
            [x - half, y + half],
        ]
    };
    let settlement = |id: &str, x: f64, half: f64| {
        json!({
            "id": id, "class": "test_settlement", "center": [x, 2000.0],
            "outline": square(x, 2000.0, half),
            "districts": [{"id": id, "kind": "test_district", "ring": square(x, 2000.0, half),
                "area_m2": 4.0 * half * half, "anchor": [x, 2000.0], "categories": []}]
        })
    };
    let road = |line| json!({"kind": "country_road", "shape": {"kind": "stroke", "points": line, "width_m": 10.0}});
    let mut value = json!({
        "size": [4000.0, 4000.0], "fog_cell_m": 8.0, "height_grid_m": 4.0, "slope_cutoff_deg": 35.0,
        "settlements": [settlement("main", 2000.0, 100.0), settlement("satellite", 500.0, 200.0)],
        "surfaces": [road(vec![[2000.0, 0.0], [2000.0, 2000.0], [2000.0, 4000.0]]),
            road(vec![[0.0, 2000.0], [500.0, 2000.0], [2000.0, 2000.0], [4000.0, 2000.0]])]
    });
    let presets =
        PresetDefinitions::from_json(include_str!("../../../fixtures/map-presets.json")).unwrap();
    // Class ranks and approach tuning cannot pick a different main settlement.
    for half in [200.0, 300.0] {
        value["settlements"][1] = settlement("satellite", 500.0, half);
        let plan: MapPlan = serde_json::from_value(value.clone()).unwrap();
        let transit = measure(&plan, &presets).transit;
        assert_eq!(transit.centre_m, 0.0);
        assert_eq!(transit.top.unwrap().route_m, 2000.0);
        assert_eq!(transit.bottom.unwrap().route_m, 2000.0);
        assert_eq!(transit.top_bottom.unwrap().route_m, 4000.0);
    }
}
