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
