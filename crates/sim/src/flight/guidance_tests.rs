//! Deterministic physical flight after a missile's one attempt misses.
use super::*;

fn field() -> (Projectiles, WorldGeometry) {
    let rules: contract::scenario::Rules =
        serde_json::from_value(crate::fixtures::test_rules()).unwrap();
    let map = serde_json::from_value(serde_json::json!({
        "size":[5000,5000],"fog_cell_m":40,"height_grid_m":40,"slope_cutoff_deg":35
    }))
    .unwrap();
    (
        Projectiles::new(FlightConfig::new(&rules.physics.flight, rules.tick_hz).unwrap()),
        WorldGeometry::new(&map, &rules),
    )
}

#[test]
fn a_missile_that_passes_its_aim_never_circles_back_or_reacquires() {
    let (mut store, world) = field();
    let id = store.launch(Launch {
        origin: v3(2000.0, 2000.0, 300.0),
        carrier: V3::default(),
        velocity: v3(200.0, 0.0, 0.0),
        gravity_scale: 1.0,
        lifetime_s: 8.0,
        suppression_radius_m: 0.0,
        shooter: None,
        guidance: Some(Guidance {
            point: v3(2100.0, 2060.0, 300.0),
            turn_rad_s: 30_f64.to_radians(),
            supported: false,
            launch_hold_s: 0.0,
            loft: None,
        }),
        motor: None,
        fall: None,
    });
    let mut events = Vec::new();
    let mut path = Vec::new();
    let ticks = (6.0 / store.config().tick_s()).round() as usize;
    for _ in 0..ticks {
        advance_projectiles(
            &mut store,
            &world,
            &[],
            &mut events,
            &mut |_: &ImpactContext| ImpactDecision::Stop,
        );
        path.push(
            store
                .get(id)
                .expect("a miss stays live in physical flight")
                .position,
        );
    }
    let reversed = path.windows(2).find(|p| p[1].x < p[0].x);
    assert!(
        reversed.is_none(),
        "turned back around the stale point: {reversed:?}"
    );
    assert!(
        path.last().unwrap().z < 290.0,
        "the missed round falls normally"
    );
    // A new command towards a point behind it cannot restart homing.
    store.steer_seeker(id, v3(1000.0, 2000.0, 300.0));
    let before = store.get(id).unwrap().position;
    for _ in 0..30 {
        advance_projectiles(
            &mut store,
            &world,
            &[],
            &mut events,
            &mut |_: &ImpactContext| ImpactDecision::Stop,
        );
    }
    assert!(store.get(id).unwrap().position.x > before.x);
    assert!(store.get(id).unwrap().guidance.is_none());
    for _ in 0..60 {
        if store.get(id).is_none() {
            break;
        }
        advance_projectiles(
            &mut store,
            &world,
            &[],
            &mut events,
            &mut |_: &ImpactContext| ImpactDecision::Stop,
        );
    }
    assert!(
        matches!(events.last(), Some(FlightEvent::Expired(Expired {
        projectile, cause: Expiry::Lifetime, ..
    })) if *projectile == id),
        "the ordinary lifetime ends the live miss: {events:?}"
    );
}
