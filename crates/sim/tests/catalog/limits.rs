use super::*;

/// No hull is admitted past the absolute limits for its drive: what the map
/// generator and the route planner are proven against (tests drive hulls
/// built at the limits), so every unit within them is covered too.
#[test]
fn a_hull_past_its_drives_limits_is_refused_at_load() {
    for (unit, patch, limit) in [
        (
            "test_tank",
            json!({ "body": { "hull": { "half_extents_m": [3.5, 2.2, 1.2] } } }),
            "tracked half width",
        ),
        (
            "test_tank",
            json!({ "body": { "hull": { "half_extents_m": [4.9, 1.8, 1.2] } } }),
            "tracked half length",
        ),
        (
            "test_supply",
            json!({ "body": { "hull": { "half_extents_m": [3.0, 1.7, 1.8] } } }),
            "wheeled half width",
        ),
        (
            "test_supply",
            json!({ "body": { "hull": { "half_extents_m": [6.3, 1.4, 1.8] } } }),
            "wheeled half length",
        ),
        (
            "test_supply",
            json!({ "mobility": { "wheeled": { "turning_radius_m": 8.5 } } }),
            "wheeled turning radius",
        ),
    ] {
        let mut fixture = fixtures::test_game();
        fixtures::patch_catalog(&mut fixture, "units", unit, patch);
        let rules: Rules = serde_json::from_value(fixture).unwrap();
        let error = sim::admit(&rules).expect_err(limit).to_string();
        assert!(
            error.contains(unit) && error.contains(limit),
            "{limit}: {error}"
        );
    }
    // The shipped roster stands within them.
    sim::admit(&serde_json::from_value::<Rules>(common::game()).unwrap()).unwrap();
}
