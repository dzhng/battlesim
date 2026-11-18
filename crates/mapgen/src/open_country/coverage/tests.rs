//! The ground-sight certificate's own cases.
use super::*;

fn physics() -> GenerationPhysics {
    GenerationPhysics::from_rules_json(&sim::fixtures::test_rules().to_string()).unwrap()
}
fn map(size: f64, bounds: [f64; 4]) -> MapDefinition {
    serde_json::from_value(serde_json::json!({
        "size":[size,size], "fog_cell_m":8., "height_grid_m":4., "slope_cutoff_deg":35.,
        "forests":[{"shape":{"kind":"polygon","ring":[
            [bounds[0],bounds[1]],[bounds[2],bounds[1]],
            [bounds[2],bounds[3]],[bounds[0],bounds[3]]
        ]}}], "props":[],"surfaces":[],"relief":[],"buildings":[],"bridges":[]
    }))
    .unwrap()
}

#[test]
fn the_current_twenty_kilometre_forest_profile_fits_the_proof_envelope() {
    let physics = physics();
    let map = map(20_000., [0., 0., 20_000., 20_000.]);
    let c = Coverage::new(&map, &physics, 100.).unwrap();
    for p in [[50., 50.], [10_050., 10_050.], [19_950., 19_950.]] {
        assert!(c.covered(sub(p, [50., 50.]), add(p, [50., 50.])));
    }
    c.check().unwrap();
    eprintln!(
        "20km full-forest charged structural work={} witnesses={}",
        PROOF_OPERATIONS - c.queries.work.0.get(),
        c.points.len()
    );
}

#[test]
fn whole_rectangle_reach_uses_its_furthest_corner() {
    let mut physics = physics();
    physics.forests.rule.trunk_spacing_m = 9.;
    physics.forests.rule.trunk_jitter = 0.;
    physics.forests.rule.canopy_radius_m = 12.;
    let map = map(2_000., [999.5, 999.5, 1008.5, 1008.5]);
    let c = Coverage::new(&map, &physics, 100.).unwrap();
    assert!(c.covered([954., 604.], [1054., 704.]));
    // Its centre is within reach, but its far corner is not.
    assert!(!c.covered([954., 584.], [1054., 704.]));
}

#[test]
fn an_outward_only_patch_cannot_certify_an_in_bounds_fog_cut() {
    let mut physics = physics();
    physics.forests.rule.trunk_spacing_m = 9.;
    physics.forests.rule.trunk_jitter = 0.;
    physics.forests.rule.canopy_radius_m = 12.;
    let map = map(2_000., [7.5, 999.5, 16.5, 1008.5]);
    let c = Coverage::new(&map, &physics, 100.).unwrap();
    assert_eq!(c.points, vec![[12., 1004.]]);
    // The only foliage is west of this eye. Its attenuated rays leave
    // the map before stopping, so it supplies no in-bounds dark bit.
    assert!(!c.covered([112., 1004.], [112., 1004.]));
}

#[test]
fn edited_spacing_is_refused_before_the_shared_sampler_can_expand_it() {
    let mut physics = physics();
    physics.forests.rule.trunk_spacing_m = 1e-9;
    let map = map(2_000., [900., 900., 1_100., 1_100.]);
    let error = Coverage::new(&map, &physics, 100.).err().unwrap();
    assert!(error.contains("operation envelope"), "{error}");
}

#[test]
fn a_three_cell_witness_keeps_its_far_bit_dark_across_native_ray_phases() {
    let mut physics = physics();
    physics.forests.rule.trunk_spacing_m = 9.;
    physics.forests.rule.trunk_jitter = 0.;
    physics.forests.rule.canopy_radius_m = 12.;
    let map = map(2_000., [999.5, 999.5, 1008.5, 1008.5]);
    let c = Coverage::new(&map, &physics, 100.).unwrap();
    assert_eq!(c.points, vec![[1004., 1004.]]);
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let mut rules = rules;
    rules.forests = physics.forests.clone();
    let world = sim::world::WorldGeometry::new(&map, &rules);
    let jeep = rules.catalog.by_id("test_jeep");
    let sight = sim::sight::Sight {
        forward: 0.,
        shape: jeep.sensors.sight_shape,
        range: jeep.sensors.ground_m,
    };
    let rays = (libm::ceil(TAU * sight.max_range() / map.fog_cell_m) as usize).max(64);
    let far = libm::floor(sight.range / map.fog_cell_m) * map.fog_cell_m;
    let delta = TAU / rays as f64;
    let mut grid = sim::visibility::OcclusionGrid::new(&world, map.fog_cell_m);
    for radial in 0..32 {
        let distance = c.reach - radial as f64 * map.fog_cell_m / 32.;
        for r in 0..rays {
            for phase in [0., 0.5] {
                let angle = (r as f64 + phase) * delta;
                let at = sub(c.points[0], scale(direction(angle), distance));
                let eye = sim::math::v3(at[0], at[1], rules.physics.eye_m(jeep));
                let mut field = grid.field();
                sim::visibility::sweep(&world, &mut grid, &rules.sensors, eye, &sight, &mut field);
                let nearest = libm::round(angle / delta) * delta;
                let end = add(at, scale(direction(nearest), far));
                assert!(
                    !field.visible(end[0], end[1]),
                    "far cell refilled at ray {r} phase {phase}, reach {}",
                    c.reach
                );
            }
        }
    }
}
