//! Fixed-wing flight laws. Helicopters remain in [`crate::movement::air`].

pub mod edge;
pub mod jet;
pub mod steer;

pub use jet::{fly, velocity, JetLimits, JetState, Steer};
pub use steer::{steer, Bounds, Intent};

use crate::math::v2;
use crate::movement::MovementContext;
use crate::units::{Motion, Unit, UnitOrder};
use crate::weapons::Target;

/// Advance one HighAir unit without consulting the helicopter air grid.
pub(crate) fn step_jet(ctx: &MovementContext, unit: &mut Unit) {
    let Motion::Air(flight) = unit.motion else {
        unreachable!("only aircraft fly");
    };
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let (afterburner, burner, turn_sign, fuel_s, last_goal) = {
        let air = unit.air.as_ref().expect("an aircraft's flight state");
        (
            air.afterburner,
            air.burner,
            air.turn_sign,
            air.fuel_s,
            air.last_goal,
        )
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
    let current_order = unit.orders.front().cloned();
    let goal = current_order.as_ref().and_then(|order| match order {
        UnitOrder::Move(m) | UnitOrder::AttackMove(m) => Some(m.destination),
        _ => None,
    });
    let on_circle = goal.is_some_and(|centre| {
        let radius = speed / flight.turn_rad_s.max(f64::MIN_POSITIVE);
        ((unit.position.xy() - centre).length() - radius).abs() <= radius * 0.08 + 2.0
    });
    if on_circle && !unit.withdrawing {
        unit.orders.pop_front();
        unit.state = if unit.orders.is_empty() {
            contract::observation::MoveState::Idle
        } else {
            contract::observation::MoveState::Moving
        };
    }
    let current_order = unit.orders.front().cloned();
    let intent = if unit.withdrawing {
        match current_order {
            Some(UnitOrder::Move(m)) => Intent::Exit { to: m.destination },
            _ => Intent::Exit {
                to: unit.position.xy(),
            },
        }
    } else {
        match current_order {
            Some(UnitOrder::Move(m)) | Some(UnitOrder::AttackMove(m)) => Intent::Orbit {
                centre: m.destination,
            },
            Some(UnitOrder::Attack { target, last_known }) => {
                let point = match target {
                    Target::Unit(id) => ctx.knowledge[unit.side.index()]
                        .track(id)
                        .map(|track| track.position)
                        .or_else(|| unit.pursuit.map(|p| p.to.with_z(unit.position.z)))
                        .or_else(|| last_known.map(|p| p.with_z(unit.position.z)))
                        .unwrap_or(unit.position),
                    Target::Ground(point) => point,
                    Target::Round(round) => round.point,
                    Target::Contact(_) => unit
                        .pursuit
                        .map(|p| p.to.with_z(unit.position.z))
                        .or_else(|| last_known.map(|p| p.with_z(unit.position.z)))
                        .unwrap_or(unit.position),
                };
                Intent::Orbit { centre: point.xy() }
            }
            _ => Intent::Orbit {
                centre: last_goal.unwrap_or_else(|| unit.position.xy()),
            },
        }
    };
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
        edge_margin_m: ctx.rules.air.jet.edge_margin_m,
        cruise_agl_m: ctx.rules.air.jet.cruise_agl_m,
    };
    let bounds = Bounds::new(v2(0.0, 0.0), v2(ctx.world.width(), ctx.world.depth()));
    let mut command = steer(&intent, &state, &limits, &bounds, dt);
    command.burner = afterburner;
    let next = fly(
        state,
        command,
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
    if goal.is_some() || air.last_goal.is_none() {
        air.last_goal = Some(next.pos.xy());
    }
    air.fuel_s = (fuel_s - dt * fuel_mult).max(0.0);
    unit.state = contract::observation::MoveState::Moving;
}
