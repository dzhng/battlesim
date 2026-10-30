use super::*;

/// The lane a vehicle drives stays clear on a 7 m street and on a 10 m
/// avenue: no car stands nearer the middle than the catalog's widest hull
/// and the preset margin, nor on the carriageway. The street parks along one
/// side and the avenue along both. A wider hull in the catalog moves the
/// cars out by as much: the lane is read from the catalog.
#[test]
fn the_driven_lane_stays_clear_and_a_narrow_street_parks_one_side() {
    let presets = presets_with(cars_only);
    let catalog = rules().catalog;
    let mut fixture = game();
    let wide = catalog
        .indices()
        .filter_map(|unit| catalog.get(unit).ground_hull())
        .map(|h| h.half_extents_m[1])
        .fold(0.0, f64::max)
        + 1.1;
    let wider = {
        let widest = catalog
            .indices()
            .max_by(|a, b| {
                let width = |unit| {
                    catalog
                        .get(unit)
                        .ground_hull()
                        .map_or(0.0, |h| h.half_extents_m[1])
                };
                width(*a).total_cmp(&width(*b))
            })
            .unwrap();
        sim::fixtures::patch_catalog(
            &mut fixture,
            "units",
            catalog.id(widest),
            json!({ "body": { "hull": { "half_extents_m": [3.5, wide, 1.2] } } }),
        );
        // A hull wider than any the game admits.
        let rules = serde_json::from_value::<Rules>(fixture).unwrap();
        rules
            .catalog
            .check_limits(
                sim::fixtures::lift_hull_limits(),
                contract::AirSpeedLimits::SHIPPED,
            )
            .unwrap();
        rules.catalog
    };
    assert!(widest_hull(&wider) > widest_hull(&catalog) + 1.0);
    for (width, sides) in [(7.0, 1), (10.0, 2)] {
        let plan = town(&[street(width)], &[]);
        // The nearest any car comes to the street's middle, and how many
        // sides of it cars stand along.
        let nearest = |catalog: &Catalog| {
            let props = place(&plan, &presets, catalog, 1);
            let cars = cars(&props);
            assert!(
                cars.len() >= 20,
                "{} cars on a {width} m street",
                cars.len()
            );
            let north = cars.iter().filter(|car| car.center[1] > 1000.0).count();
            let parked = usize::from(north > 0) + usize::from(north < cars.len());
            let near = cars
                .iter()
                .flat_map(|car| corners(car))
                .map(|corner| (corner[1] - 1000.0).abs())
                .fold(f64::INFINITY, f64::min);
            assert!(
                near >= widest_hull(catalog) + LANE_MARGIN_M && near > width / 2.0,
                "on a {width} m street a car stands {near:.2} m from the middle"
            );
            (near, parked)
        };
        let ((narrow_hull, parked), (wide_hull, _)) = (nearest(&catalog), nearest(&wider));
        assert_eq!(
            parked, sides,
            "a {width} m street parks along {parked} sides"
        );
        let moved = wide_hull - narrow_hull;
        let grew = widest_hull(&wider) - widest_hull(&catalog);
        assert!(
            (moved - grew).abs() < 0.011,
            "a hull {grew:.2} m wider moved the cars of a {width} m street {moved:.2} m"
        );
    }
}
