use super::*;

/// A hull at either drive's width limit finds its way between two walls
/// standing 1.2 m clear of it either side (a tank in a court's lane), and
/// keeps its half width from them all the way, whatever the way's heading and
/// however it lies across the grid's cells: a hull goes wherever it fits.
#[test]
fn a_hull_at_the_width_limit_fits_a_way_a_little_wider_than_itself() {
    let limits = contract::HullLimits::SHIPPED;
    for half_width_m in [limits.tracked.half_width_m, limits.wheeled.half_width_m] {
        let hull = Mobility {
            half_width_m,
            push: PushClass::Medium,
            ..TANK
        };
        let way = 2.0 * half_width_m + 2.4;
        for deg in (0..=45).step_by(5) {
            let (s, c) = f64::from(deg).to_radians().sin_cos();
            // Across the cells in steps of a fifth of one.
            for shift in [0.0, 0.4, 0.8, 1.2, 1.6] {
                let at = |along: f64, right: f64| {
                    v2(200.0 + c * along + s * right, 100.0 + s * along - c * right)
                };
                let wall = |side: f64| {
                    let p = at(0.0, shift + side * (way / 2.0 + 0.5));
                    format!(
                        r#"{{"kind":"wall","center":[{},{}],"yaw":{},"half_extents":[180,0.5,3]}}"#,
                        p.x,
                        p.y,
                        f64::from(deg).to_radians()
                    )
                };
                let w = world(&format!(r#","props":[{},{}]"#, wall(1.0), wall(-1.0)));
                let (from, goal) = (at(-40.0, shift), at(40.0, shift));
                let plan = grid(&w).plan(from, goal, &hull, RoutePolicy::Shortest);
                assert!(
                    matches!(&plan, Plan::Route(r) if route_length(from, r) < 90.0),
                    "a hull {:.1} m wide is turned from a way {way:.1} m wide at {deg}°, \
                     {shift} m across: {plan:?}",
                    2.0 * half_width_m
                );
                // How far a point stands from the nearer wall's face.
                let room = |p: V2| {
                    let across = (p.x - 200.0) * s - (p.y - 100.0) * c;
                    way / 2.0 - (across - shift).abs()
                };
                let route = route(plan);
                assert!(
                    all_along(from, &route, |p| room(p) >= half_width_m - 1e-6),
                    "a hull {:.1} m wide scrapes a wall of a way {way:.1} m wide at {deg}°, \
                     {shift} m across: {route:?}",
                    2.0 * half_width_m
                );
            }
        }
    }
}
