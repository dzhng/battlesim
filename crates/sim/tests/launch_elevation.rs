//! Guided canister launch behavior.

use crate::common::*;
use contract::ballistics::WeaponBallistics;
use contract::random::Rng;
use sim::flight::{
    fire_round, guided_timed_path, solve_launch_past, Aim, FlightConfigError, Projectiles,
};
use sim::math::{v3, V3};

#[test]
fn a_launch_elevation_is_bounded_and_requires_guidance() {
    let guided = weapon("atgm");
    let refused = |w: WeaponBallistics| {
        matches!(
            config().profile(&w),
            Err(FlightConfigError::LaunchElevation(_))
        )
    };
    assert!(refused(WeaponBallistics {
        launch_elevation_deg: Some(91.0),
        ..guided.clone()
    }));
    assert!(refused(WeaponBallistics {
        launch_elevation_deg: Some(45.0),
        turn_deg_s: None,
        ..weapon("rifle")
    }));
    assert!(refused(WeaponBallistics {
        launch_hold_s: Some(0.6),
        ..weapon("aa_long")
    }));
    assert!(refused(WeaponBallistics {
        launch_elevation_deg: Some(90.0),
        launch_hold_s: Some(-0.1),
        ..weapon("aa_long")
    }));
}

#[test]
fn vertical_canister_missiles_hold_every_coordinate_before_guiding() {
    for weapon_name in ["s300_missile", "s350_missile"] {
        let row = weapon(weapon_name);
        let profile = config().profile(&row).unwrap();
        assert_eq!(row.launch_elevation_deg, Some(90.0));
        assert_eq!(row.launch_hold_s, Some(0.6));
        assert!((profile.launch_elevation_rad.unwrap().to_degrees() - 90.0).abs() < 1e-12);
        let origin = v3(100.0, 100.0, 7.0);
        let aim = Aim {
            origin,
            target: v3(1200.0, 100.0, 250.0),
            target_velocity: V3::default(),
            carrier: V3::default(),
        };
        let world = flat([2400.0, 600.0], "");
        let solved = solve_launch_past(&world, &config(), &profile, &aim, None).unwrap();
        assert!(
            solved.velocity.xy().length() < 1e-9,
            "{:?}",
            solved.velocity
        );
        let (launch, _) = fire_round(
            &world,
            &config(),
            &profile,
            &aim,
            0.0,
            &mut Rng::new(7),
            None,
            |_| false,
            |_| false,
        )
        .unwrap()
        .expect("the target is in range");
        assert!(launch.velocity.z > 0.0);
        assert!(
            launch.velocity.xy().length() < 1e-9,
            "{:?}",
            launch.velocity
        );
        assert!((launch.velocity.length() - profile.speed_mps).abs() < 1e-9);

        let path = guided_timed_path(
            &config(),
            &profile,
            origin,
            launch.velocity,
            aim.target,
            profile.launch_hold_s,
        );
        for (point, time) in path {
            assert!(time <= profile.launch_hold_s + 1e-12);
            assert!(
                (point - origin).xy().length() < 1e-7,
                "{weapon_name} at {time}s: {point:?}"
            );
            assert!(
                point.z > origin.z || time == 0.0,
                "{weapon_name} at {time}s: {point:?}"
            );
        }

        let mut store = Projectiles::new(config());
        let id = store.launch(launch);
        let mut previous_z = origin.z;
        for tick in 1..=18 {
            let _ = fly(&mut store, &world, 1, |_| Vec::new());
            let projectile = store.get(id).expect("the missile is still flying");
            assert!(
                projectile.position.z > previous_z,
                "tick {tick}: {:?}",
                projectile.position
            );
            assert!(
                projectile.velocity.xy().length() < 1e-8,
                "tick {tick}: {:?}",
                projectile.velocity
            );
            assert!(
                (projectile.position - origin).xy().length() < 1e-7,
                "tick {tick}: {:?}",
                projectile.position
            );
            previous_z = projectile.position.z;
        }
    }
}
