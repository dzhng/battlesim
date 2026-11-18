use super::*;

#[test]
fn an_authored_gravity_scale_sets_the_rounds_screen_arc() {
    // A flat gun arc and a lob use independent authored speed and gravity.
    // The solved elevation and flown impact follow that chosen arc.
    for (name, speed_mps, gravity_scale, distance) in
        [("flat", 400.0, 0.1, 800.0), ("lob", 100.0, 5.35, 150.0)]
    {
        let mut row = weapon("rifle");
        row.speed_mps = speed_mps;
        row.gravity_scale = gravity_scale;
        row.range_m = distance;
        let p = config().profile(&row).unwrap();
        let world = flat([distance + 400.0, 400.0], "");
        let o = v3(100.0, 200.0, 2.0);
        let target = o + v3(distance, 0.0, 0.0);
        let aim = Aim {
            origin: o,
            target,
            target_velocity: V3::default(),
            carrier: V3::default(),
        };
        let s = solve_launch_past(&world, &config(), &p, &aim, None).unwrap();
        let (chosen, _) = analytic_elevations(speed_mps, G * gravity_scale, distance, 0.0);
        let rel = (elevation(s.velocity) - chosen).abs() / chosen;
        assert!(
            rel < 1e-3,
            "{name}: elevation {} vs the authored arc's {chosen}",
            elevation(s.velocity)
        );
        let flown_s = distance / (p.speed_mps * chosen.cos());
        assert!(
            (s.time_of_flight_s - flown_s).abs() < 1e-3 * flown_s,
            "{name}: {} s to {distance} m, the slowed round's {flown_s} s",
            s.time_of_flight_s
        );
        let body = Mover::standing(7, 1, tank_shape(), target - v3(0.0, 0.0, 2.0));
        let mut store = Projectiles::new(config());
        store.launch(fired(&p, o, s.velocity));
        let dt = config().tick_s();
        let ticks = (s.time_of_flight_s / dt) as u64 + 30;
        let events = fly(&mut store, &world, ticks, |k| vec![body.body(k, dt)]);
        let [(tick, hit)] = impacts(&events)[..] else {
            panic!("{name}: one impact expected: {events:?}")
        };
        assert_eq!(hit.struck, Struck::Body(sim::flight::BodyId(7)), "{name}");
        let when = event_time(tick, &FlightEvent::Impact(hit), dt);
        assert!(
            (when - s.time_of_flight_s).abs() < dt,
            "{name}: struck at {when} s"
        );
    }
}

#[test]
fn shipped_indirect_fire_reaches_its_range_under_the_high_air_floor() {
    let rules = rules();
    let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz).unwrap();
    let weapons: Vec<_> = rules
        .weapons
        .iter()
        .filter(|(_, weapon)| weapon.ballistics.trajectory == Trajectory::Indirect)
        .collect();
    assert!(!weapons.is_empty());
    for (name, weapon) in weapons {
        let row = &weapon.ballistics;
        let profile = config.profile(row).unwrap();
        let world = flat([row.range_m + 400.0, 400.0], "");
        let aim = Aim {
            origin: v3(100.0, 200.0, 1.4),
            target: v3(100.0 + row.range_m, 200.0, 0.0),
            target_velocity: V3::default(),
            carrier: V3::default(),
        };
        let solution = solve_launch_past(&world, &config, &profile, &aim, None)
            .unwrap_or_else(|error| panic!("{name} cannot reach {} m: {error:?}", row.range_m));
        assert_eq!(
            solution.arc,
            ArcKind::High,
            "{name} must prefer the indirect arc"
        );
        assert!(
            solution.time_of_flight_s < 12.0,
            "{name} takes {:.1} seconds to reach its target",
            solution.time_of_flight_s
        );
        let apex = predicted_path(
            &config,
            &profile,
            aim.origin,
            solution.velocity,
            solution.time_of_flight_s,
        )
        .iter()
        .map(|point| point.z)
        .fold(0.0, f64::max);
        assert!(
            (rules.air.rotor.ceiling_agl_m..rules.air.high_air.floor_agl_m).contains(&apex),
            "{name} reaches an altitude of {apex:.1} m"
        );
    }
}
