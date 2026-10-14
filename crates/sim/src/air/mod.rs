//! Fixed-wing flight laws. Helicopters remain in [`crate::movement::air`].

pub mod jet;

pub use jet::{fly, velocity, JetLimits, JetState, Steer};

use crate::movement::MovementContext;
use crate::units::{Motion, Unit};

/// Advance one HighAir unit without consulting the helicopter air grid.
pub(crate) fn step_jet(ctx: &MovementContext, unit: &mut Unit) {
    let Motion::Air(flight) = unit.motion else {
        unreachable!("only aircraft fly");
    };
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let (afterburner, burner, turn_sign, fuel_s) = {
        let air = unit.air.as_ref().expect("an aircraft's flight state");
        (air.afterburner, air.burner, air.turn_sign, air.fuel_s)
    };
    // The afterburner's speed is the type's; the toggle only says which way
    // the ramp runs, so switching it off eases back to cruise.
    let boost = unit
        .unit_type(ctx.rules)
        .capabilities
        .air
        .as_ref()
        .and_then(|a| a.afterburner)
        .map_or((1.0, 1.0), |a| (a.speed_multiplier, a.fuel_multiplier));
    let burner_mult = boost.0;
    let fuel_mult = if afterburner { boost.1 } else { 1.0 };
    let cruise = flight.cruise_mps;
    let speed = unit
        .air
        .as_ref()
        .map_or(cruise, |air| air.velocity.xy().length().max(cruise));
    let turn = unit
        .movement_goal()
        .map(|(goal, _)| {
            let to_goal = goal - unit.position.xy();
            if to_goal.length() <= 1e-9 {
                0.0
            } else {
                let wanted = libm::atan2(to_goal.y, to_goal.x);
                crate::math::wrap_angle(wanted - unit.yaw) / (flight.turn_rad_s * dt).max(1e-9)
            }
        })
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let state = JetState {
        pos: unit.position,
        heading: unit.yaw,
        speed_mps: speed,
        burner,
        turn_sign,
    };
    let limits = JetLimits {
        cruise_mps: cruise,
        burner_mult,
        turn_rad_s: flight.turn_rad_s,
        climb_mps: flight.climb_mps,
        ramp_s: ctx.rules.air.jet.afterburner_ramp_s,
        floor_agl_m: ctx.rules.air.jet.floor_agl_m,
    };
    let next = fly(
        state,
        Steer {
            turn,
            agl_m: ctx.rules.air.jet.cruise_agl_m,
            burner: afterburner,
        },
        &limits,
        &|p| ctx.world.height_at(p.x, p.y).unwrap_or(0.0),
        dt,
    );
    let vertical = (next.pos.z - unit.position.z) / dt;
    unit.position = next.pos;
    unit.yaw = next.heading;
    unit.drive_speed_mps = next.speed_mps;
    let air = unit.air.as_mut().expect("an aircraft's flight state");
    let horizontal = velocity(&next);
    air.velocity = crate::math::v3(horizontal.x, horizontal.y, vertical);
    air.burner = next.burner;
    air.turn_sign = next.turn_sign;
    air.fuel_s = (fuel_s - dt * fuel_mult).max(0.0);
    unit.state = contract::observation::MoveState::Moving;
}
