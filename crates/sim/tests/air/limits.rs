use super::*;

#[test]
fn an_air_hull_past_its_flight_models_limits_is_refused() {
    let mut fixture = fixture();
    let past = contract::HullLimits::SHIPPED.rotor.half_length_m + 0.5;
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "body": { "hull": { "half_extents_m": [past, 1.2, 1.6] } } }),
    );
    let rules = serde_json::from_value::<Rules>(fixture).unwrap();
    let error = sim::admit(&rules).expect_err("past the rotor limit");
    assert!(error.to_string().contains("rotor half length"), "{error}");
}

#[test]
fn an_aircraft_cruising_past_its_flight_models_limit_is_refused() {
    // Admission takes explicit limits: a helicopter cruising at 200 km/h is refused
    // under a 150 km/h rotor limit, and admitted under a 250 km/h one.
    let admitted = |limit: f64| {
        let mut fixture = fixture();
        fixtures::patch_catalog(
            &mut fixture,
            "units",
            "test_heli",
            json!({ "mobility": { "air": { "cruise_kmh": 200 } } }),
        );
        let rules: Rules = serde_json::from_value(fixture).unwrap();
        rules
            .catalog
            .check_limits(
                contract::HullLimits::SHIPPED,
                contract::AirSpeedLimits {
                    rotor: limit,
                    ..contract::AirSpeedLimits::SHIPPED
                },
            )
            .map_err(|e| e.to_string())
    };
    let error = admitted(150.0).expect_err("past the rotor speed");
    assert!(error.contains("rotor cruise_kmh"), "{error}");
    admitted(250.0).expect("within the rotor speed");
}

#[test]
fn fixed_wing_admission_uses_the_fixed_wing_hull_limit() {
    let mut fixture = fixture();
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "mobility": { "air": { "layer": "high_air", "flight": "fixed_wing" } },
            "body": { "hull": { "half_extents_m": [10.0, 4.0, 1.6] } }
        }),
    );
    serde_json::from_value::<Rules>(fixture)
        .expect("a fixed-wing hull within the fixed-wing limit is admitted");
}
